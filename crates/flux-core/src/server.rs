//! Localhost control server. The desktop app runs one so `flux-mcp` can drive the open session;
//! `flux-mcp serve` runs one with the UI files for browser-based tests.
//!
//! Security: binds 127.0.0.1 only, requires a bearer token (written to `control.json`, readable
//! only by this user), rejects foreign `Host` and `Origin` headers (DNS rebinding and CSRF).

use crate::api::{error_code, Api};
use anyhow::Result;
use serde_json::{json, Value};
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A random 128-bit token from the OS-seeded hasher keys.
pub fn new_token() -> String {
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i ^ std::process::id() as u64);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

pub struct ServerOpts {
    pub port: u16,
    /// None = no auth (only for `serve --open` in tests).
    pub token: Option<String>,
    /// Folder of UI files to serve at `/`.
    pub static_dir: Option<PathBuf>,
    /// Write `control.json` so `flux-mcp` can attach.
    pub publish: bool,
}

pub struct Server {
    pub port: u16,
    pub token: Option<String>,
    stop: Arc<AtomicBool>,
    control: Option<PathBuf>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
    pub fn stop(mut self) {
        self.shutdown();
    }
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(c) = self.control.take() {
            // only remove our own file
            if read_control(&c)
                .map(|v| v["pid"] == std::process::id())
                .unwrap_or(false)
            {
                let _ = std::fs::remove_file(c);
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
    /// Block until the server stops (used by `flux-mcp serve`).
    pub fn join(mut self) {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn control_path(root: &Path) -> PathBuf {
    root.join("control.json")
}
pub fn read_control(p: &Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(p).ok()?).ok()
}

pub fn start(api: Api, o: ServerOpts) -> Result<Server> {
    let http = tiny_http::Server::http(("127.0.0.1", o.port))
        .map_err(|e| anyhow::anyhow!("could not open the control port: {e}"))?;
    let port = http
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .unwrap_or(o.port);
    let stop = Arc::new(AtomicBool::new(false));
    let mut control = None;
    if o.publish {
        let root = api.lock().store.root.clone();
        let c = control_path(&root);
        crate::store::atomic_write(&c, serde_json::to_string_pretty(&json!({"port": port, "token": o.token, "pid": std::process::id(), "version": env!("CARGO_PKG_VERSION")}))?.as_bytes())?;
        control = Some(c);
    }
    let st = stop.clone();
    let token = o.token.clone();
    let static_dir = o.static_dir.clone();
    let thread = std::thread::Builder::new()
        .name("flux-control".into())
        .spawn(move || {
            while !st.load(Ordering::Relaxed) {
                let req = match http.recv_timeout(std::time::Duration::from_millis(200)) {
                    Ok(Some(r)) => r,
                    Ok(None) => continue,
                    Err(_) => break,
                };
                let api = api.clone();
                let token = token.clone();
                let sd = static_dir.clone();
                // each request on its own thread so a long call (wait:true) doesn't block polling
                let _ = std::thread::Builder::new()
                    .name("flux-req".into())
                    .spawn(move || handle(req, &api, token.as_deref(), sd.as_deref(), port));
            }
        })?;
    Ok(Server {
        port,
        token: o.token,
        stop,
        control,
        thread: Some(thread),
    })
}

fn header<'a>(r: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    r.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn respond(req: tiny_http::Request, status: u16, ctype: &str, body: Vec<u8>) {
    let mut res = tiny_http::Response::from_data(body).with_status_code(status);
    for (k, v) in [
        ("Content-Type", ctype),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
    ] {
        if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
            res.add_header(h);
        }
    }
    let _ = req.respond(res);
}
fn json_res(req: tiny_http::Request, status: u16, v: Value) {
    respond(
        req,
        status,
        "application/json; charset=utf-8",
        serde_json::to_vec(&v).unwrap_or_default(),
    );
}

fn handle(
    mut req: tiny_http::Request,
    api: &Api,
    token: Option<&str>,
    static_dir: Option<&Path>,
    port: u16,
) {
    // DNS rebinding: only our own host names
    let host_ok = header(&req, "Host")
        .map(|h| h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"))
        .unwrap_or(false);
    if !host_ok {
        return json_res(
            req,
            403,
            json!({"ok": false, "error": {"code": "forbidden", "message": "bad host"}}),
        );
    }
    // CSRF: browsers send Origin on cross-site requests; allow only our own origin and the app's webview
    if let Some(o) = header(&req, "Origin") {
        let ok = o == format!("http://127.0.0.1:{port}")
            || o == format!("http://localhost:{port}")
            || o == "tauri://localhost"
            || o == "http://tauri.localhost"
            || o == "https://tauri.localhost";
        if !ok {
            return json_res(
                req,
                403,
                json!({"ok": false, "error": {"code": "forbidden", "message": "bad origin"}}),
            );
        }
    }
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or("/").to_string();
    let method = req.method().clone();
    if path.starts_with("/api/") {
        if path == "/api/health" {
            return json_res(
                req,
                200,
                json!({"ok": true, "app": "flux", "version": env!("CARGO_PKG_VERSION")}),
            );
        }
        if let Some(t) = token {
            let given = header(&req, "Authorization")
                .and_then(|h| h.strip_prefix("Bearer "))
                .or_else(|| header(&req, "X-Flux-Token"));
            if given != Some(t) {
                return json_res(
                    req,
                    401,
                    json!({"ok": false, "error": {"code": "unauthorized", "message": "missing or wrong token"}}),
                );
            }
        }
        if method != tiny_http::Method::Post {
            return json_res(
                req,
                405,
                json!({"ok": false, "error": {"code": "method", "message": "use POST"}}),
            );
        }
        let mut body = vec![];
        let limit = if path == "/api/upload" {
            4 << 30
        } else {
            16 << 20
        };
        if req
            .as_reader()
            .take(limit + 1)
            .read_to_end(&mut body)
            .is_err()
            || body.len() as u64 > limit
        {
            return json_res(
                req,
                413,
                json!({"ok": false, "error": {"code": "too_large", "message": "request too large"}}),
            );
        }
        if path == "/api/upload" {
            return upload(req, api, &url, &body);
        }
        if path != "/api/call" {
            return json_res(
                req,
                404,
                json!({"ok": false, "error": {"code": "not_found", "message": "no such endpoint"}}),
            );
        }
        let v: Value = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => {
                return json_res(
                    req,
                    400,
                    json!({"ok": false, "error": {"code": "bad_json", "message": "body must be JSON"}}),
                )
            }
        };
        let m = v["method"].as_str().unwrap_or("").to_string();
        let r = api.call(&m, v.get("params").cloned().unwrap_or(Value::Null));
        return match r {
            Ok(x) => json_res(req, 200, json!({"ok": true, "result": x})),
            Err(e) => json_res(
                req,
                200,
                json!({"ok": false, "error": {"code": error_code(&e), "message": format!("{e:#}")}}),
            ),
        };
    }
    let Some(dir) = static_dir else {
        return json_res(req, 404, json!({"ok": false}));
    };
    serve_file(req, dir, &path)
}

fn upload(req: tiny_http::Request, api: &Api, url: &str, body: &[u8]) {
    let name = url
        .split_once("name=")
        .map(|(_, n)| n.split('&').next().unwrap_or(""))
        .unwrap_or("upload");
    let name: String = percent_decode(name)
        .chars()
        .filter(|c| c.is_alphanumeric() || " ._-()".contains(*c))
        .take(120)
        .collect();
    let name = if name.trim_matches('.').is_empty() {
        "upload.wav".to_string()
    } else {
        name
    };
    let dir = api
        .lock()
        .store
        .root
        .join("Uploads")
        .join(crate::store::new_id());
    let p = dir.join(&name);
    let r = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&p, body));
    match r {
        Ok(_) => json_res(
            req,
            200,
            json!({"ok": true, "result": {"path": p.display().to_string()}}),
        ),
        Err(e) => json_res(
            req,
            500,
            json!({"ok": false, "error": {"code": "io", "message": e.to_string()}}),
        ),
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && s.is_char_boundary(i + 3) {
            if let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn serve_file(req: tiny_http::Request, dir: &Path, path: &str) {
    let rel = percent_decode(path.trim_start_matches('/'));
    let rel = if rel.is_empty() {
        "index.html".to_string()
    } else {
        rel
    };
    if rel.split(['/', '\\']).any(|c| c == ".." || c.contains(':')) {
        return respond(req, 400, "text/plain", b"bad path".to_vec());
    }
    let p = dir.join(&rel);
    let p = if p.is_dir() { p.join("index.html") } else { p };
    match std::fs::read(&p) {
        Ok(b) => {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let ct = match ext.as_str() {
                "html" => "text/html; charset=utf-8",
                "js" | "mjs" => "text/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "json" | "webmanifest" => "application/json",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "woff2" => "font/woff2",
                "ico" => "image/x-icon",
                _ => "application/octet-stream",
            };
            respond(req, 200, ct, b)
        }
        Err(_) => respond(req, 404, "text/plain", b"not found".to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpStream;

    fn raw(port: u16, req: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }
    fn post(port: u16, host: &str, extra: &str, body: &str) -> String {
        raw(port, &format!("POST /api/call HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()))
    }

    #[test]
    fn auth_host_and_origin_are_enforced() {
        std::env::set_var("FLUX_NULL_AUDIO", "1");
        let d = std::env::temp_dir().join(format!("flux-srv-{}", crate::store::new_id()));
        let api = Api::new(crate::store::Store::at(d.clone()).unwrap(), true);
        let tok = new_token();
        assert_eq!(tok.len(), 32);
        let s = start(
            api,
            ServerOpts {
                port: 0,
                token: Some(tok.clone()),
                static_dir: None,
                publish: true,
            },
        )
        .unwrap();
        let c = read_control(&control_path(&d)).unwrap();
        assert_eq!(c["port"], s.port);
        let host = format!("127.0.0.1:{}", s.port);
        let body = r#"{"method":"app.info","params":{}}"#;
        assert!(post(s.port, &host, "", body).starts_with("HTTP/1.1 401"));
        assert!(
            post(s.port, &host, "Authorization: Bearer nope\r\n", body).starts_with("HTTP/1.1 401")
        );
        assert!(post(
            s.port,
            "evil.example:80",
            &format!("Authorization: Bearer {tok}\r\n"),
            body
        )
        .starts_with("HTTP/1.1 403"));
        assert!(post(
            s.port,
            &host,
            &format!("Authorization: Bearer {tok}\r\nOrigin: https://evil.example\r\n"),
            body
        )
        .starts_with("HTTP/1.1 403"));
        let ok = post(
            s.port,
            &host,
            &format!("Authorization: Bearer {tok}\r\n"),
            body,
        );
        assert!(
            ok.starts_with("HTTP/1.1 200") && ok.contains("\"ok\":true"),
            "{ok}"
        );
        let bad = post(
            s.port,
            &host,
            &format!("Authorization: Bearer {tok}\r\n"),
            r#"{"method":"nope"}"#,
        );
        assert!(bad.contains("unknown_method"), "{bad}");
        s.stop();
        assert!(!control_path(&d).exists(), "control file removed on stop");
    }

    #[test]
    fn static_files_refuse_traversal() {
        let d = std::env::temp_dir().join(format!("flux-srv-{}", crate::store::new_id()));
        std::fs::create_dir_all(d.join("ui")).unwrap();
        std::fs::write(d.join("ui/index.html"), "<p>hi</p>").unwrap();
        std::fs::write(d.join("secret.txt"), "no").unwrap();
        let api = Api::new(crate::store::Store::at(d.join("data")).unwrap(), true);
        let s = start(
            api,
            ServerOpts {
                port: 0,
                token: None,
                static_dir: Some(d.join("ui")),
                publish: false,
            },
        )
        .unwrap();
        let host = format!("127.0.0.1:{}", s.port);
        let r = raw(
            s.port,
            &format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
        );
        assert!(r.contains("<p>hi</p>"));
        let r = raw(
            s.port,
            &format!("GET /../secret.txt HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
        );
        assert!(!r.contains("\r\n\r\nno"), "{r}");
        let r = raw(
            s.port,
            &format!(
                "GET /%2e%2e/secret.txt HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
            ),
        );
        assert!(!r.contains("\r\n\r\nno"), "{r}");
    }
}

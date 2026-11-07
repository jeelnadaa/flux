//! flux-mcp: every Flux method as an MCP tool over stdio.
//!
//! When the Flux app is running, calls go to its open session through the localhost control
//! server (port and token from `control.json`). When it isn't, flux-mcp runs its own engine
//! headless on the same data folder. `flux-mcp serve` runs the HTTP API (and optionally the UI
//! files) for browser tests and scripting.

use anyhow::{bail, Context, Result};
use flux_core::api::{error_code, Api};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

const PROTOCOL: &str = "2025-06-18";

const INSTRUCTIONS: &str = "Flux is a non-destructive audio editor. Every edit is undoable (history_undo). \
Positions are in samples; most position parameters also accept `<name>_s` in seconds. \
Start with app_info or project_get. Import audio with media_import {path, wait:true}. \
Long tasks (stems, export, batch, tts) return a job id: poll job_get {id, wait_s} or pass wait:true. \
Smart tools run on this computer with DSP; transcript, dub and clone are unavailable without models. \
Synthetic speech is always labelled. Flux never uploads audio.";

/// Where calls go.
enum Backend {
    App { port: u16, token: Option<String> },
    Headless(Api),
}

struct Router {
    mode: String,
    headless: Option<Api>,
}

impl Router {
    fn new(mode: &str) -> Router {
        Router {
            mode: mode.to_string(),
            headless: None,
        }
    }

    fn app(&self) -> Option<(u16, Option<String>)> {
        if self.mode == "headless" {
            return None;
        }
        let c = flux_core::server::read_control(&flux_core::server::control_path(
            &flux_core::store::data_dir(),
        ))?;
        let port = c["port"].as_u64()? as u16;
        let token = c["token"].as_str().map(|s| s.to_string());
        let h = http(port, None, "/api/health", "", Duration::from_millis(800)).ok()?;
        (h["ok"] == true).then_some((port, token))
    }

    fn backend(&mut self) -> Result<Backend> {
        if let Some((port, token)) = self.app() {
            return Ok(Backend::App { port, token });
        }
        if self.mode == "attach" {
            bail!("The Flux app isn't running. Open Flux, or start flux-mcp without --attach to work headless.");
        }
        if self.headless.is_none() {
            self.headless = Some(Api::open_default(true)?);
        }
        Ok(Backend::Headless(self.headless.clone().unwrap()))
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, (String, String)> {
        let b = self
            .backend()
            .map_err(|e| ("unavailable".to_string(), format!("{e:#}")))?;
        match b {
            Backend::Headless(api) => api
                .call(method, params)
                .map_err(|e| (error_code(&e).to_string(), format!("{e:#}"))),
            Backend::App { port, token } => {
                let body = json!({"method": method, "params": params}).to_string();
                let v = http(
                    port,
                    token.as_deref(),
                    "/api/call",
                    &body,
                    Duration::from_secs(3 * 3600),
                )
                .map_err(|e| {
                    (
                        "unavailable".to_string(),
                        format!("Lost the connection to Flux: {e:#}"),
                    )
                })?;
                if v["ok"] == true {
                    Ok(v["result"].clone())
                } else {
                    Err((
                        v["error"]["code"].as_str().unwrap_or("failed").to_string(),
                        v["error"]["message"]
                            .as_str()
                            .unwrap_or("failed")
                            .to_string(),
                    ))
                }
            }
        }
    }

    fn status(&mut self) -> Value {
        match self.app() {
            Some((port, _)) => {
                json!({"mode": "attached", "detail": format!("Driving the running Flux app on 127.0.0.1:{port}")})
            }
            None if self.mode == "attach" => {
                json!({"mode": "waiting", "detail": "The Flux app isn't running"})
            }
            None => {
                json!({"mode": "headless", "detail": "Flux engine running inside flux-mcp (no app window)", "data_dir": flux_core::store::data_dir().display().to_string()})
            }
        }
    }
}

/// Minimal HTTP/1.1 POST to 127.0.0.1 (no client dependency).
fn http(
    port: u16,
    token: Option<&str>,
    path: &str,
    body: &str,
    timeout: Duration,
) -> Result<Value> {
    let mut s =
        TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(800))?;
    s.set_read_timeout(Some(timeout))?;
    let auth = token
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    let verb = if path == "/api/health" { "GET" } else { "POST" };
    write!(s, "{verb} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())?;
    let mut buf = vec![];
    s.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let (head, rest) = text.split_once("\r\n\r\n").context("bad response")?;
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    if status == 401 {
        bail!("Flux refused the token in control.json. Restart the app.");
    }
    serde_json::from_str(rest).with_context(|| format!("HTTP {status}"))
}

fn tool_name(m: &str) -> String {
    m.replace('.', "_")
}

fn tools_list() -> Value {
    let mut tools: Vec<Value> = flux_core::methods::METHODS
        .iter()
        .map(|m| {
            json!({
                "name": tool_name(m.name),
                "title": m.name,
                "description": m.desc,
                "inputSchema": flux_core::methods::schema(m),
                "annotations": {"readOnlyHint": !m.records && !m.name.ends_with(".set") && !m.name.contains("start"), "openWorldHint": false},
            })
        })
        .collect();
    tools.push(json!({"name": "mcp_status", "title": "mcp.status", "description": "Whether flux-mcp is driving the open Flux app or running headless.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}}));
    json!({ "tools": tools })
}

fn handle(r: &mut Router, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg["method"].as_str().unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result: Result<Value, (i64, String)> = match method {
        "initialize" => {
            let v = params["protocolVersion"].as_str().unwrap_or(PROTOCOL);
            let v = if ["2024-11-05", "2025-03-26", "2025-06-18"].contains(&v) {
                v
            } else {
                PROTOCOL
            };
            Ok(json!({
                "protocolVersion": v,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "flux", "title": "Flux", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list()),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            if name == "mcp_status" {
                Ok(text_result(&r.status(), false))
            } else if flux_core::methods::find(name).is_none() {
                Err((-32602, format!("Unknown tool {name}")))
            } else {
                let m = flux_core::methods::find(name).unwrap().name;
                match r.call(m, args) {
                    Ok(v) => Ok(text_result(&v, false)),
                    Err((code, msg)) => {
                        Ok(text_result(&json!({"error": code, "message": msg}), true))
                    }
                }
            }
        }
        "resources/list" => Ok(json!({"resources": []})),
        "prompts/list" => Ok(json!({"prompts": []})),
        _ if id.is_none() => return None, // notifications (initialized, cancelled, ...)
        _ => Err((-32601, format!("Method not found: {method}"))),
    };
    let id = id?;
    Some(match result {
        Ok(v) => json!({"jsonrpc": "2.0", "id": id, "result": v}),
        Err((code, m)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}})
        }
    })
}

fn text_result(v: &Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(v).unwrap_or_default();
    let mut o = json!({"content": [{"type": "text", "text": text}], "isError": is_error});
    if !is_error && v.is_object() {
        o["structuredContent"] = v.clone();
    }
    o
}

fn stdio(mode: &str) -> Result<()> {
    let mut r = Router::new(mode);
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                writeln!(
                    out,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}})
                )?;
                out.flush()?;
                continue;
            }
        };
        let replies: Vec<Value> = match &msg {
            Value::Array(batch) => batch.iter().filter_map(|m| handle(&mut r, m)).collect(),
            m => handle(&mut r, m).into_iter().collect(),
        };
        for rep in replies {
            writeln!(out, "{rep}")?;
        }
        out.flush()?;
    }
    Ok(())
}

fn serve(args: &[String]) -> Result<()> {
    let mut port = 7341u16;
    let mut ui: Option<PathBuf> = None;
    let mut token = Some(flux_core::server::new_token());
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = args
                    .get(i)
                    .and_then(|x| x.parse().ok())
                    .context("--port needs a number")?;
            }
            "--ui" => {
                i += 1;
                ui = Some(PathBuf::from(args.get(i).context("--ui needs a folder")?));
            }
            "--no-token" => token = None,
            a => bail!("unknown option {a}"),
        }
        i += 1;
    }
    let api = Api::open_default(false)?;
    let s = flux_core::server::start(
        api,
        flux_core::server::ServerOpts {
            port,
            token: token.clone(),
            static_dir: ui.clone(),
            publish: true,
        },
    )?;
    let frag = token
        .as_ref()
        .map(|t| format!("#token={t}"))
        .unwrap_or_default();
    eprintln!(
        "Flux API on {}{}",
        s.url(),
        if ui.is_some() {
            format!("/{frag}")
        } else {
            String::new()
        }
    );
    if let Some(t) = &token {
        eprintln!("Token: {t}");
    }
    s.join();
    Ok(())
}

const HELP: &str = "flux-mcp — Flux as an MCP server

USAGE
  flux-mcp                 stdio MCP server (attaches to the running app, or runs headless)
  flux-mcp --headless      never attach; always run the engine inside flux-mcp
  flux-mcp --attach        only drive the running app; fail when it is closed
  flux-mcp serve [--port N] [--ui DIR] [--no-token]
                           HTTP API on 127.0.0.1 (POST /api/call {method, params})
  flux-mcp tools           print the tool list as JSON
  flux-mcp config          print an mcpServers config block for this binary

Data folder: FLUX_DATA_DIR or the system data folder /Flux.";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        None => stdio("auto"),
        Some("--headless") => stdio("headless"),
        Some("--attach") => stdio("attach"),
        Some("serve") => serve(&args[1..]),
        Some("tools") => {
            println!("{}", serde_json::to_string_pretty(&tools_list())?);
            Ok(())
        }
        Some("config") => {
            let exe = std::env::current_exe()?.display().to_string();
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"mcpServers": {"flux": {"command": exe, "args": []}}})
                )?
            );
            Ok(())
        }
        Some("--version") | Some("-V") => {
            println!("flux-mcp {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("--help") | Some("-h") => {
            println!("{HELP}");
            Ok(())
        }
        Some(a) => bail!("unknown argument {a}\n\n{HELP}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_round_trip_and_are_valid() {
        let t = tools_list();
        for x in t["tools"].as_array().unwrap() {
            let n = x["name"].as_str().unwrap();
            assert!(
                n.len() <= 64 && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{n}"
            );
            if n != "mcp_status" {
                assert!(flux_core::methods::find(n).is_some(), "{n}");
            }
        }
    }

    #[test]
    fn initialize_and_unknown_method() {
        let mut r = Router::new("headless");
        let v = handle(&mut r, &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}})).unwrap();
        assert_eq!(v["result"]["protocolVersion"], "2025-03-26");
        assert!(handle(
            &mut r,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .is_none());
        let v = handle(
            &mut r,
            &json!({"jsonrpc": "2.0", "id": 2, "method": "nope"}),
        )
        .unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }
}

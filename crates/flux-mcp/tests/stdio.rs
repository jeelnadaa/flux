//! Drive the real flux-mcp binary over stdio, headless and attached to a running server.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Mcp {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    id: u64,
}

impl Mcp {
    fn start(data: &Path, args: &[&str]) -> Mcp {
        let mut child = Command::new(env!("CARGO_BIN_EXE_flux-mcp"))
            .args(args)
            .env("FLUX_DATA_DIR", data)
            .env("FLUX_NULL_AUDIO", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut m = Mcp {
            child,
            stdin,
            out,
            id: 0,
        };
        let r = m.rpc("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}));
        assert_eq!(r["result"]["serverInfo"]["name"], "flux");
        writeln!(
            m.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .unwrap();
        m
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params})
        )
        .unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.out.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("bad line {line}"));
        assert_eq!(v["id"], self.id);
        v
    }
    fn tool(&mut self, name: &str, args: Value) -> (Value, bool) {
        let r = self.rpc("tools/call", json!({"name": name, "arguments": args}));
        let res = &r["result"];
        let text = res["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"));
        (serde_json::from_str(text).unwrap(), res["isError"] == true)
    }
    fn ok(&mut self, name: &str, args: Value) -> Value {
        let (v, e) = self.tool(name, args.clone());
        assert!(!e, "{name} {args}: {v}");
        v
    }
}
impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn tmp(n: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flux-mcp-{n}-{}-{}",
        std::process::id(),
        flux_core::store::new_id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn wav(p: &Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(p, spec).unwrap();
    for i in 0..44100 {
        w.write_sample(((i as f32 * 0.0627).sin() * 12000.0) as i16)
            .unwrap();
    }
    w.finalize().unwrap();
}

#[test]
fn headless_full_flow() {
    let d = tmp("headless");
    let mut m = Mcp::start(&d.join("data"), &["--headless"]);
    let tools = m.rpc("tools/list", json!({}));
    let n = tools["result"]["tools"].as_array().unwrap().len();
    assert!(n > 100, "{n} tools");
    let st = m.ok("mcp_status", json!({}));
    assert_eq!(st["mode"], "headless");
    let f = d.join("in.wav");
    wav(&f);
    m.ok(
        "media_import",
        json!({"path": f.display().to_string(), "wait": true}),
    );
    let p = m.ok("project_get", json!({}));
    assert_eq!(p["length"], 44100);
    m.ok("clip_split", json!({"at_s": 0.5}));
    let out = d.join("out.flac");
    let e = m.ok(
        "export_start",
        json!({"format": "flac", "path": out.display().to_string(), "wait": true}),
    );
    assert!(out.is_file(), "{e}");
    // errors come back as tool errors, not protocol errors
    let (v, is_err) = m.tool("clip_move", json!({"clip": 4242, "start": 0}));
    assert!(is_err, "{v}");
    let r = m.rpc(
        "tools/call",
        json!({"name": "no_such_tool", "arguments": {}}),
    );
    assert!(r.get("error").is_some());
}

#[test]
fn attaches_to_a_running_server() {
    let d = tmp("attach");
    let data = d.join("data");
    let mut srv = Command::new(env!("CARGO_BIN_EXE_flux-mcp"))
        .args(["serve", "--port", "0"])
        .env("FLUX_DATA_DIR", &data)
        .env("FLUX_NULL_AUDIO", "1")
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let ctl = data.join("control.json");
    for _ in 0..200 {
        if ctl.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(ctl.exists());
    let mut m = Mcp::start(&data, &[]);
    assert_eq!(m.ok("mcp_status", json!({}))["mode"], "attached");
    m.ok("project_new", json!({"name": "From agent"}));
    let mut m2 = Mcp::start(&data, &["--attach"]);
    // a second client sees the same open session
    assert_eq!(m2.ok("project_get", json!({}))["name"], "From agent");
    srv.kill().unwrap();
    let _ = srv.wait();
    let _ = std::fs::remove_file(&ctl);
    let (_, is_err) = m2.tool("project_get", json!({}));
    assert!(is_err, "attach mode fails once the app is gone");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Flux Native Audio Editor CLI & standalone executable.

use flux_engine::decoder::open_decoder;
use flux_engine::flux_dsp::biquad::{BiquadChain, BiquadType};
use flux_engine::flux_dsp::node::DspNode;
use flux_engine::render::EngineHost;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

fn print_banner() {
    println!(
        r#"
  ______ _     _    _ _  __
 |  ____| |   | |  | | |/ /
 | |__  | |   | |  | | ' / 
 |  __| | |   | |  | |  <  
 | |    | |___| |__| | . \ 
 |_|    |______\____/|_|\_\
 Flux Audio Editor v0.1.0 — Native Production Engine
 Non-destructive, 32-bit float internal, sample-accurate.
"#
    );
}

fn print_usage() {
    print_banner();
    println!("USAGE:");
    println!("    flux [OPTIONS] [FILE]");
    println!("    flux <COMMAND> [ARGS...]\n");
    println!("COMMANDS:");
    println!("    info <file>          Inspect audio file metadata and streams");
    println!("    bench                Run native DSP throughput benchmark");
    println!("    host                 Start native audio processing host");
    println!("    version              Display detailed engine and compiler info\n");
    println!("OPTIONS:");
    println!("    -h, --help           Print help information");
    println!("    -v, --version        Print version information");
}

fn cmd_info(path_str: &str) {
    let path = Path::new(path_str);
    if !path.exists() {
        eprintln!("Error: file '{path_str}' does not exist");
        process::exit(1);
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening file '{path_str}': {e}");
            process::exit(1);
        }
    };

    println!("[*] Inspecting: {}", path.display());
    match open_decoder(file) {
        Ok(decoder) => {
            let info = decoder.info();
            println!("  • Format:      {:?}", info.container_format);
            println!("  • Channels:    {}", info.channels);
            println!("  • Sample Rate: {} Hz", info.sample_rate);
            println!("  • Bit Depth:   {} bits", info.bits_per_sample);
            println!("  • Total Frames: {}", info.total_frames);
            println!(
                "  • Duration:    {:.3} s ({:.2} min)",
                info.duration_seconds,
                info.duration_seconds / 60.0
            );
            if let Some(t) = &info.metadata.title {
                println!("  • Title:       {t}");
            }
            if let Some(a) = &info.metadata.artist {
                println!("  • Artist:      {a}");
            }
            if let Some(al) = &info.metadata.album {
                println!("  • Album:       {al}");
            }
            println!("\n[✓] Valid audio stream decoded successfully.");
        }
        Err(e) => {
            eprintln!("Error decoding '{path_str}': {e}");
            process::exit(1);
        }
    }
}

fn cmd_bench() {
    println!("[*] Benchmarking Flux native DSP engine...");
    let sample_rate = 48000.0f32;
    let frames = 2_000_000;
    let input = vec![0.5f32; frames];
    let mut output = vec![0.0f32; frames];

    let start = Instant::now();
    let mut chain = BiquadChain::new(4);
    chain.prepare(sample_rate, 1);
    chain.configure_stage(0, BiquadType::Lowpass, 1200.0, 0.707, 0.0);
    chain.configure_stage(1, BiquadType::Peaking, 3500.0, 1.414, 3.0);
    chain.configure_stage(2, BiquadType::HighShelf, 8000.0, 0.707, 2.0);
    chain.configure_stage(3, BiquadType::Highpass, 30.0, 0.707, 0.0);

    let in_slices = [&input[..]];
    let mut out_slices = [&mut output[..]];
    chain.process(&in_slices, &mut out_slices);

    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let mframes_per_sec = (frames as f64 / 1_000_000.0) / secs;
    let realtime_ratio = (frames as f64 / sample_rate as f64) / secs;

    println!(
        "  • Processed {} frames (4-stage biquad cascade) in {:.2?}",
        frames, elapsed
    );
    println!("  • DSP Throughput: {:.2} Mframes/s", mframes_per_sec);
    println!(
        "  • Realtime ratio: {:.1}x realtime at 48 kHz",
        realtime_ratio
    );
    println!("[✓] Benchmark completed.");
}

fn cmd_host() {
    println!("[*] Initializing Flux Engine Host at 48000 Hz, buffer 128...");
    let host = EngineHost::new(48000.0, 128);
    println!("  • Transport State: {:?}", host.transport.state());
    println!("  • Sample Rate:     {} Hz", host.transport.sample_rate());
    println!("[✓] Engine Host active and ready.");
}

fn find_web_root() -> PathBuf {
    if let Ok(dir) = env::var("FLUX_WEB_DIR") {
        let p = PathBuf::from(dir);
        if p.exists() {
            return p;
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            for rel in &[
                "apps/web",
                "resources/web",
                "web",
                "../apps/web",
                "../../apps/web",
                "../Resources/apps/web",
                "../Resources/web",
                "../Resources",
            ] {
                let candidate = parent.join(rel);
                if candidate.join("index.html").exists() {
                    return candidate;
                }
            }
        }
    }

    if let Ok(cwd) = env::current_dir() {
        for rel in &["apps/web", "resources/web", "web", "../apps/web"] {
            let candidate = cwd.join(rel);
            if candidate.join("index.html").exists() {
                return candidate;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        for sys_path in &[
            "/usr/local/share/flux/apps/web",
            "/usr/share/flux/apps/web",
            "/usr/local/share/flux",
            "/usr/share/flux",
        ] {
            let candidate = PathBuf::from(sys_path);
            if candidate.join("index.html").exists() {
                return candidate;
            }
        }
    }

    PathBuf::from("apps/web")
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "wasm" => "application/wasm",
        "json" | "webmanifest" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    }
}

fn serve_assets(server: tiny_http::Server, web_root: PathBuf) {
    for request in server.incoming_requests() {
        let raw_url = request.url().split('?').next().unwrap_or("/");
        let clean_path = if raw_url == "/" {
            "/index.html"
        } else {
            raw_url
        };

        let rel_path = clean_path.trim_start_matches('/');
        let target_file = web_root.join(rel_path);

        if target_file.is_file() {
            if let Ok(data) = std::fs::read(&target_file) {
                let mime = get_mime_type(&target_file);
                let response = tiny_http::Response::new(
                    tiny_http::StatusCode(200),
                    vec![
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
                            .unwrap(),
                        tiny_http::Header::from_bytes(
                            &b"Cross-Origin-Opener-Policy"[..],
                            &b"same-origin"[..],
                        )
                        .unwrap(),
                        tiny_http::Header::from_bytes(
                            &b"Cross-Origin-Embedder-Policy"[..],
                            &b"require-corp"[..],
                        )
                        .unwrap(),
                        tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..])
                            .unwrap(),
                    ],
                    std::io::Cursor::new(data),
                    None,
                    None,
                );
                let _ = request.respond(response);
                continue;
            }
        }

        let not_found = tiny_http::Response::new(
            tiny_http::StatusCode(404),
            vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap()],
            std::io::Cursor::new(b"Not Found".to_vec()),
            None,
            None,
        );
        let _ = request.respond(not_found);
    }
}

pub fn run_gui() {
    let web_root = find_web_root();
    let server = tiny_http::Server::http("127.0.0.1:0").expect("Failed to start loopback server");
    let port = server
        .server_addr()
        .to_ip()
        .expect("Failed to get port")
        .port();

    let root_clone = web_root.clone();
    std::thread::spawn(move || {
        serve_assets(server, root_clone);
    });

    let event_loop = tao::event_loop::EventLoop::new();
    let window = tao::window::WindowBuilder::new()
        .with_title("Flux Audio Editor")
        .with_inner_size(tao::dpi::LogicalSize::new(1440.0, 900.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("Failed to create native desktop window");

    let url = format!("http://127.0.0.1:{}/index.html", port);

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let _webview = wry::WebViewBuilder::new()
        .with_url(&url)
        .build(&window)
        .expect("Failed to initialize native webview");

    #[cfg(target_os = "linux")]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get GTK container");
        wry::WebViewBuilder::new()
            .with_url(&url)
            .build_gtk(vbox)
            .expect("Failed to initialize native webview")
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;
        if let tao::event::Event::WindowEvent {
            event: tao::event::WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = tao::event_loop::ControlFlow::Exit;
        }
    });
}

fn attach_console_if_cli() {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn AttachConsole(dwProcessId: u32) -> i32;
        }
        unsafe {
            AttachConsole(u32::MAX);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args[1] == "--gui" {
        run_gui();
        return;
    }

    attach_console_if_cli();

    match args[1].as_str() {
        "-h" | "--help" | "help" => {
            print_usage();
        }
        "-v" | "--version" | "version" => {
            println!("flux 0.1.0 (native)");
            println!("engine: flux-engine 0.1.0 (DSP 32-bit float, sample-accurate)");
        }
        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: flux info <audio-file>");
                process::exit(1);
            }
            cmd_info(&args[2]);
        }
        "bench" => {
            cmd_bench();
        }
        "host" => {
            cmd_host();
        }
        file_arg => {
            if Path::new(file_arg).exists() {
                cmd_info(file_arg);
            } else {
                eprintln!("Unknown command or file: '{}'", file_arg);
                print_usage();
                process::exit(1);
            }
        }
    }
}

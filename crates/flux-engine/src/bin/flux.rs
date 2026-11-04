#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Flux Native Audio Editor CLI & standalone executable.

use flux_dsp::loudness::LoudnessMeter;
use flux_engine::decoder::{open_decoder, AudioBuffer};
use flux_engine::export::{
    ExportJob, ExportOptions, ExportTargetFormat, FlacCompressionLevel, Mp3Bitrate, NormalizationMode,
    OpusApplication, VorbisQuality, WavBitsPerSample,
};
use flux_engine::flux_dsp::biquad::{BiquadChain, BiquadType};
use flux_engine::flux_dsp::node::DspNode;
use flux_engine::render::EngineHost;
use std::collections::HashMap;
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
                "dist-static",
                "apps/web",
                "resources/web",
                "web",
                "../dist-static",
                "../apps/web",
                "../../dist-static",
                "../../apps/web",
                "../Resources/apps/web",
                "../Resources/dist-static",
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

fn parse_query(url: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(pos) = url.find('?') {
        let q = &url[pos + 1..];
        for pair in q.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let Some(k) = parts.next() {
                let v = parts.next().unwrap_or("");
                map.insert(k.to_string(), v.to_string());
            }
        }
    }
    map
}

fn parse_export_format(fmt: &str) -> ExportTargetFormat {
    match fmt.to_lowercase().as_str() {
        "wav_16" => ExportTargetFormat::Wav(WavBitsPerSample::Bits16),
        "wav_24" => ExportTargetFormat::Wav(WavBitsPerSample::Bits24),
        "wav_32f" => ExportTargetFormat::Wav(WavBitsPerSample::Float32),
        "flac_16" => ExportTargetFormat::Flac(FlacCompressionLevel::Level5, 16),
        "flac_24" => ExportTargetFormat::Flac(FlacCompressionLevel::Level5, 24),
        "mp3_128" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr128),
        "mp3_192" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr192),
        "mp3_256" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr256),
        "mp3_320" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr320),
        "vorbis_q3" => ExportTargetFormat::Vorbis(VorbisQuality::Q3),
        "vorbis_q6" => ExportTargetFormat::Vorbis(VorbisQuality::Q6),
        "vorbis_q9" => ExportTargetFormat::Vorbis(VorbisQuality::Q9),
        "opus_64" => ExportTargetFormat::Opus(OpusApplication::Audio, 64),
        "opus_128" => ExportTargetFormat::Opus(OpusApplication::Audio, 128),
        "opus_192" => ExportTargetFormat::Opus(OpusApplication::Audio, 192),
        _ => ExportTargetFormat::Wav(WavBitsPerSample::Bits16),
    }
}

fn export_mime_type(fmt: ExportTargetFormat) -> &'static str {
    match fmt {
        ExportTargetFormat::Wav(_) => "audio/wav",
        ExportTargetFormat::Flac(_, _) => "audio/flac",
        ExportTargetFormat::Mp3(_) => "audio/mpeg",
        ExportTargetFormat::Vorbis(_) => "audio/ogg",
        ExportTargetFormat::Opus(_, _) => "audio/opus",
        ExportTargetFormat::Aac(_) => "audio/aac",
    }
}

fn api_headers(content_type: &str) -> Vec<tiny_http::Header> {
    vec![
        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap(),
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        tiny_http::Header::from_bytes(
            &b"Access-Control-Allow-Methods"[..],
            &b"GET, POST, OPTIONS"[..],
        )
        .unwrap(),
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap(),
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
        tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap(),
    ]
}

fn serve_assets(server: tiny_http::Server, web_root: PathBuf) {
    for mut request in server.incoming_requests() {
        let full_url = request.url().to_string();
        let path = full_url.split('?').next().unwrap_or("/").to_string();

        // 1. CORS preflight
        if request.method() == &tiny_http::Method::Options {
            let resp = tiny_http::Response::new(
                tiny_http::StatusCode(200),
                api_headers("text/plain"),
                std::io::Cursor::new(b"OK".to_vec()),
                None,
                None,
            );
            let _ = request.respond(resp);
            continue;
        }

        // 2. Route: GET /api/status
        if path == "/api/status" && request.method() == &tiny_http::Method::Get {
            let body = serde_json::json!({
                "status": "ok",
                "backend": "flux-native",
                "version": env!("CARGO_PKG_VERSION"),
                "sampleRate": 48000,
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "simd": true,
            });
            let data = serde_json::to_vec(&body).unwrap();
            let resp = tiny_http::Response::new(
                tiny_http::StatusCode(200),
                api_headers("application/json; charset=utf-8"),
                std::io::Cursor::new(data),
                None,
                None,
            );
            let _ = request.respond(resp);
            continue;
        }

        // 3. Route: POST /api/decode
        if path == "/api/decode" && request.method() == &tiny_http::Method::Post {
            let mut audio_bytes = Vec::new();
            if let Err(e) = request.as_reader().read_to_end(&mut audio_bytes) {
                let resp = tiny_http::Response::new(
                    tiny_http::StatusCode(400),
                    api_headers("application/json"),
                    std::io::Cursor::new(
                        format!(r#"{{"error": "Failed to read request body: {}"}}"#, e).into_bytes(),
                    ),
                    None,
                    None,
                );
                let _ = request.respond(resp);
                continue;
            }

            let cursor = std::io::Cursor::new(audio_bytes);
            match open_decoder(cursor) {
                Ok(mut decoder) => match decoder.decode_all() {
                    Ok(buf) => {
                        let num_channels = buf.num_channels();
                        let num_frames = buf.num_frames();
                        let sample_rate = buf.sample_rate;

                        // Binary serialization:
                        // [0..4] sample_rate (u32 LE)
                        // [4..8] num_channels (u32 LE)
                        // [8..16] num_frames (u64 LE)
                        // [16..] ch0 f32s, ch1 f32s...
                        let mut out = Vec::with_capacity(16 + num_channels * num_frames * 4);
                        out.extend_from_slice(&(sample_rate as u32).to_le_bytes());
                        out.extend_from_slice(&(num_channels as u32).to_le_bytes());
                        out.extend_from_slice(&(num_frames as u64).to_le_bytes());
                        for ch in 0..num_channels {
                            for &s in buf.channel(ch) {
                                out.extend_from_slice(&s.to_le_bytes());
                            }
                        }

                        let resp = tiny_http::Response::new(
                            tiny_http::StatusCode(200),
                            api_headers("application/octet-stream"),
                            std::io::Cursor::new(out),
                            None,
                            None,
                        );
                        let _ = request.respond(resp);
                        continue;
                    }
                    Err(e) => {
                        let resp = tiny_http::Response::new(
                            tiny_http::StatusCode(422),
                            api_headers("application/json"),
                            std::io::Cursor::new(
                                format!(r#"{{"error": "Decode stream error: {}"}}"#, e).into_bytes(),
                            ),
                            None,
                            None,
                        );
                        let _ = request.respond(resp);
                        continue;
                    }
                },
                Err(e) => {
                    let resp = tiny_http::Response::new(
                        tiny_http::StatusCode(400),
                        api_headers("application/json"),
                        std::io::Cursor::new(
                            format!(r#"{{"error": "Unsupported container or format: {}"}}"#, e).into_bytes(),
                        ),
                        None,
                        None,
                    );
                    let _ = request.respond(resp);
                    continue;
                }
            }
        }

        // 4. Route: POST /api/export
        if path == "/api/export" && request.method() == &tiny_http::Method::Post {
            let q = parse_query(&full_url);
            let fmt_str = q.get("format").map(|s| s.as_str()).unwrap_or("wav_16");
            let sr = q
                .get("sampleRate")
                .or_else(|| q.get("sr"))
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(48000);
            let ch = q
                .get("channels")
                .or_else(|| q.get("ch"))
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(2);
            let dither_str = q.get("dither").map(|s| s.as_str()).unwrap_or("none");
            let norm_str = q.get("norm").map(|s| s.as_str()).unwrap_or("off");

            let mut raw_bytes = Vec::new();
            if let Err(e) = request.as_reader().read_to_end(&mut raw_bytes) {
                let resp = tiny_http::Response::new(
                    tiny_http::StatusCode(400),
                    api_headers("application/json"),
                    std::io::Cursor::new(
                        format!(r#"{{"error": "Failed to read body: {}"}}"#, e).into_bytes(),
                    ),
                    None,
                    None,
                );
                let _ = request.respond(resp);
                continue;
            }

            let float_len = raw_bytes.len() / 4;
            let mut samples = Vec::with_capacity(float_len);
            for chunk in raw_bytes.chunks_exact(4) {
                samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }

            let buffer = AudioBuffer::from_interleaved(&samples, ch, sr);
            let target_format = parse_export_format(fmt_str);
            let mime = export_mime_type(target_format);

            let mut opts = ExportOptions::default();
            opts.format = target_format;
            opts.sample_rate = Some(sr);
            opts.dither = match dither_str {
                "tpdf" => flux_dsp::dither::DitherType::Tpdf,
                "noise_shaped" => flux_dsp::dither::DitherType::NoiseShaped,
                _ => flux_dsp::dither::DitherType::None,
            };
            if norm_str.starts_with("peak") {
                opts.normalization = NormalizationMode::Peak { target_dbfs: -0.3 };
            } else if norm_str.starts_with("lufs") {
                opts.normalization = NormalizationMode::Lufs {
                    target_lufs: -14.0,
                    true_peak_ceiling_dbtp: -1.0,
                };
            }

            match ExportJob::run(&buffer, &opts, None, |_| {}) {
                Ok(encoded_bytes) => {
                    let mut headers = api_headers(mime);
                    headers.push(
                        tiny_http::Header::from_bytes(
                            &b"Content-Disposition"[..],
                            &b"attachment; filename=\"export\""[..],
                        )
                        .unwrap(),
                    );
                    let resp = tiny_http::Response::new(
                        tiny_http::StatusCode(200),
                        headers,
                        std::io::Cursor::new(encoded_bytes),
                        None,
                        None,
                    );
                    let _ = request.respond(resp);
                    continue;
                }
                Err(e) => {
                    let resp = tiny_http::Response::new(
                        tiny_http::StatusCode(500),
                        api_headers("application/json"),
                        std::io::Cursor::new(
                            format!(r#"{{"error": "Export failed: {:?}"}}"#, e).into_bytes(),
                        ),
                        None,
                        None,
                    );
                    let _ = request.respond(resp);
                    continue;
                }
            }
        }

        // 5. Route: POST /api/dsp/loudness
        if path == "/api/dsp/loudness" && request.method() == &tiny_http::Method::Post {
            let q = parse_query(&full_url);
            let sr = q
                .get("sampleRate")
                .or_else(|| q.get("sr"))
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(48000);
            let ch = q
                .get("channels")
                .or_else(|| q.get("ch"))
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(2);

            let mut raw_bytes = Vec::new();
            let _ = request.as_reader().read_to_end(&mut raw_bytes);
            let float_len = raw_bytes.len() / 4;
            let mut samples = Vec::with_capacity(float_len);
            for chunk in raw_bytes.chunks_exact(4) {
                samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }

            let buffer = AudioBuffer::from_interleaved(&samples, ch, sr);
            let ch_refs: Vec<&[f32]> = (0..buffer.num_channels())
                .map(|c| buffer.channel(c))
                .collect();
            let m = LoudnessMeter::analyze(&ch_refs, sr);

            let result = serde_json::json!({
                "integratedLufs": m.integrated_lufs,
                "momentaryMaxLufs": m.max_momentary_lufs,
                "shortTermMaxLufs": m.max_short_term_lufs,
                "loudnessRangeLu": m.loudness_range_lu,
                "maxTruePeakDbtp": m.max_true_peak_dbtp,
            });

            let resp = tiny_http::Response::new(
                tiny_http::StatusCode(200),
                api_headers("application/json"),
                std::io::Cursor::new(serde_json::to_vec(&result).unwrap()),
                None,
                None,
            );
            let _ = request.respond(resp);
            continue;
        }

        // 6. Route: /flux_wasm.wasm
        if path == "/flux_wasm.wasm" {
            let candidates = [
                web_root.join("flux_wasm.wasm"),
                PathBuf::from("target/wasm32-unknown-unknown/release/flux_wasm.wasm"),
                PathBuf::from("../target/wasm32-unknown-unknown/release/flux_wasm.wasm"),
                PathBuf::from("../../target/wasm32-unknown-unknown/release/flux_wasm.wasm"),
                PathBuf::from("dist-static/flux_wasm.wasm"),
            ];
            let mut wasm_data = None;
            for cand in &candidates {
                if cand.is_file() {
                    if let Ok(data) = std::fs::read(cand) {
                        wasm_data = Some(data);
                        break;
                    }
                }
            }
            if let Some(data) = wasm_data {
                let resp = tiny_http::Response::new(
                    tiny_http::StatusCode(200),
                    api_headers("application/wasm"),
                    std::io::Cursor::new(data),
                    None,
                    None,
                );
                let _ = request.respond(resp);
                continue;
            }
        }

        // 7. Static file delivery
        let clean_path = if path == "/" {
            "/index.html"
        } else {
            &path
        };

        let rel_path = clean_path.trim_start_matches('/');
        let target_file = web_root.join(rel_path);

        if target_file.is_file() {
            if let Ok(data) = std::fs::read(&target_file) {
                let mime = get_mime_type(&target_file);
                let response = tiny_http::Response::new(
                    tiny_http::StatusCode(200),
                    api_headers(mime),
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
            api_headers("text/plain"),
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

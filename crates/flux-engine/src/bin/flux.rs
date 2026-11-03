//! Flux Native Audio Editor CLI & standalone executable.

use flux_engine::decoder::open_decoder;
use flux_engine::flux_dsp::biquad::{BiquadChain, BiquadType};
use flux_engine::flux_dsp::node::DspNode;
use flux_engine::render::EngineHost;
use std::env;
use std::fs::File;
use std::path::Path;
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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        println!("\n[i] Flux Native Audio Editor engine initialized.");
        return;
    }

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

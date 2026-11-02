"""CLI runner for Flux objective quality evaluation harness."""

import argparse
import sys
from pathlib import Path

# Ensure repository root is on sys.path
repo_root = Path(__file__).resolve().parent.parent.parent
if str(repo_root) not in sys.path:
    sys.path.insert(0, str(repo_root))

import soundfile as sf
import yaml

from tools.eval.metrics import (
    beat_accuracy,
    cer,
    dnsmos,
    log_spectral_distance,
    lufs,
    mirex_key_score,
    null_residual,
    pesq,
    si_sdr,
    snr,
    stoi,
    thd_n,
    true_peak,
    visqol,
    wer,
)
from tools.eval.reporter import EvaluationReport


def run_synthetic_eval(repo_root: Path, report: EvaluationReport):
    """Run baseline evaluations across all generated synthetic fixtures."""
    synthetic_dir = repo_root / "corpus" / "synthetic"
    if not synthetic_dir.exists():
        print(f"Warning: synthetic dir {synthetic_dir} does not exist", file=sys.stderr)
        return

    # 1. Evaluate 1 kHz 0 dBFS sine
    sine_file = synthetic_dir / "syn_sine_1k_0dbfs_100ms.wav"
    if sine_file.exists():
        data, sr = sf.read(str(sine_file))
        thd_val = thd_n(data, sr, 1000.0)
        report.add_result(
            category="DSP / Audio",
            fixture_id="syn_sine_1k_0dbfs_100ms",
            metric="THD+N",
            value=thd_val,
            target=-96.0,
            unit="dBFS",
            higher_is_better=False,
            notes="16-bit PCM quantization floor target <= -96 dBFS",
        )
        tp_val = true_peak(data)
        report.add_result(
            category="DSP / Audio",
            fixture_id="syn_sine_1k_0dbfs_100ms",
            metric="True Peak",
            value=tp_val,
            target=0.1,
            unit="dBTP",
            higher_is_better=False,
            notes="True peak ceiling target",
        )
        lufs_val = lufs(data, sr)
        report.add_result(
            category="DSP / Audio",
            fixture_id="syn_sine_1k_0dbfs_100ms",
            metric="LUFS",
            value=lufs_val,
            unit="LUFS",
            notes="Integrated loudness",
        )

    # 2. Evaluate Silence
    silence_file = synthetic_dir / "syn_silence_100ms.wav"
    if silence_file.exists():
        data, sr = sf.read(str(silence_file))
        null_res = null_residual(data, data)
        report.add_result(
            category="DSP / Audio",
            fixture_id="syn_silence_100ms",
            metric="Null Residual RMS",
            value=null_res["rms_error_db"],
            target=-120.0,
            unit="dBFS",
            higher_is_better=False,
            notes="Silence in / silence out",
        )

    # 3. Evaluate Hot Float
    hot_file = synthetic_dir / "syn_hot_float_plusminus4_50ms.wav"
    if hot_file.exists():
        data, sr = sf.read(str(hot_file))
        import numpy as np
        max_abs = float(np.max(np.abs(data)))
        report.add_result(
            category="DSP / Hot Float",
            fixture_id="syn_hot_float_plusminus4_50ms",
            metric="Peak Amplitude",
            value=max_abs,
            target=4.0,
            unit="FS",
            higher_is_better=True,
            notes="Hot float +/-4.0 representation preservation",
        )

    # 4. Evaluate Speech Metrics Benchmark
    ref_text = "the quick brown fox jumps over the lazy dog"
    hyp_text = "the quick brown fox jumped over a lazy dog"
    w_err = wer(ref_text, hyp_text)
    report.add_result(
        category="Speech / ASR",
        fixture_id="synthetic_transcript_pair_01",
        metric="WER",
        value=w_err * 100.0,
        target=25.0,
        unit="%",
        higher_is_better=False,
        notes="Levenshtein word error rate benchmark",
    )
    c_err = cer(ref_text, hyp_text)
    report.add_result(
        category="Speech / ASR",
        fixture_id="synthetic_transcript_pair_01",
        metric="CER",
        value=c_err * 100.0,
        target=15.0,
        unit="%",
        higher_is_better=False,
        notes="Character error rate benchmark",
    )

    # 5. Evaluate Separation & Restoration Metrics on broadband fixture
    chirp_file = synthetic_dir / "syn_chirp_lin_20_20k_200ms.wav"
    if chirp_file.exists():
        data, sr = sf.read(str(chirp_file))
        import numpy as np
        noise = np.random.RandomState(42).randn(len(data)) * 0.001
        degraded = data + noise

        s_sdr = si_sdr(data, degraded)
        report.add_result(
            category="Restoration / Stems",
            fixture_id="syn_chirp_with_noise_snr60",
            metric="SI-SDR",
            value=s_sdr,
            target=50.0,
            unit="dB",
            higher_is_better=True,
            notes="Scale-invariant separation ratio benchmark (target >= 50 dB)",
        )

        lsd_val = log_spectral_distance(data, degraded, sr)
        report.add_result(
            category="Restoration / Stems",
            fixture_id="syn_chirp_with_noise_snr60",
            metric="LSD",
            value=lsd_val,
            target=2.0,
            unit="dB",
            higher_is_better=False,
            notes="Log-spectral distance distortion benchmark (target <= 2 dB)",
        )

        mos_val = visqol(data, degraded, sr)
        report.add_result(
            category="Perceptual Quality",
            fixture_id="syn_chirp_with_noise_snr60",
            metric="ViSQOL MOS-LQO",
            value=mos_val,
            target=4.0,
            unit="MOS",
            higher_is_better=True,
            notes="Perceptual quality objective estimator (target >= 4.0 MOS)",
        )

    # 6. Evaluate MIR Metrics
    k_score = mirex_key_score("C major", "A minor")
    report.add_result(
        category="MIR / Key",
        fixture_id="c_major_vs_a_minor_relative",
        metric="MIREX Key Score",
        value=k_score,
        target=0.3,
        unit="score",
        higher_is_better=True,
        notes="Relative major/minor credit = 0.3",
    )

    ref_beats = [0.5, 1.0, 1.5, 2.0, 2.5]
    est_beats = [0.51, 0.99, 1.52, 1.98, 2.51]
    b_acc = beat_accuracy(ref_beats, est_beats)
    report.add_result(
        category="MIR / Beat",
        fixture_id="synthetic_isochronous_beats_120bpm",
        metric="Beat F-Measure",
        value=b_acc["f_measure"],
        target=0.85,
        unit="score",
        higher_is_better=True,
        notes="Beat tracking accuracy target >= 0.85",
    )


def main():
    parser = argparse.ArgumentParser(description="Flux Objective Quality Evaluation Harness")
    parser.add_argument(
        "--manifest",
        type=str,
        default="corpus/corpus.yaml",
        help="Path to corpus manifest YAML",
    )
    parser.add_argument(
        "--out-json",
        type=str,
        default="tools/eval/reports/eval_latest.json",
        help="Output path for JSON report",
    )
    parser.add_argument(
        "--out-csv",
        type=str,
        default="tools/eval/reports/eval_latest.csv",
        help="Output path for CSV report",
    )
    parser.add_argument(
        "--out-dashboard",
        type=str,
        default="tools/eval/reports/DASHBOARD.md",
        help="Output path for Markdown dashboard",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent.parent
    report = EvaluationReport()

    print(f"Running Flux evaluation harness (commit: {report.commit})...")
    run_synthetic_eval(repo_root, report)

    report.to_json(repo_root / args.out_json)
    report.to_csv(repo_root / args.out_csv)
    report.save_dashboard(repo_root / args.out_dashboard)

    print(f"\nEvaluation complete: {len(report.records)} metrics recorded.")
    print(f"  Passed: {report.to_dict()['passed_count']}")
    print(f"  Failed: {report.to_dict()['failed_count']}")
    print(f"  Reports written to: {args.out_json}, {args.out_csv}, {args.out_dashboard}")


if __name__ == "__main__":
    main()

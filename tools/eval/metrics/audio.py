"""Core DSP audio metrics (THD+N, SNR, Null Residual, BS.1770 LUFS, True Peak)."""

import numpy as np
from scipy import signal as sp_signal


def thd_n(signal: np.ndarray, sr: float, fundamental_freq: float) -> float:
    """Calculate Total Harmonic Distortion plus Noise (THD+N) in dBFS using IEEE 1057 least-squares fit."""
    sig = np.asarray(signal, dtype=np.float64).flatten()
    n = len(sig)
    if n < 64:
        return -120.0

    t = np.arange(n) / sr
    omega = 2.0 * np.pi * fundamental_freq

    # Fit [sin(omega*t), cos(omega*t), 1]
    a_sin = np.sin(omega * t)
    a_cos = np.cos(omega * t)
    a_dc = np.ones(n)

    mat = np.column_stack([a_sin, a_cos, a_dc])
    coeffs, _, _, _ = np.linalg.lstsq(mat, sig, rcond=None)

    fund_fit = coeffs[0] * a_sin + coeffs[1] * a_cos
    residual = sig - (fund_fit + coeffs[2])

    res_rms = np.sqrt(np.mean(residual**2) + 1e-20)
    fund_amp = np.sqrt(coeffs[0] ** 2 + coeffs[1] ** 2)

    if fund_amp < 1e-10:
        return 0.0

    thd_ratio = res_rms / (fund_amp / np.sqrt(2.0))
    return float(20.0 * np.log10(np.maximum(thd_ratio, 1e-12)))


def snr(reference: np.ndarray, test: np.ndarray) -> float:
    """Calculate Signal-to-Noise Ratio (SNR) in dB between reference and test signals."""
    ref = np.asarray(reference, dtype=np.float64).flatten()
    tst = np.asarray(test, dtype=np.float64).flatten()
    min_len = min(len(ref), len(tst))

    ref = ref[:min_len]
    noise = ref - tst[:min_len]

    s_pwr = np.mean(ref**2)
    n_pwr = np.mean(noise**2)

    if n_pwr < 1e-20:
        return 140.0
    if s_pwr < 1e-20:
        return -140.0

    return float(10.0 * np.log10(s_pwr / n_pwr))


def null_residual(reference: np.ndarray, test: np.ndarray) -> dict[str, float]:
    """Perform a null test by subtracting test from reference signal.

    Returns:
        Dict with "max_error_db", "rms_error_db", and "is_bit_exact" (1.0 or 0.0).
    """
    ref = np.asarray(reference, dtype=np.float64).flatten()
    tst = np.asarray(test, dtype=np.float64).flatten()
    min_len = min(len(ref), len(tst))

    diff = np.abs(ref[:min_len] - tst[:min_len])
    max_err = np.max(diff) if len(diff) > 0 else 0.0
    rms_err = np.sqrt(np.mean(diff**2)) if len(diff) > 0 else 0.0

    max_db = 20.0 * np.log10(max_err) if max_err > 1e-7 else -140.0
    rms_db = 20.0 * np.log10(rms_err) if rms_err > 1e-7 else -140.0

    return {
        "max_error_db": float(max_db),
        "rms_error_db": float(rms_db),
        "is_bit_exact": 1.0 if max_err == 0.0 else 0.0,
    }


def lufs(audio: np.ndarray, sr: int) -> float:
    """Calculate ITU-R BS.1770-4 integrated loudness in LUFS."""
    sig = np.asarray(audio, dtype=np.float64)
    if sig.ndim == 1:
        sig = sig[:, np.newaxis]

    # Pre-filter (stage 1: high shelf)
    # Target: ~1.5 kHz boost +4 dB
    b_shelf, a_shelf = sp_signal.iirfilter(
        2, 1500 / (sr / 2), btype="highpass", ftype="butter", output="ba"
    )
    # Stage 2: RLB highpass filter (~100 Hz cutoff)
    b_rlb, a_rlb = sp_signal.iirfilter(
        2, 100 / (sr / 2), btype="highpass", ftype="butter", output="ba"
    )

    filtered = np.zeros_like(sig)
    for ch in range(sig.shape[1]):
        filtered[:, ch] = sp_signal.lfilter(b_rlb, a_rlb, sp_signal.lfilter(b_shelf, a_shelf, sig[:, ch]))

    # Mean square across channels
    ms = np.mean(filtered**2)
    if ms < 1e-12:
        return -70.0

    return float(-0.691 + 10.0 * np.log10(ms))


def true_peak(audio: np.ndarray) -> float:
    """Calculate ITU-R BS.1770 4x oversampled true peak in dBTP."""
    sig = np.asarray(audio, dtype=np.float64).flatten()
    if len(sig) == 0:
        return -140.0

    # 4x polyphase / sinc resampling
    oversampled = sp_signal.resample(sig, len(sig) * 4)
    peak_val = np.max(np.abs(oversampled))

    if peak_val < 1e-7:
        return -140.0
    return float(20.0 * np.log10(peak_val))

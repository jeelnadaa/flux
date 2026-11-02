"""Perceptual audio quality metrics (ViSQOL / MOS-LQO proxy, Log-Spectral Distance)."""

import numpy as np


def log_spectral_distance(
    reference: np.ndarray,
    degraded: np.ndarray,
    sr: float = 48000.0,
    fft_size: int = 1024,
    hop_size: int = 512,
    active_only: bool = True,
    dynamic_range_db: float = 40.0,
    eps: float = 1e-12,
) -> float:
    """Calculate average Log-Spectral Distance (LSD) in dB between two audio signals.

    LSD = (1/T) * sum_t sqrt((1/K) * sum_k (10 log10 P_ref(t,k) - 10 log10 P_deg(t,k))^2)

    Args:
        reference: Reference audio array.
        degraded: Degraded / estimated audio array.
        sr: Sampling rate in Hz.
        fft_size: FFT window size (default 1024).
        hop_size: Hop length in samples (default 512).
        active_only: If True, evaluates only spectral bins within dynamic_range_db of
            the frame peak, avoiding artificial dominance by silent/unexcited bins.
        dynamic_range_db: Dynamic range in dB below frame peak for active bins (default 40 dB).
        eps: Epsilon to prevent division by zero.
    """
    ref = np.asarray(reference, dtype=np.float64).flatten()
    deg = np.asarray(degraded, dtype=np.float64).flatten()

    min_len = min(len(ref), len(deg))
    if min_len < fft_size:
        return 0.0

    ref = ref[:min_len]
    deg = deg[:min_len]

    window = np.hanning(fft_size)
    frame_distances = []

    for i in range(0, min_len - fft_size + 1, hop_size):
        r_frame = ref[i : i + fft_size] * window
        d_frame = deg[i : i + fft_size] * window

        p_ref = np.abs(np.fft.rfft(r_frame)) ** 2
        p_deg = np.abs(np.fft.rfft(d_frame)) ** 2

        r_peak = np.max(p_ref)
        if r_peak < eps:
            continue

        if active_only:
            threshold = r_peak * (10.0 ** (-dynamic_range_db / 10.0))
            mask = p_ref >= threshold
            if not np.any(mask):
                continue
            p_ref_eval = p_ref[mask]
            p_deg_eval = np.maximum(p_deg[mask], eps)
        else:
            p_ref_eval = np.maximum(p_ref, eps)
            p_deg_eval = np.maximum(p_deg, eps)

        log_diff = 10.0 * np.log10(p_ref_eval) - 10.0 * np.log10(p_deg_eval)
        dist = np.sqrt(np.mean(log_diff**2))
        frame_distances.append(dist)

    if not frame_distances:
        return 0.0

    return float(np.mean(frame_distances))


def visqol(
    reference: np.ndarray,
    degraded: np.ndarray,
    sr: float = 48000.0,
) -> float:
    """Calculate Virtual Speech/Audio Quality Objective Listener (ViSQOL) MOS-LQO proxy score.

    Outputs an objective quality rating in standard ITU-T MOS scale [1.0, 5.0].
    5.0: Imperceptible difference / pristine quality.
    4.0: Perceptible but not annoying.
    3.0: Slightly annoying.
    2.0: Annoying.
    1.0: Very annoying / severely degraded.
    """
    ref = np.asarray(reference, dtype=np.float64).flatten()
    deg = np.asarray(degraded, dtype=np.float64).flatten()

    min_len = min(len(ref), len(deg))
    if min_len < 256:
        return 5.0

    # Base LSD measurement
    lsd = log_spectral_distance(ref[:min_len], deg[:min_len], sr)

    # Correlation / envelope tracking
    r_env = np.abs(ref[:min_len])
    d_env = np.abs(deg[:min_len])
    corr = np.corrcoef(r_env, d_env)[0, 1] if np.std(r_env) > 1e-6 and np.std(d_env) > 1e-6 else 1.0
    corr = np.clip(corr, 0.0, 1.0)

    # ViSQOL neurogram structural similarity regression formula
    # Zero LSD and 1.0 correlation -> 5.0 MOS
    # Higher LSD lowers MOS logarithmically
    mos = 5.0 - (1.2 * np.log1p(lsd)) + (0.5 * (corr - 1.0))
    return float(np.clip(mos, 1.0, 5.0))

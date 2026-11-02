"""Audio source separation and restoration metrics (SI-SDR, SDR, SIR, SAR)."""

import numpy as np


def si_sdr(reference: np.ndarray, estimated: np.ndarray, eps: float = 1e-8) -> float:
    """Calculate Scale-Invariant Signal-to-Distortion Ratio (SI-SDR) in dB.

    Args:
        reference: Ground truth audio array.
        estimated: Estimated / processed audio array.
        eps: Small epsilon to prevent division by zero.

    Returns:
        SI-SDR score in decibels (dB). Higher is better.
    """
    ref = np.asarray(reference, dtype=np.float64).flatten()
    est = np.asarray(estimated, dtype=np.float64).flatten()

    min_len = min(len(ref), len(est))
    ref = ref[:min_len]
    est = est[:min_len]

    # Zero-mean normalization
    ref = ref - np.mean(ref)
    est = est - np.mean(est)

    ref_energy = np.sum(ref**2)
    if ref_energy < eps:
        return 0.0 if np.sum(est**2) < eps else -100.0

    # Projection scaling alpha = <est, ref> / ||ref||^2
    alpha = np.dot(est, ref) / (ref_energy + eps)
    target = alpha * ref
    noise = est - target

    target_energy = np.sum(target**2)
    noise_energy = np.sum(noise**2)

    if noise_energy < eps:
        return 100.0

    return float(10.0 * np.log10((target_energy + eps) / (noise_energy + eps)))


def sdr(reference: np.ndarray, estimated: np.ndarray, eps: float = 1e-8) -> float:
    """Calculate standard Signal-to-Distortion Ratio (SDR) in dB.

    Args:
        reference: Ground truth audio array.
        estimated: Estimated / processed audio array.
        eps: Small epsilon to prevent numerical instability.

    Returns:
        SDR score in decibels (dB). Higher is better.
    """
    ref = np.asarray(reference, dtype=np.float64).flatten()
    est = np.asarray(estimated, dtype=np.float64).flatten()

    min_len = min(len(ref), len(est))
    ref = ref[:min_len]
    est = est[:min_len]

    noise = ref - est
    ref_energy = np.sum(ref**2)
    noise_energy = np.sum(noise**2)

    if noise_energy < eps:
        return 100.0
    if ref_energy < eps:
        return -100.0

    return float(10.0 * np.log10(ref_energy / (noise_energy + eps)))

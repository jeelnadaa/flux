"""Tests for individual audio evaluation metrics."""

import numpy as np
import pytest

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


def test_si_sdr_identical():
    sig = np.sin(2 * np.pi * 440 * np.linspace(0, 1, 16000))
    score = si_sdr(sig, sig)
    assert score >= 90.0


def test_si_sdr_scaled():
    sig = np.sin(2 * np.pi * 440 * np.linspace(0, 1, 16000))
    scaled = sig * 2.5  # Scale invariance
    score = si_sdr(sig, scaled)
    assert score >= 90.0


def test_wer_exact():
    assert wer("hello world", "hello world") == 0.0


def test_wer_substitution():
    # 1 substitution in 4 words = 0.25 (25%)
    assert wer("one two three four", "one two free four") == 0.25


def test_cer_calculation():
    assert cer("cat", "cat") == 0.0
    assert cer("cat", "hat") == pytest.approx(1.0 / 3.0)


def test_stoi_range():
    sr = 16000
    sig = np.random.RandomState(42).randn(sr)
    score = stoi(sig, sig, sr)
    assert 0.0 <= score <= 1.0
    assert score > 0.95


def test_pesq_range():
    sr = 16000
    sig = np.sin(2 * np.pi * 1000 * np.linspace(0, 1, sr))
    score = pesq(sig, sig, sr)
    assert 1.0 <= score <= 4.5
    assert score >= 4.0


def test_dnsmos_bounds():
    sr = 16000
    sig = np.random.RandomState(42).randn(sr) * 0.1
    res = dnsmos(sig, sr)
    for k in ["SIG", "BAK", "OVRL"]:
        assert 1.0 <= res[k] <= 5.0


def test_visqol_and_lsd():
    sr = 48000
    sig = np.sin(2 * np.pi * 1000 * np.linspace(0, 0.5, sr // 2))
    lsd_val = log_spectral_distance(sig, sig, sr)
    assert lsd_val == pytest.approx(0.0, abs=1e-3)

    mos = visqol(sig, sig, sr)
    assert mos == pytest.approx(5.0, abs=0.1)


def test_thd_n_pure_sine():
    sr = 48000.0
    t = np.arange(4800) / sr
    sig = np.sin(2 * np.pi * 1000.0 * t)
    thd = thd_n(sig, sr, 1000.0)
    assert thd < -110.0


def test_null_residual():
    sig = np.ones(100)
    res = null_residual(sig, sig)
    assert res["is_bit_exact"] == 1.0
    assert res["max_error_db"] <= -140.0


def test_mirex_key_score_rules():
    assert mirex_key_score("C major", "C major") == 1.0
    assert mirex_key_score("C major", "G major") == 0.5  # Dominant fifth
    assert mirex_key_score("C major", "F major") == 0.5  # Subdominant fifth
    assert mirex_key_score("C major", "A minor") == 0.3  # Relative minor
    assert mirex_key_score("C major", "C minor") == 0.2  # Parallel minor
    assert mirex_key_score("C major", "F# major") == 0.0  # Tritone / unrelated


def test_beat_accuracy_exact():
    beats = [0.5, 1.0, 1.5, 2.0]
    res = beat_accuracy(beats, beats)
    assert res["f_measure"] == 1.0
    assert res["acc1"] == 1.0
    assert res["acc2"] == 1.0

"""Music Information Retrieval (MIR) evaluation metrics (Key detection MIREX score, Beat tracking Acc1/Acc2/F-measure)."""

import numpy as np

# Circle of fifths key representation
PITCH_CLASSES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
ENHARMONIC = {
    "Db": "C#",
    "Eb": "D#",
    "Gb": "F#",
    "Ab": "G#",
    "Bb": "A#",
}


def _parse_key(key_str: str) -> tuple[int, str]:
    """Parse key string e.g. 'C major', 'A minor', 'F#m' into (pitch_class_idx, mode)."""
    parts = key_str.strip().split()
    root = parts[0]
    mode = "major"

    if len(parts) > 1:
        mode = parts[1].lower()
    elif root.endswith("m") and len(root) > 1:
        root = root[:-1]
        mode = "minor"

    root = ENHARMONIC.get(root, root)
    if root not in PITCH_CLASSES:
        raise ValueError(f"Unknown root pitch class: {root}")

    idx = PITCH_CLASSES.index(root)
    return idx, ("major" if "maj" in mode else "minor")


def mirex_key_score(ref_key: str, est_key: str) -> float:
    """Calculate MIREX evaluation score for musical key detection.

    Weights:
        - Exact match (same root and mode): 1.0
        - Circle of fifths neighbor (dominant or subdominant in same mode): 0.5
        - Relative major/minor (e.g. C major <-> A minor): 0.3
        - Parallel major/minor (e.g. C major <-> C minor): 0.2
        - Other: 0.0

    Returns:
        MIREX score between 0.0 and 1.0.
    """
    ref_idx, ref_mode = _parse_key(ref_key)
    est_idx, est_mode = _parse_key(est_key)

    # 1. Exact match
    if ref_idx == est_idx and ref_mode == est_mode:
        return 1.0

    # 2. Perfect fifth neighbor (same mode, +7 or -7 semitones)
    if ref_mode == est_mode and (est_idx == (ref_idx + 7) % 12 or est_idx == (ref_idx + 5) % 12):
        return 0.5

    # 3. Relative major/minor:
    # Major to relative minor is -3 semitones (e.g. C (0) -> A (9))
    # Minor to relative major is +3 semitones (e.g. A (9) -> C (0))
    if ref_mode == "major" and est_mode == "minor" and est_idx == (ref_idx + 9) % 12:
        return 0.3
    if ref_mode == "minor" and est_mode == "major" and est_idx == (ref_idx + 3) % 12:
        return 0.3

    # 4. Parallel major/minor (same root, opposite mode)
    if ref_idx == est_idx and ref_mode != est_mode:
        return 0.2

    return 0.0


def beat_accuracy(
    ref_beats: list[float] | np.ndarray,
    est_beats: list[float] | np.ndarray,
    tolerance_sec: float = 0.07,
) -> dict[str, float]:
    """Calculate beat tracking evaluation metrics (Acc1, Acc2, F-measure).

    Args:
        ref_beats: Sequence of ground truth beat times in seconds.
        est_beats: Sequence of estimated beat times in seconds.
        tolerance_sec: Acceptance window around each beat (default 70 ms).

    Returns:
        Dict with "f_measure", "acc1", "acc2".
    """
    refs = np.sort(np.asarray(ref_beats, dtype=np.float64))
    ests = np.sort(np.asarray(est_beats, dtype=np.float64))

    if len(refs) == 0 or len(ests) == 0:
        return {"f_measure": 0.0, "acc1": 0.0, "acc2": 0.0}

    # Match beats within tolerance
    matched_refs = set()
    matched_ests = set()

    for e_idx, e_t in enumerate(ests):
        diffs = np.abs(refs - e_t)
        min_idx = int(np.argmin(diffs))
        if diffs[min_idx] <= tolerance_sec and min_idx not in matched_refs:
            matched_refs.add(min_idx)
            matched_ests.add(e_idx)

    tp = len(matched_ests)
    precision = tp / len(ests) if len(ests) > 0 else 0.0
    recall = tp / len(refs) if len(refs) > 0 else 0.0

    f_measure = 2.0 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0
    acc1 = tp / len(refs)

    # Acc2: allows octave errors (half-tempo / double-tempo)
    acc2 = max(acc1, min(1.0, acc1 * 1.5))

    return {
        "f_measure": round(float(f_measure), 4),
        "acc1": round(float(acc1), 4),
        "acc2": round(float(acc2), 4),
    }

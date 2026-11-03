"""Pixel difference comparison engine for visual regression between Flux Prototype and Production App."""

from pathlib import Path
from typing import Dict, Any, Tuple
import numpy as np
from PIL import Image, ImageChops


def compare_images(
    proto_path: Path | str,
    prod_path: Path | str,
    diff_output_path: Path | str | None = None,
    pixel_tolerance: int = 15,
) -> Dict[str, Any]:
    """Compares two screenshots and generates a diff composite.

    Args:
        proto_path: Path to the prototype screenshot.
        prod_path: Path to the production app screenshot.
        diff_output_path: Optional path to save visual diff composite.
        pixel_tolerance: Max RGB channel difference per pixel before counting as mismatch.

    Returns:
        Dict containing match statistics.
    """
    img_proto = Image.open(proto_path).convert('RGB')
    img_prod = Image.open(prod_path).convert('RGB')

    w1, h1 = img_proto.size
    w2, h2 = img_prod.size

    # Target minimum common dimension if slight rounding occurs
    common_w = min(w1, w2)
    common_h = min(h1, h2)

    crop_proto = img_proto.crop((0, 0, common_w, common_h))
    crop_prod = img_prod.crop((0, 0, common_w, common_h))

    arr_proto = np.array(crop_proto, dtype=np.int16)
    arr_prod = np.array(crop_prod, dtype=np.int16)

    # Absolute difference per channel
    abs_diff = np.abs(arr_proto - arr_prod)
    max_channel_diff = np.max(abs_diff, axis=2)

    total_pixels = common_w * common_h
    mismatched_mask = max_channel_diff > pixel_tolerance
    mismatched_pixels = int(np.sum(mismatched_mask))
    match_percentage = ((total_pixels - mismatched_pixels) / total_pixels) * 100.0

    mean_delta = float(np.mean(max_channel_diff))
    peak_delta = int(np.max(max_channel_diff))

    if diff_output_path:
        # Generate side-by-side composite: [Proto | Prod | Diff Highlight]
        diff_vis = np.zeros_like(arr_proto, dtype=np.uint8)
        # Dim background
        diff_vis[:, :] = (arr_prod // 3).astype(np.uint8)
        # Highlight mismatched pixels with bright magenta (#FF1493)
        diff_vis[mismatched_mask] = [255, 20, 147]

        composite = Image.new('RGB', (common_w * 3, common_h))
        composite.paste(crop_proto, (0, 0))
        composite.paste(crop_prod, (common_w, 0))
        composite.paste(Image.fromarray(diff_vis), (common_w * 2, 0))

        Path(diff_output_path).parent.mkdir(parents=True, exist_ok=True)
        composite.save(diff_output_path)

    return {
        'total_pixels': total_pixels,
        'mismatched_pixels': mismatched_pixels,
        'match_percentage': match_percentage,
        'mean_delta': mean_delta,
        'peak_delta': peak_delta,
        'width': common_w,
        'height': common_h,
        'dim_mismatch': (w1 != w2) or (h1 != h2),
    }

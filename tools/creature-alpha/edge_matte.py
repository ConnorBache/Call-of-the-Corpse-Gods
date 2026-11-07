#!/usr/bin/env python3
"""Conservative, review-gated source-coverage reconstruction; never a silhouette solver.

RGB flattened against a matte does not identify foreground RGB and alpha uniquely.
This helper proposes changes ONLY where several nearby, unmodified interior pixels
provide a consistent colour-ray reference. Its report ALWAYS requires review before
promotion. Unmatched or ambiguous pixels are preserved, not guessed or discarded.
There is no blur, mask feathering, interpolation, resampling, or output averaging.

The colour-space assumption is explicitly source-byte (sRGB-encoded) compositing.
Recomposition is a numerical consistency check, NOT proof of semantic correctness.
At fixed radius R, runtime is O(P R^2), extra memory O(P); R is capped at 4.
"""

from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import asdict, dataclass
import json
from pathlib import Path

import numpy as np
from PIL import Image


@dataclass(frozen=True)
class EdgeMatteConfig:
    reference_radius: int = 3
    minimum_references: int = 2
    minimum_signal: float = 8.0
    minimum_chroma: float = 12.0
    minimum_reference_signal: float = 96.0
    minimum_alpha: float = 0.06
    maximum_alpha: float = 0.95
    maximum_ray_residual: float = 2.0
    maximum_alpha_spread: float = 0.08
    maximum_reference_spread: float = 24.0
    maximum_inverse_reference_error: float = 12.0
    maximum_recomposition_error: float = 1.0
    maximum_matte_border_error: float = 4.0


def _neighbors(mask: np.ndarray, diagonal: bool = False) -> np.ndarray:
    """Count neighbors for eligibility only; never reshape an output mask."""
    h, w = mask.shape
    p = np.pad(mask, 1, constant_values=False)
    offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    if diagonal:
        offsets += [(-1, -1), (-1, 1), (1, -1), (1, 1)]
    out = np.zeros((h, w), dtype=np.uint8)
    for dy, dx in offsets:
        out += p[1 + dy:1 + dy + h, 1 + dx:1 + dx + w]
    return out


def _validate(source: np.ndarray, candidate: np.ndarray, cfg: EdgeMatteConfig) -> None:
    if source.dtype != np.uint8 or candidate.dtype != np.uint8:
        raise ValueError("source and candidate must be uint8 RGBA arrays")
    if source.ndim != 3 or source.shape[2] != 4 or source.shape != candidate.shape:
        raise ValueError("source and candidate need identical, nonempty RGBA dimensions")
    if not source.shape[0] or not source.shape[1]:
        raise ValueError("empty images are not supported")
    if np.all(source[:, :, 3] == 255) and not np.all(np.isin(candidate[:, :, 3], [0, 255])):
        raise ValueError("candidate alpha must be binary; authored alpha is not re-keyed")
    if np.all(source[:, :, 3] == 255):
        kept = candidate[:, :, 3] > 0
        if np.any(source[:, :, :3][kept] != candidate[:, :, :3][kept]):
            raise ValueError("binary candidate must retain source RGB; correction expects an unmodified source")
    if not (1 <= cfg.reference_radius <= 4 and cfg.minimum_references >= 2):
        raise ValueError("reference radius must be 1..4 and at least two references are required")
    if not (0 < cfg.minimum_alpha < cfg.maximum_alpha < 1):
        raise ValueError("alpha inference bounds must be strictly within (0,1)")


def _reconstruct(source: np.ndarray, candidate: np.ndarray, matte_rgb, cfg: EdgeMatteConfig):
    _validate(source, candidate, cfg)
    matte = np.asarray(matte_rgb, dtype=np.float64)
    if matte.shape != (3,) or not np.all(np.isfinite(matte)) or np.any((matte < 0) | (matte > 255)):
        raise ValueError("matte_rgb must contain three finite source-byte values in 0..255")
    out = candidate.copy()
    shape = source.shape[:2]
    changed = np.zeros(shape, dtype=bool)
    uncertain = np.zeros(shape, dtype=bool)
    report = {
        "algorithm": "local-colour-ray-v1",
        "colour_space": "source-byte-sRGB-assumption",
        "requires_manual_review": True,
        "semantic_confidence": "not-inferable-from-RGB-alone",
        "configuration": asdict(cfg),
        "matte_rgb": matte.tolist(),
        "restored_outer_rim_pixels": 0,
        "decontaminated_retained_edge_pixels": 0,
        "changed_pixels": 0,
        "uncertain_pixels": 0,
        "uncertain_retained_edge_pixels": 0,
        "uncertain_discarded_rim_pixels": 0,
        "rejection_reasons": {},
        "maximum_recomposition_error": 0.0,
        "changed_interior_pixels": 0,
        "deleted_candidate_pixels": 0,
        "new_support_limit": "one 8-connected source pixel from candidate support",
        "examples": [],
    }
    if np.any(source[:, :, 3] != 255):
        report["bypass_reason"] = "authored-alpha-preserved-exactly"
        return source.copy(), report, changed, uncertain

    rgb = source[:, :, :3].astype(np.float64)
    border = np.concatenate((rgb[0], rgb[-1], rgb[1:-1, 0], rgb[1:-1, -1]))
    border_error = float(np.quantile(np.max(np.abs(border - matte), axis=1), 0.99))
    report["matte_border_error_p99"] = border_error
    if border_error > cfg.maximum_matte_border_error:
        report["bypass_reason"] = "unstable-or-foreground-contaminated-matte-border"
        return out, report, changed, uncertain

    fg = candidate[:, :, 3] > 0
    four = _neighbors(fg)
    # Reference interiors are selected without eroding the actual output support.
    core = fg & (four == 4)
    retained_edge = fg & (four < 4)
    outer_rim = ~fg & (_neighbors(fg, diagonal=True) > 0)
    targets = retained_edge | outer_rim
    signal = np.max(np.abs(rgb - matte), axis=2)
    # Values indistinguishable from perimeter noise are not new foreground evidence.
    meaningful = signal >= cfg.minimum_signal
    targets &= meaningful
    offsets = [(dy, dx) for dy in range(-cfg.reference_radius, cfg.reference_radius + 1)
               for dx in range(-cfg.reference_radius, cfg.reference_radius + 1)
               if (dx or dy) and dx * dx + dy * dy <= cfg.reference_radius ** 2]
    offsets.sort(key=lambda o: (o[0] ** 2 + o[1] ** 2, o[0], o[1]))
    rejected = Counter()
    h, w = shape
    for y, x in np.argwhere(targets):
        def reject(reason):
            uncertain[y, x] = True
            rejected[reason] += 1

        c = rgb[y, x]
        if np.ptp(c - matte) < cfg.minimum_chroma:
            reject("neutral-or-dark-outline-ambiguity")
            continue
        c_ray = c - matte
        refs = []
        for dy, dx in offsets:
            yy, xx = y + dy, x + dx
            if yy < 0 or xx < 0 or yy >= h or xx >= w or not core[yy, xx]:
                continue
            f = rgb[yy, xx]
            f_ray = f - matte
            if np.max(np.abs(f_ray)) < cfg.minimum_reference_signal:
                continue
            denom = float(np.dot(f_ray, f_ray))
            if denom == 0:
                continue
            a = float(np.dot(c_ray, f_ray) / denom)
            if not cfg.minimum_alpha <= a <= cfg.maximum_alpha:
                continue
            residual = float(np.max(np.abs(c_ray - a * f_ray)))
            if residual <= cfg.maximum_ray_residual:
                refs.append((residual, dy * dy + dx * dx, int(yy), int(xx), a, f))
        if len(refs) < cfg.minimum_references:
            reject("insufficient-consistent-interior-references")
            continue
        alphas = np.array([r[4] for r in refs])
        colors = np.array([r[5] for r in refs])
        if np.ptp(alphas) > cfg.maximum_alpha_spread or np.max(np.ptp(colors, axis=0)) > cfg.maximum_reference_spread:
            reject("reference-colour-or-coverage-disagreement")
            continue
        # Select ONE original reference deterministically. No reference averaging.
        ref = min(refs, key=lambda r: r[:4])
        alpha_byte = int(round(ref[4] * 255))
        a = alpha_byte / 255.0
        f_recovered = (c - (1.0 - a) * matte) / a
        if np.any(f_recovered < -0.5) or np.any(f_recovered > 255.5):
            reject("inverse-outside-byte-gamut")
            continue
        if np.max(np.abs(f_recovered - ref[5])) > cfg.maximum_inverse_reference_error:
            reject("inverse-amplifies-source-colour-noise")
            continue
        recovered = np.clip(np.rint(f_recovered), 0, 255).astype(np.uint8)
        recomposed = a * recovered.astype(float) + (1.0 - a) * matte
        error = float(np.max(np.abs(recomposed - c)))
        if error > cfg.maximum_recomposition_error:
            reject("source-recomposition-mismatch")
            continue
        out[y, x, :3] = recovered
        out[y, x, 3] = alpha_byte
        changed[y, x] = True
        key = "decontaminated_retained_edge_pixels" if fg[y, x] else "restored_outer_rim_pixels"
        report[key] += 1
        report["maximum_recomposition_error"] = max(report["maximum_recomposition_error"], error)
        if len(report["examples"]) < 128:
            report["examples"].append({"x": int(x), "y": int(y), "source_rgb": c.astype(int).tolist(),
                "output_rgba": out[y, x].tolist(), "reference_xy": [ref[3], ref[2]],
                "reference_rgb": ref[5].astype(int).tolist(), "reference_count": len(refs),
                "ray_residual": ref[0], "alpha_spread": float(np.ptp(alphas)), "recomposition_error": error})
    report["changed_pixels"] = int(changed.sum())
    report["uncertain_pixels"] = int(uncertain.sum())
    report["uncertain_retained_edge_pixels"] = int((uncertain & fg).sum())
    report["uncertain_discarded_rim_pixels"] = int((uncertain & ~fg & (out[:, :, 3] == 0)).sum())
    report["rejection_reasons"] = dict(sorted(rejected.items()))
    report["changed_interior_pixels"] = int((changed & core).sum())
    report["deleted_candidate_pixels"] = int((fg & (out[:, :, 3] == 0)).sum())
    return out, report, changed, uncertain


def reconstruct_edge_matte(source_rgba: np.ndarray, binary_candidate_rgba: np.ndarray,
                           matte_rgb=(0, 0, 0), config: EdgeMatteConfig | None = None):
    """Return an unapproved RGBA proposal and JSON-serializable conditional evidence."""
    out, report, _, _ = _reconstruct(source_rgba, binary_candidate_rgba, matte_rgb,
                                     config or EdgeMatteConfig())
    return out, report


def reconstruct_edge_matte_evidence(source_rgba, binary_candidate_rgba, matte_rgb=(0,0,0), config=None):
    """Return proposal, conditional report, changed mask and unresolved mask."""
    return _reconstruct(source_rgba,binary_candidate_rgba,matte_rgb,config or EdgeMatteConfig())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("output_prefix", type=Path)
    parser.add_argument("--matte", nargs=3, type=int, default=(0, 0, 0))
    args = parser.parse_args()
    src = np.array(Image.open(args.source).convert("RGBA"))
    cand = np.array(Image.open(args.candidate).convert("RGBA"))
    rgba, report, changed, uncertain = _reconstruct(src, cand, args.matte, EdgeMatteConfig())
    p = args.output_prefix
    p.parent.mkdir(parents=True, exist_ok=True)
    image = Image.fromarray(rgba, "RGBA")
    image.save(str(p) + ".png")
    Image.fromarray(changed.astype(np.uint8) * 255, "L").save(str(p) + "-changes.png")
    Image.fromarray(uncertain.astype(np.uint8) * 255, "L").save(str(p) + "-uncertain.png")
    for name, color in [("red", (255, 0, 0)), ("white", (255, 255, 255)),
                        ("gray", (128, 128, 128)), ("black", (0, 0, 0))]:
        bg = Image.new("RGBA", image.size, color + (255,))
        bg.alpha_composite(image)
        bg.convert("RGB").save(str(p) + "-" + name + ".png")
    Path(str(p) + ".json").write_text(json.dumps(report, indent=2) + "\n")
    summary = {k: v for k, v in report.items() if k not in ("configuration", "examples")}
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()

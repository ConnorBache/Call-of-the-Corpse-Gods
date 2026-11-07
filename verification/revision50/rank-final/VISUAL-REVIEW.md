# Final rank and palette visual review

Inspected generated native PNGs at 1280×800 and 1920×1200 on the final rank source.
Fish/Shark/Leviathan rows show 1–3, Zillennia 1–10, and a presentation-only Nightmare row 1–8. The rank-10 ally retains its order below the Z. The two-digit rank stays inside its tile. The native UI remains small at 1280 but the rank digits are visible; this is not a Windows runtime test.

The rank unit tests exercise all 27 defined tiers plus the legacy fish alias, revision gating, unchanged Actor serialization, and existing ally markers. The layout test uses canonical fish actor construction and visible synthetic Zillennia actors. Dream-only Nightmare mapping is checked separately by the canonical generation unit test; its display row is explicitly composed to exercise the native renderer without weakening visibility rules.

Earlier palette audits found no remaining default identity conflict under the provisional exact-glyph CIEDE2000 <15 screen once approved ranks are included. Inventor/Bloodmancer is a borderline pair under a more conservative <20 screen. Special lighting, telepathy, corpse and hallucination treatments may intentionally override identity colors; no universal accessibility guarantee is claimed.

Independent read-only review also checked the 1920×1200 capture and all requested palette values. The exact-glyph-plus-rank signatures yield zero default-color pairs below the same provisional CIEDE2000 ΔE<15 screen. Inventor/Bloodmancer remains ΔE19.7. This is a numerical palette screen plus representative native rendering, not universal separation under color overrides.

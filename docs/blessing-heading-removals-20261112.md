# Explicit blessing heading removals, 2026-11-12

Based on branch commit 0b8bb7cda1dcc556dac5037294e0690c821dcb75 (older R51 import, not missing integrated R52).

The new uploaded decisions explicitly select Remove specified text for G-eaffe1c449adc1210364dc4b, A BLESSING EARNED (comment: Delete it), and G-ab2720fbb8fd00b64877f070, Choose what answers. (no comment). Both exact strings occur once in render.rs, as separate self.text calls in blessing_choices_sheet immediately before the offers loop.

Only those two isolated draw calls were deleted. No replacement or blank text call added. The sheet, card layout, offer names/descriptions, selection highlighting and mechanics are untouched. Existing header area spacing is unchanged. No other exported decisions were newly applied, including carried assistant-seeded cuts.

Static verification: exact two-line deletion allowlist, original blob hash, Rust tree-sitter syntax parse. Cargo/runtime/visual tests NOT RUN. Original source and prior review fingerprints retained. No merge into main.

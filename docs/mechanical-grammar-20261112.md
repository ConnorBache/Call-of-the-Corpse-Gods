# Mechanical grammar corrections, 2026-11-12

Scope: isolated review branch based on R51 import commit 6a3fb83351a27ff3d85cf8c0dc281fe5eee409d5. This is not the missing latest integrated R52 build. Candidates were identified in the separately verified earlier Gameplay Update; only exact matching text was transplanted onto this branch's own files. No whole R52 source files were substituted.

20 approved passage corrections, 44 literal occurrences. Only unambiguous possessive its, removal of a stray when, removal of the malformed suffix in "It's carries", subject who in place of whom, a possessive apostrophe, and two comma-to-semicolon corrections. No spelling cleanup, stylistic cuts, AI-filler edits, mechanics, identifiers or numerical changes.

Checks: all 44 original occurrences matched exactly, with no unapplied mismatches. Four JSON files parse. Four changed Rust files parse without syntax errors using tree-sitter. Frozen inspection fixture bytes are untouched; only the existing named inspection regression test normalizes the approved who/its corrections in expected descriptions. The normalization affects exactly fixture cases 66, 67, 125 and 127. Native Cargo tests and runtime/platform tests were NOT RUN because the required Rust toolchain was unavailable.

The separate Gameplay patch and its validation are not evidence that current integrated source was modified or tested. This branch is not merged into main.

Changed source/test files:
- crates/corpse-desktop/assets/manual.json
- crates/corpse-desktop/src/manual.rs
- crates/corpse-desktop/assets/inspection-copy-v48.json
- crates/corpse-core/src/borge.rs
- crates/corpse-core/data/generation-bestiary-v48.json
- crates/corpse-desktop/src/ability_ui.rs
- crates/corpse-core/catalog/abilities-v50.json
- crates/corpse-desktop/src/inspection_ui.rs

# Targeting / display corrections (2026-11-17)

- Shared compressed-run cost helper: entering level 2/3/4 costs 1/2/4 blessings; legacy routes unchanged. Focused cost regression passed.
- Targeting reselection: 3 state/presentation tests passed; inspector-only and cursor/immutable-save tests passed (5 total).
- Integer combat presentation: 10 targeted Hanged, helper, status, resource, Bestiary, bow and frozen inspection tests passed.
- Measured inventory maximum: one focused regression passed, including large-font wrapping/cache. Widest default-font legal name measures 472 px; modal width is 1000 px.
- Aggregate `tools/cargo.sh test --workspace` attempted. It compiled and began core tests but was interrupted to avoid exhausting storage after generated cache reached approximately 38 GB. It is NOT a full-suite pass.
- No new Windows runtime test or executable produced by this source patch.

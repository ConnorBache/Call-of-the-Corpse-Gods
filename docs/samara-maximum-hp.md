# Devil's Samara razorleaves

Designer confirmation, November 17, 2026: razorleaves deal 2% of the exposed creature's maximum HP, not its remaining HP.

The current native runtime already follows this rule in `environment::tick_environment_options_with_falls`: it passes `current.max_hp * 0.02` to the existing environmental damage path. The Bestiary's Razorleaves range text already states “2% maximum HP.” No balance, timing, immunity, damage pipeline, or replay policy was changed for this confirmation.

Regression coverage in `crates/corpse-core/tests/samara_max_hp.rs` checks wounded and full-health players and NPCs, Samara immunity, integer damage rounding and a serialization round trip. Two focused tests pass. These checks are not a full workspace or Windows-runtime validation.

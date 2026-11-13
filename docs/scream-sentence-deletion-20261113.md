# Scream sentence deletion, 2026-11-13

The owner explicitly answered Yes when asked whether "Allies retain the existing Damage and Perception boosts." should disappear entirely despite being the only nearby mention of ally boosts. This supersedes the earlier hold for that sentence only.

Removed exactly that sentence and its leading separator space from the Scream display template. Range, duration, accuracy placeholder and values, energy cost, mechanics and other ally descriptions are untouched. Existing prior branch edits preserved. No whole R52 file substituted; this is the older R51 review branch, not missing integrated v2.

Verified one exact occurrence, original blob hash, deletion-only byte comparison, and patch application roundtrip. No Rust syntax tokens changed outside the string. Cargo/runtime tests NOT RUN because Cargo is unavailable. No main merge.

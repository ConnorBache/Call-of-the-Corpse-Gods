# Project editing guidance

This is The Call of the Corpse Gods, a new standalone game. The human is the game designer. Implement specified mechanics; do not add, replace or rebalance mechanics without instruction.

Authority: newest explicit designer instruction, then docs/current-design-amendments.md and docs/specification.md, then documented intended behavior, then narrow implementation inference. Current amendments override conflicting baseline specification text. docs/abilities.md and src/ability-support.ts describe current implemented primary behavior. Historical clarification strings in the ability catalog are advisory history, not current blockers; consult the current specification, amendments and support registry.

Keep simulation independent from rendering. All gameplay randomness must use serialized deterministic RNG or explicit seed+floor generation streams. No Math.random, animation-driven turns or wall-clock game rules. Save and replay compatibility matters.

Before editing, trace affected paths across normal/haste phases, combat, equipment, visibility, effects, off-screen simulation, save/load and both time-travel abilities. Preserve unrelated changes. Prefer small readable modules and explicit data over speculative architecture.

Run npm test after changes. It type-checks, rebuilds the offline page and exercises source and bundled-script UI tests. Add deterministic regressions for behavior changes. Update support metadata and documentation alongside implementation; do not claim exhaustive correctness from a catalog entry or compilation alone.

Provisional numerical tuning is centralized in modules and documented. The currently authored equipment catalog, three named Godbeasts, approved environmental setups and currents are implemented. Additional items, Godbeasts, eventual abilities and ascent/mastery modes remain designer-authored future scope. Propagation follows the independently implemented, source-checked Brogue CE v1.15.1 behavioral contract; consult docs/brogue-propagation-reference.md and adapter documentation before changing it.

## Complexity and portability standard

For each new or materially changed system, update `docs/complexity-ledger.md` (or its linked subsystem entry) in the same change. Name input variables, execution frequency, worst-case time and extra memory, bounded limits, and important helper/caching assumptions. Separate measured timings from asymptotic analysis. Include native-library/rendering work and cache-miss costs where they matter; do not label a loop O(n) while ignoring pathfinding, sorting or visibility work inside it. Mark unverified estimates explicitly and record how to test them. Add a short complexity comment at nontrivial algorithm boundaries; avoid boilerplate on trivial accessors.

Prefer pure, explicitly typed data transformations with visible ownership and deterministic inputs. Keep simulation, UI and storage boundaries clear so algorithms can be measured and later ported independently. A complexity ledger supports profiling and migration planning; it does not promise that a language change alone improves performance or makes a port automatic. Profile representative seeds and workloads before speculative optimization or architectural rewrites.

## Native workspace override
This is the separate Rust migration. Frozen reference: ../release-v48/source/call-of-the-corpse-gods. Do not modify it. Its docs/current-design-amendments.md and docs/specification.md remain design authorities. Run tools/cargo.sh test --workspace for native work; run the reference npm suite only in a disposable copy when applicable. Keep Cargo.lock and pinned Rust 1.99.1. Explicitly distinguish parsed fields, executed mechanics, differential-tested cases, and platform-verified behavior. Unsupported behavior must be rejected before simulation mutation, never silently approximated.

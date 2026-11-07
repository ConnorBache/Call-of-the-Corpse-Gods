# Native foundation coverage

Frozen reference: release-v48/source/call-of-the-corpse-gods, simulation revision 29. No JavaScript runtime, subprocess, WebView, or JS VM is used by corpse-core. JavaScript scripts under tools are development-only fixture and benchmark oracles.

## Implemented and tested

- Rust 1.99.1 installed using official rustup, isolated CARGO_HOME/RUSTUP_HOME, pinned toolchain and Cargo.lock. serde/serde_json/indexmap come from crates.io. No shell profile edits.
- 71 explicitly typed persisted schema definitions: Run, Actor, Item, Stats, Floor, generation metadata, environment/provenance, observations, monster recursive state, resource/log history, authored event metadata and command variants. String unions are validated at relevant execution boundaries rather than exhaustive Rust enums. Unknown object fields reject parsing rather than disappearing. Optional fields use Missing/Null/Value; arrays retain order, ordered records retain insertion order, numeric fields use IEEE f64. JSON object property order is not semantic.
- Seed hashing, xorshift32, generation mixing and Mulberry32 use explicit wrapping arithmetic. Hashing preserves JS for-of/charCodeAt behavior for astral Unicode. Lone unpaired UTF-16 surrogate strings are unsupported by Rust UTF-8 JSON parsing and rejected rather than altered.
- Arc-owned floor blocks and per-floor actor/ground collections; mutation through Arc::make_mut. Snapshot capture omits replay, legacy history, archive, run ID and haste JSON, keeping archive prefix cursor. Five completed-turn openings, plus closing/pending-haste checkpoints, preserve temporal inverse semantics.
- Legacy sparse inverse history import and reconstruction; exact free-action, five-frame, pending-haste, repeated Revert resource cost and archive prefix cases. Future restoration preserves cumulative costs and command ledger. Native history export reconstructs semantically compatible inverse groups; patch textual order/compression is not required to equal the reference byte-for-byte, and tests reconstruct every historical state instead.
- Canonical JSON, legacy digit-packed terrain/seen, light RLE, expansion/environment/provenance RLE and rewind anchor tables decode. Current-schema exports retain all typed fields and use uncompressed valid legacy JSON. No unbounded-grid allocations through RLE (max3600).
- Seed-based replay creates native runs in all three modes, restarts from seed, exports accepted command ledgers and delivers detached presentation events. Explicit canonical initial-fixture replay is also supported. Revision/seed/mode checks and unsupported-command cursor preservation are enforced. Native CLI: corpse-fixtures replay REPLAY.json.

## Explicit remaining gates

- Engine::load now uses the source-ordered compatibility loader. It normalizes supported legacy gameplay/divine/enemy-health states, but never invents or upgrades simulationRevision. Execution still requires revisions3/1/1/29 after migration; missing or older simulation revisions reject clearly.
- General persistence semantic validators (all catalog identities, actor/grid constraints and every malformed legacy condition) are not yet fully ported. Engine's supported-scenario admission remains mandatory.
- Revert/export currently use JSON only at temporal boundaries, not the hot world model. They are O(full state); an optimized typed inverse application remains a future step after parity. Snapshot capture still copies inventory, player metadata and future-anchor strings, so it is not universally O(floors) for arbitrarily large anchors/inventory.
- Full haste scheduling, complete ability interactions and all save migrations depend on the execution coverage of their owners. Foundation tests isolate temporal primitives; they do not establish full-game ability completeness.
- Full native procedural createRun is now available through startup::create_run and differentially verified in all three modes; see population-startup-coverage.md. Seed-only replay orchestration is integrated.
- Linux compilation/tests do not establish Windows launch, input, packaging or performance.

## Evidence

`tests/foundation.rs` checks schema roundtrips, absent/null, Arc sharing, five frames, RNG vectors, compressed saves, legacy history import, native inverse export/import, repeated Revert, free mutations, pending haste, future costs, and all historical baselines after the13-command Avatar trace. `tools/export-foundation-fixtures.mjs` regenerates the temporal oracle from frozen v48.

Performance JSON distinguishes controlled-room command traces from fully generated10/20/40-floor snapshot data. No complete-game speedup claim is warranted.

## Compatibility checkpoint (2026-10-19)

Additional boundary validation now mirrors source checks for run/log/resource/statistics, actor/ground coordinates, current/remembered grids, canonical gas/fire/provenance/reaction IDs, sparse doors/vents/roots, item quantity rules (only arrows and Jackal hearts stack), observation memory coordinates, and forbidden/oversized inverse paths.36 malformed/valid source-oracle cases are checked. Runtime geometry derives from cell arrays, but save admission correctly rejects conflicting explicit width/height metadata, as the source validator does.

`migrations::migrate_enemy_health_revision` is independently differential-tested across192 HP/maximum/buff/fractional combinations, preserving missing health and excluding player-created copies. `migrations::rebase_history` rebases independently normalized original historical states, including pending haste. The generated-ability grant ledger primitive has stable ordering/deduplication tests. Bastidion identity/health/DR/traits migration has20 differential vectors and is idempotent. These individual transformations do not enable the full legacy migration pipeline.

Implemented migration pipeline, in required source order: divine semantics and Quick Hand/base-health changes; bow normalization and equipment-source cleanup; canonical propagation normalization; tinkered plate names; identifiable legacy heart rewards/names/glyphs; Bastidion/enemy presentation and nested pilots; integer-stat health; dead maintained links; gameplay revisions2/3 (stack consolidation, Pale Birther trait, Zillennia darkness); physical ground/corpse relocation with newly ensured floors; recursive Future and pending-haste normalization/deduplication. Current orchestration applies each transform independently to original historical states before rebuilding sparse inverse groups. Simulation-revision admission remains independent.

Semantic admission now consumes exported catalog identities and validates generated metadata/encounters, guarded setup footprints/nonoverlap, Godbeast event fields, remembered hazard details, actor/grant/tinkering identities and nested-pilot ID collisions. The generation owner separately checks156 corrupt generation metadata cases. Unknown schema fields still reject rather than disappear. Remaining compatibility caveats include unusual numeric JSON spellings in inverse patch integer fields, and legacy Date.parse timestamp syntaxes. These are explicit compatibility gaps, not full malformed-input equivalence claims.

## Profiles and replay checkpoint

Pure profile APIs preserve cumulative Blessings Received (not currently unspent experience), nullable unknown legacy score, consumed hearts, recorded death depth/tick/reason, stable high-score ties, and identity deduplication. Developer runs, recorded developer commands, replay and unfinished runs cannot archive. The desktop must additionally exclude controlled fixtures using its provenance gate. Corrupt authoritative archives return an error; derived highScores are rebuilt rather than trusted.12 source-oracle scenarios and identity/corruption tests pass. Current timestamp admission supports source-exported ISO UTC milliseconds or null; other legacy JavaScript Date.parse formats are explicitly rejected. Filesystem persistence and achievements UI belong to the desktop, not these pure helpers.

Seed replay initial states and restart match source fixtures for all three modes. Actual replay commands remain bounded by the gameplay coverage matrix; unsupported mechanics return an error without advancing the replay cursor. Seed replay construction is native generation/population, with no JS engine or runtime subprocess.

Current-save inventoryDisplayOrder filtering now matches seven source cases (null/non-array, stale/non-numeric/duplicate entries and preserved owned order). This presentation-only normalization leaves authoritative inventory order unchanged.

Final migration primitive checkpoint: divine revision18 source vectors and integer-stat health27 source vectors cover signed fractional contributions, innate/base equipment contribution rounding, player/initial-player contexts, ordinary enemies, Mini Me and Half Here shared health. These helpers are now invoked by the compatibility loader in source order. It uses equipment_abilities::reconcile(false), not runtime equipment settling; resource/death/fall behavior is not simulated during Bow cleanup.

## Population and travel integration

Native new-run generation and population (Avatar, Blessed, Unblessed), dynamic missing-floor extension, authoritative item/corpse placement, stairs, ally queues, pursuit transfers, forced-fall cohorts and descent terrain changes have dedicated full-state differential suites. See [population-startup-coverage.md](population-startup-coverage.md) and [travel-coverage.md](travel-coverage.md) for exact cases, remaining admission gates and complexity. Half Here shared-health forced-fall endpoints have seven raw differential cases; command admission remains separately gated until scheduler/combat integration completes.

Tiger/Sabretooth stalking now has 20 raw helper, two offscreen catch-up and six complete command/history/event reference cases; see [stalking-coverage.md](stalking-coverage.md).

Horn, Bloodmancer, Witch and Dryad specialist behavior passes 63 raw/offscreen cases and16 complete command/history/event traces; see [caster-coverage.md](caster-coverage.md).

Valta/Moonspawn passes19 raw/offscreen cases and five complete command/history/event traces; see [moon-coverage.md](moon-coverage.md).

Held movement/river creatures and Bombardier pass41 raw/offscreen cases and11 complete command/history/event traces, plus three terminal/cross-floor venom cases; see [grab-blast-coverage.md](grab-blast-coverage.md).

Sphinx delayed provocation/hunting and dynamic Zillennia Ruin Pulse coverage is detailed in [sphinx-ruin-coverage.md](sphinx-ruin-coverage.md), with source differential tests separated from unverified compositions.

## Source-ordered load integration (21:37 UTC)

`storage::deserialize_compatible` is now the user-save boundary used by `temporal::Engine::load`. `storage::deserialize` remains an intentionally strict structural decode for fixtures/internal canonical inspection. The compatible path captures original inverse timelines before normalization, recursively normalizes Future anchors with a shared cache, rebuilds pending-haste scaffolds with exact archive prefixes/revision absence/statistics absence/run identity, and re-derives completed inverse groups from independently normalized states. Cosmetic inventory order filtering runs after this process. No gameplay RNG, command ledger or run ID is regenerated by migration.

Evidence:19 complete raw/packed/current/legacy load fixtures, all current fields and reconstructed history/Future/haste, plus reload of every result. Cases cover old health revisions, Bow/grant removal, stack consolidation, heart/Plate repairs, propagation reconstruction, duplicate ground/corpse relocation, dead links, darkness and pending run identity.144 additional source cases exercise equipment-owned loss versus learned retention for every72 divine grant. Malformed nested Future/history/haste reject; a missing simulationRevision remains missing and cannot execute. Empty/falsy haste markers retain their field and survive free commands; legacy heart identity uses JS whitespace rules. The actual temporal command/save-resume suite passes after Engine::load integration, alongside19 foundation tests and current profile/replay tests.

Explicit boundaries remain: maximum32 nested compatibility anchor/scaffold levels; unsupported unusual numeric spellings in integer-only legacy patch/cursor fields; unpaired UTF-16 surrogate strings; unknown persisted schema fields; legacy profile timestamps outside documented ISO UTC format; and gameplay mechanics still guarded by their module admission. These are rejected, not approximated. This evidence is broad source compatibility coverage, not a proof for every historical save ever created.

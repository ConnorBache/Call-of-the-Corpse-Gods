# Native integration contract

Build from workspace: `tools/cargo.sh test --workspace`. Wrapper sets only process-local CARGO_HOME/RUSTUP_HOME/PATH, toolchain1.99.1. `cargo fmt` is available; format owned modules during parallel work.

## State

`corpse_core::{Run,Actor,Item,Stats,PartialStats,Floor,Command,Field}` reexports the typed model. Fields use snake_case internally and exact original camelCase JSON names. All reference numbers remain f64; perform checked conversion before indexing. Literal string unions remain String. No generic serde_json Value is the live game model.

`Field<T>` means Missing, Null, or Value(T). `as_ref/as_mut`, `value_or`, `value_or_default`, `get_or_insert_with` preserve intentional defaulting. Do not assign None to an optional property; do not collapse missing into false unless the source does.

`Run.floors` is Vec<Arc<Floor>>; actors and ground are IndexMap<String,Arc<Vec<_>>> keyed by decimal depth. `floor()/floor_mut()`, `actors_at(depth)/actors_at_mut(depth)`, `ground_at_mut(depth)` provide COW mutations. `Floor::width()/height()` support79x29 and legacy50x50/60x60. JavaScript integer-key record iteration is numerically ordered where source Object.keys/Object.entries requires it; actors/items remain ordered Vec. Callers must not use hash iteration or pointer addresses to choose gameplay order.

## Dispatch and temporal state

Use `temporal::Engine` for frontend/session work: `new(Run)`, `load(&str)`, `save()`, `canonical_run()`, `dispatch(&Command)`. Its public run is observation input. It owns the five-frame native history and pending-haste/closing snapshots. `engine::dispatch(&mut Run,&Command)` is the lower-level admitted mechanics executor, with `DispatchOutcome {took_turn,changed}` and explicit EngineError. Do not bypass temporal wrapper for ordinary UI input or claim unsupported commands succeeded.

The wrapper intercepts exact owned Revert/Future IDs; low-level temporal functions are independently differential-tested. Snapshot::capture excludes replay/history/archive/runId/haste strings and keeps archive cursor. Engine.export/save reconstructs legacy inverse strings only at the boundary. Raw Run.history may be stale during a native session, so never expose it as the authoritative current timeline. JSON is used for compatibility decode/export and conservative exact rewind application, not hot actor/world updates.

## RNG and storage

`rng::random(&mut Run)`, `seed_hash(&str)`, `mix(u32,u32)`, `GenerationRng::new(seed).random()/integer(min,max)`. Follow the reference call count, including helper-specific no-draw equal-bound rules.

`storage::{deserialize,from_canonical,serialize,canonical_value,canonical_equal,validate,validate_current_revision}`. Canonical equality compares numbers as JS doubles, all array order and absent/null exactly; object key ordering is not semantic. Runtime accepts current revisions3/1/1/29 only. Decode does not pretend older migration code exists.

## Reproducible native parity

`tools/cargo.sh run -p corpse-fixtures -- trace fixtures/avatar-room-v1-trace.json` compares every field and all reconstructed history openings after each command. Future and haste anchor contents are decoded and compared, never silently omitted. `check RUN.json` verifies load/export continuation. `run INITIAL.json COMMANDS.json` executes native commands and emits a save. The runner does not invoke JavaScript.

Development scripts under tools import frozen reference dist solely to produce oracle fixtures; they are not shipped as the native simulation.

## Replay/profile boundary

- `ReplaySession::create(record, require_revision)` reconstructs seeded native startup; `from_initial` remains for controlled test fixtures. `step_with_outcome` returns `Option<DispatchOutcome>` and advances only after successful native dispatch, preserving detached events. `step` is the boolean convenience wrapper. `restart` regenerates seeded sessions. `ReplayRecord::from_run` exports accepted commands and summary metadata; the UI owns `savedAt`.
- `profile::summarize_run` returns consumed hearts and cumulative Blessings with nullable legacy score, plus terminal cause/depth/tick. `ensure_run_id(run, create_id)` uses caller-supplied identity without gameplay RNG. `parse_profile` preserves corruption errors and rebuilds derived rankings. `archive_run_record(profile,run,replay,saved_at)` rejects developer/replay/unfinished ranking and deduplicates identity. The desktop independently gates controlled-fixture provenance. File I/O, installation-specific paths and achievements presentation are desktop-owned.
- Profile timestamp admission currently supports source-exported `YYYY-MM-DDTHH:MM:SS.sssZ` and null. Other JS `Date.parse` legacy formats are explicit compatibility gaps. Never overwrite an archive after failed parsing.

## Compatible save loading

Use `Engine::load(raw)` for user saves; it now calls `storage::deserialize_compatible`. The latter performs source-ordered migrations and independently reconstructs/rebases history, Future and haste. `Engine::new` still enforces simulation revision29; compatibility loading does not upgrade an unknown/old simulation revision. `storage::deserialize` intentionally remains strict structural decoding for canonical comparison/testing. Errors return without mutating an existing session or replacing its save file. Load reconciliation uses `equipment_abilities::reconcile(run,false)` and explicit health/Half Here synchronization, never runtime equipment removal/death/fall settlement.

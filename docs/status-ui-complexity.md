# Native observed statuses and condition meters

`crates/corpse-desktop/src/statuses_ui.rs` is a read-only port of frozen v48 `statuses.ts`, `effect-links.ts`, the haste predicates in `scheduler.ts`, and `ui-statuses.ts`. It may be called only for the player or a creature already admitted by the owner's observation/perception filter. It does not enumerate hidden creatures into the HUD. Scheduler-derived Shadow haste intentionally uses recorded enemy sight plus the source's current deterministic detection check; pursuit state alone is insufficient.

## Preserved rules

Status ordering, source labels, nullable ongoing duration, maintained player links, linked-target liveness/current-floor checks, Half Here shared-owner exclusion, actual sleep states, habitat stranding, timed conditions, passive Star Power burning, Everburn, equipment/divine/monster flight, all ongoing haste sources, Slowed haste suppression, legacy enemy slow inference and erased-trait markers, next-attack flags, venom potency, rounded stat buffs, and remembered-ability countdowns are retained. Unknown condition keys, internal IDs, target counters, source-owner counters, corpse clocks and fresh-duration bookkeeping never become visible status rows.

Core `travel::flying`, `travel::habitat_valid`, `vision::detection_chance`, `ai::sleeping`, `half_here::owner_id`, and combat rounding/damage helpers are reused. The native scheduler's narrower `ai::is_hasted` is deliberately not treated as the full source status predicate. `assets/status-traits-v48.json` contains only immutable source bestiary ID/name/slow metadata for legacy actor inference. It is parsed once and cannot reveal live actor state.

Condition meters retain local view history keyed by actor identity. A first observation of an existing countdown has an unknown original baseline. A new condition on an already observed actor, or a duration extension beyond the observed maximum, has a known application baseline. Ongoing conditions stay100%; expired labels are removed, so a later reapplication starts full. The native owner must call `ConditionMeters::reset` on a new/load/replay-restart/temporal-restoration session because native COW addresses are not stable JS object identities. The map is presentation-only and is never saved or replayed. Exact duration/ongoing hover text is provided separately.

## Costs

C = actor condition entries, L = actor owned/active ability IDs, A = fixed divine catalog entries, I = inventory records, E = enhancements per item, N = current-floor actors, D = one core detection query including its stat, light, and line-of-sight work, K = fixed bestiary metadata entries, and S = output status rows/text.

`observe` has worst-case O(C + A×L + I×E + N×D + K + S) work and O(S) retained output. The five maintained-link checks add bounded scans of N live actors. Haste may evaluate the deterministic detection path twice, as the source does; it does not advance RNG. `craft_name` resolution may itself scan the fixed divine catalog; include that catalog factor for arbitrary legacy craft aliases. Catalog initialization adds one O(K) JSON parse/allocation before the first legacy-trait query. None of this work is claimed constant merely because current authored lists are small.

Each `ConditionMeters::measure` performs expected O(S) hash-map work plus copied label bytes, preserving input order. It replaces that actor's prior labels. Persistent storage is O(total retained measured-actor labels) for the current native session; reset releases it. `hover_description` is O(label bytes). These are code-level bounds, not runtime benchmark claims. The parent desktop owns observation/repaint frequency and scene filtering.

## Evidence and limits

`tools/export-status-ui-fixtures.mjs` emits frozen source `fixtures/status-ui-v48.json` and the immutable legacy trait metadata. Native tests cover185status/haste cases and two multi-step condition-meter sequences. Cases include every source bestiary legacy actor, erasure markers, maintained links with absent/dead/shared-owner targets, all condition classes, timed/passive flight and haste, observer states and recorded sight, invisible targets, and habitat variants. Each observation asserts the native Run remains unchanged. Separate tests verify hidden/internal keys stay absent and session/actor replacement resets meter certainty.

This establishes display and meter behavior for the listed source cases, not execution support for every represented ability. Core dispatch admission still owns mechanical coverage. Real-window HUD clipping, input and the parent's Rest-loop interruption behavior require integration tests; status projection alone is not that validation.

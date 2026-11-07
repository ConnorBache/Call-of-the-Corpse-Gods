# Revision32 native balance audit harness

`crates/corpse-core/examples/physical_trade_arena.rs` executes the real dispatcher,
not a DPR approximation. `tools/monster-balance/compare_native_roster.py` pairs
identical actual-generator joint player profiles and combat seeds for r31/r32.
The prescribed player starts fully healed with a Shortsword and no optional
blessing abilities or equipment. Profile origin/track/hash accompany each row.
Primary references are the earliest approved natural/conditional appearance.
Separate midpoint/last-window sensitivities do not retune the primary target.
Half-floor diagnostic references alternate the two neighboring entry-layer cohorts.

## Reproduce

```sh
sh tools/cargo.sh build -p corpse-core --example physical_trade_arena --locked
python tools/monster-balance/compare_native_roster.py \
  --binary target/debug/examples/physical_trade_arena \
  --profiles tools/monster-balance/baseline-player-progressions-r32-joint-profiles.json \
  --cases tools/monster-balance/approved-native-introduction-cases.json \
  --trials 128 --output tools/monster-balance/native-roster-before-after.json
```

The exact generated baseline filename depends on the cohort receipt. Use the final
r32 receipt for r32 placement comparisons; an r31 cohort is only a preliminary
same-player stat comparison. Do not relabel preliminary measurements as final.
Errors and nonapplicable rows remain in the output rather than disappearing.
Optional `--before-profiles` compares r31-generated vs r32-generated player
cohorts after intersecting seed identities; that is a separate overall-placement
comparison, not the same-player primary audit. Candidate fitting may explicitly
supply `summoned_candidates` (child kind → numerical candidate); new children are
patched after native birth and before future actor phases, preserving incidental
HP loss fraction. Final shipping measurements omit all candidate overrides and
therefore exercise the shipping constructor/overlay only.

## Metrics and scopes

- Primary `mean_gross_combat_damage_fraction` is the average of **each trial's**
  incoming native damage packets through the END of the dispatch containing the
  primary creature's death, divided by that trial's initial HP. Same-command
  status ticks remain part of this explicit boundary. It is not divided by mean
  HP. Raw packets can overkill and can exceed 100%. An opt-in thread-local observer
  at `apply_hp_loss` captures every packet after immunity/mitigation but before HP
  capping, including unlogged Ruin Pulse/attack-channel spells.
- `mean_gross_damage_fraction` adds subsequent postdeath exposure. The separately
  reported `mean_gross_postdeath_damage_fraction` is the subsequent-dispatch tail;
  `mean_status_channel_postdeath_damage_fraction` isolates logged Venom, Poison,
  Fire, Bleeding and Blood loss channels. Fire can be refreshed by terrain, so the
  status-channel figure is not claimed as a pure causal attached-status estimate.
  Channel logs otherwise remain diagnostic; only the native observer is the
  complete incoming-packet total.
- `mean_capped_actual_damage_fraction` uses the same native observer's
  actual HP removed per packet, including multiple attacks and extra slow world
  turns. Capping each packet does not cap the entire fight at 100% if healing
  occurs. Commands capture distinct batches, so haste does not double count.
- `mean_cost` is final net HP loss after regeneration/status settlement. It is
  separate from gross damage and is **not** the primary balance target.
- Player attack attempts count hits **and misses**; landed attacks are separate.
  `mean_attacks_to_kill_conditional_on_kill` and world turns to kill summarize only
  actual kills. When death/censor rate is nonzero they are not an unconditional
  expected lifespan. The player is never resurrected or secretly made immortal to
  finish a lethal sample. Terminal attempts, death rates and censor rates remain.
- Adjacent scenarios start the hunting monster one tile away, with the player
  attacking first. `chase` starts four tiles away and follows a stable shortest
  legal route in a 9×9 lit room. The AI still retreats, casts, moves and recovers.
  This is a reproducible approach cost, **not optimal player tactics**. Water/ichor
  cases preserve required habitat. Rooted/invulnerable/environmental actors with
  no approved one-to-one target receive explicit N/A rows.
- `three-cast-children` invokes the actual Birther casting path exactly three times
  at setup ticks0,10,20 with native energy/cell checks. No intervening world turns
  are claimed. It then removes the parent and pits the full-health player against
  the first-generation children and any descendants. It measures the combined
  **isolated summon payload**, not twenty turns of exposure to the summoner, and
  can be lethal/censored. Ordinary summoner scenes remain distinct.
- `support-with-fighter` places a requested `partner_kind` frontliner between
  player and support. The player defeats the frontliner first, then chases the
  support. Pair `without_support: false/true` with identical player/seed and
  partner stats to measure the realized support increment. This is a bounded
  cooperative benchmark, not an isolated support DPR target.
- The legacy `wait` settlement is stationary-room-hazard stress. Opt-in `escape`
  settlement uses a stable BFS to the nearest reachable native-hazard-free cell,
  replanning after every real command. It stops once the player is locally safe
  and attached harm has ended; remote remaining fields are explicitly retained in
  output. No reachable clear cell produces an explicit trapped aftermath censor,
  not a claimed complete lifetime. This is bounded-room counterplay, not optimal
  floor navigation. A surviving additional actor after the primary target dies
  is an aftermath censor. `combat_censored` and `aftermath_censored`, and combat
  versus residual deaths, are separate. `all_primary_lifespans_observed` requires
  all primary targets killed; `all_settlements_observed` additionally requires no
  aftermath censor. No claim
  is made that the arena covers allies, kiting across a floor, terrain counterplay,
  multi-room pursuit, optional loot or all emergent encounters.

## Complexity (analysis-only)

Let T≤1,000,000 trials/request, C≤100,000 combat commands plus S≤100,000 settlement
commands/trial, V=81 arena cells, A live/dead actors including generated summons,
K copied actor-state keys and E emitted log strings. Each recording step costs
O(A²+K+V+E), plus the real dispatch's validation, pathfinding, visibility, terrain,
status and native-library costs. Chase and escape BFS add O((V+grid edges)×H+A) path construction, with H the
native passability/hazard helper cost (fixed catalog, no grates and fixed starter
loadout in these arenas),
per command. Summon growth is not capped or stripped by instrumentation; the
finite room bounds simultaneous occupancy but not historical actor records.
The disabled damage observer has O(1) branch overhead per packet and no
allocation; enabled command batches append O(P) packets in O(P) time/memory,
without RNG use or authoritative state mutation. Outcome storage is O(T); per-command traces, when explicitly enabled, are
O(T(C+S)(A+K+E)). Normal runs disable traces. The driver stores O(roster×revisions)
summaries and O(T) request profiles, running one native process with incremental
JSON/CSV checkpoints. Timing is not inferred from these asymptotic bounds.

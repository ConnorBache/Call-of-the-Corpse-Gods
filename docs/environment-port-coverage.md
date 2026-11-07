# Environment port coverage (frozen v48)

This remains a partial whole-game migration. Admitted native environment paths have executable turn integration and deterministic reference tests. Engine trait/cross-floor gates are separate and must not be relaxed merely because a helper exists.

| Frozen source | Native location | Evidence / remaining scope |
|---|---|---|
| brogue-propagation.ts | propagation.rs | All gas/fire exposure/scan/timed-promotion kernels translated; 64 gas + 64 exposure differential cases compare RNG and events |
| propagation-state.ts | environment.rs | Canonical normalization and compatibility views; exercised by full-state fixtures |
| propagation-adapter.ts | fire_adapter.rs + environment.rs | Source-layer promotion, gas closure, per-reaction shared hit pools, complete gas owner/player/vent provenance; full-state flame/explosive/hydrogen fixtures |
| gas-reactions.ts | gas_reactions.rs | Ranked union with minimum public identity; unit + full-state reaction fixtures |
| environment.ts | environment.rs | Normal gas/fire contact, exact DR-bypassing damage, direct ignition/blasts, roots/vents, water/item drift, Entropy/Star Power/frozen fields, Mercuria flood/fish, razorleaf emitters, Breather emission, Black Pit restoration. Half Here provenance and ordinary per-body versus gas per-owner exposure integrated; eight shared-health/death-closure vectors pass, overall scheduler admission coordinated separately |
| doors.ts | environment.rs | Open/block/settle helpers, burn removal |
| currents.ts | currents.rs + environment.rs | Exact generation and destination; normal-turn actor push hook, snapshot occupancy; generation differential fixtures |
| water.ts | environment.rs + inventory.rs | Swimming/flight gate, weighted item floating, ground drift integrated; cross-floor placement/relocation composes with authoritative item_placement and ensure_floors, verified by inventory/travel fixtures |
| fungi.ts, vents.ts | fungi.rs + environment.rs | Exact conduits/emission/sever/root query/countdown/pulse/clearing; three-tick full-state scenes |
| corpses.ts, burning.ts | corpses.rs | Chance/roll/reset/revive/timers; three-tick corpse burning and offscreen age/roll fixtures |
| hazard-memory.ts | hazard_memory.rs | Full pure cell snapshots and observed-cell commit, including remembered root orientation and Godbeast fixture descriptions; scene differentials |
| hazards.ts | environment.rs | Pure creature routing hazard predicate; interactive user-facing walking/wait warning UI not ported here |
| offscreen.ts | offscreen.rs | Exact source clean-floor stationary approximation (ordinary actors/rest/revival), precise empty-floor hazards, untouched Arc preservation. Precise ordinary hunting, confusion, bleeding, timed statuses, slow/fast/flight/minotaur and leaves/fungus/permanent-burning hooks included. Mercuria lifecycle, stationary ally rest/queues, deferred historical hunter transfers, and scheduled flight-expiry falls integrated. Other trait/status callbacks remain explicit errors |
| godbeasts.ts flood, mercuria-creatures.ts spawn | ichor.rs | Exact frontier/openings, finite roster selection/spawn, revival transform; 40-depth actor fixtures plus three-tick flood/spawn scene. Habitat/stranding/siphon hooks integrated into normal engine and tested with24 dispatch commands |
| monster-statuses.ts, monster-lifecycle.ts | monster_statuses.rs + lifecycle.rs | Exact status application, Hemophilia replacement, poison packet, regen multiplier, Banshee clock/alarm and Ursa death cry helpers; eight full-state/event vectors. Normal scheduler composition verified with 80 dispatch commands including Banshee, Troll and Ursa; offscreen status clocks integrated |
| terrain.ts generation helpers | generation worker | Separate ownership; no replacement terrain rules introduced here |

## Verified tests

- 64 volumetric kernel cases and 64 fire exposure cases, generated from frozen compiled v48 reference.
- Complete-state static environment update.
- Twenty-two active three-tick scenes: burning, Sparkle, caustic, explosive, hydrogen, vent, water, fungi/corpse, Reverse Entropy, Entropy, Star Power, multi-target deaths, Mercuria flood/fish, razorleaf emitter/contact, hydrogen/steam/confusion vents, Black Pit tile restoration, Breather passive emission with vulnerable target, Half Here fire/gas provenance and shared exposure, stationary chasm item preservation.
- Eight full-state/status-event vectors for Hemophilia (including forced movement), poison remote attribution, Banshee duplicate/LOS/proxy guards and Ursa companion wake.
- 24 complete normal-dispatch ichor commands with exact presentation output; temporal internal history/haste snapshots use the separate temporal suites.
- 120 complete fish actor fixtures across all 40 depth bands and three selection quantiles.
- Hazard-memory/current generation outputs for those scenes.
- Twenty-three ordinary explosion variants, with exact ordered presentation events: ignition-first/after, area ignition on/off, connected gas, multi-target deaths and Half Here shared-owner survival/lethal dismissal, Nova passive/active immunity and retained dismissed endpoint-source provenance.
- Sixty offscreen cases: regen/rest, ordinary and already-mature corpse revival, empty dirty gas/fire, hunting, confusion/bleeding, paralysis, sticky movement, dirty corpse revival/death, slow/fast/flight/minotaur hooks, Black Pit tile restoration, leaves/fungus/permanent burning, Mercuria/stranding/revival, hunter transfer/legacy/blocked/direct, ally rest/arrival/held queues, flight-expiry falls, poison/Hemophilia clocks including poison death, regeneration, buff expiry/death, remembered spells, environmental/vision upkeep, Reverse Entropy frozen actors, venom packets, Pale passive/energy traits, slime-walker/venom, Ursa death/wake, vampire forms, Phase wall traversal/crushing, Torrent/Black Pit upkeep, hidden target-link death and crossfloor carried-item/corpse death, Wendigo feeding/retreat, Inventor mounting and piloted Mech, passive Entropy/Star Power/Breather precise clocks, plus untouched floors. Assert Arc identity of untouched blocks.

Exports live in tools/export-*-fixtures.mjs and are development-only oracle scripts. Native runtime never invokes TypeScript or Node.

## Complexity

V = cells, F = floors, A = actors, L <= 8 layers, T = tracked vents, P = root/pulse entries, B = consumed explosive cells, R <= 9 burst radius, D = doors, O = occupied cells, G = grates, K = reaction IDs.

- Gas kernel: O(V) time and auxiliary space, bounded ordered neighbors. Two-pass provenance/vent adapter O(V*T + P), O(V + P) auxiliary memory.
- Fire kernel: exposure O(L); scan O(V) plus callback work; promotions O(V*L) time/pending storage. Adapter caches active floor index, with O(F) one-time lookup, so hot per-cell access does not scan all floors.
- Closure: O(B*R²*(V + ray length) + V*alpha(K)) currently. The visibility helper rebuilds an opacity vector per ray; this is accounted for, not hidden as constant-time LOS. Extra working memory O(V+B+K). Future optimization must invalidate visibility when burning removes doors/trees within a closure.
- Reaction union/find amortized inverse-Ackermann, O(K) metadata. Hash-map iteration never determines event order.
- Source deposit O(GasKinds) after initialization; gasKinds=8. First initialization/migration O(V*GasKinds).
- Fungi: O(fungi*corpses*(V+range) + P) because current LOS helper allocates opacity per query; O(V+P+A) temporary space. Conduit range <= 11 cells. Vents O(T+P) plus emitted initialization and visibility helper costs.
- Door settlement expected O(D+O), O(O) occupancy. Current destination O(G) grate scan; generation O(V*G) as implemented (source optimizes grates to a set), O(V) storage. Actor current push O(A log A + A*G + movement hooks), O(A) storage.
- Hazard context O(P + vents + lights + permanent fires); cell snapshots O(1) with that context except bounded gas catalog. Full commit O(V+P+features), sparse output allocation. Single-cell convenience function rebuilds the context; batch callers should reuse it.
- Corpse timers/chances O(1); revival O(condition count). Clean offscreen catchup O(V + 298*A) plus stat/rest helpers and COW touched actor block. Empty dirty floor catchup runs exact environmental work for each elapsed tick, then collapses only after source dynamic criteria clear. Proxy clone shares Arc floor/actor/ground blocks; other top-level metadata can still clone, so proxy creation is not claimed O(1).
- Ichor flood visits each reachable cell once per closed topology, O(V) frontier/visited storage; reopening adds explicitly enqueued cells. Successful fish spawn O(A + 9 + visibility), at most ten spawns; fixed nine-entry catalog lookup is bounded.
- Razorleaf rebuild O(E*121*(V + ray length)) with current visibility implementation, E emitters; O(V + leaf cells) map/storage. Precise offscreen actor pathfields use O(V) per distinct destination/capability signature, then run exact tick environment; no elapsed-time cap skips dynamic rounds.
- Runtime writes use Arc::make_mut. Unchanged floors retain pointer identity; first writes copy touched blocks. Offscreen elapsed time is not permission to freeze dirty floors.

These are asymptotic implementation bounds, not measured speed claims. No full-game or Windows readiness claim follows.

Status helpers run O(A across stored floors) worst case for source-ID lookup; application is O(condition count) hashing/serialization-state updates. Banshee world pass is O(A) plus each actual shriek LOS; no per-tick timer increment or RNG is introduced. Deferred transfer resolution uses stable IDs and at most nine landing candidates, with explicit COW of changed actor/floor blocks.

Remaining offscreen dependencies are explicitly gated: advanced divine upkeep for Bonded and Wheel, unsupported specialist AI traits, grabbed/raised-wall lifecycle states. Half Here proxy-floor composition needs separate scheduler admission and tests. Ground-item cross-floor placement composes with verified inventory/travel APIs; stationary items are intentionally not normalized every environment tick.

## Advanced divine boundaries

`divine_environment.rs` provides Black Pit departure (original-tile capture and four substrate clears), Riptide forced movement, Drown regeneration over both owner endpoints, Reverse Entropy hostile-healing/corpse predicates, and exact Phase loss ordering (shared-owner crushing, clone fall, player fall or pit victory). Activation and paid upkeep remain in abilities; normal/haste scheduling and attack hooks remain in engine/combat. Phase upkeep is 5, Torrent 15, Riptide 10 after 15 initial activation, Black Pit 5; Drown/Reverse Entropy/Shine have zero upkeep. Offscreen charges retain their separate source ordering and no invented resource-history logging.

Black Pit edits O(1) cells/map entries plus first COW floor detachment; Riptide costs O(A) target lookup plus shared movement callbacks. Healing and phase endpoint predicates inspect at most two endpoints, plus O(A) linked-clone lookup. Phase-loss settlement adds resource history/death/floor-transition work and does not hide generation or catchup as constant-time. Source fixtures cover eight phase-loss boundary states; Seven additional compound dispatcher scenes (28 commands) verify Half Here swaps and Phase crushing, Black Pit substrate/restoration, endpoint-aware Drown/Reverse Entropy, and Riptide forced rooted movement. Ability-worker command suites cover activation/payment/expiry boundaries separately.

Hidden-floor proxies commit the whole Arc-backed floor/actor/ground collections because death/drop callbacks can touch deeper floors. Untouched Arc blocks keep identity. Actual-world maintained-link cleanup runs after commit for newly dead/removed targets; hidden proxy logs are not copied except authored explosion sounds. This commit is O(F + actor-map keys + ground-map keys) ownership movement, with death-target verification O(A across stored floors) time and temporary ID-set memory per precise tick, followed by maintained-link cleanup for actual death IDs only.

Retained-source blasts expose `explode_with_source_snapshot`; they capture provenance before any victim callback, and never temporarily insert removed actors into occupancy. Default blasts also retain initial source identity. Damage/reaction callers must prefer live stable-ID relookup and use the retained source only when that endpoint no longer exists.

Latest handoff dependencies: Dream Craft run-aware flight/contact conversion is owned by abilities/inventory, with engine/AI owned by combat. Body-only flight helper cannot prove equipped-Dream behavior until those callers and fixtures compose. New feature owners may expand explicit ownership gates after exact source tests; unsupported offscreen traits/actives and inactive shared-endpoint states continue to reject.

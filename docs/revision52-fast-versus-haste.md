# Revision 52: Fast versus Haste

## Designer instruction and scope

“Make it so that the game communicates that a creature is fast, not hasted. Fast means it can move two tiles. After the first move, it can attack (this means stuff like cave bats should catch up to you and start hitting you as you run.) Hasted means it takes 2 turns.”

The previous scheduler classified innate Fast and Vampire Bat form as Haste and ran the bonus as movement-only before the player's action. A pursuing bat could fail to move onto the player's still-occupied square, then spend its normal action catching up after the player moved. Genuine Haste was also movement-only rather than a complete second action.

Revision 52 separates these mechanics. Earlier simulation revisions retain their historical predicates, action order, movement-only Haste and snapshot behavior.

## Current behavior

- An ordinary Fast NPC action may move twice, or move once and make its ordinary melee attack. Beginning adjacent and attacking consumes the action, with no extra Fast attack. A failed, held or blocked first movement does not earn a second attempt.
- Fast is resolved within that NPC's action, after the player's ordinary action. It no longer participates in the earlier Haste pass. The existing Minotaur regression still proves the player can hit the displayed Minotaur, or kill it, before its normal Fast movement. A full Minotaur action owns both committed movement steps regardless of the player's incoming phase; a player's Haste-only action no longer moves a non-Hasted Minotaur.
- A genuinely Hasted actor gets two complete actions per world turn. Its bonus action can attack, use an ability, move or wait. World-clock status expiry, regeneration, hunger, feeding progress, currents and cooldown deadlines do not gain another tick.
- Fast applies within each complete action of a Hasted creature: at most four moves, or two move-then-attacks. This natural composition was the implementation assumption approved during coordination, not a separate verbatim designer statement.
- Slowed suppresses both bonuses and retains the existing alternate-normal-turn NPC action rule. Innate Slow continues suppressing bonus phases and acting on alternate normal turns. No new cancellation/stacking balance rule was introduced.
- Vampire Bat form retains its innate movement benefit and is shown as Fast, not Haste. A Vampire that first transforms during its action still moves once on that transformation action, preserving the old timing; subsequent complete bat actions receive Fast. Eligibility is checked before and after the first action, so losing the form does not create a continuation. Swimming and other actual Haste sources remain Haste. Swimming eligibility is sampled from the departure contact tile, including for Wait; it does not grant a mid-action bonus merely by entering water.
- NPC Fast continuation does not cast an additional spell or repeat recovery/rest/perception. It follows the existing specialist AI movement direction, pursuit/rally/stalking target, terrain, occupied-cell, portal, habitat, grip and status checks. Attack-related forced movement does not qualify as a first Fast move.
- The player's existing ability to move once using Fast and then issue a normal command is preserved, including an attack or ability. Fast alone does not grant a second action after an initial attack, ability or Wait.

## Player input and save state

The existing `phase` field continues to describe `haste` versus `normal`. The optional typed `fastActionPhase` field is written only after a successful first Fast movement. Its values are `haste` or `normal`, naming the complete action being continued; while it is present, `phase` is `normal` so a continuation never reruns the NPC Haste pass or resource-turn opening.

- Fast plus Haste, four successful movement inputs: save Fast(Haste), complete Haste, save Fast(Normal), complete Normal and advance the world.
- Haste without Fast: one full bonus input, then one full normal input and world tick.
- Fast without Haste: save Fast(Normal), then complete Normal and world tick.
- An initial attack, ability or Wait consumes its complete action without creating a Fast marker. A rejected command preserves an already-earned marker. Any actual world tick clears it.

Missing/null markers preserve older saved in-progress phase meanings without inventing an earned continuation. Invalid marker strings fail typed decoding; a present marker on a pre-52 run or without `phase: normal` is rejected by validation. The marker is copied by command rollback, snapshots and native Future anchors, included in snapshot optional-root accounting, and omitted when missing. Historical inverse patches do not acquire a new pre-52 removal entry. Revert remains unavailable under revision 41+ rules; this change does not restore it.

## Active and inactive floors

Active creatures share one complete-action body between ordinary and genuine Haste opportunities, with normal-only rest/corpse clocks explicitly gated. Fast has a bounded move-or-melee continuation. A command-local receipt records the acting creature's voluntary movement and attempted attacks, independent of presentation or visibility; it cannot persist beyond the synchronous action. This prevents an Unstoppable Force advance following an attack from granting a second Fast attack.

Exact inactive-floor simulation retains its own no-wandering/no-proxy-targeting behavior, world clocks and pending transfer ownership. Its complete Haste action uses recorded targets and chase destinations; reaching a stair edge does not discard the pending chase. Actor IDs are resnapshotted after the bonus pass because mounting can remove an inventor and introduce its nested pilot state. The existing stationary aggregate approximation is unchanged.

The active Haste scan stops on terminal state or depth change and tolerates actors removed by earlier actions. A lethal NPC Haste attack cannot then execute the dead player's requested command.

## Presentation and disclosure

Observed native creature status labels expose Fast separately from Haste. Trait descriptions and the revision-aware status-description helper explain two-tile/move-then-attack versus two complete actions. The helper preserves legacy copy; this change does not add a newly wired native hover interaction. Existing observation and Bestiary admission boundaries remain unchanged; this work does not reveal hidden actors or their current powers. Desktop travel and hazard projections are updated by their owning subsystem to understand the four player stages and departure-tile Swimming.

## Complexity

Let A be current-floor actors, B an actor's abilities, V floor cells, P the existing path query cost (including allowed/hazard mask construction and distance-field builds on cache miss), D target detection/visibility/stat query cost, and C the existing combat/special-action cost. Counts are bounded by the existing floor and actor limits; no new unbounded action loop is introduced.

- Each complete NPC action performs the existing AI work. Fast adds at most one continuation: O(B + A + P + C) worst case, with existing specialist/visibility helper costs included. Its target/rally checks are scans; it does not rerun stochastic perception. A genuine Haste pass remains O(A log A) stable ID ordering plus up to A complete-action costs. Fast+Haste has a constant maximum of four movement attempts/two ordinary attacks, not a new asymptotic multiplier.
- Active actions reuse the existing phase path cache. Exact offscreen normal Fast continuations currently use a fresh bounded path cache per actor, so a continuation can pay the full mask/distance-field miss cost P and O(V + portal links) scratch. The existing offscreen occupancy reconstruction costs O(A) after movement. No claim of constant-cost pathfinding is made.
- Receipt bookkeeping is O(1) per action/move/attack and O(synchronous nested action depth) transient memory. It performs no RNG, serialization or presentation query and cleans up on errors/unwinding.
- Player phase accounting is O(B) ability/predicate work plus existing conditional-Haste detection cost D; the optional enum adds O(1) save/snapshot metadata. Four input stages do not add world ticks or duplicate resource clocks.
- The UI label change is O(B) per admitted observation. Route/proof graph state expansion and UI prediction costs are documented with those subsystems.

These are source-derived estimates, not benchmark claims. Final test and full-world/desktop receipts are recorded by the integration task after all source changes land.


## Focused verification receipt

Cloud Linux, pinned Rust 1.99.1, shared target directory; these focused runs explicitly set dev/test opt-level 1 and retained default test debug information, assertions and overflow checks. They are not the later integration profile (debug 0, incremental 0, jobs 2), Windows verification, or a complete-workspace pass.

- `revision52_fast_haste`: 12 passed. Five consecutive fleeing-player commands each require exactly one Cave Bat melee impact after catching up, including legitimate misses; this does not rely on HP deltas that regeneration could hide. Covers full Haste, per-action composition, Slow/Slowed, failed/held moves, save/resume and Snapshot at each phase, actual Engine See The Future capture/restore, invalid marker validation, Chill ordering and lethal pre-player Haste.
- `revision52_minotaur_order`: 5 passed, including player attack before the normal Fast action and no movement after the Minotaur is killed.
- Core `revision52_speed` unit filter: 4 passed. Exact inactive-floor four-move composition with one hunger/status tick, inventor mounting/removal, pending stair transfer, and direct action-kernel attack/forced-motion receipt. The last fixture intentionally calls the kernel because ordinary NPC admission rejects synthetic borrowed divine powers; admission was not widened.
- `check-snapshot-roots.py`: passed, 26 optional roots.
- Unmodified `revision52_generated_avatar_smoke`: 2 passed, 1 failed. The 1/4/8 seed died after 47 completed commands at tick 41, (27,25), final blow Hemogoblin, with pursuing Cave Bats also attacking. This is reported to integration as a real changed survival trajectory; no runtime or seed alteration was made to satisfy the bot. The other two cases completed their generated gameplay/save-resume/seed-replay checks.
- Desktop Fast status/copy tests are staged for the integration owner, not run in this receipt.

Retained logs: `verification/revision52-fast-haste/`. Initial failed fixture/test attempts are preserved with the final focused logs; they are not relabeled as passes.

## Portal and special-tile movement receipt follow-up

Final review found that the player's initial Fast test reused `CommandMoveTrace.edge`, which deliberately describes only local Timeweave-compatible movement. Portal traversal and staircase/chasm/pit endpoints can commit movement without creating such an edge. The command trace now records a separate, transient `voluntary_move` bit at the actual non-forced coordinate commit. The original Timeweave filters, trail updates and free-reverse probes are unchanged.

A same-floor portal traversal, top-floor up-stair step, or flight over chasm/pit can therefore earn the normal Fast continuation. A completed floor transfer, death or victory cannot create new Fast credit. Attack-related displacement, forced movement and unsuccessful movement remain excluded. This adds only O(1) command-local bookkeeping, with no serialized field, RNG change or additional movement algorithm.

Four regression cases were added: paired portal movement with/without Haste and Weave plus exact save/resume; same-floor stair/flying special-tile entry; floor transfers and terminal arrivals; and failed/forced/attack displacement. The narrow source change was independently reviewed. Final command results are retained in `verification/revision52-fast-haste/portal-stair-integration.log` and `timeweave-kernel.log`; the exact shared build profile is recorded in `PORTAL-STAIR-PROFILE.txt`.

The follow-up completed under the exact shared profile: **16 Fast/Haste integration tests passed, one Timeweave seed-replay test passed, and all 31 Timeweave-filtered core unit tests passed**. The first follow-up attempt's sole failure was fixture admission: persisted portals require depth 10 or deeper. That fixture now supplies ten complete floor/actor/ground blocks and validates before its first command. Its first failure is retained as `portal-stair-first-fixture-failure.log`; no runtime behavior or validation rule was weakened to make it pass. Core/runtime source was frozen again after these results.

## Legacy hover-copy correction

The combined desktop check caught a presentation-only regression: the new complete-action Haste description was also being used for old revisions. The read-only hover formatter now takes the simulation revision explicitly and enables the Fast/Haste explanatory text only at revision 52 or later. Earlier revisions retain the frozen generic status copy, including ongoing, singular and plural durations. Status observation and scheduling predicates are unchanged. Compile21 passed all 15 status tests, including the revision-boundary regression and the unchanged 185-case legacy fixture, plus all 11 creature-ability tests. Receipts are verification/r52-avatar-world-focused/statuses-21.log and creature-abilities-21.log. The later frozen full-workspace gate remains the release authority.


## Generated Avatar terminal-outcome verification

The later generated-play suite preserves that genuine changed outcome instead
of changing the seed, planner or combat. Two cases complete 64 public commands
and acquire real generated loot; the middle retreat case stops after 47 commands
at its actual death. All three verify exact midpoint continuation and complete
seed replay through their actual final command, with no post-death commands.
The minimum movement/tick gates remain. See
verification/avatar-generated-smoke-review/receipt.json and README.md; the final
frozen-source run exports fresh fixtures under
verification/r52-avatar-world-final/generated-avatar-final/.

## Dragon landing and Duel aggregate correction

An already-paid Dragon landing is the narrow exception to the generic sleeping
Fast-continuation return. It may complete its second landing movement while
resting, both active and offscreen, without new takeoff, wake or attack. Dragon
planning now compares complete-action costs using a two-stage Fast graph and
accounts for the current genuine-Haste bonus/normal action. The phase tag lives
only in the existing synchronous receipt scope and is cleared on normal return,
error or unwind; no saved state is added. See the Dragon subsystem note for the
additional bounded minimax pass and asymptotic/helper costs.

The Duel cleanup failure was a target-choice fixture issue: genuine Haste now
runs perception, and equidistant hostiles let it choose the player instead of
leaving the Duel. The fixture makes its intended ally strictly closer and retains
an exact departed position, immediate cleanup, unchanged upkeep and replay
comparison. No Duel lifecycle or targeting rule was changed. The correction was
applied and the shared-profile core library run passed 717 tests (four ignored).
The separate arrow and corpse-visibility checks passed two and one tests,
respectively. Receipts are under `verification/r52-aggregate-corrections/`.
The later five-move flight-budget boundary regressions are a separate change;
they are not covered by those results and are awaiting focused execution. The
final frozen full-workspace gate remains the release authority.

The Dragon safety-cap follow-up is a constant admission correction: six flight
ticks minus the next upkeep and the existing one-tick landing reserve allow at
most four movement attempts, without relying on Fast or Haste. Price, duration
and active pursuit are unchanged. Five-move rejection and four-move successful
landing regressions use actual dispatch for both Fast and non-Fast bodies. The
old inconsistency was source-traced, not run as a failing baseline; corrected
focused/full-core execution passed: 719 tests, zero failures, four ignored.
See verification/r52-aggregate-corrections/core-02.log. The final combined
workspace gate is recorded separately under verification/r52-avatar-world-final/.

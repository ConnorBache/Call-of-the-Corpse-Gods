# R50 control amulet implementation checkpoint

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

This is source work behind the closed R50 release gate. No normal new-run or
release admission is enabled here. The old simulation revisions retain their
existing paths and carry no new required serialized fields.

## Authored effects

- Beckoning (15 energy) selects one visible same-plane creature within 8 tiles.
  It plans the nearest empty adjacent destination and a clear, collision-safe
  pull before resource payment. It uses ordinary forced movement and arrival
  hooks, including Roots release and Fated Duel departure. Redirecting portal
  steps are rejected rather than rerolled or redirected to an invented endpoint.
- Discord (30 energy) targets one visible creature within 8 tiles for 6 normal
  turns. Temporary mutual hostility is an overlay; permanent state, allegiance,
  perfect-ally flag and orders are retained. Normal and haste actor phases and
  precise inactive-floor phases use the same overlay. Haste is movement-only
  and never reduces the duration. Attack protection, enemy action eligibility,
  player collision, Sphinx provocation and healing/scream alignment use the
  temporary hostility relation. Direct spell damage respects Fated Duel; new Beckoning, Discord and Roots impacts reject a protected target before payment. Indirect hazards and existing Roots keep their existing behavior. No permanent conversion is performed.
- Obstruction (25 energy) places a perpendicular three-tile wall, with unlimited
  center distance and a currently visible center. Full-footprint terrain and occupancy are validated atomically, but unseen outer cells do not add a visibility restriction. Creatures, stairs,
  portals/spatial links, existing solid terrain, authored terrain modifications,
  doors and overlapping temporary edits are not replaced. The exact grid
  orientation follows Cogworks: cardinal/exact diagonal aim. Other slopes
  remain outside this existing grid-orientation target contract.
  The wall blocks movement, including phase-through, sight and projectiles.
  Its three cells expire after 8 normal turns and restore their predecessors
  only if still owned by this temporary edit.
- Roots (20 energy) targets one grounded visible creature within 6 tiles for 4
  normal turns, doing 3 damage each turn. It blocks voluntary translation only;
  attacks and abilities remain available. Forced movement/teleporting and fire
  break it. The periodic packet is indirect damage with its stored source, not
  a direct hit for Reaping.

Teleportation/Floods live in `revision50_wave_teleport` and share the dispatcher,
arrival/Roots and temporary-terrain boundaries. Floods uses the approved policy:
each lane independently stops at existing water. Its preview and execution use
the same policy.

## State, failure and verification boundaries

The shared amulet dispatcher owns energy admission, action credit and atomic
command rollback. Effects never invent capacities, cooldowns, partial-payment
policy or Reaping allocation. Target failure precedes payment and world time.
UI checks use observed terrain/visible creatures and never reveal an invisible
occupant through preview shape or blocker prose.

`revision50TemporaryTerrain` is an optional validated Floor field, retained by
raw serialization and existing snapshots. Each record stores cell, predecessor,
replacement and absolute normal-turn expiry. Actor control conditions are
revision-gated. Temporary terrain causes exact offscreen catch-up, including
expiry of an inactive dream plane, rather than being skipped by stationary
catch-up. Conditional restoration does not erase an independent later edit.

Eighteen focused control regression tests cover successful/blocked Beckoning,
perpendicular/unlimited Obstruction, collision and expiry ownership, invalid
placement, raw save/snapshot restore, mutual Discord and allegiance restoration,
haste duration, Roots movement/damage/fire, plane/range/revision gates and hidden
occupant UI invariance, center-only visibility with an unseen outer wall cell, exact inactive-floor expiry action-free resource accounting, direct-spell Duel protection, Discord curse Willpower, last-paralyzed-turn offscreen ordering, and source-scoped control admission under Fated Duel without removing existing Roots. These are authored tests, not a claim that they ran;
consult the current verification receipt for actual command results. Release,
Windows visual acceptance and campaign balance remain unverified.

## Complexity

Let V be floor cells, A live/current-floor actors, P spatial links, T temporary
terrain records, M existing terrain modifications and C conditions per actor.
The floor is bounded by existing generation; each Obstruction adds exactly 3
records and each wave at most 15, with no overlapping records. T is at most V.

- Beckoning planning tries at most eight destinations, each with a straight
  path of L cells (bounded by the floor): O(8 L (A + P + M)) plus existing passability,
  visibility and spatial-distance queries. Scratch is O(L). Execution performs
  L existing movement/arrival calls, whose Grappler, travel, contact and death
  work is additional, not constant-time geometry.
- Obstruction geometry is O(1), three cells. Admission includes observation,
  occupancy O(A), spatial-link lookup O(P), and existing modification O(M) work
  per cell. Commit and memory are O(3); COW floor detachment is O(V + floor
  metadata) if the current floor is shared. Ordinary movement checks scan T
  records; pathfinding includes this O(T) mask cost per examined cell.
- Terrain expiry scans O(T) per normal turn and retains O(T) temporary scratch.
  It triggers COW only when a record is due. Offscreen admission also scans
  floor/dream-floor collections and their T records; exact catch-up costs scale
  with elapsed turns and the existing actor/environment costs per turn.
- Discord selection is O(A) candidate scanning with visibility and portal
  distance work per candidate; the no-Discord fast rejection still scans actors in O(A) per actor phase. The active overlay retains only one condition
  per affected creature and uses O(A) worst-case existing selection/path scratch.
  Each executed turn may invoke existing ability/combat/pathfinding, including
  their terrain masks, portal graphs and cache misses. No performance claim
  treats those helpers as O(1).
- Roots bookkeeping is O(C) for ordered-condition removal; each normal tick also
  executes the existing damage/death/provenance path. Admission adds visibility,
  spatial distance and grounded-trait queries. No per-frame simulation scan is
  introduced. UI adds existing observe-terrain/visibility cost and O(3) geometry.

All timings are unmeasured. Profile representative crowded floors with multiple
spatial links, maximum temporary cells and offscreen catch-up; separately record
cold COW/visibility/path cache costs and warm steady-state work.

## Focused verification receipt

The combined core `revision50_` unit run recorded in
`verification/revision50/amulets-combined-focused.log` passed the 16 control
regressions included in that compilation. The seventeenth, source-scoped control
admission under Fated Duel, was added afterward and awaits the aggregate run. That run had 113 passes and one failure in the newly added
Teleportation synthetic-Clay fixture: top-level validation correctly rejected an
ephemeral Clay attacker marker. The fixture was corrected to expect that
validation error while separately checking effect admission and unchanged
resources. The subsequent player-source/proxy attribution alignment and that
fixture correction require final aggregate verification; this
receipt is not a full-workspace or release claim. No Cargo commands were run by
the control-effect edit concurrently with the test process.

The later full-workspace pass exposed a fixture interaction after the Roots
source/proxy correction: Roots correctly provoked its neutral target under the
ordinary player-damage rule, while the combined expiry fixture expected it to
stay neutral. That fixture now gives Discord to an undamaged neutral and Roots
to a separate hostile, with both paralyzed to isolate the expiry clocks. The
neutral-allegiance assertion is retained. A separate regression requires Roots
to provoke neutrals identically on-screen and offscreen. These narrow test
changes leave production provocation and Discord semantics unchanged and await
the next scheduled aggregate run.

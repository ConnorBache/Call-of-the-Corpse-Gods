# Revision 51 confirmed ability rules

Authority: designer's 2026-11-07 correction: I Came For You has no turn limit;
Sparkle has no combustible-terrain targeting limit and its fire goes out on
water contact; restore All-Seeing Eyes' duration and Shine's active suppression.

## Executed rules and compatibility

- I Came For You already has no expiry in revision 50 and later. `cameTurns=1`
  is an enabled sentinel, not a turn remaining. Keep that implementation, its
  restored 10-energy reserve-first activation, target telepathy, bonus movement,
  and existing target-death/ability-loss cleanup. Before revision 50, the original
  20-turn countdown remains unchanged.
- Sparkle admits every visible in-bounds square, including walls, chasms and
  water. It still costs 20 energy and takes no action. In revision 51, casting on
  deep or shallow water immediately quenches the new flame and its brazier light,
  including a water square with Ichor. A legal wet cast still pays. Invalid,
  unseen, silenced and unfunded attempts do not place or pay. Non-water placement
  remains a permanent fire/light source. Hellfire is not removed.
- Revision 51 ordinary fire cannot persist on water without Ichor. Direct
  ignition, spread fire writes, and adapter entry (including offscreen upkeep)
  clear the flame, fresh/reaction/provenance state, permanent source entry and
  corresponding brazier light. Unrelated fixed lights remain. Gas-reaction and
  steam processing remain in place. The new contact rule means the same tile,
  not a neighboring water tile. Revision 50 retains its existing wet Sparkle.
- All-Seeing Eyes still costs 30 energy and takes no action. Revision 51 reveal
  lasts until a successful time-consuming action finishes. Haste, I Came For You,
  Relentless Pursuit credit and backward Weave bonus moves preserve it. No-action
  commands and failed actions preserve it. A normal move, attack, Wait, ordinary
  consumption or other time-consuming action expires it after final observation.
  Revision 50 keeps its action-completion boundary, including bonus movement;
  pre-50 keeps its historical world-turn boundary.
- Active Shine suppresses the whole emitted source in revision 51, matching
  revisions 46-49. It emits no light on self, terrain, other actors or the
  renderer's source list. Other illumination and the target-light accuracy bonus
  remain independent. Revision 50 and <=45 keep their prior self-only exclusion.
  Toggling is still free, including under Silence and on bonus phases.

**Pending interpretation:** ordinary Ichor remains a distinct combustible layer.
Spread fire currently records local fuel kind, not Sparkle ancestry. The direct
wet Sparkle rule is implemented, but distinguishing only Sparkle-origin spread
onto Ichor-covered water from independent Ichor combustion needs additional
origin state or a broader rule. This is not silently changed here.

## Persistence, planes and caches

No new serialized field, duration timer, migration, gameplay RNG or wall-clock
behavior is introduced. All changes are revision gated; retained save/replay
revisions are not upgraded. Existing conditions, active lists and floor source
arrays pass through canonical save/load, Snapshot, See The Future and replay.
Player-plane activation keeps dream fire isolated from waking fire. Ordinary
loaded/offscreen fire uses the same adapter cleanup. This is not a claim that
all independent offscreen Shine actors are supported by existing admission.

Shine's scalar query, exact dependency cache and observed source list all use
`vision::emits_shine`. Suppressed source motion has no lighting dependency;
revision changes/toggles alter the actual source list and invalidate the cache.
Water source removal similarly changes both fire and fixed-light dependencies.

`ability_ui::display_description` provides R51 prose without changing frozen
catalog text or the description-derived offer weighting. Existing command/support
identities are unchanged.

## Deterministic coverage and verification

`revision51_ability_restorations` covers terrain admission, direct water including
Ichor, energy/Silence rejection, free Shine, unlimited I Came For You versus the
legacy countdown, all four bonus movement modes, normal/failed/no-action expiry,
canonical saves, Future/Snapshot/replay, wet-source cache cleanup, water spread,
Hellfire preservation and dream-plane isolation. `shine_revision46` also runs
its whole-source, overlapping light/darkness, dream/body, portal/hidden-source
and free-toggle/shared-body assertions at revision 51.

Focused native verification passed on 2026-11-07. The pinned Rust 1.99.1 compiler
built the integration test binaries directly against the parent's freshly built
`libcorpse_core-ccbda60d807cc80b.rlib` artifact, avoiding competing Cargo work.
All 11 restoration tests, 11 Shine tests, 12 R50 preserved-edge tests and 12 R50
activation/upkeep tests passed. The first restoration run found a test-only
AlreadyInitialized reserve setup error; the corrected fixture passed all 11 on
rerun. See `verification/r51-ui/ability-restorations-rerun.log` and
`verification/r51-ui/direct-core-tests.log` (the latter retains the initial
fixture failure and unrelated integration results). Artifact hash is retained in
`verification/r51-ui/direct-core-tests/core-artifact.sha256`.

Final frozen-source workspace verification and the pending Ichor-spread decision
remain separate. Platform screenshots are handled by the native rendering pass.
No reference parity or Windows execution claim follows from these focused tests.

## Complexity and bounds

Let V be floor cells (currently admitted sizes <=3600), P permanent-fire entries,
L fixed lights, A actors, B ability entries and C ordered condition entries.
The pre-existing environment admission and floor COW copy costs remain additive.

- Direct wet contact is O(1) layer checks plus O(P+L) stable source removal,
  O(1) scratch, and any first floor COW write. Ordinary non-wet contact only adds
  O(1). Each spread fire write adds the same wet-contact work; with W such writes
  in a propagation pass the additional worst case is O(W*(P+L)). Existing gas
  reaction closure, propagation and root cleanup bounds are unchanged, not O(1).
- Fire-adapter entry adds O(V+P+L) time and O(V) temporary boolean mask for wet
  source cleanup. It is invoked by active/offscreen fire processing and immediate
  gas ignition paths, not once per rendered tile. No retained cache is added.
- All-Seeing expiry routing remains O(1); removing the two ordered conditions
  is O(C) worst-case time and O(1) scratch. All existing finish/observation work
  runs once, with its normal visibility/cache/actor costs.
- I Came For You adds no work beyond a source comment/tested existing behavior.
  Shine's helper retains O(B) scans per eligible actor. Existing cache dependency
  capture includes O(V+A*B+L) work plus ring LOS geometry and cold build costs.
  Suppression reduces sources but does not promise faster rendering. Native
  rasterization and framebuffer costs remain in the rendering ledger.
- Display overrides are fixed O(1) dispatch and static text. Snapshot/save/replay
  retain whole-state/COW and serialized text costs with no new fields.

These bounds are source-derived, not measured performance results. Run focused
core and Shine tests, then the complete workspace suite on the frozen source.

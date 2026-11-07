# Revision 51 aiming paths

## Scope and visible behavior

Pack Apply, Shoot and Throw share one detached aiming result in both full
observation and cached pointer updates. Delivery cells are gold. Impact cells
and area footprints are coral and are stored separately. Stochastic portal exits
retain the existing cyan alternative markers, without choosing an outcome.

Arrow/WASD/vi/numpad movement while aiming moves the cursor, not the player.
Enter routes through the same target-selection and command-dispatch branch as a
map click. Escape follows the existing cancellation/reopen behavior. Switching
between mouse and keyboard, targets, items and effects replaces all descriptors;
clearing an aim clears route, area, impact and color together. Resize preserves a
keyboard aim coordinate and rebuilds its geometry at the new display size.

Lightning retains its authored creature piercing and extends beyond the aimed
cell until terrain, a hidden prefix boundary or an unresolved fork. Firebolt,
Poison and Acid stop at the first observed creature; Acid retains eight physical
tiles across portal transfers. Bow/throw retain their selected-endpoint physical
ray. The core portal ray planner/traversal and physical terrain predicate are
shared with execution. Preview never dispatches, spends resources or draws RNG.

Revision 51 physical rays also collide with solid Borge/Epidite fixture cells.
This predicate is version gated; earlier simulation revisions keep their replay
collision behavior. Blink now requires a clear physical trace to its chosen
endpoint before admission/payment. A blocked or unresolved-random-fork Blink is
rejected, never silently shortened. Its ten-tile Euclidean endpoint range,
visible empty endpoint restrictions and arrival behavior remain in force.

Iceblast is still the authored remote visible 3×3 area effect, not a projectile.
Its area/center is marked without fabricating a delivery ray. Changing it to
collide/detonate or reject remains a designer decision. Floods retains its
three independently stopped five-cell lanes and authored body-piercing wave.
Beckoning now displays the observed pull geometry in the same deterministic
candidate-destination order as execution; it does not promise that unobserved
occupancy or other live movement restrictions will permit the command. Mental
and conjuration effects keep their endpoint/placement footprints. The targeted
divine catalog contains mental effects, pursuit and placement, not a separate
bow-like projectile launch; no new physical rules were assigned to them.

## Knowledge and ownership

The observer gates each displayed ray cell on current terrain knowledge before
collision reads, even if a remembered path continues beyond it. It filters
creature occupancy to observed, living, same-plane actors. Known topology is
used without selecting a random branch; incomplete link groups terminate
preview rather than promising a continuation. Rendering receives only detached
coordinates/colors and makes no world query. Unknown occupants never change the
preview. The committed action remains authoritative and can meet an unseen
obstruction. No Dreamwalking amulet admission was added.

Core policy/preview: `projectiles.rs`, `revision50_amulet_effects.rs`,
`revision50_control_amulets.rs`. Desktop observation/flow: `targeting_ui.rs`,
`pack.rs`, targeting blocks in `main.rs`. Render descriptors/defaults and
invalidation: `render.rs`, `observe.rs`, `render_damage.rs`, `tile_damage.rs`,
`tile_overlay.rs`.

## Complexity and bounds

Let V be floor cells (existing admitted maximum 3600), F all stored fields cloned
with the floor, A floor actors, P spatial link endpoints/alias lookup cost, L the
bounded returned ray length, and Q the existing terrain/actor observation work.
Aiming recomputes on target, item/mode or world-observation invalidation, never
per paint/animation frame. Amulet ray preview costs O(Q + F + A×actor-visibility
+ P²×V + V×A×P) worst-case time and O(F+V+A+P+L) scratch. The P²×V term includes
the existing portal plan search and per-cell gates; alias membership can scan
links. Portal traversal is cycle bounded by the existing traversal contract.
Acid is display-truncated at eight physical tiles, but the current pure trace
can still traverse a longer bounded geometric ray before this truncation. This
is not claimed to be an O(8) ray implementation. Iceblast adds at most nine
footprint checks after observation. Beckoning checks up to eight candidate
routes of at most ten cells, plus observed-actor/terrain preparation. Each
candidate clones B actor-field bytes and each step costs O(A×P+M+H), including
alias/corner lookup, M terrain-modification scans and H habitat/trait checks.
Its additional scratch is O(V+A+L+B). Shared fixed-fixture terrain admission is O(1) per cell.
Blink adds one existing pure projectile trace before payment; no snapshots,
serialized fields or retained world caches were added.

With D delivery/area/impact/exit coordinates, full rendering builds O(D) expected
hash membership (pathological hash collisions O(D²)), then performs existing
cell and native framebuffer/GPU raster work. Retained/tile invalidation visits
old/new O(D) coordinates plus their bounded pixel/tile footprints. Additional
stored descriptor memory is O(D). Native fill cost scales with covered pixels;
GPU command count scales with drawn marks, with existing compositor upload and
mask costs unchanged. Keyboard aim stepping is O(1). No measured speedup or
hardware performance claim is made.

## Verification

Focused core regressions live in
`revision50_amulet_effects::tests::r51_*`: no-cost Blink rejection, physical
fixture blockers, hidden occupancy/remembered-terrain invariance, ray/launch
alignment, Lightning piercing, Acid range and portal distance, unchanged
Iceblast area. Existing frozen bow targeting and portal uncertainty tests remain.

`aiming_effect_tests.rs` covers mouse/keyboard detached equality, read-only
retarget/cancel/equipment changes, separate delivery/area/impact, retained/full
and tile/full pixel equality, animation timestamps, resize/aspect ratios, GPU
command capture and optional native PNGs through `CORPSE_AIM_REVIEW`.
Intermediate evidence: the public-API integration target `revision51_aiming`
passed its initial six cases (`../revision51-aiming-core-tests.log`). Desktop
artifact 6 passed all three initial aiming tests and all nine perception tests
(`verification/r51-ui/targeted-perception-aiming-1.log`; artifact receipt
`verification/r51-ui/desktop-compile-6-artifact.sha256`). This includes
full/retained and full/tile pixel equality, resize/animation, initial read-only
mouse/keyboard descriptor tests and GPU command capture.

All seven native 1280×800 screenshots in `verification/r51-aiming/` were visually
inspected: Firebolt stops at the creature, Lightning continues through it to the
wall, bow/Blink show separate delivery and endpoint marks, and Iceblast shows
only its 3×3 area. Both cancellation images have no residual aiming marks and
are byte-identical (SHA-256
`da2afb8d3b4c46e2a16bb143eb78615eba8bc5945cdbbe9f101c9e0a746ffb39`).
No visual defect was found in these captures.

Final focused verification: all seven core integration cases passed in the
actual executable run recorded by
`verification/r51-ui/aiming-final-core-direct.log` (7 passed, 0 failed). This
includes the final Beckoning known-movement restrictions. All four desktop
aiming tests passed against final source 3b in
`verification/r51-ui/illumination-targeted-3.log` (4 passed, 0 failed), including
modifier handling and modal action/descriptor cleanup. The same final desktop
run passed all nine perception tests. The compile record is
`verification/r51-ui/desktop-compile-illumination-3b.log` with its matching
`.exit` receipt.

The screenshots above remain artifact-6 visual evidence, not newly captured
final-source screenshots. Final focused tests cover the later input/Beckoning
patches and rendering parity; they do not turn the earlier PNGs into a capture
of those patches. The larger core unit filter was not separately rerun, and the
full-workspace aggregate was still running when this report was updated. No
full-workspace pass is claimed here. These checks do not establish Windows or
accelerated-hardware certification or manual keyboard/mouse input validation.

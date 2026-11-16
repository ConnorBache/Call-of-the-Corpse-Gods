# Released Floods retained while radial effects are undecided

The release split follows the instruction to retain previously released Floods
and Relentless Pursuit until their outstanding design answers, while delivering
finished independent work. The radius-four Floods implementation is preserved
separately as an unfinished checkpoint and is not compiled into this release.
Relentless Pursuit and the isolated Never Ending Flesh work are not changed here.

## Current behavior

Native Pack Apply enters targeting and emits `UseAt`. The unchanged released
`revision50_wave_teleport.rs` owns the three lanes by five steps, cardinal or
diagonal aim, one tiered damage packet per reached creature, up-to-two-tile push,
quenching and twelve-normal-turn water duration. At revision 52, each lane
includes and deepens its first preexisting shallow cell and then stops; existing
deep water stops that lane without being included. Tier damages remain
20, 35, 42, 43, 61, 62, 63, 77 and 84. Cost remains 30 with the independently
completed R52 reserve cap of 10 and existing energy fallback/payment rules.
See [the full wave contract](revision52-floods.md) for protection and expiry.

The release does not allocate outward currents, flood the player's centered
disk or add radial current metadata. Existing world currents, corpse drift and
other completed terrain/animation changes remain independent and unchanged.

## Save and replay scope

Released targeted `UseAt` commands, temporary-terrain records and their normal
save/load/expiry paths are retained. Non-targeted `Use` has the released missing-
target rejection semantics and is never translated to a wave in another direction.
The unpublished checkpoint's `floodCurrent` metadata is rejected as an unknown
field by the strict temporary-terrain schema, including ordinary/compatible
save load and Snapshot decoding. Unpublished radial saves or recordings belong
to that checkpoint and are not claimed compatible with this release. This split
does not promise seed-replay identity across unrelated completed R52 amendments.
The independently completed Firebolt replay-version boundary is untouched.

## Verification scope

The focused core tests restore the released shallow/deep wave suite and add
explicit non-targeted rejection, unpublished-metadata rejection and recorded
UseAt save/load/recast/expiry replay tests. Native tests restore visible footprint,
independent lanes, recasts, expiry, hidden-water privacy, immersion and actual A12
animation removal. Real Pack keyboard/mouse actions stay targeted at R50–R52;
the normal action handler enters target mode without casting or charging.

`CORPSE_R52_FLOODS_OUTPUT` exports the native wave scenes at 1280x800 and 960x1200.
Full, retained and tiled CPU pixels must match exactly. The new target-mode
handler regression adds no synthetic capture. Historical radial receipts remain
historical, not release certification. Current executed receipts are maintained
by the release verification owner; focused checks alone are not a full workspace
or Windows-runtime pass.

## Complexity

The existing [wave bounds](revision52-floods.md#complexity) are unchanged. Removing
radial dispatch/painting eliminates its per-Use inventory/current-layer guard,
49-cell rays, current-grid allocation and final zero-grid scan. Existing terrain
validation at Snapshot boundaries remains; no new production algorithm is added.

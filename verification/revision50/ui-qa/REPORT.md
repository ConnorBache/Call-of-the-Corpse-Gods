# Desktop UI verification

## Result and scope

The existing desktop test harness passed 24 test executions (23 unique tests), with zero failures. Twelve generated screenshots were inspected with `view_image`. No visual regression was observed in this sample: the enemy column, corpse cross, red blood dots, original menu, and compact dialogs remain intact.

This verifies focused Revision 50 UI data/projection tests and legacy/released-baseline rendering. It does **not** verify an admitted, playable Revision 50 run. Production Revision 50 storage admission remains closed. No gate, source, capacity, or item-choice changes were made during this verification pass. No compiler or software installation and no competing build were run.

## Executed tests

Harness: `target/debug/deps/corpse_desktop-793c95fc09982af2`.
Each filter ran with `--nocapture --test-threads=1`.

| Filter | Passed |
| --- | ---: |
| revision50 | 11 |
| cogworks | 7 |
| corpse_marker | 3 |
| blood_dot_sequence | 1 |
| compact_dialog | 2 |

One Cogworks targeting test appears in both the revision50 and cogworks selections. Full output is in `focused-desktop.log`. Covered paths include versioned ability names/costs, reserve display and invalidation, equipment/category actions, multiple item grants, targeting rejection without mutation, Clay/Cogworks detached markers and both renderer damage paths, corpse markers, blood-dot backgrounds, and compact-dialog transforms/restoration. Greatsword and Missed Me inspection tests are included in the revision50 filter.

Compact-dialog tests exercise transforms at 1280×800, 1920×1080, 1537×911, 960×1200 and 960×720. Their emitted screenshots were inspected separately; this is not a claim that screenshots of every resolution were manually inspected.

## Visually inspected artifacts

- `menu.png`: original menu rendering.
- `baseline-game.png`: legacy avatar-room fixture and enemy column.
- `baseline-pack.png`: legacy item/equip interface preserved.
- `baseline-inspect.png`: compact inspection sheet.
- `generated-baseline.png`: generated released-baseline forest and enemy-column scrolling layout.
- `r46-markers.png`: controlled revision-46 fixture showing the corpse cross and three floor blood dots without repainting the floor background.
- `compact-dialogs/creatures.png`, `ability.png`, `hazard.png`, `inspection.png`, `pack.png`, `blessing.png`: compact dialog boundaries, text, controls and background composition. No clipping or unexpected overlap was observed.

The synthetic `r46-markers-fixture.json` is based on `fixtures/avatar-room-v1.json`, with revision 46, empty history, one dead/heartless existing actor and three blood cells. It is a rendering fixture, not a mechanics test. Its first render attempt was rejected for missing `history`; adding an empty history resolved the fixture error.

Render-test commands used only workspace-local profile, appearance, save and replay locations under `verification/revision50/ui-qa/`. They did not launch an interactive production window.

## Binary identity and limitations

SHA-256 at verification:

- `target/debug/corpse-desktop`: `5769aa3e57cc4d99c3100ee872f0cc731d9e7ab6d007e1b802cbdbfc235b1543`
- Desktop test harness: `0bbd561460a4dcd6387bb7e8a5d131ffddb876f0e045bff6c67eea904c52e8ac`

These were already-built binaries. The parent was continuing workspace testing and correcting core test fixtures/admission setup in parallel; this report does not certify later source changes or the full workspace suite. Interactive SDL input, window-system behavior and hardware GPU presentation were not manually exercised. No platform-wide or exhaustive correctness claim is made.

Outstanding design/integration limits remain separate from the passing UI tests: reserve capacities are not invented, Cogworks wall mode remains preview-only pending its health-distribution rule, and production R50 admission remains closed.

Follow-up source and test-log verification corrects the earlier Clay occupancy audit note: current `clay_preview_for_run` calls `clay_plan_for_run(..., true)`, whose occupant rejection is limited to directly visible actors. `activate_clay` separately calls the same planner with `false` to reject all occupied destinations. The regression `clay_preview_hides_invisible_occupants_but_activation_rejects` checks equal previews with an invisible occupant versus no occupant, and rejection without state mutation on activation. Its pass is recorded in `../core-unit-current.log` (310 passed, 0 failed, 2 ignored). This resolves the specific stale full-occupancy-preview claim; it is not a UI-wide invisibility guarantee or an additional test execution by this verification pass.

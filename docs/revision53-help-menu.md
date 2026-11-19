# Help menu and keyboard controls

## Restoration provenance (2026-11-15)

Selectively restored from the authenticated GitHub release
`controls-help-review-20261110`, asset `controls-help-source-review-v2.zip`
(SHA256 `95dfa79f8c23076daaf66ca0dc599387ade20e6825f59013a63b106aac770806`).
The overlay's ten source hashes were verified before integration. The original
patch cannot safely replace the current main loop: this port retains the newer
death pause and low-health warning input precedence and auto-rest guards, along
with the current HUD, item descriptions and targeting behavior. No simulation
mechanics or game data were replaced. This restoration is local, not published.

Restoration validation: nine Help unit tests passed; actual SDL event-loop
checks passed in both software and accelerated renderers (68 total checkpoints),
including readonly save equality, cursor, targeting, Pack, rest and resize.
Six current Help layout captures and in-game event-loop screenshots were
regenerated; Help's original Reserve Energy explanation is retained.
Historical validation below describes the original overlay, not a current
Windows runtime validation.

## Controls

Enter enters keyboard inspection at the current valid hover coordinate, or the
player position. Direction keys move the detached cursor within map bounds.
Enter again uses the existing map-click path, including travel, attack and
hazard admission. Escape leaves cursor mode without opening the menu. A later
fresh Escape also leaves the live run open. Held Enter/Escape do not repeat
confirmation. Mouse motion takes ownership.
Other panels, targeting and world replacement clear general cursor mode.

Z dispatches the existing single-turn Wait command; Shift+Z retains rest until
an observed change. Item/ability targeting consumes Z rather than resting.
Inventory, Help and other modal owners retain their input precedence.

Mouse inspection and ability activation retain existing behavior: moving aims
or examines, and clicking activates an already selected ability. Help uses the
designer's requested command and Information wording, yellow labels and white
descriptions. No simulation, save, RNG or time-travel rules are changed.

## UI integration

- A clickable `?` joins the game toolbar: Pack, Bestiary, ?, Menu. Stable command
  IDs are preserved. Main Menu also has a top-right `?` button.
- F1 and Shift+Slash (`?`) open Help when the underlying view has no conflicting
  modal. The former Shift+Slash-to-Menu behavior becomes Help; the distinct Menu
  button leaves an active run.
- The heading says Commands, followed by two-column rows. Information is a
  separate heading inside the same scrollable body. Labels are yellow
  `[255,217,53]`; descriptions are white `[255,255,255]`.
- Both columns use measured wrapping. The layout is bounded and centered in the
  committed logical viewport. Rows cannot overlap the other column or footer.
  Font/profile and viewport changes invalidate the detached layout cache.
- Wheel, arrows/numpad 8/2, Page Up/Down, Home/End and visible arrow buttons
  scroll. Escape, Enter, Space, F1, `?`, Close or an outside left click dismiss.
  Dismissal consumes the event, preventing a click-through move or attack.
- Held F1/`?` cannot toggle repeatedly. Gameplay keys, mouse movement and text
  entry are consumed while open. Quit, window events and Alt+Enter fullscreen
  remain available. Closing/reopening resets scroll to the top.
- Opening cancels rest/travel/autoexplore. Help never dispatches a command. The
  selected item/ability target is retained rather than consumed by Help.
- Opening/closing forces a full repaint, and the experimental software tile
  path is disabled while Help is open. This avoids stale panel pixels and
  partial rendering when entering/leaving the modal.
- A `--render-help --render-test <file.png>` capture route uses the same painter
  without introducing a separate renderer or simulation fixture.

## Complexity

Variables: `T` is the fixed authored help text, `U` its distinct characters,
`P` the appearance profile size, `L` the retained wrapped line count, `F` the
bounded embedded-font fallback/metric lookup work per new glyph, `G` visible
text rasterization/blending work, and `A` native panel pixels. There are exactly
9 command rows and 3 information rows. Inputs are not save/replay history.

- Opening or a viewport/profile cache miss: O(P + T² + U*F) worst-case time and
  O(P + T + U) extra memory. The existing `TextMetrics::wrap` repeatedly measures
  candidate strings, so this is not claimed linear in text length. Profile
  cloning/equality is included rather than treated as a free cache key.
- Each event/paint with an unchanged layout: O(P) profile equality plus O(1)
  routing/clamping. Scroll uses saturating arithmetic and bounded offsets.
- Help painting: O(P + L + G + A), including cached-line traversal, short
  header/footer fitting, glyph-cache misses and the software panel raster.
  GPU painting emits rectangles/glyphs into the existing command stream and
  shares its atlas cache/miss/upload costs; native raster cost is not erased by
  describing the retained text as static. Existing base-scene redraw, optional
  animation, framebuffer upload and present costs remain additional.
- While Help is active, software tile painting falls back to existing full
  composition. Open/close invalidates the retained overlay/tile state. No new
  pathfinding, visibility, floor observation, simulation or history scan is
  performed by the Help module.
- Keyboard and pointer dismissal are constant-time, apart from profile-key
  comparison. Layout storage is retained across close/reopen; offset resets.

These are analytical bounds, not measured timings. Benchmark default and large
font profiles on software and GPU paths at representative sizes to measure
cache misses, retained-frame costs, uploads and input latency.

## Keyboard cursor complexity

For map width W and height H, each cursor input clamps coordinates in O(1)
time and O(1) extra state. Cursor inspection reuses the existing observation,
hover reporting and painting paths; their visibility, terrain and glyph costs
remain additional and are not claimed constant. Enter confirmation reuses the
existing click travel/pathfinding and hazard admission costs. Plain Z dispatches
the existing simulation Wait, including normal/haste phases and effects; it
has no new simulation algorithm. No history scan or new RNG calls are added.
These are analytical bounds, not measured performance claims.

## Verification status

Deterministic tests cover cursor bounds, repeat/modifier handling, modal
ownership, normal/haste wait parity, save/load, hidden information, Dreamwalking,
click geometry and hazards. Help tests cover copy/colors, toolbar hit regions,
consume-on-dismiss, scrolling, wrapping, profile/viewport invalidation and
unchanged serialized state. The opt-in SDL event-loop test injects events into
the actual production loop and captures software and accelerated output.
Offscreen SDL injection is not physical OS keyboard or Windows input coverage.
See `../verification/help-controls/` for actual run results; test presence alone
does not establish a passing result. No release publication is authorized.

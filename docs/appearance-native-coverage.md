# Native appearance studio and settings

## Authority and boundary

The immutable v48 browser build remains the oracle. `tools/export-appearance.mjs` verifies the existing exported project defaults and extracts the exact 1,665 source controls, their labels/categories/groups/ranges/enums, 17 parse/edit/reset cases, and 41 appearance-store transition cases. The native executable does not run JavaScript. All authoring state is independent from game saves, gameplay time, RNG, and simulation commands.

All schema-v1 fields are preserved, including unknown future JSON properties. Registered properties receive source type/range/color/glyph/font validation; unknown properties cannot execute code. Unsafe prototype names, excessive nesting/collection sizes/strings, unsupported schema versions, malformed JSON, and inverted brightness bounds are rejected before changing the working profile. Original built-in non-ASCII glyphs remain valid; custom glyph edits use one printable ASCII character, matching the source contract. Inert legacy fields remain data-only and are not newly exposed as controls.

## Native authoring behavior

- Settings and F8 open the studio. Its geometry, colors, and controls remain fixed while the user changes game typography, layout, or color settings; authoring cannot strand its own Back button.
- Categories, All categories, catalog-entry groups, search, scrolling, exact text input, numeric +/- controls, boolean/select cycling, and per-property resets cover the source control catalog. Enter commits an exact value. Ctrl+A replaces the field; Esc/F8 hides the editor and cancels uncommitted text/temporary preview.
- Undo/redo preserve the source 80-entry/8,000,000-character combined history limit. Reset-all and JSON imports are undoable. Store-level gestures coalesce intermediate edits; discrete native +/- clicks are individual operations.
- Original/Edited and saved-preset comparisons do not overwrite edited data. Temporary Preview changes rendering only; Commit makes it one undoable edit, Discard restores edited data, and hiding ends preview/comparison. Export always uses the edited snapshot, even while a preview or comparison is displayed.
- Save new, overwrite saved changes, load, duplicate, rename, confirmed delete, and explicit startup default are available. Names are unique ignoring case, up to 80 characters, with at most 50 presets and the source aggregate library-size cap. The selected preset's data is copied, never aliased.
- The default settings location is per-user: APPDATA/corpse-gods/appearance.json on Windows, ~/Library/Application Support/corpse-gods/appearance.json on macOS, and XDG_CONFIG_HOME/corpse-gods/appearance.json or ~/.config/corpse-gods/appearance.json on other platforms. Explicit --appearance paths remain available. Default-directory creation is explicit and tested.
- Working edits remain session-local until explicitly saved as a preset, matching source behavior. A chosen startup preset loads on restart. Reduced-motion/fullscreen preferences share a versioned native settings envelope outside game saves. Browser-style unwrapped appearance libraries can also be loaded by the settings backend.
- Export uses source-style pretty JSON unless whitespace alone would push it above the accepted 2,000,000 UTF-16-character import limit. In that narrow case it uses compact JSON so an accepted profile can be imported again; this robustness difference is regression-tested.
- Import supports an explicit JSON path, drag-and-drop, and pasted clipboard JSON. Export uses an explicit path and confirms replacement of an existing file. Copy uses the SDL clipboard through the main-loop adapter; failures are surfaced. Native path entry replaces the browser file-picker/download flow.
- Preset/preference persistence is transactional: write a new same-directory temporary file, sync it, then rename. Failed saves leave the prior file/library intact and working edits exportable. Damaged settings are reported and left untouched on load. Unknown future profile fields survive save/load/export.
- The Glyphs and Fonts page renders the authored uppercase/lowercase/digit/symbol sample through the same map-glyph painter used in the dungeon.

## Renderer coverage

The editor's complete data schema is broader than current native rendering coverage. Do not infer that a stored/imported option already changes pixels. The studio footer discloses retained-only options. `desktop-complexity.md` tracks renderer integration and scene evidence.

The profile-aware display implementation includes global color grading, source palette anchors, terrain material tuning, detached visual lighting, fog/memory/special-sense grading, map typography/aspect, actor/item override resolution, and path styling as their corresponding renderer modules are integrated. Source lighting is independently verified in `appearance_light.rs`: global/per-source color, intensity, visual radius, falloff/curve, floor/wall response, ambient, soft/additive/screen blending, illumination cap, flicker controls, glow, and burning-liquid category. Expanded light uses only known current terrain and conservative wall/tree/door/corner blocking; it cannot reveal geometry or change mechanical illumination.

Remaining native-specific limits must stay explicit:

- Installed OS/browser font discovery and arbitrary platform font families are not provided. Bundled IBM Plex Mono, Inconsolata, Brogue CE ASCII artwork, and the existing Unicode fallback fonts are used. Unsupported names retain their original JSON text and fall back; this does not certify matching platform font outlines.
- UI typography is applied to Chronicle, ability names/summaries, condition labels, creature summaries, expanded ability tooltip/heading, toolbar and inspector. Other selectors and modal reflow remain partial; Pack inventory role consumers now reflow within bounded lists/details; hotkeys role consumers are still retained-only. Bundled regular font outlines use coverage-gamma weight approximation, not exact browser font-face/variable-weight raster parity. The native studio itself deliberately uses fixed typography and geometry.
- Sidebar clamp, map spacing, Chronicle track, ability top/row/capacity and hitboxes consume authored layout values. Pack/Inspector consume shared bounds, authored inventory-row height, tooltip width, button height/padding/borders, selected/hover/panel colors and measured text reflow. Remaining panel padding/stat row spacing and other modal consumers are not yet fully integrated. The editor labels partially applied families separately from retained-only controls.
- Fog transition softness and separate All-Seeing Eyes tint/intensity apply to terrain; remaining entity/overlay consumers still require knowledge-stage integration. The editor labels these as partially applied.
- Semantic slime, mud, ichor, razorleaves, blood, roots, wards, embers, fire and visible light fixtures now consume profile material/entity colors and tuning. Actor/item ink follows observed source lighting on cosmetic frames. Ten semantic scene and ten actor-ink fixtures cover a bounded subset; gas glyph/animation-speed overrides, some fixture highlight details, full layer opacity raster equivalence and complete scene combinations remain explicit parity gaps.
- The native studio is a fixed full-screen settings view. It does not yet reproduce the browser's docked live-dungeon panel or its temporary developer test-scene/return-to-dungeon buttons. F8 returns to the unchanged current dungeon. The dedicated glyph sample is available within the studio.
- Unknown-origin lights use their redacted coordinates for deterministic flicker phase and never expand. Exact hidden-origin phase parity is unavailable without violating the detached-knowledge boundary.

The exact source role/layout/color consumer matrix, including deliberately inert legacy fields, is documented in [appearance-source-consumers.md](appearance-source-consumers.md).

## Verification

- 22 data/storage tests pass, including 17 source profile cases and 41 source store transitions, history count/size caps, preset count/aggregate-size caps, forward-compatible JSON, malformed import rollback, identity inheritance resets, CRUD/default reload, failed writes, and corrupt-file preservation.
- Eight editor interaction/raster tests cover repeated edits, exact-input cancel/error, undo/redo, search/group/scroll bounds, preview dismissal, confirmed/cancelled deletion, edited-snapshot copy/import, Original comparison display, and fixed geometry after extreme font/padding/sidebar settings.
- Lighting has 41 exact source field fixtures and 54 deterministic source flicker samples, plus existing 600 default color cases and generated-light parity tests. Tests assert observations are unchanged and unobserved/remembered cells cannot authorize radius expansion.
- Entity/material fixtures verify 59 source entity inheritance/tuning cases, 12 material inheritance cases, 26 feature-color/fallback/grade cases, nine pulse cases, six animation-detection cases, and 48 fog transition/All-Seeing Eyes knowledge-stage cases.
- Four typography helper tests verify 53 source CSS override cases, default-reset selector baselines, chronicle sizing, and bundled font fallback reporting. Final text placement/outline parity remains renderer evidence.

- Five software-rendered screenshots were inspected at 1280×800: global controls, terrain, entities, map fonts, and presets/export. They are in `artifacts/appearance-qa/`.

Commands: `tools/cargo.sh test -p corpse-desktop --test appearance`, `tools/cargo.sh test -p corpse-desktop --test appearance_lighting --test appearance_entities --test appearance_typography`, and `tools/cargo.sh test -p corpse-desktop --bin corpse-desktop appearance_ui`. Set `CORPSE_APPEARANCE_SCREENSHOTS` to an absolute output directory when running the editor tests to regenerate screenshots. Run `tools/cargo.sh test --workspace` for aggregate verification after integration.

These checks certify data transformations and software input/paint handlers. They do not claim desktop clipboard/fullscreen interaction, OS filesystem durability, browser-equivalent layout, or complete native visual parity on macOS/Windows/Linux.

## Responsive owned overlays

`modal_ui` resolves shared Pack/Inspector/hover geometry from the detached profile and display canvas. Pack configuration caches measured per-character advances, then click, keyboard focus, wheel scrolling, button rectangles and painting use the same resolved layout. Inspector disclosure hit testing and scrolling use the same authored line metrics as paint. Profile edits do not resize based on content, mutate a Run, or change the map camera. Oversized authored inventory fonts are clipped inside their explicitly fixed row height, matching the source fixed-row behavior; they are not silently shrunk.

Four geometry tests cover default geometry, authored track/containment bounds, six frozen-source Pack placements, 36 authored-width hover placements, and measured wrapping. Two interactive custom-profile tests cover Pack close/selection/keyboard/scroll/action layout and Inspector close/disclosure/scroll/hover, with Run/report immutability. Four default/authored screenshots are in `artifacts/modal-qa/`. Exact CSS descendant typography inheritance and font raster equivalence remain broader scene-parity work; this does not certify every modal or platform.

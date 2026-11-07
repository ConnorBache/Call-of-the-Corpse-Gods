# Frozen v48 appearance consumer matrix

Authority: frozen `src/ui-interface-appearance.ts`, `src/ui-map-glyph.ts`, `src/ui-fonts.ts`, and `style.css`. This is the browser consumer contract, not a declaration that every native consumer has already been wired. Native progress and remaining limits are in `appearance-native-coverage.md` and `desktop-complexity.md`.

## Typography override semantics

The browser emits a CSS declaration **only when that registered field differs from `DEFAULT_APPEARANCE`**. Default schema values do not replace every selector's stylesheet baseline. All injected selectors exclude the appearance editor and its descendants. Resetting a field removes its override; for example, headings schema weight400 resets an h2 to stylesheet weight500, and a command hotkey to its own weight600. Schema headings size14 does not shrink an unchanged h2 from21.

Each of the eight UI roles consumes exactly `fontFamily`, `fontSize` (px), `fontWeight`, `letterSpacing` (px), and `lineHeight` (unitless):

| Role | Browser consumers |
|---|---|
| primary | body, button, input, select, inspector line |
| secondary | ability summary/status, condition label, stat, status line, section-heading child span, creature state/traits |
| headings | h2, h3, eyebrow |
| chronicle | log and direct log rows |
| ability | ability-row ability-action |
| tooltips | hover tooltip, ability-expanded |
| inventory | inventory, item-name, item-name small, pack-item-details |
| hotkeys | kbd, command-bar kbd |

These are selectors, not exclusive categories: typography follows CSS inheritance and matching selectors. Native `typography_ui::resolve` deliberately accepts the caller's actual baseline; it applies the changed-field overrides without inventing a universal role baseline. It does not implement a DOM cascade. `overrides` allows consumers to apply inherited primary values selectively. The source uses13px/12px secondary labels,14px item names and13px small text,21px/500 h2, and14px/600 h3. Explicit role edits override all matched variants.

Chronicle font-size or line-height edits set line pixels to `fontSize * lineHeight`. If layout chronicleHeight is still its default108, the reserved track becomes4.5 such lines. An explicitly changed chronicleHeight wins. Font weight/family/spacing edits alone do not resize the track. Unchanged profiles preserve source compact-window22px lines, rather than forcing default24px. This is implemented by `chronicle_metrics`.

Map typography is separately active: family, font size, weight, letter spacing, line height, glyph scale, alignment, and tile aspect. Source `mapFont` scales each authored draw size by `fontSize/17`; default weight preserves the draw call's normal/bold choice. Map transform applies horizontal scale `glyphScale * min(1,aspect)` and vertical scale `glyphScale * min(1,aspect) * lineHeight/aspect`, then bounds ink against neighboring cells. Brogue is ASCII bitmap artwork, not a general UI font. Native outline/raster equivalence remains a separate visual gate.

Non-map glyphScale/alignX/alignY/tileWidth/tileHeight fields are intentionally inert legacy data and are not exposed in the source UI role controls. Preserving them without changing UI pixels is correct.

Native bundled family selection preserves recognized-family order, uses per-glyph coverage, reports unavailable names, and appends deterministic bundled fallbacks. Arbitrary installed OS fonts and CSS generic-family platform matching are unsupported. Available faces: IBM Plex Mono, Inconsolata, Corpse Glyph Mono/Symbols/CJK; Brogue CE ASCII only for the map. Exact font weights/outlines depend on the renderer's loaded face/weight support, not merely the resolved numeric value.

## Layout consumers

| Field | Actual browser consumer |
|---|---|
| rightSidebarWidth, sidebarWidthPercent | Changed values set sidebar clamp(min(290,rightWidth)px, percentage vw, rightWidth px); grid, drawer and floating-panel widths use it |
| resourceRegionHeight | HUD fixed resource row above abilities |
| chronicleHeight | Log height, first grid row plus12px, drawer/floating-panel offsets and available heights |
| panelPadding | HUD console padding, not every floating panel |
| rowSpacing | stat-grid gap, not all rows |
| abilityRowHeight | ability auto-row, grid row, height and minimum height |
| inventoryRowHeight | inventory item grid row, height and minimum height |
| tooltipMaxWidth | hover-tooltip width, capped by viewport minus16px; not ability dialog width |
| mapSpacing | main-grid column gap |
| buttonHeight | command bar height = value+4; menu-action/item-action button minimum height only |
| buttonPadding | button inline padding |
| borderWidth | button/input/select/drawer/floating panel/hover tooltip/ability-expanded/ability-management borders |
| leftSidebarWidth | Explicitly inert legacy v37 data; old outer HUD column no longer exists; no source control |

Unchanged styles have responsive sidebar breakpoints (23vw baseline,27vw below1100px,260px below900px) and compact-window log styling. Authored changed dimensions override the corresponding CSS baseline. Native viewport responsiveness must be verified separately.

## Interface color consumers

| Field | Browser declaration/consumer |
|---|---|
| primary | root ink: inherited body/control text and explicit primary text |
| secondary | root muted: secondary text |
| muted | root faint: subdued/empty/disabled states unless explicitly overridden |
| headings | root gold plus explicit h2/h3/eyebrow/ability headings color |
| health | root health and danger; health meter fills/labels and danger uses |
| energy | root energy; energy meter fills/labels and costs |
| blessing | root blessing; semantic blessing/god labels/XP label and bar fill |
| border | root edge |
| panel | root panel; drawers/floating/dialog backgrounds |
| background | root bg; page, HUD, target footer |
| hover | root surface-hover; hovered controls/ability/creature rows |
| selected | root surface-selected; active command/inventory/highlighted creature |
| disabled | explicit button:disabled text |
| button | explicit button background |
| tooltip | explicit hover-tooltip and ability-expanded background |
| chronicle | explicit log text |

These overrides, too, are emitted only for changed values. Some source components intentionally retain fixed semantic colors not represented by these controls.

## Evidence and cost

`tools/export-appearance-typography.mjs` extracts53 source CSS cases covering all eight roles, every consumed field individually and combined, chronicle derivation/explicit-track precedence, and inert legacy data. Four native tests compare resolved values to those emitted source declarations, preserve selector-specific reset behavior, and check deterministic font fallback reporting. This verifies the pure consumer contract, not every final native text placement.

Resolving a role reads a constant five fields: O(1) with fixed path depth. Parsing a font family list is O(F²) for duplicate elimination with F bounded by the validated font text length; the expected lists contain fewer than ten entries. Cache resolved styles/candidates per profile change rather than parsing them per pixel/glyph. No helper reads simulation, time, saves, or RNG.

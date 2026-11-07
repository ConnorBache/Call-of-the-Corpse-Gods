# Native presentation and encounter revision — 20 October 2026

## Implemented

- Fixed-page manual: complete authored flavor and concise mechanics; exact stat rows HP / Energy / Accuracy / Offense and Defense / Dodge / DR / Perception. Repeated full-sentence form prefixes are shown once with lossless suffix reconstruction. Every entry fits without vertical scrolling. Browsing controls select other creatures/depths rather than moving content off the page.
- Depth-specific palettes and deterministic creature compositions, large editorial titles, aspect-preserved illustrations and boundary-connected background matte. Bastidion uses the actual selected-floor birth template; companion stats are marked variable rather than fabricated.
- Twenty requested illustration revisions: rat, Pale Jackal, Needleling, Giant Beetle, Witch, horned creatures 3–7, Long-legged Saidribee, Bombardier Beetle, Kelpie, Wendigo, Troll, River Drake, Vampire, Saidribee Guardian, Ursa Minor and Ursa Major. Original masters backed up, prompts and provenance retained. All artwork is embedded, no player setup.
- Whole-screen menu composition, original logo preserved, continuous heavy cut-letter title, text-led navigation and stable high-contrast selection. Embers rasterize at drawable resolution; intentional pixel-art flames remain.
- White path tint with brighter destination, no dark path backing. Observed cells only; hidden simulation data does not reach the renderer.
- Readable remembered terrain, visible-terrain and creature contrast, subtle player backing, fixed sidebar allocation, full ability names ahead of secondary metadata, smooth fading four-and-a-half-line chronicle and text-led navigation.
- Basic Settings accessed through the ? menu: resolution and windowed/borderless/fullscreen modes only. The appearance editor is no longer exposed during normal play. No separate Settings control consumes HUD space. Display preferences persist with legacy migration, actual-mode verification and rollback. Alt+Enter cycles modes.

## Deliberate gameplay amendment

New runs use simulation revision30. Ordinary species eligibility and Ichor event species eligibility extend two floors earlier/later, clamped to1–40. Habitat, floor-one danger restrictions, horde recipes, budgets, paired units, summons and machine triggers retain their rules. Revision29 saves and replays retain original generation, including future lazy-generated floors and replay restart. Existing runs are not silently rebalanced.

Zillennia's deterministic quality selection remains unchanged: its old rule chooses exactly one quality per depth; overlapping expanded bands require a designer choice of selection rule. Godbeast placement and spawning triggers are unchanged.

The independently delivered browser v48 remains frozen. This revision changes the Rust game only.

## Verification record

Targeted tests cover manual complete-content fit, dynamic stats, all81 artwork decodes and aspect sizing, display preference persistence, menu shared input bounds, hidden-world framebuffer invariance, fixed HUD allocation, smooth chronicle wrapping/fade, white path tint and native ember pixels. Population sampling covers320 deterministic plans plus actual generated/populated floors at every depth; original revision29 population/replay fixtures remain unchanged.

Final aggregate, platform-build and interactive results are recorded in NATIVE-VERIFICATION.md after completion. Cross-compilation alone is not a Windows runtime test.

Complexity entries are linked from docs/complexity-ledger.md; rendering includes native drawable area, glyph cache misses and cold artwork decode/matte costs rather than assuming constant cost per frame.

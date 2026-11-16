# Ability detail placement and restored bottom inspection

The ability detail popup now touches the left edge of the Divine abilities rail. Its top aligns with that section, and it follows responsive window dimensions instead of the centered-sheet transform. The existing 0.84 compact text scale remains. Width follows measured text up to the existing readable 480-logical-pixel prose width. Height follows wrapped rows plus padding, without the former fixed 500-pixel empty box. Empty metadata sections are omitted. Long names, costs, descriptions and coverage copy wrap instead of being truncated. Descriptions exceeding the available height scroll, with a small up/down indicator. The panel stops above the bottom inspection line.

The bottom line reuses the already observation-filtered hover report's scene text when no action/error notice is active. Targeting instructions and errors retain precedence. Clearing or changing the hover restores/replaces the old line. Text fits the actual responsive viewport width, and damage invalidation covers the actual full-width footer at portrait and wide aspect ratios.

No simulation, gameplay, RNG, save format or visibility rule changes are involved.

## Complexity

Let T be detached ability text bytes, L wrapped lines, F bottom-line bytes, P affected native pixels, and G painted glyphs. Ability layout performs O(T) width measurement plus existing worst-case O(T²) measured wrapping (including candidate strings and repeated font-metric scans), retaining O(T + L) temporary data. It runs once per detail paint and once per detail damage query while open. Scroll clamping scans L lines, O(L) time/O(1) scratch. Placement and native bounds are O(1), with no persistent cache. Raster cost remains O(P + G plus cold native font-rasterization work); the popup's physical size is bounded by the viewport. Its native damage is the exact panel rectangle, shared by opening, replacing, scrolling and closing.

Footer selection is O(1), borrowing an existing Report rather than performing an extra world observation or path query. Text fitting is O(F log F) font-metric work/O(F) transient storage using the existing measured ellipsis routine. Footer invalidation is O(1), followed by existing bounded tile bookkeeping and O(affected native pixels) repaint/upload. Hidden-information admission remains the inspection adapter's existing responsibility.

## Verification

New deterministic tests cover short/long content, complete wrapped headers, supported/unsupported states, custom fonts, standard/wide/portrait/fractional windows, exact sidebar contact, preserved painter transform/clip, long-description last-page access, repeated open/scroll/replace/close and tile/full pixel equality. Footer tests cover observed text selection, error/targeting precedence, hover text replacement and responsive repaint parity. Screenshot output is enabled with CORPSE_ABILITY_DIALOG_OUTPUT.

Initial candidate build is blocked by unrecovered baseline stat_presentation.rs missing enemy_damage_revision, independently of these UI changes. A disposable harness uses a clearly marked compile-only bridge to the old enemy_damage solely to execute isolated UI checks. This is not a recovered stat implementation or a release validation. The bridge must never be merged. Exact source restoration and a final full locked workspace test remain required.

Isolated verification completed: all five ability-dialog tests (including catalog-wide wrapping), footer selection, compact dialog regression, prior centered-sheet regression, quiet UI and tile-overlay parity passed. Native screenshots were inspected for short, long, authored and responsive cases. Combined list/hallucination/hover regression results are tracked separately by the visible-ground-list change. The source-recovery and release caveats above still apply.

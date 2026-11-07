# Travel / Pack / inspection integration checkpoint

Implemented projection/controller behavior:

- Ordinary map click is source click-to-walk; deliberate adjacent input remains an ordinary Move/attack. Hover previews use the same known route query; replans disable direct-adjacent preference.
- Automation cancels on physical key or pointer command before modal shortcuts. Core commands still pass through the temporal Engine and the source hazard-confirmation adapter. Rejected/confirmation-only inputs do not pre-spend resources.
- Pack is modal, including item-targeted craft/Tinker selection. Item selection cannot leak a map click. A visual stack selects its first eligible physical member, and command emission is single-use until refresh.
- Deliberate Inspect remains a bounded map-column overlay; pointer interactions inside the panel are consumed, and map clicks outside it update the inspected tile without starting travel. Full mechanics is a disclosure in this deliberate inspector.
- Source living-creature hover is pointer-transparent, tile-anchored and immediately discarded when the pointer leaves the map/enters an obscuring panel. It remains available while targeting or while Inspect is pinned; only travel path previews are suppressed. Dead actors retain deliberate inspection but do not produce combat hover. Reports refresh after observation changes even when the pointer stays still.

Evidence: projection/controller tests and source differential fixtures are in Pack, statuses, inspection, dynamic creature abilities and travel modules. These tests do not establish real-window SDL/OS event routing, key repeat timing or actual mouse interaction. The desktop owner integrates the timer and verifies those input branches. Headless rendering verifies detached drawing bounds and clip restoration, not click-through behavior in an OS window.

Narrow compatibility policy: incomplete legacy rememberedTiles entries are unroutable rather than borrowing hidden live terrain. Complete-memory routing matches the frozen source; both the missing-memory policy and hidden-mutation isolation have explicit tests.

Completed targeted presentation gap: `targeting_ui.rs` now supplies knowledge-limited Throw/Shoot traces, and Bow-target hover appends its distinct source ranged forecast. The melee adjacency sections remain separately labelled. No hidden simulation collision tracer is used for the target overlay. See `targeting-ui-complexity.md` for source fixtures and costs.

Final geometry audit: default 1280x800 Pack/Inspect/hover geometry is verified, but owned overlay rectangles/row spacing still use default-profile constants. Authored sidebar width, Chronicle height, command height, tooltip maximum width, inventory row height and related typography/layout edits need a shared detached frame geometry before paint and hit testing can remain aligned. This is an explicit remaining custom-layout gate, not a completed source capability. See `owned-ui-geometry-audit.md`; core input/routing and default-profile overlays are unaffected.

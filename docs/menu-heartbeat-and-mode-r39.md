# Main-menu heartbeat and mode selection — October 30, 2026

This is a presentation-only update to simulation revision 39.

- The original emblem is separated into registered heart, veins and static
  foreground render layers using its exact original pixels. Only the heart
  scales with a modest double beat; delayed blood-pressure waves travel outward
  along the veins behind stationary ribs. No new artwork or sound is added.
- The selected Unblessed / Blessed / Avatar mode is a button beside DESCEND.
  Click, Enter or Space cycles forward; Left / Right cycles backward / forward.
  Tab, Shift+Tab and Up / Down navigate the menu. Disabled Continue is skipped.
- DESCEND opens the existing seed form with the chosen mode displayed read-only.
  The separate DEBUG floor-testing form retains its vessel and floor controls.
- Returning to the menu keeps the current selection in that application session.
  Continue resumes the existing run regardless of the new-descent selection.
- Reduced-motion settings keep the heart and other menu animation still.

The source PNG/WebP resources, game rules, deterministic RNG, save/replay format,
mode effects and authored gameplay copy are unchanged. See desktop-complexity.md
for the frame cadence and inverse-sampling bounds. Windows cross-compilation is
not Windows runtime acceptance.

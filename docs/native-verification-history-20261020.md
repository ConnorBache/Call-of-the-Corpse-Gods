# Historical native verification records through 20 October 2026

This is an archived historical record, not the current release gate. Some old counts and presentation descriptions were superseded. See [current integrated verification](../NATIVE-VERIFICATION.md).

# Native preview verification — 20 October 2026

## Prior accepted baseline (before this overhaul)

- Frozen browser reference: v48, simulation revision29.
- Full Rust workspace release test run:347 passed, zero failures or ignored tests.
- Final desktop-only release regression after UI fixes:147 passed.
- All72 divine ability names execute. Remember covers all44 authored enemy passive copies; Mini-Me covers all416 valid three-god starter combinations and32 offscreen cohorts.
- All59 fixed creature definitions and dynamic Zillennia have source-compared specialist paths.
- Current and selected legacy save/history/Future/haste loading, potions, throws, equipment, crafting, stairs, environmental propagation and generated populations have differential fixtures in the repository.

A test count does not imply exhaustive combinations. See the subsystem coverage files for exact cases and remaining admission guards.

## Actual desktop acceptance

The stripped Linux executable was run through real SDL keyboard/mouse input, using a genuinely generated Avatar run. Tested: menu/seed/mode selection; movement and combat; inventory; heart consumption; level-up selection; illustrated manual and statistics; automatic save/resume; a targeted fire ability; death; save deletion; and the run reminder.

The24-command pre-terminal gameplay state matched the JavaScript reference after replaying the same commands. Temporal representation and nondeterministic run identity were checked separately; UI-only inventory display ordering is not part of command replay equivalence.

The run ended at tick67 on depth1 from fire, with two Blorb hearts eaten and two cumulative Blessings Received. The nondeveloper resumable save was removed, F9 did not resurrect the run, and the high-score/replay records were created. No simulation state was injected for this footage.

## Platforms

- Linux x86-64: built, tested and actually run/captured.
- Windows x86-64: cross-compiled successfully; SDL2.dll bundled. Not run on Windows. The build is unsigned.
- The browser v48 release remains available independently; it was not replaced or modified by this migration.

## Remaining preview limitations

- Developer cheat commands and arbitrary custom NPC divine configurations are not admitted.
- Missing/older simulation revisions and unsupported malformed/custom states fail explicitly rather than being approximated.
- Native appearance/editor presentation differs in some cosmetic layers, CSS inheritance, docked test-scene workflow and exact platform font outlines.
- Performance notes measure simulation dispatch separately from rendering, automatic persistence and filesystem I/O. They are not an end-to-end FPS or Windows speed guarantee.
- A project-wide open-source license remains the owner's choice; third-party notices are retained. No public publication was performed.

## Native UI revision acceptance

The simplified menu, disabled/enabled Continue, mouse hover inversion, seed/vessel form, Avatar movement, and embedded bestiary art were exercised through actual cloud Linux desktop input. Continue retained identical saved bytes both within a live session and after process restart. Artwork loaded from a working directory containing no assets folder. Screenshot rendering at1920×1080 and2560×1440 exercised actual native framebuffer sizes; the cloud interactive window was1180×812. Thin-border downscale, compound glyph, native text/mask, inverse-DPI coordinates, all81-art decode and stable ability total regressions pass.

The Windows executable was cross-compiled with seven icon sizes and a per-monitor DPI-awareness manifest. It remains untested on Windows. All creature artwork is embedded as dimension-preserving quality90 WebP; PNG masters are retained in source. Requested replacements in the current overhaul have backed-up originals and individual generation provenance. The deliverable ZIP needs only extraction and launching the executable, with its included SDL2.dll. No art setup script is required.

## Current presentation and encounter overhaul

- Whole workspace release regression: 380 tests passed, zero failed/ignored (101 test groups). After final desktop-only fixes and basic display settings, 188 desktop tests passed, zero failed/ignored.
- Revision30 new-run population widens ordinary and Ichor event species windows by two floors in each direction; revision29 saves, replays and lazy floor generation retain old behavior. The deterministic sample includes320 population plans and actual generation/population across all40 depths.
- All81 manual entries fit their complete substantive text without scrolling. Exact stat order and aspect-preserving art are tested. Bastidion stats use all40 actual birth templates. Twenty image revisions are embedded with provenance/backups, with10,748,874 encoded creature-art bytes.
- Real optimized SDL/Linux acceptance: main-menu hover/click and keyboard activation, manual mouse selection and depth navigation, genuine Avatar movement and click travel stopping for a newly seen creature, pack overlay, fixed HUD, complete Unyielding Assault name and separate costs, concise Passive state, nonoverlapping Archipelago heading, and ? returning to menu.
- Basic Settings are accessed from the menu, not a separate HUD button or F8 editor. Actual desktop tests exercised borderless, exclusive fullscreen, windowed1024×768, resolution Apply, cancellation without persistence, and borderless persistence across process restart. Alt+Enter changes display mode without advancing simulation. Borderless uses current desktop resolution. No manual art/font setup is required.
- Actual recordings/screenshots include the HUD, menu, basic settings and manual. Headless native renderer checks include960×540,1280×800,1920×1080; the interactive desktop is1364×1024.
- Final Linux and Windows release builds succeeded. Windows remains cross-compiled, unsigned, and not runtime-tested on Windows. Platform-specific display-driver behavior is not certified by Linux testing.
- Zillennia quality bands remain unchanged pending an authored rule for selecting among overlapping qualities. Other conditional event/machine triggers and floor-one danger restrictions remain as documented.

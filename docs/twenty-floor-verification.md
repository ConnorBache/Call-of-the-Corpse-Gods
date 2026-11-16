# Twenty-floor conversion verification (2026-11-16)

- New current constructor enables the twenty-floor route before generation.
- Internal content coordinates preserve existing terrain/population/tiers; displayed depths are 1–20.
- Exact content40 is final physical floor20. No separate content39 floor.
- Godbeasts: 3,5,8,10,13,15,18,20.
- Legacy saves/replays retain prior rules; new seed replay equality verified after one production Wait.
- UI mode selector removed; debug entry accepts 1–20. HUD, summaries, replay filenames, Chronicle, Bestiary ranges and debug stash use display depths.
- Heart stat rewards remain doubled. Blessing awards halve from previous Fast values without rounding. First threshold1, later4,8,16…; Sap fills to next threshold.

Focused native checks: eight new core tests passed (route, milestone mapping, all-floor generation, final stair travel, save/snapshot/future anchor, seed replay, reward/cost/Sap and profile coverage); one legacy route regression passed; eleven menu tests and three compressed UI tests passed.

Windows runtime and screenshot QA were not performed. Broader core suite result is recorded in the accompanying build log if available. No GitHub publication.

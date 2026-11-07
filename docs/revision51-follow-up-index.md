# R51 follow-up index

This index describes implementation scope, not a successful-release certificate.
The final test/build records must match the packaged source snapshot. Partial
checks and earlier screenshots are not substitutes for final aggregate tests.

## Presentation and interaction

- [Quiet HUD and two-way highlighting](quiet-ui-r51.md): left profiles, three
  footer buttons, shorter condition labels, full right-aligned energy metadata,
  smaller corpse markers, ordinary terrain under the player, and native drawable
  pixels in Auto display mode.
- [Combat display units](revision51-stat-display-units.md): Damage bonus, HP
  bonus, DR, Accuracy and Dodge under the resource bars, with contextual units.
- Persistent Bestiary seen/unread state is independent of run saves. Unknown
  pages show a question mark; discoveries add the requested purple Chronicle
  message and yellow unread marker. Vampire uses V unless explicitly customized.
- Jewelry descriptions retain effects and pertinent costs without carriage,
  duplicate, exclusion or no-cooldown boilerplate.
- [Two-press X autoexplore](revision51-autoexplore.md) follows known safe routes
  to reachable unexplored frontiers and retains interruption checks.
- [Aiming paths](revision51-aiming-paths.md) distinguish physical delivery,
  impact, area and unresolved exits, with shared mouse/keyboard targeting.
  Blocked Blink attempts spend no resources; Lightning keeps its piercing rule.
- [Grass and amulet glyphs](revision51-grass-amulet-display.md),
  [native-size ember ink](revision51-ember-glyph.md), and
  [water colors, waves and immersion tint](revision51-water-presentation.md).
  Hallucination icon cadence is halved without changing simulation time.
- [Darkness rendering](revision51-darkness-rendering.md): actual dose, expiry,
  native screenshots and unchanged simulation illumination semantics.
  The later [displayed illumination calibration](revision51-illumination-rendering.md)
  supersedes its brightness curve with the designer's full -100 to +100 targets,
  separated source hue, and diminishing brightness gains above +100.

## Gameplay

- [Restored energy costs and first upkeep](revision50-ability-energy.md) are
  explicit amendments for R50 and newer. Implant Thought retains 15 energy.
- [Ability restorations](revision51-ability-restorations.md): All Seeing Eyes,
  Shine, unlimited I Came For You, and Sparkle/water contact.
- [Whole-number damage](revision51-damage-rounding.md) rounds once after
  resistance and mitigation. [Player base reduction](revision51-player-base-damage.md)
  reduces fixed attack components by 30%, preserving bonuses and enemy attacks.
- [Fungus meals](revision51-fungus-feasts.md) use one in-depth and one eligible
  +2–3-depth corpse, never a Godbeast. [Root cleanup](revision51-root-cleanup.md)
  removes stale branches and unintended purple while preserving shortest paths,
  rooted-corpse revival prevention, and unrestricted later acquisition.
- [Unique Composite bow](revision51-unique-composite-bow.md) is tracked for the
  entire run. [First-four-floor primary loot](revision51-primary-loot.md) uses a
  deterministic two-potion/one-ring/one-amulet floor order.
- [Blocked-enemy rerouting](revision51-enemy-reroute.md),
  [stair reservations and adjacent arrivals](revision51-stair-exclusion.md),
  and [Ichor creature fire immunity](revision51-ichor-fire-immunity.md).
- [Perception and awareness](revision51-perception.md): zero starting Perception,
  literal light/range and doubled creature-distance rules, approved heart rewards,
  wandering/sleeping awareness and persistent hunting.
  [Hover ranges and sight boundaries](revision51-perception-ui.md) use detached
  known geometry and omit uncertain or special-sense contours.
- [Speed-up terrain](revision51-speed-up-tiles.md) uses the shared ordered
  Non-Euclidean topology. Its document records any outstanding label decision.

Except where explicitly documented otherwise, new gameplay rules select R51;
older saves keep their recorded revision. Presentation updates can apply to older
saves. An unchanged revision number alone does not prove identical behavior.

## Evidence and limits

`verification/r51-ui/` holds integration logs and native software-render evidence.
Subsystem folders retain focused tests and reproducible scene captures. Final
workspace and Windows build logs, source snapshot and packaging verification are
created only after the integrated source is frozen. A cloud Linux render is not
a Windows monitor/DPI hardware test; a Windows PE cross-build is not execution.
The reported floor-40 unsupported-behavior error must remain unconfirmed unless
the exact failure is reproduced; passing fire-heavy probes alone cannot resolve
the user's particular run.

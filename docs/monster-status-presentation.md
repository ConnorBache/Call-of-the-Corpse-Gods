# Monster conditions and energy presentation

Designer request: show conditions as comma-separated text (for example,
`Sleeping, Flying`) and omit energy for monsters that cannot use it.

## Presentation behavior

- The left entity rail and Creatures dialog join complete, short condition names
  with commas, then wrap by measured font width. Durations remain in the text.
  The rail does not repeat a state such as Sleeping when it is already a condition.
  Empty condition lists add no blank row. Player condition meters are unchanged.
- Inspection text and hover use comma-separated conditions too, retaining their
  existing compact labels and turn counts. The full details and hover both omit
  an incapable monster's energy resource.
- The rail and Creatures summary omit the same energy label and bar. A summary
  with Health alone gives it the available width. Shared-player bodies retain
  their existing shared-resource treatment.
- Bestiary BASE STATS removes Energy only when the public, revisioned birth
  templates cannot use it. Its actual HP, damage, and other stat ranges remain
  unchanged. Character-dependent/unsupported templates still say Varies rather
  than pretending to know their capabilities. Bestiary explanatory text and
  ability-cost descriptions are unaffected.
- Retained-image and tile-damage keys include the capability flag, so erasing or
  acquiring the last paid power updates the rail even if no numeric stat changes.

No actor pool, regeneration rule, cost, AI decision, serialized field, RNG state,
save format, simulation revision or gameplay mechanic changes.

## Capability source audit

`monster_status_ui::energy_capable` is presentation-only and accepts a detached
Actor and simulation revision. It does not query the world by actor ID. It reads
actual executable abilities, rather than treating max Energy, caster classification,
or the presence of any arbitrary power as evidence of spending.

| Capability | Executed source |
| --- | --- |
| Pale breath boost, gas shot, scream, birth | `ai::enemy_action_options` |
| Pale lifesteal and repeated bite | `engine` post-strike callbacks |
| Bloodmancy/Witch curses; Dryad healing | `monster_spells`; Witch also summons in `witch` |
| Queen summoning from R33 | `saidribee_queen::summon` |
| Bombardier detonation, including an empty current pool | `monster_bombardier::special_action` |
| Approach blink, charged moonbeam | `monster_moon::special_action` |
| Water jet | `monster_grab::water_jet` |
| Ruin pulse | `monster_ruin::special_action` |
| Beckon, wall seal, horn bolt | `monster_horn::special_action` |
| R52 Dragon flight | `revision52_dragon_flight::turn_inner` and `revision52_enemy_followups::species` |
| Copied active maintenance | `abilities::pay_upkeep` / `upkeep_cost_for`, including dynamic Unspeak |
| Copied active Nova, Erase Metaphysical, counterattack | `engine` combat callbacks |
| Copied active Never Surrender and Rage | `divine_reactions::pay_surrender_from`, `divine_rescues::try_rescue` |
| R50+ copied active Execute and Commandment | `revision50_combat` |
| R52+ acquired passive Hanged | `hanged::protects` and `engine::apply_hp_loss` |

The innate branch list was cross-checked against every `enemy_spend` and
`spell_spend` caller, and the paid entries in `enemy_traits::catalog`.
Double Birth by itself does not enable the Birth branch. Hold-only blocks the
same special-action group as the real dispatcher. The existing legacy trait
inference is used only before trait initialization, so erased powers are not
silently restored in the UI.

Free monster fire breath, permanent flight, Vampire bat form, regeneration,
energy-capacity passives and the R50+ stats-only Mini-Me do not justify an energy
bar. Monsters do not issue the player's ability-activation command: merely
owning an unused player-only action is not enough. Executable copied active
powers and passive Hanged do count. Temporary sleep, paralysis, silence, lack of
a target, cooldowns, zero energy and current wounds do not remove a capability.

## Observation boundaries

Live presentation first admits actors through the existing visibility rules and
then passes their apparent bodies to capability projection. A Skin Walker or a
hallucination uses the apparent body's abilities, never a lookup of the hidden
original. Every detached hallucination phase stores its own capability flag.
Sensed actors keep the existing sensed presentation; hidden and remembered-only
tiles gain no creature, resource or capability data. Bestiary data comes only
from public birth constructors, not actors currently in a run.

## Complexity and bounds

Let K be the number of an admitted actor's power IDs, L the maximum relevant
name/ID length, D the fixed enemy-definition count, T the displayed condition
characters, C the admitted sidebar creatures, and F the public Bestiary birth
floors (at most 40).

- Capability projection runs once per observed creature/variant or public birth
  template. It performs O((K+D)*L) expected time through bounded catalog scans,
  legacy inference, hash-set de-duplication and string comparisons, with O(K*L)
  temporary storage. Hash collection costs assume their existing expected-time
  implementation; a new cold catalog load also parses the fixed bundled JSON.
  There is no additional population, visibility, pathfinding or floor scan.
- Joining labels is O(T) time/storage. Measured word wrapping remains O(T²)
  worst-case, including long-word splits and repeated font measurement, with
  O(T) temporary text. Layout/scroll/hit-testing and rasterization use those
  same wrapped rows. The rail's total layout is O(sum(T_i²)) over C admitted
  creatures; it does not inspect hidden population.
- Dialog hit-testing recalculates the same bounded-width text geometry as paint,
  O(sum(T_i²)) through rows preceding the selected point. No stale fixed meter
  heights remain. Dialog scroll semantics are unchanged (whole-creature offset);
  the rail retains pixel scrolling for cards taller than its viewport.
- Bestiary adds O(F*(K+D)*L) capability work to its existing public birth/stat
  range derivation; maximum F is 40. Existing birth constructor, revision tuning,
  cold admission caches and rendering costs are documented in
  `revision52-bestiary-stats.md`. No additional persistent cache is introduced.
- Font cache misses still incur native rasterization, and painting costs include
  the clipped glyph/bar pixel area. No empirical speedup or new GPU/platform
  claim follows from these estimates.

## Verification

Focused deterministic regressions cover paid versus free species, zero energy
and temporary incapacitation, R51/R52 Dragon differences, Queen revision gates,
legacy Pale inference and erased traits, copied powers and Hanged, Vampire form,
R50+ Mini-Me, actual paid Dryad healing, comma joins, empty and long conditions,
measured wrapping, retained and tile-only redraw, dialog hit geometry, public
Bestiary rows, hidden/remembered/sensed actors and all hallucination variants.
Existing source-fixture inspection checks and Bestiary birth/range tests remain
in place with their intentional display expectations updated.

Set `CORPSE_MONSTER_STATUS_REVIEW` to export native software-rendered rail and
dialog captures at 960×720, 1280×800 and 1920×1080, a long-condition dialog, and
Rat/Witch/Dragon Bestiary pages from the focused capture test. These are native
rendering checks, not evidence of interactive GPU or Windows verification.

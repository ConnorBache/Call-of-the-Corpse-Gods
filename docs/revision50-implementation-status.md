> 2026-11-06 inventory correction supersedes older worn/shared capacity notes below: five sections; rings, amulets, potions and hearts base five each; unlimited trinkets; all carried jewelry active and counted. Current implementation/check scope: revision50-carried-inventory.md.

# Revision 50 implementation checkpoint

Revision 50 new-game, startup/storage/replay and deep-floor paths are implemented
and host-tested. The Windows executable cross-builds successfully. It has not been
launched on Windows; see the exact verification limits below. Start a new game
for R50 rules; existing saves keep their original revision.

## Authority and preservation

The base is the verified Revision 46 UI recovery, including its eighteen UI
overlays. The uploaded request remains verbatim in `revision50-request.txt`.
Later explicit designer decisions supersede it. The original legacy catalog,
Cargo.lock and Rust 1.99.1 pin remain preserved. Older revisions retain their
rules; this work does not silently migrate an existing run to R50.

## Implemented source scope

- Versioned 72-blessing catalog, nine authorized description amendments and
  explicit starter replacement identities with unchanged pool weights.
- New combat, movement, pursuit, terrain, Nightmare, Clay and stats-only Mini-Me
  paths, plus revised preserved blessings. The player uses Greatsword arithmetic
  (45 + twice effective Offense), without arbitrary enemy retuning.
- Fourteen named rings with fixed first-build effects. Duplicate powers do not
  stack except Light, whose copies each add 10 to other LOS tiles, including
  Darkness Aura bearers. No quality-tier strength or rarity is invented.
- Twelve named amulets with actual Apply, previews, payment, damage/status,
  displacement and terrain effects. No amulet cooldowns; enemy cooldowns and
  recovery remain. Appropriate reduces amulet costs by 10, minimum 0.
- Each acquired blessing starts with 50 energy and refills to 50 on level-up.
  Blessing and amulet payments spend their own energy first, then the player
  pays any remainder, atomically. Dormant item grants keep their charge.
- Amulets must be worn to activate, start with 50 energy and never refill on
  level-up or resting. Reaping accumulates direct hostile HP damage and returns
  one energy per 20 damage to every owned amulet, including backpack items, capped
  at 50. Reaping never restores player or blessing energy, and ally damage, self
  damage, overkill, periodic damage and environmental damage do not count.
- See The Future spends all its own reserve and all current player energy,
  leaving other reserves untouched; its temporal and UI wiring is integrated
  for testing. Paid snapshots and actual release round trips need verification.
- Cogworks five-cell wall has one 200-HP pool; all cells are restored when it is
  destroyed. Floods lanes stop independently at water. Witch-Hatred's caster
  group includes Witch, Bloodmancer, Dryad, all horn creatures, Saidribee Queen
  and Valta. Bastidions always produce hallucination gas in R50.
- Twenty ordinary potions, approved one-sentence descriptions, special Sap
  preservation, permanent Freezing/Iceblast ice and heat melting. Item names and
  effects are visible in actual Pack/map hover without use-identification.
- Unlimited trinket/special-item capacity, separate from unchanged finite jewelry,
  potion and heart budgets. Bows, arrow stacks, tools, quest shards and special
  usable items retain their original identity kinds. Worn jewelry is unlimited
  and outside backpack accounting. The mask is displayed as Masque of the
  Trickster. Ordinary unequippable melee/armor drops are removed; new named jewelry
  is generated through ordinary and special reward paths with equal chances
  within each jewelry group.

## Verification and platform limits

The final full workspace run passed 1975 tests with 0 failures and
38 ignored tests across 188 target summaries. Generated starts,
save/replay, Future/Timeweave and deep-floor acceptance passed. Sixteen native UI
captures were inspected. Source/build hashes and exact commands are in the
native-verification report and windows-release receipts.

All requested mechanics have implementation paths. Repulsion at the exact center
uses the documented provisional random-valid-direction default. Shared Cogworks
wall attacks and Shattering use the documented whole-wall behavior.

The Windows x64 executable is cross-built and statically inspected; no Windows
launch/input/rendering/save check is claimed. Full campaign playthrough and broad
balance/performance certification are outside the completed verification scope.
The source preserves the R46 UI draft, pinned compiler/dependencies and older saves.

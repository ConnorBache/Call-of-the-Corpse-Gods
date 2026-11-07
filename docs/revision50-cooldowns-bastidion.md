# Revision 50: amulet-only cooldown scope and Bastidion gas

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

## Current designer direction

The latest clarification is **“I mean amulet cooldowns.”** Only amulet cooldowns
are removed. “Bastidions should always make hallucination gas” separately makes
Bastidion output hallucinogenic at simulation revision 50. Earlier revisions
retain their existing mechanics and deterministic generation streams.

Amulet accounting, Appropriate's 10-energy cost reduction, and Reaping are
tracked in their own revision-50 subsystem documents. This change does not
change their implementation.

## Cooldown and clock audit

These **non-amulet mechanics remain unchanged**, including their admission gates,
successful-use state writes, saved values, UI descriptions, and timing:

- Masque of the Trickster Teleport (legacy Mask of Trickery identity retained): `ahvamittTeleportReadyAt`, 100 world turns.
- Bloodmancer/Witch spells: `monsterSpellReadyAt`, 3 world turns.
- Dryad healing: `monsterHealReadyAt`, 2 world turns, including the ally wake gate.
- Fire breath: `monsterFireReadyAt`, 4 world turns.
- Witch, Saidribee Queen and Pale birth: `monsterSummonReadyAt`, 10 world turns.
- Beckon, Wall Seal, approach Blink and Ruin Pulse: shared `beastCooldown`,
  decremented by the existing actor schedule.
- `rangedRecovery`: the existing ranged recovery action, including the separate
  offscreen path and normal/haste scheduling.

Other clocks are also unchanged: Moonbeam charging, Minotaur charge movement,
Wall Seal's existing-wall lifetime, Banshee's 30-turn shriek cycle and warning,
Sphinx provocation delay, Waking Nightmares' 100-normal-turn charge generation,
status lifetimes, energy regeneration/rest, effect durations, speed modifiers,
and once-per-turn accounting. They are not amulet cooldowns.

## Bastidion implementation

`population::create_population_actor_revision` covers `bastidion`, `fungus`, and
`vent-fungus` aliases. At revision 50 it writes the fixed serialized gas marker 6
without consuming a generation RNG draw. Historical revisions still consume
exactly one draw for their original six-way gas selection. Direct
`create_actor_revision("bastidion", ...)` uses the same fixed marker.

`fungi::emission` is the authoritative output choice. Revision-50 Bastidions emit
`hallucinogenic` regardless of a stale gas marker retained in an existing save or
copied actor. Species identity and legacy Bastidion/Vent Fungus names are
recognized; unrelated fungal actors retain their authored gas selection.
Hallucinogenic roots use the existing gas color `#ee6cff`; retained Bastidion
root connections are recolored consistently. The original root path, acquisition
range, revival prevention, and output of 25 units times one plus the connected
corpse count remain unchanged.

The active environment step and precise inactive-floor catch-up both execute
`tick_fungi`. No separate offscreen choice or random gas path exists. Gas channels
and actor markers use existing serialization and temporal snapshot storage.
The change introduces no new wire field, schema migration, effect, or action.
Compact and detailed observed-creature descriptions identify hallucinogenic gas
at revision 50; historical descriptions are unchanged.

## Complexity and limits

Let F be live gas-emitting fungi, C corpses, A total actors, B total actor trait
bytes scanned, S bytes in cloned fungus/corpse actors, K retained root connections,
P total retained root-path cells, N floor cells, and L the cost of the existing
revision-aware line-of-sight query. Let W be the existing first-write floor COW
cost, Q gas-channel preparation/emission cost, and J total source identity bytes.
These are per active environment turn or per precise inactive-floor catch-up turn.

- Fixed birth selection adds O(1) time and memory beyond the existing actor
  template clone and condition-map insertion. It removes one RNG draw at R50.
- Gas identity selection is O(J + F) with expected constant-time condition lookup.
  It adds no geometry, sorting, ray cast, or random draw.
- The R50 source-membership set and retained-root recoloring add expected
  O(J + F + K) time and O(F) temporary memory. Strings assigned to changed roots
  have a fixed seven-byte color. Hash-set expected costs assume normal hashing;
  adversarial collision worst-case for this addition is O(F² + K×F + J).
- Including the original algorithm, expected time remains
  O(A + B + S + J + F + C + P + K + F×C×(11 + L) + F×Q + W).
  Root paths have at most 11 cells after the existing 10-tile radius check.
  Existing pair/set lookups use their normal expected O(1) hashing; a conservative
  all-collision bound replaces their work with O((F + C + K)² + F×C×K).
- Scratch/retained additions are bounded by existing actors and connections:
  O(S + F + C + K + P) in the full acquisition pass. Initial roots/gas channels
  can allocate O(N); mutating a shared floor includes its existing W-byte COW
  detach. Emission calls retain the existing Q cost, including any whole-channel
  preparation and gas ownership bookkeeping. This is not claimed O(1) per floor.
- Observed-creature text adds only fixed prose plus the identity check. Existing
  wrapping, font cache misses, glyph rasterization and native frame upload costs
  are unchanged. No filesystem or rendering work enters simulation.

These are asymptotic analyses, not measured timings. Profile with many fungi,
corpses and retained roots on linked floors, including cold gas-channel allocation,
shared temporal snapshots, and multiple inactive turns, before making performance
claims.

## Regression targets and verification status

`crates/corpse-core/tests/revision50_bastidion.rs` covers:

1. All three birth aliases, three depths and 32 seeds at revisions 29, 46 and 50;
   exact old RNG advancement and no R50 gas draw; direct Bastidion birth.
2. Active emission across every stored legacy gas value, exact historical
   mapping, fixed R50 output, and no emission RNG draw.
3. Corpse-root acquisition, output volume and recoloring retained root metadata.
4. Inactive-floor emission after storage, future restoration and snapshot restore,
   with exact repeated floor/RNG results.
5. The clarified amulet-only scope: Mask Teleport and Witch spell/summon cooldowns
   still reject early attempts without mutation at R50.

Desktop regression:
`creature_abilities_ui::tests::revision50_bastidion_copy_names_fixed_gas_without_removing_enemy_cooldowns`.
It checks fixed gas prose, unchanged historical prose and retained Queen cooldown
metadata without mutating the run.

Native execution of these new tests is pending. Existing historical differential
suites and final aggregate checks remain required. These tests do not claim native
Windows verification.

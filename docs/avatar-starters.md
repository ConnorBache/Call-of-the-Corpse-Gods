# Current Avatar starter policy 1

New production Avatar runs draw uniformly from **complete valid tuples**, one of
all six current blessings for each selected god. A tuple must contain **at least
one Direct Combat blessing and at least one actual energy spender**. The same
blessing may satisfy both. Two or three combat blessings are permitted. There is
no independent per-god weighting, curated starter restriction, or reroll-until-valid
loop. God/slot order is retained, as is current catalog order within each god.

`avatar_starters.rs` contains the explicit, independently source-audited 26 Direct
Combat and 37 energy-spender IDs. These are rule memberships, not description-word
heuristics. Activation, upkeep, and triggered energy payments qualify. Important
identity/payment details:

- `isheilah-missed-me` is current Hanged: damage redirects to actual main energy
  at 4 energy per HP (`hanged.rs`), so it qualifies, despite the historical name.
- Ragelight's first rescue is free but later rescues debit energy; Execute costs
  10 in current R52 (`engine.rs`, `revision50_combat.rs`).
- Fated Duel and other upkeep abilities qualify (`abilities.rs`, `revision50.rs`).
  Unspeak qualifies when the target has a positive-cost ability, even though it
  can be free for a target with none.
- Never Surrender is the current `pelian-never-surrender`: 8 energy on its damage
  trigger (`divine_reactions.rs`). Never Ending Flesh is not this source lineage.
- `anavani-illuminate` is Sparkle; its 20-energy payment qualifies.
- Current Futuresight is free and does not qualify; neither do blood, HP,
  reactions, fortunes, energy gain/capacity, or amulet-cost reductions.
- Cogworks/Clay Army use actual 20-energy payments (`revision50_terrain.rs`),
  Tinker 100 (`tinker.rs`), Commandment 20 on eligible hits
  (`revision50_combat.rs`); other paid activation/upkeep routes are in
  `abilities.rs` and proc routes in `engine.rs`/`divine_reactions.rs`.

All 220 unordered three-god combinations have valid tuples: minimum 33 for
Numo/Etill/Grisha and maximum 210 for Pelian/Numo/Lichor. Across them, 1,747 valid
tuples contain three Direct Combat blessings. Tests enumerate all 216 raw tuples
for each combination and all six god-order permutations, not a sampled subset.

## Determinism, RNG and compatibility

The saved/exported immutable `avatarStarterPolicy: 1` selects this policy before
startup. Missing means the historical curated starter path. Existing saves are
not retrofitted; historical revision/Fast constructors, synthetic playtest/debug
profiles, and Blessed/Unblessed runs retain their existing behavior. Unsupported
policy numbers or policy on non-Avatar/historical/synthetic records are rejected.
Rewinds preserve the live marker rather than rolling back startup provenance.
Future changes to eligibility, candidate order or sampling need a new policy.

Selection uses a domain-separated hash of the exact canonical full seed. The old
three global starter RNG advances are retained, so the number of rejection draws
cannot perturb the global dungeon-generation stream. This does not claim that
changing acquired abilities can never legitimately change subsequent simulation.

The xorshift generator's nonzero population has **2^32−1**, not 2^32, outputs.
Subtract one, discard the short remainder above the largest multiple of the valid
tuple count, then take the remainder. Every accepted index has the same number
of raw outcomes. With N <= 216 tuples, at most N−1 states in the full nonzero
cycle are rejected; its no-repeat property guarantees acceptance within N draws.
Zero RNG state and invalid bounds are rejected, never repaired with biased modulo.

## Complexity

Let A=72 catalog entries, B=6 blessings/god, D=26 combat IDs and E=37 energy IDs.
Once per new shell: O(A + B^3*(D+E)) membership/enumeration plus O(N) worst-case
RNG steps, N<=216. Extra space O(A+N) references and three copied IDs, plus O(seed
bytes) for domain hashing. Current ID/seed hashing includes their byte costs;
no floor scans, global retries, gameplay mutation or RNG draws occur in the
eligibility checks. Existing floor generation, reserve initialization, and replay
startup costs are unchanged. Bounds are source analysis, not measured timings.

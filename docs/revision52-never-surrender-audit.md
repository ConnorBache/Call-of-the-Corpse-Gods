# Revision 52 Never Surrender: DR eligibility and preview parity

## Designer decision

Current description, verbatim:

> Increase your DR for the first source of damage every turn.

The designer clarified that Never Surrender does not override DR bypass, and
must not waste its activation on damage that ignores DR. In revision 52 it waits
for the first otherwise-damaging source that uses DR. The existing toggle, 8-energy
cost, Defense-sized DR bonus and once-per-turn limit remain. Earlier revisions
retain their gameplay and wording. The displayed cost remains separate from the
requested concise description.

## Execution boundary

Ordinary melee and ranged attacks share the same strike pipeline. On the first
eligible landed strike in a tick, add Defense to DR before attack scaling and
rounding, and pay 8 from the ability's own reserve before main energy. Revision 52
caps the reserve at 10. Defense is one sixth of the defense-derived HP bonus, not
one sixth of total max HP. There is no low-HP threshold.

- Armor-piercing strikes bypass all DR. In revision 52 they neither spend energy,
  consume the tick marker, nor deactivate an underfunded toggle. The next normal
  melee or ranged hit in the same tick can still trigger Never Surrender.
- Direct spells, environmental/status packets and Bonded damage currently bypass
  DR in `damage_packet`. They do not call Never Surrender or consume its marker.
  This matches the clarified rule; no new DR is applied to those packets.
- Shard and terminal Execute hits already bypass the bonus and its payment.
- Misses and ordinary unprotected zero-damage hits do not pay. A strike reduced
  to zero specifically by the Never Surrender bonus still costs 8.
- Zero or negative Defense is not silently clamped. Payment admission still uses
  the ordinary unprotected strike, preserving existing signed-stat semantics.

Toggling is free, takes no turn, and does not reset the current marker. An
insufficient eligible payment gives no bonus, spends nothing, and deactivates the
toggle. Half Here shares the owner's reserve and marker. The sleeping Dreamwalk
body does not inherit the projection's active powers.

Never Surrender is not a rescue. Its mitigation and payment precede Hanged's
main-energy conversion and ordered death rescues. No Hanged, Rage, health/death,
rescue-order, action/regen or temporal implementation changed.

## Confirmed defects and fixes

1. The preview checked only main energy >= 8, ignoring ability reserves. With
   Defense 12, a funded reserve, zero main energy and a base-40 attack, it showed
   15–25 damage instead of 9–19. Split funding also failed; a missing reserve with
   high main energy incorrectly showed protection. Preview now calls the actual
   read-only `surrender_bonus` admission helper.
2. The Hanged preview used main energy before Never Surrender's payment. With
   1 HP, 8 main energy, no reserve, Defense 12 and a base-20 attack, it showed
   0–1 HP loss although every landed hit was lethal. It now subtracts Surrender's
   main-bar payment at each variance endpoint before Hanged and includes both
   costs in total energy loss. Zero ordinary damage and DR bypass do not pay.
3. Armor-piercing strikes charged and consumed the proc despite providing no DR
   benefit. An R52-only guard in `pay_surrender_from` fixes this; all older
   revisions preserve their original payment behavior.

Actual ranged reproduction at distance 3, base 40 and Defense 12: ordinary arrows
and other projectiles dealt 24 without Surrender versus 18 with it. Piercing
versions dealt 24 either way. Before the correction they wrongly spent 8; after
it they spend 0 and leave the proc available.

The exact description is shared by current ability rows, offers, item-granted
ability details and copied creature detail. Legacy catalogs are not rewritten.

## Complexity

Core payment adds one R52 source ability-list scan, O(K) time and O(1) extra
memory for K source abilities. Existing owner/marker/payment work is unchanged.
No RNG, damage scaling, turn advance, native rendering, cache or traversal is added.

Preview reuses owner/endpoint and reserve admission: O(A + K + L) expected time
per endpoint, where A is current actors, K active powers and L ability-ID bytes;
reserve-map collision worst case is O(B*L) for B reserve entries. Payment planning
allocates O(L) temporary ID bytes, without committing or mutating the run.

There are two endpoints per ordinary range. Only the Hanged/Surrender combination
with a main-energy contribution adds a payment plan and two unprotected
`strike_damage` queries. Those queries include existing effective-stat equipment,
condition, divine-modifier and source-ability work, not just O(1) arithmetic.
Existing full inspection/combat bounds and detached offhand costs still apply.
This is source analysis, not a measured timing claim.

## Verification scope

The isolated native diagnostic first reproduced three of seven payment/revision
preview mismatches, then verified all seven matched after the preview change.
Five new engine regressions reproduced three failures against the old core and
passed against a fresh isolated build of the patched current core. They exercise
normal/piercing ranged attacks, bypass then normal in one turn, direct and status
packets, zero/full blocking, underfunded toggles and legacy R49–51 behavior.

Further tests cover exact copy in UI consumers, read-only reserve-aware forecasts,
Hanged payment order, shared ownership and sleeping-body exclusion. Test outputs,
source hashes and commands are retained in the local test report. Isolated rustc
builds do not replace the required final canonical Cargo workspace suite, SDL input
verification or exhaustive temporal/ability-combination proof.


Measured isolated checks against the rebuilt patched core: 35 inspection tests,
24 ability UI tests, 11 creature ability tests, the 5 new engine regressions,
and 46 existing reaction/combat/Hanged/rescue tests passed (121 tests total).
The latter include 144 legacy rescue vectors and 64 damage-bank vectors.

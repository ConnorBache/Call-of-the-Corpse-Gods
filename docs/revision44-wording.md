# Revision 44 — authored ability wording

These are presentation-only replacements for revision 44 and later. The canonical ability catalog remains unchanged: offer tags, eligibility, costs, durations, RNG and simulation do not consume these overrides. Revision 43 and earlier retain their existing presentation. The separately implemented revision-44 bleeding mechanic is not defined or changed by this document's display adapter.

`ability_ui::display_description` is the common adapter. Ability/detail rows and blessing choices use `ability_ui::observe`; item grants use the same adapter from `pack::describe_item`. Omitted costs, durations and shared-clone-energy details are not removed from mechanics. The existing revision-43 Fury override remains.

## Exact requested descriptions

- `shrail-fight-long`: Heal 1/5 the damage you deal in melee.
- `shrail-beat-to-death`: Reduce target dodge by 1 and DR 5 when you hit an enemy until they die.
- `pelian-bulwark-of-mind`: Spend 20 energy and select a visible, unoccupied square. Enemies cannot enter it for 10 turns.

- `lichor-the-number-of-blood`: Every 5 turns, automatically hit and do double damage.
- `grisha-boundless-hunger`: You have no max energy.
- `dawel-reverse-entropy`: While active, enemies do not heal, regain energy, or revive, plants do not grow and neither gas nor fire spreads within 10.
- `morae-bonded-by-unseen-threads`: On activation, select an enemy, when you take damage, they take half.
- `morae-see-the-future`: Spend all your energy to save your current state. Deactivating will return you there, undoing all that has been done.
- `numo-mind-read`: While active, you have telepathy within 20. While inactive, you have telepathy within 5.
- `etill-float-away`: When active, if an enemy damages you, fly until your next turn and move 1 tile away.
- `ursa-dream-see`: You have telepathy with sleeping creatures. When active, you have telepathy with all creatures that are not obstructed.
- `kative-dilute-potion`: When you use a potion, you can use it again at half efficacy.
- `pelian-i-came-for-you`: Select an enemy. You gain telepathy on that enemy and if a move would bring you closer to the enemy, you can take 1 free turn afterward.
- `isheilah-make-slippery`: Select an enemy, when they try to move, they don't and you spend 8 energy.
- `lichor-bleed-them`: Your attacks inflict bleed proportional to your offense.
- `morae-fortune-teller`: You can have up to 3 fortunes. Spend a fortune to give yourself lucky, ensuring you hit with your next attack, or an enemy creature unlucky, ensuring they miss.
- `etill-half-here`: Activate to put down a half clone within line of sight that shares your health pool, doesn't move, but provides you sight from that location. Deactivate to remove it.
- `pelian-never-surrender`: Spend 8 energy and add your defense to your DR the first time you take damage every turn.

## Unchanged requested entries

Shadow in the Dark; Mini-Me; Rage at the Dying of the Light; Implant Thought; Uncast. Timeweave and the prior batch's other descriptions are unaffected.

## Complexity and verification

Let L be the ability ID byte length and T the displayed text length. The fixed eighteen-entry match takes O(L) time with a fixed bounded number of comparisons, O(1) auxiliary space, and returns borrowed static text. It runs only when ability/item/offer descriptions are projected, not per simulation turn or cosmetic hover. Existing row ownership copies the selected text in O(T) time and memory; no extra catalog scan, JSON conversion, whole-Run clone or renderer invalidation is introduced.

Two focused tests compare all eighteen exact strings across revisions 42/43/44, actual ability rows, item grants and blessing offer projection; verify unchanged catalog and Run bytes; and preserve the five approved unchanged entries, unknown-ID fallback and prior Fury presentation. Tests are staged for the lead's gate; no execution claimed here.

## Additional designer edits (2026-11-03)

The same display-only revision 44 adapter now supplies the exact Fight Long, Beat to Death and Bulwark of Mind wording. The catalog, offer tagging, selection order, mechanics and saved revision remain unchanged. The existing exact-copy tests now cover all 18 overrides across revisions 42/43/44 and verify unchanged catalog/Run bytes. Complexity remains a fixed bounded ID match and borrowed string, with no new allocation in the adapter.

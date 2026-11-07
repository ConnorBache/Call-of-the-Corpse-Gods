# Revision 34 — Phantasmii, Epidite and Ahvamitt

Native preview. New runs and DEBUG floor preparation use simulation revision 34.
Explicit revisions 29–33 keep their versioned generation/balance rules. Original
bestiary artwork and lore are unchanged; Nightmares have no manual page.

## Phantasmii

Apply the Dagger of the Phantasmii while on its altar. Your sleeping body stays
behind; the projection has Dreamwalking and Flying. Damage to the body affects
your shared health and notifies you, **without waking you**. A lethal shared-pool
packet resolves death once. Wake Up returns to the anchor and disappears. All
three shards are on this floor, are real stacked inventory items, and permanently
unlock the Dreamwalk toggle after collection. Early waking preserves shards.

Four persistent Nightmares are generated once per actual floor, outside ordinary
population budgets. They are always In mind to a dream observer; only their native
plane can fight them. They leave no corpse, loot, heart, experience or revival.
They use these centralized provisional body-stat baselines (no species powers):

| Floors | Nightmare descriptor | Ordinary body baseline |
|---|---|---|
|1–5|Small|Bear|
|6–10|Lesser|Long-legged Saidribee|
|11–15|Minor|Greater Pale Biter|
|16–20|none|Dire Bear|
|21–25|Major|Killspawn|
|26–30|Greater|Phoenix|
|31–35|Gargantuan|Ursa Minor|
|36–40|Terrible|Dragon (finite ordinary endgame cap)|

Numerical receipt (HP / landed melee damage against 0 DR / DR / Accuracy / Dodge):
Small 136 / 21–35 / 0 / 6 / −9; Lesser 190 / 63–105 / 0 / 45 / 27;
Minor 310 / 54–90 / 0 / 108 / 26; plain 448 / 75–124 / 15 / 124 / 9;
Major 514 / 225–374 / 25 / 140 / 31; Greater 718 / 225–375 / 0 / 158 / 56;
Gargantuan 880 / 234–390 / 50 / 160 / 53; Terrible 1018 / 559–931 / 83 / 179 / 62.
All have normal speed. Lesser's damage exceeds Minor's because the sampled
ordinary profiles differ; no silent monotonic retuning is applied.

This is not an all-floor balance certification. Active powers follow the current
player projection; Half Here remains on its creation plane, including after wake,
and retains that plane's interactions/perception. Targeted player powers cannot
use opposite-plane clone vision to cast through planes. Normal creatures can
occupy the projection's location. Hover explains that overlap. Normal-world
presentation has a dark-blue cosmetic flicker; animation never advances turns.

No stairs, ascent, consumables, crafting, throwing, dropping or normal-world ground
pickup while dreaming. Shards are the quest exception. Carried nonconsumable
weapons/equipment and normal powers remain available. Flight does not pass walls.

Dream geometry is initialized from the floor and subsequently persists separately;
spell-created dream fire, pits and walls do not mutate waking geometry. Waking
hazards continue to affect the body. The two environmental passes filter contacts
and do not duplicate creature upkeep, fungus/fish world events or item drift.
Already-inflicted bleeding/poison remains victim-local after its source changes
plane. Normal and dream floor data, body/clone identity, quest inventory and
unlock state are serialized and restored by Revert/Future/session saves.

## Epidite

A stationary solid tower pulses every 30 normal world turns, including visited
inactive-floor simulation. The visible numeric subscript reports the next pulse;
render frames do not change it. Its waking-plane line of sight deals 100 damage
through DR, retaining ordinary defensive/rescue/death rules. Sleeping bodies can
be hit; dream projections cannot.

Exposed ground hearts are Blessed once, marked with a persistent binary flag and
separate stack identity. Stats, experience and energy rewards double once;
repeated pulses never redouble. Carried inventory is not exposed. Crafted stats
inherit the doubled heart bonus through existing recipes; fixed craft abilities
are not duplicated.

## Ahvamitt

Stationary, nonattacking trickster. The first two player attack attempts advance
the dialogue and teleport him to a legal reachable empty location. The third
plays the reward dialogue, drops one Mask of Trickery and removes him outright:
no death corpse or extra loot. Hit-roll success is not required. Completed event
state prevents repeat rewards; direct allegiance-changing powers cannot bypass
the interaction.

Carrying (not equipping) the mask grants Teleport to a random standable unoccupied
spot on this floor. Its 100-world-turn cooldown persists independently of the
item, so drop/repick cannot reset it. A dream teleport moves only the projection,
never the sleeping anchor. Removal immediately removes the granted control.

## Schedule, DEBUG and verification boundaries

The seven Godbeasts occupy floors 5/10/15/20/25/30/35 once each; Administrator is
floor 40. DEBUG reconstruction does not grant prior Phantasmii quest items or
Nightmare rewards. Save/session export preserves its actual loadout. DEBUG's
zero-turn stash edits are not a command-ledger replay format: such seed-replay
imports explicitly direct the player to use the saved session instead.

See [the verification receipt](verification-r34.md) for exact test commands/counts and
actual native-renderer screenshots. Linux native/offscreen renderer verification
is distinct from a Windows cross-build, and neither establishes Windows runtime
verification. Do not redistribute an old executable as revision 34.

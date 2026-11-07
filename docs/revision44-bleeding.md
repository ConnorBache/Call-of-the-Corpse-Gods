### Revision44 Bleed Them
Only `divine_reactions::apply_bleeding` produces the bleeding status in gameplay.
It remains positive-landed-hit gated and stores max(old duration, nonnegative
attacker offense), not an additive stack sum. Source attribution remains the
existing `bleedSourceId`. Serialized/admin bleeding follows the same revision44
status policy; no extra provenance field or migration is introduced.

At each existing foreground/upkeep or exact offscreen tick, revision44 damage is
zero for nonpositive remaining stacks, otherwise `1 + floor(stacks / 5)`.
The packet bypasses only the historical global bleeding0.5 multiplier, retaining
ordinary environmental-packet protections, HP sharing, death/rescues and source
credit. Defense/DR did not reduce this packet before and still do not. Duration
and reapplication, including fractional duration and existing fresh-condition
behavior, are unchanged. <=43 keeps maxHP*0.02 then historical0.5 scaling.
Hemophilia, poison, venom and physical attack damage are unaffected.

Incremental cost per existing bleeding tick: one expected O(1) IndexMap condition
lookup plus constant arithmetic; no allocations/RNG. Overall upkeep still scans
actors/conditions and executes existing damage/death/visibility hooks; offscreen
positive bleeding already requires exact elapsed-tick simulation, so this change
adds no new tick loop or floor scan. Persistent and scratch memory unchanged.
Tests cover thresholds, fractional/zero stacks, player/NPC packet armor behavior,
Nightmare prevention/source death, max-duration reapplication/decay, foreground
vs offscreen exact ticks, free haste movement, legacy43, save/load and replay.
Worker did not execute builds; lead gate supplies execution results.

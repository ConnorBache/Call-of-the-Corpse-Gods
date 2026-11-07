# Revision 46 Chronicle noise reductions

The current run revision stops writing these ordinary notices to the Chronicle
and its archive:

- “You wait.”
- “Slowness delays your next action.”
- Other creatures' “is paralyzed and cannot act.”
- Other creatures' “pauses; it is Slow.”
- Ordinary actor-sourced “You kill …” / “… kills …” follow-ups for non-player victims.

Player death and player paralysis guidance remain. Non-actor-source deaths
(“dies”, “dies unpreventably”, or “is killed by …”) remain. Surprise-kill
sleeping/unaware context, the Shard's permanent stone-corpse notice, and separate
species/finale death effects remain. This is deliberately a narrow message
reduction, not a rewrite of all death narration.

Only the calls that append Chronicle messages are gated, for simulation revision
46 and newer. Revisions through 45 keep their recorded wording paths. Already
stored messages are not rewritten. A pre-patch revision-46 command replay now
regenerates cleaner narration; its prior Chronicle bytes are not retained. The
replay record stores commands and revision rather than message hashes. No
simulation revision is bumped and no saved run is migrated. Wait still advances time, restores reactions,
stomps corpses and emits its presentation event. Player slowness still adds the
extra turn; paralysis and creature slowness still skip their actions. Damage,
death resolution, rescue hooks, corpses, loot, permanent death, RNG, saves and
replay use the same mechanics.

## Cost and verification

Each changed message site adds an O(1) revision/metadata condition, O(1) extra
memory, at its existing per-command, per-creature-turn or per-death frequency.
There are no new scans, caches, persistent fields or rendering work. Skipped
messages avoid their existing format allocation and archive append costs;
retained messages keep the existing visibility and presentation-name costs.
No performance improvement is claimed without measurement.

The focused deterministic test file `chronicle_noise_revision46.rs` compares
serialized revision-45 and revision-46 gameplay state after removing only the
revision and Chronicle fields. It also checks normal/haste waits, slowed-player
turns, paralyzed allies/enemies, innate/applied creature slowness, player and
creature kill sources, owned/unowned environmental damage, player death and
paralysis, surprise/Shard context, plus save/replay equality. Test outcomes are
recorded by the integration task; this note alone is not a verification claim.

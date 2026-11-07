# Revision 39: Shard weapon correction and authored text

New descents and DEBUG starts select revision 39. Explicit revisions 29–38
retain their prior mechanical rules, including the historical targeted-use
Shard behavior. Display wording may receive the designer's corrections.

The Shard of the Borge is an ordinary equippable one-handed weapon. Revision 39
routes its strikes through the ordinary weapon accuracy and target checks.
A landed Shard hit permanently kills its actual target and consumes the Shard.
It can also be thrown through the ordinary physical projectile path; a landed
hit applies the same permanent death. Targeting previews and invalid commands
must not consume the item or gameplay randomness. The old targeted Apply action
is unavailable for revision 39.

The designer clarified that only a successful hit breaks the Shard. Melee
misses leave it equipped and intact. Thrown misses leave a recoverable Shard on
a legal same-floor surface; they do not delete it or drop it through a chasm.

While Dreamwalking, Throw permits this weapon only (not consumable items).
It collides with the active dream plane, never a waking actor overlapping it.
A missed dream throw remains on the dream plane; waking views and effects do
not acquire it. Returning to Dreamwalking permits pickup. A serialized
`Item.dreamGround` marker exists only on such dropped Shards and their observed
snapshots, and is cleared on pickup. Missing metadata means legacy waking
ownership. Existing Phantasmii quest-shard exceptions remain unchanged.
The sleeping body's position and damage notifications are unchanged.

Permanent death uses the shared death resolver with the no-revival marker; it
does not bypass the ordinary corpse, heart, kill credit and finale bookkeeping.
Ahvamitt's search-game interaction remains a noncombat interception before any
hit roll, and therefore does not consume or land a Shard strike.

The designer replaced the repeated-sacrifice message with "The Borge is
satisfied." The weapon-break message retains "The Shard of the Borge
disintegrates." The additional sentence "The corpse blackens and turns to
stone." is emitted only with the ordinary player "You kill the …" chronicle
entry. It is kill flavor, not a new stone-terrain or petrification system.
Death paths without that chronicle (such as Nightmare dissolution) do not add
the sentence. No ordinary weapon or unrelated death receives this text.

Batch 2 entries 12, 13, 18, 19 and 20 were approved unchanged. Entry 15 remains
unanswered, and entry 17 was a request for an explanation rather than approval.
The latter protects starting Dreamwalking when a living dream-plane actor
already occupies the destination or its linked representation. Ordinary
waking-world actor overlap remains allowed while Dreamwalking.

Generation, encounter budgets, art and the authored item description are not
rebalanced or redrawn in this revision. Verification and remaining platform
limitations will be recorded in `verification-r39.md` after the final gate.

## Complexity and boundary ownership

Let G be loose items on the floor, V floor cells, and M remembered/detected
item entries. Miss placement runs once per missed Shard throw in O(V+G) time
and O(G) occupancy memory, with stable nearest-cell ties and no extra RNG.
The plane marker adds O(1) per item filter; drift, erasure, observation and
magic detection retain their existing traversal bounds. Storage validation
adds O(G+M+I+A) time for all floor items, memories, I carried items and A
actors (including nested pilots); extra recursion memory is O(P) for maximum
nested pilot depth P, plus O(1) traversal state. The marker is serialized with Item and therefore follows existing
Arc/COW snapshots, save files, replay and temporal restoration, not a side cache.
The historical v48 TypeScript transcription script is not the current runtime
schema generator; this field is a reviewed native boundary-schema extension
like the existing Borge source-heart fields.

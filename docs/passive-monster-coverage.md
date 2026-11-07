# Yeti, Phoenix, Cancer and Ursa Major

Verified against frozen v48: 12 complete command traces / 96 commands, including every reconstructed history state and ordered presentation events; 24 direct Yeti adjacency cases; eight clean/dirty hidden-floor catch-ups. Full trace log: `validation/passive-monsters-traces.log`. Raw/offscreen log: `validation/monster-chill.log`.

Yeti adjacency affects only the player and Half Here bodies. The first living, non-clone Yeti in actor order supplies attribution. The shared owner is slowed once; the remote body is also marked when adjacent. Authored pre-action and hidden-floor boundaries call this helper; no new prepare-turn hook is invented. Movement's existing snow hook remains authoritative.

Phoenix retains its exact existing corpse roll count, revival health and permanent-burning behavior. Cancer bypasses attack mitigation in the ordinary strike formula, including defensive reaction composition. Ursa Major retains linked Minor death-cry wake behavior and ordinary attacks. These traits are admitted only alongside their integrated hooks, rather than treating catalog recognition as execution.

Complexity: `monster_chill::apply_adjacency` filters N current-floor actors and checks B player bodies, B<=2: O(NB), O(N+B) temporary storage; condition mutation may incur the actor block's O(N) COW detach. This runs at simulation boundaries, never per rendered frame. Existing revival, movement and attack costs remain in the actor/environment ledgers. Twelve trace cases do not exhaust arbitrary combinations with every divine power.

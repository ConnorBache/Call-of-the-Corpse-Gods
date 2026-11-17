# Requested enemy animation update — November 17, 2026

Implemented source behavior:
- Dryad healing uses a bright-green committed magic route, respects creature/terrain blockers and retains existing range, LOS and alliance checks.
- Banshee is a purple outward wave. For current R52 it damages all other living, same-plane LOS creatures, including monsters; legacy revisions preserve player-only damage. No new range cap or changed damage amount.
- Ursa Minor death cry and Pale Screamer use white outward waves. Ruin Pulse uses purple. Existing wake/status/range/LOS mechanics remain intact.
- Phoenix revival uses a small red/orange nova. Ahvamitt teleport endpoints use small rainbow novas.
- Epidite uses a yellow outward pulse over its existing exact LOS footprint.
- Borge's active marked target is telepathically known even for fungus/immune classifications; unmarked targets are unchanged. The grant ends once reward is created or the target dies and does not cross planes.
- Blorb caustic damage to the player logs `You take X damage from the Blorb's caustic slime.`

The world simulation stays atomic and deterministic; presentation snapshots reveal outcomes progressively as the wave reaches each target. This does not make wall-clock animation time advance the simulation. Full Windows runtime verification is separate.

Additional targeted spell presentation: Witch summoning draws a violet connection to its birth cell; Horn-family Beckon and Stone Seal draw gold connections before their existing effect. These cosmetic connections preserve the original spell's targeting mechanics rather than adding new blockers to non-projectile spells. Existing Witch/Bloodmancer curses, Hornlight, Water Jet and Moonbeam retain their already committed projectile paths. Biological births, gas emission and adjacent contact effects are not reclassified as targeted spells.

Banshee damage resolves nearest-first by the Euclidean radial distance used on screen, with actor-ID ties, so reactions follow wave arrival rather than player-first array ordering. LOS admission still uses the existing portal-aware query; the visual is a radial screen-space sound wave, not a simulation of sound transport through portal links.

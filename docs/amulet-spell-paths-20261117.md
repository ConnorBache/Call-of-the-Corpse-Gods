# Targeted amulet spell paths — November 17, 2026

The current revision's Iceblast and Blinking receipt paths use traveling magic tile light instead of local staggered flashes. Lightning/Firebolt/Poison/Acid already used traveling light. Floods retains its broad moving-water footprint rather than becoming a narrow bolt.

Beckoning, Discord, Roots, Obstruction and Teleportation now cast along the shared committed physical ray before applying their effects. This honors the designer's prior rule that non-piercing magic cannot pass intervening creatures or opaque/solid terrain. A creature-targeted effect applies to the first eligible intercepted creature, not a selected creature behind it. Reaching no creature spends the cast without applying a remote effect. Obstruction can build at a shortened empty endpoint if the existing entire-footprint rules allow it; an occupied endpoint does not build a wall on or beyond the creature. Beckoning retains its separately authored pull-path admission and movement rules. Lightning remains piercing.

Admission still validates the selected aim, not hidden intermediate occupancy. Ray resolution occurs after payment. Thus an unseen blocker cannot be probed for free by changing validity. Known-only targeting previews reuse the existing non-piercing Iceblast ray geometry and do not predict through unknown cells or random portal forks. Committed paths consume only the ray's existing serialized fork RNG and the animation reads its detached receipt.

Old simulation revisions retain earlier control mechanics. Presentation receipts remain unsaved. No new range/cost/tier values.

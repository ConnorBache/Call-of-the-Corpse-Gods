# Ordinary terrain title presentation, 2026-11-12

Owner direction: "No title. In general, remove titles like these, and keep the descriptions."

Following the earlier Tree choice, the initial title is suppressed only for currently observed ordinary terrain whose own description identifies it. Explicit allowlist:

- Ice: When a creature attempts to move off an ice tile, there is a 1/3 chance that it slips.
- Puddle: a puddle sits on the ground.
- Hellfire: Hellfire is fire that never stops burning.
- Bloody wall: Blood coats this solid wall. Hungry creatures can feed from beside it.
- Rock wall: A wall of rough rock blocks the way.
- Stone wall: A wall of fitted stone blocks the way.
- Wall: A wall blocks the way.
- Bare ground: Bare ground lies underfoot.
- Stone floor: Worn stone tiles cover the floor.
- Rock floor: Bare rock lies underfoot.
- Earthen floor: Packed earth forms the floor.
- Sand: Pale sand covers the ground.
- Soot: Soot blackens the ground.
- Ash: Ash blankets the ground.
- Grass: Grass grows across the ground.
- Tree: A tree rises from the ground.
- Deep water: Deep water fills this part of the dungeon.
- Shallow water: Shallow water covers the ground.
- Chasm: The ground falls away into a chasm.
- Upstairs: A stairway leads up.
- Downstairs: A stairway leads deeper into the dungeon.
- Bottomless pit: A bottomless pit opens before you. Jump in to win.
- Open door: A wooden door stands open.
- Closed door: A closed wooden door bars the way.
- Grate: A metal grate covers the opening.
- Bridge: A bridge crosses the gap.

All remembered titles remain. Speed-up area and Unclear memory remain. Creature/item/player titles, later portal/Non-Euclidean/feature/light-source title overrides, glyphs, descriptions, hazard rules and controls are unchanged. The terrain-copy function is unchanged; this is presentation filtering, not global string deletion.

Counts: 23 earlier exact text deletions plus 26 terrain-heading presentation targets (Tree included, 25 newly covered). Native assertions cover each allowlisted current/remembered name and special-context names; they were added but NOT EXECUTED. Rust syntax, cumulative patch roundtrip, JSON structure and preservation checks passed; no Cargo/runtime/visual test claim. Composed separately into earlier Gameplay carry and exact matching R51 branch context. Neither is missing integrated v2. No merge into main.

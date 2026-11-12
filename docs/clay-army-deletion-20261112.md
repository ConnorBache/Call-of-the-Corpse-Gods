# Clay Army reviewed sentence deletion, 2026-11-12

Follow-up to b9ed9596586e44153dbb9c69b08c42f87a0749e5 on the older R51-based review branch, not the missing integrated R52 source.

Decision G-9eeb4dca640b3edce7641318 retained its original Defer label and explicit comment "Just delete it". The original card isolates: "Implementation: Clay Soldiers are treated as unbreakable walls for pathfinding and collision."

Fresh branch inspection resolves the previous hold: ability_ui.rs appends to the same Clay Army AbilityView description, whenever simulation revision is >=50: "Each Clay Soldier uses your current attack without abilities, and attacks anyone adjacent at the end of a turn. It is an unbreakable wall." The unbreakable-wall rule remains visible in that report. Only the isolated catalog implementation sentence and its preceding blank-paragraph separator were removed. Cost, movement, attack copying, adjacent attack timing, effectSummary, appended rules and mechanics are untouched. No replacement wording was invented.

Verified JSON parsing, original Git blob hash, and exactly one changed JSON leaf: kative-clay-army.description, with deletion-only transformation. No Rust code changed. No Cargo/runtime/visual tests run. No main merge. Earlier records describing this case as held are superseded for this exact deletion only; other held cases remain held.

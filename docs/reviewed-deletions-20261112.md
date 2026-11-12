# Reviewed sentence deletions, 2026-11-12

Follow-up to mechanical grammar commit b1cf996bdcd5243c4694683f895f4aefa33a23bd. This branch remains based on the older R51 import, not the missing current integrated R52 source. The prior grammar-only record describes the earlier commits, not these separately authorized deletions.

Original exported review labels remain Defer; the explicit comments below requested deletion. Each comment and focused sentence was verified against the uploaded decision export and original card. Only the isolated sentence and its adjacent separator space are removed. No replacement wording or game behavior is introduced.

## G-af85d356988d28e010a18aef
Comment: "Yeah this should be totally killed."

Target: Implementation: Preview the placement in the sight display; color both the pit and wall in Kative's colors.

Branch outcome: Removed exactly.

Mode dimensions, costs, HP, orientation, removal, enemy wall attacks retained.

## G-fa8c98a5a88c0602fa598a99
Comment: "Delete it."

Target: Allies retain the existing Damage and Perception boosts.

Branch outcome: blocked: deletion would remove sole nearby ally-benefit information

No branch deletion made.

## G-21378e1eec886eccdfb0a77b
Comment: "Delete it"

Target: Some native source consumers are not yet integrated.

Branch outcome: Removed exactly.

Existing supported-views limitation retained.

## G-eacc28b977c707dea73bbe10
Comment: "Kill it"

Target: It emits hallucinogenic gas instead of caustic gas.

Branch outcome: Removed exactly.

Same reachable Greater Pale branch already announces hallucinogenic gas; exposure duration and perceived-identity limitation retained.

## G-41c7ebf4cf498d798395761d
Comment: "Kill it"

Target: Historical profile values remain stored for compatibility.

Branch outcome: blocked: exact span absent on R51 branch

No branch deletion made.

## G-8cfb09f8f7aef6fd3003240f
Comment: "Kill it"

Target: Energy absorbs damage below 1 HP at 4 energy per HP; health costs and unpreventable kills bypass it.

Branch outcome: blocked: exact span absent on R51 branch

No branch deletion made.

The fixed-glyph compatibility sentence and Hanged forecast recap are absent on R51. Both were removed in a separate patch against copies of the verified earlier Gameplay snapshot only. The fixed-glyph rule remains. Hanged's full passive description remains in corpse-core/src/hanged.rs, wired into the revision-52 catalog by progression.rs: it preserves the 1 HP threshold, 4 energy/HP cost, excess damage, exhausted-energy death, and health-cost/unpreventable-death exceptions. The forecast keeps its computed hit/damage/HP/energy figures.

Scream is held because its selected sentence is the only nearby ally-benefit statement in that revision branch. Clay Army, Relentless Pursuit and clone-template deletions remain held for ambiguous scope. Six Keep decisions are not deleted.

Validation: exact allowlisted deletions only; JSON parsing and Rust tree-sitter syntax parsing passed. Original source copies preserved. No Cargo, runtime or platform tests run. No merge into main.

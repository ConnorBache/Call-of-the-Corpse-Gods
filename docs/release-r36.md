# Revision 36 — Non-Euclidean pockets

New descents and DEBUG starts use simulation revision 36. This revision adds
cellular-automaton pockets from floor 20, using the designer's sequential
unassigned-square attachments. Both sources share a destination; reverse travel
uses a fair serialized-RNG choice. Pockets do not project or chain vision.

See [the complete rules and explicit defaults](non-euclidean-r36.md) and
[generation, safety proofs and bounded rejection](non-euclidean-generation-r36.md).
The generator does not change ordinary terrain, population, heart rewards or
loot budgets. No unsafe proposal is repaired with invented links.

Known pocket cells have a cyan inset. Hollow cyan corner marks show possible
known exits. A route stops at a random choice rather than promising a specific
landing. Actors and items retain single identities across linked representations.
Physical rays/throws use committed outcomes; pure previews never consume their
random choices. Dreamwalking and Half Here retain their existing plane rules.

The Epidite map countdown is now limited to 9–1; the observed description always
uses `X turns until judgement`. Its cadence and damage are unchanged.

All seven Godbeasts, prior creature content, original artwork/lore, and revision
35 portal visuals and optical behavior are retained. Historical runs/replays
keep their historical rules. The original revision-35 top-level receipts are
preserved under `verification/historical-top-level-r35/`.

## Verification boundaries

Use [the current verification report](verification-r36.md) for executed results,
frozen-source fingerprints, generated/populated samples, and renderer evidence.
Controlled scenes and seed samples are not exhaustive balance or survival
certification. Cross-compiling an unsigned Windows executable and inspecting
its PE imports/resources is **not** Windows launch, rendering, input or save
verification. No Windows runtime claim is made.

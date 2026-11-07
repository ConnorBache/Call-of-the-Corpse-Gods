# Designer environment/status hooks — simulation revision 33

Authority: designer instructions **2–5**: damage in water makes dispersing blood;
Kelpie spawn points have permanent mild blood; Greater Breathers induce two-turn
hallucinations in hallucinogenic gas, refreshed rather than stacked; the weakening
scream becomes **Disoriented** and decreases accuracy; successful root escape breaks
the root. Instruction **6: “No need”** means no new Dryad native/outsider behavior.

## Executed behavior

### Dissolved blood (not cosmetic footprints)

`water_blood.rs` owns a separate finite `Floor.waterBlood` concentration grid and
persistent `kelpieBloodSources` spawnpoint indices. A positive **actual HP loss**
deposits one unit per HP, up to **100 concentration**, at the damaged physical body
location. This uses the shared HP-loss boundary, so melee/ranged damage, reflected
packets, damage-over-time, forced-movement damage, and shared-health bodies use the
same rule. Fully prevented/zero damage deposits nothing. The designer's
“a creature” instruction includes all damaged bodies, not just the existing
cosmetic `bleeds` flag. No ordinary movement trail is added, and this transport module does not execute feeding.

Only **water and shallow-water**, without a caustic-Ichor overlay, carry dissolved
blood. Every normal environmental turn uses a double-buffered gas-like local
spread: transfer **10% of the old concentration to each of at most eight wet
neighbors**, keep the remainder, then retain **94%** and truncate values below
**0.05** to zero. Diagonal transfer requires both intervening orthogonal water
cells, so it cannot jump a dry corner. Dry tiles never receive blood. Frozen
Reverse Entropy cells neither exchange nor dilute it. This is a bounded,
deterministic *blood transport* adaptation, not a rewrite of the existing Brogue
gas adapter, global water physics, currents, or fire.

At initial floor population, each living Kelpie's actual wet spawn tile is recorded.
Those locations maintain a **3.0 concentration floor** after diffusion. Source
locations neither move with the Kelpie nor disappear when it dies. A source
currently covered by dry terrain/Ichor does not emit, but its location is retained.
This modest source strength is provisional numerical tuning, centralized in
`water_blood.rs`, and can be revised independently of damage deposits.

### Later approved feeding-cycle integration: authored blood rivers

The feeding-cycle follow-up explicitly authorizes actual generated `river`
landmark water cells as permanent mild blood sources. `bloodRiverSources` is a
**separate persisted list**, initialized by `register_river_sources` only at the
revision-33 generation boundary. Its source floor is **3.0**, matching the mild
Kelpie floor. Only actual river-landmark cells still containing water/shallow-water
without Ichor are registered; lake landmarks, ordinary water, dry landmark cells,
and purely cosmetic crimson palette treatment do not qualify. Sources are
idempotently deduplicated and survive save/rewind independently of actors.
The feeding system may read positive actual concentration; this module does not
execute feeding, healing, energy regeneration, currents, or water physics.

New populated floors include the zero-valued blood layer so observation can
separate **clean water** from low/high blood concentration. Hazard memories store
only the observed concentration; hidden cells never read the live layer. The
presentation owner adds blood-sensitive tints without brightening unrelated Flesh
terrain. Collapse clears blood on destroyed tiles, just like its other local layers.

### Hallucinogenic gas / Hallucinating

`hallucinogenic` is a distinct gas type using the existing single-gas-per-cell
transport/provenance machinery, with the existing **quick** dissipation class.
For revision 33, **Greater Breather passive and boosted exhalations emit
hallucinogenic gas instead of caustic gas**; ordinary Breathers still emit caustic
gas. This is an explicit implementation choice for the designer's request and the
single atmospheric slot, not a hidden assumption that two gases coexist in one cell.
Existing emission volumes and energy costs are unchanged.

Exposure sets `hallucinating` to **exactly 2**, never adds duration. The duration
first decreases once at the start of each normal environment turn, then exposure
refreshes it. Leaving the cloud therefore yields 1, then 0 on the next two world
turns. Haste-only movement does not advance it. Offscreen exact environmental
turns use the same clock; save/load and temporal snapshots retain its current value.
Beast gas immunity and Pale gas immunity protect against this gas, including its
Greater Breather emitter. It causes no caustic damage and does not apply Confused.

Exposed actors can carry the status, but **only player perception is hallucinated**:
no AI confusion, target selection, hostility, damage, or true statistics are changed.
The desktop observer uses a deterministic seed/tick/body-identity transform of
already-observed creatures for changed names/glyphs/colors. It does not reveal
unseen creatures and does not use gameplay RNG. See the presentation-specific
coverage/tests for the visual layer.

### Disoriented

Revision-33 Screamer enemy effects replace `screamWeakened` with **Disoriented**.
They reduce **Accuracy only: 5 points normally, 10 from Greater Screamers**, while
preserving the existing five-turn application/refresh and normal-world-turn expiry.
The old offense penalty is removed from the target, not stacked with the new one.
The existing ally Offense/Perception boost, convergence, energy cost, and range are
unchanged. These flat Accuracy penalties are provisional, visible numerical tuning;
no unrelated health/reward/stat rebalance is included.

### Breaking a root

The current sticky-terrain mechanic has a first unsuccessful struggle and a second
successful escape. In revision 33, that successful root escape clears the root at
**the binding tile** and deactivates only fungal connections whose path crosses it.
Failed struggles, blocked destinations, and forced/flying bypasses do not sever
roots. Unrelated links remain active. Inactive fungus/corpse pairs remain recorded,
so that exact connection cannot immediately reacquire the corpse. Existing fire
severing is unchanged; no new slashing-weapon mechanic was added.

## Compatibility and verification

All newly executed behaviors are gated to simulation revision 33. Revisions 29–32
retain caustic Greater Breathers, offense-weakening screams, old root escape, and no
new water layer/source initialization or deposition. The optional hallucinogenic
channel/provenance is not materialized in old runs, preserving exact legacy bytes.
New optional fields participate in typed saves, bounded schema validation, observed
hazard memory, Arc copy-on-write snapshots, and RLE decoding for the blood grid.

Focused deterministic coverage lives in
`crates/corpse-core/tests/native_environment_hooks.rs`, with desktop status labels
in `statuses_ui::tests::native_disoriented_and_hallucinating_labels_expire_without_internal_penalty_labels`.
It covers damage types/positive HP loss, actual reflection, shared-health body
position, no movement trails, finite/dry/frozen diffusion, persistent sources,
legacy gates, actual river/lake/Ichor source separation and persistence, save/schema/temporal restoration (including both actual time abilities and resume),
remembered/unknown-water privacy, non-mutating enemy status exposure, two-turn refresh, haste/offscreen
clocks, Greater/ordinary gas choice and immunity, Accuracy-only status/expiry,
failed/blocked/successful root attempts, multiple links, no reacquisition, and
unchanged fire severing. Full-workspace and native-platform evidence must be
reported separately; these focused tests are not a claim of platform verification.

## Complexity

Let **N** be floor cells (normally 2,291), **A** actors on a populated floor,
**K <= 2N** stored source cells across the two separately deduplicated lists, and **P** the total stored fungal-root path length.

- Initial source registration: O(A + N + K) time and O(N + A) transient memory
  (cell membership bitmap and candidate indices); retained grid O(N), sources O(K).
  Existing indices are deduplicated with the bitmap, not repeated linear search.
- River registration: O(N + L + K) time and O(N + L) transient memory for L
  river-landmark cell entries (generation has at most 24 landmarks); retained
  sources O(N). Revision gating is owned by the generation caller.
- Damage deposition: O(1) after allocation; first allocation is O(N). Mutating a
  shared floor may incur the existing full floor Arc copy-on-write clone, including
  all retained layers/metadata; this is not hidden in the O(1) claim.
- Blood active check + diffusion, once per normal active/offscreen world turn:
  O(N + K) time and O(N) temporary memory, at most eight neighbor transfers per cell.
  No sorting, visibility, pathfinding, or RNG occurs inside this pass. The same
  existing floor COW cost applies on first mutation after a shared snapshot.
- Hallucination duration/exposure bookkeeping: O(A) per normal environment turn,
  O(1) per actor; the existing gas diffusion remains O(N + N*T) for T tracked vents,
  with its existing gas/provenance buffers. One optional channel adds O(N) retained
  data only when used. Gas-type count remains a fixed constant (nine).
- Disoriented modifier: O(1) per existing effective-stat query; expiry uses the
  existing O(A) environment cohort. It does not add perception/path queries.
- Successful root sever: O(P) scan plus O(1) tile update, O(1) extra space beyond
  existing COW. Fungal acquisition's existing actor/LOS/path work is unchanged.
- Hazard observation adds O(1) blood lookup per already-observed cell and one optional
  numeric value per remembered wet cell. Desktop tint/status painting adds a bounded
  operation per painted cell/status; native glyph rasterization, framebuffer work,
  and observer identity transforms retain their documented subsystem costs.

These are analytical bounds, not measured timings. Validate timings with populated
wet floors, many stored sources/links, snapshot COW, and offscreen catch-up lengths.

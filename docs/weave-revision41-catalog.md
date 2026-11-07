# Revision41 ability catalog and admission

The frozen 72-entry catalog JSON and legacy catalog/weight/choice APIs are unchanged.
Revisions29–40 use the identical catalog object and legacy weighting arithmetic.
Revision41 replaces the Revert entry at its original position with the passive
`numo-weave-of-time`, named Weave of Time. Its description is exactly:

> You can freely travel along your time weave, which you leave behind as you move.

The new catalog still has72 entries. Level-up weights use revision-specific owned
metadata, since the existing algorithm derives tags from descriptions. Revision41
weights may intentionally differ once Weave is owned; this is not applied retroactively.
The starter pool is unchanged because Revert was not a starter blessing. First offers
with empty ownership retain draw count/order and replace only the affected ID.

Generated rings/amulets and Tinker use the revision-specific pool. Existing public
loot helpers remain legacy wrappers; startup explicitly passes the run's revision.
Equipment grant reconciliation and UI names understand Weave. The UI still displays
it for learning/details but provides no command binding. Numeric/pointer callers must
use `ability_ui::command_binding_allowed`; all older passive behavior remains unchanged.
Direct Weave activation has no effect or resource cost. Acquisition creates only the
normal acquired/order metadata, not Revert charges. Legacy Revert metadata and code
remain available for historical executions; no old replay compatibility is deleted.

The new boundary identity validator rejects Weave and any present `timeweave` field
at revisions≤40, and rejects Revert IDs in revision41 owned/offered/granted metadata.
It examines nested planes, carried items and pilots. Trail geometry and movement policy
are separate module/Engine responsibilities. This change does not decide direction,
forced movement, crossings, rendering, or collision consequences.

## Complexity

- `catalog_for_revision`: warm O(1); first new-catalog initialization copies O(A+text)
  bytes for A=72 entries. It does not copy a Run or discover changed world state.
- `definition`: O(A) identifier comparisons plus ID bytes, no RNG or mutation.
- New weight selection retains existing catalog/owned-ID/description scans and allocates
  a pool-length weight vector. The original legacy branch is called below41. Catalog
  constants are not an excuse to conceal string-copy/scan costs.
- Loot generation: O(L*(C+A)+item payload), L≤20 slots, C equipment entries; same RNG
  count/order for the same pool and no revision-selection draw. Tinker additionally
  scans existing inventory/ground grants before its two draws.
- Equipment reconciliation retains its preexisting ownership/payload scans. UI metadata
  lookup remains a catalog scan; the new binding predicate is O(ID bytes).
- Revision identity admission is an additional O(S) walk over the already constructed
  boundary JSON Value, O(depth) stack. It does not serialize a second world and is not
  part of normal per-command `validate_supported`; load/export validation is not O(delta).
- Revision allowlists are fixed-size checks. Floor count/depth bounds remain40, and
  convenience startup defaults remain39; desktop new-run admission chooses41 explicitly.

Focused tests cover old exact authored weights, same-slot metadata, first-offer RNG,
actual equipment generation, acquisition/grants, old40/new41/future42 admission, old
trail-presence rejection, and passive desktop display. Existing frozen replay/generation
corpora remain required gates. This is staging, not a passed release claim.


## Full generation admission follow-up

The first desktop gate exposed three remaining revision allowlists in
`generation/world.rs` and the separate `generate_setups_revision` entry guard.
They now admit revision41 explicitly; floor/depth remains1–40 and revision42
continues to reject. No generation selection, candidate budget or RNG operation
changed. Guard checks remain O(1), with existing generation costs unchanged.
A new regression calls **actual** `create_run_revision` for40 and41, generating
all ten startup floors and admitting the resulting Run; shell-only tests did not
exercise these paths. The original failed gate remains evidence, and this added
regression awaits the lead's next run. The desktop Revert presentation test was
also corrected to its unchanged full display string, including `0/5 charges`;
production legacy presentation behavior was not changed.

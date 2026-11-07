# R50 player-power command admission

This narrow patch uses the existing revision-aware execution registry for R50
player-owned ability IDs and active toggles, including shared Half Here/dream
endpoints. The existing R50 Mini-Me branch receives the same retired-ID rejection;
this does not grant the stats-only clone any powers. Unknown/retired identities
fail preflight without migration, payment, RNG, replay or log bookkeeping.

Future retains its existing separate temporal-dispatch identity exception. Its
capture-payment and UI gates are unchanged. Pre-R50 Revert handling is preserved;
R50 cannot use Revert. No global startup, storage, replay or desktop admission gate
is opened.

Existing supported passive enemy traits remain admitted for Remember/copied-power
paths. Ordinary NPC admission retains its explicit trait whitelist. Item
identities and equipment validation are unchanged, including the Borge shard,
Phantasmii quest items and bows. Category assignment must remain separate from
`Item.kind`; the existing Borge throw path needs no invented replacement attack.

## Fixtures and impact

Six regressions in `revision50_player_power_admission.rs` cover every explicit
retired identity, orphaned retired active IDs, all 72 current catalog IDs, Future,
unknown IDs, borrowed passive traits, ordinary NPCs, special items, shared
endpoints/copy admission, and historical revisions. Admission of a catalog ID is
not a claim that its effect or resource acquisition is complete.

The existing `revision50_wave_teleport::tests::room` simply relabeled the v48
avatar fixture and retained retired Unyielding Assault/Dilute Potion. Its
`teleport_dispatch_rejects_immutable_endpoints_before_payment` test therefore
would fail at the new power boundary before reaching the amulet assertion.
The patch makes that amulet-only fixture's player power/active lists empty;
the individual test still explicitly adds Spellslinger. Target rejection and
resource/position assertions are unchanged. No frozen reference fixture is edited.

Other fixture helpers found retaining the old starters are direct geometry,
resource or read-only preview tests (`revision50_grappler`,
`revision50_ability_energy`, `cogworks_ui`, and `targeting_ui`); this source audit
found no command-preflight call there. The full suite must establish
actual impact. Neither the new tests nor the adjusted fixture were run by this
patch-preparation worker.

## Complexity

Let B be player-owned/current-copy power plus active IDs checked, C=72 current
catalog entries, and L maximum ID bytes. Added revision-aware matching costs
O(B C L) worst-case time per existing command preflight and O(1) transient memory.
First R50 catalog use parses/retains O(catalog bytes), as before. Shared-owner
classification retains the existing condition/plane lookup costs; the active
check adds one such lookup for an R50 endpoint. NPC, item, floor/environment,
actor-registry and offscreen validation costs remain additive and are not included
in this isolated bound. No per-frame cache or changed simulation state is added.

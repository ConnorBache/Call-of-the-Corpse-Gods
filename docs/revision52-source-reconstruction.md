# R52 source reconstruction provenance

The preceding Accuracy Baselines source inventory is preserved separately with
its expected SHA-256 values. Most source was recovered byte-for-byte from the
R51 bundle, saved deltas, packaged documentation, and retained edit evidence.
Seven files could not be recovered exactly. With approval, they were carefully
reconstructed against the saved design, release documentation, existing code
contracts, and regression evidence:

- core: revision50_control_amulets.rs, revision50_wave_teleport.rs,
  revision52_amulet_tiers.rs, revision52_floods_tests.rs;
- desktop: main.rs, stat_presentation.rs, revision52_floods_ui_tests.rs.

These files are behavioral reconstructions, not verified byte-identical copies
of the preceding release. The separate reconstruction manifests retain the
expected, recovered-base, and reconstructed hashes and sizes. Newly requested
stat, reserve, Hanged, interface, artwork, and rendering changes were applied as
separate reviewed patches on top. The final source snapshot and matching fresh
workspace/build receipts identify the exact new deliverable.

Restored behavior includes Floods terminating on shallow water, promoting the
terminal cell to deep water with the established twelve-turn temporary lifetime,
refreshing temporary entries without replacing their original restoration tile,
protected terrain and older-revision behavior, tier text, revision-aware Damage
rounding, and Bestiary routing for the active game/replay revision. Focused core
and desktop tests were rebuilt from the documented contracts. See the Floods
reconstruction report and associated test receipts for limits and uncertainty.

No old passing receipt is used to certify the reconstructed or amended source.
The archive contains a complete usable source tree so future work does not depend
on reapplying an incomplete chain of deltas. Windows cross-compilation and static
binary checks do not establish Windows runtime or physical GPU/DPI behavior.

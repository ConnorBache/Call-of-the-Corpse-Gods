# R52 Gameplay UI layout, 13 November 2026

The attached patch is implemented and tested against the corrected R52 Gameplay source archive. It is **not** applied to this branch's older R51 source, and is not based on unavailable Integrated Review v2. Apply it to the R52 Gameplay archive after the scrollbar removal, using `git apply flush-ui.patch` from its source directory.

Changes: Chronicle/map left; observed creatures and ground contents in adjoining middle column; resources and abilities right; boxed stats at bottom, no Stats heading or Chronicle title; contiguous borders; wrapped names; corresponding hit-testing, scrolling, and retained/tiled damage geometry. No simulation mechanics changed.

33 focused regression tests and generated floor-21 screenshot test passed. Full workspace suite was started and is not yet reported passed. Native screenshot is saved in the updated source archive's provenance and in ChatGPT Library. No Windows executable was built on this Linux workspace.

Verification:
```json
{
  "changes": [
    {
      "path": "crates/corpse-desktop/src/ability_ui.rs",
      "before_sha256": "1a02d15dd76552cc47a2c23f3d39e4bbf74eed541be40f0933588d591f4d5660",
      "after_sha256": "d4a4bb5e879b63b22bebd150d1edb682a63df041e2376c6fef9d6fe967ae5ce4"
    },
    {
      "path": "docs/complexity-ledger.md",
      "before_sha256": "aabf7d1426be3034b851de0dfe87b422ff7524e8a6967f425e0c0d268db45379",
      "after_sha256": "daecc9e62e6f83a5e8433cfa44341eaf7b7f57648bb00924c0312af772a4c2cb"
    },
    {
      "path": "crates/corpse-desktop/src/enemy_list.rs",
      "before_sha256": "cdbc80a80e4b3d23fc5216c21aebefc63b3bd00597b3aa290c315e20c82dbc68",
      "after_sha256": "1f67e906929021f0b97a15bde6c822cac872415d70f278ba853f4653ba591d8e"
    },
    {
      "path": "crates/corpse-desktop/src/main.rs",
      "before_sha256": "83ad0b830802975a4cf99f680de76cfb4d61e52bb0fd9d371587adbd9102f7be",
      "after_sha256": "b0c7c77527c0b5f996997d7fa3e2f9e5083857678d3ef11e9955f50261bbe3e9"
    },
    {
      "path": "crates/corpse-desktop/src/modal_ui.rs",
      "before_sha256": "c80f3772d12246a742016507f4932c5de3479090adae5845198478a5f5c954b6",
      "after_sha256": "77d09b2438a21322065f35f95ebf8d6545720eb6ab80ba4daa82aafb00ce03ca"
    },
    {
      "path": "crates/corpse-desktop/src/render.rs",
      "before_sha256": "275514c3919fa223a1b79480bbacb8ffb6cd25ddbea8d79fc092fc9f3bf62d97",
      "after_sha256": "c56e4057c9a844c60a32046282c35484382409fdbded9acbb6832015c9c38e1d"
    },
    {
      "path": "crates/corpse-desktop/src/render_damage.rs",
      "before_sha256": "61c35155f89208a45ef18da348dd36ceabde8c59c2089ff6ca00d6d56aeec722",
      "after_sha256": "44d795adf18c8b4188b274539ec4612411ed81df81b074efd1d11ea97e87d48a"
    },
    {
      "path": "crates/corpse-desktop/src/tile_damage.rs",
      "before_sha256": "73e0bcf3fd99ba3d7d457cb7a67669fb98d5a09b935ebabbe511065679a69bdb",
      "after_sha256": "f45e8d34d53f8b6d494e45849eeaddc2b47ebeda0feb505072fa2e6ad632a93b"
    },
    {
      "path": "crates/corpse-desktop/src/tile_overlay.rs",
      "before_sha256": "8d9fea8a9e071ff137b15b92817b884dc24451edeea49be4b2e214fc52262ea2",
      "after_sha256": "3c127d612c8b12631805bdefcc90cd287caaa5d963c3b3600dc8ae98ea6b4b93"
    }
  ],
  "source": "Earlier R52 Gameplay, not Integrated Review v2",
  "native_render_screenshot": "floor-21-ui.png",
  "screenshot_sha256": "b98db287ed39293bbc02914ba2ae16601d5c553d92db2453e8de2d07bdfed5a6",
  "tests": {
    "enemy_list": 13,
    "quiet_ui": 11,
    "responsive": 6,
    "flush_layout": 1,
    "retained_damage": 1,
    "unchanged_map_toolbar": 1,
    "native_screenshot": 1
  },
  "failed": 0,
  "full_workspace": "running, not yet passed",
  "patch_roundtrip": "passed",
  "github": "Incremental patch only on divergent R51 branch, not merged into R51 source"
}
```

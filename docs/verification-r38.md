# Revision 38 verification

CORPSE GODS — REVISION 38: GENERATION HARDENING AND DESIGNER COPY

Implemented direct stair-footprint reservations for Verdania's tap, correct current-roster machine-budget preflight, safe candidate rejection for repairable connectivity failures, and hazard-free cardinal-route preservation during late sub-biome decoration. A final populated-floor safety check prevents violations of the cardinal safe-route generation contract from being accepted. These failures are not claimed to be gameplay softlocks; legal diagonal movement may still provide another route. Native generation additionally retries complete atomic floor transactions with eight bounded attempts and 32 geometry candidates per window. It retains the original seed, Godbeast/Nest/domain/stair plans and exact ordinary heart budget. Required content is never removed to make a candidate pass.

Designer text replacements 1, 2, 3, 4 and 10 are applied, including the separate Mask of Trickery item description and Teleport ability text. Their mechanics are unchanged; exact wording includes the supplied Borge “it's”. Entries 5–9 remain pending. All art and non-Queen authored catalog data are preserved; Queen Natural History is the sole authorized manual asset change.

VERIFIED
919 passed, 0 failed, 1 existing ignored across 150 workspace targets. Documentation tests passed. 1072 frozen inputs unchanged across aggregate test and Windows build gates. Required-content and budget checks cover known failing seeds, deterministic incremental/all-at-once generation, save/load and DEBUG reconstruction. The complete independent seed-stress receipt and its limits are attached in the source evidence below.

Independent review summary:
{
  "status": "passed",
  "created_utc": "2026-10-28T20:28:55.781905+00:00",
  "rustc": "1.99.1 (58cf8f9c3 2026-10-13)",
  "requested_worlds": 1024,
  "attempted_worlds": 1024,
  "successful_worlds": 1024,
  "failed_worlds": 0,
  "floors_count": 40960,
  "failures": [],
  "substitute_seeds": 0,
  "algorithmic_whole_world_seed_retries": 0,
  "seed_families": [
    "r37-event-count-0000..0127",
    "r38-stress-0..127",
    "decimal0..127",
    "0-1-2:qa-0..127",
    "11-10-9:qa-0..127",
    "Unicode生成-é-🌱-0..127",
    "256x characters plus -0..127 (core API permits long seeds)",
    "deterministic 64-bit mixed hexadecimal"
  ],
  "normal_startup_finalization_engine_wait_cases": 6,
  "additional_modes": {
    "blessed": {
      "attempted": 32,
      "successful": 32,
      "floors": 1280
    },
    "avatar": {
      "attempted": 32,
      "successful": 32,
      "floors": 1280
    }
  },
  "deterministic_repeat_worlds": 16,
  "save_load_worlds": 16,
  "recovered_population_floors": 0,
  "baseline": {
    "optimized_core_rlib_sha256": "2b9c4f18087d7986d24b144410fed32e32d54f2d709972647858ed212272f21e",
    "survey_harness_sha256": "c2317295712c522f380689cd7e4c16ec46978c7c9bc3e4e02a4bf9461f7b4ca4",
    "revision": 37,
    "attempted": 1024,
    "successful": 1017,
    "failed": 7,
    "failures": [
      {
        "index": 285,
        "seed": "生成-é-🌱-35",
        "generation_error": "Required generated item invalid/occupied on floor35"
      },
      {
        "index": 301,
        "seed": "生成-é-🌱-37",
        "generation_error": "Authored geometry leaves an inaccessible room"
      },
      {
        "index": 376,
        "seed": "r37-event-count-0047",
        "generation_error": "Required generated item invalid/occupied on floor20"
      },
      {
        "index": 454,
        "seed": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx-56",
        "generation_error": "Required generated item invalid/occupied on floor15"
      },
      {
        "index": 599,
        "seed": "qa-mixed-33cdd108d7485523",
        "generation_error": "Required generated item invalid/occupied on floor15"
      },
      {
        "index": 925,
        "seed": "生成-é-🌱-115",
        "generation_error": "Required generated item invalid/occupied on floor30"
      },
      {
        "index": 1023,
        "seed": "qa-mixed-3faf6c43aaa5d7eb",
        "generation_error": "Required generated item invalid/occupied on floor30"
      }
    ],
    "note": "Six diagnosed Veridian tap/stair collisions; one inaccessible-room early abort. Original baseline calibration stair checker was incorrect; only actual generation failures are counted here."
  },
  "first_candidate": {
    "status": "failed_invariants",
    "complete_generated_worlds": 1024,
    "failed_route_invariant_worlds": 5,
    "failures": [
      {
        "index": 624,
        "seed": "r37-event-count-0078",
        "normalized_seed": "3-6-9:r37-event-count-0078",
        "revision": 38.0,
        "floors": 40,
        "population_retries": 0,
        "errors": [
          "stairs-unreachable:30"
        ],
        "repeated": false,
        "loaded": false,
        "elapsed_ms": 2364
      },
      {
        "index": 746,
        "seed": "93",
        "normalized_seed": "0-3-6:93",
        "revision": 38.0,
        "floors": 40,
        "population_retries": 0,
        "errors": [
          "stairs-unreachable:32"
        ],
        "repeated": false,
        "loaded": false,
        "elapsed_ms": 2630
      },
      {
        "index": 909,
        "seed": "生成-é-🌱-113",
        "normalized_seed": "2-1-8:生成-é-🌱-113",
        "revision": 38.0,
        "floors": 40,
        "population_retries": 0,
        "errors": [
          "stairs-unreachable:27"
        ],
        "repeated": false,
        "loaded": false,
        "elapsed_ms": 1915
      },
      {
        "index": 963,
        "seed": "0-1-2:qa-120",
        "normalized_seed": "0-1-2:qa-120",
        "revision": 38.0,
        "floors": 40,
        "population_retries": 0,
        "errors": [
          "stairs-unreachable:22"
        ],
        "repeated": false,
        "loaded": false,
        "elapsed_ms": 2113
      },
      {
        "index": 1020,
        "seed": "11-10-9:qa-127",
        "normalized_seed": "11-10-9:qa-127",
        "revision": 38.0,
        "floors": 40,
        "population_retries": 0,
        "errors": [
          "stairs-unreachable:27"
        ],
        "repeated": false,
        "loaded": false,
        "elapsed_ms": 2740
      }
    ],
    "fingerprint": "c28003a4b1a36df7c533c868561b1b4f05b6f11ded250ac053f8f095bed121f4",
    "note": "No generation errors; five lost hazard-free cardinal transition routes after habitat blocking. This is not a claim that all five were unplayable by 8-way movement. Environment restart interrupted the first-candidate run after 614 complete records ; 410 missing indices were resumed. At most 4 partial attempts were interrupted; no completed input was substituted."
  },
  "matched_regressions": {
    "baseline_failures_retested": 7,
    "first_candidate_route_failures_retested": 5,
    "all_passed": true
  },
  "core_fingerprint": "de9b799d5fe9fe5897450c2872b6823e84717aebf48bce28c9fe2abccf7e2886",
  "core_files": 144,
  "matched_snapshot": true,
  "optimized_candidate_harness_sha256": "8dc74de79a7ba3ee0008a9bc2ef7ea8c0f2aa64bfd9be4aaff46de69912e5c0c",
  "optimized_candidate_rlib_sha256": "e8990c66f849c9be9b4beb9512c438c9ea299d0fdf9eeb126cfd93cdd7f19346",
  "historical_validation": {
    "complete_repeat_roundtrip_revisions": [
      29,
      30,
      31,
      32,
      33,
      34,
      35,
      36,
      37,
      38
    ],
    "cross_binary_serialized_sha256_equal_revisions": [
      29,
      30,
      31,
      32,
      33,
      34,
      35,
      36,
      37
    ]
  },
  "debug_validation": {
    "normal_world_prior_heart_and_target_comparisons": 16,
    "completed_rewards_admitted_engine_deterministic_wait_roundtrip": 16,
    "extra_exact_heart_audit_000071_floor27": 1
  },
  "invalid_inputs_rejected": 15,
  "custom_invalid_callback_atomic": true,
  "additional_budget_and_required_content_worlds": 4,
  "additional_full_generation_all_ground_reachability_worlds": 32,
  "timings": {
    "stress_total_ms": 907079,
    "world_median_ms": 2886.5,
    "world_p95_ms": 6553,
    "world_max_ms": 13428,
    "note": "Optimized Linux under shared build/test contention; not worst-case guarantees."
  },
  "limitations": [
    "Sample evidence is not proof for every possible seed. Bounded recovery may still explicitly fail atomically on exhaustion.",
    "Historical byte equality covers one fixed complete world per revision, not every save/seed.",
    "Generation/admission and structural invariants are tested, not full playthroughs.",
    "DEBUG chooses first legal offer; not every possible build.",
    "No Windows runtime or desktop interaction tested by this reviewer.",
    "Direct-rustc optimized QA is separate from lead workspace/Windows gates."
  ],
  "logs_manifest": {
    "normal_start.rs": "9484577a0b54f51bae9c823f5a952f5ee89440cf78e1b774ea740cae88792da7",
    "normal-start-final.jsonl": "fb127c692eb10dc272a2b4fe2dceff2bf68ea4d06691b14e9fe0487c700919d4",
    "normal-start-final.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stress.rs": "ee3231edbb0f2d59309cb1259513ce31421983912718203b2effeb8fe0a9ebaf",
    "focused.rs": "d0b162bab4c6eea67e36db245857785d8e569f8ddcbf8a67028cad80c8224c20",
    "extra.rs": "44b0867222cc092bce361fcd21658ee10df0699886ec4cf6e41b1576347f88af",
    "accessibility.rs": "32871dc969f248e8e539029bc4de3fc3cdd0053239e9acd6aef7a486b8a59773",
    "historical.rs": "1c54f0cfbc3332c8a8650d3221b9ebe860ae44ae2c02c1cfbdae468d8fc85bc7",
    "hash_historical.py": "6220c0bead959f55c5c5d232d3fe8f311ede89ee51224a208472a1ed1dd6cdf9",
    "stress-modes.rs": "e464ae8fa058885629b0e635ce413106b5de6aa6f0bfea6d5157a6d81df21348",
    "failure_set.rs": "1d4f0d3533b700fcfb5d4aa063a31cfd94e7952aeccf7724fb8bb0744ebcdb8a",
    "route_set.rs": "2cefaf1c756a81c56cc3df1164b29296cf5dc767354918532982a38e5c5382d7",
    "final-inputs.json": "d35204112e98b9085c08d1b099fcaf233805849e116ae13471ddf86b0ec2a268",
    "include-inputs.json": "07b8d523bed828b0e2a38cc76106c13ed19468f3b9768adfa081c6986d0d385e",
    "final-compile.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "final-1024.jsonl": "b505a2cf9ca58850330359dc1ec387231807f063b4c8df3c1bf8b595d255770a",
    "final-1024.log": "1bdcbcdd202cc7b4c22ca674d4758a8b31f02a8e0b72bec88662c121eee59e75",
    "modes-blessed.jsonl": "0a7fd6a517e0059d5abfba96691d09d66e8ec4b50b38b8563811028bf8d0117b",
    "modes-blessed.log": "676ce5c89b958ceab7122e37b7bb698072d46d9688a3f7157d2110fdf5113bb2",
    "modes-avatar.jsonl": "2be4de98e5c85d9fb5164309c46a94f92a1a00836e8d2b2730c0e10dea5589bf",
    "modes-avatar.log": "f5f0bbca089f059974cd24fc72afc29646d97912dd163849692ed2d5ff63b7c7",
    "focused-final.jsonl": "15d3fcfec7bfae67fc2b0a38c88f0593fd57522d9f5e1bfb241dc8e8815c6522",
    "focused-final.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "extra-final.jsonl": "626097950e12765a94a29ca66e7f74abb97fb9d6f96d4c20b294db396ad54d11",
    "extra-final.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "accessibility-final.jsonl": "82759282ce04ef75ee641ad1696d7695b562eaed0d11ba076159d5ed9282de28",
    "accessibility-final.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "historical-baseline.jsonl": "25a4ef60c716e6f90fd13596f4a1d8ea51bfba160bca4ce7ed3a3776f001003f",
    "historical-final.jsonl": "25a4ef60c716e6f90fd13596f4a1d8ea51bfba160bca4ce7ed3a3776f001003f",
    "failure-set-final.jsonl": "a37a2fca42649971130840b3b46c09e2ad8221e4b1a0954e74a96ce85a089c25",
    "failure-set-final.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "route-set-final.jsonl": "6a9eb245f0942d230b1acba6b28f9ddbe144b9bcb2d75422bd9ad0d64221eb83",
    "route-set-final.log": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "baseline-generation-summary.jsonl": "cbd94fd9f2a1e2cd0efa7baf85ec806bdd55ba32be5ed90c3d8b7e26f8ef693d",
    "pre-fix-summary.json": "d9ba2fea765f77ab1d404564c94e5ff25361acc757b80e15f7720ee0460e94b7",
    "make_receipt.py": "719e7b956b1c1cc8f79a4cebc201420b9fa1e542c83a7a8d82bd31400b7be471",
    "repro.rs": "6561a40c8c6474c6f4e2d369b2ff99ca8a0082e52112779118fe4600e8b9f6c7",
    "repro-baseline.log": "d41109bd26fcc20716b85a94755cbfb47b0379aeb0f320bc7f5d1132a1ab75b4",
    "repro-geometry.rs": "dd63bc01bf9e111480414ae5b044bde77f638413fbaa2862cbd8108b345bd64f",
    "repro-geometry-baseline.log": "af1b3f5ca66c6472b5b51d2ac819d0b138d2c1cfebaa4800fe406ea304fb55a1",
    "repro-other.rs": "388e4d22e96efdc8a6f0f12d7f450e7ec3822941148c87a917bc4a0881fe2cb3",
    "repro-other-baseline.log": "ab226cecf6480a44409f5990968645af35f62d09ebbae8f9629514bcc21a9c74",
    "transition_repro.rs": "9f74b22729972ceefd4ab4030a170353cf24fb80d206a2a4226508ba5ea17438",
    "transition-repro.log": "331fcd25eead78afb27f32645311decab4f801a77ab59d4c2c24f8b88f0597e7"
  }
}

LIMITS
New descents/DEBUG use revision 38. Explicit revisions 29–37 retain historical gameplay and generation for replay compatibility; old recorded failures remain reproducible. Invalid configuration, malformed fixtures and corrupt catalogs still fail explicitly. Exhaustion after bounded safe candidates remains an explicit error, not an unsafe fallback. Finite tests cannot prove every possible seed is infallible. No known valid-input r38 failure is silently omitted or replaced by a different seed in reported final tests.

Windows x64 cross-build and static PE/resource checks passed. Unsigned executable; WINDOWS RUNTIME NOT TESTED. No new interactive-window verification claimed.

Evidence: INTEGRATED-VERIFICATION.json, docs/generation-hardening-r38.md and verification/integration-r38/. Initial test-harness assertion mistakes are distinguished from game defects in retained notes/logs. One complete pre-final aggregate run had 917 passes and two stale test assertions (default revision37 and the superseded Mask description); both assertions were updated, and the full suite was rerun. The exact two-line test-only diff is retained under failed-gate-stale-assertions. User-authored copy evidence and newly authored generation diagnostics are preserved separately.

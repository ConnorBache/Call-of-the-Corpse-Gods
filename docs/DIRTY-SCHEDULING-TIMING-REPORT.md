# Dirty scheduling first pass — final-candidate measurements

## Results and verified candidate gates

The final corrected candidate reduced median core dispatch time by **50.0–84.2%**
across eight tested scenario/command combinations. Ordinary wait p95 was
**3.499 ms**; crowded wait p95 was **7.677 ms**. All eight final canonical states
matched the baseline byte-for-byte in all three paired measurement batches.

Crowded hover plus dirty refresh improved from **115.213 to 15.196 ms median**
and **128.562 to 24.524 ms p95**. This is substantial but still above the proposed
2 ms hover budget. Early save/paint regressions did not reproduce in isolated
final-candidate repeats. Their small changes and overlapping batch ranges do
**not** establish a targeted save or painting optimization.

The final candidate passed the full workspace suite: **142 targets, 974 passed,
0 failed, 1 ignored**, process exit 0, with runtime-timing enabled. The standard
feature-OFF Windows release cross-build also completed with exit 0. These are
verified candidate gates; artifact packaging/publication is separate and not
asserted complete here. The comparison baseline remains source v14. Windows
runtime behavior is not verified on this Linux host. The universal
function-complexity audit is still paused/incomplete.

## What changed—and what did not

This bounded first batch adds scheduled bulwark expiry, conservative dirty
maximum-HP settlement, exact lighting projection caching, and lazy path-mask
construction. It does not put every effect on a Schedule. Actor lookup remains
linear first-match; public actor edits prevented a reliable index. General
status/ability timers, equipment/shared-body dirty tracking, Banshee/Epidite,
temporary allies/flight, water/feeding active lists, and global due-floor
indexing remain deferred. AI/targeting and environment algorithms are not
redesigned. See DIRTY-SCHEDULING-FIRST-PASS.md for scope and inclusive complexity.
The combined comparison does not isolate each optimization's contribution.

## Final core dispatch: instrumentation feature OFF

All values are milliseconds; 90 samples/version/row, three alternating batches
of 30 after three warmups. Ranges are min–max batch medians, not confidence
intervals or worst-case bounds. P95 is nearest-rank from pooled raw samples.
Wait advances one world tick; pickup here advances zero and is not tick latency.

| Scenario / command | Baseline median | Final median | Baseline p95 | Final p95 | Median reduction | Baseline batch range | Final batch range |
|---|---:|---:|---:|---:|---:|---:|---:|
| generated-entry / wait | 10.168 | 2.962 | 12.054 | 3.499 | 70.9% | 9.708–10.747 | 2.905–2.976 |
| generated-entry / pickup | 7.833 | 1.722 | 9.544 | 2.320 | 78.0% | 7.450–8.218 | 1.691–1.748 |
| synthetic-no-actors / wait | 4.694 | 2.050 | 6.546 | 2.810 | 56.3% | 4.581–4.857 | 1.971–2.186 |
| synthetic-no-actors / pickup | 2.926 | 1.464 | 3.960 | 1.997 | 50.0% | 2.813–3.362 | 1.458–1.469 |
| synthetic-crowded-plus32rats / wait | 25.030 | 6.249 | 30.481 | 7.677 | 75.0% | 24.380–26.024 | 5.950–6.511 |
| synthetic-crowded-plus32rats / pickup | 16.612 | 2.622 | 19.304 | 3.375 | 84.2% | 16.103–17.134 | 2.515–2.811 |
| synthetic-three-involved / wait | 12.583 | 5.303 | 15.507 | 7.173 | 57.9% | 12.321–12.816 | 5.138–5.467 |
| synthetic-three-involved / pickup | 7.677 | 1.816 | 11.207 | 2.514 | 76.3% | 7.467–7.925 | 1.760–1.920 |

## Final isolated desktop components

Crowded immutable input, 90 samples/version/operation, three batches of 30 with
three warmups. CORPSE_TIMING_ONLY selects one exact diagnostic operation before
warmup; skipped operations are reported, not timed. Operation order is shuffled
with seed 20261030 and version order alternates. These are collector-disabled
samples in feature-enabled desktop binaries, not feature-OFF core timings.

| Operation | Baseline median | Final median | Baseline p95 | Final p95 | Median reduction | Baseline batch range | Final batch range |
|---|---:|---:|---:|---:|---:|---:|---:|
| save_encode_only | 67.142 | 66.352 | 78.201 | 73.773 | 1.2% | 64.583–71.086 | 64.822–67.568 |
| autosave_force_existing_verified | 68.904 | 65.665 | 82.425 | 75.118 | 4.7% | 66.547–72.793 | 62.302–71.482 |
| software_render_1280x800 | 7.985 | 7.514 | 11.557 | 8.513 | 5.9% | 7.610–8.127 | 7.222–7.679 |
| software_render_1920x1080 | 14.547 | 13.652 | 21.264 | 17.056 | 6.2% | 14.118–14.805 | 13.026–14.963 |
| ordinary_hover_event_plus_dirty_refresh | 115.213 | 15.196 | 128.562 | 24.524 | 86.8% | 110.187–118.991 | 14.739–17.676 |

The early combined desktop run suggested save/paint regressions. The separated
repeat above did not reproduce those regressions. Save/paint were not targeted
by this batch; do not promote the small median reductions as established gains.
Saving remains about 66 ms and 1080p painting about 14 ms, both important future
work. Hover still dominates this tested desktop interaction.

During these isolated runs one-minute load average ranged approximately
1.01–1.10. CPU-frequency telemetry was null and thermal telemetry unavailable;
therefore frequency/thermal stability is not established. Builds/tests were not
run concurrently with these measured repeats. No confidence interval or
hardware-general speedup is claimed.

## Corrected all-seeing boundary check

One paired run, 30 samples/version/command—not the 90-sample core experiment.
This checks the no-lighting-needed boundary; small differences are not a broad
performance claim. The following single-run ranges repeat the one median.

| Command | Baseline median | Final median | Baseline p95 | Final p95 | Median reduction | Baseline batch range | Final batch range |
|---|---:|---:|---:|---:|---:|---:|---:|
| wait | 2.442 | 2.277 | 2.859 | 2.894 | 6.8% | 2.442–2.442 | 2.277–2.277 |
| pickup | 1.260 | 1.198 | 1.609 | 1.468 | 5.0% | 1.260–1.260 | 1.198–1.198 |

## Correctness and measurement boundaries

- Exact canonical state equality is recorded for all final core comparisons.
  The bundled proof hashes every batch against baseline and includes one final
  canonical-state set; redundant identical state copies are omitted.
- Generated and crowded scenes at 1280×800 and 1920×1080 have exact decoded RGBA
  equality (four comparisons), not merely visually similar PNGs. Evidence is in
  evidence/render-parity/final-comparison.json (current desktop binary hash).
- Core timing excludes setup/generation, verification, and DispatchOutcome
  disposal. Every command uses an independent cloned starting state rather than
  a continuously evolving campaign. Desktop operation timing includes result
  disposal. Save encoding is distinguished from a forced existing autosave.
- Desktop painting excludes SDL upload/presentation/vsync and input processing.
  Hover measures the named inspection/route/observation pipeline, not complete
  input-to-visible latency. These times must not be added to claim a measured
  complete frame; overlapping profiling spans must not be summed either.
- Final numeric tables use comparison-final and components-final ONLY. The
  earlier comparison directory is the first candidate. Its profiling data can
  explain early investigation but is not final-candidate attribution; no final
  per-phase ranking is asserted here. Superseded timings are omitted from this
  bundle to prevent accidental substitution.
- A finite set of save and pixel matches is not a proof of every transient
  callback, long sequence, RNG edge case, or untested time-power interaction.
  The full-workspace suite passed (974 tests, zero failures, one ignored), and
  the Windows release cross-build succeeded. Evidence and full test log are
  bundled. These checks do not prove every untested sequence or Windows runtime.

## Proposed engineering budgets—not guarantees or newly implemented features

| Work | Proposed budget | Current evidence / gap |
|---|---:|---|
| Ordinary core wait p95 | ≤5 ms | 3.499 ms in this fixture |
| Crowded core wait p95 | ≤10 ms | 7.677 ms in this fixture |
| Hover pipeline | ≤2 ms | 15.196 median / 24.524 p95; still over budget |
| Observation | ≤2 ms | No final isolated observation result in this report |
| 1080p software paint | ≤4 ms | 13.652 median / 17.056 p95; still over budget |
| Input-to-visible | ≤16.7 ms | Whole pipeline not measured |
| Blocking save work | ≤1 ms | Requires future design approval; not implemented |

These are proposed engineering targets for discussion, not universal acceptance
criteria already met. Save architecture changes need a separate design decision.
No future optimization is implicitly authorized or represented as implemented.

## Reproduction and evidence

The evidence ZIP contains raw final JSON samples, exact state/pixel proof,
selected inputs and final states, measurement manifests with binary SHA-256s,
and scripts. It contains no executable. Reproduction requires matching source,
compiler/profile, dependencies and locally built binaries; scripts retain their
original workspace-relative layout and must be placed accordingly. The immutable
baseline is the profiling/menu baseline; the final corrected candidate is the
first-pass source. Diagnostic selection changes benchmark execution only.

Use run-comparison.py for the paired core runs and run-components.py for isolated
desktop operations. The all-seeing pair uses the bundled input and the core
harness --input option (see run-additional-comparison.py). Do not run comparisons
while builds/tests are active. Baseline/final binary hashes and exact execution
order are in reports/comparison-final/manifest.json and
reports/components-final/manifest.json. This report does not claim those Linux
binaries establish Windows behavior.

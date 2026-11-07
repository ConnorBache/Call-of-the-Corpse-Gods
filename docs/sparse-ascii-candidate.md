# Sparse ASCII unit-opacity candidate

This bounded candidate only changes cached ASCII map-glyph drawing with exact
unit opacity. Zero coverage previously left RGB unchanged and preserved the
destination alpha byte, so skipping those samples is exact. Text and compound
glyph routines which force destination alpha remain unchanged. No painting
policy, retained-framebuffer, simulation or gameplay change is included.

## Cache and traversal contract

Each existing ASCII cache entry retains its original native coverage bytes and
adds flat maximal nonzero row runs plus u32 row offsets. One scan builds metadata
at cache insertion. Resize and the existing cache-cap clear free metadata with
coverage. Full control and non-unit opacity still read the original bytes.

The sparse loop intersects the same logical/native damage bounds, italic shear
and ordered tile row spans with cached runs, preserving row/pixel order. It does
not rescan transparent bytes each frame. Negative origins and fractional native
sizes keep the same rounding. Cache construction costs O(W*H), extra payload
4*(H+1)+8*R bytes where R <= H*ceil(W/2); vector capacity can be greater. Warm
complexity includes clipped rows, tile-span lookups, binary search of runs,
intersected runs and actual nonzero samples. See the complexity ledger.

## Validation and measurement commands

Implementation is staged, not yet compiled or run. Run only after the lead grants
a quiet build/measurement window, using the project's pinned toolchain. Preserve
all earlier binaries and reports, and use a new target directory and report path.

Focused validation filters are `sparse_ascii`, `alpha_one_oracle_tests` and
`paint_profile_`. Normal `tools/cargo.sh test --workspace` remains required.
The new tests reconstruct run coverage; compare actual rendering with the old
independent scalar float oracle under simultaneous logical, damage and tile
clips, negative origins, italic shear, arbitrary destination alpha and fractional
1537x911 sizing; and compare full framebuffer bytes at 1537x911 and 3840x2160
across game/Pack and cosmetic clock transitions. Cache clearing is checked.

The existing `real_tick48_native_paint_profile` remains available. The new ignored
`render::paint_profile_bench::real_tick48_sparse_ascii_profile` reuses the actual
tick48 workload and emits eight same-binary controls: full/sparse ASCII, original two-pass/fused
background, and collector OFF/ON. Execution positions rotate per round. Both
candidate toggles are disabled for the expected reference frame; the independent
flags permit sparse-only, fused-only and combined comparisons. Sample
counts, raw timings and CRCs are retained for every arm; no prior report changes.
Every arm also compares every framebuffer byte against the full-control frame
outside the timed interval; CRCs are reporting fingerprints, not the parity proof.

```sh
CORPSE_PAINT_INPUT=/workspace/scratch/69f53637ba6f/overnight-timing/reports/live-probe/roundtrip.json \
CORPSE_PAINT_OUTPUT=/absolute/new/sparse-ascii-report.json \
CORPSE_PAINT_VARIANT=sparse-ascii \
CORPSE_PAINT_SAMPLES=20 \
tools/cargo.sh test --release -p corpse-desktop \
  render::paint_profile_bench::real_tick48_sparse_ascii_profile \
  -- --ignored --exact --nocapture --test-threads=1
```

The report enumerates actual cached glyph entries outside timed paint: coverage
bytes/capacity, zero/nonzero samples, run count, offset count, metadata payload
and capacity. These are not total renderer heap or RSS. ON ASCII counters sum
actually clipped full-mask area, nonzero area and intersected runs across all
draws, including repeat draws and clipping/shear, rather than treating unique
cache occupancy as the workload. Sparse pixel visits count only traversed
nonzero samples; full control pixel visits include transparent samples.

Collector OFF is the latency comparison. ON includes diagnostic costs, including
an additional cached-run scan in the full control to count nonzero occupancy.
There are no per-pixel clocks or per-frame full-mask occupancy scans. Both arms
retain and build identical metadata, so this same-binary control does not by
itself measure metadata-creation overhead versus the prior alpha-only binary.
Cold n3 is diagnostic. Exact scalar/full-frame gates and the frozen prior
baseline must accompany any performance or release conclusion. No speedup is
claimed before measurements run.

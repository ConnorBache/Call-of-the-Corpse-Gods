# Revision 50 recording protocol preparation

Desktop recording metadata supports R50. The coordinated
[release admission and acceptance gates](revision50-release-admission.md) now
also admit R50 Engine, storage, seed replay, startup, debug and playtest paths.
Protocol support and authored tests alone make no platform-verification claim.

## Protocol and compatibility

The exact new pairs are `corpse-r50-ability-roster-v1` / revision 50 for
command receipts, and `corpse-r50-ability-roster-metadata-v2` / revision 50 for
ordered metadata/command operations. Both use the existing snapshot-overwrite
policy. The metadata writer can select a header for revision 50 instead of
falling back to whole checkpoints once the independent runtime gates open.

Every R39–46 build ID and policy is unchanged. `Header::current()` still means
the historical revision 39 receipt protocol. Revisions 47–49 remain unsupported;
unknown IDs, mismatched build/revision pairs, fractional and nonfinite selections
remain rejected. Framing and CRC chaining do not change, and receipt/metadata
payloads cannot be interchanged.

Certified resume recognizes clean R50 metadata recordings only **after** the
existing full decode/replay succeeds. Receipt recordings, legacy R39/40 metadata
recordings, and recovered partial suffixes retain their existing checkpoint
migration behavior. No source-conflict protection, locking, file identity,
acknowledgment, synchronization or operation application rule is changed.

## Complexity

Let I be build identifier bytes, H compatibility-header bytes, B file bytes,
R committed batch records, P batch payload bytes, and V typed operation validation
work. Revision selection adds one member to a fixed list: O(1) time and O(1)
extra memory, apart from its bounded returned string. Exact pair/grammar and
resume selection cost O(I) time and O(1) extra memory. I is limited to 128 bytes;
H is limited to 1,024 bytes. No per-turn or rendering work is added.

Existing framing remains O(B + V) time with O(R + H) borrowed record/header
storage; it does not copy checkpoint or record payloads. Typed batches are parsed
and validated using the existing O(P) temporary representation plus their
operation-specific validation costs (display-order ID scans and run-ID string
validation). Each batch remains capped at 256 KiB and 32 operations, the file at
32 MiB, and batch records at 65,536. No new cache is introduced.

Certified load remains O(B + validation + replay work), retaining at most the
existing bounded file until attachment. Replay work is the sum of transitive
simulation costs, not constant work per command. First attachment verifies O(B)
bytes under the existing lock. Appends still cost new payload encoding/CRC/write
plus existing bounded attachment checks and OS synchronization; the protocol
extension adds no prefix scan. These are source-based bounds, not new measured
performance results.

## Verification boundary

`revision50_protocol_tests.rs` exercises literal legacy/R50 ID preservation,
exact-pair rejection before checkpoint decode, current-header identity, actual
receipt/operation grammar separation, byte-exact checkpoint and ordered metadata
records (including duplicate IDs, negative zero and UTF-8), every final-frame
truncation, complete-frame corruption, and refusal to relabel an admitted R46
checkpoint as R50. These are framing/grammar tests, not R50 gameplay replay.

`revision50_resume_header_admission_preserves_legacy_boundaries` tests the pure
post-decode resume selector without constructing an R50 engine. The separate
`revision50_protocol_integration_clean_resume_appends_without_replacing_recording`
regression is enabled with the coordinated admission patch and uses an actual
generated R50 start. It must pass after mechanics and all admission boundaries
are complete; it checks R50 load/append, metadata, command replay, prefix and
physical file identity preservation. Authored or enabled is not the same as passed.

No Cargo command was run during this patch preparation because the coordinated verification process owns
the shared build slot. Targeted filters once available:

- `tools/cargo.sh test -p corpse-desktop revision50_protocol`
- `tools/cargo.sh test -p corpse-desktop revision50_resume_header`
- Existing `revision46_protocol`, `metadata_operation_tests`, `recording_resume`
  and `incremental_log` filters for compatibility.
- Final release gate: `tools/cargo.sh test --workspace` after integration.

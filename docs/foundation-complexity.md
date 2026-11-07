# Foundation complexity ledger

Let F be floors (<=40), V total cells, A all floor actors, P player/nested actor data, I inventory/metadata, L total archive/replay entries, H<=5 retained turn openings, and S full serialized state size. Measurements are recorded separately; these bounds do not imply a speedup.

- RNG/hash: draw/mix O(1) time and space; hash O(Unicode scalar count), O(1) scratch. Wrapping integer operations match JS bitwise semantics.
- Schema decoding/encoding: O(S) time and memory. serde rejects unknown fields; bounded RLE limits a grid to3600 cells. Legacy anchors can contain nested serialized states, so string/anchor size remains part of S.
- Snapshot capture: O(F+P+I+anchor bytes) time/retained allocation, O(F) floor/actor/ground Arc references. Does not iterate V/A/L. Ordered per-floor record maps clone their F keys/references. Scalars/player/inventory and non-floor metadata are copied. Immutable catalog data is not retained in snapshots.
- Floor mutation: Arc::make_mut first edit O(cells+metadata of that floor) and same extra memory if shared; subsequent edits uniquely owned O(1) access. Actor/ground edit O(actors/items on touched floor) on first detach. Worst-case updates may touch every floor, yielding O(V+A).
- Ring: five completed openings plus closing/pending checkpoint; push/pop O(1) plus snapshot creation. Free mutations update the closing checkpoint and do not add a completed frame. Initial completed-state sharing does not excuse required offscreen updates.
- Revert: currently O(S) JSON conversion/diff/application plus snapshot restoration and resource telemetry, O(S) scratch. Snapshot comparison uses exact parsed state; it does not yet exploit Arc pointer equality inside the temporal JSON conversion. This is a known optimization gap, not an O(changes) claim.
- Save/history export/import: O(H*S) time/scratch in the conservative implementation. Export produces semantically equivalent legacy inverse patches. Scalar array packing comparison includes UTF-16 JSON length. Save after Revert is tested because prior costs may introduce untouched state fields that sparse historical patches must preserve.
- Future: O(S) snapshot serialization/restoration with retained cost adjustment. Future anchor strings can dominate clone costs. No recursive anchor deduplication optimization is claimed for native export yet.
- Replay: per step cost of native dispatch; O(commands) stored ledger; fixture load separately O(S). Unsupported command retains cursor and original state.

`tools/cargo.sh run --release -p corpse-core --example foundation_bench` measures controlled-room capture and exact eight-command slice, with0 versus100000 replay/archive entries. `generated_bench` uses actual generated baseline payloads, verifies all fields roundtrip before timing, and separates parse, snapshot, touched-floor COW, history export, and undo. Both language sides run on the same shared cloud host; concurrent work and different warmup/sample counts introduce noise. No GPU, browser rendering, user-platform or full-game result is implied.

Validation extension: source-semantic boundary checks currently convert the typed state to a JSON boundary value once, then traverse applicable fields: O(S) additional time and scratch. This does not occur on ordinary dispatch; it does affect Engine startup/save import. Recorded generated benchmark `parse` measures serde parsing only, not this newly extended semantic admission cost. The initial controlled Avatar benchmark used the earlier eight-command trace; the fixture has since grown to13 commands, so rerun that command benchmark before quoting current end-to-end results.

Enemy-health migration: O(A + nested pilots), first mutation detaches affected actor blocks; no RNG or event emissions. Temporal migration rebase: O(H*S + H*normalization), H<=5, using original reconstructed states before applying any migration. Older full-pipeline admission stays disabled.

Population, full native startup, Sphinx and authoritative ground-placement costs and differential scope are documented in [population-startup-coverage.md](population-startup-coverage.md).

Profile: summary construction O(K L log K) time/O(K L) detached output, K heart labels and L maximum UTF-16 label length. Archive and parse scan P replay commands, R existing records and their heart lists; hash-set uniqueness is expected linear, ranking O(R log R) comparisons, with O(R + all retained summary bytes) output copies. Identity generation is UI supplied and never consumes gameplay RNG. Seed-based replay startup/restart costs native generation/population plus full semantic admission; step adds O(1) cursor bookkeeping to dispatch and preserves detached presentation events.

Travel, fall cohorts, descent and corpse placement bounds and admission scope are documented in [travel-coverage.md](travel-coverage.md).

Divine and integer-health migration helpers use a health-only derived projection, no gameplay dispatch/log/RNG side effects. With A actors/pilots, B ability entries, I equipped items and E enhancements, O(A*(B+I+E)) worst-case time and O(actor metadata + I+E) scratch per actor, plus COW actor-block detachments. Initial-player context intentionally excludes current equipment. These standalone tests do not authorize automatic older-save normalization until the full ordered pipeline is integrated.

Half Here identity/projection/lifecycle bounds and integration gates are documented in [half-here-coverage.md](half-here-coverage.md).

Inventor/Mech family helper costs and differential scope are documented in [inventor-mech-coverage.md](inventor-mech-coverage.md).

Tiger/Sabretooth stalking bounds and current differential scope are documented in [stalking-coverage.md](stalking-coverage.md).

Horn creatures and dedicated caster behavior costs and verification scope are in [caster-coverage.md](caster-coverage.md).

Valta/Moonspawn blink and charged-beam costs and differential scope are documented in [moon-coverage.md](moon-coverage.md).

Held movement, River Drake knockback and Bombardier blast bounds and verification scope are in [grab-blast-coverage.md](grab-blast-coverage.md).

Compatibility loader orchestration: let H<=5 history frames, S the full state bytes including nested anchors, and U distinct serialized Future anchors. Each timeline reconstructs original states and normalizes/diffs independently: O(H*S + sum normalization work), retaining O(H*S) inverse output and O(S) working copies. Future cache retains O(total distinct anchor bytes); nesting is explicitly capped at32 rather than allowing unbounded recursion. World normalization scans all items/actors/floors; equipped health projection can add O(A*I). Placement may inspect up to40 floors and trigger exact generation/population, whose separate costs apply. These are load/save boundaries, not per-command simulation costs. No new whole-game speedup is claimed.

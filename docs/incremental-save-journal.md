# Incremental durable saves and bounded checkpoints

## Ownership and coverage

The desktop's tracked Engine owns a private core Engine. Ordinary dispatch records the exact command and a small after-command sanity stamp (RNG/tick/depth bits, phase, status, replay length, retained history count). This is **not a whole-world checksum**. Any mutable reference escaping the wrapper first marks the session as requiring a full checkpoint. Thus pack display-order changes, DEBUG mutations, terminal run IDs and unknown callers cannot be silently represented as commands. Exact-equal pack order assignments are skipped without conflating missing/null/empty values. Failed/panicking dispatch and receipt overflow also require a checkpoint. Existing core Engine APIs remain public; untracked Autosave callers use full snapshots.

A successful durable save acknowledges a precise session/epoch token. Pending receipts survive failures. Appending requires the receipt base token to equal this particular file's last acknowledged token; saving another branch/path cannot accidentally attach a suffix to an unrelated prefix. Session clones, loads, imports and new games require an initial checkpoint. Free actions, pending haste, Revert and Future are replayed through the temporal dispatcher, not approximated by ordinary world ticks. In-memory selective inverse history remains separate and retains its original cost/resource semantics.

## File format and recovery

A distinct versioned UTF8 journal container contains one full legacy save envelope followed by command batches. Fixed-width frame headers include kind, monotonic sequence, prior checksum, payload length and separate header/payload CRCs. The header checksum is checked before trusting length. CRC detects accidental corruption; it is **not cryptographic authentication**.

An incomplete final header/payload/trailer is ignored only when it is a valid EOF prefix after a complete checkpoint. A complete corrupt frame, unknown field, invalid command, incompatible replay build/revision or replay-stamp mismatch rejects the file. Torn multibyte UTF8 payloads are handled as bytes before text decoding. Loading does not rewrite the file; the next successful save checkpoints the live state. Retry after append failure checkpoints rather than blindly re-appending possibly complete commands.

Only simulation revision39 enters the journal format. Existing revision29–38 sessions remain compatible full JSON snapshots; their rules are **not silently upgraded**. Legacy plain Run and version1 envelope inputs remain supported. Replay compatibility ID is `corpse-r39-incremental-v1`; maintainers MUST change it when command execution semantics change, even if they otherwise retain simulation revision39. Old replay journals must be loaded by a compatible build and exported/checkpointed before incompatible upgrades; do not bypass compatibility checks.

## Durability and checkpoint cadence

Every ordinary changed save appends and calls file `sync_all` before acknowledging receipts. At32 accumulated commands, or1MiB tail data, a full current-state checkpoint replaces the file through the existing temporary-file/write/sync/rename path. Large command batches, untracked changes, imports and explicit forced saves with no new commands also checkpoint. A near32MiB legacy snapshot that cannot fit framing overhead stays a plain full snapshot. Total input is limited by a streaming32MiB+1 read, not just a racy metadata check.

A stable empty `<save filename>.lock` sidecar uses the standard OS nonblocking exclusive file lock around validation and append/checkpoint/deletion. Contention produces a recoverable error instead of blocking the UI; kernel locks release when the handle closes or process exits. The sidecar is intentionally never deleted by the game. Do not delete/recreate it while game sessions are using the save; different symlink/hardlink names, old builds, manual exporters and editors that ignore this coordination protocol are not excluded. This is a coordination file, not a second persistent-state transaction.

Existing bytes are still read and compared on every changed flush. If they differ, the complete existing save is validated before replacement; malformed or wrong-session files are preserved. We deliberately do **not** replace this with length/mtime-only trust. Unchanged-key saves retain their skip path; terminal genuine-death deletion and replay/playtest exclusions remain. File sync/rename semantics are unchanged from the previous release. There is no new claim of directory-fsync durability, uncooperative concurrent-writer exclusion, or tested hardware power-loss behavior. Native Windows filesystem behavior still requires platform testing.

## Complexity and memory

Let E be existing file bytes, D new receipt bytes, N full current-state/export work, C replayed command count (at most32), W_i the transitive simulation cost of command i, and R retained receipt data.

- Wrapper read access/token operations: O(1). Dispatch adds O(command payload + fixed stamp strings) receipt work beyond existing simulation and inverse sealing. Mutable escape invalidation is O(1). Pending-batch cloning is O(R), bounded32 receipts but not constant bytes. Receipt overflow does not discard admitted commands before acknowledgement.
- Routine save: **O(E + D)**, plus nonblocking lock/filesystem sync latency; O(E + D) transient memory. It does not serialize/diff the full current world. E remains because exact old-file checks are preserved. The existing-byte cache reserves up to1MiB tail space during checkpoints to avoid first-append whole-buffer reallocation.
- Checkpoint: full canonical export/serialization cost N plus O(output bytes) write/sync/rename. Checkpoint spikes remain and must be measured separately and in continuous traces. Full replay/archive history inside the checkpoint can grow over a long game.
- Load: O(E + checkpoint parsing/history reconstruction + sum W_i). CRC/framing and small-batch validation are linear in file/record bytes. Replay stamps are fixed-field guards, not substitutes for exact fixture tests. Memory includes the checkpoint Engine, borrowed frame slices, bounded record metadata and small parsed batches.
- Revert: unchanged inverse-history subsystem, except exact snapshot-identity cache reuse documented in `snapshot-opening-cache-reuse.md`. Incremental disk persistence does not turn undo into replay-from-start.

The persistent format changes; canonical `Engine.save()` and exported legacy envelope remain unchanged. New functions carry local complexity comments. This work does not finish the paused project-wide every-function audit.

## Rollback/export

Older executables do not understand the new journal container. Keep earlier Library versions for rollback, but export a compatible full snapshot before opening progress with an older executable. For genuine runs, the existing F7 source-format export writes a separate full Run JSON (`.v48.json` suffix); supported older native builds can read that legacy form. Controlled/fixture sessions retain their existing provenance restrictions and must use the source conversion APIs rather than pretending to be genuine. Never replace the only journal with an older program's guessed conversion. The new decoder remains able to load old snapshots directly.

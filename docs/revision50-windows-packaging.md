# R50 Windows player packaging

`tools/package-r50-windows.py` is the R50-specific packaging tool. It does not run
Cargo, install software, sign binaries, or execute the game. It requires Python
3.11+ and the existing pinned SDK at `../native-windows-sdk`.

Do not use the older `package-native.py` or `package-compact-windows.py` for this
release: their checkpoint wording and verification assumptions are historical.

## Freeze, test, build, then package

Finish all source/tool edits before taking the snapshot. Commands below are a
recipe, not evidence that this revision has passed. Capture real exit codes and
complete logs. Never create a success report by inserting guessed pass totals.
The report helpers refuse failed/missing evidence, zero executed tests, changed
source, mismatched executable hashes, and altered raw logs.

From the repository root:

```sh
EVIDENCE=verification/revision50/windows-release
mkdir -p "$EVIDENCE"
python3 tools/test-package-r50-windows.py
python3 tools/package-r50-windows.py snapshot \
  --output "$EVIDENCE/source-snapshot.json"
```

The snapshot hashes Cargo.toml, Cargo.lock, rust-toolchain.toml, crates/, fixtures/
and tools/ (excluding Python caches). It also hashes the two docs directly
included by Rust tests: revision50-request.txt and the native physical-balance
smoke profiles. It does **not** hash verification/, target/, generated reports,
or other docs. Updating verification notes after the runs creates no circular
receipt dependency. Source/tool changes require a new snapshot and affected
verification/builds; changing only excluded report prose does not.

Run the final authorized workspace test command. Preserve any test-profile
options in the recorded command, for example:

```sh
TEST_COMMAND='CARGO_INCREMENTAL=0 sh tools/cargo.sh test --workspace --locked --no-fail-fast'
# Run with shell error-exit disabled so the actual process exit is always saved.
set +e
sh -c "$TEST_COMMAND" > "$EVIDENCE/workspace.log" 2>&1
TEST_EXIT=$?
printf '%s\n' "$TEST_EXIT" > "$EVIDENCE/workspace.exit"
set -e
test "$TEST_EXIT" -eq 0
python3 tools/package-r50-windows.py record-test \
  --source-snapshot "$EVIDENCE/source-snapshot.json" \
  --log "$EVIDENCE/workspace.log" --exit-code-file "$EVIDENCE/workspace.exit" \
  --command "$TEST_COMMAND" --platform 'Linux x86_64' \
  --output "$EVIDENCE/test-report.json"
```

`record-test` derives completed target/pass/fail/ignored counts from the Rust
summary lines. It requires an actual successful workspace test log and exit file,
not the older `verification/revision50/summary.json`. Ignored tests remain ignored;
a pass does not claim exhaustive gameplay or Windows coverage. If using a
pipeline such as tee, capture Cargo's exit, not tee's exit.

Cross-build with the pinned Rust 1.99.1 wrapper and MinGW/SDL2 SDK. Do not change
Cargo.lock. Preserve the exact environment/profile overrides if using any:

```sh
SDK="$(cd .. && pwd)/native-windows-sdk"
BUILD_COMMAND='sh tools/cargo.sh build --release --locked --target x86_64-pc-windows-gnu -p corpse-desktop'
set +e
PATH="$SDK/sysroot/usr/bin:$PATH" \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$PWD/tools/windows-linker.sh" \
SDL2_LIB_DIR="$SDK/SDL2-2.32.10/x86_64-w64-mingw32/lib" \
sh -c "$BUILD_COMMAND" > "$EVIDENCE/windows-build.log" 2>&1
BUILD_EXIT=$?
printf '%s\n' "$BUILD_EXIT" > "$EVIDENCE/windows-build.exit"
set -e
test "$BUILD_EXIT" -eq 0
python3 tools/package-r50-windows.py record-build \
  --source-snapshot "$EVIDENCE/source-snapshot.json" \
  --log "$EVIDENCE/windows-build.log" --exit-code-file "$EVIDENCE/windows-build.exit" \
  --command "$BUILD_COMMAND" --host 'Linux x86_64' \
  --environment 'PATH=../native-windows-sdk/sysroot/usr/bin:$PATH' \
  --environment 'CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=tools/windows-linker.sh' \
  --environment 'SDL2_LIB_DIR=../native-windows-sdk/SDL2-2.32.10/x86_64-w64-mingw32/lib' \
  --binary target/x86_64-pc-windows-gnu/release/corpse-desktop.exe \
  --output "$EVIDENCE/build-report.json"
```

Keep these paths/environment values accurate for the command actually executed.
`record-build` checks successful release-build completion, x64 PE shape, the exact
binary hash, and unchanged source snapshot. It records the pinned toolchain
configuration. These records establish a reproducible provenance trail, not a
cryptographic attestation that a supplied log came from a particular compiler.
Do not include secrets in the command/environment fields.

Only after the final evidence exists:

```sh
python3 tools/package-r50-windows.py package \
  --source-snapshot "$EVIDENCE/source-snapshot.json" \
  --test-report "$EVIDENCE/test-report.json" \
  --build-report "$EVIDENCE/build-report.json" \
  --binary target/x86_64-pc-windows-gnu/release/corpse-desktop.exe \
  --output ../release-r50/call-of-the-corpse-gods-R50-windows-x64.zip
```

Existing output archives/reports are never overwritten. A successful package
prints its SHA-256, size and member count and writes a sibling `.verification.json`
receipt. An error means no successful package claim, even if a partially written
archive exists. Investigate failures before replacing any previous artifact.

## Evidence schema

Both report helpers write schema 1, revision 50, source_snapshot_sha256, command,
exit_code, the raw-log/exit-file paths, byte sizes and SHA-256 hashes, and
windows_runtime_verified=false. Test reports add actual platform, parsed counts
and coverage limits. Build reports add host, target, configured toolchain,
environment/profile overrides, and exact executable bytes/hash. Packaging
rechecks the live source snapshot, both raw logs and exit files, test counts, and
executable identity. Raw logs stay in the workspace; concise reports and the
complete source-file hash snapshot go in the player ZIP.

## Runtime and integrity checks

- `START-HERE.txt` says Windows 64-bit, extract the whole ZIP, keep
  corpse-desktop.exe and SDL2.dll together, and play in the SDL graphical window.
  It is not a terminal game; the current executable can also display a console.
- The executable is copied to a temporary directory before stripping. The input
  bytes are checked again before and after writing the ZIP. Stripping must leave
  every non-debug mapped section's bytes, virtual address/size and flags
  unchanged, together with critical loader headers and non-debug directories.
  Only debug sections/directories and surplus trailing image space may change.
  Each PE's image size must still cover every retained mapped section.
- Both EXE and DLL must be PE32+ x86-64. Signed and delay-import inputs are rejected
  rather than silently weakening inspection. EXE imports are compared before and
  after stripping. SDL2.dll plus a narrow Windows system-DLL allowlist is the only
  admitted runtime dependency set. SDL2 itself may import only those system DLLs.
- The bundled SDL2.dll must exactly match the runtime member of the cached official
  SDL2 2.32.10 archive, whose pinned SHA-256 is rechecked. The compiler, sysroot,
  downloaded archives and development libraries are never bundled. Only selected
  SDK provenance JSON records are included, not download response headers.
- Every current creature/UI WebP is checked against its runtime manifest hash and
  found byte-for-byte inside the input EXE. The inherited manifests' pixel/quality
  claims are preserved as evidence, not newly re-tested by packaging. Historical
  notices are clearly labeled as historical where their counts/encoding differ.
- All locked external Cargo dependencies must have corresponding versioned notice
  entries; declared notice files must exist. Font OFL, SDL2, Brogue artwork,
  authored artwork and alpha/runtime provenance notices are retained. No new
  game-license grant is implied.
- Member order, timestamps, UNIX file mode and ZIP compression settings are fixed.
  ZIP CRC, exact member list/count, bytes and SHA-256 are read back. Identical
  inputs and compression-tool implementation yield identical ZIP bytes. The
  external receipt hashes the complete archive; MANIFEST.json hashes every other
  member, avoiding a self-hash cycle.

All packages say **unsigned cross-build, NOT Windows-runtime-tested**. Successful
host-side tests and static PE inspection do not establish Windows launch, input,
rendering, audio, fullscreen, saving or gameplay. The current provisional design
choice is Repulsion at exact center: a random valid outward direction using the
serialized gameplay RNG. Other R50 design decisions are resolved; that does not
claim all tests/platform acceptance have completed.

Basic controls were checked against corpse-desktop/src/main.rs, debug_floor_ui.rs
and the existing debug-floor UI. F9 in play explicitly reloads a saved run, losing
unsaved progress; it is not described as harmless Continue.

## Complexity and packaging-only checks

Let N be source/member paths, B total input bytes, S PE section count, A embedded
art bytes, P original executable bytes, K embedded resource count and Z archive
bytes. Each packaging invocation performs O(B + N log N) streaming source
hashing/sorting per snapshot pass. It retains O(N) hash/path metadata and a bounded
hashing buffer. PE extraction/comparison is O(S + P) time/memory. Art verification
hashes O(A) bytes and performs K exact substring searches, O(K*P + A) worst-case
with Python's bytes search, retaining O(P + largest resource) bytes. The cached
SDL gzip archive is hashed and decompressed linearly in its size.

Notice collection and ZIP generation/read-back use O(B + N log N) data visits plus
zlib's compression work and filesystem latency. At fixed deflate settings/window,
compression is linear in input size with implementation-dependent constants.
Peak memory is O(B + P + Z + N), since runtime file bytes, original/stripped PE
section copies and the final archive-hash read can coexist. There is no gameplay
or per-frame cost. Bounds here are analysis, not measured packaging timings.
No asset dimensions are expanded or decoded by this tool.

`python3 tools/test-package-r50-windows.py` exercises synthetic PE rejection,
strip comparison, DLL allowlists, actual-summary parsing, stale-source and
altered-report/log rejection, deterministic ZIP read-back, and player wording.
It neither builds nor launches the game and cannot replace final workspace
regression tests, cross-build evidence, or actual Windows QA.

## Attachment-size-safe delivery

The chat attachment limit is 20,971,520 bytes. The original full R50 ZIP exceeded
that limit even though its standalone executable ZIP fit. Do not claim successful
chat delivery from successful Space upload alone, or resend the oversized file.

After creating the new final game ZIP, run:

    python3 tools/test-split-game-delivery.py
    python3 tools/split-game-delivery.py --archive <verified-game.zip> --output-dir <new-parts-directory>

This produces two ordinary Deflate ZIPs, not multi-volume ZIPs or encoded chunks.
The first holds corpse-desktop.exe; the second holds SDL2.dll, instructions,
licenses, manifests and all other original files. Download both and extract both
into the same folder, with SDL2.dll beside the executable. No joining script,
PowerShell command, extra archive software, or executable change is required.
Send the two native attachments in separate messages if message-level byte limits
also apply. Neither attachment alone is the complete game.

The helper validates the original package manifest, preserves the input archive,
checks every output member and byte limit, then extracts both archives together
in a fresh directory and compares the complete file tree byte-for-byte with the
original. It publishes neither part if either exceeds the limit. Its receipt says
file-tree assembly, not byte concatenation of the original ZIP. Game code, art,
DLLs and notices are never recompressed internally, stripped again or changed.

For N files and B total uncompressed bytes, hashing, reading, extraction and exact
comparison cost O(B + N); sorting archive paths adds O(N log N), plus Deflate work
and filesystem I/O. Memory is O(B + N) for original and assembled byte maps, with
additional zlib buffers. Temporary storage is O(B + compressed archive bytes).
This is a one-time delivery operation with no runtime/frame cost. Four tests cover
exact assembly, limit rejection without a partial pair, manifest mismatch and
existing-output preservation. Actual final game parts require their own receipt.

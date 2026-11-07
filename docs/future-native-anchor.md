# Native Future anchor (revision41 performance follow-up)

This is an implementation candidate pending its build, correctness and performance gates. It changes runtime representation, not the Future command's energy/payment, replay or world rules. No compression or new persistence format is introduced.

## One immutable authority

`Run.future` holds an optional `FutureAnchor`: an Arc to either exact imported/legacy text, or an immutable native Run plus an optional derived encoded-string cache. Newly captured finite revision41 anchors use the native representation. Revisions29–40 keep text capture/restore and the existing `Frame.future: Option<String>` wire. Ordinary presence checks never encode. Cloning an anchor is O(1); there is no mutable Run/string escape.

A native anchor is a second game state, but existing COW floor/actor/ground/trail blocks are initially shared. The anchor clone helper explicitly omits Future, replay, replaySnapshotFrom and completed history **before cloning**, preventing an anchor chain and needless command-buffer copies. It retains the full log archive, phase, run identity and remaining canonical metadata. It is not the old Snapshot helper, which has different archive semantics.

Capture still happens after the full current-energy payment/resource-loss recording, before activation log/equipment/door postprocessing. Restore preflight remains before command bookkeeping. Restore preserves live replay, replay policy and runId, restores the anchor world/log archive/phase/trail, clears Future and reapplies cumulative energy costs. Derived optical/light/actor/health caches are reset or invalidated; health is invalidated after the cost overlay.

## Finite native admission and exceptions

A no-output serde visitor traverses canonical numeric values once at capture. It emits neither JSON objects nor text, performs no float formatting and ignores serde-skipped derived caches. Nonfinite f32/f64 values or custom serialization failure choose the old string path instead. Thus optional nonfinite values retain old JSON-null behavior, and mandatory nonfinite values retain the old later-deserialization errors. Nothing is coerced to zero, and there is no new global gameplay-admission restriction.

Native restore uses direct typed inventory-display-order normalization: Missing stays Missing; any present value, including Null, becomes a Value vector of existing inventory IDs, first occurrence only, in original order, treating signed zero IDs as equal. Fresh native anchors retain the existing storage validator on restore: it builds a full canonical JSON Value tree, although the anchor itself is no longer stringified and parsed. Revision/history/trail checks remain. Only imported anchors already successfully validated during load carry a private, process-local certificate that skips this duplicate storage validation. The certificate is not serialized and cannot be supplied by a save file; fresh capture does not acquire it merely from finiteness.

During Engine admission, compatible imported revision41 text is prepared once. A valid matching-revision finite anchor is promoted to native and keeps its original wire spelling as the derived encoded cache. Invalid/mismatched anchors remain exact text; a failed warm-up does not introduce a new load error or discard the anchor. Actual restoration still reports the existing failure. Successful promotion certifies the normalized immutable restore image after all existing admission checks, not the retained raw wire spelling. Removing nested Future/replay/policy cannot invalidate current world validators; inventory normalization is idempotent. Public access returns only an immutable reference, so later current-world mutations cannot invalidate the certificate. Old revisions do not promote. Ordinary commands never retry admission. The cached imported string is the original wire representation, not necessarily a byte-for-byte serialization of the normalized native restoration image: normalization may remove stale display-order IDs, and restoration discards nested Future/replay metadata. Their required equivalence is the resulting restored game under the existing policy. The retained original string and normalized native image both consume memory and are counted; neither is an independently mutable authority.

## Serialization and comparisons

`wire_text()` is an explicit persistence/export boundary. First native use serializes the immutable anchor and retains that string; subsequent calls borrow it. The surrounding save still escapes and copies the entire anchor string into its JSON envelope. A checkpoint therefore does not become O(1) just because the inner string is cached. Imported text saves remain byte-preserving at this wrapper boundary.

`PartialEq` means shared immutable identity or equal legacy text, not arbitrary world semantic equivalence. Distinct native identities and native-versus-text conservatively differ even if their saved worlds are equal. This cannot produce a false Snapshot cache hit; it can produce a safe miss. It never serializes merely for comparison. Whole-world correctness tests explicitly compare canonical output/restored state outside measured intervals.

Normal current-world/admin edits do not invalidate a historical anchor. Replacing or clearing the anchor replaces its authority and cache together. Restart/load owns a fresh wrapper. Engine clones share the immutable anchor **and lazy cache**: calling save on a proof clone warms the live cache too.

## Complexity and actual memory

Let F be COW block handles, M deeply owned metadata/inventory/archive payload, N canonical serialized nodes, D maximum nesting, S encoded anchor bytes, I inventory entries and O display-order entries.

- Native capture: O(F+M+N) time; O(F+M) new retained native storage plus O(D) visitor stack, while large unchanged payload blocks remain shared. Custom Serialize implementations retain their own costs.
- Ordinary anchor clone/presence check: O(1). Existing ordinary core dispatch already takes Future out before candidate Run cloning; this change is not claimed to remove a large string copy on every normal turn.
- Native restore: O(F+M+I+O+validation) plus discarded-current-state destruction. Validation may traverse the world; this is not a pointer-swap-only operation.
- Text restore/import preparation: O(S+validation+N), with old parsing/normalization allocations; paid once when loading/promoting a valid41 anchor.
- First native boundary: O(S) encoding/output allocation, plus O(S) outer escaping. Warm boundary: O(S) outer escaping/output even though inner cache lookup is O(1).
- Current-world divergence: first mutation of a shared floor/group/trail copies that block. A long shared trail can require O(L) detachment. An anchor can retain a substantial second world's old payload after many mutations.
- Peak/retained memory: native metadata + retained old COW blocks + current world + cached encoded anchor after first boundary + transient outer save buffers. `sizeof(Arc)` is not the memory cost. No numerical RAM claim is made before measurement.

The standalone `native_future_alloc` example uses a counting System allocator in that executable only. It reports requested allocation layout bytes, incremental retained/peak deltas and allocation traffic, not RSS or allocator overhead. Stages include real capture, first real Move divergence, first/warm serialization with separately dropped outer outputs, and real restore. Optional10k-arrow imported state measures trail COW retention. Normal latency measurements must use a separate uninstrumented binary. All allocation scenarios use revision41 (10 and40 mean floor counts, not mechanics revisions). No revision40 baseline comparison is made, because that would conflate retained Revert history with the anchor representation. Retained inner cached-wire bytes remain live after each outer output is dropped and are therefore included in the measured allocation totals.

## Proof ordering

The Future action harness defers capture serialization/readback proof until after the first cold checkpoint. Native structural assertions do not encode. The cold checkpoint asserts an empty wire cache before and a populated cache after; a separately labeled warm checkpoint repeats explicit administrative access. All proofs that call save, including cloned-engine proofs, occur after the cold boundary. The older unified-save stress harness intentionally encodes in setup and is not the cold-checkpoint authority.

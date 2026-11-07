# Recovered Git import validation

This source import was reconstructed after the temporary cloud workspace reset.
The complete R51 checkpoint and canonical baseline were recovered from Space.
Their hashes and all 3,844 original source files are recorded in
`git-import-provenance.json`. Fresh restoration and byte-preservation verification
passed for 3,032,245,962 original bytes. The oversized fixture's exact bytes are
retained in two raw parts, with SHA-256 hashes in its fixture manifest.

The signed GitHub root commit, tree and README blob were recovered from exact
GitHub API evidence and verified against their original object IDs. New source
import commits are new commits; the lost pre-reset local commit IDs are not
claimed as recovered or fabricated.

The std-only Cargo restoration helper preserves the previously tested design:
ordered concatenation, fixed-buffer comparison, atomic hard-link publication,
no overwriting local changes, and Cargo rerun tracking for the parts/output.
The helper and full-game tests have not been rerun in this recovered workspace:
Rust/Cargo are unavailable. The source-only helper runner is included for a
properly provisioned host, with explicit pinned toolchain and fixture hashes.
Windows execution is not newly verified. Existing historical test reports remain
historical evidence, not claims about a fresh run.

Freshly run: checkpoint hash-checked restoration, complete source-byte verifier,
fixture-part hash verification, and original Git object reconstruction/fsck.
Bundle and clean-clone verification are recorded alongside the bundle by the
publishing assistant after creating the final commits.

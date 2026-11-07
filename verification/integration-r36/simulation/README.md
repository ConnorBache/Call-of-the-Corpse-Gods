# Simulation evidence — r36

This evidence-only directory was written after the lead froze1057 build inputs at `f0b38fb5e06763c1cae69d2fc0a72c8cc647cd612ca75b50ba5c2ae1eff2ec03`. No source, tests or build-input documentation was changed for this receipt, and no cargo process was started.

## What passed

- Latest simulation physics target:17 tests, including fair reverse draws, local vision, same-plane Half Here, exact physical ray distance, actual items/potions, NPC possible-shot admission, actual horn/witch hit-versus-miss bookkeeping, actual-branch range rejection, and legacy trajectory/charged-moon behavior.
- Earlier precise offscreen phase:1 stochastic-pursuit/no-telefrag unit test.
- Earlier NPC/legacy focused run:15 physics tests plus caster, horn, moon and passive-monster reference-trace targets.

The earlier runs predate later fixes and are not represented as final frozen validation. The lead owns combined focused and complete workspace gates. See `superseded-partial-runs.md` for failed/intermediate development checks.

## Provenance

The `.tool-transcript.log` files transcribe retained `exec`/`write_stdin` output. They are not original shell tee files; this distinction is intentional. Commands, session identifiers, observed completion times and scope are retained. `receipt.json` records the same provenance and the exact simulation file inventory. `modified-files.sha256` records final frozen source hashes for the16 files edited in this simulation workstream.

No Windows runtime verification is claimed.

#!/usr/bin/env python3
"""Bounded paired native R46/R50 bare-player diagnostic, not a full-run fit.

Uses the dedicated revision50_offense_arena example. No compilation or installs.
Preserves input joint profiles and initial combat seeds across the comparison.
Each native response is persisted immediately; errors and censors stay visible.
"""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
PROFILES = ROOT / "tools/monster-balance/relocated-player-progressions-r32-joint-profiles.json"
CASES = [
    ("blorb", "adjacent", 1, "negligible-body"),
    ("rat", "adjacent", 1, "plain-fighter"),
    ("cave-bat", "adjacent", 1, "fast-skirmisher"),
    ("shellback-beetle", "adjacent", 1, "slow-armored"),
    ("hemogoblin", "adjacent", 1, "plain-fighter"),
    ("needleling", "adjacent", 1, "venom-and-aftermath"),
    ("bear", "adjacent", 6, "plain-fighter"),
    ("troll", "adjacent", 19, "regenerating-fighter"),
    ("dire-bear", "adjacent", 20, "armored-fighter"),
    ("minotaur", "adjacent", 33, "late-fighter"),
    ("dragon", "adjacent", 38, "fire-breath-and-aftermath"),
    ("pale:Spitter", "chase", 4, "ranged-gas-and-aftermath"),
    ("pale:Birther", "three-cast-children", 6, "isolated-native-summon-payload"),
    ("ursa-major", "ursa-pair", 36, "linked-pair-stress"),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/examples/revision50_offense_arena")
    parser.add_argument("--trials", type=int, default=32)
    parser.add_argument("--kinds", nargs="*")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/artifacts/revision50-native-offense")
    args = parser.parse_args()
    if not 1 <= args.trials <= 127:
        parser.error("Use 1..127 distinct stored joint profiles per case.")
    blob = json.loads(PROFILES.read_text())
    worlds = blob["profiles"]["all_initial_unblessed"]["entry"]
    selected = [i * len(worlds) // args.trials for i in range(args.trials)]
    cases = [case for case in CASES if args.kinds is None or case[0] in args.kinds]
    if not cases:
        parser.error("No selected cases")
    prefix = args.output.resolve()
    prefix.parent.mkdir(parents=True, exist_ok=True)
    inputs = []
    for kind, scenario, depth, role in cases:
        for revision in (46, 50):
            inputs.append(dict(kind=kind, scenario=scenario, revision=revision,
                profiles=[worlds[index][depth - 1] for index in selected], trials=args.trials,
                seed_prefix=f"r50-native-bare:{kind}:{scenario}", max_turns=128, settle_turns=64,
                settlement_strategy="escape", include_trials=True))
    prefix.with_suffix(".requests.json").write_text(json.dumps(inputs, indent=2) + "\n")
    source_paths = [PROFILES, Path(__file__), ROOT / "crates/corpse-core/examples/revision50_offense_arena.rs",
        ROOT / "Cargo.lock", ROOT / "rust-toolchain.toml"]
    source_paths += sorted((ROOT / "crates/corpse-core/src").rglob("*.rs"))
    source_paths += sorted((ROOT / "crates/corpse-core/src/catalog").glob("*.json"))
    sources = {str(path.relative_to(ROOT)): digest(path) for path in source_paths}
    started = time.monotonic()
    rows = []
    with prefix.with_suffix(".stderr.log").open("w") as errors, prefix.with_suffix(".raw.jsonl").open("w") as raw:
        proc = subprocess.Popen([str(args.binary.resolve()), str(prefix.with_suffix(".requests.json"))],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=errors, text=True)
        for request, line in zip(inputs, proc.stdout):
            raw.write(line); raw.flush()
            row = json.loads(line); row["requested_kind"] = request["kind"]; row["requested_revision"] = request["revision"]
            rows.append(row)
            if "error" in row:
                print(f"{request['kind']} r{request['revision']}: ERROR {row['error']}", flush=True)
            else:
                print(f"{row['kind']} r{row['revision']:.0f}: gross={row['mean_gross_combat_damage_fraction']:.1%}, "
                    f"attempts={row['mean_player_attack_attempts']:.3f}, deaths={row['deaths']}, "
                    f"combat/aftermath censors={row['combat_censored']}/{row['aftermath_censored']}", flush=True)
        returncode = proc.wait()
    pairs = []
    for index, (kind, scenario, depth, role) in enumerate(cases):
        pair = dict(kind=kind, scenario=scenario, reference_depth=depth, role=role)
        before, after = (rows[2*index:2*index+2] + [None, None])[:2]
        if before is None or after is None or "error" in before or "error" in after:
            pair.update(status="incomplete_or_error", before=before, after=after); pairs.append(pair); continue
        assert before["initial_actors"] == after["initial_actors"], f"Enemy baseline changed for {kind}"
        outcomes = list(zip(before["outcomes"], after["outcomes"]))
        assert len(outcomes) == args.trials
        assert all(a["seed"] == b["seed"] and a["profile"] == b["profile"]
            and a["initial_actors"] == b["initial_actors"] for a, b in outcomes), f"Unpaired inputs for {kind}"
        diffs = [b["gross_combat_damage_fraction"] - a["gross_combat_damage_fraction"] for a, b in outcomes]
        mean = sum(diffs) / len(diffs)
        se = math.sqrt(sum((value - mean)**2 for value in diffs) / (len(diffs)*(len(diffs)-1))) if len(diffs) > 1 else None
        pair.update(status="native_paired", trials=args.trials,
            paired_gross_damage_delta=mean, paired_delta_standard_error=se,
            initial_enemy_profiles_identical=True, initial_combat_seeds_identical=True,
            before_gross=before["mean_gross_combat_damage_fraction"], after_gross=after["mean_gross_combat_damage_fraction"],
            before_attempts=before["mean_attacks_to_kill_conditional_on_kill"], after_attempts=after["mean_attacks_to_kill_conditional_on_kill"],
            before_deaths=before["deaths"], after_deaths=after["deaths"],
            before_kills=before["kills"], after_kills=after["kills"],
            before_combat_censors=before["combat_censored"], after_combat_censors=after["combat_censored"],
            before_aftermath_censors=before["aftermath_censored"], after_aftermath_censors=after["aftermath_censored"],
            before_postdeath_gross=before["mean_gross_postdeath_damage_fraction"],
            after_postdeath_gross=after["mean_gross_postdeath_damage_fraction"])
        pairs.append(pair)
    receipt = dict(method="Real native dispatcher, paired bare-player fixed-profile diagnostic; no numeric enemy candidates",
        limitations=["R32 stored joint progression, not regenerated R50 campaign",
            "No optional player items, blessings, amulets or per-ability reserves",
            "Bounded lit arena and deterministic chase/escape policy, not optimal tactics or floor navigation",
            "Conditional kills, combat mortality and censored aftermath remain separate",
            "Three-cast children are setup payload, not twenty simulated summoner turns",
            "Initial seeds match; different attack timing naturally changes subsequent RNG consumption",
            "No enemy retuning follows automatically from these diagnostic means"],
        trials_per_revision_case=args.trials, cases=len(cases), rows_received=len(rows), expected_rows=len(inputs),
        native_process_returncode=returncode, elapsed_seconds=time.monotonic()-started,
        binary=str(args.binary.resolve()), binary_sha256=digest(args.binary.resolve()),
        profile_indices=selected, profile_revision=blob["revision"], sources=sources, pairs=pairs)
    prefix.with_suffix(".json").write_text(json.dumps(receipt, indent=2) + "\n")
    with prefix.with_suffix(".csv").open("w", newline="") as file:
        keys = [key for pair in pairs for key in pair if key not in ("before", "after")]
        writer = csv.DictWriter(file, fieldnames=list(dict.fromkeys(keys)), extrasaction="ignore")
        writer.writeheader(); writer.writerows(pairs)
    if returncode or len(rows) != len(inputs) or any("error" in row for row in rows):
        raise SystemExit("Native diagnostic incomplete; explicit errors retained in artifacts")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Exact body-only attack-count sensitivity, NOT native encounter balance.

Reuses unchanged r46 birth overlays and stored r32 joint player cohorts. Solves
the damage-distribution recurrence for a stationary, non-healing target. Does
not model AI, incoming attacks, death risk, abilities, summons or R50 progression.
No random samples, packages, native binary or game-state mutations are used.
"""
import csv
import hashlib
import json
import math
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CAT = ROOT / "crates/corpse-core/src/catalog"
PROFILE = ROOT / "tools/monster-balance/relocated-player-progressions-r32-joint-profiles.json"
OUTPUT = ROOT / "docs/artifacts/revision50-greatsword-body-sensitivity.json"
KINDS = ["blorb", "rat", "cave-bat", "shellback-beetle", "hemogoblin", "bear",
         "giant-beetle", "tiger", "troll", "dire-bear", "minotaur", "dragon"]


def read(path):
    return json.loads(path.read_text())


def integer_stat(value):
    return math.copysign(math.floor(abs(value) + 0.5), value)


def damage_pmf(base, dr):
    """Exact masses of max(0, ceil((base*Uniform[.75,1.25)-DR)/2))."""
    lo, hi = (base * 0.75 - dr) / 2, (base * 1.25 - dr) / 2
    if hi <= 0:
        return {0: 1.0}
    if hi <= lo:
        return {max(0, math.ceil(lo)): 1.0}
    mass = {0: max(0.0, min(hi, 0) - lo) / (hi - lo)}
    for damage in range(max(1, math.floor(lo)), math.ceil(hi) + 1):
        probability = max(0.0, min(hi, damage) - max(lo, damage - 1)) / (hi - lo)
        if probability:
            mass[damage] = probability
    assert abs(sum(mass.values()) - 1) < 1e-12
    return mass


@lru_cache(maxsize=None)
def landed_hits(hp, base, dr):
    """Solve a finite damage recurrence, including zero-damage landed hits."""
    pmf = damage_pmf(base, dr)
    productive = 1 - pmf.get(0, 0)
    if productive <= 0:
        return math.inf
    expectation = [0.0] * (hp + 1)
    for remaining in range(1, hp + 1):
        expectation[remaining] = (1 + sum(prob * expectation[max(0, remaining - damage)]
            for damage, prob in pmf.items() if damage > 0)) / productive
    return expectation[hp]


def main():
    actors = read(CAT / "population-v48.json")["actors"]
    equipment_path = ROOT / "crates/corpse-core/catalog/equipment-v48.json"
    equipment = read(equipment_path)
    physical31 = {row["kind"]: row for row in read(CAT / "physical-balance-v31.json")}
    physical32 = {row["kind"]: row for row in read(CAT / "physical-balance-v32.json")}
    accuracy33 = {row["kind"]: row for row in read(CAT / "accuracy-dodge-v33.json")}
    final42 = {row["kind"]: row for row in read(CAT / "balance-v42.json")}
    blob = read(PROFILE)
    profiles = blob["profiles"]["all_initial_unblessed"]["entry"]
    rows = []
    for kind in KINDS:
        physical = physical32.get(kind, physical31.get(kind))
        stats = dict(actors[kind]["stats"])
        for key in ["offense", "defense", "accuracy", "dodge", "dr"]:
            stats[key] = physical[key]
        for overlay, keys in [(accuracy33, ["accuracy", "dodge"]), (final42, ["offense", "dodge"])]:
            for key in keys:
                if kind in overlay:
                    stats[key] = overlay[kind][key]
        depth = physical["reference_depth"]
        assert depth == int(depth), "Selected diagnostic references must be whole floors."
        hp = int(100 + 6 * stats["defense"])
        dodge, dr = integer_stat(stats["dodge"]), integer_stat(stats["dr"])
        old_hits, new_hits, old_attempts, new_attempts, offense, old_packet, new_packet = [], [], [], [], [], [], []
        for world in profiles:
            player = world[int(depth) - 1]
            off, accuracy = integer_stat(player[0]), integer_stat(player[2])
            chance = min(1.0, 0.65 * 0.9 ** (dodge - accuracy))
            old_base = equipment["Shortsword"]["damage"] + off * equipment["Shortsword"].get("offenseMultiplier", 1)
            new_base = equipment["Greatsword"]["damage"] + off * equipment["Greatsword"]["offenseMultiplier"]
            before, after = landed_hits(hp, old_base, dr), landed_hits(hp, new_base, dr)
            old_hits.append(before); new_hits.append(after)
            old_attempts.append(before / chance); new_attempts.append(after / chance)
            offense.append(off)
            old_packet.append(sum(d * p for d, p in damage_pmf(old_base, dr).items()))
            new_packet.append(sum(d * p for d, p in damage_pmf(new_base, dr).items()))
        mean = lambda values: sum(values) / len(values)
        rows.append(dict(kind=kind, reference_depth=depth, profiles=len(profiles), enemy_hp=hp,
            enemy_dodge=dodge, enemy_dr=dr, mean_player_offense=mean(offense),
            mean_old_landed_packet=mean(old_packet), mean_new_landed_packet=mean(new_packet),
            mean_old_landed_hits=mean(old_hits), mean_new_landed_hits=mean(new_hits),
            mean_old_attempts=mean(old_attempts), mean_new_attempts=mean(new_attempts),
            body_only_attempt_reduction_fraction=1 - mean(new_attempts) / mean(old_attempts),
            abilities_excluded=actors[kind]["abilities"]))
    files = [PROFILE, Path(__file__), equipment_path,
        ROOT / "crates/corpse-core/src/revision50_player_attack.rs", *(CAT / name for name in ["population-v48.json", "physical-balance-v31.json",
        "physical-balance-v32.json", "accuracy-dodge-v33.json", "balance-v42.json"])]
    result = dict(method="Exact body-only recurrence, not a native encounter simulation",
        limitations=["Stored r32 all-initial Unblessed joint cohorts, not regenerated R50 progression",
            "No enemy healing, AI, incoming damage, player deaths, statuses, blessings, terrain or summons",
            "No enemy retuning inferred from this diagnostic; use paired native encounters before numeric changes"],
        profile_revision=blob["revision"], track="all_initial_unblessed", profiles=len(profiles),
        source_sha256={str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in files},
        rows=rows)
    OUTPUT.write_text(json.dumps(result, indent=2) + "\n")
    with OUTPUT.with_suffix(".csv").open("w", newline="") as out:
        writer = csv.DictWriter(out, fieldnames=[key for key in rows[0] if key != "abilities_excluded"], extrasaction="ignore")
        writer.writeheader(); writer.writerows(rows)
    for row in rows:
        print(f"{row['kind']}: attempts {row['mean_old_attempts']:.3f} -> {row['mean_new_attempts']:.3f}, "
              f"body-only reduction {row['body_only_attempt_reduction_fraction']:.1%}")


if __name__ == "__main__":
    assert landed_hits(22, 30, 0) == 2.0
    assert abs(landed_hits(22, 45, 0) - (1 + 11 / 30)) < 1e-12
    assert landed_hits(10, 30, 0) == 1.0
    main()

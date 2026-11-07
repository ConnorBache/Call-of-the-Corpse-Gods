#!/usr/bin/env python3
"""Run only after every code owner has confirmed a source freeze."""
import datetime, hashlib, json, os, pathlib, re, subprocess, time

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "verification/integration-r36/final-gate"
OUT.mkdir(parents=True, exist_ok=True)

def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def snapshot():
    paths = []
    for name in ["crates", "fixtures", "tools"]:
        paths.extend(p for p in (ROOT/name).rglob("*") if p.is_file() and "__pycache__" not in p.parts)
    paths.extend(ROOT/n for n in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "AGENTS.md"])
    return {str(p.relative_to(ROOT)): digest(p) for p in sorted(set(paths))}
def fingerprint(rows):
    return hashlib.sha256(json.dumps(rows, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
def write(name, data): (OUT/name).write_text(json.dumps(data, indent=2)+"\n")

env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_PROFILE_TEST_OPT_LEVEL="2",
    CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_TEST_DEBUG_ASSERTIONS="true",
    CARGO_PROFILE_TEST_OVERFLOW_CHECKS="true",
    CORPSE_NON_EUCLIDEAN_UI_REVIEW=str(ROOT/"verification/integration-r36/desktop-ui"),
    CORPSE_NON_EUCLIDEAN_GENERATED_REVIEW=str(ROOT/"verification/integration-r36/desktop-ui"))
before = snapshot()
write("code-freeze.json", before)
receipt = {
    "status":"running", "simulation_revision":36, "started_utc":now(),
    "scope":"All files in crates, fixtures and tools except __pycache__, plus Cargo.toml, Cargo.lock, rust-toolchain.toml and AGENTS.md. Generated verification/release documentation is excluded.",
    "frozen_input_count":len(before), "input_fingerprint_before":fingerprint(before),
    "profile":{k:env[k] for k in ["CARGO_INCREMENTAL","CARGO_PROFILE_TEST_OPT_LEVEL","CARGO_PROFILE_TEST_DEBUG","CARGO_PROFILE_TEST_DEBUG_ASSERTIONS","CARGO_PROFILE_TEST_OVERFLOW_CHECKS"]},
    "cargo_version":subprocess.check_output(["sh","tools/cargo.sh","--version"],cwd=ROOT,env=env,text=True).strip(),
    "rust_toolchain":(ROOT/"rust-toolchain.toml").read_text().strip(), "checks":[]}
write("final-gate-receipt.json",receipt)
pattern = re.compile(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out",re.M)
for label, extra in [("workspace",["--all-targets","--no-fail-fast"]),("doctests",["--doc","--no-fail-fast"])]:
    command = ["sh","tools/cargo.sh","test","--workspace","--locked"]+extra
    start = time.monotonic(); path = OUT/f"{label}.log"
    with path.open("w") as log:
        code = subprocess.call(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    content = path.read_text(); summaries = [tuple(map(int,x)) for x in pattern.findall(content)]
    row = {"label":label,"command":command,"exit_code":code,"seconds":round(time.monotonic()-start,3),
        "targets":len(summaries),"log":str(path.relative_to(ROOT)),"log_sha256":digest(path),
        "count_method":"Sum each target's test result summary once; never count individual test lines additionally.",
        "failed_tests":re.findall(r"^test (.+) \.\.\. FAILED$",content,re.M)}
    for index,key in enumerate(["passed","failed","ignored","measured","filtered_out"]):
        row[key] = sum(s[index] for s in summaries)
    receipt["checks"].append(row);write("final-gate-receipt.json",receipt);print(json.dumps(row),flush=True)
after = snapshot();write("code-freeze-after.json",after)
receipt.update(finished_utc=now(),input_fingerprint_after=fingerprint(after),
    changed_inputs=[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)],
    code_build_inputs_unchanged=before==after)
passed = before==after and all(c["exit_code"]==0 and c["failed"]==0 for c in receipt["checks"])
receipt["status"] = "passed" if passed else "failed"
write("final-gate-receipt.json",receipt);print(json.dumps({k:receipt[k] for k in ["status","finished_utc","changed_inputs"]}),flush=True)
raise SystemExit(0 if passed else 1)

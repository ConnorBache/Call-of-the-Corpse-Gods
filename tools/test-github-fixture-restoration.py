#!/usr/bin/env python3
"""Exercise import helper independently of game compilation; requires Rust/Cargo."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
NAME = 'mini-me-starter-combinations-v48.json'
EXPECTED = '4dee02f538eabe5b9023c1be910ec3655b2ee80026c84062f1b39aade94eaa31'
PART_HASHES = ['f4c34deb334417cf7138e634777825707974cdf7b1e5654d3769027f216fb9a8', 'a1504869f3c14a1db00ff11207dc82df07d088f4601e0162f1eb0d5a4951ca1a']

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

rustc = shutil.which('rustc')
cargo = shutil.which('cargo')
if not rustc or not cargo:
    raise SystemExit('Rust/Cargo unavailable: helper tests were not run. Use the pinned 1.99.1 toolchain.')
version = subprocess.check_output([rustc, '--version'], text=True).strip()
if not version.startswith('rustc 1.99.1 '):
    raise SystemExit(f'Pinned Rust 1.99.1 required; found {version}')
print(version, flush=True)
with tempfile.TemporaryDirectory(prefix='fixture-import-test-') as temp:
    root = Path(temp)
    unit = root / ('helper-unit-tests.exe' if os.name == 'nt' else 'helper-unit-tests')
    subprocess.run([rustc, '--edition=2024', '--test', str(ROOT / 'crates/corpse-core/build.rs'), '-o', str(unit)], check=True)
    subprocess.run([str(unit)], env=os.environ | {'TEST_TMPDIR': str(root)}, check=True)
    crate = root / 'crates/core'
    (crate / 'src').mkdir(parents=True)
    (root / 'fixtures').mkdir()
    shutil.copyfile(ROOT / 'crates/corpse-core/build.rs', crate / 'build.rs')
    for i, expected in enumerate(PART_HASHES, 1):
        part = ROOT / 'fixtures' / (NAME + f'.part{i}')
        assert digest(part) == expected
        shutil.copyfile(part, root / 'fixtures' / part.name)
    (crate / 'Cargo.toml').write_text('[package]\nname="fixture-import-harness"\nversion="0.0.0"\nedition="2024"\n')
    (crate / 'src/lib.rs').write_text('''#[test]
fn fixture_present() {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/mini-me-starter-combinations-v48.json");
    assert_eq!(std::fs::metadata(path).unwrap().len(),116885985);
}
''')
    command = [cargo, 'test', '--offline', '--manifest-path', str(crate / 'Cargo.toml')]
    output = root / 'fixtures' / NAME
    subprocess.run(command, check=True)
    assert digest(output) == EXPECTED
    modified = output.stat().st_mtime_ns
    subprocess.run(command, check=True)
    assert output.stat().st_mtime_ns == modified
    output.unlink()
    subprocess.run(command, check=True)
    assert digest(output) == EXPECTED
    with output.open('r+b') as stream:
        stream.write(b'!')
    assert subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE).returncode != 0
    with output.open('rb') as stream:
        assert stream.read(1) == b'!'
print('Import helper tests passed: 4 Rust unit tests, clean/idempotent/deleted fixture Cargo restoration, SHA-256 preservation, and local-change refusal.')

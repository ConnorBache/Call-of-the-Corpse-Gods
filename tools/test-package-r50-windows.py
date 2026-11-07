#!/usr/bin/env python3
"""Packaging-only tests with synthetic fixtures, no Cargo or game artifact."""
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location('package_r50', Path(__file__).with_name('package-r50-windows.py'))
pkg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pkg)


def fake_pe(raw=b'CODE', *, machine=0x8664, rva=0x1000, signed=False, delayed=False, debug=False):
    data = bytearray(1536)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 60, 128)
    data[128:132] = b'PE\0\0'
    struct.pack_into('<HH', data, 132, machine, 2 if debug else 1)
    struct.pack_into('<H', data, 148, 240)
    op = 152
    struct.pack_into('<H', data, op, 0x20b)
    struct.pack_into('<I', data, op + 16, 0x1000)
    struct.pack_into('<Q', data, op + 24, 0x140000000)
    struct.pack_into('<I', data, op + 32, 4096)
    struct.pack_into('<I', data, op + 56, 0x3000 if debug else 0x2000)
    struct.pack_into('<H', data, op + 68, 3)
    struct.pack_into('<I', data, op + 108, 16)
    if signed:
        struct.pack_into('<II', data, op + 112 + 4 * 8, 1100, 100)
    if delayed:
        struct.pack_into('<II', data, op + 112 + 13 * 8, 4096, 20)
    section = op + 240
    data[section:section + 8] = b'.text\0\0\0'
    struct.pack_into('<IIII', data, section + 8, len(raw), rva, len(raw), 512)
    struct.pack_into('<I', data, section + 36, 0x60000020)
    data[512:512 + len(raw)] = raw
    if debug:
        section += 40
        data[section:section + 8] = b'.debug\0\0'
        struct.pack_into('<IIII', data, section + 8, 4, 0x2000, 4, 1024)
        struct.pack_into('<I', data, section + 36, 0x42000040)
        data[1024:1028] = b'DBUG'
    return bytes(data)


class PackagingTests(unittest.TestCase):
    def test_pe_rejects_non_x64_truncated_signed_and_delay_imports(self):
        for value in (b'no', fake_pe()[:300], fake_pe(machine=0x14c),
                      fake_pe(signed=True), fake_pe(delayed=True)):
            with self.subTest(size=len(value)), self.assertRaises(ValueError):
                pkg.pe_sections(value)

    def test_pe_compares_bytes_layout_and_critical_headers(self):
        baseline = pkg.pe_sections(fake_pe())
        self.assertNotEqual(baseline, pkg.pe_sections(fake_pe(raw=b'DIFF')))
        self.assertNotEqual(baseline, pkg.pe_sections(fake_pe(rva=0x1100)))
        header_changed = bytearray(fake_pe())
        struct.pack_into('<I', header_changed, 152 + 16, 0x1010)
        self.assertNotEqual(baseline, pkg.pe_sections(header_changed))

    def test_removing_only_debug_sections_and_trailing_image_space_is_allowed(self):
        self.assertEqual(pkg.pe_sections(fake_pe(debug=True)), pkg.pe_sections(fake_pe()))

    def test_import_allowlist_checks_both_executable_and_dll(self):
        with patch.object(pkg.subprocess, 'check_output', return_value='DLL Name: KERNEL32.dll\nDLL Name: SDL2.dll\n'):
            self.assertEqual(pkg.imports(Path('game'), Path('objdump'), True), ['KERNEL32.dll', 'SDL2.dll'])
            with self.assertRaises(ValueError):
                pkg.imports(Path('sdl'), Path('objdump'), False)
        with patch.object(pkg.subprocess, 'check_output', return_value='DLL Name: SDL2.dll\nDLL Name: libgcc_s_seh-1.dll\n'):
            with self.assertRaises(ValueError):
                pkg.imports(Path('game'), Path('objdump'), True)
        with patch.object(pkg.subprocess, 'check_output', return_value='DLL Name: KERNEL32.dll\n'):
            with self.assertRaises(ValueError):
                pkg.imports(Path('game'), Path('objdump'), True)

    def test_counts_come_from_successful_actual_summary_lines(self):
        data = b'test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out\n' \
               b'test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out\n'
        self.assertEqual(pkg.test_counts(data), {'completed_targets': 2, 'passed': 10, 'failed': 0, 'ignored': 3})
        for data in (b'', b'test result: ok. 0 passed; 0 failed; 1 ignored;',
                     b'test result: FAILED. 3 passed; 1 failed; 0 ignored;'):
            with self.assertRaises(ValueError):
                pkg.test_counts(data)

    def test_source_snapshot_is_reproducible_and_rejects_stale_sources(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in pkg.SOURCE_FILES:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            for name in pkg.SOURCE_DIRS:
                (root / name).mkdir(exist_ok=True)
            (root / 'crates/code.rs').write_text('source')
            with patch.object(pkg, 'ROOT', root):
                snapshot = pkg.json_bytes(pkg.source_snapshot())
                self.assertEqual(snapshot, pkg.json_bytes(pkg.source_snapshot()))
                path = root / 'snapshot.json'
                path.write_bytes(snapshot)
                self.assertEqual(pkg.current_snapshot(path)[1], pkg.digest(snapshot))
                (root / 'crates/code.rs').write_text('changed')
                with self.assertRaises(ValueError):
                    pkg.current_snapshot(path)

    def test_evidence_revalidates_log_counts_and_exit_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            log = root / 'tests.log'
            log.write_bytes(b'test result: ok. 2 passed; 0 failed; 1 ignored;\n')
            exit_file = root / 'tests.exit'
            exit_file.write_text('0\n')
            report = {'schema': 1, 'revision': 50, 'kind': 'r50-test-report',
                      'source_snapshot_sha256': 'frozen', 'exit_code': 0,
                      'command': 'sh tools/cargo.sh test --workspace',
                      'windows_runtime_verified': False, 'counts': pkg.test_counts(log.read_bytes()),
                      'log': pkg.logged_file(log)[0], 'exit_code_file': pkg.logged_file(exit_file)[0]}
            path = root / 'test-report.json'
            path.write_bytes(pkg.json_bytes(report))
            pkg.validate_evidence(path, 'r50-test-report', 'frozen')
            with self.assertRaises(ValueError):
                pkg.validate_evidence(path, 'r50-test-report', 'changed')
            report['counts']['passed'] = 999
            path.write_bytes(pkg.json_bytes(report))
            with self.assertRaises(ValueError):
                pkg.validate_evidence(path, 'r50-test-report', 'frozen')
            log.write_text('changed')
            with self.assertRaises(ValueError):
                pkg.validate_evidence(path, 'r50-test-report', 'frozen')

    def test_zip_is_deterministic_verified_and_exclusive(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp) / 'a.zip', Path(tmp) / 'b.zip'
            files = {'START-HERE.txt': b'synthetic test', 'notices/license.txt': b'notice'}
            pkg.write_zip(a, files)
            pkg.write_zip(b, dict(reversed(list(files.items()))))
            self.assertEqual(a.read_bytes(), b.read_bytes())
            with zipfile.ZipFile(a) as archive:
                self.assertEqual(archive.namelist(), sorted(files))
                for info in archive.infolist():
                    self.assertEqual(info.date_time, (1980, 1, 1, 0, 0, 0))
                    self.assertEqual(info.external_attr, 0o100644 << 16)
            with self.assertRaises(FileExistsError):
                pkg.write_zip(a, files)
            for name in ('../escape', '/absolute', 'folder\\escape'):
                with self.assertRaises(ValueError):
                    pkg.write_zip(Path(tmp) / 'bad.zip', {name: b'bad'})

    def test_player_readme_does_not_claim_windows_execution(self):
        self.assertIn('NOT been runtime-tested', pkg.START_HERE)
        self.assertIn('graphical-window game', pkg.START_HERE)
        self.assertIn('Repulsion', pkg.START_HERE)
        self.assertIn('Dire Bear uses a black B', pkg.START_HERE)
        self.assertIn('extract BOTH into that same folder', pkg.START_HERE)
        self.assertIn('Carried rings and amulets are automatically active', pkg.START_HERE)
        self.assertNotIn('select/equip loot', pkg.START_HERE)
        self.assertNotIn('r39-compact', pkg.START_HERE)


if __name__ == '__main__':
    unittest.main()

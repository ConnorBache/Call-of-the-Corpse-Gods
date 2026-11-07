#!/usr/bin/env python3
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('split_delivery', Path(__file__).with_name('split-game-delivery.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class DeliveryTests(unittest.TestCase):
    def archive(self, root):
        files = {m.EXE: b'MZ' + bytes(range(256))*128, m.DLL: b'DLL'*500,
                 'START-HERE.txt': b'Extract both archives into the same folder.',
                 'notices/test.txt': b'Preserve this notice.'}
        manifest = {'files': [{'path': n, 'bytes': len(b), 'sha256': m.digest(b)} for n, b in files.items()]}
        files['MANIFEST.json'] = json.dumps(manifest).encode()
        p = root / 'game.zip'
        m.write_zip(p, files)
        return p, files

    def test_exact_complete_assembly_and_two_standard_archives(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d); archive, original = self.archive(root)
            before = archive.read_bytes(); out = root/'parts'
            receipt = m.split(archive, out)
            self.assertTrue(receipt['exact_original_file_tree_verified'])
            self.assertEqual(archive.read_bytes(), before)
            self.assertEqual(receipt['original_member_count'], len(original))
            self.assertEqual([r['members'] for r in receipt['parts']], [1, len(original)-1])
            for row in receipt['parts']:
                self.assertLessEqual(row['bytes'], m.LIMIT)
                self.assertTrue(zipfile.is_zipfile(out/row['name']))

    def test_limit_failure_publishes_no_partial_pair(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); archive,_=self.archive(root);out=root/'parts'
            with self.assertRaisesRegex(ValueError, 'exceeds attachment limit'):
                m.split(archive, out, limit=10)
            self.assertFalse(out.exists())

    def test_mismatched_manifest_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); archive,files=self.archive(root)
            files[m.EXE]=b'changed'; bad=root/'bad.zip';m.write_zip(bad,files)
            with self.assertRaisesRegex(ValueError,'differs from package manifest'):
                m.split(bad,root/'parts')

    def test_existing_destination_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);archive,_=self.archive(root);out=root/'parts';out.mkdir()
            with self.assertRaisesRegex(ValueError,'must not exist'):
                m.split(archive,out)

if __name__=='__main__': unittest.main()

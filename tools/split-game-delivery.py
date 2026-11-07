#!/usr/bin/env python3
"""Split a verified Windows game ZIP into two standard ZIPs, never changing files.

Both archives are extracted into the SAME folder with Windows Extract All.
This is file-tree assembly, not a multi-volume ZIP or concatenation format.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import stat
import tempfile
import zipfile

LIMIT = 20_971_520
EXE = 'corpse-desktop.exe'
DLL = 'SDL2.dll'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def read_archive(path):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(set(names)) != len(names) or archive.testzip() is not None:
            raise ValueError('Duplicate member or corrupt input archive')
        for item in archive.infolist():
            p = PurePosixPath(item.filename)
            if (p.is_absolute() or '..' in p.parts or str(p) != item.filename
                or '\\' in item.filename or ':' in item.filename
                or item.is_dir() or stat.S_ISLNK(item.external_attr >> 16)):
                raise ValueError('Unsafe or unsupported ZIP member')
        files = {name: archive.read(name) for name in names}
    if not all(name in files for name in [EXE, DLL, 'START-HERE.txt', 'MANIFEST.json']):
        raise ValueError('Expected self-contained game package members are missing')
    manifest = json.loads(files['MANIFEST.json'])
    indexed = {r['path']: r for r in manifest['files']}
    if set(indexed) != set(files) - {'MANIFEST.json'}:
        raise ValueError('Input package manifest does not cover exact member set')
    for name, row in indexed.items():
        if row['bytes'] != len(files[name]) or row['sha256'] != digest(files[name]):
            raise ValueError('Input member differs from package manifest: ' + name)
    return files

def write_zip(path, files):
    with zipfile.ZipFile(path, 'x', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data, compresslevel=9)
    with zipfile.ZipFile(path) as archive:
        if archive.testzip() is not None or set(archive.namelist()) != set(files):
            raise ValueError('Output ZIP integrity/member verification failed')
        for name, data in files.items():
            if archive.read(name) != data:
                raise ValueError('Output ZIP changed a file: ' + name)

def split(archive, output_dir, limit=LIMIT):
    if output_dir.exists():
        raise ValueError('Output directory must not exist; preserve earlier delivery artifacts')
    before = digest(archive.read_bytes())
    files = read_archive(archive)
    groups = [('R50-game-part-1.zip', {EXE: files[EXE]}),
              ('R50-required-files-part-2.zip', {n: b for n, b in files.items() if n != EXE})]
    # Build/validate everything privately before publishing a partial pair.
    with tempfile.TemporaryDirectory(prefix='r50-delivery-') as temporary:
        temporary = Path(temporary)
        rows = []
        for name, content in groups:
            path = temporary / name
            write_zip(path, content)
            if path.stat().st_size > limit:
                raise ValueError(f'{name} exceeds attachment limit; no delivery pair published')
            rows.append({'name': name, 'bytes': path.stat().st_size,
                         'sha256': digest(path.read_bytes()), 'members': len(content)})
        assembled = temporary / 'assembled'
        assembled.mkdir()
        for name, _ in groups:
            with zipfile.ZipFile(temporary / name) as z:
                z.extractall(assembled)  # Paths were validated against original archive.
        actual = {p.relative_to(assembled).as_posix(): p.read_bytes()
                  for p in assembled.rglob('*') if p.is_file()}
        if actual != files or digest(archive.read_bytes()) != before:
            raise ValueError('Exact assembled file tree or original ZIP identity changed')
        receipt = {'original_zip': archive.name, 'original_zip_sha256': before,
                   'limit_bytes_per_attachment': limit, 'parts': rows,
                   'exact_original_file_tree_verified': True,
                   'original_member_count': len(files), 'binary_bytes_unchanged': True,
                   'format': 'Two independent Windows-compatible ZIPs; extract both into the same folder',
                   'windows_runtime_verified': False}
        output_dir.mkdir(parents=True)
        for name, _ in groups:
            (output_dir / name).write_bytes((temporary / name).read_bytes())
        (output_dir / 'DELIVERY-VERIFICATION.json').write_text(json.dumps(receipt, indent=2) + '\n')
        return receipt

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--limit-bytes', type=int, default=LIMIT)
    args = parser.parse_args()
    print(json.dumps(split(args.archive, args.output_dir, args.limit_bytes), indent=2))

if __name__ == '__main__':
    main()

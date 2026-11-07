#!/usr/bin/env python3
"""Package r39-compact without rebuilding or modifying the input binary.

Use after the frozen build and art-quality validation, e.g.:
  python tools/package-compact-windows.py --binary target/x86_64-pc-windows-gnu/release/corpse-desktop.exe --art-manifest <verified.json> --output <play.zip>
The SDK must be restored using docs/windows-cross-build.md. ZIP entries use fixed
metadata and sorted paths; the archive contains only runtime files and notices.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SYSTEM_DLLS = {
    'bcryptprimitives.dll', 'api-ms-win-core-synch-l1-2-0.dll', 'kernel32.dll',
    'msvcrt.dll', 'ntdll.dll', 'advapi32.dll', 'gdi32.dll', 'imm32.dll',
    'ole32.dll', 'oleaut32.dll', 'setupapi.dll', 'shell32.dll', 'user32.dll',
    'version.dll', 'winmm.dll',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pe_sections(data):
    """Linear in PE section count plus bytes copied; rejects non-x64 PE files."""
    if data[:2] != b'MZ':
        raise ValueError('Expected Windows executable')
    pe = struct.unpack_from('<I', data, 60)[0]
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Missing PE signature')
    machine, count = struct.unpack_from('<HH', data, pe + 4)
    optional = struct.unpack_from('<H', data, pe + 20)[0]
    if machine != 0x8664 or struct.unpack_from('<H', data, pe + 24)[0] != 0x20b:
        raise ValueError('Expected PE32+ x86-64')
    # Security directory is a file offset, not an RVA; reject signed inputs.
    cert_offset, cert_size = struct.unpack_from('<II', data, pe + 24 + 112 + 4 * 8)
    if cert_offset or cert_size:
        raise ValueError('Signed input: stripping would invalidate its signature')
    symbols, symbol_count = struct.unpack_from('<II', data, pe + 12)
    string_table = symbols + symbol_count * 18 if symbols else 0
    sections = {}
    for index in range(count):
        offset = pe + 24 + optional + index * 40
        name = data[offset:offset + 8].rstrip(b'\0').decode('ascii')
        if name.startswith('/') and name[1:].isdigit() and string_table:
            begin = string_table + int(name[1:])
            end = data.find(b'\0', begin)
            if end < begin:
                raise ValueError('Malformed COFF section name')
            name = data[begin:end].decode('ascii')
        size, start = struct.unpack_from('<II', data, offset + 16)
        if start and start + size > len(data):
            raise ValueError('Truncated PE section')
        characteristics = struct.unpack_from('<I', data, offset + 36)[0]
        # Only debug/non-loadable sections may disappear. All mapped code, data,
        # resources, relocations and unwind information must remain byte-identical.
        if characteristics & 0xE0000000 and not name.startswith(('.debug', '.zdebug')):
            sections[name] = data[start:start + size] if start else b''
    return sections


def imports(path, objdump):
    text = subprocess.check_output([str(objdump), '-p', str(path)], text=True)
    result = sorted(set(re.findall(r'DLL Name:\s*(\S+)', text)), key=str.lower)
    if not result:
        raise ValueError(f'No import dependencies found: {path}')
    unknown = [n for n in result if n.lower() not in SYSTEM_DLLS | {'sdl2.dll'}]
    if unknown:
        raise ValueError(f'Unbundled/non-allowlisted imports in {path}: {unknown}')
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--art-manifest', type=Path,
                   default=ROOT / 'crates/corpse-desktop/assets/creature-runtime-manifest.json',
                   help='Validated quality/provenance manifest for the exact embedded art')
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--label', choices=['r39-compact', 'r39-compact-small'], default='r39-compact')
    p.add_argument('--build-profile-json', type=Path, help='Exact external build-profile provenance')
    p.add_argument('--sdk', type=Path, default=ROOT.parent / 'native-windows-sdk')
    p.add_argument('--strip-tool', type=Path)
    p.add_argument('--objdump', type=Path)
    a = p.parse_args()
    strip = (a.strip_tool or a.sdk / 'sysroot/usr/bin/x86_64-w64-mingw32-strip').resolve()
    objdump = (a.objdump or a.sdk / 'sysroot/usr/bin/x86_64-w64-mingw32-objdump').resolve()
    sdl = a.sdk / 'SDL2-2.32.10'
    if a.output.exists():
        raise ValueError('Refusing to overwrite an existing output archive')
    art_bytes = a.art_manifest.read_bytes()
    json.loads(art_bytes)  # Preserve supplied verification evidence unchanged.
    profile = json.loads(a.build_profile_json.read_text()) if a.build_profile_json else None
    original = a.binary.read_bytes()
    original_sections = pe_sections(original)
    strip_version = subprocess.check_output([str(strip), '--version'], text=True).splitlines()[0]
    objdump_version = subprocess.check_output([str(objdump), '--version'], text=True).splitlines()[0]
    with tempfile.TemporaryDirectory(prefix='corpse-r39-package-') as temporary:
        staged = Path(temporary) / 'corpse-desktop.exe'
        shutil.copyfile(a.binary, staged)
        subprocess.run([str(strip), '--strip-all', str(staged)], check=True)
        stripped = staged.read_bytes()
        stripped_sections = pe_sections(stripped)
        if original_sections != stripped_sections:
            raise ValueError('Stripping changed PE sections; refuse distribution')
        exe_imports = imports(staged, objdump)
        if 'sdl2.dll' not in {n.lower() for n in exe_imports}:
            raise ValueError('Expected SDL2 dependency is missing')
        dll = sdl / 'x86_64-w64-mingw32/bin/SDL2.dll'
        pe_sections(dll.read_bytes())
        dll_imports = imports(dll, objdump)
        if 'sdl2.dll' in {n.lower() for n in dll_imports}:
            raise ValueError('Unexpected SDL2 self-dependency')
    if digest(a.binary.read_bytes()) != digest(original):
        raise ValueError('Original build binary changed during packaging')
    assets = ROOT / 'crates/corpse-desktop/assets'
    files = {'corpse-desktop.exe': stripped, 'SDL2.dll': dll.read_bytes(),
             'notices/SDL2-LICENSE.txt': (sdl / 'LICENSE.txt').read_bytes(),
             'notices/compact-art-manifest.json': art_bytes}
    ui_manifest = assets / 'ui-runtime-manifest.json'
    if ui_manifest.exists():
        files['notices/ui-runtime-manifest.json'] = ui_manifest.read_bytes()
    for src in sorted(assets.glob('*OFL.txt')):
        files['notices/' + src.name] = src.read_bytes()
    for src, dest in [('NOTICE.md', 'assets-NOTICE.md'),
                      ('art-distribution.json', 'legacy-art-distribution.json'),
                      ('creature-alpha-manifest.json', 'creature-alpha-manifest.json')]:
        files['notices/' + dest] = (assets / src).read_bytes()
    for source, destination in [(assets / 'brogue', 'notices/brogue/'),
                                 (ROOT / 'third-party-notices', 'notices/crates/')]:
        for path in sorted(source.rglob('*')):
            if path.is_file():
                files[destination + path.relative_to(source).as_posix()] = path.read_bytes()
    notice_manifest = json.loads((ROOT / 'third-party-notices/manifest.json').read_text())
    for dependency in notice_manifest:
        for notice in dependency.get('notices', []):
            if 'notices/crates/' + notice not in files:
                raise ValueError(f'Missing dependency notice: {notice}')
    files['START-HERE.txt'] = ("""THE CALL OF THE CORPSE GODS - r39-compact

Extract this entire ZIP into a folder, then double-click corpse-desktop.exe.
Keep SDL2.dll beside it. Do not launch from inside the ZIP. No installer,
compiler, SDK, or separate artwork download is required.

This is revision 39 with compact distribution artwork, not new game mechanics.
The approved high-quality RGB compression preserves original image dimensions
and exact alpha transparency. RGB is not pixel-identical to the original art.
Original masters and verification history remain in the separate source package.
The notices folder includes dependency licenses and artwork provenance.

Windows x64. Unsigned cross-build, statically inspected but NOT runtime-tested
on Windows. Launch, input, rendering, and save behavior on Windows are unverified.
""").encode('utf-8')
    files['START-HERE.txt'] = files['START-HERE.txt'].replace(b'r39-compact', a.label.encode())
    manifest = {
        'label': a.label, 'build_profile': profile, 'simulation_revision': 39,
        'windows_runtime_verified': False, 'unsigned': True,
        'art_manifest_sha256': digest(art_bytes),
        'original_binary': {'bytes': len(original), 'sha256': digest(original)},
        'stripped_binary': {'bytes': len(stripped), 'sha256': digest(stripped)},
        'strip': {'tool': strip.name, 'version': strip_version,
                  'tool_sha256': digest(strip.read_bytes()), 'arguments': ['--strip-all'],
                  'all_loadable_pe_section_bytes_unchanged': True},
        'pe_inspection': {'tool': objdump.name, 'version': objdump_version,
                          'exe_imports': exe_imports, 'sdl2_imports': dll_imports},
        'files': [{'path': n, 'bytes': len(b), 'sha256': digest(b)} for n, b in sorted(files.items())],
    }
    files['MANIFEST.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    a.output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(a.output, 'x', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data, compresslevel=9)
    with zipfile.ZipFile(a.output) as z:
        if z.testzip() is not None or set(z.namelist()) != set(files):
            raise ValueError('ZIP integrity or member-set failure')
        for name, data in files.items():
            if z.read(name) != data:
                raise ValueError(f'ZIP byte verification failure: {name}')
    receipt = {'path': str(a.output), 'bytes': a.output.stat().st_size,
               'sha256': digest(a.output.read_bytes()), 'archive_verified': True,
               'windows_runtime_verified': False}
    a.output.with_suffix('.verification.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()

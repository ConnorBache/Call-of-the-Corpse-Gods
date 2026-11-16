#!/usr/bin/env python3
"""Prepare evidence and package R52 Windows x64 without building or running the game.

See docs/revision50-windows-packaging.md. Commands: snapshot, record-test,
record-build, package. Packaging requires current, successful evidence; none of
these commands runs Cargo, installs an SDK, or claims Windows runtime testing.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import tarfile
import tomllib
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = 'x86_64-pc-windows-gnu'
BINARY = ROOT / 'target' / TARGET / 'release/corpse-desktop.exe'
SOURCE_DIRS = ('crates', 'fixtures', 'tools', 'docs', 'third-party-notices')
SOURCE_FILES = ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'AGENTS.md', 'BUILD-FROM-SOURCE.txt',
                'docs/current-design-amendments.md', 'docs/complexity-ledger.md',
                'docs/artifacts/physical-balance-v31-native-smoke-profiles.json',
                'docs/revision50-request.txt', 'docs/quiet-ui-r51.md', 'docs/revision52-release-status.md',
                'docs/revision52-admission-and-settings.md', 'docs/revision52-darkness.md',
                'docs/revision52-inventory-animation-cues.md', 'docs/revision52-fighter-balance.md',
                'docs/revision52-action-animation-contract.md', 'docs/animation-r52-coverage.csv',
                'docs/revision52-enemy-animation-matrix.md', 'docs/revision52-player-animation-coverage.md',
                'docs/revision52-player-animation-complexity.md', 'docs/revision52-targeting-sparkle.md',
                'docs/revision52-enemy-followups.md', 'docs/revision52-amulet-tiers.md',
                'docs/revision52-amulet-tier-ui.md', 'docs/revision52-all-role-balance.md',
                'docs/revision52-bestiary-stats.md', 'docs/revision52-floods.md',
                'docs/r52-floods-native-ui-verification.md',
                'docs/revision52-weave-consumption.md', 'docs/revision52-charge-collapse.md',
                'docs/r52-ground-markers.md', 'docs/r52-negative-illumination.md',
                'docs/revision52-accuracy-formula.md',
                'docs/revision52-stat-followups.md', 'docs/revision52-reserves-wait.md',
                'docs/revision52-hanged.md', 'docs/revision52-source-reconstruction.md',
                'docs/revision52-floods-reconstruction.md', 'docs/revision52-desktop-source-reconstruction.md',
                'docs/revision52-brogue-animation-style.md', 'docs/r52-chasm-color-correction.md',
                'docs/revision52-glyph-mask-coverage.md', 'docs/bestiary-continuous-colors.md',
                'docs/bestiary-portrait-mask-repairs-20261108.md')
SYSTEM_DLLS = {
    'bcryptprimitives.dll', 'api-ms-win-core-synch-l1-2-0.dll', 'kernel32.dll',
    'msvcrt.dll', 'ntdll.dll', 'advapi32.dll', 'gdi32.dll', 'imm32.dll',
    'ole32.dll', 'oleaut32.dll', 'setupapi.dll', 'shell32.dll', 'user32.dll',
    'version.dll', 'winmm.dll',
}
TEST_RESULT = re.compile(
    r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', re.M)
ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def read_json(path):
    return json.loads(path.read_bytes())


def write_new(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as out:
        out.write(data)


def source_snapshot():
    """O(B + N log N) hashing/sorting; no source bytes retained between files."""
    paths = [ROOT / name for name in SOURCE_FILES]
    for name in SOURCE_DIRS:
        paths += [p for p in (ROOT / name).rglob('*') if p.is_file()
                  and '__pycache__' not in p.parts and p.suffix != '.pyc']
    rows = []
    for path in sorted(set(paths)):
        if path.is_symlink():
            raise ValueError(f'Source symlink is not supported: {path}')
        with path.open('rb') as source:
            sha = hashlib.file_digest(source, 'sha256').hexdigest()
        rows.append({'path': path.relative_to(ROOT).as_posix(),
                     'bytes': path.stat().st_size, 'sha256': sha})
    return {'schema': 1, 'revision': 52, 'kind': 'r52-source-snapshot',
            'scope': list(SOURCE_FILES) + [name + '/ (excluding Python cache)' for name in SOURCE_DIRS],
            'files': rows}


def current_snapshot(path):
    data = path.read_bytes()
    if read_json(path) != source_snapshot():
        raise ValueError('Source differs from the frozen snapshot; rerun affected tests/build on final source')
    return data, digest(data)


def evidence_path(path):
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path.resolve())


def logged_file(path):
    data = path.read_bytes()
    return {'path': evidence_path(path), 'bytes': len(data), 'sha256': digest(data)}, data


def verified_log(row):
    path = Path(row['path'])
    if not path.is_absolute():
        path = ROOT / path
    data = path.read_bytes()
    if row['bytes'] != len(data) or row['sha256'] != digest(data):
        raise ValueError(f'Evidence file changed: {path}')
    return data


def test_counts(data):
    text = ANSI.sub('', data.decode('utf-8', errors='replace'))
    rows = TEST_RESULT.findall(text)
    if not rows or any(status != 'ok' or int(failed) for status, _, failed, _ in rows):
        raise ValueError('Test log must contain completed Rust test summaries and no failed targets')
    result = {'completed_targets': len(rows), 'passed': sum(int(r[1]) for r in rows),
              'failed': sum(int(r[2]) for r in rows), 'ignored': sum(int(r[3]) for r in rows)}
    if not result['passed']:
        raise ValueError('Zero tests passed; this is not successful regression evidence')
    return result


def require_command(command, *tokens):
    if not isinstance(command, str) or not all(t in command.split() for t in tokens):
        raise ValueError(f'Command must document these exact arguments: {tokens}')


def record_evidence(args):
    _, snapshot_sha = current_snapshot(args.source_snapshot)
    exit_record, exit_data = logged_file(args.exit_code_file)
    if exit_data.strip() != b'0':
        raise ValueError('The recorded process did not finish successfully (exit file must contain 0)')
    log, data = logged_file(args.log)
    report = {'schema': 1, 'revision': 52, 'kind': args.action.replace('record-', 'r52-') + '-report',
              'source_snapshot_sha256': snapshot_sha, 'command': args.command,
              'exit_code': 0, 'exit_code_file': exit_record, 'log': log,
              'windows_runtime_verified': False}
    if args.action == 'record-test':
        require_command(args.command, 'test', '--workspace')
        report.update({'test_platform': args.platform, 'counts': test_counts(data),
                       'limits': 'Only executed tests in the recorded log; ignored tests did not run. Not Windows runtime verification.'})
    else:
        require_command(args.command, 'build', '--release', '--target', TARGET, '-p', 'corpse-desktop')
        text = ANSI.sub('', data.decode('utf-8', errors='replace'))
        if not re.search(r'Finished\s+[`\']?release\b[^\n]*target', text):
            raise ValueError('Build log has no completed Cargo release target line')
        original = args.binary.read_bytes()
        pe_sections(original)
        report.update({'target': TARGET, 'profile': 'release', 'host': args.host,
                       'binary': {'bytes': len(original), 'sha256': digest(original)},
                       'rust_toolchain': '1.99.1', 'environment': args.environment,
                       'limits': 'Successful cross-compilation only; no Windows execution.'})
    write_new(args.output, json_bytes(report))
    print(json.dumps({'report': str(args.output), 'sha256': digest(args.output.read_bytes())}))


def validate_evidence(path, kind, snapshot_sha, original=None):
    report = read_json(path)
    if (report.get('schema'), report.get('revision'), report.get('kind')) != (1, 52, kind):
        raise ValueError(f'Expected current {kind}: {path}')
    if report.get('source_snapshot_sha256') != snapshot_sha or report.get('exit_code') != 0:
        raise ValueError(f'Failed or stale-source evidence: {path}')
    if verified_log(report['exit_code_file']).strip() != b'0':
        raise ValueError('Process exit evidence is not successful')
    log = verified_log(report['log'])
    if report.get('windows_runtime_verified') is not False:
        raise ValueError('This cross-build packager does not admit a Windows-runtime-tested claim')
    if kind == 'r52-test-report':
        require_command(report['command'], 'test', '--workspace')
        if report.get('counts') != test_counts(log):
            raise ValueError('Test report counts do not match the actual log')
    else:
        require_command(report['command'], 'build', '--release', '--target', TARGET, '-p', 'corpse-desktop')
        if report.get('target') != TARGET or report.get('rust_toolchain') != '1.99.1':
            raise ValueError('Unexpected build target/toolchain')
        if not re.search(r'Finished\s+[`\']?release\b[^\n]*target', ANSI.sub('', log.decode('utf-8', errors='replace'))):
            raise ValueError('Missing completed release-build evidence')
        if report.get('binary') != {'bytes': len(original), 'sha256': digest(original)}:
            raise ValueError('Build report is for a different executable')
    return report


def pe_sections(data):
    """Compare mapped section bytes/RVAs/sizes/flags and selected loader headers.

    O(S + B) time/extra bytes for S sections and B section bytes. Debug sections
    alone may be removed. Signed or delay-import inputs fail closed.
    """
    if len(data) < 64 or data[:2] != b'MZ':
        raise ValueError('Expected Windows PE executable')
    pe = struct.unpack_from('<I', data, 60)[0]
    if pe + 24 > len(data) or data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Missing or truncated PE header')
    machine, count = struct.unpack_from('<HH', data, pe + 4)
    optional = struct.unpack_from('<H', data, pe + 20)[0]
    op = pe + 24
    if machine != 0x8664 or optional < 240 or op + optional + count * 40 > len(data):
        raise ValueError('Expected complete PE32+ x86-64 headers')
    if struct.unpack_from('<H', data, op)[0] != 0x20b:
        raise ValueError('Expected PE32+ optional header')
    directories = [struct.unpack_from('<II', data, op + 112 + i * 8) for i in range(16)]
    if directories[4] != (0, 0):
        raise ValueError('Signed input: packaging does not preserve Authenticode signatures')
    if directories[13] != (0, 0):
        raise ValueError('Delay imports need separate dependency validation; refusing this input')
    symbols, symbol_count = struct.unpack_from('<II', data, pe + 12)
    string_table = symbols + symbol_count * 18 if symbols else 0
    sections = {}
    for index in range(count):
        offset = op + optional + index * 40
        name = data[offset:offset + 8].rstrip(b'\0').decode('ascii')
        if name.startswith('/') and name[1:].isdigit():
            begin = string_table + int(name[1:])
            end = data.find(b'\0', begin)
            if not string_table or begin >= len(data) or end < begin:
                raise ValueError('Malformed COFF section name')
            name = data[begin:end].decode('ascii')
        virtual_size, rva, size, start = struct.unpack_from('<IIII', data, offset + 8)
        flags = struct.unpack_from('<I', data, offset + 36)[0]
        if start and start + size > len(data):
            raise ValueError('Truncated PE section')
        if flags & 0xE0000000 and not name.startswith(('.debug', '.zdebug')):
            if name in sections:
                raise ValueError('Duplicate mapped PE section')
            sections[name] = (virtual_size, rva, flags, data[start:start + size] if start else b'')
    if not sections:
        raise ValueError('PE has no mapped sections')
    mapped_end = max(rva + max(virtual_size, len(raw))
                     for virtual_size, rva, _, raw in sections.values())
    if struct.unpack_from('<I', data, op + 56)[0] < mapped_end:
        raise ValueError('PE SizeOfImage does not cover mapped sections')
    headers = {'entry': struct.unpack_from('<I', data, op + 16)[0],
               'image_base': struct.unpack_from('<Q', data, op + 24)[0],
               'section_alignment': struct.unpack_from('<I', data, op + 32)[0],
               'subsystem': struct.unpack_from('<H', data, op + 68)[0],
               'dll_characteristics': struct.unpack_from('<H', data, op + 70)[0],
               'directories': tuple(d for i, d in enumerate(directories) if i not in (4, 6))}
    return headers, sections


def imports(path, objdump, allow_sdl):
    output = subprocess.check_output([str(objdump), '-p', str(path)], text=True)
    found = sorted(set(re.findall(r'DLL Name:\s*(\S+)', output)), key=str.lower)
    allowed = SYSTEM_DLLS | ({'sdl2.dll'} if allow_sdl else set())
    unknown = [name for name in found if name.lower() not in allowed]
    if not found or unknown:
        raise ValueError(f'Missing/unbundled/non-allowlisted import dependencies in {path}: {unknown}')
    if allow_sdl and 'sdl2.dll' not in {name.lower() for name in found}:
        raise ValueError('Expected SDL2.dll import is missing')
    return found


def art_evidence(original):
    assets = ROOT / 'crates/corpse-desktop/assets'
    creature = read_json(assets / 'creature-runtime-manifest.json')
    ui = read_json(assets / 'ui-runtime-manifest.json')
    ids = {path.stem for path in (assets / 'creatures').glob('*.png')}
    runtime = {path.stem for path in (assets / 'creatures-runtime').glob('*.webp')}
    plates = creature['plates']
    if not ids or ids != runtime or ids != {r['id'] for r in plates} or len(plates) != len(ids):
        raise ValueError('Creature source/runtime/manifest member mismatch')
    if creature['count'] != len(ids):
        raise ValueError('Creature manifest count mismatch')
    rows = [(assets / 'creatures-runtime' / r['encoded_file'], r['encoded_sha256']) for r in plates]
    rows += [(assets / 'ui-runtime' / (r['name'] + '.webp'), r['webp_sha256']) for r in ui['assets']]
    if {p.name for p, _ in rows[len(plates):]} != {p.name for p in (assets / 'ui-runtime').glob('*.webp')}:
        raise ValueError('UI runtime/manifest member mismatch')
    # N exact byte searches cost O(N*B + A), not merely O(A) asset hashing.
    for path, expected in rows:
        path.resolve().relative_to(assets.resolve())
        data = path.read_bytes()
        if not data or digest(data) != expected or data not in original:
            raise ValueError(f'Runtime artwork differs from manifest or is not embedded: {path.name}')
    return {'creature_count': len(ids), 'creature_ids': sorted(ids), 'ui_count': len(ui['assets']),
            'runtime_asset_hashes_checked': True, 'exact_encoded_bytes_found_in_input_executable': True,
            'scope': 'Encoded-byte identity only; does not newly verify decoded pixels or artwork quality.'}


def verify_pinned_sdl(sdk, dll):
    archive = sdk / 'downloads/SDL2-devel-2.32.10-mingw.tar.gz'
    data = archive.read_bytes()
    expected = '83a5d74012311edc3c0d40ea6faecbe57ad692aa033fa5dc273cc937e3938ff2'
    if digest(data) != expected:
        raise ValueError('SDL2 archive differs from the pinned official 2.32.10 digest')
    with tarfile.open(archive, 'r:gz') as source:
        member = source.extractfile('SDL2-2.32.10/x86_64-w64-mingw32/bin/SDL2.dll')
        if member is None or member.read() != dll:
            raise ValueError('SDL2.dll differs from the pinned archive runtime DLL')
    return expected


def runtime_files(sdk):
    assets = ROOT / 'crates/corpse-desktop/assets'
    sdl = sdk / 'SDL2-2.32.10'
    files = {'notices/SDL2-LICENSE.txt': (sdl / 'LICENSE.txt').read_bytes()}
    for name in ('creature-runtime-manifest.json', 'ui-runtime-manifest.json',
                 'creature-alpha-manifest.json', 'art-distribution.json', 'NOTICE.md'):
        files['notices/artwork/' + name] = (assets / name).read_bytes()
    for src in sorted(assets.glob('*OFL.txt')):
        files['notices/fonts/' + src.name] = src.read_bytes()
    if not any(name.startswith('notices/fonts/') for name in files):
        raise ValueError('Missing font licenses')
    for source, destination in ((assets / 'brogue', 'notices/brogue/'),
                                (ROOT / 'third-party-notices', 'notices/crates/')):
        for path in sorted(source.rglob('*')):
            if path.is_file():
                files[destination + path.relative_to(source).as_posix()] = path.read_bytes()
    dependency_notices = read_json(ROOT / 'third-party-notices/manifest.json')
    covered = {(row['name'], row['version']) for row in dependency_notices}
    for dependency in tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']:
        if 'source' in dependency and (dependency['name'], dependency['version']) not in covered:
            raise ValueError(f'Missing Cargo.lock dependency notice: {dependency["name"]}')
    for dependency in dependency_notices:
        for notice in dependency.get('notices', []):
            if 'notices/crates/' + notice not in files:
                raise ValueError(f'Missing dependency notice: {notice}')
    # Include narrow provenance records, never compiler/sysroot/archive payloads
    # or HTTP headers containing temporary download credentials.
    for name in ('pinned-archives-manifest.json', 'rust-target-manifest.json',
                 'sdl-official-asset-verification.json'):
        data = (sdk / 'receipts' / name).read_bytes()
        json.loads(data)
        files['provenance/sdk/' + name] = data
    files['provenance/Cargo.lock'] = (ROOT / 'Cargo.lock').read_bytes()
    files['provenance/rust-toolchain.toml'] = (ROOT / 'rust-toolchain.toml').read_bytes()
    files['provenance/Cargo.toml'] = (ROOT / 'Cargo.toml').read_bytes()
    return files


START_HERE = """R52 Avatar and world follow-up, using simulation revision 52 for new runs. New runs are Avatar-only; existing saves and replays keep their recorded modes and revisions.

Choose three distinct gods on the main menu. Each god button uses the authored god color and the designer's exact name, alias and domain, including NUMO the Pregenitor. The player is displayed as @ without a mode superscript; creature glyphs and tier marks remain unchanged. Historical save/replay compatibility is preserved. Monster conditions use comma-separated wrapped text, and monster energy is shown only for actual energy-using capabilities. Actual wall-feeding creatures show Feeding. Innate Fast is distinguished from true Haste; see the included scheduling notes for movement, attacks and saved phase compatibility. New corpse roots grow one tile per three normal turns and retain root appearance under hallucination. Being in any gas suppresses only gas-entry warnings. Enemy sight previews enclose the full confidently known visible area without exposing unknown space. Chasms retain their dark palette in current sight and memory. Corpses keep their living identity colors under a larger red X. Exposed-wall and stale-flesh # glyphs use distinct colors while preserving neutral fog. Automatic routes use shallow water without its old detour penalty, never deep water. Fated Duel ends when its selected target dies or leaves the anchored area.

New R52 runs generate one floor ahead rather than whole blocks. Newly generated deep lakes use seeded overlapping square rotations and the specified collision rules; saved floors retain their geometry/currents. Only islands without an ordinary swimming escape receive a Ring of Swimming, including the starting landmass when eligible. It grants ordinary nonstacking haste in shallow/deep water while not flying, with normal ring capacity. Generation checks ring-assisted escape and entrance-to-downstairs progress without a preowned ring; static validation does not guarantee safety from dynamic enemies or hazards. See the bundled generation/Swimming documents for exact bounds and compatibility.

This candidate adds immediate -100 radius-3 Darkness and approved outcome-based action feedback, with Normal, Reduced and Instant animation settings. New R52 creatures retain role-specific Accuracy offsets and use first-appearance Dodge on every floor. This update also includes the approved player damage/Dodge, heart capacity, Jackal, reserve, reaction, Hanged, interface, and glyph/light presentation changes. Seven prior source files were behaviorally reconstructed rather than recovered byte-for-byte; see provenance/design/revision52-source-reconstruction.md. Earlier encounter balance measurements do not certify the new hit formula. Existing R51 UI and visibility behavior are retained. This is a release candidate; scope, provisional choices and platform limits are listed below. Linux tests and Windows cross-compilation are verified in the bundled reports. Windows runtime and physical DPI transitions have not been tested.

THE CALL OF THE CORPSE GODS - R52 - WINDOWS 64-BIT

Extract the WHOLE ZIP into a folder, then double-click corpse-desktop.exe.
If supplied as two ZIP attachments, extract BOTH into that same folder first.
Keep SDL2.dll beside corpse-desktop.exe and retain the notices/provenance folders.
Do not launch from inside the ZIP. No installer, compiler, SDK, terminal commands,
or separate artwork download is required. This is an SDL graphical-window game,
not a text-terminal game. Play in the game window if a console also appears.

This is an unsigned Windows x64 cross-build. It has NOT been runtime-tested on
Windows. Static PE/import/archive checks and recorded source tests do not prove
Windows launch, rendering, input, audio, fullscreen, saves or gameplay behavior.
Do not bypass security warnings merely because the archive passed these checks.
See MANIFEST.json and provenance/ for exact source/build/test evidence and limits.

BASIC CONTROLS (context-sensitive; dialogs consume their own input)
- Move: arrow keys, WASD, H/J/K/L; diagonals Y/U/B/N or numeric keypad 7/9/1/3.
  Numeric keypad 4/6/8/2 also moves. Click known ground to walk toward it.
- Wait: period, Space, or keypad 5. Shift+Z begins resting; Escape interrupts.
- X previews a route to the nearest reachable unexplored frontier; a second
  distinct X press follows it. Other input cancels; threats and hazards stop travel.
- I opens Pack. M opens Bestiary. ? opens the menu/basic display settings.
- Alt+Enter cycles windowed, borderless, and exclusive fullscreen modes.
- F5 saves a normal run. F9 reloads the saved run, discarding unsaved progress.
  On the title menu, Continue resumes the available run. DEBUG saves are separate.
- Choose powers and item actions from their displayed controls; unavailable
  actions explain their blockers. Escape cancels/closes the current modal.
- While aiming, movement keys move the target cursor and Enter confirms it.
  Gold shows delivery; coral shows impact or affected area. Escape cancels.

DEBUG FLOOR TESTING
Title menu: PLAY FROM X'TH FLOOR. Choose floor 1-40 and three distinct gods for
your Avatar, keep or enter the seed, then make the earned power choices and select carried loot.
Space confirms loadout. F4 reopens the DEBUG stash during play. F2 rerolls the
setup seed; Ctrl+C/Ctrl+V copies/pastes it in DEBUG setup. These are reconstructed
prior-floor choices, not a played-through run. DEBUG does not replace normal
Continue/autosaves or enter high scores. F7 explicitly exports a .debug.json state.

INVENTORY
Pack has five sections: Trinkets, Amulets, Rings, Potions, and Hearts. Rings,
amulets and potions each start at five units; R52 hearts start at ten. Trinkets are unlimited.
Carried rings and amulets are automatically active. Some blessings raise category
limits. Older over-limit R50 saves are rejected without modifying the save file.

CREATURE DISPLAY
Biters are now Chompers, Breathers are Gassers, and Bastidion is Corpse Fungus.
Blorb uses a dark-green b; Bear uses a brown B; Dire Bear uses a black B with
a thin neutral outline for legibility. Devil's Samara Tree uses a brown T; Yeti Y.
Approved tier superscripts distinguish Ichor Fish/Sharks/Leviathans (1-3),
Zillennia (1-10), and Nightmares (1-8). Ranked ally orders appear below the glyph.
Vampire uses uppercase V. Internal species IDs remain unchanged.

BESTIARY PROGRESS
Only directly seen creatures unlock their pages. Unknown pages show a question
mark; yellow exclamation marks identify unread pages. Discovery is stored beside
the selected player profile (profile.json.bestiary.json in the default application
configuration directory), separately from run saves. No cloud account is needed.

ACCURACY AND NEW-RUN BASELINES
R52 ordinary hit chance uses Accuracy / (2 * max(Dodge, 1)), bounded to 0-100%,
then the existing range penalty. Existing guaranteed-hit/status/Bow rules remain.
New players start with 15 Accuracy and 15 Dodge. New creatures start with
15 Accuracy plus species/floor modifiers and 10 Dodge plus the exact type's
first-appearance Dodge offset. Innate player melee fixed damage is 37.8 (up
20% from 31.5); Offense/equipment bonuses and Bow/thrown bases are unchanged.
New Jackals are heartless; normal retained-loot compensation remains in effect.
Start a new run to receive the new player starting stats. Existing R52 saves
keep their stored stats but use the new formula; an older zero-Accuracy player
is not automatically migrated. Pre-R52 saves retain their previous formula.
These changes are not a new full encounter-balance certification.
See provenance/design/revision52-stat-followups.md and revision52-accuracy-formula.md for details.

RESERVES, REACTIONS AND HANGED
R52 ability and amulet reserves cap at 10, including loaded R52 reserves. Main
player energy and activation/upkeep costs are unchanged. Wait refreshes one
reaction up to the maximum. Hanged replaces Missed Me as a permanent passive:
post-mitigation damage below 1 HP uses main energy at 4 energy per HP, including
overflow from the same hit. Insufficient energy is drained and uncovered damage
can kill. It adds no reaction and has no toggle. Health costs and terminal deaths
retain their existing rules. Pre-R52 saves preserve their original behavior.

STAT LABELS AND ENERGY
Damage shows the nominal player melee bonus, not total hit damage or bow damage.
HP bonuses replace former Defense at six HP per point; total HP stays in the bar.
Dream-See and Mind Read cost 1 per turn, Bonded costs 15 plus 8 per turn, and
I Came For You costs 10. Implant Thought remains 15. Immediate paid benefits
charge their first upkeep when activated and do not double-charge that turn.
Revision51 and newer runs round damage to nearest whole numbers after mitigation,
limit generated Composite bows to one per dungeon, and give each valid fungus
feast one current-depth and one 2-3-floor-deeper corpse, excluding Godbeasts.
Older saves retain their recorded revision for these new rules.

DESIGN STATUS
This is a candidate, not a final release. The designer resolved the reachable
Iceblast/Blink line, Sparkle's Ichor-water exception, adjacent Speed-up destination
pairs, and the Dragon/Blorb/Killspawn follow-up mechanics. Their actual scope and
verification limits are recorded in the bundled source/test evidence and design
notes. Newly born amulets use Tier 1 on floors 1-4, then a new tier every five
floors through Tier 9 on floor 40. Existing amulets keep their tier and remaining
energy up to the new R52 reserve ceiling of 10. Costs stay unchanged. Later damage tiers are independently
fitted toward approximate rising casts per individual; periodic amulets retain
their durations and are also measured by elapsed time. Utility tiers increase
Blinking/Beckoning/Teleportation target range or Discord/Obstruction duration.
The integrated all-role catalog contains 550 birth profiles. Supports, casters,
summoner families and the deliberately harder Nest retain their distinct roles;
measured variation and exceptional encounters are disclosed in the design notes.
Bestiary pages display current base-stat ranges across the relevant spawn floors
without revealing unseen pages. Floods consumes shallow water into deep water;
converted terrain uses the existing temporary-water lifetime. The terminal
shallow cell retains ordinary wave damage/push as a documented interpretation.

Repulsion's exact-center case remains provisional: choose a random valid outward direction using the
serialized gameplay RNG. This is a documented implementation choice, not a new
balance claim. Test coverage is exactly the supplied reports, not exhaustive.

ARTWORK AND LICENSE NOTICES
Runtime creature/UI artwork and fonts are embedded in the executable. The current
runtime manifests are in notices/artwork/. Creature WebP color is lossy at its
recorded quality; dimensions/alpha preservation is recorded in that manifest.
UI WebP is recorded as lossless. Packaging verifies current encoded file hashes
and their presence in the executable, not new visual or decoded-pixel acceptance.
Original historical notices are preserved verbatim; older counts/encoding notes
in them describe earlier checkpoints, not this package's runtime asset inventory.
Font OFL, adapted Brogue artwork, SDL2, and Rust dependency notices are retained.
Dependency/artwork notices do not establish a new license for the game itself.
The SDK/compiler and authoring master artwork are not shipped in this play ZIP.
"""


def write_zip(output, files):
    for name in files:
        parts = PurePosixPath(name)
        if parts.is_absolute() or '..' in parts.parts or '\\' in name:
            raise ValueError(f'Unsafe archive member: {name}')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data, compresslevel=9)
    with zipfile.ZipFile(output) as archive:
        names = archive.namelist()
        if archive.testzip() is not None or len(names) != len(files) or set(names) != set(files):
            raise ValueError('ZIP integrity or member-set failure')
        for name, data in files.items():
            actual = archive.read(name)
            if len(actual) != len(data) or digest(actual) != digest(data) or actual != data:
                raise ValueError(f'ZIP byte/hash verification failure: {name}')


def package(args):
    receipt_path = args.output.with_suffix('.verification.json')
    if args.output.exists() or receipt_path.exists():
        raise ValueError('Refusing to overwrite an archive or verification receipt')
    snapshot, snapshot_sha = current_snapshot(args.source_snapshot)
    if 'channel = "1.99.1"' not in (ROOT / 'rust-toolchain.toml').read_text():
        raise ValueError('Expected pinned Rust 1.99.1')
    original = args.binary.read_bytes()
    before = pe_sections(original)
    tests = validate_evidence(args.test_report, 'r52-test-report', snapshot_sha)
    build = validate_evidence(args.build_report, 'r52-build-report', snapshot_sha, original)
    art = art_evidence(original)
    strip = (args.sdk / 'sysroot/usr/bin/x86_64-w64-mingw32-strip').resolve()
    objdump = (args.sdk / 'sysroot/usr/bin/x86_64-w64-mingw32-objdump').resolve()
    dll_path = args.sdk / 'SDL2-2.32.10/x86_64-w64-mingw32/bin/SDL2.dll'
    dll = dll_path.read_bytes()
    pe_sections(dll)
    sdl_archive_sha = verify_pinned_sdl(args.sdk, dll)
    strip_version = subprocess.check_output([str(strip), '--version'], text=True).splitlines()[0]
    objdump_version = subprocess.check_output([str(objdump), '--version'], text=True).splitlines()[0]
    with tempfile.TemporaryDirectory(prefix='corpse-r52-package-') as temporary:
        staged = Path(temporary) / 'corpse-desktop.exe'
        shutil.copyfile(args.binary, staged)
        original_imports = imports(staged, objdump, True)
        subprocess.run([str(strip), '--strip-all', str(staged)], check=True)
        stripped = staged.read_bytes()
        if before != pe_sections(stripped):
            raise ValueError('Stripping changed mapped PE section bytes/layout/flags or selected loader headers')
        exe_imports = imports(staged, objdump, True)
        if exe_imports != original_imports:
            raise ValueError('Stripping changed executable import dependencies')
        # Inspect the same DLL bytes that will be included, not a mutable SDK path.
        staged_dll = Path(temporary) / 'SDL2.dll'
        staged_dll.write_bytes(dll)
        dll_imports = imports(staged_dll, objdump, False)
    files = runtime_files(args.sdk)
    files.update({'corpse-desktop.exe': stripped, 'SDL2.dll': dll,
                  'START-HERE.txt': START_HERE.encode(),
                  'provenance/source-snapshot.json': snapshot,
                  'provenance/test-report.json': args.test_report.read_bytes(),
                  'provenance/build-report.json': args.build_report.read_bytes(),
                  'provenance/workspace-test.log': verified_log(tests['log']),
                  'provenance/windows-build.log': verified_log(build['log'])})
    for name in ('revision52-release-status.md', 'revision52-fighter-balance.md',
                 'revision52-action-animation-contract.md', 'revision52-targeting-sparkle.md',
                 'revision52-enemy-followups.md', 'revision52-amulet-tiers.md',
                 'revision52-amulet-tier-ui.md', 'revision52-all-role-balance.md',
                 'revision52-bestiary-stats.md', 'revision52-floods.md',
                 'r52-floods-native-ui-verification.md', 'revision52-accuracy-formula.md',
                 'revision52-stat-followups.md', 'revision52-reserves-wait.md',
                 'revision52-hanged.md', 'revision52-source-reconstruction.md',
                 'revision52-floods-reconstruction.md', 'revision52-desktop-source-reconstruction.md',
                 'revision52-brogue-animation-style.md', 'r52-chasm-color-correction.md',
                 'revision52-glyph-mask-coverage.md', 'bestiary-continuous-colors.md',
                 'bestiary-portrait-mask-repairs-20261108.md',
                 'avatar-new-run-boundary-r52.md', 'revision52-avatar-menu.md',
                 'player-glyph-presentation.md', 'monster-status-presentation.md',
                 'gas-entry-warnings.md', 'revision52-root-growth.md',
                 'revision52-square-currents.md', 'revision52-swimming-islands.md',
                 'revision52-enemy-perception-overlay.md', 'revision52-dark-chasm-memory.md',
                 'revision52-corpse-colors-cross.md', 'revision52-shallow-water-pathing.md',
                 'revision52-wall-flesh-colors.md', 'revision52-feeding-state.md',
                 'revision52-fast-versus-haste.md',
                 'revision52-fated-duel.md'):
        files['provenance/design/' + name] = (ROOT / 'docs' / name).read_bytes()
    manifest = {
        'schema': 1, 'label': 'R52-windows-x64', 'simulation_revision': 52,
        'unsigned': True, 'windows_runtime_verified': False,
        'source_snapshot_sha256': snapshot_sha,
        'test_counts_from_verified_log': tests['counts'],
        'test_limits': tests['limits'], 'build_target': build['target'],
        'embedded_art': art, 'sdk_included': False,
        'sdl2_archive_sha256': sdl_archive_sha, 'sdl2_runtime_matches_pinned_archive': True,
        'design_status': {'release_status': 'candidate',
                          'unresolved': [],
                          'unfinished': [],
                          'existing_behavior_preserved_pending_decisions': False,
                          'provisional': "Creature Accuracy baseline 15 is provisionally designer-approved; earlier encounter fits do not certify the new hit formula. Repulsion exact-center direction uses serialized gameplay RNG; terminal shallow-water Floods impact retains ordinary damage/push as a documented implementation inference."},
        'original_binary': {'bytes': len(original), 'sha256': digest(original)},
        'packaged_binary': {'bytes': len(stripped), 'sha256': digest(stripped)},
        'strip': {'tool': strip.name, 'version': strip_version, 'tool_sha256': digest(strip.read_bytes()),
                  'arguments': ['--strip-all'], 'input_binary_preserved': True,
                  'non_debug_mapped_sections_and_selected_headers_equal': True,
                  'compared_header_fields': ['AddressOfEntryPoint', 'ImageBase',
                      'SectionAlignment', 'Subsystem', 'DllCharacteristics',
                      'DataDirectory entries except Security and Debug'],
                  'scope': 'Mapped non-debug section bytes, virtual sizes, RVAs and flags plus the listed headers; not every PE header byte.'},
        'pe_inspection': {'tool': objdump.name, 'version': objdump_version,
                          'tool_sha256': digest(objdump.read_bytes()),
                          'exe_imports': exe_imports, 'sdl2_imports': dll_imports,
                          'system_dll_allowlist': sorted(SYSTEM_DLLS),
                          'scope': 'Static import tables; not runtime loading or execution.'},
        'files': [{'path': name, 'bytes': len(data), 'sha256': digest(data)} for name, data in sorted(files.items())],
    }
    files['MANIFEST.json'] = json_bytes(manifest)
    # Recheck immediately before publication to reject concurrent edits/builds.
    current_snapshot(args.source_snapshot)
    if args.binary.read_bytes() != original:
        raise ValueError('Original build binary changed during packaging')
    write_zip(args.output, files)
    if args.binary.read_bytes() != original:
        raise ValueError('Original build binary changed while writing ZIP; no successful receipt issued')
    receipt = {'path': str(args.output), 'bytes': args.output.stat().st_size,
               'sha256': digest(args.output.read_bytes()), 'archive_verified': True,
               'member_count': len(files), 'source_snapshot_sha256': snapshot_sha,
               'windows_runtime_verified': False}
    write_new(receipt_path, json_bytes(receipt))
    print(json.dumps(receipt, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    snap = sub.add_parser('snapshot', help='Hash source inputs before tests/build, without running either')
    snap.add_argument('--output', type=Path, required=True)
    for action in ('record-test', 'record-build'):
        item = sub.add_parser(action, help='Record actual completed log and exit-file evidence')
        item.add_argument('--source-snapshot', type=Path, required=True)
        item.add_argument('--log', type=Path, required=True)
        item.add_argument('--exit-code-file', type=Path, required=True)
        item.add_argument('--command', required=True)
        item.add_argument('--output', type=Path, required=True)
        if action == 'record-test':
            item.add_argument('--platform', required=True, help='Actual test host, e.g. Linux x86_64')
        else:
            item.add_argument('--binary', type=Path, default=BINARY)
            item.add_argument('--host', required=True, help='Actual cross-build host')
            item.add_argument('--environment', action='append', default=[],
                              help='Build-relevant NAME=value only; repeat for profile overrides; no secrets')
    pack = sub.add_parser('package', help='Verify evidence and create the final runtime ZIP')
    pack.add_argument('--binary', type=Path, default=BINARY)
    pack.add_argument('--source-snapshot', type=Path, required=True)
    pack.add_argument('--test-report', type=Path, required=True)
    pack.add_argument('--build-report', type=Path, required=True)
    pack.add_argument('--sdk', type=Path, default=ROOT.parent / 'native-windows-sdk')
    pack.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.action == 'snapshot':
            write_new(args.output, json_bytes(source_snapshot()))
            print(json.dumps({'snapshot': str(args.output), 'sha256': digest(args.output.read_bytes())}))
        elif args.action.startswith('record-'):
            record_evidence(args)
        else:
            package(args)
    except (ValueError, KeyError, OSError, struct.error, subprocess.CalledProcessError) as error:
        parser.exit(1, f'R52 packaging refused: {error}\n')


if __name__ == '__main__':
    main()

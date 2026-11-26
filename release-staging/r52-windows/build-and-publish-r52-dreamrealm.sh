#!/usr/bin/env bash
set -euo pipefail

mode=${1:?build or publish}
commit=955cb8105874e767cfa4e42b04c098666dcba4ca
tree=e89b9799b7c411d382e3570355368091804175af
tag=r52-dreamrealm-20261126
asset=Call-of-the-Corpse-Gods-R52-Dreamrealm-Windows-x64.zip
sdk="${RUNNER_TEMP:?}/corpse-r52-sdk"
out="$RUNNER_TEMP/corpse-r52-release"
source="${GITHUB_WORKSPACE:?}/game-source"

if [[ "$mode" == build ]]; then
  mkdir -p "$sdk"/{downloads,receipts,sysroot,toolchain} "$out"
  cat > "$sdk/receipts/pinned-archives-manifest.json" <<'JSON'
[
  {
    "name": "binutils-mingw-w64-x86-64",
    "url": "https://deb.debian.org/debian/pool/main/b/binutils-mingw-w64/binutils-mingw-w64-x86-64_2.44-3+12+b1_amd64.deb",
    "sha256": "b53f4a091b16a0d0952b8e3bd6027931641386258f96b37db1f0c62c787221d3"
  },
  {
    "name": "gcc-mingw-w64-base",
    "url": "https://deb.debian.org/debian/pool/main/g/gcc-mingw-w64/gcc-mingw-w64-base_14.2.0-19+27+b1_amd64.deb",
    "sha256": "8d2c64b886ab4a435a78ddc4cd3b510f149b65fbd6be8472e8f460627562718a"
  },
  {
    "name": "gcc-mingw-w64-x86-64-posix",
    "url": "https://deb.debian.org/debian/pool/main/g/gcc-mingw-w64/gcc-mingw-w64-x86-64-posix_14.2.0-19+27+b1_amd64.deb",
    "sha256": "e32d24cfd1482394f16d9a84c1d0a9f5aa08ee826a79acc383c70cc8c23c4fe5"
  },
  {
    "name": "gcc-mingw-w64-x86-64-posix-runtime",
    "url": "https://deb.debian.org/debian/pool/main/g/gcc-mingw-w64/gcc-mingw-w64-x86-64-posix-runtime_14.2.0-19+27+b1_amd64.deb",
    "sha256": "c898d96177574a8a7238564a1e799d8ddfed5a3d2ea005510305f9ad466a6f59"
  },
  {
    "name": "mingw-w64-common",
    "url": "https://deb.debian.org/debian/pool/main/m/mingw-w64/mingw-w64-common_12.0.0-5_all.deb",
    "sha256": "97dce5d0d8aff1cade4786ad4ef5078345477a598829afcac2e7f2fbc5399990"
  },
  {
    "name": "mingw-w64-x86-64-dev",
    "url": "https://deb.debian.org/debian/pool/main/m/mingw-w64/mingw-w64-x86-64-dev_12.0.0-5_all.deb",
    "sha256": "0bf89cf7454cccb49cd2a46a6f7b33896f6ae3a1a2fe33e02c40be6155bbb385"
  }
]
JSON
  cat > "$sdk/receipts/rust-target-manifest.json" <<'JSON'
{
  "compiler": "1.99.1",
  "component": "rust-std",
  "target": "x86_64-pc-windows-gnu",
  "installation": "rustup target add --toolchain 1.99.1 x86_64-pc-windows-gnu",
  "available": true,
  "url": "https://static.rust-lang.org/dist/2026-10-15/rust-std-1.99.1-x86_64-pc-windows-gnu.tar.gz",
  "hash": "6546651238a3ac092b763511be524ca6eda6103e5d47342416923be1b3ce837c",
  "xz_url": "https://static.rust-lang.org/dist/2026-10-15/rust-std-1.99.1-x86_64-pc-windows-gnu.tar.xz",
  "xz_hash": "b17e3bc962ff24b3bc47a750d838defed85b280746250e3a3ad029e6c1a29dd7"
}
JSON
  cat > "$sdk/receipts/sdl-official-asset-verification.json" <<'JSON'
{"name": "SDL2-devel-2.32.10-mingw.tar.gz", "url": "https://github.com/libsdl-org/SDL/releases/download/release-2.32.10/SDL2-devel-2.32.10-mingw.tar.gz", "digest": "sha256:83a5d74012311edc3c0d40ea6faecbe57ad692aa033fa5dc273cc937e3938ff2"}
JSON
  python3 - "$sdk" <<'PY_SDK'
import concurrent.futures, hashlib, json, pathlib, subprocess, sys
root = pathlib.Path(sys.argv[1])
def receipt(name): return json.loads((root / 'receipts' / name).read_text())
mingw = receipt('pinned-archives-manifest.json')
rust = receipt('rust-target-manifest.json')
sdl = receipt('sdl-official-asset-verification.json')
base = 'https://static.rust-lang.org/dist/2026-10-15/'
host = {
    'cargo-1.99.1-x86_64-unknown-linux-gnu.tar.xz': '53f3521949404d595efb06a55c12c5658d4ed1c75566f33d3d514eaebe70bd8a',
    'rust-std-1.99.1-x86_64-unknown-linux-gnu.tar.xz': '109b074225bf5b9ea3820890876bfdf4ed161adbfaf94b2b4b5984c85ee5d958',
    'rustc-1.99.1-x86_64-unknown-linux-gnu.tar.xz': '2ff05ff9c08a90861e7f6e4065b40a8b87463b4576186fd613871c3e37f2091f',
}
jobs = [(e['url'], e['sha256']) for e in mingw] + [
    (rust['xz_url'], rust['xz_hash']), (sdl['url'], sdl['digest'].split(':')[1]),
] + [(base + name, sha) for name, sha in host.items()]
def download(job):
    url, expected = job
    path = root / 'downloads' / url.rsplit('/', 1)[1]
    subprocess.run(['curl','--proto','=https','--proto-redir','=https','-fLsS','--retry','4','--max-time','240','-o',str(path),url],check=True)
    with path.open('rb') as f: actual = hashlib.file_digest(f, 'sha256').hexdigest()
    if actual != expected: raise RuntimeError('Unexpected SHA-256 for ' + path.name)
    print('Verified ' + path.name, flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
    list(pool.map(download, jobs))
PY_SDK
  for pkg in "$sdk"/downloads/*.deb; do dpkg-deb -x "$pkg" "$sdk/sysroot"; done
  tar xf "$sdk/downloads/SDL2-devel-2.32.10-mingw.tar.gz" -C "$sdk"
  for archive in "$sdk"/downloads/{rustc,cargo,rust-std}-*.tar.xz; do
    tar xf "$archive" -C "$sdk"
    component=${archive##*/}
    "$sdk/${component%.tar.xz}/install.sh" --prefix="$sdk/toolchain" --disable-ldconfig
  done
  export SDK="$sdk/sysroot"
  export SDL2_LIB_DIR="$sdk/SDL2-2.32.10/x86_64-w64-mingw32/lib"
  cat > "$sdk/linker.sh" <<'LINKER'
#!/usr/bin/env bash
set -euo pipefail
exec "$SDK/usr/bin/x86_64-w64-mingw32-gcc-posix" -B"$SDK/usr/lib/gcc/x86_64-w64-mingw32/14-posix/" -B"$SDK/usr/x86_64-w64-mingw32/lib/" -L"$SDK/usr/x86_64-w64-mingw32/lib" -L"$SDL2_LIB_DIR" "$@"
LINKER
  chmod +x "$sdk/linker.sh"
  export PATH="$sdk/toolchain/bin:$SDK/usr/bin:$PATH"
  export RUSTC="$sdk/toolchain/bin/rustc"
  export WINDRES="$SDK/usr/bin/x86_64-w64-mingw32-windres"
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$sdk/linker.sh"
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="--sysroot=$sdk/toolchain"
  export CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  export CARGO_INCREMENTAL=0 SDL_VIDEODRIVER=dummy
  export CARGO_TARGET_DIR="$source/target"
  cd "$source"
  [[ "$(git rev-parse HEAD)" == "$commit" && "$(git rev-parse HEAD^{tree})" == "$tree" ]]
  git diff --quiet && git diff --cached --quiet
  rustc --version
  cargo --version
  cargo fetch --locked
  evidence="target/r52-release-evidence/$commit"
  mkdir -p "$evidence"
  packer=tools/package-r52-windows.py
  # The tracked parts are canonical. Materialize the combined fixture before
  # freezing source, so the core build's own verified restoration is a no-op.
  fixture=fixtures/mini-me-starter-combinations-v48.json
  if [[ ! -e "$fixture" ]]; then
    cat "$fixture.part1" "$fixture.part2" > "$fixture"
  fi
  cmp "$fixture" <(cat "$fixture.part1" "$fixture.part2")
  python3 "$packer" snapshot --output "$evidence/source.json"
  build_args=(build --release --offline --locked --target x86_64-pc-windows-gnu -p corpse-desktop)
  if cargo "${build_args[@]}" > "$evidence/windows-build.log" 2>&1; then
    printf '0
' > "$evidence/windows-build.exit"
  else
    cat "$evidence/windows-build.log" >&2
    exit 1
  fi
  exe=target/x86_64-pc-windows-gnu/release/corpse-desktop.exe
  sed -i -e "s|$source|<checkout>|g" -e "s|$sdk|<windows-sdk>|g" "$evidence/windows-build.log"
  python3 "$packer" record-build --source-snapshot "$evidence/source.json"     --log "$evidence/windows-build.log" --exit-code-file "$evidence/windows-build.exit"     --command "cargo ${build_args[*]}" --binary "$exe" --host 'GitHub Actions Ubuntu 24.04 x86_64'     --environment "SOURCE_GIT_COMMIT=$commit" --environment 'Rust 1.99.1 pinned xz components'     --environment 'MinGW POSIX and SDL2 pinned archives' --output "$evidence/build.json"
  test_args=(test --workspace --locked --offline --lib --bins)
  filters=('gas_inspector' 'phantasmii' 'dream_ui::' 'borge_throw_consumes_exact_heart' 'star_power' 'release_policy::' 'replay::policy_tests::')
  (
    unset SDL2_LIB_DIR
    for filter in "${filters[@]}"; do
      echo "\$ cargo ${test_args[*]} $filter"
      cargo "${test_args[@]}" "$filter" || exit $?
    done
  ) > "$evidence/linux-test.log" 2>&1 || { tail -150 "$evidence/linux-test.log" >&2; exit 1; }
  printf '0
' > "$evidence/linux-test.exit"
  sed -i -e "s|$source|<checkout>|g" -e "s|$sdk|<windows-sdk>|g" "$evidence/linux-test.log"
  python3 "$packer" record-test --source-snapshot "$evidence/source.json"     --log "$evidence/linux-test.log" --exit-code-file "$evidence/linux-test.exit"     --command "cargo ${test_args[*]} (each filter: ${filters[*]})" --platform 'Ubuntu 24.04 x86_64 focused native checks'     --output "$evidence/test.json"
  python3 "$packer" package --source-snapshot "$evidence/source.json"     --test-report "$evidence/test.json" --build-report "$evidence/build.json"     --binary "$exe" --sdk "$sdk" --output "$out/$asset"
  (cd "$out"; sha256sum "$asset" > SHA256SUMS.txt; sha256sum -c SHA256SUMS.txt; unzip -tq "$asset")
  exit 0
fi

[[ "$mode" == publish && "${GH_REPO:?}" == ConnorBache/Call-of-the-Corpse-Gods && -n "${GH_TOKEN:?}" ]]
cd "$out"
sha256sum -c SHA256SUMS.txt
unzip -tq "$asset"
cat > RELEASE-NOTES.md <<'NOTES'
Extract the Windows x64 ZIP and double-click `corpse-desktop.exe`. Keep `SDL2.dll` next to it.

- Shards of the Phantasmii can only be seen and collected while Dreamwalking, including the bonus shard on each floor.
- Ritual shards still unlock Dreamwalking through the altar dagger. Like the bonus shards, each carried ritual shard reduces each level-up cost by 3 Blessings, to a minimum of zero.
- The tile inspector now reports the actual refresh duration of paralysis, confusion, and hallucination gas, including lingering clouds. Gas mechanics are unchanged.
- Includes Star Power's opt-in burning, the centered heartless corpse marker, and both redesigned menus from the previous playtest.

The ZIP ships the graphical Windows x64 executable directly, with no terminal launcher. The build is unsigned. The packaged executable was launched to the title screen on a hosted Windows runner; full gameplay and Windows display configurations were not tested.
NOTES
lookup() {
  gh api --paginate "repos/$GH_REPO/releases?per_page=100" | jq -c --arg tag "$tag" '.[] | select(.tag_name == $tag)' |
    jq -cs 'if length <= 1 then .[0] // empty else error("duplicate release") end'
}
existing_tag=$(git ls-remote --refs "https://github.com/$GH_REPO.git" "refs/tags/$tag" | cut -f1)
[[ -z "$existing_tag" || "$existing_tag" == "$commit" ]]
release=$(lookup)
if [[ -z "$release" ]]; then
  gh release create "$tag" "$asset" SHA256SUMS.txt --target "$commit" --title 'R52: Dreamrealm Shards & Gas Inspector'     --notes-file RELEASE-NOTES.md --draft --prerelease
  release=$(lookup)
fi
jq -e --arg sha "$commit" --arg zip "$asset" --arg hash "sha256:$(sha256sum "$asset" | cut -d' ' -f1)" '
  .target_commitish == $sha and .prerelease and
  any(.assets[]; .name == $zip and .state == "uploaded" and .digest == $hash) and
  any(.assets[]; .name == "SHA256SUMS.txt" and .state == "uploaded")
' <<<"$release" >/dev/null
mkdir verify-release
gh release download "$tag" --pattern "$asset" --pattern SHA256SUMS.txt --dir verify-release
cmp SHA256SUMS.txt verify-release/SHA256SUMS.txt
(cd verify-release; sha256sum -c SHA256SUMS.txt)
if jq -e '.draft' <<<"$release" >/dev/null; then gh release edit "$tag" --draft=false; fi
test "$(gh api "repos/$GH_REPO/git/ref/tags/$tag" --jq .object.sha)" = "$commit"
gh api "repos/$GH_REPO/releases/tags/$tag" --jq 'select(.draft == false and .prerelease == true) | .html_url' | tee -a "$GITHUB_STEP_SUMMARY"

# Native desktop test scene

Build from workspace root:

    ./tools/cargo.sh build --release -p corpse-desktop
    DISPLAY=:0 ./target/release/corpse-desktop

Linux SDL2 runtime is dynamically linked; build.rs creates a linker alias inside Cargo output when the installed library lacks its development symlink. No system paths are modified. Windows is not validated.

The menu now opens the authored Seed/Vessel form and creates a real native-generated descent. Avatar is the default; Blessed and Unblessed are available. --seed TEXT and --mode MODE prefill reproducible starts. --fixture PATH explicitly opens a controlled reference-exported migration scene instead. Coverage remains incomplete and is labeled on every gameplay view. Fixture and loaded sessions do not enter ranked scores. Unsupported fixtures are rejected by the native core rather than silently dropping behavior. SDL receives keyboard/window events; no JavaScript runtime, browser engine or webview participates.

Inputs:
- Enter: open new descent / submit Seed and Vessel; Escape: menu / quit
- Start form: Tab focuses Seed/Vessel, Left/Right change Vessel, F2 rerolls seed, Ctrl+A clears Seed
- Arrows, WASD or vi HJKL: move / melee; YUBN or keypad: diagonals
- Period, Space or keypad 5: wait
- I: Pack; number: choose an ability, or equip while Pack is open; Shift+number: use item
- Ability targets: click an observed map cell; Escape cancels; PgUp/PgDn scroll abilities
- Toolbar: Wait, Rest, Down, Up, Pack, Inspect, History, Bestiary, Settings, Menu
- R: Revert; F: See The Future (core may reject unavailable behavior)
- F5: save to native-save.json; F9: reload; --save PATH overrides
- M: all 81 illustrated bestiary entries; Left/Right: creatures, PgUp/PgDn: depths
- Tab: roster category; Up/Down or mouse wheel: full entry text; click roster to select
- V: deliberately unsupported command, demonstrating transactional rejection

Test-only output:

    ./target/release/corpse-desktop --check-fixture
    ./target/release/corpse-desktop --dump-observation observation.json
    ./target/release/corpse-desktop --render-test menu.png
    ./target/release/corpse-desktop --render-test manual.png --manual

Headless output verifies rasterization but is not evidence of a running native window. The real-window QA captures must launch SDL with a connected display and send actual input.

The renderer accepts only desktop View; read-only observation adapters apply source knowledge rules before producing detached UI data. Terrain remembered without current visibility uses last-seen snapshots. Hidden actors/items are omitted. Rendering cannot change game state or RNG. Glyph atlas masks and UI glyphs are cached; frame animation is cosmetic only. See ../../docs/desktop-complexity.md for asymptotic/native costs and coverage limitations.

Assets retain their existing notices in assets/. UI uses original IBM Plex Mono outlines (lossless WOFF2 to sfnt decompression), with Inconsolata and the reference's Noto subsets for glyph fallback. Map ASCII uses the actual Brogue CE bitmap atlas. Illustrated creatures are not claims of mechanically implemented creatures.

Headless generated map QA: --render-game generated.png --generated --seed 00-09-10:native-parity-v1 --mode Avatar. This still does not replace real-window capture.

Replay and profile surfaces:
- F6 exports the accepted command ledger for an explicitly generated session into replays/.
- View Replay browses replays/; --replay-dir PATH changes the folder. --replay PATH loads directly; JSON files may be dropped on the replay browser.
- Replay: Space/Right steps, P plays/pauses, R restarts, Escape returns to menu. Playback uses its own unranked native engine.
- High Scores reads profile.json or --profile PATH. Existing source records are displayed, not fabricated. Drop a profile JSON on the scores screen to read it. No incomplete native sessions are archived as ranked scores.
- Chronicle: mouse wheel over the top log scrolls grouped recent events. Full history keeps every event. Inspect opens known creature summaries; Up/Down or wheel scrolls, Escape closes.

- Shift+Z or Rest repeats explicit Wait commands until the source stop conditions change; any key cancels.
- Entering a known hazard opens a Y/N confirmation before dispatch.
- Earned blessings open a blocking chooser: 1–3 or arrows and Enter.
- Right-click an ability for its authored rules; wheel the rail to scroll ability rows or condition meters.
- Pack has equipped/pack grouping, item details, explicit actions, targeting and separate list/detail scrolling. Unported actions explain their limitation.

Native persistence:
- F5 writes an atomic native save envelope carrying genuine/controlled session provenance outside Run. F9 or menu Resume restores it.
- Import save accepts that envelope and plain current-v48 complete JSON saves. Drop the file on the Import save screen.
- F7 explicitly exports a genuine session in canonical source JSON format. Controlled fixtures retain their native envelope.
- A move after death/victory opens the terminal reminder with hearts eaten, cumulative Blessings received, depth and reason. Enter descends again; Escape opens the menu.
- Terminal profile/replay recording is wired but the release ranking gate remains disabled until native parity/acceptance approval. Tests exercise the enabled path, deduplication, genuine resume and fixture/replay exclusions without writing a user profile.

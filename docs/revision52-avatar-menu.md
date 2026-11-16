# Revision 52 Avatar-only menu and god selection

New desktop games use Avatar. The mode-cycle button is removed from the title
screen and DEBUG setup. The title screen instead offers three colored god
buttons. Each displays the exact god name, alias, and domain supplied by the
designer, including **NUMO the Pregenitor, Goddess of Memory**. Colors are the
existing authored `ability_ui::god_color` palette, not new approximate colors.
The original heart/rib illustration and heartbeat are retained at a smaller
size above those controls.

Click a god slot or focus it with Tab/Up/Down and Enter/Space. The picker shows
all twelve gods in a three-column grid. Arrows and Tab/Shift+Tab move focus;
Enter, keypad Enter, Space, or clicking chooses. A god already in another slot
cannot be chosen and produces an explicit notice. Escape or Cancel leaves both
the seed and selections unchanged. A committed choice closes the picker;
reopening starts at that slot's current god. Repeated activation after closing
cannot commit another choice. Picker input does not click through to Descend.
The title menu retains Continue, Replay, Scores, Bestiary, Settings, DEBUG and
seed entry. Space and Enter both activate its buttons. Disabled Continue stays
skipped by keyboard traversal.

## Seeds and startup boundary

- An ordinary initial menu visit and every return select three random distinct
  gods. Choices remain stable until the next return, including while painting,
  navigating, or opening/cancelling a god picker. Players may change them as before.
  Blank seed text requests a random suffix while keeping those selected gods.
  Menu-entry randomness and new-run suffix randomness use separate OS reads,
  never the serialized simulation RNG. See [menu visits](revision52-menu-god-randomization.md).
- Entering/pasting a complete numeric-prefix seed synchronizes the three slots.
  Valid supplied full seeds retain their exact trimmed bytes and god order for
  the current visit, including the first menu reached with an explicit CLI seed.
  Later menu returns replace its god prefix while retaining the complete suffix.
  Free-form seed text is a suffix for the currently selected gods.
- Changing a slot rewrites only the god prefix, preserving the suffix including
  internal colons and UTF-8 text. Choosing the already-selected god changes no
  seed bytes. Duplicate/out-of-range numeric prefixes are rejected on launch;
  a deliberate god edit can replace the invalid prefix.
- Input and final composed seeds are bounded to 180 bytes and reject control
  characters. Invalid data does not enqueue generation or mark a session changed.
  Three valid distinct IDs are checked before the worker is started.
- All current normal and DEBUG constructors use the explicit Avatar creation
  APIs. Removed mode arguments are rejected before creation. Read-only historical
  fixture/replay paths retain recorded modes. Save import, Continue, replay
  reconstruction/restart, old constructors and existing files are not retuned.
- Generation continues to use the existing bounded asynchronous loader. Escape
  invalidates its result; repeated starts cannot enqueue a second worker, and
  leaving/reopening the form cannot commit a cancelled result.
- DEBUG keeps starting-floor selection, seed copy/paste/reroll, and F3/F4/F8
  god cycling. Returning to the main menu randomizes all three slots and rewrites
  only the full seed's god prefix, retaining its suffix.

## Complexity and portability

Let S be seed bytes (at most 180), K the fixed 12-god catalog, A the fixed starter
ability catalog scanned by the historical shell parser, G rendered glyph
coverage, and P painted drawable pixels. Title hit testing and focus traversal
are O(K) time and O(1) extra memory per input, with K fixed; picker choice checks
three IDs and performs O(S) copying. Seed changes and startup validation call the
canonical historical shell parser only for numeric-prefix seeds, O(S + A)
time and O(S + A) bounded transient storage, including starter-item/ability
construction and cold immutable-catalog initialization. No floors, visibility,
or live simulation state are queried. Seed checks do not change live RNG state.
Before each queued input and after each event batch, comparison with the last
observed seed costs O(S); the parser runs only when that value changes.
The UI retains three IDs, one optional picker, one bounded notice, and a copy of
the last observed seed, O(S) extra storage.

Drawing uses a fixed number of rectangles and text calls, O(K + P + G) work,
including existing cold font rasterization and warm glyph-cache costs. Resize
uses the shared centered 1280×800 logical transform and drawable framebuffer
allocation/upload, O(P) time/memory; hit testing uses the corresponding inverse
transform. Full-screen picker paint has no persistent image cache. Existing
menu fire, heart-layer rasterization, glyph caches, and SDL/compositor work retain
their documented renderer costs. These are analytical bounds, not measured frame
times. New-run generation and DEBUG reconstruction retain their existing core
costs; the new seed/choice boundary does not change those algorithms.

## Verification

Focused tests cover all twelve names/aliases/domains and exact button RGB fills;
36 slot-by-god distinctness combinations; pointer edges/gaps; keyboard/mouse
parity; cancel/reopen/repeated activation; full/blank/free-form/UTF-8 seeds;
bounded/invalid data; rejection of non-Avatar normal/DEBUG starts; scaled logical
geometry; and native painter captures at 1280×800, 1600×900 and 1920×1200.
The first focused pass passed 29 menu-related tests (including all nine new
Avatar tests), four loader tests, and eight Continue-filter tests. These filters
overlap; their counts are not additive. Two pre-existing opt-in heartbeat and
benchmark tests were left ignored in that focused pass. Nine headless native
CLI checks also passed: removed/missing mode rejection, default/explicit Avatar
and DEBUG creation, duplicate-god seed rejection, and synthetic legacy
Unblessed/Blessed fixture loading.

The UI-final pass using desktop compile03 repeated 29 menu, four loader and
eight Continue-filter passes, including the strengthened normal/2× input-transform
resize regression. All six new captures were visually reviewed: the subtitle/focus
border overlap is fixed, focus matches the live selection, and labels remain
readable without clipping at all three sizes. This binary predates later lake/ring
changes and is not whole-release-final verification. Its source/binary hashes,
logs, six capture hashes and review receipt are in
`verification/r52-avatar-menu/ui-final/`. The initial nine CLI cases passed;
rerunning them awaits the rebuilt ordinary application binary.

Headless painter captures verify native raster output, not an interactive Windows
runtime or a live SDL input session.
An isolated cloud Xorg launch and one scoped escalation attempt both failed
before display creation because local Unix listening sockets were unavailable;
no user computer, system settings, or software installation was used.

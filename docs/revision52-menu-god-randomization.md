# Revision 52 random god choices on main-menu visits

## Behavior

A normal initial blank/suffix-seed main-menu visit chooses three random distinct
gods, and every subsequent return chooses three again. The existing authored
names, aliases, domains and RGB colors are unchanged, including NUMO the
Pregenitor. Each of the three controls still opens its existing editable picker.
Fresh samples may match an earlier triple by chance; there is no extra exclusion
policy and no changes to keyboard bindings, layout, or Avatar-only mode policy.

The selection is UI state. A 12-byte OS entropy read supplies a fixed three-step
partial Fisher–Yates shuffle over the 12 IDs. This follows the existing desktop
new-seed sampling convention (u32 modulo remaining count) and never consumes a
Run RNG or generation RNG. Rendering and picker interaction draw no entropy.
The pre-existing seed suffix generation is unchanged and separately triggered
only when starting with blank text.

## Entry lifecycle and interruption paths

`menu_visit::Visits` observes whether the main menu is editable at input/frame
boundaries, not paint calls. Its single last-active flag prevents repeat entry
work on hover, resize, focus changes, tabbing, idle animation or repeated renders.
The observer runs before each queued SDL event and before/after loader polling.
This catches multiple transitions in one event batch as well as a worker that
finishes before the first poll. Summary keyboard/mouse New Descent and dead-game
Enter explicitly observe the menu before submitting their immediate new launch.

All screen departures use the same observer: ordinary Game Escape/toolbar,
DEBUG preparation cancellation, Summary/death, Start/debug setup Back/Cancel,
ImportSave, Replays, ReplayGame, Scores, Bestiary/Manual, and Settings. Returning
from a Bestiary opened in Game/ReplayGame returns there, without a menu reroll.
A failed replay/save load staying on its own screen rerolls only once that screen
returns to the menu. Continue and successful imports retain the existing run.

Menu-based asynchronous generation makes the menu temporarily non-editable.
Cancelled workers remain bounded and invalid until drained by the existing loader;
when cancellation or generation failure restores the editable menu, choices reroll
once. Successful generation enters Game and causes no extra menu selection.
A validation error before a worker starts is not a departure and keeps the choices.
Errors on the separate Start form keep that form; Back/Cancel then rerolls once.
Picker Escape/Cancel only dismisses the menu's overlay and does not constitute a
main-menu return.

OS entropy failure keeps the existing choices, reports the failure in the menu
notice, and does not retry every frame. The next real return may try again.

## Seed, save and replay boundaries

A full numeric-prefix CLI seed is honored exactly on initial menu entry. A first
observed Start, ReplayGame or Manual screen is not this exception: returning from
that screen randomizes the menu choices, even when a CLI full seed exists.
Entering/pasting a full seed or deliberately changing a god during a visit retains
the existing sync and validation behavior. On a later return the randomized gods
replace a numeric seed's prefix and preserve the full suffix, including internal
colons and UTF-8. Blank/free-form suffix text stays unchanged. Choosing new gods
can change geometry and loot because the full seed remains dungeon identity.

Replacing a short prefix such as `0-1-2:` with the canonical padded prefix can grow
an input from 180 to 183 bytes. The suffix is never truncated. The menu reports
that the seed must be shortened, and the unchanged 180-byte launch validation
rejects it before generation. Backspace can shorten it normally. Automatic return
selection replaces a recognized numeric prefix just as a deliberate god edit does,
including an old duplicate/out-of-range prefix; a full invalid seed typed during
the current visit is still rejected normally.

No live/saved Run, replay record, session mode, simulation revision, temporal
anchor or serialized RNG is passed to the observer. It never dispatches a command,
starts generation, resets a running game or writes a file. Continue, save import,
replay playback/restart, NUMO Revert, and Morae future state remain independent.

## Complexity

Let K=12 be the fixed god catalog, S seed bytes (input at most 180, at most 183
immediately after padded-prefix expansion), and A the bounded starter catalog
used by the existing canonical seed-sync shell parser. Every observed queued
input and pre/post-poll boundary adds O(1) time/O(1) persistent state. An actual
entry makes one 12-byte OS entropy request, creates a K-element stack array,
performs three swaps, and optionally rebuilds/validates the seed: O(K + S) time
and O(K + S) temporary storage. OS entropy service latency is platform-dependent,
not asserted to have a measured constant wall-clock bound. There is no rejection
loop, allocation proportional to a world, cache, sorting or pathfinding.

The existing changed-seed comparison remains O(S) per input/batch. A rewritten
full prefix invokes the existing O(S + A) canonical shell parser at the following
seed-sync boundary, including starter-item/ability construction and cold immutable
catalog initialization, O(S + A) transient storage. No floor or visibility work
is added. Existing renderer, glyph/font-cache miss, framebuffer, texture upload,
SDL/compositor and new-run generation costs remain unchanged. The bounds above
are analytical; no platform timing or frame-rate claim is made.

## Verification scope

`menu_visit_tests.rs` adds eight deterministic tests covering:

- all 1,320 ordered distinct god triples and extreme u32 samples;
- repeated returns from every Screen variant, without duplicate entry work;
- stable manual edits and picker cancellation/reopen across repeated observations;
- exact initial CLI full seed, later prefix rewrite, and CLI non-menu starts;
- busy cancellation/error/instant-poll recovery and direct New Descent preparation;
- one visible OS-random failure without per-frame retry;
- the 180-to-183-byte prefix edge, intact suffix and blocked launch until shortened;
- actual save/reload, gameplay RNG, commands, replay playback and replay restart
  parity while interleaving menu visits.

These tests exercise the same observer/helpers used by the native main loop.
They do not claim live OS keyboard/mouse delivery, interactive platform coverage,
or a whole-workspace pass. Verification results are recorded separately once run.

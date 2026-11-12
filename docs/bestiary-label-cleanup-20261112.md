# Bestiary labels and markers, 2026-11-12

The owner was shown ALL CREATURES, FIRST APPEARANCE, PREV / NEXT, the page counter, unseen ? and unread ! markers and answered "Remove all of that" on November 11. This applies that exact scope to the older R51 review branch.

Removed the two labels, page counter, PREV/NEXT words, unseen list/detail question marks and unread exclamation rendering. Previous/next arrow indicators, borders, buttons, hitboxes and input behavior remain. Unlock guards, discovery/unread persistence, natural history and page content remain. The separate gameplay toolbar question mark is untouched.

Two existing presentation tests now assert discovery changes the visible page but marking an already known page read does not change pixels; unread state still updates. Rust syntax parsing, exact diff, original blob hash and unchanged non-rendering code checks passed. Native tests were NOT RUN: Cargo/toolchain unavailable. No runtime/visual claim. Main is not modified or merged; no whole R52 file was substituted.

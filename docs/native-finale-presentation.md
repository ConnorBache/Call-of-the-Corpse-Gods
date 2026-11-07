# Native finale presentation

## Terminal and archive boundary (October 25, 2026)

`RunSummary.ending` is an optional, detached string populated only by
`finale::ending(run)`. The escaped-collapse victory is displayed verbatim as
`Collapsed the flesh!`, replacing both legacy bottom-reaching labels for that
result. Collapse death keeps the exact recorded death reason,
`Fell while escaping the collapse of the flesh.` The renderer never derives
victory from floor depth and never mutates gameplay state.

Archives and ranked records preserve the optional ending through serialization,
parsing, and score sorting. Missing endings default to `None` and are omitted
when encoded, leaving revision 29/30 summary and archive shapes unchanged. The
existing legacy victory text remains the fallback for old records. Archive
details show an ending when no death reason is present. Ending strings use the
same nonempty, 512 UTF-16-unit validation bound as death reasons. This is an
additive profile field, not a profile version migration or a ranking change.

### Complexity and bounds

Let K be the number of distinct recorded heart kinds, R the number of archived
runs, L the ending's UTF-8 length, and B the serialized profile size. Selecting
the optional finale label is O(1) time and extra memory; summary construction
adds O(L) time and owned string memory. The known authored ending has constant
length. Existing heart sorting remains O(K log K) comparisons with UTF-16
string-comparison costs, independent of the world-floor count. The existing
archive clone, score sorting, and full-profile encode/decode remain in place;
retaining endings adds O(R × L) string copying/storage to the existing record
copies, and serialization/parsing is O(B) plus existing record validation and
ranking work. Per-record ending validation scans at most the supplied string
length to enforce the 512 UTF-16-unit limit. No filesystem reads or gameplay
queries occur while painting.

Terminal heading/reason selection adds O(1) borrowed-string work per paint. The
existing depth/reason formatting and line wrapping use O(L) temporary text for
the authored bounded labels. Pixel rasterization, native glyph-cache misses,
framebuffer clearing and upload retain the costs documented in
`native-resolution-rendering.md`; painting is not claimed O(1) in native pixel
area. Archive detail painting uses the same existing text wrapping and glyph
rendering path, without a new cache, layout pass, or simulation step.

### Verification scope

Core profile tests cover exact victory/death strings, escaped-state summary
projection, detached construction, archive roundtrip and ranked preservation,
invalid ending validation, explicit revision 29/30 omission, and the unchanged
frozen source-parity fixture. Desktop tests cover exact new and legacy labels,
read-only painting, and the new heading's rendered pixels. These are native
schema and software-painter checks, not a claim of Windows display verification
or full boss/escape gameplay coverage; those mechanics are tested separately.
Set `CORPSE_FINALE_UI_REVIEW` to an output directory while running the desktop
`terminal_ui` tests to export the victory and collapse-loss review frames.

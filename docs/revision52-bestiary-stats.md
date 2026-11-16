# Revision 52 Bestiary stat presentation

The Bestiary has no dungeon-floor selector. Its `depth` is the selected page's
first-appearance color, not the current run floor. Unlocked pages now show
minimum–maximum base-stat values across authoritative birth floors, with a
single value when constant. The header remains **Base stats**. Numeric text is
measured against its existing cell and uses a fitting font size; no selector,
explanatory paragraph or change to the seven stat labels is added.

## Sources and compatibility

- Exact R52 natural profile keys supply their own floors and apply the same
  `balance52::apply_new_actor_kind` overlay after the public revisioned birth
  constructor. Nest, Queen, Witch and Birther offspring are not mixed into
  natural species ranges. There is no duplicate balance table.
- The Nest-only Queen uses its real revision-33 base plus the R52 Nest profile,
  matching `pocket_biomes::populate_revision`. Before that species existed, its
  page cannot invent numbers. Unsupported or character-dependent templates
  show `Varies`.
- Corpse Fungus uses the actual per-floor template, but its eligible floors
  come from revisioned setup admission (including executable fungus stencil24,
  whose metadata has no `kinds`) and, from R51 onward, both feast victim
  pools. Current floors 37–40 cannot supply the required +2/+3 introduction,
  so the current HP range is 154–574, not the obsolete 154–622. The existing
  special-depth line is corrected to 1–36 when rendering that page. Geometry,
  terrain and eligible victims still make each placement conditional. No
  Godbeast is admitted as a feast victim.
- Zillennia quality floors come from its real per-depth catalog and use the
  revisioned constructor. Sphinx chamber guardians intentionally retain the
  frozen revision-29 birth constructor, as startup does. Ichor event species
  use their authoritative eligible-fish floors and are checked against real
  event births. Other unprofiled natural species use revisioned ordinary
  recipe membership; fixed special templates retain their authored source
  floors without inventing a new profile.
- The originating active game or replay supplies only its simulation revision.
  Inactive/menu views and absent runs default to current revision 52. Missing
  revisions on legacy active runs use their existing revision-29 convention.
  No dungeon, actor, hidden creature, RNG or saved gameplay state is read to
  build the stat rows, and no existing actor is retuned.

HP is actual unwounded `max_hp`, including the native `100 + 6*Defense` rule.
Damage is the species' nominal melee strike before variance, DR and abilities:
its own weapon base plus its own Offense coefficient, multiplied by 0.5. R51+
use nearest-integer rounding and older revisions retain ceiling rounding.
Other legacy fractional stats display at most two decimal places; simulation
values are untouched.
Energy, Accuracy, Dodge, DR and Perception retain their native display units.
This does not change player bonuses or combat arithmetic. Current saved healing
and special-damage overrides have no stale fixed numeric claims in the visible
Dryad/Valta ability prose; authored costs and other abilities remain unchanged.

## Privacy and storage

The unknown-page guard returns before stat construction, art or content layout.
No new page is unlocked by displaying a range or changing revisions. Existing
profile-level seen/unread storage, atomic writes and the Chronicle discovery
notification are unchanged. Tests compare locked pixels across revisions and
secret-content changes, and verify persisted discovery bytes do not change
when stats are rendered. Viewing a discovered page still clears only its
unread flag.

## Complexity

These are source-derived bounds, not measured platform timings. Let D <= 40 be
admitted floors, S = 7 displayed stats, R = 550 R52 rows, T all historical
balance/identity/trait rows, A a complete cloned actor payload, J maximum kind
identifier bytes, Q candidate recipes across queried floors, M <= 5 recipe
members, K ordinary species, F setup definitions and B encoded catalog bytes.

- R52 floor selection scans O(R*J). Each birth reconstruction clones O(A),
  scans the existing O(T*J) overlays and O(R*J) current profile, then reduces
  S values. Total warm stat construction is O(R*J + D*(A + (T+R)*J + S)) time,
  with O(A + D + S) scratch and output. No per-actor retained cache is added.
- Legacy/unprofiled ordinary floor selection scans recipe members over at most
  40 floors: O(Q*M*J), plus existing species/pair lookup. A cold recipe cache
  pays its construction and retained memory costs, documented in
  `native-monster-depth-windows.md` and `flavor-placement-v32.md`; this is not
  an O(1) helper. Ichor floor membership scans its fixed nine species per floor.
  Zillennia scans 40 existing templates. Fixed special metadata sorting is
  O(D log D), with O(D) scratch.
- Fungus setup-floor admission is O(D*F*M*J) plus the setup helper's flavor
  lookup for member-bearing definitions. For R51 and R52 separately, one cold
  `OnceLock` build additionally invokes real victim-pool admission per eligible
  floor: O(D*C*M*(Qp+K+Tp)*J), where C is the recipes over three target floors,
  Qp is the pair count and Tp is flavor rows. Cold recipe construction is
  additive. See `revision51-fungus-feasts.md` for transitive bounds. Each
  immutable result retains at most 40 floor integers; warm reads clone O(D).
  No geometry, LOS, pathfinding, floor or live population is generated here.
- Public JSON catalogs retain their existing one-time O(B) parse/storage cost.
  Stat formatting is O(S*L), for L formatted characters. Fitting tests at most
  13 font sizes per value, each O(L) native font-metric queries. Painting adds
  glyph raster/cache-miss work and blending proportional to glyph pixel area;
  those existing native renderer costs are not constant-time string writes.
- The active-run handoff reads one revision scalar in O(1). Unchanged hidden
  pages bypass all new stat/profile work. The static Bestiary redraw schedule,
  artwork decode/trim and full-frame upload behavior remain unchanged.

To profile, compare first opening of Fungus and a legacy natural page against
warm selection, and separately measure fixed/ranged text at portrait and wide
sizes. No Windows runtime or frame-time claim follows from native CPU captures.

## Verification receipts

Focused and existing-suite logs, source hashes, exit receipts and native CPU
PNG captures are under `verification/r52-bestiary-stats-01`. Focused tests cover
production population births, revisioned rounding, all quality templates,
actual Nest Queen/event fish constructors, natural/context isolation, stale
appearance metadata, fungus omission, unknown-page pixels, persisted discovery,
active/inactive/replay revision routing and every page's stat width.

The release owner must run the merged whole-workspace gate after integration.
Native CPU rendering is not a Windows application launch or Windows runtime QA.

Final focused run: 15 passed, zero failed. Existing manual/discovery/stat/routing
suite: 56 passed, zero failed, one pre-existing ignored timing benchmark.
Sixteen PNGs were produced; five representative full-size captures were visually
inspected. The first focused run found and fixed setup24 metadata admission
and repeating legacy-fraction formatting before the successful rerun.


## Source reconstruction, 2026-11-08

The missing `stat_presentation::enemy_damage_revision` and `main.rs` revision
handoff were functionally reconstructed from this document and their recovered
callers. The original source bytes were not recovered. Legacy ceiling and R51+
nearest rounding use existing combat helpers; only the originating game/replay
revision crosses the display boundary. Existing Bestiary catalog, discovery and
rendering code is retained. See [the reconstruction record](revision52-desktop-source-restoration.md)
for provenance, current checks and unchanged complexity. Historical receipts
above are not proof of byte-exact restoration or final merged release validation.

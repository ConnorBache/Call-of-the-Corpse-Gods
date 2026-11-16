# Bestiary nearby-effect ranges

The designer requested exact ranges on full Bestiary pages, not the creature
preview. `manual_ranges::page_text` replaces only the body prose handed to the
existing Bestiary typesetter. It does not mutate `manual.json`, the mechanics'
names/costs, natural history, appearance, forms, artwork, page theme, navigation,
statistics, discovery, or either string returned by `creature_abilities_ui::describe`.
The `Manual::preview_all` flag is the existing discovery-unlock test/developer
flag, not the brief in-game creature preview.

## Complete audited inventory

Seven effect families explicitly say “nearby” in the full catalog: Fungal gas,
Ruin Pulse, Mouth of Silence, Darkness Aura, Reservoir Explosion, Stone Seal and
Scream. Healing Grove uses “nearby” in the enemy-trait name, although the manual
body says “within sight.” The same clarification includes the remaining hazardous
proximity effects Razorleaves and Winter's Trail, and Banshee Shriek because it has
no radius and must not imply that merely backing away makes the player safe.
There are 11 effect families, 40 mechanic replacements and 20 affected pages:

- Corpse Fungus, Dryad, Bombardier Beetle, Devil's Samara Tree, Yeti, Banshee.
- Pentacorn and Heptacorn.
- Pale Screamer and Greater Pale Screamer.
- All ten Zillennia qualities, each with Ruin, Silence and Darkness.

Targeted bolts, jets, breath cones, curses and teleports are not described as
nearby area effects and are not expanded in this change. Summons, grabs and slime
trails already identify adjacency; they are not converted into circular auras.
The previously explicit Skin Walker two-square disguise range is unchanged.

## Geometry vocabulary

All circular upper bounds are inclusive. A straight-line range R admits a tile
with dx²+dy² <= R², not the entire (2R+1)-square box. “Spatial shortcuts” means the
existing `portals::actor_distance` metric: sums of Euclidean segments with zero-cost
portal, non-Euclidean and active Speed-up links. This metric does not itself check
walls. Same-plane gating and optical line of sight are separate predicates.
`vision::line_of_sight` checks a Bresenham ray and may traverse ordinary portals;
it does not impose a Perception/detection range. Interior walls, trees and
`environment::blocks_sight` occluders block its segments. An opaque endpoint itself
is visible. Direct-radius effects do not shorten their numeric distance through
spatial links, even where the separate LOS test can use a portal.

### Ruin Pulse: up to 3 straight-line tiles, with spatial shortcuts

`monster_ruin::special_action` requires the selected target within 3, zero
`beastCooldown`, and payment of 20 Energy. It then damages every other living
endpoint within 3 with LOS, including the caster's allies. `actor_distance` rejects
other planes. It snapshots the caster, so source movement/death during resolution
does not relocate the burst. Successful casts set `beastCooldown` to 3; that counter
is decremented by `ai::enemy_turn_effects` on normal world ticks, not render time.
The normal AI entry additionally requires a live, awake, non-paralyzed caster, a
hostile live same-plane target, no invisible target without telepathy, and LOS.
Silence/payment, Oblivious, Fated Duel and ordinary damage admission remain intact.

### Mouth of Silence: up to 3 straight-line tiles, with spatial shortcuts

`abilities::energy_activation_silenced` and `monster_horn::spell_spend` check the
actual current source/target positions at activation. The source must be living,
other than the target, on the same plane and on the opposing player/allied side;
LOS is required. Oblivious protection excludes its blocked source. There is no
sleep/paralysis suppression of the passive aura. `monster_horn::world_turn`
refreshes the visible `auraSilenced` condition each world turn, but the actual
payment gate is live geometry, not that old condition. Existing effects/upkeep
are not retroactively canceled by entry. Independent ordinary Silence still works.

### Darkness Aura: orthogonal distance strictly below 10

`vision::illumination_in_plane` subtracts max(100 - 10*(abs(dx)+abs(dy)), 0) from a living
same-plane source after the existing light computation. Thus an integer-cell
orthogonal distance of 9 still subtracts 10; distance 10 subtracts zero. A diagonal
step counts 2. It uses `floor_ray` directly, with no portal/shortcut routing. Walls,
trees and other sight blockers occlude interior ray cells. There is no energy,
action or Perception requirement. Multiple sources subtract before the final
light clamp. Personal darkness/light rules in `vision::light` are unchanged.

### Healing Grove: up to 6 straight-line tiles, with spatial shortcuts

`monster_spells::heal_others` scans other live, injured, non-hostile same-plane
actors within 6 with LOS, excluding healing-suppressed recipients. It selects the
lowest HP/max-HP ratio, breaking ties by actor ID, pays 5 Energy through the spell
gate, heals the recipient owner, and sets the next valid world tick to tick+2.
Self-healing is excluded. A Dryad need not detect an enemy to perform this scan;
the dedicated-caster path can heal without an observable attack target. The
amount is birth-profile dependent, so the page does not invent a fixed HP amount.

### Scream: up to 10 straight-line tiles for debuff/buffs; whole-floor call

`ai::enemy_action_options` spends 15 Energy after its normal cast-target admission.
For the resulting effect, `actor_distance <= 10` applies the five-turn Disoriented
penalty to foes and five-turn Damage/Perception boosts to allies. There is no
recipient LOS test: walls do not shelter recipients. Recipients must be living
and on the same plane; decoys and Oblivious-blocked recipients are skipped.
Current control/Discord-aware hostility chooses which packet they receive.
The ordinary/greater Disoriented penalty is 5/10 Accuracy. The same-side wake and
last-seen update is outside the radius predicate: eligible same-side non-player,
non-allied monsters throughout the floor hear the call, even beyond 10. The
caster still needs an admissible hostile target with LOS to choose the action.
The page distinguishes the effect radius from this whole-floor call.

### Reservoir Explosion: direct radius 5

`monster_bombardier::special_action` admits a target at raw Euclidean distance <=5,
requires positive Energy, and spends the entire reservoir. `environment::explode`
receives radius 5, `ignite_first=true`, and the retained source. Direct victims
and ignition cells also use the raw radius and LOS; victims are on the source or
current environment plane. No metric shortcut extends the radius. Explosion
immunity protects the Bombardier and other immune targets. Ignition precedes
victim resolution, so newly ignited obstacles or gas reactions can affect the
outcome. Subsequent fire/gas propagation is not constrained to this initial disk;
standing outside 5 is not a promise of safety from secondary hazards.

### Stone Seal: target up to 5 straight-line tiles; placement in 8-cell ring

`monster_horn::special_action` uses spatial distance <=5, zero `beastCooldown`, no
existing unexpired raised wall and normal spell/target permission. The normal AI
path adds LOS; `permitted` enforces same plane, invisibility/telepathy and target
protections. It gathers the target's eight raw adjacent squares, excluding the
center. A candidate must be free traversable interior floor, not occupied by a
living creature on the relevant plane or blocked by environment. At least three
candidates are required; one is randomly selected. Costs 15 Energy; the wall
expires at tick+5 on its creation plane and the cooldown is 3 world ticks. The
placement ring is not itself extended by spatial-distance shortcuts.

### Fungal gas / roots: direct radius 10 for acquisition; gas has no fixed radius

`fungi::tick_fungi`/`tick_growing_fungi` acquire a dead corpse at raw Euclidean
distance <=10 with LOS. The recorded root is the direct Bresenham path, not a
pathfinder or portal route. Acquisition rejects any frozen/burning path cell and
any previously recorded source/corpse pair, including severed history. Waking
normal environment ticks run the fungus system; the dream environment phase
skips it. The acquisition actor lists have no additional per-actor plane filter,
so this audit does not invent one. Existing valid roots are retained by canonical
path/material/source/corpse checks rather than a fresh LOS test each tick.

R52 new roots begin at the source and advance one path cell after three eligible
normal ticks. Haste does not grow them; offscreen exact normal ticks do. Fire ahead
or frozen prefixes pause growth; fire/cuts on grown material sever it. Only a
completed intact link stops revival and increases gas emission. Legacy roots can
already be grown, and older revisions use their old acquisition timing; the
page's range prose therefore does not impose R52 growth cadence on old saves.
Gas is deposited at the fungus with volume 25*(1+completed links) and follows
the existing diffusion/reactivity rules. Its footprint is not a radius-10 aura.
See [incremental roots](revision52-root-growth.md) for full lifecycle semantics.

### Razorleaves: direct radius 5, normal turn-end contact

`environment::refresh_razorleaves` paints a raw Euclidean disk <=5 around living
current-plane leaf sources, excluding wall destinations and requiring LOS. The
normal environment tick's living/contact-plane actor list supplies the sources
and recipients; startup initialization retains the same local disk. During
normal turn-end environment resolution, a creature on a leaf cell takes 2% of
maximum HP through the ordinary environmental damage protections. Possessing
Razorleaves exempts it. Flight is not an exemption. The radius does not use
spatial-distance shortcuts; the LOS predicate retains its normal portal rules.

### Winter's Trail: distance <= sqrt(2), including spatial shortcuts

`monster_chill::apply_adjacency` checks living Half Here player origins against
living, non-clone same-plane Yetis. This is the eight surrounding squares on an
ordinary grid; diagonals count. No LOS test is made, so adjacent corner walls
are not protective. ICE_IMMUNITY protects each body independently. Slowed is
applied at the beginning of the shared player turn/action stream (not a visual
frame); hitting an unprotected Half Here body can slow its owner across planes.
Status-source, Oblivious and ordinary admission checks remain in effect.

### Banshee Shriek: no numeric range cap

`lifecycle::world_turn` checks every 30 normal ticks from initial floor entry or
revival, once per tick, for a living Banshee and living player in an active game.
It marks the audible location for one tick regardless of whether damage lands.
Damage is aimed at the player, requires LOS, and is rejected across planes by
`engine::damage_environment_core`. There is no distance/Perception cutoff in the
shriek itself. Its damage continues through the existing armor/protection rules.
Moving farther along a clear corridor alone does not leave its reach.

## Rendering and compatibility boundary

Only the full-page mechanics body uses the replacement prose. All headings and
cost labels remain authored. Original catalog objects remain intact and the
preview adapter is not changed. Full-page locked-entry admission precedes layout,
so range copy cannot reveal undiscovered pages. The engine, deterministic RNG,
normal/haste scheduling, offscreen simulation, save format and both time-travel
paths are unchanged. Actual range behavior is tested through public engine APIs,
not duplicated geometry formulas in the description layer.

## Complexity

The adapter is a fixed match on the current entry ID and mechanic name. For M
mechanics and L maximum ID/name length, classification is O(M*L) per text-layout
attempt and O(1) additional retained memory. Returned text is borrowed static
prose; the existing layout owns a copied String. Across at most five font-size
attempts for each existing bounded layout candidate, added layout cost is O(C)
font-metric/wrapping work and O(C) temporary text/line memory for the selected
page's C prose bytes. Existing art decoding, font-cache misses, glyph raster work,
framebuffer scaling/upload and stat queries retain their costs. No added
simulation work, actor scans, path queries, visibility calls, disk IO or RNG is
performed by the new UI adapter. These are source-derived bounds, not measured
latency claims.

## Verification

Focused filter: `bestiary_effect_ranges_` in `corpse-desktop`.
The suite inventories all catalog “nearby” strings, asserts the 40 replacements
and 20 pages, checks all affected content fits without losing lore or mechanics,
and checks the preview stays separate and Run bytes are untouched. Executable
boundary tests exercise inclusive circular edges, diagonal exclusions, walls,
planes, portal vs direct distance, Darkness's strict Manhattan boundary, Dryad
cooldown, Scream's whole-floor call, root growth and Banshee cadence.

`CORPSE_BESTIARY_RANGE_REVIEW=verification/bestiary-effect-ranges/captures`
exports 60 native Painter PNGs: native 1280x800, wide 1920x1080 and portrait
900x1200 for every one of the 20 affected pages. Layout checks use all three
aspects too. Both brief/detail preview strings are compared byte-for-byte before
and after full-page painting, and the unchanged preview adapter/catalog files
have SHA256 baseline checks. These are native software renderer outputs, not
screenshots from a Windows runtime. Focused results, direct visual review and
aggregate-gate scope are recorded separately after execution.

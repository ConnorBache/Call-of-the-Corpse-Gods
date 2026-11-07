# Revision46 Shine emission suppression

## Runtime contract

From simulation revision46, activating Shine suppresses that actor's entire
Shine source. It supplies no illumination to itself, terrain or other actors,
and no appearance source or footprint. Deactivation resumes the original
100-light source with the existing 10-per-square falloff and ray rules.
Ambient light, fixed sources, fire, other unsuppressed Shine carriers, darkness
and target-specific light modifiers retain their existing ordering and effects.
The accuracy bonus still reads the target's actual illumination even while the
attacker's Shine is suppressed. Toggle remains free, with no upkeep.

Missing revision and every revision through45 retain the former behavior:
active Shine emits normally but is excluded from its owner's actor-light query.
No historical catalog, saved state or replay is rewritten.

## Call chain and ownership

`vision::emits_shine` supplies one revision-aware source predicate to:

- `vision::illumination_in_plane`: scalar tile/actor lighting, AI sight and
  engine attack accuracy. Desktop strike previews use the same `vision::light`.
- `lighting_cache::Dependencies::capture`: cached tile fields, terrain visibility,
  projected actor visibility, resource observations and terrain observation.
- `vision::observe_terrain`: knowledge-filtered source collection before origin
  visibility checks, emission grouping, footprints or appearance coloring.

Existing body/dead/plane filters stay in those collectors. Half Here copies
owner powers and active flags at synchronization/dispatch boundaries, so both
endpoints suppress together. Dreamwalking sleeping bodies never emit; dream
sources remain confined to their plane. Offscreen simulation uses cloned Run
proxies with the recorded revision and fresh derived caches. Existing environment
admission still rejects independent Shine carriers on inactive floors, despite
the outer offscreen trait allowlist naming Shine; this change does not broaden
that admission. Read-only loaded-floor light queries remain revision-aware.
Portal visibility
still reads the selected plane's corrected light; light itself gains no new
portal propagation. No hidden actor coordinates or source footprint are exposed.

Cache keys contain effective emitter contributions. A same-tick toggle, removal
or revision45/46 transition changes dependencies whenever illumination changes.
Suppressed actor movement or identity edits need not rebuild the field. Cache
clone/load/snapshot rules remain unchanged; caches are neither serialized nor
shared as mutable gameplay state. Future/Weave/replay use the saved active list
and revision. Legacy Revert is unavailable at revision46 as before this change.

## Complexity and timing boundaries

Let V be cells, A actors, B the maximum ability/active-list length, L fixed
lights, S effective positive sources, D darkness sources, R ray length and Q_s
a source's bounded footprint. The predicate is O(B) time and O(1) scratch,
without allocation or owner lookup. It replaces the existing ability check and
adds an active-list scan only at revision46. The scalar source pass remains
O(A + nearby-A*B + nearby-emitter*R), besides unchanged fire/fixed-light/floor
work. Suppressed sources perform no ray or footprint work.

Each immutable cache pass still pays O(V + A*B + L) exact dependency validation.
Cold builds remain O(V + S + sum(Q_s*R) + D*V*R) time and O(V+S+D) storage;
fields are bounded to two planes. Warm queries after capture are O(1). At
revision46 no active-Shine exclusion arrays are needed, while <=45 retains
its exact top-two-source representation. Observation still includes visibility,
portal geometry, knowledge filtering and native painting costs documented in
[the lighting contract](LIGHTING-CACHE-CONTRACT.md) and
[visibility performance](VISIBILITY-PAINT-PERFORMANCE.md); this is not a broad
lighting or rendering rewrite. Desktop status wording adds O(1) revision
selection to existing bounded text construction, without querying hidden actors.

`shine_revision46::bounded_shine_lighting_timings` is opt-in with
`CORPSE_SHINE_TIMING=1`. It uses a fixed 225-cell room and 33 Shine carriers:
3,600 scalar tile reads, 16 cold field builds including Run clone cost, 128 warm
field validations, and 8 full observations for each revision45/46 inactive/
active combination. It reports elapsed durations without timing assertions;
correctness and state equality are checked outside timing. These are diagnostic
microbenchmarks, not production frame-rate or percentage claims. Execution
results belong to the integration gate; no timings were claimed while authoring.

## Deterministic coverage

`crates/corpse-core/tests/shine_revision46.rs` checks legacy/missing revisions,
self/terrain/other actors, mixed source overlap and darkness, bitwise scalar/cache
agreement, same-tick cache reuse/invalidation, source removal/death/body state,
dream planes, hidden suppressed emitters with portal visibility, free zero-energy
normal/haste toggles and shared Half Here endpoints, actual melee/ranged accuracy
against externally lit targets, unchanged inactive-floor admission refusal and
read-only loaded-floor lighting,
serialization/clone/Snapshot/Future/Weave/replay/restart. A bounded timing test
is dormant unless explicitly enabled. Existing source differential fixtures
continue to own legacy exact-state coverage.

`creature_abilities_ui` tests revision-aware status wording and read-only state.
The main ability description adapter is documented separately in
[revision46 wording](revision46-wording.md).

Initial focused execution passed 10 of 11 tests. The proposed independent
Shine offscreen catch-up scene failed the pre-existing environment admission,
before catch-up ran. The fixture now asserts that exact refusal and no mutation
at both revisions45/46, then separately checks read-only loaded-floor lighting.
This is an explicit unsupported-execution boundary, not offscreen parity evidence.
The initial integration log is retained by the parent gate.

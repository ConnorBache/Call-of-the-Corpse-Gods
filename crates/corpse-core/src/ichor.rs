//! Mercuria's finite flood frontier and event-only nine-creature fish roster.
use crate::{Actor, Field, Floor, Run, Stats};
fn solid(tile: &str) -> bool {
    ["wall", "tree", "chasm", "pit"].contains(&tile)
}
pub fn tick_ichor(f: &mut Floor) { tick_ichor_revision(f, 29.); }
/// O(V) initialization, then O(frontier) spread; serialized frontier and RNG are unchanged.
pub fn tick_ichor_revision(f: &mut Floor, revision: f64) {
    let solid = |tile: &str| solid(tile) || (revision >= 51. && crate::revision51_stairs::tile(tile));
    let w = f.width();
    let h = f.height();
    let n = f.tiles.len();
    let Some(e) = f.godbeast_event.as_mut() else {
        return;
    };
    if e.kind != "Mercuria" || !e.activated.value_or(false) {
        return;
    }
    e.flood_age = (e.flood_age.value_or(0.) + 1.).into();
    let ichor = f.ichor.get_or_insert_with(|| vec![0.; n]);
    if e.flood_visited.as_ref().is_none() || e.flood_frontier.as_ref().is_none() {
        let mut seen = vec![0.; n];
        let mut frontier = vec![];
        for i in 0..n {
            if revision >= 51. && crate::revision51_stairs::tile(&f.tiles[i]) { ichor[i] = 0.; }
            if ichor[i] != 0. {
                seen[i] = 2.;
                frontier.push(i as f64);
            }
        }
        if frontier.is_empty() {
            let i = e.y as usize * w + e.x as usize;
            if revision >= 51. && crate::revision51_stairs::tile(&f.tiles[i]) { return; }
            seen[i] = 2.;
            frontier.push(i as f64);
            ichor[i] = 100.;
        }
        e.flood_visited = seen.into();
        e.flood_frontier = frontier.into();
    }
    let mut next = vec![];
    for &cell in e.flood_frontier.as_ref().unwrap() {
        let i = cell as usize;
        e.flood_visited.as_mut().unwrap()[i] = 1.;
        if ichor[i] == 0. || solid(&f.tiles[i]) {
            if revision >= 51. && crate::revision51_stairs::tile(&f.tiles[i]) { ichor[i] = 0.; }
            continue;
        }
        let x = i % w;
        let y = i / w;
        for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
            let xx = x as isize + dx;
            let yy = y as isize + dy;
            if xx < 1 || xx > w as isize - 2 || yy < 1 || yy > h as isize - 2 {
                continue;
            }
            let j = yy as usize * w + xx as usize;
            if e.flood_visited.as_ref().unwrap()[j] != 0. || solid(&f.tiles[j]) {
                continue;
            }
            e.flood_visited.as_mut().unwrap()[j] = 2.;
            ichor[j] = 100.;
            if f.tiles[j] == "water" || f.tiles[j] == "shallow-water" {
                f.tiles[j] = "floor".into()
            }
            next.push(j as f64);
        }
    }
    if !next.is_empty() {
        f.dirty = true;
    }
    e.flood_frontier = next.into();
}
pub fn enqueue_opening(f: &mut Floor, i: usize) { enqueue_opening_revision(f, i, 29.); }
pub fn enqueue_opening_revision(f: &mut Floor, i: usize, revision: f64) {
    let solid = |tile: &str| solid(tile) || (revision >= 51. && crate::revision51_stairs::tile(tile));
    let w = f.width();
    let h = f.height();
    let Some(e) = f.godbeast_event.as_mut() else {
        return;
    };
    if e.kind != "Mercuria"
        || !e.activated.value_or(false)
        || e.flood_visited.as_ref().is_none()
        || e.flood_frontier.as_ref().is_none()
        || f.ichor.as_ref().is_none()
    {
        return;
    }
    let x = i % w;
    let y = i / w;
    if x < 1
        || x > w - 2
        || y < 1
        || y > h - 2
        || solid(&f.tiles[i])
        || e.flood_visited.as_ref().unwrap()[i] == 2.
    {
        return;
    }
    let ichor = f.ichor.as_mut().unwrap();
    if ichor[i] == 0.
        && ![i - w, i - 1, i + 1, i + w]
            .iter()
            .any(|&j| ichor[j] > 0. && !solid(&f.tiles[j]))
    {
        return;
    }
    ichor[i] = 100.;
    if f.tiles[i] == "water" || f.tiles[i] == "shallow-water" {
        f.tiles[i] = "floor".into();
    }
    e.flood_visited.as_mut().unwrap()[i] = 2.;
    e.flood_frontier.as_mut().unwrap().push(i as f64);
    f.dirty = true;
}
struct Fish {
    kind: &'static str,
    name: &'static str,
    first: f64,
    last: f64,
    defense: f64,
    offense: f64,
    dr: f64,
    glyph: &'static str,
    color: &'static str,
}
const FISH: [Fish; 9] = [
    Fish {
        kind: "lesser-ichor-fish",
        name: "Lesser Ichor-Fish",
        first: 1.,
        last: 6.,
        defense: -12.,
        offense: 7.,
        dr: 0.,
        glyph: "f",
        color: "#b99a48",
    },
    Fish {
        kind: "ichor-fish-standard",
        name: "Ichor-Fish",
        first: 3.,
        last: 11.,
        defense: -8.,
        offense: 15.,
        dr: 0.,
        glyph: "f",
        color: "#e5bb4c",
    },
    Fish {
        kind: "greater-ichor-fish",
        name: "Greater Ichor-Fish",
        first: 7.,
        last: 16.,
        defense: -4.,
        offense: 25.,
        dr: 1.,
        glyph: "f",
        color: "#ffe08a",
    },
    Fish {
        kind: "lesser-ichor-shark",
        name: "Lesser Ichor-Shark",
        first: 12.,
        last: 21.,
        defense: 0.,
        offense: 35.,
        dr: 1.,
        glyph: "F",
        color: "#b99a48",
    },
    Fish {
        kind: "ichor-shark",
        name: "Ichor Shark",
        first: 17.,
        last: 26.,
        defense: 4.,
        offense: 45.,
        dr: 2.,
        glyph: "F",
        color: "#e5bb4c",
    },
    Fish {
        kind: "greater-ichor-shark",
        name: "Greater Ichor Shark",
        first: 22.,
        last: 31.,
        defense: 8.,
        offense: 55.,
        dr: 3.,
        glyph: "F",
        color: "#ffe08a",
    },
    Fish {
        kind: "lesser-ichor-leviathan",
        name: "Lesser Ichor Leviathan",
        first: 27.,
        last: 36.,
        defense: 12.,
        offense: 65.,
        dr: 3.,
        glyph: "L",
        color: "#b99a48",
    },
    Fish {
        kind: "ichor-leviathan",
        name: "Ichor Leviathan",
        first: 32.,
        last: 40.,
        defense: 17.,
        offense: 75.,
        dr: 4.,
        glyph: "L",
        color: "#e5bb4c",
    },
    Fish {
        kind: "greater-ichor-leviathan",
        name: "Greater Ichor Leviathan",
        first: 37.,
        last: 40.,
        defense: 21.,
        offense: 85.,
        dr: 5.,
        glyph: "L",
        color: "#ffe08a",
    },
];
pub fn make_fish(id: f64, x: f64, y: f64, depth: f64, random: f64) -> Actor {
    make_fish_revision(id, x, y, depth, random, 29.0)
}
fn fish_eligible(d: &Fish, depth: f64, revision: f64) -> bool {
    let margin = if revision >= 30.0 { 2.0 } else { 0.0 };
    (d.first - margin).max(1.0) <= depth && depth <= (d.last + margin).min(40.0)
}
/// Event eligibility only: the well-opening, ichor and spawn cadence gates remain mandatory.
pub fn eligible_fish_kinds(depth: u32, revision: f64) -> Vec<&'static str> {
    if !(1..=40).contains(&depth) {
        return Vec::new();
    }
    FISH.iter()
        .filter(|d| fish_eligible(d, depth as f64, revision))
        .map(|d| d.kind)
        .collect()
}
pub fn make_fish_revision(
    id: f64,
    x: f64,
    y: f64,
    depth: f64,
    random: f64,
    revision: f64,
) -> Actor {
    let depth = depth.floor().clamp(1., 40.);
    let pool: Vec<_> = FISH
        .iter()
        .filter(|d| fish_eligible(d, depth, revision))
        .collect();
    let d = pool[((random * pool.len() as f64).floor() as usize).min(pool.len() - 1)];
    let index = FISH.iter().position(|f| f.kind == d.kind).unwrap();
    let (accuracy, dodge, stealth) = [
        (1., 3., 3.),
        (2., 3., 3.),
        (2., 2., 2.),
        (3., 2., 2.),
        (4., 1., 1.),
        (4., -1., -2.),
        (3., -3., -4.),
        (3., -5., -6.),
        (4., -6., -8.),
    ][index];
    let mut actor = Actor {
        id,
        x,
        y,
        creature_kind: Field::Value(d.kind.into()),
        name: d.name.into(),
        glyph: d.glyph.into(),
        color: d.color.into(),
        hp: 100. + 6. * d.defense,
        max_hp: 100. + 6. * d.defense,
        energy: 50.,
        stats: Stats {
            offense: d.offense,
            defense: d.defense,
            accuracy,
            dodge,
            dr: d.dr,
            perception: 45.,
            max_energy: 50.,
            ..Default::default()
        },
        state: "wandering".into(),
        abilities: vec![
            "enemy:beast:ichor-bound".into(),
            "enemy:beast:ichor-siphon".into(),
        ],
        active: vec![],
        conditions: indexmap::IndexMap::from([
            ("bestiaryTraitsInitialized".into(), 1.),
            ("stealth".into(), stealth),
        ]),
        ..Default::default()
    };
    crate::physical_balance::apply_new_actor(&mut actor, revision);
    actor
}
/// O(A) occupancy preparation plus bounded landing checks; birth admission adds
/// O(d) clean or O(A*d + F + H) dirty registry work (H retained slot history).
pub fn tick_fish(r: &mut Run) -> Result<(), String> {
    let f = r.floor().ok_or("Missing floor")?;
    let Some(e) = f.godbeast_event.as_ref() else {
        return Ok(());
    };
    if e.kind != "Mercuria"
        || !e.activated.value_or(false)
        || e.fish_spawned.value_or(0.) >= 10.
        || e.flood_age.value_or(0.) < e.next_fish_age.value_or(8.)
    {
        return Ok(());
    }
    let w = f.width();
    let h = f.height();
    let mut occupied: std::collections::HashSet<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as isize * w as isize + a.x as isize)
        .collect();
    occupied.insert(r.player.y as isize * w as isize + r.player.x as isize);
    let target = [
        (0, 0),
        (0, -1),
        (-1, 0),
        (1, 0),
        (0, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ]
    .into_iter()
    .filter_map(|(dx, dy)| {
        let x = e.x as isize + dx;
        let y = e.y as isize + dy;
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            None
        } else {
            Some(y as usize * w + x as usize)
        }
    })
    .find(|&i| {
        f.ichor.as_ref().is_some_and(|v| v[i] != 0.)
            && !(r.simulation_revision.value_or(29.) >= 51. && crate::revision51_stairs::stair_cell(f,i))
            && !f
                .bulwarks
                .as_ref()
                .and_then(|v| v.get(&i.to_string()))
                .is_some_and(|&expiry| expiry >= r.tick)
            && !occupied.contains(&(i as isize))
            && !crate::environment::blocks_movement(f, i)
            && !solid(&f.tiles[i])
    });
    let Some(i) = target else { return Ok(()) };
    crate::creature_index::check_next_creature_id(r)?;
    let id = r.next_id;
    r.next_id += 1.;
    let random = crate::rng::random(r);
    let actor = make_fish_revision(
        id,
        (i % w) as f64,
        (i / w) as f64,
        r.depth,
        random,
        r.simulation_revision.value_or(29.0),
    );
    r.actors_at_mut(r.depth).push(actor.clone());
    let f = r.floor_mut().unwrap();
    let e = f.godbeast_event.as_mut().unwrap();
    e.fish_spawned = (e.fish_spawned.value_or(0.) + 1.).into();
    e.next_fish_age = (e.flood_age.value_or(0.) + 8.).into();
    f.dirty = true;
    if crate::vision::visible_actor(r, &actor) {
        crate::engine::log(
            r,
            format!("{} emerges from Mercuria’s well.", crate::actor_presentation::name(r, &actor)),
            "environment",
        );
        crate::events::actor_event(r, &actor, "summon", &actor.name, None, None);
    }
    Ok(())
}
/// Revival transforms only actors standing on ichor; preserves original species stats.
pub fn mercurialize_on_revival(r: &mut Run, id: f64, depth: f64) {
    let Some(f) = r.floors.iter().find(|f| f.depth == depth) else {
        return;
    };
    let Some(a) = r.actors_at(depth).iter().find(|a| a.id == id) else {
        return;
    };
    let i = a.y as usize * f.width() + a.x as usize;
    if !f.ichor.as_ref().is_some_and(|v| v[i] != 0.) {
        return;
    }
    let Some(a) = r.actors_slice_mut(depth).iter_mut().find(|a| a.id == id) else {
        return;
    };
    mercurialize_actor(a);
    let a=a.clone();
    if depth == r.depth { crate::animation_enemy::mark(r, &a, "E80", "rise-gold", crate::events::Stage::Special); }
}
pub fn mercurialize_actor(a: &mut Actor) {
    if !a.name.starts_with("Mercuria's ") && !a.name.starts_with("Mercuria’s ") {
        a.name = format!("Mercuria's {}", a.name);
    }
    a.color = "#d9ae48".into();
    for id in ["enemy:beast:ichor-bound", "enemy:beast:ichor-siphon"] {
        if !a.abilities.iter().any(|v| v == id) {
            a.abilities.push(id.into());
        }
    }
    a.conditions.shift_remove("underwater");
    a.conditions.shift_remove("submergedStealth");
}

/// Source habitat.ts: Mercurial conversion overrides original habitat limits.
pub fn habitat_valid(f: &Floor, a: &Actor, x: f64, y: f64) -> bool {
    if x < 0.
        || y < 0.
        || x >= f.width() as f64
        || y >= f.height() as f64
        || x.fract() != 0.
        || y.fract() != 0.
    {
        return false;
    }
    let i = y as usize * f.width() + x as usize;
    let has = |id: &str| a.abilities.iter().any(|s| s == id);
    let ichor = f.ichor.as_ref().is_some_and(|v| v[i] > 0.);
    if has("enemy:beast:ichor-bound") {
        return ichor;
    }
    if has("enemy:beast:slime-bound") && !f.slime.as_ref().is_some_and(|v| v[i] != 0.) {
        return false;
    }
    if has("enemy:beast:aquatic")
        && (!matches!(f.tiles[i].as_str(), "water" | "shallow-water") || ichor)
    {
        return false;
    }
    true
}
pub fn habitat_path_allowed(f: &Floor, a: &Actor, i: usize) -> bool {
    let has = |id: &str| a.abilities.iter().any(|s| s == id);
    if has("enemy:beast:slime-bound")
        && has("enemy:beast:slime-trail")
        && !has("enemy:beast:ichor-bound")
    {
        return true;
    }
    habitat_valid(f, a, (i % f.width()) as f64, (i / f.width()) as f64)
}
pub fn stranded(r: &Run, a: &Actor) -> bool {
    !habitat_valid(r.floor().expect("current floor"), a, a.x, a.y)
}
pub fn squirm_if_stranded(r: &mut Run, id: f64) -> bool {
    let Some(a) = crate::engine::actor(r, id) else {
        return false;
    };
    if !stranded(r, a) {
        return false;
    }
    if id == r.player.id || crate::vision::visible_actor(r, a) {
        let message = if id == r.player.id {
            "You squirm helplessly.".into()
        } else {
            format!("{} squirms helplessly.", crate::actor_presentation::name(r, &a))
        };
        crate::engine::log(r, message, "player");
    }
    true
}
pub fn movement_allowed(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    if a.conditions.get("forcedMove").copied().unwrap_or(0.) != 0. {
        return true;
    }
    if a.abilities.iter().any(|id| id == "enemy:beast:rooted") || stranded(r, a) {
        return false;
    }
    habitat_valid(r.floor().unwrap(), a, x, y)
}
/// enemyHitEffects invocation is after the landed hit, even zero damage, but
/// the source returns immediately if the defender has died.
pub fn siphon_hit(r: &mut Run, attacker: f64, defender: f64) {
    let Some(b) = crate::engine::actor(r, defender) else {
        return;
    };
    if b.dead.value_or(false) || b.hp <= 0. {
        return;
    }
    let Some(a) = crate::engine::actor(r, attacker).cloned() else {
        return;
    };
    if !a
        .abilities
        .iter()
        .any(|id| id == "enemy:beast:ichor-siphon")
    {
        return;
    }
    let f = r.floor().unwrap();
    let i = a.y as usize * f.width() + a.x as usize;
    if !f.ichor.as_ref().is_some_and(|v| v[i] != 0.) {
        return;
    }
    let hp = (a.hp + (a.max_hp * 0.02).ceil().max(1.)).min(a.max_hp);
    crate::engine::actor_mut_trusted(r, attacker).unwrap().hp = hp;
    if hp > a.hp { crate::animation_enemy::mark(r, &a, "E22", "droplet-ichor", crate::events::Stage::Health); }
    if r.player.x >= 0. && (attacker == r.player.id || crate::vision::visible_actor(r, &a)) {
        let who = if attacker == r.player.id {
            "You".to_string()
        } else {
            crate::actor_presentation::name(r, &a).to_string()
        };
        crate::engine::log(
            r,
            format!("{who} draws strength from the ichor."),
            if attacker == r.player.id {
                "player"
            } else {
                "enemy"
            },
        );
    }
}

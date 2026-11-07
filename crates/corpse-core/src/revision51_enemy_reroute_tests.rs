use super::*;
use crate::{ActorLastSeen, Field};

const ID: f64 = 101.;
const BLOCKER: f64 = 102.;
const EAST: Point = Point { x: 1, y: 0 };
const NORTHEAST: Point = Point { x: 1, y: -1 };
const GOAL: Point = Point { x: 14, y: 10 };

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 51.0.into();
    r.player.x = 14.; r.player.y = 10.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.actors_at_mut(r.depth).clear();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("wall".into());
    let w = f.width();
    for y in 8..=12 { for x in 8..=16 { f.tiles[y*w+x] = "floor".into(); } }
    f.portals = Field::Missing; f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
    let mut a = Actor { id: ID, x: 10., y: 10., hp: 100., max_hp: 100., state: "hunting".into(),
        last_seen: ActorLastSeen { x: GOAL.x as f64, y: GOAL.y as f64 }.into(), ..Default::default() };
    a.conditions.insert("targetId".into(), r.player.id);
    r.actors_at_mut(r.depth).push(a);
    add_blocker(&mut r, 11., 10.);
    r
}

fn add_blocker(r: &mut Run, x: f64, y: f64) {
    r.actors_at_mut(r.depth).push(Actor { id: BLOCKER, x, y, hp: 100., max_hp: 100.,
        state: "hunting".into(), abilities: vec!["enemy:beast:slow".into()], ..Default::default() });
}
fn tile(r: &mut Run, x: usize, y: usize, value: &str) {
    let f = r.floor_mut().unwrap(); let w = f.width(); f.tiles[y*w+x] = value.into();
}
fn actor_mut(r: &mut Run) -> &mut Actor { engine::actor_mut_trusted(r, ID).unwrap() }
fn fallback(r: &Run) -> Option<Point> { replacement_step(r, ID, GOAL, EAST) }

#[test]
fn unexpected_slow_ally_uses_diagonal_without_a_new_rng_draw() {
    let mut r = room();
    r.actors_at_mut(r.depth).retain(|a| a.id != BLOCKER);
    let planned = engine::creature_path_step(&mut r, ID, GOAL).unwrap();
    assert_eq!(planned, EAST);
    add_blocker(&mut r, 11., 10.);
    let before = r.clone();
    assert_eq!(replacement_step(&r, ID, GOAL, planned), Some(NORTHEAST));
    assert_eq!(r, before, "planning must be pure");
    assert_eq!(engine::creature_path_step(&mut r, ID, GOAL), Some(NORTHEAST));
    let rng = r.rng;
    assert!(engine::move_actor(&mut r, ID, 1., -1., false).unwrap());
    let a = engine::actor(&r, ID).unwrap();
    assert_eq!((a.x, a.y), (11., 9.));
    assert_eq!(r.rng, rng);
    assert_eq!(engine::actor(&r, BLOCKER).unwrap().x, 11.);
}

#[test]
fn clear_paths_and_legacy_revisions_keep_the_original_step() {
    let mut r = room();
    for revision in [29., 48., 49., 50.] {
        r.simulation_revision = revision.into();
        assert_eq!(fallback(&r), None);
        assert_eq!(engine::creature_path_step(&mut r, ID, GOAL), Some(EAST));
    }
    r.simulation_revision = 51.0.into();
    r.actors_at_mut(r.depth).retain(|a| a.id != BLOCKER);
    assert_eq!(fallback(&r), None);
    assert_eq!(engine::creature_path_step(&mut r, ID, GOAL), Some(EAST));
}

#[test]
fn disconnected_corridor_waits_instead_of_wandering() {
    let mut r = room();
    r.floor_mut().unwrap().tiles.fill("wall".into());
    for x in 10..=14 { tile(&mut r, x, 10, "floor"); }
    assert_eq!(fallback(&r), None);
    assert_eq!(engine::creature_path_step(&mut r, ID, GOAL), Some(EAST));
    assert!(!engine::move_actor(&mut r, ID, 1., 0., false).unwrap());
    assert_eq!((actor_mut(&mut r).x, actor_mut(&mut r).y), (10., 10.));
}

#[test]
fn ties_are_stable_and_new_wall_corners_are_respected() {
    let mut r = room();
    for _ in 0..4 { assert_eq!(fallback(&r), Some(NORTHEAST)); }
    // A wall north of the source forbids NE, even with an open destination.
    tile(&mut r, 10, 9, "wall");
    assert_eq!(fallback(&r), Some(Point { x: 1, y: 1 }));
    tile(&mut r, 10, 11, "wall");
    // The only legal route now starts west, increasing Euclidean distance but
    // decreasing the actual remaining legal path cost around the obstruction.
    assert_eq!(fallback(&r), Some(Point { x: -1, y: 0 }));
}

#[test]
fn actual_hidden_quarry_position_does_not_replace_last_seen() {
    let mut r = room();
    r.player.conditions.insert("invisible".into(), 10.);
    let expected = fallback(&r);
    r.player.x = 9.; r.player.y = 9.;
    assert_eq!(fallback(&r), expected);
    assert_eq!(replacement_step(&r, ID, Point { x: 9, y: 9 }, EAST), None);
    // The same rule applies when the target is a non-player body/decoy.
    let target = Actor { id: 103., x: 9., y: 9., hp: 100., max_hp: 100., ..Default::default() };
    actor_mut(&mut r).conditions.insert("targetId".into(), target.id);
    r.actors_at_mut(r.depth).push(target);
    assert_eq!(fallback(&r), expected);
}

#[test]
fn other_goals_and_action_restrictions_are_not_reinterpreted() {
    let r = room();
    for state in ["ally", "neutral", "sleeping", "wandering", "stalking", "fleeing"] {
        let mut test = r.clone(); actor_mut(&mut test).state = state.into();
        assert_eq!(fallback(&test), None, "{state}");
    }
    for ability in ["enemy:beast:rooted", "enemy:beast:minotaur"] {
        let mut test = r.clone(); actor_mut(&mut test).abilities.push(ability.into());
        assert_eq!(fallback(&test), None, "{ability}");
    }
    for condition in ["confused", "paralyzed", "forcedMove", "riptide", "monsterRetreating"] {
        let mut test = r.clone(); actor_mut(&mut test).conditions.insert(condition.into(), 1.);
        assert_eq!(fallback(&test), None, "{condition}");
    }
    let mut dead = r.clone(); actor_mut(&mut dead).dead = true.into();
    assert_eq!(fallback(&dead), None);
}

#[test]
fn habitat_flight_phase_and_slime_capabilities_shape_legal_routes() {
    let mut r = room();
    tile(&mut r, 11, 9, "chasm"); tile(&mut r, 11, 11, "chasm");
    assert_eq!(fallback(&r), Some(Point { x: 0, y: -1 }));
    actor_mut(&mut r).abilities.push("enemy:beast:flight".into());
    assert_eq!(fallback(&r), Some(NORTHEAST));
    tile(&mut r, 10, 9, "wall"); tile(&mut r, 10, 11, "wall");
    assert_eq!(fallback(&r), Some(Point { x: -1, y: 0 }));
    actor_mut(&mut r).active.push("etill-phase-through".into());
    assert_eq!(fallback(&r), Some(NORTHEAST));

    let mut aquatic = room();
    actor_mut(&mut aquatic).abilities.push("enemy:beast:aquatic".into());
    for (x, y) in [(10,10), (11,10), (11,11), (12,10), (13,10), (14,10)] {
        tile(&mut aquatic, x, y, "water");
    }
    assert_eq!(fallback(&aquatic), Some(Point { x: 1, y: 1 }));
    tile(&mut aquatic, 11, 11, "floor");
    assert_eq!(fallback(&aquatic), None);

    let mut slime = room(); let n = slime.floor().unwrap().tiles.len();
    let w = slime.floor().unwrap().width(); let mut cells = vec![0.; n];
    for (x, y) in [(10,10), (11,10), (11,11), (12,10), (13,10), (14,10)] { cells[y*w+x] = 1.; }
    slime.floor_mut().unwrap().slime = cells.into();
    actor_mut(&mut slime).abilities = vec!["enemy:beast:slime-bound".into(), "enemy:beast:slime-walker".into()];
    assert_eq!(fallback(&slime), Some(Point { x: 1, y: 1 }));
}

#[test]
fn sticky_departure_cost_is_counted_and_cannot_bypass_the_struggle() {
    let mut r = room(); let w = r.floor().unwrap().width(); let n = r.floor().unwrap().tiles.len();
    let mut mud = vec![0.; n]; mud[9*w+11] = 1.;
    r.floor_mut().unwrap().mud = mud.into();
    assert_eq!(fallback(&r), Some(Point { x: 1, y: 1 }), "equal-length north route costs a struggle");
    r.floor_mut().unwrap().mud.as_mut().unwrap()[10*w+10] = 1.;
    let chosen = engine::creature_path_step(&mut r, ID, GOAL).unwrap();
    assert!(engine::move_actor(&mut r, ID, chosen.x as f64, chosen.y as f64, false).unwrap());
    assert_eq!((actor_mut(&mut r).x, actor_mut(&mut r).y), (10., 10.));
    assert_eq!(engine::condition(actor_mut(&mut r), "stuck"), 1.);
    assert!(engine::move_actor(&mut r, ID, chosen.x as f64, chosen.y as f64, false).unwrap());
    assert_eq!((actor_mut(&mut r).x, actor_mut(&mut r).y), (11., 11.));
}

#[test]
fn newly_solid_selected_step_is_rerouted_before_attempt() {
    let mut r = room();
    r.actors_at_mut(r.depth).retain(|a| a.id != BLOCKER);
    assert_eq!(engine::creature_path_step(&mut r, ID, GOAL), Some(EAST));
    tile(&mut r, 11, 10, "wall");
    // Both eastward diagonals are blocked by the new wall corner.
    assert_eq!(fallback(&r), Some(Point { x: 0, y: -1 }));
}

#[test]
fn stair_quarry_goal_routes_to_legal_adjacent_contact_without_entering_stairs() {
    for stair in ["up", "down"] {
        let mut r = room();
        tile(&mut r, GOAL.x as usize, GOAL.y as usize, stair);
        assert_eq!(fallback(&r), Some(NORTHEAST), "{stair}");
        // A hidden quarry moving elsewhere must not replace its known stair.
        r.player.x = 9.; r.player.y = 9.;
        assert_eq!(fallback(&r), Some(NORTHEAST), "{stair} last seen");
        // All approachable contact squares occupied means no invented route
        // through them, and no permission to step onto the forbidden stair.
        for (i, d) in movement::DIRECTIONS.into_iter().enumerate() {
            r.actors_at_mut(r.depth).push(Actor { id: 200. + i as f64,
                x: (GOAL.x + d.x) as f64, y: (GOAL.y + d.y) as f64,
                hp: 100., max_hp: 100., state: "hunting".into(), ..Default::default() });
        }
        assert_eq!(fallback(&r), None, "{stair} occupied contact squares");
    }
}

#[test]
fn stair_contact_goals_do_not_cut_wall_corners_or_bypass_habitat() {
    let mut r = room();
    r.floor_mut().unwrap().tiles.fill("wall".into());
    // The route reaches only the NW diagonal of the stair. The wall to its
    // west blocks that diagonal contact, so it is not an eligible terminal.
    for x in 10..=13 { tile(&mut r, x, 9, "floor"); }
    for x in 10..=11 { tile(&mut r, x, 10, "floor"); }
    tile(&mut r, 14, 10, "down");
    assert_eq!(fallback(&r), None);
    tile(&mut r, 13, 10, "floor");
    tile(&mut r, 14, 9, "floor");
    assert_eq!(fallback(&r), Some(NORTHEAST));
    actor_mut(&mut r).abilities.push("enemy:beast:aquatic".into());
    for x in 10..=12 { tile(&mut r, x, 9, "water"); }
    for x in 10..=11 { tile(&mut r, x, 10, "water"); }
    // The adjacent floor cells are not legal aquatic terminal cells.
    assert_eq!(fallback(&r), None);
}

#[test]
fn portals_count_as_one_move_and_all_occupied_aliases_are_blocked() {
    let mut r = room(); let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().portals = vec![crate::portals_generation::PortalPair {
        a: 9*w+10, b: 10*w+13, color_index: 0,
    }].into();
    assert_eq!(fallback(&r), Some(Point { x: 0, y: -1 }));
    let rng = r.rng;
    assert!(engine::move_actor(&mut r, ID, 0., -1., true).unwrap());
    assert_eq!((actor_mut(&mut r).x, actor_mut(&mut r).y), (13., 10.));
    assert_eq!(r.rng, rng);

    let mut r = room(); let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().portals = vec![crate::portals_generation::PortalPair {
        a: 9*w+11, b: 10*w+11, color_index: 0,
    }].into();
    assert_eq!(fallback(&r), Some(Point { x: 1, y: 1 }));
}

fn edge(from: usize, arrivals: &[usize], actions: u32) -> Edge {
    Edge { from, direction: EAST, arrivals: arrivals.into(), actions }
}

#[test]
fn weighted_costs_are_exact_and_stochastic_exits_require_guaranteed_progress() {
    let graph = vec![edge(0, &[1], 1), edge(1, &[4], 3), edge(0, &[2], 1),
        edge(2, &[3], 1), edge(3, &[4], 1)];
    assert_eq!(remaining_actions(5, &graph, &[4]), vec![3, 3, 2, 1, 0]);
    let graph = vec![edge(0, &[1, 2], 1), edge(1, &[3], 1), edge(2, &[1], 1)];
    assert_eq!(remaining_actions(4, &graph, &[3]), vec![3, 1, 2, 0]);
    let graph = vec![edge(0, &[1, 2], 1), edge(1, &[3], 1), edge(2, &[0], 1)];
    assert_eq!(remaining_actions(4, &graph, &[3]), vec![UNREACHABLE, 1, UNREACHABLE, 0]);
}

#[test]
fn bounded_full_grid_has_exact_chebyshev_cost_without_per_candidate_searches() {
    let n = 3600; let w = 60; let mut graph = Vec::with_capacity(n*8);
    for from in 0..n { for direction in movement::DIRECTIONS {
        let (x, y) = ((from%w) as i32 + direction.x, (from/w) as i32 + direction.y);
        if (0..60).contains(&x) && (0..60).contains(&y) {
            graph.push(edge(from, &[y as usize*w+x as usize], 1));
        }
    }}
    assert!(graph.len() <= n*8);
    let costs = remaining_actions(n, &graph, &[n-1]);
    for (at, cost) in costs.into_iter().enumerate() {
        assert_eq!(cost, (59-at%w).max(59-at/w) as u32);
    }
}

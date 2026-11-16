//! R52 route preferences, detached previews and real movement boundaries.
use super::*;
use corpse_core::{Floor, revision52_swimming};
use std::sync::Arc;

pub(super) const W: usize = 9;
pub(super) const N: usize = W * W;
pub(super) fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into(); r.status = "playing".into(); r.tick = 0.;
    r.player.x = 1.; r.player.y = 4.; r.player.hp = r.player.max_hp;
    r.player.energy = r.player.stats.max_energy;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.player.amulet_ledger = Field::Missing;
    r.player.conditions.insert("allSeeingReveal".into(), 1.);
    r.player.conditions.insert("allSeeingDepth".into(), r.depth);
    r.player.stats.perception = 300.;
    r.inventory.clear(); r.actors.clear(); r.ground.clear(); r.offers.clear();
    r.history.clear(); r.replay.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.dreamwalking = Field::Missing; r.phase = "normal".to_string().into(); r.next_id = 1000.;
    let mut tiles = vec!["wall".to_owned(); N];
    for y in 1..W-1 { for x in 1..W-1 { tiles[y*W+x] = "floor".into(); } }
    r.floors = vec![Arc::new(Floor {
        depth: r.depth, domain: "Dungeon".into(), width: (W as f64).into(), height: (W as f64).into(),
        remembered_tiles: tiles.clone().into(), tiles, lights: vec![100.; N], seen: vec![true; N],
        ..Default::default()
    })];
    r.actors.insert(r.depth.to_string(), Arc::new(Vec::new()));
    r.ground.insert(r.depth.to_string(), Arc::new(Vec::new()));
    r
}
pub(super) fn set_tile(r: &mut Run, x: usize, y: usize, tile: &str) {
    let f = r.floor_mut().unwrap(); f.tiles[y*W+x] = tile.into();
    f.remembered_tiles.as_mut().unwrap()[y*W+x] = tile.into();
}
pub(super) fn corridor(r: &mut Run) {
    for y in 1..W-1 { for x in 1..W-1 { set_tile(r, x, y, if y == 4 { "floor" } else { "wall" }); } }
}
fn entered(path: &[Step]) -> Vec<(usize, usize)> { path.iter().map(|s| (s.x, s.y)).collect() }
fn straight() -> Vec<(usize, usize)> { (2..=7).map(|x| (x, 4)).collect() }
fn has_deep(path: &[Step], k: &Knowledge) -> bool {
    path.iter().any(|s| k.deep(s.y*W+s.x) || k.deep(s.arrival_y*W+s.arrival_x))
}

#[test]
fn r52_shallow_water_has_normal_cost_for_visible_and_remembered_routes_in_both_phases() {
    for current in [false, true] { for phase in ["haste", "normal"] { for haste in [false, true] {
        let mut r = room(); set_tile(&mut r, 4, 4, "shallow-water");
        r.phase = phase.to_string().into();
        if haste { r.player.conditions.insert("hasted".into(), 3.); }
        let before = r.clone();
        let k = Knowledge::observe(&r, &[current; N], |_| false).unwrap();
        for direct in [false, true] {
            assert_eq!(entered(&k.path(7, 4, direct)), straight());
            assert_eq!(entered(&k.path(4, 4, direct)), vec![(2,4),(3,4),(4,4)]);
        }
        assert!(!k.wet.iter().any(|wet| *wet)); assert_eq!(r, before);
        r.simulation_revision = 51.0.into();
        let legacy = Knowledge::observe(&r, &[current; N], |_| false).unwrap();
        assert!(!entered(&legacy.path(7,4,false)).contains(&(4,4)), "legacy wet preference remains frozen");
    } } }
}

#[test]
fn r52_deep_water_is_not_an_endpoint_or_bridge_with_swimming_flight_drown_or_memory() {
    for current in [false, true] { for mobility in ["ordinary", "swimming", "flight", "drown"] {
        let mut r = room();
        match mobility {
            "swimming" => r.inventory.push(revision52_swimming::make_ring(900.)),
            "flight" => { r.player.conditions.insert("flying".into(), 10.); }
            "drown" => r.player.abilities.push("hie-drown".into()),
            _ => {}
        }
        for y in 1..W-1 { set_tile(&mut r, 4, y, "water"); }
        let k = Knowledge::observe(&r, &[current; N], |_| false).unwrap();
        assert!(k.path(7,4,false).is_empty(), "deep bridge: {current}/{mobility}");
        assert!(k.path(4,4,true).is_empty(), "deep target: {current}/{mobility}");
        // One shallow connection, not the adjacent deep squares, opens the route.
        set_tile(&mut r, 4, 4, "shallow-water");
        let k = Knowledge::observe(&r, &[current; N], |_| false).unwrap();
        let path = k.path(7,4,false); assert_eq!(entered(&path), straight()); assert!(!has_deep(&path,&k));
    } }
}

#[test]
fn r52_hidden_live_water_and_unknown_terrain_cannot_change_saved_routes() {
    let mut r = room(); set_tile(&mut r, 4, 4, "shallow-water");
    let historical = Knowledge::observe(&r, &[false; N], |_| false).unwrap().path(7,4,false);
    assert_eq!(entered(&historical), straight());
    for tile in ["water", "wall", "floor"] {
        let f = r.floor_mut().unwrap(); f.tiles.fill(tile.into());
        f.ichor = vec![1.; N].into(); f.currents = vec![3.; N].into();
        let before = r.clone();
        assert_eq!(Knowledge::observe(&r, &[false; N], |_| false).unwrap().path(7,4,false), historical);
        assert_eq!(r,before);
    }
    // Known deep water stays blocked even when the hidden live bridge dries out.
    for y in 1..W-1 { r.floor_mut().unwrap().remembered_tiles.as_mut().unwrap()[y*W+4] = "water".into(); }
    assert!(Knowledge::observe(&r, &[false; N], |_| false).unwrap().path(7,4,false).is_empty());
    // Neither unknown shallows nor an absent legacy memory are a walkability hint.
    for y in 0..W { r.floor_mut().unwrap().seen[y*W+4] = false; }
    r.floor_mut().unwrap().tiles.fill("shallow-water".into());
    assert!(Knowledge::observe(&r, &[false; N], |_| false).unwrap().path(7,4,false).is_empty());
    r.floor_mut().unwrap().remembered_tiles = Field::Missing;
    assert!(Knowledge::observe(&r, &[false; N], |_| false).unwrap().path(7,4,false).is_empty());
}

#[test]
fn r52_shallow_water_does_not_override_independent_fire_hazard_costs() {
    let mut r = room(); set_tile(&mut r, 4, 4, "shallow-water");
    let mut environment = corpse_core::EnvironmentalState::default();
    environment.fire = vec![0.; N]; environment.fire[4*W+4] = 8.;
    r.floor_mut().unwrap().environment = environment.into();
    let k = Knowledge::observe(&r, &[true; N], |_| false).unwrap();
    assert!(k.hazards[4*W+4]);
    let path = k.path(7,4,false); assert!(!path.is_empty()); assert!(!entered(&path).contains(&(4,4)));
}

#[test]
fn r52_hover_click_and_real_automatic_commands_share_the_shallow_route() {
    for swimming in [false,true] {
        let mut r = room(); set_tile(&mut r, 3,4,"shallow-water"); set_tile(&mut r,4,4,"shallow-water");
        if swimming { r.inventory.push(revision52_swimming::make_ring(900.)); }
        corpse_core::validation::validate(&r).unwrap(); engine::validate_supported(&r).unwrap();
        let before = r.clone(); let hover = path_to(&r,7,4,true); assert_eq!(entered(&hover),straight());
        let mut view = crate::observe::observe(&r);
        crate::sync_pointer_view(&mut view,&r,Some((7,4)),entered(&hover),None);
        assert_eq!(view.path,straight());
        let Start::Walk(mut walk) = start(&r,7,4) else { panic!("distant click should start travel"); };
        assert_eq!(walk.path(),hover); assert_eq!(r,before);
        let mut visited = Vec::new(); let mut bonus_moves = 0;
        for _ in 0..8 {
            let Some(command) = walk.next_command(&r).unwrap() else { break; };
            let outcome = engine::dispatch(&mut r,&command).unwrap();
            bonus_moves += usize::from(!outcome.took_turn);
            visited.push((r.player.x as usize,r.player.y as usize));
            if !walk.after_step(&r,false).unwrap() { break; }
        }
        assert_eq!(visited,straight()); assert_eq!((r.player.x,r.player.y),(7.,4.));
        assert_eq!(bonus_moves > 0, swimming, "existing conditional Swimming haste remains active");
        assert!(walk.path().is_empty());
    }
}

#[test]
fn r52_manual_adjacent_deep_water_input_and_swimming_are_unchanged() {
    for swimming in [false,true] {
        let mut r = room(); set_tile(&mut r,2,4,"water");
        if swimming { r.inventory.push(revision52_swimming::make_ring(900.)); }
        assert!(path_to(&r,2,4,true).is_empty());
        let Start::Direct(command) = start(&r,2,4) else { panic!("adjacent click remains manual input"); };
        assert_eq!(command,Command::Move{dx:1.,dy:0.});
        assert!(engine::dispatch(&mut r,&command).unwrap().took_turn);
        assert_eq!((r.player.x,r.player.y),(2.,4.));
        assert!(corpse_core::environment::is_swimming(&r,&r.player));
        assert_eq!(corpse_core::ai::is_hasted_in(&r,&r.player),swimming);
        set_tile(&mut r,3,4,"shallow-water");
        assert_eq!(entered(&path_to(&r,3,4,true)),vec![(3,4)], "a swimmer can plan a shallow exit");
    }
}

#[test]
fn r52_automatic_replans_stop_when_a_shallow_bridge_becomes_known_deep_water() {
    let mut r = room(); corridor(&mut r); set_tile(&mut r,4,4,"shallow-water");
    let Start::Walk(mut walk) = start(&r,7,4) else { panic!("known shallow bridge should route"); };
    let command = walk.next_command(&r).unwrap().unwrap(); engine::dispatch(&mut r,&command).unwrap();
    set_tile(&mut r,4,4,"water");
    assert_eq!(walk.after_step(&r,false).unwrap_err(),"Travel paused: no known route reaches that destination.");
    assert!(walk.path().is_empty()); assert!(walk.next_command(&r).unwrap().is_none());
    assert!(matches!(start(&r,7,4),Start::NoPath));
}

#[test]
fn r52_autoexplore_uses_known_shallows_but_never_deep_or_unknown_water() {
    for material in ["shallow-water","water"] {
        let mut r = room(); corridor(&mut r); set_tile(&mut r,3,4,material);
        r.player.conditions.clear(); r.player.stats.perception = 0.;
        r.floor_mut().unwrap().lights.fill(-100.);
        r.floor_mut().unwrap().seen[4*W+6] = false; r.floor_mut().unwrap().seen[4*W+7] = false;
        let mask = vision::terrain_visibility(&r);
        assert!(!mask[4*W+3] && !mask[4*W+6], "R52 darkness fixture must actually hide the water and frontier");
        let before = r.clone(); let mut explore = AutoExplore::default();
        assert!(explore.press(&r).is_none()); assert_eq!(r,before);
        if material == "water" {
            assert!(explore.path().unwrap().is_empty()); assert!(explore.press(&r).is_none());
        } else {
            assert_eq!(entered(explore.path().unwrap()),vec![(2,4),(3,4),(4,4),(5,4)]);
            let walking = explore.press(&r).unwrap(); assert!(walking.exploring());
            assert!(walking.path().iter().all(|s| before.floor().unwrap().seen[s.y*W+s.x]));
            // Hidden live deep water cannot rewrite the remembered frontier preview.
            r.floor_mut().unwrap().tiles[4*W+3] = "water".into();
            r.floor_mut().unwrap().tiles[4*W+6] = "water".into();
            let mut hidden = AutoExplore::default(); hidden.press(&r);
            assert_eq!(hidden.path().unwrap(),walking.path());
        }
    }
}

#[test]
fn r52_ichor_covered_water_retains_dry_footing_for_routes_and_portal_arrivals() {
    for current in [false,true] { for portal in [false,true] {
        let mut r = room(); corridor(&mut r); set_tile(&mut r,6,4,"water");
        let f = r.floor_mut().unwrap(); let mut ichor = vec![0.; N]; ichor[4*W+6] = 1.;
        f.ichor = ichor.clone().into(); f.remembered_ichor = ichor.into();
        if portal { f.portals = vec![corpse_core::portals_generation::PortalPair {
            a:4*W+2,b:4*W+6,color_index:0
        }].into(); }
        let k = Knowledge::observe(&r,&[current;N], |_|false).unwrap();
        assert!(!k.deep(4*W+6), "ichor suppresses water contact");
        let path = k.path(7,4,false);
        assert_eq!(entered(&path),if portal {vec![(2,4),(7,4)]} else {straight()});
        assert!(!has_deep(&path,&k));
        let f = r.floor_mut().unwrap(); f.ichor = Field::Missing; f.remembered_ichor = Field::Missing;
        let k = Knowledge::observe(&r,&[current;N], |_|false).unwrap();
        assert!(k.deep(4*W+6)); assert!(k.path(7,4,false).is_empty());
    } }
}

#[test]
fn r52_portal_arrivals_use_the_same_shallow_and_deep_boundary() {
    for current in [false,true] { for material in ["shallow-water","water"] {
        let mut r = room(); corridor(&mut r); set_tile(&mut r,6,4,material);
        r.floor_mut().unwrap().portals = vec![corpse_core::portals_generation::PortalPair {
            a:4*W+2,b:4*W+6,color_index:0
        }].into();
        let k = Knowledge::observe(&r,&[current;N], |_|false).unwrap();
        let path = k.path(7,4,false);
        if material == "water" { assert!(path.is_empty()); }
        else {
            assert_eq!(entered(&path),vec![(2,4),(7,4)]);
            assert_eq!((path[0].arrival_x,path[0].arrival_y),(6,4)); assert!(!has_deep(&path,&k));
        }
    } }
}

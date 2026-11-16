//! Frozen-v48 environment-linked divine boundaries. Resource clocks stay in the scheduler.
use crate::{Actor, FloorAlteredChasmsEntry, Run};
const PHASE: &str = "etill-phase-through";
/// O(1) terrain writes; COW detaches only the current floor.
pub fn leave_black_pit(r: &mut Run, id: f64, old: usize) {
    crate::dreamwalking::with_actor(r, id, |r| leave_black_pit_inner(r, id, old));
}
fn leave_black_pit_inner(r: &mut Run, id: f64, old: usize) {
    let Some(a) = crate::engine::actor(r, id) else {
        return;
    };
    if (id != r.player.id && !crate::half_here::is_clone(a))
        || !a.active.iter().any(|s| s == "grisha-black-pit")
    {
        return;
    }
    let f = r.floor_mut().unwrap();
    let tile = f.tiles[old].clone();
    let changed=tile!="chasm";
    let point=crate::events::EventPoint{x:(old%f.width()) as f64,y:(old/f.width()) as f64};
    let altered = f.altered_chasms.get_or_insert_with(Default::default);
    if tile != "chasm" && !altered.contains_key(&old.to_string()) {
        altered.insert(old.to_string(), FloorAlteredChasmsEntry { tile, age: 0. });
    }
    f.tiles[old] = "chasm".into();
    for layer in [&mut f.ichor, &mut f.slime, &mut f.mud, &mut f.roots] {
        if let Some(v) = layer.as_mut() {
            v[old] = 0.;
        }
    }
    f.dirty = true;
    if changed {crate::events::at(r,"P53","collapse",crate::events::Stage::Special,&[point]);}
}
/// O(A) stable target lookup; forced movement invokes the shared exact movement pipeline.
pub fn apply_riptide(r: &mut Run) -> Result<(), String> {
    if !r.player.active.iter().any(|s| s == "etill-riptide") {
        return Ok(());
    }
    let id = crate::engine::condition(&r.player, "riptideTarget");
    let Some(a) = r
        .actors_at(r.depth)
        .iter()
        .find(|a| a.id == id && !a.dead.value_or(false)
            && crate::dreamwalking::same_plane(r, &r.player, a))
    else {
        return Ok(());
    };
    let (dx, dy) = crate::portals::direction_toward(r, a,
        crate::engine::condition(&r.player, "riptideX"),
        crate::engine::condition(&r.player, "riptideY"));
    let before=a.clone();
    crate::engine::actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .insert("forcedMove".into(), 1.);
    crate::engine::move_actor(r, id, dx, dy, false).map_err(|e| e.to_string())?;
    if let Some(a) = crate::engine::actor_mut_trusted(r, id) {
        a.conditions.shift_remove("forcedMove");
        a.conditions.insert("riptide".into(), 1.);
    }
    if let Some(after)=crate::engine::actor(r,id) {crate::animation_player::moved(r,&before,after,"P30","wave");}
    Ok(())
}
pub fn water_at(r: &Run, a: &Actor) -> bool {
    let Some(f) = crate::dreamwalking::floor_for(r, a) else {
        return false;
    };
    a.x >= 0.
        && a.y >= 0.
        && f.tiles
            .get(a.y as usize * f.width() + a.x as usize)
            .is_some_and(|t| t == "water" || t == "shallow-water")
}
/// Owner regeneration benefits from either physical endpoint; ordinary actors use their own cell.
pub fn drown_regen_multiplier(r: &Run, a: &Actor) -> f64 {
    if !a.abilities.iter().any(|s| s == "etill-drown") {
        return 1.;
    }
    if (a.id == r.player.id && crate::half_here::origins(r).iter().any(|a| water_at(r, a)))
        || (a.id != r.player.id && water_at(r, a))
    {
        3.
    } else {
        1.
    }
}
/// Active-player aura excludes friendly healing; corpse time freezing is a separate predicate.
pub fn healing_suppressed(r: &Run, a: &Actor) -> bool {
    a.id != r.player.id && a.state != "ally" && corpse_frozen(r, a)
}
pub fn corpse_frozen(r: &Run, a: &Actor) -> bool {
    r.player.active.iter().any(|s| s == "dawel-reverse-entropy")
        && crate::half_here::origins(r).iter().any(|origin| {
            crate::dreamwalking::same_plane(r, origin, a)
                && crate::portals::actor_distance(r, origin, a) <= 10.
        })
}
pub fn phase_crushing_endpoint(r: &Run) -> Option<f64> {
    if r.player.active.iter().any(|s| s == PHASE) {
        return None;
    }
    crate::half_here::origins(r)
        .iter()
        .find(|a| {
            crate::dreamwalking::floor_for(r, a).is_some_and(|f| f.tiles
                .get(a.y as usize * f.width() + a.x as usize)
                .is_some_and(|t| t == "wall"))
        })
        .map(|a| a.id)
}
/// O(A + floor-transition work). Crushing precedes all rescue/fall handling and uses the shared owner pool.
pub fn settle_phase_loss(r: &mut Run, defer_falls: bool) -> Result<(), String> {
    if r.status != "playing" || r.player.active.iter().any(|s| s == PHASE) {
        return Ok(());
    }
    crate::half_here::sync(r);
    if let Some(id) = phase_crushing_endpoint(r) {
        let owner_id = crate::half_here::owner_id(r, crate::engine::actor(r, id).unwrap());
        let owner = crate::engine::actor(r, owner_id).unwrap().clone();
        crate::resources::record_loss(r, &owner, "hp", owner.hp.max(0.));
        crate::engine::actor_mut_trusted(r, owner_id).unwrap().hp = 0.;
        return crate::engine::unpreventable_death(r, id, "phase-crushing")
            .map_err(|e| e.to_string());
    }
    if defer_falls {
        return Ok(());
    }
    crate::engine::death_if_nonpositive(r, r.player.id).map_err(|e| e.to_string())?;
    if r.status != "playing" {
        return Ok(());
    }
    if let Some(id) = crate::half_here::clone_id(r) {
        if !crate::travel::flying(r, crate::engine::actor(r, id).unwrap()) {
            crate::travel::fall_actors_one_level(r, r.depth, &[id], None)?;
        }
    }
    if r.status != "playing" {
        return Ok(());
    }
    let f = r.floor().unwrap();
    let tile = f.tiles[r.player.y as usize * f.width() + r.player.x as usize].clone();
    if !crate::travel::flying(r, &r.player) {
        if tile == "chasm" {
            crate::travel::travel(r, 1., true, false)?;
        } else if tile == "pit" {
            r.status = "won".into();
            crate::engine::log(
                r,
                "Without flight, you fall into the bottomless pit. Victory.",
                "player",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn dream_plane_black_pit_and_water_queries_use_each_endpoint_floor() {
        let mut r: crate::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: r.depth, ..Default::default() }.into();
        r.player.active = vec!["grisha-black-pit".into()];
        let id = r.player.id;
        let at = r.player.y as usize * r.floor().unwrap().width() + r.player.x as usize;
        let physical_tile = r.floor().unwrap().tiles[at].clone();
        super::leave_black_pit(&mut r, id, at);
        assert_eq!(r.floor().unwrap().tiles[at], physical_tile);
        assert_eq!(crate::dreamwalking::floor_for(&r, &r.player).unwrap().tiles[at], "chasm");
        crate::dreamwalking::with_actor(&mut r, id, |r| r.floor_mut().unwrap().tiles[at] = "water".into());
        assert!(super::water_at(&r, &r.player));
        let mut physical = r.player.clone(); physical.id = 90.;
        assert!(!super::water_at(&r, &physical));
        assert!(!crate::dreamwalking::context_dream(&r));
    }
    #[test]
    fn dream_plane_riptide_and_reverse_entropy_keep_endpoint_planes() {
        let mut r: crate::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: r.depth, ..Default::default() }.into();
        r.player.active = vec!["etill-riptide".into(), "dawel-reverse-entropy".into()];
        let target = r.actors_at(1.)[0].clone();
        r.player.conditions.insert("riptideTarget".into(), target.id);
        r.player.conditions.insert("riptideX".into(), target.x - 1.);
        r.player.conditions.insert("riptideY".into(), target.y);
        super::apply_riptide(&mut r).unwrap();
        assert_eq!(crate::engine::actor(&r, target.id), Some(&target));
        assert!(!super::corpse_frozen(&r, &target));
        let mut dream_target = target.clone();
        dream_target.conditions.insert("phantasmiiPlane".into(), 1.);
        assert!(super::corpse_frozen(&r, &dream_target));
        // A Half Here created before dreaming retains a physical-plane aura.
        r.player.abilities.push(crate::half_here::ID.into());
        r.player.active.push(crate::half_here::ID.into());
        r.player.conditions.insert("halfHereId".into(), 90.);
        let mut clone = r.player.clone(); clone.id = 90.;
        clone.conditions.clear();
        clone.conditions.insert("halfHereClone".into(), 1.);
        clone.conditions.insert("halfHereOwner".into(), r.player.id);
        clone.conditions.insert("phantasmiiPlane".into(), 0.);
        r.actors_at_mut(1.).push(clone);
        assert!(super::corpse_frozen(&r, &target));
        r.player.active.retain(|s| s != crate::half_here::ID);
        r.simulation_revision = 33.0.into();
        assert!(super::corpse_frozen(&r, &target));
    }
    fn compare(a: &serde_json::Value, b: &serde_json::Value, path: &str) {
        match (a, b) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                assert_eq!(a.len(), b.len(), "keys {path}");
                for (k, v) in a {
                    compare(
                        v,
                        b.get(k).unwrap_or_else(|| panic!("missing {path}.{k}")),
                        &format!("{path}.{k}"),
                    );
                }
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "length {path}");
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    compare(a, b, &format!("{path}[{i}]"));
                }
            }
            _ => assert_eq!(a, b, "{path}"),
        }
    }
    #[test]
    fn phase_loss_matches_frozen_reference() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/divine-environment-v48.json"
        ))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let mut r: crate::Run = serde_json::from_value(case["initial"].clone()).unwrap();
            assert_eq!(
                super::phase_crushing_endpoint(&r),
                case["endpoint"].as_f64(),
                "{}",
                case["name"]
            );
            crate::events::begin();
            super::settle_phase_loss(&mut r, case["defer"].as_bool().unwrap()).unwrap();
            let actual = crate::events::finish();
            let expected: crate::Run = serde_json::from_value(case["expected"].clone()).unwrap();
            compare(
                &serde_json::to_value(r).unwrap(),
                &serde_json::to_value(expected).unwrap(),
                case["name"].as_str().unwrap(),
            );
            let events: Vec<crate::events::PresentationEvent> =
                serde_json::from_value(case["events"].clone()).unwrap();
            assert_eq!(actual, events, "{}", case["name"]);
        }
    }
}

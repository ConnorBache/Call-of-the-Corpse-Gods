//! Mini-Me snapshot construction; payment/admission and activation events are caller-owned.
use crate::{Actor, Field, Run, combat::integer_stat};
pub const ID: &str = "kative-mini-me";
/// Pure R50 admission; snapshotting stats does not copy item or power ownership.
pub fn preflight(r: &Run) -> Result<(), String> {
    if r.simulation_revision.value_or(29.0)<50.0 {return Ok(());}
    if r.player.conditions.contains_key("miniMeId") || r.player.active.iter().any(|id|id==ID) {
        return Err("Dismiss the existing Mini-Me before creating another copy.".into());
    }
    if !r.next_id.is_finite() || r.next_id<0.0 || r.next_id.fract()!=0.0 || r.next_id+1.0>9007199254740991.0 {
        return Err("Mini-Me would exhaust exact actor identities.".into());
    }
    Ok(())
}
/// Freeze current effective numeric stats, including item and numeric buff
/// contributions, once. No power/status/target/item/reserve state is inherited.
/// Cost is effective_stats' full derivation; the resulting snapshot is fixed-size.
fn stats_preview(r: &Run, x: f64, y: f64) -> Actor {
    let stats=crate::combat::effective_stats(r,&r.player);
    let base=r.player.max_hp-6.0*stats.defense-stats.max_hp.value_or(0.0);
    let mut a=Actor {
        id:r.next_id,x,y,name:"Mini-Me".into(),glyph:r.player.glyph.clone(),color:r.player.color.clone(),
        hp:r.player.hp,max_hp:r.player.max_hp,base_max_hp:base.into(),energy:r.player.energy,
        stats,state:"ally".into(),perfect:true.into(),order:"a".to_string().into(),
        dead:false.into(),heartless:false.into(),..Default::default()
    };
    a.conditions.insert("miniMe".into(),1.0);
    if crate::dreamwalking::enabled(r) {
        a.conditions.insert("phantasmiiPlane".into(),if crate::dreamwalking::dream_actor(r,&r.player){1.0}else{0.0});
    }
    a.conditions.insert("weaponDamage".into(),crate::revision50_player_attack::innate_damage_for_revision(
        r.simulation_revision.value_or(29.0)));
    a.conditions.insert("weaponOffenseMultiplier".into(), if crate::combat::raw_attack_damage_enabled(r.simulation_revision.value_or(29.)) { 1.0 } else { crate::revision50_player_attack::OFFENSE_MULTIPLIER });
    a
}
/// Eight row-major candidates: O(F+A+G+P), O(1) extra space, for floor lookup F,
/// actors A, grate entries G and player powers P; event-solid shapes are bounded.
pub fn destination(r: &Run) -> Option<(f64, f64)> {
    let placement=if r.simulation_revision.value_or(29.0)>=50.0 {stats_preview(r,r.player.x,r.player.y)} else {r.player.clone()};
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (x, y) = (r.player.x + dx as f64, r.player.y + dy as f64);
            if !crate::engine::actor_passable(r, &placement, x, y) {
                continue;
            }
            let f = r.floor().unwrap();
            let i = y as usize * f.width() + x as usize;
            if matches!(f.tiles[i].as_str(), "chasm" | "pit") {
                continue;
            }
            if !r.actors_at(r.depth).iter().any(|a| !a.dead.value_or(false)
                && crate::dreamwalking::same_plane(r, &r.player, a)
                && crate::portals::same_position(r, a, x, y))
                && (!crate::portals::enabled(r) || !crate::portals::same_position(r, &r.player, x, y))
            {
                return Some((x, y));
            }
        }
    }
    None
}
/// Detached prospective actor permits validating copied powers before spending energy.
pub fn preview(r: &Run, x: f64, y: f64) -> Actor {
    if r.simulation_revision.value_or(29.0)>=50.0 {return stats_preview(r,x,y);}
    let initial = r.initial_player.as_ref().unwrap_or(&r.player);
    let mut a = initial.clone();
    a.id = r.next_id;
    a.x = x;
    a.y = y;
    a.name = "Mini-Me".into();
    a.glyph = "@".into();
    a.color = "#88d9cd".into();
    a.state = "ally".into();
    a.perfect = true.into();
    a.order = "a".to_string().into();
    a.dead = false.into();
    a.heartless = false.into();
    a.active.clear();
    a.conditions.clear();
    a.conditions.insert("miniMe".into(), 1.);
    // Creation-plane is serialized; later owner sleep/wake never migrates this summon.
    if crate::dreamwalking::enabled(r) {
        a.conditions.insert("phantasmiiPlane".into(),
            if crate::dreamwalking::dream_actor(r, &r.player) { 1. } else { 0. });
    }
    let s = &mut a.stats;
    s.offense = integer_stat(s.offense);
    s.defense = integer_stat(s.defense);
    s.accuracy = integer_stat(s.accuracy);
    s.dodge = integer_stat(s.dodge);
    s.dr = integer_stat(s.dr);
    s.perception = integer_stat(s.perception);
    s.max_energy = integer_stat(s.max_energy);
    if let Some(v) = s.max_hp.as_mut() {
        *v = integer_stat(*v)
    }
    for item in r
        .inventory
        .iter()
        .filter(|i| crate::crafting::is_equipped(i))
    {
        if let Some(b) = item.base_bonus.as_ref().or(item.bonus.as_ref()) {
            s.offense += integer_stat(b.offense.value_or(0.));
            s.defense += integer_stat(b.defense.value_or(0.));
            s.accuracy += integer_stat(b.accuracy.value_or(0.));
            s.dodge += integer_stat(b.dodge.value_or(0.));
            s.dr += integer_stat(b.dr.value_or(0.));
            s.perception += integer_stat(b.perception.value_or(0.));
            s.max_energy += integer_stat(b.max_energy.value_or(0.));
            if let Some(v) = b.max_hp.as_ref() {
                s.max_hp = Field::Value(s.max_hp.value_or(0.) + integer_stat(*v));
            }
        }
    }
    let melee = |i: &&crate::Item| {
        i.kind == "weapon"
            && !i
                .ranged_weapon
                .as_ref()
                .is_some_and(|v| matches!(v.as_str(), "bow" | "composite-bow"))
    };
    let main = r
        .inventory
        .iter()
        .filter(melee)
        .find(|i| i.equipped.as_ref().is_some_and(|v| v == "hand1"));
    let off = r
        .inventory
        .iter()
        .filter(melee)
        .find(|i| i.equipped.as_ref().is_some_and(|v| v == "hand2"));
    a.conditions.insert(
        "weaponDamage".into(),
        main.map(|i| i.damage.value_or(30.)).unwrap_or(30.),
    );
    a.conditions.insert(
        "weaponOffenseMultiplier".into(),
        main.map(|i| i.offense_multiplier.value_or(1.))
            .unwrap_or(1.),
    );
    if let Some(i) = off {
        a.conditions
            .insert("offhandDamage".into(), i.damage.value_or(0.));
        a.conditions.insert(
            "offhandOffenseMultiplier".into(),
            i.offense_multiplier.value_or(1.),
        );
    }
    a.max_hp = 100. + 6. * a.stats.defense;
    a.hp = a.max_hp;
    a.energy = a.stats.max_energy;
    a
}
/// O(N) on first shared actor-block mutation, otherwise amortized O(1) append.
/// Birth admission adds O(d) clean, O(A*d + F + H) dirty registry work.
pub fn commit(r: &mut Run, a: Actor) -> Result<(), String> {
    preflight(r)?;
    if a.id != r.next_id { return Err("Mini-Me preview ID no longer matches the allocator".into()); }
    if r.simulation_revision.value_or(29.0)>=50.0 && (!a.abilities.is_empty() || !a.active.is_empty()
        || !a.copied_inventory.is_missing() || !a.ability_reserves.is_missing()) {
        return Err("Revision 50 Mini-Me must contain stats only.".into());
    }
    crate::creature_index::check_next_creature_id(r)?;
    r.next_id+=1.0;
    r.player.conditions.insert("miniMeId".into(), a.id);
    crate::animation_player::local(r,&a,"P62","gear",crate::events::Stage::Special);
    r.actors_at_mut(r.depth).push(a);
    Ok(())
}
/// Every floor is searched, retaining actor order and removing only the recorded ID.
pub fn dismiss(r: &mut Run) {
    if let Some(id) = r.player.conditions.get("miniMeId").copied() {
        if let Some(a)=crate::engine::actor(r,id) {crate::animation_player::local(r,a,"P62","dissolve",crate::events::Stage::Special);}
        let matches = r
            .actors
            .values()
            .flat_map(|v| v.iter())
            .filter(|a| a.id == id)
            .count();
        for _ in 0..matches {
            crate::maintained_links::end_maintained_links_to_dead_target(r, id);
        }
        for block in r.actors.values_mut() {
            if block.iter().any(|a| a.id == id) {
                crate::mutation_journal::make_mut(block).retain(|a| a.id != id);
            }
        }
    }
    r.player.conditions.shift_remove("miniMeId");
}

/// Settle copied-actor crushing without touching the player pool. Falls stay at
/// the scheduler's existing end-turn flight-expiration boundary. O(actor lookup
/// + existing death handling); no AI targeting or independent temporal policy.
pub fn settle_phase_loss(r: &mut Run, id: f64) -> Result<(), crate::engine::EngineError> {
    if r.simulation_revision.value_or(29.0)<50.0 || r.status!="playing" {return Ok(());}
    let Some(a)=crate::engine::actor(r,id).cloned() else {return Ok(());};
    if a.conditions.get("miniMe")!=Some(&1.0) || a.dead.value_or(false) || a.hp<=0.0
        || a.active.iter().any(|s|s=="etill-phase-through") {return Ok(());}
    let wall=crate::dreamwalking::floor_for(r,&a).is_some_and(|f|
        f.tiles.get(a.y as usize*f.width()+a.x as usize).is_some_and(|tile|tile=="wall"));
    if wall {
        crate::resources::record_loss(r,&a,"hp",a.hp.max(0.0));
        crate::engine::actor_mut_trusted(r,id).unwrap().hp=0.0;
        return crate::engine::unpreventable_death(r,id,"phase-crushing");
    }
    Ok(())
}

#[cfg(test)]
mod plane_tests {
    use super::*;
    use crate::dreamwalking as dream;
    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into(); r.actors_at_mut(r.depth).clear();
        r.player.x = 8.; r.player.y = 8.; r.player.conditions.clear();
        let f = r.floor_mut().unwrap(); let w = f.width();
        for y in 7..=9 { for x in 7..=9 { f.tiles[y*w+x] = "floor".into(); } }
        f.environment = Field::Missing;
        r
    }
    #[test]
    fn r34_mini_me_serializes_creation_plane_and_never_copies_body_or_nightmare() {
        let mut r = room();
        let mut initial = r.player.clone();
        for key in ["phantasmiiBody", "phantasmiiNightmare", "phantasmiiPlane"] {
            initial.conditions.insert(key.into(), 1.);
        }
        r.initial_player = initial.into();
        r.dreamwalking = dream::State { active: true, depth: r.depth, ..Default::default() }.into();
        let a = preview(&r, 7., 7.);
        assert!(dream::dream_actor(&r, &a));
        assert!(!dream::is_body(&a)); assert!(!dream::is_nightmare(&a));
        let a: Actor = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        r.dreamwalking.as_mut().unwrap().active = false;
        assert!(dream::dream_actor(&r, &a), "waking never migrates an existing Mini-Me");
        let physical = preview(&r, 7., 7.);
        assert_eq!(physical.conditions.get("phantasmiiPlane"), Some(&0.));
        r.simulation_revision = 33.0.into();
        assert!(!preview(&r, 7., 7.).conditions.contains_key("phantasmiiPlane"));
    }
    #[test]
    fn r34_mini_me_spawn_occupancy_ignores_only_opposite_plane_actors() {
        let mut r = room();
        let mut physical = r.player.clone(); physical.id = 20.; physical.x = 7.; physical.y = 7.;
        r.actors_at_mut(r.depth).push(physical.clone());
        r.dreamwalking = dream::State { active: true, depth: r.depth, ..Default::default() }.into();
        assert_eq!(destination(&r), Some((7., 7.)));
        let mut dream_clone = physical; dream_clone.id = 21.;
        dream_clone.conditions.insert("phantasmiiPlane".into(), 1.);
        r.actors_at_mut(r.depth).push(dream_clone);
        assert_eq!(destination(&r), Some((8., 7.)));
        r.actors_at_mut(r.depth).retain(|a| a.id != 20.);
        r.dreamwalking.as_mut().unwrap().active = false;
        assert_eq!(destination(&r), Some((7., 7.)));
        r.simulation_revision = 33.0.into();
        assert_eq!(destination(&r), Some((8., 7.)));
    }
}

#[cfg(test)]
mod identity_birth_tests {
    use super::*;
    /// O(S) serialized-state comparison for fixed-size rejected birth fixtures.
    #[test]
    fn stale_preview_and_collision_preserve_state() {
        let mut r = Run { depth: 1., next_id: 1., ..Default::default() };
        let before = serde_json::to_value(&r).unwrap();
        assert!(commit(&mut r, Actor { id: 2., ..Default::default() }).is_err());
        assert_eq!(serde_json::to_value(&r).unwrap(), before);
        r.next_id = 0.;
        let before = serde_json::to_value(&r).unwrap();
        assert!(commit(&mut r, Actor { id: 0., ..Default::default() }).is_err());
        assert_eq!(serde_json::to_value(&r).unwrap(), before);
    }
}

#[cfg(test)]
mod revision50_stats_tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.active.clear();r.player.conditions.clear();r.inventory.clear();r
    }
    #[test]
    fn freezes_effective_stats_without_inheriting_powers_items_or_statuses() {
        let mut r=room();r.player.hp=47.0;r.player.energy=123.0;r.player.stats.offense=37.0;
        r.player.abilities=vec![ID.into(),"lichor-hardened-blood".into(),"ursa-waking-nightmares".into()];
        r.player.active=vec!["etill-phase-through".into(),"shrail-grappler".into()];
        for (k,n) in [("hasted",7.0),("buff:offense",3.0),("buffTicks:offense",2.0),("r50PursuitTarget",22.0),("dreamX",5.0)] {r.player.conditions.insert(k.into(),n);}
        r.player.blood=5.0.into();r.player.nightmares=3.0.into();r.player.reactions=2.0.into();
        crate::revision50_ability_energy::initialize_reserve(&mut r.player,ID,30.0,30.0).unwrap();
        r.inventory.push(crate::Item{id:900.0,kind:"ring".into(),equipped:"ring1".to_string().into(),granted_ability:"etill-drown".to_string().into(),
            bonus:crate::PartialStats{offense:9.0.into(),defense:2.0.into(),..Default::default()}.into(),..Default::default()});
        let expected=crate::combat::effective_stats(&r,&r.player);let before=serde_json::to_vec(&r).unwrap();
        assert!(preflight(&r).is_ok());let a=preview(&r,7.0,7.0);
        assert_eq!(a.stats,expected);assert_eq!(a.hp,47.0);assert_eq!(a.max_hp,r.player.max_hp);assert_eq!(a.energy,123.0);
        assert_eq!(crate::combat::effective_stats(&r,&a),expected);
        assert!(a.abilities.is_empty());assert!(a.active.is_empty());assert!(a.ability_reserves.is_missing());
        assert!(a.copied_inventory.is_missing());assert!(a.equipment_abilities.is_missing());assert!(a.ability_order.is_missing());
        assert!(a.blood.is_missing());assert!(a.nightmares.is_missing());assert!(a.reactions.is_missing());assert!(a.damage_bank.is_missing());
        assert!(a.conditions.keys().all(|k|["miniMe","phantasmiiPlane","weaponDamage","weaponOffenseMultiplier"].contains(&k.as_str())));
        assert_eq!(a.conditions.get("weaponDamage"),Some(&45.0));assert_eq!(a.conditions.get("weaponOffenseMultiplier"),Some(&2.0));
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        let id=a.id;let max=a.max_hp;commit(&mut r,a).unwrap();assert_eq!(r.next_id,id+1.0);
        r.inventory.clear();r.player.abilities.clear();r.player.conditions.clear();
        crate::equipment::sync_actor_health(&mut r,id).unwrap();
        let a=crate::engine::actor(&r,id).unwrap();assert_eq!(a.max_hp,max);assert_eq!(a.stats,expected);
        let loaded:Actor=serde_json::from_str(&serde_json::to_string(a).unwrap()).unwrap();assert_eq!(&loaded,a);
    }
    #[test]
    fn owner_phase_through_does_not_allow_spawning_stats_only_copy_in_wall() {
        let mut r=room();r.player.x=8.0;r.player.y=8.0;r.player.active=vec!["etill-phase-through".into()];
        r.actors_at_mut(r.depth).clear();let f=r.floor_mut().unwrap();let width=f.width();
        for y in 7..=9 {for x in 7..=9 {f.tiles[y*width+x]="wall".into();}}
        f.tiles[8*width+8]="floor".into();assert_eq!(destination(&r),None);
        r.floor_mut().unwrap().tiles[9*width+9]="floor".into();assert_eq!(destination(&r),Some((9.0,9.0)));
    }
    #[test]
    fn commit_rejects_power_bearing_r50_copy_without_mutation() {
        let mut r=room();let mut a=preview(&r,7.0,7.0);a.abilities.push("numo-mind-read".into());
        let before=serde_json::to_vec(&r).unwrap();assert!(commit(&mut r,a).is_err());assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}

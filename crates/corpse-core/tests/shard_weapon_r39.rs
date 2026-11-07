use corpse_core::{borge, engine, inventory, storage, Command, Field, Run};

fn room(rev:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.floors.truncate(1);r.actors.retain(|k,_|k=="1");r.ground.retain(|k,_|k=="1");
    r.simulation_revision=rev.into();r.actors_at_mut(1.).clear();r.inventory.clear();r.ground_at_mut(1.).clear();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.x=8.;r.player.y=8.;r.player.hp=100.;r.player.max_hp=100.;
    r.player.energy=200.;r.player.stats.max_energy=200.;r.phase=Field::Missing;r.offers.clear();
    let f=r.floor_mut().unwrap();f.environment=Field::Missing;f.vents=Field::Missing;
    f.permanent_fire=Field::Missing;f.godbeast_event=Field::Missing;f.godbeast=false;f.doors=Field::Missing;
    let w=f.width();let h=f.height();
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();
    r.next_id=1000.;r
}
fn add(r:&mut Run,kind:&str,x:f64,y:f64)->f64 {
    let id=r.next_id;r.next_id+=1.;let mut a=corpse_core::population::create_actor(kind,id,x,y,false).unwrap();
    a.state="hunting".into();r.actors_at_mut(r.depth).push(a);id
}
fn equip(r:&mut Run,slot:&str)->f64 {
    let id=r.next_id;r.next_id+=1.;let mut i=borge::shard(id);i.equipped=slot.to_string().into();r.inventory.push(i);id
}
fn lucky(r:&mut Run){r.player.conditions.insert("lucky".into(),1.);}
#[test]
fn actual_melee_hit_permakills_consumes_and_pairs_stone_with_kill_chronicle() {
    for kind in ["phoenix","banshee","sphinx","dragon"] {
        let mut r=room(39.);let target=add(&mut r,kind,9.,8.);equip(&mut r,"hand1");lucky(&mut r);
        engine::actor_mut(&mut r,target).unwrap().stats.dr=99999.;
        let rng=r.rng;engine::player_attack(&mut r,target).unwrap();assert_ne!(r.rng,rng);
        let a=engine::actor_mut(&mut r,target).unwrap();assert!(a.dead.value_or(false));
        assert!(a.abilities.iter().any(|s|s=="enemy:beast:no-revival"));a.corpse_age=1000000.0.into();
        assert!(!corpse_core::corpses::roll_corpse_revival(a,||panic!("Shard corpse must not roll")));
        corpse_core::corpses::revive_corpse(a,1000001.);assert!(a.dead.value_or(false));
        assert!(r.inventory.is_empty());
        assert_eq!(r.log.iter().filter(|s|s.contains("The corpse blackens and turns to stone.")).count(),1);
        assert!(r.log.iter().filter(|s|s.contains("The corpse blackens")).all(|s|s.starts_with("You kill ")));
        assert_eq!(r.log.iter().filter(|s|*s=="The Shard of the Borge disintegrates.").count(),1);
        assert!(!r.log.iter().any(|s|s.contains("Its victim will not return")));
        storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    }
}
#[test]
fn targeted_apply_is_not_an_accuracy_bypass_and_throw_hit_is_supported() {
    let mut r=room(39.);let target=add(&mut r,"dragon",11.,8.);let shard=equip(&mut r,"hand1");
    let before=r.clone();assert_eq!(inventory::command(&mut r,&Command::UseAt{id:shard,x:11.,y:8.}).unwrap(),Some(false));assert_eq!(r,before);
    lucky(&mut r);assert_eq!(inventory::command(&mut r,&Command::Throw{id:shard,x:11.,y:8.}).unwrap(),Some(true));
    assert!(engine::actor(&r,target).unwrap().dead.value_or(false));
    assert!(!r.inventory.iter().any(borge::is_shard));assert!(!r.ground["1"].iter().any(|g|borge::is_shard(&g.item)));
}
#[test]
fn blocked_or_invalid_commands_do_not_consume_item_or_rng() {
    let mut r=room(39.);let shard=equip(&mut r,"hand1");let before=r.clone();
    assert_eq!(inventory::command(&mut r,&Command::Throw{id:shard,x:f64::NAN,y:8.}).unwrap(),Some(false));assert_eq!(r,before);
    assert_eq!(inventory::command(&mut r,&Command::Shoot{id:shard,x:9.,y:8.}).unwrap(),Some(false));assert_eq!(r,before);
}
#[test]
fn historical_revision_retains_guaranteed_use_and_disallows_throw() {
    let mut r=room(38.);let target=add(&mut r,"dragon",9.,8.);let shard=equip(&mut r,"hand1");
    r.player.conditions.insert("unlucky".into(),1.);let rng=r.rng;
    assert_eq!(inventory::command(&mut r,&Command::Throw{id:shard,x:9.,y:8.}).unwrap(),Some(false));
    assert_eq!(inventory::command(&mut r,&Command::UseAt{id:shard,x:9.,y:8.}).unwrap(),Some(true));
    assert!(engine::actor(&r,target).unwrap().dead.value_or(false));assert_eq!(r.rng,rng);
    assert!(r.log.iter().any(|s|s.starts_with("You kill ")&&s.ends_with("The corpse blackens and turns to stone.")));
}
#[test]
fn ordinary_non_shard_kill_has_no_stone_text() {
    let mut r=room(39.);let target=add(&mut r,"rat",9.,8.);lucky(&mut r);
    r.player.stats.offense=9999.;engine::player_attack(&mut r,target).unwrap();
    assert!(engine::actor(&r,target).unwrap().dead.value_or(false));assert!(!r.log.iter().any(|s|s.contains("turns to stone")));
}

#[test]
fn misses_preserve_the_unique_weapon_and_thrown_miss_can_be_picked_up() {
    let mut r=room(39.);let target=add(&mut r,"dragon",9.,8.);let id=equip(&mut r,"hand1");
    r.player.conditions.insert("unlucky".into(),1.);
    engine::player_attack(&mut r,target).unwrap();
    assert!(r.inventory.iter().any(|i|i.id==id));assert!(!engine::actor(&r,target).unwrap().dead.value_or(false));
    r.player.conditions.insert("unlucky".into(),1.);
    inventory::command(&mut r,&Command::Throw{id,x:9.,y:8.}).unwrap();
    assert!(!r.inventory.iter().any(|i|i.id==id));
    let g=r.ground["1"].iter().find(|g|g.item.id==id).unwrap().clone();
    assert!(g.item.equipped.as_ref().is_none());
    r.player.x=g.x;r.player.y=g.y;inventory::collect_underfoot(&mut r,100.).unwrap();
    assert!(r.inventory.iter().any(|i|i.id==id));
    assert!(!r.log.iter().any(|s|s.contains("disintegrates")||s.contains("turns to stone")));
}
#[test]
fn empty_throw_into_chasm_recovers_on_nearby_surface_without_falling() {
    let mut r=room(39.);let id=equip(&mut r,"hand1");
    let f=r.floor_mut().unwrap();let w=f.width();f.tiles[8*w+11]="chasm".into();
    inventory::command(&mut r,&Command::Throw{id,x:11.,y:8.}).unwrap();
    let g=r.ground["1"].iter().find(|g|g.item.id==id).unwrap();let f=r.floor().unwrap();
    assert!(!["chasm","pit"].contains(&f.tiles[g.y as usize*f.width()+g.x as usize].as_str()));
    assert_eq!(r.floors.len(),1);assert!(!r.inventory.iter().any(|i|i.id==id));
}

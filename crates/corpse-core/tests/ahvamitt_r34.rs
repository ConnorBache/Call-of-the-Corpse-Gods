use corpse_core::{Field, Run, ahvamitt, dreamwalking, engine, generation, storage};
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.floors.truncate(1);r.actors.retain(|k,_|k=="1");r.ground.retain(|k,_|k=="1");r.actors_at_mut(1.).clear();r.ground_at_mut(1.).clear();r.inventory.clear();
    r.simulation_revision=34.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.x=8.;r.player.y=8.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();r.next_id=1000.;r.offers.clear();r.phase=Field::Missing;
    let f=r.floor_mut().unwrap();f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;f.doors=Field::Missing;f.grates=Field::Missing;f.godbeast_event=Field::Missing;f.tiles=vec!["floor".into();2291];f.lights=vec![0.;2291];
    for at in 0..2291 {if at%79==0||at%79==78||at/79==0||at/79==28 {f.tiles[at]="wall".into();}}
    f.tiles[80]="up".into();f.tiles[27*79+77]="down".into();r
}
#[test]
fn two_attacks_relocate_then_third_gives_one_mask_and_disappears() {
    let mut r=room();r.actors_at_mut(1.).push(ahvamitt::actor(10.,9.,8.));
    for (n,message) in [ahvamitt::FIRST,ahvamitt::SECOND,ahvamitt::THIRD].iter().enumerate() {
        let old=engine::actor(&r,10.).unwrap().clone();let hp=old.hp;
        assert!(ahvamitt::attack(&mut r,10.).unwrap());
        if n<2 {let a=engine::actor(&r,10.).unwrap();assert_ne!((a.x,a.y),(old.x,old.y));assert_eq!(a.hp,hp);assert_eq!(a.conditions["ahvamittFinds"],(n+1) as f64);}
        else {assert!(engine::actor(&r,10.).is_none());}
        assert!(r.log.iter().any(|line|line==message));
        if n<2 {assert!(r.ground["1"].is_empty());} else {
            assert_eq!(r.ground["1"].len(),1);let g=&r.ground["1"][0];assert!(ahvamitt::is_mask(&g.item));assert_eq!((g.x,g.y),(old.x,old.y));
        }
    }
    let before=r.clone();assert!(!ahvamitt::attack(&mut r,10.).unwrap());assert_eq!(r,before);
    let loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    assert!(engine::actor(&loaded,10.).is_none());assert_eq!(loaded.ground,r.ground);
}
#[test]
fn carried_mask_teleports_deterministically_and_drop_repickup_cannot_reset_cooldown() {
    let mut r=room();assert!(!ahvamitt::teleport(&mut r).unwrap());r.inventory.push(ahvamitt::mask(100.));
    let mut twin=r.clone();let old=(r.player.x,r.player.y);
    assert!(ahvamitt::teleport(&mut r).unwrap());assert!(ahvamitt::teleport(&mut twin).unwrap());assert_eq!(r,twin);assert_ne!((r.player.x,r.player.y),old);
    assert_eq!(ahvamitt::cooldown(&r),100);let mask=r.inventory.pop().unwrap();assert!(!ahvamitt::has_mask(&r));r.tick+=99.;r.inventory.push(mask);
    let before=r.clone();assert!(!ahvamitt::teleport(&mut r).unwrap());assert_eq!(r,before);assert_eq!(ahvamitt::cooldown(&r),1);
    r.tick+=1.;assert!(ahvamitt::teleport(&mut r).unwrap());assert_eq!(ahvamitt::cooldown(&r),100);
}
#[test]
fn random_destinations_are_standable_vacant_and_do_not_mutate_a_dream_body_anchor() {
    let mut r=room();r.inventory.push(ahvamitt::mask(100.));
    let mut body=r.player.clone();body.id=101.;body.conditions.insert("phantasmiiBody".into(),1.);r.actors_at_mut(1.).push(body);
    r.dreamwalking=dreamwalking::State{active:true,depth:1.,body_id:Some(101.),anchor_x:8.,anchor_y:8.,..Default::default()}.into();
    dreamwalking::ensure_layer(&mut r);
    for _ in 0..8 {
        assert!(ahvamitt::teleport(&mut r).unwrap());let s=r.dreamwalking.as_ref().unwrap();assert_eq!((s.anchor_x,s.anchor_y),(8.,8.));
        let body=engine::actor(&r,101.).unwrap();assert_eq!((body.x,body.y),(8.,8.));
        let f=r.floor().unwrap();assert!(!["wall","tree","pit","chasm"].contains(&f.tiles[r.player.y as usize*f.width()+r.player.x as usize].as_str()));
        r.tick+=100.;
    }
}
#[test]
fn seven_bag_is_one_of_each_before_final_boss_and_legacy33_is_unchanged() {
    for seed in 0..50 {let plan=generation::domain_plan_revision(seed,34.);assert_eq!(plan[..7].iter().map(|e|e.godbeast_kind.as_str()).collect::<std::collections::BTreeSet<_>>(),["Ahvamitt","Borge","Epidite","Mercuria","Phantasmii","Veridian","Zillennia"].into_iter().collect());assert_eq!(plan[7].godbeast_depth,-1);}
    let old=generation::domain_plan_revision(44,33.);assert!(!old.iter().any(|e|e.godbeast_kind=="Ahvamitt"));
}
#[test]
fn actual_attack_attempt_keeps_ahvamitt_alive_and_no_ordinary_attack_roll_is_needed() {
    let mut r=room();r.player.stats.accuracy=-10000.;r.actors_at_mut(1.).push(ahvamitt::actor(10.,9.,8.));
    engine::player_attack(&mut r,10.).unwrap();let a=engine::actor(&r,10.).unwrap();assert_eq!(a.hp,100.);assert_eq!(a.conditions["ahvamittFinds"],1.);
    let before=a.clone();engine::damage_environment(&mut r,10.,10000.,None,"fire",true).unwrap();let a=engine::actor(&r,10.).unwrap();assert_eq!(a.hp,before.hp);
}
#[test]
fn reward_disappearance_revert_and_future_restore_the_exact_branch() {
    use corpse_core::{Command,temporal::Engine};
    let mut r=room();let mut a=ahvamitt::actor(10.,9.,8.);a.conditions.insert("ahvamittFinds".into(),2.);r.actors_at_mut(1.).push(a);
    r.player.energy=2000.;r.player.stats.max_energy=2000.;
    for id in ["numo-revert","morae-see-the-future"] {r.player.abilities.push(id.into());corpse_core::progression::initialize_ability_state(&mut r.player,id,0.).unwrap();}
    r.player.conditions.insert("revertCharges".into(),3.);
    let ability=|id:&str|Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    let mut e=Engine::new(r).unwrap();
    e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();assert!(engine::actor(&e.run,10.).is_none());assert_eq!(e.run.ground["1"].iter().filter(|g|ahvamitt::is_mask(&g.item)).count(),1);
    e.dispatch(&ability("numo-revert")).unwrap();assert_eq!(engine::actor(&e.run,10.).unwrap().conditions["ahvamittFinds"],2.);assert!(e.run.ground["1"].iter().all(|g|!ahvamitt::is_mask(&g.item)));
    e.dispatch(&ability("morae-see-the-future")).unwrap();e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();assert!(engine::actor(&e.run,10.).is_none());
    let mut e=Engine::load(&e.save().unwrap()).unwrap();e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert_eq!(engine::actor(&e.run,10.).unwrap().conditions["ahvamittFinds"],2.);assert!(e.run.ground["1"].iter().all(|g|!ahvamitt::is_mask(&g.item)));
}

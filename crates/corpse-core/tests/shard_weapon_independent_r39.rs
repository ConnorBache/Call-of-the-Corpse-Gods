//! Independent controlled combat-path checks. Not natural-world or balance samples.
use corpse_core::{Command, Field, Item, Run, borge, engine, inventory, storage};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 39.0.into();
    r.floors.truncate(1); r.actors.retain(|k, _| k == "1"); r.ground.retain(|k, _| k == "1");
    r.depth = 1.; r.actors_at_mut(1.).clear(); r.ground_at_mut(1.).clear(); r.inventory.clear();
    r.offers.clear(); r.history.clear(); r.replay.clear(); r.future = None;
    r.phase = Field::Missing; r.haste_snapshot = Field::Missing;
    r.log.clear(); r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.x = 8.; r.player.y = 8.; r.player.hp = 1000.; r.player.max_hp = 1000.;
    r.player.energy = 1000.; r.player.stats.max_energy = 1000.; r.player.stats.accuracy = 10000.;
    r.player.stats.offense = 0.; r.next_id = 1000.; r.rng = 1.;
    let f = r.floor_mut().unwrap();
    f.environment = Field::Missing; f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    f.godbeast_event = Field::Missing; f.godbeast = false; f.doors = Field::Missing;
    f.light_sources = Field::Missing; f.blood = Field::Missing; f.roots = Field::Missing;
    f.fungal_roots = Field::Missing; f.razorleaves = Field::Missing; f.grates = Field::Missing;
    f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    let (w,h) = (f.width(), f.height());
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into(); f.lights.fill(100.);
    r
}
fn enemy(r: &mut Run, kind: &str, x: f64, y: f64) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = corpse_core::population::create_actor_revision(kind,id,x,y,false,39.).unwrap();
    a.state = "hunting".into(); a.hp = 10000.; a.max_hp = 10000.; a.stats.dr = 10000.; a.stats.dodge = -10000.;
    r.actors_at_mut(r.depth).push(a); id
}
fn shard(r: &mut Run, id: f64, hand: Option<&str>) {
    let mut i = borge::shard(id); if let Some(hand)=hand { i.equipped = hand.to_string().into(); } r.inventory.push(i);
}
fn dead(r: &Run, id: f64) -> bool { engine::actor(r,id).is_none_or(|a|a.dead.value_or(false)) }
fn stones(r: &Run) -> usize { r.log.iter().filter(|s|s.contains("The corpse blackens and turns to stone.")).count() }
fn check_stone_chronicle(r: &Run) {
    assert_eq!(stones(r),1,"{:?}",r.log);
    let entry=r.log.iter().find(|s|s.contains("The corpse blackens and turns to stone.")).unwrap();
    assert!(entry.starts_with("You kill "),"stone must share the kill chronicle: {entry}");
    assert!(!r.log.iter().any(|s|s.contains("Its victim will not return.")));
}

#[test]
fn mainhand_ordinary_kill_does_not_spend_offhand_shard_or_add_stone_text() {
    let mut r=room(); let id=enemy(&mut r,"rat",9.,8.);
    let a=engine::actor_mut(&mut r,id).unwrap(); a.hp=1.; a.stats.dr=0.;
    r.inventory.push(Item{id:499.,name:"QA sword".into(),kind:"weapon".into(),hands:1.0.into(),damage:100000.0.into(),offense_multiplier:0.0.into(),equipped:"hand1".to_string().into(),..Default::default()});
    shard(&mut r,500.,Some("hand2"));
    engine::player_attack(&mut r,id).unwrap();
    assert!(dead(&r,id)); assert!(r.inventory.iter().any(|i|i.id==500.)); assert_eq!(stones(&r),0);
}

#[test]
fn two_shards_and_assault_do_not_reuse_consumed_weapon_or_spend_unused_hand() {
    let mut r=room(); let id=enemy(&mut r,"dragon",9.,8.);
    shard(&mut r,500.,Some("hand1")); shard(&mut r,501.,Some("hand2"));
    r.player.abilities.push(corpse_core::abilities::ASSAULT.into()); r.player.active.push(corpse_core::abilities::ASSAULT.into());
    engine::player_attack(&mut r,id).unwrap();
    assert!(dead(&r,id)); assert!(!r.inventory.iter().any(|i|i.id==500.)); assert!(r.inventory.iter().any(|i|i.id==501.));
    check_stone_chronicle(&r);
    assert_eq!(r.log.iter().filter(|s|s.as_str()=="The Shard of the Borge disintegrates.").count(),1);
}

#[test]
fn successful_shard_hit_bypasses_sphinx_immunity_and_death_rescue_and_revival() {
    for kind in ["sphinx","phoenix"] {
        let mut r=room(); let id=enemy(&mut r,kind,9.,8.); shard(&mut r,500.,Some("hand1"));
        let a=engine::actor_mut(&mut r,id).unwrap();
        a.abilities.extend([corpse_core::divine_rescues::RAGE.into(),corpse_core::divine_rescues::OVERFLOWING.into(),corpse_core::divine_rescues::PELIAN.into()]);
        a.active.extend([corpse_core::divine_rescues::RAGE.into(),corpse_core::divine_rescues::PELIAN.into()]); a.blood=10.0.into(); a.energy=1000.;
        engine::player_attack(&mut r,id).unwrap(); assert!(dead(&r,id),"{kind}"); check_stone_chronicle(&r);
        let a=engine::actor_mut(&mut r,id).unwrap(); a.corpse_age=100000.0.into();
        assert!(!corpse_core::corpses::roll_corpse_revival(a,||panic!("permakill cannot roll revival")));
        corpse_core::corpses::revive_corpse(a,100001.); assert!(a.dead.value_or(false));
    }
}

#[test]
fn thrown_shard_through_portal_kills_resolved_actor_once_and_leaves_no_shard() {
    let mut r=room(); let id=enemy(&mut r,"dragon",30.,8.); shard(&mut r,500.,None);
    let w=r.floor().unwrap().width(); r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:8*w+9,b:8*w+30,color_index:0}].into();
    r.player.conditions.insert("lucky".into(),1.);
    assert_eq!(inventory::command(&mut r,&Command::Throw{id:500.,x:9.,y:8.}).unwrap(),Some(true));
    assert!(dead(&r,id)); assert!(!r.inventory.iter().any(borge::is_shard));
    assert!(!r.ground.values().flat_map(|g|g.iter()).any(|g|borge::is_shard(&g.item))); check_stone_chronicle(&r);
}

#[test]
fn dream_plane_target_is_not_harmed_from_waking_plane() {
    let mut r=room(); let id=enemy(&mut r,"rat",9.,8.); shard(&mut r,500.,Some("hand1"));
    engine::actor_mut(&mut r,id).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
    let before=r.clone(); engine::player_attack(&mut r,id).unwrap();
    assert!(!dead(&r,id)); assert_eq!(r.inventory,before.inventory); assert_eq!(r.rng,before.rng); assert_eq!(stones(&r),0);
}

#[test]
fn nightmare_shard_death_has_no_stone_suffix_without_you_kill_chronicle() {
    let mut r=room(); r.dreamwalking=corpse_core::dreamwalking::State{unlocked:true,..Default::default()}.into();
    assert_eq!(corpse_core::dreamwalking::ability(&mut r,corpse_core::dreamwalking::DREAMWALK_ID).unwrap(),Some(true));
    let id=2000.; let mut a=corpse_core::phantasmii_generation::nightmare(1,id,9.,8.);a.state="hunting".into();a.stats.dodge=-10000.;
    r.actors_at_mut(1.).push(a); shard(&mut r,500.,Some("hand1")); r.log.clear();
    engine::player_attack(&mut r,id).unwrap();
    assert!(dead(&r,id)); assert!(!r.inventory.iter().any(borge::is_shard)); assert_eq!(stones(&r),0);
    assert!(r.log.iter().any(|s|s.contains("dissolves"))); assert!(r.ground["1"].is_empty());
}

#[test]
fn saved_permanent_death_and_snapshot_restore_keep_item_identity_consistent() {
    let mut r=room(); let id=enemy(&mut r,"phoenix",9.,8.); shard(&mut r,500.,Some("hand1"));
    let before=corpse_core::temporal::Snapshot::capture(&r); engine::player_attack(&mut r,id).unwrap();
    let mut loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    assert!(dead(&loaded,id)); assert!(!loaded.inventory.iter().any(borge::is_shard));
    assert!(!corpse_core::corpses::can_revive_corpse(engine::actor(&loaded,id).unwrap()));
    before.restore(&mut loaded).unwrap(); assert!(!dead(&loaded,id)); assert_eq!(loaded.inventory.iter().filter(|i|i.id==500.).count(),1);
}

#[test]
fn ordinary_death_never_gets_shard_stone_sentence() {
    let mut r=room(); let id=enemy(&mut r,"rat",9.,8.);
    engine::actor_mut(&mut r,id).unwrap().hp=0.;
    engine::resolve_death(&mut r,id,Some(0.),None,true).unwrap(); assert_eq!(stones(&r),0);
}

#[test]
fn player_counterstrike_uses_its_equipped_mainhand_shard() {
    let mut r=room(); let id=enemy(&mut r,"rat",9.,8.); shard(&mut r,500.,Some("hand1"));
    r.player.abilities.push("pelian-never-without-a-fight".into());
    r.player.active.push("pelian-never-without-a-fight".into()); r.player.conditions.insert("lucky".into(),1.);
    let a=engine::actor_mut(&mut r,id).unwrap(); a.stats.accuracy=10000.; a.stats.offense=0.; a.conditions.insert("weaponDamage".into(),0.);
    engine::attack(&mut r,id,0.,1.0).unwrap();
    assert!(dead(&r,id)); assert!(!r.inventory.iter().any(borge::is_shard)); check_stone_chronicle(&r);
}

#[test]
fn shard_strike_preserves_active_nova_after_attack() {
    let mut r=room(); let id=enemy(&mut r,"rat",9.,8.); shard(&mut r,500.,Some("hand1"));
    r.player.abilities.push("anavani-nova".into()); r.player.active.push("anavani-nova".into());
    let energy=r.player.energy;
    engine::player_attack(&mut r,id).unwrap();
    assert!(dead(&r,id)); assert_eq!(r.player.energy,energy-30.);
    assert!(!r.inventory.iter().any(borge::is_shard)); check_stone_chronicle(&r);
}

#[test]
fn erase_metaphysical_cannot_erase_the_shards_permanent_death_marker() {
    let mut r=room(); let id=enemy(&mut r,"rat",9.,8.); shard(&mut r,500.,Some("hand1"));
    engine::actor_mut(&mut r,id).unwrap().abilities.clear();
    r.player.abilities.push("grisha-erase-metaphysical".into()); r.player.active.push("grisha-erase-metaphysical".into());
    engine::player_attack(&mut r,id).unwrap();
    let a=engine::actor_mut(&mut r,id).unwrap(); assert!(a.dead.value_or(false));
    assert!(a.abilities.iter().any(|s|s=="enemy:beast:no-revival"));
    assert!(!corpse_core::corpses::can_revive_corpse(a));
    a.corpse_age=100000.0.into(); assert!(!corpse_core::corpses::roll_corpse_revival(a,||panic!("permakill marker lost")));
}

#[test]
fn deflected_melee_or_thrown_shard_hits_actual_target_once_without_false_corpse_log() {
    for thrown in [false,true] {
        let mut r=room(); let id=enemy(&mut r,"rat",9.,8.); shard(&mut r,500.,Some("hand1"));
        let a=engine::actor_mut(&mut r,id).unwrap(); a.abilities.push(corpse_core::divine_reactions::DEFLECT.into());
        a.active.push(corpse_core::divine_reactions::DEFLECT.into()); a.reactions=1.0.into();
        r.player.conditions.insert("distracted".into(),1.);
        if thrown {assert_eq!(inventory::command(&mut r,&Command::Throw{id:500.,x:9.,y:8.}).unwrap(),Some(true));}
        else {engine::player_attack(&mut r,id).unwrap();}
        assert_eq!(r.status,"dead"); assert!(!dead(&r,id));
        assert!(!r.inventory.iter().any(borge::is_shard));
        assert!(!r.ground.values().flat_map(|g|g.iter()).any(|g|borge::is_shard(&g.item)));
        assert_eq!(stones(&r),0);
        assert_eq!(r.log.iter().filter(|s|s.as_str()=="The Shard of the Borge disintegrates.").count(),1);
    }
}

#[test]
fn thrown_shard_crosses_random_non_euclidean_branch_and_resolves_one_hit() {
    use corpse_core::non_euclidean_generation::{NonEuclideanGroup as Group,NonEuclideanPocket as Pocket};
    for seed in [1.,8192.] {
        let mut r=room(); r.player.x=11.; r.player.y=10.; r.rng=seed;
        let id=enemy(&mut r,"dragon",9.,10.); let id2=enemy(&mut r,"dragon",9.,11.); shard(&mut r,500.,None);
        let w=r.floor().unwrap().width();let at=|x|10*w+x;
        r.floor_mut().unwrap().non_euclidean=vec![Pocket{cells:vec![at(10),at(11),at(10)+w],groups:vec![Group{sources:vec![at(10),at(10)+w],destination:at(11)}]}].into();
        r.player.conditions.insert("lucky".into(),1.);
        assert_eq!(inventory::command(&mut r,&Command::Throw{id:500.,x:8.,y:10.}).unwrap(),Some(true));
        assert_ne!(dead(&r,id),dead(&r,id2),"seed={seed} ground={:?} logs={:?}",r.ground,r.log); assert!(!r.inventory.iter().any(borge::is_shard));
        assert!(!r.ground.values().flat_map(|g|g.iter()).any(|g|borge::is_shard(&g.item)));
        check_stone_chronicle(&r);
    }
}

#[test]
fn successful_hit_only_policy_preserves_melee_miss_and_recovers_thrown_miss() {
    for thrown in [false,true] {
        let mut r=room(); let id=enemy(&mut r,"rat",9.,8.); shard(&mut r,500.,Some("hand1"));
        r.player.conditions.insert("distracted".into(),1.);
        if thrown {assert_eq!(inventory::command(&mut r,&Command::Throw{id:500.,x:9.,y:8.}).unwrap(),Some(true));}
        else {engine::player_attack(&mut r,id).unwrap();}
        assert!(!dead(&r,id)); assert_eq!(engine::actor(&r,id).unwrap().hp,10000.);
        assert_eq!(stones(&r),0); assert!(!r.log.iter().any(|s|s.contains("disintegrates")));
        if thrown {
            assert!(!r.inventory.iter().any(borge::is_shard));
            let g=r.ground["1"].iter().find(|g|borge::is_shard(&g.item)).unwrap();
            assert_eq!(g.item.id,500.); assert!(g.item.equipped.as_ref().is_none());
            let (x,y)=(g.x,g.y); r.player.x=x; r.player.y=y;
            inventory::collect_underfoot(&mut r,100.).unwrap();
            assert_eq!(r.inventory.iter().filter(|i|i.id==500.).count(),1);
        } else {assert_eq!(r.inventory.iter().filter(|i|i.id==500.).count(),1);}
    }
}

#[test]
fn dream_throw_miss_ignores_waking_actor_preserves_anchor_and_recovers_only_in_dream() {
    use corpse_core::dreamwalking as dream;
    let mut r=room(); let waking=enemy(&mut r,"rat",9.,8.);
    r.dreamwalking=dream::State{unlocked:true,..Default::default()}.into();
    assert_eq!(dream::ability(&mut r,dream::DREAMWALK_ID).unwrap(),Some(true));
    shard(&mut r,500.,Some("hand1"));
    let body=dream::body(&r).unwrap().clone(); let before=corpse_core::temporal::Snapshot::capture(&r);
    let c=Command::Throw{id:500.,x:11.,y:8.}; assert!(dream::blocker(&r,&c).is_none());
    assert_eq!(dream::with_actor(&mut r,0.,|r|inventory::command(r,&c)).unwrap(),Some(true));
    assert_eq!(engine::actor(&r,waking).unwrap().hp,10000.);
    assert_eq!(dream::body(&r).map(|b|(b.id,b.x,b.y)),Some((body.id,body.x,body.y)));
    let g=r.ground["1"].iter().find(|g|borge::is_shard(&g.item)).unwrap().clone();
    assert_eq!((g.x,g.y),(11.,8.)); assert!(dream::dream_ground_item(&g.item));
    assert!(!r.inventory.iter().any(borge::is_shard)); assert_eq!(stones(&r),0);
    let mut loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    assert!(dream::dream_ground_item(&loaded.ground["1"][0].item));
    dream::wake(&mut loaded); loaded.player.x=g.x; loaded.player.y=g.y;
    assert_eq!(inventory::collect_underfoot(&mut loaded,100.).unwrap(),0);
    assert!(dream::ability(&mut loaded,dream::DREAMWALK_ID).unwrap().unwrap());
    assert_eq!(dream::with_actor(&mut loaded,0.,|r|inventory::collect_underfoot(r,100.)).unwrap(),1);
    let i=loaded.inventory.iter().find(|i|i.id==500.).unwrap(); assert!(!dream::dream_ground_item(i));
    before.restore(&mut loaded).unwrap(); assert!(dream::active(&loaded));
    assert_eq!(loaded.inventory.iter().filter(|i|i.id==500.).count(),1);
    assert!(!loaded.ground["1"].iter().any(|g|borge::is_shard(&g.item)));
}

#[test]
fn thrown_shard_hits_only_dream_actor_overlapping_waking_actor() {
    use corpse_core::dreamwalking as dream;
    let mut r=room(); let waking=enemy(&mut r,"rat",9.,8.);
    r.dreamwalking=dream::State{unlocked:true,..Default::default()}.into();
    assert!(dream::ability(&mut r,dream::DREAMWALK_ID).unwrap().unwrap());
    let mut a=corpse_core::phantasmii_generation::nightmare(1,2000.,9.,8.);a.state="hunting".into();a.stats.dodge=-10000.;
    r.actors_at_mut(1.).push(a); shard(&mut r,500.,None);r.player.conditions.insert("lucky".into(),1.);r.log.clear();
    let c=Command::Throw{id:500.,x:9.,y:8.};
    assert_eq!(dream::with_actor(&mut r,0.,|r|inventory::command(r,&c)).unwrap(),Some(true));
    assert!(dead(&r,2000.));assert_eq!(engine::actor(&r,waking).unwrap().hp,10000.);
    assert_eq!(stones(&r),0);assert!(r.log.iter().any(|s|s.contains("dissolves")));
    assert!(!r.inventory.iter().any(borge::is_shard)); assert!(!r.ground["1"].iter().any(|g|borge::is_shard(&g.item)));
}

#[test]
fn command_boundary_equipped_attack_and_dream_throw_replay_deterministically() {
    for dreaming in [false,true] {
        let mut r=room(); shard(&mut r,500.,Some("hand1"));
        let c=if dreaming {
            r.dreamwalking=corpse_core::dreamwalking::State{unlocked:true,..Default::default()}.into();
            assert!(corpse_core::dreamwalking::ability(&mut r,corpse_core::dreamwalking::DREAMWALK_ID).unwrap().unwrap());
            Command::Throw{id:500.,x:11.,y:8.}
        } else {let _=enemy(&mut r,"rat",9.,8.);Command::Move{dx:1.,dy:0.}};
        let mut replayed=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
        assert!(engine::dispatch(&mut r,&c).unwrap().took_turn);
        assert!(engine::dispatch(&mut replayed,&c).unwrap().took_turn);
        assert_eq!(storage::serialize(&r).unwrap(),storage::serialize(&replayed).unwrap());
        assert_eq!(r.replay.last(),Some(&c));
        if dreaming {assert!(r.ground["1"].iter().any(|g|corpse_core::dreamwalking::dream_ground_item(&g.item)));}
        else {assert!(!r.inventory.iter().any(borge::is_shard));check_stone_chronicle(&r);}
    }
}

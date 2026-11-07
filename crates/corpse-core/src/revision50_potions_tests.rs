use super::*;
use crate::{Item,Stats,movement::Point};
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.player.x=10.;r.player.y=10.;
    r.player.hp=100.;r.player.max_hp=100.;r.player.base_max_hp=100.0.into();r.player.energy=100.;
    r.player.stats=Stats{perception:300.,max_energy:100.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;
    r.inventory.clear();r.actors=Default::default();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.next_id=900.;r.phase="normal".to_string().into();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;
    f.environment=Field::Missing;f.permanent_fire=Field::Missing;f.hellfire=Field::Missing;f.ice=Field::Missing;f.snow=Field::Missing;
    r
}
fn creature(r:&mut Run,id:f64,x:f64,y:f64) {
    let a=Actor{id,x,y,name:"Potion test actor".into(),hp:100.,max_hp:100.,base_max_hp:100.0.into(),state:"neutral".into(),stats:Stats{max_energy:100.,..Default::default()},..Default::default()};
    r.actors_at_mut(r.depth).push(a);
}
fn dose(r:&mut Run,effect:&str,x:f64,y:f64,thrown:bool,diluted:bool) {
    r.inventory.push(Item{id:800.,name:"Potion".into(),kind:"potion".into(),glyph:"!".into(),effect:effect.to_string().into(),diluted:diluted.into(),..Default::default()});
    let index=r.inventory.len()-1;assert!(crate::potions::apply(r,index,x,y,thrown).unwrap());
}
#[test] fn exact_twenty_roster_and_legacy_generation_preserved() {
    let expected=ORDINARY.iter().copied().collect::<std::collections::HashSet<_>>();assert_eq!(expected.len(),20);
    for removed in ["offense","defense","accuracy","dodge","detect magic","Veridian Sap"] {assert!(!expected.contains(removed));}
    let mut found=std::collections::HashSet::new();let mut legacy=std::collections::HashSet::new();
    for seed in 1..=80 {
        for revision in [48.,50.] {
            let mut rng=crate::rng::GenerationRng::new(seed);let mut id=100.;
            let loot=crate::equipment::generate_floor_loot_revision(5,&mut rng,&mut id,&[],20,revision).unwrap();
            for item in loot.into_iter().filter(|i|i.kind=="potion") {let effect=item.effect.into_option().unwrap();if revision==50. {assert!(expected.contains(effect.as_str()));found.insert(effect);}else{legacy.insert(effect);}}
        }
    }
    assert_eq!(found.len(),20);for removed in ["offense","defense","accuracy","dodge","detect magic"] {assert!(legacy.contains(removed));}
}
#[test] fn old_revision_rejects_new_effect_before_mutation() {
    let mut r=room();r.simulation_revision=48.0.into();let before=r.clone();let id=r.player.id;
    assert!(apply(&mut r,"fire immunity",Some(id),10.,10.,1.,true).is_err());assert_eq!(r,before);
}
#[test] fn cleansing_removes_only_harmful_temporaries_and_metadata() {
    let mut r=room();
    for key in ["poisoned","poisonSourceId","poisonedFresh","r50PoisonDamage","slowed","slowSourceId","confused","hallucinating","burning","burningOwner","r50Discord","r50Roots","r50RootsX","r50RootsY","r50RootsDepth","r50RootsSource","r50AcidTurns","r50AcidPenalty","venom:offense","venomTurns","grabbed","grabbedBy","unlucky","distracted"] {r.player.conditions.insert(key.into(),3.);}
    let benefits=["hasted","flying","invisible","telepathic","magicSensitive","buff:offense","buffTicks:offense","beastCooldown","monsterSpellReadyAt",FIRE_IMMUNITY];
    for key in benefits {r.player.conditions.insert(key.into(),7.);}
    dose(&mut r,"cleansing",10.,10.,false,false);
    for key in benefits {assert_eq!(engine::condition(&r.player,key),7.,"{key}");}
    assert_eq!(r.player.conditions.len(),benefits.len());
    r.player.abilities=vec!["enemy:beast:everburn".into(),"enemy:beast:fire-immunity".into()];r.player.conditions.insert("burning".into(),1.);
    dose(&mut r,"cleansing",10.,10.,false,false);assert_eq!(engine::condition(&r.player,"burning"),1.);
}
#[test] fn healing_energy_and_sap_keep_ownership_caps_and_dilution() {
    let mut r=room();r.player.hp=10.;r.player.energy=10.;dose(&mut r,"healing",10.,10.,false,false);assert_eq!(r.player.hp,60.);
    dose(&mut r,"energy",10.,10.,false,true);assert_eq!(r.player.energy,35.);
    r.player.abilities.push("grisha-boundless-hunger".into());r.player.energy=120.;dose(&mut r,"energy",10.,10.,false,false);assert_eq!(r.player.energy,170.);
    r.exp=0.;dose(&mut r,"Veridian Sap",10.,10.,false,true);assert_eq!(r.player.hp,80.);assert_eq!(r.player.energy,170.);
}
#[test] fn fire_immunity_blocks_fire_and_new_burning_not_other_damage() {
    let mut r=room();let id=r.player.id;dose(&mut r,"fire immunity",10.,10.,false,false);
    assert_eq!(engine::condition(&r.player,FIRE_IMMUNITY),20.);
    engine::damage_environment(&mut r,id,30.,None,"fire",true).unwrap();assert_eq!(r.player.hp,100.);
    assert_eq!(crate::corpses::apply_burning(&mut r.player,0.,None,None,None),0.);
    engine::damage_environment(&mut r,id,7.,None,"cold",true).unwrap();assert_eq!(r.player.hp,93.);
    crate::revision50_control_amulets::tick_actor(&mut r,id).unwrap();assert_eq!(engine::condition(&r.player,FIRE_IMMUNITY),20.);
    for _ in 0..20 {crate::revision50_control_amulets::tick_actor(&mut r,id).unwrap();}
    assert!(!fire_immune(&r.player));engine::damage_environment(&mut r,id,10.,None,"fire",true).unwrap();assert_eq!(r.player.hp,83.);
}
#[test] fn darkness_exact_square_timer_and_cache_invalidation() {
    let mut r=room();let p=Point{x:10,y:10};let original=crate::lighting_cache::LightingProjection::new(&r).current(p);assert_eq!(original,100.);
    dose(&mut r,"darkness",10.,10.,false,false);assert_eq!(r.floor().unwrap().revision50_darkness.as_ref().unwrap().len(),25);
    for (x,y,value) in [(10,10,50.),(8,8,50.),(7,10,100.),(13,13,100.)] {
        assert_eq!(crate::vision::tile_light(&r,x,y),value);
        assert_eq!(crate::lighting_cache::LightingProjection::new(&r).current(Point{x:x as i32,y:y as i32}),value);
    }
    let tick=r.tick;r.tick+=19.;expire(&mut r);assert_eq!(crate::vision::tile_light(&r,10,10),50.);
    r.tick=tick+20.;expire(&mut r);assert!(r.floor().unwrap().revision50_darkness.is_missing());assert_eq!(crate::lighting_cache::LightingProjection::new(&r).current(p),100.);
}
#[test] fn darkness_reapplication_refreshes_without_stacking() {
    let mut r=room();dose(&mut r,"darkness",10.,10.,false,true);assert_eq!(crate::vision::tile_light(&r,10,10),75.);
    dose(&mut r,"darkness",10.,10.,false,false);assert_eq!(crate::vision::tile_light(&r,10,10),50.);assert_eq!(r.floor().unwrap().revision50_darkness.as_ref().unwrap().len(),25);
}
#[test] fn flooding_restores_exact_ground_and_preserves_structures() {
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[9*w+9]="grass".into();r.floor_mut().unwrap().tiles[9*w+10]="wall".into();r.floor_mut().unwrap().tiles[9*w+11]="down".into();
    dose(&mut r,"flooding",10.,10.,false,false);assert_eq!(r.floor().unwrap().tiles[9*w+9],"shallow-water");assert_eq!(r.floor().unwrap().tiles[9*w+10],"wall");assert_eq!(r.floor().unwrap().tiles[9*w+11],"down");
    r.tick+=20.;crate::revision50_control_amulets::expire_terrain(&mut r);assert_eq!(r.floor().unwrap().tiles[9*w+9],"grass");assert_eq!(r.floor().unwrap().tiles[10*w+10],"floor");
}
#[test] fn freezing_water_persists_until_heat_and_ring_prevents_slow() {
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+10]="water".into();
    creature(&mut r,100.,11.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().copied_inventory=vec![Item{id:90.,kind:"ring".into(),effect:"ice-immunity".to_string().into(),equipped:"ring1".to_string().into(),..Default::default()}].into();
    dose(&mut r,"freezing",10.,10.,false,false);assert_eq!(engine::condition(&r.player,"slowed"),3.);assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),"slowed"),0.);
    assert!(crate::terrain_runtime::ice_at(r.floor().unwrap(),10*w+10));assert!(!crate::terrain_runtime::ice_at(r.floor().unwrap(),10*w+11));
    r.tick+=100.;crate::revision50_control_amulets::expire_terrain(&mut r);assert!(crate::terrain_runtime::ice_at(r.floor().unwrap(),10*w+10));
    crate::environment::ignite(&mut r,10.,10.,Some(8.),None).unwrap();assert!(!crate::terrain_runtime::ice_at(r.floor().unwrap(),10*w+10));assert_eq!(r.floor().unwrap().tiles[10*w+10],"water");
}
#[test] fn repulsion_without_center_keeps_radius_collision_order_and_rng() {
    let mut r=room();r.player.x=5.;r.player.y=5.;
    creature(&mut r,100.,11.,10.);creature(&mut r,101.,12.,10.);creature(&mut r,102.,13.,13.);
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+15]="wall".into();
    let mut reversed=r.clone();let depth=reversed.depth;reversed.actors_at_mut(depth).reverse();
    let rng=r.rng;
    for run in [&mut r,&mut reversed] {
        dose(run,"repulsion",10.,10.,true,false);assert_eq!(run.rng,rng);assert_eq!((run.player.x,run.player.y),(5.,5.));
        for (id,x,y) in [(101.,14.,10.),(100.,13.,10.),(102.,13.,13.)] {
            let a=engine::actor(run,id).unwrap();assert_eq!((a.x,a.y),(x,y));
        }
    }
}
#[test] fn repulsion_drink_and_direct_throw_share_seeded_center_rule() {
    for phase in ["normal","haste"] {for thrown in [false,true] {for diluted in [false,true] {
        let mut r=room();r.phase=phase.to_string().into();r.rng=0x12345678u32 as f64;
        let target=if thrown {r.player.x=6.;creature(&mut r,100.,10.,10.);100.}else{r.player.id};
        let mut expected=r.clone();let d=crate::movement::DIRECTIONS[(crate::rng::random(&mut expected)*8.).floor() as usize];
        r.inventory.push(Item{id:800.,name:"Potion".into(),kind:"potion".into(),glyph:"!".into(),effect:"repulsion".to_string().into(),diluted:diluted.into(),..Default::default()});
        let command=if thrown {crate::Command::Throw{id:800.,x:10.,y:10.}}else{crate::Command::Use{id:800.}};
        let used=if thrown {crate::projectiles::command(&mut r,&command)}else{crate::inventory::command(&mut r,&command)};
        assert_eq!(used.unwrap(),Some(true));assert!(r.inventory.is_empty());
        let steps=if diluted{2.}else{3.};let a=engine::actor(&r,target).unwrap();
        assert_eq!((a.x,a.y),(10.+d.x as f64*steps,10.+d.y as f64*steps),"{phase} thrown={thrown} diluted={diluted}");
        assert_eq!(r.rng,expected.rng);assert!(!a.conditions.contains_key("forcedMove"));
        if thrown {assert_eq!((r.player.x,r.player.y),(6.,10.));}
    }}}
}
#[test] fn repulsion_center_chooses_clear_first_step_then_uses_arrival_and_collision_rules() {
    let mut r=room();let w=r.floor().unwrap().width();
    r.floor_mut().unwrap().tiles.fill("wall".into());
    for x in 10..=12 {r.floor_mut().unwrap().tiles[10*w+x]="floor".into();}
    r.player.conditions.insert(crate::revision50_control_amulets::ROOTS.into(),4.);
    r.player.conditions.insert("forcedMove".into(),7.);
    let mut expected=r.clone();crate::rng::random(&mut expected);
    dose(&mut r,"repulsion",10.,10.,false,false);
    assert_eq!((r.player.x,r.player.y),(12.,10.));assert_eq!(r.rng,expected.rng);
    assert_eq!(engine::condition(&r.player,crate::revision50_control_amulets::ROOTS),0.);
    assert_eq!(engine::condition(&r.player,"forcedMove"),7.);
}
#[test] fn repulsion_center_with_no_valid_direction_consumes_no_rng() {
    for blocker in ["wall","tree","chasm","pit","door","grate","portal","occupant","bulwark"] {
        let mut r=room();r.player.x=5.;r.player.y=5.;creature(&mut r,100.,10.,10.);
        engine::actor_mut_trusted(&mut r,100.).unwrap().state="hunting".into();
        let w=r.floor().unwrap().width();
        for y in 9..=11 {for x in 9..=11 {r.floor_mut().unwrap().tiles[y*w+x]="wall".into();}}
        let at=10*w+11;r.floor_mut().unwrap().tiles[10*w+10]="floor".into();r.floor_mut().unwrap().tiles[at]="floor".into();
        match blocker {
            "wall"|"tree"|"chasm"|"pit"=>r.floor_mut().unwrap().tiles[at]=blocker.into(),
            "door"=>{r.floor_mut().unwrap().doors.get_or_insert_with(Default::default).insert(at.to_string(),crate::FloorDoorsEntry{open:false,burning:Field::Missing});},
            "grate"=>r.floor_mut().unwrap().grates=vec![at as f64].into(),
            "portal"=>r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:at,b:20*w+20,color_index:0}].into(),
            "occupant"=>{creature(&mut r,101.,11.,10.);engine::actor_mut_trusted(&mut r,101.).unwrap().conditions.insert("ahvamitt".into(),1.);},
            "bulwark"=>{let expiry=r.tick+5.;r.floor_mut().unwrap().bulwarks.get_or_insert_with(Default::default).insert(at.to_string(),expiry);},
            _=>unreachable!(),
        }
        let rng=r.rng;dose(&mut r,"repulsion",10.,10.,true,false);
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),(10.,10.),"{blocker}");assert_eq!(r.rng,rng,"{blocker}");
        assert!(!a.conditions.contains_key("forcedMove"));assert!(r.inventory.is_empty());
    }
}
#[test] fn repulsion_immutable_or_other_plane_center_consumes_no_rng() {
    for marker in ["ahvamitt","phantasmiiPlane",crate::revision50_nightmares::MARKER] {
        let mut r=room();r.player.x=5.;r.player.y=5.;creature(&mut r,100.,10.,10.);
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert(marker.into(),1.);
        let rng=r.rng;dose(&mut r,"repulsion",10.,10.,true,false);
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),(10.,10.),"{marker}");assert_eq!(r.rng,rng,"{marker}");
    }
}
#[test] fn repulsion_center_choice_repeats_from_serde_snapshot_and_future_capture() {
    let mut r=room();r.rng=0x12345678u32 as f64;
    let mut restored:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    let snapshot=crate::temporal::Snapshot::capture(&r);
    let future=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    let mut future_state=future.native_state().unwrap().clone();
    dose(&mut r,"repulsion",10.,10.,false,false);let expected=(r.player.x,r.player.y,r.rng);
    for run in [&mut restored,&mut future_state] {dose(run,"repulsion",10.,10.,false,false);assert_eq!((run.player.x,run.player.y,run.rng),expected);}
    snapshot.restore(&mut r).unwrap();dose(&mut r,"repulsion",10.,10.,false,false);
    assert_eq!((r.player.x,r.player.y,r.rng),expected);
}
#[test] fn discord_self_drink_preserves_player_control_and_allegiance() {
    let mut r=room();creature(&mut r,100.,11.,10.);let source=r.player.clone();dose(&mut r,"discord",10.,10.,false,false);
    assert_eq!(engine::condition(&r.player,crate::revision50_control_amulets::DISCORD),10.);assert_eq!((r.player.x,r.player.y),(source.x,source.y));assert_eq!(r.player.state,source.state);
    assert!(crate::revision50_control_amulets::hostile(&r,&r.player,engine::actor(&r,100.).unwrap()));
    dose(&mut r,"discord",11.,10.,true,false);assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),crate::revision50_control_amulets::DISCORD),10.);assert_eq!(engine::actor(&r,100.).unwrap().state,"neutral");
}
#[test] fn teleportation_can_move_drinker_and_uses_serialized_rng() {
    let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles.fill("wall".into());for at in [10*w+10,12*w+12] {r.floor_mut().unwrap().tiles[at]="floor".into();}
    let mut b=r.clone();dose(&mut r,"teleportation",10.,10.,false,false);dose(&mut b,"teleportation",10.,10.,false,false);assert_eq!((r.player.x,r.player.y),(12.,12.));assert_eq!(r.rng,b.rng);assert_eq!(r.depth,b.depth);
}
#[test] fn shattering_scope_preserves_clay_and_radius_and_boundary() {
    let mut r=room();let w=r.floor().unwrap().width();for (x,y) in [(10,10),(10,11),(11,11),(12,12)] {r.floor_mut().unwrap().tiles[y*w+x]="wall".into();}
    r.floor_mut().unwrap().tiles[10*w+9]="tree".into();r.floor_mut().unwrap().tiles[10*w]="wall".into();
    r.floor_mut().unwrap().revision50_terrain=vec![crate::revision50_terrain::Modification{kind:crate::revision50_terrain::Kind::ClaySoldier,cells:vec![crate::revision50_terrain::Cell{x:10,y:11}],previous_tiles:vec!["floor".into()],hp:None}].into();
    let door=(9*w+10).to_string();r.floor_mut().unwrap().doors.get_or_insert_with(Default::default).insert(door.clone(),crate::FloorDoorsEntry{open:false,burning:Field::Missing});r.floor_mut().unwrap().grates=vec![(10*w+11)as f64].into();
    shatter(&mut r,10.,10.);let f=r.floor().unwrap();assert_eq!(f.tiles[10*w+10],"floor");assert_eq!(f.tiles[11*w+10],"wall");assert_eq!(f.tiles[11*w+11],"floor");assert_eq!(f.tiles[12*w+12],"wall");assert_eq!(f.tiles[10*w+9],"floor");assert!(!f.doors.as_ref().unwrap().contains_key(&door));assert!(f.grates.as_ref().unwrap().is_empty());
    shatter(&mut r,0.,10.);assert_eq!(r.floor().unwrap().tiles[10*w],"wall");
}
#[test] fn potion_state_roundtrip_snapshot_and_future_capture() {
    let mut r=room();dose(&mut r,"darkness",10.,10.,false,false);dose(&mut r,"flooding",10.,10.,false,false);dose(&mut r,"freezing",10.,10.,false,false);dose(&mut r,"fire immunity",10.,10.,false,false);
    let original=r.clone();let encoded=serde_json::to_string(&r).unwrap();let restored:Run=serde_json::from_str(&encoded).unwrap();assert_eq!(r,restored);
    let snapshot=crate::temporal::Snapshot::capture(&r);let future=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    r.tick+=100.;crate::revision50_control_amulets::expire_terrain(&mut r);snapshot.restore(&mut r).unwrap();assert_eq!(r.floor().unwrap(),original.floor().unwrap());
    assert_eq!(future.native_state().unwrap().floor().unwrap(),original.floor().unwrap());assert_eq!(future.native_state().unwrap().player.conditions,original.player.conditions);
}
#[test] fn offscreen_expires_darkness_water_and_fire_immunity() {
    let mut r=room();creature(&mut r,100.,15.,14.);dose(&mut r,"fire immunity",15.,14.,true,false);dose(&mut r,"darkness",15.,14.,true,false);dose(&mut r,"flooding",15.,14.,true,false);
    let source=r.depth;r.depth=2.;r.tick+=21.;r.actors_at_mut(2.).clear();crate::offscreen::catch_up_floor(&mut r,source).unwrap();
    let f=r.floors.iter().find(|f|f.depth==source).unwrap();assert!(f.revision50_darkness.is_missing());assert!(f.revision50_temporary_terrain.is_missing());assert!(!fire_immune(r.actors_at(source).iter().find(|a|a.id==100.).unwrap()));
}
#[test] fn cleansing_negative_timed_buffs_settles_health_and_preserves_benefits() {
    let mut r=room();
    for (key,value) in [("buff:defense",-5.),("buffTicks:defense",8.),("buff:maxEnergy",-30.),("buffTicks:maxEnergy",8.),("buff:accuracy",-4.),("buffTicks:accuracy",8.),("buff:offense",6.),("buffTicks:offense",8.),("buff:dodge",-2.)] {r.player.conditions.insert(key.into(),value);}
    crate::equipment::sync_health(&mut r,false);assert_eq!(r.player.max_hp,70.);r.player.hp=40.;r.player.energy=60.;
    dose(&mut r,"cleansing",10.,10.,false,false);
    for stat in ["defense","maxEnergy","accuracy"] {assert!(!r.player.conditions.contains_key(&format!("buff:{stat}")));assert!(!r.player.conditions.contains_key(&format!("buffTicks:{stat}")));}
    assert_eq!(r.player.max_hp,100.);assert_eq!(r.player.hp,70.);assert_eq!(r.player.energy,60.);assert_eq!(crate::combat::effective_stats(&r,&r.player).max_energy,100.);
    assert_eq!(engine::condition(&r.player,"buff:offense"),6.);assert_eq!(engine::condition(&r.player,"buffTicks:offense"),8.);assert_eq!(engine::condition(&r.player,"buff:dodge"),-2.);
    // The bottle is consumed after health settlement. Inventory length is a
    // conservative cache dependency, so one no-op refresh is expected.
    let settled=r.clone();
    assert!(crate::equipment::flush_health(&mut r));
    assert_eq!(r,settled); // No HP, capacity, actor, item, clock or RNG mutation.
    assert!(!crate::equipment::flush_health(&mut r));
}
#[test] fn telepathy_is_floor_wide_for_drinker_and_thrown_target() {
    let mut r=room();creature(&mut r,100.,60.,20.);let w=r.floor().unwrap().width();for y in 0..r.floor().unwrap().height() {r.floor_mut().unwrap().tiles[y*w+40]="wall".into();}
    dose(&mut r,"telepathy",10.,10.,false,false);assert_eq!(engine::condition(&r.player,"telepathic"),30.);assert!(crate::vision::telepathic(&r,engine::actor(&r,100.).unwrap()));
    dose(&mut r,"telepathy",60.,20.,true,false);let other=engine::actor(&r,100.).unwrap();assert_eq!(engine::condition(other,"telepathic"),30.);assert_eq!(crate::vision::detection_chance(&r,other,&r.player),100.);
}
#[test] fn paralysis_impact_five_turns_is_separate_from_gas_refresh() {
    let mut r=room();creature(&mut r,100.,11.,10.);dose(&mut r,"paralysis",11.,10.,true,false);assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),"paralyzed"),5.);
    let f=r.floor().unwrap();let at=10*f.width()+11;assert_eq!(f.environment.as_ref().unwrap().paralysis[at],100.);
}
#[test] fn discord_expires_and_does_not_redirect_player_movement() {
    let mut r=room();creature(&mut r,100.,20.,10.);let player=r.player.id;dose(&mut r,"discord",10.,10.,false,false);
    assert!(engine::move_actor(&mut r,player,0.,1.,true).unwrap());assert_eq!((r.player.x,r.player.y),(10.,11.));
    for _ in 0..11 {crate::revision50_control_amulets::tick_actor(&mut r,player).unwrap();}
    assert!(!crate::revision50_control_amulets::discordant(&r,&r.player));assert!(!crate::revision50_control_amulets::discord_hostility(&r,&r.player,engine::actor(&r,100.).unwrap()));
}
#[test] fn shattering_intersecting_one_cell_destroys_whole_shared_wall() {
    let mut r=room();let w=r.floor().unwrap().width();let cells:Vec<_>=(8..=12).map(|y|crate::revision50_terrain::Cell{x:14,y}).collect();
    for cell in &cells {r.floor_mut().unwrap().tiles[cell.y as usize*w+14]="wall".into();}
    r.floor_mut().unwrap().revision50_terrain=vec![crate::revision50_terrain::Modification{kind:crate::revision50_terrain::Kind::Wall,cells,previous_tiles:vec!["grass".into();5],hp:Some(200.)}].into();
    shatter(&mut r,12.,10.);let f=r.floor().unwrap();
    for y in 8..=12 {assert_eq!(f.tiles[y*w+14],"grass");}
    assert!(f.revision50_terrain.as_ref().unwrap().is_empty());
}
#[test] fn shattering_does_not_rebreak_restored_underlying_wall_in_same_blast() {
    let mut r=room();let w=r.floor().unwrap().width();let cells:Vec<_>=(8..=12).map(|y|crate::revision50_terrain::Cell{x:14,y}).collect();
    for c in &cells {r.floor_mut().unwrap().tiles[c.y as usize*w+14]="wall".into();}
    r.floor_mut().unwrap().revision50_terrain=vec![crate::revision50_terrain::Modification{kind:crate::revision50_terrain::Kind::Wall,cells,previous_tiles:vec!["wall".into();5],hp:Some(200.)}].into();
    shatter(&mut r,14.,10.);let f=r.floor().unwrap();
    for y in 8..=12 {assert_eq!(f.tiles[y*w+14],"wall");}
    assert!(f.revision50_terrain.as_ref().unwrap().is_empty());
}

#[test] fn iceblast_retains_damage_and_wall_shielding_and_leaves_permanent_ice() {
    let mut r=room();let w=r.floor().unwrap().width();let mut item=crate::revision50_amulet_effects::make_amulet(crate::revision50_amulet_effects::Effect::Iceblast,801.).unwrap();item.equipped="amulet".to_string().into();r.inventory.push(item);
    r.floor_mut().unwrap().tiles[9*w+10]="wall".into();r.floor_mut().unwrap().tiles[10*w+9]="wall".into();
    let result=crate::revision50_amulet_effects::command(&mut r,&crate::Command::UseAt{id:801.,x:10.,y:10.}).unwrap();assert_eq!(result,Some(true));assert_eq!(r.player.hp,70.);assert_eq!(engine::condition(&r.player,"slowed"),3.);
    assert!(crate::terrain_runtime::ice_at(r.floor().unwrap(),10*w+10));assert!(crate::terrain_runtime::ice_at(r.floor().unwrap(),11*w+11));assert!(!crate::terrain_runtime::ice_at(r.floor().unwrap(),9*w+9));assert!(!crate::terrain_runtime::ice_at(r.floor().unwrap(),9*w+10));
    r.tick+=50.;crate::revision50_control_amulets::expire_terrain(&mut r);assert!(crate::terrain_runtime::ice_at(r.floor().unwrap(),10*w+10));
}
#[test] fn ice_melting_is_revision_gated_and_preserves_snow_water_underlay() {
    let mut r=room();let at=10*r.floor().unwrap().width()+10;let n=r.floor().unwrap().tiles.len();r.floor_mut().unwrap().ice=vec![false;n].into();r.floor_mut().unwrap().ice.as_mut().unwrap()[at]=true;
    r.simulation_revision=48.0.into();crate::environment::ignite(&mut r,10.,10.,Some(8.),None).unwrap();assert!(crate::terrain_runtime::ice_at(r.floor().unwrap(),at));
    r.simulation_revision=50.0.into();crate::terrain_runtime::melt_burning_snow(&mut r);assert!(!crate::terrain_runtime::ice_at(r.floor().unwrap(),at));assert_eq!(r.floor().unwrap().tiles[at],"floor");
}
#[test] fn drinking_discord_draws_neutral_and_ally_ai_until_expiry() {
    for state in ["ally","neutral"] {
        let mut r=room();creature(&mut r,100.,11.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().state=state.into();
        assert!(!crate::revision50_control_amulets::discord_turn(&mut r,100.,false).unwrap());
        dose(&mut r,"discord",10.,10.,false,false);
        assert!(crate::revision50_control_amulets::discord_turn(&mut r,100.,false).unwrap());
        assert_eq!(engine::actor(&r,100.).unwrap().state,state);assert_eq!((r.player.x,r.player.y),(10.,10.));
        let player=r.player.id;for _ in 0..11 {crate::revision50_control_amulets::tick_actor(&mut r,player).unwrap();}
        assert!(!crate::revision50_control_amulets::discord_turn(&mut r,100.,false).unwrap());
    }
}
#[test] fn drinking_discord_never_crosses_planes_or_turns_shared_body_against_owner() {
    let mut r=room();creature(&mut r,100.,11.,10.);engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
    dose(&mut r,"discord",10.,10.,false,false);assert!(!crate::revision50_control_amulets::discord_turn(&mut r,100.,false).unwrap());
    let mut endpoint=r.player.clone();endpoint.id=101.;endpoint.conditions.insert("halfHereClone".into(),1.);endpoint.conditions.insert("halfHereOwner".into(),r.player.id);
    assert!(!crate::revision50_control_amulets::discord_hostility(&r,&r.player,&endpoint));
}
#[test] fn fire_immunity_does_not_grant_explosion_immunity() {
    let mut r=room();let id=r.player.id;dose(&mut r,"fire immunity",10.,10.,false,false);
    let before=r.player.hp;let blast=crate::combat::explosion_damage(r.player.max_hp);
    engine::damage_environment(&mut r,id,1.,None,"explosion",true).unwrap();
    assert_eq!(r.player.hp,before-blast);assert!(fire_immune(&r.player));
}
#[test] fn cleansing_half_here_settles_owner_capacity_not_unrelated_local_penalties() {
    let mut r=room();
    r.player.abilities.push(crate::half_here::ID.into());r.player.active.push(crate::half_here::ID.into());
    for (key,value) in [("buff:defense",-3.),("buffTicks:defense",8.),("buff:maxHp",-12.),("buffTicks:maxHp",8.),("buff:maxEnergy",-30.),("buffTicks:maxEnergy",8.),("buff:accuracy",-4.),("buffTicks:accuracy",8.),("buff:offense",6.),("buffTicks:offense",8.),("hasted",7.)] {r.player.conditions.insert(key.into(),value);}
    crate::equipment::sync_health(&mut r,false);assert_eq!(r.player.max_hp,70.);r.player.hp=40.;r.player.energy=60.;
    let clone=crate::half_here::create(&mut r,11.,10.).unwrap();
    let endpoint=engine::actor_mut_trusted(&mut r,clone).unwrap();
    endpoint.conditions.insert("buff:accuracy".into(),-5.);endpoint.conditions.insert("buffTicks:accuracy".into(),8.);
    endpoint.conditions.insert("poisoned".into(),4.);endpoint.conditions.insert("poisonSourceId".into(),42.);
    endpoint.conditions.insert("invisible".into(),9.);
    dose(&mut r,"cleansing",11.,10.,true,false);
    for stat in ["defense","maxHp","maxEnergy"] {assert!(!r.player.conditions.contains_key(&format!("buff:{stat}")));assert!(!r.player.conditions.contains_key(&format!("buffTicks:{stat}")));}
    assert_eq!(r.player.max_hp,100.);assert_eq!(r.player.hp,70.);assert_eq!(r.player.energy,60.);
    assert_eq!(engine::condition(&r.player,"buff:accuracy"),-4.);assert_eq!(engine::condition(&r.player,"buffTicks:accuracy"),8.);
    assert_eq!(engine::condition(&r.player,"buff:offense"),6.);assert_eq!(engine::condition(&r.player,"hasted"),7.);
    let endpoint=engine::actor(&r,clone).unwrap();assert_eq!(endpoint.max_hp,100.);assert_eq!(endpoint.hp,70.);assert_eq!(endpoint.energy,60.);
    for key in ["buff:accuracy","buffTicks:accuracy","poisoned","poisonSourceId"] {assert!(!endpoint.conditions.contains_key(key));}
    assert_eq!(engine::condition(endpoint,"invisible"),9.);assert_eq!(crate::combat::effective_stats(&r,endpoint).max_energy,100.);
    // The bottle is consumed after health settlement. Inventory length is a
    // conservative cache dependency, so one no-op refresh is expected.
    let settled=r.clone();
    assert!(crate::equipment::flush_health(&mut r));
    assert_eq!(r,settled); // No HP, capacity, actor, item, clock or RNG mutation.
    assert!(!crate::equipment::flush_health(&mut r));
}

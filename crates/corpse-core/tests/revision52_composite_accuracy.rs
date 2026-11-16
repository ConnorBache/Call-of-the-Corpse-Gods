//! Both bows use normal stats without changing projectile mechanics.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, combat, engine, projectiles, rng};
fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    r.player.stats=Stats{offense:20.,accuracy:15.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.inventory.clear();r.actors_at_mut(r.depth).clear();r.history.clear();
    r.future=None;r.haste_snapshot=Field::Missing;
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);
    f.environment=Field::Missing;f.doors=Field::Missing;f.portals=Field::Missing;
    f.non_euclidean=Field::Missing;f.currents=Field::Missing;
    r.actors_at_mut(r.depth).push(Actor{id:901.,x:11.,y:10.,hp:1000.,max_hp:1000.,
        base_max_hp:1000.0.into(),name:"Target".into(),state:"hunting".into(),
        heartless:true.into(),stats:Stats{dodge:15.,..Default::default()},..Default::default()});r
}
fn add_bow(r:&mut Run,name:&str) {
    let mut bow=corpse_core::equipment::make_equipment(name,950.).unwrap();
    bow.equipped="hand1".to_string().into();r.inventory.push(bow);
    r.inventory.push(Item{id:951.,kind:"ammo".into(),effect:"arrows".to_string().into(),
        quantity:3.0.into(),..Default::default()});
}
fn shoot(r:&mut Run,range:f64)->bool {
    engine::actor_mut(r,901.).unwrap().x=10.+range;
    projectiles::command(r,&Command::Shoot{id:950.,x:10.+range,y:10.}).unwrap();
    engine::actor(r,901.).unwrap().hp<1000.
}
#[test]
fn both_bows_use_normal_ratio_at_every_distance_and_clamp() {
    for name in ["Bow","Composite bow"] { for phase in ["normal","haste"] { for range in [1.,4.,10.] {
        for (accuracy,dodge,chance) in [(15.,15.,0.5),(30.,15.,1.),(100.,15.,1.),
            (0.,15.,0.),(-5.,15.,0.),(1.,0.,0.5),(2.,-5.,1.)] {
            for seed in [1.,1973.,8192.,12345.,50000.,100000.,123456789.] {
                let mut r=room();r.rng=seed;r.phase=phase.to_string().into();
                r.player.stats.accuracy=accuracy;engine::actor_mut(&mut r,901.).unwrap().stats.dodge=dodge;
                add_bow(&mut r,name);let mut oracle=r.clone();
                let expected=rng::random(&mut oracle)<chance;
                assert_eq!(shoot(&mut r,range),expected,"{phase}, range{range}, A{accuracy}, D{dodge}, seed{seed}");
            }
        }
    }}}
}
#[test]
fn both_bows_use_current_accuracy_and_pre52_preserves_old_accuracy() {
    for revision in [51.,52.] { for name in ["Bow","Composite bow"] {
        for range in [1.,4.,10.] { for seed in [1.,1973.,8192.,12345.,50000.,100000.] {
            let mut r=room();r.simulation_revision=revision.into();r.rng=seed;add_bow(&mut r,name);
            let normal=revision>=52.;
            let base=combat::hit_chance_for_run(&r,15.,15.,if normal{0.}else{range});
            let chance=if normal{base}else{1.-(1.-base)*0.5};
            let mut oracle=r.clone();let expected=rng::random(&mut oracle)<chance;
            assert_eq!(shoot(&mut r,range),expected,"rev{revision} {name} range{range}");
        }}
    }}
}
#[test]
fn effective_stats_and_override_priority_match_shared_combat() {
    for case in ["buff","unlucky","lucky","distracted","paralyzed","surprise"] {
        let mut r=room();r.rng=12345.;add_bow(&mut r,"Composite bow");
        r.player.stats.accuracy=0.;
        match case {
            "buff"=>{r.player.conditions.insert("buff:accuracy".into(),30.);},
            "unlucky"=>{r.player.stats.accuracy=100.;r.player.conditions.insert("unlucky".into(),1.);},
            "lucky"=>{r.player.conditions.insert("lucky".into(),1.);},
            "distracted"|"paralyzed"=>{r.player.conditions.insert("lucky".into(),1.);r.player.conditions.insert("distracted".into(),1.);},
            "surprise"=>engine::actor_mut(&mut r,901.).unwrap().state="wandering".into(),_=>{}
        }
        if case=="paralyzed"{engine::actor_mut(&mut r,901.).unwrap().conditions.insert("paralyzed".into(),1.);}
        assert_eq!(shoot(&mut r,10.),matches!(case,"buff"|"lucky"|"paralyzed"),"{case}");
    }
}
#[test]
fn guaranteed_composite_doubles_ordinary_packet_without_extra_rng() {
    let mut r=room();r.rng=12345.;add_bow(&mut r,"Composite bow");
    r.player.conditions.insert("lucky".into(),1.);engine::actor_mut(&mut r,901.).unwrap().x=14.;
    let mut legacy_route=r.clone();
    let base=engine::player_weapon_strike_base(&r,&r.inventory[0]);let player=r.player.id;
    engine::attack_ranged_resolved(&mut legacy_route,player,901.,base,4.,0.5,true).unwrap();
    assert!(shoot(&mut r,4.));
    assert_eq!(1000.-engine::actor(&r,901.).unwrap().hp,2.*(1000.-engine::actor(&legacy_route,901.).unwrap().hp));
    assert_eq!(r.rng,legacy_route.rng);
}

#[test]
fn both_bows_still_consume_ammunition_and_stop_at_walls() {
    for name in ["Bow","Composite bow"] {
        let mut r=room();add_bow(&mut r,name);r.player.conditions.insert("lucky".into(),1.);
        let f=r.floor_mut().unwrap();let at=10*f.width()+12;f.tiles[at]="wall".into();
        assert!(!shoot(&mut r,4.));
        assert_eq!(r.inventory.iter().find(|i|i.id==951.).unwrap().quantity.value_or(0.),2.);
    }
}

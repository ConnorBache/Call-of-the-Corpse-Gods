//! Independent regression coverage for the approved R52 stat/carrying follow-ups.
use corpse_core::{balance52, engine, inventory, population, revision50_inventory as carrying,
    revision50_player_attack as melee, startup, storage, temporal::{Engine, Snapshot},
    Actor, GroundItem, Item, Run};
#[path = "support/balance52_birth.rs"] mod birth;

fn close(a: f64, b: f64) { assert!((a-b).abs()<1e-10, "{a} != {b}"); }
fn heart(id: f64) -> Item {
    let actor=population::create_actor_revision("rat",900.,2.,2.,false,52.).unwrap();
    inventory::heart_for_revision(&actor,id,52.).unwrap()
}
fn carried_hearts(r: &Run) -> f64 {
    r.inventory.iter().filter(|i| i.kind=="heart").map(inventory::quantity).sum()
}

#[test]
fn player_birth_changes_only_dodge_and_does_not_rewrite_saved_stats() {
    let old=startup::new_run_shell_revision("stat-followup", "Unblessed", false,51.).unwrap();
    let new=startup::new_run_shell_revision("stat-followup", "Unblessed", false,52.).unwrap();
    let mut expected=old.player.clone(); expected.stats.accuracy=15.; expected.stats.dodge=15.;
    assert_eq!(new.player,expected);
    let mut r=startup::create_run_revision("stat-followup", "Unblessed",false,52.).unwrap();
    r.player.stats.dodge=10.;
    let mut actor=population::create_actor_revision("rat",900.,3.,3.,false,52.).unwrap();
    actor.stats.dodge=43.; actor.heartless=false.into();
    r.actors_at_mut(r.depth).push(actor);
    let saved=storage::serialize(&r).unwrap();
    let loaded=Engine::load(&saved).unwrap();
    assert_eq!(loaded.run.player.stats.dodge,10.);
    assert_eq!(loaded.run.actors_at(r.depth).last().unwrap().stats.dodge,43.);
    assert!(!loaded.run.actors_at(r.depth).last().unwrap().heartless.value_or(false));
    let snapshot=Snapshot::capture(&r);
    r.player.stats.dodge=100.; r.actors_at_mut(r.depth).last_mut().unwrap().stats.dodge=100.;
    snapshot.restore(&mut r).unwrap();
    assert_eq!(r.player.stats.dodge,10.);
    assert_eq!(r.actors_at(r.depth).last().unwrap().stats.dodge,43.);
}

#[test]
fn innate_base_increases_exactly_twenty_percent_without_scaling_bonuses() {
    for (revision,base) in [(50.,45.),(51.,31.5),(52.,18.9)] {
        close(melee::innate_damage_for_revision(revision),base);
        let mut r=Run::default(); r.simulation_revision=revision.into();
        for offense in [-3.,0.,1.5,20.] {
            close(melee::with_offense_for_run(&r,offense),base+if revision==52. {offense} else {2.*offense});
        }
        let mini=corpse_core::mini_me::preview(&r,3.,3.);
        close(mini.conditions["weaponDamage"],base);
        assert_eq!(mini.conditions["weaponOffenseMultiplier"],if revision==52. {1.} else {2.});
    }
    let bow=Item{kind:"weapon".into(),ranged_weapon:"bow".to_string().into(),damage:15.0.into(),
        offense_multiplier:0.5.into(),enchantment:3.0.into(),..Default::default()};
    let thrown=Item{kind:"weapon".into(),hands:1.0.into(),damage:30.0.into(),
        offense_multiplier:1.0.into(),enchantment:3.0.into(),thrown:true.into(),..Default::default()};
    for revision in [51.,52.] {
        close(engine::player_weapon_strike_base_for_revision(&bow,20.,revision),
            if revision >= 52. {38.9} else {23.5});
        assert_eq!(corpse_core::projectiles::player_throw_base_for_revision(&thrown,20.,revision),if revision==52. {22.} else {44.});
    }
    assert_eq!((31.5_f64*0.75*0.5).round(),12.);
    assert_eq!((31.5_f64*1.25*0.5).round(),20.);
    assert_eq!((37.8_f64*0.75*0.5).round(),14.);
    assert_eq!((37.8_f64*1.25*0.5).round(),24.);
}

#[test]
fn real_melee_uses_new_base_then_existing_variance_mitigation_and_rounding() {
    for revision in [51.,52.] {
        for seed in [1,1973,12345,50000] {
            let mut r=birth::scene(1,revision,seed);
            r.player.stats.offense=20.; r.player.stats.accuracy=1000.;
            r.player.conditions.insert("buff:offense".into(),2.);
            r.inventory.push(Item{id:1000.,kind:"ring".into(),
                bonus:corpse_core::PartialStats{offense:5.0.into(),..Default::default()}.into(),
                ..Default::default()});
            r.actors_at_mut(1.).push(Actor{id:900.,name:"Test target".into(),state:"hunting".into(),
                x:3.,y:2.,hp:1000.,max_hp:1000.,base_max_hp:1000.0.into(),
                stats:corpse_core::Stats{dodge:1.,dr:4.,..Default::default()},..Default::default()});
            let mut forecast=r.clone(); let _=corpse_core::rng::random(&mut forecast);
            let variance=0.75+0.5*corpse_core::rng::random(&mut forecast);
            let expected=if revision==52. { ((18.9+27.)*variance-4.).max(0.).round() }
                else { (((31.5+2.*27.)*variance-4.)*0.5).max(0.).round() };
            let player=r.player.id;
            engine::attack(&mut r,player,900.,1.).unwrap();
            assert_eq!(1000.-engine::actor(&r,900.).unwrap().hp,expected,
                "revision {revision}, seed {seed}");
        }
    }
}

#[test]
fn all_profiled_stats_are_species_fixed() {
    let mut changed=0;
    for row in balance52::profiles() {
        let first=birth::first_dodge_offset(&row.kind, f64::NAN);
        let mut a=Actor::default();
        assert!(balance52::apply_new_actor_kind(&mut a,&row.kind,row.floor as f64,52.));
        assert_eq!(a.stats.dodge,(first+10.).max(10.),"{} floor {}",row.kind,row.floor);
        assert_eq!((a.stats.offense,a.stats.defense,a.stats.accuracy,a.stats.dr),
            (row.offense,row.defense,row.accuracy+15.,row.dr));
        assert_eq!(a.max_hp,100.+6.*row.defense);
        if row.dodge!=first {changed+=1; assert!(first<row.dodge);}
        for depth in [1.,20.,40.] {
            balance52::apply_new_actor_kind(&mut a,&row.kind,depth,52.);
            assert_eq!(a.stats.dodge,(first+10.).max(10.));
        }
    }
    assert_eq!(changed,0);
    for (kind,dodge) in [("rat",10.),("cave-bat",10.),("dragon",balance52::profile("dragon",1.).unwrap().dodge+10.)] {
        let a=population::create_actor_revision(kind,900.,3.,3.,false,52.).unwrap();
        assert_eq!(a.stats.dodge,dodge, "{kind}");
    }
    assert_eq!(balance52::birth_dodge_offset("unknown",52.,-3.),-3.);
    assert_eq!(balance52::birth_dodge_offset("rat",51.,99.),99.);
}

#[test]
fn jackals_are_heartless_at_every_new_birth_but_other_identities_and_legacy_stay_authored() {
    for revision in [51.,52.] {
        for depth in 1..=7 {
            let mut floor=birth::floor(depth); let mut id=900.;
            let mut rng=population::population_random("jackal-heartless");
            let a=population::create_population_actor_revision(&mut id,&mut floor,"pale:Jackal",160,false,&mut rng,revision).unwrap();
            assert_eq!(a.heartless.value_or(false),revision==52.);
        }
        for kind in ["rat","pale:Biter","pale:Greater Jackal","pale:Birther"] {
            if let Ok(a)=population::create_actor_revision(kind,900.,3.,3.,false,revision) {
                assert!(!a.heartless.value_or(false),"{kind}");
            }
        }
    }
    for parent in ["pale:Birther"] {
        let mut jackals=0;
        for seed in 1..=64 {
            let r=birth::summon_once(parent,20,52.,seed,false);
            for child in r.actors_at(r.depth).iter().skip(1) {
                if birth::natural_kind(child)=="pale:Jackal" {
                    jackals+=1; assert!(child.heartless.value_or(false));
                }
            }
        }
        assert!(jackals>0,"{parent} must exercise Jackal birth");
    }
}

#[test]
fn killed_new_jackals_leave_no_heart_but_legacy_jackals_still_do() {
    for revision in [51.,52.] {
        let mut r=birth::scene(1,revision,91);
        let a=population::create_actor_revision("pale:Jackal",900.,3.,3.,false,revision).unwrap();
        r.actors_at_mut(1.).push(a); let player=r.player.id;
        engine::damage_direct(&mut r,900.,10000.,player,"attack",true).unwrap();
        let hearts=r.ground.get("1").unwrap().iter().filter(|g|g.item.kind=="heart").count();
        assert_eq!(hearts,if revision==52. {0}else{1});
    }
}

#[test]
fn heart_capacity_is_ten_with_all_existing_other_category_bonuses_preserved() {
    let abilities=vec!["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into(),"kative-appropriate".into()];
    let old=carrying::capacity_for_abilities_revision(&abilities,51.);
    let new=carrying::capacity_for_abilities_revision(&abilities,52.);
    assert_eq!((old.rings,old.amulets,old.potions,old.hearts),(13.,10.,15.,5.));
    assert_eq!((new.rings,new.amulets,new.potions,new.hearts),(13.,10.,15.,10.));
    assert_eq!(carrying::capacity_for_abilities_revision(&[],52.).total(),25.);
    for revision in [51.,52.] {
        let mut r=birth::scene(1,revision,91);
        let (x,y)=(r.player.x,r.player.y);
        for id in 1000..1011 {
            r.ground_at_mut(1.).push(GroundItem{x,y,item:heart(id as f64),auto_pickup_blocked:Default::default()});
        }
        inventory::collect_underfoot(&mut r,15.).unwrap();
        assert_eq!(carried_hearts(&r),if revision==52.{10.}else{5.});
        assert_eq!(r.ground.get("1").unwrap().len(),if revision==52.{1}else{6});
        assert_eq!(carrying::space_for(&r,&heart(2000.)),0.);
    }
}

#[test]
fn save_admission_and_capacity_jewelry_removal_share_the_new_limit() {
    for (revision,count,valid) in [(51.,5,true),(51.,6,false),(52.,10,true),(52.,11,false)] {
        let mut r=startup::create_run_revision("capacity-followup","Unblessed",false,revision).unwrap();
        r.inventory=(0..count).map(|i|heart(10000.+i as f64)).collect();
        assert_eq!(storage::validate(&r).is_ok(),valid,"revision {revision}, count {count}");
        assert_eq!(Engine::load(&storage::serialize(&r).unwrap()).is_ok(),valid);
    }
    let mut r=birth::scene(1,52.,91);
    r.inventory=(0..10).map(|i|heart(10000.+i as f64)).collect();
    r.inventory.push(Item{id:9000.,kind:"ring".into(),name:"Capacity ring".into(),
        granted_ability:"isheilah-tools-of-the-trade".to_string().into(),..Default::default()});
    r.player.abilities.push("isheilah-tools-of-the-trade".into());
    r.player.equipment_abilities=vec!["isheilah-tools-of-the-trade".into()].into();
    assert!(corpse_core::equipment::removal_blocker(&r,9000.,true).is_none());
}

#[test]
fn heartless_birther_child_log_changes_only_in_revision52() {
    for revision in [51.,52.] {
        let mut r=birth::scene(1,revision,91);
        let mut a=population::create_actor_revision("pale:Jackal",900.,3.,3.,false,revision).unwrap();
        a.heartless=true.into(); a.born_from_birther=true.into();
        r.actors_at_mut(1.).push(a);let player=r.player.id;
        engine::damage_direct(&mut r,900.,10000.,player,"attack",true).unwrap();
        let line=r.log.iter().find(|line|line.contains("collapses and vanishes")).unwrap();
        assert_eq!(line.contains("leaving its heart"),revision<52.);
        assert!(r.ground.get("1").unwrap().iter().all(|g|g.item.kind!="heart"));
    }
}

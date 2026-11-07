//! Cosmetic identities retain stable mechanics, saves and species accounting.
use corpse_core::{Actor, Field, GroundItem, Run, population, revision50_creature_identity as identity};
use std::sync::Arc;

fn actor(kind:&str,revision:f64)->Actor {
    population::create_actor_revision(kind,77.,3.,3.,false,revision).unwrap()
}
#[test]
fn revision50_creature_identity_defaults_and_legacy_versions_are_exact() {
    for (kind,name,glyph,color) in [
        ("blorb","Blorb","b",identity::BLORB_COLOR),
        ("bear","Bear","B",identity::BEAR_COLOR),
        ("dire-bear","Dire Bear","B",identity::DIRE_BEAR_COLOR),
        ("bastidion","Corpse Fungus","F","#9fc875"),
        ("hemogoblin-inventor","Hemogoblin Inventor","h",identity::INVENTOR_COLOR),
        ("yeti","Yeti","Y","#D7E8F1"),
        ("samara","Devil's Samara Tree","T","#8a5a32"),
        ("banshee","Banshee","B","#d600ff"),
        ("vampire","Vampire","V","#D54B5C"),
        ("short-saidri","Short-legged Saidribee","b","#f8d100"),
        ("long-saidri","Long-legged Saidribee","b","#8a5a32"),
        ("saidri-guardian","Saidribee Guardian","B","#f8d100"),
        ("pale:Birther","Pale Birther","b","#b9b8b1"),
        ("pale:Greater Birther","Greater Pale Birther","B","#f0e8dc"),
        ("chimera","Chimera","C","#ddb435"),
        ("sphinx","Sphinx","S","#ffd34a"),
        ("pale:Biter","Pale Chomper","c","#b9b8b1"),
        ("pale:Greater Biter","Greater Pale Chomper","C","#f0e8dc"),
        ("pale:Breather","Pale Gasser","h","#b9b8b1"),
        ("pale:Greater Breather","Greater Pale Gasser","H","#f0e8dc"),
    ] {
        let new=actor(kind,50.);
        assert_eq!((&*new.name,&*new.glyph,&*new.color),(name,glyph,color),"{kind}");
        let legacy=actor(kind,29.);
        let template=&population::catalog().actors[kind];
        assert_eq!((&legacy.name,&legacy.glyph,&legacy.color),(&template.name,&template.glyph,&template.color));
        let mut expected=new.clone(); identity::apply_actor(&mut expected);assert_eq!(new,expected,"idempotent {kind}");
        corpse_core::enemy_traits::initialize_enemy_traits_revision(&mut expected,50.);
        assert_eq!((&expected.name,&expected.glyph,&expected.color),(&new.name,&new.glyph,&new.color),"trait refresh must retain {kind} identity");
    }
}
#[test]
fn revision50_creature_identity_renames_only_cosmetics_and_preserves_custom_appearance() {
    for kind in ["banshee","short-saidri","long-saidri","saidri-guardian","pale:Birther","pale:Greater Birther","chimera","sphinx","blorb","bear","dire-bear","bastidion","hemogoblin-inventor","yeti","samara","pale:Biter","pale:Greater Biter","pale:Breather","pale:Greater Breather"] {
        let old=actor(kind,46.);let mut renamed=old.clone();identity::apply_actor(&mut renamed);
        let mut expected=old.clone();expected.name=renamed.name.clone();expected.glyph=renamed.glyph.clone();expected.color=renamed.color.clone();
        assert_eq!(renamed,expected,"non-cosmetic mutation for {kind}");
        assert_eq!(corpse_core::enemy_traits::intrinsic(&old),corpse_core::enemy_traits::intrinsic(&renamed));
        assert_eq!(corpse_core::corpses::can_revive_corpse(&old),corpse_core::corpses::can_revive_corpse(&renamed));
        let mut custom=old.clone();custom.glyph="X".into();custom.color="#123456".into();identity::apply_actor(&mut custom);
        assert_eq!((&*custom.glyph,&*custom.color),("X","#123456"));
    }
    assert_eq!(identity::display_name("Mercuria’s Greater Pale Biter"),"Mercuria’s Greater Pale Chomper");
    assert_eq!(identity::display_name("Biter-shaped decoration"),"Biter-shaped decoration");
}
#[test]
fn revision50_creature_identity_hearts_keep_rewards_and_species_counters() {
    for kind in ["bastidion","samara","pale:Biter","pale:Greater Biter","pale:Breather","pale:Greater Breather"] {
        let old=actor(kind,46.);let new=actor(kind,50.);
        let mut expected=corpse_core::inventory::heart_for(&old,201.).unwrap();
        let heart=corpse_core::inventory::heart_for(&new,201.).unwrap();
        assert_eq!(corpse_core::inventory::heart_creature_type(&heart),corpse_core::inventory::heart_creature_type(&expected));
        expected.name=identity::display_name(&expected.name).into_owned();assert_eq!(heart,expected);
        let mut inferred=heart.clone();inferred.heart_creature_type=Field::Missing;
        assert_eq!(corpse_core::inventory::heart_creature_type(&inferred),corpse_core::inventory::heart_creature_type(&heart));
    }
    let r=Run {simulation_revision:50.0.into(),..Default::default()};
    for (kind,gas) in [("pale:Breather","caustic"),("pale:Greater Breather","hallucinogenic")] {
        assert_eq!(corpse_core::environment::breather_gas(&r,&actor(kind,50.)),gas);
    }
    let mut possessed=actor("pale:Greater Breather",50.);possessed.name=format!("Mercuria’s {}",possessed.name);
    assert_eq!(corpse_core::environment::breather_gas(&r,&possessed),"hallucinogenic");
    assert!(corpse_core::enemy_traits::intrinsic(&possessed).iter().any(|id|id=="enemy:pale:breath"));
    let mut fungus=actor("bastidion",50.);fungus.dead=true.into();fungus.corpse_age=500.0.into();
    assert!(!corpse_core::corpses::roll_corpse_revival(&mut fungus,||panic!("fungus cannot consume a revival roll")));
}
#[test]
fn revision50_creature_identity_load_covers_all_owned_locations_without_rng_or_history_edits() {
    let old=actor("pale:Biter",46.);let heart=corpse_core::inventory::heart_for(&old,201.).unwrap();
    let ground=GroundItem{x:4.,y:4.,item:heart.clone(),..Default::default()};
    let mut run=Run {simulation_revision:50.0.into(),rng:774.,inventory:vec![heart.clone()],..Default::default()};
    run.actors.insert("1".into(),Arc::new(vec![old.clone()]));
    run.actors.insert("2".into(),Arc::new(vec![actor("bastidion",46.)]));
    run.ground.insert("1".into(),Arc::new(vec![ground.clone()]));
    // Preserve map's concrete serde type without depending on map implementation.
    run.remembered_items=serde_json::from_value(serde_json::json!({"1":[ground.clone()]})).unwrap();
    run.detected_items=serde_json::from_value(serde_json::json!({"1":[ground]})).unwrap();
    let pilot=Box::new(actor("dire-bear",46.));
    run.actors_at_mut(1.)[0].monster_state=corpse_core::ActorMonsterState {pilot:pilot.into(),carried_items:vec![heart].into(),..Default::default()}.into();
    identity::normalize_world(&mut run);assert_eq!(run.rng,774.);
    assert_eq!(run.actors_at(1.)[0].name,"Pale Chomper");assert_eq!(run.actors_at(2.)[0].name,"Corpse Fungus");
    let state=run.actors_at(1.)[0].monster_state.as_ref().unwrap();
    assert_eq!(state.pilot.as_ref().unwrap().glyph,"B");assert_eq!(state.pilot.as_ref().unwrap().color,"#000000");
    assert_eq!(state.carried_items.as_ref().unwrap()[0].name,"Pale Chomper <heart>");
    assert_eq!(run.inventory[0].name,"Pale Chomper <heart>");
    assert_eq!(run.ground["1"][0].item.name,"Pale Chomper <heart>");
    assert_eq!(run.remembered_items.as_ref().unwrap()["1"][0].item.name,"Pale Chomper <heart>");
    assert_eq!(run.detected_items.as_ref().unwrap()["1"][0].item.name,"Pale Chomper <heart>");
    let stable=run.clone();identity::normalize_world(&mut run);assert_eq!(run,stable);
    let mut old_run=Run {simulation_revision:46.0.into(),..Default::default()};old_run.actors.insert("1".into(),Arc::new(vec![old]));
    let before=old_run.clone();identity::normalize_world(&mut old_run);assert_eq!(old_run,before);
}
#[test]
fn vampire_uppercase_identity_preserves_custom_glyph_and_legacy(){
    let legacy=actor("vampire",46.);assert_eq!(legacy.glyph,"v");
    let current=actor("vampire",50.);assert_eq!(current.glyph,"V");
    let mut custom=legacy.clone();custom.glyph="@".into();identity::apply_actor(&mut custom);assert_eq!(custom.glyph,"@");
    let mut restored=legacy;identity::apply_actor(&mut restored);assert_eq!(restored.glyph,"V");
}

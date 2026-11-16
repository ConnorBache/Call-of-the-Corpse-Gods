use corpse_core::{ai, Actor, Item, Run, revision50_rings as rings};
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into(); r.tick=10.;
    r.player.hp=100.; r.player.max_hp=100.; r.player.conditions.clear();
    r.player.abilities.clear(); r.player.active.clear(); r.inventory.clear();
    for actors in r.actors.values_mut() { *actors=Default::default(); }
    for (id, strength) in [(901.,"venom-weak"),(902.,"venom"),(903.,"venom-strong")] {
        std::sync::Arc::make_mut(r.actors.entry(r.depth.to_string()).or_default()).push(Actor {
            id, hp:100., max_hp:100., name:"Venom test".into(),
            abilities:vec![format!("enemy:beast:{strength}")], ..Default::default()
        });
    }
    r
}
fn hit(r:&mut Run, source:f64) {let player=r.player.id; ai::enemy_hit_effects(r,source,player);}
fn tick(r:&mut Run, now:f64) {r.tick=now;let player=r.player.id;ai::enemy_turn_effects(r,player).unwrap();}
fn severity(r:&Run)->f64 {r.player.conditions.iter().filter(|(k,_)|k.starts_with("venom:")).map(|(_,v)|v).sum()}
#[test]
fn severity_adds_across_sources_and_refreshes_every_stack() {
    let mut r=room(52.);hit(&mut r,901.);hit(&mut r,902.);hit(&mut r,903.);
    assert_eq!(severity(&r),7.);assert_eq!(r.player.conditions["envenomed"],5.);
    tick(&mut r,11.);assert_eq!(r.player.hp,93.);
    r.tick=14.;hit(&mut r,901.);assert_eq!(severity(&r),8.);
    assert_eq!(r.player.conditions["envenomed"],5.);
    assert_eq!(r.player.conditions.keys().filter(|k|k.starts_with("venom:")).count(),1);
    // The original dose would have expired at 15. All eight remain through 19.
    tick(&mut r,16.);assert_eq!(r.player.hp,85.);assert_eq!(r.player.conditions["envenomed"],3.);
    tick(&mut r,19.);assert_eq!(r.player.hp,77.);assert_eq!(severity(&r),0.);
    assert!(!r.player.conditions.contains_key("envenomed"));
    tick(&mut r,20.);assert_eq!(r.player.hp,77.);
}
#[test]
fn resistance_rounds_one_combined_packet_and_save_roundtrips() {
    let mut r=room(52.);
    r.inventory.push(Item{id:800.,kind:"ring".into(),name:"Ring of Poison Resistance".into(),effect:rings::POISON_RESISTANCE.to_string().into(),..Default::default()});
    hit(&mut r,901.);r.tick=11.;hit(&mut r,901.);
    let mut r:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    tick(&mut r,12.);
    // Two separate one-point packets would each round 0.5 up to one.
    assert_eq!(r.player.hp,99.);assert_eq!(severity(&r),2.);
}
#[test]
fn existing_r52_buckets_merge_without_extending_expiry_and_dead_source_is_safe() {
    let mut r=room(52.);
    r.player.conditions.insert("venom:12:901".into(),1.);
    r.player.conditions.insert("venom:14:902".into(),2.);
    r.player.conditions.insert("venom:9:903".into(),4.);
    for actors in r.actors.values_mut(){*actors=Default::default();}
    tick(&mut r,11.);assert_eq!(r.player.hp,97.);assert_eq!(severity(&r),3.);
    assert_eq!(r.player.conditions["envenomed"],3.);
    tick(&mut r,13.);assert_eq!(r.player.hp,94.);assert_eq!(severity(&r),3.);
    tick(&mut r,14.);assert_eq!(r.player.hp,91.);assert_eq!(severity(&r),0.);
}
#[test]
fn legacy_r51_keeps_independent_expiries() {
    let mut r=room(51.);hit(&mut r,901.);r.tick=12.;hit(&mut r,902.);
    assert_eq!(r.player.conditions.keys().filter(|k|k.starts_with("venom:")).count(),2);
    tick(&mut r,15.);assert_eq!(r.player.hp,97.);assert_eq!(severity(&r),2.);
    tick(&mut r,16.);assert_eq!(r.player.hp,95.);
}

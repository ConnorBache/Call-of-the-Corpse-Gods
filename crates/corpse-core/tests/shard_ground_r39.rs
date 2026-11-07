use corpse_core::{borge, debug_floor, GroundItem, storage};
fn room() -> corpse_core::Run {
    let mut r=debug_floor::create_run("shard-ground-validation", "Unblessed",1).unwrap();
    r.ground_at_mut(1.).clear(); r.actors_at_mut(1.).clear(); r.inventory.clear();
    r
}
fn put(r:&mut corpse_core::Run) {
    let mut item=borge::shard(900001.); item.dream_ground=true.into();
    let (x,y)=(r.player.x,r.player.y); r.ground_at_mut(1.).push(GroundItem{x,y,item,..Default::default()});
}
#[test]
fn marker_roundtrips_only_on_r39_ground_or_observation_shard() {
    let mut r=room(); put(&mut r);
    let json=storage::serialize(&r).unwrap();
    assert_eq!(storage::serialize(&storage::deserialize(&json).unwrap()).unwrap(),json);
    let mut wrong=r.clone(); wrong.simulation_revision=38.0.into(); assert!(storage::validate(&wrong).is_err());
    let mut wrong=r.clone(); wrong.ground_at_mut(1.)[0].item.dream_ground=false.into(); assert!(storage::validate(&wrong).is_err());
    let mut wrong=r.clone(); wrong.ground_at_mut(1.)[0].item.name="Other weapon".into(); assert!(storage::validate(&wrong).is_err());
    let mut wrong=r.clone(); let item=wrong.ground_at_mut(1.).pop().unwrap().item; wrong.inventory.push(item); assert!(storage::validate(&wrong).is_err());
    let mut wrong=r.clone(); let item=wrong.ground_at_mut(1.).pop().unwrap().item; wrong.player.monster_state.get_or_insert_with(Default::default).carried_items=vec![item].into(); assert!(storage::validate(&wrong).is_err());
}
#[test]
fn waking_zillennia_cannot_erase_a_dream_dropped_shard() {
    let mut r=room(); put(&mut r);
    let mut a=corpse_core::population::zillennia_revision(30,39.).unwrap();
    a.id=900002.;a.x=r.player.x;a.y=r.player.y;
    r.actors_at_mut(1.).push(a);
    let ordinary=borge::shard(900003.);let (x,y)=(r.player.x,r.player.y);
    r.ground_at_mut(1.).push(GroundItem{x,y,item:ordinary,..Default::default()});
    corpse_core::zillennia_ground::on_arrival(&mut r,900002.);
    assert_eq!(r.ground["1"].len(),1);assert!(corpse_core::dreamwalking::dream_ground_item(&r.ground["1"][0].item));
}

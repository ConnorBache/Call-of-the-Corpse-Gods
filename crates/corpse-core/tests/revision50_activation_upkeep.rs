use corpse_core::{abilities, engine, revision50_ability_energy as energy, Command, Field, Run};
fn room(id: &str, reserve: f64, shared: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into(); r.player.abilities = vec![id.into()];
    r.player.active.clear(); r.player.conditions.clear(); r.player.energy = shared;
    r.inventory.clear(); for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.offers.clear(); let depth=r.depth;
    r.actors_at_mut(depth).clear();
    energy::initialize_reserve(&mut r.player,id,50.,reserve).unwrap(); r
}
fn toggle(r: &mut Run,id: &str) {
    abilities::activate(r,id,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
}
fn charge(r: &Run,id: &str)->f64 {r.player.ability_reserves.as_ref().unwrap()[id].energy}
#[test]
fn immediate_upkeep_split_exact_funding_and_repeated_toggles() {
    let id="dawel-shadow-in-the-dark"; let mut r=room(id,1.,1.);
    toggle(&mut r,id); assert_eq!((charge(&r,id),r.player.energy),(0.,0.));
    assert!(r.player.active.iter().any(|s|s==id));
    assert!(!abilities::remove_unfunded(&mut r));
    toggle(&mut r,id); toggle(&mut r,id);
    assert!(r.player.active.iter().any(|s|s==id));
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert!(r.player.active.iter().any(|s|s==id));
    assert_eq!((charge(&r,id),r.player.energy),(0.,0.));
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert!(!r.player.active.iter().any(|s|s==id));
}
#[test]
fn priced_immediate_modes_pay_before_action_and_preserve_serialized_credit() {
    for id in ["etill-phase-through","etill-rushing-torrent","grisha-black-pit", "dawel-shadow-in-the-dark","dawel-not-there","morae-turning-the-wheel","ursa-dream-see","numo-mind-read"] {
        let mut r=room(id,50.,0.); toggle(&mut r,id);
        let expected=50.-abilities::upkeep_cost(id); assert_eq!(charge(&r,id),expected,"{id}");
        let saved=serde_json::to_string(&r).unwrap();
        let mut loaded:Run=serde_json::from_str(&saved).unwrap();
        let actor=loaded.player.id; loaded.tick+=1.;
        abilities::pay_upkeep(&mut loaded,actor).unwrap(); assert_eq!(charge(&loaded,id),expected);
        loaded.tick+=1.; abilities::pay_upkeep(&mut loaded,actor).unwrap();
        assert_eq!(charge(&loaded,id),expected-abilities::upkeep_cost(id));
    }
}
#[test]
fn silence_and_insufficient_energy_reject_before_payment() {
    let id="dawel-shadow-in-the-dark";
    let mut r=room(id,1.,0.); let before=r.player.clone(); toggle(&mut r,id); assert_eq!(r.player,before);
    let mut r=room(id,50.,0.); r.player.conditions.insert("silenced".into(),1.);
    toggle(&mut r,id); assert_eq!(charge(&r,id),50.); assert!(r.player.active.is_empty());
}
#[test]
fn legacy_toggle_cost_and_unpriced_r50_powers_remain_unchanged() {
    let id="dawel-shadow-in-the-dark"; let mut r=room(id,50.,20.); r.simulation_revision=49.0.into();
    toggle(&mut r,id); assert_eq!((charge(&r,id),r.player.energy),(50.,20.));
    for id in ["anavani-nova"] {
        let mut r=room(id,50.,0.); toggle(&mut r,id); assert_eq!(charge(&r,id),50.);
    }
}
#[test]
fn unspeak_retarget_pays_difference_without_extending_coverage() {
    let id="dawel-unspeak"; let mut r=room(id,50.,0.);
    let mut target=r.player.clone(); target.id=100.; target.abilities.clear(); target.active.clear();
    target.conditions.insert("maxAbilityCost".into(),5.); target.x+=1.;
    r.actors_at_mut(r.depth).push(target);
    abilities::activate(&mut r,id,&100.0.into(),&Field::Missing,&Field::Missing).unwrap();
    assert_eq!(charge(&r,id),45.);
    r.tick+=1.;
    toggle(&mut r,id);
    engine::actor_mut(&mut r,100.).unwrap().conditions.insert("maxAbilityCost".into(),8.);
    abilities::activate(&mut r,id,&100.0.into(),&Field::Missing,&Field::Missing).unwrap();
    assert_eq!(charge(&r,id),42.);
    let actor=r.player.id; r.tick+=1.; abilities::pay_upkeep(&mut r,actor).unwrap();
    assert_eq!(charge(&r,id),34.);
}
#[test]
fn future_restores_paid_turn_with_the_paid_reserve() {
    use corpse_core::temporal::{self, History};
    let id="dawel-shadow-in-the-dark"; let mut r=room(id,10.,0.);
    let future="morae-see-the-future"; r.player.abilities.push(future.into());
    energy::initialize_reserve(&mut r.player,future,50.,0.).unwrap();
    toggle(&mut r,id);
    temporal::remember_future(&mut r,&History::default()).unwrap();
    let anchor=r.future.as_ref().unwrap().native_state().unwrap();
    assert_eq!(anchor.player.conditions.get(&format!("upkeepPaidThrough:{id}")),Some(&(r.tick+1.)));
    assert_eq!(anchor.player.ability_reserves.as_ref().unwrap()[id].energy,8.);
    let wire=corpse_core::storage::serialize(&r).unwrap();
    let mut restored=corpse_core::storage::deserialize(&wire).unwrap();
    restored.tick+=3.; restored.player.active.clear();
    restored.player.conditions.clear();
    restored.player.ability_reserves.as_mut().unwrap()[id].energy=1.;
    assert!(temporal::restore_future(&mut restored,&mut History::default()).unwrap());
    assert_eq!(restored.tick,r.tick);
    assert_eq!(charge(&restored,id),8.);
    assert_eq!(abilities::activation_upkeep_due(&restored,id),0.);
    let actor=restored.player.id;restored.tick+=1.;
    abilities::pay_upkeep(&mut restored,actor).unwrap();assert_eq!(charge(&restored,id),8.);
}
fn target(r: &mut Run) -> f64 {
    let mut a=r.player.clone(); a.id=100.; a.x+=1.; a.abilities.clear(); a.active.clear();
    a.conditions.clear(); r.actors_at_mut(r.depth).push(a); 100.
}
fn targeted(r: &mut Run,id: &str,tid: f64) {
    abilities::activate(r,id,&tid.into(),&Field::Missing,&Field::Missing).unwrap();
}
#[test]
fn restored_bonded_combines_activation_and_first_upkeep_atomically() {
    let id="morae-bonded-by-unseen-threads";
    let mut r=room(id,20.,3.); let tid=target(&mut r);
    targeted(&mut r,id,tid);
    assert_eq!((charge(&r,id),r.player.energy),(0.,0.));
    assert!(r.player.active.iter().any(|a|a==id));
    assert_eq!(abilities::activation_upkeep_due(&r,id),0.);
    let actor=r.player.id; r.tick+=1.; abilities::pay_upkeep(&mut r,actor).unwrap();
    assert!(r.player.active.iter().any(|a|a==id));
    r.tick+=1.; abilities::pay_upkeep(&mut r,actor).unwrap();
    assert!(!r.player.active.iter().any(|a|a==id));
    for (reserve,shared,silenced) in [(22.,0.,false),(20.,2.,false),(50.,0.,true)] {
        let mut r=room(id,reserve,shared); let tid=target(&mut r);
        if silenced {r.player.conditions.insert("silenced".into(),1.);}
        let before=r.player.clone(); targeted(&mut r,id,tid); assert_eq!(r.player,before);
    }
}
#[test]
fn bonded_reactivation_pays_activation_again_but_not_same_turn_upkeep() {
    let id="morae-bonded-by-unseen-threads";let mut r=room(id,50.,0.);let tid=target(&mut r);
    targeted(&mut r,id,tid);assert_eq!(charge(&r,id),27.);
    toggle(&mut r,id);targeted(&mut r,id,tid);assert_eq!(charge(&r,id),12.);
}
#[test]
fn restored_i_came_for_you_charges_ten_and_rejects_unfunded_or_silenced() {
    let id="pelian-i-came-for-you";let mut r=room(id,6.,4.);let tid=target(&mut r);
    targeted(&mut r,id,tid);assert_eq!((charge(&r,id),r.player.energy),(0.,0.));
    for (reserve,silenced) in [(9.,false),(50.,true)] {
        let mut r=room(id,reserve,0.);let tid=target(&mut r);
        if silenced {r.player.conditions.insert("silenced".into(),1.);}
        let before=r.player.clone();targeted(&mut r,id,tid);assert_eq!(r.player,before);
    }
}
#[test]
fn restored_telepathy_modes_pay_one_immediately_and_preserve_same_turn_credit() {
    for id in ["ursa-dream-see","numo-mind-read"] {
        let mut r=room(id,0.,1.);toggle(&mut r,id);
        assert_eq!(r.player.energy,0.);assert!(r.player.active.iter().any(|a|a==id));
        toggle(&mut r,id);toggle(&mut r,id);assert!(r.player.active.iter().any(|a|a==id));
        let actor=r.player.id;r.tick+=1.;abilities::pay_upkeep(&mut r,actor).unwrap();
        assert!(r.player.active.iter().any(|a|a==id));
        r.tick+=1.;abilities::pay_upkeep(&mut r,actor).unwrap();assert!(!r.player.active.iter().any(|a|a==id));
        let mut r=room(id,50.,0.);r.player.conditions.insert("silenced".into(),1.);
        toggle(&mut r,id);assert!(r.player.active.is_empty());assert_eq!(charge(&r,id),50.);
    }
}
#[test]
fn haste_dispatch_activation_is_paid_once_before_next_normal_upkeep() {
    let id="ursa-dream-see";let mut r=room(id,5.,0.);
    r.phase="haste".to_string().into();r.player.conditions.insert("hasted".into(),10.);
    let tick=r.tick;
    let cmd=Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    assert!(!engine::dispatch(&mut r,&cmd).unwrap().took_turn);
    assert_eq!((charge(&r,id),r.tick),(4.,tick));
    assert!(!engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    assert_eq!((charge(&r,id),r.tick),(4.,tick));
    assert!(engine::dispatch(&mut r,&Command::Wait).unwrap().took_turn);
    assert_eq!((charge(&r,id),r.tick),(4.,tick+1.));
}
#[test]
fn offscreen_catchup_honors_credit_then_charges_next_normal_tick() {
    let id="dawel-shadow-in-the-dark";let mut r=room(id,5.,0.);toggle(&mut r,id);
    let mut a=r.player.clone();a.id=100.;a.state="ally".into();
    a.conditions.insert("paralyzed".into(),20.);
    let mut floor=r.floor().unwrap().clone();floor.depth=2.;floor.last_tick=r.tick;floor.dirty=true;
    let index=r.floors.iter().position(|f|f.depth==2.).unwrap();
    r.floors[index]=std::sync::Arc::new(floor);
    r.actors_at_mut(2.).clear();r.actors_at_mut(2.).push(a);r.next_id=101.;
    r.tick+=1.;corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    let actor=&r.actors_at(2.)[0];assert_eq!(actor.ability_reserves.as_ref().unwrap()[id].energy,3.);
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(r.actors_at(2.)[0].ability_reserves.as_ref().unwrap()[id].energy,3.);
    r.tick+=1.;corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(r.actors_at(2.)[0].ability_reserves.as_ref().unwrap()[id].energy,1.);
    assert_eq!(charge(&r,id),3.,"inactive actor does not spend player's reserve");
}

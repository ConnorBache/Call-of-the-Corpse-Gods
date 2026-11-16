//! Approved enemy presentation taxonomy. No gameplay state or RNG is touched.
use crate::{Actor, Run, events::{self, EventPoint, Stage}};
/// O(T) trait lookup for T authored traits; constant-size dispatch, no allocation.
pub fn classify(a: &Actor, kind: &str, label: &str) -> Option<(&'static str, &'static str, Stage)> {
    use Stage::*;
    let has = |s: &str| a.abilities.iter().any(|v| v == s);
    Some(match label {
        "Vampire Bat" | "Vampire" => ("E01", "fold-red", Special),
        "Mech activated" => ("E06", "gear", Special),
        _ if label.starts_with("Wendigo level ") => ("E10", "droplet-feast", Special),
        "Fire breath" | "Fire Breath" => ("E03", "fan-orange", Travel),
        "Blood wrest" => ("E04", "root-crimson", Travel),
        "Hemophilia" if has("enemy:beast:bloodmancy") => ("E04", "bolt-crimson", Travel),
        "Vomit blood" => ("E71", "droplet-spray", Health),
        "Charging moonbeam" => ("E49", "spiral-pale", Special),
        "Moonbeam" => ("E49", "bolt-blue", Travel),
        "Distant spit" => ("E61", "glob", Travel),
        "Hornlight" => ("E38", "bolt-ivory", Travel),
        "Water jet" => ("E47", "bolt-blue", Travel),
        "Vanish" if has("enemy:beast:blink") => ("E48", "fold-moon", Travel),
        "Appear" if has("enemy:beast:blink") => ("E48", "unfold-moon", Special),
        "Held" if has("enemy:beast:deep-water") => ("E13", "ripple-water", Special),
        "Held" if has("enemy:beast:hold-only") => ("E50", "bracket", Special),
        "Held" => ("E46", "bracket", Special),
        "Envenomed" if has("enemy:beast:venom-strong") => ("E35", "venom-four", Special),
        "Envenomed" if has("enemy:beast:venom") => ("E34", "venom-two", Special),
        "Envenomed" => ("E33", "venom-one", Special),
        "Healing" if has("enemy:beast:dryad") => ("E17", "leaf-green", Health),
        "Slowed" | "Hemophilia" | "Poisoned" if has("enemy:beast:witch") => ("E08", "bolt", Travel),
        "Ruin pulse" => ("E51", "crossed-ring-violet", Special),
        "Stone seal" | "Stone Seal" => ("E40", "rise-stone", Special),
        "Scream" => ("E65", "sound-pale", Special),
        "Caustic gas" if has("enemy:pale:breath-boost") => ("E57", "puff-green", Special),
        "Hallucinogenic gas" if has("enemy:pale:breath-boost") => ("E57", "puff-violet", Special),
        "Summon Saidribee" => ("E66", "summon-honey", Special),
        "Summon" if has("enemy:beast:witch") => ("E08", "summon-violet", Special),
        "Mercuria’s well" => ("E78", "wave-ichor", Special),
        "Shard of the Borge" => ("E74", "diamond-amber", Special),
        _ if label.starts_with("Birth ") => ("E63", "membrane-ivory", Special),
        _ if kind == "summon" && has("enemy:beast:ichor-bound") => ("E79", "summon-ichor", Special),
        _ => return None,
    })
}
/// Local exact-outcome mark. Visibility/plane/apparent-identity admission is central.
pub fn mark(r: &Run, a: &Actor, key: &str, style: &str, stage: Stage) {
    events::cue(r, a, key, style, stage, &[EventPoint::from(a)]);
}
/// O(1) visual gas state, without allocating or observing any neighboring cell.
pub fn gas_cell(r:&Run, cell:usize)->(Option<&str>,f64) {
    let environment=r.floor().and_then(|f|f.environment.as_ref());
    (environment.and_then(|e|e.gas_type.as_ref()).and_then(|v|v.get(cell)).and_then(|v|v.as_deref()),
     environment.and_then(|e|e.gas_volume.as_ref()).and_then(|v|v.get(cell)).copied().unwrap_or(0.))
}
/// The caller snapshots only the old type match and volume before deposit.
/// An accepted zero/unchanged deposit is not a new cloud. Actual propagation
/// is not re-run or extrapolated for presentation.
pub fn gas_deposited(r:&Run,a:&Actor,cell:usize,before:(bool,f64),gas:&str,key:&str) {
    let (after_type,after_volume)=gas_cell(r,cell);
    if after_type!=Some(gas) || after_volume<=0. || (before.0 && before.1==after_volume) {return;}
    let Some(f)=r.floor() else{return};
    events::cue(r,a,key,if gas=="hallucinogenic"{"ripple-exhale-violet"}else{"ripple-exhale-green"},Stage::Special,
        &[EventPoint{x:(cell%f.width())as f64,y:(cell/f.width())as f64}]);
}
/// Optional rejection glints/static classifications deliberately have no idle emitter.
pub const STATIC_ONLY: &[&str] = &["E15","E21","E24","E25","E26","E27","E28","E29","E30","E32","E36","E41","E43","E44","E54","E55","E68","E69"];
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn venom_strength_is_one_special_not_extra_attacks() {
        for (trait_, key) in [("venom-weak","E33"),("venom","E34"),("venom-strong","E35")] {
            let a=Actor { abilities:vec![format!("enemy:beast:{trait_}")], ..Default::default() };
            let (actual,_,stage)=classify(&a,"status","Envenomed").unwrap();
            assert_eq!(actual,key); assert_eq!(stage,Stage::Special);
        }
    }
    #[test] fn static_and_unrelated_events_do_not_fabricate_casts() {
        let a=Actor::default();
        for label in ["Recovering","Stalking: waiting","Fast","Flying","Fire Immunity"] {assert!(classify(&a,"status",label).is_none());}
        assert!(STATIC_ONLY.contains(&"E69"));
    }
}

/// Called only after an actual positive packet; never on immune contact.
pub fn damage(r: &Run, a: &Actor, kind: &str) {
    let (key,style)=match kind {
        "roots" => ("A10","root-tighten"),
        "poison"|"poisoned"|"venom"|"envenomed" => ("E71","droplet-green"),
        "bleeding" | "killspawnBleed" => ("E71","droplet-red"),
        // The actual vomiting episode has its own enriched event, even if mitigated.
        "hemophilia" => return,
        "fire"|"burning" => ("E71","ember"),
        "razorleaf"|"razorleaves" => ("E42","leaf-slash"),
        "caustic"|"breatherGas"|"gas"|"ichor" => ("E72","droplet"),
        _ => return,
    };
    mark(r,a,key,style,Stage::Health);
}
/// Called after terminal death admission, never before rescue resolution.
pub fn death(r: &Run, a: &Actor) {
    mark(r,a,if crate::monster_inventor::is_mech(a){"E06"}else{"E70"},"collapse",Stage::Death);
}
/// Actual state transition only; blocked movement never receives a speed trail.
pub fn moved(r: &Run, before: &Actor, after: &Actor) {
    if before.x==after.x && before.y==after.y {return;}
    if before.abilities.iter().any(|s|s=="enemy:beast:minotaur") {
        events::cue(r,after,"E09","speed-dust",Stage::Travel,&[before.into(),after.into()]);
    }
    let submerged=|a:&Actor| crate::engine::condition(a,"submergedStealth")>0. || crate::engine::condition(a,"underwater")>0.;
    if submerged(before)!=submerged(after) {mark(r,after,"E45","ripple",Stage::Special);}
}
pub fn strike(r: &Run, a: &Actor, b: &Actor, hit: bool, damage: f64) {
    if hit && damage>0. && a.abilities.iter().any(|s|s=="enemy:beast:armor-piercing") {
        mark(r,b,"E18","puncture",Stage::Special);
    }
}
/// Actual increases only; caller selects provenance rather than inferring from species.
pub fn healed(r:&Run,before:&Actor,after:&Actor,key:&str) {
    if after.hp>before.hp && !after.dead.value_or(false) {
        mark(r,after,key,if key=="E07"{"stitch"}else{"droplet"},Stage::Health);
    }
}
pub fn revived(r:&Run,a:&Actor) {
    let phoenix=a.abilities.iter().any(|s|s=="enemy:beast:phoenix");
    mark(r,a,if phoenix{"E16"}else{"E70"},if phoenix{"ember-rise"}else{"rise"},Stage::Death);
}

#[cfg(test)] mod outcome_tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let f=r.floor_mut().unwrap();let n=f.tiles.len();f.tiles=vec!["floor".into();n];f.lights=vec![100.;n];r
    }
    #[test] fn healing_marks_only_real_increases_and_preserves_run() {
        let r=room();let before=r.player.clone();let mut after=before.clone();
        let snapshot=r.clone(); events::begin();healed(&r,&before,&after,"E70");assert!(events::finish().is_empty());
        after.hp+=1.;events::begin();healed(&r,&before,&after,"E70");let out=events::finish();
        assert_eq!(out.len(),1);assert_eq!(out[0].animation.as_ref().unwrap().proposal,"E70");assert_eq!(r,snapshot);
        after.dead=true.into();events::begin();healed(&r,&before,&after,"E70");assert!(events::finish().is_empty());
    }
    #[test] fn blocked_movement_and_legacy_runs_have_no_added_cues() {
        let mut r=room();let mut a=r.player.clone();a.abilities.push("enemy:beast:minotaur".into());
        events::begin();moved(&r,&a,&a);assert!(events::finish().is_empty());
        r.simulation_revision=51.0.into();events::begin();death(&r,&a);damage(&r,&a,"fire");assert!(events::finish().is_empty());
    }
    #[test] fn hidden_and_offscreen_bodies_never_receive_identity_cues() {
        let mut r=room();let mut a=r.player.clone();a.id=999.;a.x=-100.;a.y=-100.;
        events::begin();mark(&r,&a,"E01","fold",Stage::Special);assert!(events::finish().is_empty());
        r.player.x=-1.;events::begin();mark(&r,&r.player,"E71","droplet",Stage::Health);assert!(events::finish().is_empty());
    }
    #[test] fn cancer_miss_and_zero_damage_do_not_puncture() {
        let r=room();let mut a=r.player.clone();a.abilities.push("enemy:beast:armor-piercing".into());
        events::begin();strike(&r,&a,&r.player,false,10.);strike(&r,&a,&r.player,true,0.);assert!(events::finish().is_empty());
        events::begin();strike(&r,&a,&r.player,true,1.);assert_eq!(events::finish()[0].animation.as_ref().unwrap().proposal,"E18");
    }
    #[test] fn sphinx_provocation_is_once_without_simulation_difference() {
        let mut r=room();
        let a=crate::population::create_actor_revision("sphinx",900.,4.,2.,false,52.).unwrap();
        r.actors_at_mut(r.depth).push(a);
        let source=r.player.clone();
        events::begin();assert!(crate::monster_sphinx::provoked_with_source(&mut r,900.,Some(&source)));
        let out=events::finish();assert_eq!(out.iter().filter(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E12")).count(),1);
        events::begin();assert!(crate::monster_sphinx::provoked_with_source(&mut r,900.,Some(&source)));assert!(events::finish().is_empty());
    }
    #[test] fn zillennia_erase_cue_requires_actual_removed_contents() {
        let mut r=room();
        let mut a=r.player.clone();a.id=901.;a.x=4.;a.y=2.;a.creature_kind="zillennia".to_string().into();a.zillennia_quality="greater".to_string().into();
        r.actors_at_mut(r.depth).push(a.clone());
        events::begin();assert!(!crate::zillennia_ground::on_arrival(&mut r,a.id));assert!(events::finish().is_empty());
        let f=r.floor_mut().unwrap();let at=2*f.width()+4;f.tiles[at]="grass".into();
        events::begin();assert!(crate::zillennia_ground::on_arrival(&mut r,a.id));let out=events::finish();
        assert_eq!(out.iter().filter(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E81")).count(),1);
        events::begin();assert!(!crate::zillennia_ground::on_arrival(&mut r,a.id));assert!(events::finish().is_empty());
    }

    #[test] fn queen_success_emits_at_newborn_failed_and_cooldown_casts_are_quiet() {
        let mut r=room();r.actors_at_mut(r.depth).clear();r.next_id=1000.;
        let mut queen=crate::population::create_actor_revision("saidribee-queen",900.,4.,2.,false,52.).unwrap();
        queen.state="hunting".into();queen.energy=100.;r.actors_at_mut(r.depth).push(queen);
        let player=r.player.id;
        events::begin();assert!(crate::saidribee_queen::summon(&mut r,900.,player).unwrap());
        let out=events::finish();let child=crate::engine::actor(&r,1000.).unwrap();
        let event=out.iter().find(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E66")).unwrap();
        assert_eq!(event.to,EventPoint::from(child));
        let snapshot=r.clone();events::begin();assert!(!crate::saidribee_queen::summon(&mut r,900.,player).unwrap());
        assert!(events::finish().is_empty());assert_eq!(r,snapshot);
        crate::engine::actor_mut_trusted(&mut r,900.).unwrap().conditions.shift_remove("monsterSummonReadyAt");
        let f=r.floor_mut().unwrap();let w=f.width();for y in 1..=3 {for x in 3..=5 {if x!=4 || y!=2 {f.tiles[y*w+x]="wall".into();}}}
        let snapshot=r.clone();events::begin();assert!(!crate::saidribee_queen::summon(&mut r,900.,player).unwrap());
        assert!(events::finish().is_empty());assert_eq!(r,snapshot);
    }
    #[test] fn dryad_full_health_is_quiet_actual_healing_has_one_grove_event() {
        let mut r=room();r.actors_at_mut(r.depth).clear();r.player.hp=r.player.max_hp;
        let mut dryad=crate::population::create_actor_revision("dryad",900.,4.,2.,false,52.).unwrap();
        dryad.state="ally".into();dryad.energy=100.;r.actors_at_mut(r.depth).push(dryad);
        events::begin();assert!(!crate::monster_spells::heal_others(&mut r,900.).unwrap());assert!(events::finish().is_empty());
        r.player.hp-=25.;let before=r.player.hp;
        events::begin();assert!(crate::monster_spells::heal_others(&mut r,900.).unwrap());let out=events::finish();
        assert!(r.player.hp>before);assert_eq!(out.iter().filter(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E17" && v.style=="stitch-green")).count(),1);
        let link=out.iter().find(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E17" && v.style=="leaf-green")).unwrap();
        assert_eq!(link.from,Some(EventPoint{x:4.,y:2.}));assert_eq!(link.to,EventPoint::from(&r.player));
    }

    #[test] fn paid_exhalation_marks_only_committed_deposit_for_both_gas_types() {
        for (kind,gas,style) in [("pale:Breather","caustic","ripple-exhale-green"),("pale:Greater Breather","hallucinogenic","ripple-exhale-violet")] {
            let mut r=room();r.actors_at_mut(r.depth).clear();
            let mut a=crate::population::create_actor_revision(kind,900.,4.,2.,false,52.).unwrap();
            a.state="hunting".into();a.energy=100.;r.actors_at_mut(r.depth).push(a.clone());
            let player=r.player.id;let w=r.floor().unwrap().width();let cell=2*w+4;
            events::begin();assert!(crate::ai::enemy_action_options(&mut r,900.,player,false).unwrap());
            let out=events::finish();let cues:Vec<_>=out.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="E57").collect();
            assert_eq!(cues.len(),1);assert_eq!(cues[0].style,style);
            assert_eq!(cues[0].cells,vec![EventPoint{x:4.,y:2.}]);
            assert_eq!(gas_cell(&r,cell).0,Some(gas));assert!(gas_cell(&r,cell).1>0.);
            for neighbor in [cell-1,cell+1,cell-w,cell+w] {assert_eq!(gas_cell(&r,neighbor).1,0.);}
        }
    }
    #[test] fn rejected_and_unchanged_gas_deposits_have_no_exhalation_cue() {
        let mut r=room();r.actors_at_mut(r.depth).clear();
        let mut a=crate::population::create_actor_revision("pale:Breather",900.,4.,2.,false,52.).unwrap();
        a.state="hunting".into();a.energy=100.;a.conditions.insert("monsterGasBoostVolume".into(),0.);
        r.actors_at_mut(r.depth).push(a.clone());let player=r.player.id;
        events::begin();assert!(crate::ai::enemy_action_options(&mut r,900.,player,false).unwrap());
        assert!(events::finish().iter().all(|e|e.animation.as_ref().is_none_or(|a|a.proposal!="E57")));
        let cell=2*r.floor().unwrap().width()+4;
        crate::environment::add_gas(r.floor_mut().unwrap(),cell,"caustic",80.,900.).unwrap();
        events::begin();gas_deposited(&r,&a,cell,(true,80.),"caustic","E57");assert!(events::finish().is_empty());
    }

    #[test] fn yeti_contact_observation_survives_duration_tick_and_rearms_on_exit() {
        let mut r=room();r.actors_at_mut(r.depth).clear();
        let yeti=crate::population::create_actor_revision("yeti",900.,3.,2.,false,52.).unwrap();
        r.actors_at_mut(r.depth).push(yeti);
        assert!(crate::monster_chill::observed_chilled_ids(&r).is_empty());
        crate::monster_chill::apply_adjacency(&mut r);
        assert_eq!(crate::monster_chill::observed_chilled_ids(&r),vec![r.player.id]);
        crate::engine::tick_condition(&mut r.player,"slowed");
        assert_eq!(crate::engine::condition(&r.player,"slowed"),0.);
        let before=crate::monster_chill::observed_chilled_ids(&r);
        events::begin();crate::monster_chill::apply_adjacency(&mut r);
        assert_eq!(crate::monster_chill::observed_chilled_ids(&r),before);
        assert!(events::finish().iter().all(|e|e.animation.as_ref().is_none_or(|a|a.proposal!="E14")));
        r.player.x=8.;assert!(crate::monster_chill::observed_chilled_ids(&r).is_empty());
        r.player.x=2.;crate::monster_chill::apply_adjacency(&mut r);
        assert_eq!(crate::monster_chill::observed_chilled_ids(&r),vec![r.player.id]);
    }
    #[test] fn ruin_pulse_receipt_has_exact_visible_radius_and_blocker_clipping() {
        let mut r=room();r.actors_at_mut(r.depth).clear();r.player.hp=1000.;r.player.max_hp=1000.;
        let mut source=r.player.clone();source.id=900.;source.x=4.;source.y=2.;source.state="hunting".into();source.energy=100.;
        source.abilities=vec!["enemy:beast:ruin-pulse".into()];
        source.conditions.insert("bestiaryTraitsInitialized".into(),1.);
        r.actors_at_mut(r.depth).push(source.clone());
        let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+5]="wall".into();
        let target=r.player.id;events::begin();assert!(crate::monster_ruin::special_action(&mut r,900.,target).unwrap());
        let out=events::finish();let rings:Vec<_>=out.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="E51"&&a.style=="crossed-ring-violet").collect();
        assert_eq!(rings.len(),1);assert!(rings[0].cells.len()>2);
        for p in &rings[0].cells {assert!((p.x-source.x).hypot(p.y-source.y)<=3.);}
        assert!(!rings[0].cells.contains(&EventPoint{x:6.,y:2.}));
        assert!(out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E51"&&a.style=="puncture-violet")));
    }

}

/// Called only with the actual committed-charge move result. A false action
/// caused by a status is not a collision; geometry/occupancy must also block the
/// selected step. No path, hidden obstacle or other actor is exposed.
pub fn blocked_charge(r:&Run,before:&Actor,dx:f64,dy:f64,accepted:bool) {
    if !events::enabled(r)||accepted||!before.abilities.iter().any(|s|s=="enemy:beast:minotaur")
        || crate::engine::condition(before,"monsterChargeRemaining")<=0.
        || crate::engine::condition(before,"monsterChargeDx")!=dx
        || crate::engine::condition(before,"monsterChargeDy")!=dy
        || ["confused","riptide","forcedMove","paralyzed"].iter().any(|k|crate::engine::condition(before,k)>0.) {return;}
    let Some(after)=crate::engine::actor(r,before.id).filter(|a|!a.dead.value_or(false)&&a.hp>0.&&a.x==before.x&&a.y==before.y)else{return};
    let (x,y)=(before.x+dx,before.y+dy);
    let blocked=!crate::engine::can_traverse(r,before,x,y)
        || crate::portals::step_options(r,before,x,y).iter().any(|&(x,y)| {
            crate::engine::bulwark_blocks(r,before,x,y)
            || std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|a|a.id!=before.id&&!a.dead.value_or(false)&&a.hp>0.
                &&crate::dreamwalking::same_plane(r,before,a)&&crate::portals::same_position(r,a,x,y))
        });
    if blocked {mark(r,after,"E09","rejected-dust",Stage::Impact);}
}

/// Observed endpoint fades only for positive-to-zero transitions after actual
/// simulation status updates. Refreshes, absent conditions and dead/removed
/// bodies stay quiet. Multiple simultaneous endpoints share one local fade.
/// The caller owns the before snapshot at the real status-clock boundary.
pub fn expired_statuses(r:&Run,before:&Actor) {
    if !events::enabled(r)||!events::actor_observed(r,before) {return;}
    let Some(after)=crate::engine::actor(r,before.id).filter(|a|!a.dead.value_or(false)&&a.hp>0.)else{return};
    let ended=["poisoned","envenomed","bleeding","killspawnBleeding","burning","hemophilia","slowed","paralyzed","confused","hallucinating","disoriented","grabbed","gasVulnerableTicks","weakened","emboldened","flying","hasted","magicSensitive"]
        .iter().any(|key|crate::engine::condition(before,key)>0. && crate::engine::condition(after,key)<=0.
            // One-turn Yeti Slow expires numerically while continuous contact
            // remains applied; do not flash a status fade every upkeep either.
            && (*key!="slowed"||!crate::monster_chill::observed_chilled_ids(r).contains(&after.id)));
    if ended {mark(r,after,"E71","dissolve-status",Stage::Special);}
}

#[cfg(test)] mod final_endpoint_tests {
 use super::*;
 fn room()->Run {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  r.simulation_revision=52.0.into();r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.actors_at_mut(r.depth).clear();
  let f=r.floor_mut().unwrap();let n=f.tiles.len();f.tiles=vec!["floor".into();n];f.lights=vec![100.;n];r
 }
 #[test] fn charge_impact_requires_real_failed_obstructed_move() {
  let mut r=room();let mut a=r.player.clone();a.id=900.;a.x=4.;a.y=2.;a.state="hunting".into();a.abilities=vec!["enemy:beast:minotaur".into()];
  for (k,v) in [("monsterChargeRemaining",2.),("monsterChargeDx",1.),("monsterChargeDy",0.)] {a.conditions.insert(k.into(),v);}
  r.actors_at_mut(r.depth).push(a.clone());let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+5]="wall".into();
  events::begin();let accepted=crate::engine::move_actor(&mut r,900.,1.,0.,true).unwrap();assert!(!accepted);blocked_charge(&r,&a,1.,0.,accepted);
  let events=events::finish();assert_eq!(events.iter().filter(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E09"&&v.style=="rejected-dust")).count(),1);
  r.floor_mut().unwrap().tiles[2*w+5]="floor".into();
  crate::engine::actor_mut_trusted(&mut r,900.).unwrap().conditions.insert("riptide".into(),1.);let before=crate::engine::actor(&r,900.).unwrap().clone();
  events::begin();let accepted=crate::engine::move_actor(&mut r,900.,1.,0.,true).unwrap();blocked_charge(&r,&before,1.,0.,accepted);assert!(events::finish().is_empty());
 }
 #[test] fn status_endpoint_fade_uses_real_expiry_not_refresh_or_legacy() {
  let mut r=room();r.player.conditions.insert("poisoned".into(),1.);let before=r.player.clone();
  crate::engine::tick_condition(&mut r.player,"poisoned");events::begin();expired_statuses(&r,&before);
  assert_eq!(events::finish().iter().filter(|e|e.animation.as_ref().is_some_and(|v|v.proposal=="E71"&&v.style=="dissolve-status")).count(),1);
  r.player.conditions.insert("poisoned".into(),5.);events::begin();expired_statuses(&r,&before);assert!(events::finish().is_empty());
  r.player.conditions.insert("poisoned".into(),0.);r.simulation_revision=51.0.into();events::begin();expired_statuses(&r,&before);assert!(events::finish().is_empty());
 }
}

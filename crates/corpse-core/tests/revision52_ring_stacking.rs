use corpse_core::{Run,Item,Field,Stats,combat,revision50_rings as rings};
fn room()->Run {
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.simulation_revision=52.0.into();r.ring_rules_from=Field::Null; // Historical R52 ring roster/powers.
r.inventory.clear();r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();r.player.equipment_abilities=Field::Missing;r.player.stats=Stats::default();r.player.hp=20.;r.player.max_hp=100.;r
}
fn copies(r:&mut Run,effect:&str,n:usize){r.inventory=(0..n).map(|i|Item{id:900.+i as f64,kind:"ring".into(),effect:effect.to_string().into(),..Default::default()}).collect();}
fn close(a:f64,b:f64){assert!((a-b).abs()<1e-9,"{a} != {b}");}
#[test]fn additive_stats_and_removal(){
 let mut r=room();let base=combat::effective_stats(&r,&r.player);
 for n in 0..=3 {copies(&mut r,rings::PROTECTION,n);close(combat::effective_stats(&r,&r.player).dr,base.dr+4.*n as f64);
 copies(&mut r,rings::SLAYING,n);close(combat::effective_stats(&r,&r.player).offense,base.offense+4.*n as f64);}
 copies(&mut r,rings::PROTECTION,1);close(combat::effective_stats(&r,&r.player).dr,base.dr+4.);
}
#[test]fn multiplicative_powers_and_binary_immunity(){
 let mut r=room();
 for n in 0..=3 {copies(&mut r,rings::REGENERATION,n);close(rings::natural_hp_rate(&r,&r.player),0.002*2f64.powi(n as i32));
 for (effect,kind) in [(rings::FIRE_RESISTANCE,"fire"),(rings::POISON_RESISTANCE,"poison"),(rings::CORROSION_RESISTANCE,"acid")] {copies(&mut r,effect,n);close(rings::resist_damage(&r,&r.player,kind,80.),80.*0.5f64.powi(n as i32));}
 copies(&mut r,rings::STEALTH,n);close(rings::stack_multiplier(&r,&r.player,rings::STEALTH,1.2),1.2f64.powi(n as i32));
 copies(&mut r,rings::WILLPOWER,n);close(corpse_core::monster_statuses::willpower_duration(&r,&r.player,16.),16./2f64.powi(n as i32));
 copies(&mut r,rings::ICE_IMMUNITY,n);close(rings::resist_damage(&r,&r.player,"cold",80.),if n>0{0.}else{80.});}
}
#[test]fn vampirism_sustain_and_serialization(){
 let mut r=room();copies(&mut r,rings::VAMPIRISM,3);let a=r.player.clone();corpse_core::divine_combat::heal_melee_damage(&mut r,&a,40.,true);close(r.player.hp,26.);
 copies(&mut r,rings::SUSTAIN_STRENGTH,2);r.player.conditions.insert("weakened".into(),5.);close(combat::effective_stats(&r,&r.player).offense,-1.25);
 r.player.conditions.clear();r.player.conditions.insert("buff:offense".into(),-8.);r.player.conditions.insert("buffTicks:offense".into(),5.);close(combat::effective_stats(&r,&r.player).offense,-2.);
 let loaded:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();close(combat::effective_stats(&loaded,&loaded.player).offense,-2.);
 r.player.conditions.insert("miniMe".into(),1.);assert_eq!(rings::stacking_count(&r,&r.player,rings::SUSTAIN_STRENGTH),0);
}
#[test]fn old_revisions_keep_fixed_power_behavior(){let mut r=room();r.simulation_revision=51.0.into();copies(&mut r,rings::REGENERATION,3);close(rings::natural_hp_rate(&r,&r.player),0.004);copies(&mut r,rings::PROTECTION,3);close(combat::effective_stats(&r,&r.player).dr,4.);}

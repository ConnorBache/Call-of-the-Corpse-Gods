//! Mechanical before/after presentation-delta probe, not a balance measurement.
use corpse_core::{engine,population,startup,Field,Floor,Run,Command};
use std::sync::Arc;
fn room(kind:&str,seed:u32)->Run {
 let mut r=startup::new_run_shell_revision("0-5-11:presentation-smoke","Unblessed",false,33.).unwrap();
 r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
 r.player.stats.offense=200.;r.player.stats.defense=100.;r.player.stats.accuracy=110.;r.player.stats.dodge=110.;r.player.stats.dr=10.;r.player.stats.perception=1000.;r.player.hp=700.;r.player.max_hp=700.;r.player.base_max_hp=100.0.into();
 let w=79;let h=29;let mut tiles=vec!["wall".into();w*h];for y in 1..h-1{for x in 1..w-1{tiles[y*w+x]="floor".into();}}
 let mut f=Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
 let mut a=population::create_actor_revision(kind,4.,6.,2.,false,33.).unwrap();a.state="hunting".into();r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![a]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=95.;r.tick=3.;r.rng=seed as f64;r.generated_ability_grants=Vec::new().into();r.inventory.clear();r.log.clear();r.log_kinds=Vec::new().into();r
}
fn main(){let out=std::env::args().nth(1).unwrap();std::fs::create_dir_all(&out).unwrap();for kind in ["night-crawler","skin-walker","vampire"]{for seed in [1,7,41,127]{let mut r=room(kind,seed);let mut commands=0;while commands<128&&r.status=="playing"&&engine::actor(&r,4.).is_some_and(|a|a.hp>0.&&!a.dead.value_or(false)){let a=engine::actor(&r,4.).unwrap();let step=|d:f64| if d>0.{1.}else if d<0.{-1.}else{0.};let dx=step(a.x-r.player.x);let dy=step(a.y-r.player.y);engine::dispatch(&mut r,&Command::Move{dx,dy}).unwrap();commands+=1;}let hp=r.player.hp;let target_hp=engine::actor(&r,4.).unwrap().hp;for _ in 0..20 {if r.status!="playing"{break;}engine::dispatch(&mut r,&Command::Wait).unwrap();}std::fs::write(format!("{out}/{kind}-{seed}.json"),corpse_core::storage::serialize(&r).unwrap()).unwrap();println!("{kind},{seed},{commands},{hp},{target_hp},{},{},{}",r.player.hp,r.rng,r.status);}}}

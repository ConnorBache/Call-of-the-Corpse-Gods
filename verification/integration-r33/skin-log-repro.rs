use corpse_core::{engine,population,startup,new_creatures as n,Field,Floor,Command};
use std::sync::Arc;
fn main(){
 let mut r=startup::new_run_shell_revision("0-5-11:skin-log-repro","Unblessed",false,33.).unwrap();r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
 let w=79;let h=29;let mut tiles=vec!["wall".into();w*h];for y in 1..h-1{for x in 1..w-1{tiles[y*w+x]="floor".into();}}
 let mut f=Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
 let mut a=population::create_actor_revision("skin-walker",4.,6.,2.,false,33.).unwrap();a.state="sleeping".into();r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![a]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=95.;r.tick=3.;r.generated_ability_grants=Vec::new().into();r.log.clear();r.log_kinds=Vec::new().into();
 let mut bow=corpse_core::equipment::make_equipment("Bow",90.).unwrap();bow.equipped="hand1".to_string().into();r.inventory=vec![bow,corpse_core::equipment::make_arrows(91.,5.)];r.player.conditions.insert("unlucky".into(),1.);
 let a=engine::actor(&r,4.).unwrap();println!("before true={} disguise={:?} visible={} distance={}",a.name,n::disguise_kind(&r,a),corpse_core::vision::visible_actor(&r,a),(r.player.x-a.x).abs().max((r.player.y-a.y).abs()));
 let result=engine::dispatch(&mut r,&Command::Shoot{id:90.,x:6.,y:2.});println!("result={result:?}");println!("log={:?}",r.log);let a=engine::actor(&r,4.).unwrap();println!("after true={} disguise={:?} distance={}",a.name,n::disguise_kind(&r,a),(r.player.x-a.x).abs().max((r.player.y-a.y).abs()));
}

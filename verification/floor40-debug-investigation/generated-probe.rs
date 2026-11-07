use corpse_core::{engine,Command};
fn main() {
 let args:Vec<_>=std::env::args().skip(1).collect();
 let count=args.first().and_then(|s|s.parse().ok()).unwrap_or(10);
 let turns=args.get(1).and_then(|s|s.parse().ok()).unwrap_or(10);
 let revision=args.get(2).and_then(|s|s.parse().ok()).unwrap_or(51.);
 let mode=args.get(3).map(|s|s.as_str()).unwrap_or("Avatar");
 for index in 0..count {
  let seed=format!("floor40-fire-review-{index}");
  let mut r=match corpse_core::debug_floor::create_run_revision(&seed,mode,40,revision){Ok(r)=>r,Err(e)=>{println!("GEN {seed}: {e}");continue;}};
  while let Some(id)=r.offers.first().cloned(){engine::dispatch(&mut r,&Command::Choose{id}).unwrap();}
  if let Err(e)=corpse_core::debug_floor::enter(&mut r){println!("ENTER {seed}: {e}");continue;}
  r.player.hp=100000.;r.player.max_hp=100000.;
  println!("START {seed} fire={} actors={}",r.floor().unwrap().environment.as_ref().unwrap().fire.iter().filter(|v|**v>0.).count(),r.actors_at(40.).len());
  if let Err(e)=engine::validate_supported(&r){println!("VALIDATE {seed}: {e}");continue;}
  engine::monster_floor_entered(&mut r);
  if let Err(e)=engine::monster_prepare_player_turn(&mut r){println!("PREP {seed}: {e}");continue;}
  for turn in 0..turns {
   if let Err(e)=engine::dispatch(&mut r,&Command::Wait){println!("ERROR {seed} turn{turn} depth{}: {e}",r.depth);break;}
   if r.status!="playing"{println!("END {seed} {turn} tick{} status{}",r.tick,r.status);break;}
  }
 }
}

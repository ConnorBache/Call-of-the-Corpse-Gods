//! Independent reproducible review, not a gameplay feature or benchmark promise.
use corpse_core::{startup, portals, non_euclidean_generation as ne, portals_generation as pg, pocket_biomes, engine, storage, Command, movement::Point};
use std::time::Instant;
fn main() {
 let args:Vec<_>=std::env::args().skip(1).collect();let seeds=if args.is_empty(){vec!["r37-independent-0".into(),"r37-independent-1".into()]}else{args};
 for seed in seeds {
  let t=Instant::now(); let mut r=startup::new_run_shell_revision(&seed,"Unblessed",false,37.).unwrap();
  if let Err(e)=startup::ensure_floors(&mut r,40){println!("GEN_ERROR seed={seed} error={e}");continue;}
  println!("GEN seed={seed} elapsed_ms={:.3}",t.elapsed().as_secs_f64()*1000.);
  let mut repeat=startup::new_run_shell_revision(&seed,"Unblessed",false,37.).unwrap();startup::ensure_floors(&mut repeat,40).unwrap();assert_eq!(r,repeat,"generation determinism {seed}");
  let mut selected=None;let mut max_links=0;
  for (fi,f) in r.floors.iter().enumerate(){
   assert!(pocket_biomes::validate(f),"biome {seed}:{}",f.depth);assert!(ne::validate_floor(f),"link shape {seed}:{}",f.depth);
   assert!(ne::traversable(f),"routes {seed}:{}",f.depth);assert!(pg::all_endpoints_optional(f),"portals mandatory {seed}:{}",f.depth);
   let actors=r.actors_at(f.depth);
   for a in actors.iter().filter(|a|a.creature_kind.as_ref().is_some_and(|k|k=="wendigo"||k=="yeti")) {
    let at=a.y as usize*f.width()+a.x as usize;
    assert!(f.pocket_biomes.as_ref().into_iter().flatten().any(|b|b.theme=="mountainous"&&b.cells.contains(&at)),"ice creature outside mountains {seed}:{} id{}",f.depth,a.id);
   }
   if let Some(c)=f.pocket_biomes.as_ref().into_iter().flatten().find(|b|b.theme=="corn-dimension") {
    let corn:Vec<_>=actors.iter().filter(|a|c.cells.contains(&(a.y as usize*f.width()+a.x as usize))&&a.creature_kind.as_ref().is_some_and(|k|k.starts_with("horn-"))).collect();
    assert!(corn.len()>=8);let strong=if f.depth>=36.{"horn-7"}else{"horn-5"};assert!(corn.iter().any(|a|a.creature_kind.as_ref().is_some_and(|k|k==strong)));
    let p=f.portals.as_ref().into_iter().flatten().filter(|p|c.cells.contains(&p.a)&&c.cells.contains(&p.b)).count();
    let s:usize=f.speed_up.as_ref().into_iter().flatten().filter(|p|p.cells.iter().all(|i|c.cells.contains(i))).map(|p|p.cells.len()).sum();
    let n:usize=f.non_euclidean.as_ref().into_iter().flatten().filter(|p|p.cells.iter().all(|i|c.cells.contains(i))).map(|p|p.cells.len()).sum();
    println!("CORN seed={seed} depth={} cells={} residents={} portals={p} speed_cells={s} random_cells={n} links={}",f.depth,c.cells.len(),corn.len(),portals::links(f).len());
    if portals::links(f).len()>=max_links{max_links=portals::links(f).len();selected=Some(fi);}
   }
  }
  let Some(fi)=selected else{continue};let f=&r.floors[fi];let w=f.width();
  let cells:Vec<_>=f.tiles.iter().enumerate().filter(|(_,t)|matches!(t.as_str(),"floor"|"grass"|"shallow-water")).map(|(i,_)|i).collect();
  let from=portals::point(f,cells[0]);let targets:Vec<Point>=(0..40).map(|j|portals::point(f,cells[j*cells.len()/40])).collect();
  let rng=r.rng;let start=Instant::now();for &to in &targets {std::hint::black_box(portals::route(f,from,to,false,|_,_|true));}let cold=start.elapsed().as_secs_f64()*1000.;
  let start=Instant::now();for _ in 0..10{for &to in &targets{std::hint::black_box(portals::route(f,from,to,false,|_,_|true));}}let warm=start.elapsed().as_secs_f64()*1000./10.;
  let pass=|p:Point|portals::cell(f,p.x as f64,p.y as f64).is_some_and(|i|!matches!(f.tiles[i].as_str(),"wall"|"tree"));
  let start=Instant::now();for &to in &targets {std::hint::black_box(portals::aimed_ray(f,from,to,pass));}let ray=start.elapsed().as_secs_f64()*1000.;assert_eq!(rng,r.rng);
  println!("PERF seed={seed} depth={} links={max_links} route40_cold_ms={cold:.3} route40_warm_ms={warm:.3} ray40_ms={ray:.3}",f.depth);
  let depth=f.depth;let corn_cells=&f.pocket_biomes.as_ref().unwrap().iter().find(|b|b.theme=="corn-dimension").unwrap().cells;let free=corn_cells.iter().copied().find(|i|!r.actors_at(depth).iter().any(|a|a.x==(*i%w)as f64&&a.y==(*i/w)as f64)).unwrap();
  r.depth=depth;r.player.x=(free%w)as f64;r.player.y=(free/w)as f64;r.player.hp=100000.;
  let saved=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&saved).unwrap();assert_eq!(loaded.floor().unwrap(),r.floor().unwrap());assert_eq!(loaded.rng,r.rng);
  let mut a=r.clone();let mut b=loaded;let start=Instant::now();let oa=engine::dispatch(&mut a,&Command::Wait);let elapsed=start.elapsed().as_secs_f64()*1000.;let ob=engine::dispatch(&mut b,&Command::Wait);
  match (oa,ob){(Ok(x),Ok(y))=>{assert_eq!(x.took_turn,y.took_turn);assert!(storage::canonical_equal(&serde_json::to_value(&a).unwrap(),&serde_json::to_value(&b).unwrap()));println!("TURN seed={seed} depth={depth} ms={elapsed:.3} took_turn={} deterministic=true",x.took_turn);},(a,b)=>panic!("TURN_ERROR seed={seed} a={a:?} b={b:?}")}
 }
}

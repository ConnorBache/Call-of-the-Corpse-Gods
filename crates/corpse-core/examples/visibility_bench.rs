//! Read-only normal-build diagnostic. Setup, cloning and equality checks outside
//! timed regions; cold and fresh-origin costs are measured separately from warm.
use corpse_core::{Run,vision,portals_generation::PortalPair};
use serde_json::json;
use std::{hint::black_box,time::Instant,path::Path};
/// O(bytes) deterministic output fingerprint, not a timing or correctness oracle.
fn hash(s:&str)->String{let mut h=0xcbf29ce484222325u64;for b in s.bytes(){h^=b as u64;h=h.wrapping_mul(0x100000001b3)}format!("{h:016x}")}
/// O(S log S), outside measurement; retains all samples.
fn stats(mut v:Vec<u128>)->serde_json::Value{v.sort();json!({"median_ns":v[v.len()/2],"p95_ns":v[v.len()*95/100],"raw_ns":v})}
/// O(generation+A+N), deterministic cases; full states exported for validation.
fn scenes()->Vec<(String,Run)>{
 let base=corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap();
 let mut crowded=base.clone();let rat=base.actors.values().flat_map(|g|g.iter()).find(|a|a.creature_kind.as_ref().is_some_and(|k|k=="rat")).unwrap().clone();
 let f=base.floor().unwrap();let w=f.width();let cells:Vec<_>=f.tiles.iter().enumerate().filter(|(i,t)|matches!(t.as_str(),"floor"|"grass")&&!base.actors_at(base.depth).iter().any(|a|a.x==(*i%w)as f64&&a.y==(*i/w)as f64)&&((*i%w)as f64-base.player.x).abs()+((*i/w)as f64-base.player.y).abs()>4.).map(|(i,_)|i).take(128).collect();assert_eq!(cells.len(),128);
 for at in cells{let mut a=rat.clone();a.id=crowded.next_id;crowded.next_id+=1.;a.x=(at%w)as f64;a.y=(at/w)as f64;corpse_core::ai::wake(&mut a,true);a.state="hunting".into();a.last_seen=corpse_core::ActorLastSeen{x:base.player.x,y:base.player.y}.into();a.conditions.insert("targetId".into(),base.player.id);crowded.actors_at_mut(base.depth).push(a)}
 let mut portal=base.clone();portal.depth=10.;let f=portal.floor().unwrap();let w=f.width();let cells:Vec<_>=f.tiles.iter().enumerate().filter(|(i,t)|matches!(t.as_str(),"floor"|"grass")&&!corpse_core::environment::blocks_sight(f,*i)).map(|(i,_)|i).collect();assert!(cells.len()>=16);
 portal.player.x=(cells[0]%w)as f64;portal.player.y=(cells[0]/w)as f64;
 portal.floor_mut().unwrap().portals=(0..7).map(|i|PortalPair{a:cells[1+i],b:cells[cells.len()-1-i],color_index:i as u8}).collect::<Vec<_>>().into();
 vec![("generated".into(),base),("crowded148".into(),crowded),("seven-portals".into(),portal)]
}
/// O(cases*samples*observation); extra report/snapshot memory O(N+samples).
fn main(){let args:Vec<_>=std::env::args().collect();let dir=Path::new(args.get(1).expect("output directory"));std::fs::create_dir_all(dir).unwrap();let count=args.get(2).and_then(|s|s.parse::<usize>().ok()).unwrap_or(30).max(3);let mut reports=vec![];
 for(name,run)in scenes(){std::fs::write(dir.join(format!("{name}-input.json")),serde_json::to_string(&run).unwrap()).unwrap();let initial=serde_json::to_string(&run).unwrap();
  for mode in ["cold","warm","fresh-origin"]{let mut r=run.clone();let mut samples=vec![];let positions:Vec<_>=r.floor().unwrap().tiles.iter().enumerate().filter(|(_,t)|matches!(t.as_str(),"floor"|"grass")).map(|(i,_)|i).collect();let w=r.floor().unwrap().width();
   if mode!="cold"{black_box(vision::observe_terrain(&r));}
   let mut last=String::new();for i in 0..count+3{if mode=="cold"{r=run.clone()}if mode=="fresh-origin"{let at=positions[(i*31)%positions.len()];r.player.x=(at%w)as f64;r.player.y=(at/w)as f64;}
    let start=Instant::now();let observed=black_box(vision::observe_terrain(black_box(&r)));let elapsed=start.elapsed().as_nanos();if i>=3{samples.push(elapsed)}
    last=format!("{observed:?}");if mode!="fresh-origin"{assert_eq!(serde_json::to_string(&r).unwrap(),initial)}
   }
   std::fs::write(dir.join(format!("{name}-{mode}-observation.txt")),&last).unwrap();reports.push(json!({"case":name,"mode":mode,"actors":r.actors_at(r.depth).len(),"timing":stats(samples),"observation_hash":hash(&last)}));
  }
 }
 std::fs::write(dir.join("visibility.json"),serde_json::to_string_pretty(&reports).unwrap()).unwrap();
}

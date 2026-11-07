//! Allocation-only standalone example. Do NOT use this instrumented executable
//! for normal latency claims: every allocation has atomic-counter overhead.
use corpse_core::{Run,Field,Command,temporal::Engine};
use serde::Serialize;
use std::{alloc::{GlobalAlloc,Layout,System},sync::{Arc,atomic::{AtomicI64,AtomicU64,Ordering}}};
struct Meter;
static LIVE:AtomicI64=AtomicI64::new(0);static PEAK:AtomicI64=AtomicI64::new(0);
static ALLOC:AtomicU64=AtomicU64::new(0);static FREED:AtomicU64=AtomicU64::new(0);static CALLS:AtomicU64=AtomicU64::new(0);
/// O(1) single-thread counter update; no allocations or logging in allocator.
fn added(bytes:usize){ALLOC.fetch_add(bytes as u64,Ordering::Relaxed);CALLS.fetch_add(1,Ordering::Relaxed);let live=LIVE.fetch_add(bytes as i64,Ordering::Relaxed)+bytes as i64;PEAK.fetch_max(live,Ordering::Relaxed);}
/// O(1) accounting of requested layout bytes, not allocator metadata/RSS.
fn removed(bytes:usize){FREED.fetch_add(bytes as u64,Ordering::Relaxed);LIVE.fetch_sub(bytes as i64,Ordering::Relaxed);}
unsafe impl GlobalAlloc for Meter {
 /// System allocation cost plus O(1) instrumentation.
 unsafe fn alloc(&self,l:Layout)->*mut u8{let p=unsafe{System.alloc(l)};if !p.is_null(){added(l.size());}p}
 /// System zeroed allocation cost plus O(1) instrumentation.
 unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8{let p=unsafe{System.alloc_zeroed(l)};if !p.is_null(){added(l.size());}p}
 /// System deallocation cost plus O(1) instrumentation.
 unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)};removed(l.size());}
 /// System reallocation cost; account successful old release/new request, even
 /// if allocator grows in place. Thus allocated_bytes is traffic, not copying.
 unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{let next=unsafe{System.realloc(p,l,n)};if !next.is_null(){removed(l.size());added(n);}next}
}
#[global_allocator] static GLOBAL:Meter=Meter;
#[derive(Serialize)] struct Row {cached_anchor_wire_utf8_bytes:Option<usize>,native_anchor_present:bool,floors:usize,imported_trail_length:usize,stage:&'static str,before_live_requested_bytes:i64,after_live_requested_bytes:i64,retained_delta_bytes:i64,peak_live_requested_bytes:i64,peak_above_start_bytes:i64,allocated_requested_bytes:u64,freed_requested_bytes:u64,successful_alloc_or_realloc_calls:u64,shared_anchor_floor_blocks:usize,total_floor_blocks:usize}
/// O(operation cost), counter reads O(1); returned value remains live in row delta.
fn measured<T>(floors:usize,imported_trail_length:usize,stage:&'static str,operation:impl FnOnce()->T)->(T,Row){
 let before=LIVE.load(Ordering::Relaxed);PEAK.store(before,Ordering::Relaxed);let a=ALLOC.load(Ordering::Relaxed);let f=FREED.load(Ordering::Relaxed);let c=CALLS.load(Ordering::Relaxed);
 let value=operation();let after=LIVE.load(Ordering::Relaxed);let peak=PEAK.load(Ordering::Relaxed);
 (value,Row{cached_anchor_wire_utf8_bytes:None,native_anchor_present:false,floors,imported_trail_length,stage,before_live_requested_bytes:before,after_live_requested_bytes:after,retained_delta_bytes:after-before,peak_live_requested_bytes:peak,peak_above_start_bytes:peak-before,allocated_requested_bytes:ALLOC.load(Ordering::Relaxed)-a,freed_requested_bytes:FREED.load(Ordering::Relaxed)-f,successful_alloc_or_realloc_calls:CALLS.load(Ordering::Relaxed)-c,shared_anchor_floor_blocks:0,total_floor_blocks:0})
}
/// O(F), no payload traversal/allocation; supplements actual allocation counters.
fn shares(e:&Engine,row:&mut Row){row.cached_anchor_wire_utf8_bytes=e.run.future.as_ref().and_then(|a|a.text_if_encoded()).map(str::len);row.native_anchor_present=e.run.future.as_ref().is_some_and(|a|a.native_state().is_some());row.total_floor_blocks=e.run.floors.len();if let Some(a)=e.run.future.as_ref().and_then(|a|a.native_state()){row.shared_anchor_floor_blocks=e.run.floors.iter().zip(&a.floors).filter(|(x,y)|Arc::ptr_eq(x,y)).count();}}
/// O(world parse/setup), outside measured stages; authored no-NPC ordinary floor.
fn fixture(floors:usize)->Run {
 let text=if floors==40{std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap()}else{include_str!("../../../fixtures/avatar-room-v1.json").to_owned()};
 let mut r=Engine::load(&text).unwrap().run;assert_eq!(r.floors.len(),floors);r.simulation_revision=41.0.into();
 for g in r.actors.values_mut(){*g=Arc::new(vec![]);}for g in r.ground.values_mut(){*g=Arc::new(vec![]);}
 r.player.x=10.;r.player.y=10.;r.player.abilities=vec!["morae-see-the-future".into()];r.player.ability_order=Field::Missing;r.player.active.clear();r.player.conditions.clear();r.player.hp=100000.;r.player.max_hp=100000.;r.player.base_max_hp=100000.0.into();r.player.energy=100.;r.player.stats.max_energy=100.;
 r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;r.timeweave=Field::Missing;r.offers.clear();r.phase="normal".to_owned().into();r.replay_snapshot_from=Field::Missing;
 let f=r.floor_mut().unwrap();for t in &mut f.tiles{*t="floor".into();}f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;r
}
/// O(L+F) imported geometry setup, outside measurements; not a naturally walked
/// route. Reserve current source/destination so the divergence is a paid forward.
fn install_long_trail(r:&mut Run,length:usize){
 use corpse_core::timeweave::{Cell,Plane,Segment,Trail};
 let w=r.floor().unwrap().width();let reserved=[(10*w+10)as u32,(10*w+11)as u32];
 let mut t=Trail::default();
 'floors:for f in &r.floors{let width=f.width();for i in 0..f.tiles.len(){
   if i%width+1>=width || (f.depth==1.&&reserved.contains(&(i as u32))){continue;}
   let cell=Cell{floor:f.depth as u32,plane:Plane::Waking,tile:i as u32};
   t.append(Segment{cell,from:cell,to:Cell{tile:cell.tile+1,..cell}});
   if t.len()==length{break 'floors;}
 }}
 assert_eq!(t.len(),length);r.timeweave=Arc::new(t).into();r.player.abilities.push(corpse_core::progression::WEAVE_ID.into());corpse_core::timeweave::validate_run(r).unwrap();
}
/// O(two worlds' setup + measured operations + report bytes). No worker threads,
/// timers, SDL, durable flush, compression, or hidden proof serialization.
fn main(){
 let mut rows=Vec::with_capacity(30);let mut cases=vec![(10usize,0usize),(40,0)];if std::env::var("NATIVE_FUTURE_ALLOC_LONG_TRAIL").as_deref()==Ok("1"){cases.push((10,10000));}
 for (floors,trail) in cases {
  let mut e=Engine::new(fixture(floors)).unwrap();e.dispatch(&Command::Wait).unwrap();e.run.player.energy=100.;if trail>0{install_long_trail(&mut e.run,trail);}
  let command=Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
  let tick=e.run.tick;
  let(out,mut row)=measured(floors,trail,"actual-future-capture",||e.dispatch(&command).map(|o|(o.changed,o.took_turn)));assert!(out.unwrap().0);assert!(e.run.future.as_ref().unwrap().native_state().is_some());assert!(e.run.future.as_ref().unwrap().text_if_encoded().is_none());shares(&e,&mut row);rows.push(row);
  let(out,mut row)=measured(floors,trail,"first-real-move-divergence",||e.dispatch(&Command::Move{dx:1.,dy:0.}).map(|o|(o.changed,o.took_turn)));assert!(out.unwrap().1);assert_eq!((e.run.player.x,e.run.player.y),(11.,10.));shares(&e,&mut row);rows.push(row);
  assert!(e.run.future.as_ref().unwrap().text_if_encoded().is_none());
  let(wire,mut row)=measured(floors,trail,"first-wire-encode-output-retained",||e.save().unwrap());shares(&e,&mut row);rows.push(row);assert!(e.run.future.as_ref().unwrap().text_if_encoded().is_some());
  let((),mut row)=measured(floors,trail,"drop-first-outer-wire-only",||drop(wire));shares(&e,&mut row);rows.push(row);
  let(wire,mut row)=measured(floors,trail,"warm-wire-encode-output-retained",||e.save().unwrap());shares(&e,&mut row);rows.push(row);
  let((),mut row)=measured(floors,trail,"drop-warm-outer-wire-only",||drop(wire));shares(&e,&mut row);rows.push(row);
  let(out,mut row)=measured(floors,trail,"actual-future-restore",||e.dispatch(&command).map(|o|(o.changed,o.took_turn)));assert!(out.unwrap().0);assert!(e.run.future.is_none());assert_eq!(e.run.tick,tick);assert_eq!((e.run.player.x,e.run.player.y),(10.,10.));shares(&e,&mut row);rows.push(row);
 }
 let report=serde_json::json!({"schema":1,"scope":"single-thread counting-System allocation diagnostic; NOT normal latency, RSS or allocator-overhead measurement","units":"requested allocation layout bytes; realloc counts full old release/new request, not copied bytes","fixture":"same10/40floor sources; no NPCs, ground cleared, ordinary authored active floor, one setup Wait before capture","retention":"native and current share unchanged Arc blocks; first wire retains immutable derived anchor text; each outer output is separately dropped","rows":rows});
 println!("{}",serde_json::to_string_pretty(&report).unwrap());
}

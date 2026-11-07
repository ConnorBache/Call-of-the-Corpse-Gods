//! Isolated lookup and retained-allocation diagnostic, not a game/AI throughput test.
//! Build --release. No wall-clock value affects simulation or save state.
use corpse_core::{Actor,Run,engine};
use std::{alloc::{GlobalAlloc,Layout,System},sync::{Arc,atomic::{AtomicUsize,Ordering}},hint::black_box,time::Instant};
struct Counting;
static LIVE:AtomicUsize=AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    /// Allocator-dependent system cost; counter O(1). Tracks requested bytes, not RSS.
    unsafe fn alloc(&self,l:Layout)->*mut u8 {let p=unsafe{System.alloc(l)};if !p.is_null(){LIVE.fetch_add(l.size(),Ordering::Relaxed);}p}
    /// Allocator-dependent system cost; counter O(1).
    unsafe fn dealloc(&self,p:*mut u8,l:Layout){LIVE.fetch_sub(l.size(),Ordering::Relaxed);unsafe{System.dealloc(p,l)}}
    /// Allocator-dependent, potentially O(old/new allocation bytes); counter O(1).
    unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8 {let q=unsafe{System.realloc(p,l,n)};if !q.is_null(){LIVE.fetch_sub(l.size(),Ordering::Relaxed);LIVE.fetch_add(n,Ordering::Relaxed);}q}
}
#[global_allocator]static ALLOC:Counting=Counting;
/// O(N) actor construction; empty payloads deliberately exclude gameplay setup.
fn world(n:usize)->Run {let mut r=Run{depth:1.,next_id:(n+1)as f64,player:Actor{id:0.,..Default::default()},..Default::default()};r.actors.insert("1".into(),Arc::new((1..=n).map(|id|Actor{id:id as f64,..Default::default()}).collect()));r}
/// O(N*d + indexed_queries*d + linear_queries*N); retained index memory measured
/// after construction, with actor vectors excluded. Five independently built worlds.
fn main(){
 println!("actors,index_build_ms,index_requested_bytes,indexed_ns_per_lookup,linear_ns_per_lookup,indexed_queries,linear_queries,actor_struct_bytes");
 for n in [32,100,1000,10_000,100_000] {
  let r=world(n);let before=LIVE.load(Ordering::Relaxed);let start=Instant::now();assert_eq!(engine::actor(&r,n as f64).unwrap().id,n as f64);let build=start.elapsed().as_secs_f64()*1000.;let bytes=LIVE.load(Ordering::Relaxed)-before;
  let q=1_000_000usize;let start=Instant::now();for i in 0..q {let id=(i.wrapping_mul(7919)%n+1)as f64;assert_eq!(black_box(engine::actor(black_box(&r),black_box(id))).unwrap().id,id);}let indexed=start.elapsed().as_nanos()as f64/q as f64;
  let linear_q=(10_000_000/n).clamp(100,100_000);let start=Instant::now();for i in 0..linear_q {let id=(i.wrapping_mul(7919)%n+1)as f64;assert_eq!(black_box(r.actors_at(r.depth).iter().find(|a|a.id==black_box(id))).unwrap().id,id);}let linear=start.elapsed().as_nanos()as f64/linear_q as f64;
  println!("{n},{build:.3},{bytes},{indexed:.3},{linear:.3},{q},{linear_q},{}",std::mem::size_of::<Actor>());
 }
 eprintln!("Requested heap bytes exclude allocator bookkeeping/RSS. Actor payload vectors excluded from index bytes; values are synthetic default Actors. Lookup loops include equality assertions. This is NOT turn-time or 100k-enemy AI performance.");
}

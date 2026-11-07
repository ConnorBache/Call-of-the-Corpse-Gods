//! Lead-controlled bounded metadata benchmark; no raster, SDL or simulation.
use super::*;
fn inputs(case:&str,count:usize)->Vec<Rect>{
    match case {
        "sparse"=>(0..count).map(|i|Rect::new(12+(i%16)*76,12+(i/16)*42,4,4)).collect(),
        "dense"=>(0..count).map(|i|Rect::new(300+(i%16)*2,200+(i/16)*2,64,64)).collect(),
        "ui-plus-sparse"=>[Rect::new(0,0,990,128),Rect::new(990,0,290,800)].into_iter().chain((0..count-2).map(|i|Rect::new(12+(i%16)*60,150+(i/16)*36,4,4))).collect(),
        _=>panic!("unknown controlled metadata case")
    }
}
/// Exact continuous-rectangle coverage proof, outside the timed interval. Because
/// output rectangles are disjoint, intersection areas sum without double counting.
fn check(d:&Damage,source:&[Rect]){
    assert!(d.rects().len()<=8);
    for(i,a)in d.rects().iter().enumerate(){assert!(a.right()<=1280&&a.bottom()<=800);for b in &d.rects()[..i]{assert!(a.intersect(*b).is_none());}}
    for input in source{if let Some(input)=input.intersect(Rect::new(0,0,1280,800)){let covered:usize=d.rects().iter().filter_map(|r|r.intersect(input)).map(Rect::area).sum();assert_eq!(covered,input.area(),"lost input coverage {input:?}");}}
}
#[test]
#[ignore="lead-controlled damage metadata benchmark; no parallel benchmark jobs"]
fn sparse_dense_128_256_region_metadata_cost(){
    let repeats:usize=std::env::var("DAMAGE_META_REPEATS").unwrap_or_else(|_|"100".into()).parse().unwrap();assert!((10..=1000).contains(&repeats));
    let mut rows=Vec::new();
    for case in ["sparse","dense","ui-plus-sparse"]{for count in [128,256]{
        let source=inputs(case,count);
        let mut warm=Damage::empty(1280,800);for r in &source{warm.add(*r);}check(&warm,&source);
        for sample in 0..repeats{
            let start=std::time::Instant::now();let mut d=Damage::empty(1280,800);
            for r in std::hint::black_box(&source){d.add(*r);}std::hint::black_box(&d);
            let add_ns=start.elapsed().as_nanos();
            // Correctness and result extraction deliberately outside timing.
            check(&d,&source);let regions=d.rects().len();let area=d.area();let budget_merge=d.coalesced_to_bounds;
            let start=std::time::Instant::now();drop(d);let drop_ns=start.elapsed().as_nanos();
            rows.push(serde_json::json!({"case":case,"inputs":count,"sample":sample,"create_and_add_ns":add_ns,"drop_ns":drop_ns,"regions":regions,"damage_area":area,"budget_merge":budget_merge}));
        }
    }}
    let report=serde_json::json!({"scope":"Damage metadata only. Timed create+all additions, allocation included; drop separately timed. Inputs prepared, exact coverage/disjointness checks and serialization outside intervals. No raster, SDL, observation or gameplay. No absolute hardware-independent latency assertion.","repeats":repeats,"rows":rows});
    if let Some(path)=std::env::var_os("DAMAGE_META_OUTPUT"){
        let path=std::path::PathBuf::from(path);assert!(path.is_absolute()&&!path.exists(),"explicit new absolute output path required");std::fs::write(&path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();println!("DAMAGE_METADATA_REPORT {}",path.display());
    }else{println!("DAMAGE_METADATA {}",report);}
}

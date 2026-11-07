use super::*;
use super::super::{chronicle_height, game_color};

fn native_bounds(p:&Painter) -> (i32,i32,i32,i32) {
    (p.native_x(12.).max(0),p.native_y(6.).max(0),
        p.native_x((p.hud_left()-13) as f64).min(p.width as i32),
        p.native_y((6+chronicle_height(&p.profile)) as f64).min(p.height as i32))
}

/// Independent original scalar color arithmetic, with per-pixel tile membership
/// rather than optimized row spans. It deliberately ignores logical clipping.
fn scalar(p:&Painter, input:&[u8], background:Rgb, bounds:(i32,i32,i32,i32)) -> (Vec<u8>,u64) {
    let mut out=input.to_vec();let mut visits=0;
    let(left,top,right,bottom)=bounds;
    let d=p.damage_clip.unwrap_or(damage::Rect::new(0,0,p.width,p.height));
    for y in top.max(d.y as i32)..bottom.min(d.bottom() as i32) {
        let opacity=crate::chronicle::fade((y-top) as f64/(bottom-top).max(1) as f64);
        for x in left.max(d.x as i32)..right.min(d.right() as i32) {
            if p.tile_clip_enabled&&!p.tile_mask.contains(x as usize,y as usize) {continue;}
            let at=(y as usize*p.width+x as usize)*4;
            let color=crate::color::composite(background,[out[at],out[at+1],out[at+2]],opacity);
            out[at..at+3].copy_from_slice(&color);visits+=1;
        }
    }
    (out,visits)
}

#[test]
fn chronicle_cached_rows_match_scalar_for_every_channel_value() {
    let mut cache=FadeCache::default();
    for height in [1,2,107,108,291,292] {
        for background in [[0,0,0],[255,255,255],[5,149,231]] {
            assert!(cache.prepare(height,background));
            for row in 0..height {for value in 0..=255u8 {
                let input=[value,255-value,value.wrapping_mul(73)];
                let expected=crate::color::composite(background,input,
                    crate::chronicle::fade(row as f64/height.max(1) as f64));
                let actual:Rgb=std::array::from_fn(|k|cache.rows[row][k][input[k] as usize]);
                assert_eq!(actual,expected,"height={height} row={row} value={value}");
            }}
        }
    }
}

#[test]
fn chronicle_cache_key_capacity_budget_and_reuse_are_bounded() {
    let mut cache=FadeCache::default();assert!(!cache.prepare(0,[0;3]));
    assert!(cache.prepare(292,[5,4,12]));
    let address=cache.rows.as_ptr();let capacity=cache.rows.capacity();
    assert_eq!(cache.rebuilds,1);assert_eq!(cache.generated_rows,292);
    assert!(cache.prepare(292,[5,4,12]));assert_eq!(cache.rebuilds,1);
    assert!(cache.prepare(292,[5,4,13]));assert_eq!(cache.rebuilds,2);
    assert_eq!(cache.rows.as_ptr(),address);assert_eq!(cache.rows.capacity(),capacity);
    assert!(cache.prepare(108,[5,4,13]));assert_eq!(cache.rows.as_ptr(),address);
    let key=cache.key;assert!(!cache.prepare(MAX_CACHED_ROWS+1,[1,2,3]));assert_eq!(cache.key,key);
    assert!(cache.prepare(MAX_CACHED_ROWS,[7,11,19]));
    assert_eq!(cache.rows.len(),MAX_CACHED_ROWS);
    assert_eq!(cache.rows.len()*std::mem::size_of::<RowTable>(),1_572_864);
    cache.invalidate();assert!(cache.key.is_none());assert!(cache.rows.is_empty());
    let capacity=cache.rows.capacity();let rebuilds=cache.rebuilds;
    assert!(cache.prepare(292,[5,4,12]));assert_eq!(cache.rebuilds,rebuilds+1);
    assert_eq!(cache.rows.capacity(),capacity);
}

#[test]
fn chronicle_cached_fade_matches_scalar_under_native_clips_and_arbitrary_alpha() {
    let mut p=Painter::new();
    for(w,h)in [(320,200),(1537,911),(3840,2160)] {
        p.resize(w,h);p.set_responsive_canvas(true);
        if w==1537 {p.origin=(-17,-11);}
        let bounds=native_bounds(&p);
        let initial:Vec<_>=(0..p.pixels.len()).map(|i|i.wrapping_mul(37) as u8).collect();
        for mode in 0..6 {
            let background=[[5,4,12],[0,255,17],[197,13,241]][mode%3];
            p.clip=if mode==0 {None}else{Some((2,3,0,1))};
            p.damage_clip=match mode {
                1=>Some(damage::Rect::new(w/7,7,w/2,h/4)),
                3=>Some(damage::Rect::new(7,9,0,13)),
                4=>Some(damage::Rect::new(w+1,h+1,7,11)),_=>None,
            };
            p.tile_clip_enabled=matches!(mode,1|2|5);p.tile_mask.reset(w,h);
            if mode!=5 {
                p.tile_mask.add(damage::Rect::new(3,2,w/5,h/2));
                p.tile_mask.add(damage::Rect::new(w/3,5,w/3,h/3));
                p.tile_mask.add(damage::Rect::new(w/2,11,w/3,h/2));
            }
            p.tile_mask.finish();
            let(expected,visits)=scalar(&p,&initial,background,bounds);
            for enabled in [false,true,true] {
                p.pixels.copy_from_slice(&initial);p.chronicle_fade_cache_enabled=enabled;
                p.raster_stats=damage::RasterStats{enabled:true,..Default::default()};
                p.paint_chronicle_fade(background,bounds);
                assert_eq!(p.pixels,expected,"{w}x{h} mode={mode} cache={enabled}");
                assert_eq!(p.raster_stats.pixel_visits,visits);
                assert!(p.pixels.chunks_exact(4).zip(initial.chunks_exact(4)).all(|(a,b)|a[3]==b[3]));
            }
        }
        p.clip=None;p.damage_clip=None;p.tile_clip_enabled=false;
    }
}

#[test]
fn chronicle_fade_reuses_height_across_positions_and_falls_back_above_budget() {
    let mut p=Painter::new();p.resize(320,2400);
    let initial:Vec<_>=(0..p.pixels.len()).map(|i|i.wrapping_mul(53) as u8).collect();
    for bounds in [(2,3,211,111),(11,43,123,151),(3,5,127,MAX_CACHED_ROWS as i32+6)] {
        let before=p.chronicle_fade_cache.rebuilds;
        let(expected,visits)=scalar(&p,&initial,[3,17,249],bounds);
        p.pixels.copy_from_slice(&initial);p.raster_stats=damage::RasterStats{enabled:true,..Default::default()};
        p.paint_chronicle_fade([3,17,249],bounds);
        assert_eq!(p.pixels,expected);assert_eq!(p.raster_stats.pixel_visits,visits);
        if bounds.1==3 {assert_eq!(p.chronicle_fade_cache.rebuilds,before+1);}
        else {assert_eq!(p.chronicle_fade_cache.rebuilds,before);}
    }
    p.resize(321,2400);assert!(p.chronicle_fade_cache.key.is_none());
}

#[test]
fn chronicle_fade_cache_matches_complete_game_and_pack_after_resize_and_color_change() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let saved=engine.save().unwrap();let mut view=crate::observe::observe(&engine.run);
    view.measure_conditions(&mut crate::statuses_ui::ConditionMeters::default());
    let mut p=Painter::new();
    for(w,h)in [(1537,911),(3840,2160)] {
        p.resize(w,h);
        for(clock,color)in [(1000.,"#05040c"),(1125.,"#417cab")] {
            p.profile=p.profile.set("ui.colors.background",serde_json::json!(color)).unwrap();
            p.chronicle_fade_cache_enabled=false;p.game_at(&view,"",false,clock);let expected=p.pixels.clone();
            p.chronicle_fade_cache_enabled=true;p.game_at(&view,"",false,clock);
            assert_eq!(p.pixels,expected,"game {w}x{h} clock={clock}");
            let mut pack=crate::pack::PackView::observe(&engine.run);pack.configure(&p,p.frame().map.tuple());
            pack.paint(&mut p);let expected=p.pixels.clone();
            p.chronicle_fade_cache_enabled=false;p.game_at(&view,"",false,clock);pack.paint(&mut p);
            assert_eq!(p.pixels,expected,"Pack {w}x{h} clock={clock}");
        }
    }
    assert_eq!(saved,engine.save().unwrap());
}

/// Isolated CPU fade cost, not a game/FPS/SDL benchmark. All resets, reference
/// pixels, exact byte checks and JSON work occur outside the measured interval.
#[test]
#[ignore="lead-controlled optimized isolated fade benchmark; explicit NEW output path required"]
fn chronicle_fade_cache_cost_profile() {
    use std::{fs,hint::black_box,io::Write,path::PathBuf,time::Instant};
    use serde_json::json;
    assert!(!cfg!(debug_assertions));
    let output=PathBuf::from(std::env::var_os("CORPSE_CHRONICLE_FADE_REPORT").expect("new report path"));
    assert!(output.is_absolute()&&!output.exists());
    let n:usize=std::env::var("CORPSE_CHRONICLE_FADE_SAMPLES").unwrap_or_else(|_|"20".into()).parse().unwrap();
    assert!((4..=100).contains(&n));
    #[cfg(feature="runtime-timing")]
    corpse_core::timing::set_enabled(false);
    let mut cases=Vec::new();let mut p=Painter::new();
    for(w,h)in [(1280,800),(1920,1080),(3840,2160)] {
        p.resize(w,h);p.set_responsive_canvas(true);p.raster_stats.enabled=false;
        let bounds=native_bounds(&p);let height=(bounds.3-bounds.1).max(1) as usize;
        let background=game_color(&p.profile,"ui.colors.background");
        let initial:Vec<_>=(0..p.pixels.len()).map(|i|i.wrapping_mul(37) as u8).collect();
        let(expected,visits)=scalar(&p,&initial,background,bounds);
        let mut samples=Vec::new();let mut raw:[Vec<u64>;4]=std::array::from_fn(|_|Vec::with_capacity(n));
        let names=["old_per_row_tables","cached_warm","cached_rebuild_reused_capacity","cached_first_allocation"];
        for round in 0..n {for order in 0..4 {
            let arm=(round+order)%4;p.pixels.copy_from_slice(&initial);
            p.chronicle_fade_cache_enabled=arm!=0;
            match arm {
                1=>{assert!(p.chronicle_fade_cache.prepare(height,background));},
                2=>{assert!(p.chronicle_fade_cache.prepare(height,background));p.chronicle_fade_cache.invalidate();},
                3=>{p.chronicle_fade_cache=FadeCache::default();},_=>{},
            }
            let generated=p.chronicle_fade_cache.generated_rows;
            let start=Instant::now();p.paint_chronicle_fade(background,bounds);black_box(&p.pixels);
            let elapsed=u64::try_from(start.elapsed().as_nanos()).unwrap();
            assert_eq!(p.pixels,expected,"isolated fade exact parity");raw[arm].push(elapsed);
            samples.push(json!({"round":round,"order":order,"arm":names[arm],"elapsed_ns":elapsed,
                "cache_generated_rows":p.chronicle_fade_cache.generated_rows-generated,
                "old_fallback_generated_rows":if arm==0 {height}else{0},
                "retained_table_rows":p.chronicle_fade_cache.rows.len(),
                "retained_payload_bytes":p.chronicle_fade_cache.rows.len()*std::mem::size_of::<RowTable>(),
                "retained_capacity_bytes":p.chronicle_fade_cache.rows.capacity()*std::mem::size_of::<RowTable>()}));
        }}
        let summaries=raw.into_iter().enumerate().map(|(arm,raw)|{
            let mut sorted=raw.clone();sorted.sort_unstable();let len=sorted.len();
            json!({"arm":names[arm],"raw_ns":raw,"n":len,
                "median_ns":if len%2==0 {(sorted[len/2-1] as f64+sorted[len/2] as f64)/2.}else{sorted[len/2] as f64},
                "p95_ns":sorted[(len*95).div_ceil(100)-1],"min_ns":sorted[0],"max_ns":sorted[len-1]})
        }).collect::<Vec<_>>();
        cases.push(json!({"width":w,"height":h,"fade_bounds":bounds,"fade_height":height,
            "background":background,"pixel_visits":visits,"arms":summaries,"samples_in_execution_order":samples}));
    }
    let report=json!({"schema":"corpse-chronicle-fade-cache-cost-v1","cases":cases,
        "max_cached_rows":MAX_CACHED_ROWS,"max_requested_payload_bytes":MAX_CACHED_ROWS*std::mem::size_of::<RowTable>(),
        "scope":"Synthetic RGBA isolated CPU Chronicle fade with actual default native geometry; not whole-game, GPU, SDL, FPS, input latency or Windows timing. Four arms rotate within one binary. Collector disabled; test-only guards remain. Exact full-buffer comparison, reset, scalar reference, reporting, and warm/reused-capacity preparation are untimed. First-allocation and cold reconstruction occur inside their respective clocks. Cache capacity excludes allocator overhead."});
    if let Some(parent)=output.parent(){fs::create_dir_all(parent).unwrap();}
    fs::OpenOptions::new().create_new(true).write(true).open(&output).unwrap()
        .write_all(&serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("CHRONICLE_FADE_REPORT {}",output.display());
}

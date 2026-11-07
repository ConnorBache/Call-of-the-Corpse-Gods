//! Test-only bounded native software-paint attribution and same-binary ASCII
//! algorithm controls. No SDL, input-loop or Windows timing. OFF is mandatory.
use super::{Painter, View};
use crate::{observe, pack, statuses_ui};
use serde_json::{json, Value};
use std::{cell::RefCell, fs, hint::black_box, path::{Path, PathBuf}, time::Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(super) enum Kind {
    WholePaint, Game, Background, Chronicle, TerrainPreparation, DreamPreparation,
    Map, HudResources, Enemies, HudAbilities, Interactions, Footer, PackConfigure, Pack,
    Clear, FillOpaque, FillAlpha, Mask, ScaledMaskUnit, ScaledMaskOther,
    AsciiUnit, AsciiScaled, Compound, Text, TextRole, TextMetrics, Prose, Picture, Fade,
}
const KINDS: [Kind; 29] = [
    Kind::WholePaint, Kind::Game, Kind::Background, Kind::Chronicle,
    Kind::TerrainPreparation, Kind::DreamPreparation, Kind::Map, Kind::HudResources,
    Kind::Enemies, Kind::HudAbilities, Kind::Interactions, Kind::Footer,
    Kind::PackConfigure, Kind::Pack, Kind::Clear, Kind::FillOpaque, Kind::FillAlpha,
    Kind::Mask, Kind::ScaledMaskUnit, Kind::ScaledMaskOther, Kind::AsciiUnit,
    Kind::AsciiScaled, Kind::Compound, Kind::Text, Kind::TextRole, Kind::TextMetrics,
    Kind::Prose, Kind::Picture, Kind::Fade,
];
const COUNT: usize = KINDS.len();
const LAYERS: usize = Kind::Clear as usize;
const MAX_DEPTH: usize = 32;
impl Kind {
    fn name(self) -> &'static str {
        ["whole_paint", "game", "background", "chronicle", "animated_terrain_preparation",
        "dream_preparation", "map", "hud_resources", "enemies", "hud_abilities",
        "interactions", "footer", "pack_configure", "pack", "clear", "fill_opaque",
        "fill_alpha", "blend_mask", "blend_scaled_mask_alpha1", "blend_scaled_mask_nonunit",
        "ascii_alpha1", "ascii_nonunit", "compound_prepare", "text", "text_role",
        "text_metrics", "prose", "picture", "chronicle_fade"][self as usize]
    }
    fn is_layer(self) -> bool { (self as usize) < LAYERS }
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
struct Stat {
    calls: u64, inclusive_ns: u64, exclusive_ns: u64, pixel_visits: u64, cache_misses: u64,
    ascii_clipped_full_mask_pixels: u64, ascii_clipped_nonzero_pixels: u64,
    ascii_clipped_nonzero_runs: u64,
}
impl Stat {
    fn add(&mut self, other: Self) {
        self.calls += other.calls; self.inclusive_ns += other.inclusive_ns;
        self.exclusive_ns += other.exclusive_ns; self.pixel_visits += other.pixel_visits;
        self.cache_misses += other.cache_misses;
        self.ascii_clipped_full_mask_pixels += other.ascii_clipped_full_mask_pixels;
        self.ascii_clipped_nonzero_pixels += other.ascii_clipped_nonzero_pixels;
        self.ascii_clipped_nonzero_runs += other.ascii_clipped_nonzero_runs;
    }
}
#[derive(Clone, Copy)]
struct Frame { kind: Kind, layer: Kind, child_ns: u64 }
const EMPTY_FRAME: Frame = Frame { kind: Kind::WholePaint, layer: Kind::WholePaint, child_ns: 0 };
struct Collector {
    enabled: bool, depth: usize, stack: [Frame; MAX_DEPTH],
    totals: [Stat; COUNT], by_layer: [[Stat; COUNT]; LAYERS],
}
impl Collector {
    const fn new() -> Self {
        const ZERO: Stat = Stat { calls:0, inclusive_ns:0, exclusive_ns:0, pixel_visits:0, cache_misses:0, ascii_clipped_full_mask_pixels:0, ascii_clipped_nonzero_pixels:0, ascii_clipped_nonzero_runs:0 };
        Self { enabled:false, depth:0, stack:[EMPTY_FRAME;MAX_DEPTH], totals:[ZERO;COUNT], by_layer:[[ZERO;COUNT];LAYERS] }
    }
}
thread_local! { static COLLECTOR: RefCell<Collector> = const { RefCell::new(Collector::new()) }; }

/// One fixed-array update per completed span, O(1) time/memory per call; no
/// allocation or clock call per pixel. Local row counters are conditional on ON.
/// OFF still pays test-only guard checks; compare both arms rather than assuming
/// instrumentation is free. Every added call is absent from non-test binaries.
pub(super) struct Span { started: Option<Instant>, kind: Kind, pixels: u64, misses: u64, ascii_full:u64, ascii_nonzero:u64, ascii_runs:u64 }
pub(super) fn span(kind: Kind) -> Span {
    let started = COLLECTOR.with(|cell| {
        let mut c = cell.borrow_mut();
        if !c.enabled { return None; }
        assert!(c.depth < MAX_DEPTH, "bounded profiling stack exhausted");
        let depth = c.depth;
        let layer = if kind.is_layer() {kind} else if depth == 0 {Kind::WholePaint} else {c.stack[depth-1].layer};
        c.stack[depth] = Frame {kind, layer, child_ns:0}; c.depth += 1;
        Some(Instant::now())
    });
    Span {started, kind, pixels:0, misses:0, ascii_full:0, ascii_nonzero:0, ascii_runs:0}
}
impl Span {
    #[inline] pub(super) fn enabled(&self) -> bool { self.started.is_some() }
    #[inline] pub(super) fn ascii_coverage(&mut self, full:usize, nonzero:usize, runs:usize) {
        if self.started.is_some() {
            self.ascii_full += full as u64; self.ascii_nonzero += nonzero as u64; self.ascii_runs += runs as u64;
        }
    }
    #[inline] pub(super) fn pixels(&mut self, n: usize) { if self.started.is_some() {self.pixels += n as u64;} }
    #[inline] pub(super) fn cache_miss(&mut self) { if self.started.is_some() {self.misses += 1;} }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some(started) = self.started else {return;};
        let inclusive = ns(started);
        COLLECTOR.with(|cell| {
            let mut c = cell.borrow_mut(); assert!(c.depth > 0);
            c.depth -= 1; let depth = c.depth; let frame = c.stack[depth];
            assert_eq!(frame.kind, self.kind, "profiling scopes must be LIFO");
            let stat = Stat {calls:1, inclusive_ns:inclusive,
                exclusive_ns:inclusive.checked_sub(frame.child_ns).expect("nested time exceeds parent"),
                pixel_visits:self.pixels, cache_misses:self.misses,
                ascii_clipped_full_mask_pixels:self.ascii_full, ascii_clipped_nonzero_pixels:self.ascii_nonzero,
                ascii_clipped_nonzero_runs:self.ascii_runs};
            c.totals[self.kind as usize].add(stat);
            if !self.kind.is_layer() { c.by_layer[frame.layer as usize][self.kind as usize].add(stat); }
            if depth > 0 {c.stack[depth-1].child_ns += inclusive;}
        });
    }
}
fn begin(enabled: bool) {
    COLLECTOR.with(|cell| {
        let mut c=cell.borrow_mut(); assert_eq!(c.depth,0);
        c.totals.fill(Stat::default()); for row in &mut c.by_layer {row.fill(Stat::default());}
        c.enabled=enabled;
    });
}
fn finish() -> ([Stat; COUNT], [[Stat; COUNT]; LAYERS]) {
    COLLECTOR.with(|cell| {let mut c=cell.borrow_mut(); assert_eq!(c.depth,0);c.enabled=false;(c.totals,c.by_layer)})
}
fn ns(start: Instant) -> u64 {u64::try_from(start.elapsed().as_nanos()).unwrap()}
fn summary(raw: Vec<u64>) -> Value {
    assert!(!raw.is_empty()); let mut s=raw.clone(); s.sort_unstable(); let n=s.len();
    json!({"n":n,"median_ns":if n%2==0 {(s[n/2-1] as f64+s[n/2] as f64)/2.}else{s[n/2] as f64},
        "p95_ns":s[(n*95).div_ceil(100)-1],"p99_ns":s[(n*99).div_ceil(100)-1],
        "min_ns":s[0],"max_ns":s[n-1],"raw_ns":raw})
}
fn stat_rows(stats: &[Stat; COUNT]) -> Vec<Value> {
    KINDS.iter().filter_map(|kind| {let s=stats[*kind as usize]; (s.calls>0).then(||json!({"scope":kind.name(),"is_scene_layer":kind.is_layer(),"stats":s}))}).collect()
}
fn layer_rows(stats: &[[Stat; COUNT]; LAYERS]) -> Vec<Value> {
    (0..LAYERS).filter_map(|i| {let rows=stat_rows(&stats[i]); (!rows.is_empty()).then(||json!({"layer":KINDS[i].name(),"primitives":rows}))}).collect()
}
fn detached(run: &corpse_core::Run) -> View {
    let mut view=observe::observe(run); view.provenance.clear();
    view.measure_conditions(&mut statuses_ui::ConditionMeters::default()); view
}
fn resize(p:&mut Painter,w:usize,h:usize) {
    let _=p.resize(w,h); assert_eq!((p.width,p.height),(w,h));
}
fn paint(p:&mut Painter,v:&View,pack:&mut Option<pack::PackView>,clock:f64) {
    let _root=span(Kind::WholePaint);
    p.game_at(v,"",false,clock);
    if let Some(pack)=pack {
        {let _configure=span(Kind::PackConfigure);pack.configure(p,p.frame().map.tuple());}
        let _pack=span(Kind::Pack); pack.paint(p);
    }
    black_box(&p.pixels);
}
#[derive(Clone, Copy)]
enum Mode { Warm, Cold, Transition }
impl Mode {fn name(self)->&'static str {match self {Self::Warm=>"warm",Self::Cold=>"first_paint_after_resize",Self::Transition=>"changed_full_frame"}}}

/// 2n alternating OFF/ON paints of identical inputs. Hashes, collector reset and
/// serialization are outside the whole-paint clock. Transition preparation is a
/// real preceding full frame, never an extra dispatched command in the timer.
fn measure(p:&mut Painter,view:&View,pack:&mut Option<pack::PackView>,clock:f64,
    n:usize,mode:Mode,previous:Option<(&View,f64)>,compare_sparse:bool) -> Value {
    if compare_sparse {return measure_sparse(p,view,pack,clock,n,mode,previous);}
    let (w,h)=(p.width,p.height); begin(false);paint(p,view,pack,clock);let _=finish();
    let expected=crc32fast::hash(&p.pixels);
    let (mut off,mut on,mut samples)=(Vec::with_capacity(n),Vec::with_capacity(n),Vec::with_capacity(n*2));
    let mut totals=[Stat::default();COUNT];let mut layer_totals=[[Stat::default();COUNT];LAYERS];
    for pair in 0..n {
        for (order,enabled) in if pair%2==0 {[false,true]}else{[true,false]}.into_iter().enumerate() {
            begin(false);p.raster_stats.enabled=false;
            match mode {
                Mode::Cold=>{resize(p,w+1,h);resize(p,w,h);},
                Mode::Transition=>{let (before,time)=previous.expect("transition has preceding actual frame");paint(p,before,pack,time);},
                Mode::Warm=>{},
            }
            let _=finish();
            p.raster_stats=super::damage::RasterStats {enabled,..Default::default()};
            begin(enabled);let start=Instant::now();paint(p,view,pack,clock);let whole=ns(start);
            let(stats,layers)=finish();p.raster_stats.enabled=false;
            let crc=crc32fast::hash(&p.pixels);assert_eq!(crc,expected,"collector must not alter exact pixels");
            let exclusive:u64=stats.iter().map(|s|s.exclusive_ns).sum();
            let visits:u64=stats.iter().map(|s|s.pixel_visits).sum();
            if enabled {
                assert_eq!(stats[Kind::WholePaint as usize].calls,1);
                assert_eq!(exclusive,stats[Kind::WholePaint as usize].inclusive_ns,"exclusive partition must reconcile per sample");
                assert!(exclusive<=whole);assert!(visits<=p.raster_stats.pixel_visits,"no double-counted pixel visits");
                on.push(whole);for i in 0..COUNT {totals[i].add(stats[i]);}
                for layer in 0..LAYERS {for i in 0..COUNT {layer_totals[layer][i].add(layers[layer][i]);}}
            } else {assert_eq!(exclusive,0);assert_eq!(visits,0);off.push(whole);}
            samples.push(json!({"pair":pair,"order_in_pair":order,"collector_enabled":enabled,
                "whole_paint_ns":whole,"rgba_crc32":format!("{crc:08x}"),"scopes":stat_rows(&stats),
                "primitive_by_nearest_scene_layer":layer_rows(&layers),"exclusive_partition_ns":exclusive,
                "outside_root_ns":whole-exclusive,"counted_clipped_pixel_visits":if enabled {json!(p.raster_stats.pixel_visits)}else{Value::Null},
                "attributed_pixel_visits":if enabled {json!(visits)}else{Value::Null},
                "unattributed_sparse_pixel_visits":if enabled {json!(p.raster_stats.pixel_visits-visits)}else{Value::Null},
                "cells_considered":if enabled {json!(p.raster_stats.cells_considered)}else{Value::Null},
                "cells_painted":if enabled {json!(p.raster_stats.cells_painted)}else{Value::Null}}));
        }
    }
    json!({"mode":mode.name(),"pairs":n,"collector_off_whole_paint":summary(off),
        "collector_on_whole_paint":summary(on),"collector_on_aggregate_scopes":stat_rows(&totals),
        "collector_on_aggregate_by_scene_layer":layer_rows(&layer_totals),"samples_in_execution_order":samples})
}
/// Outside paint clocks: retained cache occupancy, not a sampled demonstration
/// glyph. Payload and capacity are distinguished; allocator/map overhead excluded.
fn ascii_cache_occupancy(p:&Painter) -> Value {
    let mut entries=Vec::new();
    let(mut bytes,mut nonzero,mut runs,mut offsets,mut metadata,mut capacity)=(0usize,0usize,0usize,0usize,0usize,0usize);
    for (&(ch,w,h,weight),mask) in &p.map_masks {
        let occupied=mask.runs.iter().map(|&(a,b)|(b-a) as usize).sum::<usize>();
        let meta=mask.runs.len()*std::mem::size_of::<(i32,i32)>()+mask.row_offsets.len()*std::mem::size_of::<u32>();
        let cap=mask.runs.capacity()*std::mem::size_of::<(i32,i32)>()+mask.row_offsets.capacity()*std::mem::size_of::<u32>();
        bytes+=mask.coverage.len();nonzero+=occupied;runs+=mask.runs.len();offsets+=mask.row_offsets.len();metadata+=meta;capacity+=cap;
        entries.push(json!({"character":ch.to_string(),"width":w,"height":h,"weight":weight,
            "coverage_bytes":mask.coverage.len(),"coverage_capacity_bytes":mask.coverage.capacity(),
            "zero_pixels":mask.coverage.len()-occupied,"nonzero_pixels":occupied,"nonzero_runs":mask.runs.len(),
            "row_offsets":mask.row_offsets.len(),"run_metadata_bytes":meta,"run_metadata_capacity_bytes":cap}));
    }
    entries.sort_by_key(|v|(v["character"].as_str().unwrap().to_owned(),v["width"].as_i64(),v["height"].as_i64(),v["weight"].as_u64()));
    json!({"cached_masks":entries.len(),"coverage_bytes":bytes,"zero_pixels":bytes-nonzero,
        "nonzero_pixels":nonzero,"nonzero_runs":runs,"row_offsets":offsets,
        "run_metadata_bytes":metadata,"run_metadata_capacity_bytes":capacity,
        "coverage_capacity_bytes":p.map_masks.values().map(|m|m.coverage.capacity()).sum::<usize>(),
        "cache_value_inline_bytes":p.map_masks.len()*std::mem::size_of::<super::ascii_mask::AsciiMask>(),
        "entries":entries,"scope":"Existing coverage payload plus added flat run/offset vectors; lengths and capacities separately reported. Excludes allocator overhead, HashMap keys/buckets and unrelated caches. Both same-binary controls construct and retain this metadata."})
}

/// Eight rotating arms per round: full/sparse ASCII x two-pass/fused background
/// x collector OFF/ON. The reference disables both candidate algorithms.
/// Rotation balances execution position; all paints use the identical detached
/// scene/clock. OFF pays no diagnostic run scan. ON has no per-pixel clocks.
fn measure_sparse(p:&mut Painter,view:&View,pack:&mut Option<pack::PackView>,clock:f64,
    n:usize,mode:Mode,previous:Option<(&View,f64)>) -> Value {
    let(w,h)=(p.width,p.height);p.ascii_sparse_enabled=false;p.background_fused_enabled=false;p.raster_stats.enabled=false;
    begin(false);paint(p,view,pack,clock);let _=finish();
    let expected_pixels=p.pixels.clone();let expected=crc32fast::hash(&expected_pixels);
    let mut raw:[Vec<u64>;8]=std::array::from_fn(|_|Vec::with_capacity(n));
    let mut totals=[[Stat::default();COUNT];8];
    let mut layers_total=[[[Stat::default();COUNT];LAYERS];8];
    let mut samples=Vec::with_capacity(8*n);
    let arms=[(false,false,false),(true,false,false),(false,true,false),(true,true,false),
        (false,false,true),(true,false,true),(false,true,true),(true,true,true)];
    for round in 0..n {
        for order in 0..8 {
            let arm=(round+order)%8;let(sparse,fused,enabled)=arms[arm];
            p.ascii_sparse_enabled=sparse;p.background_fused_enabled=fused;begin(false);p.raster_stats.enabled=false;
            match mode {
                Mode::Cold=>{resize(p,w+1,h);resize(p,w,h);},
                Mode::Transition=>{let(before,time)=previous.expect("transition has preceding frame");paint(p,before,pack,time);},
                Mode::Warm=>{},
            }
            let _=finish();p.raster_stats=super::damage::RasterStats{enabled,..Default::default()};
            begin(enabled);let start=Instant::now();paint(p,view,pack,clock);let whole=ns(start);
            let(stats,layers)=finish();p.raster_stats.enabled=false;
            assert_eq!(p.pixels,expected_pixels,"ASCII/background/collector controls must preserve every framebuffer byte");
            let crc=crc32fast::hash(&p.pixels);assert_eq!(crc,expected,"full/sparse frame CRC");
            let exclusive=stats.iter().map(|s|s.exclusive_ns).sum::<u64>();
            let visits=stats.iter().map(|s|s.pixel_visits).sum::<u64>();
            if enabled {
                assert_eq!(stats[Kind::WholePaint as usize].calls,1);
                assert_eq!(exclusive,stats[Kind::WholePaint as usize].inclusive_ns);
                assert!(exclusive<=whole);assert!(visits<=p.raster_stats.pixel_visits);
                let ascii=stats[Kind::AsciiUnit as usize];
                assert_eq!(ascii.pixel_visits,if sparse {ascii.ascii_clipped_nonzero_pixels}else{ascii.ascii_clipped_full_mask_pixels});
                for i in 0..COUNT {totals[arm][i].add(stats[i]);}
                for l in 0..LAYERS {for i in 0..COUNT {layers_total[arm][l][i].add(layers[l][i]);}}
            } else {assert_eq!(exclusive,0);assert_eq!(visits,0);}
            raw[arm].push(whole);
            samples.push(json!({"round":round,"order_in_round":order,"sparse_ascii_enabled":sparse,
                "fused_background_enabled":fused,"collector_enabled":enabled,"whole_paint_ns":whole,"rgba_crc32":format!("{crc:08x}"),
                "scopes":stat_rows(&stats),"primitive_by_nearest_scene_layer":layer_rows(&layers),
                "exclusive_partition_ns":exclusive,"outside_root_ns":whole-exclusive,
                "counted_clipped_pixel_visits":if enabled {json!(p.raster_stats.pixel_visits)}else{Value::Null},
                "attributed_pixel_visits":if enabled {json!(visits)}else{Value::Null},
                "unattributed_sparse_pixel_visits":if enabled {json!(p.raster_stats.pixel_visits-visits)}else{Value::Null},
                "cells_considered":if enabled {json!(p.raster_stats.cells_considered)}else{Value::Null},
                "cells_painted":if enabled {json!(p.raster_stats.cells_painted)}else{Value::Null}}));
        }
    }
    p.ascii_sparse_enabled=true;p.background_fused_enabled=true;
    json!({"mode":mode.name(),"rounds":n,"same_binary_sparse_comparison":true,"same_binary_fused_background_comparison":true,
        "arms":arms.iter().enumerate().map(|(i,&(sparse,fused,enabled))|json!({
            "sparse_ascii_enabled":sparse,"fused_background_enabled":fused,"collector_enabled":enabled,"whole_paint":summary(std::mem::take(&mut raw[i])),
            "aggregate_scopes":stat_rows(&totals[i]),"aggregate_by_scene_layer":layer_rows(&layers_total[i])})).collect::<Vec<_>>(),
        "ascii_cache_occupancy":ascii_cache_occupancy(p),"samples_in_execution_order":samples})
}

fn write_new(path:&Path,bytes:&[u8]) {
    use std::io::Write;
    if let Some(parent)=path.parent() {fs::create_dir_all(parent).unwrap();}
    fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap().write_all(bytes).unwrap();
}

#[test]
fn paint_profile_nested_exclusive_and_disabled_control() {
    begin(false); {let mut root=span(Kind::WholePaint);root.pixels(99);let _child=span(Kind::Text);}
    let(stats,layers)=finish();assert!(stats.iter().all(|s|s.calls==0));assert!(layers.iter().flatten().all(|s|s.calls==0));
    begin(true);{let _root=span(Kind::WholePaint);let _map=span(Kind::Map);{let mut text=span(Kind::Text);text.cache_miss();let mut mask=span(Kind::Mask);mask.pixels(37);}}
    let(stats,layers)=finish();assert_eq!(stats[Kind::Mask as usize].pixel_visits,37);
    assert_eq!(layers[Kind::Map as usize][Kind::Mask as usize].pixel_visits,37);
    assert_eq!(stats[Kind::Text as usize].cache_misses,1);
    assert_eq!(stats.iter().map(|s|s.exclusive_ns).sum::<u64>(),stats[Kind::WholePaint as usize].inclusive_ns);
}
#[test]
fn paint_profile_summary_preserves_raw() {
    let s=summary(vec![4,1,3,2]);assert_eq!(s["median_ns"],2.5);assert_eq!(s["p95_ns"],4);assert_eq!(s["raw_ns"],json!([4,1,3,2]));
}

#[test]
fn paint_profile_ascii_occupancy_is_weighted_and_clipped() {
    let mut p=Painter::new();let(w,h)=(12,8);
    let coverage:Vec<_>=(0..w*h).map(|i|if i%w>=3 && i%w<9 {127}else{0}).collect();
    let weight=p.profile.number("typography.map.fontWeight").round().clamp(100.,900.) as u16;
    p.map_masks.insert(('@',w,h,weight),super::ascii_mask::AsciiMask::new(coverage,w as usize,h as usize));
    p.clip=Some((4,2,6,4));
    for sparse in [false,true] {
        p.ascii_sparse_enabled=sparse;p.raster_stats=super::damage::RasterStats{enabled:true,..Default::default()};
        begin(true);
        for _ in 0..3 {p.map_glyph('@',0,0,w,h,[13,19,23]);}
        let(stats,_)=finish();let ascii=stats[Kind::AsciiUnit as usize];
        assert_eq!(ascii.calls,3);
        assert_eq!(ascii.ascii_clipped_full_mask_pixels,3*6*4);
        assert_eq!(ascii.ascii_clipped_nonzero_pixels,3*5*4);
        assert_eq!(ascii.ascii_clipped_nonzero_runs,3*4);
        assert_eq!(ascii.pixel_visits,if sparse {3*5*4}else{3*6*4});
        assert_eq!(ascii.pixel_visits,p.raster_stats.pixel_visits);
    }
    let cache=ascii_cache_occupancy(&p);
    assert_eq!(cache["coverage_bytes"],w*h);
    assert_eq!(cache["nonzero_pixels"],6*h);
    assert_eq!(cache["zero_pixels"],6*h);
    assert_eq!(cache["nonzero_runs"],h);
    assert_eq!(cache["run_metadata_bytes"],8*h+4*(h+1));
}

#[test]
#[ignore="lead-controlled optimized paint profile; explicit real tick48 input and new output required"]
fn real_tick48_native_paint_profile() { run_profile(false); }

#[test]
#[ignore="lead-controlled optimized sparse ASCII comparison; explicit real tick48 input and new output required"]
fn real_tick48_sparse_ascii_profile() { run_profile(true); }

fn run_profile(compare_sparse:bool) {
    assert!(!cfg!(debug_assertions),"profile only an optimized release binary");
    let input=PathBuf::from(std::env::var_os("CORPSE_PAINT_INPUT").expect("real normalized tick48 roundtrip.json"));
    let output=PathBuf::from(std::env::var_os("CORPSE_PAINT_OUTPUT").expect("new absolute report path"));
    let variant=std::env::var("CORPSE_PAINT_VARIANT").expect("exact source variant label");
    assert!(input.is_absolute()&&output.is_absolute());assert!(!output.exists(),"never overwrite earlier evidence");
    assert!(!variant.is_empty()&&variant.chars().all(|c|c.is_ascii_alphanumeric()||c=='-'||c=='_'));
    let n:usize=std::env::var("CORPSE_PAINT_SAMPLES").unwrap_or_else(|_|"20".into()).parse().unwrap();assert!((3..=100).contains(&n));
    #[cfg(feature="runtime-timing")] corpse_core::timing::set_enabled(false);
    let raw=fs::read_to_string(&input).unwrap();let(engine,_)=crate::session_store::decode(&raw).unwrap();
    assert_eq!(engine.run.tick,48.);let original=engine.save().unwrap();let view=detached(&engine.run);
    let mut rows=Vec::new();
    for(width,height)in [(1280,800),(1920,1080),(3840,2160)] {
        for with_pack in [false,true] {
            let mut p=Painter::new();resize(&mut p,width,height);
            let mut pack=with_pack.then(||pack::PackView::observe(&engine.run));
            paint(&mut p,&view,&mut pack,1000.);
            if let Some(pack)=pack.as_mut() {assert_eq!(pack.key(sdl2::keyboard::Keycode::Home,sdl2::keyboard::Mod::NOMOD),pack::PackAction::None);}
            for _ in 0..3 {paint(&mut p,&view,&mut pack,1000.);}
            let warm=measure(&mut p,&view,&mut pack,1000.,n,Mode::Warm,None,compare_sparse);
            let cold=measure(&mut p,&view,&mut pack,1000.,3,Mode::Cold,None,compare_sparse);
            rows.push(json!({"case":if with_pack {"tick48_selected_pack"}else{"tick48_game"},"width":width,"height":height,
                "clock_ms":1000.,"observed_cells":view.cells.len(),"measurements":[warm,cold]}));
        }
    }
    // One genuine accepted movement on an isolated engine clone, strictly outside
    // paint samples. No repeated gameplay, visibility override, or fake actors.
    let origin=(engine.run.player.x,engine.run.player.y);let mut moved=None;let mut attempted=Vec::new();
    for(dx,dy)in [(1.,0.),(0.,1.),(-1.,0.),(0.,-1.),(1.,1.),(-1.,1.),(-1.,-1.),(1.,-1.)] {
        let mut candidate=engine.clone();let command=corpse_core::Command::Move{dx,dy};
        match candidate.dispatch(&command) {
            Ok(out)=>{let destination=(candidate.run.player.x,candidate.run.player.y);attempted.push(json!({"delta":[dx,dy],"took_turn":out.took_turn,"changed":out.changed,"destination":destination}));
                if out.changed&&destination!=origin {moved=Some((candidate,command));break;}},
            Err(error)=>attempted.push(json!({"delta":[dx,dy],"error":error.to_string()})),
        }
    }
    let movement=if let Some((moved,command))=moved {
        let saved=moved.save().unwrap();let after=detached(&moved.run);let mut p=Painter::new();resize(&mut p,1920,1080);let mut pack=None;
        let measurement=measure(&mut p,&after,&mut pack,1000.,n,Mode::Transition,Some((&view,1000.)),compare_sparse);
        assert_eq!(saved,moved.save().unwrap());rows.push(json!({"case":"actual_post_move_game","width":1920,"height":1080,
            "clock_ms":1000.,"tick":moved.run.tick,"command":command,"from":origin,"to":[moved.run.player.x,moved.run.player.y],
            "observed_cells":after.cells.len(),"measurements":[measurement]}));json!({"available":true,"attempts":attempted})
    }else{json!({"available":false,"reason":"No accepted player movement among eight bounded direction probes; not replaced with fabricated world edits.","attempts":attempted})};
    let mut p=Painter::new();resize(&mut p,1920,1080);let mut pack=None;
    paint(&mut p,&view,&mut pack,1000.);let before_crc=crc32fast::hash(&p.pixels);paint(&mut p,&view,&mut pack,1125.);let after_crc=crc32fast::hash(&p.pixels);
    rows.push(json!({"case":"actual_cosmetic_clock_transition","width":1920,"height":1080,"from_clock_ms":1000.,"clock_ms":1125.,
        "observed_animated_cells":view.cells.iter().filter(|c|!c.animations.is_empty()||c.pulse_speed!=0.).count(),
        "pixels_actually_changed":before_crc!=after_crc,"before_crc32":format!("{before_crc:08x}"),"after_crc32":format!("{after_crc:08x}"),
        "measurements":[measure(&mut p,&view,&mut pack,1125.,n,Mode::Transition,Some((&view,1000.)),compare_sparse)]}));
    let mut hover=detached(&engine.run);
    let target=view.cells.iter().find(|c|c.current&&(c.x,c.y)!=(origin.0 as usize,origin.1 as usize)).map(|c|(c.x,c.y));
    if let Some(xy)=target {
        let path=crate::travel_ui::path_to(&engine.run,xy.0,xy.1,true).iter().map(|s|(s.x,s.y)).collect();
        crate::sync_pointer_view(&mut hover,&engine.run,Some(xy),path,None);
        rows.push(json!({"case":"actual_pointer_hover_projection","width":1920,"height":1080,"target":xy,"clock_ms":1000.,
            "scope":"Real observed-cell route and pointer projection before timing; game path/highlight only, no invented creature tooltip.",
            "measurements":[measure(&mut p,&hover,&mut pack,1000.,n,Mode::Transition,Some((&view,1000.)),compare_sparse)]}));
    }
    assert_eq!(original,engine.save().unwrap(),"source run/RNG/replay unchanged");
    let report=json!({"schema":if compare_sparse {"corpse-native-sparse-fused-profile-v1"}else{"corpse-native-paint-profile-v1"},"variant":variant,"input":input,
        "same_binary_sparse_comparison":compare_sparse,"same_binary_fused_background_comparison":compare_sparse,
        "fixture_crc32":format!("{:08x}",crc32fast::hash(raw.as_bytes())),"seed":engine.run.seed,"tick":engine.run.tick,
        "samples_per_arm":n,"cold_samples_per_arm":3,"rows":rows,"movement_setup":movement,"source_game_state_unchanged":true,
        "scope":"Detached actual observations; full native game paint and selected Pack. No SDL upload/present, input event loop, Windows runtime, simulation, save, observation or route timing. Movement dispatch, route, snapshot construction, constructor/resize, hash, reset, reporting and setup remain outside paint clocks.",
        "attribution":"Each raw sample retains its own whole paint and subspans. Inclusive scopes overlap and MUST NOT be added. Exclusive elapsed subtracts immediate child intervals and partitions that same sample's root. Category totals preserve nested same-kind calls. Primitive-by-layer entries use the nearest enclosing scene layer, with the same exclusive values. Terrain/dream preparation is scene construction, not pixel work.",
        "counters":"ON enables test-only spans and existing raster counts. Pixel visits are actual clipped raster-loop visits and repeated overdraw, not unique changed pixels or nonzero writes. Full-mask loops include zero coverage; sparse ASCII unit-opacity loops omit it. Sparse native_pixel work has no per-pixel timer and appears as unattributed_sparse_pixel_visits; its time remains in its parent scope. Cache misses count text/styled-text/ASCII/compound cache insertions, not memory bytes. OFF disables collectors and counters but still has test-only guard checks; non-test builds compile these additions out.",
        "sparse_control_contract":"When enabled, eight arms rotate full-mask/sparse ASCII, two-pass/fused background and collector OFF/ON within the same binary. The expected framebuffer uses both candidates disabled, and every arm compares every byte outside timing. Both algorithms retain the same run cache metadata; this does not isolate metadata creation cost versus the previous alpha-only binary. ASCII unit-opacity counters report weighted actually clipped full mask pixels, nonzero pixels and nonzero runs, including repeated draws; sparse pixel_visits counts only traversed nonzero samples. ON full-mask control additionally reads cached runs to count occupancy; no such scan occurs with collector OFF. Cache occupancy is enumerated outside all paint clocks. Other glyph paths are unchanged.",
        "controls":"The original harness alternates OFF/ON per pair. Sparse/fused-comparison mode rotates all eight algorithm/collector arms by round in one binary, with exact CRC parity. ON timings include instrumentation overhead and are diagnostic, never a production latency estimate. OFF is the whole-paint timing control. First-paint resets native caches via real resize; allocation is outside timing. Cold n3 is diagnostic, not a tail estimate. Samples are not SDL frames per second.",
        "quantiles":"Median averages middle pair; p95/p99 use nearest rank. Raw ns retained; n20 p99 is maximum. Source/compiler/build command must accompany each report."});
    write_new(&output,&serde_json::to_vec_pretty(&report).unwrap());println!("NATIVE_PAINT_PROFILE_REPORT {}",output.display());
}

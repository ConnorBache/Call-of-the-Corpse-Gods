//! Lead-controlled native software-paint evidence. Included as a test-only child
//! of render.rs so the unchanged native raster caches can be inspected directly.
//! No SDL, command dispatch, fixture edits, visibility overrides, or GPU claim.
use super::{Painter, View};
use crate::{modal_ui, observe, pack, statuses_ui};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, hint::black_box, path::{Path, PathBuf}, time::Instant};

const CLOCK_MS: f64 = 1000.;
const NOTICE: &str = "";
const SIZES: [(usize, usize); 5] = [(1280,800),(1920,1080),(3840,2160),(1537,911),(960,1200)];

// The frozen baseline has no inherent frame()/logical_*() methods. Rust chooses
// the candidate's real inherent methods when present; the same harness source
// therefore follows each version's production frame geometry without a switch.
#[allow(dead_code)]
trait BaselineFrame {
    fn frame(&self) -> modal_ui::Frame;
    fn logical_width(&self) -> i32;
    fn logical_height(&self) -> i32;
}
impl BaselineFrame for Painter {
    fn frame(&self) -> modal_ui::Frame { modal_ui::Frame::from_profile(&self.profile) }
    fn logical_width(&self) -> i32 { super::WIDTH as i32 }
    fn logical_height(&self) -> i32 { super::HEIGHT as i32 }
}

fn ns(start: Instant) -> u64 { u64::try_from(start.elapsed().as_nanos()).unwrap() }

/// O(n log n) summary, O(n) copied samples; all summarization is outside timing.
fn summary(raw: Vec<u64>) -> Value {
    assert!(!raw.is_empty());
    let mut sorted = raw.clone(); sorted.sort_unstable(); let n = sorted.len();
    let rank = |p: usize| sorted[(n*p).div_ceil(100)-1];
    let median = if n%2==0 {(sorted[n/2-1] as f64+sorted[n/2] as f64)/2.} else {sorted[n/2] as f64};
    json!({"n":n,"median_ns":median,"p95_ns":rank(95),"p99_ns":rank(99),
        "min_ns":sorted[0],"max_ns":sorted[n-1],"raw_ns":raw})
}

#[test]
fn resolution_summary_preserves_raw_and_nearest_rank() {
    let s = summary(vec![4,1,3,2]);
    assert_eq!(s["median_ns"],2.5); assert_eq!(s["p95_ns"],4);
    assert_eq!(s["p99_ns"],4); assert_eq!(s["raw_ns"],json!([4,1,3,2]));
}

fn resize(p: &mut Painter, w: usize, h: usize) {
    let _ = p.resize(w,h);
    assert_eq!((p.width,p.height),(w,h),"requested valid native size must be accepted");
    assert_eq!(p.pixels.len(),w.checked_mul(h).unwrap().checked_mul(4).unwrap());
}

/// O(output pixels + actually painted masks/text). Pack configuration is its real
/// production geometry/metrics path; the canonical Run is never available here.
fn paint(p: &mut Painter, v: &View, pack: &mut Option<pack::PackView>) {
    p.game_at(v,NOTICE,false,CLOCK_MS);
    if let Some(pack) = pack {
        pack.configure(p,p.frame().map.tuple());
        pack.paint(p);
    }
    black_box(&p.pixels);
}

fn write_new(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap()
        .write_all(bytes).unwrap();
}

fn png(path: &Path, p: &Painter) {
    let file = fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
    let mut encoder = png::Encoder::new(file,p.width as u32,p.height as u32);
    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}

/// Enumerates actual cached masks after the real scene, not a demonstration glyph
/// added to the frame. Payload byte counts are lengths only, not total heap/RSS.
fn raster_proof(p: &Painter) -> Value {
    let mut text = p.glyphs.iter().map(|((ch,size),(m,b))|
        json!({"character":ch.to_string(),"raster_px":size,"mask_width":m.width,
            "mask_height":m.height,"mask_payload_bytes":b.len()})).collect::<Vec<_>>();
    text.sort_by_key(|v|(v["raster_px"].as_u64(),v["character"].as_str().unwrap().to_owned()));
    let sizes = p.glyphs.keys().map(|(_,s)|*s).collect::<BTreeSet<_>>();
    let styled_sizes = p.styled_glyphs.keys().map(|(_,s,_,_)|*s).collect::<BTreeSet<_>>();
    let prose_sizes = p.prose_glyphs.keys().map(|(_,s,_)|*s).collect::<BTreeSet<_>>()
        .into_iter().map(f32::from_bits).collect::<Vec<_>>();
    let mut ascii = p.map_masks.iter().map(|((ch,w,h,_),b)|
        json!({"character":ch.to_string(),"native_mask_width":w,"native_mask_height":h,
            "mask_payload_bytes":b.coverage.len()})).collect::<Vec<_>>();
    ascii.sort_by_key(|v|(v["character"].as_str().unwrap().to_owned(),v["native_mask_width"].as_i64(),v["native_mask_height"].as_i64()));
    let mut compound = p.identity_masks.iter().map(|((s,w,h,source_scale,_,_,_),(_,_,mw,mh,b,_))|
        json!({"glyph":s,"native_cell_width":w,"native_cell_height":h,
            "outline_source_scale":f32::from_bits(*source_scale),"mask_width":mw,
            "mask_height":mh,"mask_payload_bytes":b.len()})).collect::<Vec<_>>();
    compound.sort_by_key(|v|(v["glyph"].as_str().unwrap().to_owned(),v["native_cell_width"].as_i64(),v["native_cell_height"].as_i64()));
    json!({"uniform_scale":p.scale,"origin_native_pixels":[p.origin.0,p.origin.1],
        "logical_canvas":[p.logical_width(),p.logical_height()],
        "text_glyph_raster_sizes_px":sizes,"styled_glyph_raster_sizes_px":styled_sizes,
        "text_mask_count":text.len(),"text_mask_examples":text.into_iter().take(12).collect::<Vec<_>>(),
        "ascii_mask_count":ascii.len(),"ascii_mask_examples":ascii.into_iter().take(12).collect::<Vec<_>>(),
        "compound_mask_count":compound.len(),"compound_mask_examples":compound.into_iter().take(12).collect::<Vec<_>>(),
        "prose_mask_count":p.prose_glyphs.len(),"prose_glyph_raster_sizes_px":prose_sizes,
        "interpretation":"Both versions already paint native RGBA pixels. Text cache keys are actual font raster pixel sizes. ASCII is an authored atlas sampled into native-sized masks, not newly invented vector outlines. These cache lengths do not measure total renderer memory."})
}

fn geometry(p: &Painter, v: &View, pack: &mut Option<pack::PackView>) -> Value {
    let frame = p.frame();
    let (cw,ch,ox,oy) = p.map_layout(v.width,v.height);
    let mut probes = Vec::new();
    // Test only actually observed map cells. Fractional-scale rounding can shift
    // a point, so use the midpoint of its actual native pixel rectangle.
    for c in v.cells.iter().take(64) {
        let lx = ox+c.x as i32*cw; let ly = oy+c.y as i32*ch;
        let left = p.native_x(lx as f64); let top = p.native_y(ly as f64);
        let right = p.native_x((lx+cw) as f64); let bottom = p.native_y((ly+ch) as f64);
        if right<=left || bottom<=top || left<0 || top<0 || right>p.width as i32 || bottom>p.height as i32 {continue;}
        let native = ((left+right)/2,(top+bottom)/2);
        let logical = p.logical_point(native.0,native.1,(p.width as u32,p.height as u32));
        let hit = crate::pointer_tile(p,v,logical.0,logical.1);
        assert_eq!(hit,Some((c.x,c.y)),"native mouse transform and production map hit geometry");
        probes.push(json!({"cell":[c.x,c.y],"native_point":native,"logical_point":logical,"hit":hit}));
    }
    let pack_geometry = pack.as_mut().map(|pack| {
        let b = pack.bounds();
        assert!(b.0>=frame.map.0 && b.1>=frame.map.1);
        assert!(b.0+b.2<=frame.map.0+frame.map.2 && b.1+b.3<=frame.map.1+frame.map.3);
        // Resolve the same public layout used by Pack, then execute only its
        // close result. This is detached UI selection, never a game command.
        let layout = modal_ui::PackLayout::resolve(&p.profile,frame.map);
        assert_eq!(layout.bounds.tuple(),b);
        let close = layout.close;
        let native = (p.native_x((close.0+close.2/2) as f64),p.native_y((close.1+close.3/2) as f64));
        let logical = p.logical_point(native.0,native.1,(p.width as u32,p.height as u32));
        assert_eq!(pack.click(logical.0,logical.1),pack::PackAction::Close);
        json!({"bounds_logical":b,"close_logical_rect":close.tuple(),"close_native_point":native,
            "close_pointer_result":"Close","contained_in_map_frame":true})
    });
    json!({"map_frame_logical":frame.map.tuple(),"sidebar_logical_x":frame.sidebar,
        "map_cell_logical":[cw,ch],"map_origin_logical":[ox,oy],"map_size_cells":[v.width,v.height],
        "map_hit_test_count":probes.len(),"map_hit_test_examples":probes.into_iter().take(4).collect::<Vec<_>>(),
        "pack":pack_geometry})
}

#[test]
#[ignore="lead-controlled optimized native resolution benchmark; explicit source fixture/output/variant required"]
fn real_tick48_native_resolution_matrix() {
    assert!(!cfg!(debug_assertions),"Build with --release; debug timings are not accepted");
    let input = PathBuf::from(std::env::var_os("CORPSE_RESOLUTION_INPUT").expect("actual tick48 roundtrip fixture"));
    let output = PathBuf::from(std::env::var_os("CORPSE_RESOLUTION_OUTPUT").expect("new report JSON"));
    let directory = PathBuf::from(std::env::var_os("CORPSE_RESOLUTION_FRAMES").expect("new native PNG directory"));
    let variant = std::env::var("CORPSE_RESOLUTION_VARIANT").expect("source variant label");
    assert!(!variant.is_empty() && variant.chars().all(|c|c.is_ascii_alphanumeric()||c=='-'||c=='_'));
    assert!(input.is_absolute() && output.is_absolute() && directory.is_absolute());
    assert!(!output.exists(),"never overwrite prior evidence");
    assert!(!directory.exists(),"use a new frame directory for this run");
    let n: usize = std::env::var("CORPSE_RESOLUTION_SAMPLES").unwrap_or_else(|_|"20".into()).parse().unwrap();
    assert!((20..=100).contains(&n));
    fs::create_dir_all(&directory).unwrap();
    if let Some(parent) = output.parent() { fs::create_dir_all(parent).unwrap(); }
    #[cfg(feature="runtime-timing")]
    corpse_core::timing::set_enabled(false);
    let raw = fs::read_to_string(&input).unwrap();
    let (engine,_) = crate::session_store::decode(&raw).unwrap();
    assert_eq!(engine.run.tick,48.,"use the real normalized live-probe tick48 fixture");
    let canonical = engine.save().unwrap();
    let mut view = observe::observe(&engine.run);
    // Genuine live Game does this after observation. Remove only the helper's
    // fixture footer, never world cells, visibility, actors, items, or state.
    view.provenance.clear();
    view.measure_conditions(&mut statuses_ui::ConditionMeters::default());
    let mut rows = Vec::new();
    for (width,height) in SIZES {
        for with_pack in [false,true] {
            let scene = if with_pack {"pack"} else {"game"};
            let mut painter = Painter::new();
            resize(&mut painter,width,height);
            painter.raster_stats.enabled = false;
            let mut pack = with_pack.then(||pack::PackView::observe(&engine.run));
            // Configure once before the non-dispatching first-item UI selection.
            paint(&mut painter,&view,&mut pack);
            if let Some(pack) = pack.as_mut() {
                assert!(matches!(pack.key(sdl2::keyboard::Keycode::Home,sdl2::keyboard::Mod::NOMOD),pack::PackAction::None));
            }
            for _ in 0..3 {paint(&mut painter,&view,&mut pack);}
            let expected_crc = crc32fast::hash(&painter.pixels);
            let mut warm = Vec::with_capacity(n);
            for _ in 0..n {
                let start = Instant::now(); paint(&mut painter,&view,&mut pack); warm.push(ns(start));
                assert_eq!(crc32fast::hash(&painter.pixels),expected_crc,"warm full frame is deterministic");
            }
            let stem = format!("{variant}-{scene}-{width}x{height}-tick48-clock1000");
            let png_path = directory.join(format!("{stem}.png")); png(&png_path,&painter);
            let rgba_path = if (width,height)==(1280,800) {
                let p = directory.join(format!("{stem}.rgba")); write_new(&p,&painter.pixels); Some(p)
            } else {None};
            let native_raster = raster_proof(&painter);
            // Actual resize from a warmed nearby dimension. Previous-mask
            // disposal, zeroed framebuffer allocation, and first paint are kept
            // separate and also enclosed by one combined wall-clock root.
            let (mut resizes,mut firsts,mut totals) = (Vec::with_capacity(n),Vec::with_capacity(n),Vec::with_capacity(n));
            for _ in 0..n {
                resize(&mut painter,width+1,height);
                paint(&mut painter,&view,&mut pack);
                let total = Instant::now();
                let start = Instant::now(); resize(&mut painter,width,height); resizes.push(ns(start));
                let start = Instant::now(); paint(&mut painter,&view,&mut pack); firsts.push(ns(start));
                totals.push(ns(total));
                assert_eq!(crc32fast::hash(&painter.pixels),expected_crc,"resize first frame matches warmed frame");
            }
            let geometry = geometry(&painter,&view,&mut pack);
            rows.push(json!({"variant":variant,"scene":scene,"width":width,"height":height,
                "framebuffer_pixels":width*height,"framebuffer_byte_length":painter.pixels.len(),
                "framebuffer_vec_capacity_bytes":painter.pixels.capacity(),"framebuffer_crc32":format!("{expected_crc:08x}"),
                "other_renderer_total_heap_bytes":Value::Null,"process_rss_bytes":Value::Null,
                "warm_full_paint":summary(warm),"resize_allocation_and_invalidation":summary(resizes),
                "first_paint_after_resize":summary(firsts),"resize_and_first_paint":summary(totals),
                "png":png_path,"raw_rgba_for_exact_parity":rgba_path,"native_raster":native_raster,"geometry":geometry}));
        }
    }
    assert_eq!(canonical,engine.save().unwrap(),"no gameplay, RNG, visibility, or storage-state mutation");
    let report = json!({"schema":"corpse-native-resolution-v1","variant":variant,"input":input,
        "fixture_crc32":format!("{:08x}",crc32fast::hash(raw.as_bytes())),"tick":engine.run.tick,"seed":engine.run.seed,
        "samples_per_row":n,"warmup_full_paints":3,"fixed_presentation_clock_ms":CLOCK_MS,
        "profile":"optimized release; caller records exact compiler/source/build command separately",
        "collector_enabled":false,"raster_counters_enabled":false,"rows":rows,
        "workload":{"observed_cells":view.cells.len(),"current_cells":view.cells.iter().filter(|c|c.current).count(),
            "direct_cells":view.cells.iter().filter(|c|c.direct).count(),"observed_creatures":view.creatures.len(),
            "animated_observed_cells":view.cells.iter().filter(|c|!c.animations.is_empty()).count(),
            "inventory_records":engine.run.inventory.len(),"map_cells":[view.width,view.height]},
        "scope":"Same real tick48 fixture and observation, default appearance, fixed clock1000, complete software game paint plus selected-first-item Pack when named. Empty notice and cleared detached observation-helper provenance match genuine live Game. No fixture edits, reveal override, simulation commands, effects added, SDL upload/present, GPU, input-event throughput, save timing, or Windows-runtime claim. Baseline was already native-pixel/native-glyph rendering; candidate changes fit/layout, not the existence of native rendering.",
        "timing_scope":"Warm includes game_at and, for Pack, real configure and paint; this is not an idle-event-frequency measurement. Three final-scene warmups follow setup. Resize starts with the same scene warmed at width+1,height; includes allocation/cache invalidation/disposal. First paint includes real game layout transition, cold native glyph masks and Pack reconfiguration. Combined root encloses both timing probes and small raw-sample pushes. Fixture parsing, world observation (including the live main resize View rebuild), SDL texture creation, renderer/font/asset construction, hashing, PNG encoding, geometry checks and JSON writing are outside timed intervals.",
        "memory_scope":"Framebuffer byte length/capacity are known. Native-mask payloads are examples, not deep cache/Arc/allocator memory. Other heap and process RSS were not instrumented and remain unknown.",
        "quantiles":"Median averages middle pair; p95/p99 nearest rank; n20 p99 equals maximum. Raw nanoseconds retained in execution order. Separate source builds are not alternating same-process arms.",
        "game_state_unchanged":true});
    write_new(&output,&serde_json::to_vec_pretty(&report).unwrap());
    println!("NATIVE_RESOLUTION_REPORT {}",output.display());
}

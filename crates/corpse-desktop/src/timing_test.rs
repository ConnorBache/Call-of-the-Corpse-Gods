//! Opt-in detached desktop benchmark. Never opens a window or touches player profiles.
//! Compile with --features runtime-timing; generated scene setup is outside measurements.
use crate::{ability_ui, appearance_profile, inspection_ui, non_euclidean_ui, observe, render::Painter, session_store, travel_ui};
use corpse_core::{temporal::Engine, timing};
use serde_json::{json, Value};
use std::{hint::black_box, path::{Path, PathBuf}, time::{Instant, SystemTime, UNIX_EPOCH}};

/// O(args) comparisons plus copied argument bytes; no game reads.
fn option<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().position(|s| s == key).and_then(|i| args.get(i + 1)).map(String::as_str)
}
/// O(args + digit count); bounded positive sample controls avoid empty percentile sets.
fn count(args: &[String], key: &str, default: usize, minimum: usize) -> Result<usize, String> {
    if args.iter().any(|s| s == key) && option(args,key).is_none() { return Err(format!("{key} requires a number")); }
    let n = option(args, key).map(str::parse::<usize>).transpose().map_err(|_| format!("Invalid {key}"))?.unwrap_or(default);
    if n < minimum || n > 10000 { return Err(format!("{key} must be {minimum}..10000")); }
    Ok(n)
}
/// O(n log n) time/O(n) extra storage for raw sample summary. Not timed as game work.
fn summary(raw: Vec<u64>) -> Value {
    let mut sorted = raw.clone(); sorted.sort_unstable();
    let n = sorted.len();
    let median = if n % 2 == 0 { (sorted[n/2-1] as f64 + sorted[n/2] as f64)/2. } else { sorted[n/2] as f64 };
    json!({"samples": n, "median_ns": median, "p95_ns": sorted[((n * 95).div_ceil(100) - 1).min(n-1)], "min_ns":sorted[0], "max_ns":sorted[n-1], "raw_ns":raw})
}
/// O((warmup+n)*operation + n log n) time; O(n+span labels) bookkeeping.
/// Optional environment selection: O(environment bytes + name bytes) time and
/// O(filter bytes) temporary storage, outside timing; skipped operations do no warmup.
/// Timer surrounds operation only. Inclusive span totals overlap; exclusive do not.
fn measure<T>(name: &str, n: usize, warmup: usize, enabled: bool, mut op: impl FnMut() -> Result<T,String>) -> Result<Value,String> {
    // Diagnostic selection only; checked before warmup and outside all timed regions.
    if let Some(only) = std::env::var_os("CORPSE_TIMING_ONLY") {
        if only != std::ffi::OsStr::new(name) {
            return Ok(json!({"operation":name,"instrumentation_enabled":enabled,"skipped":true}));
        }
    }
    timing::set_enabled(false);
    for _ in 0..warmup { black_box(op()?); }
    timing::reset(); timing::set_enabled(enabled);
    let mut times = Vec::with_capacity(n);
    for _ in 0..n {
        let start = Instant::now();
        black_box(op()?); // Include result disposal, not just construction.
        let elapsed = start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        times.push(elapsed);
    }
    timing::set_enabled(false);
    let spans = timing::take_report();
    Ok(json!({"operation":name,"instrumentation_enabled":enabled,"wall_time":summary(times),"spans":spans}))
}
/// O(1) attempts expected; creates a new benchmark-only directory, never an existing profile.
fn fresh_directory() -> Result<PathBuf,String> {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let path=std::env::temp_dir().join(format!("corpse-timing-{}-{epoch}",std::process::id()));
    std::fs::create_dir(&path).map_err(|e|e.to_string())?; Ok(path)
}
/// O(serialized state bytes) validation; no mutation or gameplay RNG reads beyond serialization.
fn verify_unchanged(before: &str, engine: &Engine) -> Result<(),String> {
    if before != engine.save().map_err(|e|e.to_string())? { return Err("Desktop timing changed game state".into()); } Ok(())
}
/// Historical synthetic double-hover plus full-refresh workload. O(two inspections +
/// two routes + main observation + exit observation); each callee retains its own nested costs.
// Historical synthetic workload, retained only for comparing old raw reports.
// This deliberately performs two inspections/routes and a full view refresh; it is
// NOT the current ordinary MouseMotion path (one inspection/route on changed tile).
fn legacy_double_hover_pipeline(engine: &Engine, profile: &appearance_profile::Profile, x: usize, y: usize) -> usize {
    let _span=timing::span("desktop.legacy_synthetic_double_hover_and_full_refresh");
    black_box(inspection_ui::observe_tile(&engine.run,x,y));
    black_box(travel_ui::path_to(&engine.run,x,y,true));
    black_box(inspection_ui::observe_tile(&engine.run,x,y));
    black_box(travel_ui::path_to(&engine.run,x,y,true));
    let view=observe::observe_with_profile(&engine.run,profile);
    black_box(non_euclidean_ui::possible_exit_marks(&engine.run,x,y));
    view.cells.len()
}
/// Benchmark orchestration: sum of setup/generation and measured operations. Only operation spans
/// are timed. Storage O(save bytes + framebuffer pixels + detached views + samples). This CLI
/// intentionally excludes SDL event polling, upload/present/vsync, transient overlays and input.
pub fn run(args: &[String]) -> Result<(),String> {
    let output=option(args,"--timing-test").ok_or("--timing-test requires output JSON path")?;
    let samples=count(args,"--timing-samples",20,1)?;
    let warmup=count(args,"--timing-warmup",3,0)?;
    let seed=option(args,"--seed").unwrap_or("1");
    timing::set_enabled(false);
    let (engine,source) = if let Some(path)=option(args,"--timing-input") {
        let raw=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
        (session_store::decode(&raw)?.0,format!("Input save: {path}; observed without mutation"))
    } else {
        (Engine::new(corpse_core::startup::create_run(seed,"Blessed",false)?).map_err(|e|e.to_string())?,format!("Genuine generated Blessed floor-1 entrance, seed {seed}; no commands advanced"))
    };
    let before=engine.save().map_err(|e|e.to_string())?;
    let profile=appearance_profile::Profile::default();
    let view=observe::observe_with_profile(&engine.run,&profile);
    let f=engine.run.floor().ok_or("No active floor")?;
    let x=(engine.run.player.x as usize+1).min(f.width().saturating_sub(1));
    let y=engine.run.player.y as usize;
    let temp=fresh_directory()?;
    let save_path=temp.join("benchmark-save.json");
    let mut autosave=session_store::Autosave::default();
    // Fixture provenance safely permits the same encode/write implementation for synthetic inputs.
    // Debug/playtest saves deliberately disable autosave; report that omission rather than fake writes.
    let autosave_supported=engine.run.playtest.is_missing();
    if autosave_supported { autosave.flush(&save_path,&engine,session_store::Kind::Fixture,true)?; }
    let mut rows=Vec::new();
    // Blocked on/off measurements share a fixed scene; reverse order on another process run
    // to expose cache/order bias. No different seeds or command outcomes are compared.
    let reversed=args.iter().any(|s|s == "--timing-reverse");
    for enabled in if reversed { [true,false] } else { [false,true] } {
        rows.push(measure("observe_with_profile",samples,warmup,enabled,||Ok(observe::observe_with_profile(&engine.run,&profile)))?);
        rows.push(measure("inspection_only",samples,warmup,enabled,||Ok(inspection_ui::observe_tile(&engine.run,x,y)))?);
        rows.push(measure("route_only",samples,warmup,enabled,||Ok(travel_ui::path_to(&engine.run,x,y,true)))?);
        rows.push(measure("possible_exits_negative_or_positive_check",samples,warmup,enabled,||Ok(non_euclidean_ui::possible_exit_marks(&engine.run,x,y)))?);
        rows.push(measure("legacy_synthetic_double_hover_plus_full_refresh",samples,warmup,enabled,||Ok(legacy_double_hover_pipeline(&engine,&profile,x,y)))?);
        if let Some(id)=engine.run.player.abilities.first() {
            rows.push(measure("ability_detail_projection",samples,warmup,enabled,||Ok(ability_ui::observe(&engine.run,id)))?);
        }
        rows.push(measure("save_encode_only",samples,warmup,enabled,||session_store::encode(&engine,session_store::Kind::Fixture))?);
        if autosave_supported {
            rows.push(measure("autosave_force_existing_verified",samples,warmup,enabled,||autosave.flush(&save_path,&engine,session_store::Kind::Fixture,true))?);
            rows.push(measure("autosave_unchanged_guard",samples,warmup,enabled,||autosave.flush(&save_path,&engine,session_store::Kind::Fixture,false))?);
        }
        for (width,height) in [(1280,800),(1920,1080)] {
            let mut painter=Painter::new(); painter.resize(width,height); painter.profile=profile.clone();
            rows.push(measure(&format!("software_render_{width}x{height}"),samples,warmup,enabled,|| {
                painter.game_at(&view,"",false,1000.);
                black_box(painter.pixels.as_slice());
                Ok(())
            })?);
        }
    }
    verify_unchanged(&before,&engine)?;
    if autosave_supported {
        let raw=std::fs::read_to_string(&save_path).map_err(|e|e.to_string())?;
        let (roundtrip,_)=session_store::decode(&raw)?;
        verify_unchanged(&before,&roundtrip)?;
    }
    let report=json!({"schema":"corpse-desktop-timing-v1","build":"release recommended; run records no claim of an idle host","scope":source,
        "seed":engine.run.seed,"depth":engine.run.depth,"tick":engine.run.tick,"actors":engine.run.actors_at(engine.run.depth).len(),
        "floor_cells":f.tiles.len(),"stored_floors":engine.run.floors.len(),"save_bytes":before.len(),"hover_cell":[x,y],"warmup":warmup,"instrumentation_order":if reversed { "enabled-then-disabled" } else { "disabled-then-enabled" },
        "unchanged_game_state":true,"save_roundtrip_checked":autosave_supported,"temporary_save_directory":temp,
        "coverage":"Read-only projections, actual software framebuffer paint, encode and actual atomic file write/fsync. No SDL upload/present/vsync, window event pump, timed command dispatch, disk cold-cache claim, GPU or physical Windows runtime claim.",
        "interpretation":"Rows overlap: hover pipeline calls several isolated rows. Do not sum isolated-row medians. Inclusive spans overlap; exclusive spans can be attributed within one operation. Instrumentation is blocked-order; use --timing-reverse in repeated invocations to expose order effects. Painter startup/resize/glyph warmup excluded. Result destruction is included in wall samples; function spans end at return, before caller disposal.","results":rows});
    std::fs::write(Path::new(output),serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    println!("Desktop timing report written to {output}; game state unchanged."); Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// O(n log n), four fixed samples; checks nearest-rank p95 and even median arithmetic.
    #[test] fn summary_uses_nearest_rank_and_preserves_raw() {
        let s=summary(vec![40,10,30,20]); assert_eq!(s["median_ns"],25.); assert_eq!(s["p95_ns"],40); assert_eq!(s["raw_ns"][0],40);
    }
    /// O(args), fixed inputs; zero warmup allowed but zero measured samples rejected.
    #[test] fn count_rejects_empty_measurement() {
        assert!(count(&["--n".into(),"0".into()],"--n",20,1).is_err());
        assert_eq!(count(&["--n".into(),"0".into()],"--n",3,0).unwrap(),0);
    }
}

// Test-only natural post-command observation profiling. No save/gameplay changes.
// Read/reset occurs outside observe wall timer. Extracting/parity occurs AFTER
// complete action wall timer; it may perturb subsequent caches equally in arms.
fn observe_profile_begin(repeat:usize)->bool {
    #[cfg(feature="runtime-timing")]{
        let enabled=match std::env::var("OBSERVE_PROFILE_MODE").as_deref(){Ok("on")=>true,Ok("alternate")=>repeat%2==0,_=>false};
        corpse_core::timing::reset();corpse_core::timing::set_enabled(enabled);enabled
    }
    #[cfg(not(feature="runtime-timing"))]{let _=repeat;false}
}
fn observe_profile_pause(){#[cfg(feature="runtime-timing")]corpse_core::timing::set_enabled(false);}
fn observe_profile_finish(enabled:bool,key:String,view:&crate::render::View)->serde_json::Value {
    #[cfg(feature="runtime-timing")]{
        let spans=corpse_core::timing::take_report();let counts=corpse_core::timing::take_counts();
        if std::env::var_os("OBSERVE_PROFILE_MODE").is_some(){
            thread_local! {static VIEWS:std::cell::RefCell<std::collections::HashMap<String,String>>=Default::default();}
            // Every View field, cell/animation, detached terrain/footprint, sidebar,
            // HUD/status/ability/log descriptor participates; no timing interval.
            // These generated fixtures contain finite numeric observations. Debug
            // parity is not a general IEEE NaN-payload serialization guarantee.
            let exact=format!("{view:?}");VIEWS.with(|m|{let mut m=m.borrow_mut();if let Some(old)=m.get(&key){assert_eq!(old,&exact,"profiling changed full View descriptor: {key}");}else{m.insert(key,exact);}});
        }
        let t=view.terrain_snapshot.as_ref().unwrap();
        serde_json::json!({"cache":"natural post-command, unchanged existing action harness setup","collection_enabled":enabled,"spans":spans,"counts":counts,"floor_cells":view.width*view.height,"known_cells":t.cells.iter().filter(|c|c.tile.is_some()).count(),"sources":t.light_sources.len(),"footprint_entries":t.light_sources.iter().map(|s|s.cells.len()).sum::<usize>(),"render_cells":view.cells.len(),"creatures":view.creatures.len()})
    }
    #[cfg(not(feature="runtime-timing"))]{let _=(enabled,key,view);serde_json::Value::Null}
}

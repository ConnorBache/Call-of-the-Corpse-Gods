//! Staged current-UI revision40–43 action measurement; no production behavior change.
//! Include as a private main.rs test module only after the lead freezes source.
//! All cases are ignored by default. Correctness mode is n=1 with every-row reload;
//! timed mode defaults to n=20, first/last-repeat reload, with raw sample stages.
//! The timed root ends at software pixels: no SDL polling/upload/present or sleep.
use super::*;
use corpse_core::{Item, PartialStats, Run};
use std::{path::{Path, PathBuf}, sync::Arc};

struct Options { timed: bool, repeats: usize, revisions: Vec<u32>, floors: Vec<usize>, root: PathBuf }
impl Options {
    /// O(environment text); explicit caller-owned overlay directory, never /tmp.
    fn read() -> Self {
        let timed=std::env::var("WEAVE_ACTION_TIMING").as_deref()==Ok("1");
        let repeats=std::env::var("WEAVE_ACTION_REPEATS").unwrap_or_else(|_|if timed{"20"}else{"1"}.into()).parse().unwrap();
        assert!((1..=50).contains(&repeats));
        let revisions=std::env::var("WEAVE_ACTION_REVISIONS").unwrap_or_else(|_|"40,41".into()).split(',').map(|v|v.parse().unwrap()).collect::<Vec<u32>>();
        assert!(!revisions.is_empty()&&revisions.iter().all(|r|matches!(r,40|41|42|43)));
        let floors=std::env::var("WEAVE_ACTION_FLOORS").unwrap_or_else(|_|"10,40".into()).split(',').map(|v|v.parse().unwrap()).collect::<Vec<usize>>();
        assert!(!floors.is_empty()&&floors.iter().all(|n|matches!(n,10|40)));
        let supplied=PathBuf::from(std::env::var_os("WEAVE_ACTION_OVERLAY_DIR").expect("explicit existing workspace overlay directory required"));assert!(supplied.is_absolute());
        let root=std::fs::canonicalize(supplied).unwrap();
        assert!(root.is_absolute()&&root.is_dir());
        assert!(!root.starts_with("/tmp")&&!root.starts_with("/dev/shm"),"RAM temp paths are not this harness's storage fixture; independently record filesystem type");
        Self{timed,repeats,revisions,floors,root}
    }
    /// O(1), proof cadence does not change gameplay or measured commands.
    fn prove(&self,repeat:usize)->bool {!self.timed||repeat==1||repeat==self.repeats}
}
struct Sandbox(PathBuf);
impl Sandbox {
    /// O(path bytes + one filesystem mkdir), refuses an existing experiment directory.
    fn new(root:&Path,label:&str)->Self {
        let p=root.join(format!("weave-actions-{label}-{}",std::process::id()));
        assert!(!p.exists(),"refusing existing audit output {p:?}");std::fs::create_dir(&p).unwrap();Self(p)
    }
}
impl Drop for Sandbox {
    /// O(test-owned file entries/bytes), cleanup outside all sample intervals.
    fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}
}
/// O(1) Unix metadata, outside timed roots; inode replacement identifies checkpoint.
#[cfg(unix)] fn stamp(path:&Path)->(u64,u64,u64){use std::os::unix::fs::MetadataExt;let m=std::fs::metadata(path).unwrap();(m.dev(),m.ino(),m.len())}
/// O(1), unsupported hosts fail rather than guessing persistence classification.
#[cfg(not(unix))] fn stamp(_: &Path)->(u64,u64,u64){panic!("Unix inode evidence required")}
/// O(1) timer branch; correctness mode does not collect misleading durations.
fn start(timed:bool)->Option<Instant>{timed.then(Instant::now)}
/// O(1) elapsed clock read; formatting is never inside a measured stage.
fn elapsed(t:Option<Instant>)->Option<u128>{t.map(|t|t.elapsed().as_nanos())}
/// O(report bytes) JSON formatting/write after all intervals; one output per test.
fn report(label:&str,options:&Options,rows:Vec<serde_json::Value>){
    let value=serde_json::json!({"schema":1,"suite":label,"timed":options.timed,"repeats":options.repeats,"revision_order":options.revisions,"floor_order":options.floors,"overlay_directory":options.root,"proof_cadence":if options.timed{"every row in first and last fresh repeat"}else{"every row"},"clock":"fixed 300ms presentation increments, no SDL","controls":"Eat and weave have no NPCs; ordinary trace has one paralyzed NPC. Eat zero EXP. Long trails are imported geometry-valid states, not naturally walked traces. Config floor list applies to Eat; revision list applies to Eat/ordinary, while weave suites are41 only.","profile":"caller must attach actual build command/profile; this file makes no release/runtime claim","paint_scope":"Full and tile same binary. Identical preceding frame/selected pack setup. Timed paint includes old View destruction, planner comparison/index/footprint/row spans, software base/effects/open-pack pixels, upload-rectangle construction and checked borrowed upload source; no SDL calls. Diagnostic counters off unless explicitly enabled. Pack refresh before paint in both arms. No old RetainedGame compositor.","scope":"Same current-UI binary, revision40–43 controlled cases. Timed action helper + actual tracked autosave + current world observation + software game/effect pixels and pack only if still open. Pack helper's own refresh remains inside action. Initial checkpoint, fixture generation, keyboard selection/reopening, metadata/effect assertions and selected full readbacks are outside root. Cached filesystem, not hardware power-loss or Windows runtime evidence.","rows":rows});
    let out=options.root.join(format!("{label}.json"));assert!(!out.exists(),"preserve prior report {out:?}");std::fs::write(&out,serde_json::to_vec_pretty(&value).unwrap()).unwrap();println!("WEAVE_ACTION_REPORT {}",out.display());
}
/// O(world/save bytes + replay/history), outside timers. Metadata-v2 replay
/// restores exact order before commands; no normalization exemption is allowed.
fn reload_proof(engine:&temporal::Engine,path:&Path)->bool {
    let (loaded,kind)=session_store::decode_bytes(&std::fs::read(path).unwrap()).unwrap();assert_eq!(kind,session_store::Kind::Fixture);
    let live=engine.save().unwrap();let saved=loaded.save().unwrap();
    assert_eq!(live,saved,"exact metadata-v2 live/reload state, including inventoryDisplayOrder");
    true
}
/// O(1) fixed heart payload, zero EXP keeps level-up outside this selected case.
fn heart(id:f64,quantity:usize)->Item{Item{id,kind:"heart".into(),name:if quantity>1{"Jackal heart"}else{"Rat heart"}.into(),glyph:";".into(),bonus:PartialStats{offense:1.0.into(),..Default::default()}.into(),exp:0.0.into(),energy:2.0.into(),quantity:(quantity as f64).into(),..Default::default()}}
/// O(world bytes + actor groups), setup only. Same physical world in both arms;
/// every old NPC block key is preserved, no claim about populated combat worlds.
fn eat_fixture(floors:usize,revision:u32,stack:bool)->Run{
    let mut r:Run=if floors==40 {let text=std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/floors-40.json")).unwrap();corpse_core::temporal::Engine::load(&text).unwrap().run}else{serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()};
    assert_eq!(r.floors.len(),floors);r.simulation_revision=(revision as f64).into();
    for a in r.actors.values_mut(){*a=Arc::new(vec![]);}
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    r.player.hp=50.;r.player.max_hp=100.;r.player.base_max_hp=100.0.into();r.player.energy=20.;r.player.stats.max_energy=100.;
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.timeweave=Field::Missing;r.time_costs=Field::Missing;r.offers.clear();r.exp=0.;r.inventory_display_order=Field::Missing;
    let id=r.next_id.max(10000.);r.next_id=id+100.;r.inventory=if stack{vec![heart(id,4)]}else{(0..4).map(|n|heart(id+n as f64,1)).collect()};r
}
/// Benchmark-only owner mirrors the main loop's cached View lifetime. No View or
/// Cell clone is used to feed the planner; old-view destruction is timed equally.
struct Ui {
    view:pack::PackView,painter:Painter,effects:effects::Effects,open:bool,
    target:Option<pack::PackTarget>,notice:String,
    tile_enabled:bool,diagnostics:bool,paint_clock:f64,previous_paint_clock:f64,game:Option<render::View>,
    tile_plan:render::tile_damage::TilePlanner,tile_overlays:tile_overlay::Tracker,
    tile_panels:Option<tile_overlay::PanelKeys>,tile_notice:String,damage_pixels:usize,
    upload_bytes:usize,upload_calls:usize,upload_full:bool,requested_upload_bytes:usize,requested_upload_calls:usize,
}
impl Ui {
    /// O(pack grouping + assets + initial view/paint), identical setup in both arms.
    fn new(run:&Run)->Self{
        assert!(std::env::var_os("WEAVE_RETAINED_PAINT").is_none(),"old RetainedGame arm is archived; use WEAVE_TILE_PAINT=full|tile");
        let mode=std::env::var("WEAVE_TILE_PAINT").unwrap_or_else(|_|"full".into());assert!(matches!(mode.as_str(),"full"|"tile"));
        let mut ui=Self{view:pack::PackView::observe(run),painter:Painter::new(),effects:effects::Effects::default(),open:false,target:None,notice:String::new(),tile_enabled:mode=="tile",diagnostics:std::env::var("WEAVE_TILE_DIAGNOSTICS").as_deref()==Ok("1"),paint_clock:0.,previous_paint_clock:0.,game:None,tile_plan:render::tile_damage::TilePlanner::default(),tile_overlays:tile_overlay::Tracker::default(),tile_panels:None,tile_notice:String::new(),damage_pixels:0,upload_bytes:0,upload_calls:0,upload_full:false,requested_upload_bytes:0,requested_upload_calls:0};
        let game=observe::observe_with_profile(run,&ui.painter.profile);ui.paint_game(game,0.);ui
    }
    /// Includes borrowed comparison/index/footprint setup, old View drop, mask,
    /// paint, upload-span construction/checked borrowed slices and scratch drops.
    /// Actual SDL update/present is NOT performed or claimed by this CPU harness.
    fn paint_game(&mut self,game:render::View,now:f64){
        self.previous_paint_clock=self.paint_clock;self.reset_paint_stats();
        if self.tile_enabled{self.tile_plan.replace_view(self.game.as_ref(),&game,&self.painter);}
        drop(self.game.replace(game));
        self.paint_cached_inner(now);self.paint_clock=now;
    }
    fn reset_paint_stats(&mut self){
        self.tile_plan.eligibility=render::tile_damage::Eligibility{enabled:self.diagnostics,..Default::default()};
        self.painter.raster_stats=render::damage::RasterStats{enabled:self.diagnostics,..Default::default()};
    }
    /// Idle/hover path keeps the existing cached View; it never re-observes Run.
    fn paint_cached(&mut self,now:f64){self.previous_paint_clock=self.paint_clock;self.reset_paint_stats();self.paint_cached_inner(now);self.paint_clock=now;}
    fn paint_cached_inner(&mut self,now:f64){
        let game=self.game.as_ref().unwrap();
        if !self.tile_enabled{
            self.painter.tile_clip_enabled=false;self.painter.damage_clip=None;
            self.painter.game_at(game,&self.notice,false,now);self.painter.effects(game,&self.effects.cues,now,false);
            if self.open{self.view.paint(&mut self.painter);}
            self.damage_pixels=self.painter.width*self.painter.height;self.upload_calls=1;self.upload_bytes=self.damage_pixels*4;self.upload_full=true;self.requested_upload_calls=1;self.requested_upload_bytes=self.upload_bytes;
            std::hint::black_box(&self.painter.pixels);return;
        }
        let panels=tile_overlay::PanelKeys::read(&self.painter);
        panels.mark_changes(self.tile_panels,&self.painter,game,&mut self.tile_plan);
        let state=tile_overlay::Frame{effects:tile_overlay::effects(&self.painter,game,&self.effects.cues,now,false),pack:self.open.then(||self.view.paint_state(&self.painter)),..Default::default()};
        for r in self.tile_overlays.changes(&self.painter,&state){self.tile_plan.mark_native_rect(r);}
        // Same notice footprint/invalidation as the actual main loop.
        if self.tile_notice!=self.notice{let y=render::toolbar_y(&self.painter.profile).min(715-(self.painter.profile.number("ui.layout.buttonHeight").round()as i32-42)).min(786)-24;if let Some(r)=self.painter.logical_damage_rect(0,y,render::WIDTH as i32,render::HEIGHT as i32-y){self.tile_plan.mark_native_rect(r);}self.tile_notice=self.notice.clone();}
        self.tile_plan.mark_animation(game,&self.painter,now,false);
        tile_overlay::finish_dependencies(&mut self.painter,&mut self.tile_plan,&state);
        self.tile_plan.paint_base(&mut self.painter,game,&self.notice,now);
        if !self.painter.tile_mask.is_empty(){
            self.painter.effects(game,&self.effects.cues,now,false);
            if let Some(pack)=&state.pack{self.view.paint_prepared(&mut self.painter,pack);}
        }
        self.damage_pixels=self.painter.tile_mask.area();
        let uploads=tile_overlay::upload_plan(&self.painter,false);self.upload_calls=uploads.calls;self.upload_bytes=uploads.bytes;self.upload_full=uploads.full;
        self.requested_upload_calls=uploads.requested_calls;self.requested_upload_bytes=uploads.requested_bytes;
        if uploads.full{std::hint::black_box((&self.painter.pixels,self.painter.width*4));}
        else{for &r in &uploads.rectangles{let(bytes,pitch)=tile_overlay::upload_source(&self.painter,r).unwrap();std::hint::black_box((bytes,pitch));}}
        drop(uploads);self.painter.tile_clip_enabled=false;self.tile_plan.clear_frame();
        self.tile_panels=Some(panels);self.tile_overlays.commit(state);
    }
    fn paint_metadata(&self,mut row:serde_json::Value,proof:bool)->serde_json::Value{
        row["paint_arm"]=if self.tile_enabled{"tile"}else{"full"}.into();
        row["fixed_clock_ms"]=self.paint_clock.into();row["preceding_presented_clock_ms"]=self.previous_paint_clock.into();row["paint_stage_composes_pack_if_open"]=true.into();
        row["preceding_frame_prepared"]=true.into();row["old_view_drop_included"]=row.get("changed_view_replacement_and_old_drop_timed").and_then(serde_json::Value::as_bool).unwrap_or(true).into();
        row["metadata_allocation_time_included"]=true.into();row["metadata_allocation_bytes_measured"]=false.into();
        row["tile_diagnostics_enabled"]=self.diagnostics.into();row["paint_damage_pixels"]=self.damage_pixels.into();
        row["planned_upload_bytes"]=self.upload_bytes.into();row["planned_upload_calls"]=self.upload_calls.into();
        row["chosen_upload_full_texture"]=self.upload_full.into();row["raw_requested_upload_calls"]=self.requested_upload_calls.into();row["raw_requested_upload_bytes"]=self.requested_upload_bytes.into();
        row["actual_sdl_upload_measured"]=false.into();row["full_pixel_oracle"]=proof.into();
        if self.diagnostics{let e=&self.tile_plan.eligibility;let r=&self.painter.raster_stats;
            row["eligibility_overlapping_not_partition"]=serde_json::json!({"both_no_paint_layers":e.both_no_paint_layers,"same_paint_locally_nonanimated_not_final_skip":e.same_paint_nonanimated,"memory_transition":e.memory_transition,"changed_fg_bg":e.changed_colors,"changed_other_paint":e.changed_other_paint,"local_animation_candidates":e.animated_candidates,"global_animation_candidates":e.global_animation_candidates,"overlay_marks":e.overlay_marks,"spill_marks":e.spill_marks,"compared_layers":e.compared_layers,"footprint_entries":e.footprint_entries,"raw_span_entries":e.raw_span_entries,"raw_span_entries_meaning":"logical rectangle-row contributions; not allocated intervals","bitset_words_touched":e.bitset_words_touched,"bitset_words_scanned":e.bitset_words_scanned,"bitset_capacity_bytes":e.bitset_capacity_bytes,"merged_span_entries":e.merged_span_entries,"index_capacity_bytes":e.index_capacity_bytes,"capacity_growth_events":e.capacity_growth_events,"span_capacity_bytes":e.span_capacity_bytes});
            row["raster"]=serde_json::json!({"pixel_visits":r.pixel_visits,"pixel_writes":r.pixel_writes,"cells_considered":r.cells_considered,"cells_painted":r.cells_painted});
        }row
    }
    /// Independent full pixels only after action wall interval has stopped.
    fn prove_pixels(&self,now:f64){
        let game=self.game.as_ref().unwrap();let mut full=Painter::new();full.resize(self.painter.width,self.painter.height);full.profile=self.painter.profile.clone();
        full.chronicle_hover=self.painter.chronicle_hover;full.ability_hover=self.painter.ability_hover;full.toolbar_focus=self.painter.toolbar_focus;full.enemy_hover=self.painter.enemy_hover;full.enemy_list_offset=self.painter.enemy_list_offset;
        full.game_at(game,&self.notice,false,now);full.effects(game,&self.effects.cues,now,false);if self.open{self.view.paint(&mut full);}
        assert_eq!(self.painter.pixels.len(),full.pixels.len(),"actual action pixel buffer length");
        if let Some((i,(got,expected)))=self.painter.pixels.iter().zip(&full.pixels).enumerate().find(|(_, (a,b))|a!=b){panic!("actual action tile/full pixel oracle: byte {i}, pixel ({},{}), channel {}, got {got}, expected {expected}, clock {now}ms, tile={}, pack={}",(i/4)%self.painter.width,(i/4)/self.painter.width,i%4,self.tile_enabled,self.open); }
    }
    /// Existing grouping/configuration remains setup for opening the pack.
    fn reopen(&mut self,run:&Run){self.open=true;self.view.refresh(run);self.view.configure(&self.painter,modal_ui::Frame::from_profile(&self.painter.profile).map.tuple());}
}
/// O(G + text), real Home/Tab/Enter Eat selection, outside action timer.
fn select_eat(view:&mut pack::PackView,id:f64)->pack::PackAction{
    assert_eq!(view.key(Keycode::Home,Mod::NOMOD),pack::PackAction::None);assert_eq!(view.key(Keycode::Tab,Mod::NOMOD),pack::PackAction::None);
    let action=view.key(Keycode::Return,Mod::NOMOD);assert_eq!(action,pack::PackAction::Command(Command::Use{id}));action
}
/// O(I²+I*A+G log G+text + G) selected equipment UI path, setup only. This
/// fixture has distinct named groups; matching the returned Command certifies
/// the actual keyboard target rather than assuming a group index is an item ID.
fn select_equipment(ui:&mut Ui,run:&Run,id:f64,unequip:bool)->pack::PackAction{
    ui.reopen(run);let row=ui.view.display_order().iter().position(|n|*n==id).unwrap();
    ui.view.key(Keycode::Home,Mod::NOMOD);for _ in 0..row{ui.view.key(Keycode::Down,Mod::NOMOD);}
    // Bow also exposes Apply/fire; resolve the intended label rather than a
    // fragile fixed index, then inject the same Tab/Enter keys as the UI.
    let label=if unequip{"Unequip"}else{"Both hands"};
    let tabs=ui.view.keyboard_tabs_for_action_label(label).expect("authored action label must exist");
    for _ in 0..tabs{assert_eq!(ui.view.key(Keycode::Tab,Mod::NOMOD),pack::PackAction::None);}
    let action=ui.view.key(Keycode::Return,Mod::NOMOD);
    let expected=if unequip{Command::Unequip{id}}else{Command::Equip{id,slot:"hand1".into()}};
    assert_eq!(action,pack::PackAction::Command(expected));action
}
/// Transitive real command/save/view/render cost. Metadata/proofs/effect checks
/// are outside root; there is no whole-world diff or serialization for timing
/// observation. The actual checkpoint/command persistence may serialize state.
fn sample(options:&Options,engine:&mut temporal::Engine,auto:&mut session_store::Autosave,path:&Path,ui:&mut Ui,label:&str,repeat:usize,step:usize,pack_action:Option<pack::PackAction>,command:&Command,tick_delta:f64)->serde_json::Value{
    let before=stamp(path);let tick=engine.run.tick;let groups=engine.history.frames.len();let revision=engine.run.simulation_revision.value_or(39.);let is_pack=pack_action.is_some();
    let order_changed=is_pack&&engine.run.inventory_display_order!=Field::Value(ui.view.display_order());
    assert!(!engine.pending_batch().checkpoint_required);let now=step as f64*300.;if is_pack{ui.paint_cached(now-300.);}let wall=start(options.timed);let t=start(options.timed);
    let outcome=if let Some(action)=pack_action{apply_pack_action(action,&mut ui.view,engine,&mut ui.effects,now,&mut ui.open,&mut ui.target,&mut ui.notice);None}else{Some(dispatch(engine,&mut ui.effects,now,command).unwrap())};
    let action_ns=elapsed(t);let t=start(options.timed);auto.flush_tracked(path,engine,session_store::Kind::Fixture,false).unwrap();let flush_ns=elapsed(t);
    let view_profile_enabled=observe_profile_begin(repeat);
    let t=start(options.timed);let game=observe::observe_with_profile(&engine.run,&ui.painter.profile);let observe_ns=elapsed(t);
    observe_profile_pause();
    let refresh_ns=if ui.open{let t=start(options.timed);ui.view.refresh(&engine.run);ui.view.configure(&ui.painter,modal_ui::Frame::from_profile(&ui.painter.profile).map.tuple());elapsed(t)}else{None};
    let t=start(options.timed);ui.paint_game(game,now);let game_ns=elapsed(t);let pack_ns:Option<u128>=None;
    std::hint::black_box(&ui.painter.pixels);let wall_ns=elapsed(wall);
    assert!(ui.notice.is_empty(),"{label}: {}",ui.notice);assert!(ui.target.is_none());assert_eq!(engine.run.status,"playing");assert_eq!(engine.run.tick,tick+tick_delta,"{label}: actual tick");
    if is_pack{assert_eq!(ui.open,tick_delta==0.,"current UI closes on actual normal turn");}else{assert_eq!(outcome.as_ref().unwrap().took_turn,tick_delta!=0.,"{label}: actual outcome");}
    if revision>=41.{assert!(engine.history.frames.is_empty());assert!(engine.run.history.is_empty());assert!(engine.history.pending_haste.is_none());}
    let after=stamp(path);let checkpoint=(before.0,before.1)!=(after.0,after.1);assert!(!checkpoint,"{label}: known inventory metadata and command must append without a world checkpoint");if !checkpoint{assert!(after.2>before.2);}
    let proof=options.prove(repeat);if proof{ui.prove_pixels(now);}let raw_equal=proof.then(||reload_proof(engine,path));
    let stages=[action_ns,flush_ns,observe_ns,game_ns,refresh_ns,pack_ns];let measured_sum:Option<u128>=options.timed.then(||stages.into_iter().flatten().sum());if let(Some(w),Some(sum))=(wall_ns,measured_sum){assert!(w>=sum);}
    ui.paint_metadata(serde_json::json!({"sample_id":format!("r{revision:.0}-{label}-repeat{repeat}-step{step}"),"case":label,"repeat":repeat,"step":step,"revision":revision,"floors":engine.run.floors.len(),"tick_before":tick,"tick_after":engine.run.tick,"history_before":groups,"history_after":engine.history.frames.len(),"pack_action":is_pack,"pack_open_after":ui.open,"order_changed":order_changed,"checkpoint":checkpoint,"checkpoint_reason":if checkpoint{"unexpected full checkpoint in known-operation trace"}else if order_changed{"none; ordered metadata plus command append"}else{"none; recorded command append"},"bytes_written":if checkpoint{after.2}else{after.2-before.2},"file_bytes":after.2,"proof_performed":proof,"raw_live_reload_equal":raw_equal,"exact_ordinary_load_history_proof":proof.then_some(true),"allowed_raw_reload_normalization":"none; metadata-v2 exact replay","action_ns":action_ns,"flush_ns":flush_ns,"observe_ns":observe_ns,"view_profile":observe_profile_finish(view_profile_enabled,format!("{}-{}-{label}-{step}",engine.run.simulation_revision.value_or(39.),engine.run.floors.len()),ui.game.as_ref().unwrap()),"software_game_effects_paint_ns":game_ns,"pack_refresh_configure_ns":refresh_ns,"software_pack_paint_ns":pack_ns,"wall_ns":wall_ns,"stage_sum_ns":measured_sum,"unattributed_timer_ns":wall_ns.zip(measured_sum).map(|(w,s)|w-s)}),proof)
}
/// O(repeats*cases*(setup S + five turns + three real Eat/save/view steps +
/// selected full reload proofs)). H5 under40, H0 under41 after SAME five Waits.
#[test]
#[ignore="lead-controlled correctness or quiet timing; explicit overlay output required"]
fn timeweave_action_eat(){
    let options=Options::read();let dir=Sandbox::new(&options.root,"eat");let mut rows=vec![];
    for &revision in &options.revisions{for &floors in &options.floors{for stack in [false,true]{for repeat in 1..=options.repeats{
        let case=format!("eat-{}-{floors}floors",if stack{"stack"}else{"separate"});let mut engine=temporal::Engine::new(eat_fixture(floors,revision,stack)).unwrap();
        for _ in 0..5{assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);}assert_eq!(engine.history.frames.len(),if revision==40{5}else{0});
        let path=dir.0.join(format!("r{revision}-{case}-{repeat}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);
        for step in 1..=3{
            ui.reopen(&engine.run);let id=engine.run.inventory[0].id;let quantity=engine.run.inventory[0].quantity.value_or(1.);let offense=engine.run.player.stats.offense;
            let action=select_eat(&mut ui.view,id);let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,&case,repeat,step,Some(action),&Command::Use{id},1.);
            assert_eq!(engine.run.player.stats.offense,offense+1.,"heart bonus must actually apply");let remaining=engine.run.inventory.iter().find(|i|i.id==id);
            if quantity>1.{assert_eq!(remaining.unwrap().quantity.value_or(1.),quantity-1.);}else{assert!(remaining.is_none());}
            row["quantity_before"]=serde_json::json!(quantity);row["quantity_after"]=serde_json::json!(remaining.map(|i|i.quantity.value_or(1.)));row["offense_before"]=serde_json::json!(offense);row["offense_after"]=serde_json::json!(engine.run.player.stats.offense);row["actual_effect_asserted"]=true.into();row["prep_waits"]=5.into();
            assert_eq!(row["checkpoint"],serde_json::json!(false));rows.push(row);
        }
    }}}}report("timeweave-action-eat",&options,rows);
}
/// O(authored fixture bytes), one paralyzed reachable NPC for actual bump damage.
/// Pickup precedes movement so auto-pickup cannot invalidate the fixture premise.
fn ordinary_fixture(revision:u32)->Run{
    let cases:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/combat-starters-v48.json")).unwrap();let mut value=cases.as_array().unwrap().iter().find(|c|c["kind"]=="number").unwrap()["initial"].clone();
    value["simulationRevision"]=revision.into();value["player"]["abilities"]=serde_json::json!([]);value["player"]["abilityOrder"]=serde_json::json!([]);value["player"]["stats"]["accuracy"]=1000.into();value["actors"]["1"][0]["conditions"]["paralyzed"]=100.into();value["actors"]["1"][0]["stats"]["defense"]=0.into();value["actors"]["1"][0]["stats"]["dodge"]=0.into();
    let mut bow=value["inventory"].as_array_mut().unwrap().remove(1);bow["id"]=10000.into();value["nextId"]=10001.into();value["ground"]["1"]=serde_json::json!([{"x":12,"y":14,"item":bow}]);value.as_object_mut().unwrap().remove("inventoryDisplayOrder");serde_json::from_value(value).unwrap()
}
/// O(repeats*six actual actions + setup/proofs/view paint), no added generation.
#[test]
#[ignore="lead-controlled correctness or quiet timing; explicit overlay output required"]
fn timeweave_action_ordinary(){
    let options=Options::read();let dir=Sandbox::new(&options.root,"ordinary");let mut rows=vec![];
    for &revision in &options.revisions{for repeat in 1..=options.repeats{
        let mut engine=temporal::Engine::new(ordinary_fixture(revision)).unwrap();let path=dir.0.join(format!("r{revision}-{repeat}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);
        let commands=[("keyboard-pickup",Command::Pickup,0.),("move-left",Command::Move{dx:-1.,dy:0.},1.),("move-right",Command::Move{dx:1.,dy:0.},1.),("bump-combat",Command::Move{dx:1.,dy:0.},1.),("pack-equip",Command::Equip{id:10000.,slot:"hand1".into()},1.),("pack-unequip",Command::Unequip{id:10000.},1.)];
        for (index,(label,command,delta)) in commands.into_iter().enumerate(){
            let hp=engine.run.actors["1"][0].hp;ui.open=false;let action=match label{"pack-equip"=>Some(select_equipment(&mut ui,&engine.run,10000.,false)),"pack-unequip"=>Some(select_equipment(&mut ui,&engine.run,10000.,true)),_=>None};
            let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,label,repeat,index+1,action,&command,delta);
            match label{
                "keyboard-pickup"=>{assert!(engine.run.inventory.iter().any(|i|i.id==10000.));assert!(!engine.run.ground["1"].iter().any(|g|g.item.id==10000.));},
                "move-left"=>assert_eq!(engine.run.player.x,11.),"move-right"=>assert_eq!(engine.run.player.x,12.),
                "bump-combat"=>{assert!(engine.run.actors["1"][0].hp<hp);assert_eq!(engine.run.player.x,12.);},
                "pack-equip"=>assert!(engine.run.inventory.iter().find(|i|i.id==10000.).unwrap().equipped.as_ref().is_some_and(|s|!s.is_empty())),
                "pack-unequip"=>assert!(engine.run.inventory.iter().find(|i|i.id==10000.).unwrap().equipped.as_ref().is_none_or(|s|s.is_empty())),_=>unreachable!()
            }row["actual_effect_asserted"]=true.into();row["npc_hp_before"]=serde_json::json!(hp);row["npc_hp_after"]=serde_json::json!(engine.run.actors["1"][0].hp);rows.push(row);
        }
    }}report("timeweave-action-ordinary",&options,rows);
}
/// O(fixture cells+actor blocks), mirrors the already authored normal waking
/// movement oracle. No forced movement, plane transition or populated floor claim.
fn weave_room()->Run{
    let mut r=eat_fixture(10,41,false);r.player.x=10.;r.player.y=10.;r.player.abilities=vec![corpse_core::progression::WEAVE_ID.into()];r.player.hp=10000.;r.player.max_hp=10000.;r.player.base_max_hp=10000.0.into();
    for g in r.ground.values_mut(){*g=Arc::new(vec![]);}let f=r.floor_mut().unwrap();for tile in &mut f.tiles{*tile="floor".into();}f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;r
}
/// O(repeats*seven real movement/save/view steps), proves actual free travel and
/// source-cell prefix collision separately; no fixture-created live trail used.
#[test]
#[ignore="lead-controlled correctness or quiet timing; explicit overlay output required"]
fn timeweave_action_free_and_collision(){
    let options=Options::read();let dir=Sandbox::new(&options.root,"weave");let mut rows=vec![];
    for repeat in 1..=options.repeats{
        let mut engine=temporal::Engine::new(weave_room()).unwrap();let path=dir.0.join(format!("weave-{repeat}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);
        let moves=[("weave-forward",1.,0.,1.,1),("weave-free-back",-1.,0.,0.,1),("weave-forward-source-collision",1.,0.,1.,1),("weave-extend",1.,0.,1.,2),("weave-turn",0.,1.,1.,3),("weave-wrong-side-entry",-1.,-1.,1.,4),("weave-prefix-collision",0.,1.,1.,3)];
        for(index,(label,dx,dy,delta,len))in moves.into_iter().enumerate(){
            let rng=engine.run.rng;let old=(delta==0.).then(||engine.run.timeweave.clone());let x=engine.run.player.x;let y=engine.run.player.y;let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,label,repeat,index+1,None,&Command::Move{dx,dy},delta);
            assert_eq!((engine.run.player.x,engine.run.player.y),(x+dx,y+dy));assert_eq!(engine.run.timeweave.as_ref().unwrap().len(),len);
            if delta==0.{assert_eq!(engine.run.rng,rng);assert_eq!(engine.run.timeweave,old.as_ref().unwrap().clone());}
            // Only free travel retains this observer Arc; mutating samples never hold it.
            drop(old);row["actual_effect_asserted"]=true.into();row["trail_length_after"]=len.into();rows.push(row);
        }
    }report("timeweave-action-free-and-collision",&options,rows);
}
/// O(F cell capacity + L) setup, expected hash insertion. This is an IMPORTED
/// geometry-valid state, not a naturally simulated walk: independent historical
/// arrows occupy unique source cells over existing floors. The current source
/// and destination are reserved so the intended command cannot accidentally
/// match an unrelated reverse arrow. Setup/admission is outside timing.
fn long_weave_fixture(length:usize,mode:&str)->Run{
    use corpse_core::timeweave::{Cell,Plane,Segment,Trail};
    let mut run=weave_room();let width=run.floor().unwrap().width();
    let source=Cell{floor:1,plane:Plane::Waking,tile:(10*width+10)as u32};
    let destination=Cell{tile:source.tile+1,..source};let side=Cell{tile:source.tile+width as u32,..source};
    let mut segments=Vec::with_capacity(length);let ordinary_count=length-usize::from(mode!="forward");
    'floors:for floor in &run.floors{
        let w=floor.width();let h=floor.tiles.len()/w;
        for y in 1..h-1{for x in 1..w-1{
            let cell=Cell{floor:floor.depth as u32,plane:Plane::Waking,tile:(y*w+x)as u32};
            if [source,destination,side].contains(&cell){continue;}
            let next=if x+1<w-1{cell.tile+1}else{cell.tile-1};
            segments.push(Segment{cell,from:cell,to:Cell{tile:next,..cell}});
            if segments.len()==ordinary_count{break 'floors;}
        }}
    }
    assert_eq!(segments.len(),ordinary_count,"selected real floor geometry must hold the imported trail");
    if mode!="forward"{
        let to=if mode=="free"{destination}else{side};
        segments.insert(length-33,Segment{cell:source,from:source,to});
    }
    let mut trail=Trail::default();for segment in segments{assert_eq!(trail.append(segment),0,"no duplicate imported source cell");}
    assert_eq!(trail.len(),length);run.timeweave=Arc::new(trail).into();
    if mode=="free"{run.player.x=11.;}
    corpse_core::timeweave::validate_run(&run).unwrap();run
}
/// O(repeats*(L setup/admission + actual command/save/view + selected reload)),
/// separate 1k/10k free, forward and large-prefix cases. The actual engine may
/// create COW ownership at its existing command boundary; no measurement-only
/// Arc is held for mutating commands. Full boundary checks are not in timer.
#[test]
#[ignore="lead-controlled imported-state correctness or quiet timing; explicit overlay required"]
fn timeweave_action_long_imported_trail(){
    let options=Options::read();let dir=Sandbox::new(&options.root,"long-weave");let mut rows=vec![];
    for length in [1000usize,10000]{for mode in ["free","forward","collision"]{for repeat in 1..=options.repeats{
        let run=long_weave_fixture(length,mode);let mut engine=temporal::Engine::new(run).unwrap();
        let path=dir.0.join(format!("L{length}-{mode}-{repeat}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);
        let old_rng=engine.run.rng;let old_phase=engine.run.phase.clone();
        // Free travel cannot write the trail; retain its old value for exact proof only.
        let free_old=(mode=="free").then(||engine.run.timeweave.clone());
        let command=Command::Move{dx:if mode=="free"{-1.}else{1.},dy:0.};let delta=if mode=="free"{0.}else{1.};let label=format!("imported-trail-L{length}-{mode}");
        let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,&label,repeat,1,None,&command,delta);
        let expected_len=match mode{"free"=>length,"forward"=>length+1,"collision"=>33,_=>unreachable!()};let actual=engine.run.timeweave.as_ref().unwrap();assert_eq!(actual.len(),expected_len);
        assert_eq!((engine.run.player.x,engine.run.player.y),(if mode=="free"{10.}else{11.},10.));
        if mode=="free"{assert_eq!(engine.run.timeweave,free_old.unwrap());assert_eq!(engine.run.rng,old_rng);assert_eq!(engine.run.phase,old_phase);}
        else{let width=engine.run.floor().unwrap().width();let last=actual.iter().next_back().unwrap();assert_eq!(last.cell.tile,(10*width+10)as u32);assert_eq!(last.to.tile,(10*width+11)as u32);}
        corpse_core::timeweave::validate_run(&engine.run).unwrap();
        row["actual_effect_asserted"]=true.into();row["imported_geometry_valid_state"]=true.into();row["naturally_walked_trace"]=false.into();row["trail_length_before"]=length.into();row["trail_length_after"]=expected_len.into();row["prefix_removed"]=if mode=="collision"{length-32}else{0}.into();rows.push(row);
        if mode=="collision" {
            // Same live Trail after deletion, no reload/replacement: its backing
            // allocations can retain peak capacity H even though live length is33.
            // Source B(11,10) and destination C(10,11) were both reserved by the
            // imported fixture, so this is an ordinary noncolliding new append.
            let width=engine.run.floor().unwrap().width();
            let source=corpse_core::timeweave::Cell{floor:1,plane:corpse_core::timeweave::Plane::Waking,tile:(10*width+11)as u32};
            let destination=corpse_core::timeweave::Cell{tile:(11*width+10)as u32,..source};
            assert!(engine.run.timeweave.as_ref().unwrap().get(source).is_none());
            assert!(engine.run.timeweave.as_ref().unwrap().get(destination).is_none());
            let label=format!("imported-trail-L{length}-post-collision-forward");
            let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,&label,repeat,2,None,&Command::Move{dx:-1.,dy:1.},1.);
            assert_eq!((engine.run.player.x,engine.run.player.y),(10.,11.));
            let trail=engine.run.timeweave.as_ref().unwrap();assert_eq!(trail.len(),34);let last=trail.iter().next_back().unwrap();assert_eq!((last.cell,last.from,last.to),(source,source,destination));
            corpse_core::timeweave::validate_run(&engine.run).unwrap();
            row["actual_effect_asserted"]=true.into();row["imported_geometry_valid_state"]=true.into();row["naturally_walked_trace"]=false.into();row["same_live_trail_after_collision"]=true.into();row["previous_peak_live_length"]=length.into();row["trail_length_before"]=33.into();row["trail_length_after"]=34.into();row["prefix_removed"]=0.into();row["capacity_H_not_directly_observed"]=true.into();rows.push(row);
        }
    }}}report("timeweave-action-long-imported-trail",&options,rows);
}

// Append this test-only fragment to the current timeweave_action_tests.rs after
// source thaw. It reuses only existing private harness helpers; no main changes.

/// O(fixture/world bytes), setup only. Same ten/forty-floor fixture sources as
/// Eat, with no NPCs/ground pickup and controlled ordinary current-floor cells.
fn future_fixture(floors:usize)->Run {
    let mut r=eat_fixture(floors,41,false);
    r.player.x=10.;r.player.y=10.;r.player.hp=100_000.;r.player.max_hp=100_000.;r.player.base_max_hp=100_000.0.into();
    r.player.energy=100.;r.player.stats.max_energy=100.;
    r.player.abilities=vec!["morae-see-the-future".into()];r.player.active.clear();r.player.conditions.clear();
    r.phase="normal".to_string().into();r.future=None;r.run_id="future-audit-anchor-id".to_string().into();r.replay_snapshot_from=Field::Missing;
    for g in r.ground.values_mut(){*g=Arc::new(vec![]);}
    let f=r.floor_mut().unwrap();for tile in &mut f.tiles{*tile="floor".into();}
    f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
    r
}
/// O(1) bounded command payload construction; actual temporal policy is in Engine.
fn future_command()->Command {Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
/// O(1) history metadata assertions, no full-world traversal.
fn future_assert_no_history(engine:&temporal::Engine) {
    assert!(engine.history.frames.is_empty());assert!(engine.history.closing.is_none());assert!(engine.history.pending_haste.is_none());
    assert!(engine.run.history.is_empty());assert!(engine.run.haste_snapshot.is_missing());assert!(!engine.run.player.conditions.contains_key("revertCharges"));
}
/// Actual Future command/flush/view/paint transitive cost, or a checkpoint-only
/// operation followed by the same view/paint. The optional temporal command is
/// always free (`took_turn=false`), but restore can REWIND tick. Thus tick target
/// and outcome are checked independently, unlike the ordinary sample helper.
/// Full serialization/replay oracles run only outside the measured interval.
fn future_sample(options:&Options,engine:&mut temporal::Engine,auto:&mut session_store::Autosave,path:&Path,ui:&mut Ui,label:&str,repeat:usize,step:usize,command:Option<&Command>,expected_tick:f64,expected_checkpoint:bool)->serde_json::Value {
    let before=stamp(path);let old_tick=engine.run.tick;let old_replay=engine.run.replay.len();let old_energy=engine.run.player.energy;
    let old_costs=engine.run.time_costs.value_or_default();let proof=options.prove(repeat)&&label!="future-capture";let wire_cached_before=engine.run.future.as_ref().is_some_and(|a|a.text_if_encoded().is_some());
    let oracle_before=if proof&&command.is_some(){Some(engine.save().unwrap())}else{None};
    assert_eq!(engine.pending_batch().checkpoint_required,command.is_none(),"only checkpoint row starts administratively invalidated");
    assert!(!ui.open);let future_profile_enabled=future_profile_begin(repeat);let now=step as f64*300.;let wall=start(options.timed);let t=start(options.timed);
    let outcome=command.map(|c|dispatch(engine,&mut ui.effects,now,c).unwrap());let action_ns=if command.is_some(){elapsed(t)}else{None};
    let future_profile=future_profile_pause_take(future_profile_enabled);
    let t=start(options.timed);auto.flush_tracked(path,engine,session_store::Kind::Fixture,false).unwrap();let flush_ns=elapsed(t);
    let view_profile_enabled=observe_profile_begin(repeat);
    let t=start(options.timed);let game=observe::observe_with_profile(&engine.run,&ui.painter.profile);let observe_ns=elapsed(t);
    observe_profile_pause();
    let t=start(options.timed);ui.paint_game(game,now);std::hint::black_box(&ui.painter.pixels);let paint_ns=elapsed(t);let wall_ns=elapsed(wall);
    if let Some(out)=outcome.as_ref(){assert!(!out.took_turn);assert!(out.changed);assert!(out.pending_snapshot.is_none());}
    assert_eq!(engine.run.tick,expected_tick);assert_eq!(engine.run.status,"playing");assert!(!ui.open);assert!(ui.notice.is_empty());future_assert_no_history(engine);
    let after=stamp(path);let checkpoint=(before.0,before.1)!=(after.0,after.1);assert_eq!(checkpoint,expected_checkpoint);if !checkpoint{assert!(after.2>before.2);}
    let mut exact_core_oracle=None;
    if let(Some(before),Some(command))=(oracle_before,command){
        let mut expected=corpse_core::temporal::Engine::load(&before).unwrap();let expected_out=expected.dispatch(command).unwrap();assert_eq!(expected_out.took_turn,outcome.as_ref().unwrap().took_turn);assert_eq!(expected_out.changed,outcome.as_ref().unwrap().changed);
        assert_eq!(expected.save().unwrap(),engine.save().unwrap(),"fresh core-only temporal execution versus desktop wrapper");exact_core_oracle=Some(true);
    }
    if proof{ui.prove_pixels(now);}let raw_equal=proof.then(||reload_proof(engine,path));
    let sum=options.timed.then(||[action_ns,flush_ns,observe_ns,paint_ns].into_iter().flatten().sum::<u128>());if let(Some(w),Some(s))=(wall_ns,sum){assert!(w>=s);}
    ui.paint_metadata(serde_json::json!({"sample_id":format!("future-{}floors-{label}-repeat{repeat}",engine.run.floors.len()),"case":label,"repeat":repeat,"step":step,"revision":41,"floors":engine.run.floors.len(),"took_turn":outcome.as_ref().map(|o|o.took_turn),"tick_before":old_tick,"tick_after":engine.run.tick,"tick_delta":engine.run.tick-old_tick,"replay_before":old_replay,"replay_after":engine.run.replay.len(),"energy_before":old_energy,"energy_after":engine.run.player.energy,"costs_before":old_costs,"costs_after":engine.run.time_costs.value_or_default(),"future_bytes":engine.run.future.as_ref().and_then(|a|a.text_if_encoded()).map(str::len),"future_wire_cached_before":wire_cached_before,"future_native_after":engine.run.future.as_ref().is_some_and(|a|a.native_state().is_some()),"capture_proof_deferred_for_cold_checkpoint":label=="future-capture","history_after":engine.history.frames.len(),"checkpoint":checkpoint,"checkpoint_reason":if checkpoint{"explicit administrative mutable access before timer; warm row repeats same runId value"}else{"none; actual Future command appended"},"file_bytes":after.2,"bytes_written":if checkpoint{after.2}else{after.2-before.2},"proof_performed":proof,"raw_live_reload_equal":raw_equal,"exact_ordinary_load_history_proof":proof.then_some(true),"complete_core_temporal_oracle":exact_core_oracle,"pack_open_after":false,"action_ns":action_ns,"future_profile":future_profile,"flush_ns":flush_ns,"observe_ns":observe_ns,"view_profile":observe_profile_finish(view_profile_enabled,format!("{}-{}-{label}-{step}",engine.run.simulation_revision.value_or(39.),engine.run.floors.len()),ui.game.as_ref().unwrap()),"software_game_effects_paint_ns":paint_ns,"wall_ns":wall_ns,"stage_sum_ns":sum,"unattributed_timer_ns":wall_ns.zip(sum).map(|(w,s)|w-s)}),proof)
}
/// O(n*(world/anchor setup + two Future operations + two normal turns + one
/// active-anchor checkpoint + selected full proofs)). Default timed n=5 is a
/// bounded supplemental experiment, not a stable p95/p99 distribution.
#[test]
#[ignore="lead-controlled Future correctness or quiet timing; explicit overlay required"]
fn timeweave_action_future_retained_cost(){
    let mut options=Options::read();options.revisions=vec![41];
    options.repeats=std::env::var("WEAVE_FUTURE_REPEATS").unwrap_or_else(|_|if options.timed{"5"}else{"1"}.into()).parse().unwrap();assert!((1..=10).contains(&options.repeats));
    let dir=Sandbox::new(&options.root,"future");let mut rows=vec![];
    for &floors in &options.floors{for repeat in 1..=options.repeats{
        let mut engine=temporal::Engine::new(future_fixture(floors)).unwrap();let path=dir.0.join(format!("future-{floors}-{repeat}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);
        let initial_tick=engine.run.tick;let initial_energy=engine.run.player.energy;let initial_costs=engine.run.time_costs.value_or_default();let command=future_command();
        let mut capture=future_sample(&options,&mut engine,&mut auto,&path,&mut ui,"future-capture",repeat,1,Some(&command),initial_tick,false);
        assert_eq!(engine.run.player.energy,0.);assert_eq!(engine.run.time_costs.value_or_default().energy,initial_costs.energy+initial_energy);assert_eq!(engine.run.time_costs.value_or_default().charges,initial_costs.charges);
        let anchor=engine.run.future.as_ref().unwrap().native_state().expect("finite41 capture is native").clone();assert!(anchor.history.is_empty());assert!(anchor.haste_snapshot.is_missing());assert!(anchor.future.is_none());assert_eq!(anchor.phase,Field::Value("normal".into()));assert_eq!(anchor.player.energy,0.);assert!(anchor.replay.is_empty());
        capture["actual_anchor_and_full_energy_cost_asserted"]=true.into();rows.push(capture);
        // Actual unknown administrative edits, intentionally outside the checkpoint
        // timer; changing runId also proves restore retains LIVE, not anchor, identity.
        let live_id=format!("future-audit-live-{floors}-{repeat}");engine.run.run_id=live_id.clone().into();let order=engine.run.inventory.iter().map(|i|i.id).collect::<Vec<_>>();set_pack_order(&mut engine,order.into()).unwrap();assert!(engine.pending_batch().checkpoint_required);
        assert!(engine.run.future.as_ref().unwrap().text_if_encoded().is_none(),"cold checkpoint must precede every anchor serialization proof");
        let checkpoint=future_sample(&options,&mut engine,&mut auto,&path,&mut ui,"active-future-cold-administrative-checkpoint",repeat,2,None,initial_tick,true);assert!(engine.run.future.as_ref().unwrap().text_if_encoded().is_some());rows.push(checkpoint);
        // Same-value explicit mutable admin access still invalidates append authority;
        // unlike the cold row, this checkpoint deliberately reuses encoded anchor.
        engine.run.run_id=live_id.clone().into();
        let warm=future_sample(&options,&mut engine,&mut auto,&path,&mut ui,"active-future-warm-administrative-checkpoint",repeat,2,None,initial_tick,true);assert!(engine.run.future.as_ref().unwrap().text_if_encoded().is_some());rows.push(warm);
        for(step,label,c)in [(3,"future-between-move",Command::Move{dx:1.,dy:0.}),(4,"future-between-wait",Command::Wait)]{
            let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,label,repeat,step,None,&c,1.);assert!(engine.run.future.is_some());row["control_between_future_activations"]=true.into();row["sample_id"]=format!("future-{floors}floors-{label}-repeat{repeat}").into();rows.push(row);
        }
        assert_eq!((engine.run.player.x,engine.run.player.y),(anchor.player.x+1.,anchor.player.y));assert_eq!(engine.run.tick,initial_tick+2.);
        let retained_replay=engine.run.replay.clone();let retained_costs=engine.run.time_costs.value_or_default();let retained_policy=engine.run.replay_snapshot_from.clone();
        let mut restored=future_sample(&options,&mut engine,&mut auto,&path,&mut ui,"future-restore",repeat,5,Some(&command),initial_tick,false);
        assert!(engine.run.future.is_none());assert_eq!((engine.run.player.x,engine.run.player.y),(anchor.player.x,anchor.player.y));assert_eq!(engine.run.phase,anchor.phase);assert_eq!(engine.run.run_id.as_ref(),Some(&live_id));assert_eq!(engine.run.time_costs.value_or_default(),retained_costs);assert_eq!(engine.run.player.energy,0.);assert_eq!(engine.run.replay_snapshot_from,retained_policy);
        let mut expected_replay=retained_replay;expected_replay.push(command.clone());assert_eq!(engine.run.replay,expected_replay);future_assert_no_history(&engine);
        // Genuine old world payload restoration, checked outside timer. Full
        // fresh core-only execution above additionally covers source post-hooks/logs.
        if options.prove(repeat){
            assert_eq!(serde_json::to_value((&engine.run.floors,&engine.run.actors,&engine.run.ground,&engine.run.inventory,&engine.run.timeweave)).unwrap(),serde_json::to_value((&anchor.floors,&anchor.actors,&anchor.ground,&anchor.inventory,&anchor.timeweave)).unwrap(),"actual restored world payload");
        }
        restored["actual_anchor_world_restore_asserted"]=options.prove(repeat).into();restored["live_replay_run_id_and_costs_retained"]=true.into();rows.push(restored);
    }}
    let value=serde_json::json!({"schema":1,"suite":"timeweave-action-future-retained-cost","timed":options.timed,"repeats":options.repeats,"revision":41,"floors":options.floors,"overlay_directory":options.root,"proof_cadence":if options.timed{"every row of first and last fresh repeat; effects/identity/ledger each repeat"}else{"every row"},"tail_caveat":"n5 or n10 nearest-rank p95 and p99 both equal the maximum; these are not stable tail estimates","scope":"Actual Future capture, active-anchor administrative checkpoint, Move+Wait and Future restore with current desktop helper, actual durable flush, world observe and software paint. Both Future actions take no turn; restore rewinds tick by2. No pack/SDL/input/present timing. Explicit runId/order edits are outside checkpoint timer. Initial save, anchor parsing, selected exact reload/core/world proofs are outside timers and may perturb following caches. No optimization, physical-disk durability or Windows runtime claim.","rows":rows});
    let out=options.root.join("timeweave-action-future-retained-cost.json");assert!(!out.exists());std::fs::write(&out,serde_json::to_vec_pretty(&value).unwrap()).unwrap();println!("WEAVE_FUTURE_REPORT {}",out.display());
}
/// O(authored fixture bytes + one trail segment), setup only. Both arms retain
/// Weave and identical RNG/actors; only the matching reverse arrow differs.
fn failed_reverse_probe_fixture(with_trail:bool)->Run {
    use corpse_core::timeweave::{Cell,Plane,Segment,Trail};
    let mut run=ordinary_fixture(41);
    run.player.abilities=vec![corpse_core::progression::WEAVE_ID.into()];
    run.player.ability_order=Field::Missing;
    run.player.stats.accuracy=1000.;
    let group=run.actors.get_mut("1").unwrap();
    let npc=&mut Arc::make_mut(group)[0];
    // Preserve authored500 HP: inflating max HP makes its turn recovery hide attack damage.
    npc.conditions.insert("paralyzed".into(),100.);
    npc.stats.defense=0.;npc.stats.dodge=0.;
    assert_eq!((npc.x,npc.y),(13.,14.));
    assert_eq!((run.player.x,run.player.y),(12.,14.));
    run.timeweave=Field::Missing;
    if with_trail {
        let width=run.floor().unwrap().width();
        let a=Cell{floor:1,plane:Plane::Waking,tile:(14*width+13)as u32};
        let b=Cell{tile:a.tile-1,..a};
        let mut trail=Trail::default();trail.append(Segment{cell:a,from:a,to:b});
        run.timeweave=Arc::new(trail).into();
    }
    corpse_core::timeweave::validate_run(&run).unwrap();run
}
/// O(repeats*(two fixture/checkpoint setups + two actual paid attacks, saves and
/// software frames + selected reloads + full canonical equality)). Equality and
/// setup are outside timers. This is a small deterministic combat probe case,
/// not a bound on arbitrarily expensive combat/arrival cascades.
#[test]
#[ignore="lead-controlled failed-probe correctness or quiet timing; explicit overlay required"]
fn timeweave_action_failed_reverse_probe(){
    let options=Options::read();let dir=Sandbox::new(&options.root,"failed-reverse-probe");let mut rows=vec![];
    for repeat in 1..=options.repeats {
        let mut images=vec![];
        // Alternate paired order without changing the fixture or measured action.
        let arms=if repeat%2==1{[false,true]}else{[true,false]};
        for with_trail in arms {
            let run=failed_reverse_probe_fixture(with_trail);
            let mut engine=temporal::Engine::new(run).unwrap();
            let old_trail=engine.run.timeweave.clone();
            let hp=engine.run.actors["1"][0].hp;
            let rng=engine.run.rng;
            let label=if with_trail{"reverse-bump-failed-probe"}else{"reverse-bump-no-trail-control"};
            let path=dir.0.join(format!("{label}-{repeat}.save"));
            let mut auto=session_store::Autosave::default();
            auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();
            let mut ui=Ui::new(&engine.run);
            let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,label,repeat,1,None,&Command::Move{dx:1.,dy:0.},1.);
            assert_eq!((engine.run.player.x,engine.run.player.y),(12.,14.),"attack must not displace player");
            assert_eq!(engine.run.actors["1"].len(),1);
            assert!(engine.run.actors["1"][0].hp<hp&&engine.run.actors["1"][0].hp>0.,"real nonlethal attack required");
            assert_eq!(engine.run.timeweave,old_trail,"neither attack may create/delete trail");
            assert!(engine.run.offers.is_empty(),"no level-up noise");
            let mut image=serde_json::to_value(&engine.run).unwrap();
            image.as_object_mut().unwrap().remove("timeweave");
            images.push(image);
            row["matching_reverse_trail_before"]=with_trail.into();
            row["actual_effect_asserted"]=true.into();
            row["npc_hp_before"]=hp.into();row["npc_hp_after"]=engine.run.actors["1"][0].hp.into();
            row["rng_before"]=rng.into();row["rng_after"]=engine.run.rng.into();
            row["trail_unchanged"]=true.into();
            row["paired_canonical_equality_excluding_only_timeweave"]=true.into();
            row["scope_note"]="Small nonlethal paralyzed-NPC bump attack, not arbitrary combat-cascade worst case. Both arms have Weave; matching trail invokes rejected movement probe before the paid attack.".into();
            rows.push(row);
        }
        assert_eq!(images[0],images[1],"failed probe must leave paid command RNG/logs/state exactly equal to no-trail control");
    }
    report("timeweave-action-failed-reverse-probe",&options,rows);
}

include!("observe_profile_hooks.rs");
include!("observe_breakdown_tests.rs");

// Test-only Future action attribution. No state/anchor inspection or serialization.
#[derive(serde::Serialize)]
struct FutureActionProfile {
    collection_enabled:bool,
    #[cfg(feature="runtime-timing")]
    spans:std::collections::BTreeMap<&'static str,corpse_core::timing::Totals>,
    #[cfg(feature="runtime-timing")]
    counts:std::collections::BTreeMap<&'static str,u64>,
}
/// O(previous labels + environment text), before root/action clocks. Off and
/// alternate are explicit diagnostic controls; absent mode returns no report.
fn future_profile_begin(repeat:usize)->Option<bool>{
    #[cfg(feature="runtime-timing")]{
        let enabled=match std::env::var("FUTURE_PROFILE_MODE").as_deref(){Ok("on")=>Some(true),Ok("off")=>Some(false),Ok("alternate")=>Some(repeat%2==0),_=>None};
        corpse_core::timing::set_enabled(false);corpse_core::timing::reset();
        corpse_core::timing::set_enabled(enabled.unwrap_or(false));enabled
    }
    #[cfg(not(feature="runtime-timing"))]{let _=repeat;None}
}
/// O(fixed counter count) extraction after action clock, before flush; BTreeMap
/// span ownership moves without formatting. This bookkeeping remains inside the
/// diagnostic whole-action wall interval and its unattributed residual, not the
/// command stage. Observation profiling subsequently resets its own collector.
fn future_profile_pause_take(enabled:Option<bool>)->Option<FutureActionProfile>{
    #[cfg(feature="runtime-timing")]{
        corpse_core::timing::set_enabled(false);
        let spans=corpse_core::timing::take_report();let counts=corpse_core::timing::take_counts();
        enabled.map(|collection_enabled|FutureActionProfile{collection_enabled,spans,counts})
    }
    #[cfg(not(feature="runtime-timing"))]{let _=enabled;None}
}
/// O(n*(two40-floor setup/load preparations + two actual restore commands +
/// exact canonical proofs)). Core-command-only: no save/observe/paint in timers.
/// Fresh and imported states are equal before the measured paired commands;
/// load and proof serialization are deliberately outside both action timers.
#[test]
#[ignore="lead-controlled imported Future restore comparison; explicit overlay output required"]
fn timeweave_imported_future_restore_core_only(){
    let mut options=Options::read();options.revisions=vec![41];options.floors=vec![40];
    options.repeats=std::env::var("WEAVE_FUTURE_REPEATS").unwrap_or_else(|_|if options.timed{"5"}else{"1"}.into()).parse().unwrap();assert!((1..=10).contains(&options.repeats));
    let command=future_command();let mut rows=vec![];
    for repeat in 1..=options.repeats {
        let mut fresh=corpse_core::temporal::Engine::new(future_fixture(40)).unwrap();
        fresh.dispatch(&command).unwrap();fresh.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();
        let wire=fresh.save().unwrap();let loaded=corpse_core::temporal::Engine::load(&wire).unwrap();
        assert_eq!(wire,loaded.save().unwrap(),"paired canonical precondition including anchor wire");
        assert!(!fresh.run.future.as_ref().unwrap().is_certified_import());
        assert!(loaded.run.future.as_ref().unwrap().is_certified_import());
        let mut arms=if repeat%2==1{vec![("fresh-native-restore-core",fresh),("imported-certified-restore-core",loaded)]}else{vec![("imported-certified-restore-core",loaded),("fresh-native-restore-core",fresh)]};
        let mut canonical=vec![];let mut events=vec![];
        for(label,engine)in &mut arms {
            let imported=engine.run.future.as_ref().unwrap().is_certified_import();
            let old_tick=engine.run.tick;let ledger=engine.run.time_costs.clone();let replay=engine.run.replay.clone();let run_id=engine.run.run_id.clone();
            let t=start(options.timed);let outcome=engine.dispatch(&command).unwrap();let action_ns=elapsed(t);
            assert!(!outcome.took_turn);assert!(outcome.changed);assert!(outcome.pending_snapshot.is_none());assert!(engine.run.future.is_none());
            assert_eq!(engine.run.tick,old_tick-1.);assert_eq!(engine.run.time_costs,ledger);assert_eq!(engine.run.run_id,run_id);
            let mut expected_replay=replay;expected_replay.push(command.clone());assert_eq!(engine.run.replay,expected_replay);
            canonical.push(engine.save().unwrap());events.push(serde_json::to_value(&outcome.events).unwrap());
            rows.push(serde_json::json!({"case":label,"repeat":repeat,"revision":41,"floors":40,"imported_certified_before":imported,"action_ns":action_ns,"scope":"core Future restore command only; load/setup/proof outside timer; no durable save, observation or painting","paired_full_canonical_and_events_equal":true,"ledger_replay_run_id_and_rewound_tick_asserted":true}));
        }
        assert_eq!(canonical[0],canonical[1],"fresh/imported restore exact full canonical result");assert_eq!(events[0],events[1],"same presentation outcomes");
    }
    report("timeweave-imported-future-restore-core-only",&options,rows);
}

include!("tile_action_frame_tests.rs");

include!("tile_animated_action_tests.rs");

include!("tile_paint_attribution_tests.rs");

include!("tile_sdl_timing_tests.rs");

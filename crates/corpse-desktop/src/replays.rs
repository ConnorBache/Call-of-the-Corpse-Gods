//! Local replay browser; playback owns a separate engine and never enters score records.
use crate::{
    effects::Effects,
    observe,
    render::{Painter, View, GOLD, INK, MUTED},
};
use corpse_core::replay::{ReplayRecord, ReplaySession};
use std::path::{Path, PathBuf};
pub struct Replays {
    pub files: Vec<PathBuf>,
    fast_files: std::collections::HashSet<PathBuf>,
    pub selected: usize,
    pub session: Option<ReplaySession>,
    pub playing: bool,
    pub last_step: f64,
    pub notice: String,
    view: Option<View>,
    effects: Effects,
    paint_stage: Option<crate::staged_firebolt::PaintStage>,
    meters: crate::statuses_ui::ConditionMeters,
    last_tick: f64,
    profile: crate::appearance_profile::Profile,
}
impl Replays {
    pub fn new(directory: &Path) -> Self {
        let mut files = std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect::<Vec<_>>();
        files.sort();
        // Metadata is learned only from an explicitly loaded replay, never scanned eagerly.
        let fast_files=std::collections::HashSet::new();
        Self {
            fast_files,
            files,
            selected: 0,
            session: None,
            playing: false,
            last_step: 0.,
            notice: String::new(),
            view: None,
            effects: Effects::default(),
            paint_stage: None,
            meters: Default::default(),
            last_tick: 0.,
            profile: Default::default(),
        }
    }
    pub fn load(&mut self, path: &Path) -> Result<(), String> {
        let m = std::fs::metadata(path).map_err(|e| e.to_string())?;
        if m.len() > 32 * 1024 * 1024 {
            return Err("Replay exceeds32MiB safety limit".into());
        }
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let record: ReplayRecord = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let session = ReplaySession::create(record, true).map_err(|e| e.to_string())?;
        if session.record.fast_mode.value_or(false) && !session.engine.run.compressed_depths.value_or(false) { self.fast_files.insert(path.to_owned()); }
        else { self.fast_files.remove(path); }
        self.session = Some(session);
        self.playing = false;
        self.effects.clear();
        self.meters.reset();
        self.refresh();
        self.notice = String::new();
        Ok(())
    }
    fn refresh(&mut self) {
        if let Some(s) = &self.session {
            if s.engine.run.tick < self.last_tick {
                self.meters.reset();
            }
            self.last_tick = s.engine.run.tick;
            let mut v = observe::observe_with_profile(&s.engine.run, &self.profile);
            v.measure_conditions(&mut self.meters);
            v.provenance = if s.engine.run.playtest.is_missing() {
                "Native replay / unranked / port coverage incomplete"
            } else { "PLAYTEST REPLAY • Synthetic prior-floor collection • Unranked" }.into();
            self.view = Some(v);
        }
    }
    pub fn configure_motion(&mut self,reduced:bool,instant:bool) {
        self.effects.configure_motion(reduced,instant);
    }
    /// Autoplay waits for detached travel, not simulation time. Manual stepping
    /// remains an explicit interruption and never waits for presentation.
    pub fn ready_for_step(&self,now:f64)->bool { !self.effects.travel_pending(now) }
    pub fn animating(&self,now:f64)->bool {self.effects.animate(now)||self.effects.travel_pending(now)}
    pub fn step(&mut self, now: f64) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let sequence_before = s.engine.run.clone();
        let profile = self.profile.clone();
        let observer = corpse_core::events::PresentationObserver::install(|run| std::rc::Rc::new(run.clone()));
        let before=s.record.commands.get(s.cursor)
            .filter(|command|crate::staged_firebolt::needs_snapshot(&s.engine.run,command))
            .map(|_|observe::observe_with_profile(&s.engine.run,&self.profile));
        match s.step_with_outcome() {
            Ok(out) => {
                if let Some(out) = out {
                    self.effects.add(
                        out.events,
                        s.engine.run.tick,
                        s.engine.run.depth,
                        now,
                        false,
                    );
                    self.effects.sequence.capture_run(sequence_before, observer.take(), &self.effects.cues, &profile);
                    if let Some(before)=before {self.effects.firebolt_terrain.capture(before,&self.effects.cues);}
                    self.effects.defer_until_frame(now);
                }
                self.last_step = now;
                if s.finished() {
                    self.playing = false;
                }
                self.refresh();
            }
            Err(e) => {
                self.notice = e.to_string();
                self.playing = false;
            }
        }
    }
    pub fn restart(&mut self) {
        if let Some(s) = self.session.as_mut() {
            match s.restart() {
                Ok(()) => {
                    self.playing = false;
                    self.effects.clear();
                    self.meters.reset();
                    self.notice.clear();
                    self.refresh();
                }
                Err(e) => self.notice = e.to_string(),
            }
        }
    }
    pub fn library(&self, p: &mut Painter) {
        p.clear([8, 8, 8]);
        p.text("View Replay", 60, 110, 36, INK);
        p.text("Choose an archived descent.",60,170,18,MUTED);
        if self.files.is_empty() {
            p.text("No archived descents.", 60, 250, 20, INK);
        }
        for (i, path) in self
            .files
            .iter()
            .enumerate()
            .skip(self.selected.saturating_sub(7))
            .take(12)
        {
            let y = 235 + (i - self.selected.saturating_sub(7)) as i32 * 35;
            if i == self.selected {
                p.rect(60, y - 3, 1150, 32, [38, 34, 22]);
            }
            if self.fast_files.contains(path) { p.text("Fast mode", 1100, y, 14, GOLD); }
            p.text(
                &crate::render::truncate(
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                    if self.fast_files.contains(path) { 82 } else { 95 },
                ),
                74,
                y,
                17,
                INK,
            );
        }
        p.text(
            &crate::render::truncate(&self.notice, 115),
            60,
            675,
            15,
            MUTED,
        );
        p.text(
            "Drop a replay JSON file to import.  [Enter] View  [↑/↓] Select  [Esc] Main menu",
            60,
            730,
            16,
            GOLD,
        );
    }
    pub fn paint(&mut self, p: &mut Painter, now: f64, reduced: bool) {
        if self.profile.data() != p.profile.data() {
            self.profile = p.profile.clone();
            self.refresh();
        }
        self.effects.begin_frame(now);
        self.effects.prune(now);
        p.firebolt_terrain=None;
        if let Some(v)=self.view.as_mut(){crate::hallucination_ui::apply_view(v,crate::hallucination_ui::sample(now,reduced));}
        if let (Some(s), Some(v)) = (&self.session, &self.view) {
            let v = self.effects.sequence.view(now).unwrap_or(v);
            self.effects.firebolt_terrain.prepare(v,&self.effects.cues,now,reduced,&mut self.paint_stage);
            p.firebolt_terrain=self.paint_stage.take();
            p.game_at(v, "", false, if reduced { 0. } else { now });
            p.effects(v, &self.effects.cues, now, reduced);
            p.rect(12, p.logical_height()-94, p.logical_width()-24, 82, [11, 13, 21]);
            p.text(
                &format!(
                    "Replay {} / {}   {}",
                    s.cursor,
                    s.record.commands.len(),
                    if s.finished() {
                        "Finished"
                    } else if self.playing {
                        "Playing"
                    } else {
                        "Paused"
                    }
                ),
                25,
                p.logical_height()-85,
                17,
                GOLD,
            );
            p.text(
                "[Space] Step   [P] Play / Pause   [R] Restart   [Esc] Exit replay",
                25,
                p.logical_height()-50,
                17,
                INK,
            );
            if !self.notice.is_empty() {
                p.text(
                    &crate::render::truncate(&self.notice, 95),
                    25,
                    777,
                    12,
                    [240, 130, 110],
                );
            }
        }
        // Painter is shared with live play and menus; replay never retains its
        // detached mask beyond its own composition call.
        self.paint_stage=p.firebolt_terrain.take();
    }
}

#[cfg(test)]
mod player_glyph_tests {
    use super::*;

    #[test]
    fn replay_library_does_not_eagerly_read_record_metadata() {
        let dir=std::env::temp_dir().join(format!("corpse-no-replay-scan-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path=dir.join("unopened.json");
        std::fs::write(&path,b"{\"fastMode\":true}").unwrap();
        let replay=Replays::new(&dir);
        assert_eq!(replay.files,vec![path.clone()]);
        assert!(replay.fast_files.is_empty(),"unopened file contents must not be scanned");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn player_glyph_legacy_replay_refresh_step_restart_and_profile_reload_are_plain_at() {
        let initial = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut record = ReplayRecord::from_run(&initial);
        record.commands = vec![corpse_core::Command::Wait];
        let mut replay = Replays::new(Path::new("/__no_player_glyph_replay_directory__"));
        replay.session = Some(ReplaySession::from_initial(initial, record).unwrap());
        let mut painter = Painter::new();
        painter.profile = painter.profile.clone()
            .set("entities.objects.player.inherited", serde_json::json!(false)).unwrap()
            .set("entities.objects.player.glyph", serde_json::json!("R")).unwrap();
        for phase in 0..3 {
            if phase == 1 { replay.step(100.); }
            if phase == 2 { replay.restart(); }
            assert_eq!(replay.session.as_ref().unwrap().cursor, usize::from(phase == 1));
            let before = serde_json::to_vec(&replay.session.as_ref().unwrap().engine.run).unwrap();
            replay.refresh();
            replay.paint(&mut painter, 0., true);
            let view = replay.view.as_ref().unwrap();
            let cell = view.cells.iter().find(|cell| cell.actor_id == Some(view.player_id)).unwrap();
            assert_eq!(cell.glyph, "@");
            assert!(cell.superscript.is_empty());
            assert_eq!(serde_json::to_vec(&replay.session.as_ref().unwrap().engine.run).unwrap(), before);
            assert!(replay.notice.is_empty());
            if phase == 0 {
                if let Ok(directory) = std::env::var("CORPSE_PLAYER_GLYPH_REVIEW") {
                    std::fs::create_dir_all(&directory).unwrap();
                    let file = std::fs::File::create(format!("{directory}/player-legacy-replay-1280x800.png")).unwrap();
                    let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod staged_playback_tests {
    use super::*;
    fn replay()->Replays {
        let initial=crate::staged_firebolt::tests::room();
        let mut record=ReplayRecord::from_run(&initial);
        record.commands=vec![corpse_core::Command::UseAt{id:900.,x:11.,y:10.},corpse_core::Command::Wait];
        let mut playback=Replays::new(Path::new("/__no_staged_replay_directory__"));
        playback.session=Some(ReplaySession::from_initial(initial,record).unwrap());
        playback.refresh();playback
    }
    #[test]
    fn firebolt_waits_for_first_frame_and_travel_while_manual_step_interrupts() {
        let mut replay=replay();let mut painter=Painter::new();
        replay.step(0.);
        assert!(!replay.ready_for_step(900.),"queued unseen frame cannot expire while minimized");
        let canonical=replay.session.as_ref().unwrap().engine.save().unwrap();
        replay.paint(&mut painter,1000.,false);
        assert!(painter.firebolt_terrain.is_none(),"replay mask must not leak into shared painter");
        assert!(replay.paint_stage.is_some());assert!(!replay.ready_for_step(1120.));
        assert_eq!(replay.session.as_ref().unwrap().cursor,1);
        let cells=replay.paint_stage.as_ref().unwrap().cells.clone();
        replay.paint(&mut painter,1001.,false);
        assert!(std::sync::Arc::ptr_eq(&cells,&replay.paint_stage.as_ref().unwrap().cells));
        assert_eq!(replay.session.as_ref().unwrap().engine.save().unwrap(),canonical);
        assert!(replay.ready_for_step(10000.));
        replay.step(1100.); // Explicit Space remains an allowed interruption.
        assert_eq!(replay.session.as_ref().unwrap().cursor,2);
        replay.paint(&mut painter,1120.,false);
        assert!(replay.paint_stage.is_none());assert!(painter.firebolt_terrain.is_none());
    }
    #[test]
    fn reduced_and_instant_replay_wait_for_visible_static_feedback() {
        for instant in [false,true] {
            let mut replay=replay();let mut painter=Painter::new();
            replay.configure_motion(true,instant);replay.step(0.);
            assert!(!replay.ready_for_step(0.));replay.paint(&mut painter,1000.,false);
            assert!(replay.paint_stage.is_none());assert!(painter.firebolt_terrain.is_none());
            assert!(!replay.ready_for_step(1000.));
            assert!(replay.ready_for_step(10000.));
        }
    }
}

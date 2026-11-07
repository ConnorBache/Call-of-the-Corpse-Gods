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
    pub selected: usize,
    pub session: Option<ReplaySession>,
    pub playing: bool,
    pub last_step: f64,
    pub notice: String,
    view: Option<View>,
    effects: Effects,
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
        Self {
            files,
            selected: 0,
            session: None,
            playing: false,
            last_step: 0.,
            notice: String::new(),
            view: None,
            effects: Effects::default(),
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
    pub fn step(&mut self, now: f64) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
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
        p.text("A RECORD OF WHAT HAPPENED", 60, 65, 18, GOLD);
        p.text("View Replay", 60, 110, 36, INK);
        p.text("Choose an archived descent. Replays from an older simulation need their original build.",60,170,18,MUTED);
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
            p.text(
                &crate::render::truncate(
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                    95,
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
        self.effects.prune(now);
        if let Some(v)=self.view.as_mut(){crate::hallucination_ui::apply_view(v,crate::hallucination_ui::sample(now,reduced));}
        if let (Some(s), Some(v)) = (&self.session, &self.view) {
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
    }
}

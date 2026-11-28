//! Read-only profile/score viewer until full native mechanical coverage is gated.
use crate::render::{Painter, GOLD, INK, MUTED};
use corpse_core::profile::Profile;
use std::path::Path;
pub struct Profiles {
    pub profile: Profile,
    pub selected: usize,
    pub notice: String,
    pub detail: bool,
}
impl Profiles {
    pub fn new(path: &Path) -> Self {
        let mut s = Self {
            profile: Profile::default(),
            selected: 0,
            notice: String::new(),
            detail: false,
        };
        if path.exists() {
            if let Err(e) = s.load(path) {
                s.notice = e;
            }
        }
        s
    }
    pub fn load(&mut self, path: &Path) -> Result<(), String> {
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
            return Err("Profile exceeds32MiB safety limit".into());
        }
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.profile =
            corpse_core::profile::parse_profile(Some(&raw)).map_err(|e| e.to_string())?;
        self.selected = 0;
        self.detail = false;
        self.notice.clear();
        Ok(())
    }
    pub fn paint(&self, p: &mut Painter) {
        p.clear([8, 8, 8]);
        p.text("High Scores", 60, 110, 36, INK);
        if self.profile.high_scores.is_empty() {
            p.text("No recorded descents.", 60, 250, 20, INK);
        }
        if self.detail {
            if let Some(record) = self.profile.high_scores.get(self.selected) {
                let s = &record.summary;
                p.text(
                    &format!("{} / {} / Depth {:.0}", s.mode_label(), s.status, s.depth),
                    60,
                    230,
                    24,
                    GOLD,
                );
                p.text(
                    &format!(
                        "Blessings {:.0}   Turns {:.0}",
                        s.observed_blessings, s.tick
                    ),
                    60,
                    279,
                    20,
                    INK,
                );
                p.text(&crate::render::truncate(&s.seed, 90), 60, 319, 18, MUTED);
                if let Some(reason) = s.death_reason.as_ref().or(s.ending.as_ref()) {
                    for (i, line) in crate::manual::wrap(reason, 100).iter().enumerate() {
                        p.text(line, 60, 360 + i as i32 * 24, 16, INK);
                    }
                }
                p.text("Hearts eaten", 60, 440, 19, GOLD);
                for (i, h) in s.hearts.iter().take(7).enumerate() {
                    p.text(
                        &format!("{}  {:.0}", h.creature, h.count),
                        60,
                        480 + i as i32 * 24,
                        16,
                        INK,
                    );
                }
            }
        } else {
            p.text(
                "BLESSINGS     VESSEL      DEPTH   TURNS    RESULT",
                74,
                221,
                15,
                GOLD,
            );
            let first = self.selected.saturating_sub(7);
            for (i, r) in self
                .profile
                .high_scores
                .iter()
                .enumerate()
                .skip(first)
                .take(12)
            {
                let y = 255 + (i - first) as i32 * 32;
                if i == self.selected {
                    p.rect(60, y - 3, 1150, 30, [38, 34, 22]);
                }
                p.text(
                    &format!(
                        "{:>8.0}     {:<10}  {:>3.0}   {:>6.0}   {}",
                        r.summary.score.unwrap_or(0.),
                        r.summary.mode_label(),
                        r.summary.depth,
                        r.summary.tick,
                        r.summary.status
                    ),
                    74,
                    y,
                    17,
                    INK,
                );
            }
        }
        p.text(
            &crate::render::truncate(&self.notice, 120),
            60,
            678,
            14,
            [240, 130, 110],
        );
        p.text(
            "Drop a profile JSON to view.  [↑/↓] Select  [Enter] Details",
            60,
            744,
            16,
            GOLD,
        );
        p.close_hint(60,760);
    }
}

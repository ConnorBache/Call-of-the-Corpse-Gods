//! Detached terminal reminder; it never changes score records or simulation state.
use crate::render::{Painter, GOLD, INK, MUTED};
use corpse_core::profile::RunSummary;

fn headline(s: &RunSummary) -> &str {
    if s.status == "won" {
        s.ending.as_deref().unwrap_or("You reached the bottom.")
    } else {
        "Your descent has ended."
    }
}

fn result_reason(s: &RunSummary) -> &str {
    s.death_reason
        .as_deref()
        .or(s.ending.as_deref())
        .unwrap_or(if s.status == "won" {
            "Reached the bottom"
        } else {
            "Cause not recorded"
        })
}

pub fn paint(p: &mut Painter, s: &RunSummary, offset: usize, notice: &str) {
    p.clear([5, 4, 12]);
    p.text("THE DESCENT IS WRITTEN", 180, 105, 16, GOLD);
    p.text(
        headline(s),
        180,
        145,
        34,
        INK,
    );
    let reason = result_reason(s);
    for (i, line) in crate::manual::wrap(&format!("Depth {:.0}, {reason}", s.depth), 86)
        .iter()
        .take(3)
        .enumerate()
    {
        p.text(line, 180, 205 + i as i32 * 23, 17, MUTED);
    }
    p.text("Hearts eaten", 180, 295, 24, GOLD);
    p.text("Blessings received", 710, 295, 24, GOLD);
    p.text(
        &s.score
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|| "Unknown".into()),
        710,
        345,
        54,
        GOLD,
    );
    let score_note = if s.score.is_some() {
        "Every blessing received, including blessings already spent.".into()
    } else {
        format!("{:.0} blessings recorded since tracking began. The earlier total is unknown; this run is not ranked.", s.observed_blessings)
    };
    for (i, line) in crate::manual::wrap(&score_note, 40).iter().enumerate() {
        p.text(line, 710, 425 + i as i32 * 23, 16, MUTED);
    }
    if s.hearts.is_empty() {
        p.text(
            if s.hearts_complete {
                "No hearts eaten."
            } else {
                "No recorded hearts eaten."
            },
            180,
            345,
            17,
            INK,
        );
    }
    for (i, h) in s
        .hearts
        .iter()
        .skip(offset.min(s.hearts.len().saturating_sub(9)))
        .take(9)
        .enumerate()
    {
        p.text(
            &crate::render::truncate(&h.creature, 36),
            180,
            345 + i as i32 * 27,
            17,
            INK,
        );
        p.text(
            &format!("{:.0}", h.count),
            615,
            345 + i as i32 * 27,
            17,
            GOLD,
        );
    }
    if !s.hearts_complete {
        p.text("Only recorded hearts are shown.", 180, 605, 14, MUTED);
    }
    p.text(&crate::render::truncate(notice, 95), 180, 645, 14, MUTED);
    if notice.starts_with("Cannot")
        || notice.starts_with("Could not")
        || notice.starts_with("Invalid")
    {
        p.text("[R] Retry saving records", 180, 670, 14, GOLD);
    }
    p.text("[Enter] Descend again", 180, 700, 20, GOLD);
    p.text("[Esc] Main menu", 710, 700, 20, INK);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn summary() -> RunSummary {
        let run = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        corpse_core::profile::summarize_run(&run)
    }

    #[test]
    fn finale_labels_keep_exact_designer_wording() {
        let mut s = summary();
        s.status = "won".into();
        s.ending = Some("Collapsed the flesh!".into());
        assert_eq!(headline(&s), "Collapsed the flesh!");
        assert_eq!(result_reason(&s), "Collapsed the flesh!");

        s.status = "dead".into();
        s.ending = None;
        s.death_reason = Some("Fell while escaping the collapse of the flesh.".into());
        assert_eq!(headline(&s), "Your descent has ended.");
        assert_eq!(
            result_reason(&s),
            "Fell while escaping the collapse of the flesh."
        );
    }

    #[test]
    fn legacy_victory_retains_original_labels() {
        let mut s = summary();
        s.status = "won".into();
        assert_eq!(headline(&s), "You reached the bottom.");
        assert_eq!(result_reason(&s), "Reached the bottom");
    }

    #[test]
    fn collapse_victory_paints_exact_heading() {
        let mut s = summary();
        s.status = "won".into();
        s.ending = Some("Collapsed the flesh!".into());
        let before = s.clone();
        let mut actual = Painter::new();
        paint(&mut actual, &s, 0, "");
        let mut expected = Painter::new();
        expected.clear([5, 4, 12]);
        expected.text("Collapsed the flesh!", 180, 145, 34, INK);
        for y in 140..195 {
            let row = y * actual.width * 4..(y + 1) * actual.width * 4;
            assert_eq!(&actual.pixels[row.clone()], &expected.pixels[row]);
        }
        assert_eq!(s, before, "terminal paint must not mutate the summary");
    }

    #[test]
    fn collapse_loss_paints_exact_reason() {
        let mut s = summary();
        s.status = "dead".into();
        s.depth = 40.0;
        s.death_reason = Some("Fell while escaping the collapse of the flesh.".into());
        let mut actual = Painter::new();
        paint(&mut actual, &s, 0, "");
        let mut expected = Painter::new();
        expected.clear([5, 4, 12]);
        expected.text(
            "Depth 40, Fell while escaping the collapse of the flesh.",
            180,
            205,
            17,
            MUTED,
        );
        for y in 200..235 {
            let row = y * actual.width * 4..(y + 1) * actual.width * 4;
            assert_eq!(&actual.pixels[row.clone()], &expected.pixels[row]);
        }
    }

    #[test]
    fn export_finale_review() {
        let Ok(directory) = std::env::var("CORPSE_FINALE_UI_REVIEW") else {
            return;
        };
        std::fs::create_dir_all(&directory).unwrap();
        let mut s = summary();
        let mut painter = Painter::new();
        for (status, depth, ending, reason) in [
            ("won", 1.0, Some("Collapsed the flesh!"), None),
            (
                "dead",
                40.0,
                None,
                Some("Fell while escaping the collapse of the flesh."),
            ),
        ] {
            s.status = status.into();
            s.depth = depth;
            s.ending = ending.map(str::to_owned);
            s.death_reason = reason.map(str::to_owned);
            paint(&mut painter, &s, 0, "Controlled fixture / unranked");
            let file = std::fs::File::create(format!("{directory}/finale-{status}.png")).unwrap();
            let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        }
    }

    #[test]
    fn export_modern_collapse_loss_review() {
        let Ok(directory) = std::env::var("CORPSE_MODERN_LOSS_REVIEW") else { return; };
        let input = std::env::var("CORPSE_MODERN_LOSS_INPUT").expect("modern loss input save");
        let mut run = corpse_core::storage::deserialize(&std::fs::read_to_string(input).unwrap()).unwrap();
        assert_eq!(run.simulation_revision.as_ref(), Some(&31.0));
        assert!(run.run_stats.as_ref().is_some());
        let wave = &run.collapse.as_ref().unwrap().waves[0];
        let depth = wave.depth as f64;
        let at = wave.cells.iter().position(|v| *v).unwrap();
        let width = run.floors.iter().find(|f| f.depth == depth).unwrap().width();
        run.depth = depth;
        run.player.x = (at % width) as f64;
        run.player.y = (at / width) as f64;
        assert!(corpse_core::travel::travel(&mut run, 1.0, true, false).unwrap());
        assert_eq!(run.status, "dead");
        let summary = corpse_core::profile::summarize_run(&run);
        assert_eq!(summary.death_reason.as_deref(), Some(corpse_core::finale::FALL_MESSAGE));
        let mut painter = Painter::new();
        paint(&mut painter, &summary, 0, "REV31 native QA fixture / unranked");
        std::fs::create_dir_all(&directory).unwrap();
        let file = std::fs::File::create(format!("{directory}/rev31-modern-collapse-loss.png")).unwrap();
        let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        std::fs::write(format!("{directory}/rev31-modern-collapse-loss.json"), corpse_core::storage::serialize(&run).unwrap()).unwrap();
    }

    #[test]
    fn terminal_summary_is_detached_and_handles_victory() {
        let mut r = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        r.status = "won".into();
        let before = r.clone();
        let s = corpse_core::profile::summarize_run(&r);
        let mut p = Painter::new();
        paint(&mut p, &s, usize::MAX, "Controlled fixture / unranked");
        assert_eq!(r, before);
        assert!(p.pixels.iter().any(|v| *v == 255));
    }
}

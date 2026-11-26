use super::*;

#[test]
fn paper_and_art_backing_change_continuously_but_ink_switches_once() {
    let mut previous = Theme::at(0);
    let mut flips = 0;
    for sample in 1..=39_000 {
        let next = Theme::at_depth(sample as f64 / 1000.);
        for (a, b) in [(previous.paper, next.paper), (previous.void_paper, next.void_paper)] {
            assert!(a.into_iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 1));
        }
        assert!((next.void_inset - previous.void_inset).abs() <= 1);
        flips += usize::from(next.ink != previous.ink);
        previous = next;
    }
    assert_eq!(flips, 1);
    assert_eq!(Theme::at(0).ink, [0; 3]);
    assert_eq!(Theme::at(39).ink, [255; 3]);
}

#[test]
fn page_and_selected_index_fill_contrast_without_a_keyline() {
    for sample in 0..=39_000 {
        let depth = sample as f64 / 1000.;
        let theme = Theme::at_depth(depth);
        assert!(theme_contrast(theme.ink, theme.paper) >= 4.5, "{depth}: page text");
        assert!(theme_contrast(theme.selected_ink, theme.ink) >= 4.5, "{depth}: selected text");
    }
}

#[test]
fn void_field_blends_from_paper_without_a_visibility_or_size_threshold() {
    for sample in 0..=1800 {let t=Theme::at_depth(sample as f64/100.);assert_eq!(t.void_paper,t.paper);assert_eq!(t.void_inset,0);}
    assert_eq!(Theme::at(28).void_paper,[150,64,70]);
    assert_eq!(Theme::at(28).void_inset,12);
    assert_eq!(Theme::at(39).void_paper,[150,64,70]);
    let mut prior=0;
    for sample in 1800..=2800 {let t=Theme::at_depth(sample as f64/100.);assert!(t.void_inset>=prior && t.void_inset<=12);prior=t.void_inset;}
}

#[test]
fn full_names_and_stat_labels_fit_and_badges_keep_their_glyph_color() {
    let mut manual = Manual::new();
    let painter = Painter::new();
    for id in manual.ids().to_vec() {
        manual.select_id(&id);
        manual.prepare_art();
        let entry = manual.entry().unwrap();
        let page = resolve_page(&painter, entry, manual.art.as_ref().map(|(_, a)| a));
        assert!(painter.prose_width(&entry.name, INDEX_LABEL_SIZE, false)
            <= INDEX_RIGHT - INDEX_LEFT - 2 * INDEX_LABEL_PAD, "{id}: full index name");
        let cell = page.stats.2 / if page.stats.3 { 8 } else { 4 };
        for (label, _) in stat_rows(entry, 52.) {
            assert!(painter.prose_width(label, STAT_LABEL_SIZE, false) <= cell - 6, "{id}: {label}");
        }
        let glyph = crate::color::hex(&entry.color);
        assert!(theme_contrast(glyph, badge_paper(glyph)) >= 4.5, "{id}: glyph badge");
    }
    assert_eq!(badge_paper([0; 3]), GLYPH_LIGHT);
}

#[test]
fn render_all_current_pages_for_readability_review() {
    let Ok(directory) = std::env::var("CORPSE_BESTIARY_READABILITY_OUTPUT") else { return; };
    std::fs::create_dir_all(&directory).unwrap();
    let mut manual = Manual::new();
    manual.preview_all();
    let mut painter = Painter::new();
    for (index, id) in manual.ids().to_vec().iter().enumerate() {
        assert!(manual.select_id(id));
        for (width, height) in [(1280, 800), (1024, 600), (1920, 1080), (900, 1200)] {
            painter.try_resize(width, height).unwrap();
            painter.set_responsive_canvas(false);
            manual.paint(&mut painter);
            let path = format!("{directory}/{index:02}-{width}x{height}.png");
            let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Fast);
            encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        }
    }
}

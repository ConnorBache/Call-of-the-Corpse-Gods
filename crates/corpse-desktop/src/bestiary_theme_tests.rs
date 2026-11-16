use super::*;

fn colors(t:&Theme)->[[u8;3];10] {
    [t.paper,t.ink,t.secondary,t.rule,t.accent,t.unread,t.selected_ink,t.selected_unread,t.void_paper,t.keyline]
}
fn outlined_contrast(foreground:[u8;3],background:[u8;3],keyline:[u8;3])->f64 {
    theme_contrast(foreground,background).max(theme_contrast(keyline,background))
}

#[test]
fn every_palette_color_has_only_byte_quantization_not_threshold_jumps() {
    let mut previous=Theme::at_depth(0.);
    for sample in 1..=39_000 {
        let next=Theme::at_depth(sample as f64/1000.);
        for (a,b) in colors(&previous).into_iter().zip(colors(&next)) {
            assert!(a.into_iter().zip(b).all(|(a,b)|a.abs_diff(b)<=1),"depth {sample}: {previous:?} -> {next:?}");
        }
        assert!((next.void_inset-previous.void_inset).abs()<=1);
        previous=next;
    }
    for anchor in [0.,4.,6.,8.,16.,18.,24.,28.,39.] {
        let a=Theme::at_depth(anchor-0.00001);let b=Theme::at_depth(anchor+0.00001);
        for (a,b) in colors(&a).into_iter().zip(colors(&b)) {
            assert!(a.into_iter().zip(b).all(|(a,b)|a.abs_diff(b)<=1),"anchor {anchor}");
        }
    }
}

#[test]
fn palette_anchors_and_fractional_color_interpolation_are_explicit() {
    assert_eq!(Theme::at(0).paper,[239,232,204]);
    assert_eq!(Theme::at(24).paper,[143,28,32]);
    assert_eq!(Theme::at(39).paper,[0,0,0]);
    assert_eq!(Theme::at(0).ink,[0,0,0]);
    assert_eq!(Theme::at(0).keyline,Theme::at(0).paper);
    assert_eq!(Theme::at(4).keyline,[0,0,0]);
    assert_eq!(Theme::at(16).ink,[255,255,255]);
    assert_eq!(Theme::at_depth(7.5).ink,[120,120,120]);
    assert_eq!(Theme::at(39).secondary,[222,222,222]);
    assert_eq!(Theme::at(0).unread,[205,137,20]);
    assert_eq!(Theme::at(16).unread,[255,255,255]);
    assert_eq!(Theme::at(39).unread,[255,215,65]);
    for d in [0.5,4.5,12.5,15.5,20.5,30.5,38.5] {
        let low=Theme::at_depth(f64::floor(d));let high=Theme::at_depth(f64::ceil(d));let middle=Theme::at_depth(d);
        for (a,(b,m)) in colors(&low).into_iter().zip(colors(&high).into_iter().zip(colors(&middle))) {
            for k in 0..3 {assert!((2*m[k] as i16-a[k] as i16-b[k] as i16).abs()<=2,"depth {d}");}
        }
    }
    for invalid in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-5.] {assert_eq!(colors(&Theme::at_depth(invalid)),colors(&Theme::at(0)));}
    assert_eq!(colors(&Theme::at_depth(100.)),colors(&Theme::at(39)));
}

#[test]
fn text_and_unread_marks_keep_a_contrasting_edge_on_actual_row_backgrounds() {
    // A continuous black-to-white fill necessarily crosses the page luminance.
    // The smoothly colored keyline is part of the rendered glyph, not a claim
    // that fill-only WCAG contrast remains >=4.5 at that crossing.
    for sample in 0..=39_000 {
        let t=Theme::at_depth(sample as f64/1000.);
        for color in [t.ink,t.secondary] {
            assert!(outlined_contrast(color,t.paper,t.keyline)>=4.5,"page sample {sample}: {t:?}");
        }
        assert!(outlined_contrast(t.unread,t.paper,[0,0,0])>=4.5,"unread sample {sample}: {t:?}");
        for color in [t.selected_ink,t.selected_unread] {
            assert!(outlined_contrast(color,t.ink,[0,0,0])>=4.5,"selected sample {sample}: {t:?}");
        }
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
fn fixed_keyline_uses_supported_glyph_commands_without_contrast_sampling() {
    let mut p=Painter::new();p.gpu=Some(crate::gpu_render::Frame::default());p.pixels.fill(17);
    p.bestiary_prose("BESTIARY !",20,20,14,[128,128,128],false,[0,0,0]);
    assert!(p.pixels.iter().all(|&b|b==17));
    assert!(!p.prose_contrast_outline);
    let frame=p.gpu.as_ref().unwrap();assert!(frame.unsupported.is_none());assert!(!frame.quads.is_empty());
}

#[test]
fn render_fractional_transition_pages_for_native_review() {
    let Ok(directory)=std::env::var("CORPSE_BESTIARY_THEME_OUTPUT") else {return;};
    std::fs::create_dir_all(&directory).unwrap();
    for (width,height) in [(1280,800),(1920,1080),(900,1200)] {
        let mut p=Painter::new();p.try_resize(width,height).unwrap();p.set_responsive_canvas(false);
        let mut m=Manual::new();m.preview_all();
        let ids=m.ids().to_vec();m.progress.discover(ids.iter().map(String::as_str));
        for (id,depths) in [("blorb",vec![0.,5.99,6.,7.31,8.,12.,15.99,16.,16.13,18.,24.,39.]),
            ("zillennia:timeless",vec![17.99,18.,18.01,23.99,24.,24.01,27.99,28.,28.01,39.])] {
            assert!(m.select_id(id));
            for depth in depths {
                m.paint_at_visual_depth(&mut p,0.,depth);
                let path=format!("{directory}/{}-{width}x{height}-depth-{depth:.2}.png",id.replace(':',"-"));
                let file=std::fs::File::create(path).unwrap();let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
                encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
            }
        }
    }
}

#[test]
fn first_floor_keyline_preserves_original_text_weight_pixel_for_pixel() {
    let theme=Theme::at(0);
    for (w,h) in [(1280,800),(1920,1080),(900,1200)] {
        let mut plain=Painter::new();plain.try_resize(w,h).unwrap();
        let mut keyed=Painter::new();keyed.try_resize(w,h).unwrap();
        for size in [10,11,14,20,25,50] {
            for title in [false,true] {
                plain.clear(theme.paper);keyed.clear(theme.paper);
                plain.prose("The Flesh: BESTIARY 123",30,30,size,theme.ink,title);
                keyed.bestiary_prose("The Flesh: BESTIARY 123",30,30,size,theme.ink,title,theme.keyline);
                assert_eq!(plain.pixels,keyed.pixels,"{w}x{h} size {size} title {title}");
            }
        }
    }
}

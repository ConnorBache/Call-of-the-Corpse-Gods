//! Exact-output fused-background tests, including an independent two-pass control.
use super::super::*;

fn seed_pixels(p: &mut Painter, seed: usize) {
    for (i,v) in p.pixels.iter_mut().enumerate() {
        *v=i.wrapping_mul(37).wrapping_add(seed.wrapping_mul(61)) as u8;
    }
}

// Reference calls the untouched public clear and rect, not fused geometry helpers.
// The initial arbitrary alpha pattern makes unintended writes outside either
// primitive's true coverage visible in the whole-frame equality assertion.
fn compare(p: &mut Painter, rect:(i32,i32,i32,i32), base:Rgb, fog:Rgb, seed:usize) {
    seed_pixels(p,seed);
    p.raster_stats=damage::RasterStats {enabled:true,..Default::default()};
    p.clear(base);
    let clear_writes=p.raster_stats.pixel_writes;
    p.rect(rect.0,rect.1,rect.2,rect.3,fog);
    let old_writes=p.raster_stats.pixel_writes;
    let expected=p.pixels.clone();
    seed_pixels(p,seed);
    p.raster_stats=damage::RasterStats {enabled:true,..Default::default()};
    p.background_fused_enabled=true;
    p.clear_with_rect(base,rect,fog);
    assert_eq!(p.pixels,expected,
        "{}x{} responsive={} rect={rect:?} clip={:?} damage={:?} tiles={} seed={seed}",
        p.width,p.height,p.responsive_canvas,p.clip,p.damage_clip,p.tile_clip_enabled);
    assert_eq!(p.raster_stats.pixel_writes,clear_writes,"every clear-selected pixel written once");
    assert_eq!(p.raster_stats.pixel_visits,clear_writes);
    assert!(old_writes>=clear_writes);
}

/// Bounded native/fractional/portrait/odd-size matrix. O(cases * native pixels),
/// one expected RGBA framebuffer per case, plus existing tile-span metadata.
#[test]
fn fused_background_matches_clear_then_rect_with_all_clip_layers() {
    let mut p=Painter::new();
    for (width,height) in [(1280,800),(1920,1080),(3840,2160),(1279,803),(719,1201),(320,200)] {
        p.resize(width,height);
        for responsive in [false,true] {
            p.set_responsive_canvas(responsive);
            let (lw,lh)=(p.logical_width(),p.logical_height());
            let map_top=chronicle_height(&p.profile)+12;
            let game_rect=(0,map_top,
                p.hud_left()-p.profile.number("ui.layout.mapSpacing").round() as i32,
                (p.frame().map.1+p.frame().map.3-map_top).max(0));
            let middle=damage::Rect::new(width/5,height/7,width*3/5,height*4/7);
            // (logical clip, native damage, tile mode, logical rectangle).
            // Mode 0 ignores stale tiles; 1 is empty; 2 disjoint/overlapping bands;
            // 3 full native coverage. Nonzero logical strokes test min-one pixels.
            let cases=[
                (None,None,0,game_rect),
                (Some((-10,50,300,250)),Some(middle),2,game_rect),
                (Some((2,3,0,20)),None,3,game_rect),
                (Some((2,3,20,0)),None,0,game_rect),
                (Some((20,30,-3,50)),None,2,game_rect),
                (None,Some(damage::Rect::new(2,3,0,7)),0,game_rect),
                (None,Some(damage::Rect::new(width+3,height+5,7,11)),3,game_rect),
                (Some((5,7,lw-9,lh-13)),Some(middle),2,(3,4,0,40)),
                (None,None,0,(3,4,20,-2)),
                (Some((0,0,10,10)),None,2,(1,1,1,1)),
                (Some((-3,-4,lw+6,lh+8)),None,3,(-31,-5,lw+64,lh+32)),
                (Some((5,7,lw-9,lh-13)),Some(damage::Rect::new(width/2,height/2,width,height)),2,game_rect),
                (None,None,1,game_rect),
                (None,None,2,game_rect),
                (None,None,0,(lw+100,lh+100,30,40)),
            ];
            for (i,(clip,damage,tiles,rect)) in cases.into_iter().enumerate() {
                p.clip=clip; p.damage_clip=damage; p.tile_clip_enabled=tiles!=0;
                p.tile_mask.reset(width,height);
                match tiles {
                    2=>{
                        p.tile_mask.add(damage::Rect::new(0,1,width/3,height-2));
                        p.tile_mask.add(damage::Rect::new(width/2,0,width-width/2,height/2));
                        p.tile_mask.add(damage::Rect::new(width/4,height/3,width/2,height/3));
                    }
                    3=>p.tile_mask.add(damage::Rect::new(0,0,width,height)),
                    _=>{}
                }
                p.tile_mask.finish();
                let base=[8,(i as u8).wrapping_mul(17),14];
                let fog=if i==13 {base} else {[53,71,(i as u8).wrapping_mul(23)]};
                compare(&mut p,rect,base,fog,i);
            }
        }
        p.clip=None; p.damage_clip=None; p.tile_clip_enabled=false;
    }
}

/// Tiny drawables force many positive logical strokes into a single native
/// pixel. Exercise exact rect minimum-stroke and clip minimum-stroke rules.
#[test]
fn fused_background_preserves_subpixel_minimum_strokes() {
    let mut p=Painter::new();
    for (width,height) in [(1,1),(3,7),(17,23),(39,71)] {
        p.resize(width,height);
        for responsive in [false,true] {
            p.set_responsive_canvas(responsive);
            for clip in [None,Some((0,0,1,1)),Some((10,12,1,1)),Some((-4,-3,7,9))] {
                p.clip=clip;
                for rect in [(0,0,1,1),(1,1,1,1),(10,12,1,1),(-4,-3,7,9),(1280,800,1,1)] {
                    compare(&mut p,rect,[253,17,29],[7,143,241],29);
                }
            }
        }
    }
}

/// The independent test-only toggle selects the old path without changing
/// sparse ASCII controls or any production API/configuration.
#[test]
fn background_control_selects_the_original_two_pass_path() {
    let mut p=Painter::new(); p.resize(320,200); p.set_responsive_canvas(true);
    p.clip=Some((7,9,900,500));
    let rect=(-2,103,999,597);
    seed_pixels(&mut p,43);
    p.raster_stats=damage::RasterStats {enabled:true,..Default::default()};
    p.clear([8,6,14]); p.rect(rect.0,rect.1,rect.2,rect.3,[51,37,23]);
    let expected=p.pixels.clone(); let writes=p.raster_stats.pixel_writes;
    seed_pixels(&mut p,43);
    p.raster_stats=damage::RasterStats {enabled:true,..Default::default()};
    p.background_fused_enabled=false;
    p.clear_with_rect([8,6,14],rect,[51,37,23]);
    assert_eq!(p.pixels,expected);
    assert_eq!(p.raster_stats.pixel_writes,writes);
}

/// Integration oracle: exact final game pixels, not just the background helper.
/// One Painter reuses the same decoded assets; toggles differ only in background.
#[test]
fn complete_game_matches_with_fused_background() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let view=crate::observe::observe(&engine.run);
    let mut p=Painter::new();
    for (width,height) in [(1280,800),(1920,1080),(3840,2160)] {
        p.resize(width,height);
        for now in [0.,420.] {
            p.background_fused_enabled=false;
            p.game_at(&view,"Background parity",false,now);
            let expected=p.pixels.clone();
            p.background_fused_enabled=true;
            p.game_at(&view,"Background parity",false,now);
            assert_eq!(p.pixels,expected,"{width}x{height} time={now}");
        }
    }
}

/// Isolate background pixels from glyph coverage, then exercise actor movement
/// and changed lit terrain through the retained cache as well as full painting.
#[test]
fn player_background_matches_terrain_and_retained_repaints() {
    let run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut view = crate::observe::observe(&run);
    view.terrain_snapshot = None;
    let mut player = view.cells.iter().find(|c| c.entity_ink.is_some_and(|i| i.player)).unwrap().clone();
    player.glyph.clear();
    player.superscript.clear();
    player.subscript.clear();
    player.heartless = false;
    player.arrow.clear();
    let mut painter = Painter::new();
    let mut reference = Painter::new();
    let mut retained = super::super::RetainedGame::default();
    let mut cached = Painter::new();
    for (step, bg) in [[71, 94, 52], [21, 76, 130], [115, 82, 43], [3, 5, 9], [172, 185, 157]].into_iter().enumerate() {
        player.x = 10 + step;
        player.y = 10;
        player.bg = bg;
        view.cells = vec![Cell { x: player.x, y: player.y, bg, ..Cell::default() }, player.clone()];
        view.terrain_count = 1;
        let actor = view.cells.pop().unwrap();
        reference.game_at(&view, "", false, 0.);
        view.cells.push(actor);
        painter.game_at(&view, "", false, 0.);
        assert_eq!(painter.pixels, reference.pixels, "player must not paint a backing, frame {step}");
        // Reuse the previous frame without clearing: moving the player must
        // restore its old tile and changing light must replace the cached color.
        let damage = retained.update(&mut cached, &view, "", 0., true, false).unwrap();
        retained.restore(&mut cached, &damage).unwrap();
        assert_eq!(cached.pixels, reference.pixels, "retained frame {step}");
    }
}

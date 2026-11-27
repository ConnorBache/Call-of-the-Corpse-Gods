use super::*;
#[test]
fn responsive_extents_and_authored_sheets_use_their_own_native_scales(){
    let mut p=Painter::new();
    for (w,h) in [(1280,720),(1280,800),(1920,1080),(3840,2160),(1537,911),(960,1200)]{
        p.try_resize(w,h).unwrap();p.set_responsive_canvas(true);
        assert_eq!(p.origin,(0,0));
        assert_eq!(p.logical_width(),(w as f64/p.scale).ceil()as i32);
        assert_eq!(p.logical_height(),(h as f64/p.scale).ceil()as i32);
        assert_eq!(p.frame().sidebar,hud_left(&p.profile)+p.logical_width()-WIDTH as i32);
        assert_eq!(p.toolbar_y(),toolbar_y(&p.profile)+p.logical_height()-HEIGHT as i32);
        let scale=p.scale;let sheet_scale=canvas_scale(w,h,false);let dimensions=(p.width,p.height);let length=p.pixels.len();
        p.centered_sheet(|sheet|{assert_eq!((sheet.logical_width(),sheet.logical_height()),(1280,800));assert!((sheet.scale-sheet_scale*DIALOG_SCALE).abs()<1e-9);});
        assert_eq!(p.origin,(0,0));assert_eq!(p.scale,scale);assert_eq!((p.width,p.height),dimensions);assert_eq!(p.pixels.len(),length);
    }
}
#[test]
fn responsive_and_sheet_input_are_distinct_and_dpi_inverse(){
    let mut p=Painter::new();p.try_resize(1920,1080).unwrap();p.set_responsive_canvas(true);
    // Window points at half the drawable pixel resolution.
    let window=(960,540);let logical=(500,300);
    let screen=(((logical.0 as f64+0.5)*p.scale/2.)as i32,((logical.1 as f64+0.5)*p.scale/2.)as i32);
    let game=p.logical_point_for_canvas(screen.0,screen.1,window,true);
    assert!((game.0-logical.0).abs()<=2&&(game.1-logical.1).abs()<=2);
    let sheet=p.logical_point_for_canvas(screen.0,screen.1,window,false);
    assert!(sheet.0<game.0);assert!(sheet.1>game.1);
    let dialog=p.logical_point_for_sheet(screen.0,screen.1,window);
    p.centered_sheet(|s|{assert_eq!(s.logical_point(screen.0,screen.1,window),dialog);});
    assert_eq!(p.logical_point(screen.0,screen.1,window),game);
}
#[test]
fn rejected_and_uncommitted_resize_preserve_last_frame(){
    let mut p=Painter::new();p.pixels[0]=173;let size=(p.width,p.height);let ptr=p.pixels.as_ptr();
    assert!(p.prepare_resize(usize::MAX,2).is_err());assert!(p.prepare_resize(8192,8192).is_err());
    assert!(p.prepare_resize(0,800).unwrap().is_none());
    let prepared=p.prepare_resize(1440,900).unwrap().unwrap();
    assert_eq!((p.width,p.height),size);assert_eq!(p.pixels.as_ptr(),ptr);assert_eq!(p.pixels[0],173);
    // Models texture allocation refusal: discard preparation, never commit.
    drop(prepared);assert_eq!((p.width,p.height),size);assert_eq!(p.pixels[0],173);
    p.try_resize(1440,900).unwrap();assert_eq!((p.width,p.height),(1440,900));
    p.try_resize(1280,800).unwrap();assert_eq!((p.width,p.height),size);
}
#[test]
fn baseline_game_grid_fits_the_whole_floor_and_preserves_hud_hit_targets() {
    for (width, height) in [(1280, 720), (1920, 1080), (2560, 1440)] {
        let mut p = Painter::new();
        p.resize(width, height);
        p.set_responsive_canvas(true);
        assert_eq!((p.logical_width(), p.logical_height()), (1280, 720));
        let frame = p.frame();
        assert_eq!(frame.map, crate::modal_ui::Rect(0, 112, 736, 528));
        assert_eq!(frame.sidebar, 984);
        assert_eq!(frame.rail_bottom, 712);
        let (cw, ch, ox, oy) = p.map_layout(79, 29);
        assert_eq!((cw, ch), (9, 14));
        assert!(frame.map.contains(ox, oy));
        assert!(frame.map.contains(ox + 79*cw - 1, oy + 29*ch - 1));
        let window = (width as u32, height as u32);
        for (slot, action) in TOOLBAR_VISIBLE_ACTIONS.into_iter().enumerate() {
            let (x, w) = p.game_toolbar_rect(slot);
            let point = (p.native_x((x + w/2) as f64), p.native_y((p.toolbar_y()+20) as f64));
            let (hx, hy) = p.logical_point_for_canvas(point.0, point.1, window, true);
            assert_eq!(p.toolbar_action(hx, hy), Some(action));
        }
        assert_eq!(p.toolbar_action(frame.map.2, p.toolbar_y()+20), None);
        let top=crate::ability_ui::top(&p.profile);
        let row=crate::ability_ui::row(&p.profile);
        let count=p.ability_capacity();
        assert!(top + row*count as i32 <= frame.rail_bottom-88);
        assert_eq!(p.ability_inspect_at(100, frame.sidebar+20, top+row*count as i32),None);
    }
}
#[test]
fn queued_screen_switches_use_selected_canvas_without_resize(){
    let mut p=Painter::new();p.try_resize(1537,911).unwrap();
    let dimensions=(p.width,p.height);let window=(1537,911);
    let sheet=p.logical_point_for_canvas(900,450,window,false);
    let game=p.logical_point_for_canvas(900,450,window,true);
    assert_ne!(sheet,game);
    for responsive in [false,true,false,true]{p.set_responsive_canvas(responsive);assert_eq!(p.logical_point(900,450,window),if responsive{game}else{sheet});assert_eq!((p.width,p.height),dimensions);}
}
#[test]
fn slender_supported_canvas_remains_bounded_and_refusal_keeps_origin(){
    let mut p=Painter::new();p.try_resize(1,8192).unwrap();p.set_responsive_canvas(false);
    let prior=(p.width,p.height,p.origin,p.scale,p.pixels.len());
    assert!(p.prepare_resize(1,33_554_432).is_err());
    assert_eq!((p.width,p.height,p.origin,p.scale,p.pixels.len()),prior);
    p.set_responsive_canvas(true);assert!(p.logical_height()>800);assert!(p.logical_height()<i32::MAX/4);
}

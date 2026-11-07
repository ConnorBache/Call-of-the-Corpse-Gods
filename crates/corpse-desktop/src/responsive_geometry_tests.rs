use super::*;
#[test]
fn responsive_extents_and_authored_sheets_share_native_scale(){
    let mut p=Painter::new();
    for (w,h) in [(1280,800),(1920,1080),(3840,2160),(1537,911),(960,1200)]{
        p.try_resize(w,h).unwrap();p.set_responsive_canvas(true);
        assert_eq!(p.origin,(0,0));
        assert_eq!(p.logical_width(),(w as f64/p.scale).ceil()as i32);
        assert_eq!(p.logical_height(),(h as f64/p.scale).ceil()as i32);
        assert_eq!(p.frame().sidebar,hud_left(&p.profile)+p.logical_width()-WIDTH as i32);
        assert_eq!(p.toolbar_y(),toolbar_y(&p.profile)+p.logical_height()-HEIGHT as i32);
        let scale=p.scale;let dimensions=(p.width,p.height);let length=p.pixels.len();
        p.centered_sheet(|sheet|{assert_eq!((sheet.logical_width(),sheet.logical_height()),(1280,800));assert!((sheet.scale-scale*DIALOG_SCALE).abs()<1e-9);});
        assert_eq!(p.origin,(0,0));assert_eq!((p.width,p.height),dimensions);assert_eq!(p.pixels.len(),length);
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
    assert!(sheet.0<game.0);assert_eq!(sheet.1,game.1);
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
fn baseline_canvas_keeps_original_layout(){
    let mut p=Painter::new();p.set_responsive_canvas(true);
    assert_eq!(p.frame(),crate::modal_ui::Frame::from_profile(&p.profile));
    assert_eq!(p.map_layout(79,29),map_layout_profile(79,29,&p.profile));
    assert_eq!(p.toolbar_y(),toolbar_y(&p.profile));
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

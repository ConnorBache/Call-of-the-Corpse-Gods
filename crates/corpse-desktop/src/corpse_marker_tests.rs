use super::*;
// Existing isolated marker tests obtain real glyph geometry without touching
// their CPU pixels or replacing an active command recorder.
pub(super) fn marker_test_bounds(p: &mut Painter) -> Option<glyph_ink::Bounds> {
    let previous=p.gpu.take();p.gpu=Some(crate::gpu_render::Frame::default());
    let bounds=p.map_identity_aligned("B",40,50,15,24,[255;3],false);
    p.gpu=previous;bounds
}

#[test]
fn corpse_cross_preserves_glyph_and_observed_death_semantics() {
    let mut r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before=serde_json::to_string(&r).unwrap();let mut v=crate::observe::observe(&r);
    assert!(!v.cells.iter().find(|c|c.actor_id==Some(r.player.id)).unwrap().corpse);
    let id=r.actors_at(r.depth)[0].id;let a=r.actors_at_mut(r.depth).iter_mut().find(|a|a.id==id).unwrap();
    a.dead=true.into();a.hp=0.;a.heartless=true.into();
    v=crate::observe::observe(&r);
    let corpse=v.cells.iter().find(|c|c.actor_id==Some(id)).expect("visible corpse");
    assert!(corpse.corpse);assert!(!corpse.heartless);assert!(!corpse.glyph.is_empty());
    assert!(!v.creatures.iter().any(|a|a.id==id),"dead creatures do not become live previews");
    let frozen=serde_json::to_string(&r).unwrap();let mut p=Painter::new();p.game_at(&v,"",false,0.);
    assert_eq!(serde_json::to_string(&r).unwrap(),frozen);assert_ne!(before,frozen);
}
#[test]
fn corpse_cross_is_clipped_and_gpu_recording_does_not_paint_cpu_pixels() {
    for scale in [1.,1.5,2.] {
        let mut p=Painter::new();p.resize((1280.*scale)as usize,(800.*scale)as usize);p.clear([10,20,30]);let before=p.pixels.clone();
        { let bounds=corpse_marker_tests::marker_test_bounds(&mut p); p.corpse_marker(bounds,40,50,15,24,None); };
        let bounds=(p.native_x(40.),p.native_y(50.),p.native_x(55.),p.native_y(74.));let mut changed=0;
        for (i,(a,b)) in p.pixels.chunks_exact(4).zip(before.chunks_exact(4)).enumerate(){if a!=b {changed+=1;let(x,y)=((i%p.width)as i32,(i/p.width)as i32);assert!(x>=bounds.0&&x<bounds.2&&y>=bounds.1&&y<bounds.3);}}
        assert!(changed>0);
        p.gpu=Some(crate::gpu_render::Frame::default());p.pixels.fill(17);{ let bounds=corpse_marker_tests::marker_test_bounds(&mut p); p.corpse_marker(bounds,40,50,15,24,None); };
        assert!(p.pixels.iter().all(|&b|b==17));assert!(!p.gpu.as_ref().unwrap().quads.is_empty());
    }
}
#[test]
fn corpse_toggle_repairs_retained_and_tile_frames() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut v=crate::observe::observe(&engine.run);let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    retained.update(&mut p,&v,"",0.,true,true).unwrap();
    let cell=v.cells.iter_mut().find(|c|c.actor_id==Some(engine.run.player.id)).unwrap();cell.corpse=true;cell.heartless=false;
    retained.update(&mut p,&v,"",0.,true,false).unwrap();full.game_at(&v,"",false,0.);assert_eq!(p.pixels,full.pixels);
    let old=crate::observe::observe(&engine.run);let mut plan=tile_damage::TilePlanner::default();let mut p=Painter::new();
    plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
    plan.replace_view(Some(&old),&v,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;assert_eq!(p.pixels,full.pixels);
}

fn color_room(revision: f64) -> corpse_core::Run {
    let mut r = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.stats.perception = 300.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = corpse_core::Field::Missing;
    r.player.equipment_abilities = corpse_core::Field::Missing;
    r.actors.clear(); r.ground.clear(); r.inventory.clear(); r.history.clear();
    r.actors.insert("1".into(),std::sync::Arc::new(Vec::new()));
    r.ground.insert("1".into(),std::sync::Arc::new(Vec::new()));
    let w = 79; let h = 29;
    let mut tiles = vec!["floor".to_owned(); w*h];
    for y in 0..h { tiles[y*w]="wall".into(); tiles[y*w+w-1]="wall".into(); }
    for x in 0..w { tiles[x]="wall".into(); tiles[(h-1)*w+x]="wall".into(); }
    r.floors = vec![std::sync::Arc::new(corpse_core::Floor {
        depth:1., width:(w as f64).into(), height:(h as f64).into(), domain:"Dungeon".into(),
        tiles:tiles.clone(), remembered_tiles:tiles.into(), seen:vec![true;w*h],
        lights:vec![0.;w*h], substrates:vec!["rock".into();w*h].into(), ..Default::default()
    })];
    r.next_id=1000.;
    corpse_core::storage::validate(&r).expect("valid color-room fixture");
    r
}
fn shown_actor(v: &View, id: f64) -> &Cell {
    v.cells.iter().find(|c|c.actor_id==Some(id)).unwrap_or_else(||panic!("visible actor {id}"))
}

#[test]
fn corpse_covers_its_heart_and_gets_circle_only_after_stomp_and_pickup() {
    let mut r = color_room(52.);
    let mut a = corpse_core::population::create_actor_revision("rat",80.,13.,10.,false,52.).unwrap();
    a.hp = 0.;
    r.actors_at_mut(1.).push(a);
    corpse_core::engine::death_if_nonpositive(&mut r,80.).unwrap();
    let heart_id = r.ground["1"][0].item.id;
    let before = crate::observe::observe(&r);
    let corpse = shown_actor(&before,80.);
    assert!(corpse.corpse && !corpse.heartless);
    assert!(!before.cells.iter().any(|c|c.ground_item_id == Some(heart_id.to_bits())));
    assert!(before.ground_entities.iter().any(|g|g.id == heart_id.to_bits()));

    r.player.x = 12.;
    let player = r.player.id;
    assert!(corpse_core::engine::move_actor(&mut r,player,1.,0.,false).unwrap());
    assert!(!corpse_core::corpses::heart_harvested(corpse_core::engine::actor(&r,80.).unwrap()));
    assert!(r.ground["1"].iter().any(|g|g.item.id == heart_id));
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r,10.).unwrap(),1);
    assert!(r.inventory.iter().any(|i|i.id == heart_id));
    assert!(corpse_core::engine::move_actor(&mut r,player,-1.,0.,false).unwrap());
    let wire = corpse_core::storage::serialize(&r).unwrap();
    let loaded = corpse_core::storage::deserialize(&wire).unwrap();
    let view = crate::observe::observe(&loaded);
    let corpse = shown_actor(&view,80.);
    assert!(corpse.corpse && corpse.heartless);
    let mut p = Painter::new();
    p.resize(2560,1600);
    p.game_at(&view,"",false,0.);
    let (cw,ch,ox,oy)=p.map_layout(view.width,view.height);
    let (x,y)=(ox+corpse.x as i32*cw,oy+corpse.y as i32*ch);
    let crop = (p.native_x(x as f64),p.native_y(y as f64),
        p.native_x((x+cw) as f64),p.native_y((y+ch) as f64));
    let is_rim = |c: &[u8]| c[0] > 150 && c[1] > 130 && c[2] > 100;
    let mut dark_hole = false;
    for yy in crop.1..crop.3 {
        let pixel = |xx| &p.pixels[((yy as usize*p.width + xx as usize)*4)..][..3];
        let rim: Vec<_> = (crop.0..crop.2).filter(|&xx| is_rim(pixel(xx))).collect();
        dark_hole |= rim.first().zip(rim.last()).is_some_and(|(&a,&b)|
            (a+1..b).any(|xx| pixel(xx)[0] < 65));
    }
    assert!(dark_hole, "the ring hole remains legible in the normal map layout");
    if std::env::var_os("CORPSE_COLOR_OUTPUT").is_some() {
        write_corpse_capture("Heartless_Corpse", &p);
        println!("HEARTLESS_CORPSE_CROP={},{},{},{}", crop.0,crop.1,crop.2,crop.3);
    }
}

#[test]
fn corpse_heartless_circle_overlays_the_cross_at_its_native_center() {
    let mut p = Painter::new();
    for (width, height) in [(1280,800), (1537,911), (1920,1200), (2560,1600)] {
        p.resize(width,height);
        for glyph in ["r", "^", "Ω", "^^", " "] {
            p.clear([0;3]);
            let bounds = p.map_identity_aligned(glyph,100,100,30,48,[255;3],false);
            p.clear([0;3]);
            let center = p.corpse_marker(bounds,100,100,30,48,None);
            let rect = native_test_rect(&p);
            let cross = pixel_ink_bounds(&p,rect,true).unwrap();
            p.corpse_heartless_marker(center,100,100,30,48,None);
            assert_eq!(pixel_ink_bounds(&p,rect,true),Some(cross),"ring leaves all four red arms");
            let marked = p.pixels.clone();
            p.clear([0;3]);
            p.corpse_heartless_marker(center,100,100,30,48,None);
            let ring = pixel_ink_bounds(&p,rect,false).unwrap();
            assert!(cross.left < ring.left && ring.right < cross.right);
            assert!(cross.top < ring.top && ring.bottom < cross.bottom);
            assert!((ring.center().0-center.0).abs()<=0.5 && (ring.center().1-center.1).abs()<=0.5);
            let at = (center.1 as usize*p.width+center.0 as usize)*4;
            assert_eq!(&marked[at..at+3],&[24,23,21],"dark center covers the X intersection");
        }
    }
}
fn write_corpse_capture(name: &str, p: &Painter) {
    let Ok(dir)=std::env::var("CORPSE_COLOR_OUTPUT") else {return;};
    std::fs::create_dir_all(&dir).unwrap();
    let f=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut png=png::Encoder::new(f,p.width as u32,p.height as u32);
    png.set_color(png::ColorType::Rgba);png.set_depth(png::BitDepth::Eight);
    png.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}

#[test]
fn corpse_color_survives_actual_death_save_and_revival_without_new_fields() {
    for revision in [29.,52.] {
        let mut r=color_room(revision);
        let mut a=corpse_core::population::create_actor_revision("rat",80.,13.,10.,false,revision).unwrap();
        a.color="#3bcda6".into();a.heartless=true.into();
        r.actors_at_mut(1.).push(a);
        let live=crate::observe::observe(&r);
        assert_eq!(shown_actor(&live,80.).entity_ink.unwrap().color,[59,205,166]);
        r.actors_at_mut(1.)[0].hp=0.;
        corpse_core::engine::death_if_nonpositive(&mut r,80.).unwrap();
        let dead=&r.actors_at(1.)[0];
        assert!(dead.dead.value_or(false));assert_eq!(dead.color,"#3bcda6");
        let wire=corpse_core::storage::serialize(&r).unwrap();
        let mut loaded=corpse_core::storage::deserialize(&wire).unwrap();
        assert_eq!(loaded.actors_at(1.)[0].color,"#3bcda6");
        let before=corpse_core::storage::serialize(&loaded).unwrap();
        let view=crate::observe::observe(&loaded);let corpse=shown_actor(&view,80.);
        assert!(corpse.corpse);assert!(!corpse.heartless);
        assert_eq!(corpse.entity_ink.unwrap().color,shown_actor(&live,80.).entity_ink.unwrap().color);
        assert_eq!(corpse.glyph,shown_actor(&live,80.).glyph);
        assert_eq!(corpse_core::storage::serialize(&loaded).unwrap(),before,"observation never rewrites a legacy save");
        let tick=loaded.tick;
        corpse_core::corpses::revive_corpse(&mut loaded.actors_at_mut(1.)[0],tick);
        assert!(!loaded.actors_at(1.)[0].dead.value_or(false));
        assert_eq!(loaded.actors_at(1.)[0].color,"#3bcda6");
        let revived=crate::observe::observe(&loaded);
        assert!(!shown_actor(&revived,80.).corpse);
        assert_eq!(shown_actor(&revived,80.).entity_ink.unwrap().color,[59,205,166]);
    }
}

#[test]
fn corpse_color_follows_saved_variants_and_living_creature_profile() {
    use serde_json::json;
    let mut r=color_room(52.);
    let mut rat=corpse_core::population::create_actor_revision("rat",80.,13.,10.,false,52.).unwrap();
    rat.color="#3bcda6".into();r.actors_at_mut(1.).push(rat);
    let profile=crate::appearance_profile::Profile::default()
        .set("entities.creatures.rat.inherited",json!(false)).unwrap()
        .set("entities.creatures.rat.color",json!("#e0a030")).unwrap()
        .set("entities.creatures.rat.glyph",json!("r")).unwrap()
        .set("entities.objects.corpse.inherited",json!(false)).unwrap()
        .set("entities.objects.corpse.color",json!("#9933ff")).unwrap()
        .set("entities.objects.corpse.glyph",json!("!")).unwrap();
    let live=crate::observe::observe_with_profile(&r,&profile);
    r.actors_at_mut(1.)[0].dead=true.into();r.actors_at_mut(1.)[0].hp=0.;
    let dead=crate::observe::observe_with_profile(&r,&profile);
    let (a,b)=(shown_actor(&live,80.),shown_actor(&dead,80.));
    assert_eq!(a.entity_ink.unwrap().color,[224,160,48]);
    assert_eq!(b.entity_ink.unwrap().color,a.entity_ink.unwrap().color);
    assert_eq!(a.glyph,b.glyph);assert_eq!(b.glyph,"r");
    assert_eq!(r.actors_at(1.)[0].color,"#3bcda6","profile never overwrites the individual variant");
}

#[test]
fn corpse_cross_is_red_large_and_still_preserves_visible_glyph_ink() {
    for scale in [1.,1.5,2.] {
        let mut p=Painter::new();p.resize((1280.*scale)as usize,(800.*scale)as usize);
        p.clear([10,20,30]);{ let bounds=corpse_marker_tests::marker_test_bounds(&mut p); p.corpse_marker(bounds,40,50,15,24,None); };
        let mut red=Vec::new();
        for (i,c) in p.pixels.chunks_exact(4).enumerate() {
            if c[0]>c[1].saturating_add(45) && c[0]>c[2].saturating_add(45) {
                red.push(((i%p.width)as i32,(i/p.width)as i32));
            }
        }
        let lo=red.iter().map(|p|p.0).min().unwrap();let hi=red.iter().map(|p|p.0).max().unwrap();
        assert!((hi-lo)as f64 >= 7.*scale,"red X spans most of the tile at scale {scale}");
        assert!((hi-lo)as f64 <= 12.*scale,"cross is still clipped within tile");
        p.clear([10,20,30]);p.map_identity("B",40,50,15,24,[40,220,110]);
        let before=p.pixels.clone();{ let bounds=corpse_marker_tests::marker_test_bounds(&mut p); p.corpse_marker(bounds,40,50,15,24,None); };
        assert!(p.pixels.chunks_exact(4).zip(before.chunks_exact(4)).any(|(a,b)|a==b&&a[1]>a[0]+40&&a[1]>a[2]+40),"the color identity must remain visible around the X");
    }
}

#[test]
fn corpse_variants_and_legacy_save_have_native_color_captures() {
    for revision in [29.,52.] {
        let mut r=color_room(revision);
        // Center the viewer between all pairs. Legacy sight is probability-based;
        // the vampire at x=25 was below its 50% threshold from x=10.
        r.player.x=19.;
        for (i,kind) in ["rat","dryad","hemogoblin","dire-bear","vampire"].iter().enumerate() {
            let mut a=corpse_core::population::create_actor_revision(kind,100.+i as f64*2.,13.+i as f64*3.,8.,false,revision).unwrap();
            if i==0 {a.color="#3bcda6".into();} // Individual saved variant, not a species guess.
            if i==4 {a.conditions.insert("monsterVampireBat".into(),1.);}
            let mut corpse=a.clone();corpse.id+=1.;corpse.y=11.;corpse.dead=true.into();corpse.hp=0.;corpse.heartless=true.into();
            r.actors_at_mut(1.).extend([a,corpse]);
        }
        // The ordinary loader exercises legacy-format and current saves, neither
        // needs a new corpse-color field nor a guessed historical per-actor color.
        let wire=corpse_core::storage::serialize(&r).unwrap();
        r=corpse_core::storage::deserialize(&wire).unwrap();
        let frozen=corpse_core::storage::serialize(&r).unwrap();
        for a in r.actors_at(1.) {
            assert!(corpse_core::vision::directly_visible_actor(&r,a),"capture actor {} ({}) is visible in revision {revision}",a.id,a.name);
        }
        let view=crate::observe::observe(&r);
        for i in 0..5 {
            let live=shown_actor(&view,100.+i as f64*2.);let dead=shown_actor(&view,101.+i as f64*2.);
            assert_eq!(live.glyph,dead.glyph);assert_eq!(live.entity_ink.unwrap().color,dead.entity_ink.unwrap().color);
            assert_eq!(live.entity_ink.unwrap().identity_outline,dead.entity_ink.unwrap().identity_outline);
            assert!(!live.corpse&&dead.corpse);
        }
        for (w,h) in [(1280,800),(1920,1200)] {
            let mut p=Painter::new();p.resize(w,h);p.game_at(&view,"",false,0.);
            write_corpse_capture(&format!("corpse-variants-revision{revision}-{w}x{h}"),&p);
        }
        assert_eq!(corpse_core::storage::serialize(&r).unwrap(),frozen);
    }
}

#[test]
fn corpse_hallucinations_keep_projected_color_and_hide_actual_variants() {
    let mut r=color_room(52.);
    let mut a=corpse_core::population::create_actor_revision("rat",80.,13.,10.,false,52.).unwrap();
    a.color="#123456".into();a.dead=true.into();a.hp=0.;r.actors_at_mut(1.).push(a);
    r.player.conditions.insert("hallucinating".into(),10.);
    let before=corpse_core::storage::serialize(&r).unwrap();
    let view=crate::observe::observe(&r);let cell=shown_actor(&view,80.);
    assert!(cell.corpse);assert!(cell.hallucination.is_some());
    for phase in 0..crate::hallucination_ui::VARIANTS {
        let projected=crate::creature_presentation::project_kind(&r,&r.actors_at(1.)[0],crate::hallucination_ui::kind(&r,&r.actors_at(1.)[0],phase)).unwrap();
        let variant=&cell.hallucination.as_ref().unwrap()[phase];
        assert_eq!(variant.entity_ink.unwrap().color,crate::color::hex(&projected.color));
    }
    assert_eq!(corpse_core::storage::serialize(&r).unwrap(),before);
    r.actors_at_mut(1.)[0].color="#ffeedd".into();
    assert_eq!(view.cells,crate::observe::observe(&r).cells,"hidden true color cannot leak through hallucination");
}

fn center_profile(font: &str, custom: bool) -> crate::appearance_profile::Profile {
    use serde_json::json;
    let mut p=crate::appearance_profile::Profile::default();
    if !font.is_empty() {p=p.set("typography.map.fontFamily",json!(font)).unwrap();}
    if custom {
        for (key,value) in [("glyphScale",0.84),("fontSize",20.),("lineHeight",1.15),
            ("alignX",2.),("alignY",-2.),("letterSpacing",0.7),("fontWeight",700.),
            ("tileWidth",18.),("tileHeight",26.)] {
            p=p.set(&format!("typography.map.{key}"),json!(value)).unwrap();
        }
    }
    p
}

// Independent pixel oracle, never reads glyph cache bounds or marker geometry.
fn pixel_ink_bounds(p: &Painter, rect:(i32,i32,i32,i32), red_only:bool) -> Option<glyph_ink::Bounds> {
    let mut result=None;
    for y in rect.1.max(0)..rect.3.min(p.height as i32) {
        for x in rect.0.max(0)..rect.2.min(p.width as i32) {
            let at=(y as usize*p.width+x as usize)*4;let c=&p.pixels[at..at+3];
            let ink=if red_only {c[0]>c[1].saturating_add(35)&&c[0]>c[2].saturating_add(35)} else {c.iter().any(|&v|v!=0)};
            if ink {glyph_ink::Bounds::extend(&mut result,glyph_ink::Bounds{left:x,top:y,right:x+1,bottom:y+1});}
        }
    }
    result
}
fn native_test_rect(p: &Painter) -> (i32,i32,i32,i32) {
    (p.native_x(70.),p.native_y(70.),p.native_x(180.),p.native_y(180.))
}

#[test]
fn corpse_center_geometry_matches_actual_ink_across_fonts_profiles_and_dpi() {
    let mut p=Painter::new();let mut cases=0;
    for (width,height) in [(960,600),(1280,800),(1537,911),(1920,1200),(2560,1600)] {
        p.resize(width,height);
        for (font,custom) in [("",false),("IBM Plex Mono",false),("Inconsolata",true),("",true)] {
            p.profile=center_profile(font,custom);
            for glyph in ["r","B","g","^","y","i",".","Ω","♣","人","^^","Wi","(≬)","Ωi"," "] {
                for (w,h) in [(15,24),(30,48)] {
                    p.clear([0;3]);
                    let bounds=p.map_identity_aligned(glyph,100,100,w,h,[255;3],false);
                    assert_eq!(bounds,pixel_ink_bounds(&p,native_test_rect(&p),false),"actual pixel bounds: {glyph:?} {font} custom={custom} {width}x{height} {w}x{h}");
                    // Zero alpha, black glyphs and outlines must not alter attachment geometry.
                    p.gpu=Some(crate::gpu_render::Frame::default());
                    let gpu=p.map_identity_aligned(glyph,100,100,w,h,[0;3],false);
                    assert_eq!(bounds,gpu);p.gpu=None;
                    p.clear([0;3]);p.map_identity_outline(glyph,100,100,w,h,[0;3]);
                    let count=(p.map_masks.len(),p.identity_masks.len());
                    let black=p.map_identity_aligned(glyph,100,100,w,h,[0;3],false);
                    assert_eq!(bounds,black);assert_eq!(count,(p.map_masks.len(),p.identity_masks.len()));
                    cases+=1;
                }
            }
        }
    }
    // Ω uses the existing proportional serif fallback; no new face-selection rule.
    assert_ne!(p.fallback[4].metrics('W',30.).advance_width,p.fallback[4].metrics('i',30.).advance_width);
    assert_ne!(p.fallback[4].lookup_glyph_index('Ω'),0);
    assert_eq!(cases,600);
}

#[test]
fn corpse_center_italic_and_bottom_aligned_bounds_match_painted_ink() {
    let mut p=Painter::new();
    for (width,height) in [(1280,800),(1537,911),(2560,1600)] {
        p.resize(width,height);
        for glyph in ['0','g','^'] {
            p.clear([0;3]);
            let b=p.map_glyph_placed(glyph,100,100,21,32,[255;3],1.,true,None);
            assert_eq!(b,pixel_ink_bounds(&p,native_test_rect(&p),false));
        }
        for glyph in ["\"","10","Ω"] {
            p.clear([0;3]);let b=p.map_identity_aligned(glyph,100,100,30,48,[255;3],true);
            assert_eq!(b,pixel_ink_bounds(&p,native_test_rect(&p),false));
        }
        for glyph in ["g","Ωi"] {
            let opaque=p.map_identity_alpha_placed(glyph,100,100,30,48,[255;3],1.,None);
            let transparent=p.map_identity_alpha_placed(glyph,100,100,30,48,[0;3],0.,None);
            assert_eq!(opaque,transparent,"opacity/color do not affect ink attachment");
        }
    }
}

#[test]
fn corpse_center_cross_pixels_are_symmetric_about_glyph_not_tile() {
    let mut p=Painter::new();let mut differs_from_tile=0;
    for (width,height) in [(960,600),(1280,800),(1537,911),(1920,1200),(2560,1600)] {
        p.resize(width,height);
        for custom in [false,true] {
            p.profile=center_profile("",custom);
            for glyph in ["r","g","^","B","Ω","^^","(≬)"] {
                p.clear([0;3]);
                let bounds=p.map_identity_aligned(glyph,100,100,30,48,[255;3],false).unwrap();
                let measured=pixel_ink_bounds(&p,native_test_rect(&p),false).unwrap();
                assert_eq!(bounds,measured);
                p.clear([0;3]);p.corpse_marker(Some(bounds),100,100,30,48,None);
                let red=pixel_ink_bounds(&p,native_test_rect(&p),true).unwrap();
                assert_eq!(red.center(),measured.center(),"red extent centered on native glyph ink: {glyph} {width}x{height} custom={custom}");
                let rect=native_test_rect(&p);let (cx,cy)=measured.center();
                for y in red.top..red.bottom {for x in red.left..red.right {
                    let mirrored_x=(2.*cx)as i32-1-x;let mirrored_y=(2.*cy)as i32-1-y;
                    let a=(y as usize*p.width+x as usize)*4;let b=(mirrored_y as usize*p.width+mirrored_x as usize)*4;
                    assert_eq!(&p.pixels[a..a+4],&p.pixels[b..b+4],"X has rotational symmetry about actual ink {glyph} {rect:?}");
                }}
                let before=p.pixels.clone();p.clear([0;3]);p.corpse_marker(None,100,100,30,48,None);
                if p.pixels!=before {differs_from_tile+=1;}
            }
        }
    }
    assert!(differs_from_tile>35,"reject the previous tile-center implementation");
}

#[test]
fn corpse_center_clip_gpu_and_blank_fallback_preserve_anchor() {
    let mut p=Painter::new();p.resize(1537,911);p.profile=center_profile("IBM Plex Mono",true);
    for glyph in ["g","Ωi","(≬)"] {
        p.clip=None;p.clear([0;3]);
        let bounds=p.map_identity_aligned(glyph,100,100,30,48,[255;3],false);
        p.clear([0;3]);p.corpse_marker(bounds,100,100,30,48,None);let full=p.pixels.clone();
        for clip in [(108,100,22,48),(100,113,30,35),(108,113,12,12)] {
            p.clear([0;3]);p.clip=Some(clip);
            let clipped=p.map_identity_aligned(glyph,100,100,30,48,[0;3],false);
            assert_eq!(clipped,bounds,"clipping cannot re-center the surviving fragment");
            p.corpse_marker(clipped,100,100,30,48,None);
            let (left,top,right,bottom)=p.native_bounds();
            for y in 80..180 {for x in 80..180 {
                let at=(y*p.width+x)*4;
                assert_eq!(&p.pixels[at..at+3],if (left..right).contains(&(x as i32))&&(top..bottom).contains(&(y as i32)){&full[at..at+3]}else{&[0;3]},"clip crop {glyph}");
            }}
        }
        p.clip=None;p.clear([0;3]);p.gpu=Some(crate::gpu_render::Frame::default());
        let gpu_bounds=p.map_identity_aligned(glyph,100,100,30,48,[255;3],false);
        assert_eq!(gpu_bounds,bounds);p.gpu.as_mut().unwrap().begin();
        p.corpse_marker(gpu_bounds,100,100,30,48,None);
        assert!(p.pixels.chunks_exact(4).all(|v|v[..3]==[0;3]));
        // Replay only existing native-pixel marker quads through the native CPU primitive.
        // This checks command recording, not accelerated GPU hardware readback.
        let frame=p.gpu.take().unwrap();
        for q in frame.quads {
            assert!(q.mask.is_none());assert_eq!((q.rect.2-q.rect.0,q.rect.3-q.rect.1),(1,1));
            p.native_pixel(q.rect.0,q.rect.1,[q.color[0],q.color[1],q.color[2]],q.color[3]);
        }
        assert_eq!(p.pixels,full,"GPU marker commands match software pixels");
    }
    p.profile=center_profile("",false);p.clear([0;3]);
    let blank=p.map_identity_aligned(" ",100,100,30,48,[255;3],false);assert_eq!(blank,None);
    p.corpse_marker(blank,100,100,30,48,None);
    let red=pixel_ink_bounds(&p,native_test_rect(&p),true).unwrap().center();
    let old=(p.origin.0 as f64+115.*p.scale,p.origin.1 as f64+124.*p.scale);
    assert!((red.0-old.0).abs()<=0.5&&(red.1-old.1).abs()<=0.5,"blank retains old tile-centered death indication");
}

#[test]
fn corpse_center_main_and_effect_repaint_use_actual_glyph_geometry() {
    let mut r=color_room(52.);let mut a=corpse_core::population::create_actor_revision("rat",80.,13.,10.,false,52.).unwrap();
    a.dead=true.into();a.hp=0.;r.actors_at_mut(1.).push(a);
    let frozen=corpse_core::storage::serialize(&r).unwrap();
    // Real actor layers never inherit terrain animations, including a saved
    // custom '~' corpse on animated water and its hallucinated projections.
    for revision in [29.,52.] {
        let mut water=r.clone();water.simulation_revision=revision.into();
        water.floor_mut().unwrap().tiles[10*79+13]="water".into();
        water.floor_mut().unwrap().remembered_tiles.as_mut().unwrap()[10*79+13]="water".into();
        let profile=center_profile("",false)
            .set("entities.creatures.rat.inherited",serde_json::json!(false)).unwrap()
            .set("entities.creatures.rat.glyph",serde_json::json!("~")).unwrap();
        let view=crate::observe::observe_with_profile(&water,&profile);
        let corpse=shown_actor(&view,80.);
        assert!(corpse.corpse);assert_eq!(corpse.glyph,"~");assert!(corpse.animations.is_empty());
        assert!(view.cells.iter().any(|c|c.x==13&&c.y==10&&c.actor_id.is_none()
            &&c.animations.iter().any(|a|matches!(a,Animation::Water{..}))),"water animation stays on its terrain layer, revision {revision}");
        water.player.conditions.insert("hallucinating".into(),10.);
        let hallucinated=crate::observe::observe_with_profile(&water,&profile);
        let corpse=shown_actor(&hallucinated,80.);assert!(corpse.animations.is_empty());
        if let Some(variants)=&corpse.hallucination {assert!(variants.iter().all(|c|c.animations.is_empty()));}
    }
    for glyph in ["r","^","Ω","^^","(≬)"] {for custom in [false,true] {
        let mut p=Painter::new();p.resize(1537,911);p.profile=center_profile("",custom);
        let mut v=crate::observe::observe_with_profile(&r,&p.profile);v.terrain_snapshot=None;
        let cell=v.cells.iter_mut().find(|c|c.actor_id==Some(80.)).unwrap();
        cell.glyph=glyph.into();cell.display_light=None;cell.fg=[0,220,120];cell.bg=[0;3];cell.emphasis=1.;cell.superscript.clear();cell.subscript.clear();
        let cell=cell.clone();p.game_at(&v,"",false,0.);
        let (cw,ch,ox,oy)=p.map_layout(v.width,v.height);let (x,y)=(ox+cell.x as i32*cw,oy+cell.y as i32*ch);
        let rect=(p.native_x(x as f64),p.native_y(y as f64),p.native_x((x+cw)as f64),p.native_y((y+ch)as f64));
        let base=pixel_ink_bounds(&p,rect,true).unwrap();
        p.clear([0;3]);
        let cue=crate::effects::Cue{event:corpse_core::events::PresentationEvent{kind:"status".into(),from:None,to:corpse_core::events::EventPoint{x:13.,y:10.},glyph:None,color:Some("#000000".into()),label:None,depth:v.depth,animation:None},started:0.,held:false,static_marks:false};
        p.effects(&v,&[cue],50.,true);
        let effect=pixel_ink_bounds(&p,rect,true).unwrap();assert_eq!(base,effect,"main/effects attachment {glyph} custom={custom}");
        p.clear([0;3]);let ink=p.map_identity_aligned(glyph,x,y,cw,ch,[255;3],false).unwrap();
        let (cx,cy)=ink.center();
        // Some font/profile offsets clip a large cross at the cell edge. Its
        // intersection still follows the full ink center; do not move it inward.
        assert!((base.center().0-cx).abs()<=1.&&(base.center().1-cy).abs()<=1.);
    }}
    assert_eq!(corpse_core::storage::serialize(&r).unwrap(),frozen);
}

#[test]
fn corpse_center_native_boards_and_legacy_current_views() {
    let Ok(dir)=std::env::var("CORPSE_CENTER_OUTPUT") else {return;};
    std::fs::create_dir_all(&dir).unwrap();
    let save=|name:&str,p:&Painter| {
        let f=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut png=png::Encoder::new(f,p.width as u32,p.height as u32);png.set_color(png::ColorType::Rgba);png.set_depth(png::BitDepth::Eight);
        png.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    };
    let glyphs=["r","B","g","^","y","i","Ω","♣","人","^^","Wi","(≬)"];
    let mut p=Painter::new();
    for (width,height) in [(1280,800),(1920,1200),(2560,1600)] {
        p.resize(width,height);
        for (label,font,custom) in [("default","",false),("ibm","IBM Plex Mono",false),("custom-offset","",true)] {
            p.profile=center_profile(font,custom);p.clear([12,17,25]);
            p.text(&format!("CORPSE X CENTERING: {label} / {width} x {height}"),30,20,20,INK);
            p.text("Actual native glyphs. Each pair: glyph / same glyph with red death X. 15x24 and 30x48 cells.",30,52,16,MUTED);
            for (i,glyph) in glyphs.iter().enumerate() {
                let x=35+(i%4)as i32*305;let y=110+(i/4)as i32*200;
                p.text(&format!("Glyph {glyph}"),x,y,18,INK);
                for (j,(w,h)) in [(15,24),(30,48)].into_iter().enumerate() {
                    let xx=x+j as i32*140;let yy=y+45;
                    for dead in [false,true] {
                        let gx=xx+if dead {w+17}else{0};p.border(gx-2,yy-2,w+4,h+4,[48,59,74]);
                        p.map_identity_outline(glyph,gx,yy,w,h,[0;3]);
                        let bounds=p.map_identity_aligned(glyph,gx,yy,w,h,[73,214,150],false);
                        if dead {p.corpse_marker(bounds,gx,yy,w,h,None);}
                    }
                }
            }
            save(&format!("center-board-{label}-{width}x{height}"),&p);
        }
    }
    for revision in [29.,52.] {
        let mut r=color_room(revision);r.player.x=19.;
        for (i,kind) in ["rat","dryad","hemogoblin","dire-bear","vampire","pale:Birther"].iter().enumerate() {
            let mut live=corpse_core::population::create_actor_revision(kind,100.+i as f64*2.,13.+i as f64*3.,8.,false,revision).unwrap();
            if i==0 {live.color="#3bcda6".into();}if i==4 {live.conditions.insert("monsterVampireBat".into(),1.);}
            let mut dead=live.clone();dead.id+=1.;dead.y=11.;dead.dead=true.into();dead.hp=0.;r.actors_at_mut(1.).extend([live,dead]);
        }
        r=corpse_core::storage::deserialize(&corpse_core::storage::serialize(&r).unwrap()).unwrap();
        for a in r.actors_at(1.) {assert!(corpse_core::vision::directly_visible_actor(&r,a));}
        let frozen=corpse_core::storage::serialize(&r).unwrap();
        for custom in [false,true] {
            let mut profile=center_profile("",custom);
            if custom {profile=profile.set("entities.creatures.rat.inherited",serde_json::json!(false)).unwrap()
                .set("entities.creatures.rat.glyph",serde_json::json!("^")).unwrap()
                .set("entities.creatures.pale:Birther.inherited",serde_json::json!(false)).unwrap()
                .set("entities.creatures.pale:Birther.glyph",serde_json::json!("(≬)")).unwrap();}
            let view=crate::observe::observe_with_profile(&r,&profile);
            for (w,h) in [(1280,800),(1920,1200)] {
                p.profile=profile.clone();p.resize(w,h);p.game_at(&view,"",false,0.);
                save(&format!("center-scene-r{revision}-{}-{w}x{h}",if custom {"custom"}else{"default"}),&p);
            }
        }
        assert_eq!(corpse_core::storage::serialize(&r).unwrap(),frozen);
    }
}

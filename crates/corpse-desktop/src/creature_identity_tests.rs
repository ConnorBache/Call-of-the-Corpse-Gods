//! Actual R50 defaults through observation, native pixel painting and retained damage.
use crate::{appearance_profile::Profile,render::{Painter,View}};
use corpse_core::{Run,Field,population};

pub(crate) fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.player.x=10.;r.player.y=12.;r.player.stats.perception=100.;
    r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();r.inventory.clear();for block in r.actors.values_mut(){std::sync::Arc::make_mut(block).clear();}for block in r.ground.values_mut(){std::sync::Arc::make_mut(block).clear();}
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.environment=Field::Missing;f.doors=Field::Missing;
    for (i,kind) in ["blorb","bear","dire-bear","bastidion","pale:Biter","pale:Breather","hemogoblin-inventor","yeti","samara","banshee","short-saidri","long-saidri","saidri-guardian","pale:Birther","pale:Greater Birther","chimera","sphinx"].into_iter().enumerate() {
        r.actors_at_mut(r.depth).push(population::create_actor_revision(kind,200.+i as f64,7.+i as f64,10.,false,50.).unwrap());
    }
    r
}
fn cell(view:&View,id:f64)->&crate::render::Cell {view.cells.iter().find(|c|c.actor_id==Some(id)).unwrap()}

#[test]
fn enemy_background_matches_lit_terrain_in_sight_and_through_telepathy() {
    let mut run = room();
    run.simulation_revision = 52.0.into();
    run.actors_at_mut(run.depth).truncate(1);
    let id = run.actors_at(run.depth)[0].id;
    let (x, y) = (7, 10);
    let width = run.floor().unwrap().width();
    for tile in ["floor", "grass", "shallow-water"] {
        run.floor_mut().unwrap().tiles[y * width + x] = tile.into();
        for telepathic in [false, true] {
            let mut r = run.clone();
            if telepathic {
                r.player.conditions.insert("telepathic".into(), 10.);
                // A full wall between the Avatar and creature blocks sight, not thought.
                let floor = r.floor_mut().unwrap();
                for row in 0..floor.height() { floor.tiles[row * width + 8] = "wall".into(); }
            }
            let mut view = crate::observe::observe(&r);
            let actor = cell(&view, id);
            assert_eq!((actor.x, actor.y), (x, y));
            assert_eq!(actor.entity_ink.unwrap().telepathic, telepathic);
            assert_eq!(actor.background_overlay, None);
            let ground = view.cells[..view.terrain_count].iter()
                .find(|c| (c.x, c.y) == (x, y)).expect("observed terrain");
            assert_eq!(actor.bg, ground.bg, "{tile}, telepathic={telepathic}");
            // Isolate the background pixels from both the terrain and creature ink.
            let ground = crate::render::Cell { x, y, bg: ground.bg, current: true, ..Default::default() };
            let actor = crate::render::Cell {
                glyph: String::new(), superscript: String::new(), subscript: String::new(),
                heartless: false, ..actor.clone()
            };
            view.terrain_snapshot = None;
            view.terrain_count = 1;
            view.cells = vec![ground];
            let mut painter = Painter::new();
            painter.game_at(&view, "", false, 0.);
            let empty = painter.pixels.clone();
            view.cells.push(actor);
            painter.game_at(&view, "", false, 0.);
            assert_eq!(painter.pixels, empty, "{tile}, telepathic={telepathic}");
        }
    }

    // Leave deliberate appearance overrides available.
    let profile = Profile::default().set("entities.creatures.blorb.inherited", false.into()).unwrap()
        .set("entities.creatures.blorb.background", "#123456".into()).unwrap();
    let view = crate::observe::observe_with_profile(&run, &profile);
    assert_eq!(cell(&view, id).background_overlay.map(|(_, alpha)| alpha), Some(1.));
}

fn capture(p:&Painter,name:&str) {
    if let Ok(dir)=std::env::var("CORPSE_CREATURE_IDENTITY_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();
        let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
#[test]
fn revision50_creature_identity_observation_manual_and_custom_preferences() {
    let r=room();let before=r.clone();let view=crate::observe::observe(&r);
    for (id,name,glyph) in [(200.,"Blorb","b"),(201.,"Bear","B"),(202.,"Dire Bear","B"),(203.,"Corpse Fungus","F"),(204.,"Pale Chomper","c"),(205.,"Pale Gasser","g"),(206.,"Hemogoblin Inventor","h"),(207.,"Yeti","Y"),(208.,"Devil's Samara Tree","T"),(209.,"Banshee","B"),(210.,"Short-legged Saidribee","b"),(211.,"Long-legged Saidribee","b"),(212.,"Saidribee Guardian","B"),(213.,"Pale Birther","b"),(214.,"Greater Pale Birther","B"),(215.,"Chimera","C"),(216.,"Sphinx","S")] {
        let c=cell(&view,id);assert_eq!((&*c.label,&*c.glyph),(name,glyph));
        assert_eq!(view.creatures.iter().find(|a|a.id==id).unwrap().name,name);
        assert_eq!(crate::inspection_ui::observe_tile(&r,c.x,c.y).title,name);
    }
    let black=cell(&view,202.);assert_eq!(black.fg,[0,0,0]);assert_eq!(black.entity_ink.unwrap().identity_outline,None);assert!(black.entity_ink.unwrap().plain_black);
    let old_inherited=Profile::default().set("entities.creatures.dire-bear.glyph","U".into()).unwrap().set("entities.creatures.dire-bear.color","#e0a368".into()).unwrap().set("entities.creatures.dire-bear.inherited",true.into()).unwrap();
    assert!(cell(&crate::observe::observe_with_profile(&r,&old_inherited),202.).entity_ink.unwrap().plain_black);
    let custom=Profile::default().set("entities.creatures.dire-bear.inherited",false.into()).unwrap()
        .set("entities.creatures.dire-bear.glyph","D".into()).unwrap().set("entities.creatures.dire-bear.color","#ff00ff".into()).unwrap();
    let custom_view=crate::observe::observe_with_profile(&r,&custom);let c=cell(&custom_view,202.);
    assert_eq!(c.glyph,"D");assert_eq!(c.entity_ink.unwrap().identity_outline,None);assert!(!c.entity_ink.unwrap().plain_black);
    assert_eq!(custom_view.creatures.iter().find(|a|a.id==202.).unwrap().color,[255,0,255]);
    let custom=custom.set("entities.creatures.pale:Biter.inherited",false.into()).unwrap().set("entities.creatures.pale:Biter.glyph","J".into()).unwrap();
    assert_eq!(cell(&crate::observe::observe_with_profile(&r,&custom),204.).glyph,"J");
    let manual=crate::manual::Manual::new();
    for (kind,name,glyph,color) in [("blorb","Blorb","b","#28703c"),("bear","Bear","B","#a47c55"),("dire-bear","Dire Bear","B","#000000"),("bastidion","Corpse Fungus","F","#9fc875"),("hemogoblin-inventor","Hemogoblin Inventor","h","#d600ff"),("yeti","Yeti","Y","#D7E8F1"),("samara","Devil's Samara Tree","T","#8a5a32"),("banshee","Banshee","B","#d600ff"),("short-saidri","Short-legged Saidribee","b","#f8d100"),("long-saidri","Long-legged Saidribee","b","#8a5a32"),("saidri-guardian","Saidribee Guardian","B","#f8d100"),("pale:Birther","Pale Birther","b","#b9b8b1"),("pale:Greater Birther","Greater Pale Birther","B","#f0e8dc"),("chimera","Chimera","C","#ddb435"),("sphinx","Sphinx","S","#ffd34a")] {
        let e=manual.catalog.entries.iter().find(|e|e.id==kind).unwrap();assert_eq!((&*e.name,&*e.glyph,&*e.color),(name,glyph,color));
    }
    let parent=population::create_actor_revision("pale:Greater Birther",701.,10.,10.,false,50.).unwrap();
    let (_,details)=crate::creature_abilities_ui::describe(&r,&parent);assert!(details.contains("Chomper, Gasser"),"{details}");assert!(!details.contains("Biter"));
    let projected=crate::creature_presentation::project_kind(&r,&r.actors_at(r.depth)[0],"blorb").unwrap();assert_eq!((&*projected.glyph,&*projected.color),("b","#28703c"));
    assert_eq!(r,before);
}
#[test]
fn revision50_creature_identity_plain_black_native_pixels_varied_floors() {
    for tile in ["floor","water","slime"] {
        let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+9]=tile.into();
        let view=crate::observe::observe(&r);assert_eq!(cell(&view,202.).fg,[0,0,0]);
        assert_eq!(cell(&view,202.).entity_ink.unwrap().identity_outline,None);
        for (width,height) in [(1280,800),(1920,1200),(2560,1600)] {
            let mut p=Painter::new();p.resize(width,height);p.game_at(&view,"",false,0.);
            let (cw,ch,ox,oy)=p.map_layout(view.width,view.height);let c=cell(&view,202.);
            let (x,y)=(ox+c.x as i32*cw,oy+c.y as i32*ch);
            let mut black=0;let mut darkest=[255u8;3];
            for yy in p.native_y(y as f64)..p.native_y((y+ch) as f64) {for xx in p.native_x(x as f64)..p.native_x((x+cw) as f64) {
                let rgb=&p.pixels[(yy as usize*p.width+xx as usize)*4..][..3];
                // Native resampling can leave every pixel partially covered.
                // Black ink blends with the terrain at antialiased edges.
                if rgb.iter().all(|&v|v<=18) {black+=1;}if rgb.iter().copied().max().unwrap()<*darkest.iter().max().unwrap(){darkest.copy_from_slice(rgb);}
            }}
            capture(&p,&format!("creature-defaults-{tile}-{width}x{height}"));
            assert!(black>0,"black ink exceeds native antialias tolerance for {tile} {width}; darkest native pixel {darkest:?}");
            let mut with_outline=view.clone();for c in &mut with_outline.cells {if c.actor_id==Some(202.) {c.entity_ink.as_mut().unwrap().identity_outline=Some([190;3]);}}
            let mut normal=Painter::new();normal.resize(width,height);normal.game_at(&with_outline,"",false,0.);
            assert_ne!(p.pixels,normal.pixels,"the removed outline changes the native pixels");
            // The bare glyph leaves the terrain beyond its mask untouched.
            for (xx,yy) in [(x,y),(x+cw-1,y),(x,y+ch-1),(x+cw-1,y+ch-1)] {
                let at=(p.native_y(yy as f64) as usize*p.width+p.native_x(xx as f64) as usize)*4;
                assert_eq!(&p.pixels[at..at+4],&normal.pixels[at..at+4]);
            }
        }
    }
}
#[test]
fn revision50_creature_identity_damage_and_hidden_world_parity() {
    let r=room();let old=crate::observe::observe(&r);let mut moved=r.clone();moved.actors_at_mut(r.depth)[2].x+=1.;moved.actors_at_mut(r.depth)[2].y+=1.;
    let new=crate::observe::observe(&moved);
    for (w,h) in [(1280,800),(1537,911),(2560,1600)] {
        let mut p=Painter::new();let mut full=Painter::new();p.resize(w,h);full.resize(w,h);
        let mut retained=crate::render::RetainedGame::default();retained.update(&mut p,&old,"",0.,true,true).unwrap();retained.update(&mut p,&new,"",0.,true,false).unwrap();full.game_at(&new,"",false,0.);
        assert!(p.pixels==full.pixels,"retained outline {w}x{h}");
        let mut plan=crate::render::tile_damage::TilePlanner::default();let mut p=Painter::new();p.resize(w,h);
        plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
        plan.replace_view(Some(&old),&new,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&new,"",0.);p.tile_clip_enabled=false;
        assert!(p.pixels==full.pixels,"tile outline {w}x{h}");
    }
    let mut hidden=room();hidden.player.stats.perception=0.;hidden.actors_at_mut(r.depth)[2].x=60.;hidden.actors_at_mut(r.depth)[2].y=20.;let w=hidden.floor().unwrap().width();
    for y in 1..28 {hidden.floor_mut().unwrap().tiles[y*w+20]="wall".into();}
    let before=crate::observe::observe(&hidden);assert!(!before.cells.iter().any(|c|c.actor_id==Some(202.)));
    hidden.actors_at_mut(r.depth)[2].name="Secret rename".into();hidden.actors_at_mut(r.depth)[2].color="#ff00ff".into();
    let after=crate::observe::observe(&hidden);assert_eq!(before.cells,after.cells);
}

#[test]
fn revision50_creature_rank_observation_and_native_captures() {
    let mut r=room();r.actors_at_mut(r.depth).clear();
    for (i,kind) in ["lesser-ichor-fish","ichor-fish-standard","greater-ichor-fish","lesser-ichor-shark","ichor-shark","greater-ichor-shark","lesser-ichor-leviathan","ichor-leviathan","greater-ichor-leviathan"].iter().enumerate() {
        r.actors_at_mut(r.depth).push(population::create_actor_revision(kind,300.+i as f64,7.+i as f64,10.,false,50.).unwrap());
    }
    for (i,q) in ["infant","child","lesser","unqualified","mature","greater","hulking","ancient","timeless","transcendent"].iter().enumerate() {
        let mut a=corpse_core::Actor {id:400.+i as f64,x:7.+i as f64,y:11.,creature_kind:"zillennia".to_string().into(),zillennia_quality:q.to_string().into(),glyph:"Z".into(),color:"#d6b1e3".into(),name:format!("{q} Zillennia"),hp:100.,max_hp:100.,..Default::default()};
        if i==9 {a.state="ally".into();a.order="a".to_string().into();}
        r.actors_at_mut(r.depth).push(a);
    }
    for i in 0..8 {r.actors_at_mut(r.depth).push(corpse_core::phantasmii_generation::nightmare(i*5+1,500.+i as f64,7.+i as f64,13.));}
    let view=crate::observe::observe(&r);
    for i in 0..9 {assert_eq!(cell(&view,300.+i as f64).superscript,(i%3+1).to_string());}
    for i in 0..10 {assert_eq!(cell(&view,400.+i as f64).superscript,(i+1).to_string());}
    assert_eq!(cell(&view,409.).subscript,"a");
    // Dream-only actors are tested by the rank unit mapping; explicitly compose
    // their visible presentation here for layout without changing world visibility.
    let mut view=view;
    for i in 0..8 {view.cells.push(crate::render::Cell {x:7+i,y:13,glyph:"N".into(),superscript:(i+1).to_string(),fg:[216,163,240],bg:[5,4,12],..Default::default()});}
    for (width,height) in [(1280,800),(1920,1200),(2560,1600)] {
        let mut p=Painter::new();p.resize(width,height);p.game_at(&view,"",false,0.);capture(&p,&format!("creature-ranks-{width}x{height}"));
        let first=p.pixels.clone();p.game_at(&view,"",false,0.);assert_eq!(p.pixels,first);
    }
}

#[test]
fn beetle_gasser_dryad_cosmetic_identity_and_flicker() {
    let mut r=room();r.actors_at_mut(r.depth).clear();
    let kinds=[("bombardier","b","#E69B40"),("dryad","d","#81CF94"),("pale:Breather","g","#B9B8B1"),("pale:Greater Breather","G","#F0E8DC"),("giant-beetle","Q","#8a5a32"),("shellback-beetle","q","#C7985B")];
    for (i,(kind,_,_)) in kinds.iter().enumerate() { r.actors_at_mut(r.depth).push(population::create_actor_revision(kind,600.+i as f64,7.+i as f64,10.,false,50.).unwrap()); }
    let before=r.clone();let view=crate::observe::observe(&r);assert_eq!(r,before);
    let manual=crate::manual::Manual::new();
    for (i,(kind,glyph,color)) in kinds.iter().enumerate() {
        let c=cell(&view,600.+i as f64);assert_eq!(&c.glyph,glyph);assert_eq!(c.fire_flicker,i==0);assert!(!c.fire);
        assert_eq!(view.creatures.iter().find(|a|a.id==600.+i as f64).unwrap().glyph,*glyph);
        let e=manual.catalog.entries.iter().find(|e|e.id==*kind).unwrap();assert_eq!((&*e.glyph,e.color.to_lowercase()),(*glyph,color.to_lowercase()));
        let identity=crate::entity_profile::resolve(&Profile::default(),"creatures",kind,None,&r.actors_at(r.depth)[i].glyph,crate::color::hex(&r.actors_at(r.depth)[i].color));assert_eq!(identity.color,crate::color::hex(color));
    }
    let custom=Profile::default().set("entities.creatures.bombardier.inherited",false.into()).unwrap().set("entities.creatures.bombardier.glyph","!".into()).unwrap().set("entities.creatures.bombardier.color","#00ffff".into()).unwrap();
    let custom_view=crate::observe::observe_with_profile(&r,&custom);assert!(!cell(&custom_view,600.).fire_flicker);assert_eq!(cell(&custom_view,600.).glyph,"!");
    let mut p=Painter::new();p.resize(1280,800);p.game_at(&view,"",false,100.);let first=p.pixels.clone();capture(&p,"glyph-update-100ms");p.game_at(&view,"",false,330.);capture(&p,"glyph-update-330ms");
    let c=cell(&view,600.);let(cw,ch,ox,oy)=p.map_layout(view.width,view.height);let(x,y)=(ox+c.x as i32*cw,oy+c.y as i32*ch);
    let mut different=0;for yy in p.native_y(y as f64)..p.native_y((y+ch) as f64){for xx in p.native_x(x as f64)..p.native_x((x+cw) as f64){let i=(yy as usize*p.width+xx as usize)*4;if first[i..i+4]!=p.pixels[i..i+4]{different+=1;}}}assert!(different>0,"Bombardier glyph must actually flicker in native pixels");
    let mut retained=crate::render::RetainedGame::default();let mut cached=Painter::new();cached.resize(1280,800);retained.update(&mut cached,&view,"",100.,true,true).unwrap();retained.update(&mut cached,&view,"",330.,true,false).unwrap();assert_eq!(cached.pixels,p.pixels,"animated damage must redraw the beetle");
    r.actors_at_mut(r.depth)[0].hp=0.;r.actors_at_mut(r.depth)[0].dead=true.into();let dead=crate::observe::observe(&r);assert!(dead.cells.iter().filter(|c|c.actor_id==Some(600.)).all(|c|!c.fire_flicker));
}

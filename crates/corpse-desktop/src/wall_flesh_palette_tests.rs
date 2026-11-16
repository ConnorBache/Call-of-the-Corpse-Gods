//! Knowledge-only wall face/backing ink, native pixels, and retained rendering.
use super::*;
use crate::{appearance_profile::Profile, render::{Painter, RetainedGame, tile_damage::TilePlanner}};
use corpse_core::{Field, Floor, Run, vision::TerrainObservation};
use std::sync::Arc;

const W: usize = 79;
const H: usize = 29;
const FACE: usize = 14 * W + 11;
const BACKING: usize = 14 * W + 10;

fn room() -> Run {
    let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = 52.0.into();
    run.player.x = 16.; run.player.y = 14.; run.player.stats.perception = 30.;
    run.player.abilities.clear(); run.player.active.clear(); run.player.conditions.clear();
    run.player.ability_order = Field::Missing; run.player.equipment_abilities = Field::Missing;
    run.inventory.clear(); run.actors = Default::default(); run.ground = Default::default();
    run.history.clear(); run.log.clear(); run.log_kinds = Field::Missing; run.offers.clear();
    let mut tiles = vec!["wall".to_owned(); W * H];
    for y in 8..22 { for x in 12..52 { tiles[y * W + x] = "floor".into(); } }
    run.floors = vec![Arc::new(Floor { depth:run.depth, width:(W as f64).into(), height:(H as f64).into(),
        tiles:tiles.clone(), remembered_tiles:tiles.into(), seen:vec![true;W*H], lights:vec![0.;W*H],
        ..Default::default() })];
    run
}

/// A controlled, fully known material review, detached from live visibility.
/// Unknown margins stay unknown. All production color and native paint paths run.
fn scene(level: f64, remembered: bool) -> (Run, TerrainObservation) {
    let run = room();
    let mut observation = corpse_core::vision::observe_terrain(&run).unwrap();
    observation.light_sources.clear();
    for cell in &mut observation.cells {
        let known = (5..64).contains(&(cell.index % W));
        cell.tile = known.then(|| run.floor().unwrap().tiles[cell.index].clone());
        cell.current = known && !remembered; cell.direct = cell.current; cell.remembered = known;
        cell.knowledge = if !known { "layout" } else if remembered { "remembered" } else { "direct" }.into();
        cell.chance = if cell.current {100.} else {0.}; cell.direct_chance = cell.chance; cell.distance = 0.;
        cell.ambient = cell.current.then_some(level); cell.illumination = cell.ambient;
        cell.substrate = None; cell.door_open = None; cell.grate = false; cell.bridge = false;
        cell.entrance = false; cell.snow = false; cell.ice = false; cell.puddles = false;
        cell.hellfire = false; cell.flowers = false; cell.biome = None; cell.hazards = None;
    }
    (run, observation)
}

fn view(run: &Run, observation: &TerrainObservation, profile: &Profile) -> View {
    let mut view = observe_with_profile(run, profile);
    view.cells = terrain_frame(observation, profile, 17, 0.).0;
    view.terrain_count = view.cells.len();
    view.terrain_snapshot = Some(Arc::new(observation.clone()));
    view.cosmetic_seed = 17;
    view
}
fn cell(view: &View, at: usize) -> &Cell {
    view.cells[..view.terrain_count].iter().find(|c|c.y * W + c.x == at).unwrap()
}
fn pixels(p: &Painter, view: &View, at: usize) -> Vec<u8> {
    let (cw,ch,ox,oy) = p.map_layout(view.width,view.height);
    let r = p.logical_damage_rect(ox+(at%W) as i32*cw,oy+(at/W) as i32*ch,cw,ch).unwrap();
    (r.y..r.bottom()).flat_map(|y|p.pixels[(y*p.width+r.x)*4..(y*p.width+r.right())*4].iter().copied()).collect()
}
fn png(name: &str, p: &Painter) {
    if let Ok(dir) = std::env::var("CORPSE_WALL_FLESH_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut encoder = png::Encoder::new(file,p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}

#[test]
fn wall_face_ink_reuses_cardinal_known_edge_identity_and_preserves_fixtures() {
    let (_, mut observation) = scene(0.,false);
    let base = [180,163,154];
    let ink = |o: &TerrainObservation, at, enabled| flesh_palette::wall_ink(base,&o.cells[at],o,enabled);
    assert_eq!(ink(&observation,FACE,true),base);
    assert_eq!(ink(&observation,BACKING,true),[187,124,114]);
    assert_eq!(ink(&observation,BACKING,false),base,"legacy and edited colors bypass the patch");
    // A diagonal opening, unknown neighbor, chasm or pit is not an existing wall edge.
    for at in [BACKING-1,BACKING+1,BACKING-W,BACKING+W] { observation.cells[at].tile = Some("wall".into()); }
    observation.cells[BACKING+W+1].tile = Some("floor".into());
    for tile in [None,Some("wall"),Some("chasm"),Some("pit")] {
        observation.cells[BACKING+1].tile = tile.map(str::to_owned);
        assert_eq!(ink(&observation,BACKING,true),[187,124,114]);
    }
    observation.cells[BACKING+1].tile = Some("floor".into());
    assert_eq!(ink(&observation,BACKING,true),base);
    observation.cells[BACKING].current = false;
    assert_eq!(ink(&observation,BACKING,true),[187,124,114],"memory never borrows a live neighbor");
    observation.cells[BACKING+1].current = false;
    assert_eq!(ink(&observation,BACKING,true),base,"remembered openings remain usable");
    observation.cells[BACKING+1].tile = Some("wall".into());
    for fixture in 0..8 {
        let mut test = observation.clone(); let c = &mut test.cells[BACKING];
        match fixture {0=>c.door_open=Some(false),1=>c.grate=true,2=>c.bridge=true,3=>c.snow=true,
            4=>c.ice=true,5=>c.puddles=true,6=>c.hellfire=true,_=>c.tile=Some("floor".into())}
        assert_eq!(ink(&test,BACKING,true),base);
    }
    // Bounds do not wrap the row into a fabricated exposed face.
    observation.cells[W-1].tile = Some("wall".into());
    observation.cells[W-2].tile = Some("wall".into());
    observation.cells[2*W-1].tile = Some("wall".into());
    observation.cells[W].tile = Some("floor".into());
    assert_eq!(ink(&observation,W-1,true),[187,124,114]);
}

#[test]
fn wall_flesh_native_colors_follow_illumination_and_existing_neutral_memory() {
    let mut rows = Vec::new();
    for remembered in [false,true] { for level in [-100.,0.,100.] {
        let (run,observation) = scene(level,remembered); let before = serde_json::to_vec(&run).unwrap();
        let view = view(&run,&observation,&Profile::default());
        let face = cell(&view,FACE); let backing = cell(&view,BACKING);
        assert_eq!((face.glyph.as_str(),backing.glyph.as_str()),("#","#"));
        assert_eq!((face.label.as_str(),backing.label.as_str()),("wall","wall"));
        assert_ne!(face.fg,backing.fg,"distinct source glyph ink at light {level}, memory={remembered}");
        if remembered {
            let (face_ink,face_bg) = crate::render::readable_cell_colors(face);
            let (backing_ink,backing_bg) = crate::render::readable_cell_colors(backing);
            assert_eq!(face_ink,[81;3],"existing gray fog retains the lighter wall face");
            assert_eq!(backing_ink,[70;3],"backing retains existing wall readability floor");
            assert_eq!(face_bg,backing_bg,"only foreground classification changes");
        }
        let mut p = Painter::new(); p.resize(1280,800); p.game_at(&view,"",false,0.);
        let native = p.pixels.clone();
        let name = format!("wall-flesh-{}-{level}",if remembered {"remembered"} else {"visible"});
        png(&name,&p);
        assert!(pixels(&p,&view,14*W+3).chunks_exact(4).all(|p|p[..3]==[0;3]),"unknown remains black");
        for (role,at) in [("wall-face",FACE),("stale-flesh",BACKING)] {
            let c = cell(&view,at); let (fg,bg) = crate::render::readable_cell_colors(c);
            if remembered { assert_eq!(fg,[fg[0];3]); assert_eq!(bg,[bg[0];3]); assert!(c.display_light.is_none()); }
            else if level < 0. { assert!(crate::color::contrast(fg,bg)>=2.95); }
            rows.push(serde_json::json!({"role":role,"remembered":remembered,"illumination":level,
                "sourceForeground":c.fg,"sourceBackground":c.bg,"paintForeground":fg,"paintBackground":bg,
                "contrast":crate::color::contrast(fg,bg),"luminance":crate::color::luminance(fg),
                "nativeTileRgbaCrc32":crc32fast::hash(&pixels(&p,&view,at)),"nativeRgbaCrc32":crc32fast::hash(&native)}));
        }
        p.game_at(&view,"",false,731.); assert_eq!(native,p.pixels,"static wall material");
        assert_eq!(serde_json::to_vec(&run).unwrap(),before);
    }}
    if let Ok(dir) = std::env::var("CORPSE_WALL_FLESH_OUTPUT") {
        std::fs::write(format!("{dir}/colors-and-contrast.json"),serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}

#[test]
fn wall_flesh_custom_color_grading_and_legacy_reference_are_preserved() {
    let (_,observation) = scene(0.,false);
    for (key,value) in [("terrain.wall.color",serde_json::json!("#31b5d3")),
        ("terrain.wall.background",serde_json::json!("#311b25")),("terrain.wall.glyph",serde_json::json!("X")),
        ("terrain.wall.opacity",serde_json::json!(0.4)),("terrain.wall.brightness",serde_json::json!(0.65)),
        ("global.exposure",serde_json::json!(0.6)),("fog.rememberedBrightness",serde_json::json!(0.4))] {
        let p = Profile::default().set(key,value).unwrap();
        for flesh in [false,true] {
            let a = terrain_frame_palette(&observation,&p,17,731.,flesh);
            let b = terrain_frame_palette_json_reference(&observation,&p,17,731.,flesh);
            assert_eq!(a.0,b.0,"compiled/reference {key}, flesh={flesh}"); assert_eq!(a.1,b.1);
        }
    }
    // Only geometry changes at the same sampled tile. Explicit ink still wins.
    let mut exposed = observation.clone(); exposed.cells[BACKING+1].tile = Some("floor".into());
    let p = Profile::default().set("terrain.wall.color",serde_json::json!("#31b5d3")).unwrap();
    for (p,flesh) in [(&p,true),(Profile::defaults(),false)] {
        let a = terrain_frame_palette(&observation,p,17,0.,flesh).0;
        let b = terrain_frame_palette(&exposed,p,17,0.,flesh).0;
        let find = |cells:Vec<Cell>|cells.into_iter().find(|c|c.y*W+c.x==BACKING).unwrap();
        assert_eq!(find(a).fg,find(b).fg,"custom ink and v48 do not classify backing");
    }
}

#[test]
fn hidden_live_neighbors_never_recolor_visible_or_remembered_wall_ink() {
    let mut run = room(); run.player.x = 13.; run.player.stats.perception = 10.;
    let unknown = 14*W+10;
    run.floor_mut().unwrap().seen[unknown] = false;
    run.floor_mut().unwrap().remembered_tiles.as_mut().unwrap()[unknown].clear();
    let baseline = observe(&run); let observed = baseline.terrain_snapshot.as_ref().unwrap();
    assert!(observed.cells[FACE].current && observed.cells[FACE].direct,"wall face actually visible");
    assert!(!observed.cells[unknown].current && observed.cells[unknown].tile.is_none(),"neighbor actually hidden");
    let remembered = 14*W+8;
    assert!(!observed.cells[remembered].current && observed.cells[remembered].remembered);
    assert!(!baseline.cells.iter().any(|c|c.y*W+c.x==unknown));
    let front = cell(&baseline,FACE).clone(); let old = cell(&baseline,remembered).clone();
    // Leave the occluding face in place. Hidden live openings must not redefine either material.
    for y in 0..H {for x in 0..11 {
        run.floor_mut().unwrap().tiles[y*W+x] = "floor".into();
        run.floor_mut().unwrap().lights[y*W+x] = 100.;
    }}
    let serialized = serde_json::to_vec(&run).unwrap();
    let changed = observe(&run);
    assert_eq!(cell(&changed,FACE),&front);
    assert_eq!(cell(&changed,remembered),&old);
    assert!(changed.terrain_snapshot.as_ref().unwrap().cells[unknown].tile.is_none());
    let mut painted = Painter::new(); painted.game_at(&baseline,"",false,0.);
    let native = painted.pixels.clone();
    png("wall-flesh-real-observation",&painted);
    assert!(pixels(&painted,&baseline,unknown).chunks_exact(4).all(|p|p[..3]==[0;3]));
    painted.game_at(&changed,"",false,0.);
    assert_eq!(painted.pixels,native,"hidden neighbor mutations leave all native pixels unchanged");
    assert_eq!(serde_json::to_vec(&run).unwrap(),serialized,"observation never mutates save state");
    let restored:Run = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(cell(&observe(&restored),remembered),&old,"save reload keeps remembered classification");
}

#[test]
fn wall_flesh_full_retained_and_tile_pixels_match_visibility_and_geometry_changes() {
    let mut full = Painter::new(); let mut retained = Painter::new(); let mut tiled = Painter::new();
    for p in [&mut full,&mut retained,&mut tiled] {p.resize(1280,800);}
    let mut cache = RetainedGame::default(); let mut planner = TilePlanner::default(); let mut previous = None;
    for (level,remembered,opening) in [(0.,false,false),(100.,false,false),(-100.,false,false),
        (0.,true,false),(0.,false,true),(0.,true,true),(0.,false,false)] {
        let (run,mut observation) = scene(level,remembered);
        if opening {observation.cells[BACKING-1].tile=Some("floor".into());}
        let view = view(&run,&observation,&Profile::default());
        for now in [0.,731.] {
            full.game_at(&view,"",false,now);
            cache.update(&mut retained,&view,"",now,true,previous.is_none()).unwrap();
            assert_eq!(retained.pixels,full.pixels,"retained {level}/{remembered}/{opening}/{now}");
            planner.replace_view(previous.as_ref(),&view,&tiled); planner.finish_damage(&mut tiled);
            planner.paint_base(&mut tiled,&view,"",now); tiled.tile_clip_enabled=false; planner.clear_frame();
            assert_eq!(tiled.pixels,full.pixels,"tile {level}/{remembered}/{opening}/{now}");
        }
        previous=Some(view);
    }
}

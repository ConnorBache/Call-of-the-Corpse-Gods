//! Detached, knowledge-gated portal presentation. No simulation or cosmetic RNG.
use corpse_core::{Actor, Run, vision};
use crate::render::{Cell, Rgb};

pub const COLORS: [Rgb; 7] = [
    [70, 145, 255], [255, 145, 55], [242, 70, 80], [250, 222, 65],
    [83, 213, 116], [111, 100, 230], [196, 113, 243],
];
pub const NAMES: [&str; 7] = ["Blue", "Orange", "Red", "Yellow", "Green", "Indigo", "Violet"];

pub fn terrain(run: &Run, observation: &vision::TerrainObservation, cells: &mut [Cell]) {
    if !corpse_core::portals::enabled(run) { return; }
    let Some(f) = run.floor() else { return; };
    for cell in cells {
        let at = cell.y * observation.width + cell.x;
        let Some(o) = observation.cells.get(at).filter(|o| o.current || o.remembered) else { continue; };
        let Some(pair) = corpse_core::portals::pair_at(f, at) else { continue; };
        let color = pair.color_index as usize;
        cell.glyph = "0".into();
        cell.italic = true;
        cell.label = format!("{} portal", NAMES[color.min(6)]);
        cell.fg = crate::color::composite(cell.bg, COLORS[color.min(6)], if o.current { 1. } else { 0.48 });
        cell.animations.clear();
    }
}

/// Remote endpoint occupancy is disclosed only through observed cells and actors.
pub fn observed_alias<'a>(run: &'a Run, x: usize, y: usize) -> Option<&'a Actor> {
    observed_alias_inner(run, x, y, None)
}
/// The supplied terrain must belong to this same projected Run and world generation.
pub fn observed_alias_with_terrain<'a>(run: &'a Run, x: usize, y: usize, terrain: &vision::TerrainObservation) -> Option<&'a Actor> {
    observed_alias_inner(run, x, y, Some(terrain))
}
fn observed_alias_inner<'a>(run: &'a Run, x: usize, y: usize, terrain: Option<&vision::TerrainObservation>) -> Option<&'a Actor> {
    if !corpse_core::portals::enabled(run) { return None; }
    let f = run.floor()?;
    let at = y.checked_mul(f.width())?.checked_add(x)?;
    if x >= f.width() || y >= f.height() { return None; }
    let mate = corpse_core::portals::mate(f, at)?;
    let owned;
    let observation = match terrain.filter(|o| o.width == f.width() && o.height == f.height() && o.cells.len() == f.tiles.len()) {
        Some(o) => o,
        None => { owned = vision::observe_terrain(run)?; &owned }
    };
    if !observation.cells.get(at)?.current { return None; }
    run.actors_at(run.depth).iter().find(|a| !a.dead.value_or(false)
        && a.x == (mate % f.width()) as f64 && a.y == (mate / f.width()) as f64
        && corpse_core::dreamwalking::same_plane(run, &run.player, a)
        && vision::visible_actor(run, a))
}

/// Ground items require direct observation at both ends, never live metadata
/// behind a remembered or unexplored endpoint. Returns only a display alias.
pub fn observed_item_mate(run: &Run, observation: &vision::TerrainObservation, at: usize) -> Option<usize> {
    if !corpse_core::portals::enabled(run) || !observation.cells.get(at)?.direct { return None; }
    let mate = corpse_core::portals::mate(run.floor()?, at)?;
    observation.cells.get(mate)?.direct.then_some(mate)
}

pub const INSPECTOR_SCENE: &str = "Reality warps around this portal.";
#[cfg(test)]
const CREATURE_SCENE_SUFFIX: &str = " while emerging from a portal.";

pub fn known_name(run: &Run, cell: &vision::ObservedCell) -> Option<String> {
    if !corpse_core::portals::enabled(run) || !(cell.current || cell.remembered) { return None; }
    let f = run.floor()?;
    let pair = corpse_core::portals::pair_at(f, cell.index)?;
    Some(format!("{} portal", NAMES[(pair.color_index as usize).min(6)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::{Field, portals_generation::PortalPair};
    fn scene() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 35.0.into();
        r.player.x=9.; r.player.y=10.; r.player.abilities.clear(); r.player.active.clear();
        r.player.conditions.clear(); r.actors.clear(); r.ground.clear(); r.offers.clear();
        r.player.conditions.insert("allSeeingReveal".into(),1.);
        r.player.conditions.insert("allSeeingDepth".into(),r.depth);
        let f=r.floor_mut().unwrap(); let w=f.width(); let n=f.tiles.len();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.lights=vec![100.;n];f.seen=vec![true;n];f.remembered_tiles=f.tiles.clone().into();
        f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;
        f.portals=vec![PortalPair{a:10*w+11,b:10*w+45,color_index:0}].into();r
    }
    #[test]
    fn portal_ui_ascii_italic_pair_color_and_legacy_gate() {
        let r=scene();let before=r.clone();let v=crate::observe::observe(&r);
        let a=v.cells.iter().find(|c|c.x==11&&c.y==10&&c.italic).unwrap();
        let b=v.cells.iter().find(|c|c.x==45&&c.y==10&&c.italic).unwrap();
        assert_eq!(a.glyph,"0");assert_eq!(a.fg,b.fg);assert!(a.label.contains("Blue"));
        assert_eq!(r,before);
        let mut legacy=r.clone();legacy.simulation_revision=34.0.into();
        assert!(!crate::observe::observe(&legacy).cells.iter().any(|c|c.italic));
    }
    #[test]
    fn portal_ui_unknown_mate_stays_unknown() {
        let mut r=scene();r.player.conditions.clear();
        let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
        for y in 1..f.height()-1 {f.tiles[y*w+20]="wall".into();}
        f.seen=vec![false;n];f.remembered_tiles=Field::Missing;
        let mut o=vision::observe_terrain(&r).unwrap();
        // The adapter must honor the detached visibility result, irrespective of
        // any pair metadata available in the live floor.
        o.cells[10*w+45].current=false;o.cells[10*w+45].remembered=false;
        let mut cells=vec![Cell{x:45,y:10,..Default::default()}];terrain(&r,&o,&mut cells);
        assert!(!cells[0].italic);
    }
    #[test]
    fn portal_ui_inspector_names_empty_and_occupied_portals_without_revealing_hidden_ones() {
        let mut r = scene();
        r.actors_at_mut(r.depth).clear();
        let empty = crate::inspection_ui::observe_tile(&r, 11, 10);
        assert_eq!(empty.title, "Blue portal");
        assert_eq!(empty.scene, "Reality warps around this portal.");
        assert_eq!(empty.lines.first().map(String::as_str), Some(INSPECTOR_SCENE));
        let actor = corpse_core::population::create_actor_revision("rat", 900., 11., 10., false, 35.).unwrap();
        r.actors_at_mut(r.depth).push(actor);
        let occupied = crate::inspection_ui::observe_tile(&r, 11, 10);
        assert!(occupied.scene.starts_with("A giant rat "));
        assert!(occupied.scene.ends_with(CREATURE_SCENE_SUFFIX));
        assert!(occupied.creature.unwrap().description.starts_with(&occupied.scene));
        r.actors_at_mut(r.depth).clear();
        let mut observed = vision::observe_terrain(&r).unwrap();
        let cell = &mut observed.cells[10 * observed.width + 11];
        cell.current = false;
        cell.remembered = false;
        let hidden = crate::inspection_ui::observe_tile_with_terrain(&r, 11, 10, &observed);
        assert!(!hidden.scene.contains("portal"));
    }
    #[test]
    fn portal_ui_remote_actor_alias_inspection_and_one_list_entry() {
        let mut r=scene();let a=corpse_core::population::create_actor_revision("rat",900.,45.,10.,false,34.).unwrap();
        r.actors_at_mut(r.depth).push(a);
        let v=crate::observe::observe(&r);
        let aliases:Vec<_>=v.cells.iter().filter(|c|c.actor_id==Some(900.)).collect();
        assert_eq!(aliases.len(),2);assert!(aliases.iter().any(|c|c.x==11));
        assert_eq!(v.creatures.iter().filter(|c|c.id==900.).count(),1);
        let report=crate::inspection_ui::observe_tile(&r,11,10);
        assert!(report.creature.is_some());
        assert_eq!(report.scene, "A giant rat stands from within a portal.");
    }
    #[test]
    fn portal_ui_ground_item_alias_observed_only_and_single_identity() {
        let mut r=scene();
        let item=corpse_core::equipment::make_equipment("Ring",901.).unwrap();
        let name=item.name.clone();
        r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:45.,y:10.,item,..Default::default()});
        let before=r.clone();let v=crate::observe::observe(&r);
        assert!(v.cells.iter().any(|c|c.x==11&&c.y==10&&c.label==name));
        assert!(v.cells.iter().any(|c|c.x==45&&c.y==10&&c.label==name));
        let report=crate::inspection_ui::observe_tile(&r,11,10);
        assert!(report.scene.contains("lies in a portal"));
        assert_eq!(r,before);assert_eq!(r.ground.get(&r.depth.to_string()).unwrap().len(),1);
        let w=r.floor().unwrap().width();let mut o=vision::observe_terrain(&r).unwrap();
        assert_eq!(observed_item_mate(&r,&o,10*w+11),Some(10*w+45));
        o.cells[10*w+45].direct=false;
        assert_eq!(observed_item_mate(&r,&o,10*w+11),None);
        o.cells[10*w+45].direct=true;o.cells[10*w+11].direct=false;
        assert_eq!(observed_item_mate(&r,&o,10*w+11),None);
        r.simulation_revision=34.0.into();
        assert!(!crate::inspection_ui::observe_tile(&r,11,10).scene.contains("lies in a portal"));
    }
    #[test]
    fn portal_ui_actual_renderer_italic_is_distinct_and_capture() {
        let mut r=scene();let w=r.floor().unwrap().width();
        r.floor_mut().unwrap().portals=(0..7).map(|i|PortalPair{a:(4+i*3)*w+15,b:(4+i*3)*w+55,color_index:i as u8}).collect::<Vec<_>>().into();
        let mut v=crate::observe::observe(&r);let mut p=crate::render::Painter::new();
        p.game_at(&v,"Portal pairs: blue, orange, red, yellow, green, indigo, violet",false,824.);let italic=p.pixels.clone();
        if let Ok(dir)=std::env::var("CORPSE_PORTAL_UI_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();
            let file=std::fs::File::create(format!("{dir}/portal-colors-italic-controlled.png")).unwrap();
            let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
        for c in &mut v.cells {c.italic=false;}
        v.terrain_snapshot=None;
        p.game_at(&v,"Portal pairs: blue, orange, red, yellow, green, indigo, violet",false,824.);
        assert_ne!(p.pixels,italic);
    }
    #[test]
    fn portal_ui_generated_floor_capture() {
        let Ok(dir)=std::env::var("CORPSE_PORTAL_GENERATED_REVIEW") else { return; };
        let mut r=corpse_core::startup::create_run_revision("portal-review-r35", "Unblessed", true,35.).unwrap();
        r.depth=10.;r.offers.clear();
        let f=r.floor().unwrap();let pairs=f.portals.as_ref().unwrap();assert!(!pairs.is_empty());
        let portal=pairs[0].a;let w=f.width();
        let at=[portal.saturating_sub(1),portal+1,portal.saturating_sub(w),portal+w].into_iter()
            .find(|&at|at<f.tiles.len()&&matches!(f.tiles[at].as_str(),"floor"|"grass")&&!pairs.iter().any(|p|p.a==at||p.b==at)).unwrap();
        r.player.x=(at%w)as f64;r.player.y=(at/w)as f64;
        r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),10.);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/generated-floor10-capture-run.json"),serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        let v=crate::observe::observe(&r);assert!(v.cells.iter().any(|c|c.italic));
        let mut p=crate::render::Painter::new();p.game_at(&v,"DEBUG capture: generated floor 10; full reveal; no turns simulated",false,824.);
        let file=std::fs::File::create(format!("{dir}/portals-generated-floor10-debug.png")).unwrap();
        let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }


    #[test]
    fn supplied_terrain_alias_preserves_visibility_gates() {
        let mut r=scene();
        r.actors_at_mut(r.depth).push(corpse_core::population::create_actor_revision("rat",900.,45.,10.,false,43.).unwrap());
        let before=serde_json::to_vec(&r).unwrap();
        let projected=crate::dream_ui::observation_run(&r);let r=projected.as_ref();
        let mut o=vision::observe_terrain(r).unwrap();
        for x in [10,11,12,13,14,45,60] {
            assert_eq!(observed_alias_with_terrain(r,x,10,&o).map(|a|a.id),observed_alias(r,x,10).map(|a|a.id));
            
        }
        let at=10*o.width+11;o.cells[at].current=false;
        assert!(observed_alias_with_terrain(r,11,10,&o).is_none());
        assert_eq!(serde_json::to_vec(r).unwrap(),before);
    }
}

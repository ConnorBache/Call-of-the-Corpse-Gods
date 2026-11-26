//! Knowledge-gated, detached Cogworks ink. Raster painting never reads the Run.
use corpse_core::{Run, revision50_terrain::Kind, vision::TerrainObservation};
use crate::render::Cell;

const CLAY_LABEL: &str = "Clay Soldier (unbreakable wall)";
const CLAY_AVATAR_ORANGE: [u8;3] = [255, 197, 106];
const CLAY_WALL_ORANGE: [u8;3] = [241, 105, 25];
/// Paint the ordinary "#" silhouette under a smaller, brighter "@". Use a
/// terrain underlay rather than creating an actor in the presentation.
fn clay_glyph(cell: &mut Cell) {
    cell.glyph = "@".into();
    cell.fg = CLAY_AVATAR_ORANGE;
    cell.emphasis = 0.86;
    cell.underlays.push(("#".into(), CLAY_WALL_ORANGE, 0.82));
}

/// O(V + K + C), O(V) temporary bits: V observed grid size, K total stored
/// modification footprint cells, C projected terrain cells. Called on observation.
/// Remembered terrain is not a live view. Without saved color provenance, leave
/// its existing remembered tile untouched instead of consulting hidden changes.
pub fn terrain(run: &Run, observation: &TerrainObservation, cells: &mut [Cell]) {
    if run.simulation_revision.value_or(29.) < 50. { return; }
    let Some(floor)=run.floor() else { return; };
    let Some(modifications)=floor.revision50_terrain.as_ref() else { return; };
    if observation.width==0 || observation.width!=floor.width() { return; }
    let mut marked=vec![None;observation.cells.len()];
    for modification in modifications {
        let expected=match modification.kind {Kind::Pit=>"chasm",Kind::Wall|Kind::ClaySoldier=>"wall"};
        for point in &modification.cells {
            if point.x<0 || point.y<0 || point.x as usize>=observation.width
                || point.y as usize>=observation.height { continue; }
            let at=point.y as usize*observation.width+point.x as usize;
            if observation.cells.get(at).is_some_and(|o|o.current && o.tile.as_deref()==Some(expected)) {
                marked[at]=Some((modification.kind,modification.hp));
            }
        }
    }
    for cell in cells {
        if cell.x>=observation.width || cell.y>=observation.height {continue;}
        let Some((kind,hp))=marked.get(cell.y*observation.width+cell.x).copied().flatten() else {continue;};
        cell.cogworks=true;
        cell.fg=crate::ability_ui::god_color(10);
        // Preserve independent pocket backgrounds and the underlying terrain.
        if cell.background_overlay.is_none() {
            cell.background_overlay=Some((crate::ability_ui::god_color(10),0.18));
        }
        cell.label=match kind {
            Kind::ClaySoldier=>{clay_glyph(cell);CLAY_LABEL.into()},
            Kind::Wall=>format!("Cogworks wall ({}/200 shared HP)",hp.unwrap_or(0.)),
            Kind::Pit=>format!("Cogworks {}",cell.label),
        };
    }
}

/// O(label bytes), detached values only. Used after animated terrain rebuilds.
pub fn restore_marker(cell: &mut Cell, original: &Cell) {
    if original.cogworks {
        cell.cogworks=true;
        cell.fg=original.fg;
        cell.background_overlay=original.background_overlay;
        cell.label=original.label.clone();
        if original.label==CLAY_LABEL {clay_glyph(cell);}
    }
    if original.glyph=="⚑" {cell.glyph=original.glyph.clone();cell.fg=original.fg;}
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::revision50_terrain::{Cell as Point,Modification};
    fn fixture() -> (Run,TerrainObservation,Vec<Cell>) {
        let mut run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=50.0.into();
        let mut observation=corpse_core::vision::observe_terrain(&run).unwrap();
        let at=observation.cells.iter().position(|o|o.current&&o.tile.is_some()).unwrap();
        let x=at%observation.width;let y=at/observation.width;
        observation.cells[at].tile=Some("chasm".into());
        run.floor_mut().unwrap().revision50_terrain=vec![Modification{kind:Kind::Pit,
            cells:vec![Point{x:x as i32,y:y as i32}],previous_tiles:vec!["floor".into()],hp:None}].into();
        (run,observation,vec![Cell{x,y,glyph:" ".into(),label:"chasm".into(),current:true,..Default::default()}])
    }
    #[test] fn current_color_survives_reconstruction_without_world_mutation() {
        let (run,observation,mut cells)=fixture();let before=run.clone();
        terrain(&run,&observation,&mut cells);
        assert!(cells[0].cogworks);assert_eq!(cells[0].fg,crate::ability_ui::god_color(10));
        let mut rebuilt=Cell::default();restore_marker(&mut rebuilt,&cells[0]);
        assert!(rebuilt.cogworks);assert_eq!(rebuilt.fg,cells[0].fg);
        assert_eq!(rebuilt.label,cells[0].label);assert_eq!(rebuilt.background_overlay,cells[0].background_overlay);
        assert_eq!(run,before);
    }
    #[test] fn remembered_and_unknown_cells_ignore_hidden_modification_changes() {
        let (mut run,mut observation,cells)=fixture();
        let at=cells[0].y*observation.width+cells[0].x;
        observation.cells[at].current=false;observation.cells[at].direct=false;
        let modifications=run.floor().unwrap().revision50_terrain.clone();
        for remembered in [true,false] {
            run.floor_mut().unwrap().revision50_terrain=modifications.clone();
            observation.cells[at].remembered=remembered;
            let mut first=cells.clone();terrain(&run,&observation,&mut first);
            run.floor_mut().unwrap().revision50_terrain=corpse_core::Field::Missing;
            let mut after=cells.clone();terrain(&run,&observation,&mut after);
            assert_eq!(first,after);assert_eq!(first,cells);
        }
    }
    #[test] fn legacy_revision_and_changed_terrain_do_not_get_authored_ink() {
        let (mut run,mut observation,cells)=fixture();
        run.simulation_revision=46.0.into();let mut old=cells.clone();terrain(&run,&observation,&mut old);assert_eq!(old,cells);
        run.simulation_revision=50.0.into();let at=cells[0].y*observation.width+cells[0].x;
        observation.cells[at].tile=Some("floor".into());let mut changed=cells.clone();terrain(&run,&observation,&mut changed);assert_eq!(changed,cells);
    }
    #[test] fn clay_layers_avatar_over_wall_glyph_through_reconstruction() {
        let (mut run,mut observation,mut cells)=fixture();
        run.floor_mut().unwrap().revision50_terrain.as_mut().unwrap()[0].kind=Kind::ClaySoldier;
        let at=cells[0].y*observation.width+cells[0].x;
        observation.cells[at].tile=Some("wall".into());cells[0].glyph="#".into();cells[0].label="wall".into();
        terrain(&run,&observation,&mut cells);
        assert_eq!(cells[0].label,CLAY_LABEL);assert_eq!(cells[0].glyph,"@");
        assert_eq!(cells[0].fg,CLAY_AVATAR_ORANGE);
        assert_eq!(cells[0].underlays.last().map(|g| (&g.0[..],g.1)),Some(("#",CLAY_WALL_ORANGE)));
        assert_eq!(cells[0].actor_id,None);
        let mut rebuilt=Cell{glyph:"#".into(),..Default::default()};restore_marker(&mut rebuilt,&cells[0]);
        assert_eq!(rebuilt.label,cells[0].label);assert_eq!(rebuilt.glyph,"@");assert!(rebuilt.cogworks);
        assert_eq!(rebuilt.emphasis,cells[0].emphasis);
        assert_eq!(rebuilt.underlays,cells[0].underlays);
    }
    #[test] fn visible_wall_reports_shared_pool_without_leaking_remembered_health() {
        let (mut run,mut observation,mut cells)=fixture();
        let m=&mut run.floor_mut().unwrap().revision50_terrain.as_mut().unwrap()[0];m.kind=Kind::Wall;m.hp=Some(163.);
        let at=cells[0].y*observation.width+cells[0].x;observation.cells[at].tile=Some("wall".into());
        terrain(&run,&observation,&mut cells);assert_eq!(cells[0].label,"Cogworks wall (163/200 shared HP)");
        let remembered=cells.clone();observation.cells[at].current=false;
        run.floor_mut().unwrap().revision50_terrain.as_mut().unwrap()[0].hp=Some(7.);
        terrain(&run,&observation,&mut cells);assert_eq!(cells,remembered);
    }

    #[test]
    fn native_clay_soldier_overlay_after_ability_command() {
        let mut run=corpse_core::startup::create_avatar_run("10-03-09:clay-demo").unwrap();
        let ability=corpse_core::revision50_terrain::CLAY_ARMY;
        if !run.player.abilities.iter().any(|id| id==ability) {
            run.player.abilities.push(ability.into());
        }
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut run).unwrap();
        run.player.energy=run.player.energy.max(21.);
        let mut engine=corpse_core::temporal::Engine::new(run).unwrap();
        let (from_x, from_y)=(engine.run.player.x as usize,engine.run.player.y as usize);
        let (to_x, to_y)=corpse_core::movement::DIRECTIONS.iter().find_map(|d| {
            let x=from_x as i32+d.x;let y=from_y as i32+d.y;
            corpse_core::revision50_terrain::clay_preview_for_run(&engine.run,x as f64,y as f64)
                .ok().map(|_|(x,y))
        }).expect("starting room has a valid Clay Army step");
        engine.dispatch(&corpse_core::Command::Ability {
            id:ability.into(),target:corpse_core::Field::Missing,x:(to_x as f64).into(),y:(to_y as f64).into(),
        }).unwrap();
        assert_eq!((engine.run.player.x,engine.run.player.y),(to_x as f64,to_y as f64));
        let view=crate::observe::observe(&engine.run);
        let soldier=view.cells.iter().take(view.terrain_count).find(|c| c.x==from_x && c.y==from_y).unwrap();
        assert_eq!(soldier.label,CLAY_LABEL);
        assert_eq!(soldier.glyph,"@");
        assert_eq!(soldier.underlays.last().map(|g| g.0.as_str()),Some("#"));
        assert!(soldier.actor_id.is_none());
        assert!(!view.creatures.iter().any(|c| c.name.starts_with("Clay Soldier")));

        // Native framebuffer, not a standalone glyph drawing. Live animated
        // terrain rebuild must carry the same centered overlay.
        let mut painter=crate::render::Painter::new();
        painter.resize(2560,1600);
        painter.set_responsive_canvas(true);
        painter.game_at(&view,"",false,1250.);
        if let Ok(directory)=std::env::var("CORPSE_CLAY_CAPTURE") {
            let write_png=|name:&str,w:u32,h:u32,pixels:&[u8]| {
                let file=std::fs::File::create(format!("{directory}/{name}.png")).unwrap();
                let mut png=png::Encoder::new(file,w,h);
                png.set_color(png::ColorType::Rgba);png.set_depth(png::BitDepth::Eight);
                png.write_header().unwrap().write_image_data(pixels).unwrap();
            };
            write_png("Clay_Army",painter.width as u32,painter.height as u32,&painter.pixels);
            let (cw,ch,ox,oy)=painter.map_layout(view.width,view.height);
            let x=ox+cw*from_x as i32;let y=oy+ch*from_y as i32;
            eprintln!("Clay tile: x={} y={} w={} h={}",x*2,y*2,cw*2,ch*2);
        }
    }
}

pub fn bubble_hit(x:i32,y:i32,left:i32,top:i32)->Option<u8>{
 (0..3).find(|i|{let dx=x-(left+23+i*16);let dy=y-(top+11);dx*dx+dy*dy<=49}).map(|i|i as u8+1)
}
/// Draw only on a tile already represented by the knowledge-gated projection.
/// Preserve the highest-priority entity and its existing rank marker.
pub fn rally(run:&Run,cells:&mut [Cell]){
 let Some(flag)=run.rally.as_ref() else{return;};
 if !flag.x.is_finite()||!flag.y.is_finite()||flag.x<0.||flag.y<0.{return;}
 let (x,y)=(flag.x as usize,flag.y as usize);
 let count=cells.iter().filter(|c|c.x==x&&c.y==y).count();
 if let Some(cell)=cells.iter_mut().rev().find(|c|c.x==x&&c.y==y){
  crate::hallucination_ui::map_cell(cell,|c|{if count>1{c.superscript.push('⚑');}else{c.glyph="⚑".into();c.fg=[255,209,57];}});
 }
}

#[cfg(test)]mod marker_tests {
 use super::*;
 #[test]fn bubble_hits_are_distinct(){for i in 0..3{assert_eq!(bubble_hit(23+i*16,11,0,0),Some(i as u8+1));}assert_eq!(bubble_hit(80,11,0,0),None);}
 #[test]fn rally_keeps_entity_rank_and_unknown_space_hidden(){let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.rally=Some(corpse_core::RunRally{x:2.,y:3.});let terrain=Cell{x:2,y:3,glyph:".".into(),..Default::default()};let mut cells=vec![terrain.clone(),Cell{x:2,y:3,glyph:"B".into(),superscript:"2".into(),..Default::default()}];rally(&r,&mut cells);assert_eq!(cells[1].glyph,"B");assert_eq!(cells[1].superscript,"2⚑");assert_eq!(cells[0].glyph,".");let mut bare=vec![terrain];rally(&r,&mut bare);assert_eq!(bare[0].glyph,"⚑");let mut rebuilt=Cell::default();restore_marker(&mut rebuilt,&bare[0]);assert_eq!(rebuilt.glyph,"⚑");r.rally.as_mut().unwrap().x=50.;let before=cells.clone();rally(&r,&mut cells);assert_eq!(before,cells);}
}

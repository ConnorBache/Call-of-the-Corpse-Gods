//! Knowledge-gated pocket presentation. All outcomes are possibilities, never RNG draws.
use corpse_core::{Actor, Run, vision, non_euclidean_generation as pockets};
use crate::render::{Cell, Rgb};

pub const INK: Rgb = [95, 211, 211];
pub fn enabled(run: &Run) -> bool { run.simulation_revision.value_or(29.) >= 36. }

/// O(C*G*S), bounded saved groups (G<=384, S<=2). r51 uses matching
/// integer glyphs, never portal glyph 0, and teal ground for Speed-up terrain.
/// Both knowledge and material come from this observation, not hidden live tiles.
pub fn terrain(run: &Run, observation: &vision::TerrainObservation, cells: &mut [Cell]) {
    if !enabled(run) { return; }
    let Some(f) = run.floor() else { return; };
    for cell in cells {
        let at = cell.y * observation.width + cell.x;
        let Some(o) = observation.cells.get(at).filter(|o| o.current || o.remembered) else { continue; };
        if observed_group(run, observation, at).is_none() { continue; }
        cell.non_euclidean = true;
        let fast = pockets::speedup_at(f, at).is_some();
        if run.simulation_revision.value_or(29.) >= 51. {
            cell.glyph = pockets::group_number(f, at).unwrap().to_string();
            cell.fg = crate::color::composite(cell.bg, INK, if o.current { 0.95 } else { 0.38 });
            cell.bottom_aligned = !fast;
            if fast {
                // This is the material itself, not an overlay of grass/water.
                cell.background_overlay = None;
                cell.label = "Speed-up area".into();
            } else {
                cell.background_overlay = Some((INK, if o.current { 0.22 } else { 0.09 }));
                cell.label = "Non-Euclidean space".into();
            }
        } else {
            cell.background_overlay = Some((if fast { [139, 224, 147] } else { INK }, if o.current { 0.22 } else { 0.09 }));
            cell.label = format!("{} {}", if fast { "Speed-up" } else { "Non-Euclidean" }, cell.label);
        }
    }
}
/// Restore detached topology ink after animated material reconstruction. The
/// historical overlay continues to retain its animated underlying glyph/color.
pub fn restore_marker(cell: &mut Cell, original: &Cell) {
    cell.non_euclidean = true;
    cell.background_overlay = original.background_overlay;
    cell.label = original.label.clone();
    if original.glyph.parse::<usize>().is_ok() {
        cell.glyph = original.glyph.clone();
        cell.fg = original.fg;
        cell.bottom_aligned = original.bottom_aligned;
    }
}
/// Stable assignment metadata is not knowledge of its current terrain. Only
/// this endpoint's observed/remembered tile can identify it as Speed-up space.
fn observed_group<'a>(run: &'a Run, observation: &vision::TerrainObservation, at: usize)
    -> Option<&'a pockets::NonEuclideanGroup>
{
    let f = run.floor()?;
    let o = observation.cells.get(at).filter(|o|o.current || o.remembered)?;
    if f.speed_up_terrain.value_or(false) && pockets::speedup_at(f, at).is_some()
        && o.tile.as_deref() != Some("speed-up") { return None; }
    pockets::assigned_group_at(f, at)
}

pub fn known(observation: &vision::TerrainObservation, at: usize) -> bool {
    observation.cells.get(at).is_some_and(|o| o.current || o.remembered)
}
/// Only return coordinates already observed. A hidden exit is never materialized
/// into a preview, even when its group is present in the serialized floor.
pub fn observed_exits(run: &Run, observation: &vision::TerrainObservation, at: usize) -> Vec<usize> {
    if !enabled(run) || !known(observation, at) { return vec![]; }
    let Some(f) = run.floor() else { return vec![]; };
    let Some(group) = observed_group(run, observation, at) else { return vec![]; };
    let exits = if at == group.destination { group.sources.clone() } else { vec![group.destination] };
    let fast = f.speed_up_terrain.value_or(false) && pockets::speedup_at(f, at).is_some();
    exits.into_iter().filter(|&exit| known(observation, exit)
        && (!fast || observation.cells[exit].tile.as_deref() == Some("speed-up"))).collect()
}
/// Walking-only local reversal, using the same observed terrain as the exits.
pub(crate) fn observed_walking_exits(run: &Run, observation: &vision::TerrainObservation, from: usize, at: usize) -> Vec<usize> {
    let exits = observed_exits(run, observation, at);
    if run.simulation_revision.value_or(29.) >= 37. && exits.contains(&from)
        && run.floor().is_some_and(|f| pockets::speedup_at(f, at).is_some()) {
        vec![at]
    } else { exits }
}
/// Per hover: O(D + F + P*log C + G*S + E) before observation, with O(E)
/// scratch. D is dream projection (borrow normally; clone/layer cost when active),
/// F floor lookup, P pockets, C cells/pocket, G selected pocket groups, S sources/
/// group, E exits. No links/disabled: O(D+F); unrelated tile skips full observation.
/// Relevant exits additionally pay vision::observe_terrain time/memory (see its
/// transitive bound); knowledge filtering and coordinate mapping are O(E).
pub fn possible_exit_marks(run: &Run, x: usize, y: usize) -> Vec<(usize, usize)> {
    possible_exit_marks_inner(run, x, y, None)
}
/// Reuse terrain from this exact unchanged world's dream projection, never a prior tick.
pub fn possible_exit_marks_with_terrain(run: &Run, x: usize, y: usize, terrain: &vision::TerrainObservation) -> Vec<(usize, usize)> {
    possible_exit_marks_inner(run, x, y, Some(terrain))
}
fn possible_exit_marks_inner(run: &Run, x: usize, y: usize, terrain: Option<&vision::TerrainObservation>) -> Vec<(usize, usize)> {
    let _timing = corpse_core::timing::span("desktop.possible_exits");
    let view = crate::dream_ui::observation_run(run); let run = view.as_ref();
    let Some(f) = run.floor() else { return vec![]; };
    if x >= f.width() || y >= f.height() { return vec![]; }
    if !enabled(run) { return vec![]; }
    let at = y*f.width()+x;
    // Topology is only an applicability test, never knowledge: do not disclose
    // any endpoint until the unchanged observation gates approve both ends.
    if pockets::assigned_group_at(f, at).is_none() { return vec![]; }
    let owned;
    let o = match terrain.filter(|o| o.width == f.width() && o.height == f.height() && o.cells.len() == f.tiles.len()) {
        Some(o) => o,
        None => { owned = vision::observe_terrain(run); let Some(o) = owned.as_ref() else { return vec![]; }; o }
    };
    if !known(&o, at) { return vec![]; }
    observed_exits(run, &o, at).into_iter()
        .map(|at| (at%f.width(), at/f.width())).collect()
}
pub fn description(run: &Run, observation: &vision::TerrainObservation, x: usize, y: usize) -> Option<String> {
    if !enabled(run) { return None; }
    let f = run.floor()?; let at = y.checked_mul(f.width())?.checked_add(x)?;
    if x >= f.width() || y >= f.height() || !known(observation, at) { return None; }
    let group = observed_group(run, observation, at)?;
    let all = if at == group.destination { group.sources.clone() } else { vec![group.destination] };
    let exits = observed_exits(run, observation, at);
    let fast = f.speed_up.as_ref().into_iter().flatten().any(|p|p.cells.contains(&at));
    let mut text = if fast { "Speed-up area." } else { "Non-Euclidean space." }.to_string();
    if run.simulation_revision.value_or(29.) >= 51. {
        text.push_str(&format!(" Link {}. Matching numbers mark the same linked space.", pockets::group_number(f, at)?));
    }
    if group.destination == at && group.sources.len() == 2 {
        text.push_str(" Random return: either source is equally likely.");
    }
    if !exits.is_empty() {
        let coordinates: Vec<_> = exits.iter().map(|at| format!("({}, {})", at%f.width(), at/f.width())).collect();
        text.push_str(&format!(" Known possible {}: {}.", if exits.len()==1 { "exit" } else { "exits" }, coordinates.join(" or ")));
    }
    if all.iter().any(|&exit| !known(observation, exit)) { text.push_str(" An attached exit is unexplored."); }
    else if exits.len() < all.len() { text.push_str(" A paired tile has been replaced; the link is inactive."); }
    if fast {
        text.push_str(" Steps between paired endpoints stay local. Vision stays local.");
    } else {
        text.push_str(" Stepping here crosses once; vision stays local. Auto-travel stops at random choices.");
    }
    Some(text)
}

/// The alias identifies an already visible same-plane actor, not projected sight.
pub fn observed_alias<'a>(run: &'a Run, x: usize, y: usize) -> Option<&'a Actor> {
    observed_alias_inner(run, x, y, None)
}
/// The supplied terrain must belong to this same projected Run and world generation.
pub fn observed_alias_with_terrain<'a>(run: &'a Run, x: usize, y: usize, terrain: &vision::TerrainObservation) -> Option<&'a Actor> {
    observed_alias_inner(run, x, y, Some(terrain))
}
fn observed_alias_inner<'a>(run: &'a Run, x: usize, y: usize, terrain: Option<&vision::TerrainObservation>) -> Option<&'a Actor> {
    if !enabled(run) { return None; }
    let f = run.floor()?; if x>=f.width() || y>=f.height() { return None; }
    let at = y*f.width()+x; let g = pockets::group_at(f, at)?;
    let owned;
    let o = match terrain.filter(|o| o.width == f.width() && o.height == f.height() && o.cells.len() == f.tiles.len()) {
        Some(o) => o,
        None => { owned = vision::observe_terrain(run)?; &owned }
    };
    if !o.cells.get(at)?.current { return None; }
    run.actors_at(run.depth).iter().find(|a| {
        let pos = a.y as usize*f.width()+a.x as usize;
        pos != at && (g.sources.contains(&pos) || g.destination==pos)
            && !a.dead.value_or(false) && a.hp>0.
            && corpse_core::dreamwalking::same_plane(run, &run.player, a)
            && vision::visible_actor(run, a)
    })
}
/// Both physical cells require direct observation. Aliases do not add item identities.
pub fn observed_item_aliases(run: &Run, observation: &vision::TerrainObservation, at: usize) -> Vec<usize> {
    if !enabled(run) || !observation.cells.get(at).is_some_and(|o| o.direct) { return vec![]; }
    let Some(f) = run.floor() else { return vec![]; };
    let Some(g) = pockets::group_at(f, at) else { return vec![]; };
    g.sources.iter().copied().chain(std::iter::once(g.destination))
        .filter(|&other| other!=at && observation.cells.get(other).is_some_and(|o| o.direct)).collect()
}
pub fn actor_alias_cells(run: &Run, observation: &vision::TerrainObservation, at: usize) -> Vec<usize> {
    if !enabled(run) { return vec![]; }
    let Some(f) = run.floor() else { return vec![]; };
    let Some(g) = pockets::group_at(f, at) else { return vec![]; };
    g.sources.iter().copied().chain(std::iter::once(g.destination))
        .filter(|&other| other!=at && observation.cells.get(other).is_some_and(|o| o.current)).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use corpse_core::{Field, non_euclidean_generation::{NonEuclideanPocket,NonEuclideanGroup}};
    pub(crate) fn scene() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=36.0.into();r.player.x=10.;r.player.y=10.;
        r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        r.actors.clear();r.ground.clear();r.offers.clear();
        let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.depth=20.;f.lights=vec![100.;n];f.seen=vec![true;n];f.remembered_tiles=f.tiles.clone().into();
        f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.portals=Field::Missing;
        f.non_euclidean=vec![NonEuclideanPocket{cells:(11..19).map(|x|10*w+x).collect(),groups:vec![
            NonEuclideanGroup{sources:vec![10*w+11,10*w+12],destination:10*w+13},
            NonEuclideanGroup{sources:vec![10*w+14,10*w+15],destination:10*w+16},
            NonEuclideanGroup{sources:vec![10*w+17],destination:10*w+18},
        ]}].into();r.depth=20.;r
    }
    /// Frozen v15 order: O(D + F + observation + P*log C + G*S + E)
    /// time and observation/E scratch; used only as a differential oracle.
    fn possible_exit_marks_v15(run: &Run, x: usize, y: usize) -> Vec<(usize, usize)> {
        let view = crate::dream_ui::observation_run(run); let run = view.as_ref();
        let Some(f) = run.floor() else { return vec![]; };
        if x >= f.width() || y >= f.height() { return vec![]; }
        let Some(o) = vision::observe_terrain(run) else { return vec![]; };
        observed_exits(run, &o, y*f.width()+x).into_iter()
            .map(|at| (at%f.width(), at/f.width())).collect()
    }

    /// O(Q*(D+F+observation+topology)) time; O(run+observation) scratch for
    /// fixed Q hover samples. Covers each fast path and relevant projected views.
    #[test]
    fn possible_exit_sequencing_matches_v15_across_views_and_topology() {
        let original=scene();
        let mut cases=vec![original.clone()];
        let mut absent=original.clone();
        absent.floor_mut().unwrap().non_euclidean=Field::Missing;
        cases.push(absent);
        let mut legacy=original.clone();legacy.simulation_revision=35.0.into();cases.push(legacy);
        let mut speed=original.clone();
        let f=speed.floor_mut().unwrap();f.speed_up=f.non_euclidean.clone();f.non_euclidean=Field::Missing;
        cases.push(speed);
        let mut portal=original.clone();let w=portal.floor().unwrap().width();
        portal.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair {
            a:10*w+11,b:10*w+45,color_index:0
        }].into();cases.push(portal);
        let mut unseen=original.clone();
        let f=unseen.floor_mut().unwrap();let w=f.width();
        for y in 1..f.height()-1 {f.tiles[y*w+12]="wall".into();}
        f.seen.fill(false);f.remembered_tiles=Field::Missing;
        cases.push(unseen.clone());
        let f=unseen.floor_mut().unwrap();f.seen.fill(true);f.remembered_tiles=f.tiles.clone().into();
        cases.push(unseen);
        // Dream projection can have different topology; never test the waking
        // floor for applicability before selecting the actual observation run.
        let mut dream=original.clone();
        dream.dreamwalking=corpse_core::dreamwalking::State {
            active:true,depth:20.,anchor_x:10.,anchor_y:10.,..Default::default()
        }.into();
        corpse_core::dreamwalking::ensure_layer(&mut dream);
        dream.floor_mut().unwrap().non_euclidean=Field::Missing;
        cases.push(dream.clone());
        corpse_core::dreamwalking::with_plane(&mut dream,true,|r| {
            r.floor_mut().unwrap().non_euclidean=Field::Missing;
        });
        dream.floor_mut().unwrap().non_euclidean=original.floor().unwrap().non_euclidean.clone();
        cases.push(dream);
        for (case,r) in cases.iter().enumerate() {
            let before=r.clone();
            let view=crate::observe::observe(r);let terrain=view.terrain_snapshot.as_deref().unwrap();
            for (x,y) in [(10,10),(11,10),(13,10),(18,10),(usize::MAX,10)] {
                assert_eq!(possible_exit_marks_with_terrain(r,x,y,terrain),possible_exit_marks_v15(r,x,y),
                    "supplied case {case}, hover ({x},{y})");
                assert_eq!(possible_exit_marks(r,x,y),possible_exit_marks_v15(r,x,y),
                    "case {case}, hover ({x},{y})");
            }
            assert_eq!(r,&before,"hover must not mutate state/RNG: case {case}");
        }
    }

    /// Fixed-case differential plus O(observation) gates; O(run+observation)
    /// memory. Hidden topology is never sufficient to expose an endpoint.
    #[test]
    fn possible_exit_sequencing_preserves_hidden_and_remembered_endpoints() {
        let mut r=scene();let f=r.floor_mut().unwrap();let w=f.width();
        for y in 1..f.height()-1 {f.tiles[y*w+12]="wall".into();}
        f.seen.fill(false);f.remembered_tiles=Field::Missing;
        assert!(possible_exit_marks(&r,11,10).is_empty());
        assert!(possible_exit_marks(&r,13,10).is_empty());
        let f=r.floor_mut().unwrap();f.seen[10*w+13]=true;
        f.remembered_tiles=f.tiles.clone().into();
        assert_eq!(possible_exit_marks(&r,11,10),vec![(13,10)]);
        assert_eq!(possible_exit_marks(&r,11,10),possible_exit_marks_v15(&r,11,10));
    }

    /// O(Q*observation) test time and O(run+observation+L) scratch, Q fixed;
    /// diagnostic span counts prove the fast path omits the costly operation.
    #[cfg(feature="runtime-timing")]
    #[test]
    fn possible_exit_sequencing_skips_observation_when_inapplicable() {
        let mut r=scene();
        for (x,y,expected) in [(10,10,0),(13,10,1),(usize::MAX,10,0)] {
            corpse_core::timing::reset();corpse_core::timing::set_enabled(true);
            let _=possible_exit_marks(&r,x,y);
            let report=corpse_core::timing::take_report();corpse_core::timing::set_enabled(false);
            assert_eq!(report.get("vision.observe_terrain").map_or(0,|t|t.calls),expected);
        }
        r.floor_mut().unwrap().non_euclidean=Field::Missing;
        corpse_core::timing::reset();corpse_core::timing::set_enabled(true);
        let _=possible_exit_marks(&r,13,10);
        let after=corpse_core::timing::take_report();
        let _=possible_exit_marks_v15(&r,13,10);
        let before=corpse_core::timing::take_report();corpse_core::timing::set_enabled(false);
        assert!(!after.contains_key("vision.observe_terrain"));
        assert_eq!(before["vision.observe_terrain"].calls,1);
    }

    #[test]
    fn non_euclidean_ui_marker_preserves_terrain_and_known_exits_without_rng() {
        let r=scene();let before=r.clone();let v=crate::observe::observe(&r);
        let cell=v.cells.iter().find(|c|c.x==13&&c.y==10).unwrap();
        assert!(cell.non_euclidean);assert_eq!(cell.glyph,".");assert!(!cell.italic);
        let o=vision::observe_terrain(&r).unwrap();let w=o.width;
        let text=description(&r,&o,13,10).unwrap();
        assert!(text.contains("equally likely"));assert!(text.contains("(11, 10) or (12, 10)"));
        assert_eq!(possible_exit_marks(&r,13,10),vec![(11,10),(12,10)]);
        for _ in 0..4 {let _=crate::observe::observe(&r);let _=possible_exit_marks(&r,13,10);}
        assert_eq!(r,before);
        let mut hidden=o;hidden.cells[10*w+12].current=false;hidden.cells[10*w+12].remembered=false;
        assert_eq!(observed_exits(&r,&hidden,10*w+13),vec![10*w+11]);
        assert!(!description(&r,&hidden,13,10).unwrap().contains("(12, 10)"));
        hidden.cells[10*w+13].current=false;hidden.cells[10*w+13].remembered=false;
        assert!(description(&r,&hidden,13,10).is_none());
    }
    #[test]
    fn non_euclidean_ui_aliases_keep_actor_and_item_identity_single() {
        let mut r=scene();let a=corpse_core::population::create_actor_revision("rat",900.,13.,10.,false,36.).unwrap();
        r.actors_at_mut(r.depth).push(a);
        let item=corpse_core::equipment::make_equipment("Ring",901.).unwrap();let name=item.name.clone();
        r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:16.,y:10.,item,..Default::default()});
        let before=r.clone();let v=crate::observe::observe(&r);
        assert_eq!(v.creatures.iter().filter(|c|c.id==900.).count(),1);
        let aliases:Vec<_>=v.cells.iter().filter(|c|c.actor_id==Some(900.)).collect();assert_eq!(aliases.len(),3);
        assert!(aliases.iter().any(|c|c.x==11));assert!(aliases.iter().any(|c|c.x==12));
        assert!(crate::inspection_ui::observe_tile(&r,11,10).scene.contains("already visible"));
        assert!(v.cells.iter().any(|c|c.x==14&&c.label==name));
        assert!(crate::inspection_ui::observe_tile(&r,14,10).scene.contains("linked Non-Euclidean"));
        assert_eq!(r,before);assert_eq!(r.ground.get(&r.depth.to_string()).unwrap().len(),1);
        let mut o=vision::observe_terrain(&r).unwrap();let w=o.width;o.cells[10*w+14].direct=false;
        assert!(!observed_item_aliases(&r,&o,10*w+16).contains(&(10*w+14)));
    }
    #[test]
    fn non_euclidean_ui_no_projected_sight_and_legacy_gate() {
        let mut r=scene();let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
        f.non_euclidean=vec![NonEuclideanPocket{cells:(11..19).map(|x|10*w+x).collect(),groups:vec![
            NonEuclideanGroup{sources:vec![10*w+11,10*w+12],destination:10*w+18},
            NonEuclideanGroup{sources:vec![10*w+17,10*w+16],destination:10*w+13},
            NonEuclideanGroup{sources:vec![10*w+14],destination:10*w+15},
        ]}].into();
        for y in 1..f.height()-1 {f.tiles[y*w+15]="wall".into();}
        f.seen=vec![false;n];f.remembered_tiles=Field::Missing;
        let a=corpse_core::population::create_actor_revision("rat",900.,18.,10.,false,36.).unwrap();r.actors_at_mut(r.depth).push(a);
        let before=r.clone();assert!(!vision::visible_actor(&r,&r.actors_at(r.depth)[0]));
        assert!(observed_alias(&r,11,10).is_none());
        assert!(!crate::observe::observe(&r).cells.iter().any(|c|c.actor_id==Some(900.)));
        assert_eq!(r,before);
        r.simulation_revision=35.0.into();assert!(!crate::observe::observe(&r).cells.iter().any(|c|c.non_euclidean));
    }
    #[test]
    fn non_euclidean_ui_actual_renderer_controlled_capture() {
        let r=scene();let before=r.clone();let mut v=crate::observe::observe(&r);
        v.provenance="CONTROLLED r36 pocket topology fixture; not generated gameplay".into();
        v.hover=Some((13,10));v.possible_exits=possible_exit_marks(&r,13,10);
        let report=crate::inspection_ui::observe_tile(&r,13,10);
        let mut p=crate::render::Painter::new();p.game_at(&v,&report.scene,false,824.);
        let marked=p.pixels.clone();
        if let Ok(dir)=std::env::var("CORPSE_NON_EUCLIDEAN_UI_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/non-euclidean-controlled-run.json"),serde_json::to_vec_pretty(&r).unwrap()).unwrap();
            let file=std::fs::File::create(format!("{dir}/non-euclidean-possible-exits-controlled.png")).unwrap();
            let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
        for c in &mut v.cells {if c.non_euclidean {c.background_overlay=None;c.non_euclidean=false;}}
        v.possible_exits.clear();p.game_at(&v,&report.scene,false,824.);assert_ne!(marked,p.pixels);assert_eq!(r,before);
    }
    #[test]
    fn non_euclidean_ui_generated_debug_capture() {
        let Ok(dir)=std::env::var("CORPSE_NON_EUCLIDEAN_GENERATED_REVIEW") else {return;};
        let mut r=corpse_core::startup::create_run_revision("non-euclidean-review-r36","Unblessed",true,36.).unwrap();
        corpse_core::startup::ensure_floors(&mut r,20).unwrap();
        let (depth,at,w)=r.floors.iter().filter(|f|f.depth>=20.).find_map(|f| {
            let pocket=f.non_euclidean.as_ref()?.first()?;
            let at=*pocket.cells.first()?;Some((f.depth,at,f.width()))
        }).expect("review seed has a generated pocket");
        r.depth=depth;r.offers.clear();
        let f=r.floor().unwrap();let adjacent=[at.saturating_sub(1),at+1,at.saturating_sub(w),at+w].into_iter()
            .find(|&i|i<f.tiles.len()&&matches!(f.tiles[i].as_str(),"floor"|"grass"|"shallow-water")&&pockets::group_at(f,i).is_none())
            .unwrap_or(at);
        r.player.x=(adjacent%w)as f64;r.player.y=(adjacent/w)as f64;
        r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),depth);
        let mut v=crate::observe::observe(&r);v.provenance="DEBUG generated r36 map; controlled player placement and full reveal; no turns simulated".into();
        v.hover=Some((at%w,at/w));v.possible_exits=possible_exit_marks(&r,at%w,at/w);
        assert!(v.cells.iter().any(|c|c.non_euclidean));
        let mut painter=crate::render::Painter::new();painter.game_at(&v,&format!("DEBUG: generated floor {depth}; cyan cells are Non-Euclidean; corner marks show known possible exits."),false,824.);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/non-euclidean-generated-debug-run.json"),serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        let file=std::fs::File::create(format!("{dir}/non-euclidean-generated-floor{}-debug.png",depth as u32)).unwrap();
        let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
    }

    #[test]
    fn non_euclidean_ui_aliases_remain_in_the_viewed_plane() {
        let mut r=scene();
        let rat=corpse_core::population::create_actor_revision("rat",900.,13.,10.,false,36.).unwrap();
        let nightmare=corpse_core::phantasmii_generation::nightmare(20,901.,13.,10.);
        r.actors_at_mut(r.depth).extend([rat,nightmare]);
        let awake=crate::observe::observe(&r);
        assert_eq!(awake.cells.iter().filter(|c|c.actor_id==Some(900.)).count(),3);
        assert!(!awake.cells.iter().any(|c|c.actor_id==Some(901.)));
        r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:20.,anchor_x:10.,anchor_y:10.,..Default::default()}.into();
        corpse_core::dreamwalking::ensure_layer(&mut r);let before=r.clone();
        let dreaming=crate::observe::observe(&r);
        assert_eq!(dreaming.cells.iter().filter(|c|c.actor_id==Some(900.)).count(),1);
        assert_eq!(dreaming.cells.iter().filter(|c|c.actor_id==Some(901.)).count(),3);
        assert_eq!(r,before);
    }


    #[test]
    fn supplied_terrain_alias_preserves_visibility_gates() {
        let mut r=scene();
        r.actors_at_mut(r.depth).push(corpse_core::population::create_actor_revision("rat",900.,13.,10.,false,43.).unwrap());
        let before=serde_json::to_vec(&r).unwrap();
        let projected=crate::dream_ui::observation_run(&r);let r=projected.as_ref();
        let mut o=vision::observe_terrain(r).unwrap();
        for x in [10,11,12,13,14,45,60] {
            assert_eq!(observed_alias_with_terrain(r,x,10,&o).map(|a|a.id),observed_alias(r,x,10).map(|a|a.id));
            assert_eq!(possible_exit_marks_with_terrain(r,x,10,&o),possible_exit_marks(r,x,10));
        }
        let at=10*o.width+11;o.cells[at].current=false;
        assert!(observed_alias_with_terrain(r,11,10,&o).is_none());
        assert_eq!(serde_json::to_vec(r).unwrap(),before);
    }
}

#[cfg(test)]
mod revision51_tests {
    use super::*;
    use corpse_core::non_euclidean_generation::{NonEuclideanGroup as Group, NonEuclideanPocket as Pocket};
    pub(super) fn scene() -> Run {
        let mut r=super::tests::scene();r.simulation_revision=51.0.into();
        let f=r.floor_mut().unwrap();let w=f.width();
        f.speed_up=vec![Pocket{cells:(11..=16).map(|x|12*w+x).collect(),groups:(11..=15).step_by(2)
            .map(|x|Group{sources:vec![12*w+x],destination:12*w+x+1}).collect()}].into();
        pockets::install_speedup_terrain(f);f.remembered_tiles=f.tiles.clone().into();r
    }
    #[test]
    fn revision51_speed_up_is_teal_material_not_a_modifier() {
        let r=scene();let before=r.clone();let v=crate::observe::observe(&r);
        let at=12*v.width+11;let o=&v.terrain_snapshot.as_ref().unwrap().cells[at];
        assert_eq!(o.tile.as_deref(),Some("speed-up"));
        let c=v.cells.iter().find(|c|c.x==11&&c.y==12).unwrap();
        assert_eq!(c.label,"Speed-up area");assert!(c.non_euclidean);assert!(!c.italic);
        assert!(c.background_overlay.is_none());assert!(c.fg[1]>c.fg[0]&&c.fg[2]>c.fg[0]);
        assert!(!c.bottom_aligned);assert_eq!(r,before);
        assert_eq!(crate::inspection_ui::observe_tile(&r,11,12).title,"Speed-up area");
    }
    #[test]
    fn revision51_speed_up_hidden_replacement_does_not_change_memory_or_exit_preview() {
        let mut r=scene();let o=vision::observe_terrain(&r).unwrap();let at=12*o.width+11;
        let old_exits=observed_exits(&r,&o,at);let old_text=description(&r,&o,11,12);
        r.floor_mut().unwrap().tiles[at+1]="wall".into();
        assert_eq!(observed_exits(&r,&o,at),old_exits);assert_eq!(description(&r,&o,11,12),old_text);
        let mut hidden=o.clone();hidden.cells[at+1].current=false;hidden.cells[at+1].remembered=false;hidden.cells[at+1].tile=None;
        assert!(observed_exits(&r,&hidden,at).is_empty());
        assert!(!description(&r,&hidden,11,12).unwrap().contains("(12, 12)"));
        let fresh=vision::observe_terrain(&r).unwrap();assert!(observed_exits(&r,&fresh,at).is_empty());
        let mut own_changed=o.clone();own_changed.cells[at].tile=Some("grass".into());
        assert!(description(&r,&own_changed,11,12).is_none());
    }
    #[test]
    fn revision51_speed_up_item_and_actor_layers_keep_one_identity_and_teal_ground() {
        let mut r=scene();let item=corpse_core::equipment::make_equipment("Ring",901.).unwrap();
        r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:13.,y:12.,item,..Default::default()});
        r.actors_at_mut(r.depth).push(corpse_core::population::create_actor_revision("rat",902.,15.,12.,false,51.).unwrap());
        let before=r.clone();let v=crate::observe::observe(&r);
        assert_eq!(v.creatures.iter().filter(|c|c.id==902.).count(),1);
        for x in [13,14,15,16] {let ground=v.cells.iter().take(v.terrain_count).find(|c|c.x==x&&c.y==12).unwrap();
            let overlay=v.cells.iter().skip(v.terrain_count).find(|c|c.x==x&&c.y==12).unwrap();
            assert_eq!(overlay.bg,ground.bg);}
        assert_eq!(r,before);assert_eq!(r.ground["20"].len(),1);
    }
    #[test]
    fn revision51_speed_up_animated_renderer_preserves_numbered_tiles() {
        let r=scene();let before=r.clone();let v=crate::observe::observe(&r);
        for original in v.cells.iter().take(v.terrain_count).filter(|c|c.non_euclidean) {
            let mut restored=Cell::default();restore_marker(&mut restored,original);
            assert_eq!(restored.glyph,original.glyph);assert_eq!(restored.fg,original.fg);
        }
        if let Ok(dir)=std::env::var("CORPSE_SPEED_UP_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut p=crate::render::Painter::new();
            for time in [0.,824.] {p.game_at(&v,"",false,time);
                let file=std::fs::File::create(format!("{dir}/speed-up-own-tiles-{time}.png")).unwrap();
                let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
                encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();}
            std::fs::write(format!("{dir}/speed-up-controlled-run.json"),serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        }
        assert_eq!(r,before);
    }
}

#[cfg(test)]
mod revision52_tests {
    use super::*;

    fn scene() -> Run {
        let mut r=super::revision51_tests::scene();r.simulation_revision=52.0.into();r
    }

    #[test]
    fn revision52_speed_up_numbers_match_actual_destinations_not_whole_areas() {
        let r=scene();let before=r.clone();let view=crate::observe::observe(&r);
        let observation=view.terrain_snapshot.as_ref().unwrap();let f=r.floor().unwrap();
        for pocket in f.non_euclidean.as_ref().into_iter().flatten()
            .chain(f.speed_up.as_ref().into_iter().flatten()) {
            for group in &pocket.groups {
                let number=pockets::group_number(f,group.destination).unwrap().to_string();
                for at in group.sources.iter().copied().chain(std::iter::once(group.destination)) {
                    let cell=view.cells.iter().take(view.terrain_count)
                        .find(|c|c.y*view.width+c.x==at).unwrap();
                    assert_eq!(cell.glyph,number);
                    let destinations=observed_exits(&r,observation,at);
                    assert_eq!(destinations,pockets::exits(f,at));
                    for exit in destinations {
                        let destination=view.cells.iter().take(view.terrain_count)
                            .find(|c|c.y*view.width+c.x==exit).unwrap();
                        assert_eq!(destination.glyph,number);
                    }
                }
            }
        }
        let speed:Vec<_>=view.cells.iter().take(view.terrain_count)
            .filter(|c|c.y==12&&(11..=16).contains(&c.x)).map(|c|c.glyph.as_str()).collect();
        assert_eq!(speed,vec!["4","4","5","5","6","6"]);
        // A viewport subset must retain floor-wide destinations, not renumber
        // visible cells or collapse the entire Speed-up area to one number.
        let mut subset:Vec<_>=view.cells.iter().take(view.terrain_count)
            .filter(|c|c.y==12&&(13..=16).contains(&c.x)).cloned().collect();
        for cell in &mut subset {cell.glyph.clear();}
        terrain(&r,observation,&mut subset);
        assert_eq!(subset.iter().map(|c|c.glyph.as_str()).collect::<Vec<_>>(),vec!["5","5","6","6"]);
        let at=12*view.width+11;let mut hidden=observation.as_ref().clone();
        hidden.cells[at+1].current=false;hidden.cells[at+1].remembered=false;hidden.cells[at+1].tile=None;
        assert!(observed_exits(&r,&hidden,at).is_empty());
        assert_eq!(r,before);
    }

    #[test]
    fn revision52_speed_up_click_preview_and_actual_arrival_agree_in_both_phases() {
        for phase in ["normal","haste"] {
            let mut r=scene();r.phase=phase.to_string().into();r.player.x=10.;r.player.y=12.;
            for (target,arrival) in [(11,12),(13,14),(13,13),(12,11)] {
                let before=r.clone();let path=crate::travel_ui::path_to(&r,target,12,true);
                assert_eq!(path.len(),1);assert!(!path[0].uncertain);
                assert_eq!((path[0].arrival_x,path[0].arrival_y),(arrival,12));
                assert_eq!(r,before,"click preview cannot draw RNG or mutate the run");
                let mut travel=crate::travel_ui::Travel::new(&r,target,12,path);
                let Some(corpse_core::Command::Move{dx,dy})=travel.next_command(&r).unwrap()
                    else {panic!("expected one movement command")};
                let id=r.player.id;
                assert!(corpse_core::engine::move_actor(&mut r,id,dx,dy,true).unwrap());
                assert_eq!((r.player.x,r.player.y),(arrival as f64,12.));
                assert!(!travel.after_step(&r,false).unwrap());assert!(travel.path().is_empty());
            }
        }
    }

    #[test]
    fn revision52_speed_up_destination_digits_survive_native_resize_and_animation() {
        let r=scene();let before=r.clone();let view=crate::observe::observe(&r);
        let mut changed=crate::observe::observe(&r);let marker=changed.cells.iter_mut().take(changed.terrain_count)
            .find(|c|c.x==11&&c.y==12).unwrap();
        assert_eq!(marker.glyph,"4");marker.glyph="9".into();
        let marker=marker.clone();let mut painter=crate::render::Painter::new();
        for (width,height) in [(960,600),(1600,900)] {
            painter.resize(width,height);
            let bounds=painter.cell_damage_rect(&marker,view.width,view.height).unwrap();
            for now in [0.,824.] {
                painter.game_at(&view,"",false,now);let original=painter.pixels.clone();
                painter.game_at(&changed,"",false,now);
                let mut different=0;
                for y in bounds.y..bounds.bottom() {for x in bounds.x..bounds.right() {
                    let offset=4*(y*width+x);
                    if original[offset..offset+4]!=painter.pixels[offset..offset+4] {different+=1;}
                }}
                assert!(different>0,"numeric destination ink lost at {width}x{height}, t={now}");
            }
        }
        assert_eq!(r,before);
    }
}

#[cfg(test)]
mod movement_marker_regressions {
    #[test]
    fn linked_space_has_one_player_image_before_and_after_crossing() {
        let mut run=super::tests::scene();run.simulation_revision=52.0.into();
        for x in [11.,13.,12.,13.,10.] {
            run.player.x=x;
            let before=run.clone();let view=crate::observe::observe(&run);
            let players:Vec<_>=view.cells.iter().filter(|c|c.entity_ink.is_some_and(|e|e.player)).collect();
            assert_eq!(players.len(),1);assert_eq!(players[0].x,x as usize);
            assert_eq!(run,before);
        }
    }
    #[test]
    fn restored_number_keeps_ground_alignment() {
        let mut marker=crate::render::Cell{glyph:"12".into(),bottom_aligned:true,non_euclidean:true,..Default::default()};
        let original=marker.clone();marker.glyph="~".into();marker.bottom_aligned=false;
        super::restore_marker(&mut marker,&original);
        assert_eq!(marker.glyph,"12");assert!(marker.bottom_aligned);
    }
}

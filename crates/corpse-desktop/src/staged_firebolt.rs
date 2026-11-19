//! Detached Firebolt terrain and recovered-projectile presentation. The command has
//! already resolved; neither this cache nor the painter mutates or queries Run.
use crate::{effects::{self, Cue}, render::{Cell, View}};
use corpse_core::{Command, Run, events::{Stage, Delivery}, vision::{ObservedCell, TerrainObservation}};
use std::{collections::{BTreeMap,BTreeSet}, sync::Arc};

/// O(I) carried-item classification. Ordinary movement/melee never snapshots a
/// world solely for this effect; legacy commands retain their historical view.
pub fn is_firebolt(run: &Run, command: &Command) -> bool {
    let Command::UseAt { id, .. } = command else { return false; };
    run.simulation_revision.value_or(29.) >= 52. && run.inventory.iter().any(|item|
        item.id == *id && corpse_core::revision50_amulet_effects::effect(item)
            == Some(corpse_core::revision50_amulet_effects::Effect::Firebolt))
}

/// Physical items may be drawn on the ground by the completed command before
/// their detached flight arrives. Consumable potion effects are never delayed.
pub fn needs_snapshot(run:&Run,command:&Command)->bool {
    is_firebolt(run,command) || (run.simulation_revision.value_or(29.)>=52. && match command {
        Command::Shoot{id,..}=>run.inventory.iter().any(|item|item.id==*id),
        Command::Throw{id,..}=>run.inventory.iter().any(|item|item.id==*id&&item.kind!="potion"),
        _=>false,
    })
}

#[derive(Clone)]
struct Previous { observed: ObservedCell, terrain: Cell, fire: Option<Cell> }
struct Route { action: u32, cells: Vec<(usize, usize)> }
struct Landing { action:u32, position:usize, index:usize, item:u64 }
#[derive(Default)]
pub struct Staging {
    width: usize,
    height: usize,
    depth: f64,
    dreamwalking: bool,
    seed: u32,
    previous: BTreeMap<usize, Previous>,
    routes: Vec<Route>,
    landings: Vec<Landing>,
    ground_before: BTreeSet<u64>,
}
/// A painter-only projection. Canonical View cells, visibility, inspection and
/// terrain_snapshot remain untouched, including current actor/health/death state.
pub struct PaintStage {
    source: Arc<TerrainObservation>,
    phase: usize,
    withheld: Vec<usize>,
    landed: Vec<(usize,u64)>,
    layers: Vec<usize>,
    pub cells: Arc<Vec<Cell>>,
    pub snapshot: Arc<TerrainObservation>,
}
impl PaintStage {
    /// O(P) cached glyph-footprint queries for P replaced layers. Old terrain
    /// can overhang a smaller final fire glyph, so cover both every live frame.
    pub fn damage(&self,painter:&crate::render::Painter,view:&View)->Vec<crate::render::damage::Rect> {
        self.layers.iter().flat_map(|&index|[self.cells.get(index),view.cells.get(index)])
            .flatten().filter_map(|cell|painter.cell_damage_rect(cell,view.width,view.height)).collect()
    }
}
impl Staging {
    pub fn clear(&mut self) { *self = Self::default(); }

    /// O(N + V + E log(G+1) + R log(R+1) + G log(G+1)) setup, retaining
    /// O(R+G) observed route/ground data. Full pre-command observation is
    /// caller-owned temporary work only for Firebolt or recoverable projectiles.
    pub fn capture(&mut self, before: View, cues: &[Cue]) {
        self.clear();
        let Some(observation) = before.terrain_snapshot.as_ref() else { return; };
        self.width=before.width; self.height=before.height;
        self.depth=before.depth; self.dreamwalking=before.dreamwalking; self.seed=before.cosmetic_seed;
        let mut terrain=vec![None;self.width*self.height];
        let mut fires=vec![None;self.width*self.height];
        for cell in before.cells.iter().take(before.terrain_count) {
            if let Some(slot)=terrain.get_mut(cell.y*self.width+cell.x) {*slot=Some(cell);}
        }
        for cell in before.cells.iter().skip(before.terrain_count).filter(|cell|cell.fire) {
            if let Some(slot)=fires.get_mut(cell.y*self.width+cell.x) {*slot=Some(cell);}
        }
        self.ground_before=before.ground_entities.iter().filter(|row|row.kind==crate::render::enemy_list::GroundKind::Item).map(|row|row.id).collect();
        let source=before.cells.iter().rev().find(|cell|cell.actor_id==Some(before.player_id)).map(|cell|(cell.x,cell.y));
        for cue in cues {
            if let Some(animation)=cue.event.animation.as_ref().filter(|a|a.stage==Stage::Travel&&a.delivery==Delivery::Projectile) {
                // Exact item provenance is carried by the existing optional
                // receipt item_id. Never infer identity from a similar glyph.
                if let (Some(item),Some(first),Some(last))=(animation.item_id,animation.cells.first(),animation.cells.last()) {
                    if source==Some((first.x as usize,first.y as usize)) && last.x>=0. && last.y>=0. && last.x<self.width as f64 && last.y<self.height as f64 {
                        let index=last.y as usize*self.width+last.x as usize;
                        if observation.cells.get(index).is_some_and(|o|o.current&&o.direct)&&!self.ground_before.contains(&item.to_bits()) {
                            self.landings.push(Landing {action:animation.action,position:animation.cells.len()-1,index,item:item.to_bits()});
                        }
                    }
                }
            }
            let Some(animation)=cue.event.animation.as_ref().filter(|a|
                a.proposal=="A02" && a.stage==Stage::Travel) else {continue;};
            let mut route=Route {action:animation.action,cells:Vec::new()};
            for (position, point) in animation.cells.iter().enumerate() {
                if !point.x.is_finite() || !point.y.is_finite() || point.x < 0. || point.y < 0.
                    || point.x >= self.width as f64 || point.y >= self.height as f64 {continue;}
                let index=point.y as usize*self.width+point.x as usize;
                let Some(observed)=observation.cells.get(index).filter(|o|o.current&&o.direct&&o.tile.is_some()) else {continue;};
                let Some(cell)=terrain[index] else {continue;};
                self.previous.entry(index).or_insert_with(||Previous {observed:observed.clone(),terrain:cell.clone(),fire:fires[index].cloned()});
                route.cells.push((position,index));
            }
            if !route.cells.is_empty() {self.routes.push(route);}
        }
    }

    /// O(K*E + R log(R+1) + H*(E+G) + H log(H+1)) for K Firebolt routes
    /// and H physical endpoints. Changed masks additionally copy O(V+N+L)
    /// records/bytes and do O((V+R+S) log(R+1)+H*V) layer/source lookups.
    /// No full-view clone at the display-frame frequency.
    /// Caller conservatively invalidates/reindexes tile damage when true.
    pub fn prepare(&self, view:&View, cues:&[Cue], now:f64, reduced:bool,
        target:&mut Option<PaintStage>) -> bool {
        let snapshot=view.terrain_snapshot.as_ref();
        if reduced || (self.routes.is_empty()&&self.landings.is_empty()) || self.width!=view.width || self.height!=view.height
            || self.depth!=view.depth || self.dreamwalking!=view.dreamwalking
            || self.seed!=view.cosmetic_seed || snapshot.is_none() {
            return target.take().is_some();
        }
        let snapshot=snapshot.unwrap();
        let mut departures=BTreeMap::<usize,bool>::new();
        for route in &self.routes {
            let Some(cue)=cues.iter().find(|cue|cue.event.animation.as_ref().is_some_and(|a|
                a.proposal=="A02"&&a.stage==Stage::Travel&&a.action==route.action)) else {continue;};
            if cue.static_marks {continue;}
            for &(position,index) in &route.cells {
                // Never re-expose a previous cell that is now outside direct
                // observation; legitimate post-command visibility wins.
                if !snapshot.cells.get(index).is_some_and(|o|o.current&&o.direct&&o.tile.is_some()) {continue;}
                let passed=now-cue.started>=effects::departure_ms(&cue.event,position);
                departures.entry(index).and_modify(|seen|*seen|=passed).or_insert(passed);
            }
        }
        let withheld:Vec<_>=departures.into_iter().filter_map(|(index,passed)|(!passed).then_some(index)).collect();
        let mut landed=Vec::new();
        for landing in &self.landings {
            let Some(cue)=cues.iter().find(|cue|cue.event.animation.as_ref().is_some_and(|a|a.stage==Stage::Travel&&a.delivery==Delivery::Projectile&&a.action==landing.action&&a.item_id.map(f64::to_bits)==Some(landing.item))) else {continue;};
            if cue.static_marks || now-cue.started>=effects::arrival_ms(&cue.event,landing.position)
                || !snapshot.cells.get(landing.index).is_some_and(|o|o.current&&o.direct) {continue;}
            if view.ground_entities.iter().any(|row|row.kind==crate::render::enemy_list::GroundKind::Item
                &&row.id==landing.item&&row.y*view.width+row.x==landing.index) {
                landed.push((landing.index,landing.item));
            }
        }
        landed.sort_unstable();landed.dedup();
        if withheld.is_empty()&&landed.is_empty() {return target.take().is_some();}
        if target.as_ref().is_some_and(|old|old.withheld==withheld&&old.landed==landed&&old.phase==view.hallucination_phase
            && Arc::ptr_eq(&old.source,snapshot)) {return false;}
        let mut cells=view.cells.clone();
        let mut layers=Vec::new();
        for (layer,cell) in cells.iter_mut().enumerate() {
            let index=cell.y*view.width+cell.x;
            if withheld.binary_search(&index).is_err() {continue;}
            let old=&self.previous[&index];
            // Keep layer indices stable for native damage. A new fire overlay
            // paints the old terrain in its original z-slot until the beam head leaves.
            // Living actors, corpses, items and other overlays are never reset.
            let replacement=if layer<view.terrain_count {Some(&old.terrain)}
                else if cell.fire {Some(old.fire.as_ref().unwrap_or(&old.terrain))}else{None};
            if let Some(previous)=replacement {
                layers.push(layer);
                let (current,direct)=(cell.current,cell.direct);
                *cell=previous.clone(); cell.current=current; cell.direct=direct;
            }
        }
        for &(index,item) in &landed {
            let Some(terrain)=view.cells.iter().take(view.terrain_count).find(|cell|cell.y*view.width+cell.x==index) else {continue;};
            for (layer,cell) in cells.iter_mut().enumerate().skip(view.terrain_count) {
                if cell.ground_item_id==Some(item)&&cell.y*view.width+cell.x==index {
                    // One canonical item overlay is admitted per coordinate.
                    // Exact ID excludes every pre-existing item and actor.
                    *cell=terrain.clone();layers.push(layer);
                }
            }
        }
        let mut painted=(**snapshot).clone();
        for &index in &withheld {
            let current=&snapshot.cells[index];
            let mut previous=self.previous[&index].observed.clone();
            previous.current=current.current; previous.direct=current.direct;
            previous.remembered=current.remembered; previous.knowledge=current.knowledge.clone();
            previous.chance=current.chance; previous.direct_chance=current.direct_chance;
            previous.distance=current.distance;
            painted.cells[index]=previous;
        }
        // Only known terrain fire emitters at the delayed cells are removed.
        // Actor sources and unknown origins retain the post-command observation.
        painted.light_sources.retain(|source|source.kind!="fire"||source.actor_id.is_some()
            ||!source.origin_known||source.x<0.||source.y<0.
            ||withheld.binary_search(&(source.y as usize*view.width+source.x as usize)).is_err()
            ||self.previous.get(&(source.y as usize*view.width+source.x as usize)).is_some_and(|old|old.observed.hazards.as_ref().is_some_and(|hazards|hazards.fire.as_ref().is_some())));
        *target=Some(PaintStage {source:snapshot.clone(),phase:view.hallucination_phase,withheld,landed,layers,
            cells:Arc::new(cells),snapshot:Arc::new(painted)});
        true
    }
}

#[cfg(test)]
#[path="staged_firebolt_tests.rs"]
pub(crate) mod tests;

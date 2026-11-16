//! Frozen player-pathing/ui-travel projections. Planning reads only current
//! permitted terrain and saved memory; painting receives detached Steps.
use corpse_core::{
    engine,
    movement::{self, Point},
    travel, vision, Actor, Command, Field, Run,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub x: usize,
    pub y: usize,
    pub arrival_x: usize,
    pub arrival_y: usize,
    /// The route ends at this random choice; arrival coordinates are not a prediction.
    #[serde(default)]
    pub uncertain: bool,
}
fn c(a: &Actor, k: &str) -> f64 {
    engine::condition(a, k)
}
fn has(a: &Actor, k: &str) -> bool {
    a.abilities.iter().any(|s| s == k)
}
fn ability(a: &Actor, name: &str, active: bool) -> bool {
    let ids = if active { &a.active } else { &a.abilities };
    corpse_core::progression::catalog()
        .iter()
        .any(|d| d.name == name && ids.contains(&d.id))
}
fn layer(v: &Field<Vec<f64>>, i: usize) -> f64 {
    v.as_ref().and_then(|v| v.get(i)).copied().unwrap_or(0.)
}

/// R52 complete-action stage, independent of terrain and the optional pursuit
/// credit. A Fast continuation remembers which action owns it even after a
/// conditional source of Haste or Fast is lost. O(1), no world query or mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionStage { ReadyHaste, ReadyNormal, FastHaste, FastNormal }
impl ActionStage {
    fn from_index(index: usize) -> Self {
        match index { 0 => Self::ReadyHaste, 1 => Self::ReadyNormal,
            2 => Self::FastHaste, 3 => Self::FastNormal, _ => unreachable!() }
    }
    pub(crate) fn observe(r: &Run) -> Self {
        match r.fast_action_phase.as_ref() {
            Some(corpse_core::model::FastActionPhase::Haste) => Self::FastHaste,
            Some(corpse_core::model::FastActionPhase::Normal) => Self::FastNormal,
            None if r.phase.as_ref().is_some_and(|p| p == "normal") => Self::ReadyNormal,
            None => Self::ReadyHaste,
        }
    }
    /// Successful Move passes Fast eligibility; Wait passes false. The bool
    /// reports completion of a normal action (world/current time may advance).
    pub(crate) fn advance(self, hasted: bool, fast_move: bool) -> (Self, bool) {
        match self {
            Self::FastHaste => (Self::ReadyNormal, false),
            Self::FastNormal => (Self::ReadyHaste, true),
            Self::ReadyHaste | Self::ReadyNormal => {
                let bonus = self == Self::ReadyHaste && hasted;
                if fast_move { (if bonus {Self::FastHaste} else {Self::FastNormal}, false) }
                else { (if bonus {Self::ReadyNormal} else {Self::ReadyHaste}, !bonus) }
            }
        }
    }
}
/// Timing of an otherwise valid movement or Wait at the actual player origin.
/// Position-dependent Haste here uses the player's own current terrain only.
pub(crate) fn action_skips_world(r: &Run, moving: bool) -> bool {
    action_world_ticks(r,moving)==0
}
/// Bounded (0, 1, 2) world advances before pursuit-specific credit. The stranded
/// self-habitat branch precedes the scheduler and always advances once.
pub(crate) fn action_world_ticks(r: &Run, moving: bool) -> usize {
    if r.simulation_revision.value_or(29.) < 52. {
        return usize::from(!(moving && r.phase.as_ref().is_none_or(|v| v != "normal")
            && crate::statuses_ui::is_hasted(r, &r.player)));
    }
    if r.floor().is_none() || corpse_core::ichor::stranded(r,&r.player) { return 1; }
    let stage=ActionStage::observe(r);
    let hasted=crate::statuses_ui::is_hasted(r,&r.player);
    let fast=moving && corpse_core::ai::fast_movement_enabled(r,&r.player);
    // Only an unstarted Haste stage applies adjacency Chill before eligibility.
    // Use the same permitted-source projection as the route graph; unknown
    // monsters never remove a predicted bonus or create a warning.
    let chilled=stage==ActionStage::ReadyHaste
        && r.actors_at(r.depth).iter().any(|a| has(a,"enemy:beast:yeti")
            && !a.dead.value_or(false) && a.hp>0. && vision::visible_actor(r,a)
            && crate::creature_presentation::project(r,a).is_none())
        && Knowledge::observe(r,&vision::terrain_visibility(r),|a|vision::visible_actor(r,a))
            .is_some_and(|k|k.chill[k.start]);
    if stage.advance(hasted&&!chilled,fast&&!chilled).1 {
        1+usize::from(c(&r.player,"slowed")>0.||chilled)
    } else {0}
}

fn component_root(parents: &mut [usize], mut at: usize) -> usize {
    while parents[at]!=at {parents[at]=parents[parents[at]];at=parents[at];} at
}
/// Zero-length known transport links form equivalence classes. Within the
/// sqrt(2) Chill radius, at most one nonzero grid edge can occur. Union-find
/// plus one eight-neighbor pass replaces a per-cell portal shortest-path query.
fn cold_components(f:&corpse_core::Floor,known:&[bool],tiles:&[String])->Vec<usize> {
    let n=known.len();let mut roots:Vec<_>=(0..n).collect();let mut sizes=vec![1;n];
    let mut join=|a:usize,b:usize| {
        if known.get(a)!=Some(&true)||known.get(b)!=Some(&true){return;}
        let(mut a,mut b)=(component_root(&mut roots,a),component_root(&mut roots,b));
        if a==b{return;}if sizes[a]<sizes[b]{std::mem::swap(&mut a,&mut b);}
        roots[b]=a;sizes[a]+=sizes[b];
    };
    for p in f.portals.as_ref().into_iter().flatten(){join(p.a,p.b);}
    for (pocket,speed) in f.non_euclidean.as_ref().into_iter().flatten().map(|p|(p,false))
        .chain(f.speed_up.as_ref().into_iter().flatten().map(|p|(p,true))) {
        for group in &pocket.groups {
            if speed && f.speed_up_terrain.value_or(false)
                && group.sources.iter().copied().chain(std::iter::once(group.destination))
                    .any(|at|tiles.get(at).is_none_or(|t|t!="speed-up")){continue;}
            for &source in &group.sources {join(source,group.destination);}
        }
    }
    for at in 0..n {roots[at]=component_root(&mut roots,at);}roots
}
fn cold_near_components(roots:&[usize],width:usize,source_cells:impl Iterator<Item=usize>)->Vec<bool> {
    let n=roots.len();let mut source=vec![false;n];let mut near=vec![false;n];
    for at in source_cells {if at<n {source[roots[at]]=true;}}
    for at in 0..n {
        if source[roots[at]] {near[roots[at]]=true;}
        for (dx,dy) in DIRECTIONS {
            let(x,y)=((at%width)as i32+dx,(at/width)as i32+dy);
            if x>=0&&y>=0&&x<width as i32&&(y as usize)*width<n
                && source[roots[y as usize*width+x as usize]] {near[roots[at]]=true;}
        }
    }
    near
}
fn observed_chill_mask(r:&Run,known:&[bool],tiles:&[String],observable:impl Fn(&Actor)->bool)->Vec<bool> {
    let mut result=vec![false;known.len()];
    let sources:Vec<_>=r.actors_at(r.depth).iter().filter(|a| !a.dead.value_or(false)&&a.hp>0.
        && !corpse_core::half_here::is_clone(a)&&has(a,"enemy:beast:yeti")&&observable(a)
        // Apparent identities may hide this trait. Do not use true Chill to
        // distinguish two otherwise identical disguised/hallucinated actors.
        && crate::creature_presentation::project(r,a).is_none()).collect();
    if sources.is_empty(){return result;}
    let Some(view_floor)=r.floor() else{return result;};
    let roots=cold_components(view_floor,known,tiles);
    for body in corpse_core::half_here::origins(r) {
        if body.dead.value_or(false)||body.hp<=0.||corpse_core::revision50_nightmares::is_decoy(body)
            || corpse_core::revision50_rings::has_effect(r,body,corpse_core::revision50_rings::ICE_IMMUNITY){continue;}
        let Some(f)=corpse_core::dreamwalking::floor_for(r,body) else{continue;};
        let cells:Vec<_>=sources.iter().filter(|source|source.id!=body.id
            && corpse_core::dreamwalking::same_plane(r,body,source)
            && !corpse_core::revision50_combat::oblivious_blocks(r,body,Some(source)))
            .filter_map(|source|corpse_core::portals::cell(f,source.x,source.y)).collect();
        if cells.is_empty(){continue;}
        let same_plane=corpse_core::dreamwalking::same_plane(r,body,&r.player);
        let other_roots;
        let body_roots=if same_plane {&roots} else {
            // Other owned endpoints can share Chill across planes, but only
            // remembered links in that other plane may shorten the distance.
            let remembered=f.remembered_tiles.as_ref();
            let other_known:Vec<_>=(0..f.tiles.len()).map(|at|f.seen.get(at)==Some(&true)
                && remembered.and_then(|v|v.get(at)).is_some()).collect();
            let other_tiles:Vec<_>=(0..f.tiles.len()).map(|at|remembered.and_then(|v|v.get(at))
                .cloned().unwrap_or_else(||"wall".into())).collect();
            other_roots=cold_components(f,&other_known,&other_tiles);&other_roots
        };
        let near=cold_near_components(body_roots,f.width(),cells.into_iter());
        if body.id==r.player.id {
            for at in 0..result.len(){result[at]|=known[at]&&near[body_roots[at]];}
        } else if corpse_core::portals::cell(f,body.x,body.y).is_some_and(|at|near[body_roots[at]]) {
            result.fill(true);
        }
    }
    result
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    threats: Vec<(u64, String, bool)>,
}
impl Snapshot {
    pub fn observe(r: &Run) -> Self {
        Self {
            threats: r
                .actors_at(r.depth)
                .iter()
                .filter(|a| vision::visible_actor(r, a))
                .map(|a| (a.id.to_bits(), a.state.clone(), a.dead.value_or(false)))
                .collect(),
        }
    }
}
pub fn changed_threat(before: &Snapshot, after: &Snapshot) -> Option<&'static str> {
    let prior: HashMap<_, _> = before
        .threats
        .iter()
        .map(|(id, state, dead)| (*id, (state, *dead)))
        .collect();
    for (id, state, dead) in &after.threats {
        if *dead || matches!(state.as_str(), "ally" | "neutral") {
            continue;
        }
        let Some((old, false)) = prior.get(id) else {
            return Some("A creature has come into view.");
        };
        if old.as_str() == "sleeping" && state != "sleeping" {
            return Some("A creature has awakened.");
        }
        if old.as_str() != "stalking" && state == "stalking" {
            return Some("A creature has begun stalking.");
        }
        if old.as_str() != "hunting" && state == "hunting" {
            return Some("A creature has begun hunting.");
        }
    }
    None
}

struct Knowledge {
    width: usize,
    height: usize,
    tiles: Vec<String>,
    known: Vec<bool>,
    unseen: Vec<bool>,
    doors: Vec<bool>,
    closed: Vec<bool>,
    grates: Vec<bool>,
    ichor: Vec<f64>,
    slime: Vec<bool>,
    currents: Vec<f64>,
    wet: Vec<bool>,
    hazards: Vec<bool>,
    occupied: HashSet<usize>,
    portals: Vec<Option<usize>>,
    portal_entries: HashSet<usize>,
    stochastic: HashSet<usize>,
    ordered_pairs: HashSet<(usize, usize)>,
    start: usize,
    phase: usize,
    hasted: bool,
    revised_actions: bool,
    fast: bool,
    swimming_haste: bool,
    chill: Vec<bool>,
    airborne: bool,
    wall_phase: bool,
    forced: bool,
    rooted: bool,
    ichor_bound: bool,
    slime_bound: bool,
    aquatic: bool,
    came_turns: f64,
    quarry: Option<(f64, f64)>,
}
impl Knowledge {
    fn observe(r: &Run, mask: &[bool], observable: impl Fn(&Actor) -> bool) -> Option<Self> {
        let f = r.floor()?;
        let p = &r.player;
        let n = f.tiles.len();
        let width = f.width();
        if p.x < 0. || p.y < 0. || p.x >= width as f64 || p.y >= f.height() as f64 {
            return None;
        }
        let context = corpse_core::hazard_memory::observation_context_revision(f, r.simulation_revision.value_or(29.0));
        let revised_actions = r.simulation_revision.value_or(29.) >= 52.;
        let slowed = c(p, "slowed") > 0.;
        let airborne = travel::flying(r, p);
        let mut out = Self {
            width,
            height: f.height(),
            tiles: Vec::with_capacity(n),
            known: vec![false; n],
            unseen: vec![false; n],
            doors: vec![false; n],
            closed: vec![false; n],
            grates: vec![false; n],
            ichor: vec![0.; n],
            slime: vec![false; n],
            currents: vec![0.; n],
            wet: vec![false; n],
            hazards: vec![false; n],
            occupied: HashSet::new(),
            portals: vec![None; n],
            portal_entries: HashSet::new(),
            stochastic: HashSet::new(),
            ordered_pairs: HashSet::new(),
            start: p.y as usize * width + p.x as usize,
            phase: if revised_actions {ActionStage::observe(r) as usize
                + if c(p, "cameFree") != 0. {4} else {0}} else {
                usize::from(r.phase.as_ref().is_some_and(|s| s == "normal"))
                    + if c(p, "cameFree") != 0. {2} else {0}},
            hasted: if revised_actions {!slowed && (c(p,"hasted") > 0.
                || crate::statuses_ui::has_non_water_haste(r,p))}
                else {crate::statuses_ui::is_hasted(r, p)},
            revised_actions,
            fast: corpse_core::ai::fast_movement_enabled(r,p),
            swimming_haste: revised_actions && !slowed && !airborne
                && corpse_core::revision50_rings::has_effect(r,p,corpse_core::revision50_rings::SWIMMING),
            chill: vec![false;n],
            airborne,
            wall_phase: ability(p, "Phase Through", true),
            forced: c(p, "forcedMove") != 0.,
            rooted: has(p, "enemy:beast:rooted"),
            ichor_bound: has(p, "enemy:beast:ichor-bound"),
            slime_bound: has(p, "enemy:beast:slime-bound"),
            aquatic: has(p, "enemy:beast:aquatic"),
            came_turns: if revised_actions && slowed {0.} else {c(p, "cameTurns")},
            quarry: None,
        };
        // R52 shallow water has ordinary route cost. Deep water remains an
        // unconditional transition rejection; retain frozen legacy preferences.
        let avoids_water = r.simulation_revision.value_or(29.) < 52.
            && !out.airborne && !out.aquatic && !ability(p, "Drown", false);
        for i in 0..n {
            let current = mask.get(i) == Some(&true);
            let seen = f.seen.get(i) == Some(&true);
            out.unseen[i] = !current && !seen;
            // Never substitute hidden live terrain for absent legacy memory.
            let tile = if current {
                f.tiles.get(i)
            } else if seen {
                f.remembered_tiles.as_ref().and_then(|v| v.get(i))
            } else {
                None
            };
            out.known[i] = tile.is_some() && (current || seen);
            out.tiles
                .push(tile.cloned().unwrap_or_else(|| "wall".into()));
            if !out.known[i] {
                continue;
            }
            out.ichor[i] = layer(
                if current {
                    &f.ichor
                } else {
                    &f.remembered_ichor
                },
                i,
            );
            out.currents[i] = layer(
                if current {
                    &f.currents
                } else {
                    &f.remembered_currents
                },
                i,
            );
            let door = if current {
                f.doors
                    .as_ref()
                    .and_then(|m| m.get(&i.to_string()))
                    .map(|d| d.open)
            } else {
                f.remembered_doors
                    .as_ref()
                    .and_then(|m| m.get(&i.to_string()))
                    .map(|d| d.open)
            };
            out.doors[i] = door.is_some();
            out.closed[i] = door == Some(false);
            let observed;
            let h = if current {
                observed = corpse_core::hazard_memory::observed_hazards_with_context(
                    f, i, r.tick, &context,
                );
                Some(&observed)
            } else {
                f.remembered_hazards
                    .as_ref()
                    .and_then(|m| m.get(&i.to_string()))
            };
            if let Some(h) = h {
                out.hazards[i] = h.gas.as_ref().is_some()
                    || h.permanent_fire.value_or(false)
                    || h.fire
                        .as_ref()
                        .is_some_and(|s| s != "embers" && !s.is_empty());
                out.slime[i] = h.slime.value_or(false);
            }
            out.wet[i] = avoids_water
                && matches!(out.tiles[i].as_str(), "water" | "shallow-water")
                && out.ichor[i] == 0.;
        }
        if corpse_core::portals::enabled(r) {
            for pair in f.portals.as_ref().into_iter().flatten() {
                for (at, mate) in [(pair.a, pair.b), (pair.b, pair.a)] {
                    if out.known.get(at) == Some(&true) {
                        out.portal_entries.insert(at);
                        if out.known.get(mate) == Some(&true) { out.portals[at] = Some(mate); }
                    }
                }
            }
        }
        if crate::non_euclidean_ui::enabled(r) {
            for pocket in f.non_euclidean.as_ref().into_iter().flatten().chain(f.speed_up.as_ref().into_iter().flatten()) {
                for &at in &pocket.cells {
                    if out.known.get(at) != Some(&true) { continue; }
                    let Some(group) = corpse_core::non_euclidean_generation::assigned_group_at(f, at) else { continue; };
                    let typed_speed = f.speed_up_terrain.value_or(false) && corpse_core::non_euclidean_generation::speedup_at(f, at).is_some();
                    if typed_speed && out.tiles[at] != "speed-up" { continue; }
                    let mut exits = if group.destination == at { group.sources.clone() } else { vec![group.destination] };
                    exits.retain(|&exit| !typed_speed || !out.known[exit] || out.tiles[exit] == "speed-up");
                    if exits.is_empty() { continue; }
                    out.portal_entries.insert(at);
                    if exits.len() == 2 { out.stochastic.insert(at); }
                    else if let Some(&exit) = exits.first().filter(|&&exit| out.known.get(exit)==Some(&true)) {
                        out.portals[at] = Some(exit);
                    }
                }
            }
        }
        if r.simulation_revision.value_or(29.) >= 37. {
            for pocket in f.speed_up.as_ref().into_iter().flatten() {
                for g in &pocket.groups {
                    if f.speed_up_terrain.value_or(false) && g.sources.iter().copied().chain(std::iter::once(g.destination)).any(|at|out.tiles.get(at).is_none_or(|t|t!="speed-up")) { continue; }
                    for &source in &g.sources {
                        if out.known.get(source)==Some(&true) && out.known.get(g.destination)==Some(&true) {
                            out.ordered_pairs.insert((source,g.destination));
                            out.ordered_pairs.insert((g.destination,source));
                        }
                    }
                }
            }
        }
        for at in f.grates.as_ref().into_iter().flatten() {
            if *at >= 0. && (*at as usize) < n {
                out.grates[*at as usize] = true
            }
        }
        for a in r
            .actors_at(r.depth)
            .iter()
            .filter(|a| !a.dead.value_or(false) && observable(a)
                && corpse_core::dreamwalking::same_plane(r, p, a))
        {
            if a.x >= 0. && a.y >= 0. && a.x < width as f64 && a.y < out.height as f64 {
                let at = a.y as usize * width + a.x as usize;
                out.occupied.insert(at);
                if let Some(mate) = out.portals[at] { out.occupied.insert(mate); }
                if crate::non_euclidean_ui::enabled(r) {
                    if let Some(g) = corpse_core::non_euclidean_generation::group_at(f, at) {
                        for other in g.sources.iter().copied().chain(std::iter::once(g.destination)) {
                            if out.known.get(other)==Some(&true) { out.occupied.insert(other); }
                        }
                    }
                }
            }
            if a.id == c(p, "cameTarget") {
                out.quarry = Some((a.x, a.y));
            }
        }
        if revised_actions {out.chill=observed_chill_mask(r,&out.known,&out.tiles,&observable);}
        Some(out)
    }
    fn habitat(&self, i: usize) -> bool {
        if self.ichor_bound {
            return self.ichor[i] > 0.;
        }
        (!self.slime_bound || self.slime[i])
            && (!self.aquatic
                || (matches!(self.tiles[i].as_str(), "water" | "shallow-water")
                    && self.ichor[i] == 0.))
    }
    fn deep(&self, i: usize) -> bool {
        self.tiles[i] == "water" && self.ichor[i] == 0.
    }
    fn hasted_at(&self, at: usize) -> bool {
        self.hasted || (self.swimming_haste && self.known[at] && self.ichor[at] == 0.
            && matches!(self.tiles[at].as_str(), "water" | "shallow-water"))
    }
    fn phases(&self) -> usize { if self.revised_actions {8} else {4} }
    fn diagonal(&self, from: usize, to: usize) -> bool {
        movement::diagonal_step_allowed(
            &self.tiles,
            self.width,
            Point {
                x: (from % self.width) as i32,
                y: (from / self.width) as i32,
            },
            Point {
                x: (to % self.width) as i32,
                y: (to / self.width) as i32,
            },
            self.wall_phase,
        )
    }
    fn traversable(&self, from: usize, to: usize) -> bool {
        ((!matches!(self.tiles[to].as_str(), "wall" | "tree") && !self.grates[to])
            || self.wall_phase)
            && (self.forced || self.diagonal(from, to))
            && (self.forced || (!self.rooted && self.habitat(self.start) && self.habitat(to)))
            && (self.tiles[to] != "chasm" || self.airborne)
    }
    fn current(&self, i: usize) -> Option<usize> {
        let code = self.currents[i];
        if code.fract() != 0.
            || !(1. ..=8.).contains(&code)
            || !self.deep(i)
            || self.closed[i]
            || self.grates[i]
        {
            return None;
        }
        const DIR: [(i32, i32); 9] = [
            (0, 0),
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let (dx, dy) = DIR[code as usize];
        let (x, y) = ((i % self.width) as i32 + dx, (i / self.width) as i32 + dy);
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let to = y as usize * self.width + x as usize;
        (self.deep(to) && !self.closed[to] && !self.grates[to]).then_some(to)
    }
    fn transition(
        &self,
        from: usize,
        entered: usize,
        state: usize,
        target: usize,
    ) -> Option<(usize, usize)> {
        if (entered != target && matches!(self.tiles[entered].as_str(), "up" | "down" | "pit"))
            || !self.known[entered]
            || self.deep(entered)
            || !self.diagonal(from, entered)
            || (!self.traversable(from, entered)
                && !self.doors[entered]
                && !(entered == target && matches!(self.tiles[entered].as_str(), "chasm" | "pit")))
        {
            return None;
        }
        // Do not turn one possible random outcome into a promised route. A
        // stochastic entry may only be the final explicitly selected square.
        if self.stochastic.contains(&entered) {
            return (entered == target).then_some((entered, 0));
        }
        let crossed = if self.ordered_pairs.contains(&(from,entered)) { entered } else if self.portal_entries.contains(&entered) {
            let mate = self.portals[entered]?;
            if !self.known[mate] || self.deep(mate) || !self.habitat(mate)
                || (!self.wall_phase && (matches!(self.tiles[mate].as_str(), "wall" | "tree") || self.grates[mate]))
                || (self.tiles[mate] == "chasm" && !self.airborne)
                || (self.occupied.contains(&mate) && entered != target && mate != target) {
                return None;
            }
            mate
        } else { entered };
        let came_bit = if self.revised_actions {4} else {2};
        let chilled=self.revised_actions && state&3==ActionStage::ReadyHaste as usize && self.chill[from];
        if self.revised_actions {
            let (next, ticks) = ActionStage::from_index(state & 3).advance(self.hasted_at(from)&&!chilled,self.fast&&!chilled);
            if !ticks { return Some((crossed,next as usize + (state & came_bit))); }
        } else if state & 1 == 0 && self.hasted {
            return Some((crossed, 1 + (state & came_bit)));
        }
        let pursued = if self.revised_actions {crossed} else {entered};
        if state & came_bit == 0
            && !chilled
            && self.came_turns > 0.
            && self.quarry.is_some_and(|(qx, qy)| {
                ((pursued % self.width) as f64 - qx).hypot((pursued / self.width) as f64 - qy)
                    < ((from % self.width) as f64 - qx).hypot((from / self.width) as f64 - qy)
            })
        {
            return Some((crossed, 1 + came_bit));
        }
        let arrival = if self.airborne { crossed } else {
            self.current(crossed).filter(|at| !self.occupied.contains(at)).unwrap_or(crossed)
        };
        (self.known[arrival] && !self.deep(arrival)).then_some((arrival, 0))
    }
    /// One multi-goal search over the same bounded movement graph as click
    /// travel. Unknown cells contribute frontier adjacency only, never terrain.
    fn frontier_path(&self, visited: &HashSet<usize>) -> Vec<Step> {
        let frontier: Vec<bool> = (0..self.known.len()).map(|at| {
            self.known[at] && at != self.start && !visited.contains(&at)
                && !self.occupied.contains(&at)
                && DIRECTIONS.iter().any(|&(dx, dy)| {
                    let (x, y) = ((at % self.width) as i32 + dx, (at / self.width) as i32 + dy);
                    x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32
                        && self.unseen[y as usize * self.width + x as usize]
                        && self.diagonal(at, y as usize * self.width + x as usize)
                })
        }).collect();
        if !frontier.iter().any(|b| *b) { return vec![]; }
        displaced_to_goal(
            self.start, self.phase, self.phases(), self.width, self.height,
            |at| frontier[at],
            |from, entered, phase| {
                // Unlike a deliberate click, exploration never selects an attack,
                // stairs, a pit or a random crossing as its final command.
                if self.occupied.contains(&entered) { return None; }
                let (at, next) = self.transition(from, entered, phase, usize::MAX)?;
                (!self.occupied.contains(&at)).then_some((at, next))
            },
            self.wet.iter().any(|b| *b).then_some(self.wet.as_slice()),
            self.hazards.iter().any(|b| *b).then_some(self.hazards.as_slice()),
        )
    }
    fn path(&self, x: usize, y: usize, direct: bool) -> Vec<Step> {
        if x >= self.width || y >= self.height {
            return vec![];
        }
        let target = y * self.width + x;
        if direct
            && x.abs_diff(self.start % self.width)
                .max(y.abs_diff(self.start / self.width))
                <= 1
        {
            if target == self.start {
                return vec![];
            }
            if let Some((arrival, _)) = self.transition(self.start, target, self.phase, target) {
                return vec![Step {
                    x,
                    y,
                    arrival_x: arrival % self.width,
                    arrival_y: arrival / self.width,
                    uncertain: self.stochastic.contains(&target),
                }];
            }
        }
        let mut result = displaced(
            self.start,
            target,
            self.portals[target],
            self.phase,
            self.phases(),
            self.width,
            self.height,
            |from, to, state| self.transition(from, to, state, target),
            self.wet.iter().any(|b| *b).then_some(self.wet.as_slice()),
            self.hazards
                .iter()
                .any(|b| *b)
                .then_some(self.hazards.as_slice()),
        );
        for step in &mut result { step.uncertain = self.stochastic.contains(&(step.y*self.width+step.x)); }
        result
    }
}
/// Source directAdjacent is a deliberate one-step preference bypass, not an
/// unknown-terrain permission. Automatic replans must pass false.
pub fn path_to(r: &Run, x: usize, y: usize, direct_adjacent: bool) -> Vec<Step> {
    let _timing = corpse_core::timing::span("desktop.route");
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
    let mask = vision::terrain_visibility(r);
    Knowledge::observe(r, &mask, |a| vision::visible_actor(r, a))
        .map_or_else(Vec::new, |k| k.path(x, y, direct_adjacent))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    state: usize,
    priority: u64,
    cost: u64,
    order: u64,
}
impl Ord for Entry {
    fn cmp(&self, b: &Self) -> Ordering {
        (b.priority, b.cost, b.order).cmp(&(self.priority, self.cost, self.order))
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
const DIRECTIONS: [(i32, i32); 8] = [
    (0, -1),
    (1, 0),
    (0, 1),
    (-1, 0),
    (1, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
];
fn displaced(
    start: usize,
    target: usize,
    target_alias: Option<usize>,
    initial_phase: usize,
    phases: usize,
    width: usize,
    height: usize,
    transition: impl Fn(usize, usize, usize) -> Option<(usize, usize)>,
    avoid: Option<&[bool]>,
    avoid_first: Option<&[bool]>,
) -> Vec<Step> {
    if target >= width * height { return vec![]; }
    displaced_to_goal(start, initial_phase, phases, width, height,
        |at| at == target || target_alias == Some(at), transition, avoid, avoid_first)
}
fn displaced_to_goal(
    start: usize,
    initial_phase: usize,
    phases: usize,
    width: usize,
    height: usize,
    is_goal: impl Fn(usize) -> bool,
    transition: impl Fn(usize, usize, usize) -> Option<(usize, usize)>,
    avoid: Option<&[bool]>,
    avoid_first: Option<&[bool]>,
) -> Vec<Step> {
    let cells = width * height;
    let count = cells * phases;
    if start >= cells || initial_phase >= phases {
        return vec![];
    }
    let mut prev = vec![usize::MAX; count];
    let mut distances = vec![usize::MAX; count];
    let mut costs = vec![u64::MAX; count];
    let mut priorities = vec![u64::MAX; count];
    let mut entered_at = vec![0; count];
    let scale = count as u64 + 1;
    let exposure_scale = scale * scale;
    let weighted = avoid.is_some() || avoid_first.is_some();
    let initial = initial_phase * cells + start;
    let first = Entry {
        state: initial,
        priority: 0,
        cost: 0,
        order: 0,
    };
    let mut heap = BinaryHeap::from([first]);
    let mut queue = vec![first];
    let mut head = 0;
    let mut order = 0;
    let mut goal = None;
    prev[initial] = initial;
    distances[initial] = 0;
    costs[initial] = 0;
    priorities[initial] = 0;
    while if weighted {
        !heap.is_empty()
    } else {
        head < queue.len()
    } {
        let e = if weighted {
            heap.pop().unwrap()
        } else {
            let mut e = queue[head];
            head += 1;
            e.cost = costs[e.state];
            e
        };
        let from = e.state % cells;
        let phase = e.state / cells;
        if e.priority != priorities[e.state] || e.cost != costs[e.state] {
            continue;
        }
        if let Some(g) = goal {
            if if weighted {
                (e.priority, e.cost) > (priorities[g], costs[g])
            } else {
                distances[e.state] > distances[g]
            } {
                break;
            }
        }
        if is_goal(from) {
            if goal.is_none_or(|g| (e.priority, e.cost) < (priorities[g], costs[g])) {
                goal = Some(e.state)
            }
            continue;
        }
        for (dx, dy) in DIRECTIONS {
            let (x, y) = ((from % width) as i32 + dx, (from / width) as i32 + dy);
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let entered = y as usize * width + x as usize;
            let Some((at, next_phase)) = transition(from, entered, phase) else {
                continue;
            };
            if at >= cells || next_phase >= phases {
                continue;
            }
            let key = next_phase * cells + at;
            let priority = e.priority + u64::from(avoid_first.is_some_and(|v| v[entered] || v[at]));
            let cost = e.cost
                + u64::from(avoid.is_some_and(|v| v[entered] || v[at])) * exposure_scale
                + scale
                + u64::from(dx != 0 && dy != 0);
            if (priority, cost) >= (priorities[key], costs[key]) {
                continue;
            }
            if weighted || prev[key] == usize::MAX {
                order += 1;
                let next = Entry {
                    state: key,
                    priority,
                    cost,
                    order,
                };
                if weighted {
                    heap.push(next)
                } else {
                    queue.push(next)
                }
            }
            prev[key] = e.state;
            distances[key] = distances[e.state] + 1;
            costs[key] = cost;
            priorities[key] = priority;
            entered_at[key] = entered;
        }
    }
    let Some(mut key) = goal else { return vec![] };
    let mut path = vec![];
    while key != initial {
        let entered = entered_at[key];
        let at = key % cells;
        path.push(Step {
            x: entered % width,
            y: entered / width,
            arrival_x: at % width,
            arrival_y: at / width,
            uncertain: false,
        });
        key = prev[key];
    }
    path.reverse();
    path
}

pub enum Start {
    Direct(Command),
    Walk(Travel),
    NoPath,
}
fn blocking_actor(r: &Run, x: usize, y: usize) -> bool {
    let alias = crate::portal_ui::observed_alias(r, x, y)
        .or_else(|| crate::non_euclidean_ui::observed_alias(r, x, y));
    r.actors_at(r.depth).iter().any(|a| {
        !a.dead.value_or(false)
            && ((a.x == x as f64 && a.y == y as f64) || alias.is_some_and(|other| a.id == other.id))
            && a.state != "ally"
            && corpse_core::dreamwalking::same_plane(r, &r.player, a)
            && vision::visible_actor(r, a)
    })
}
pub fn start(r: &Run, x: usize, y: usize) -> Start {
    let path = path_to(r, x, y, true);
    let p = &r.player;
    let adjacent = (x as f64 - p.x).abs().max((y as f64 - p.y).abs()) <= 1.;
    if adjacent
        && (blocking_actor(r, x, y)
            || path.is_empty()
            || path.first().is_some_and(|s| s.x == x && s.y == y))
    {
        return Start::Direct(Command::Move {
            dx: x as f64 - p.x,
            dy: y as f64 - p.y,
        });
    }
    if path.is_empty() {
        Start::NoPath
    } else {
        Start::Walk(Travel::new(r, x, y, path))
    }
}
pub struct Travel {
    path: Vec<Step>,
    target: (usize, usize),
    depth: f64,
    knowledge: Snapshot,
    pending: Option<(Step, f64, bool)>,
    exploring: bool,
}
impl Travel {
    pub fn new(r: &Run, x: usize, y: usize, path: Vec<Step>) -> Self {
        Self {
            path,
            target: (x, y),
            depth: r.depth,
            knowledge: Snapshot::observe(r),
            pending: None,
            exploring: false,
        }
    }
    pub fn path(&self) -> &[Step] {
        &self.path
    }
    pub fn exploring(&self) -> bool { self.exploring }
    pub fn next_command(&mut self, r: &Run) -> Result<Option<Command>, String> {
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
        if self.pending.is_some() {
            return Ok(None);
        }
        if r.status != "playing" || !r.offers.is_empty() {
            self.path.clear();
            return Ok(None);
        }
        let Some(step) = self.path.first().copied() else {
            return Ok(None);
        };
        let p = &r.player;
        let (dx, dy) = (step.x as f64 - p.x, step.y as f64 - p.y);
        if dx.abs().max(dy.abs()) > 1. {
            self.path.clear();
            return Ok(None);
        }
        let attack = blocking_actor(r, step.x, step.y);
        if self.exploring && (attack || changed_threat(&self.knowledge, &Snapshot::observe(r)).is_some()) {
            self.path.clear();
            return Ok(None);
        }
        if !attack {
            let Some(f) = r.floor() else {
                self.path.clear();
                return Ok(None);
            };
            let at = step.y * f.width() + step.x;
            if step.x >= f.width()
                || step.y >= f.height()
                || !movement::diagonal_step_allowed(
                    &f.tiles,
                    f.width(),
                    Point {
                        x: p.x as i32,
                        y: p.y as i32,
                    },
                    Point {
                        x: step.x as i32,
                        y: step.y as i32,
                    },
                    ability(p, "Phase Through", true),
                )
                || (!travel::can_traverse(r, p, step.x as f64, step.y as f64)
                    && !f
                        .doors
                        .as_ref()
                        .is_some_and(|d| d.contains_key(&at.to_string()))
                    && !matches!(f.tiles[at].as_str(), "chasm" | "pit"))
            {
                self.path.clear();
                return Err("Travel paused: the path is blocked.".into());
            }
        }
        self.path.remove(0);
        self.pending = Some((step, p.hp, attack));
        Ok(Some(Command::Move { dx, dy }))
    }
    /// Call once after the prepared command was dispatched (or raised a hazard
    /// confirmation). No simulation snapshot clone is needed.
    pub fn after_step(&mut self, r: &Run, hazard_open: bool) -> Result<bool, String> {
        let Some((step, hp, attack)) = self.pending.take() else {
            return Ok(false);
        };
        if attack {
            self.path.clear();
            return Err("Travel paused after attacking the blocking creature.".into());
        }
        if step.uncertain && !hazard_open {
            self.path.clear();
            let arrived = r.floor().is_some_and(|f|
                corpse_core::non_euclidean_generation::exits(f,step.y*f.width()+step.x)
                    .contains(&(r.player.y as usize*f.width()+r.player.x as usize)));
            if arrived {
                return Err("Travel paused after a random Non-Euclidean crossing. Choose the next route from your actual arrival.".into());
            }
            return Ok(false);
        }
        if r.depth != self.depth
            || r.player.x != step.arrival_x as f64
            || r.player.y != step.arrival_y as f64
            || hazard_open
        {
            self.path.clear();
            return Ok(false);
        }
        let next = Snapshot::observe(r);
        let threat = changed_threat(&self.knowledge, &next);
        self.knowledge = next;
        if threat.is_some() || r.player.hp < hp || c(&r.player, "paralyzed") > 0. {
            self.path.clear();
            return Err(format!(
                "Travel paused: {}",
                threat.unwrap_or("you were harmed or incapacitated.")
            ));
        }
        let reached = (r.player.x == self.target.0 as f64 && r.player.y == self.target.1 as f64)
            || (corpse_core::portals::enabled(r) && r.floor().is_some_and(|f| {
                corpse_core::portals::mate(f, self.target.1 * f.width() + self.target.0)
                    == Some(r.player.y as usize * f.width() + r.player.x as usize)
            }))
            || (crate::non_euclidean_ui::enabled(r) && r.floor().is_some_and(|f| {
                corpse_core::non_euclidean_generation::exits(f,self.target.1*f.width()+self.target.0)
                    .contains(&(r.player.y as usize*f.width()+r.player.x as usize))
            }));
        if !reached {
            self.path = path_to(r, self.target.0, self.target.1, false);
            if self.path.is_empty() {
                return Err("Travel paused: no known route reaches that destination.".into());
            }
        }
        Ok(!self.path.is_empty())
    }
}

/// Two deliberate presses: first selects a route, second transfers it to the
/// existing travel controller. Nothing here is serialized or advances time.
#[derive(Default)]
pub struct AutoExplore {
    preview: Option<Travel>,
    origin: Option<(f64, f64, f64, f64)>,
    unseen: Vec<bool>,
    visited: HashSet<usize>,
    active: bool,
}
impl AutoExplore {
    pub fn cancel(&mut self) -> bool {
        let active = self.active;
        *self = Self::default();
        active
    }
    pub fn path(&self) -> Option<&[Step]> {
        self.active.then(|| self.preview.as_ref().map_or(&[][..], Travel::path))
    }
    pub fn press(&mut self, r: &Run) -> Option<Travel> {
        let dream_view = crate::dream_ui::observation_run(r);
        let observed = dream_view.as_ref();
        if observed.status != "playing" || !observed.offers.is_empty() {
            self.cancel();
            return None;
        }
        let origin = (observed.depth, observed.tick, observed.player.x, observed.player.y);
        if self.origin == Some(origin) {
            if let Some(route) = self.preview.take() {
                if changed_threat(&route.knowledge, &Snapshot::observe(observed)).is_some() {
                    self.cancel();
                    return None;
                }
                return Some(route);
            }
        }
        self.preview = None;
        self.active = true;
        let mask = vision::terrain_visibility(observed);
        let Some(knowledge) = Knowledge::observe(observed, &mask, |a| vision::visible_actor(observed, a)) else {
            return None;
        };
        self.plan(r, &knowledge, origin);
        None
    }
    fn plan(&mut self, r: &Run, knowledge: &Knowledge, origin: (f64, f64, f64, f64)) {
        if self.origin.is_some_and(|old| old.0 != origin.0) || self.unseen != knowledge.unseen {
            self.visited.clear();
            self.unseen.clone_from(&knowledge.unseen);
        }
        self.origin = Some(origin);
        // With no newly revealed square, do not oscillate between already visited
        // frontiers in darkness. Other deliberate input resets this UI-only set.
        self.visited.insert(knowledge.start);
        let path = knowledge.frontier_path(&self.visited);
        self.preview = path.last().copied().map(|last| {
            let mut route = Travel::new(r, last.arrival_x, last.arrival_y, path);
            route.exploring = true;
            route
        });
    }
}

#[cfg(test)]
#[path = "revision52_shallow_water_pathing_tests.rs"]
mod revision52_shallow_water_pathing_tests;
#[cfg(test)]
#[path = "revision52_action_pathing_tests.rs"]
mod revision52_action_pathing_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../../fixtures/travel-ui-v48.json")).unwrap()
    }
    #[test]
    fn source_player_routes_preserve_knowledge_hazards_and_phase() {
        for case in fixture()["cases"].as_array().unwrap() {
            let r: Run = serde_json::from_value(case["run"].clone())
                .unwrap_or_else(|e| panic!("{}: {e}", case["name"]));
            let before = r.clone();
            let mask: Vec<bool> = serde_json::from_value(case["mask"].clone()).unwrap();
            let observed: Vec<f64> = serde_json::from_value(case["observable"].clone()).unwrap();
            let k = Knowledge::observe(&r, &mask, |a| observed.contains(&a.id)).unwrap();
            let got = k.path(
                case["target"]["x"].as_u64().unwrap() as usize,
                case["target"]["y"].as_u64().unwrap() as usize,
                case["direct"].as_bool().unwrap(),
            );
            let expected: Vec<Step> = serde_json::from_value(case["expected"].clone()).unwrap();
            assert_eq!(got, expected, "{}", case["name"]);
            assert_eq!(r, before);
        }
    }
    #[test]
    fn source_displaced_phase_graphs_preserve_stable_ties_and_cycle_termination() {
        for case in fixture()["graphs"].as_array().unwrap() {
            let ints = |key: &str| case[key].as_u64().unwrap() as usize;
            let flags = |key: &str| serde_json::from_value::<Vec<bool>>(case[key].clone()).unwrap();
            let blocked = flags("blocked");
            let wet = flags("wet");
            let hazard = flags("hazard");
            let push: Vec<usize> = serde_json::from_value(case["push"].clone()).unwrap();
            let mode = ints("mode");
            let got = displaced(
                ints("start"),
                ints("target"),
                None,
                ints("initialPhase"),
                ints("phases"),
                ints("width"),
                ints("height"),
                |_, to, phase| {
                    if blocked[to] {
                        None
                    } else if phase == 0 {
                        Some((to, 1))
                    } else {
                        Some((push[to], 0))
                    }
                },
                (mode & 1 != 0).then_some(wet.as_slice()),
                (mode & 2 != 0).then_some(hazard.as_slice()),
            );
            let expected: Vec<Step> = serde_json::from_value(case["expected"].clone()).unwrap();
            assert_eq!(got, expected, "{}", case["name"]);
        }
    }
    #[test]
    fn source_observed_threat_changes_keep_reason_priority() {
        for case in fixture()["threats"].as_array().unwrap() {
            let snapshot = |key: &str| Snapshot {
                threats: case[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| {
                        (
                            a[0].as_f64().unwrap().to_bits(),
                            a[1]["state"].as_str().unwrap().into(),
                            a[1]["dead"].as_bool().unwrap(),
                        )
                    })
                    .collect(),
            };
            assert_eq!(
                changed_threat(&snapshot("before"), &snapshot("after")),
                case["expected"].as_str()
            );
        }
        let before = Snapshot {
            threats: vec![
                (2, "sleeping".into(), false),
                (1, "wandering".into(), false),
            ],
        };
        let after = Snapshot {
            threats: vec![(2, "wandering".into(), false), (1, "hunting".into(), false)],
        };
        assert_eq!(
            changed_threat(&before, &after),
            Some("A creature has awakened.")
        );
    }
    fn run() -> Run {
        serde_json::from_value(fixture()["cases"][0]["run"].clone()).unwrap()
    }
    fn exploration_run() -> Run {
        let mut r = run();
        r.player.conditions.clear(); r.player.active.clear(); r.player.abilities.clear();
        r.actors.clear(); r.inventory.clear(); r.offers.clear();
        let f = r.floor_mut().unwrap();
        f.tiles.fill("wall".into());
        for x in 1..=5 { f.tiles[3 * 7 + x] = "floor".into(); }
        f.seen.fill(true);
        for x in 3..=5 { f.seen[2 * 7 + x] = false; f.tiles[2 * 7 + x] = "floor".into(); }
        f.remembered_tiles = f.tiles.clone().into();
        f.lights.fill(-100.);
        f.environment = Field::Missing; f.doors = Field::Missing;
        f.portals = Field::Missing; f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
        r
    }
    #[test]
    fn autoexplore_first_press_only_previews_second_transfers_known_route() {
        let r = exploration_run(); let before = r.clone();
        let mut explore = AutoExplore::default();
        assert!(explore.press(&r).is_none());
        assert_eq!(explore.path().unwrap().iter().map(|s| (s.x, s.y)).collect::<Vec<_>>(), vec![(2, 3), (3, 3)]);
        assert_eq!(r, before);
        let walking = explore.press(&r).unwrap();
        assert!(walking.exploring());
        assert_eq!(walking.target, (3, 3));
        assert_eq!(walking.path().len(), 2);
        assert!(explore.path().unwrap().is_empty());
        assert_eq!(r, before);
    }
    #[test]
    fn autoexplore_hidden_walls_occupants_and_hazards_do_not_change_frontier() {
        let mut r = exploration_run();
        let before = Knowledge::observe(&r, &[false; 49], |_| false).unwrap().frontier_path(&HashSet::new());
        assert!(!before.is_empty());
        let f = r.floor_mut().unwrap();
        for x in 3..=5 { f.tiles[2 * 7 + x] = "wall".into(); }
        f.ichor = vec![1.; 49].into(); f.currents = vec![3.; 49].into();
        let mut actor = r.player.clone(); actor.id = 999.; actor.x = 2.; actor.state = "hunting".into();
        r.actors_at_mut(r.depth).push(actor);
        let k = Knowledge::observe(&r, &[false; 49], |_| false).unwrap();
        assert_eq!(k.frontier_path(&HashSet::new()), before);
        assert!(before.iter().all(|s| k.known[s.y * 7 + s.x] && k.known[s.arrival_y * 7 + s.arrival_x]));
        // A genuinely observed blocker is excluded, not attacked by exploration.
        let k = Knowledge::observe(&r, &[false; 49], |_| true).unwrap();
        assert!(k.frontier_path(&HashSet::new()).is_empty());
    }
    #[test]
    fn autoexplore_complete_inaccessible_and_legacy_unknown_maps_are_quiet() {
        for variant in ["complete", "inaccessible", "legacy"] {
            let mut r = exploration_run();
            let f = r.floor_mut().unwrap();
            match variant {
                "complete" => f.seen.fill(true),
                "inaccessible" => { f.tiles[23] = "wall".into(); f.remembered_tiles = f.tiles.clone().into(); }
                "legacy" => f.remembered_tiles = Field::Missing,
                _ => unreachable!(),
            }
            let before = r.clone(); let mut explore = AutoExplore::default();
            assert!(explore.press(&r).is_none(), "{variant}");
            assert!(explore.path().unwrap().is_empty(), "{variant}");
            assert!(explore.press(&r).is_none(), "{variant}");
            assert_eq!(r, before);
        }
    }
    #[test]
    fn autoexplore_no_visibility_progress_never_oscillates_or_steps_into_darkness() {
        let mut r = exploration_run(); let seen = r.floor().unwrap().seen.clone();
        let mut explore = AutoExplore::default(); let mut destinations = HashSet::new();
        for _ in 0..8 {
            assert!(explore.press(&r).is_none());
            if explore.path().unwrap().is_empty() { break; }
            let mut walk = explore.press(&r).unwrap();
            assert!(destinations.insert(walk.target), "frontier was revisited without revealing anything");
            for _ in 0..8 {
                let Some(Command::Move { dx, dy }) = walk.next_command(&r).unwrap() else { break; };
                r.player.x += dx; r.player.y += dy; r.tick += 1.;
                assert!(seen[r.player.y as usize * 7 + r.player.x as usize]);
                if !walk.after_step(&r, false).unwrap() { break; }
            }
            assert_eq!((r.player.x, r.player.y), (walk.target.0 as f64, walk.target.1 as f64));
        }
        assert_eq!(destinations.len(), 3);
        assert!(explore.path().unwrap().is_empty());
        assert!(explore.press(&r).is_none());
        assert!(explore.path().unwrap().is_empty());
        assert_eq!(r.floor().unwrap().seen, seen);
    }
    #[test]
    fn autoexplore_cancel_and_new_discovery_reset_frontier_preview() {
        let mut r = exploration_run(); let mut explore = AutoExplore::default();
        explore.press(&r); assert!(explore.cancel()); assert!(explore.path().is_none());
        assert!(explore.press(&r).is_none()); assert!(!explore.path().unwrap().is_empty());
        assert_eq!(explore.visited.len(), 1);
        explore.visited.extend([24, 25, 26]);
        // A changed origin invalidates pending confirmation; new knowledge resets
        // the exhausted frontier set instead of retaining a permanent blacklist.
        r.tick += 1.; r.floor_mut().unwrap().seen[17] = true;
        assert!(explore.press(&r).is_none()); assert_eq!(explore.visited.len(), 1);
        assert!(explore.preview.is_some());
    }
    #[test]
    fn autoexplore_new_observed_threat_stops_before_the_next_command() {
        let mut r = exploration_run(); let mut explore = AutoExplore::default();
        explore.press(&r); let mut walk = explore.press(&r).unwrap();
        let mut actor = r.player.clone(); actor.id = 999.; actor.x = 5.; actor.state = "hunting".into();
        r.actors_at_mut(r.depth).push(actor);
        r.floor_mut().unwrap().lights.fill(100.);
        let before = r.clone();
        assert!(walk.next_command(&r).unwrap().is_none());
        assert!(walk.path().is_empty()); assert_eq!(r, before);
    }
    #[test]
    fn autoexplore_stale_preview_is_not_confirmed_after_a_threat_or_floor_change() {
        for changed in ["threat", "floor"] {
            let mut r = exploration_run(); let mut explore = AutoExplore::default();
            explore.press(&r); assert!(!explore.path().unwrap().is_empty());
            if changed == "threat" {
                let mut actor = r.player.clone(); actor.id = 999.; actor.x = 5.; actor.state = "hunting".into();
                r.actors_at_mut(r.depth).push(actor); r.floor_mut().unwrap().lights.fill(100.);
            } else { r.depth = 99.; }
            let before = r.clone();
            assert!(explore.press(&r).is_none());
            assert!(explore.path().is_none_or(|path| path.is_empty()));
            assert_eq!(r, before);
        }
    }
    #[test]
    fn autoexplore_observed_hazard_still_requires_confirmation_and_stops() {
        let mut r = exploration_run(); let mut explore = AutoExplore::default();
        explore.press(&r); let mut walk = explore.press(&r).unwrap();
        let f = r.floor_mut().unwrap(); f.lights.fill(100.);
        let mut environment = corpse_core::EnvironmentalState::default();
        environment.fire = vec![0.; 49]; environment.fire[23] = 8.;
        f.environment = environment.into();
        let command = walk.next_command(&r).unwrap().unwrap();
        assert!(crate::hazards_ui::intentional(&r, &command).is_some());
        let before = r.clone();
        assert!(!walk.after_step(&r, true).unwrap());
        assert!(walk.path().is_empty()); assert_eq!(r, before);
    }
    #[test]
    fn portal_ui_known_pair_route_arrival_and_unknown_mate() {
        let mut r = run(); r.simulation_revision=35.0.into();
        r.player.x=1.;r.player.y=3.;r.player.conditions.clear();r.player.active.clear();r.player.abilities.clear();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.seen=vec![true;49];f.remembered_tiles=f.tiles.clone().into();
        f.environment=Field::Missing;f.ichor=Field::Missing;f.currents=Field::Missing;f.doors=Field::Missing;
        f.portals=vec![corpse_core::portals_generation::PortalPair{a:23,b:26,color_index:0}].into();
        let k=Knowledge::observe(&r,&vec![true;49], |_|false).unwrap();
        let path=k.path(5,3,false);assert_eq!(path.len(),1);
        assert_eq!(path[0],Step{x:2,y:3,arrival_x:5,arrival_y:3,uncertain:false});
        let mut k=k;k.known[26]=false;k.portals[23]=None;
        assert!(k.transition(22,23,1,26).is_none());
    }
    #[test]
    fn portal_ui_arrival_alias_completes_without_bounce() {
        let mut r=run();r.simulation_revision=35.0.into();r.player.x=1.;r.player.y=3.;
        r.player.conditions.clear();r.actors.clear();r.offers.clear();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.portals=vec![corpse_core::portals_generation::PortalPair{a:23,b:26,color_index:0}].into();
        let step=Step{x:2,y:3,arrival_x:5,arrival_y:3,uncertain:false};let mut t=Travel::new(&r,2,3,vec![step]);
        t.pending=Some((step,r.player.hp,false));t.path.clear();r.player.x=5.;
        assert!(!t.after_step(&r,false).unwrap());assert!(t.path.is_empty());
    }
    #[test]
    fn hidden_live_changes_cannot_rewrite_historical_routes() {
        let mut r = run();
        let mask = vec![false; 49];
        let before = Knowledge::observe(&r, &mask, |_| false)
            .unwrap()
            .path(5, 3, false);
        let f = r.floor_mut().unwrap();
        f.tiles.fill("wall".into());
        f.ichor = vec![1.; 49].into();
        f.currents = vec![3.; 49].into();
        f.doors = Field::Missing;
        f.environment.as_mut().unwrap().fire.fill(10.);
        let mut hidden = r.player.clone();
        hidden.id = 987.;
        hidden.x = 3.;
        hidden.state = "hunting".into();
        r.actors_at_mut(r.depth).push(hidden);
        assert_eq!(
            Knowledge::observe(&r, &mask, |_| false)
                .unwrap()
                .path(5, 3, false),
            before
        );
        r.floor_mut().unwrap().remembered_tiles = Field::Missing;
        assert!(Knowledge::observe(&r, &mask, |_| false)
            .unwrap()
            .path(5, 3, false)
            .is_empty());
    }
    #[test]
    fn deep_water_never_enters_player_paths_even_with_flight_or_drown() {
        let mut r = run();
        r.player.abilities = vec!["anavani-ascend".into(), "hie-drown".into()];
        let at = 23;
        r.floor_mut().unwrap().tiles[at] = "water".into();
        let k = Knowledge::observe(&r, &vec![true; 49], |_| false).unwrap();
        assert!(k.path(2, 3, true).is_empty());
    }
    #[test]
    fn controller_stops_on_harm_displacement_hazard_and_blocker() {
        let mut r = run();
        r.player.conditions.clear();
        let step = Step {
            x: 2,
            y: 3,
            arrival_x: 2,
            arrival_y: 3,
            uncertain: false,
        };
        for variant in [
            "harm",
            "paralyzed",
            "displaced",
            "depth",
            "hazard",
            "blocked",
        ] {
            let mut r = r.clone();
            let mut travel = Travel::new(&r, 5, 3, vec![step]);
            if variant == "blocked" {
                r.floor_mut().unwrap().tiles[23] = "wall".into();
                assert_eq!(
                    travel.next_command(&r).unwrap_err(),
                    "Travel paused: the path is blocked."
                );
                continue;
            }
            assert!(matches!(
                travel.next_command(&r).unwrap(),
                Some(Command::Move { dx: 1., dy: 0. })
            ));
            r.player.x = 2.;
            match variant {
                "harm" => r.player.hp -= 1.,
                "paralyzed" => {
                    r.player.conditions.insert("paralyzed".into(), 2.);
                }
                "displaced" => r.player.x = 4.,
                "depth" => r.depth += 1.,
                _ => {}
            }
            let got = travel.after_step(&r, variant == "hazard");
            if matches!(variant, "harm" | "paralyzed") {
                assert_eq!(
                    got.unwrap_err(),
                    "Travel paused: you were harmed or incapacitated."
                )
            } else {
                assert!(!got.unwrap())
            }
            assert!(travel.path().is_empty());
        }
    }
    #[test]
    fn controller_advances_replans_and_cancels_without_simulation_writes() {
        let mut r = run();
        r.player.conditions.clear();
        let original = r.clone();
        let path = path_to(&r, 5, 3, true);
        assert!(!path.is_empty());
        let mut walking = Travel::new(&r, 5, 3, path);
        for _ in 0..8 {
            let Some(Command::Move { dx, dy }) = walking.next_command(&r).unwrap() else {
                break;
            };
            r.player.x += dx;
            r.player.y += dy;
            r.tick += 1.;
            if !walking.after_step(&r, false).unwrap() {
                break;
            }
        }
        assert_eq!((r.player.x, r.player.y), (5., 3.));
        assert!(walking.path().is_empty());
        let mut fresh = Travel::new(&original, 5, 3, path_to(&original, 5, 3, true));
        let snapshot = original.clone();
        fresh.next_command(&original).unwrap();
        assert_eq!(snapshot, original);
        drop(fresh);
    }
    #[test]
    fn non_euclidean_ui_routes_stop_at_random_choice_and_never_draw_rng() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.player.x=13.;r.player.y=9.;
        let before=r.clone();let path=path_to(&r,13,10,true);
        assert_eq!(path.len(),1);assert!(path[0].uncertain);assert_eq!(r,before);
        let mask=vision::terrain_visibility(&r);let k=Knowledge::observe(&r,&mask, |_|false).unwrap();
        let w=k.width;assert!(k.transition(9*w+13,10*w+13,1,10*w+18).is_none());
        let mut t=Travel::new(&r,13,10,path);assert!(t.next_command(&r).unwrap().is_some());
        // Both real outcomes require a fresh choice, never a guessed continuation.
        r.player.x=11.;r.player.y=10.;
        assert!(t.after_step(&r,false).unwrap_err().contains("actual arrival"));assert!(t.path().is_empty());
        assert!(t.next_command(&r).unwrap().is_none());
    }
    #[test]
    fn non_euclidean_ui_deterministic_entry_known_arrival_and_unknown_exit() {
        let r=crate::non_euclidean_ui::tests::scene();let path=path_to(&r,11,10,true);
        assert_eq!(path.len(),1);assert_eq!((path[0].arrival_x,path[0].arrival_y),(13,10));assert!(!path[0].uncertain);
        let mut t=Travel::new(&r,11,10,path);let mut arrived=r.clone();
        assert!(t.next_command(&r).unwrap().is_some());arrived.player.x=13.;arrived.player.y=10.;
        assert!(!t.after_step(&arrived,false).unwrap());assert!(t.path().is_empty());
        let mut mask=vision::terrain_visibility(&r);let w=r.floor().unwrap().width();mask[10*w+13]=false;
        let mut hidden=r.clone();hidden.floor_mut().unwrap().seen[10*w+13]=false;
        let k=Knowledge::observe(&hidden,&mask, |_|false).unwrap();assert!(k.path(11,10,true).is_empty());
    }

    #[test]
    fn r37_speed_up_known_route_and_unknown_destination() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.simulation_revision=37.0.into();
        let f=r.floor_mut().unwrap();let w=f.width();
        f.speed_up=vec![corpse_core::non_euclidean_generation::NonEuclideanPocket {
            cells:vec![10*w+11,10*w+18],groups:vec![corpse_core::non_euclidean_generation::NonEuclideanGroup{sources:vec![10*w+11],destination:10*w+18}]
        }].into();f.non_euclidean=corpse_core::Field::Missing;
        let before=r.clone();let path=path_to(&r,11,10,true);
        assert_eq!(path.len(),1);assert_eq!((path[0].arrival_x,path[0].arrival_y),(18,10));assert!(!path[0].uncertain);assert_eq!(r,before);
        let mut mask=vision::terrain_visibility(&r);mask[10*w+18]=false;
        r.floor_mut().unwrap().seen[10*w+18]=false;
        let k=Knowledge::observe(&r,&mask, |_|false).unwrap();assert!(k.path(11,10,true).is_empty());
    }

    #[test]
    fn r37_speed_up_middle_pair_does_not_bounce_in_preview() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.simulation_revision=37.0.into();r.player.x=11.;r.player.y=10.;
        let f=r.floor_mut().unwrap();let w=f.width();
        f.speed_up=vec![corpse_core::non_euclidean_generation::NonEuclideanPocket {
            cells:vec![10*w+11,10*w+12],groups:vec![corpse_core::non_euclidean_generation::NonEuclideanGroup{sources:vec![10*w+11],destination:10*w+12}]
        }].into();f.non_euclidean=corpse_core::Field::Missing;
        let before=r.clone();let path=path_to(&r,12,10,true);
        assert_eq!(path.len(),1);assert_eq!((path[0].arrival_x,path[0].arrival_y),(12,10));assert!(!path[0].uncertain);assert_eq!(r,before);
        let mut t=Travel::new(&r,12,10,path);assert!(t.next_command(&r).unwrap().is_some());
        r.player.x=12.;assert!(!t.after_step(&r,false).unwrap());
    }

}

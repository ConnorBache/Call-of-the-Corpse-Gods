//! Exact v48 source-layer fire adapter and complete explosive-gas closure.
//! Damage is returned only after all reached gas and reaction unions resolve.
use crate::environment::{self, fire_active, gas_passable};
use crate::gas_reactions::GasReactions;
use crate::propagation::{
    self, FireLayer, FireScan, GridSize, PromotionLayer, RandomInt, TimedPromotions,
};
use crate::{Floor, Run};
use std::collections::HashSet;
#[derive(Clone, Debug)]
pub struct Burst {
    pub at: usize,
    pub radius: usize,
    pub owner: f64,
    pub player: f64,
    pub reaction: u64,
    pub detonates: bool,
}
pub struct FireAdapter<'a> {
    pub run: &'a mut Run,
    floor_index: usize,
    frozen: Vec<bool>,
    exposure: Vec<u32>,
    reactions: GasReactions,
    source_owner: f64,
    source_player: f64,
    source_reaction: u64,
    source_cell: Option<usize>,
    pending: Vec<Burst>,
    exploded: HashSet<usize>,
    external_reaction: u64,
    error: Option<String>,
}
fn scalar(v: &crate::Field<Vec<f64>>, i: usize) -> f64 {
    v.as_ref().and_then(|v| v.get(i)).copied().unwrap_or(0.)
}
impl<'a> FireAdapter<'a> {
    pub fn new(run: &'a mut Run, frozen: Vec<bool>) -> Result<Self, String> {
        let floor_index = run
            .floors
            .iter()
            .position(|f| f.depth == run.depth)
            .ok_or("Missing floor")?;
        let revision = run.simulation_revision.value_or(29.);
        crate::revision51_fire_water::extinguish_floor(
            crate::mutation_journal::make_mut(&mut run.floors[floor_index]), revision);
        let e = environment::initialize(crate::mutation_journal::make_mut(&mut run.floors[floor_index]))?;
        let next = *e
            .next_fire_reaction
            .as_ref()
            .ok_or("Missing reaction counter")?;
        if next.fract() != 0. || next < 1. || next > 9007199254740991. {
            return Err("Invalid gas reaction identity.".into());
        }
        let n = e.fire.len();
        for i in 0..n {
            if !fire_active(e.fire_kind.as_ref().unwrap()[i].as_deref()) {
                e.fire_reaction.as_mut().unwrap()[i] = 0.;
            }
            let id = e.fire_reaction.as_ref().unwrap()[i];
            if id.fract() != 0. || id < 0. || id >= next {
                return Err("Invalid gas reaction identity.".into());
            }
        }
        Ok(Self {
            run,
            floor_index,
            frozen,
            exposure: vec![0; n],
            reactions: GasReactions::new(next as u64),
            source_owner: -1.,
            source_player: 0.,
            source_reaction: 0,
            source_cell: None,
            pending: vec![],
            exploded: HashSet::new(),
            external_reaction: 0,
            error: None,
        })
    }
    fn f(&self) -> &Floor {
        self.run.floors[self.floor_index].as_ref()
    }
    fn fm(&mut self) -> &mut Floor {
        crate::mutation_journal::make_mut(&mut self.run.floors[self.floor_index])
    }
    fn e(&self) -> &crate::EnvironmentalState {
        self.f().environment.as_ref().unwrap()
    }
    fn em(&mut self) -> &mut crate::EnvironmentalState {
        self.fm().environment.as_mut().unwrap()
    }
    fn active(&self, i: usize) -> bool {
        fire_active(self.e().fire_kind.as_ref().unwrap()[i].as_deref())
    }
    fn reaction(&self, i: usize) -> u64 {
        self.e().fire_reaction.as_ref().unwrap()[i] as u64
    }
    fn merge(&mut self, a: u64, b: u64) -> u64 {
        match self.reactions.merge(a, b) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e);
                0
            }
        }
    }
    fn allocate(&mut self) -> u64 {
        match self.reactions.allocate() {
            Ok(v) => {
                self.em().next_fire_reaction = (self.reactions.next as f64).into();
                v
            }
            Err(e) => {
                self.error = Some(e);
                0
            }
        }
    }
    fn fuel_layers(&self, i: usize, creature: bool) -> Vec<FireLayer> {
        let f = self.f();
        let e = self.e();
        let tile = f.tiles[i].as_str();
        let wet = (tile == "water" || tile == "shallow-water") && scalar(&f.ichor, i) == 0.;
        let door = f.doors.as_ref().and_then(|v| v.get(&i.to_string()));
        let gas = e.gas_type.as_ref().unwrap()[i].as_deref();
        let layer = |id: &str,
                     flammable,
                     ignition_chance,
                     draw_priority,
                     extinguishes,
                     is_gas,
                     explosive| FireLayer {
            id: id.into(),
            flammable,
            ignition_chance,
            draw_priority,
            extinguishes,
            is_gas,
            explosive,
        };
        vec![
            layer(
                "door",
                door.is_some(),
                50.,
                if door.is_some_and(|d| d.open) {
                    25.
                } else {
                    8.
                },
                false,
                false,
                false,
            ),
            layer(
                "water",
                wet && tile == "water",
                if tile == "water" { 100. } else { 0. },
                if tile == "shallow-water" { 55. } else { 40. },
                wet,
                false,
                false,
            ),
            layer(
                "ichor",
                scalar(&f.ichor, i) > 0. && !self.active(i),
                100.,
                40.,
                false,
                false,
                false,
            ),
            layer(
                "gas",
                gas.is_some()
                    && gas != Some("steam")
                    && !(gas == Some("bloodMist") && crate::blood_mist::nonflammable(self.run))
                    && (!creature || e.gas_volume.as_ref().unwrap()[i] > 0.),
                100.,
                35.,
                false,
                true,
                matches!(gas, Some("explosive" | "hydrogen")),
            ),
            layer(
                "terrain",
                tile == "grass" || tile == "tree",
                15.,
                if tile == "tree" { 45. } else { 60. },
                false,
                false,
                false,
            ),
            layer(
                "slime",
                scalar(&f.slime, i) > 0.,
                100.,
                45.,
                false,
                false,
                false,
            ),
            layer(
                "roots",
                scalar(&f.roots, i) > 0.,
                15.,
                45.,
                false,
                false,
                false,
            ),
            layer(
                "leaves",
                scalar(&f.razorleaves, i) > 0.,
                15.,
                45.,
                false,
                false,
                false,
            ),
        ]
    }
    fn set_fire(&mut self, i: usize, kind: &str) {
        let revision = self.run.simulation_revision.value_or(29.);
        // Keep gas-reaction/steam processing, but never retain a flame or ember
        // on water. Hellfire and combustible Ichor keep their existing rules.
        if crate::revision51_fire_water::extinguish_cell(self.fm(), i, revision) { return; }
        let reaction = self.merge(self.reaction(i), self.source_reaction);
        self.em().fire_reaction.as_mut().unwrap()[i] = reaction as f64;
        let f = self.f();
        let prior = self.e().fire_kind.as_ref().unwrap()[i].as_deref();
        if prior == Some(kind)
            || (kind == "embers"
                && (self.active(i)
                    || ["grass", "tree"].contains(&f.tiles[i].as_str())
                    || scalar(&f.slime, i) > 0.
                    || scalar(&f.roots, i) > 0.
                    || scalar(&f.razorleaves, i) > 0.))
        {
            return;
        }
        let was = self.active(i);
        let owner = self.source_owner;
        let player = self.source_player;
        let f = self.fm();
        if ["grass", "tree"].contains(&f.tiles[i].as_str()) {
            f.tiles[i] = "floor".into();
            crate::ichor::enqueue_opening_revision(f, i, revision);
        }
        if let Some(v) = f.slime.as_mut() {
            v[i] = 0.;
        }
        if let Some(v) = f.razorleaves.as_mut() {
            v[i] = 0.;
        }
        if scalar(&f.roots, i) > 0. {
            environment::burn_roots_revision(f, i, revision)
        }
        let kind = if crate::terrain_runtime::hellfire_at(f,i) || f
            .permanent_fire
            .as_ref()
            .is_some_and(|v| v.contains(&(i as f64)))
        {
            "brazier"
        } else if scalar(&f.ichor, i) > 0. {
            "ichor"
        } else {
            kind
        };
        let e = f.environment.as_mut().unwrap();
        e.fire_kind.as_mut().unwrap()[i] = Some(kind.into());
        e.fire[i] = if fire_active(Some(kind)) { 1. } else { 0. };
        if e.fire[i] == 0. {
            e.fire_reaction.as_mut().unwrap()[i] = 0.;
        }
        if !was && e.fire[i] > 0. {
            e.fire_fresh.as_mut().unwrap()[i] = true
        }
        let p = e.provenance.as_mut().unwrap().get_mut("fire").unwrap();
        p.owner[i] = owner;
        p.player[i] = p.player[i].max(player);
    }
    fn promote_layer(&mut self, i: usize, layer: FireLayer, confined: bool) {
        let revision = self.run.simulation_revision.value_or(29.);
        match layer.id.as_str() {
            "water" => {
                if let Err(e) = environment::add_gas(self.fm(), i, "steam", 15., -1.) {
                    self.error = Some(e)
                }
            }
            "gas" => {
                let gas = self.e().gas_type.as_ref().unwrap()[i].clone();
                let volume = self.e().gas_volume.as_ref().unwrap()[i];
                let explosive = matches!(gas.as_deref(), Some("explosive" | "hydrogen"));
                let detonates = volume > 0. && confined && explosive && !self.exploded.contains(&i);
                let prior = self.e().fire_kind.as_ref().unwrap()[i].clone();
                let f = self.f();
                let fuel = prior.as_deref() == Some("plain")
                    || ["grass", "tree"].contains(&f.tiles[i].as_str())
                    || scalar(&f.slime, i) > 0.
                    || scalar(&f.roots, i) > 0.
                    || scalar(&f.razorleaves, i) > 0.;
                if explosive && volume > 0. {
                    let merged = self.merge(self.source_reaction, self.reaction(i));
                    self.source_reaction = if merged != 0 { merged } else { self.allocate() };
                    let reaction = self.source_reaction;
                    self.em().fire_reaction.as_mut().unwrap()[i] = reaction as f64;
                    if let Some(source) = self.source_cell {
                        if self.active(source) {
                            let reaction = self.merge(self.reaction(source), reaction);
                            self.em().fire_reaction.as_mut().unwrap()[source] = reaction as f64;
                        }
                    }
                }
                crate::capture_pilot::mark_gas_write(self.f().depth.to_bits(), i,
                    self.e().gas_volume.as_ref().unwrap()[i].to_bits() != 0f64.to_bits());
                self.em().gas_volume.as_mut().unwrap()[i] = 0.;
                crate::capture_pilot::mark_gas_write(self.f().depth.to_bits(), i,
                    self.e().gas_player.as_ref().unwrap()[i].to_bits() != 0f64.to_bits());
                self.em().gas_player.as_mut().unwrap()[i] = 0.;
                let kind = if detonates {
                    if fuel { "plain" } else { "gas-explosion" }
                } else if explosive && matches!(prior.as_deref(), Some("plain" | "gas-explosion")) {
                    prior.as_deref().unwrap()
                } else {
                    "gas"
                };
                self.set_fire(i, kind);
                if explosive && volume > 0. {
                    if detonates {
                        self.exploded.insert(i);
                    }
                    self.pending.push(Burst {
                        at: i,
                        radius: if detonates {
                            if gas.as_deref() == Some("hydrogen") {
                                9
                            } else {
                                2
                            }
                        } else {
                            1
                        },
                        detonates,
                        owner: self.source_owner,
                        player: self.source_player,
                        reaction: self.source_reaction,
                    });
                }
            }
            "door" => {
                let width = self.f().width();
                let observed = self.run.player.x >= 0.
                    && crate::vision::visible(self.run, i % width, i / width);
                self.run
                    .floor_mut()
                    .unwrap()
                    .doors
                    .as_mut()
                    .unwrap()
                    .shift_remove(&i.to_string());
                self.set_fire(i, "embers");
                self.fm().dirty = true;
                if observed {
                    crate::engine::log(self.run, "The wooden door burns away.", "environment");
                }
            }
            _ => {
                let f = self.fm();
                match layer.id.as_str() {
                    "terrain" => {
                        f.tiles[i] = "floor".into();
                        crate::ichor::enqueue_opening_revision(f, i, revision);
                    }
                    "slime" => f.slime.as_mut().unwrap()[i] = 0.,
                    "roots" => environment::burn_roots_revision(f, i, revision),
                    "leaves" => f.razorleaves.as_mut().unwrap()[i] = 0.,
                    _ => {}
                }
                self.set_fire(
                    i,
                    if layer.id == "ichor" {
                        "ichor"
                    } else {
                        "plain"
                    },
                );
            }
        }
    }
    fn expose(&mut self, i: usize, forced: bool, creature: bool) -> bool {
        if self.frozen.get(i).copied().unwrap_or(false) {
            return false;
        }
        if crate::revision50::enabled(self.run) {crate::terrain_runtime::melt_ice(self.fm(),i);}
        if crate::terrain_runtime::enabled(self.run) && crate::terrain_runtime::melt_snow(self.fm(),i) {
            return false;
        }
        if let Some(source) = self.source_cell {
            if self.active(source) && self.active(i) {
                self.source_reaction = self.merge(self.reaction(source), self.reaction(i));
                let reaction = self.source_reaction;
                let ids = self.em().fire_reaction.as_mut().unwrap();
                ids[source] = reaction as f64;
                ids[i] = reaction as f64;
            }
        }
        // Slot contents must be queried live after each prior layer promotion.
        let layers = self.fuel_layers(i, creature);
        if !layers.iter().any(|l| l.flammable) || self.exposure[i] >= 12 {
            return false;
        }
        self.exposure[i] += 1;
        let priority = layers
            .iter()
            .filter(|l| l.extinguishes)
            .fold(1000f64, |p, l| p.min(l.draw_priority));
        let probability = layers
            .iter()
            .filter(|l| l.flammable && (l.is_gas || l.draw_priority <= priority))
            .fold(0f64, |p, l| p.max(l.ignition_chance));
        if !forced && (probability == 0. || !propagation::percent(self, probability)) {
            return false;
        }
        let confined = layers.iter().any(|l| l.explosive)
            && propagation::explosively_confined(self.size(), i, |j| {
                self.active(j)
                    || !gas_passable(self.f(), j)
                    || matches!(
                        self.e().gas_type.as_ref().unwrap()[j].as_deref(),
                        Some("explosive" | "hydrogen")
                    )
            });
        for slot in 0..layers.len() {
            let layer = self.fuel_layers(i, creature)[slot].clone();
            if layer.flammable {
                self.promote_layer(i, layer, confined)
            }
        }
        true
    }
    pub fn ignite_creature(&mut self, i: usize, owner: f64, player: f64) {
        if self.e().fire[i] > 0. {
            return;
        }
        self.source_owner = owner;
        self.source_player = player;
        self.source_cell = Some(i);
        self.source_reaction = self.reaction(i);
        self.expose(i, true, true);
    }
    pub fn ignite_blast(&mut self, i: usize, owner: f64, player: f64) {
        if self.frozen.get(i).copied().unwrap_or(false)
            || !gas_passable(self.f(), i)
            || self.e().gas_volume.as_ref().unwrap()[i] <= 0.
            || !matches!(
                self.e().gas_type.as_ref().unwrap()[i].as_deref(),
                Some("explosive" | "hydrogen")
            )
        {
            return;
        }
        self.source_owner = owner;
        self.source_player = player;
        self.source_cell = None;
        if self.external_reaction == 0 {
            self.external_reaction = self.allocate()
        }
        self.source_reaction = self.external_reaction;
        self.expose(i, true, false);
    }
    fn size(&self) -> GridSize {
        GridSize {
            width: self.f().width(),
            height: self.f().height(),
        }
    }
    fn los(&self, from: usize, to: usize) -> bool {
        let f = self.f();
        let mut a = crate::Actor::default();
        a.x = (from % f.width()) as f64;
        a.y = (from / f.width()) as f64;
        let mut b = crate::Actor::default();
        b.x = (to % f.width()) as f64;
        b.y = (to / f.width()) as f64;
        crate::vision::line_of_sight(self.run, &a, &b)
    }
    /// Closure must finish before victim damage. O(B*R²*LOS + V*alpha(V)),
    /// B consumed explosive cells, R<=9; LOS cost follows the current ray length.
    pub fn resolve(&mut self) -> Result<Vec<Burst>, String> {
        let size = self.size();
        let mut q = 0;
        while q < self.pending.len() {
            let burst = self.pending[q].clone();
            q += 1;
            let ox = burst.at % size.width;
            let oy = burst.at / size.width;
            for y in oy.saturating_sub(burst.radius)..=(oy + burst.radius).min(size.height - 1) {
                for x in ox.saturating_sub(burst.radius)..=(ox + burst.radius).min(size.width - 1) {
                    let i = y * size.width + x;
                    let distance =
                        ((x as f64 - ox as f64).powi(2) + (y as f64 - oy as f64).powi(2)).sqrt();
                    if self.frozen.get(i).copied().unwrap_or(false)
                        || !gas_passable(self.f(), i)
                        || !matches!(
                            self.e().gas_type.as_ref().unwrap()[i].as_deref(),
                            Some("explosive" | "hydrogen")
                        )
                        || distance > burst.radius as f64
                        || !self.los(burst.at, i)
                    {
                        continue;
                    }
                    if self.active(i) && self.reaction(i) != 0 {
                        self.merge(burst.reaction, self.reaction(i));
                    }
                    if self.e().gas_volume.as_ref().unwrap()[i] <= 0. {
                        continue;
                    }
                    self.source_owner = burst.owner;
                    self.source_player = burst.player;
                    self.source_reaction = self.reactions.find(burst.reaction)?;
                    self.source_cell = Some(burst.at);
                    self.expose(i, true, false);
                }
            }
        }
        let n = self.f().tiles.len();
        let mut visited = vec![false; n];
        for start in 0..n {
            if visited[start]
                || self.frozen.get(start).copied().unwrap_or(false)
                || !self.active(start)
            {
                continue;
            }
            let mut connected = vec![start];
            visited[start] = true;
            let mut reaction = 0;
            let mut q = 0;
            while q < connected.len() {
                let at = connected[q];
                q += 1;
                reaction = self.merge(reaction, self.reaction(at));
                let neighbors = [
                    if at % size.width > 0 {
                        Some(at - 1)
                    } else {
                        None
                    },
                    if at % size.width < size.width - 1 {
                        Some(at + 1)
                    } else {
                        None
                    },
                    at.checked_sub(size.width),
                    at.checked_add(size.width).filter(|&j| j < n),
                ];
                for j in neighbors.into_iter().flatten() {
                    if visited[j] || self.frozen.get(j).copied().unwrap_or(false) || !self.active(j)
                    {
                        continue;
                    }
                    visited[j] = true;
                    connected.push(j);
                }
            }
            if reaction > 0 {
                for i in connected {
                    self.em().fire_reaction.as_mut().unwrap()[i] = reaction as f64;
                }
            }
        }
        for i in 0..n {
            let id = if self.active(i) {
                self.reactions.find(self.reaction(i))?
            } else {
                0
            };
            self.em().fire_reaction.as_mut().unwrap()[i] = id as f64;
        }
        let mut bursts = std::mem::take(&mut self.pending);
        for burst in &mut bursts {
            burst.reaction = self.reactions.find(burst.reaction)?;
        }
        self.exploded.clear();
        environment::refresh_gas_views(self.em());
        environment::refresh_fire_views(self.em());
        environment::finish_vent_clearing(self.fm())?;
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        Ok(bursts)
    }
    pub fn advance(&mut self) -> Result<Vec<Burst>, String> {
        let n = self.f().tiles.len();
        for i in 0..n {
            if scalar(&self.f().ichor, i) > 0. && self.active(i) {
                self.em().fire_kind.as_mut().unwrap()[i] = Some("ichor".into())
            }
        }
        let mut permanent = self.f().permanent_fire.value_or_default();
        if crate::terrain_runtime::enabled(self.run) {
            permanent.extend(self.f().hellfire.as_ref().into_iter().flatten().enumerate()
                .filter_map(|(i,&active)|active.then_some(i as f64)));
        }
        for i in permanent {
            let i = i as usize;
            self.em().fire_kind.as_mut().unwrap()[i] = Some("brazier".into());
            self.em().fire[i] = 1.;
        }
        propagation::promote_timed_layers(self);
        self.em().fire_fresh.as_mut().unwrap().fill(false);
        environment::refresh_fire_views(self.em());
        propagation::spread_existing_fire(self);
        self.resolve()
    }
}
impl RandomInt for FireAdapter<'_> {
    fn random_int(&mut self, min: u32, max: u32) -> u32 {
        if min == max {
            min
        } else {
            min + (crate::rng::random(self.run) * (max - min + 1) as f64).floor() as u32
        }
    }
}
impl FireScan for FireAdapter<'_> {
    fn size(&self) -> GridSize {
        self.size()
    }
    fn is_fire(&self, i: usize) -> bool {
        self.e().fire[i] > 0. && !self.frozen.get(i).copied().unwrap_or(false)
    }
    fn caught_fire(&self, i: usize) -> bool {
        self.e().fire_fresh.as_ref().unwrap()[i]
    }
    fn spreads_neighbors(&self, i: usize) -> bool {
        self.e().fire_kind.as_ref().unwrap()[i].as_deref() != Some("ichor")
            || !crate::ichor_fire_rules::current(self.run)
            || self.run.tick.rem_euclid(2.) == 0.
    }
    fn expose(&mut self, i: usize, source: usize) {
        let p = self.e().provenance.as_ref().unwrap().get("fire").unwrap();
        let (owner, player) = (p.owner[source], p.player[source]);
        self.source_owner = owner;
        self.source_player = player;
        self.source_cell = Some(source);
        self.source_reaction = self.reaction(source);
        self.expose(i, false, false);
    }
}
impl TimedPromotions for FireAdapter<'_> {
    fn size(&self) -> GridSize {
        self.size()
    }
    fn layers(&self, i: usize) -> Vec<PromotionLayer> {
        self.e().fire_kind.as_ref().unwrap()[i]
            .as_ref()
            .map(|kind| {
                vec![PromotionLayer {
                    id: kind.clone(),
                    chance: match kind.as_str() {
                        "plain" => 500,
                        "gas" => 8000,
                        "explosion" | "gas-explosion" => 10000,
                        "creature" => 3000,
                        "embers" => 300,
                        _ => 0,
                    },
                }]
            })
            .unwrap_or_default()
    }
    fn caught_fire(&self, i: usize) -> bool {
        self.e().fire_fresh.as_ref().unwrap()[i]
    }
    fn passable(&self, i: usize) -> bool {
        gas_passable(self.f(), i)
    }
    fn same_tile(&self, i: usize, j: usize, _: &PromotionLayer) -> bool {
        self.e().fire_kind.as_ref().unwrap()[i] == self.e().fire_kind.as_ref().unwrap()[j]
    }
    fn promote(&mut self, i: usize, layer: PromotionLayer) {
        self.em().fire_kind.as_mut().unwrap()[i] = if layer.id == "plain" || layer.id == "creature"
        {
            Some("embers".into())
        } else {
            None
        };
        self.em().fire_reaction.as_mut().unwrap()[i] = 0.;
    }
}

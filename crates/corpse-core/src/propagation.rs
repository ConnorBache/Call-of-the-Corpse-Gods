//! Exact v48 `brogue-propagation.ts` behavioral kernels, independent of world storage.
//! All visits are x-major; gas types mutate live while volumes are buffered.
pub const CARDINALS: [(isize, isize); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
pub const NEIGHBORS: [(isize, isize); 8] = [
    (0, -1),
    (0, 1),
    (-1, 0),
    (1, 0),
    (-1, -1),
    (-1, 1),
    (1, -1),
    (1, 1),
];
#[derive(Clone, Copy, Debug)]
pub struct GridSize {
    pub width: usize,
    pub height: usize,
}
impl GridSize {
    pub fn adjacent(self, i: usize, dx: isize, dy: isize) -> Option<usize> {
        let x = (i % self.width) as isize + dx;
        let y = (i / self.width) as isize + dy;
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            None
        } else {
            Some(y as usize * self.width + x as usize)
        }
    }
    fn scan(self) -> impl Iterator<Item = usize> {
        (0..self.width).flat_map(move |x| (0..self.height).map(move |y| y * self.width + x))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dissipation {
    None,
    Normal,
    Quick,
}
pub trait RandomInt {
    fn random_int(&mut self, min: u32, max: u32) -> u32;
}
pub fn percent(r: &mut impl RandomInt, p: f64) -> bool {
    (r.random_int(0, 99) as f64) < p.clamp(0., 100.)
}
pub trait GasGrid: RandomInt {
    type Gas: Clone + PartialEq;
    fn size(&self) -> GridSize;
    fn volume(&self, i: usize) -> u16;
    fn set_volume(&mut self, i: usize, v: u16);
    fn gas(&self, i: usize) -> Option<Self::Gas>;
    fn set_gas(&mut self, i: usize, g: Option<Self::Gas>);
    fn blocks_gas(&self, i: usize) -> bool;
    fn auto_descent(&self, _i: usize) -> bool {
        false
    }
    fn frozen(&self, _i: usize) -> bool {
        false
    }
    fn dissipation(&self, gas: Option<Self::Gas>) -> Dissipation;
    fn on_type_change(
        &mut self,
        _i: usize,
        _previous: Option<Self::Gas>,
        _next: Option<Self::Gas>,
        _source: usize,
    ) {
    }
    fn on_cell_resolved(
        &mut self,
        _i: usize,
        _source: usize,
        _contributors: &[usize],
        _output: u16,
    ) {
    }
}
fn change_type<G: GasGrid>(g: &mut G, i: usize, next: Option<G::Gas>, source: usize) {
    let previous = g.gas(i);
    g.set_gas(i, next.clone());
    if previous != next {
        g.on_type_change(i, previous, next, source)
    }
}
/// O(V) time and auxiliary memory; at most eight neighbors and nine contributors
/// per cell. `on_*` callback costs are additional. Precisely one diffusion pass.
pub fn diffuse_gas_pass<G: GasGrid>(g: &mut G) {
    let size = g.size();
    let n = size.width * size.height;
    let mut output = vec![0u16; n];
    let mut sources: Vec<_> = (0..n).collect();
    let mut contributors: Vec<_> = (0..n).map(|i| vec![i]).collect();
    for i in size.scan() {
        if g.frozen(i) {
            output[i] = g.volume(i);
            let mode = g.dissipation(g.gas(i));
            if g.volume(i) > 0
                && mode != Dissipation::None
                && percent(g, if mode == Dissipation::Quick { 50. } else { 20. })
            {
                output[i] = output[i].wrapping_sub(1)
            }
            continue;
        }
        let outlets: Vec<_> = NEIGHBORS
            .iter()
            .filter_map(|&(dx, dy)| size.adjacent(i, dx, dy))
            .filter(|&j| !g.blocks_gas(j) && !g.frozen(j))
            .collect();
        if g.blocks_gas(i) {
            if g.volume(i) == 0 {
                continue;
            }
            if !outlets.is_empty() {
                let share = g.volume(i) / outlets.len() as u16;
                if share > 0 {
                    for &j in &outlets {
                        output[j] = output[j].wrapping_add(share);
                        contributors[j].push(i);
                        sources[j] = i;
                        change_type(g, j, g.gas(i), i);
                    }
                }
            }
            output[i] = 0;
            change_type(g, i, None, i);
            continue;
        }
        let mut sum = g.volume(i) as u32;
        let mut count = 1u32;
        let mut highest = g.volume(i);
        let mut winner = g.gas(i);
        let mut source = i;
        for j in outlets {
            sum += g.volume(j) as u32;
            count += 1;
            contributors[i].push(j);
            if g.volume(j) > highest {
                highest = g.volume(j);
                winner = g.gas(j);
                source = j
            }
        }
        if g.auto_descent(i) {
            count += 1
        }
        output[i] = output[i].wrapping_add((sum / count) as u16);
        if count > 1 && g.random_int(0, count - 1) < sum % count {
            output[i] = output[i].wrapping_add(1)
        }
        sources[i] = source;
        if winner != g.gas(i) && output[i] > 3 {
            if g.gas(i).is_some() {
                output[i] = 3
            }
            change_type(g, i, winner, source)
        } else if g.gas(i).is_some() && output[i] < 1 {
            change_type(g, i, None, i)
        }
        if g.volume(i) > 0 {
            let mode = g.dissipation(g.gas(i));
            if mode != Dissipation::None
                && percent(g, if mode == Dissipation::Quick { 50. } else { 20. })
            {
                output[i] = output[i].wrapping_sub(1)
            }
        }
    }
    for i in size.scan() {
        g.on_cell_resolved(i, sources[i], &contributors[i], output[i])
    }
    for (i, &volume) in output.iter().enumerate() {
        g.set_volume(i, volume)
    }
}
#[derive(Clone, Debug)]
pub struct FireLayer {
    pub id: String,
    pub flammable: bool,
    pub ignition_chance: f64,
    pub draw_priority: f64,
    pub extinguishes: bool,
    pub is_gas: bool,
    pub explosive: bool,
}
pub trait FireExposure: RandomInt {
    fn size(&self) -> GridSize;
    fn layers(&self, i: usize) -> Vec<FireLayer>;
    fn exposure(&self, i: usize) -> u32;
    fn increment_exposure(&mut self, i: usize);
    fn qualifies_for_confinement(&self, i: usize) -> bool;
    fn promote(&mut self, i: usize, layer: FireLayer, explosive: bool);
}
pub fn explosively_confined(size: GridSize, i: usize, qualifies: impl Fn(usize) -> bool) -> bool {
    NEIGHBORS
        .iter()
        .all(|&(dx, dy)| size.adjacent(i, dx, dy).is_some_and(&qualifies))
}
pub fn expose_fire<G: FireExposure>(g: &mut G, i: usize, always_ignite: bool) -> bool {
    let layers = g.layers(i);
    if !layers.iter().any(|l| l.flammable) || g.exposure(i) >= 12 {
        return false;
    }
    g.increment_exposure(i);
    let extinguishing = layers
        .iter()
        .filter(|l| l.extinguishes)
        .fold(1000f64, |p, l| p.min(l.draw_priority));
    let probability = layers
        .iter()
        .filter(|l| l.flammable && (l.is_gas || l.draw_priority <= extinguishing))
        .fold(0f64, |p, l| p.max(l.ignition_chance));
    if !always_ignite && (probability == 0. || !percent(g, probability)) {
        return false;
    }
    let confined = layers.iter().any(|l| l.explosive)
        && explosively_confined(g.size(), i, |j| g.qualifies_for_confinement(j));
    for slot in 0..layers.len() {
        if let Some(layer) = g.layers(i).get(slot).cloned() {
            if layer.flammable {
                g.promote(i, layer, confined)
            }
        }
    }
    true
}
pub trait FireScan {
    fn size(&self) -> GridSize;
    fn is_fire(&self, i: usize) -> bool;
    fn caught_fire(&self, i: usize) -> bool;
    fn spreads_neighbors(&self, _i: usize) -> bool { true }
    fn expose(&mut self, i: usize, source: usize);
}
pub fn spread_existing_fire(g: &mut impl FireScan) {
    let size = g.size();
    for i in size.scan() {
        if !g.is_fire(i) || g.caught_fire(i) {
            continue;
        }
        // Same-cell interactions still run even when this source's cadence
        // does not allow it to spread across an edge on this world step.
        let spreads = g.spreads_neighbors(i);
        g.expose(i, i);
        if !spreads { continue; }
        for (dx, dy) in CARDINALS {
            if let Some(j) = size.adjacent(i, dx, dy) {
                g.expose(j, i)
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct PromotionLayer {
    pub id: String,
    pub chance: i32,
}
pub trait TimedPromotions: RandomInt {
    fn size(&self) -> GridSize;
    fn layers(&self, i: usize) -> Vec<PromotionLayer>;
    fn caught_fire(&self, i: usize) -> bool;
    fn passable(&self, i: usize) -> bool;
    fn same_tile(&self, i: usize, j: usize, layer: &PromotionLayer) -> bool;
    fn promote(&mut self, i: usize, layer: PromotionLayer);
}
/// O(V L) plus callback costs, O(V L) selection storage. Selection precedes mutation.
pub fn promote_timed_layers(g: &mut impl TimedPromotions) {
    let size = g.size();
    let mut selected = vec![];
    for i in size.scan() {
        if g.caught_fire(i) {
            continue;
        }
        for (slot, layer) in g.layers(i).iter().enumerate() {
            let mut chance = layer.chance;
            if chance < 0 {
                let increment = -chance;
                chance = 0;
                for (dx, dy) in CARDINALS {
                    if let Some(j) = size.adjacent(i, dx, dy) {
                        if g.passable(j) && !g.same_tile(i, j, layer) {
                            chance += increment
                        }
                    }
                }
            }
            if chance != 0 && (g.random_int(0, 10000) as i32) < chance {
                selected.push((i, slot))
            }
        }
    }
    for (i, slot) in selected {
        if let Some(layer) = g.layers(i).get(slot).cloned() {
            g.promote(i, layer)
        }
    }
}

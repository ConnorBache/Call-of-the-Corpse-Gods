//! Rebuildable tile-light cache. Dependency capture runs once per immutable
//! visibility pass, not once per tile. Exact projected dependencies (not a hash
//! or tick alone) detect same-tick/direct edits and survive save/rewind safely.
//! Legacy actor queries use an exact second-best field for active own Shine.
//! Revision46 omits suppressed Shine sources from every field and dependency.
use crate::{Actor, Run};
use crate::movement::Point;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
struct PositiveSource { x: i32, y: i32, power: u64, fire: bool, exclude: Option<u64> }
#[derive(Debug, Clone, PartialEq)]
struct DarkSource { x: u64, y: u64 }
#[derive(Debug, Clone, PartialEq)]
struct Dependencies {
    width: usize,
    height: usize,
    ambient: Vec<u64>,
    opaque: Vec<bool>,
    positive: Vec<PositiveSource>,
    dark: Vec<DarkSource>,
    potion_darkness: Vec<(usize,u64)>,
    /// Semantic version is a dependency even when same-tick mask data is identical.
    absolute_darkness: bool,
    /// Per-emitter optical LOS, excluding its own tile. Empty before R50.
    ring_light: Vec<(usize, Vec<bool>)>,
}

/// O(1): mirror f64 identity equality, including signed zero and NaN.
/// NaN never equals an excluded actor ID; infinities retain ordinary equality.
fn exclusion_key(id: f64) -> Option<u64> {
    if id.is_nan() { None } else { Some(if id == 0. { 0.0f64.to_bits() } else { id.to_bits() }) }
}

impl Dependencies {
    /// O(V + A*B + L) time/space for V cells, A actors, B ability entries and L
    /// fixed lights; door lookup is expected hash time plus cell-key formatting.
    /// Only light-affecting values enter equality: ordinary HP changes, map
    /// memory, tick, RNG and non-emitting creature movement do not dirty it.
    fn capture(r: &Run, dream: bool) -> Option<Self> {
        let f = crate::dreamwalking::floor_in_plane(r, dream).or_else(|| r.floor())?;
        let width = f.width();
        let height = f.height();
        let n = width * height;
        let mut positive = Vec::new();
        if let Some(env) = f.environment.as_ref() {
            for (i, &fire) in env.fire.iter().take(n).enumerate() {
                if !(fire <= 0.) { positive.push(PositiveSource { x: (i % width) as i32,
                    y: (i / width) as i32, power: 80f64.to_bits(), fire: true, exclude: None }); }
            }
        }
        if let Some(sources) = f.light_sources.as_ref() {
            for s in sources {
                if s.power.is_finite() && s.falloff.is_finite() && s.falloff > 0. {
                    positive.push(PositiveSource { x: s.x as i32, y: s.y as i32,
                        power: s.power.to_bits(), fire: false, exclude: None });
                }
            }
        }
        let in_plane = |a: &Actor| !crate::dreamwalking::enabled(r)
            || crate::dreamwalking::dream_actor(r, a) == dream;
        let mut dark = Vec::new();
        for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
            if !in_plane(a) || a.dead.value_or(false) { continue; }
            if !crate::dreamwalking::is_body(a) && crate::vision::emits_shine(r, a) {
                positive.push(PositiveSource { x: a.x as i32, y: a.y as i32,
                    power: 100f64.to_bits(), fire: false,
                    exclude: if a.active.iter().any(|v| v == "anavani-shine") { exclusion_key(a.id) } else { None } });
            }
            if !(a.hp <= 0.) && a.abilities.iter().any(|v| v == "enemy:beast:darkness-aura") {
                dark.push(DarkSource { x: a.x.to_bits(), y: a.y.to_bits() });
            }
        }
        // Capture exact route reachability once per emitter, including portals.
        // Derived geometry checks observe same-tick walls/doors/portal edits.
        let groups = crate::vision::ring_light_groups(r,dream);
        let ring_light = if groups.is_empty() { Vec::new() } else {
            let geometry = r.optical_cache.field(f,crate::portals::enabled(r));
            groups.into_iter().map(|group| {
                let origins:Vec<_> = group.origins.iter().map(|a|Point{x:a.x as i32,y:a.y as i32}).collect();
                let mut mask = vec![false;n];
                for &origin in &origins {
                    for (i,d) in geometry.distances(origin).iter().enumerate() { mask[i] |= d.is_finite(); }
                }
                for origin in origins {
                    if origin.x>=0 && origin.y>=0 && (origin.x as usize)<width && (origin.y as usize)<height {
                        mask[origin.y as usize*width+origin.x as usize]=false;
                    }
                }
                (group.copies,mask)
            }).collect()
        };
        Some(Self { width, height,
            ambient: (0..n).map(|i| f.lights.get(i).copied().unwrap_or(0.).to_bits()).collect(),
            opaque: (0..n).map(|i| matches!(f.tiles[i].as_str(), "wall" | "tree")
                || crate::environment::blocks_sight(f, i)).collect(),
            positive, dark, ring_light, absolute_darkness: crate::revision50_potions::absolute_darkness(r),
            potion_darkness: if crate::revision50::enabled(r) {f.revision50_darkness.as_ref().into_iter().flatten().filter(|d|d.expires_at>r.tick).map(|d|(d.cell,d.amount.to_bits())).collect()} else {Vec::new()} })
    }

    /// O(V + sum positive-source footprint*ray + D*V*ray) time, O(V) result.
    /// Ranges limit positive work; dark sources retain original actor order and
    /// floating-point x/y arithmetic so every tile subtracts in identical order.
    /// Original positive lighting takes a max; source-major traversal preserves
    /// exact selected values. Opaque destinations remain visible as in floor_ray.
    fn build(&self) -> LightField {
        let _span = crate::timing::span("vision.lighting_field_build");
        let mut values: Vec<f64> = self.ambient.iter().map(|&b| f64::from_bits(b).max(-100.)).collect();
        let has_exclusions = self.positive.iter().any(|s| s.exclude.is_some());
        let mut excluding = if has_exclusions { values.clone() } else { Vec::new() };
        let mut owners = if has_exclusions { vec![None; values.len()] } else { Vec::new() };
        let mut ring_bonus = if self.ring_light.is_empty() { Vec::new() } else { vec![0.; values.len()] };
        if self.width == 0 || self.height == 0 {
            return LightField { width: self.width, height: self.height, values, excluding, owners, ring_bonus };
        }
        for s in &self.positive {
            let power = f64::from_bits(s.power);
            if power <= 0. { continue; }
            let radius = if s.fire { 7 } else {
                (power / 10.).ceil().min(self.width.max(self.height) as f64 + 1.) as i64
            };
            // i64 intermediates preserve empty out-of-bounds source ranges without
            // overflowing at extreme admitted or directly constructed coordinates.
            let min_x = (s.x as i64 - radius).max(0);
            let max_x = (s.x as i64 + radius).min(self.width as i64 - 1);
            let min_y = (s.y as i64 - radius).max(0);
            let max_y = (s.y as i64 + radius).min(self.height as i64 - 1);
            for y in min_y..=max_y { for x in min_x..=max_x {
                let p = Point { x: x as i32, y: y as i32 };
                let source = Point { x: s.x, y: s.y };
                let amount = power - 10. * crate::vision::distance(source, p);
                let i = y as usize * self.width + x as usize;
                if amount > 0. && amount > if owners.is_empty() || s.exclude == owners[i] { values[i] } else { excluding[i] }
                    && crate::vision::ray(self.width, self.height, &self.opaque, source, p)
                {
                    if owners.is_empty() || s.exclude == owners[i] { values[i] = amount; }
                    else if amount > values[i] {
                        excluding[i] = values[i]; values[i] = amount; owners[i] = s.exclude;
                    } else { excluding[i] = amount; }
                }
            }}
        }
        for v in values.iter_mut().chain(excluding.iter_mut()) { *v = v.clamp(-100., 100.); }
        for s in &self.dark {
            let x = f64::from_bits(s.x); let y = f64::from_bits(s.y);
            let origin = Point { x: x as i32, y: y as i32 };
            for (i, value) in values.iter_mut().enumerate() {
                let p = Point { x: (i % self.width) as i32, y: (i / self.width) as i32 };
                let amount = (100. - 10. * ((x - p.x as f64).abs() + (y - p.y as f64).abs())).max(0.);
                if amount > 0. && crate::vision::ray(self.width, self.height, &self.opaque, origin, p) {
                    *value -= amount;
                    if !excluding.is_empty() { excluding[i] -= amount; }
                }
            }
        }
        for v in values.iter_mut().chain(excluding.iter_mut()) { *v = v.clamp(-100., 100.); }
        for &(cell,amount) in &self.potion_darkness {
            values[cell]=(values[cell]-f64::from_bits(amount)).clamp(-100.,100.);
            if !excluding.is_empty(){excluding[cell]=(excluding[cell]-f64::from_bits(amount)).clamp(-100.,100.);}
        }
        for (copies,mask) in &self.ring_light {
            for (i,&lit) in mask.iter().enumerate() { if lit { ring_bonus[i] += 10. * *copies as f64; }}
        }
        for (i,&bonus) in ring_bonus.iter().enumerate() {
            values[i] += bonus;
            if !excluding.is_empty() { excluding[i] += bonus; }
        }
        for v in values.iter_mut().chain(excluding.iter_mut()) { *v = v.clamp(-100., 100.); }
        // O(D): final R52 override wins over every emitter, including additive
        // rings and active-own-Shine exclusion. Original source buffers survive.
        if self.absolute_darkness {
            for &(cell,_) in &self.potion_darkness {
                values[cell]=-100.;
                if !excluding.is_empty() {excluding[cell]=-100.;}
                if !ring_bonus.is_empty() {ring_bonus[cell]=0.;}
            }
        }
        LightField { width: self.width, height: self.height, values, excluding, owners, ring_bonus }
    }
}

#[derive(Debug)]
pub struct LightField { width: usize, height: usize, values: Vec<f64>, excluding: Vec<f64>, owners: Vec<Option<u64>>, ring_bonus: Vec<f64> }
impl LightField {
    /// Additive ring component, independent of ambient and absolute aura light.
    fn ring_bonus_at(&self,p:Point)->f64 {
        if p.x<0 || p.y<0 || p.x as usize>=self.width || p.y as usize>=self.height {0.}
        else {self.ring_bonus.get(p.y as usize*self.width+p.x as usize).copied().unwrap_or(0.)}
    }
    /// O(1) lookup; out-of-bounds matches the uncached lighting helper.
    pub fn at(&self, p: Point) -> f64 {
        if p.x < 0 || p.y < 0 || p.x as usize >= self.width || p.y as usize >= self.height { -100. }
        else { self.values[p.y as usize * self.width + p.x as usize] }
    }
    /// O(1) exact source-excluding lookup; no actor, source or cell scanning.
    /// Positive max is chosen before clamping and ordered darkness subtraction.
    pub fn at_excluding(&self, p: Point, id: f64) -> f64 {
        if p.x < 0 || p.y < 0 || p.x as usize >= self.width || p.y as usize >= self.height { return -100.; }
        let i = p.y as usize * self.width + p.x as usize;
        if !self.owners.is_empty() && exclusion_key(id).is_some_and(|key| self.owners[i] == Some(key)) { self.excluding[i] }
        else { self.values[i] }
    }

}
#[derive(Debug)]
struct Entry { dependencies: Dependencies, field: Arc<LightField> }
#[derive(Debug, Default)]
pub struct LightingCache(Mutex<[Option<Entry>; 2]>);
impl Clone for LightingCache {
    /// O(1): fresh cache on transaction/snapshot clone; never share mutable cache
    /// ownership between independently edited worlds or retain stale world state.
    fn clone(&self) -> Self { Self::default() }
}
impl PartialEq for LightingCache {
    /// O(1): purely derived cache does not participate in gameplay equality.
    fn eq(&self, _: &Self) -> bool { true }
}
impl LightingCache {
    /// O(1) test-only check of the bounded two-plane cache.
    #[cfg(test)]
    pub(crate) fn is_empty_for_test(&self) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).iter().all(Option::is_none)
    }

    /// Capture/equality O(V+A*B+L) each immutable read pass. Clean result is an
    /// O(1) Arc clone afterward; dirty build costs Dependencies::build. Cache owns
    /// at most two O(V+sources) entries, not a growing history. No probabilistic
    /// hash signature and no tick-only identity can hide same-tick mutations.
    pub fn field(&self, r: &Run, dream: bool) -> Option<Arc<LightField>> {
        let _span = crate::timing::span("vision.lighting_dependency_check");
        let dependencies = Dependencies::capture(r, dream)?;
        let mut slots = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = slots[usize::from(dream)].as_ref() {
            if entry.dependencies == dependencies { crate::timing::count(crate::timing::Count::LightingHit,1);return Some(entry.field.clone()); }
        }
        crate::timing::count(crate::timing::Count::LightingMiss,1);
        // If both planes have identical projected lighting inputs (including the
        // absent dream-floor fallback), share their immutable result too.
        let field = slots[usize::from(!dream)].as_ref()
            .filter(|entry| entry.dependencies == dependencies)
            .map(|entry| entry.field.clone())
            .unwrap_or_else(|| Arc::new(dependencies.build()));
        slots[usize::from(dream)] = Some(Entry { dependencies, field: field.clone() });
        Some(field)
    }
}

/// At most two immutable fields for a read-only visibility pass. Borrowing Run
/// makes it impossible for ordinary callers to mutate simulation during the pass.
pub struct LightingProjection<'a> { _world: std::marker::PhantomData<&'a Run>, fields: [Option<Arc<LightField>>; 2], context: bool }
impl<'a> LightingProjection<'a> {
    /// O(2*(V+A*B+L)) dependency work plus dirty rebuilds; O(1) projection handles.
    pub fn new(r: &'a Run) -> Self {
        Self { _world: std::marker::PhantomData, fields: [r.lighting_cache.field(r, false), r.lighting_cache.field(r, true)],
            context: crate::dreamwalking::context_dream(r) }
    }
    /// O(1) tile query; absent field reproduces missing-floor darkness.
    pub fn in_plane(&self, p: Point, dream: bool) -> f64 {
        self.fields[usize::from(dream)].as_ref().map_or(-100., |f| f.at(p))
    }
    /// O(1) additive ring light, also applied to Darkness Aura's absolute base.
    /// R52 potion-covered cells report zero so the aura cannot bypass the mask.
    pub fn ring_bonus_in_plane(&self,p:Point,dream:bool)->f64 {
        self.fields[usize::from(dream)].as_ref().map_or(0.,|f|f.ring_bonus_at(p))
    }
    /// O(1) light with active Shine belonging to `id` excluded, in a chosen plane.
    /// Target-specific shadow/aura modifiers remain the caller's responsibility.
    pub fn in_plane_excluding(&self, p: Point, dream: bool, id: f64) -> f64 {
        self.fields[usize::from(dream)].as_ref().map_or(-100., |f| f.at_excluding(p, id))
    }
    /// O(1) source-excluding light in the current simulation plane.
    pub fn current_excluding(&self, p: Point, id: f64) -> f64 {
        self.in_plane_excluding(p, self.context, id)
    }
    /// O(1) query for current simulation plane.
    pub fn current(&self, p: Point) -> f64 { self.in_plane(p, self.context) }
}

#[cfg(test)]
mod exclusion_tests {
    use super::*;

    /// O(S*R + D*R) independent target-major oracle; matches original sparse order.
    fn reference(d: &Dependencies, p: Point, exclude: Option<f64>) -> f64 {
        let i = p.y as usize * d.width + p.x as usize;
        let mut value = f64::from_bits(d.ambient[i]).max(-100.);
        for s in &d.positive {
            if s.exclude.is_some() && s.exclude == exclude.and_then(exclusion_key) { continue; }
            if s.fire && ((s.x-p.x).abs()>7 || (s.y-p.y).abs()>7) { continue; }
            let source = Point{x:s.x,y:s.y};
            let amount = f64::from_bits(s.power)-10.*crate::vision::distance(source,p);
            if amount > 0. && amount > value && crate::vision::ray(d.width,d.height,&d.opaque,source,p) {value=amount;}
        }
        value=value.clamp(-100.,100.);
        for s in &d.dark {
            let (x,y)=(f64::from_bits(s.x),f64::from_bits(s.y));
            let amount=(100.-10.*((x-p.x as f64).abs()+(y-p.y as f64).abs())).max(0.);
            if amount>0. && crate::vision::ray(d.width,d.height,&d.opaque,Point{x:x as i32,y:y as i32},p) {value-=amount;}
        }
        value=value.clamp(-100.,100.);
        for (copies,mask) in &d.ring_light { if mask[i] { value+=10. * *copies as f64; } }
        value.clamp(-100.,100.)
    }

    /// O(cases*V*S*R), deterministic differential coverage; no gameplay RNG.
    #[test]
    fn excluded_sources_match_target_major_oracle_bit_for_bit() {
        for case in 0..48 {
            let mut d=Dependencies{width:11,height:11,
                ambient:(0..121).map(|i|(-100.+((i*7+case)%211) as f64/1.25).to_bits()).collect(),
                opaque:(0..121).map(|i|(i*13+case)%19==0).collect(),positive:vec![],dark:vec![],ring_light:vec![],potion_darkness:vec![],absolute_darkness:false};
            for j in 0..17 {
                d.positive.push(PositiveSource{x:((j*3+case)%13) as i32-1,y:((j*7+case)%13) as i32-1,
                    power:(40.+((j*11+case)%101) as f64/1.5).to_bits(),fire:j%5==0,
                    exclude:if j%3==0{None}else{Some((j%4) as u64)}});
            }
            for j in 0..4 {d.dark.push(DarkSource{x:(((j*3+case)%11) as f64+0.125).to_bits(),y:(((j*7+case)%11) as f64+0.875).to_bits()});}
            let f=d.build();
            for i in 0..121 {
                let p=Point{x:(i%11) as i32,y:(i/11) as i32};
                assert_eq!(f.at(p).to_bits(),reference(&d,p,None).to_bits(),"case={case}, cell={i}");
                for key in 0..5 {let id=f64::from_bits(key);
                    assert_eq!(f.at_excluding(p,id).to_bits(),reference(&d,p,Some(id)).to_bits(),"case={case}, cell={i}, key={key}");
                }
            }
        }
    }

    /// O(V+A*B+L) captures per edit; verifies exact same-tick exclusion invalidation.
    #[test]
    fn active_shine_identity_and_plane_edits_invalidate() {
        let mut r=Run{depth:1.,simulation_revision:39.0.into(),..Default::default()};
        r.player=Actor{id:0.,x:2.,y:2.,hp:100.,abilities:vec!["anavani-shine".into()],..Default::default()};
        r.floors.push(Arc::new(crate::Floor{depth:1.,tiles:vec!["floor".into();25],lights:vec![-50.;25],..Default::default()}));
        let p=Point{x:2,y:2};
        let initial=r.lighting_cache.field(&r,false).unwrap();
        assert_eq!(initial.at_excluding(p,0.),100.);
        r.player.active.push("anavani-shine".into());
        let active=r.lighting_cache.field(&r,false).unwrap();
        assert!(!Arc::ptr_eq(&initial,&active));assert_eq!(active.at_excluding(p,-0.),-50.);
        r.player.id=7.;
        let renamed=r.lighting_cache.field(&r,false).unwrap();
        assert_eq!(renamed.at_excluding(p,0.),100.);assert_eq!(renamed.at_excluding(p,7.),-50.);
        r.player.conditions.insert("phantasmiiPlane".into(),1.);
        let physical=r.lighting_cache.field(&r,false).unwrap();
        let dream=r.lighting_cache.field(&r,true).unwrap();
        assert_eq!(physical.at(p),-50.);assert_eq!(dream.at_excluding(p,7.),-50.);assert_eq!(dream.at(p),100.);
        assert_eq!(dream.at_excluding(Point{x:-1,y:0},7.),-100.);
    }
}

#[cfg(test)]
mod world_exclusion_tests {
    use super::*;
    /// O(planes*V*A*(sources*R)): compare against untouched scalar implementation.
    fn compare(r: &Run) {
        for dream in [false,true] {
            let field=r.lighting_cache.field(r,dream).unwrap();
            let f=crate::dreamwalking::floor_in_plane(r,dream).or_else(||r.floor()).unwrap();
            for i in 0..f.tiles.len() {let p=Point{x:(i%f.width())as i32,y:(i/f.width())as i32};
                for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
                    assert_eq!(field.at_excluding(p,a.id).to_bits(),crate::vision::illumination_in_plane(r,p,Some(a.id),dream).to_bits(),"cell={i} id={} dream={dream}",a.id);
                }
            }
        }
    }
    /// O(edits*planes*V*A*sparse-light-cost): direct edits, plane and clone oracle.
    #[test]
    fn world_source_exclusion_matches_scalar_across_mutations() {
        let mut r=Run{depth:1.,simulation_revision:39.0.into(),..Default::default()};
        r.player=Actor{id:0.,x:2.,y:2.,hp:100.,abilities:vec!["anavani-shine".into()],active:vec!["anavani-shine".into()],..Default::default()};
        r.floors.push(Arc::new(crate::Floor{depth:1.,tiles:vec!["floor".into();121],lights:vec![-55.125;121],..Default::default()}));
        for i in 1..6 {let mut a=r.player.clone();a.id=i as f64;a.x=(i+2)as f64;a.y=(i+1)as f64;
            if i>3 {a.abilities=vec!["enemy:beast:darkness-aura".into()];a.x+=0.125;a.y+=0.875;}
            r.actors_at_mut(1.).push(a);
        }
        compare(&r);
        r.player.active.clear();compare(&r);
        r.actors_at_mut(1.)[0].active.clear();compare(&r);
        r.actors_at_mut(1.)[1].dead=true.into();compare(&r);
        r.floor_mut().unwrap().tiles[4*11+4]="wall".into();compare(&r);
        r.actors_at_mut(1.)[3].hp=0.;compare(&r);
        r.actors_at_mut(1.).swap(3,4);compare(&r);
        crate::dreamwalking::ensure_layer(&mut r);
        r.player.conditions.insert("phantasmiiPlane".into(),1.);compare(&r);
        compare(&r.clone());
        r.player.id=f64::NAN; compare(&r);
        r.player.id=f64::INFINITY; compare(&r);
        r.player.id=-0.; compare(&r);
        r.player.conditions.shift_remove("phantasmiiPlane");
        r.player.active.push("anavani-shine".into()); compare(&r);
        // Equal-strength colocated sources must survive exclusion of either owner.
        let mut colocated=r.player.clone(); colocated.id=99.;
        r.actors_at_mut(1.).push(colocated); compare(&r);
    }
}

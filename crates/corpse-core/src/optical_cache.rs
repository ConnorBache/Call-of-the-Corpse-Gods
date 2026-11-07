//! Exact directed Bresenham visibility fields; no shadowcasting approximation.
//! Dependencies are checked once per immutable bulk pass, never once per tile.
use crate::{Floor, movement::Point};
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
struct Dependencies { width: usize, height: usize, opaque: Vec<bool>, pairs: Vec<(usize, usize)> }
impl Dependencies {
    /// O(N + K) time/space, N tiles and K portal endpoints. Exact equality catches
    /// direct public edits, same-tick doors and moved portals; no hash collision.
    fn capture(f: &Floor, portals: bool) -> Self {
        Self { width: f.width(), height: f.height(),
            opaque: (0..f.tiles.len()).map(|i| matches!(f.tiles[i].as_str(), "wall"|"tree")
                || crate::environment::blocks_sight(f, i)).collect(),
            pairs: if portals { f.portals.as_ref().into_iter().flatten().map(|p| (p.a,p.b)).collect() } else { vec![] } }
    }
    /// O(1) bounds/opacity query. Portal transfers cannot pass blocked endpoints.
    fn open(&self, p: Point) -> bool { p.x>=0 && p.y>=0 && (p.x as usize)<self.width && (p.y as usize)<self.height
        && !self.opaque[p.y as usize*self.width+p.x as usize] }
    /// O(R) exact directed ray, R maximum dimension; O(1) scratch.
    fn clear(&self, a: Point, b: Point) -> bool { crate::vision::ray(self.width,self.height,&self.opaque,a,b) }
    /// O(1) point conversion; admitted floors have nonzero width.
    fn point(&self, i: usize) -> Point { Point {x:(i%self.width)as i32,y:(i/self.width)as i32} }
}
#[derive(Debug)]
pub(crate) struct OpticalField {
    dependencies: Dependencies,
    endpoints: Vec<Point>,
    /// Directed endpoint-to-tile distances; infinity means obstructed.
    outgoing: Vec<Vec<f64>>,
    origins: Mutex<Vec<(Point, Arc<Vec<f64>>)>>,
}
impl OpticalField {
    /// Cold O(K*N*R + K) time, O(K*N+N) memory. Portal views are shared across
    /// every observer and target, while retaining asymmetric raster tie rules.
    fn build(dependencies: Dependencies) -> Self {
        let _span=crate::timing::span("vision.optical_field_build");
        let endpoints:Vec<_>=dependencies.pairs.iter().flat_map(|&(a,b)|[dependencies.point(a),dependencies.point(b)]).collect();
        let outgoing=endpoints.iter().map(|&a|(0..dependencies.opaque.len()).map(|i|{
            let b=dependencies.point(i); if dependencies.clear(a,b){crate::vision::distance(a,b)}else{f64::INFINITY}
        }).collect()).collect();
        Self {dependencies,endpoints,outgoing,origins:Mutex::new(vec![])}
    }
    /// O(K² + K*R + N*(R+K)) cold origin, O(8) warm; O(N+K) scratch.
    /// At most eight observer fields are retained (O(8N)), independent of actors.
    /// Dijkstra visits endpoint nodes in original order; target is terminal only.
    pub(crate) fn distances(&self, origin: Point) -> Arc<Vec<f64>> {
        let mut cached=self.origins.lock().unwrap_or_else(|e|e.into_inner());
        if let Some((_,v))=cached.iter().find(|(p,_)|*p==origin){crate::timing::count(crate::timing::Count::ObserverHit,1);return v.clone()}
        crate::timing::count(crate::timing::Count::ObserverMiss,1);
        let _span=crate::timing::span("vision.observer_field_build");
        let n=self.endpoints.len();
        let mut costs:Vec<_>=self.endpoints.iter().map(|&p|if self.dependencies.clear(origin,p){crate::vision::distance(origin,p)}else{f64::INFINITY}).collect();
        let mut done=vec![false;n];
        for _ in 0..n {
            let Some(i)=(0..n).filter(|&i|!done[i]&&costs[i].is_finite()).min_by(|&a,&b|costs[a].total_cmp(&costs[b]))else{break};
            done[i]=true;
            for j in 0..n { if i==j||done[j]{continue}
                let edge=if i/2==j/2 { if self.dependencies.open(self.endpoints[i])&&self.dependencies.open(self.endpoints[j]){0.}else{f64::INFINITY} }
                    else {let p=self.endpoints[j];self.outgoing[i].get(p.y as usize*self.dependencies.width+p.x as usize).copied().unwrap_or(f64::INFINITY)};
                let v=costs[i]+edge; if v<costs[j]{costs[j]=v}
            }
        }
        let values=Arc::new((0..self.dependencies.opaque.len()).map(|at|{
            let p=self.dependencies.point(at);
            let mut best=if self.dependencies.clear(origin,p){crate::vision::distance(origin,p)}else{f64::INFINITY};
            for (i,&cost) in costs.iter().enumerate(){let candidate=cost+self.outgoing[i][at];if candidate<best{best=candidate}}
            best
        }).collect());
        if cached.len()==8{cached.remove(0);}
        cached.push((origin,Arc::clone(&values)));values
    }
}
#[derive(Debug,Default)]
pub struct OpticalCache(Mutex<Vec<Arc<OpticalField>>>);
impl Clone for OpticalCache { /// O(1), derived caches reset on clone/rewind.
    fn clone(&self)->Self{Self::default()} }
impl PartialEq for OpticalCache { /// O(1), no canonical gameplay state.
    fn eq(&self,_:&Self)->bool{true} }
impl OpticalCache {
    /// O(N+K) exact guard per immutable pass, plus cold build if changed. Two
    /// floor geometries maximum; absent dream plane may share the physical field.
    pub(crate) fn field(&self,f:&Floor,portals:bool)->Arc<OpticalField>{
        let _s=crate::timing::span("vision.optical_dependency_check");
        let dependencies=Dependencies::capture(f,portals);
        let mut slots=self.0.lock().unwrap_or_else(|e|e.into_inner());
        if let Some(v)=slots.iter().find(|v|v.dependencies==dependencies){crate::timing::count(crate::timing::Count::OpticalHit,1);return v.clone()}
        crate::timing::count(crate::timing::Count::OpticalMiss,1);
        let field=Arc::new(OpticalField::build(dependencies));
        if slots.len()==2{slots.remove(0);}
        slots.push(field.clone());field
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portals_generation::PortalPair;
    /// O(N) deterministic synthetic geometry; independent of gameplay RNG.
    fn floor(seed:usize,pairs:usize)->Floor {
        let mut f=Floor{depth:10.,tiles:(0..81).map(|i|if (i*37+seed*19)%13<3{"wall".into()}else{"floor".into()}).collect(),..Default::default()};
        f.portals=(0..pairs).map(|i|PortalPair{a:i*3+1,b:79-i*4,color_index:i as u8}).collect::<Vec<_>>().into();
        f
    }
    /// Exhaustive origin/target comparison to original route graph. O(cases*N²*K²R).
    #[test]
    fn cached_distances_match_original_directed_graph_bit_for_bit(){
        for seed in 0..4 {for pairs in [0,1,3,7]{
            let f=floor(seed,pairs);let cache=OpticalCache::default();let field=cache.field(&f,true);
            for origin in 0..81 {let a=crate::portals::point(&f,origin);let got=field.distances(a);
                for target in 0..81 {let b=crate::portals::point(&f,target);
                    let expected=crate::portals::route_filtered(&f,a,b,false,|x,y|crate::vision::floor_ray(&f,x,y),|x,y|[x,y].into_iter().all(|p|field.dependencies.open(p))).map_or(f64::INFINITY,|r|r.distance);
                    assert_eq!(got[target].to_bits(),expected.to_bits(),"seed={seed} pairs={pairs} from={origin} to={target}");
                }
            }
        }}
    }
    /// O(N+K) each guard, dirty rebuild O(KNR), verifies exact direct edits.
    #[test]
    fn direct_opacity_portal_edits_and_clone_invalidate(){
        let mut f=floor(0,1);let cache=OpticalCache::default();let a=cache.field(&f,true);
        assert!(Arc::ptr_eq(&a,&cache.field(&f,true)));
        f.tiles[40]="tree".into();let b=cache.field(&f,true);assert!(!Arc::ptr_eq(&a,&b));
        f.doors=[("41".into(),crate::FloorDoorsEntry{open:false,..Default::default()})].into_iter().collect::<indexmap::IndexMap<_,_>>().into();
        let c=cache.field(&f,true);assert!(!Arc::ptr_eq(&b,&c));
        f.doors.as_mut().unwrap().get_mut("41").unwrap().open=true;
        let d=cache.field(&f,true);assert!(!Arc::ptr_eq(&c,&d));
        f.portals.as_mut().unwrap()[0].b=70;
        let e=cache.field(&f,true);assert!(!Arc::ptr_eq(&d,&e));
        assert!(!Arc::ptr_eq(&e,&cache.clone().field(&f,true)));
        assert!(!Arc::ptr_eq(&e,&cache.field(&f,false)));
    }
}

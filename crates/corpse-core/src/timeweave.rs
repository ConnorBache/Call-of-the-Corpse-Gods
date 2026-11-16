//! Timeweave data only: no ability, movement, clock, RNG or revision policy.
//! L=live segments, H=peak allocated capacity, k=removed prefix. Retained space O(H).
//! Hash operations are expected O(1),
//! worst-case O(L). Clone/COW detachment is O(H+L); serialization/deserialization is expected O(L).
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, VecDeque};

pub const DESCRIPTION: &str = "You can freely travel along your time weave, which you leave behind as you move.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Plane { Waking, Dream }

/// Floor identity is supplied by the simulation, not inferred from the current view.
/// This primitive treats identifiers as opaque; the Run admission layer owns geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell { pub floor: u32, pub plane: Plane, pub tile: u32 }

/// `cell` is the unique occupied weave cell; from/to describe its directed edge.
/// The caller chooses which endpoint owns the arrow. No placement rule is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment { pub cell: Cell, pub from: Cell, pub to: Cell }

/// Private storage prevents stale derived indexes and duplicate occupied cells.
/// The deque is the canonical oldest-to-newest order; the map is never serialized.
#[derive(Debug, Clone, Default)]
pub struct Trail { segments: VecDeque<Segment>, by_cell: HashMap<Cell, Segment> }
impl Trail {
    /// O(1), no traversal or allocation.
    pub fn len(&self) -> usize { self.segments.len() }
    /// O(1), no traversal or allocation.
    pub fn is_empty(&self) -> bool { self.segments.is_empty() }
    /// Expected O(1) lookup, worst O(L); no mutation or scan of chronological storage.
    pub fn get(&self, cell: Cell) -> Option<&Segment> { self.by_cell.get(&cell) }
    /// O(1) iterator construction; consuming all entries costs O(L).
    pub fn iter(&self) -> impl DoubleEndedIterator<Item=&Segment> + ExactSizeIterator {
        self.segments.iter()
    }
    /// Consume exactly one segment, preserving both older and newer segments.
    /// Expected O(1) for either endpoint; O(L) search/shift for a middle entry.
    /// O(1) scratch, retained capacity unchanged. Shared Arc detachment is caller-owned.
    pub fn remove(&mut self, cell: Cell) -> bool {
        if self.by_cell.remove(&cell).is_none() { return false; }
        if self.segments.back().is_some_and(|s| s.cell == cell) {
            self.segments.pop_back();
        } else if self.segments.front().is_some_and(|s| s.cell == cell) {
            self.segments.pop_front();
        } else if let Some(index) = self.segments.iter().position(|s| s.cell == cell) {
            self.segments.remove(index);
        }
        true
    }
    /// Expected O(1+k) / worst O((k+1)L), O(1) scratch. Remove collided segment
    /// and every older segment. Absent cells are no-ops. Retained suffix is not shifted.
    pub fn remove_through(&mut self, cell: Cell) -> usize {
        if !self.by_cell.contains_key(&cell) { return 0; }
        let mut removed=0;
        while let Some(segment)=self.segments.pop_front() {
            self.by_cell.remove(&segment.cell);
            removed+=1;
            if segment.cell==cell { break; }
        }
        removed
    }
    /// Expected amortized O(1+k), including possible deque/hash capacity growth.
    /// Worst O(L+(k+1)L), retained capacity O(H). Collision removes old prefix FIRST,
    /// then this supplied segment becomes newest. No movement policy or RNG here.
    pub fn append(&mut self, segment: Segment) -> usize {
        let removed=self.remove_through(segment.cell);
        self.segments.push_back(segment);
        self.by_cell.insert(segment.cell,segment);
        removed
    }
    /// O(L) validation/index construction, expected hash cost; rejects rather than
    /// silently truncating malformed persisted duplicate cells. O(L) extra index.
    fn from_segments(segments: VecDeque<Segment>) -> Result<Self, &'static str> {
        let mut by_cell=HashMap::with_capacity(segments.len());
        for segment in &segments {
            if by_cell.insert(segment.cell,*segment).is_some() {
                return Err("duplicate Timeweave cell");
            }
        }
        Ok(Self {segments,by_cell})
    }
}
impl PartialEq for Trail {
    /// O(L), canonical chronological equality independent of randomized hash order.
    fn eq(&self,other:&Self)->bool {self.segments==other.segments}
}
impl Eq for Trail {}
/// Borrowed tuple projection; no intermediate serialized segment vector.
struct CompactSegments<'a>(&'a VecDeque<Segment>);
impl Serialize for CompactSegments<'_> {
    /// O(L) tuple emission, O(1) wrapper scratch; rejects information-losing shapes.
    fn serialize<S:Serializer>(&self, serializer:S)->Result<S::Ok,S::Error>{
        use serde::ser::SerializeSeq;
        for segment in self.0 {
            if segment.cell!=segment.from || segment.from.floor!=segment.to.floor || segment.from.plane!=segment.to.plane {
                return Err(serde::ser::Error::custom("Timeweave segment cannot be encoded as one source-plane edge"));
            }
        }
        let mut seq=serializer.serialize_seq(Some(self.0.len()))?;
        for s in self.0 {seq.serialize_element(&(s.from.floor,s.from.plane,s.from.tile,s.to.tile))?;}
        seq.end()
    }
}
impl Serialize for Trail {
    /// O(L) output, O(1) wrapper scratch; derived lookup table is omitted.
    fn serialize<S:Serializer>(&self, serializer:S)->Result<S::Ok,S::Error> {
        #[derive(Serialize)]
        struct Wire<'a>{segments:CompactSegments<'a>}
        Wire{segments:CompactSegments(&self.segments)}.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Trail {
    /// O(L) strict tuple decode plus expected O(L) index rebuild, O(L) retained
    /// space and transient tuples. Outer admission still validates Run geometry.
    fn deserialize<D:Deserializer<'de>>(deserializer:D)->Result<Self,D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire{segments:VecDeque<(u32,Plane,u32,u32)>}
        let wire=Wire::deserialize(deserializer)?;
        let segments=wire.segments.into_iter().map(|(floor,plane,from,to)|{
            let cell=Cell{floor,plane,tile:from};
            Segment{cell,from:cell,to:Cell{floor,plane,tile:to}}
        }).collect();
        Self::from_segments(segments).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// O(1) opaque fixture identifier construction.
    fn cell(tile:u32)->Cell {Cell{floor:1,plane:Plane::Waking,tile}}
    /// O(1); fixture endpoint ownership is not a gameplay placement decision.
    fn segment(tile:u32)->Segment {Segment{cell:cell(tile),from:cell(tile),to:cell(tile+1)}}
    /// O(L) fixture collection; tests inspect canonical order explicitly.
    fn tiles(t:&Trail)->Vec<u32>{t.iter().map(|s|s.cell.tile).collect()}
    /// O(L) append/delete/roundtrip with fixed bounded fixture.
    #[test]
    fn collision_removes_collided_and_older_only_then_appends_newest(){
        let mut t=Trail::default();for n in 0..5 {assert_eq!(t.append(segment(n)),0);}
        let replacement=Segment{to:cell(99),..segment(2)};
        assert_eq!(t.append(replacement),3);assert_eq!(tiles(&t),vec![3,4,2]);
        assert_eq!(t.get(cell(2)),Some(&replacement));assert!(t.get(cell(0)).is_none());
        assert_eq!(t.remove_through(cell(3)),1);assert_eq!(tiles(&t),vec![4,2]);
        assert_eq!(t.remove_through(cell(88)),0);assert_eq!(t.remove_through(cell(2)),2);
        assert!(t.is_empty());assert_eq!(t.len(),0);
    }
    /// O(L) exact serialized chronological order and rebuilt derived lookup.
    #[test]
    fn roundtrip_order_and_duplicate_rejection(){
        let mut t=Trail::default();for n in [9,1,7] {t.append(segment(n));}
        let raw=serde_json::to_string(&t).unwrap();assert!(!raw.contains("by_cell"));
        let restored:Trail=serde_json::from_str(&raw).unwrap();assert_eq!(restored,t);
        assert_eq!(serde_json::to_string(&restored).unwrap(),raw);
        assert_eq!(restored.get(cell(7)),Some(&segment(7)));
        let s=serde_json::json!([1,"waking",9,10]);
        assert!(serde_json::from_value::<Trail>(serde_json::json!({"segments":[s,s]})).is_err());
        assert!(serde_json::from_value::<Trail>(serde_json::json!({"segments":[],"by_cell":{}})).is_err());
    }
    /// O(1) bounded case; floor/plane identity never aliases the same tile number.
    #[test]
    fn identities_and_arrow_ownership_are_explicit(){
        let mut t=Trail::default();let a=segment(3);
        let dream=Cell{plane:Plane::Dream,..a.cell};let other=Cell{floor:2,..a.cell};
        for cell in [a.cell,dream,other] {assert_eq!(t.append(Segment{cell,from:a.to,to:a.from}),0);}
        assert_eq!(t.len(),3);assert_eq!(t.get(dream).unwrap().from,a.to);
        assert_eq!(t.remove_through(dream),2);assert_eq!(t.get(other).unwrap().cell,other);
    }
    /// O(H+L) first Arc COW mutation; old snapshot survives exactly, not claimed O(1).
    #[test]
    fn shared_snapshot_detaches_without_mutating_old_order(){
        let mut live=std::sync::Arc::new(Trail::default());
        for n in 0..4 {std::sync::Arc::make_mut(&mut live).append(segment(n));}
        let old=live.clone();assert!(std::sync::Arc::ptr_eq(&live,&old));
        std::sync::Arc::make_mut(&mut live).append(segment(1));
        assert!(!std::sync::Arc::ptr_eq(&live,&old));assert_eq!(tiles(&old),vec![0,1,2,3]);
        assert_eq!(tiles(&live),vec![2,3,1]);
    }
    /// O(L) total expected work over4096 unique appends then one prefix deletion.
    #[test]
    fn large_prefix_deletion_preserves_suffix_lookup_and_order(){
        let mut t=Trail::default();for n in 0..4096 {t.append(segment(n));}
        assert_eq!(t.remove_through(cell(3999)),4000);assert_eq!(t.len(),96);
        assert_eq!(t.iter().next(),Some(&segment(4000)));
        assert_eq!(t.iter().next_back(),Some(&segment(4095)));
        for n in 4000..4096 {assert_eq!(t.get(cell(n)),Some(&segment(n)));}
        for n in 0..4000 {assert!(t.get(cell(n)).is_none());}
    }
}

#[cfg(test)]
mod run_serde_tests {
    use super::*;
    /// O(S+L) full Run serde verification; no engine dispatch or revision admission claim.
    #[test]
    fn full_run_roundtrip_and_legacy_missing_omission(){
        let raw=include_str!("../../../fixtures/avatar-room-v1.json");
        let mut run:crate::Run=serde_json::from_str(raw).unwrap();
        assert!(run.timeweave.is_missing());
        let legacy=serde_json::to_vec(&run).unwrap();
        assert!(serde_json::to_value(&run).unwrap().get("timeweave").is_none());
        let legacy_again:crate::Run=serde_json::from_slice(&legacy).unwrap();
        assert_eq!(serde_json::to_vec(&legacy_again).unwrap(),legacy);
        let a=Cell{floor:1,plane:Plane::Waking,tile:10};
        let b=Cell{tile:11,..a};let mut trail=Trail::default();
        trail.append(Segment{cell:a,from:a,to:b});
        run.timeweave=crate::Field::Value(std::sync::Arc::new(trail));
        let encoded=serde_json::to_vec(&run).unwrap();
        let restored:crate::Run=serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored.timeweave,run.timeweave);
        assert_eq!(serde_json::to_vec(&restored).unwrap(),encoded);
        let snapshot=crate::temporal::Snapshot::capture(&run);
        assert!(std::sync::Arc::ptr_eq(run.timeweave.as_ref().unwrap(),snapshot.state.timeweave.as_ref().unwrap()));
        run.timeweave=crate::Field::Null;
        let restored:crate::Run=serde_json::from_slice(&serde_json::to_vec(&run).unwrap()).unwrap();
        assert!(matches!(restored.timeweave,crate::Field::Null));
        run.timeweave=crate::Field::Missing;
        assert_eq!(serde_json::to_vec(&run).unwrap(),legacy);
    }
}

/// Boundary-only expected O(F+L) time / O(F) scratch, F physical+dream floors.
/// Never call per movement. Validates current ordinary-edge wire geometry, not
/// terrain passability or continuity, which can change after a segment is laid.
pub fn validate_run(run:&crate::Run)->Result<(),String>{
    if run.timeweave.is_missing(){return Ok(());}
    if run.simulation_revision.value_or(39.)<41. {return Err("Timeweave requires simulation revision41".into());}
    let Some(trail)=run.timeweave.as_ref() else {return Ok(())};
    let mut dimensions=HashMap::new();
    for (plane,floors) in [(Plane::Waking,run.floors.as_slice()),(Plane::Dream,run.dreamwalking.as_ref().map_or(&[][..],|d|d.floors.as_slice()))] {
        for floor in floors {
            let depth=floor.depth;let width=floor.width();
            if !depth.is_finite()||depth.fract()!=0.||depth<1.||depth>u32::MAX as f64||width==0 {
                return Err("Invalid Timeweave floor identity/geometry".into());
            }
            if dimensions.insert((depth as u32,plane),(width,floor.tiles.len())).is_some(){return Err("Duplicate Timeweave floor identity".into());}
        }
    }
    let check=|cell:Cell|->Result<usize,String>{
        if cell.floor==0 {return Err("Timeweave floor identity is one-based".into());}
        let &(width,length)=dimensions.get(&(cell.floor,cell.plane)).ok_or("Timeweave references missing floor/plane")?;
        if cell.tile as usize>=length {return Err("Timeweave tile outside floor".into());}
        Ok(width)
    };
    for segment in trail.iter(){
        check(segment.cell)?;let width=check(segment.from)?;check(segment.to)?;
        if segment.cell!=segment.from {return Err("Timeweave arrow must occupy its source cell".into());}
        if segment.from.floor!=segment.to.floor||segment.from.plane!=segment.to.plane {return Err("Cross-floor/plane Timeweave edges are not admitted".into());}
        let (a,b)=(segment.from.tile as usize,segment.to.tile as usize);
        let dx=(a%width).abs_diff(b%width);let dy=(a/width).abs_diff(b/width);
        if dx>1||dy>1||(dx==0&&dy==0){return Err("Timeweave edge must be a nonzero adjacent step".into());}
    }
    Ok(())
}
#[cfg(test)]mod geometry_tests{
 use super::*;
 /// O(fixture bytes), no movement or terrain edits.
 fn run()->crate::Run{let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=41.0.into();r}
 /// O(L) fixture mutation; validation is independently invoked by the test.
 fn assign(r:&mut crate::Run,s:Segment){let mut trail=Trail::default();trail.append(s);r.timeweave=crate::Field::Value(std::sync::Arc::new(trail));}
 /// O(F) each bounded case, rejects row wrap and every endpoint addressing error.
 #[test]fn geometry_checks_source_destination_plane_and_adjacent_cells(){
  let mut r=run();let a=Cell{floor:1,plane:Plane::Waking,tile:10};let b=Cell{tile:11,..a};let valid=Segment{cell:a,from:a,to:b};
  assign(&mut r,valid);assert!(validate_run(&r).is_ok());
  let bad=[Segment{cell:b,..valid},Segment{from:Cell{floor:0,..a},..valid},Segment{to:Cell{floor:99,..b},..valid},Segment{to:Cell{tile:u32::MAX,..b},..valid},Segment{to:Cell{plane:Plane::Dream,..b},..valid},Segment{to:a,..valid},Segment{cell:Cell{tile:78,..a},from:Cell{tile:78,..a},to:Cell{tile:79,..a}}];
  for s in bad {assign(&mut r,s);assert!(validate_run(&r).is_err(),"{s:?}");}
  let mut dream=crate::dreamwalking::State::default();dream.floors.push(r.floors[0].clone());r.dreamwalking=dream.into();
  let a=Cell{plane:Plane::Dream,..a};let b=Cell{plane:Plane::Dream,..b};assign(&mut r,Segment{cell:a,from:a,to:b});assert!(validate_run(&r).is_ok());
 }
 /// O(1) absence/revision gate, legacy Missing remains byte-compatible.
 #[test]fn legacy_missing_allowed_but_present_or_null_requires_revision41(){
  let mut r=run();r.simulation_revision=40.0.into();assert!(validate_run(&r).is_ok());
  r.timeweave=crate::Field::Null;assert!(validate_run(&r).is_err());
  r.simulation_revision=41.0.into();assert!(validate_run(&r).is_ok());
 }
}

#[cfg(test)]mod compact_wire_tests {
 use super::*;
 /// O(fixed cases), tuple shape is closed and native integer bounds are exact.
 #[test]fn strict_tuple_shape_types_and_bounds(){
  let valid=r#"{"segments":[[1,"waking",10,11],[1,"dream",10,11]]}"#;
  let t:Trail=serde_json::from_str(valid).unwrap();assert_eq!(serde_json::to_string(&t).unwrap(),valid);
  for raw in [r#"{"segments":[[1,"waking",10]]}"#,r#"{"segments":[[1,"waking",10,11,12]]}"#,r#"{"segments":[[1,"waking","10",11]]}"#,r#"{"segments":[[1,"waking",4294967296,11]]}"#,r#"{"segments":[[-1,"waking",10,11]]}"#,r#"{"segments":[[1,"unknown",10,11]]}"#,r#"{"segments":[{"cell":1,"from":10,"to":11}]}"#] {
   assert!(serde_json::from_str::<Trail>(raw).is_err(),"{raw}");
  }
 }
 /// O(L) serialization gate; never silently collapses a mismatched public Segment.
 #[test]fn tuple_serializer_rejects_information_losing_segment_shapes(){
  let a=Cell{floor:1,plane:Plane::Waking,tile:10};let b=Cell{tile:11,..a};
  for s in [Segment{cell:b,from:a,to:b},Segment{cell:a,from:a,to:Cell{floor:2,..b}},Segment{cell:a,from:a,to:Cell{plane:Plane::Dream,..b}}] {
   let mut t=Trail::default();t.append(s);assert!(serde_json::to_string(&t).is_err());
  }
 }
}

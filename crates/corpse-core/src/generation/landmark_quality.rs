use crate::Floor;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaselineComponent {
    pub family: String,
    pub cells: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorBaseline {
    pub id: usize,
    pub kind: String,
    pub initial_cells: usize,
    pub components: Vec<BaselineComponent>,
}
/// Transient pre-environment baseline; never stored in Floor or save state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LandmarkQualityBaseline {
    pub width: usize,
    pub height: usize,
    pub majors: Vec<MajorBaseline>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorLandmarkQuality {
    pub id: usize,
    pub kind: String,
    pub substantial: bool,
    pub initial_cells: usize,
    pub retained_cells: usize,
    pub matching_fraction: f64,
    pub initial_components: usize,
    pub largest_initial_component_cells: usize,
    pub largest_initial_component_family: String,
    pub retained_cells_in_largest_initial_component: usize,
    pub largest_connected_survivor_cells: usize,
    pub largest_component_fraction: f64,
    pub connected_component_enforced: bool,
    pub matching_deficit: f64,
    pub connected_deficit: f64,
    pub acceptable: bool,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandmarkQualityResult {
    pub acceptable: bool,
    pub substantial_majors: usize,
    pub failed_majors: usize,
    pub minimum_matching_fraction: f64,
    pub minimum_largest_component_fraction: f64,
    pub majors: Vec<MajorLandmarkQuality>,
}
fn family(tile: &str) -> &str {
    if tile == "water" || tile == "shallow-water" {
        "water"
    } else if ["floor", "grass", "up", "down", "pit"].contains(&tile) {
        "land"
    } else {
        tile
    }
}
fn components(cells: &[usize], w: usize, h: usize) -> Vec<Vec<usize>> {
    let mut remaining: IndexSet<usize> = cells.iter().copied().collect();
    let mut result = vec![];
    for &first in cells {
        if !remaining.swap_remove(&first) {
            continue;
        }
        let mut q = vec![first];
        let mut head = 0;
        while head < q.len() {
            let at = q[head];
            head += 1;
            let x = at % w;
            let y = at / w;
            for next in [
                if x > 0 { Some(at - 1) } else { None },
                if x + 1 < w { Some(at + 1) } else { None },
                if y > 0 { Some(at - w) } else { None },
                if y + 1 < h { Some(at + w) } else { None },
            ]
            .into_iter()
            .flatten()
            {
                if remaining.swap_remove(&next) {
                    q.push(next)
                }
            }
        }
        result.push(q)
    }
    result
}
/// O(LV) grouping with L<=24; component ordering and largest-region tie retain source order.
pub fn capture_landmark_quality_baseline(f: &Floor) -> LandmarkQualityBaseline {
    let w = f.width();
    let h = f.height();
    let landmarks = f
        .generation
        .as_ref()
        .and_then(|g| g.landmarks.as_ref())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut majors = vec![];
    for major in landmarks.iter().filter(|l| l.scale == "major") {
        let mut ids = IndexSet::from([major.id as usize]);
        for landmark in landmarks {
            if landmark
                .parent_id
                .as_ref()
                .is_some_and(|&id| ids.contains(&(id as usize)))
            {
                ids.insert(landmark.id as usize);
            }
        }
        let cells: IndexSet<usize> = landmarks
            .iter()
            .filter(|l| ids.contains(&(l.id as usize)))
            .flat_map(|l| l.cells.iter().map(|&at| at as usize))
            .collect();
        let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();
        for &at in &cells {
            groups
                .entry(family(&f.tiles[at]).into())
                .or_default()
                .push(at)
        }
        let mut regions: Vec<BaselineComponent> = groups
            .into_iter()
            .flat_map(|(kind, cells)| {
                components(&cells, w, h)
                    .into_iter()
                    .map(move |cells| BaselineComponent {
                        family: kind.clone(),
                        cells,
                    })
            })
            .collect();
        let mut largest = 0;
        for i in 1..regions.len() {
            if regions[i].cells.len() > regions[largest].cells.len()
                || (regions[i].cells.len() == regions[largest].cells.len()
                    && regions[i].cells[0] < regions[largest].cells[0])
            {
                largest = i
            }
        }
        if largest != 0 {
            regions.swap(0, largest)
        }
        majors.push(MajorBaseline {
            id: major.id as usize,
            kind: major.kind.clone(),
            initial_cells: cells.len(),
            components: regions,
        })
    }
    LandmarkQualityBaseline {
        width: w,
        height: h,
        majors,
    }
}
/// Measures exact corresponding material regions; O(V) graph work, O(V) scratch.
pub fn measure_landmark_quality(
    f: &Floor,
    baseline: &LandmarkQualityBaseline,
) -> Result<LandmarkQualityResult, String> {
    if f.width() != baseline.width || f.height() != baseline.height {
        return Err("Landmark quality baseline geometry changed".into());
    }
    let mut majors = vec![];
    for major in &baseline.majors {
        let (mut retained, mut retained_largest, mut connected) = (0, 0, 0);
        for (index, original) in major.components.iter().enumerate() {
            let kept: Vec<usize> = original
                .cells
                .iter()
                .copied()
                .filter(|&at| family(&f.tiles[at]) == original.family)
                .collect();
            retained += kept.len();
            if index == 0 {
                retained_largest = kept.len();
                for region in components(&kept, baseline.width, baseline.height) {
                    connected = connected.max(region.len())
                }
            }
        }
        let largest = major.components.first();
        let initial = largest.map_or(0, |l| l.cells.len());
        let matching = if major.initial_cells > 0 {
            retained as f64 / major.initial_cells as f64
        } else {
            1.0
        };
        let fraction = if initial > 0 {
            connected as f64 / initial as f64
        } else {
            1.0
        };
        let substantial = major.initial_cells >= 32;
        let enforced = substantial && initial >= 8;
        let matching_deficit = if substantial {
            (0.6 - matching).max(0.0)
        } else {
            0.0
        };
        let connected_deficit = if enforced {
            (0.5 - fraction).max(0.0)
        } else {
            0.0
        };
        majors.push(MajorLandmarkQuality {
            id: major.id,
            kind: major.kind.clone(),
            substantial,
            initial_cells: major.initial_cells,
            retained_cells: retained,
            matching_fraction: matching,
            initial_components: major.components.len(),
            largest_initial_component_cells: initial,
            largest_initial_component_family: largest.map_or("land", |l| l.family.as_str()).into(),
            retained_cells_in_largest_initial_component: retained_largest,
            largest_connected_survivor_cells: connected,
            largest_component_fraction: fraction,
            connected_component_enforced: enforced,
            matching_deficit,
            connected_deficit,
            acceptable: matching_deficit == 0.0 && connected_deficit == 0.0,
        });
    }
    Ok(LandmarkQualityResult {
        acceptable: majors.iter().all(|m| m.acceptable),
        substantial_majors: majors.iter().filter(|m| m.substantial).count(),
        failed_majors: majors.iter().filter(|m| !m.acceptable).count(),
        minimum_matching_fraction: majors
            .iter()
            .filter(|m| m.substantial)
            .map(|m| m.matching_fraction)
            .reduce(f64::min)
            .unwrap_or(1.0),
        minimum_largest_component_fraction: majors
            .iter()
            .filter(|m| m.substantial && m.connected_component_enforced)
            .map(|m| m.largest_component_fraction)
            .reduce(f64::min)
            .unwrap_or(1.0),
        majors,
    })
}

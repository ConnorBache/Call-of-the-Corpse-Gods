use super::Point;
use crate::rng::{GenerationRng, mix};
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FloorStairs {
    pub up: Point,
    pub down: Point,
}
fn height(size: i32) -> i32 {
    if size == 79 { 29 } else { size }
}
fn next_point(r: &mut GenerationRng, size: i32, previous: Option<Point>) -> Point {
    let candidates: Vec<Point> = (0..16)
        .map(|_| Point {
            x: 5 + (r.random() * (size - 10) as f64).floor() as i32,
            y: 5 + (r.random() * (height(size) - 10) as f64).floor() as i32,
        })
        .collect();
    let Some(prev) = previous else {
        return candidates[0];
    };
    let distance = |p: Point| ((p.x - prev.x) as f64).hypot((p.y - prev.y) as f64);
    if let Some(p) = candidates
        .iter()
        .find(|&&p| distance(p) >= size as f64 * 0.4)
    {
        return *p;
    }
    let mut all = candidates;
    all.extend([
        Point { x: 5, y: 5 },
        Point {
            x: 5,
            y: height(size) - 6,
        },
        Point { x: size - 6, y: 5 },
        Point {
            x: size - 6,
            y: height(size) - 6,
        },
    ]);
    all.sort_by(|a, b| distance(*b).total_cmp(&distance(*a)));
    all[0]
}
/// Independent vertical spine, 16 candidates per floor. O(depth) time, O(1) memory (depth<=40).
pub fn floor_stairs(
    depth: u32,
    seed: u32,
    size: i32,
    actual_up: Option<Point>,
) -> Result<FloorStairs, String> {
    if !(1..=40).contains(&depth) {
        return Err("Invalid stair-plan depth".into());
    }
    if ![79, 50, 60].contains(&size) {
        return Err("Unsupported stair-plan size".into());
    }
    let mut r = GenerationRng::new(seed ^ 0x71a12a5d);
    let mut up = next_point(&mut r, size, None);
    let mut down = up;
    for _ in 1..=depth {
        up = down;
        down = next_point(&mut r, size, Some(up))
    }
    if let Some(p) = actual_up {
        let landing = Point {
            x: p.x.clamp(5, size - 6),
            y: p.y.clamp(5, height(size) - 6),
        };
        if landing != up {
            up = landing;
            down = next_point(
                &mut GenerationRng::new(seed ^ depth.wrapping_mul(0x9e3779b1) ^ 0x47f232b1),
                size,
                Some(up),
            );
        }
    }
    Ok(FloorStairs { up, down })
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainEntry {
    #[serde(rename = "type")]
    pub domain: String,
    pub godbeast_depth: i32,
    pub godbeast_kind: String,
}
/// Eight five-floor domains and independent three-beast bag; exact reference draw ordering.
pub fn domain_plan(seed: u32) -> Vec<DomainEntry> {
    let mut r = GenerationRng::new(mix(seed, 9182));
    let mut beast_r = GenerationRng::new(mix(seed, 67191));
    let mut bag: Vec<&str> = Vec::new();
    let mut beasts: Vec<&str> = Vec::new();
    let mut previous = None;
    let mut result = Vec::new();
    for i in 0..8 {
        if bag.is_empty() {
            bag = vec![
                "Dungeon",
                "Cave",
                "Meadow",
                "Forest",
                "Archipelago",
                "Mountain",
            ]
        }
        let dungeon = bag.iter().position(|&s| s == "Dungeon");
        let chosen = if dungeon.is_some() && (bag.len() == 1 || r.random() < 0.5) {
            dungeon.unwrap()
        } else {
            let options: Vec<usize> = (0..bag.len()).filter(|&j| Some(j) != dungeon).collect();
            options[r.integer(0, options.len() as i32 - 1) as usize]
        };
        let domain = bag.remove(chosen).to_owned();
        if beasts.is_empty() {
            beasts = vec!["Mercuria", "Zillennia", "Veridian"]
        }
        let candidates: Vec<&str> = beasts
            .iter()
            .copied()
            .filter(|&k| Some(k) != previous)
            .collect();
        let beast = candidates[beast_r.integer(0, candidates.len() as i32 - 1) as usize];
        beasts.remove(beasts.iter().position(|&k| k == beast).unwrap());
        previous = Some(beast);
        result.push(DomainEntry {
            domain,
            godbeast_depth: i * 5 + r.integer(if i == 0 { 4 } else { 1 }, 5),
            godbeast_kind: beast.to_owned(),
        });
    }
    result
}

/// R33: seven non-final encounters from four authored designs. R34 adds Phantasmii, Epidite and Ahvamitt
/// as the fifth through seventh bag entries without changing the seven existing milestone depths.
/// Domain draws retain the legacy stream; the revised beast bag is independent.
/// O(8) time and memory. Legacy revisions are byte-for-byte the original plan.
pub fn domain_plan_revision(seed: u32, revision: f64) -> Vec<DomainEntry> {
    let mut result = domain_plan(seed);
    if revision < 33.0 { return result; }
    let mut rng = GenerationRng::new(mix(seed, 67191));
    let mut bag = Vec::new();
    let mut previous = "";
    for (i, entry) in result.iter_mut().enumerate() {
        if i == 7 {
            entry.godbeast_depth = -1;
            entry.godbeast_kind.clear();
            continue;
        }
        if bag.is_empty() {
            bag = vec!["Mercuria", "Zillennia", "Veridian", "Borge"];
            if revision >= 34.0 { bag.extend(["Phantasmii", "Epidite", "Ahvamitt"]); }
        }
        let candidates: Vec<_> = bag.iter().copied().filter(|k| *k != previous).collect();
        let kind = candidates[rng.integer(0, candidates.len() as i32 - 1) as usize];
        bag.remove(bag.iter().position(|k| *k == kind).unwrap());
        previous = kind;
        entry.godbeast_depth = (i as i32 + 1) * 5;
        entry.godbeast_kind = kind.into();
    }
    result
}

//! Explicit actual-world selected-creature tether cleanup. Never forwards hidden logs.
use crate::{Actor, Run};
use std::{
    collections::{HashMap, HashSet},
};
struct Link {
    id: &'static str,
    target: &'static str,
    keys: &'static [&'static str],
    status: Option<&'static str>,
}
const LINKS: [Link; 5] = [
    Link {
        id: "morae-bonded-by-unseen-threads",
        target: "bondedTarget",
        keys: &["bondedTarget"],
        status: None,
    },
    Link {
        id: "pelian-i-came-for-you",
        target: "cameTarget",
        keys: &["cameTarget", "cameTurns", "cameFree"],
        status: None,
    },
    Link {
        id: "isheilah-make-slippery",
        target: "slipperyTarget",
        keys: &["slipperyTarget"],
        status: None,
    },
    Link {
        id: "etill-riptide",
        target: "riptideTarget",
        keys: &["riptideTarget", "riptideX", "riptideY"],
        status: Some("riptide"),
    },
    Link {
        id: "dawel-unspeak",
        target: "unspeakTarget",
        keys: &["unspeakTarget", "unspeakCost"],
        status: Some("silenced"),
    },
];
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Loc {
    Player,
    Actor(String, usize),
}
fn locations(r: &Run) -> Vec<Loc> {
    let mut a = vec![Loc::Player];
    for (d, actors) in &r.actors {
        for i in 0..actors.len() {
            a.push(Loc::Actor(d.clone(), i));
        }
    }
    a
}
fn get<'a>(r: &'a Run, l: &Loc) -> &'a Actor {
    match l {
        Loc::Player => &r.player,
        Loc::Actor(d, i) => &r.actors[d][*i],
    }
}
/// Expected O(1) indexed access plus first-write actor COW; identity preserved.
fn get_mut<'a>(r: &'a mut Run, l: &Loc) -> &'a mut Actor {
    match l {
        Loc::Player => &mut r.player,
        Loc::Actor(d, i) => &mut r.actors.trusted_slice_mut(d).unwrap()[*i],
    }
}
fn owner(r: &Run, l: &Loc) -> Loc {
    let a = get(r, l);
    if crate::half_here::is_clone(a) && a.conditions.get("halfHereOwner") == Some(&r.player.id) {
        Loc::Player
    } else {
        l.clone()
    }
}
fn owners(r: &Run, all: &[Loc]) -> Vec<Loc> {
    let mut seen = HashSet::new();
    all.iter()
        .map(|l| owner(r, l))
        .filter(|l| seen.insert(l.clone()))
        .collect()
}
fn clear(r: &mut Run, owner: &Loc, link: &Link, target: Option<&Loc>) -> bool {
    let a = get_mut(r, owner);
    let mut changed = a.active.iter().any(|id| id == link.id);
    a.active.retain(|id| id != link.id);
    for key in link.keys {
        changed = a.conditions.shift_remove(*key).is_some() || changed;
    }
    if let (Some(t), Some(status)) = (target, link.status) {
        changed = get_mut(r, t).conditions.shift_remove(status).is_some() || changed;
    }
    changed
}
/// Call with the actual world, not an absent-player offscreen proxy. IDs are unique
/// at current-run admission; legacy normalization below explicitly guards ambiguity.
pub fn clear_maintained_link(r: &mut Run, actor_id: f64, ability_id: &str) -> bool {
    let Some(link) = LINKS.iter().find(|l| l.id == ability_id) else {
        return false;
    };
    let all = locations(r);
    let Some(body) = all.iter().find(|l| get(r, l).id == actor_id) else {
        return false;
    };
    let own = owner(r, body);
    let target = get(r, &own)
        .conditions
        .get(link.target)
        .and_then(|id| all.iter().find(|l| get(r, l).id == *id))
        .cloned();
    clear(r, &own, link, target.as_ref())
}
/// Actual death only, after rescue and before corpse placement/removal. Callers
/// simulating a hidden floor explicitly forward this event to their actual world.
pub fn end_maintained_links_to_dead_target(r: &mut Run, target_id: f64) {
    let all = locations(r);
    let owners = owners(r, &all);
    let target = all.iter().find(|l| get(r, l).id == target_id).cloned();
    let mut player_changed = false;
    for own in owners {
        for link in &LINKS {
            if get(r, &own).conditions.get(link.target) != Some(&target_id) {
                continue;
            }
            let changed = clear(r, &own, link, target.as_ref());
            player_changed = (own == Loc::Player && changed) || player_changed;
        }
    }
    if player_changed {
        crate::half_here::sync(r);
    }
}
fn id_key(n: f64) -> u64 {
    if n == 0.0 { 0 } else { n.to_bits() }
}
/// O(N) expected time/storage over world actors; missing/living/ambiguous targets
/// are preserved, including zero HP awaiting rescue and living off-floor targets.
pub fn normalize_dead_maintained_links(r: &mut Run) -> bool {
    let all = locations(r);
    let owners = owners(r, &all);
    let mut by_id: HashMap<u64, Vec<Loc>> = HashMap::new();
    for l in &all {
        by_id
            .entry(id_key(get(r, l).id))
            .or_default()
            .push(l.clone());
    }
    let mut changed = false;
    let mut player_changed = false;
    for own in owners {
        for link in &LINKS {
            let Some(id) = get(r, &own).conditions.get(link.target) else {
                continue;
            };
            let Some(targets) = by_id.get(&id_key(*id)) else {
                continue;
            };
            if targets.len() != 1 || !get(r, &targets[0]).dead.value_or(false) {
                continue;
            }
            let cleared = clear(r, &own, link, Some(&targets[0]));
            changed = cleared || changed;
            player_changed = (own == Loc::Player && cleared) || player_changed;
        }
    }
    if player_changed {
        crate::half_here::sync(r);
    }
    changed
}

//! R52 ability targeting knowledge is separate from ordinary visual perception.
use crate::{Actor, Run, movement::Point};

/// Physical, portal-aware line of effect from the acting endpoint. No RNG or
/// observation mutation. O(F + K²R) time and O(K + R) scratch for K portal endpoints
/// and maximum ray length R; F is floor lookup. Creature occupancy is not a wall.
pub fn line_of_effect(r: &Run, x: f64, y: f64) -> bool {
    let Some(f) = crate::dreamwalking::floor_for(r, &r.player) else { return false; };
    let Some(at) = crate::portals::cell(f, x, y) else { return false; };
    let from = Point { x: r.player.x as i32, y: r.player.y as i32 };
    let to = crate::portals::point(f, at);
    let pass = |p| crate::projectiles::terrain_pass(r.simulation_revision.value_or(29.), f, p);
    // An opaque destination may itself be selected (for example Sparkle on a
    // wall); only intervening cells obstruct reaching its surface.
    let clear = |a, b| {
        if (a != from && !pass(a)) || (b != to && !pass(b)) { return false; }
        let cells = crate::portals::linear_cells(a, b);
        cells.iter().skip(1).take(cells.len().saturating_sub(2)).all(|p| pass(*p))
    };
    crate::portals::route_filtered(f, from, to, false, clear, |a, b| pass(a) && pass(b)).is_some()
}

/// Telepathy supplies creature knowledge, never permission to affect through a
/// wall. Legacy revisions retain their original targeting visibility exactly.
pub fn actor(r: &Run, target: &Actor) -> bool {
    crate::vision::visible_actor(r, target)
        && (r.simulation_revision.value_or(29.) < 52.
            || (crate::dreamwalking::same_plane(r, &r.player, target)
                && line_of_effect(r, target.x, target.y)))
}

pub fn tile(r: &Run, x: usize, y: usize) -> bool {
    crate::vision::visible(r, x, y)
        && (r.simulation_revision.value_or(29.) < 52.
            || line_of_effect(r, x as f64, y as f64))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn room(revision: f64) -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = revision.into();
        r.player.x = 10.; r.player.y = 10.;
        r.player.conditions.clear(); r.player.active.clear(); r.player.abilities.clear();
        r.player.conditions.insert("telepathic".into(), 100.);
        let mut a = r.player.clone(); a.id = 999.; a.x = 14.; a.conditions.clear();
        a.conditions.insert("invisible".into(), 100.); a.state = "idle".into();
        r.actors_at_mut(r.depth).clear(); r.actors_at_mut(r.depth).push(a);
        let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.doors = crate::Field::Missing;
        f.portals = crate::Field::Missing;
        r.inventory.clear();
        r
    }
    #[test]
    fn telepathy_grants_ability_sight_without_changing_direct_vision() {
        let r = room(52.); let a = &r.actors_at(r.depth)[0];
        assert!(!crate::vision::directly_visible_actor(&r, a));
        assert!(actor(&r, a)); assert!(tile(&r, 14, 10));
        let mut no_telepathy = r.clone(); no_telepathy.player.conditions.clear();
        assert!(!actor(&no_telepathy, &no_telepathy.actors_at(r.depth)[0]));
        let before = serde_json::to_string(&r).unwrap();
        assert!(actor(&r, a)); assert!(tile(&r, 14, 10));
        assert_eq!(serde_json::to_string(&r).unwrap(), before);
    }
    #[test]
    fn walls_and_trees_block_telepathic_abilities_but_not_detection() {
        for obstacle in ["wall", "tree"] {
            let mut r = room(52.); let w = r.floor().unwrap().width();
            r.floor_mut().unwrap().tiles[10*w+12] = obstacle.into();
            let a = &r.actors_at(r.depth)[0];
            assert!(crate::vision::visible_actor(&r, a));
            assert!(!actor(&r, a)); assert!(!tile(&r, 14, 10));
            r.simulation_revision = 51.0.into();
            assert!(actor(&r, &r.actors_at(r.depth)[0])); assert!(tile(&r, 14, 10));
        }
    }
    #[test]
    fn physical_portals_allow_effect_but_cross_plane_does_not() {
        let mut r = room(52.); let w = r.floor().unwrap().width();
        for y in 0..r.floor().unwrap().height() { r.floor_mut().unwrap().tiles[y*w+12] = "wall".into(); }
        assert!(!actor(&r, &r.actors_at(r.depth)[0]));
        r.floor_mut().unwrap().portals = vec![crate::portals_generation::PortalPair {
            a: 10*w+11, b: 10*w+13, color_index: 0,
        }].into();
        assert!(actor(&r, &r.actors_at(r.depth)[0]));
        r.actors_at_mut(r.depth)[0].conditions.insert("phantasmiiPlane".into(), 1.);
        assert!(!actor(&r, &r.actors_at(r.depth)[0]));
    }
    #[test]
    fn targeted_ability_rejects_obstructed_telepathy_before_payment() {
        let mut r = room(52.); let id = "numo-implant-thought";
        r.player.abilities.push(id.into());
        crate::revision50_ability_energy::initialize_reserve(&mut r.player, id, 100., 100.).unwrap();
        let mut victim = r.actors_at(r.depth)[0].clone(); victim.id = 998.; victim.x = 15.;
        r.actors_at_mut(r.depth).push(victim);
        let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+12] = "wall".into();
        let before = serde_json::to_string(&r).unwrap();
        crate::abilities::activate(&mut r, id, &999.0.into(), &15.0.into(), &10.0.into()).unwrap();
        assert_eq!(serde_json::to_string(&r).unwrap(), before);
        r.floor_mut().unwrap().tiles[10*w+12] = "floor".into();
        crate::abilities::activate(&mut r, id, &999.0.into(), &15.0.into(), &10.0.into()).unwrap();
        assert_ne!(serde_json::to_string(&r).unwrap(), before);
    }
}

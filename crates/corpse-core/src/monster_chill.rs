//! Authored Yeti adjacency affects the shared player action stream, never companions.
use crate::Run;
/// B<=2 body endpoints; ring admission adds O(B * (I+C)) for owner inventory I
/// and condition entries C before the existing actor/portal adjacency work.
/// Source order chooses the first adjacent living Yeti. Only this cold source
/// is blocked: generic Slowed can also come from unrelated Witchcraft.
pub fn apply_adjacency(r: &mut Run) {
    let plane = |a: &crate::Actor| crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r, a);
    let bodies: Vec<_> = crate::half_here::origins(r)
        .into_iter()
        .filter(|a| !a.dead.value_or(false) && a.hp > 0.)
        .filter(|a| !crate::revision50_rings::has_effect(r, a, crate::revision50_rings::ICE_IMMUNITY))
        .map(|a| (a.id, a.x, a.y, plane(a)))
        .collect();
    let yetis: Vec<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| {
            !a.dead.value_or(false)
                && a.hp > 0.
                && !crate::half_here::is_clone(a)
                && a.abilities.iter().any(|id| id == "enemy:beast:yeti")
        })
        .map(|a| (a.id, a.x, a.y, plane(a)))
        .collect();
    let mut owner_slowed = false;
    for (id, _x, _y, plane) in bodies {
        let Some(source) = yetis.iter().find(|other| {
            other.0 != id && plane == other.3
                && crate::engine::actor(r, id).zip(crate::engine::actor(r, other.0))
                    .is_some_and(|(a,b)| crate::portals::actor_distance(r, a, b) <= std::f64::consts::SQRT_2)
        }) else {
            continue;
        };
        if !crate::monster_statuses::apply_status(r, id, "slowed", 1., Some(source.0)) {
            continue;
        }
        if !owner_slowed {
            if id != r.player.id {
                // A valid hit on Half Here slows the shared action stream even if its
                // owner now occupies another plane. Only the native endpoint was targeted.
                let player = r.player.id;
                if crate::dreamwalking::with_actor(r, player, |r|
                    crate::monster_statuses::apply_status(r, player, "slowed", 1., None))
                {
                    r.player.conditions.insert("slowSourceId".into(), source.0);
                }
            }
            owner_slowed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dream_plane_chill_chooses_native_yeti_and_preserves_half_here_shared_slow() {
        let mut r: crate::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: 1., ..Default::default() }.into();
        r.player.conditions.clear(); r.player.active.clear();
        let (x, y) = (r.player.x, r.player.y);
        let yeti = r.actors_at_mut(1.).first_mut().unwrap();
        yeti.x = x + 1.; yeti.y = y; yeti.abilities = vec!["enemy:beast:yeti".into()];
        let normal_id = yeti.id;
        let mut dream_yeti = yeti.clone(); dream_yeti.id = 91.;
        dream_yeti.conditions.insert("phantasmiiPlane".into(), 1.);
        r.actors_at_mut(1.).push(dream_yeti);
        super::apply_adjacency(&mut r);
        assert_eq!(r.player.conditions.get("slowSourceId"), Some(&91.));
        r.actors_at_mut(1.).retain(|a| a.id != 91.); r.player.conditions.clear();
        super::apply_adjacency(&mut r);
        assert!(!r.player.conditions.contains_key("slowed"));
        r.player.abilities.push(crate::half_here::ID.into());
        r.player.active.push(crate::half_here::ID.into());
        r.player.conditions.insert("halfHereId".into(), 90.);
        let mut clone = r.player.clone(); clone.id = 90.; clone.conditions.clear();
        clone.conditions.insert("halfHereClone".into(), 1.);
        clone.conditions.insert("halfHereOwner".into(), r.player.id);
        clone.conditions.insert("phantasmiiPlane".into(), 0.);
        r.actors_at_mut(1.).push(clone);
        super::apply_adjacency(&mut r);
        assert_eq!(r.player.conditions.get("slowSourceId"), Some(&normal_id));
        assert_eq!(r.player.conditions.get("slowed"), Some(&1.));
        assert_eq!(crate::engine::actor(&r, 90.).unwrap().conditions.get("slowed"), Some(&1.));
    }
}

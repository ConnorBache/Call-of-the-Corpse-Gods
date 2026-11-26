//! Shard generation and Blessing discount command boundary.
use crate::{Field, Run};

pub fn uses_three_blessing_discount(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.phantasmii_discount_from, Field::Value(_))
}
pub(crate) fn validate_discount(r: &Run) -> Result<(), String> {
    if !r.phantasmii_discount_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.phantasmii_discount_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Phantasmii discount command boundary".into());
    }
    Ok(())
}
pub fn admit_discount(r: &mut Run) -> Result<(), String> {
    validate_discount(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.phantasmii_discount_from.is_missing() {
        r.phantasmii_discount_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_discount_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.phantasmii_discount_from = cursor.into(); }
    else if cursor == 0 { r.phantasmii_discount_from = Field::Missing; }
}

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.phantasmii_blessings_from, Field::Value(_))
}
pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.phantasmii_blessings_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.phantasmii_blessings_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Phantasmii blessing command boundary".into());
    }
    if !r.phantasmii_birth_shards.is_missing() && !(
        r.phantasmii_birth_shards.as_ref() == Some(&true) && r.phantasmii_blessings_from.as_ref() == Some(&0)
    ) { return Err("Invalid Phantasmii birth policy".into()); }
    Ok(())
}
/// Adopt after the historical command prefix, not while replaying a save journal.
pub fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.phantasmii_blessings_from.is_missing() {
        r.phantasmii_blessings_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) { r.phantasmii_blessings_from = cursor.into(); }
    else if cursor == 0 { r.phantasmii_blessings_from = Field::Missing; }
}

pub(crate) fn configure_birth(r: &mut Run, policy: &Field<usize>) {
    apply_replay_policy(r, policy, 0);
    r.phantasmii_birth_shards = if current(r) { true.into() } else { Field::Missing };
}

/// Upgrade an already-generated waking floor only when the active player
/// encounters it. The per-floor marker survives pickup, saves and revisits.
pub(crate) fn seed_active_legacy_floor(r: &mut Run) {
    use crate::{GroundItem, phantasmii_generation as shards};
    if !current(r) || r.status != "playing" || crate::dreamwalking::context_dream(r) { return; }
    let Some(f) = r.floor() else { return };
    if f.phantasmii_shard_seeded.value_or(false) { return; }
    let depth = r.depth;
    let ground = r.ground.get(&depth.to_string()).map_or(&[][..], |items| items.as_slice());
    if !ground.iter().any(|g| shards::is_floor_shard(&g.item)) {
        // A damaged old floor can lack a navigable dry spot. Keep the run
        // playable and leave the marker unset so it can be retried if restored.
        let Ok(at) = shards::floor_shard_cell(&r.seed, f, r.actors_at(depth), ground) else { return };
        let w = f.width();
        let item = shards::floor_shard(r.next_id);
        r.next_id += 1.;
        r.ground_at_mut(depth).push(GroundItem {
            x: (at % w) as f64, y: (at / w) as f64, item, ..Default::default()
        });
    }
    r.floor_mut().unwrap().phantasmii_shard_seeded = true.into();
}

#[cfg(test)]
mod tests {
    use crate::{Command, GroundItem, engine, inventory, phantasmii_generation as shards, progression, replay, startup};

    #[test]
    fn phantasmii_floor_generation_and_seed_replays_select_the_same_shards() {
        let new = startup::create_avatar_run("0-1-2:shard-blessings").unwrap();
        assert_eq!(new.floors.len(), 2);
        for floor in &new.floors {
            let prizes: Vec<_> = new.ground[&floor.depth.to_string()].iter()
                .filter(|g| shards::is_floor_shard(&g.item)).collect();
            assert_eq!(prizes.len(), 1);
            let at = prizes[0].y as usize * floor.width() + prizes[0].x as usize;
            assert!(crate::generation::reachable_without_braziers(floor, None).contains(&at));
            assert!(crate::dreamwalking::item_in_plane(&new, &prizes[0].item));
        }
        let record = replay::ReplayRecord::from_run(&new);
        let same = replay::ReplaySession::create(record.clone(), true).unwrap();
        assert_eq!(new.ground, same.engine.run.ground);
        let mut old = record;
        old.phantasmii_blessings_from = crate::Field::Missing;
        old.phantasmii_birth_shards = crate::Field::Missing;
        let old = replay::ReplaySession::create(old, true).unwrap();
        assert!(old.engine.run.ground.values().flat_map(|items| items.iter()).all(|i| !shards::is_floor_shard(&i.item)));
    }

    #[test]
    fn phantasmii_upgraded_save_seeds_each_older_floor_once() {
        let initial = startup::create_avatar_run("0-1-2:shard-live-upgrade").unwrap();
        let mut record = replay::ReplayRecord::from_run(&initial);
        record.phantasmii_blessings_from = crate::Field::Missing;
        record.phantasmii_birth_shards = crate::Field::Missing;
        let old = replay::ReplaySession::create(record, true).unwrap().engine.run;
        assert!(old.floors.iter().all(|f| f.phantasmii_shard_seeded.is_missing()));
        let mut live = crate::temporal::Engine::new(old).unwrap().run;
        engine::dispatch(&mut live, &Command::Pickup).unwrap();
        let first_depth = live.depth;
        let prize = live.ground[&first_depth.to_string()].iter()
            .find(|g| shards::is_floor_shard(&g.item)).unwrap().clone();
        assert!(live.floor().unwrap().phantasmii_shard_seeded.value_or(false));
        let mut upgraded_replay = replay::ReplaySession::create(replay::ReplayRecord::from_run(&live), true).unwrap();
        upgraded_replay.step().unwrap();
        assert_eq!(live.ground, upgraded_replay.engine.run.ground);
        assert_eq!(live.next_id, upgraded_replay.engine.run.next_id);
        let next_depth = crate::fast_mode::below(&live, first_depth);
        assert!(!live.floors.iter().find(|f| f.depth == next_depth).unwrap().phantasmii_shard_seeded.value_or(false));

        (live.player.x, live.player.y) = (prize.x, prize.y);
        engine::dispatch(&mut live, &Command::Pickup).unwrap();
        let raw = crate::storage::serialize(&live).unwrap();
        let mut live = crate::storage::deserialize(&raw).unwrap();
        super::seed_active_legacy_floor(&mut live);
        assert!(!live.ground[&first_depth.to_string()].iter().any(|g| shards::is_floor_shard(&g.item)));
        assert!(live.inventory.iter().any(shards::is_floor_shard));
        live.depth = next_depth;
        super::seed_active_legacy_floor(&mut live);
        assert_eq!(live.ground[&next_depth.to_string()].iter().filter(|g| shards::is_floor_shard(&g.item)).count(), 1);
    }

    #[test]
    fn phantasmii_pickup_chains_all_earned_levels_through_normal_choices() {
        let mut r = startup::create_avatar_run("0-1-2:shard-blessings-pickup").unwrap();
        r.level = 1.; r.exp = 0.; r.offers.clear(); r.gods.clear();
        let depth = r.depth;
        let id = r.next_id;
        r.next_id += 1.;
        let (x, y) = (r.player.x, r.player.y);
        r.ground_at_mut(depth).retain(|g| !shards::is_floor_shard(&g.item));
        r.ground_at_mut(depth).push(GroundItem { x, y, item: shards::floor_shard(id), ..Default::default() });
        engine::dispatch(&mut r, &Command::Pickup).unwrap();
        assert_eq!(r.level, 2.);
        while let Some(id) = r.offers.first().cloned() {
            engine::dispatch(&mut r, &Command::Choose { id }).unwrap();
        }
        // Compressed 1 and 2 are free; the next threshold is 4-3.
        assert_eq!((r.level, r.exp, progression::level_up_cost(&r)), (3., 0., 1.));
        assert!(!crate::dreamwalking::unlocked(&r));

        let mut second = shards::floor_shard(r.next_id); r.next_id += 1.;
        second.quantity = 2.0.into();
        r.ground_at_mut(depth).push(GroundItem { x, y, item: second, ..Default::default() });
        engine::dispatch(&mut r, &Command::Pickup).unwrap();
        assert_eq!(r.level, 4.);
        while let Some(id) = r.offers.first().cloned() {
            engine::dispatch(&mut r, &Command::Choose { id }).unwrap();
        }
        assert_eq!(r.level, 5.);
        let stack = r.inventory.iter().position(shards::is_floor_shard).unwrap();
        assert_eq!((inventory::quantity(&r.inventory[stack]), progression::level_up_cost(&r)), (3., 7.));
        let raw = crate::storage::serialize(&r).unwrap();
        let saved = crate::storage::deserialize(&raw).unwrap();
        assert_eq!(progression::level_up_cost(&saved), 7.);
        inventory::take_one(&mut r, stack);
        assert_eq!((r.level, progression::level_up_cost(&r)), (5., 10.));
    }

    #[test]
    fn phantasmii_original_discount_is_preserved_before_replay_boundary() {
        let mut r = startup::create_avatar_run("0-1-2:shard-discount-history").unwrap();
        r.level = 5.;
        r.inventory.push(shards::floor_shard(r.next_id));
        let boundary = crate::Field::Value(1);
        super::apply_discount_replay_policy(&mut r, &boundary, 0);
        assert_eq!(progression::level_up_cost(&r), 6.);
        super::apply_discount_replay_policy(&mut r, &boundary, 1);
        assert_eq!(progression::level_up_cost(&r), 13.);
    }
}

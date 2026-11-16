//! Authored heart rewards; ordinary species templates are frozen from v48.
use crate::{engine::EngineError, model::*};
/// Effective heart rewards are derived on demand, never persisted back to an item.
/// O(1) time/space: eight fixed stat fields, no item or enhancement cloning.
#[derive(Debug, Clone, PartialEq)]
pub struct HeartRewards {
    pub blessings: f64,
    pub bonus: Option<PartialStats>,
}
pub fn effective_heart_rewards(run: &Run, item: &Item) -> HeartRewards {
    let fast = item.kind == "heart" && crate::fast_mode::enabled(run);
    let multiplier = if fast && !run.compressed_depths.value_or(false) { 2. } else { 1. };
    let bonus = item.bonus.as_ref().map(|b| {
        let scale = |f: &Field<f64>| match f {
            Field::Value(n) if fast => Field::Value(2. * crate::combat::integer_stat(*n)),
            other => other.clone(),
        };
        PartialStats {
            offense: scale(&b.offense), defense: scale(&b.defense),
            accuracy: scale(&b.accuracy), dodge: scale(&b.dodge),
            dr: scale(&b.dr), perception: scale(&b.perception),
            max_energy: scale(&b.max_energy), max_hp: scale(&b.max_hp),
        }
    });
    HeartRewards { blessings: item.exp.value_or(0.) * multiplier, bonus }
}
fn original(s: &str) -> &str {
    s.strip_prefix("Mercuria’s ")
        .or_else(|| s.strip_prefix("Mercuria's "))
        .unwrap_or(s)
}
/// Newly dropped R51 hearts add the designer-approved sensory rewards. Existing
/// serialized hearts and pre-R51 rewards remain unchanged. O(1) fixed identities.
pub fn heart_for_revision(a: &Actor, id: f64, revision: f64) -> Result<Item, EngineError> {
    let mut item=heart_for(a,id)?;
    if revision < 51. {return Ok(item);}
    let kind=a.creature_kind.as_ref().map(|s|original(s)).unwrap_or("");
    let name=crate::revision50_creature_identity::legacy_name(original(&a.name));
    let added=match (kind,name) {
        ("cave-bat",_)|(_,"Cave Bat")=>1.,
        ("tiger",_)|(_,"Tiger")=>2.,
        ("sabretooth",_)|(_,"Sabretooth Tiger")=>3.,
        ("dragon",_)|(_,"Dragon")=>4.,
        _=>0.,
    };
    if added>0. {
        let bonus=item.bonus.get_or_insert_with(PartialStats::default);
        bonus.perception=(bonus.perception.value_or(0.)+added).into();
        let effect=item.effect.value_or(String::new());
        item.effect=format!("{effect} +{added} Perception.").into();
    }
    Ok(item)
}
pub fn heart_for(a: &Actor, id: f64) -> Result<Item, EngineError> {
    static ITEMS: std::sync::OnceLock<indexmap::IndexMap<String, Item>> =
        std::sync::OnceLock::new();
    let items = ITEMS.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/heart-items-v48.json")).unwrap()
    });
    if let Some(item) = crate::new_creatures::heart(a, id) { return Ok(item); }
    let name = crate::revision50_creature_identity::legacy_name(original(&a.name));
    let kind = a
        .creature_kind
        .as_ref()
        .map(|s| original(s))
        .unwrap_or(name);
    if kind == "zillennia" || items.contains_key(&format!("zillennia-name:{name}")) {
        if let Some(q) = a.zillennia_quality.as_ref() {
            if let Some(template) = items.get(&format!("quality:{q}")) {
                let mut i = template.clone();
                i.id = id;
                return Ok(i);
            }
        }
        let depth = a.conditions.get("godbeastDepth").copied().unwrap_or(1.0);
        let n = 3.0 + (depth / 2.0).ceil();
        return Ok(Item{id,name:"Zillennia <heart>".into(),kind:"heart".into(),heart_creature_type:"Zillennia".to_string().into(),glyph:";".into(),bonus:PartialStats{offense:n.into(),defense:n.into(),accuracy:n.into(),dodge:n.into(),dr:(depth/5.0).ceil().max(1.0).into(),perception:n.into(),max_energy:(n*2.0).into(),..Default::default()}.into(),energy:100.0.into(),exp:depth.into(),effect:format!("{depth} blessings; a sizeable permanent increase to every stat; restores 100 energy.").into(),..Default::default()});
    }
    if matches!(name, "Ichor Fish" | "Bastidion" | "Vent Fungus") {
        let fish = name == "Ichor Fish";
        let n = (a.conditions.get("ichorFishDepth").copied().unwrap_or(1.0) / 8.0)
            .ceil()
            .max(1.0);
        return Ok(Item {
            id,
            name: format!("{} <heart>", a.name),
            kind: "heart".into(),
            heart_creature_type: if fish { "Ichor Fish" } else { "Bastidion" }
                .to_string()
                .into(),
            glyph: ";".into(),
            bonus: if fish {
                PartialStats {
                    offense: n.into(),
                    ..Default::default()
                }
            } else {
                PartialStats {
                    defense: 1.0.into(),
                    ..Default::default()
                }
            }
            .into(),
            energy: if fish { 15.0 } else { 10.0 }.into(),
            exp: if fish { n } else { 1.0 }.into(),
            effect: if fish {
                "Strengthens offense and restores energy (depth-scaled)."
            } else {
                "Strengthens defense and restores energy."
            }
            .to_string()
            .into(),
            ..Default::default()
        });
    }
    if let Some(template) = items.get(if matches!(kind, "vent-fungus" | "Vent Fungus") {
        "bastidion"
    } else {
        kind
    }) {
        let mut i = template.clone();
        i.id = id;
        if name != original(&a.name) {i.name=crate::revision50_creature_identity::display_name(&i.name).into_owned();}
        return Ok(i);
    }
    let pale = name.strip_prefix("Greater ").unwrap_or(name);
    if !pale.starts_with("Pale ") {
        return Err(EngineError(format!("unknown heart species {}", a.name)));
    }
    let species = [
        "Biter", "Breather", "Spitter", "Birther", "Screamer", "Jackal",
    ]
    .into_iter()
    .find(|s| name.contains(s))
    .unwrap_or("Jackal");
    let greater = name.starts_with("Greater");
    let key = match species {
        "Spitter" => "accuracy",
        "Biter" | "Jackal" => "offense",
        "Screamer" => "dr",
        "Breather" => "defense",
        _ => "maxEnergy",
    };
    let amount = match species {
        "Screamer" => 1.0,
        "Jackal" => 0.0,
        _ => 2.0,
    };
    let exp = if greater {
        2.0
    } else if species == "Jackal" {
        0.0
    } else {
        1.0
    };
    let energy = if greater {
        40.0
    } else if species == "Jackal" {
        5.0
    } else {
        15.0
    };
    let b = serde_json::from_value(serde_json::json!({key:amount}))
        .map_err(|e| EngineError(e.to_string()))?;
    Ok(Item {
        id,
        name: format!("{} <heart>", a.name),
        kind: "heart".into(),
        heart_creature_type: name.to_string().into(),
        glyph: ";".into(),
        bonus: Field::Value(b),
        energy: energy.into(),
        exp: exp.into(),
        effect: format!("{exp} blessings; +{amount} {key}; {energy} energy (provisional values)")
            .into(),
        ..Default::default()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_all_authored_heart_rewards() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/hearts-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let a: Actor = serde_json::from_value(c["actor"].clone()).unwrap();
            let expected: Item = serde_json::from_value(c["item"].clone()).unwrap();
            assert_eq!(
                heart_for(&a, 123.0).unwrap(),
                expected,
                "{} {:?}",
                a.name,
                a.zillennia_quality
            );
        }
    }
}

#[cfg(test)]
mod fast_reward_tests {
    use super::*;
    fn room(fast: bool) -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.fast_mode = if fast { true.into() } else { Field::Missing };
        r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        r.inventory.clear(); r.offers.clear(); r.exp = 0.; r.player.energy = 0.;
        r
    }
    fn heart() -> Item {
        Item { id: 9999., name: "Rat heart".into(), kind: "heart".into(),
            heart_creature_type: "Rat".to_string().into(), exp: 0.25.into(), energy: 3.5.into(),
            quantity: 2.0.into(), bonus: PartialStats {
                offense: 0.125.into(), defense: 0.25.into(), accuracy: 0.375.into(),
                dodge: 0.625.into(), dr: 0.75.into(), perception: 1.125.into(),
                max_energy: 1.25.into(), max_hp: 1.375.into(),
            }.into(), ..Default::default() }
    }
    #[test]
    fn fast_rewards_double_all_awarded_fields_after_normal_rounding_without_mutation() {
        let r = room(true); let i = heart(); let saved = i.clone();
        let reward = effective_heart_rewards(&r, &i);
        assert_eq!(reward.blessings, 0.5);
        let b = reward.bonus.unwrap();
        assert_eq!([b.offense.value_or(0.), b.defense.value_or(0.), b.accuracy.value_or(0.),
            b.dodge.value_or(0.), b.dr.value_or(0.), b.perception.value_or(0.),
            b.max_energy.value_or(0.), b.max_hp.value_or(0.)], [0.,0.,0.,2.,2.,2.,2.,2.]);
        assert_eq!(i, saved);
        let normal = effective_heart_rewards(&room(false), &i);
        assert_eq!(normal.bonus.as_ref(), i.bonus.as_ref()); assert_eq!(normal.blessings, 0.25);
        let mut sparse = i.clone(); sparse.bonus = PartialStats { offense: Field::Null, ..Default::default() }.into();
        let b = effective_heart_rewards(&r, &sparse).bonus.unwrap();
        assert_eq!(b.offense, Field::Null); assert_eq!(b.defense, Field::Missing);
    }
    #[test]
    fn fast_consumption_uses_base_item_once_and_keeps_energy_and_armor_unscaled() {
        let mut r = room(true); let i = heart();
        r.player.stats.offense = 0.625; r.player.stats.defense = 0.375;
        r.player.stats.accuracy = 0.625; r.player.stats.dodge = 0.375;
        r.player.stats.dr = 0.625; r.player.stats.perception = 300.375;
        r.player.stats.max_energy = 100.625; r.player.stats.max_hp = 0.375.into();
        let base = r.player.stats.clone();
        r.player.abilities.push("kative-adaptive-armor".into());
        r.inventory.push(i.clone());
        assert_eq!(crate::inventory::command(&mut r, &Command::Use { id: i.id }).unwrap(), Some(true));
        let mut expected = base; crate::equipment::add_bonus(&mut expected, effective_heart_rewards(&r, &i).bonus.as_ref().unwrap());
        assert_eq!(r.player.stats, expected); assert_eq!(r.exp, 0.5); assert_eq!(r.player.energy, 3.5);
        assert_eq!(r.player.adaptive_hearts.as_ref().unwrap().get("Rat"), Some(&1.));
        assert_eq!(r.run_stats.as_ref().unwrap().hearts_eaten.get("Rat"), Some(&1.));
        assert_eq!(r.inventory[0].bonus, i.bonus); assert_eq!(r.inventory[0].exp, i.exp);
        assert_eq!(r.inventory[0].quantity.value_or(0.), 1.);
        // The stack retains base rewards through persistence, so the second eat is not quadrupled.
        let raw = crate::storage::serialize(&r).unwrap();
        let mut loaded: Run = serde_json::from_str(&raw).unwrap();
        assert_eq!(loaded, r);
        crate::inventory::command(&mut loaded, &Command::Use { id: i.id }).unwrap();
        assert_eq!(loaded.exp, 1.); assert_eq!(loaded.player.energy, 7.);
        assert_eq!(loaded.player.stats.dodge, r.player.stats.dodge + 2.);
        assert_eq!(loaded.run_stats.as_ref().unwrap().hearts_eaten.get("Rat"), Some(&2.));
    }
    #[test]
    fn epidite_base_multiplier_composes_once_with_fast_rewards() {
        let mut r = room(true);
        let mut i = heart();
        i.bonus = PartialStats { offense: 2.0.into(), defense: 1.0.into(), ..Default::default() }.into();
        i.exp = 1.0.into();
        assert!(crate::epidite::bless(&mut i)); assert!(!crate::epidite::bless(&mut i));
        let blessed = i.clone();
        let rewards = effective_heart_rewards(&r, &i);
        assert_eq!(rewards.blessings, 4.);
        assert_eq!(rewards.bonus.as_ref().unwrap().offense.value_or(0.), 8.);
        assert_eq!(rewards.bonus.as_ref().unwrap().defense.value_or(0.), 4.);
        assert_eq!(i.energy.value_or(0.), 7.);
        assert_eq!(i, blessed);
        r.inventory.push(i.clone());
        crate::inventory::command(&mut r, &Command::Use { id: i.id }).unwrap();
        assert_eq!(r.player.stats.offense, 8.); assert_eq!(r.player.stats.defense, 4.);
        assert_eq!(r.player.energy, 7.);
        assert_eq!(r.inventory[0].bonus, blessed.bonus);
    }
    #[test]
    fn normal_consumption_preserves_existing_rounding_and_save_shape() {
        let mut r = room(false); let i = heart(); let mut expected = r.player.stats.clone();
        crate::equipment::add_bonus(&mut expected, i.bonus.as_ref().unwrap());
        r.inventory.push(i.clone());
        crate::inventory::command(&mut r, &Command::Use { id: i.id }).unwrap();
        assert_eq!(r.player.stats, expected); assert_eq!(r.exp, 0.25); assert_eq!(r.player.energy, 3.5);
        let raw = crate::storage::serialize(&r).unwrap(); assert!(!raw.contains("fastMode"));
        let loaded: Run = serde_json::from_str(&raw).unwrap(); assert_eq!(loaded, r);
    }
    #[test]
    fn compressed_rewards_halve_fast_blessings_but_keep_stat_multiplier() {
        let mut r = room(true); let i = heart();
        let old = effective_heart_rewards(&r, &i);
        r.compressed_depths = true.into();
        let new = effective_heart_rewards(&r, &i);
        assert_eq!(new.blessings, old.blessings / 2.);
        assert_eq!(new.blessings, 0.25); // Fractional authored values are not rounded away.
        assert_eq!(new.bonus, old.bonus);
        let mut enhanced = i.clone();
        crate::epidite::bless(&mut enhanced);
        assert_eq!(effective_heart_rewards(&r, &enhanced).blessings, 0.5);
    }
    #[test]
    fn compressed_first_level_costs_one_and_following_thresholds_are_unchanged() {
        let mut r = room(true); r.level = 1.; r.compressed_depths = true.into();
        assert_eq!(crate::progression::level_up_cost(&r), 1.);
        r.exp = 1.; r.gods.clear();
        crate::progression::level_up(&mut r);
        assert_eq!(r.level, 2.); assert_eq!(r.exp, 0.);
        assert_eq!(crate::progression::level_up_cost(&r), 4.);
        r.level = 3.; assert_eq!(crate::progression::level_up_cost(&r), 8.);
        r.level = 1.; r.compressed_depths = Field::Missing;
        assert_eq!(crate::progression::level_up_cost(&r), 2.);
    }

    #[test]
    fn compressed_sap_uses_actual_first_threshold_without_double_halving() {
        let mut r = room(true); r.level = 1.; r.compressed_depths = true.into();
        r.inventory.push(Item { id: 9000., kind: "potion".into(), name: "Verdania Sap".into(),
            effect: "Veridian Sap".to_string().into(), ..Default::default() });
        let (x,y) = (r.player.x,r.player.y);
        assert!(crate::potions::apply(&mut r, 0, x, y, false).unwrap());
        assert_eq!(r.level, 2.); assert_eq!(r.exp, 0.);
        assert_eq!(r.run_stats.as_ref().unwrap().blessings_received, 1.);
    }

}

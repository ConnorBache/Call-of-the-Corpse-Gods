//! Authored heart rewards; ordinary species templates are frozen from v48.
use crate::{engine::EngineError, model::*};
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

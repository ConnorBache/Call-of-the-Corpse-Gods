//! Read-only Revision 50 item knowledge. Saved discovery fields are retained for
//! old saves, but never gate R50 names or effects and are not changed by hovering.
use crate::{Item, Run};

/// Fixed authored lookup: O(effect bytes), constant scratch.
pub fn potion_name(effect: &str) -> Option<&'static str> {
    Some(match effect {
        "healing" => "Potion of Healing",
        "energy" => "Potion of Energy",
        "incineration" => "Potion of Incineration",
        "caustic gas" => "Potion of Caustic Gas",
        "paralysis" => "Potion of Paralysis",
        "confusion" => "Potion of Confusion",
        "telepathy" => "Potion of Telepathy",
        "levitation" => "Potion of Levitation",
        "speed" => "Potion of Speed",
        "invisibility" => "Potion of Invisibility",
        "descent" => "Potion of Descent",
        "cleansing" => "Potion of Cleansing",
        "fire immunity" => "Potion of Fire Immunity",
        "darkness" => "Potion of Darkness",
        "flooding" => "Potion of Flooding",
        "freezing" => "Potion of Freezing",
        "repulsion" => "Potion of Repulsion",
        "discord" => "Potion of Discord",
        "teleportation" => "Potion of Teleportation",
        "shattering" => "Potion of Shattering",
        "Veridian Sap" => "Verdania Sap",
        _ => return None,
    })
}

/// O(item identity bytes + output bytes), no actor, visibility, RNG or world query.
/// Callers must first admit the item through their own inventory/ground knowledge.
pub fn display_name(r: &Run, item: &Item) -> String {
    let mut name=base_display_name(r,item);
    if crate::revision50::enabled(r) && item.kind == "heart"
        && item.source_actor_id.as_ref().is_some() && item.borge_depth.as_ref().is_some() {
        name.push_str(" (marked)");
    }
    if !crate::revision50::enabled(r)||!item.tinkered.value_or(false)||!matches!(item.kind.as_str(),"ring"|"amulet"){return name;}
    // Tinker is admitted once. An existing grant is retained in the primary
    // field, and the new grant is appended; otherwise it is the primary grant.
    let id=item.additional_granted_abilities.last().or_else(||item.granted_ability.as_ref());
    let Some(ability)=id.and_then(|id|crate::progression::definition_for_revision(r.simulation_revision.value_or(29.),id)) else{return name;};
    let perk=if ability.id=="shrail-rage-at-the-dying-of-the-light"{"Ragelight"}else if ability.id=="pelian-you-shall-never-be-rid-of-me"{"Never Be Rid of Me"}else{ability.name.as_str()};
    if name.ends_with(&format!(" and {}",ability.name)){name.truncate(name.len()-ability.name.len());name.push_str(perk);return name;}
    if !name.ends_with(&format!(" and {perk}")){name.push_str(" and ");name.push_str(perk);}
    name
}
fn base_display_name(r: &Run, item: &Item) -> String {
    if !crate::revision50::enabled(r) { return item.name.clone(); }
    if crate::ahvamitt::is_mask(item) { return crate::ahvamitt::MASQUE_NAME.into(); }
    if let Some(name) = crate::revision52_amulet_tiers::display_name(r,item) { return name; }
    if let Some(name) = crate::revision50_rings::display_name(item)
        .or_else(|| crate::revision50_amulet_effects::display_name(item)) {
        return if crate::revision50_rings::retired(r,item) { format!("Retired {name}") } else { name.into() };
    }
    if item.kind == "potion" {
        if let Some(name) = item.effect.as_ref().and_then(|effect| potion_name(effect)) {
            return format!("{}{name}{}", if crate::revision50_potions::retired(r,item) {"Retired "} else {""}, if item.diluted.value_or(false) { " (diluted)" } else { "" });
        }
    }
    crate::revision50_creature_identity::display_name(&item.name).into_owned()
}

/// Authored default copy with existing half-dose scalar/duration values; spatial
/// footprint rules remain fixed unless the effect explicitly scales them.
/// O(effect bytes + output bytes), constant scratch beyond the returned string.
pub fn potion_effect_description(effect: &str, diluted: bool) -> Option<String> {
    let strength = if diluted { 0.5 } else { 1.0 };
    let turns = |normal: f64| (normal * strength).ceil();
    Some(match effect {
        "healing" => format!("Restores {} HP.", 50.0 * strength),
        "energy" => format!("Restores {} energy.", 50.0 * strength),
        "incineration" => "Ignites the impact square and sets creatures there on fire.".into(),
        "caustic gas" => "Releases a cloud of caustic gas at the impact location.".into(),
        "paralysis" => format!("Releases paralysis gas and paralyzes an affected creature for {} turns.", turns(5.0)),
        "confusion" => "Releases a cloud of confusion gas at the impact location.".into(),
        "telepathy" => format!("Reveals creatures throughout the floor for {} turns.", turns(30.0)),
        "levitation" => format!("Grants flight for {} turns.", turns(20.0)),
        "speed" => format!("Grants haste for {} turns.", turns(10.0)),
        "invisibility" => format!("Grants invisibility for {} turns.", turns(20.0)),
        "descent" => "Collapses the ground around the impact location.".into(),
        "cleansing" => "Removes harmful temporary effects from the affected creature.".into(),
        "fire immunity" => format!("Grants immunity to fire for {} turns.", turns(20.0)),
        "darkness" => format!("Reduces illumination by {} in a 5×5 area for {} turns.", 50.0 * strength, turns(20.0)),
        "flooding" => format!("Turns a 3×3 area into shallow water for {} turns.", turns(20.0)),
        "freezing" => format!("Freezes water into ice and slows creatures in a 3×3 area for {} turns, and the ice remains until melted.", turns(3.0)),
        "repulsion" => format!("Pushes creatures within 3 tiles of the impact location up to {} tiles away.", turns(3.0)),
        "discord" => format!("Makes an affected creature attack other creatures regardless of allegiance for {} turns.", turns(10.0)),
        "teleportation" => "Teleports the affected creature to a random valid square on the current floor.".into(),
        "shattering" => "Destroys destructible walls and obstacles within 2 tiles of the impact location.".into(),
        "Veridian Sap" if diluted => "Restores half your missing health and energy and grants half the blessings still needed for your next level, rounded up.".into(),
        "Veridian Sap" => "Fully restores health and energy and grants the blessings still needed for your next level.".into(),
        _ => return None,
    })
}

/// Current live copy follows the same policy as mechanics; historical replay copy
/// remains unchanged. O(effect bytes + output bytes), constant scratch.
pub fn potion_description_for_run(run:&Run,item:&Item)->String {
    if crate::revision50_potions::retired(run,item) {
        return format!("{}\n{}",display_name(run,item),crate::revision50_potions::RETIRED_MESSAGE);
    }
    if crate::potion_rules::current(run) {
        let description=match item.effect.as_ref().map(String::as_str) {
            Some("incineration")=>Some("Ignites a 5-tile-diameter circle at the impact location."),
            Some("shattering")=>Some("Destroys destructible walls and obstacles within 6 tiles of the impact location."),
            _=>None,
        };
        if let Some(description)=description { return format!("{}\n{description}",display_name(run,item)); }
    }
    potion_description_for_revision(item,run.simulation_revision.value_or(29.))
}

pub fn potion_description(item: &Item) -> String {
    potion_description_for_revision(item,50.)
}
/// Revision-aware text; legacy public helper retains the original R50 wording.
pub fn potion_description_for_revision(item:&Item,revision:f64)->String {
    let effect = item.effect.as_ref().map_or("", String::as_str);
    let name: &str = match potion_name(effect) { Some(name) => name, None => &item.name };
    let description = if revision>=52. && effect=="darkness" {
        Some(format!("Immediately sets illumination to -100 in a radius-3 circle for {} turns, suppressing all light while active.",if item.diluted.value_or(false) {10} else {20}))
    } else {potion_effect_description(effect, item.diluted.value_or(false))}
        .unwrap_or_else(|| "This potion's effect is unavailable in this revision.".into());
    format!("{name}{}\n{description}", if item.diluted.value_or(false) { " (diluted)" } else { "" })
}

#[cfg(test)]mod tinkered_names_tests {
 use super::*;
 #[test]fn jewelry_suffix_is_consistent_tiered_and_read_only(){
  let r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let mut r=r;r.simulation_revision=52.0.into();
  let mut i=Item{id:900.,kind:"ring".into(),name:"Ring of Light and Execute".into(),effect:"light".to_string().into(),tinkered:true.into(),granted_ability:"shrail-execute".to_string().into(),..Default::default()};
  let before=i.clone();assert_eq!(display_name(&r,&i),"Ring of Light and Execute");assert_eq!(i,before);
  i.kind="amulet".into();i.effect="lightning".to_string().into();i.name="Amulet of Lightning and Execute".into();let n=display_name(&r,&i);assert!(n.starts_with("Tier ")&&n.ends_with(" Amulet of Lightning and Execute"));assert_eq!(n.matches("Execute").count(),1);
  i.additional_granted_abilities.push("numo-distract".into());assert!(display_name(&r,&i).ends_with(" and Distract"));
  i.additional_granted_abilities=vec!["shrail-rage-at-the-dying-of-the-light".into()];assert!(display_name(&r,&i).ends_with(" and Ragelight"));
  i.tinkered=false.into();assert!(!display_name(&r,&i).contains(" and "));r.simulation_revision=49.0.into();assert_eq!(display_name(&r,&i),i.name);
 }
}

#[cfg(test)]
mod marked_heart_tests {
    use super::*;
    #[test]
    fn marked_heart_pickup_names_preserve_identity_and_provenance() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.inventory.clear();r.offers.clear();
        let mut heart=Item { id: 909., kind: "heart".into(), name: "Rat heart".into(), source_actor_id: 44.0.into(), borge_depth: 12.0.into(), ..Default::default() };
        let before=heart.clone();
        assert_eq!(display_name(&r,&heart),"Rat heart (marked)");assert_eq!(heart,before);
        let (depth,x,y)=(r.depth,r.player.x,r.player.y);
        r.ground_at_mut(depth).clear();r.ground_at_mut(depth).push(crate::GroundItem {x,y,item:heart.clone(),..Default::default()});
        assert_eq!(crate::inventory::collect_underfoot(&mut r,100.).unwrap(),1);
        assert!(r.log.iter().any(|line|line=="You pick up Rat heart (marked)."));
        let carried=&r.inventory[0];assert_eq!(carried.name,heart.name);assert_eq!(carried.source_actor_id,heart.source_actor_id);assert_eq!(carried.borge_depth,heart.borge_depth);
        assert_eq!(display_name(&r,carried),"Rat heart (marked)");
        heart.borge_depth=crate::Field::Missing;assert_eq!(display_name(&r,&heart),"Rat heart");
        heart.borge_depth=12.0.into();heart.source_actor_id=crate::Field::Missing;assert_eq!(display_name(&r,&heart),"Rat heart");
        r.simulation_revision=49.0.into();assert_eq!(display_name(&r,&before),"Rat heart");
    }
}

#[cfg(test)]
mod potion_policy_copy_tests {
    use super::*;
    #[test]
    fn potion_footprint_copy_tracks_live_and_historical_rules() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        for (effect,current,legacy) in [("incineration","5-tile-diameter","impact square"),("shattering","within 6 tiles","within 2 tiles")] {
            let item=Item{kind:"potion".into(),effect:effect.to_string().into(),..Default::default()};
            r.potion_rules_from=crate::Field::Missing;
            assert!(potion_description_for_run(&r,&item).contains(current));
            r.potion_rules_from=crate::Field::Null;
            assert!(potion_description_for_run(&r,&item).contains(legacy));
        }
    }
}

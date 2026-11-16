//! Revision 50 carrying categories. Identity kinds are never rewritten.
//! Every carried ring/amulet is active and counted; only trinket stacks are unlimited.
use crate::{Item, Run};

pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.0) >= 50.0 }
/// R50 carried-jewelry identity. Call only for an item in an actor's inventory.
/// Historical equipped flags do not turn carried jewelry off or free capacity.
pub fn worn(i: &Item) -> bool { matches!(i.kind.as_str(), "ring" | "amulet") }
/// Revision-aware equipment activity, preserving historical slot semantics.
pub fn active(r: &Run, i: &Item) -> bool {
    if enabled(r) { worn(i) } else { crate::crafting::is_equipped(i) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category { Ring, Amulet, Potion, Heart, Trinket }
/// Carrying classification is separate from action/quest identity (Item.kind).
/// O(identity bytes), O(1) scratch; ordinary unequippable armor/melee are excluded.
pub fn category(i: &Item) -> Option<Category> {
    match i.kind.as_str() {
        "ring" => Some(Category::Ring), "amulet" => Some(Category::Amulet),
        "potion" => Some(Category::Potion), "heart" => Some(Category::Heart),
        "trinket" | "tool" => Some(Category::Trinket),
        _ if crate::projectiles::is_bow(i) || crate::inventory::is_arrow(i)
            || crate::borge::is_shard(i) || crate::phantasmii_generation::is_shard(i)
            || crate::royal_honey::is_honey(i) => Some(Category::Trinket),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capacity {
    pub rings: f64,
    pub amulets: f64,
    pub potions: f64,
    pub hearts: f64,
    /// None means unlimited. No infinity is returned or written to JSON/UI.
    pub trinkets: Option<f64>,
}
impl Capacity {
    /// Sum of finite category limits, not a global backpack/trinket limit.
    pub fn total(self) -> f64 {
        self.rings + self.amulets + self.potions + self.hearts
    }
}
pub fn capacity(r: &Run) -> Capacity {
    capacity_for_abilities_revision(&r.player.abilities, r.simulation_revision.value_or(29.))
}
/// R52 adds five base heart slots; every existing ability capacity bonus is retained.
/// O(B) existing ability scans, O(1) additional arithmetic and scratch.
pub fn capacity_for_abilities_revision(abilities: &[String], revision: f64) -> Capacity {
    let mut capacity = capacity_for_abilities(abilities);
    if revision >= 52. { capacity.hearts += 5.; }
    capacity
}
pub fn capacity_for_abilities(abilities: &[String]) -> Capacity {
    let has = |id: &str| abilities.iter().any(|a| a == id);
    let tools = has("isheilah-tools-of-the-trade");
    let bottomless = has("kative-bottomless-pack");
    let appropriate = has("kative-appropriate");
    Capacity {
        rings: 5.0 + if tools { 5.0 } else { 0.0 } + if appropriate { 3.0 } else { 0.0 },
        amulets: 5.0 + if bottomless { 2.0 } else { 0.0 } + if appropriate { 3.0 } else { 0.0 },
        potions: 5.0 + if tools { 5.0 } else { 0.0 } + if bottomless { 5.0 } else { 0.0 },
        hearts: 5.0,
        trinkets: None,
    }
}
pub fn classified(i: &Item) -> bool { category(i).is_some() }
pub fn unlimited(i: &Item) -> bool { category(i) == Some(Category::Trinket) }
pub fn units(i: &Item) -> f64 {
    if category(i) == Some(Category::Trinket) { 1.0 }
    else { crate::inventory::quantity(i) }
}
pub fn limited_occupancy(r: &Run) -> f64 {
    r.inventory.iter().filter(|i| !unlimited(i)).map(units).fold(0., |total, units| total + units)
}
#[derive(Default)]
struct Usage { rings: f64, amulets: f64, potions: f64, hearts: f64 }
impl Usage {
    fn of(items: &[Item]) -> Self {
        let mut used = Self::default();
        for i in items {
            let n = units(i);
            match category(i) {
                Some(Category::Ring) => used.rings += n,
                Some(Category::Amulet) => used.amulets += n,
                Some(Category::Potion) => used.potions += n,
                Some(Category::Heart) => used.hearts += n,
                _ => {},
            }
        }
        used
    }
    fn fits(&self, c: Capacity) -> bool {
        self.rings <= c.rings && self.amulets <= c.amulets
            && self.potions <= c.potions && self.hearts <= c.hearts
    }
}
/// O(I+B) time, O(1) scratch; special items never weaken finite-category limits.
pub fn finite_categories_fit(items: &[Item], abilities: &[String]) -> bool {
    Usage::of(items).fits(capacity_for_abilities(abilities))
}
/// Revision-aware admission shares the same category limits as live pickup/UI.
pub fn finite_categories_fit_revision(items: &[Item], abilities: &[String], revision: f64) -> bool {
    Usage::of(items).fits(capacity_for_abilities_revision(abilities, revision))
}
/// O(I+B) time, O(1) scratch for I inventory entries and B owned blessings.
/// Each finite category has its own budget, independent of every other category.
/// Unlimited trinkets return this item's full quantity, never a nonfinite sentinel.
pub fn space_for(r: &Run, item: &Item) -> f64 {
    if category(item) == Some(Category::Trinket) { return crate::inventory::quantity(item); }
    let c = capacity(r);
    let used = Usage::of(&r.inventory);
    match category(item) {
        Some(Category::Ring) => c.rings - used.rings,
        Some(Category::Amulet) => c.amulets - used.amulets,
        Some(Category::Potion) => c.potions - used.potions,
        Some(Category::Heart) => c.hearts - used.hearts,
        _ => 0.0,
    }.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.inventory.clear(); r.player.abilities.clear(); r.player.active.clear();
        r.simulation_revision = 50.0.into(); r
    }
    fn item(kind: &str, n: f64) -> Item { Item { kind: kind.into(), quantity: n.into(), ..Default::default() } }
    #[test]
    fn base_and_combined_capacity() {
        let mut r=run(); assert_eq!(capacity(&r).total(),20.0);
        r.player.abilities=vec!["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into(),"kative-appropriate".into()];
        assert_eq!(capacity(&r),Capacity{rings:13.0,amulets:10.0,potions:15.0,hearts:5.0,trinkets:None});
        assert_eq!(capacity(&r).total(),43.0);
    }
    #[test]
    fn five_independent_categories_count_every_carried_jewel() {
        let mut r=run(); r.inventory=vec![item("ring",5.0),item("amulet",5.0),item("potion",5.0),item("heart",5.0)];
        r.inventory[0].equipped="ring1".to_string().into();
        for kind in ["ring","amulet","potion","heart"] { assert_eq!(space_for(&r,&item(kind,1.0)),0.0); }
        assert_eq!(crate::inventory::occupancy(&r),20.0);
        assert!(finite_categories_fit(&r.inventory,&r.player.abilities));
        r.player.abilities.push("isheilah-tools-of-the-trade".into());
        assert_eq!(space_for(&r,&item("ring",1.0)),5.0);
        assert_eq!(space_for(&r,&item("amulet",1.0)),0.0);
        assert_eq!(space_for(&r,&item("potion",1.0)),5.0);
        r.inventory.reverse();
        assert_eq!(space_for(&r,&item("ring",1.0)),5.0);
    }
    #[test]
    fn carried_jewelry_has_no_manual_equipment_toggle_or_capacity_escape() {
        let mut r=run(); let mut ring=item("ring",1.0); ring.id=99.0;
        r.inventory=vec![item("amulet",5.0),ring];
        assert_eq!(crate::inventory::occupancy(&r),6.0);
        assert!(worn(&r.inventory[1]));
        for c in [crate::Command::Equip{id:99.0,slot:"ring1".into()}, crate::Command::Unequip{id:99.0}] {
            assert!(crate::equipment::change_blocker(&r,&c).is_some());
        }
        r.inventory[1].equipped="ring1".to_string().into();
        assert_eq!(crate::inventory::occupancy(&r),6.0);
        assert_eq!(space_for(&r,&item("ring",1.0)),4.0);
    }
    #[test]
    fn weapons_and_armor_cannot_be_equipped() {
        let mut r=run(); let mut weapon=item("weapon",1.0); weapon.id=1.0; r.inventory.push(weapon);
        assert!(!crate::equipment::equip(&mut r,1.0,"hand1").unwrap());
        assert!(r.inventory[0].equipped.is_missing());
    }
    #[test]
    fn pickup_respects_categories_and_collects_tools() {
        let mut r=run(); let depth=r.depth;
        r.inventory.push(item("potion",4.0));
        let (x,y)=(r.player.x,r.player.y);
        *r.ground_at_mut(depth)=vec![
            crate::GroundItem{x,y,item:item("potion",3.0),auto_pickup_blocked:crate::Field::Missing},
            crate::GroundItem{x,y,item:item("heart",3.0),auto_pickup_blocked:crate::Field::Missing},
            crate::GroundItem{x,y,item:item("tool",1.0),auto_pickup_blocked:crate::Field::Missing},
        ];
        assert_eq!(crate::inventory::collect_underfoot(&mut r,15.0).unwrap(),5);
        assert_eq!(space_for(&r,&item("potion",1.0)),0.0);
        assert_eq!(space_for(&r,&item("heart",1.0)),2.0);
        let ground=r.ground.get(&depth.to_string()).unwrap();
        assert_eq!(ground.len(),1);
        assert_eq!(crate::inventory::quantity(&ground[0].item),2.0);
        assert!(r.inventory.iter().any(|i| i.kind == "tool"));
    }
    #[test]
    fn bow_apply_admission_does_not_require_equipping() {
        let mut r=run(); let bow=crate::equipment::make_equipment("Bow",10.0).unwrap();
        r.inventory=vec![bow,crate::equipment::make_arrows(11.0,3.0)];
        let shot=crate::Command::Shoot{id:10.0,x:r.player.x,y:r.player.y};
        assert!(crate::projectiles::blocker(&r,&shot).is_none());
        r.simulation_revision=49.0.into();
        assert!(crate::projectiles::blocker(&r,&shot).is_some());
    }
    #[test]
    fn legacy_capacity_unchanged() {
        let mut r=run(); r.simulation_revision=49.0.into();
        let mut ring=item("ring",1.0);ring.equipped="ring1".to_string().into();r.inventory.push(ring);
        assert_eq!(crate::inventory::occupancy(&r),1.0);
        assert_eq!(crate::inventory::carrying_capacity(&r).unwrap(),20.0);
    }

    #[test]
    fn special_categories_preserve_identity_and_have_unlimited_stack_space() {
        let mut r=run();
        let mut special=vec![crate::equipment::make_equipment("Bow",201.).unwrap(),
            crate::equipment::make_arrows(202.,500.),crate::borge::shard(203.),
            crate::phantasmii_generation::dagger(204.),crate::phantasmii_generation::shard(205.),
            crate::ahvamitt::mask_for_revision(206.,50.),
            Item{id:207.,kind:"tool".into(),effect:"tree-tap".to_string().into(),..Default::default()},
            crate::royal_honey::item(208.)];
        for i in &special {assert_eq!(category(i),Some(Category::Trinket));assert_eq!(units(i),1.);}
        assert!(crate::inventory::is_arrow(&special[1]));assert!(crate::borge::is_shard(&special[2]));
        assert!(crate::phantasmii_generation::is_dagger(&special[3]));assert!(crate::phantasmii_generation::is_shard(&special[4]));
        assert!(crate::ahvamitt::is_mask(&special[5]));
        for id in 300..450 {r.inventory.push(crate::equipment::make_equipment("Composite bow",id as f64).unwrap());}
        let original=special.clone();let (x,y)=(r.player.x,r.player.y);let depth=r.depth;
        for item in special.drain(..) {r.ground_at_mut(depth).push(crate::GroundItem{x,y,item,..Default::default()});}
        // Phantasmii shards retain their dream-plane identity even though their
        // carrying category is the same unlimited category as waking specials.
        assert_eq!(crate::inventory::collect_underfoot(&mut r,15.).unwrap(),506);
        let remaining=r.ground.get(&depth.to_string()).unwrap();
        assert_eq!(remaining.len(),1);assert_eq!(remaining[0].item,original[4]);
        assert!(!r.inventory.iter().any(|i|i.id==205.));
        r.dreamwalking=crate::dreamwalking::State{unlocked:true,..Default::default()}.into();
        assert_eq!(crate::dreamwalking::ability(&mut r,crate::dreamwalking::DREAMWALK_ID).unwrap(),Some(true));
        assert_eq!(crate::inventory::collect_underfoot(&mut r,15.).unwrap(),1);
        assert!(r.ground.get(&depth.to_string()).unwrap().is_empty());
        for item in &original {assert_eq!(r.inventory.iter().find(|i|i.id==item.id).unwrap(),item);}
        assert_eq!(limited_occupancy(&r),0.);assert!(!limited_occupancy(&r).is_sign_negative());assert_eq!(crate::inventory::occupancy(&r),158.);
        assert_eq!(crate::inventory::carrying_capacity(&r).unwrap(),20.);
        assert_eq!(space_for(&r,&crate::equipment::make_arrows(600.,10000.)),10000.);
        assert!(crate::equipment::change_blocker(&r,&crate::Command::Equip{id:203.,slot:"hand1".into()}).is_some());
        assert!(crate::borge::use_blocker(&r,&crate::borge::shard(203.),r.player.x+1.,r.player.y).is_some());
    }
    #[test]
    fn unlimited_trinkets_do_not_relax_any_finite_limit() {
        let mut r=run();
        for id in 0..120 { r.inventory.push(crate::equipment::make_equipment("Bow",id as f64).unwrap()); }
        r.inventory.extend([item("ring",5.),item("amulet",5.),item("potion",5.),item("heart",5.)]);
        assert!(finite_categories_fit(&r.inventory,&r.player.abilities));
        assert_eq!(limited_occupancy(&r),20.);
        for kind in ["ring","amulet","potion","heart"] {assert_eq!(space_for(&r,&item(kind,1.)),0.);}
        let mut extra=item("ring",1.);extra.equipped="ring1".to_string().into();r.inventory.push(extra);
        assert!(!finite_categories_fit(&r.inventory,&r.player.abilities));
    }
    #[test]
    fn pickup_activates_bonuses_grants_and_amulets_without_flags_or_refills() {
        let mut r=run(); r.player.conditions.clear(); r.player.equipment_abilities=crate::Field::Missing;
        r.player.ability_order=crate::Field::Missing; r.player.ability_reserves=crate::Field::Missing;
        r.ground.clear(); r.actors_at_mut(r.depth).clear();
        let mut ring=item("ring",1.); ring.id=900.; ring.name="Carried ring".into();
        ring.effect=crate::revision50_rings::PROTECTION.to_string().into();
        ring.granted_ability="numo-distract".to_string().into();
        ring.bonus=crate::PartialStats{defense:2.0.into(),..Default::default()}.into();
        let amulet=crate::revision50_amulet_effects::make_amulet(crate::revision50_amulet_effects::Effect::Firebolt,901.).unwrap();
        let (x,y,depth)=(r.player.x,r.player.y,r.depth);
        for item in [ring,amulet] { r.ground_at_mut(depth).push(crate::GroundItem{x,y,item,..Default::default()}); }
        let defense=crate::combat::effective_stats(&r,&r.player).defense;
        let energy=r.player.energy;
        assert_eq!(crate::inventory::collect_underfoot(&mut r,20.).unwrap(),2);
        assert_eq!(crate::combat::effective_stats(&r,&r.player).defense,defense+2.);
        assert!(crate::revision50_rings::has_effect(&r,&r.player,crate::revision50_rings::PROTECTION));
        assert!(r.player.abilities.contains(&"numo-distract".into()));
        assert!(r.inventory.iter().all(|i|i.equipped.is_missing()));
        let spec=crate::revision50_amulet_effects::item_spec(&r.inventory[1]).unwrap();
        let plan=crate::revision50_amulets::plan_item_activation(&r,901.,Some(&spec)).unwrap();
        crate::revision50_amulets::commit_item_activation(&mut r,901.,&plan,Some(&spec)).unwrap();
        r.player.ability_reserves.as_mut().unwrap()["numo-distract"].energy=3.;
        for id in [900.,901.] { assert_eq!(crate::inventory::command(&mut r,&crate::Command::Drop{id}).unwrap(),Some(true)); }
        assert!(!r.player.abilities.contains(&"numo-distract".into()));
        assert!(!crate::revision50_rings::has_effect(&r,&r.player,crate::revision50_rings::PROTECTION));
        // Ordinary placement gives each dropped item its own ground cell.
        // Visit both actual destinations rather than assuming a stacked pile.
        let destinations:Vec<_>=r.ground_at_mut(depth).iter_mut().map(|entry| {
            entry.auto_pickup_blocked=crate::Field::Missing;(entry.x,entry.y)
        }).collect();
        assert_eq!(destinations.len(),2);
        let mut reacquired=0;
        for (x,y) in destinations {
            (r.player.x,r.player.y)=(x,y);
            reacquired+=crate::inventory::collect_underfoot(&mut r,20.).unwrap();
        }
        assert_eq!(reacquired,2);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()["numo-distract"].energy,3.);
        assert_eq!(r.inventory.iter().find(|i|i.id==901.).unwrap().amulet_state.as_ref().unwrap().energy,30.);
        assert_eq!(r.player.energy,energy);
    }
    #[test]
    fn removing_capacity_grants_never_discards_remaining_items() {
        let mut r=run(); r.player.conditions.clear(); r.player.equipment_abilities=crate::Field::Missing;
        let mut source=item("ring",1.); source.id=900.;
        source.granted_ability="isheilah-tools-of-the-trade".to_string().into();
        r.inventory=vec![source,item("potion",6.)];
        crate::equipment::sync_ownership(&mut r).unwrap();
        assert_eq!(capacity(&r).potions,10.);
        let before=r.inventory.clone();
        assert!(crate::equipment::removal_blocker(&r,900.,false).is_some());
        assert_eq!(crate::inventory::command(&mut r,&crate::Command::Drop{id:900.}).unwrap(),Some(false));
        assert_eq!(r.inventory,before);
        r.inventory[1].quantity=5.0.into();
        assert_eq!(crate::inventory::command(&mut r,&crate::Command::Drop{id:900.}).unwrap(),Some(true));
        assert_eq!(capacity(&r).potions,5.);
        assert_eq!(crate::inventory::quantity(&r.inventory[0]),5.);
    }
    #[test]
    fn tree_tap_cannot_create_a_sixth_potion() {
        let mut r=run(); r.inventory=vec![item("potion",5.)];
        let (x,y)=(r.player.x+1.,r.player.y);
        let event=crate::GodbeastEvent{kind:"Veridian".into(),x,y,..Default::default()};
        r.floor_mut().unwrap().godbeast_event=event.into();
        let before=r.clone(); let result=crate::godbeast_interactions::tap_veridian(&mut r);
        assert!(!result.success); assert!(result.sap.is_none()); assert_eq!(r,before);
    }
    #[test]
    fn masque_name_is_revisioned_and_old_spellings_keep_teleport_identity() {
        let mut r=run();
        for old_name in [crate::ahvamitt::MASK_NAME,"Mask of the Trickster",crate::ahvamitt::MASQUE_NAME] {
            let mut mask=crate::ahvamitt::mask(900.);mask.name=old_name.into();
            assert!(crate::ahvamitt::is_mask(&mask));assert_eq!(category(&mask),Some(Category::Trinket));
            let before=mask.clone();
            assert_eq!(crate::revision50_item_knowledge::display_name(&r,&mask),crate::ahvamitt::MASQUE_NAME);
            assert_eq!(mask,before);
            r.inventory=vec![mask];assert!(crate::ahvamitt::has_mask(&r));
        }
        r.simulation_revision=49.0.into();let old=crate::ahvamitt::mask(901.);
        assert_eq!(crate::revision50_item_knowledge::display_name(&r,&old),crate::ahvamitt::MASK_NAME);
        assert_eq!(crate::ahvamitt::mask_for_revision(901.,49.),old);
        assert_eq!(crate::ahvamitt::mask_for_revision(901.,50.).name,crate::ahvamitt::MASQUE_NAME);
    }

}

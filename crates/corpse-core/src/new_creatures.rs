//! Revision33 additions. Frozen population and reward catalogs remain unchanged.
use crate::{Actor, Item, PartialStats, Run};
use std::sync::OnceLock;
pub const NIGHT_HISTORY: &str = "Terrible creatures of tentacle and sharp appendage. They skewer their victims with their tentacles, to drink their blood.";
pub const SKIN_HISTORY: &str = "Terrible creatures which devour their victims minds, taking their form.";
/// Approved factual reference copy; unlike Night/Skin histories, not a verbatim designer quote.
pub const QUEEN_HISTORY: &str = "Queens such as this take care of their nests, spawning new young and weaving royal jelly to nourish them.";
pub const DISGUISES: [&str;6] = ["rat", "blorb", "cave-bat", "shellback-beetle", "hemogoblin", "needleling"];
pub fn templates() -> &'static Vec<Actor> {
    static DATA: OnceLock<Vec<Actor>> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("catalog/new-creatures-v33.json")).expect("revision33 new creatures"))
}
pub fn template(kind: &str) -> Option<&'static Actor> {
    templates().iter().find(|a| a.creature_kind.as_ref().is_some_and(|k|k==kind))
}
pub fn definition(a: &Actor) -> Option<&'static crate::enemy_traits::Definition> {
    static DATA: OnceLock<Vec<crate::enemy_traits::Definition>> = OnceLock::new();
    DATA.get_or_init(|| templates().iter().map(|a| crate::enemy_traits::Definition {
        bleeds: Some(true), id: a.creature_kind.as_ref().unwrap().clone(), name:a.name.clone(),
        glyph:a.glyph.clone(), color:a.color.clone(), abilities:a.abilities.clone(),
    }).collect()).iter().find(|d| a.creature_kind.as_ref()==Some(&d.id) || a.name==d.name)
}
pub fn heart_reward(kind:&str) -> Option<i32> { match kind {"night-crawler"|"skin-walker"=>Some(2),"saidribee-queen"=>Some(3),_=>None} }
pub fn danger(kind:&str) -> Option<f64> { match kind {"night-crawler"|"skin-walker"=>Some(12.),"saidribee-queen"=>Some(24.),_=>None} }
pub fn heart(a:&Actor,id:f64) -> Option<Item> {
    let d = definition(a)?;
    if d.id=="saidribee-queen" {
        // Exact existing Guardian reward pattern; only Queen identity is new.
        let guardian=&crate::population::catalog().actors["saidri-guardian"];
        let mut item=crate::inventory::heart_for(guardian,id).expect("authored Guardian heart");
        item.name="Saidribee Queen <heart>".into();
        item.heart_creature_type="Saidribee Queen".to_string().into();
        return Some(item);
    }
    let night=d.id=="night-crawler";
    Some(Item {id,name:format!("{} <heart>",d.name),kind:"heart".into(),glyph:";".into(),
        heart_creature_type:d.name.clone().into(), energy:25.0.into(),exp:2.0.into(),
        bonus:if night { PartialStats {offense:2.0.into(),..Default::default()} }
            else { PartialStats {perception:2.0.into(),..Default::default()} }.into(),
        effect:format!("2 blessings; +2 {}; restores 25 energy (provisional).",if night {"offense"} else {"perception"}).into(),
        ..Default::default() })
}
pub fn is_skin_walker(a:&Actor)->bool { a.creature_kind.as_ref().is_some_and(|k|k=="skin-walker") }
/// Fixed construction-time choice stored in serialized conditions, not presentation RNG.
pub fn initialize_disguise(a:&mut Actor) {
    if is_skin_walker(a) && !a.conditions.contains_key("skinWalkerDisguise") {
        let choice=(a.id as u64).wrapping_mul(2_654_435_761)%DISGUISES.len() as u64;
        a.conditions.insert("skinWalkerDisguise".into(),choice as f64);
    }
}
/// O(A + B*R), B<=2 player bodies; proximity alone never reveals through occlusion.
/// Caller must independently gate actor visibility. No RNG or simulation mutation.
pub fn disguise_kind<'a>(r:&Run,a:&'a Actor)->Option<&'static str> {
    if r.simulation_revision.value_or(29.)<33. || !is_skin_walker(a) { return None; }
    let revealed=crate::half_here::origins(r).iter().any(|body| !body.dead.value_or(false) && body.hp>0.
        && (body.x-a.x).abs().max((body.y-a.y).abs())<=2.
        && crate::vision::line_of_sight(r,body,a)
        && crate::vision::detection_chance(r,body,a)>0.);
    if revealed { return None; }
    let n=a.conditions.get("skinWalkerDisguise").copied().unwrap_or(0.) as usize;
    Some(DISGUISES[n%DISGUISES.len()])
}

//! The Queen's one-use, non-reviving health-and-energy food reward.
use crate::{Item, Run};
pub const NAME: &str = "Royal Honey";
pub fn item(id: f64) -> Item {
    Item { id, name: NAME.into(), kind: "food".into(), glyph: "%".into(),
        effect: "royal-honey".to_string().into(), identified: true.into(), ..Default::default() }
}
pub fn is_honey(i: &Item) -> bool { i.kind=="food" && i.name==NAME && i.effect.as_ref().is_some_and(|s|s=="royal-honey") }
/// Resource maxima are recomputed from current equipment and conditions. No
/// ability, blessing, healing-over-time, resurrection or over-cap bonus is added.
pub fn eat(r: &mut Run) -> bool {
    if r.simulation_revision.value_or(29.0)<33.0 || r.status!="playing"
        || r.player.dead.value_or(false) || r.player.hp<=0.0 { return false; }
    crate::equipment::sync_health(r,false);
    let energy=crate::combat::effective_stats(r,&r.player).max_energy;
    if r.simulation_revision.value_or(29.0)>=43.0 {
        // Fixed restorative amounts, not an increase to either resource maximum.
        r.player.hp=(r.player.hp+9999.0).min(r.player.max_hp);
        r.player.energy=(r.player.energy+9999.0).min(energy);
    } else {
        r.player.hp=r.player.max_hp;
        r.player.energy=energy;
    }
    crate::half_here::sync(r);
    crate::engine::log(r,if r.simulation_revision.value_or(29.0)>=43.0 {
        "You eat Royal Honey."
    } else {"You eat Royal Honey, fully restoring health and energy."},"player");
    crate::events::actor_event(r,&r.player,"heal","Eat Royal Honey",None,None);
    true
}

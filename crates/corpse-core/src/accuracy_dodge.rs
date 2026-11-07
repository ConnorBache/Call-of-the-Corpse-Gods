//! Revision33 flavor specialization. Absolute birth-time values, never a global
//! hit-formula change, per-turn adjustment, saved-actor migration or disguise rule.
use crate::Actor;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug,Deserialize)]
pub struct Profile {
    pub kind:String,
    pub reference_depth:u32,
    pub role:String,
    pub accuracy:f64,
    pub dodge:f64,
}
pub fn natural_profiles()->&'static [Profile] {
    static ROWS:OnceLock<Vec<Profile>>=OnceLock::new();
    ROWS.get_or_init(||serde_json::from_str(include_str!("catalog/accuracy-dodge-v33.json"))
        .expect("revision33 natural Accuracy/Dodge profiles"))
}
pub fn witch_profiles()->&'static [Profile] {
    static ROWS:OnceLock<Vec<Profile>>=OnceLock::new();
    ROWS.get_or_init(||serde_json::from_str(include_str!("catalog/witch-accuracy-dodge-v33.json"))
        .expect("revision33 Witch newborn Accuracy/Dodge profiles"))
}
fn apply(actor:&mut Actor,kind:&str,revision:f64,rows:&[Profile]) {
    if revision<33. {return;}
    if let Some(row)=rows.iter().find(|row|row.kind==kind) {
        actor.stats.accuracy=row.accuracy;
        actor.stats.dodge=row.dodge;
    }
}
/// O(K), K=17 fixed profiles, once at construction after the frozen r31/r32
/// body overlay. No RNG, allocation after lazy catalog parsing, rewards or growth.
pub fn apply_new_actor_kind(actor:&mut Actor,kind:&str,revision:f64) {
    if revision>=33. {apply(actor,kind,revision,natural_profiles());}
}
/// O(B), B=2 fixed profiles, once after Witch's separate lighter birth profile.
/// It intentionally replaces the natural Accuracy/Dodge, never appearance data.
pub fn apply_witch_newborn(actor:&mut Actor,kind:&str,revision:f64) {
    if revision>=33. {apply(actor,kind,revision,witch_profiles());}
}

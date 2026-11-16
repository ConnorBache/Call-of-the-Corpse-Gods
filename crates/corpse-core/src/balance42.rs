//! Revision42 birth-only tuning. No saved actor retuning, runtime depth scaling,
//! player reward changes or RNG consumption. Data are offline native-fit results.
use crate::Actor;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BirthProfile {
    pub kind: String,
    pub offense: f64,
    pub dodge: f64,
    #[serde(default)] pub gas_passive: Option<f64>,
    #[serde(default)] pub gas_boost: Option<f64>,
    #[serde(default)] pub gas_shot: Option<f64>,
}

pub fn profiles() -> &'static [BirthProfile] {
    static ROWS: OnceLock<Vec<BirthProfile>> = OnceLock::new();
    ROWS.get_or_init(|| serde_json::from_str(include_str!("catalog/balance-v42.json"))
        .expect("validated revision42 birth profiles"))
}

/// O(R) fixed catalog lookup once per birth; O(1) field/map writes, bounded key allocation
/// after the one-time O(catalog bytes) parse. Existing serialized actors untouched.
/// Must run AFTER r32 physical + r33 accuracy/dodge overlays, not before them.
/// Explicit child context keys (e.g. witch:night-crawler) are final overrides,
/// not multipliers stacked on the natural profile.
pub fn apply_new_actor_kind(actor: &mut Actor, kind: &str, revision: f64) {
    if !matches!(revision, 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0) { return; }
    if let Some(row) = profiles().iter().find(|row| row.kind == kind) {
        actor.stats.offense = row.offense;
        actor.stats.dodge = row.dodge;
        for (key,value) in [("monsterGasPassiveVolume",row.gas_passive),
            ("monsterGasBoostVolume",row.gas_boost),("monsterGasShotVolume",row.gas_shot)] {
            if let Some(value)=value {actor.conditions.insert(key.into(),value);}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_revision_is_exact_noop() {
        let mut actor=Actor::default(); actor.stats.offense=123.; actor.stats.dodge=17.;
        for revision in [29.,30.,31.,32.,33.,34.,35.,36.,37.,38.,39.,40.,41.] {
            apply_new_actor_kind(&mut actor,"rat",revision);
            assert_eq!(actor.stats.offense,123.); assert_eq!(actor.stats.dodge,17.);
        }
    }
    #[test]
    fn catalog_has_unique_finite_rows() {
        let mut keys=std::collections::BTreeSet::new();
        assert!(!profiles().is_empty(),"r42 must not ship an uncalibrated empty catalog");
        for row in profiles() {
            assert!(keys.insert(&row.kind));
            assert!(row.offense.is_finite() && row.dodge.is_finite());
            for v in [row.gas_passive,row.gas_boost,row.gas_shot].into_iter().flatten(){assert!(v.is_finite()&&v>=0.);}
        }
    }
}

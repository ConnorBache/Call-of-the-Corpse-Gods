//! Build-time current-revision roster synchronization. Never rewrites authored prose.
use corpse_core::{flavor_placement, generation, ichor, ordinary_roster, population};
use serde_json::{Value,json};
use std::collections::{BTreeMap,BTreeSet};
fn main() {
    let path=std::env::args().nth(1).expect("manual.json path");
    let mut manual:Value=serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let order:Vec<String>=manual["entries"].as_array().unwrap().iter()
        .map(|e|e["id"].as_str().unwrap().into()).collect();
    let mut ordinary:BTreeMap<String,Vec<u32>>=BTreeMap::new();
    // Recompute, do not retain stale prior conditional introductions (corpses revive).
    let mut conditional:BTreeMap<String,BTreeSet<u32>>=order.iter()
        .map(|id|(id.clone(),BTreeSet::new())).collect();
    for depth in 1..=40 {
        let present:BTreeSet<_>=ordinary_roster::candidates_revision(depth,true,32.0).unwrap().iter()
            .flat_map(|c|c.kinds.iter().cloned()).collect();
        for id in &present { ordinary.entry(id.clone()).or_default().push(depth); }
        for setup in generation::setup_definitions() {
            if generation::setup_eligible_at_depth_revision(setup,depth,32.0).unwrap() {
                for id in &setup.kinds { conditional.entry(id.clone()).or_default().insert(depth); }
            }
        }
        conditional.entry(flavor_placement::water_watcher_kind(depth,None).unwrap()).or_default().insert(depth);
        conditional.entry(flavor_placement::corpse_kind(depth).into()).or_default().insert(depth);
        conditional.entry("bastidion".into()).or_default().insert(depth);
        if depth>=12 { conditional.entry("sphinx".into()).or_default().insert(depth); }
        if depth>=4 {
            let actor=&population::catalog().zillennia[depth as usize-1];
            let id=format!("zillennia:{}",actor.zillennia_quality.as_ref().unwrap());
            conditional.entry(id).or_default().insert(depth);
            for kind in ichor::eligible_fish_kinds(depth,32.0) {
                conditional.entry(kind.into()).or_default().insert(depth);
            }
        }
        let greater=present.contains("pale:Greater Birther") || conditional["pale:Greater Birther"].contains(&depth);
        if greater { for role in ["Biter","Breather","Spitter","Birther","Screamer"] {
            conditional.entry(format!("pale:{role}")).or_default().insert(depth);
        }}
        if greater || present.contains("pale:Birther") || conditional["pale:Birther"].contains(&depth) {
            conditional.entry("pale:Jackal".into()).or_default().insert(depth);
        }
        let ordinary_ids:Vec<_>=order.iter().filter(|id|present.contains(*id)).collect();
        let conditional_ids:Vec<_>=order.iter().filter(|id| !present.contains(*id)
            && conditional.get(*id).is_some_and(|d|d.contains(&depth))).collect();
        manual["depths"][depth as usize-1]=json!({"depth":depth,"ordinary":ordinary_ids,"conditional":conditional_ids});
    }
    for entry in manual["entries"].as_array_mut().unwrap() {
        let id=entry["id"].as_str().unwrap().to_string();
        entry["ordinaryDepths"]=json!(ordinary.get(&id).cloned().unwrap_or_default());
        entry["conditionalDepths"]=json!(conditional[&id]);
    }
    manual["source"]=json!("Authored creature prose preserved; native simulation revision32 ordinary, setup, corpse, birth and event eligibility; runtime birth stats");
    std::fs::write(path,serde_json::to_string_pretty(&manual).unwrap()+"\n").unwrap();
}

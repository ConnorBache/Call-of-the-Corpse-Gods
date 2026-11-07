//! Explicit headless debug workflow; never selects an ability on the user's behalf.
//! new SEED MODE FLOOR FILE | inspect FILE | choose FILE OFFER_ID
//! take/put/equip FILE ITEM_ID | enter FILE. Every mutation persists only FILE.
use corpse_core::{Command, Run, debug_floor as debug, temporal::Engine};
use serde_json::json;
fn report(r: &Run) -> serde_json::Value {
    let p=r.playtest.as_ref().unwrap();let s=debug::state(r).unwrap();
    let stats=corpse_core::combat::effective_stats(r,&r.player);
    json!({"debug":true,"seed":r.seed,"floor":r.depth,"mode":r.mode,"gods":r.gods,"level":r.level,
        "unspentBlessings":r.exp,"priorHearts":p.prior_hearts,"priorBlessings":p.prior_blessings,
        "consumedHearts":s.consumed,"remainingHearts":s.pending.len(),"entered":s.entered,
        "stats":stats,"maxHp":r.player.max_hp,"inventorySlots":corpse_core::inventory::occupancy(r),
        "offers":r.offers.iter().map(|id|json!({"id":id,"ability":corpse_core::progression::catalog().iter().find(|a|&a.id==id).map(|a|json!({"name":a.name,"description":a.description}))})).collect::<Vec<_>>(),
        "inventory":r.inventory,"stash":s.stash,"assumptions":debug::ASSUMPTIONS})
}
fn main() -> Result<(),String> {
    let a:Vec<_>=std::env::args().skip(1).collect();
    let arg=|n:usize|a.get(n).map(String::as_str).ok_or("Usage: debug_floor new SEED MODE FLOOR FILE | inspect FILE | choose FILE OFFER_ID | take/put/equip FILE ITEM_ID | enter FILE".to_string());
    let op=arg(0)?;
    let (mut e,path)=if op=="new" {
        if std::path::Path::new(arg(4)?).exists() {
            let existing=std::fs::read_to_string(arg(4)?).map_err(|e|e.to_string())?;
            let prior=Engine::load(&existing).map_err(|_|"Refusing to replace an existing non-debug or unreadable file")?;
            if debug::state(&prior.run).is_none() { return Err("Refusing to replace a normal save with DEBUG".into()); }
        }
        let run=debug::create_run(arg(1)?,arg(2)?,corpse_core::playtest::parse_floor(arg(3)?)?)?;
        (Engine::new(run).map_err(|e|e.to_string())?,arg(4)?)
    } else {
        let path=arg(1)?;
        let raw=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
        let engine=Engine::load(&raw).map_err(|e|e.to_string())?;
        if debug::state(&engine.run).is_none(){return Err("Refusing to mutate a non-DEBUG run".into());}
        (engine,path)
    };
    match op {
        "new"|"inspect"=>{},
        "choose"=>{let id=arg(2)?;if !e.run.offers.iter().any(|v|v==id){return Err("Choose one of the current real offer IDs; no state changed.".into());}e.dispatch(&Command::Choose{id:id.into()}).map_err(|e|e.to_string())?;},
        "take"|"put"|"equip"=>{let id=arg(2)?.parse().map_err(|_|"Expected numeric item ID")?;let ok=match op{"take"=>debug::take(&mut e.run,id),"put"=>debug::put(&mut e.run,id),_=>debug::equip(&mut e.run,id)}?;if !ok{return Err("No such eligible item; no state changed".into());}},
        "enter"=>debug::enter(&mut e.run)?,
        _=>return Err("Unknown debug command".into())
    }
    if op!="inspect" {
        let raw=e.save().map_err(|e|e.to_string())?;
        let temporary=format!("{path}.tmp");
        std::fs::write(&temporary,raw).map_err(|e|e.to_string())?;
        std::fs::rename(&temporary,path).map_err(|e|e.to_string())?;
    }
    println!("{}",serde_json::to_string_pretty(&report(&e.run)).unwrap());
    Ok(())
}

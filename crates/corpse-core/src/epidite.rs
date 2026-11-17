//! A fixed physical tower pulses once per thirty normal world turns.
//! Blessing is binary item state; actual heart rewards are doubled only once.
use crate::{Actor, Floor, GodbeastEvent, Item, Run};
use crate::engine::{self, EngineError};
pub const PERIOD: u32 = 30;
pub const DAMAGE: f64 = 100.0;
pub const PULSE_LABEL: &str = "Epidite cleansing light";
pub const PULSE_COLOR: &str = "#fff7d6";
pub fn countdown(e: &GodbeastEvent) -> u32 {
    PERIOD - (e.epidite_age.value_or(0.0) as u32 % PERIOD)
}
pub fn displayed_name(i: &Item) -> &str { &i.name }
pub fn solid_at(f: &Floor, x: f64, y: f64) -> bool {
    f.godbeast_event.as_ref().is_some_and(|e| e.kind == "Epidite" && e.x == x && e.y == y)
        && x >= 0. && y >= 0. && x < f.width() as f64 && y < f.height() as f64
        && f.tiles[y as usize * f.width() + x as usize] != "chasm"
}
/// O(1) fields plus name/description length. Whole stacks share one binary flag;
/// quantity and Borge provenance are untouched. Copying the bonus into a craft
/// naturally retains its doubled stats, but fixed craft abilities do not change.
pub fn bless(i: &mut Item) -> bool {
    if i.kind != "heart" || i.blessed.value_or(false) { return false; }
    // Legacy hearts may infer species from the name. Preserve that identity before
    // adding the visible adjective, so consumption and Adaptive Armor stay correct.
    if i.heart_creature_type.as_ref().is_none_or(String::is_empty) {
        if let Some(kind) = crate::inventory::heart_creature_type(i) { i.heart_creature_type = kind.into(); }
    }
    i.blessed = true.into();
    i.name = format!("Blessed {}", i.name);
    if let Some(b) = i.bonus.as_mut() {
        for value in [&mut b.offense, &mut b.defense, &mut b.accuracy, &mut b.dodge,
            &mut b.dr, &mut b.perception, &mut b.max_energy, &mut b.max_hp] {
            if let Some(n) = value.as_mut() { *n *= 2.; }
        }
    }
    for value in [&mut i.exp, &mut i.energy] { if let Some(n) = value.as_mut() { *n *= 2.; } }
    i.effect = "Blessed by an Epidite: twice this heart's ordinary stat, blessing and energy rewards. A heart can be blessed only once.".to_string().into();
    true
}
fn sees(r: &Run, source: &Actor, x: f64, y: f64) -> bool {
    let target = Actor { x, y, ..Default::default() };
    crate::vision::line_of_sight(r, source, &target)
}
/// Once per normal active/off-screen floor turn, O(1) except each thirtieth turn:
/// O((A+I)*L + A*damage-hooks), L<=max(width,height), with O(A+I) target scratch.
/// Damage/death hooks retain their own actor scans, rescue and item-placement costs.
/// LOS ignores lighting/perception/invisibility, but respects physical opaque cells.
/// Ground targets are snapshotted before creature damage: fresh death hearts wait
/// until the next flash. Inventory and carried hearts are not exposed to the light.
pub fn world_turn(r: &mut Run) -> Result<(), EngineError> {
    if r.simulation_revision.value_or(29.) < 34. || crate::dreamwalking::context_dream(r) { return Ok(()); }
    let Some(e) = r.floor().and_then(|f| f.godbeast_event.as_ref()).filter(|e| e.kind == "Epidite").cloned() else { return Ok(()); };
    if !r.floor().is_some_and(|f| solid_at(f, e.x, e.y)) { return Ok(()); }
    let age = (e.epidite_age.value_or(0.) as u32 + 1) % PERIOD;
    r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().epidite_age = (age as f64).into();
    if age != 0 { return Ok(()); }
    let source = Actor { id:-1., x:e.x, y:e.y, name:"Epidite".into(), ..Default::default() };
    let wave_action=crate::events::current_action();
    let exposed: std::collections::HashSet<u64> = r.ground.get(&r.depth.to_string()).into_iter().flat_map(|g|g.iter())
        .filter(|g|g.item.kind == "heart" && !g.item.blessed.value_or(false) && sees(r,&source,g.x,g.y))
        .map(|g|g.item.id.to_bits()).collect();
    let targets: Vec<f64> = std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|a|!a.dead.value_or(false) && a.hp > 0. && crate::dreamwalking::physical(r,a)
            && sees(r,&source,a.x,a.y)).map(|a|a.id).collect();
    for g in r.ground_at_mut(r.depth) { if exposed.contains(&g.item.id.to_bits()) { bless(&mut g.item); } }
    let blessed:Vec<_>=r.ground.get(&r.depth.to_string()).into_iter().flat_map(|g|g.iter()).filter(|g|exposed.contains(&g.item.id.to_bits())).map(|g|crate::events::EventPoint{x:g.x,y:g.y}).collect();
    if r.player.x >= 0. && crate::vision::visible(r,e.x as usize,e.y as usize) {
        engine::log(r,"The Epidite flashes with bright, cleansing light.","environment");
    }
    if crate::events::enabled(r) {
        // Exact physical LOS footprint, independent from which bodies survived
        // the packet; central terrain admission clips to observed cells.
        let f=r.floor().unwrap();let w=f.width();
        let cells:Vec<_>=(0..f.tiles.len()).filter_map(|i| {
            let (x,y)=((i%w)as f64,(i/w)as f64);
            sees(r,&source,x,y).then_some(crate::events::EventPoint{x,y})
        }).collect();
        let serial=crate::events::emission_serial();
        crate::events::at(r,"E77","wave-reveal-yellow",crate::events::Stage::Special,&cells);
        crate::events::decorate(serial,"E77","wave-reveal-yellow",(&source).into(),"#ffff40",None);
    } else { crate::events::emit(r, crate::events::PresentationEvent {
        kind:"burst".into(), from:Some(crate::events::EventPoint{x:e.x,y:e.y}),
        to:crate::events::EventPoint{x:e.x,y:e.y}, glyph:Some("✧".into()),
        color:Some(PULSE_COLOR.into()), label:Some(PULSE_LABEL.into()), depth:r.depth,
     animation: None, }); }
    crate::events::at(r,"E77","diamond-gold",crate::events::Stage::Special,&blessed);
    for id in targets {
        if engine::actor(r,id).is_some_and(|a|!a.dead.value_or(false) && a.hp > 0.) {
            engine::damage_environment(r,id,DAMAGE,None,"epidite",true)?;
        }
    }
    crate::events::checkpoint_for_action(r,wave_action);
    Ok(())
}

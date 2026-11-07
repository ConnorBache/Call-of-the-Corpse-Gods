//! Authored Mercuria lever and one-sap-per-run Veridian interaction.
use crate::{Floor, GroundItem, Item, Run};
pub fn activate_mercuria(f: &mut Floor) -> bool { activate_mercuria_revision(f,29.) }
pub fn activate_mercuria_revision(f: &mut Floor, revision: f64) -> bool {
    let w = f.width();
    let n = f.tiles.len();
    let Some(e) = f.godbeast_event.as_mut() else {
        return false;
    };
    if e.kind != "Mercuria" || e.activated.value_or(false)
        || (revision >= 51. && crate::revision51_stairs::tile(&f.tiles[e.y as usize*w+e.x as usize])) {
        return false;
    }
    e.activated = true.into();
    e.flood_age = 0.0.into();
    f.dirty = true;
    f.ichor.get_or_insert_with(|| vec![0.; n])[e.y as usize * w + e.x as usize] = 100.;
    true
}
pub fn lever_pending(r: &Run, x: f64, y: f64) -> bool {
    r.floor()
        .and_then(|f| f.godbeast_event.as_ref())
        .is_some_and(|e| {
            e.kind == "Mercuria"
                && !e.activated.value_or(false)
                && e.lever.as_ref().is_some_and(|p| p.x == x && p.y == y)
        })
}
/// No simulation RNG. Reward uses the authored seed/depth population stream.
pub fn pull_lever(r: &mut Run, x: f64, y: f64) -> Result<bool, String> {
    if !lever_pending(r, x, y) {
        return Ok(false);
    }
    let revision = r.simulation_revision.value_or(29.);
    if !activate_mercuria_revision(r.floor_mut().unwrap(),revision) {
        return Ok(false);
    }
    let e = r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();
    if !e.reward_created.value_or(false) {
        let mut rng =
            crate::population::population_random(&format!("{}:floor:{}:Mercuria", r.seed, r.depth));
        let depth = r.depth as u32;
        let item = crate::equipment::make_godbeast_permanent_for_run(r, depth, &mut rng)?;
        r.next_id += 1.;
        crate::item_placement::place_ground_item(
            r,
            r.depth,
            GroundItem {
                x: e.x,
                y: e.y,
                item,
                ..Default::default()
            },
            |r, d| crate::startup::ensure_floors(r, d as u32),
        )?;
        r.floor_mut()
            .unwrap()
            .godbeast_event
            .as_mut()
            .unwrap()
            .reward_created = true.into();
    }
    crate::engine::log(
        r,
        "You pull Mercuria’s lever. Ichor rises from the well around a gleaming permanent.",
        "player",
    );
    crate::events::actor_event(
        r,
        &r.player,
        "pulse",
        "Mercuria’s well",
        Some(crate::events::EventPoint { x: e.x, y: e.y }),
        None,
    );
    Ok(true)
}
#[derive(Debug)]
pub struct TapResult {
    pub success: bool,
    pub message: &'static str,
    pub sap: Option<Item>,
}
pub fn tap_veridian(r: &mut Run) -> TapResult {
    let valid = r
        .floors
        .iter()
        .find(|f| f.depth == r.depth)
        .and_then(|f| f.godbeast_event.as_ref())
        .is_some_and(|e| {
            e.kind == "Veridian" && (r.player.x - e.x).abs().max((r.player.y - e.y).abs()) == 1.
        });
    if !valid {
        return TapResult {
            success: false,
            message: if r.simulation_revision.value_or(29.)>=33. {"You must stand next to Verdania to use the tree tap."} else {"You must stand next to Veridian to use the tree tap."},
            sap: None,
        };
    }
    if r.veridian_sap_harvested.value_or(false)
        || r.floors.iter().any(|f| {
            f.godbeast_event
                .as_ref()
                .is_some_and(|e| e.tapped.value_or(false))
        })
    {
        return TapResult {
            success: false,
            message: if r.simulation_revision.value_or(29.)>=33. {"Verdania has already yielded its one gift this run."} else {"Veridian has already yielded its one gift this run."},
            sap: None,
        };
    }
    if crate::revision50::enabled(r) && crate::revision50_inventory::space_for(r,
        &Item { kind: "potion".into(), ..Default::default() }) < 1.0 {
        return TapResult { success: false,
            message: "Make room in your potion section before collecting Verdania Sap.", sap: None };
    }
    r.floor_mut()
        .unwrap()
        .godbeast_event
        .as_mut()
        .unwrap()
        .tapped = true.into();
    r.veridian_sap_harvested = true.into();
    let item = Item {
        id: r.next_id,
        name: if r.simulation_revision.value_or(29.)>=33. {"Verdania Sap"} else {"Veridian Sap"}.into(),
        glyph: "!".into(),
        kind: "potion".into(),
        effect: "Veridian Sap".to_string().into(),
        identified: true.into(),
        ..Default::default()
    };
    r.next_id += 1.;
    TapResult {
        success: true,
        message: if r.simulation_revision.value_or(29.)>=33. {"The tree tap breaks as you collect Verdania Sap."} else {"The tree tap breaks as you collect Veridian Sap."},
        sap: Some(item),
    }
}

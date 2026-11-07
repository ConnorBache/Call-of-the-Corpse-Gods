//! Frozen hazards.ts warning projection. Pure queries; no RNG, world writes or hidden-occupant warnings.
use corpse_core::{
    model::{Actor, Command, Run},
    movement::{self, Point},
    travel, vision,
};
fn condition(a: &Actor, k: &str) -> f64 {
    a.conditions.get(k).copied().unwrap_or(0.)
}
fn has(a: &Actor, id: &str) -> bool {
    a.abilities.iter().any(|s| s == id)
}
fn exposure(r: &Run, x: f64, y: f64, verb: &str) -> Option<String> {
    let f = r.floor()?;
    if x < 0.
        || y < 0.
        || x >= f.width() as f64
        || y >= f.height() as f64
        || !vision::visible(r, x as usize, y as usize)
    {
        return None;
    }
    let h = corpse_core::hazard_memory::observed_hazards_at_revision(
        f,
        y as usize * f.width() + x as usize,
        r.tick,
        r.simulation_revision.value_or(29.0),
    );
    let p = &r.player;
    if h.razorleaves.value_or(false) {
        let rounding = if corpse_core::combat::nearest_damage_enabled(r) { "rounded to the nearest whole number" } else { "rounded up" };
        return Some(format!("{verb} amid razorleaves? Ending your turn there costs 2% of your maximum health ({rounding})."));
    }
    if (h.fire.as_ref().is_some() || h.permanent_fire.value_or(false))
        && !has(p, "anavani-star-power")
        && !has(p, "enemy:beast:fire-immunity")
    {
        return Some(format!(
            "{verb} in the fire? It can burn you and cause damage."
        ));
    }
    let gas = h.gas.as_ref()?;
    let kind = gas.r#type.as_str();
    if has(p, "enemy:beast:gas-immunity")
        || (kind=="hallucinogenic" && has(p,"enemy:pale:gas-immunity"))
        || (matches!(kind, "caustic" | "breather")
            && (has(p, "grisha-entropy") || has(p, "enemy:pale:gas-immunity")))
    {
        return None;
    }
    match kind {
        "caustic" => Some(format!("{verb} in caustic gas? It can damage you.")),
        "breather" if r.simulation_revision.value_or(29.)>=50. => Some(format!("{verb} in Gasser gas? It can damage you.")),
        "breather" => Some(format!("{verb} in breather gas? It can damage you.")),
        "hallucinogenic" => Some(format!("{verb} in hallucinogenic gas? It can distort perceived creature identities.")),
        "paralysis" => Some(format!("{verb} in paralysis gas? It can paralyze you.")),
        "confusion" => Some(format!(
            "{verb} in confusion gas? It refreshes confusion to 8 turns while present."
        )),
        _ => None,
    }
}
fn current_exposure(r: &Run, x: f64, y: f64) -> Option<String> {
    let f = r.floor()?;
    if travel::flying(r, &r.player) || !vision::visible(r, x as usize, y as usize) {
        return None;
    }
    let at = y as usize * f.width() + x as usize;
    let code = *f.currents.as_ref()?.get(at)?;
    if code.fract() != 0. || !(1. ..=8.).contains(&code) {
        return None;
    }
    let dirs = [
        (0, 0),
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    let (dx, dy) = dirs[code as usize];
    let (xx, yy) = (x + dx as f64, y + dy as f64);
    if xx < 0.
        || yy < 0.
        || xx >= f.width() as f64
        || yy >= f.height() as f64
        || !vision::visible(r, xx as usize, yy as usize)
    {
        return None;
    }
    corpse_core::environment::current_destination_revision(f, at, r.simulation_revision.value_or(29.0))?;
    if r.actors_at(r.depth)
        .iter()
        .any(|a| !a.dead.value_or(false) && a.x == xx && a.y == yy && corpse_core::dreamwalking::same_plane(r, &r.player, a) && vision::visible_actor(r, a))
    {
        return None;
    }
    exposure(r, xx, yy, "Be swept by the current")
}
pub fn waiting(r: &Run) -> Option<String> {
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
    exposure(r, r.player.x, r.player.y, "Wait")
        .or_else(|| current_exposure(r, r.player.x, r.player.y))
}
fn fall_warning(r: &Run, x: f64, y: f64) -> Option<String> {
    if r.depth >= 40. {
        return None;
    }
    if let Some(f) = r.floors.iter().find(|f| f.depth == r.depth + 1.) {
        if x >= 0. && y >= 0. && x < f.width() as f64 && y < f.height() as f64 {
            let at = y as usize * f.width() + x as usize;
            if f.seen.get(at) == Some(&true) && f.tiles.get(at).is_some_and(|t| t == "water") {
                return None;
            }
        }
    }
    Some(if r.simulation_revision.value_or(29.) >= 43. { "Jump into Chasm." } else { "Jump into this chasm? Falling may cost 6.25% of your maximum health (rounded up) unless you land in deep water." }.into())
}
fn entry_allowed(r: &Run, x: f64, y: f64, phase: bool) -> bool {
    let f = r.floor().unwrap();
    let p = &r.player;
    if !travel::passable(r, p, x, y)
        || !corpse_core::ichor::movement_allowed(r, p, x, y)
        || !movement::diagonal_step_allowed(
            &f.tiles,
            f.width(),
            Point {
                x: p.x as i32,
                y: p.y as i32,
            },
            Point {
                x: x as i32,
                y: y as i32,
            },
            phase,
        )
    {
        return false;
    }
    if let Some(a) = r
        .actors_at(r.depth)
        .iter()
        .find(|a| !a.dead.value_or(false) && a.x == x && a.y == y && vision::visible_actor(r, a))
    {
        if a.state != "ally" || !travel::can_traverse(r, a, p.x, p.y) {
            return false;
        }
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        if e.kind == "Mercuria"
            && !e.activated.value_or(false)
            && e.lever.as_ref().is_some_and(|v| v.x == x && v.y == y)
        {
            return false;
        }
    }
    true
}
fn portal_arrival_warning(r: &Run, x: f64, y: f64, bonus: bool) -> Option<String> {
    if !corpse_core::portals::enabled(r) { return None; }
    let f=r.floor()?;
    let at=corpse_core::portals::cell(f,x,y)?;
    if !vision::visible(r,x as usize,y as usize) { return None; }
    let mate=corpse_core::portals::mate(f,at)?;
    let (xx,yy)=((mate%f.width())as f64,(mate/f.width())as f64);
    let current=vision::visible(r,xx as usize,yy as usize);
    let tile=if current {f.tiles.get(mate)} else if f.seen.get(mate)==Some(&true) {
        f.remembered_tiles.as_ref().and_then(|tiles|tiles.get(mate))
    } else {None};
    if tile.is_some_and(|t|t=="chasm") && !travel::flying(r,&r.player) {
        return fall_warning(r,xx,yy).map(|s|format!("Cross this portal? {s}"));
    }
    exposure(r,xx,yy,"Emerge from the portal").or_else(||if bonus{None}else{current_exposure(r,xx,yy)})
}
/// Inspect every known possible exit without sampling the simulation RNG.
fn non_euclidean_arrival_warning(r: &Run, x: f64, y: f64, bonus: bool) -> Option<String> {
    if !crate::non_euclidean_ui::enabled(r) { return None; }
    let f = r.floor()?; let at = corpse_core::portals::cell(f,x,y)?;
    let o = vision::observe_terrain(r)?;
    if !o.cells.get(at)?.current { return None; }
    let mut warnings = Vec::new();
    let from=corpse_core::portals::cell(f,r.player.x,r.player.y)?;
    let exits=crate::non_euclidean_ui::observed_walking_exits(r,&o,from,at);
    for exit in exits {
        let (xx,yy)=((exit%f.width()) as f64,(exit/f.width()) as f64);
        let cell=&o.cells[exit];
        let warning=if cell.tile.as_deref()==Some("chasm") && !travel::flying(r,&r.player) {
            fall_warning(r,xx,yy)
        } else { exposure(r,xx,yy,"Arrive in Non-Euclidean space")
            .or_else(||if bonus {None} else {current_exposure(r,xx,yy)}) };
        if let Some(warning)=warning { if !warnings.contains(&warning) { warnings.push(warning); } }
    }
    (!warnings.is_empty()).then(||format!("A possible Non-Euclidean exit is hazardous. {}",warnings.join(" ")))
}
pub fn intentional(r: &Run, command: &Command) -> Option<String> {
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
    let Command::Move { dx, dy } = *command else {
        return None;
    };
    let p = &r.player;
    let c = |k| condition(p, k);
    if r.status != "playing"
        || c("deathResolutionBlocked") > 0.
        || !r.offers.is_empty()
        || c("paralyzed") > 0.
        || c("forcedMove") > 0.
        || c("riptide") > 0.
        || has(p, "enemy:beast:rooted")
    {
        return None;
    }
    let f = r.floor()?;
    if !travel::habitat_valid(f, p, p.x, p.y)
        || dx.fract() != 0.
        || dy.fract() != 0.
        || dx.abs() > 1.
        || dy.abs() > 1.
        || (dx == 0. && dy == 0.)
    {
        return None;
    }
    let grabbed = c("grabbed") > 0.
        && std::iter::once(p)
            .chain(r.actors_at(r.depth).iter())
            .any(|a| {
                a.id == c("grabbedBy")
                    && !a.dead.value_or(false)
                    && a.hp > 0.
                    && (a.x - p.x).hypot(a.y - p.y) <= std::f64::consts::SQRT_2
            });
    let at = p.y as usize * f.width() + p.x as usize;
    let h = corpse_core::hazard_memory::observed_hazards_at_revision(f, at, r.tick, r.simulation_revision.value_or(29.0));
    let held = grabbed
        || ((h.mud.value_or(false)
            || h.roots.value_or(false)
            || (h.slime.value_or(false) && !has(p, "enemy:beast:slime-walker")))
            && c("stuck") == 0.
            && !travel::flying(r, p));
    if held {
        return None;
    }
    let (x, y) = (p.x + dx, p.y + dy);
    let phase = p.active.iter().any(|id| id == "etill-phase-through");
    let fast =
        r.phase.as_ref().is_none_or(|v| v != "normal") && crate::statuses_ui::is_hasted(r, p);
    let attack = r.actors_at(r.depth).iter().any(|a| {
        !a.dead.value_or(false)
            && a.state != "ally"
            && ((a.x == x && a.y == y) || (corpse_core::portals::enabled(r)
                && corpse_core::dreamwalking::same_plane(r,p,a)
                && corpse_core::portals::same_position(r,a,x,y)))
            && vision::visible_actor(r, a)
    });
    if c("confused") > 0. && !attack {
        for d in movement::DIRECTIONS {
            let (xx, yy) = (p.x + d.x as f64, p.y + d.y as f64);
            if xx < 0.
                || yy < 0.
                || xx >= f.width() as f64
                || yy >= f.height() as f64
                || !vision::visible(r, xx as usize, yy as usize)
                || !entry_allowed(r, xx, yy, phase)
            {
                continue;
            }
            if (!travel::flying(r, p)
                && f.tiles[yy as usize * f.width() + xx as usize] == "chasm"
                && fall_warning(r, xx, yy).is_some())
                || exposure(r, xx, yy, "Move").is_some()
                || (!fast && current_exposure(r, xx, yy).is_some())
                || portal_arrival_warning(r,xx,yy,fast).is_some()
                || non_euclidean_arrival_warning(r,xx,yy,fast).is_some()
            {
                return Some(
                    "Move while confused? Your random step could enter a nearby visible hazard."
                        .into(),
                );
            }
        }
        return None;
    }
    if x < 0. || y < 0. || x >= f.width() as f64 || y >= f.height() as f64 || attack {
        return None;
    }
    if !movement::diagonal_step_allowed(
        &f.tiles,
        f.width(),
        Point {
            x: p.x as i32,
            y: p.y as i32,
        },
        Point {
            x: x as i32,
            y: y as i32,
        },
        phase,
    ) {
        return None;
    }
    let at = y as usize * f.width() + x as usize;
    let visible = vision::visible(r, x as usize, y as usize);
    let tile = &f.tiles[at];
    if (matches!(tile.as_str(), "wall" | "tree")
        || (visible && f.grates.as_ref().is_some_and(|v| v.contains(&(at as f64)))))
        && !phase
    {
        return None;
    }
    if visible && !entry_allowed(r, x, y, phase) {
        return None;
    }
    if tile == "chasm" && !travel::flying(r, p) && (visible || f.seen[at]) {
        return fall_warning(r, x, y);
    }
    let mut bonus = fast;
    if !bonus && c("cameTurns") > 0. && c("cameFree") == 0. {
        bonus = r
            .actors_at(r.depth)
            .iter()
            .find(|a| {
                !a.dead.value_or(false) && a.id == c("cameTarget") && vision::visible_actor(r, a)
            })
            .is_some_and(|a| (x - a.x).hypot(y - a.y) < (p.x - a.x).hypot(p.y - a.y));
    }
    let local=exposure(r, x, y, "Step").or_else(|| {
        if bonus {None} else {current_exposure(r, x, y)}
    });
    let remote=portal_arrival_warning(r,x,y,bonus).or_else(||non_euclidean_arrival_warning(r,x,y,bonus));
    match(local,remote) {
        (Some(a),Some(b)) if a!=b=>Some(format!("{a} {b}")),
        (Some(a),_)=>Some(a),(_,b)=>b,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    #[test]
    fn frozen_warning_projection() {
        let specs: Vec<Value> =
            serde_json::from_str(include_str!("../assets/hazards-parity.json")).unwrap();
        for spec in specs {
            let mut raw: Value =
                serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json"))
                    .unwrap();
            let n = 79 * 29;
            let at = 14 * 79 + 12 + spec["offset"].as_u64().unwrap() as usize;
            raw["actors"]["1"] = json!([]);
            raw["phase"] = json!("normal");
            let f = &mut raw["floors"][0];
            f["environment"] = json!({"fire":vec![0;n],"caustic":vec![0;n],"breather":vec![0;n],"paralysis":vec![0;n],"gasType":vec![Value::Null;n],"gasVolume":vec![0;n]});
            match spec["hazard"].as_str().unwrap() {
                "fire" => f["environment"]["fire"][at] = json!(3),
                "razorleaves" => {
                    f["razorleaves"] = json!(vec![0; n]);
                    f["razorleaves"][at] = json!(1);
                }
                "chasm" => f["tiles"][at] = json!("chasm"),
                "none" => {}
                gas => {
                    f["environment"]["gasType"][at] = json!(gas);
                    f["environment"]["gasVolume"][at] = json!(1);
                }
            }
            if spec["current"] == true {
                f["tiles"][at] = json!("water");
                f["tiles"][at - 1] = json!("water");
                f["currents"] = json!(vec![0; n]);
                f["currents"][at - 1] = json!(3);
            }
            if let Some(id) = spec["ability"].as_str() {
                raw["player"]["abilities"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(id));
            }
            if let Some(c) = spec["condition"].as_str() {
                raw["player"]["conditions"][c] = json!(1);
            }
            let run: Run = serde_json::from_value(raw).unwrap();
            let before = serde_json::to_value(&run).unwrap();
            assert_eq!(
                serde_json::to_value(waiting(&run)).unwrap(),
                spec["wait"],
                "waiting {spec}"
            );
            assert_eq!(
                serde_json::to_value(intentional(&run, &Command::Move { dx: 1., dy: 0. })).unwrap(),
                spec["move"],
                "move {spec}"
            );
            assert_eq!(serde_json::to_value(&run).unwrap(), before);
        }
    }
    #[test]
    fn portal_ui_hazard_warning_uses_known_arrival_not_hidden_live_state() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=35.0.into();r.player.x=10.;r.player.y=10.;r.player.conditions.clear();
        r.player.active.clear();r.player.abilities.clear();r.offers.clear();r.actors.clear();
        r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),1.);
        let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();f.tiles.fill("floor".into());f.doors=corpse_core::Field::Missing;
        f.environment=corpse_core::Field::Missing;f.seen=vec![true;n];f.remembered_tiles=f.tiles.clone().into();
        f.portals=vec![corpse_core::portals_generation::PortalPair{a:10*w+11,b:10*w+45,color_index:0}].into();
        f.tiles[10*w+45]="chasm".into();
        assert!(intentional(&r,&Command::Move{dx:1.,dy:0.}).unwrap().contains("portal"));
        r.player.conditions.clear();r.player.x=3.;r.player.y=3.;let f=r.floor_mut().unwrap();
        for y in 0..f.height(){f.tiles[y*w+8]="wall".into();}
        f.seen[10*w+45]=false;
        let before=portal_arrival_warning(&r,11.,10.,false);assert!(before.is_none());
        r.floor_mut().unwrap().tiles[10*w+45]="floor".into();
        assert_eq!(portal_arrival_warning(&r,11.,10.,false),before);
    }

    #[test]
    fn non_euclidean_ui_hazard_warning_checks_both_known_exits_not_hidden_state() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.player.x=13.;r.player.y=9.;
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+12]="chasm".into();
        let before=r.clone();let warning=non_euclidean_arrival_warning(&r,13.,10.,false).unwrap();
        assert!(warning.contains("possible Non-Euclidean exit"));assert_eq!(r,before);
        // An unseen/unremembered destination cannot disclose its live hazard.
        let f=r.floor_mut().unwrap();for y in 1..f.height()-1 {f.tiles[y*w+12]="wall".into();}
        f.seen[10*w+11]=false;f.remembered_tiles=corpse_core::Field::Missing;
        let prior=non_euclidean_arrival_warning(&r,13.,10.,false);
        r.floor_mut().unwrap().tiles[10*w+11]="chasm".into();
        assert_eq!(non_euclidean_arrival_warning(&r,13.,10.,false),prior);
    }

    #[test]
    fn r37_speed_up_middle_pair_warning_uses_actual_arrival() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.simulation_revision=37.0.into();r.player.x=11.;r.player.y=10.;
        let f=r.floor_mut().unwrap();let w=f.width();
        f.speed_up=vec![corpse_core::non_euclidean_generation::NonEuclideanPocket {
            cells:vec![10*w+11,10*w+12],groups:vec![corpse_core::non_euclidean_generation::NonEuclideanGroup{sources:vec![10*w+11],destination:10*w+12}]
        }].into();f.non_euclidean=corpse_core::Field::Missing;
        f.permanent_fire=vec![(10*w+11) as f64].into();
        let before=r.clone();assert!(non_euclidean_arrival_warning(&r,12.,10.,false).is_none());assert_eq!(r,before);
    }

}

#[cfg(test)]
mod revision43_copy_tests {
    use super::*;
    #[test]
    fn chasm_prompt_is_short_in43_and_preserves_legacy_and_known_water_guards() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.depth=1.;r.floors.truncate(1);r.simulation_revision=43.0.into();
        assert_eq!(fall_warning(&r,2.,2.).as_deref(),Some("Jump into Chasm."));
        r.simulation_revision=42.0.into();assert!(fall_warning(&r,2.,2.).unwrap().contains("6.25%"));
        r.simulation_revision=43.0.into();let mut next=(*r.floors[0]).clone();next.depth=2.;let at=2*next.width()+2;next.tiles[at]="water".into();next.seen[at]=true;r.floors.push(next.into());
        assert_eq!(fall_warning(&r,2.,2.),None);
        r.depth=40.;assert_eq!(fall_warning(&r,2.,2.),None);
    }
}

#[cfg(test)]
mod creature_identity_warning_tests {
    #[test]
    fn revision50_creature_identity_gasser_warning_keeps_legacy_copy() {
        for revision in [46.,50.] {
            let mut r=crate::render::creature_identity_tests::room();r.simulation_revision=revision.into();
            let w=r.floor().unwrap().width();let at=12*w+11;
            corpse_core::environment::initialize(r.floor_mut().unwrap()).unwrap();
            corpse_core::environment::add_gas(r.floor_mut().unwrap(),at,"breather",100.,0.).unwrap();
            let message=super::exposure(&r,11.,12.,"Step").unwrap();
            assert!(message.contains(if revision>=50. {"Gasser gas"} else {"breather gas"}),"{message}");
        }
    }
}

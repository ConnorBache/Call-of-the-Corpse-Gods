//! Movement/wait warning projection. Pure queries; no RNG, world writes or hidden-occupant warnings.
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
fn exposure(r: &Run, x: f64, y: f64, verb: &str, entering: bool) -> Option<String> {
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
    // Reuse simulation occupancy at the player's own cell. O(1) with the fixed
    // gas catalog; no extra floor/visibility query or concentration threshold.
    if entering && corpse_core::portals::cell(f, p.x, p.y).is_some_and(|at| {
        f.environment.as_ref().is_some_and(|e| corpse_core::environment::gas_present(e, at))
    }) { return None; }
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
fn entry_exposure(r: &Run, x: f64, y: f64, verb: &str) -> Option<String> {
    exposure(r, x, y, verb, true)
}
/// At most two observed current steps: Slowed advances the normal world twice.
/// Visibility and occupancy are rechecked for each candidate; no hidden endpoint
/// or future actor motion is used to manufacture a warning.
fn current_exposure(r: &Run, mut x: f64, mut y: f64, entering: bool) -> Option<String> {
    let f = r.floor()?;
    if travel::flying(r, &r.player) { return None; }
    let dirs=[(0,0),(0,-1),(1,-1),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1)];
    for _ in 0..crate::travel_ui::action_world_ticks(r,entering) {
        if x<0.||y<0.||x>=f.width()as f64||y>=f.height()as f64
            || !vision::visible(r,x as usize,y as usize) {return None;}
        let at=y as usize*f.width()+x as usize;
        let code=*f.currents.as_ref()?.get(at)?;
        if code.fract()!=0.||!(1. ..=8.).contains(&code){return None;}
        let(dx,dy)=dirs[code as usize];let(xx,yy)=(x+dx as f64,y+dy as f64);
        if xx<0.||yy<0.||xx>=f.width()as f64||yy>=f.height()as f64
            || !vision::visible(r,xx as usize,yy as usize){return None;}
        corpse_core::environment::current_destination_revision(f,at,r.simulation_revision.value_or(29.))?;
        if r.actors_at(r.depth).iter().any(|a|!a.dead.value_or(false)&&a.x==xx&&a.y==yy
            && corpse_core::dreamwalking::same_plane(r,&r.player,a)&&vision::visible_actor(r,a)){return None;}
        let warning=if entering {entry_exposure(r,xx,yy,"Be swept by the current")}
            else {exposure(r,xx,yy,"Be swept by the current",false)};
        if warning.is_some(){return warning;}
        (x,y)=(xx,yy);
    }
    None
}

pub fn waiting(r: &Run) -> Option<String> {
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
    if crate::travel_ui::action_skips_world(r, false) { return None; }
    exposure(r, r.player.x, r.player.y, "Wait", false)
        .or_else(|| current_exposure(r, r.player.x, r.player.y, false))
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
    entry_exposure(r,xx,yy,"Emerge from the portal").or_else(||if bonus{None}else{current_exposure(r,xx,yy,true)})
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
        } else { entry_exposure(r,xx,yy,"Arrive in Non-Euclidean space")
            .or_else(||if bonus {None} else {current_exposure(r,xx,yy,true)}) };
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
    let fast = crate::travel_ui::action_skips_world(r, true);
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
                || entry_exposure(r, xx, yy, "Move").is_some()
                || (!fast && current_exposure(r, xx, yy, true).is_some())
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
    if !bonus && c("cameTurns") > 0. && c("cameFree") == 0.
        && (r.simulation_revision.value_or(29.) < 52. || c("slowed") <= 0.) {
        bonus = r
            .actors_at(r.depth)
            .iter()
            .find(|a| {
                !a.dead.value_or(false) && a.id == c("cameTarget") && vision::visible_actor(r, a)
            })
            .is_some_and(|a| (x - a.x).hypot(y - a.y) < (p.x - a.x).hypot(p.y - a.y));
    }
    let local=entry_exposure(r, x, y, "Step").or_else(|| {
        if bonus {None} else {current_exposure(r, x, y, true)}
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
            let message=super::entry_exposure(&r,11.,12.,"Step").unwrap();
            assert!(message.contains(if revision>=50. {"Gasser gas"} else {"breather gas"}),"{message}");
        }
    }
}

#[cfg(test)]
mod gas_entry_tests {
    use super::*;
    use corpse_core::{environment, Field, Floor};

    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.phase = "normal".to_string().into();
        r.player.x = 2.; r.player.y = 3.;
        r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        r.actors.clear(); r.offers.clear(); r.floors.truncate(1);
        // Native geometry is derived from tile-array length: fixtures must be
        // square (or the production 79x29 size), regardless of legacy metadata.
        let mut tiles = vec!["floor".to_string(); 9 * 9];
        for y in 0..9 { for x in 0..9 {
            if x == 0 || x == 8 || y == 0 || y == 8 { tiles[y * 9 + x] = "wall".into(); }
        }}
        r.floors[0] = Floor { depth: r.depth, width: 9.0.into(), height: 9.0.into(),
            remembered_tiles: tiles.clone().into(), tiles, lights: vec![100.; 81],
            seen: vec![true; 81], ..Default::default() }.into();
        assert_eq!((r.floor().unwrap().width(), r.floor().unwrap().height()), (9, 9));
        r
    }
    fn gas(r: &mut Run, x: usize, y: usize, kind: &str) {
        let f = r.floor_mut().unwrap(); let at = y * f.width() + x;
        assert!(x < f.width() && y < f.height(), "gas test coordinate ({x}, {y}) is outside {}x{}", f.width(), f.height());
        assert!(environment::add_gas(f, at, kind, 25., -1.).unwrap(), "gas test deposit at ({x}, {y}) was rejected");
    }
    fn step(r: &Run) -> Option<String> {
        intentional(r, &Command::Move { dx: 1., dy: 0. })
    }

    #[test]
    fn gas_entry_same_or_different_gas_suppresses_only_entry_and_never_mutates_run() {
        for from in environment::GAS_TYPES {
            for to in ["caustic", "breather", "paralysis", "confusion", "hallucinogenic"] {
                let mut r = room(); gas(&mut r, 2, 3, from); gas(&mut r, 3, 3, to);
                let before = r.clone();
                assert_eq!(step(&r), None, "{from} -> {to}");
                if matches!(from, "caustic" | "breather" | "paralysis" | "confusion" | "hallucinogenic") {
                    assert!(waiting(&r).unwrap().starts_with("Wait in "));
                }
                assert_eq!(r, before);
            }
        }
    }

    #[test]
    fn gas_entry_clean_air_warns_again_after_leaving_any_gas_in_normal_and_haste_phases() {
        for phase in ["normal", "bonus"] {
            let mut r = room(); r.phase = phase.to_string().into();
            r.player.conditions.insert("hasted".into(), 3.);
            gas(&mut r, 3, 3, "confusion");
            assert!(step(&r).unwrap().contains("Step in confusion gas"));
            gas(&mut r, 2, 3, "steam"); assert_eq!(step(&r), None);
            // Moving into clean air restores the next entry confirmation without
            // remembering or caching a previous gas warning.
            r.player.x = 4.;
            assert!(intentional(&r, &Command::Move { dx: -1., dy: 0. }).unwrap().contains("confusion gas"));
        }
    }

    #[test]
    fn gas_entry_uses_canonical_presence_and_positive_legacy_channels_without_new_cutoff() {
        let mut r = room(); gas(&mut r, 3, 3, "paralysis");
        let at = 3 * 9 + 2;
        let e = r.floor_mut().unwrap().environment.as_mut().unwrap();
        e.gas_type.as_mut().unwrap()[at] = Some("steam".into());
        e.gas_volume.as_mut().unwrap()[at] = 0.;
        assert_eq!(step(&r), None, "canonical presence does not invent a volume threshold");
        let e = r.floor_mut().unwrap().environment.as_mut().unwrap();
        e.gas_type.as_mut().unwrap()[at] = None;
        e.caustic[at] = 10.;
        assert!(step(&r).is_some(), "canonical absence overrides stale legacy channels");
        let e = r.floor_mut().unwrap().environment.as_mut().unwrap();
        e.gas_type = Field::Missing; e.gas_volume = Field::Missing;
        e.caustic[at] = 0.01;
        assert_eq!(step(&r), None, "any positive legacy presence qualifies");
        r.floor_mut().unwrap().environment.as_mut().unwrap().caustic[at] = 0.;
        assert!(step(&r).is_some(), "legacy clean air still warns");
    }

    #[test]
    fn gas_entry_fire_razorleaves_chasm_and_confused_non_gas_warnings_survive() {
        for (hazard, expected) in [("fire", "fire"), ("razorleaves", "razorleaves"), ("chasm", "Chasm")] {
            let mut r = room(); gas(&mut r, 2, 3, "steam"); gas(&mut r, 3, 3, "paralysis");
            let f = r.floor_mut().unwrap(); let at = 3 * f.width() + 3;
            match hazard {
                "fire" => f.environment.as_mut().unwrap().fire[at] = 8.,
                "razorleaves" => { f.razorleaves = vec![0.; 81].into(); f.razorleaves.as_mut().unwrap()[at] = 1.; }
                "chasm" => f.tiles[at] = "chasm".into(),
                _ => unreachable!(),
            }
            assert!(step(&r).unwrap().contains(expected));
            r.player.conditions.insert("confused".into(), 3.);
            assert!(step(&r).unwrap().contains("Move while confused"));
        }
        let mut r = room(); gas(&mut r, 2, 3, "steam"); gas(&mut r, 3, 3, "paralysis");
        r.player.conditions.insert("confused".into(), 3.);
        assert_eq!(step(&r), None);
    }

    #[test]
    fn gas_entry_hidden_or_remembered_target_gas_does_not_leak_into_warnings() {
        for remembered in [false, true] {
            let mut r = room();
            // R51+ full darkness follows the range formula, not an opacity
            // cutoff: 20 + (-100 / 5) + (0 / 5) = 0 tiles of optical range.
            r.player.stats.perception = 0.; r.inventory.clear();
            r.floor_mut().unwrap().lights.fill(-100.);
            let at = 3 * r.floor().unwrap().width() + 3;
            r.floor_mut().unwrap().seen[at] = remembered;
            assert_eq!(corpse_core::combat::effective_stats(&r, &r.player).perception, 0.);
            assert_eq!(vision::tile_light(&r, 3, 3), -100.);
            assert!(!vision::visible(&r, 3, 3));
            let before = step(&r);
            assert!(before.is_none());
            gas(&mut r, 3, 3, "paralysis");
            if remembered {
                r.floor_mut().unwrap().remembered_hazards = serde_json::from_value(serde_json::json!({
                    at.to_string(): {"gas": {"type": "paralysis", "concentration": 25}}
                })).unwrap();
            }
            assert!(!vision::visible(&r, 3, 3));
            assert_eq!(step(&r), before);
            // Positive control: it is the visibility gate, not a blocked move
            // or missing gas, that prevents the hidden information warning.
            r.floor_mut().unwrap().lights[at] = 100.;
            assert!(vision::visible(&r, 3, 3));
            assert!(step(&r).unwrap().contains("paralysis gas"));
            r.floor_mut().unwrap().lights[at] = -100.;
            gas(&mut r, 2, 3, "steam");
            let before = r.clone();
            assert!(!vision::visible(&r, 3, 3));
            assert_eq!(step(&r), None); assert_eq!(r, before);
        }
    }

    #[test]
    fn gas_entry_current_and_portal_arrivals_keep_other_hazards_and_waiting_warnings() {
        let mut r = room(); gas(&mut r, 4, 3, "paralysis");
        let f = r.floor_mut().unwrap(); let at = 3 * f.width() + 3;
        f.tiles[at] = "water".into(); f.tiles[at + 1] = "water".into();
        f.currents = vec![0.; 81].into(); f.currents.as_mut().unwrap()[at] = 3.;
        assert!(step(&r).unwrap().contains("Be swept by the current"));
        gas(&mut r, 2, 3, "steam"); assert_eq!(step(&r), None);
        r.floor_mut().unwrap().permanent_fire = vec![(at + 1) as f64].into();
        assert!(step(&r).unwrap().contains("fire"));
        r.floor_mut().unwrap().permanent_fire = Field::Missing;
        r.player.x = 3.; gas(&mut r, 3, 3, "steam");
        assert!(waiting(&r).unwrap().contains("Be swept by the current"));

        let mut r = room(); gas(&mut r, 6, 3, "paralysis");
        r.floor_mut().unwrap().portals = vec![corpse_core::portals_generation::PortalPair {
            a: 3 * 9 + 3, b: 3 * 9 + 6, color_index: 0,
        }].into();
        assert!(step(&r).unwrap().contains("Emerge from the portal"));
        gas(&mut r, 2, 3, "steam"); assert_eq!(step(&r), None);
        r.floor_mut().unwrap().permanent_fire = vec![(3 * 9 + 6) as f64].into();
        assert!(step(&r).unwrap().contains("fire"));
    }

    #[test]
    fn gas_entry_keyboard_mouse_and_queued_route_share_the_same_confirmation() {
        for in_gas in [false, true] {
            let mut r = room(); gas(&mut r, 3, 3, "paralysis");
            if in_gas { gas(&mut r, 2, 3, "steam"); }
            let keyboard = crate::key_command(sdl2::keyboard::Keycode::Right).unwrap();
            let crate::travel_ui::Start::Direct(mouse) = crate::travel_ui::start(&r, 3, 3) else {
                panic!("adjacent mouse click should remain a direct move");
            };
            let mut walk = crate::travel_ui::Travel::new(&r, 3, 3, vec![crate::travel_ui::Step {
                x: 3, y: 3, arrival_x: 3, arrival_y: 3, uncertain: false,
            }]);
            let queued = walk.next_command(&r).unwrap().unwrap();
            let before = r.clone();
            for command in [&keyboard, &mouse, &queued] {
                assert_eq!(intentional(&r, command).is_some(), !in_gas);
            }
            assert_eq!(r, before);
            if in_gas {
                r.player.x = 3.; r.player.hp -= 1.;
                assert!(walk.after_step(&r, false).unwrap_err().contains("harmed or incapacitated"));
            }
        }
    }

    fn exploration_room() -> Run {
        let raw: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/travel-ui-v48.json")).unwrap();
        let mut r: Run = serde_json::from_value(raw["cases"][0]["run"].clone()).unwrap();
        r.player.conditions.clear(); r.player.active.clear(); r.player.abilities.clear();
        r.actors.clear(); r.inventory.clear(); r.offers.clear();
        let f = r.floor_mut().unwrap(); f.tiles.fill("wall".into());
        for x in 1..=5 { f.tiles[3 * 7 + x] = "floor".into(); }
        f.seen.fill(true);
        for x in 3..=5 { f.seen[2 * 7 + x] = false; f.tiles[2 * 7 + x] = "floor".into(); }
        f.remembered_tiles = f.tiles.clone().into(); f.lights.fill(-100.);
        f.environment = Field::Missing; f.doors = Field::Missing;
        f.portals = Field::Missing; f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
        r
    }

    #[test]
    fn gas_entry_autoexplore_rechecks_occupancy_and_preserves_threat_and_hazard_stops() {
        for change in ["gas", "fire", "threat", "clean-air"] {
            let mut r = exploration_room(); let mut explore = crate::travel_ui::AutoExplore::default();
            explore.press(&r); let mut walk = explore.press(&r).unwrap();
            assert!(walk.exploring());
            r.floor_mut().unwrap().lights.fill(100.);
            gas(&mut r, 2, 3, "paralysis");
            if change != "clean-air" { gas(&mut r, 1, 3, "steam"); }
            if change == "fire" { r.floor_mut().unwrap().environment.as_mut().unwrap().fire[23] = 8.; }
            if change == "threat" {
                let mut a = r.player.clone(); a.id = 999.; a.x = 5.; a.state = "hunting".into();
                r.actors_at_mut(r.depth).push(a);
                assert!(walk.next_command(&r).unwrap().is_none());
                continue;
            }
            let command = walk.next_command(&r).unwrap().unwrap();
            let before = r.clone(); let warning = intentional(&r, &command);
            assert_eq!(warning.is_some(), change != "gas", "{change}");
            if change == "fire" { assert!(warning.as_ref().unwrap().contains("fire")); }
            if warning.is_some() { assert!(!walk.after_step(&r, true).unwrap()); }
            assert_eq!(r, before);
        }
    }

    #[test]
    fn gas_entry_non_euclidean_and_dream_projection_use_the_current_player_plane() {
        let mut r = crate::non_euclidean_ui::tests::scene(); r.player.x = 13.; r.player.y = 9.;
        gas(&mut r, 11, 10, "paralysis");
        assert!(non_euclidean_arrival_warning(&r, 13., 10., false).is_some());
        gas(&mut r, 13, 9, "steam");
        assert!(non_euclidean_arrival_warning(&r, 13., 10., false).is_none());
        let w = r.floor().unwrap().width();
        r.floor_mut().unwrap().permanent_fire = vec![(10 * w + 11) as f64].into();
        assert!(non_euclidean_arrival_warning(&r, 13., 10., false).unwrap().contains("fire"));

        let mut r = room(); gas(&mut r, 2, 3, "steam");
        let mut dream = room(); gas(&mut dream, 3, 3, "paralysis");
        r.dreamwalking = corpse_core::dreamwalking::State { active: true, depth: r.depth,
            floors: dream.floors.clone(), ..Default::default() }.into();
        assert!(step(&r).is_some(), "waking gas cannot suppress a dream-plane warning");
        gas(&mut dream, 2, 3, "steam");
        r.dreamwalking.as_mut().unwrap().floors = dream.floors;
        let before = r.clone(); assert_eq!(step(&r), None); assert_eq!(r, before);
    }
}

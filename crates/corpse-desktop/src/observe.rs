//! Knowledge-filtered map adapter reading canonical Run state.
//! It emits visible evidence / last-seen snapshots, never hidden live actors.
use crate::render::{Cell, Rgb, View};
#[path = "flesh_palette.rs"]
mod flesh_palette;
#[path = "creature_rank_ui.rs"]
mod creature_rank_ui;
#[path = "timeweave_ui.rs"]
mod timeweave_ui;
#[path="ground_compiled.rs"]
mod ground_compiled;
use corpse_core::model::Run;
fn hex(s: &str) -> Rgb {
    corpse_core::timing::count(corpse_core::timing::Count::HexParse,1);
    if s.len() == 7 && s.starts_with('#') {
        let value = u32::from_str_radix(&s[1..], 16).unwrap_or(0xffffff);
        [(value >> 16) as u8, (value >> 8) as u8, value as u8]
    } else {
        [255, 248, 232]
    }
}
fn terrain(tile: &str) -> (char, Rgb, Rgb) {
    match tile {
        "wall" => ('#', [119, 121, 135], [23, 25, 35]),
        "water" => ('~', [24, 125, 197], [8, 44, 77]),
        "shallow-water" => ('~', [24, 125, 197], [18, 69, 106]),
        "grass" => ('"', [120, 199, 107], [13, 38, 28]),
        "tree" => ('♣', [70, 169, 120], [7, 25, 22]),
        "chasm" => ('⛶', [101, 85, 141], [0, 1, 2]),
        "up" => ('<', [255, 248, 232], [54, 36, 81]),
        "down" => ('>', [255, 251, 232], [54, 36, 81]),
        "pit" => ('○', [196, 132, 255], [8, 5, 27]),
        _ => ('.', [81, 91, 107], [21, 25, 35]),
    }
}

// Speed-up is a distinct simulation tile, but shares ordinary floor material.
// The common spatial marker is applied separately to known group endpoints.
pub(super) fn visual_tile(tile: &str) -> &str {
    if tile == "speed-up" { "floor" } else { tile }
}

fn ground(o: &corpse_core::vision::ObservedCell) -> (String, Rgb, Rgb, bool, bool) {
    let tile = visual_tile(o.tile.as_deref().unwrap_or("floor"));
    let (g, mut fg, mut bg) = terrain(tile);
    let mut glyph = g.to_string();
    let mut fixture = false;
    let stone = o
        .substrate
        .as_ref()
        .is_none_or(|s| s == "tiled" || s == "rock");
    if tile == "floor" {
        if let Some(s) = o.substrate.as_ref() {
            let colors = match s.as_str() {
                "earth" => Some(([189, 145, 103], [39, 24, 26])),
                "sand" => Some(([213, 189, 122], [49, 38, 26])),
                "ash" => Some(([175, 164, 185], [35, 30, 43])),
                "soot" => Some(([110, 102, 129], [14, 11, 22])),
                _ => None,
            };
            if let Some((f, b)) = colors {
                fg = f;
                bg = b;
            }
        }
    }
    if let Some(open) = o.door_open {
        glyph = if open { "/" } else { "+" }.into();
        fg = [226, 167, 100];
        bg = [62, 34, 26];
        fixture = true;
    } else if o.grate {
        glyph = "#".into();
        fg = [216, 226, 236];
        bg = [66, 72, 78];
        fixture = true;
    } else if o.bridge {
        glyph = "=".into();
        fg = [211, 178, 132];
        bg = [53, 36, 28];
        fixture = true;
    }
    if o.snow && !fixture && matches!(tile, "floor" | "grass") {
        fg = [242, 251, 255];
        bg = [65, 87, 112];
    }
    (glyph, fg, bg, fixture, stone)
}
#[cfg(test)]
fn material_settings(
    profile: &crate::appearance_profile::Profile,
    o: &corpse_core::vision::ObservedCell,
) -> serde_json::Value {
    let tile = visual_tile(o.tile.as_deref().unwrap_or("floor"));
    // New terrain keeps the existing editable floor material, with a distinct
    // default palette. No new appearance keys change historical profiles.
    if !matches!(tile, "up" | "down") && o.door_open.is_none() && !o.grate && !o.bridge && (o.ice || o.puddles || o.hellfire) {
        let mut material = profile.material("floor", None).unwrap_or_default();
        let (fg,bg,glyph) = if o.hellfire { ("#e33824","#090307",".") }
            else if o.ice { ("#a8dce9","#183c52","_") }
            else { ("#759ba5","#1a303b","~") };
        material["color"] = fg.into(); material["background"] = bg.into();
        material["edgeColor"] = fg.into(); material["glyph"] = glyph.into();
        material["animationSpeed"] = 0.0.into();
        return material;
    }
    let (key, parent) = if let Some(open) = o.door_open {
        (
            if open {
                "fixture:door-open"
            } else {
                "fixture:door"
            }
            .into(),
            Some("fixture:door"),
        )
    } else if o.grate {
        ("fixture:grate".into(), None)
    } else if o.bridge {
        ("fixture:bridge".into(), None)
    } else if o.snow && matches!(tile, "floor" | "grass") {
        ("fixture:snow".into(), None)
    } else if tile == "floor" {
        (
            o.substrate
                .as_ref()
                .map(|s| format!("ground:{s}"))
                .unwrap_or_else(|| "floor".into()),
            Some("floor"),
        )
    } else {
        (tile.to_string(), None)
    };
    profile.material(&key, parent).unwrap_or_else(|| {
        profile
            .value(&format!("terrain.{tile}"))
            .cloned()
            .unwrap_or_default()
    })
}
#[cfg(test)]
fn material_number(
    profile: &crate::appearance_profile::Profile,
    o: &corpse_core::vision::ObservedCell,
    key: &str,
) -> f64 {
    material_settings(profile, o)[key].as_f64().unwrap_or(1.)
}
#[cfg(test)]
fn ground_profile(
    profile: &crate::appearance_profile::Profile,
    o: &corpse_core::vision::ObservedCell,
    flesh: bool,
) -> (String, Rgb, Rgb, bool, bool) {
    let (mut glyph, mut fg, mut bg, fixture, stone) = ground(o);
    let m = displayed_material_settings(profile, o, flesh);
    if let Some(g) = m["glyph"].as_str().filter(|g| !g.is_empty()) {
        glyph = g.into();
    }
    if let Some(c) = m["color"].as_str() {
        fg = hex(c);
    }
    if let Some(c) = m["background"].as_str() {
        bg = hex(c);
    }
    if !fixture && matches!(o.tile.as_deref(),Some("floor"|"grass")) {
        let defaults=material_settings(crate::appearance_profile::Profile::defaults(),o);
        let original=material_settings(profile,o);
        let palette=match o.biome.as_deref() {
            Some("saidribee-nest")=>Some(([241,194,88],[66,43,19])),
            Some("flower-grove")=>Some(([149,203,114],[18,46,32])),
            Some("mountainous")=>Some(([162,177,193],[27,37,52])),
            Some("corn-dimension")=>Some(([226,197,246],[58,24,77])),
            _=>None,
        };
        if let Some((f,b))=palette.filter(|_| !o.ice && !o.snow && !o.puddles && !o.hellfire) {
            if original["color"]==defaults["color"] {fg=f;}
            if original["background"]==defaults["background"] {bg=b;}
        }
        if o.flowers && !o.snow && !o.ice && !o.puddles && !o.hellfire {
            glyph="*".into();
            fg=match o.index%3 {0=>[244,164,208],1=>[253,220,112],_=>[193,178,248]};
        }
    }
    if o.entrance {
        return ("Ω".into(), [76, 170, 255], [8, 25, 82], true, stone);
    }
    (glyph, fg, bg, fixture, stone)
}
#[cfg(test)]
fn displayed_material_settings(
    profile: &crate::appearance_profile::Profile,
    o: &corpse_core::vision::ObservedCell,
    flesh: bool,
) -> serde_json::Value {
    let mut material = material_settings(profile, o);
    if flesh {
        let defaults = material_settings(crate::appearance_profile::Profile::defaults(), o);
        let fixture = o.door_open.is_some() || o.grate || o.bridge || o.snow || o.ice || o.puddles || o.hellfire;
        flesh_palette::material(&mut material, &defaults, o.tile.as_deref().unwrap_or("floor"),
            o.substrate.as_deref(), fixture);
    }
    material
}

fn gas_color(kind: &str) -> Rgb {
    match kind {
        "bloodMist" => [185, 30, 60],
        "caustic" => [86, 255, 55],
        "breather" => [35, 213, 157],
        "paralysis" => [161, 62, 255],
        "confusion" => [255, 47, 203],
        "hallucinogenic" => [238, 108, 255],
        "flammable" => [255, 156, 69],
        "explosive" => [255, 214, 0],
        "steam" => [181, 239, 255],
        "hydrogen" => [255, 243, 109],
        _ => [255, 255, 255],
    }
}

fn knowledge(s: &str) -> crate::color::Knowledge {
    match s {
        "direct" => crate::color::Knowledge::Direct,
        "telepathic" => crate::color::Knowledge::Telepathic,
        "remembered" => crate::color::Knowledge::Remembered,
        _ => crate::color::Knowledge::Layout,
    }
}
fn item_color(kind: &str) -> Rgb {
    match kind {
        "potion" => [182, 77, 255],
        "heart" => [255, 47, 120],
        "food" => [255, 210, 83],
        "ring" => [255, 218, 53],
        _ => [255, 248, 232],
    }
}
fn item_glyph(item: &corpse_core::model::Item) -> String {
    match item.kind.as_str() {
        "ring" => "o".into(),
        "amulet" => "?".into(),
        "heart" => ";".into(),
        "weapon" => {
            if item
                .ranged_weapon
                .as_ref()
                .is_some_and(|v| v == "bow" || v == "composite-bow")
            {
                ")".into()
            } else {
                "{".into()
            }
        }
        "ammo" if item.effect.as_ref().is_some_and(|e| e == "arrows") => "→".into(),
        _ => item.glyph.clone(),
    }
}

fn actor_cell(
    run: &Run,
    a: &corpse_core::model::Actor,
    width: usize,
    backgrounds: &[Rgb],
    player: bool,
) -> Cell {
    actor_cell_profile(
        crate::appearance_profile::Profile::defaults(),
        run,
        a,
        width,
        backgrounds,
        player,
    )
}
fn actor_cell_profile(
    profile: &crate::appearance_profile::Profile,
    run: &Run,
    a: &corpse_core::model::Actor,
    width: usize,
    backgrounds: &[Rgb],
    player: bool,
) -> Cell {
    let presented=if player&&run.simulation_revision.value_or(29.)>=43. {None}else{crate::creature_presentation::project(run,a)};
    let mut cell=actor_cell_presented(profile,run,a,width,backgrounds,player,presented.as_ref());
    if !player && presented.is_some() && crate::hallucination_ui::affects(run,a) {
        let variants=std::array::from_fn(|phase| {
            let shown=crate::creature_presentation::project_kind(run,a,crate::hallucination_ui::kind(run,a,phase)).unwrap();
            let mut c=actor_cell_presented(profile,run,a,width,backgrounds,false,Some(&shown));
            crate::hallucination_ui::question(&mut c.label);crate::hallucination_ui::question(&mut c.superscript);if c.superscript.is_empty(){c.superscript.push('?');} c
        });
        cell.hallucination=Some(std::sync::Arc::new(variants));
        crate::hallucination_ui::question(&mut cell.label);crate::hallucination_ui::question(&mut cell.superscript);if cell.superscript.is_empty(){cell.superscript.push('?');}
    }
    cell
}
fn actor_cell_presented(profile:&crate::appearance_profile::Profile,run:&Run,a:&corpse_core::Actor,width:usize,backgrounds:&[Rgb],player:bool,presented:Option<&corpse_core::Actor>)->Cell {
    let invisible_player = player && corpse_core::engine::condition(a, "invisible") > 0.;
    let nightmare = corpse_core::dreamwalking::is_nightmare(a);
    let body = corpse_core::dreamwalking::is_body(a);
    let dead = a.dead.value_or(false);
    let x = a.x as usize;
    let y = a.y as usize;
    let i = y * width + x;
    let telepathic = nightmare || (!player && !corpse_core::vision::directly_visible_actor(run, a));
    let marked = !player && corpse_core::borge::is_marked_target(run,a);
    let a=presented.unwrap_or(a);
    let kind = a.creature_kind.as_ref().map(String::as_str).unwrap_or("");
    let mut glyph = a.glyph.clone();
    // Death retains the saved actor identity, including individual color variants.
    // The separate corpse X carries death status; no generic corpse palette.
    let mut fg = hex(&a.color);
    if run.simulation_revision.value_or(29.) < 50. && kind == "blorb" && ["o", "B", "¤", "a"].contains(&glyph.as_str()) {
        glyph = "a".into();
    }
    if corpse_core::monster_vampire::flying(a) {
        glyph = "^".into();
        fg = [227, 77, 85];
    }
    let mut arrow = String::new();
    if kind == "minotaur" || a.abilities.iter().any(|a| a == "enemy:beast:minotaur") {
        glyph = "M".into();
        let dx = a.conditions.get("monsterChargeDx").copied().unwrap_or(0.) as i32;
        let dy = a.conditions.get("monsterChargeDy").copied().unwrap_or(0.) as i32;
        arrow = match (dx, dy) {
            (-1, -1) => "↖",
            (0, -1) => "↑",
            (1, -1) => "↗",
            (-1, 0) => "←",
            (1, 0) => "→",
            (-1, 1) => "↙",
            (0, 1) => "↓",
            (1, 1) => "↘",
            _ => "",
        }
        .into();
    }
    let half = a.conditions.get("halfHereClone") == Some(&1.);
    let superscript = if dead || body || player {
        String::new()
    } else if half {
        "h".into()
    } else if a.state == "ally" {
        a.order.value_or("a".into())
    } else {
        String::new()
    };
    let (superscript, subscript) = creature_rank_ui::markers(a, run.simulation_revision.value_or(29.), dead || body, player, superscript);
    if player {
        glyph = "@".into();
        let brightness =
            0.56 + 0.44 * ((corpse_core::vision::light(run, a).clamp(-100., 100.) + 100.) / 200.);
        let ink = if invisible_player { [135., 183., 243.] } else { [255., 243., 189.] };
        fg = ink.map(|c| (c * brightness).round() as u8);
    } else if telepathic {
        fg = [135, 183, 243];
    }
    let key = if player {
        "player".to_string()
    } else if let Some(q) = a.zillennia_quality.as_ref() {
        format!("zillennia:{q}")
    } else if kind.is_empty() {
        let legacy=corpse_core::revision50_creature_identity::legacy_name(corpse_core::enemy_traits::base_name(&a.name));
        if let Some(role)=legacy.strip_prefix("Greater Pale ") {format!("pale:Greater {role}")}
        else if let Some(role)=legacy.strip_prefix("Pale ") {format!("pale:{role}")}
        else {a.name.clone()}
    } else {
        kind.into()
    };
    let entity = crate::entity_profile::resolve(
        profile,
        if player {
            "objects"
        } else {
            "creatures"
        },
        &key,
        None,
        &glyph,
        fg,
    );
    // Player identity is fixed presentation, including the sleeping physical
    // endpoint. Keep saved species/profile data and all non-player glyphs intact.
    glyph = if player || body { "@".into() } else { entity.glyph };
    // Invisibility is a player status cue, independent of custom identity ink.
    // Retain the same illumination response and all other appearance settings.
    if !invisible_player { fg = entity.color; }
    let identity_outline = (!player && !telepathic && kind == "dire-bear"
        && run.simulation_revision.value_or(29.) >= 50. && glyph == "B" && fg == [0,0,0]
        && entity.background.is_none()
        && profile.value("entities.creatures.dire-bear.inherited").and_then(serde_json::Value::as_bool) == Some(true)
        && ["brightness","saturation"].iter().all(|field|profile.number(&format!("entities.creatures.dire-bear.{field}"))==1.)
        && profile.value("entities.defaults") == crate::appearance_profile::Profile::defaults().value("entities.defaults")
        && profile.value("global") == crate::appearance_profile::Profile::defaults().value("global"))
        .then_some([190,190,190]);
    let bg = backgrounds.get(i).copied().unwrap_or([5, 4, 12]);
    Cell {
        dream_entity: corpse_core::dreamwalking::dream_actor(run, a),
        x,
        y,
        glyph,
        label: { let label = if body { "Your sleeping body".into() } else if corpse_core::monster_vampire::flying(a) {
            "Vampire Bat".into()
        } else {
            a.name.clone()
        }; if marked { format!("{label} · Borge-marked") } else { label } },
        emphasis: entity.emphasis,
        pulse_speed: entity.speed,
        fire_flicker: entity.fire_flicker && !dead && !telepathic && !player,
        actor_id: Some(a.id),
        controllable_ally: !dead && a.state == "ally" && a.perfect.value_or(false)
            && corpse_core::dreamwalking::same_plane(run, &run.player, a),
        fg,
        bg,
        entity_ink: Some(crate::render::EntityInk {
            identity_outline,
            color: fg,
            player,
            telepathic,
            important: !dead,
            remembered: false,
            detected: false,
        }),
        // Actors inherit the lit terrain. Telepathy changes the ink, not
        // the tile; only an explicitly authored background overrides it.
        background_overlay: entity.background.map(|bg| (bg, 1.)),
        heartless: !body && if dead { corpse_core::corpses::heart_harvested(a) } else { a.heartless.value_or(false) },
        corpse: dead,
        superscript,
        subscript,
        arrow,
        ..Cell::default()
    }
}

/// Read the waking floor even in a detached dream view. O(A + F) for a body
/// endpoint lookup and floor selection, performed once per observation, not cell.
fn waking_epidite(run: &Run) -> Option<&corpse_core::GodbeastEvent> {
    let floor = corpse_core::dreamwalking::body(run)
        .and_then(|body| corpse_core::dreamwalking::floor_for(run, body))
        .or_else(|| run.floor())?;
    let event = floor.godbeast_event.as_ref().filter(|event| event.kind == "Epidite")?;
    corpse_core::epidite::solid_at(floor, event.x, event.y).then_some(event)
}
/// The countdown is live evidence only at a directly observed tower, not a value
/// copied into fog memory. Inspector callers have already admitted this cell.
pub fn observed_epidite_countdown(run: &Run, o: &corpse_core::vision::ObservedCell, width: usize) -> Option<u32> {
    if !o.direct || width == 0 { return None; }
    let event = waking_epidite(run)?;
    if event.x != (o.index % width) as f64 || event.y != (o.index / width) as f64 { return None; }
    Some(corpse_core::epidite::countdown(event))
}

// O(C * visibility_cost + A + I); visible() owns ray/lighting complexity.
pub fn observe(run: &Run) -> View {
    observe_with_profile(run, crate::appearance_profile::Profile::defaults())
}
pub fn observe_with_profile(run: &Run, profile: &crate::appearance_profile::Profile) -> View {
    let _timing = corpse_core::timing::span("desktop.observe");
    let view = { let _s=corpse_core::timing::span("desktop.observe.dream_projection"); crate::dream_ui::observation_run(run) };
    observe_palette(view.as_ref(), profile, true)
}
// Explicit historical palette retained for source-color differential tests, not live gameplay.
/// Adds one lazy immutable visibility projection for actor batches: O(C) setup
/// once, then cached light/geometry per actor. C includes exact cache guards and
/// dirty-field costs; see corpse_core::vision::VisibilityProjection.
fn observe_palette(run: &Run, profile: &crate::appearance_profile::Profile, flesh: bool) -> View {
    let actor_visibility = std::cell::OnceCell::new();
    let visibility = || actor_visibility.get_or_init(|| corpse_core::vision::VisibilityProjection::new(run));
    let mut terrain_snapshot = None;
    let mut terrain_count = 0;
    let mut ground_entities = Vec::new();
    let mut cells = vec![];
    let mut width = 79;
    let mut height = 29;
    if let Some(observation) = corpse_core::vision::observe_terrain(run) {
        width = observation.width;
        height = observation.height;
        let seed = crate::color::cosmetic_seed(&run.seed, run.depth as u32);
        let (mut terrain, backgrounds, light) = terrain_frame_palette(&observation, profile, seed, 0., flesh);
        let topology_span=corpse_core::timing::span("desktop.observe.topology");
        if let Some(event) = waking_epidite(run) {
            let at = event.y as usize * width + event.x as usize;
            if observation.cells.get(at).is_some_and(|o| o.direct) {
                if let Some(cell) = terrain.iter_mut().find(|c| c.x == event.x as usize && c.y == event.y as usize) {
                    let remaining = corpse_core::epidite::countdown(event);
                    if remaining <= 9 { cell.subscript = remaining.to_string(); }
                }
            }
        }
        crate::portal_ui::terrain(run, &observation, &mut terrain);
        crate::non_euclidean_ui::terrain(run, &observation, &mut terrain);
        crate::cogworks_ui::terrain(run, &observation, &mut terrain);
        // One indexed lookup per already observed terrain cell; no trail/world scan.
        for cell in &mut terrain {
            cell.timeweave = timeweave_ui::direction(run, width, height, cell.x, cell.y, cell.current, cell.direct);
        }
        drop(topology_span);
        terrain_count = terrain.len();
        terrain_snapshot = {let _s=corpse_core::timing::span("desktop.observe.snapshot_clone"); Some(std::sync::Arc::new(observation.clone()))};
        let _entity_span=corpse_core::timing::span("desktop.observe.entity_projection");
        let mut material_backgrounds = vec![[0, 0, 0]; width * height];
        for cell in &terrain {
            if let Some(bg) = cell.material_background {
                material_backgrounds[cell.y * width + cell.x] = bg;
            }
        }
        cells.extend(terrain);
        // Ground hearts are below corpses; other items keep their priority.
        let mut observed_actors: Vec<_> = run
            .actors_at(run.depth)
            .iter()
            .filter(|a| visibility().visible_actor(a))
            .collect();
        // Stable layering places nightmares above overlapping normal-plane actors.
        observed_actors.sort_by_key(|a| corpse_core::dreamwalking::is_nightmare(a));
        let active: std::collections::HashSet<_> = observed_actors
            .iter()
            .filter(|a| !a.dead.value_or(false))
            .map(|a| a.y as usize * width + a.x as usize)
            .chain(std::iter::once(
                run.player.y as usize * width + run.player.x as usize,
            ))
            .collect();
        let fires: std::collections::HashSet<_> = observation
            .cells
            .iter()
            .filter(|o| {
                o.hellfire || o.hazards.as_ref().is_some_and(|h| {
                    h.fire.as_ref().is_some_and(|f| f != "embers")
                        || h.permanent_fire.value_or(false)
                })
            })
            .map(|o| o.index)
            .collect();
        // Collect direct evidence independently of map z-order, retaining stacked items.
        use crate::render::enemy_list::{GroundEntity,GroundKind};
        let distance=|x:usize,y:usize| (x as f64-run.player.x).abs().max((y as f64-run.player.y).abs());
        for o in observation.cells.iter().filter(|o|o.current&&o.direct) {
            if matches!(o.tile.as_deref(),Some("up"|"down")) {
                let style=ground_compiled::prepare(profile,o,flesh);let glyph=style.glyph;let color=style.fg;let x=o.index%width;let y=o.index/width;
                ground_entities.push(GroundEntity{reserve:None,hallucination:None,kind:GroundKind::Stair,x,y,id:o.index as u64,
                    name:if o.tile.as_deref()==Some("up"){"Upward Staircase".into()}else{"Downward Staircase".into()},glyph,color,distance:distance(x,y)});
            }
            if let Some(vent)=o.hazards.as_ref().and_then(|h|h.vent.as_ref()) {
                let x=o.index%width;let y=o.index/width;
                let color=profile.value(&format!("terrain.gas:{}.color",vent.gas)).and_then(|v|v.as_str()).map(hex).unwrap_or_else(||gas_color(&vent.gas));
                let style=crate::entity_profile::resolve(profile,"objects","vent",None,"=",color);
                ground_entities.push(GroundEntity{reserve:None,hallucination:None,kind:GroundKind::Vent,x,y,id:o.index as u64,
                    name:format!("{} vent",vent.gas),glyph:style.glyph,color:style.color,distance:distance(x,y)});
            }
        }
        let mut items = Vec::new();
        let mut item_cells = std::collections::HashSet::new();
        let depth = run.depth.to_string();
        if let Some(ground) = run.ground.get(&depth) {
            for g in ground.iter().filter(|g| crate::dream_ui::item_visible(run, &g.item)) {
                let i = g.y as usize * width + g.x as usize;
                if observation.cells.get(i).is_some_and(|o|o.current&&o.direct) {
                    let key=if g.item.kind=="potion"{format!("potion:{}",g.item.effect.as_ref().map(String::as_str).unwrap_or(""))}else{g.item.name.clone()};
                    let style=crate::entity_profile::resolve(profile,"items",&key,Some(&format!("kind:{}",g.item.kind)),&item_glyph(&g.item),item_color(&g.item.kind));
                    ground_entities.push(GroundEntity{reserve:crate::render::resource_ui::amulet_reserve(&g.item,run.simulation_revision.value_or(29.)),hallucination:None,kind:GroundKind::Item,x:g.x as usize,y:g.y as usize,id:g.item.id.to_bits(),
                        name:if corpse_core::revision50::enabled(run){crate::pack::displayed_name(run,&g.item)}else{g.item.name.clone()},
                        glyph:style.glyph,color:style.color,distance:distance(g.x as usize,g.y as usize)});
                }
                if observation.cells.get(i).is_some_and(|c| c.direct) && item_cells.insert(i) {
                    items.push((i, &g.item, false, false));
                }
                if let Some(mate) = crate::portal_ui::observed_item_mate(run, &observation, i) {
                    if item_cells.insert(mate) { items.push((mate, &g.item, false, false)); }
                }
                for alias in crate::non_euclidean_ui::observed_item_aliases(run, &observation, i) {
                    if item_cells.insert(alias) { items.push((alias, &g.item, false, false)); }
                }
            }
        }
        if let Some(detected) = run.detected_items.as_ref().and_then(|m| m.get(&depth)) {
            for g in detected.iter().filter(|g| crate::dream_ui::item_visible(run, &g.item)) {
                let i = g.y as usize * width + g.x as usize;
                if i < width * height && item_cells.insert(i) {
                    items.push((i, &g.item, false, true));
                }
            }
        }
        if let Some(remembered) = run.remembered_items.as_ref().and_then(|m| m.get(&depth)) {
            for g in remembered.iter().filter(|g| crate::dream_ui::item_visible(run, &g.item)) {
                let i = g.y as usize * width + g.x as usize;
                if observation.cells.get(i).is_some_and(|c| !c.current) && item_cells.insert(i) {
                    items.push((i, &g.item, true, false));
                }
            }
        }
        let nonheart_cells: std::collections::HashSet<_> = items.iter()
            .filter(|(_, item, _, _)| item.kind != "heart").map(|(i, _, _, _)| *i).collect();
        let mut corpses = std::collections::HashSet::new();
        for a in &observed_actors {
            let i = a.y as usize * width + a.x as usize;
            if a.dead.value_or(false) && visibility().directly_visible_actor(a) && observation.cells.get(i).is_some_and(|o|o.current&&o.direct) {
                let cell=actor_cell_profile(profile,run,a,width,&backgrounds,false);
                let hallucination=cell.hallucination.as_ref().map(|variants|std::sync::Arc::new(std::array::from_fn(|i|(format!("{} corpse?",variants[i].label.trim_end_matches('?')),variants[i].glyph.clone(),variants[i].fg))));
                ground_entities.push(GroundEntity{reserve:None,hallucination,kind:GroundKind::Corpse,x:cell.x,y:cell.y,id:a.id.to_bits(),
                    name:format!("{} corpse",cell.label.trim_end_matches('?')),glyph:cell.glyph,color:cell.fg,distance:distance(cell.x,cell.y)});
            }
            if a.dead.value_or(false)
                && !active.contains(&i)
                && !fires.contains(&i)
                && !nonheart_cells.contains(&i)
                && corpses.insert(i)
            {
                let mut cell = actor_cell_profile(profile, run, a, width, &backgrounds, false);
                crate::hallucination_ui::map_cell(&mut cell, |cell| {
                    cell.fg = entity_ink(profile,&observation.cells[i],observation.all_seeing,
                        light[i],material_backgrounds[i],backgrounds[i],cell.entity_ink.unwrap());
                });
                cells.push(cell);
            }
        }
        for (i, item, remembered, detected) in items {
            if active.contains(&i) || fires.contains(&i) || corpses.contains(&i) {
                continue;
            }
            let key = if item.kind == "potion" {
                format!(
                    "potion:{}",
                    item.effect.as_ref().map(String::as_str).unwrap_or("")
                )
            } else {
                item.name.clone()
            };
            let style = crate::entity_profile::resolve(
                profile,
                "items",
                &key,
                Some(&format!("kind:{}", item.kind)),
                &item_glyph(item),
                item_color(&item.kind),
            );
            let ink = crate::render::EntityInk {
                identity_outline: None,
                color: style.color,
                player: false,
                telepathic: false,
                important: true,
                remembered,
                detected,
            };
            let fg = entity_ink(
                profile,
                &observation.cells[i],
                observation.all_seeing,
                light[i],
                material_backgrounds[i],
                backgrounds[i],
                ink,
            );
            cells.push(Cell {
                ground_item_id: Some(item.id.to_bits()),
                dream_entity: corpse_core::phantasmii_generation::is_shard(item)
                    || corpse_core::dreamwalking::dream_ground_item(item),
                x: i % width,
                y: i / width,
                glyph: style.glyph,
                emphasis: style.emphasis,
                pulse_speed: style.speed,
                label: if corpse_core::revision50::enabled(run) { crate::pack::displayed_name(run, item) } else { item.name.clone() },
                entity_ink: Some(ink),
                background_overlay: style.background.map(|c| (c, 1.)),
                fg,
                bg: backgrounds[i],
                ..Cell::default()
            });
        }
        for o in &observation.cells {
            if !fires.contains(&o.index)
                || active.contains(&o.index)
                || o.tile.as_deref() == Some("wall")
            {
                continue;
            }
            let mut cell = fire_cell(
                profile,
                o,
                observation.all_seeing,
                light[o.index],
                material_backgrounds[o.index],
                backgrounds[o.index],
                width,
                seed,
                crate::color::cosmetic_seed(&run.seed, 0),
                0.,
            );
            cell.fire = true;
            cells.push(cell);
        }
        for a in observed_actors {
            if !a.dead.value_or(false) {
                let i = a.y as usize * width + a.x as usize;
                let mut cell = actor_cell_profile(profile, run, a, width, &backgrounds, false);
                crate::hallucination_ui::map_cell(&mut cell, |cell| {
                    cell.fg = entity_ink(profile,&observation.cells[i],observation.all_seeing,
                        light[i],material_backgrounds[i],backgrounds[i],cell.entity_ink.unwrap());
                });
                if corpse_core::portals::enabled(run) && corpse_core::dreamwalking::same_plane(run, &run.player, a) {
                    if let Some(mate) = run.floor().and_then(|f| corpse_core::portals::mate(f, i)) {
                        if observation.cells.get(mate).is_some_and(|o| o.current) {
                            let mut alias = cell.clone();
                            alias.x = mate % width; alias.y = mate / width;
                            alias.bg = backgrounds[mate];
                            crate::hallucination_ui::append_alias(&mut alias," (through portal)");
                            cells.push(alias);
                        }
                    }
                }
                if corpse_core::dreamwalking::same_plane(run, &run.player, a) {
                    for other in crate::non_euclidean_ui::actor_alias_cells(run, &observation, i) {
                        let mut alias = cell.clone(); alias.x = other % width; alias.y = other / width;
                        alias.bg = backgrounds[other]; crate::hallucination_ui::append_alias(&mut alias," (linked space)");
                        cells.push(alias);
                    }
                }
                cells.push(cell);
            }
        }
        let mut cell = actor_cell_profile(profile, run, &run.player, width, &backgrounds, true);
        if flesh {
            // The historical player-only light ramp must not multiply the new
            // shared calibration. Preserve its exact neutral-light base instead.
            let invisible = corpse_core::engine::condition(&run.player, "invisible") > 0.;
            let palette = if invisible { [135., 183., 243.] } else { [255., 243., 189.] };
            let neutral = palette.map(|v: f64| (v * 0.78).round() as u8);
            let style = crate::entity_profile::resolve(profile, "objects", "player", None, "@", neutral);
            cell.fg = if invisible { neutral } else { style.color };
            if let Some(ink) = &mut cell.entity_ink { ink.color = cell.fg; }
        }
        cell.fg = crate::color_profile::world(profile, cell.fg, true);
        if corpse_core::portals::enabled(run) {
            let at = run.player.y as usize * width + run.player.x as usize;
            if let Some(mate) = run.floor().and_then(|f| corpse_core::portals::mate(f, at)) {
                if observation.cells.get(mate).is_some_and(|o| o.current) {
                    let mut alias = cell.clone(); alias.x = mate % width; alias.y = mate / width;
                    alias.bg = backgrounds[mate]; crate::hallucination_ui::append_alias(&mut alias," (through portal)");
                    cells.push(alias);
                }
            }
        }
        // The counter belongs to the real running Avatar only. No clone,
        // sleeping body, corpse, or portal alias receives a duplicate.
        if !run.player.dead.value_or(false) && run.player.hp > 0. {
            cell.number_of_blood=corpse_core::engine::number_of_blood_phase(run,&run.player);
        }
        // Linked topology does not leave a second player image at the departure
        // endpoint. Actor/item aliases and ordinary portal presentation are unchanged.
        cells.push(cell);
        for cell in &mut cells {
            let i = cell.y * width + cell.x;
            if active.contains(&i) || (item_cells.contains(&i) && cell.label != "Fire") {
                cell.animations.retain(|anim| matches!(anim, crate::render::Animation::Razorleaves { .. }));
            }
        }
        crate::illumination_ui::finish_cells(&mut cells, &light, width, profile);
    }
    corpse_core::timing::count(corpse_core::timing::Count::OutputCells,cells.len() as u64);
    if corpse_core::dreamwalking::active(run) {
        let seed = crate::color::cosmetic_seed(&run.seed, run.depth as u32);
        for cell in &mut cells { crate::hallucination_ui::map_cell(cell,|c|crate::dream_ui::override_cell(c,0.,seed)); }
    }
    let catalog: &serde_json::Value = {
        static CATALOG: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
        CATALOG
            .get_or_init(|| serde_json::from_str(include_str!("../assets/abilities.json")).unwrap())
    };
    let creature_span=corpse_core::timing::span("desktop.observe.creature_sidebar");
    let actor_cells: std::collections::HashMap<_, _> = cells
        .iter()
        .filter_map(|c| c.actor_id.map(|id| (id.to_bits(), c)))
        .collect();
    // One temporary projection scratch for sidebar stats; never retained by View.
    let mut sidebar_projection:Option<Run>=None;
    let creatures = {
        let mut actors: Vec<_> = run
            .actors_at(run.depth)
            .iter()
            .filter(|a| !a.dead.value_or(false) && visibility().visible_actor(a))
            .filter_map(|a| {
                let cell = actor_cells.get(&a.id.to_bits())?;
                let direct=visibility().directly_visible_actor(a);
                let original=a;
                let presented=crate::creature_presentation::project(run,a);
                let a=presented.as_ref().unwrap_or(a);
                let mut row=crate::render::Creature {
                    perception_range: crate::perception_ui::range_label(run,a),
                    identity_outline: if corpse_core::dreamwalking::active(run) {None} else {cell.entity_ink.and_then(|ink|ink.identity_outline)},
                    hallucination: None,
                    shared_resources: corpse_core::half_here::is_clone(a) || corpse_core::dreamwalking::is_body(a),
                    invulnerable: a.abilities.iter().any(|id| id == "enemy:beast:sphinx"),
                    id: a.id,
                    statuses: {
                        let mut statuses=crate::statuses_ui::observe(run,a);
                        if crate::monster_status_ui::feeding_description(run,a,direct,presented.is_some()) == Some("Feeding") {
                            statuses.push(crate::statuses_ui::Status{name:"Feeding".into(),duration:None});
                        }
                        statuses
                    },
                    meters: Vec::new(),
                    name: cell.label.clone(),
                    glyph: format!("{}{}", cell.glyph, cell.arrow),
                    color: if corpse_core::dreamwalking::active(run) && !corpse_core::dreamwalking::dream_actor(run, a) {
                        crate::dream_ui::NORMAL_INK
                    } else if direct {
                        if corpse_core::monster_vampire::flying(a) {
                            [227, 77, 85]
                        } else {
                            if run.simulation_revision.value_or(29.)>=50. {cell.entity_ink.map_or_else(||hex(&a.color),|ink|ink.color)} else {hex(&a.color)}
                        }
                    } else {
                        [134, 185, 237]
                    },
                    state: if corpse_core::dreamwalking::is_nightmare(a) {
                        "In mind".into()
                    } else if corpse_core::dreamwalking::is_body(a) {
                        "Sleeping".into()
                    } else if direct {
                        a.state.clone()
                    } else {
                        "sensed".into()
                    },
                    distance: (a.x - run.player.x).abs().max((a.y - run.player.y).abs()),
                    hp: a.hp,
                    max_hp: a.max_hp,
                    energy: a.energy,
                    uses_energy: crate::monster_status_ui::energy_capable(run.simulation_revision.value_or(29.), a),
                    max_energy: if a.abilities.iter().any(|id| id == "grisha-boundless-hunger") {
                        f64::INFINITY
                    } else {
                        corpse_core::combat::effective_stats(run, a).max_energy
                    },
                };
                if cell.hallucination.is_some() {
                    let scratch=sidebar_projection.get_or_insert_with(||run.clone());
                    let variants=std::array::from_fn(|phase| {
                        let shown=crate::creature_presentation::project_kind(run,original,crate::hallucination_ui::kind(run,original,phase)).unwrap();
                        let mut v=row.clone(); let c=&cell.hallucination.as_ref().unwrap()[phase];
                        v.name=c.label.clone();v.glyph=format!("{}{}",c.glyph,c.arrow);
                        v.identity_outline=if corpse_core::dreamwalking::active(run) {None} else {c.entity_ink.and_then(|ink|ink.identity_outline)};
                        v.color=if corpse_core::dreamwalking::active(run)&&!corpse_core::dreamwalking::dream_actor(run,&shown){crate::dream_ui::NORMAL_INK}else if direct {hex(&shown.color)}else{[134,185,237]};
                        v.hp=shown.hp;v.max_hp=shown.max_hp;v.energy=shown.energy;
                        if let Some(slot)=scratch.actors_at_mut(run.depth).iter_mut().find(|a|a.id==shown.id){*slot=shown.clone();}
                        v.max_energy=corpse_core::combat::effective_stats(scratch,&shown).max_energy;
                        v.uses_energy=crate::monster_status_ui::energy_capable(run.simulation_revision.value_or(29.), &shown);
                        v.perception_range=crate::perception_ui::range_label(scratch,&shown);
                        if let Some(label)=v.perception_range.as_mut(){crate::hallucination_ui::question(label);}
                        v.statuses=crate::statuses_ui::observe(scratch,&shown);v
                    });
                    if let Some(slot)=scratch.actors_at_mut(run.depth).iter_mut().find(|a|a.id==original.id){*slot=original.clone();}
                    row=variants[0].clone();row.hallucination=Some(std::sync::Arc::new(variants));
                }
                Some(row)
            })
            .collect();
        actors.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then(a.id.total_cmp(&b.id))
        });
        actors
    };
    drop(creature_span);
    let _hud_span=corpse_core::timing::span("desktop.observe.hud_log_inventory");
    crate::cogworks_ui::rally(run,&mut cells);
    let abilities = crate::ability_ui::visible_abilities(run);
    let pursuit_path=crate::targeting_ui::pursuit_path(run);
    let player_stats=corpse_core::combat::effective_stats(run,&run.player);
    let perception_boundaries=crate::perception_ui::observe(run,terrain_snapshot.as_deref(),&creatures);
    // P09 is anchored to the selected target's original 5x5 area, not its later
    // movement. Only presently directly observed cells receive the static edge.
    if run.simulation_revision.value_or(29.)>=52.&&run.player.active.iter().any(|s|s=="pelian-fated-duel")&&run.player.conditions.get("r50DuelDepth")==Some(&run.depth){
        if let (Some(&x),Some(&y))=(run.player.conditions.get("r50DuelX"),run.player.conditions.get("r50DuelY")){
            for c in &mut cells {if c.current&&c.direct&&(c.x as f64-x).abs()<=2.&&(c.y as f64-y).abs()<=2. {c.duel_edges=[c.x as f64==x+2.,c.y as f64==y+2.,c.x as f64==x-2.,c.y as f64==y-2.];}}
        }
    }
    if crate::hallucination_ui::active(run) {for row in &mut ground_entities {crate::hallucination_ui::question(&mut row.name);}}
    crate::render::enemy_list::sort_ground(&mut ground_entities);
    View {
        ground_entities,
        perception_boundaries,
        ability_overlays: crate::persistent_ability_overlays::observe(run,terrain_snapshot.as_deref()),
        hallucination_cells:cells.iter().enumerate().filter_map(|(i,c)|c.hallucination.is_some().then_some(i)).collect(),
        hallucination_phase:0,
        dreamwalking: corpse_core::dreamwalking::active(run),
        deep_water: corpse_core::environment::is_swimming(run, &run.player),
        terrain_snapshot,
        terrain_count,
        cosmetic_seed: crate::color::cosmetic_seed(&run.seed, run.depth as u32),
        fire_seed: crate::color::cosmetic_seed(&run.seed, 0),
        path: pursuit_path.clone(),
        pursuit_path,
        target_path: Vec::new(),
        target_area: Vec::new(),
        target_impact: None,
        target_color: None,
        selected_ability: None,
        cogworks_mode: corpse_core::revision50_terrain::mode(run),
        possible_exits: Vec::new(),
        hover: None,
        player_id: run.player.id,
        player_status_offset: 0,
        selected_actor: None,
        player_statuses: crate::statuses_ui::observe(run, &run.player),
        player_meters: Vec::new(),
        provenance: "Native migration test scene / incomplete port".into(),
        domain: run.floor().map(|f| f.domain.clone()).unwrap_or_default(),
        mode: if run.fast_mode.value_or(false) && !run.compressed_depths.value_or(false) { "Fast mode".into() } else { run.mode.clone() },
        gods: run
            .gods
            .iter()
            .filter_map(|id| {
                catalog["gods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|g| g["id"].as_f64() == Some(*id))
                    .and_then(|g| g["name"].as_str())
            })
            .collect::<Vec<_>>()
            .join(" / "),
        blessings: run.exp,
        blessing_goal: corpse_core::progression::level_up_cost(run),
        offers: run
            .offers
            .iter()
            .map(|id| crate::ability_ui::observe(run, id))
            .collect(),
        level: run.level,
        god_labels: run
            .gods
            .iter()
            .filter_map(|id| {
                catalog["gods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|g| g["id"].as_f64() == Some(*id))
                    .and_then(|g| g["name"].as_str())
                    .map(|name| (name.to_string(), crate::ability_ui::god_color(*id as usize)))
            })
            .collect(),
        total_abilities: abilities.len(),
        abilities,

        width,
        height,
        cells,
        hp: run.player.hp,
        max_hp: run.player.max_hp,
        resource_changes: crate::render::resource_ui::player_changes(run),
        pending_energy_loss: corpse_core::abilities::projected_player_upkeep(run),
        player_stats:[crate::stat_presentation::damage_bonus(run,player_stats.offense),6.*player_stats.defense,player_stats.dr,player_stats.accuracy,player_stats.dodge],
        energy: run.player.energy,
        max_energy: if run
            .player
            .abilities
            .iter()
            .any(|id| id == "grisha-boundless-hunger")
        {
            f64::INFINITY
        } else {
            player_stats.max_energy
        },
        turn: run.tick,
        depth: run.depth,
        display_depth: corpse_core::fast_mode::display_depth(run,run.depth),
        max_depth: corpse_core::fast_mode::maximum_display_depth(run),
        creatures,
        chronicle_offset: 0,
        log: run.log.clone(),
        log_groups: crate::chronicle::compact(
            &run.log,
            run.log_kinds.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        )
        .into_iter()
        .filter(|g| {
            run.dev || g.message != "Creature and loot tuning is provisional and documented."
        })
        .collect(),
        inventory: run
            .inventory
            .iter()
            .map(|i| {
                format!(
                    "{}{}",
                    if corpse_core::revision50::enabled(run) { crate::pack::displayed_name(run, i) } else { i.name.clone() },
                    if corpse_core::revision50::enabled(run) {
                        if corpse_core::revision50_inventory::worn(i) { " [active]" } else { "" }
                    } else if i.equipped.as_ref().is_some() {
                        " [equipped]"
                    } else {
                        ""
                    }
                )
            })
            .collect(),
        dead: run.player.dead.value_or(false) || run.player.hp <= 0.,
        state: format!(
            "{} | {} | history {} | {}",
            run.mode,
            run.phase.value_or("haste".into()),
            run.history.len(),
            run.player
                .conditions
                .iter()
                .filter(|(k, v)| **v > 0.
                    && ["hasted", "flying", "paralyzed", "weakened", "emboldened"]
                        .contains(&k.as_str()))
                .map(|(k, v)| format!("{k} {v:.0}"))
                .collect::<Vec<_>>()
                .join(" / ")
        ),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::model::{Actor, Field, Floor};
    use std::sync::Arc;
    #[test]
    fn ability_total_survives_scrolling() {
        let mut run = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        run.player.abilities = vec![
            "numo-revert".into(),
            "morae-see-the-future".into(),
            "hie-drown".into(),
        ];
        let mut view = observe(&run);
        let ids: Vec<_> = view.abilities.iter().map(|a| a.id.clone()).collect();
        assert_eq!(view.total_abilities, ids.len());
        assert!(view.total_abilities >= 3);
        // The desktop scroll path slices only the displayed rows.
        view.abilities = view.abilities.into_iter().skip(1).collect();
        assert_eq!(view.total_abilities, ids.len());
        assert_eq!(view.abilities.len(), ids.len() - 1);
        assert_eq!(view.abilities[0].id, ids[1]);
        view.abilities.clear();
        assert_eq!(view.total_abilities, ids.len());
    }
    #[test]
    fn hidden_live_terrain_and_actors_are_not_emitted() {
        let mut run = Run::default();
        run.depth = 1.;
        run.player.x = 1.;
        run.player.y = 1.;
        run.player.glyph = "@".into();
        let mut floor = Floor::default();
        floor.depth = 1.;
        floor.width = Field::Value(79.);
        floor.height = Field::Value(29.);
        floor.tiles = vec!["wall".into(); 79 * 29];
        floor.lights = vec![-100.; 79 * 29];
        floor.seen = vec![false; 79 * 29];
        run.floors = vec![Arc::new(floor)];
        let mut enemy = Actor::default();
        enemy.x = 70.;
        enemy.y = 20.;
        enemy.glyph = "Z".into();
        run.actors.insert("1".into(), Arc::new(vec![enemy]));
        let view = observe(&run);
        assert!(!view.cells.iter().any(|c| c.glyph == "Z"));
        assert!(view.creatures.is_empty());
        assert!(!view.cells.iter().any(|c| c.x == 70 && c.y == 20));
    }
    #[test]
    fn compound_identity_and_annotations_survive_projection() {
        let mut run = Run::default();
        run.mode = "Avatar".into();
        run.player.glyph = "(≬)".into();
        run.player.heartless = Field::Value(true);
        let c = actor_cell(&run, &run.player, 79, &vec![[0, 0, 0]; 79 * 29], true);
        assert_eq!(c.glyph, "@");
        let creature = actor_cell(&run, &run.player, 79, &vec![[0, 0, 0]; 79 * 29], false);
        assert_eq!(creature.glyph, "(≬)");
        assert!(c.heartless);
        assert!(c.superscript.is_empty());
    }
    #[test]
    fn wendigo_map_label_omits_growth_level() {
        let run = Run::default();
        let mut a = Actor { name: "Wendigo".into(), creature_kind: "wendigo".to_string().into(), ..Default::default() };
        a.conditions.insert("monsterWendigoLevel".into(), 5.);
        assert_eq!(actor_cell(&run, &a, 79, &[], false).label, "Wendigo");
    }
    #[test]
    fn source_renderer_avatar_scene_projection() {
        let run = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        let view = observe_palette(&run, crate::appearance_profile::Profile::defaults(), false);
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../assets/avatar-scene-parity.json")).unwrap();
        assert_source_scene(&view, &source);
    }
    #[test]
    fn source_renderer_generated_scene_projection() {
        let run =
            corpse_core::startup::create_run_revision("00-09-10:native-parity-v1", "Avatar", false, 30.).unwrap();
        let view = observe_palette(&run, crate::appearance_profile::Profile::defaults(), false);
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../assets/generated-scene-parity.json")).unwrap();
        assert_source_scene(&view, &source);
    }
    fn assert_source_scene(view: &View, source: &serde_json::Value) {
        let mut backgrounds = std::collections::HashMap::new();
        for t in source["tiles"].as_array().unwrap() {
            let i = t["index"].as_u64().unwrap() as usize;
            let c = view
                .cells
                .iter()
                .find(|c| c.y * view.width + c.x == i)
                .unwrap();
            let bg: [u8; 3] = serde_json::from_value(t["bg"].clone()).unwrap();
            assert_eq!(c.bg, bg, "source terrain background at {i}");
            backgrounds.insert(i, bg);
        }
        for d in source["drawn"].as_array().unwrap() {
            let font = d["font"].as_str().unwrap();
            let size = font
                .split_whitespace()
                .find_map(|v| v.strip_suffix("px").and_then(|n| n.parse::<f64>().ok()))
                .unwrap();
            if size < 10. {
                continue;
            } // Authored superscripts are validated separately.
            let i = (d["y"].as_f64().unwrap() / 15.).floor() as usize * view.width
                + (d["x"].as_f64().unwrap() / 15.).floor() as usize;
            let c = view
                .cells
                .iter()
                .rev()
                .find(|c| c.y * view.width + c.x == i)
                .unwrap();
            assert_eq!(c.glyph, d["glyph"].as_str().unwrap(), "source glyph at {i}");
            let rgb: Vec<u8> = d["color"]
                .as_str()
                .unwrap()
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|v| v.parse().ok())
                .collect();
            let expected = crate::color::composite(
                backgrounds[&i],
                [rgb[0], rgb[1], rgb[2]],
                d["alpha"].as_f64().unwrap(),
            );
            assert_eq!(c.fg, expected, "source ink at {i}");
        }
    }
    #[test]
    fn source_actor_ink_across_ambient_and_distance() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../assets/entity-ink-parity.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let mut run = corpse_core::storage::deserialize(include_str!(
                "../../../fixtures/avatar-room-v1.json"
            ))
            .unwrap();
            run.floor_mut()
                .unwrap()
                .lights
                .fill(case["ambient"].as_f64().unwrap());
            let (x, y) = (
                run.player.x + case["distance"].as_f64().unwrap(),
                run.player.y,
            );
            let actor = &mut std::sync::Arc::make_mut(run.actors.get_mut("1").unwrap())[0];
            actor.x = x;
            actor.y = y;
            let id = actor.id;
            let view = observe_palette(&run, crate::appearance_profile::Profile::defaults(), false);
            let actual = view.cells.iter().find(|c| c.actor_id == Some(id));
            assert_eq!(
                actual.is_some(),
                case["visible"].as_bool().unwrap(),
                "{case}"
            );
            if let Some(cell) = actual {
                let expected: [u8; 3] = serde_json::from_value(case["color"].clone()).unwrap();
                assert_eq!(cell.fg, expected, "{case}");
            }
        }
    }
    #[test]
    fn source_semantic_features_and_fixture_ink() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../assets/feature-scene-parity.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let mut value: serde_json::Value =
                serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json"))
                    .unwrap();
            let (x, y) = (
                case["x"].as_u64().unwrap() as usize,
                case["y"].as_u64().unwrap() as usize,
            );
            let index = y * 79 + x;
            value["actors"]["1"] = serde_json::json!([]);
            let tick = value["tick"].as_f64().unwrap();
            let floor = &mut value["floors"][0];
            let key = case["key"].as_str().unwrap();
            match key {
                "fire" => floor["permanentFire"] = serde_json::json!([index]),
                "blood" => {
                    let mut blood = vec![false; 79 * 29];
                    blood[index] = true;
                    floor[key] = serde_json::json!(blood);
                }
                "bulwarks" => floor[key] = serde_json::json!({index.to_string():tick+2.}),
                "torch" | "brazier" | "beacon" => {
                    floor["lightSources"] =
                        serde_json::json!([{"x":x,"y":y,"power":50,"falloff":5,"kind":key}])
                }
                _ => {
                    let mut cells = vec![0; 79 * 29];
                    cells[index] = 1;
                    floor[key] = serde_json::json!(cells);
                }
            }
            let run = corpse_core::storage::deserialize(&value.to_string()).unwrap();
            let view = observe_palette(&run, crate::appearance_profile::Profile::defaults(), false);
            let cell = view
                .cells
                .iter()
                .rev()
                .find(|c| c.x == x && c.y == y)
                .unwrap();
            let bg: [u8; 3] = serde_json::from_value(case["bg"].clone()).unwrap();
            assert_eq!(cell.bg, bg, "background {key}");
            if let Some(glyph) = case["glyph"].as_str() {
                assert_eq!(cell.glyph, glyph, "{key}");
                let rgb: [u8; 3] = serde_json::from_value(case["color"].clone()).unwrap();
                let expected = crate::color::composite(bg, rgb, case["alpha"].as_f64().unwrap());
                assert_eq!(cell.fg, expected, "ink {key}");
            }
        }
    }
    fn visual_scene() -> (Run, corpse_core::vision::TerrainObservation) {
        let run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut observation = corpse_core::vision::observe_terrain(&run).unwrap();
        observation.light_sources.clear();
        for c in &mut observation.cells {
            c.current = true; c.direct = true; c.knowledge = "direct".into();
            c.chance = 100.; c.direct_chance = 100.; c.distance = 0.;
            c.ambient = Some(0.); c.illumination = Some(0.);
            c.tile = Some("floor".into()); c.substrate = None;
            c.door_open = None; c.grate = false; c.bridge = false; c.snow = false;
            c.entrance = false; c.hazards = None;
        }
        (run, observation)
    }
    #[test]
    fn ichor_surface_dimming_preserves_hue_and_leaves_other_materials_unchanged() {
        let p=crate::appearance_profile::Profile::defaults();
        for key in ["feature:ichor", "feature:fire", "feature:slime", "feature:mud", "feature:razorleaves"] {
            let (raw_fg,raw_bg)=crate::appearance_material::feature_colors(p,key,3,4,17,None);
            let (fg,bg)=crate::world_features::feature_surface(p,key,3,4,17);
            if key=="feature:ichor" {
                for (raw,dim) in [(raw_fg,fg),(raw_bg,bg)] {
                    for (raw,dim) in raw.into_iter().zip(dim) {
                        let ratio=crate::color::to_linear(dim as f64)/crate::color::to_linear(raw as f64);
                        assert!((0.80..=0.84).contains(&ratio),"{ratio}");
                    }
                }
            } else {
                assert_eq!((fg,bg),(raw_fg,raw_bg),"{key}");
            }
        }
    }

    #[test]
    fn native_blood_river_remains_water_and_ichor_stays_distinct() {
        let (run, mut observation) = visual_scene();
        let before = serde_json::to_string(&run).unwrap();
        for c in &mut observation.cells[200..210] { c.tile = Some("water".into()); }
        // Ordinary water is blue; a blood river needs an observed blood amount.
        observation.cells[204].hazards = Some(serde_json::from_value(serde_json::json!({"waterBlood":100})).unwrap());
        observation.cells[205].hazards = Some(serde_json::from_value(serde_json::json!({"ichor":true})).unwrap());
        observation.cells[211].hazards = Some(serde_json::from_value(serde_json::json!({"blood":true})).unwrap());
        let p = crate::appearance_profile::Profile::defaults();
        let (cells,_,_) = terrain_frame(&observation,p,17,0.);
        let river = &cells[204]; let ichor = &cells[205];
        assert!(cells[203].bg[2] > cells[203].bg[0], "unbloodied water stays blue");
        assert_eq!(river.label,"water"); assert_eq!(river.glyph,"~");
        assert!(river.bg[0] > river.bg[1] && river.bg[0] > river.bg[2],"{:?}",river.bg);
        assert!(river.fg[0] > river.fg[1] && river.fg[0] > river.fg[2],"{:?}",river.fg);
        assert!(ichor.bg[1] > ichor.bg[0] && ichor.fg[1] > ichor.fg[0],"{:?} {:?}",ichor.bg,ichor.fg);
        // Floor blood background equality is checked against the same dry cell
        // in blood_dot_sequence_tests, not a differently textured neighbor.
        assert_eq!(before,serde_json::to_string(&run).unwrap());
        let first:Vec<_> = cells.iter().map(|c| (c.fg,c.bg,c.glyph.clone())).collect();
        let (again,_,_) = terrain_frame(&observation,p,17,0.);
        assert_eq!(first,again.iter().map(|c| (c.fg,c.bg,c.glyph.clone())).collect::<Vec<_>>());
    }
    #[test]
    fn unknown_cells_never_gain_flesh_or_blood_details() {
        let (_,mut observation) = visual_scene();
        observation.cells[205].tile = None;
        observation.cells[205].current = false;
        observation.cells[205].hazards = None;
        let (cells,backgrounds,_) = terrain_frame(&observation,crate::appearance_profile::Profile::defaults(),17,0.);
        assert!(!cells.iter().any(|c|c.y*observation.width+c.x==205));
        assert_eq!(backgrounds[205],[0;3]);
    }
    #[test]
    fn entrance_uses_blue_omega_without_changing_stair_identity() {
        let (_,mut observation) = visual_scene();
        observation.cells[205].tile=Some("up".into());
        observation.cells[205].entrance=true;
        let (cells,_,_)=terrain_frame(&observation,crate::appearance_profile::Profile::defaults(),17,0.);
        assert_eq!(cells[205].glyph,"Ω"); assert_eq!(cells[205].label,"up");
        assert!(cells[205].bg[2]>cells[205].bg[0]);
        assert!(cells[205].fg[2]>cells[205].fg[0]);
    }
    #[test]
    fn export_flesh_visual_review() {
        let Ok(directory)=std::env::var("CORPSE_FLESH_REVIEW") else {return;};
        std::fs::create_dir_all(&directory).unwrap();
        let run = corpse_core::startup::create_run("blood-river-visual-review", "Avatar", false).unwrap();
        let mut view=observe(&run);
        view.provenance="Visual QA: normal new-run visibility, no reveal.".into();
        // A second detached sheet exercises blood/ichor side by side without touching a Run.
        let (_,mut observation)=visual_scene();
        for c in &mut observation.cells {
            let x=c.index%observation.width; let y=c.index/observation.width;
            c.tile=Some(if x==0||y==0||x==78||y==28 {"wall"} else if (32..40).contains(&x) {"water"} else {"floor"}.into());
            if (34..38).contains(&x)&&(8..15).contains(&y) { c.hazards=Some(serde_json::from_value(serde_json::json!({"ichor":true})).unwrap()); }
            if (15..26).contains(&x)&&y==14 { c.hazards=Some(serde_json::from_value(serde_json::json!({"blood":true})).unwrap()); }
        }
        let profile=crate::appearance_profile::Profile::defaults();
        let mut painter=crate::render::Painter::new();
        for label in ["flesh-generated", "flesh-generated-revealed", "flesh-materials"] {
            if label=="flesh-generated-revealed" {
                let mut revealed=run.clone();
                revealed.player.conditions.insert("allSeeingReveal".into(),1.);
                revealed.player.conditions.insert("allSeeingDepth".into(),revealed.depth);
                view=observe(&revealed);
                view.provenance="Visual QA: actual generated floor with All-Seeing reveal for palette inspection.".into();
            }
            if label=="flesh-materials" {
                view.provenance="Visual QA: detached synthetic material sheet; crimson WATER, green-gold Ichor, cosmetic blood stains.".into();
                view.cells=terrain_frame(&observation,profile,17,0.).0;
                view.terrain_count=view.cells.len();
                view.terrain_snapshot=Some(std::sync::Arc::new(observation.clone()));
            }
            painter.game_at(&view,"",false,0.);
            let file=std::fs::File::create(format!("{directory}/{label}.png")).unwrap();
            let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);
            encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        }
    }
    #[test]
    fn palette_is_authored() {
        assert_eq!(terrain("water"), ('~', [24, 125, 197], [8, 44, 77]));
    }
}

/// O(E*C*V*L) display-text work for E archived events (or recent log entries);
/// catalog C and combat verbs V are fixed, L is message length. No Run mutation.
pub fn history(run: &Run) -> Vec<String> {
    history_with_notices(run, &[])
}

/// Merge sorted discovery notices at their raw log positions before filtering.
pub fn history_with_notices(run: &Run, positions: &[usize]) -> Vec<String> {
    use crate::chronicle::{present, visible};
    let mut lines = Vec::new();
    let mut positions = positions.iter().copied().peekable();
    let mut add = |index, message: Option<&String>| {
        while positions.next_if(|&at| at <= index).is_some() {
            lines.push(crate::bestiary_progress::ADDED.into());
        }
        if let Some(m) = message.filter(|m| visible(m)) {
            lines.push(present(m));
        }
    };
    if let Some(a) = run.log_archive.as_ref() {
        for (i, v) in a.events.iter().enumerate() {
            add(i, a.messages.get((*v as usize) / 4));
        }
    } else {
        for (i, m) in run.log.iter().enumerate() {
            add(i, Some(m));
        }
    }
    add(usize::MAX, None);
    lines
}

#[cfg(test)]
#[test]
fn chronicle_history_omits_routine_and_area_lines_with_or_without_archive() {
    let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.log.clear();
    run.log_archive = corpse_core::Field::Missing;
    corpse_core::engine::log(&mut run, "Tiger waits silently beside the doorway.", "enemy");
    corpse_core::engine::log(&mut run, "Pale Spitter recovers after its ranged attack.", "enemy");
    corpse_core::engine::log(&mut run, "Floor 2: Mountain.", "environment");
    corpse_core::engine::log(&mut run, "You take 3 damage from fire.", "environment");
    let before = run.clone();
    assert_eq!(history(&run), vec!["You take 3 damage from fire."]);
    assert_eq!(history_with_notices(&run, &[1]), vec!["Bestiary Pages Added!", "You take 3 damage from fire."]);
    assert!(before == run);
    run.log_archive = corpse_core::Field::Missing;
    assert_eq!(history(&run), vec!["You take 3 damage from fire."]);
}

pub fn ability_needs_target_for_run(run: &Run, id: &str) -> bool {
    if run.simulation_revision.value_or(29.) < 50. { return ability_needs_target(id); }
    corpse_core::progression::definition_for_revision(50., id)
        .is_some_and(|a| a.flags.iter().any(|f| f == "target"))
}

pub fn ability_needs_target(id: &str) -> bool {
    static C: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("../assets/abilities.json")).unwrap())
        ["abilities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"].as_str() == Some(id))
        .is_some_and(|a| {
            a["flags"]
                .as_array()
                .is_some_and(|flags| flags.iter().any(|f| f == "target"))
        })
}

fn razorleaf_appearance(
    profile: &crate::appearance_profile::Profile, o: &corpse_core::vision::ObservedCell,
    x:usize, y:usize, seed:u32, source:Rgb, bg:&mut Rgb, ctx:crate::color::Context<'_>,
    all_seeing:bool, animations:&mut Vec<crate::render::Animation>,
) {
    const KEY:&str="feature:razorleaves";
    if o.current {
        let (color, _) = crate::world_features::feature_surface(profile,KEY,x,y,seed);
        let speed=profile.number("terrain.feature:razorleaves.animationSpeed");
        let opacity=profile.number("terrain.feature:razorleaves.opacity");
        animations.push(crate::render::Animation::Razorleaves {
            phase:crate::color::cosmetic_hash(x as i32,y as i32,seed),speed,
            color:crate::color_profile::world(profile,color,false),opacity,
        });
    } else {
        // Only last-seen terrain retains a stationary hint.
        let (color,alpha)=crate::world_features::overlay(profile,KEY,x,y,seed,0.,0.22);
        let color=crate::world_features::tone(profile,color,source,*bg,ctx,o.chance,all_seeing,
            Some(profile.number("terrain.feature:razorleaves.lightingResponse")),true,false,false);
        *bg=crate::color::composite(*bg,color,alpha);
    }
}

/// Repaints detached known terrain for cosmetic lighting; no Run or RNG access.
pub fn terrain_frame(
    observation: &corpse_core::vision::TerrainObservation,
    profile: &crate::appearance_profile::Profile,
    seed: u32,
    now: f64,
) -> (Vec<Cell>, Vec<Rgb>, Vec<crate::color::Light>) {
    let (mut cells, mut backgrounds, lights) = terrain_frame_baseline(observation, profile, seed, now);
    crate::illumination_ui::finish_cells(&mut cells, &lights, observation.width, profile);
    for cell in &cells { backgrounds[cell.y * observation.width + cell.x] = cell.bg; }
    (cells, backgrounds, lights)
}
/// Internal pre-calibration frame, so entities and terrain share one final pass.
pub(crate) fn terrain_frame_baseline(
    observation: &corpse_core::vision::TerrainObservation,
    profile: &crate::appearance_profile::Profile,
    seed: u32,
    now: f64,
) -> (Vec<Cell>, Vec<Rgb>, Vec<crate::color::Light>) {
    terrain_frame_palette(observation, profile, seed, now, true)
}
fn terrain_frame_palette(
    observation: &corpse_core::vision::TerrainObservation,
    profile: &crate::appearance_profile::Profile,
    seed: u32,
    now: f64,
    flesh: bool,
) -> (Vec<Cell>, Vec<Rgb>, Vec<crate::color::Light>) {
    let width = observation.width;
    let height = observation.height;
    let _terrain_span=corpse_core::timing::span("desktop.observe.terrain_frame");
    let mut light = crate::appearance_light::light_field(observation, profile, now);
    // Frozen source-v48 parity uses the historical color pipeline explicitly.
    if !flesh { for light in &mut light { light.display = None; } }
    let _style_span=corpse_core::timing::span("desktop.observe.terrain_style_cells");
    let quiet_chasm_edges = flesh && flesh_palette::brogue_chasm_edges_enabled(profile);
    // Separate diagnostic mode. 0=off, 1=all cells; >1=sparse grid-index sample.
    // Parse once per frame, never per profile lookup. No simulation/cache mutation.
    #[cfg(feature="runtime-timing")]
    let style_stride=std::env::var("OBSERVE_STYLE_STRIDE").ok().and_then(|x|x.parse::<usize>().ok()).unwrap_or(0);
    #[cfg(not(feature="runtime-timing"))]
    let style_stride=0usize;
    #[cfg(feature="runtime-timing")]
    let style_offset=std::env::var("OBSERVE_STYLE_OFFSET").ok().and_then(|x|x.parse::<usize>().ok()).unwrap_or(0);
    #[cfg(not(feature="runtime-timing"))]
    let style_offset=0usize;
    let prepared:Vec<_>={let _s=corpse_core::timing::span("desktop.observe.ground_prepare");
        observation.cells.iter().map(|o|o.tile.as_ref().map(|_|ground_compiled::prepare(profile,o,flesh))).collect()};
    let mut cells = Vec::new();
    let mut backgrounds = vec![if flesh {crate::illumination_ui::unexplored(profile)} else {profile.rgb("fog.unexplored")}; width * height];
    for o in &observation.cells {
        let Some(tile) = o.tile.as_deref().map(visual_tile) else {
            continue;
        };
        corpse_core::timing::count(corpse_core::timing::Count::KnownCells,1);
        let x = o.index % width;
        let y = o.index / width;
        let knowledge = knowledge(&o.knowledge);
        let style_sample=style_stride>0 && o.index%style_stride==style_offset%style_stride;
        let ground_span=corpse_core::timing::sampled_span("desktop.style_sample.ground",style_sample);
        let ground=prepared[o.index].as_ref().expect("known cell prepared");
        let (mut glyph,fg,mut bg,fixture,stone)=(ground.glyph.clone(),ground.fg,ground.bg,ground.fixture,ground.stone);
        drop(ground_span);
        let mut important = matches!(tile, "up" | "down" | "pit" | "chasm") || fixture;
        let ctx = crate::color::Context {
            knowledge,
            tile,
            distance: (o.distance / (1. + light[o.index].baseline().illumination.min(0.) / 100.).max(0.05))
                .min(3. + 21. * (1. - ((o.direct_chance - 5.) / 95.).clamp(0., 1.).sqrt()))
                as f32 as f64,
            light: light[o.index],
            important,
        };
        let neighbor_span=corpse_core::timing::sampled_span("desktop.style_sample.neighbors",style_sample);
        if !fixture
            && !o.snow
            && matches!(
                tile,
                "floor" | "grass" | "tree" | "water" | "shallow-water"
            )
        {
            let mut neighbors = Vec::new();
            for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let other = &observation.cells[ny as usize * width + nx as usize];
                if !o.current && other.current {
                    continue;
                }
                if other.tile.as_deref().map(visual_tile).is_some_and(|t| {
                    matches!(
                        t,
                        "floor" | "grass" | "tree" | "water" | "shallow-water"
                    )
                }) && !other.snow
                {
                    let other_ground=prepared[other.index].as_ref().expect("known neighbor prepared");
                    let(c,f)=(other_ground.bg,other_ground.fixture);
                    if !f && c != bg {
                        neighbors.push((visual_tile(other.tile.as_deref().unwrap()), c));
                    }
                }
            }
            if !neighbors.is_empty() {
                let weight = (neighbors.len() as f64
                    * (0.1 + crate::color::noise(x as f64, y as f64, seed, 6.) * 0.02))
                    .min(0.18);
                let bank = matches!(tile, "water" | "shallow-water")
                    && neighbors
                        .iter()
                        .any(|(t, _)| !matches!(*t, "water" | "shallow-water"));
                bg = std::array::from_fn(|i| {
                    let target = if bank {
                        if flesh { [123., 39., 56.][i] } else { [18., 69., 106.][i] }
                    } else {
                        neighbors.iter().map(|(_, c)| c[i] as f64).sum::<f64>()
                            / neighbors.len() as f64
                    };
                    (bg[i] as f64 * (1. - weight) + target * weight).round() as u8
                });
            }
        }
        drop(neighbor_span);
        let material_span=corpse_core::timing::sampled_span("desktop.style_sample.material_prepare",style_sample);
        let material=ground.displayed;
        let original_material=ground.original;
        let fg = flesh_palette::wall_ink(fg, o, observation, flesh && original_material.color_default);
        bg = flesh_palette::veins(bg, tile, x, y, seed, flesh && !fixture && !o.snow && o.biome.is_none()
            && original_material.background_default);
        drop(material_span);
        let color_span=corpse_core::timing::sampled_span("desktop.style_sample.color_math",style_sample);
        let (mut fg, mut bg, source_material_background) = crate::color_profile::terrain_material_resolved(
            profile,
            fg,
            bg,
            ctx,
            x,
            y,
            seed,
            fixture,
            stone,
            material,
            o.chance,
            observation.all_seeing,
            (flesh && o.direct && !fixture && matches!(tile, "water" | "shallow-water"))
                .then(|| flesh_palette::water_illumination(profile, fg, bg, tile, ctx.light)),
        );
        let pulse = crate::entity_profile::material_pulse(
            material.animation_speed,
            now,
            x,
            y,
        );
        if pulse != 1. {
            fg = crate::color_profile::grade(fg, pulse, 1., 1.);
            bg = crate::color_profile::grade(bg, pulse, 1., 1.);
        }
        drop(color_span);
        let feature_span=corpse_core::timing::sampled_span("desktop.style_sample.features",style_sample);
        let mut underlays = Vec::new();
        let mut glyph_opacity = 1.;
        let mut glyph_emphasis = 1.;
        macro_rules! detail {
            ($detail:expr,$important:expr,$emissive:expr) => {{
                let d = $detail;
                glyph = d.glyph;
                fg = crate::world_features::tone(
                    profile,
                    d.color,
                    source_material_background,
                    bg,
                    ctx,
                    o.chance,
                    observation.all_seeing,
                    d.response,
                    false,
                    $important,
                    $emissive,
                );
                glyph_opacity = d.opacity;
                glyph_emphasis = d.emphasis;
                if let Some((c, a)) = d.background {
                    let color = crate::world_features::tone(
                        profile,
                        c,
                        source_material_background,
                        bg,
                        ctx,
                        o.chance,
                        observation.all_seeing,
                        d.response,
                        true,
                        false,
                        false,
                    );
                    bg = crate::color::composite(bg, color, a);
                }
            }};
        }
        let mut arrow = String::new();
        let mut animations = Vec::new();
        let energy = if matches!(tile, "wall" | "tree") {
            ctx.light.wall_energy
        } else {
            ctx.light.energy
        };
        let peak = energy.into_iter().fold(0f64, f64::max);
        let burning = o
            .hazards
            .as_ref()
            .is_some_and(|h| h.fire.as_ref().is_some() || h.permanent_fire.value_or(false));
        if o.direct && ctx.light.display.is_none() && peak > 0.08 && !matches!(tile, "chasm" | "pit") && !burning {
            animations.push(crate::render::Animation::Reflection {
                phase: crate::color::noise(x as f64, y as f64, seed, 10.) * std::f64::consts::PI,
                color: energy.map(|v| crate::color::from_linear(v / peak * 0.35).round() as u8),
            });
        }

        if o.direct && matches!(tile, "water" | "shallow-water")
            && crate::appearance_material::idle_overlay_enabled(profile,tile) {
            animations.push(crate::render::Animation::Water {
                phase: crate::color::noise(x as f64, y as f64, seed, 8.) * std::f64::consts::PI,
                speed: profile.number(&format!("terrain.{tile}.animationSpeed")),
            });
        }
        if let Some(h) = &o.hazards {
            if h.razorleaves.value_or(false) {
                razorleaf_appearance(profile,o,x,y,seed,source_material_background,&mut bg,ctx,observation.all_seeing,&mut animations);
            }
            if tile == "wall" && h.bloody_wall.value_or(false) {
                let stain=crate::world_features::tone(profile,[206,34,61],source_material_background,bg,ctx,o.chance,observation.all_seeing,None,true,false,false);
                fg=crate::color::composite(fg,stain,0.75);
            }
            if matches!(tile,"water"|"shallow-water") && !h.ichor.value_or(false) {
                if let Some(amount)=h.water_blood.as_ref().filter(|v|**v>0.) {
                    let alpha=(0.20+0.65*(amount/100.).sqrt()).clamp(0.,0.85);
                    let stain=crate::world_features::tone(profile,[206,34,61],source_material_background,bg,ctx,o.chance,observation.all_seeing,None,true,false,false);
                    let dark=crate::world_features::tone(profile,[104,20,39],source_material_background,bg,ctx,o.chance,observation.all_seeing,None,true,false,false);
                    fg=crate::color::composite(fg,stain,alpha);
                    bg=crate::color::composite(bg,dark,alpha);
                }
            }
            for (enabled, key, alpha) in [
                (h.slime.value_or(false), "feature:slime", 0.22),
                (h.mud.value_or(false), "feature:mud", 0.24),
                (h.ichor.value_or(false), "feature:ichor", if flesh { 0.78 } else { 0.28 }),
            ] {
                if enabled {
                    let (c, alpha) =
                        crate::world_features::overlay(profile, key, x, y, seed, now, alpha);
                    let response = profile
                        .value(&format!("terrain.{key}.lightingResponse"))
                        .and_then(|v| v.as_f64());
                    let c = crate::world_features::tone(
                        profile,
                        c,
                        source_material_background,
                        bg,
                        ctx,
                        o.chance,
                        observation.all_seeing,
                        response,
                        true,
                        false,
                        false,
                    );
                    bg = crate::color::composite(bg, c, alpha);
                    if flesh && key == "feature:ichor" {
                        let (ichor, _) = crate::world_features::feature_surface(
                            profile, key, x, y, seed);
                        fg = crate::world_features::tone(profile, ichor, source_material_background,
                            bg, ctx, o.chance, observation.all_seeing, response, false, false, false);
                    }
                    let keypath = format!("terrain.{key}.glyph");
                    if profile.value(&keypath)
                        != crate::appearance_profile::Profile::defaults().value(&keypath)
                    {
                        let overlay_glyph = profile.text(&keypath).to_string();
                        let (c, _) = crate::world_features::feature_surface(
                            profile, key, x, y, seed,
                        );
                        let overlay_color = crate::world_features::tone(
                            profile,
                            c,
                            source_material_background,
                            bg,
                            ctx,
                            o.chance,
                            observation.all_seeing,
                            response,
                            false,
                            false,
                            false,
                        );
                        underlays.push((
                            overlay_glyph,
                            overlay_color,
                            profile.number(&format!("terrain.{key}.opacity")),
                        ));
                    }
                }
            }
            if let Some(gas) = h.gas.as_ref() {
                let key = format!("gas:{}", gas.r#type);
                let fallback=(matches!(gas.r#type.as_str(),"hallucinogenic"|"bloodMist") && profile.resolved_material(&key,crate::appearance_profile::MaterialParent::None).is_none())
                    .then(|| {let color=gas_color(&gas.r#type);(color,color)});
                let (_, c) =
                    crate::appearance_material::feature_colors(profile, &key, x, y, seed, fallback);
                let c = crate::color::gas_field(c);
                let opacity = profile
                    .value(&format!("terrain.{key}.opacity"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.);
                let response = profile
                    .value(&format!("terrain.{key}.lightingResponse"))
                    .and_then(|v| v.as_f64());
                let c = crate::world_features::tone(
                    profile,
                    c,
                    source_material_background,
                    bg,
                    ctx,
                    o.chance,
                    observation.all_seeing,
                    response,
                    true,
                    false,
                    false,
                );
                if o.direct && crate::appearance_material::idle_overlay_enabled(profile,&key) {
                    animations.push(crate::render::Animation::Gas {
                        phase: crate::color::noise(x as f64, y as f64, seed, 8.)
                            * std::f64::consts::PI,
                        color: c,
                    });
                }
                bg = crate::color::composite(bg, c, 0.26 * opacity);
            }
            if tile != "wall"
                && (h.permanent_fire.value_or(false)
                    || h.fire.as_ref().is_some_and(|f| f != "embers"))
            {
                let field = if profile.value("terrain.feature:fire.background")
                    == crate::appearance_profile::Profile::defaults()
                        .value("terrain.feature:fire.background")
                {
                    profile.rgb("palette.fire.background")
                } else {
                    profile.rgb("terrain.feature:fire.background")
                };
                let c = crate::world_features::tone(
                    profile,
                    field,
                    source_material_background,
                    bg,
                    ctx,
                    o.chance,
                    observation.all_seeing,
                    None,
                    true,
                    false,
                    false,
                );
                bg = crate::color::composite(
                    bg,
                    c,
                    0.25 * profile.number("terrain.feature:fire.opacity"),
                );
            }
            if h.blood.value_or(false) && (glyph == "." || (flesh && glyph == ",")) {
                detail!(
                    crate::world_features::blood_dot(
                        profile,
                        &glyph,
                        if flesh { [232, 47, 64] } else { [255, 23, 76] },
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
            }
            if let Some(root) = h.root.as_ref() {
                detail!(
                    crate::world_features::material(
                        profile,
                        "feature:roots",
                        &root.glyph,
                        hex(&root.color),
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if let Some(vent) = h.vent.as_ref() {
                let key = format!("terrain.gas:{}.color", vent.gas);
                let color = profile
                    .value(&key)
                    .and_then(|v| v.as_str())
                    .map(hex)
                    .unwrap_or_else(|| gas_color(&vent.gas));
                detail!(
                    crate::world_features::object(profile, "vent", None, "=", color, x, y, now),
                    false,
                    false
                );
                important = true;
            }
            if let Some(feature) = h.feature.as_ref() {
                detail!(
                    crate::world_features::object(
                        profile,
                        &feature.name,
                        None,
                        &feature.glyph,
                        hex(&feature.color),
                        x,
                        y,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if let Some(source) = h.light_source.as_ref() {
                glyph = "*".into();
                if o.direct && tile == "wall" && crate::appearance_material::idle_torch_enabled(profile,source) {
                    animations.push(crate::render::Animation::Torch);
                }
                important = true;
                detail!(
                    crate::world_features::object(
                        profile,
                        source,
                        Some("light"),
                        "*",
                        if source == "beacon" {
                            [182, 77, 255]
                        } else {
                            [255, 211, 74]
                        },
                        x,
                        y,
                        now
                    ),
                    true,
                    true
                );
            }
            if h.fire.as_ref().is_some_and(|f| f == "embers") && h.light_source.as_ref().is_none() {
                detail!(
                    crate::world_features::material(
                        profile,
                        "feature:embers",
                        "·",
                        [227, 38, 54],
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if h.bulwark.as_ref().is_some() {
                detail!(
                    crate::world_features::material(
                        profile,
                        "feature:ward",
                        "∷",
                        [86, 255, 55],
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if tile == "water" && !h.ichor.value_or(false) {
                if let Some(code) = h.current.as_ref() {
                    arrow = ["", "↑", "↗", "→", "↘", "↓", "↙", "←", "↖"]
                        .get(*code as usize)
                        .unwrap_or(&"")
                        .to_string();
                }
            }
        }
        drop(feature_span);
        let _final_span=corpse_core::timing::sampled_span("desktop.style_sample.edges_contrast_output",style_sample);
        let mut edges = [None; 4];
        for (d, (dx, dy)) in [(1, 0), (0, 1), (-1, 0), (0, -1)].into_iter().enumerate() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let other = &observation.cells[ny as usize * width + nx as usize];
            if !o.current && other.current {
                continue;
            }
            let Some(nt) = other.tile.as_deref().map(visual_tile) else {
                continue;
            };
            let mut brogue_chasm_edge = false;
            let edge = if o.biome.as_deref() == Some("corn-dimension") && other.biome.as_deref() != Some("corn-dimension") {
                Some(([201, 152, 236], if o.current { 0.55 } else { 0.24 }))
            } else if tile == "wall" && flesh_palette::wall_face_neighbor(o, other) {
                Some((
                    material.edge.filter(|_|material.edge_nonempty).unwrap_or([197,195,211]),
                    if o.current { 0.4 } else { 0.21 },
                ))
            } else if matches!(tile, "water" | "shallow-water")
                && !matches!(
                    nt,
                    "wall" | "chasm" | "pit" | "water" | "shallow-water"
                )
            {
                let edge = if flesh {
                    material.edge.unwrap_or([239,137,145])
                } else { [115, 239, 255] };
                Some((edge, if o.current { 0.21 } else { 0.14 }))
            } else if tile == "chasm" && !matches!(nt, "chasm" | "pit") {
                brogue_chasm_edge = flesh;
                Some((if flesh {flesh_palette::brogue_chasm(x,y,seed)}else{[170,145,220]}, if o.current { 0.62 } else { 0.46 }))
            } else if !matches!(tile, "wall" | "chasm" | "pit")
                && matches!(nt, "chasm" | "pit")
            {
                brogue_chasm_edge = flesh && nt == "chasm";
                Some((if flesh && nt == "chasm" {flesh_palette::brogue_chasm(nx as usize,ny as usize,seed)}else{[170,145,220]}, 0.28))
            } else {
                None
            };
            if let Some((c, a)) = edge {
                // Decorative void boundaries must not pass through the generic
                // important-symbol contrast rescue, which whitens Brogue ink.
                let quiet = brogue_chasm_edge && quiet_chasm_edges;
                let edge_bg = if quiet && tile == "chasm" { [0; 3] } else { bg };
                let ink = if quiet {
                    if o.current { c } else { crate::color::memory(c) }
                } else { crate::color_profile::map_colors(profile, c, bg, ctx).0 };
                edges[d] = Some(crate::color::composite(edge_bg, ink, a * material.opacity));
            }
        }
        backgrounds[o.index] = bg;
        if profile.is_default("global")
        {
            fg = crate::color::distinguish(
                fg,
                bg,
                if o.current {
                    if important {
                        2.8
                    } else if tile == "floor" {
                        1.4
                    } else if matches!(tile, "water" | "shallow-water") {
                        1.55
                    } else {
                        1.85
                    }
                } else {
                    1.45
                },
            );
        }
        let fg = crate::color::composite(
            bg,
            fg,
            crate::color::ink_alpha(tile, important)
                * material.opacity
                * glyph_opacity
                * if o.current {
                    1.
                } else {
                    profile.number("fog.rememberedOpacity")
                },
        );
        let (fg,bg)=if flesh && tile == "chasm" && !fixture && !o.snow && !o.ice && !o.hellfire
            && glyph_opacity == 1. && flesh_palette::brogue_palette_enabled(profile) {
            let ink=if profile.is_default("terrain.chasm.color") {
                let color=flesh_palette::brogue_chasm(x,y,seed);
                if o.current {color} else {crate::color::memory(color)}
            }else{fg};
            let ground=if profile.is_default("terrain.chasm.background") {[0;3]}else{bg};
            (ink,ground)
        }else{(fg,bg)};
        backgrounds[o.index]=bg;
        cells.push(Cell {
            x,
            y,
            bottom_aligned: tile == "grass" && glyph == "\"" && !fixture && !o.snow
                && !o.ice && !o.puddles && !o.hellfire && !o.flowers,
            glyph,
            label: if o.hellfire { "Hellfire".into() } else if o.ice { "Ice".into() } else if o.puddles { "Puddle".into() } else if tile == "wall" && o.hazards.as_ref().is_some_and(|h|h.bloody_wall.value_or(false)) { "Bloody wall".into() } else { o.tile.clone().unwrap_or_default() },
            emphasis: glyph_emphasis,
            underlays,
            material_background: Some(source_material_background),
            current: o.current,
            direct: o.direct,
            fg,
            bg,
            arrow,
            edges,
            animations,
            ..Cell::default()
        });
    }
    (cells, backgrounds, light)
}

/// Source tone for a detached entity, reused by cosmetic lighting frames.
pub fn entity_ink(
    p: &crate::appearance_profile::Profile,
    o: &corpse_core::vision::ObservedCell,
    all_seeing: bool,
    light: crate::color::Light,
    material_bg: Rgb,
    background: Rgb,
    ink: crate::render::EntityInk,
) -> Rgb {
    if ink.player {
        return crate::color_profile::world(p, ink.color, true);
    }
    if ink.telepathic {
        return crate::color_profile::world(
            p,
            crate::color_profile::grade(
                p.rgb("fog.telepathyColor"),
                p.number("fog.telepathyIntensity"),
                1.,
                1.,
            ),
            true,
        );
    }
    if ink.identity_outline.is_some() { return ink.color; }
    let mut base = ink.color;
    if ink.detected {
        if p.text("fog.revealColor") != "#87b7f3" {
            base = crate::color::composite(base, p.rgb("fog.revealColor"), 0.35);
        }
        base = crate::color_profile::grade(base, p.number("fog.revealIntensity"), 1., 1.);
        if !o.current {
            return crate::color_profile::world(p, base, true);
        }
    }
    let tile = o.tile.as_deref().unwrap_or("");
    let context = crate::color::Context {
        knowledge: if ink.remembered {
            crate::color::Knowledge::Remembered
        } else {
            knowledge(&o.knowledge)
        },
        tile,
        distance: (o.distance / (1. + light.baseline().illumination.min(0.) / 100.).max(0.05))
            .min(3. + 21. * (1. - ((o.direct_chance - 5.) / 95.).clamp(0., 1.).sqrt()))
            as f32 as f64,
        light,
        important: ink.important,
    };
    let response = p
        .value(&format!("terrain.{tile}.lightingResponse"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1.);
    let fg = crate::color_profile::map_colors_observed(
        p,
        base,
        material_bg,
        context,
        false,
        response,
        o.chance,
        all_seeing,
    )
    .0;
    let fg = if p.value("global") != crate::appearance_profile::Profile::defaults().value("global")
    {
        fg
    } else {
        crate::color::distinguish(
            fg,
            background,
            if ink.remembered || !o.current {
                1.45
            } else if ink.important {
                2.8
            } else {
                1.85
            },
        )
    };
    if ink.remembered {
        crate::color::composite(background, fg, 0.76 * p.number("fog.rememberedOpacity"))
    } else {
        fg
    }
}

/// O(1) fixed palette work per known fire; stores detached anchors for idle paint.
/// Uses detached knowledge and a cosmetic clock only, never simulation RNG.
pub fn fire_cell(
    p: &crate::appearance_profile::Profile,
    o: &corpse_core::vision::ObservedCell,
    all_seeing: bool,
    light: crate::color::Light,
    material_bg: Rgb,
    bg: Rgb,
    width: usize,
    seed: u32,
    fire_seed: u32,
    now: f64,
) -> Cell {
    let x = o.index % width;
    let y = o.index / width;
    let speed = p.number("terrain.feature:fire.animationSpeed");
    let phase = crate::color::cosmetic_hash(x as i32, y as i32, fire_seed) * std::f64::consts::TAU;
    let position = if speed == 0. {
        1.5
    } else {
        (0.5 + 0.5 * (now * speed / 340. + phase).sin()) * 3.
    };
    let colors = if o.hellfire { [[7, 2, 5], [103, 9, 17], [220, 35, 19], [255, 115, 31]] } else { [
        p.rgb("palette.fire.red"),
        p.rgb("palette.fire.lava"),
        p.rgb("terrain.feature:fire.color"),
        p.rgb("palette.fire.yellow"),
    ] };
    let low = position.floor() as usize;
    let high = (low + 1).min(3);
    let original = if !o.current {
        if o.hellfire { [166, 36, 25] } else { [255, 118, 46] }
    } else {
        crate::color::composite(colors[low], colors[high], position - low as f64)
    };
    let base = if o.hellfire || p.value("terrain.feature:fire.color")
        == crate::appearance_profile::Profile::defaults().value("terrain.feature:fire.color")
    {
        original
    } else {
        p.rgb("terrain.feature:fire.color")
    };
    let (mut color, _) = crate::appearance_material::feature_colors(
        p,
        "feature:fire",
        x,
        y,
        seed,
        Some((base, p.rgb("terrain.feature:fire.background"))),
    );
    let mut glyph = p.text("terrain.feature:fire.glyph").to_string();
    let mut emphasis = 1.;
    if let Some(source) = o.hazards.as_ref().and_then(|h| h.light_source.as_ref()) {
        let d = crate::world_features::object(p, source, Some("light"), "*", color, x, y, now);
        color = d.color;
        glyph = d.glyph;
        emphasis = d.emphasis;
    }
    let tile = o.tile.as_deref().unwrap_or("");
    let context = crate::color::Context {
        knowledge: knowledge(&o.knowledge),
        tile,
        distance: (o.distance / (1. + light.baseline().illumination.min(0.) / 100.).max(0.05))
            .min(3. + 21. * (1. - ((o.direct_chance - 5.) / 95.).clamp(0., 1.).sqrt()))
            as f32 as f64,
        light,
        important: true,
    };
    let color = crate::world_features::tone(
        p,
        color,
        material_bg,
        bg,
        context,
        o.chance,
        all_seeing,
        Some(p.number("terrain.feature:fire.lightingResponse")),
        false,
        true,
        true,
    );
    let opacity = p.number("terrain.feature:fire.opacity")
        * if o.current {
            1.
        } else {
            p.number("fog.rememberedOpacity")
        };
    // Keep usable transitions readable when the collapse's trailing hellfire
    // reaches them. Flame-colored ground and an ember underlay frame the intact
    // stair glyph instead of replacing it with the regular fire glyph.
    if o.hellfire && matches!(tile, "up" | "down") {
        let stair = ground_compiled::prepare(p, o, false);
        let hot_bg = crate::color::composite(bg, [115, 22, 12], opacity * 0.78);
        let fg = crate::color::composite(hot_bg, stair.fg, if o.current { 1. } else { opacity });
        return Cell {
            x, y, glyph: stair.glyph, emphasis,
            underlays: vec![(glyph, color, opacity * 0.55)],
            fire: true, current: o.current, direct: o.direct, fg, bg,
            background_overlay: Some(([115, 22, 12], opacity * 0.78)),
            label: if o.entrance { "Blue entrance in hellfire" }
                else if tile == "up" { "Upstairs in hellfire" }
                else { "Downstairs in hellfire" }.into(),
            ..Cell::default()
        };
    }
    // O(1): four detached, already knowledge/light-graded palette anchors per known fire.
    // Idle painting interpolates these only; it never re-observes terrain or rebuilds light fields.
    let animations=if o.current && speed!=0.{
        let custom=!o.hellfire && p.value("terrain.feature:fire.color")!=crate::appearance_profile::Profile::defaults().value("terrain.feature:fire.color");
        let colors=colors.map(|anchor|{
            let base=if custom{p.rgb("terrain.feature:fire.color")}else{anchor};
            let(mut color,_)=crate::appearance_material::feature_colors(p,"feature:fire",x,y,seed,Some((base,p.rgb("terrain.feature:fire.background"))));
            if let Some(source)=o.hazards.as_ref().and_then(|h|h.light_source.as_ref()){color=crate::world_features::object(p,source,Some("light"),"*",color,x,y,now).color;}
            let color=crate::world_features::tone(p,color,material_bg,bg,context,o.chance,all_seeing,Some(p.number("terrain.feature:fire.lightingResponse")),false,true,true);
            crate::color::composite(bg,color,opacity)
        });
        vec![crate::render::Animation::Fire{phase,speed,colors}]
    }else{Vec::new()};
    Cell {
        x,
        y,
        glyph,
        emphasis,
        animations,
        fire: true,
        current: o.current,
        direct: o.direct,
        fg: crate::color::composite(bg, color, opacity),
        bg,
        label: if o.hellfire { if o.current { "Hellfire" } else { "Remembered Hellfire" } } else if o.current { "Fire" } else { "Remembered fire" }.into(),
        ..Cell::default()
    }
}

#[cfg(test)]
mod feeding_observation_tests {
    use super::*;
    #[test]
    fn bloody_wall_inspection_map_and_fog_use_observed_overlay_only() {
        let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=33.0.into();r.floors.retain(|f|f.depth==1.);r.actors_at_mut(1.).clear();
        r.player.x=10.;r.player.y=10.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let f=r.floor_mut().unwrap();let n=f.tiles.len();let w=f.width();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.lights=vec![100.;n];f.environment=corpse_core::Field::Missing;f.vents=corpse_core::Field::Missing;
        let at=10*w+11;f.tiles[at]="wall".into();let mut marks=vec![false;n];marks[at]=true;f.bloody_walls=marks.into();
        assert!(crate::inspection_ui::observe_tile(&r,11,10).title.is_empty());
        assert!(crate::inspection_ui::observe_tile(&r,11,10).lines.iter().any(|line| line.contains("Blood coats this solid wall.")));
        assert!(observe(&r).cells.iter().any(|c|c.x==11&&c.y==10&&c.label=="Bloody wall"));
        let mut mask=vec![false;n];mask[at]=true;corpse_core::hazard_memory::remember_observed_hazards(r.floor_mut().unwrap(),&mask,0.);
        let f=r.floor_mut().unwrap();f.remembered_tiles=f.tiles.clone().into();f.seen=vec![true;n];
        f.bloody_walls.as_mut().unwrap()[at]=false;
        for y in 1..f.height()-1 {f.tiles[y*w+20]="wall".into();}
        r.player.x=65.;r.player.y=20.;
        assert_eq!(crate::inspection_ui::observe_tile(&r,11,10).title,"Bloody wall");
        assert!(observe(&r).cells.iter().any(|c|c.x==11&&c.y==10&&c.label=="Bloody wall"));
        r.floor_mut().unwrap().remembered_hazards=corpse_core::Field::Missing;
        assert_ne!(crate::inspection_ui::observe_tile(&r,11,10).title,"Bloody wall");
    }
}

#[cfg(test)]
mod pocket_biome_observation_tests {
    use super::*;
    #[test]
    fn localized_nest_flowers_and_permafrost_render_without_replacing_stairs_or_leaking_fog() {
        let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=33.0.into();r.floors.retain(|f|f.depth==1.);r.actors_at_mut(1.).clear();
        r.player.x=10.;r.player.y=10.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let f=r.floor_mut().unwrap();let n=f.tiles.len();let w=f.width();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.lights=vec![100.;n];f.environment=corpse_core::Field::Missing;f.vents=corpse_core::Field::Missing;f.doors=corpse_core::Field::Missing;
        let a=10*w+11;let b=10*w+12;let c=10*w+13;
        f.tiles[b]="grass".into();
        f.pocket_biomes=vec![
            corpse_core::pocket_biomes::PocketBiome{theme:"saidribee-nest".into(),cells:vec![a],..Default::default()},
            corpse_core::pocket_biomes::PocketBiome{theme:"flower-grove".into(),cells:vec![b],flowers:vec![b],..Default::default()},
            corpse_core::pocket_biomes::PocketBiome{theme:"permafrost".into(),cells:vec![c],..Default::default()},
        ].into();let mut snow=vec![false;n];snow[c]=true;f.snow=snow.into();
        let o=corpse_core::vision::observe_terrain(&r).unwrap();let p=crate::appearance_profile::Profile::defaults();
        let nest=ground_profile(p,&o.cells[a],true);assert_eq!(nest.2,[66,43,19]);
        let flowers=ground_profile(p,&o.cells[b],true);assert_eq!(flowers.0,"*");assert_eq!(flowers.2,[18,46,32]);
        let frost=ground_profile(p,&o.cells[c],true);assert_ne!(frost.2,nest.2);assert_ne!(frost.2,flowers.2);
        let (rendered,_,_)=terrain_frame(&o,p,42,0.);assert!(rendered.iter().any(|c|c.x==12&&c.y==10&&c.glyph=="*"));
        // Actual Greater Zillennia erasure removes grass; known flowers disappear,
        // while an unseen last-observed grass tile retains the flower memory.
        {let f=r.floor_mut().unwrap();f.remembered_tiles=f.tiles.clone().into();f.seen=vec![true;n];}
        let mut z=corpse_core::population::zillennia_revision(21,33.0).unwrap();z.id=990.;z.x=12.;z.y=10.;z.dead=false.into();r.actors_at_mut(1.).push(z);
        corpse_core::zillennia_ground::on_arrival(&mut r,990.);
        let cleared=corpse_core::vision::observe_terrain(&r).unwrap();assert!(!cleared.cells[b].flowers);assert_ne!(ground_profile(p,&cleared.cells[b],true).0,"*");
        {let f=r.floor_mut().unwrap();for y in 1..f.height()-1 {f.tiles[y*w+20]="wall".into();}}r.player.x=65.;r.player.y=20.;
        let remembered=corpse_core::vision::observe_terrain(&r).unwrap();assert!(remembered.cells[b].flowers);assert_eq!(ground_profile(p,&remembered.cells[b],true).0,"*");
        let f=r.floor_mut().unwrap();f.seen=vec![false;n];f.remembered_tiles=corpse_core::Field::Missing;
        let hidden=corpse_core::vision::observe_terrain(&r).unwrap();assert!(hidden.cells[b].biome.is_none());assert!(!hidden.cells[b].flowers);
    }
}

#[cfg(test)]
mod epidite_ui_tests {
    use super::*;
    fn scene() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=34.0.into();r.player.x=10.;r.player.y=10.;
        r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.actors.clear();r.ground.clear();r.offers.clear();
        let f=r.floor_mut().unwrap();let n=f.tiles.len();let w=f.width();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.lights=vec![100.;n];f.environment=corpse_core::Field::Missing;f.vents=corpse_core::Field::Missing;f.doors=corpse_core::Field::Missing;
        f.seen=vec![true;n];f.tiles[10*w+13]="wall".into();f.remembered_tiles=f.tiles.clone().into();
        f.godbeast_event=corpse_core::GodbeastEvent{kind:"Epidite".into(),x:13.,y:10.,epidite_age:27.0.into(),..Default::default()}.into();r
    }
    #[test]
    fn epidite_ui_countdown_is_direct_observation_not_fog_or_render_time() {
        let mut r=scene();let before=r.clone();let view=observe(&r);
        assert_eq!(view.cells.iter().find(|c|c.x==13&&c.y==10).unwrap().subscript,"3");
        assert!(crate::inspection_ui::observe_tile(&r,13,10).scene.contains("3 turns until judgement"));
        let mut o=view.terrain_snapshot.as_ref().unwrap().cells[10*79+13].clone();o.direct=false;
        assert_eq!(observed_epidite_countdown(&r,&o,79),None);
        assert_eq!(r,before);
        r.floor_mut().unwrap().tiles[10*79+13]="chasm".into();o.direct=true;
        assert_eq!(observed_epidite_countdown(&r,&o,79),None);
    }
    #[test]
    fn epidite_ui_single_digit_boundary_and_exact_hover_clock() {
        for (age,remaining,subscript) in [(0.,30,""),(20.,10,""),(21.,9,"9"),(29.,1,"1"),(30.,30,"")] {
            let mut r=scene();r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().epidite_age=age.into();
            let before=r.clone();let view=observe(&r);
            assert_eq!(view.cells.iter().find(|c|c.x==13&&c.y==10).unwrap().subscript,subscript);
            assert!(crate::inspection_ui::observe_tile(&r,13,10).scene.contains(&format!("{remaining} turns until judgement")));
            let o=&view.terrain_snapshot.as_ref().unwrap().cells[10*79+13];
            assert_eq!(observed_epidite_countdown(&r,o,79),Some(remaining));assert_eq!(r,before);
            if [9,10].contains(&remaining) {
                if let Ok(dir)=std::env::var("CORPSE_NON_EUCLIDEAN_UI_REVIEW") {
                    std::fs::create_dir_all(&dir).unwrap();let mut view=view;
                    view.provenance="CONTROLLED Epidite clock fixture; native renderer; no turns simulated".into();
                    view.hover=Some((13,10));let mut painter=crate::render::Painter::new();
                    painter.game_at(&view,&crate::inspection_ui::observe_tile(&r,13,10).scene,false,824.);
                    let file=std::fs::File::create(format!("{dir}/epidite-{remaining}-turns-controlled.png")).unwrap();
                    let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);
                    encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
                }
            }
        }
        let mut r=scene();r.player.hp=500.;r.player.max_hp=500.;
        r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().epidite_age=29.0.into();
        corpse_core::epidite::world_turn(&mut r).unwrap();
        assert_eq!(corpse_core::epidite::countdown(r.floor().unwrap().godbeast_event.as_ref().unwrap()),30);
        assert!(observe(&r).cells.iter().find(|c|c.x==13&&c.y==10).unwrap().subscript.is_empty());
        assert!(crate::inspection_ui::observe_tile(&r,13,10).scene.contains("30 turns until judgement"));
    }
    #[test]
    fn epidite_ui_blessed_hearts_have_one_label_and_doubled_observed_rewards() {
        let r=scene();let rat=corpse_core::population::create_actor_revision("rat",66.,1.,1.,false,34.).unwrap();
        let mut heart=corpse_core::inventory::heart_for(&rat,77.).unwrap();let old=heart.clone();
        assert!(corpse_core::epidite::bless(&mut heart));assert!(!corpse_core::epidite::bless(&mut heart));
        let name=crate::pack::displayed_name(&r,&heart);assert_eq!(name.matches("Blessed").count(),1);
        assert_eq!(heart.exp.value_or(0.),old.exp.value_or(0.)*2.);
        let description=crate::pack::describe_item(&r,&heart);
        assert!(description.contains("Blessed"));
        assert!(description.contains(&format!("{} blessings",heart.exp.value_or(0.))));
    }
    #[test]
    fn epidite_ui_actual_renderer_subscript_and_review() {
        let r=scene();let mut view=observe(&r);let mut p=crate::render::Painter::new();
        p.game_at(&view,"",false,824.);let with_timer=p.pixels.clone();
        for c in &mut view.cells {c.subscript.clear();}
        p.game_at(&view,"",false,824.);assert_ne!(p.pixels,with_timer);
        if let Ok(directory)=std::env::var("CORPSE_DREAM_UI_REVIEW") {
            p.game_at(&observe(&r),"",false,824.);
            let file=std::fs::File::create(format!("{directory}/epidite-countdown.png")).unwrap();
            let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
            encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
}

#[cfg(test)]mod idle_fire_descriptor_tests{
 use super::*;
 /// O(observed cells + painted pixels), detached idle frames never mutate canonical state/RNG.
 #[test]fn current_fire_default_animates_from_cached_descriptor_but_memory_and_zero_speed_do_not(){
    let r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let before=serde_json::to_vec(&r).unwrap();let mut v=observe(&r);let mut o=v.terrain_snapshot.as_ref().unwrap().cells.iter().find(|o|o.current&&o.tile.is_some()).unwrap().clone();o.current=true;o.direct=true;o.knowledge="direct".into();o.chance=100.;o.direct_chance=100.;
    let p=crate::appearance_profile::Profile::defaults();assert!(!crate::appearance_light::animated(p));assert!(!crate::appearance_material::animated(p));
    let make=|profile:&crate::appearance_profile::Profile,o:&corpse_core::vision::ObservedCell|fire_cell(profile,o,false,crate::color::Light::ambient(100.),[20,15,20],[20,15,20],v.width,17,31,0.);
    let cell=make(p,&o);assert!(matches!(cell.animations.as_slice(),[crate::render::Animation::Fire{speed,..}] if *speed==1.));
    let mut remembered=o.clone();remembered.current=false;remembered.direct=false;assert!(make(p,&remembered).animations.is_empty());
    let stopped=p.set("terrain.feature:fire.animationSpeed",serde_json::json!(0)).unwrap();assert!(make(&stopped,&o).animations.is_empty());
    // No terrain snapshot is available: painting cannot run terrain_frame/lighting reconstruction.
    v.cells=vec![cell];v.terrain_count=0;v.terrain_snapshot=None;let mut painter=crate::render::Painter::new();painter.game_at(&v,"Controlled cached fire descriptor",false,100.);let first=painter.pixels.clone();painter.game_at(&v,"Controlled cached fire descriptor",false,700.);assert_ne!(painter.pixels,first);painter.game_at(&v,"Controlled cached fire descriptor",false,0.);let reduced=painter.pixels.clone();painter.game_at(&v,"Controlled cached fire descriptor",false,0.);assert_eq!(painter.pixels,reduced);assert_eq!(serde_json::to_vec(&r).unwrap(),before);
 }
}

#[cfg(test)]
mod idle_material_review {
    use super::*;
    /// O(N) fixture preparation, no gameplay dispatch; this is a controlled actual Run.
    fn scene() -> Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.player.x=12.;r.player.y=14.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        for group in r.actors.values_mut() {*group=std::sync::Arc::new(Vec::new());}
        for group in r.ground.values_mut() {*group=std::sync::Arc::new(Vec::new());}r.offers.clear();
        let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.lights=vec![100.;n];f.seen=vec![true;n];f.environment=corpse_core::Field::Missing;
        f.doors=corpse_core::Field::Missing;f.vents=corpse_core::Field::Missing;
        for y in 10..19 {f.tiles[y*w+18]="wall".into();}
        for y in 11..18 {for x in 7..10 {f.tiles[y*w+x]="water".into();}}
        f.remembered_tiles=f.tiles.clone().into();
        for (x,y) in [(14.,12.),(15.,14.),(14.,16.)] {corpse_core::environment::ignite(&mut r,x,y,Some(8.),None).unwrap();}
        r
    }
    /// O(P) PNG encoding only when the review output environment variable is set.
    fn png(p:&crate::render::Painter,dir:&str,name:&str){
        let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);
        enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
    /// O(N observation + P per frame); both idle frames reuse one detached View.
    #[test]
    fn controlled_flesh_cached_fire_review(){
        let r=scene();let before=serde_json::to_vec(&r).unwrap();
        corpse_core::temporal::Engine::load(std::str::from_utf8(&before).unwrap()).expect("exported review fixture must pass ordinary storage admission");
        let profile=crate::appearance_profile::Profile::defaults();
        let mut view=observe_with_profile(&r,profile);
        assert!(view.cells.iter().any(|c| c.animations.iter().any(|a|matches!(a,crate::render::Animation::Fire{..}))));
        // Removing the detached reconstruction input proves these paints cannot rebuild terrain/light.
        view.terrain_snapshot=None;
        let dir=std::env::var("CORPSE_IDLE_MATERIAL_REVIEW").ok();
        if let Some(dir)=&dir {std::fs::create_dir_all(dir).unwrap();std::fs::write(format!("{dir}/controlled-flesh-fire.json"),&before).unwrap();}
        let mut p=crate::render::Painter::new();
        p.game_at(&view,"Controlled Flesh scene — same cached View, no turns",false,0.);let zero=p.pixels.clone();
        if let Some(dir)=&dir {png(&p,dir,"flesh-after-fire-0000");}
        p.game_at(&view,"Controlled Flesh scene — same cached View, no turns",false,500.);assert_ne!(p.pixels,zero);
        if let Some(dir)=&dir {png(&p,dir,"flesh-after-fire-0500");}
        p.game_at(&view,"Controlled Flesh scene — same cached View, no turns",false,0.);assert_eq!(p.pixels,zero);
        if let Some(dir)=&dir {png(&p,dir,"flesh-reduced-motion-0000");}
        let custom=profile.set("terrain.feature:fire.color",serde_json::json!("#00aaff")).unwrap();
        let mut custom_view=observe_with_profile(&r,&custom);custom_view.terrain_snapshot=None;p.profile=custom;
        p.game_at(&custom_view,"Controlled explicit custom fire color",false,500.);
        if let Some(dir)=&dir {png(&p,dir,"flesh-custom-fire");}
        if let Some(dir)=&dir {
            // Existing actual scene, no new generation or visibility changes.
            p.profile=profile.clone();
            let lines=history(&r);
            let mut shutter=crate::chronicle_overlay::Shutter::default();shutter.open(0.,false);
            for time in [110.,220.] {
                p.game_at(&view,"Controlled Flesh scene — Chronicle overlay",false,0.);
                crate::chronicle_overlay::paint(&mut p,&lines,&shutter,time,false);
                png(&p,dir,&format!("chronicle-over-game-{}",time as u32));
            }
            p.menu(18,0,false,"Avatar");p.menu_seed("",false,false);png(&p,dir,"menu-seed-blank");
            p.menu(18,crate::menu_ui::SEED_INDEX,false,"Avatar");p.menu_seed("00-09-10:example",true,false);png(&p,dir,"menu-seed-focused");
        }
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}

#[cfg(test)]
mod generated_palette_review {
    use super::*;
    /// O(three real floor generations + three observations + painted pixels), opt-in QA only.
    /// No terrain/entity edits, visibility reveal, RNG override or turn dispatch.
    #[test]
    fn generated_revision39_normal_visibility_palette_review(){
        let Ok(dir)=std::env::var("CORPSE_GENERATED_PALETTE_REVIEW") else {return;};
        std::fs::create_dir_all(&dir).unwrap();
        let profile=crate::appearance_profile::Profile::defaults();
        let mut receipt=Vec::new();
        for (index,seed) in ["00-09-10:native-parity-v1","00-09-10:menu-seed-review","00-09-10:palette-review-3"].into_iter().enumerate(){
            let r=corpse_core::startup::create_run_revision(seed,"Avatar",false,39.).unwrap();
            let raw=serde_json::to_vec(&r).unwrap();
            corpse_core::temporal::Engine::load(std::str::from_utf8(&raw).unwrap()).unwrap();
            let view=observe_with_profile(&r,profile);
            let mut materials=std::collections::BTreeMap::<String,usize>::new();
            if let Some(terrain)=&view.terrain_snapshot {for o in &terrain.cells {
                if o.current {if let Some(tile)=&o.tile {*materials.entry(tile.clone()).or_default()+=1;}}
            }}
            let mut painter=crate::render::Painter::new();
            // Match --render-game fixture CLI exactly for a true baseline binary comparison.
            let mut view=view;view.measure_conditions(&mut crate::statuses_ui::ConditionMeters::default());
            painter.game_at(&view,"Native migration test scene; port coverage incomplete.",false,0.);
            let stem=format!("generated-r39-{index}");
            std::fs::write(format!("{dir}/{stem}.json"),&raw).unwrap();
            let file=std::fs::File::create(format!("{dir}/{stem}-after.png")).unwrap();
            let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);
            encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
            assert_eq!(serde_json::to_vec(&r).unwrap(),raw);
            receipt.push(serde_json::json!({"fixture":stem,"seed":seed,"simulationRevision":39,"currentVisibleMaterials":materials,"observedCreatures":view.creatures.len(),"editedTerrain":false,"editedActors":false,"revealedCells":false,"turnsDispatched":0,"renderer":"native Linux software renderer; Windows runtime not tested"}));
        }
        std::fs::write(format!("{dir}/generated-review-receipt.json"),serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}

#[cfg(test)]
fn terrain_frame_palette_json_reference(
    observation: &corpse_core::vision::TerrainObservation,
    profile: &crate::appearance_profile::Profile,
    seed: u32,
    now: f64,
    flesh: bool,
) -> (Vec<Cell>, Vec<Rgb>, Vec<crate::color::Light>) {
    let width = observation.width;
    let height = observation.height;
    let _terrain_span=corpse_core::timing::span("desktop.observe.terrain_frame");
    let mut light = crate::appearance_light::light_field(observation, profile, now);
    // Frozen source-v48 parity uses the historical color pipeline explicitly.
    if !flesh { for light in &mut light { light.display = None; } }
    let _style_span=corpse_core::timing::span("desktop.observe.terrain_style_cells");
    let quiet_chasm_edges = flesh && flesh_palette::brogue_chasm_edges_enabled(profile);
    // Separate diagnostic mode. 0=off, 1=all cells; >1=sparse grid-index sample.
    // Parse once per frame, never per profile lookup. No simulation/cache mutation.
    #[cfg(feature="runtime-timing")]
    let style_stride=std::env::var("OBSERVE_STYLE_STRIDE").ok().and_then(|x|x.parse::<usize>().ok()).unwrap_or(0);
    #[cfg(not(feature="runtime-timing"))]
    let style_stride=0usize;
    #[cfg(feature="runtime-timing")]
    let style_offset=std::env::var("OBSERVE_STYLE_OFFSET").ok().and_then(|x|x.parse::<usize>().ok()).unwrap_or(0);
    #[cfg(not(feature="runtime-timing"))]
    let style_offset=0usize;
    let mut cells = Vec::new();
    let mut backgrounds = vec![if flesh {crate::illumination_ui::unexplored(profile)} else {profile.rgb("fog.unexplored")}; width * height];
    for o in &observation.cells {
        let Some(tile) = o.tile.as_deref().map(visual_tile) else {
            continue;
        };
        corpse_core::timing::count(corpse_core::timing::Count::KnownCells,1);
        let x = o.index % width;
        let y = o.index / width;
        let knowledge = knowledge(&o.knowledge);
        let style_sample=style_stride>0 && o.index%style_stride==style_offset%style_stride;
        let ground_span=corpse_core::timing::sampled_span("desktop.style_sample.ground",style_sample);
        let (mut glyph, fg, mut bg, fixture, stone) = ground_profile(profile, o, flesh);
        drop(ground_span);
        let mut important = matches!(tile, "up" | "down" | "pit" | "chasm") || fixture;
        let ctx = crate::color::Context {
            knowledge,
            tile,
            distance: (o.distance / (1. + light[o.index].baseline().illumination.min(0.) / 100.).max(0.05))
                .min(3. + 21. * (1. - ((o.direct_chance - 5.) / 95.).clamp(0., 1.).sqrt()))
                as f32 as f64,
            light: light[o.index],
            important,
        };
        let neighbor_span=corpse_core::timing::sampled_span("desktop.style_sample.neighbors",style_sample);
        if !fixture
            && !o.snow
            && matches!(
                tile,
                "floor" | "grass" | "tree" | "water" | "shallow-water"
            )
        {
            let mut neighbors = Vec::new();
            for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let other = &observation.cells[ny as usize * width + nx as usize];
                if !o.current && other.current {
                    continue;
                }
                if other.tile.as_deref().map(visual_tile).is_some_and(|t| {
                    matches!(
                        t,
                        "floor" | "grass" | "tree" | "water" | "shallow-water"
                    )
                }) && !other.snow
                {
                    let (_, _, c, f, _) = ground_profile(profile, other, flesh);
                    if !f && c != bg {
                        neighbors.push((visual_tile(other.tile.as_deref().unwrap()), c));
                    }
                }
            }
            if !neighbors.is_empty() {
                let weight = (neighbors.len() as f64
                    * (0.1 + crate::color::noise(x as f64, y as f64, seed, 6.) * 0.02))
                    .min(0.18);
                let bank = matches!(tile, "water" | "shallow-water")
                    && neighbors
                        .iter()
                        .any(|(t, _)| !matches!(*t, "water" | "shallow-water"));
                bg = std::array::from_fn(|i| {
                    let target = if bank {
                        if flesh { [123., 39., 56.][i] } else { [18., 69., 106.][i] }
                    } else {
                        neighbors.iter().map(|(_, c)| c[i] as f64).sum::<f64>()
                            / neighbors.len() as f64
                    };
                    (bg[i] as f64 * (1. - weight) + target * weight).round() as u8
                });
            }
        }
        drop(neighbor_span);
        let material_span=corpse_core::timing::sampled_span("desktop.style_sample.material_prepare",style_sample);
        let material = displayed_material_settings(profile, o, flesh);
        let original_material = material_settings(profile, o);
        let default_material = material_settings(crate::appearance_profile::Profile::defaults(), o);
        let fg = flesh_palette::wall_ink(fg, o, observation,
            flesh && original_material["color"] == default_material["color"]);
        bg = flesh_palette::veins(bg, tile, x, y, seed, flesh && !fixture && !o.snow && o.biome.is_none()
            && original_material["background"] == default_material["background"]);
        drop(material_span);
        let color_span=corpse_core::timing::sampled_span("desktop.style_sample.color_math",style_sample);
        let (mut fg, mut bg, source_material_background) = crate::color_profile::terrain_material(
            profile,
            fg,
            bg,
            ctx,
            x,
            y,
            seed,
            fixture,
            stone,
            &material,
            o.chance,
            observation.all_seeing,
            (flesh && o.direct && !fixture && matches!(tile, "water" | "shallow-water"))
                .then(|| flesh_palette::water_illumination(profile, fg, bg, tile, ctx.light)),
        );
        let pulse = crate::entity_profile::material_pulse(
            material["animationSpeed"].as_f64().unwrap_or(1.),
            now,
            x,
            y,
        );
        if pulse != 1. {
            fg = crate::color_profile::grade(fg, pulse, 1., 1.);
            bg = crate::color_profile::grade(bg, pulse, 1., 1.);
        }
        drop(color_span);
        let feature_span=corpse_core::timing::sampled_span("desktop.style_sample.features",style_sample);
        let mut underlays = Vec::new();
        let mut glyph_opacity = 1.;
        let mut glyph_emphasis = 1.;
        macro_rules! detail {
            ($detail:expr,$important:expr,$emissive:expr) => {{
                let d = $detail;
                glyph = d.glyph;
                fg = crate::world_features::tone(
                    profile,
                    d.color,
                    source_material_background,
                    bg,
                    ctx,
                    o.chance,
                    observation.all_seeing,
                    d.response,
                    false,
                    $important,
                    $emissive,
                );
                glyph_opacity = d.opacity;
                glyph_emphasis = d.emphasis;
                if let Some((c, a)) = d.background {
                    let color = crate::world_features::tone(
                        profile,
                        c,
                        source_material_background,
                        bg,
                        ctx,
                        o.chance,
                        observation.all_seeing,
                        d.response,
                        true,
                        false,
                        false,
                    );
                    bg = crate::color::composite(bg, color, a);
                }
            }};
        }
        let mut arrow = String::new();
        let mut animations = Vec::new();
        let energy = if matches!(tile, "wall" | "tree") {
            ctx.light.wall_energy
        } else {
            ctx.light.energy
        };
        let peak = energy.into_iter().fold(0f64, f64::max);
        let burning = o
            .hazards
            .as_ref()
            .is_some_and(|h| h.fire.as_ref().is_some() || h.permanent_fire.value_or(false));
        if o.direct && ctx.light.display.is_none() && peak > 0.08 && !matches!(tile, "chasm" | "pit") && !burning {
            animations.push(crate::render::Animation::Reflection {
                phase: crate::color::noise(x as f64, y as f64, seed, 10.) * std::f64::consts::PI,
                color: energy.map(|v| crate::color::from_linear(v / peak * 0.35).round() as u8),
            });
        }

        if o.direct && matches!(tile, "water" | "shallow-water")
            && crate::appearance_material::idle_overlay_enabled(profile,tile) {
            animations.push(crate::render::Animation::Water {
                phase: crate::color::noise(x as f64, y as f64, seed, 8.) * std::f64::consts::PI,
                speed: profile.number(&format!("terrain.{tile}.animationSpeed")),
            });
        }
        if let Some(h) = &o.hazards {
            if h.razorleaves.value_or(false) {
                razorleaf_appearance(profile,o,x,y,seed,source_material_background,&mut bg,ctx,observation.all_seeing,&mut animations);
            }
            if tile == "wall" && h.bloody_wall.value_or(false) {
                let stain=crate::world_features::tone(profile,[206,34,61],source_material_background,bg,ctx,o.chance,observation.all_seeing,None,true,false,false);
                fg=crate::color::composite(fg,stain,0.75);
            }
            if matches!(tile,"water"|"shallow-water") && !h.ichor.value_or(false) {
                if let Some(amount)=h.water_blood.as_ref().filter(|v|**v>0.) {
                    let alpha=(0.20+0.65*(amount/100.).sqrt()).clamp(0.,0.85);
                    let stain=crate::world_features::tone(profile,[206,34,61],source_material_background,bg,ctx,o.chance,observation.all_seeing,None,true,false,false);
                    let dark=crate::world_features::tone(profile,[104,20,39],source_material_background,bg,ctx,o.chance,observation.all_seeing,None,true,false,false);
                    fg=crate::color::composite(fg,stain,alpha);
                    bg=crate::color::composite(bg,dark,alpha);
                }
            }
            for (enabled, key, alpha) in [
                (h.slime.value_or(false), "feature:slime", 0.22),
                (h.mud.value_or(false), "feature:mud", 0.24),
                (h.ichor.value_or(false), "feature:ichor", if flesh { 0.78 } else { 0.28 }),
            ] {
                if enabled {
                    let (c, alpha) =
                        crate::world_features::overlay(profile, key, x, y, seed, now, alpha);
                    let response = profile
                        .value(&format!("terrain.{key}.lightingResponse"))
                        .and_then(|v| v.as_f64());
                    let c = crate::world_features::tone(
                        profile,
                        c,
                        source_material_background,
                        bg,
                        ctx,
                        o.chance,
                        observation.all_seeing,
                        response,
                        true,
                        false,
                        false,
                    );
                    bg = crate::color::composite(bg, c, alpha);
                    if flesh && key == "feature:ichor" {
                        let (ichor, _) = crate::world_features::feature_surface(
                            profile, key, x, y, seed);
                        fg = crate::world_features::tone(profile, ichor, source_material_background,
                            bg, ctx, o.chance, observation.all_seeing, response, false, false, false);
                    }
                    let keypath = format!("terrain.{key}.glyph");
                    if profile.value(&keypath)
                        != crate::appearance_profile::Profile::defaults().value(&keypath)
                    {
                        let overlay_glyph = profile.text(&keypath).to_string();
                        let (c, _) = crate::world_features::feature_surface(
                            profile, key, x, y, seed,
                        );
                        let overlay_color = crate::world_features::tone(
                            profile,
                            c,
                            source_material_background,
                            bg,
                            ctx,
                            o.chance,
                            observation.all_seeing,
                            response,
                            false,
                            false,
                            false,
                        );
                        underlays.push((
                            overlay_glyph,
                            overlay_color,
                            profile.number(&format!("terrain.{key}.opacity")),
                        ));
                    }
                }
            }
            if let Some(gas) = h.gas.as_ref() {
                let key = format!("gas:{}", gas.r#type);
                let fallback=(matches!(gas.r#type.as_str(),"hallucinogenic"|"bloodMist") && profile.material(&key,None).is_none())
                    .then(|| {let color=gas_color(&gas.r#type);(color,color)});
                let (_, c) =
                    crate::appearance_material::feature_colors(profile, &key, x, y, seed, fallback);
                let c = crate::color::gas_field(c);
                let opacity = profile
                    .value(&format!("terrain.{key}.opacity"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.);
                let response = profile
                    .value(&format!("terrain.{key}.lightingResponse"))
                    .and_then(|v| v.as_f64());
                let c = crate::world_features::tone(
                    profile,
                    c,
                    source_material_background,
                    bg,
                    ctx,
                    o.chance,
                    observation.all_seeing,
                    response,
                    true,
                    false,
                    false,
                );
                if o.direct && crate::appearance_material::idle_overlay_enabled(profile,&key) {
                    animations.push(crate::render::Animation::Gas {
                        phase: crate::color::noise(x as f64, y as f64, seed, 8.)
                            * std::f64::consts::PI,
                        color: c,
                    });
                }
                bg = crate::color::composite(bg, c, 0.26 * opacity);
            }
            if tile != "wall"
                && (h.permanent_fire.value_or(false)
                    || h.fire.as_ref().is_some_and(|f| f != "embers"))
            {
                let field = if profile.value("terrain.feature:fire.background")
                    == crate::appearance_profile::Profile::defaults()
                        .value("terrain.feature:fire.background")
                {
                    profile.rgb("palette.fire.background")
                } else {
                    profile.rgb("terrain.feature:fire.background")
                };
                let c = crate::world_features::tone(
                    profile,
                    field,
                    source_material_background,
                    bg,
                    ctx,
                    o.chance,
                    observation.all_seeing,
                    None,
                    true,
                    false,
                    false,
                );
                bg = crate::color::composite(
                    bg,
                    c,
                    0.25 * profile.number("terrain.feature:fire.opacity"),
                );
            }
            if h.blood.value_or(false) && (glyph == "." || (flesh && glyph == ",")) {
                detail!(
                    crate::world_features::blood_dot(
                        profile,
                        &glyph,
                        if flesh { [232, 47, 64] } else { [255, 23, 76] },
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
            }
            if let Some(root) = h.root.as_ref() {
                detail!(
                    crate::world_features::material(
                        profile,
                        "feature:roots",
                        &root.glyph,
                        hex(&root.color),
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if let Some(vent) = h.vent.as_ref() {
                let key = format!("terrain.gas:{}.color", vent.gas);
                let color = profile
                    .value(&key)
                    .and_then(|v| v.as_str())
                    .map(hex)
                    .unwrap_or_else(|| gas_color(&vent.gas));
                detail!(
                    crate::world_features::object(profile, "vent", None, "=", color, x, y, now),
                    false,
                    false
                );
                important = true;
            }
            if let Some(feature) = h.feature.as_ref() {
                detail!(
                    crate::world_features::object(
                        profile,
                        &feature.name,
                        None,
                        &feature.glyph,
                        hex(&feature.color),
                        x,
                        y,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if let Some(source) = h.light_source.as_ref() {
                glyph = "*".into();
                if o.direct && tile == "wall" && crate::appearance_material::idle_torch_enabled(profile,source) {
                    animations.push(crate::render::Animation::Torch);
                }
                important = true;
                detail!(
                    crate::world_features::object(
                        profile,
                        source,
                        Some("light"),
                        "*",
                        if source == "beacon" {
                            [182, 77, 255]
                        } else {
                            [255, 211, 74]
                        },
                        x,
                        y,
                        now
                    ),
                    true,
                    true
                );
            }
            if h.fire.as_ref().is_some_and(|f| f == "embers") && h.light_source.as_ref().is_none() {
                detail!(
                    crate::world_features::material(
                        profile,
                        "feature:embers",
                        "·",
                        [227, 38, 54],
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if h.bulwark.as_ref().is_some() {
                detail!(
                    crate::world_features::material(
                        profile,
                        "feature:ward",
                        "∷",
                        [86, 255, 55],
                        x,
                        y,
                        seed,
                        now
                    ),
                    false,
                    false
                );
                important = true;
            }
            if tile == "water" && !h.ichor.value_or(false) {
                if let Some(code) = h.current.as_ref() {
                    arrow = ["", "↑", "↗", "→", "↘", "↓", "↙", "←", "↖"]
                        .get(*code as usize)
                        .unwrap_or(&"")
                        .to_string();
                }
            }
        }
        drop(feature_span);
        let _final_span=corpse_core::timing::sampled_span("desktop.style_sample.edges_contrast_output",style_sample);
        let mut edges = [None; 4];
        for (d, (dx, dy)) in [(1, 0), (0, 1), (-1, 0), (0, -1)].into_iter().enumerate() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let other = &observation.cells[ny as usize * width + nx as usize];
            if !o.current && other.current {
                continue;
            }
            let Some(nt) = other.tile.as_deref().map(visual_tile) else {
                continue;
            };
            let mut brogue_chasm_edge = false;
            let edge = if o.biome.as_deref() == Some("corn-dimension") && other.biome.as_deref() != Some("corn-dimension") {
                Some(([201, 152, 236], if o.current { 0.55 } else { 0.24 }))
            } else if tile == "wall" && flesh_palette::wall_face_neighbor(o, other) {
                Some((
                    material["edgeColor"]
                        .as_str()
                        .filter(|c| !c.is_empty())
                        .map(hex)
                        .unwrap_or([197, 195, 211]),
                    if o.current { 0.4 } else { 0.21 },
                ))
            } else if matches!(tile, "water" | "shallow-water")
                && !matches!(
                    nt,
                    "wall" | "chasm" | "pit" | "water" | "shallow-water"
                )
            {
                let edge = if flesh {
                    material["edgeColor"].as_str().map(hex).unwrap_or([239, 137, 145])
                } else { [115, 239, 255] };
                Some((edge, if o.current { 0.21 } else { 0.14 }))
            } else if tile == "chasm" && !matches!(nt, "chasm" | "pit") {
                brogue_chasm_edge = flesh;
                Some((if flesh {flesh_palette::brogue_chasm(x,y,seed)}else{[170,145,220]}, if o.current { 0.62 } else { 0.46 }))
            } else if !matches!(tile, "wall" | "chasm" | "pit")
                && matches!(nt, "chasm" | "pit")
            {
                brogue_chasm_edge = flesh && nt == "chasm";
                Some((if flesh && nt == "chasm" {flesh_palette::brogue_chasm(nx as usize,ny as usize,seed)}else{[170,145,220]}, 0.28))
            } else {
                None
            };
            if let Some((c, a)) = edge {
                // Decorative void boundaries must not pass through the generic
                // important-symbol contrast rescue, which whitens Brogue ink.
                let quiet = brogue_chasm_edge && quiet_chasm_edges;
                let edge_bg = if quiet && tile == "chasm" { [0; 3] } else { bg };
                let ink = if quiet {
                    if o.current { c } else { crate::color::memory(c) }
                } else { crate::color_profile::map_colors(profile, c, bg, ctx).0 };
                edges[d] = Some(crate::color::composite(edge_bg, ink, a * material_number(profile, o, "opacity")));
            }
        }
        backgrounds[o.index] = bg;
        if profile.value("global") == crate::appearance_profile::Profile::defaults().value("global")
        {
            fg = crate::color::distinguish(
                fg,
                bg,
                if o.current {
                    if important {
                        2.8
                    } else if tile == "floor" {
                        1.4
                    } else if matches!(tile, "water" | "shallow-water") {
                        1.55
                    } else {
                        1.85
                    }
                } else {
                    1.45
                },
            );
        }
        let fg = crate::color::composite(
            bg,
            fg,
            crate::color::ink_alpha(tile, important)
                * material_number(profile, o, "opacity")
                * glyph_opacity
                * if o.current {
                    1.
                } else {
                    profile.number("fog.rememberedOpacity")
                },
        );
        let (fg,bg)=if flesh && tile == "chasm" && !fixture && !o.snow && !o.ice && !o.hellfire
            && glyph_opacity == 1. && flesh_palette::brogue_palette_enabled(profile) {
            let ink=if profile.is_default("terrain.chasm.color") {
                let color=flesh_palette::brogue_chasm(x,y,seed);
                if o.current {color} else {crate::color::memory(color)}
            }else{fg};
            let ground=if profile.is_default("terrain.chasm.background") {[0;3]}else{bg};
            (ink,ground)
        }else{(fg,bg)};
        backgrounds[o.index]=bg;
        cells.push(Cell {
            x,
            y,
            bottom_aligned: tile == "grass" && glyph == "\"" && !fixture && !o.snow
                && !o.ice && !o.puddles && !o.hellfire && !o.flowers,
            glyph,
            label: if o.hellfire { "Hellfire".into() } else if o.ice { "Ice".into() } else if o.puddles { "Puddle".into() } else if tile == "wall" && o.hazards.as_ref().is_some_and(|h|h.bloody_wall.value_or(false)) { "Bloody wall".into() } else { o.tile.clone().unwrap_or_default() },
            emphasis: glyph_emphasis,
            underlays,
            material_background: Some(source_material_background),
            current: o.current,
            direct: o.direct,
            fg,
            bg,
            arrow,
            edges,
            animations,
            ..Cell::default()
        });
    }
    (cells, backgrounds, light)
}

include!("appearance_ground_tests.rs");

#[cfg(test)]
#[path = "wall_flesh_palette_tests.rs"]
mod wall_flesh_palette_tests;

#[cfg(test)]
mod blood_dot_sequence_tests {
    use super::*;
    #[test]
    fn floor_blood_changes_dots_not_background_in_full_and_reference_paths() {
        let run=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let before=serde_json::to_string(&run).unwrap();
        let mut observation=corpse_core::vision::observe_terrain(&run).unwrap();observation.light_sources.clear();
        for c in &mut observation.cells {
            c.current=true;c.direct=true;c.knowledge="direct".into();c.chance=100.;c.direct_chance=100.;c.distance=0.;
            c.ambient=Some(0.);c.illumination=Some(0.);c.tile=Some("floor".into());c.substrate=None;
            c.door_open=None;c.grate=false;c.bridge=false;c.snow=false;c.entrance=false;c.hazards=None;
        }
        let at=211;
        let custom=crate::appearance_profile::Profile::default().set("terrain.feature:blood.background",serde_json::json!("#00ff00")).unwrap().set("terrain.feature:blood.glyph",serde_json::json!("X")).unwrap();
        for p in [crate::appearance_profile::Profile::default(),custom] {for flesh in [false,true] {for remembered in [false,true] {for now in [0.,713.] {
            let mut dry=observation.clone();if remembered {let c=&mut dry.cells[at];c.current=false;c.direct=false;c.knowledge="remembered".into();}
            let mut wet=dry.clone();wet.cells[at].hazards=Some(serde_json::from_value(serde_json::json!({"blood":true})).unwrap());
            let(dry_cells,_,_)=terrain_frame_palette(&dry,&p,17,now,flesh);let(wet_cells,_,_)=terrain_frame_palette(&wet,&p,17,now,flesh);
            let dry_cell=dry_cells.iter().find(|c|c.x==at%wet.width&&c.y==at/wet.width).unwrap();let wet_cell=wet_cells.iter().find(|c|c.x==at%wet.width&&c.y==at/wet.width).unwrap();
            assert_eq!(dry_cell.bg,wet_cell.bg,"floor blood must not tint any background");assert_eq!(dry_cell.glyph,wet_cell.glyph);assert_ne!(dry_cell.fg,wet_cell.fg);
            let(reference,_,_)=terrain_frame_palette_json_reference(&wet,&p,17,now,flesh);assert_eq!(wet_cells,reference,"compiled/reference parity");
        }}}}
        assert_eq!(serde_json::to_string(&run).unwrap(),before);
    }
}

#[cfg(test)] mod carried_inventory_projection_tests {
    use super::*;
    #[test] fn r50_inventory_projection_labels_carried_jewelry_active_without_mutation() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.inventory=vec![
            corpse_core::Item{id:900.,kind:"ring".into(),name:"Ring".into(),..Default::default()},
            corpse_core::Item{id:901.,kind:"amulet".into(),name:"Amulet".into(),equipped:"amulet".to_string().into(),..Default::default()},
            corpse_core::equipment::make_equipment("Bow",902.).unwrap()];
        let before=r.clone();let view=observe(&r);
        assert_eq!(&view.inventory[..2],["Ring [active]","Amulet [active]"]);
        assert_eq!(view.inventory[2],"Bow");assert_eq!(r,before);
        r.simulation_revision=46.0.into();let view=observe(&r);
        assert_eq!(&view.inventory[..2],["Ring","Amulet [equipped]"]);
    }
}

#[cfg(test)]
mod player_background_tests {
    use super::*;
    #[test]
    fn player_inherits_each_lit_terrain_background() {
        let run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let view = observe(&run);
        let mut observation = (*view.terrain_snapshot.unwrap()).clone();
        let at = run.player.y as usize * observation.width + run.player.x as usize;
        for tile in ["floor", "grass", "water", "shallow-water", "chasm"] {
            for illumination in [-100., 0., 100.] {
                let observed = observation.cells.iter_mut().find(|c| c.index == at).unwrap();
                observed.tile = Some(tile.into());
                observed.illumination = Some(illumination);
                observed.ambient = Some(illumination);
                for now in [0., 731.] {
                    let profile = crate::appearance_profile::Profile::defaults();
                    let (terrain, backgrounds, _) = terrain_frame(&observation, profile, 17, now);
                    let player = actor_cell_profile(profile, &run, &run.player, observation.width, &backgrounds, true);
                    let ground = terrain.iter().find(|c| c.x == player.x && c.y == player.y).unwrap();
                    assert_eq!(player.bg, ground.bg, "{tile}, light={illumination}, time={now}");
                    assert_eq!(player.background_overlay, None);
                }
            }
        }
    }
    #[test]
    fn explicit_player_background_is_still_respected() {
        let run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let profile = crate::appearance_profile::Profile::default()
            .set("entities.objects.player.inherited", serde_json::json!(false)).unwrap()
            .set("entities.objects.player.background", serde_json::json!("#123456")).unwrap();
        let cell = actor_cell_profile(&profile, &run, &run.player, 79, &vec![[30, 40, 50]; 79*29], true);
        assert_eq!(cell.background_overlay, Some(([0x12, 0x34, 0x56], 1.)));
    }
}

include!("grass_amulet_observe_tests.rs");

#[cfg(test)]
#[path = "player_glyph_tests.rs"]
mod player_glyph_tests;

#[cfg(test)]
#[path = "revision52_corpse_fall_observation_tests.rs"]
mod revision52_corpse_fall_observation_tests;

#[cfg(test)]
mod fast_mode_label_tests {
    #[test]
    fn observed_fast_label_is_detached_and_does_not_change_saved_mode() {
        let mut run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let normal=super::observe(&run);
        assert_eq!(normal.mode,run.mode);
        run.fast_mode=corpse_core::Field::Value(true);
        let v=super::observe(&run);
        assert_eq!(v.mode,"Fast mode");
        assert_eq!(run.mode,normal.mode);
    }
}

#[cfg(test)]
mod compressed_depth_label_tests {
    #[test]
    fn new_route_displays_twenty_floors_without_changing_animation_depth() {
        let mut run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.fast_mode=true.into();run.compressed_depths=true.into();
        let view=super::observe(&run);
        assert_eq!(view.depth,run.depth);assert_eq!(view.display_depth,1.);assert_eq!(view.max_depth,20);
        assert_eq!(view.mode,"Avatar");
        run.compressed_depths=false.into();
        let view=super::observe(&run);assert_eq!(view.max_depth,40);assert_eq!(view.mode,"Fast mode");
    }
}

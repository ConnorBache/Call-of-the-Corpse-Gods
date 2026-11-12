//! Observation-gated v48 tile/creature inspection. The painter sees only Report.
//! `observe_tile` is the sole admission boundary; remembered terrain and items
//! come from detached observation snapshots, never current hidden occupants.
use crate::{
    modal_ui::{self, Rect},
    pack,
    render::{Painter, GOLD, INK, MUTED},
    statuses_ui,
    typography_ui::{self, Role, RoleStyle},
};
use corpse_core::{
    ai, combat, engine, half_here, lifecycle, progression, travel, vision, Actor, Field, Floor,
    HazardSnapshot, Item, PartialStats, Run,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::OnceLock};

pub const BOUNDS: (i32, i32, i32, i32) = (24, 138, 520, 572);
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    #[serde(skip)]
    pub hallucination: Option<std::sync::Arc<[Report;6]>>,
    pub title: String,
    pub scene: String,
    pub lines: Vec<String>,
    pub creature: Option<CreatureReport>,
    pub details_open: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_hover: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CreatureReport {
    pub title: String,
    pub description: String,
    pub scene: String,
    pub behavior: String,
    pub resources: String,
    pub conditions: String,
    pub your_attack: String,
    pub their_attack: String,
    pub abilities: String,
    pub ability_details: String,
    pub heart: String,
    #[serde(default)]
    pub hover_resources: Vec<HoverResource>,
    #[serde(default)]
    pub hover_available: bool,
    #[serde(default)]
    pub bow_shot: String,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HoverResource {
    pub label: String,
    pub value: Option<f64>,
    pub maximum: Option<f64>,
    pub reference: f64,
    pub color: [u8; 3],
}
impl CreatureReport {
    fn report(self) -> Report {
        Report {
            hallucination: None,
            title: self.title.clone(),
            scene: self.scene.clone(),
            lines: [
                self.description.clone(),
                self.behavior.clone(),
                self.resources.clone(),
                self.conditions.clone(),
                self.your_attack.clone(),
                self.their_attack.clone(),
                self.abilities.clone(),
                self.heart.clone(),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect(),
            creature: Some(self),
            details_open: false,
            item_hover: Vec::new(),
        }
    }
}
/// Detached observation uncertainty, never canonical log or inventory text.
fn question_report(r:&mut Report){
 let q=crate::hallucination_ui::question;q(&mut r.title);q(&mut r.scene);for line in &mut r.lines{q(line);}for line in &mut r.item_hover{q(line);}
 if let Some(c)=r.creature.as_mut(){for text in [&mut c.title,&mut c.description,&mut c.scene,&mut c.behavior,&mut c.resources,&mut c.conditions,&mut c.your_attack,&mut c.their_attack,&mut c.abilities,&mut c.ability_details,&mut c.heart,&mut c.bow_shot]{q(text);}}
}
pub fn apply_hallucination(r:&mut Report,phase:usize){if let Some(cache)=r.hallucination.clone(){if r.title==cache[phase].title{return;}let open=r.details_open;*r=cache[phase].clone();r.details_open=open;r.hallucination=Some(cache);}}
fn has(a: &Actor, id: &str) -> bool {
    a.abilities.iter().any(|s| s == id)
}
fn cond(a: &Actor, key: &str) -> f64 {
    engine::condition(a, key)
}
fn ability(a: &Actor, name: &str) -> bool {
    a.abilities.iter().any(|id| {
        progression::catalog()
            .iter()
            .any(|d| d.id == *id && d.name == name)
    })
}
fn active(a: &Actor, name: &str) -> bool {
    a.active.iter().any(|id| {
        progression::catalog()
            .iter()
            .any(|d| d.id == *id && d.name == name)
    })
}
fn owner<'a>(r: &'a Run, a: &'a Actor) -> &'a Actor {
    engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a)
}
fn base_name(s: &str) -> &str {
    s.strip_prefix("Mercuria’s ")
        .or_else(|| s.strip_prefix("Mercuria's "))
        .unwrap_or(s)
}
#[derive(Deserialize)]
struct Definition {
    id: String,
    name: String,
    description: String,
    #[serde(rename = "ichorFamily")]
    ichor_family: bool,
    #[serde(rename = "heartOrder")]
    heart_order: Vec<String>,
}
#[derive(Deserialize)]
struct Special {
    name: String,
    description: String,
    zero: String,
}
#[derive(Deserialize)]
struct Copy {
    authored: Vec<Definition>,
    special: Vec<Special>,
    wendigo: Vec<String>,
}
fn copy() -> &'static Copy {
    static COPY: OnceLock<Copy> = OnceLock::new();
    COPY.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/inspection-copy-v48.json"))
            .expect("frozen inspection copy")
    })
}
fn definition(a: &Actor) -> Option<&'static Definition> {
    let raw = corpse_core::revision50_creature_identity::legacy_name(base_name(a.creature_kind.as_ref().unwrap_or(&a.name)));
    let name = if matches!(raw, "vent-fungus" | "Vent Fungus") {
        "bastidion"
    } else {
        raw
    };
    copy()
        .authored
        .iter()
        .find(|d| d.id == name || d.name == name)
}
fn creature_title(r: &Run, a: &Actor) -> String {
    let name=creature_title_base(a);
    if corpse_core::borge::is_marked_target(r,a) {format!("{name} · Borge-marked")} else {name}
}
fn creature_title_base(a: &Actor) -> String {
    if corpse_core::dreamwalking::is_body(a) { return "Your sleeping body".into(); }
    if corpse_core::monster_vampire::flying(a) {
        return "Vampire Bat".into();
    }
    if a.creature_kind.as_ref().is_some_and(|s| s == "wendigo") || has(a, "enemy:beast:wendigo") {
        return format!(
            "Wendigo level {}",
            a.conditions
                .get("monsterWendigoLevel")
                .copied()
                .unwrap_or(1.)
                .clamp(1., 5.)
        );
    }
    a.name.clone()
}
fn flavor(a: &Actor) -> String {
    if a.creature_kind.as_ref().is_some_and(|kind| kind=="ahvamitt") {
        return corpse_core::ahvamitt::DESCRIPTION.into();
    }
    if corpse_core::dreamwalking::is_nightmare(a) {
        return "A nightmare inhabiting the dream plane. It is always In mind, even when the waking world is out of sight.".into();
    }
    if corpse_core::dreamwalking::is_body(a) {
        return "Your sleeping body remains where you entered the dream. It shares your Health and Energy and is vulnerable to creatures in the waking world.".into();
    }
    match a.creature_kind.as_ref().map(String::as_str) {
        Some("night-crawler")=>return corpse_core::new_creatures::NIGHT_HISTORY.into(),
        Some("skin-walker")=>return corpse_core::new_creatures::SKIN_HISTORY.into(),
        Some("saidribee-queen")=>return corpse_core::new_creatures::QUEEN_HISTORY.into(),
        _=>{}
    }
    let name = if half_here::is_clone(a) {
        "Half Here"
    } else {
        corpse_core::revision50_creature_identity::legacy_name(base_name(&a.name))
    };
    let mut text = if let Some(s) = copy().special.iter().find(|s| s.name == name) {
        if a.energy == 0. {
            s.zero.clone()
        } else {
            s.description.clone()
        }
    } else if let Some(d) = definition(a) {
        if d.id == "wendigo" {
            let level = a
                .conditions
                .get("monsterWendigoLevel")
                .copied()
                .unwrap_or(1.)
                .clamp(1., 5.);
            if level.fract() == 0. {
                copy().wendigo[level as usize - 1].clone()
            } else {
                d.description.clone()
            }
        } else {
            d.description.clone()
        }
    } else {
        "An unfamiliar creature.".into()
    };
    if a.born_from_birther.value_or(false) {
        text.push_str(" It is covered in placenta.")
    };
    text
}
fn gas_label(kind: &str) -> &str {
    match kind {
        "caustic" => "Caustic gas",
        "breather" => "Gasser gas",
        "paralysis" => "Paralysis gas",
        "confusion" => "Confusion gas",
        "hallucinogenic" => "Hallucinogenic gas",
        "flammable" => "Flammable gas",
        "explosive" => "Explosive gas",
        "steam" => "Steam",
        "hydrogen" => "Hydrogen",
        _ => kind,
    }
}
/// Every dynamic detail is sourced from the one observed snapshot.
// Presentation only: remembered terrain and later feature-title overrides remain intact.
fn terrain_report_title(name: String, current: bool) -> String {
    if current && matches!(name.as_str(),
        "Ice" | "Puddle" | "Hellfire" | "Bloody wall" | "Rock wall" | "Stone wall" | "Wall" | "Bare ground" | "Stone floor" | "Rock floor" | "Earthen floor" | "Sand" | "Soot" | "Ash" | "Grass" | "Tree" | "Deep water" | "Shallow water" | "Chasm" | "Upstairs" | "Downstairs" | "Bottomless pit" | "Open door" | "Closed door" | "Grate" | "Bridge"
    ) { String::new() } else { name }
}
fn terrain_copy(f: &Floor, o: &vision::ObservedCell) -> (String, String) {
    let mut details = Vec::new();
    let h = o.hazards.as_ref();
    let tile = o.tile.as_deref();
    let natural = f
        .generation
        .as_ref()
        .and_then(|g| g.natural_walls.as_ref())
        .map_or_else(
            || {
                o.substrate
                    .as_ref()
                    .map_or(f.domain != "Dungeon", |s| s != "tiled")
            },
            |v| v.contains(&(o.index as f64)),
        );
    let ichor = if o.current {
        h.is_some_and(|h| h.ichor.value_or(false))
    } else {
        f.remembered_ichor
            .as_ref()
            .and_then(|v| v.get(o.index))
            .copied()
            .unwrap_or(0.)
            != 0.
    };
    let bridge = o.bridge && o.door_open.is_none() && !o.grate && tile == Some("floor") && !ichor;
    let key = if tile.is_none() {
        ""
    } else if let Some(open) = o.door_open {
        if open {
            "open-door"
        } else {
            "closed-door"
        }
    } else if o.grate {
        "grate"
    } else if bridge {
        "bridge"
    } else if tile == Some("wall") {
        if h.is_some_and(|h| h.bloody_wall.value_or(false)) {
            "bloody-wall"
        } else if natural {
            "natural-wall"
        } else {
            "stone-wall"
        }
    } else if tile == Some("floor") {
        o.substrate.as_deref().unwrap_or("floor")
    } else {
        tile.unwrap()
    };
    let key = if o.hellfire { "hellfire" } else if o.ice { "ice" } else if o.puddles { "puddle" } else { key };
    let (name, scene, memory) = match key {
        "ice" => ("Ice", "When a creature attempts to move off an ice tile, there is a 1/3 chance that it slips.", "ice"),
        "puddle" => ("Puddle", "a puddle sits on the ground.", "a puddle"),
        "hellfire" => ("Hellfire", "Hellfire is fire that never stops burning.", "Hellfire"),
        "bloody-wall" => ("Bloody wall", "Blood coats this solid wall. Hungry creatures can feed from beside it.", "a blood-coated wall"),
        "natural-wall" => (
            "Rock wall",
            "A wall of rough rock blocks the way.",
            "a wall of rough rock",
        ),
        "stone-wall" => (
            "Stone wall",
            "A wall of fitted stone blocks the way.",
            "a wall of fitted stone",
        ),
        "wall" => ("Wall", "A wall blocks the way.", "a wall blocking the way"),
        "speed-up" => ("Speed-up area", "Teal Speed-up tiles form ordered linked space.", "Speed-up tiles"),
        "floor" => ("Bare ground", "Bare ground lies underfoot.", "bare ground"),
        "tiled" => (
            "Stone floor",
            "Worn stone tiles cover the floor.",
            "worn stone tiles",
        ),
        "rock" => (
            "Rock floor",
            "Bare rock lies underfoot.",
            "a floor of bare rock",
        ),
        "earth" => (
            "Earthen floor",
            "Packed earth forms the floor.",
            "a floor of packed earth",
        ),
        "sand" => (
            "Sand",
            "Pale sand covers the ground.",
            "pale sand covering the ground",
        ),
        "soot" => ("Soot", "Soot blackens the ground.", "soot-blackened ground"),
        "ash" => ("Ash", "Ash blankets the ground.", "ash covering the ground"),
        "grass" => (
            "Grass",
            "Grass grows across the ground.",
            "grass growing across the ground",
        ),
        "tree" => (
            "Tree",
            "A tree rises from the ground.",
            "a tree growing here",
        ),
        "water" => (
            "Deep water",
            "Deep water fills this part of the dungeon.",
            "deep water",
        ),
        "shallow-water" => (
            "Shallow water",
            "Shallow water covers the ground.",
            "shallow water covering the ground",
        ),
        "chasm" => (
            "Chasm",
            "The ground falls away into a chasm.",
            "a chasm opening here",
        ),
        "up" => ("Upstairs", "A stairway leads up.", "a stairway leading up"),
        "down" => (
            "Downstairs",
            "A stairway leads deeper into the dungeon.",
            "a stairway leading deeper into the dungeon",
        ),
        "pit" => (
            "Bottomless pit",
            "A bottomless pit opens before you. Jump in to win.",
            "the bottomless pit. Jump in to win",
        ),
        "open-door" => (
            "Open door",
            "A wooden door stands open.",
            "an open wooden door",
        ),
        "closed-door" => (
            "Closed door",
            "A closed wooden door bars the way.",
            "a closed wooden door",
        ),
        "grate" => (
            "Grate",
            "A metal grate covers the opening.",
            "a metal grate",
        ),
        "bridge" => (
            "Bridge",
            "A bridge crosses the gap.",
            "a bridge crossing the gap",
        ),
        _ => ("Unclear memory", "", ""),
    };
    let scene = if o.current {
        scene.to_string()
    } else if memory.is_empty() {
        "You have been here before, but cannot recall the ground.".into()
    } else {
        format!("You remember {memory}.")
    };
    if tile.is_some() {
        if let Some(theme)=o.biome.as_deref() {
            details.push(match theme {
                "saidribee-nest"=>"This chamber belongs to the connected Saidribee Nest.",
                "permafrost"=>"Permafrost spreads through these connected chambers.",
                "flower-grove"=>"A flower-rich grove fills this pocket of the Flesh.",
                "mountainous"=>"Mountainous area",
                "corn-dimension"=>"Corn Dimension",
                _=>"",
            }.into());
        }
        if o.flowers {details.push("Colorful flowers grow here. They are decorative.".into());}
    }
    if tile.is_some() && o.puddles && (o.ice || o.hellfire) {
        details.push(if o.current { "a puddle sits on the ground." } else { "You remember a puddle." }.into());
    }
    if tile.is_some() && o.snow {
        details.push(
            if o.current {
                "A trail of snow lies here."
            } else {
                "A trail of snow lay here."
            }
            .into(),
        )
    }
    if let Some(h) = h {
        if let Some(amount)=h.water_blood.as_ref().filter(|v|**v>0.) {
            details.push(format!("{} blood clouds the water ({}%).",
                if o.current {"Diluting"} else {"Remembered"},js_round(*amount)));
        }
        if tile == Some("water") && !h.ichor.value_or(false) {
            if let Some(code) = h.current.as_ref() {
                let names = [
                    "still",
                    "north",
                    "northeast",
                    "east",
                    "southeast",
                    "south",
                    "southwest",
                    "west",
                    "northwest",
                ];
                if code.fract() == 0. && (1. ..=8.).contains(code) {
                    details.push(format!(
                        "A current {} {}.",
                        if o.current { "flows" } else { "flowed" },
                        names[*code as usize]
                    ))
                }
            }
        }
        for (present, now, past) in [
            (
                h.blood.value_or(false),
                "There is blood on the floor.",
                "There was blood on the floor.",
            ),
            (
                h.slime.value_or(false),
                "Green slime coats the ground.",
                "Green slime coated the ground.",
            ),
            (
                h.mud.value_or(false),
                "Mud covers the ground.",
                "Mud covered the ground.",
            ),
            (
                h.roots.value_or(false),
                "Fungal roots cross the floor.",
                "Fungal roots crossed the floor.",
            ),
            (
                h.ichor.value_or(false),
                "Mercuria’s ichor covers the ground.",
                "Mercuria’s ichor covered the ground.",
            ),
            (
                h.razorleaves.value_or(false),
                "Falling razorleaves fill the air.",
                "Falling razorleaves filled the air.",
            ),
        ] {
            if present {
                details.push(if o.current { now } else { past }.into())
            }
        }
        if let Some(n) = h.bulwark.as_ref() {
            details.push(if o.current {
                format!("Bulwark of Mind wards this square for {n} more turns.")
            } else {
                format!("Bulwark of Mind had {n} turns of protection left here.")
            })
        }
        if !o.hellfire && (h.permanent_fire.value_or(false) || h.fire.as_ref().is_some_and(|s| s != "embers")) {
            let kind = if h.fire.as_ref().is_some_and(|s| s == "ichor") {
                "Ichor fire"
            } else if h.permanent_fire.value_or(false) {
                "An undying fire"
            } else {
                "Fire"
            };
            details.push(format!(
                "{kind} {} here.",
                if o.current { "burns" } else { "burned" }
            ));
        } else if h.fire.as_ref().is_some_and(|s| s == "embers") {
            details.push(
                if o.current {
                    "Spent embers remain here."
                } else {
                    "Spent embers remained here."
                }
                .into(),
            )
        }
        if let Some(v) = h.vent.as_ref() {
            details.push(format!(
                "{} vent. {}",
                gas_label(&v.gas),
                if v.phase == "waiting" {
                    if o.current {
                        format!("Next burst in {} normal turns.", v.countdown)
                    } else {
                        format!("Its next burst was due in {} normal turns.", v.countdown)
                    }
                } else {
                    if o.current {
                        "Waiting for gas to clear."
                    } else {
                        "It was waiting for gas to clear."
                    }
                    .into()
                }
            ));
        }
        if let Some(g) = h.gas.as_ref() {
            details.push(format!(
                "{} {} in the air{}.",
                gas_label(&g.r#type),
                if o.current { "hangs" } else { "hung" },
                if g.concentration == 0. {
                    " in a lingering haze"
                } else {
                    ""
                }
            ));
        }
        if !o.current {
            if let Some(feature) = h.feature.as_ref() {
                details.push(format!("You last saw {} here.", feature.name));
                details.push(feature.description.replace("Walk into the lever to open the ichor well.","The lever was ready to open the ichor well.").replace("An ancient tree. Apply a tree tap while adjacent to collect Veridian Sap.","A tree tap could draw Veridian Sap from it.").replace("has ","had ").replace("rises ","rose ").replace("waits ","stood "));
            }
            if let Some(light) = h.light_source.as_ref() {
                details.push(
                    match light.as_str() {
                        "beacon" => "A magical beacon glowed here.",
                        "torch" => "A wall torch flickered here.",
                        _ => "A brazier burned here.",
                    }
                    .into(),
                );
            }
        }
    }
    let name = if !o.current {
        h.and_then(|h| h.feature.as_ref())
            .map(|f| f.name.clone())
            .unwrap_or(name.into())
    } else {
        name.into()
    };
    (
        name,
        std::iter::once(scene)
            .chain(details)
            .collect::<Vec<_>>()
            .join(" "),
    )
}
fn creature_scene(r: &Run, a: &Actor, o: Option<&vision::ObservedCell>) -> String {
    let name = creature_title(r, a);
    let subject = if name.starts_with("Mercuria’s ")
        || name.starts_with("Mercuria's ")
        || name.starts_with("Your ")
        || name.starts_with("The ")
        || ["Ursa Minor", "Ursa Major", "Bastidion"].contains(&name.as_str())
    {
        name.clone()
    } else if ["Half Here", "Mini-Me"].contains(&name.as_str()) {
        format!("Your {name}")
    } else {
        format!(
            "{} {}",
            if name
                .chars()
                .next()
                .is_some_and(|c| "aeiouAEIOU".contains(c))
            {
                "An"
            } else {
                "A"
            },
            name.to_lowercase().replace("zillennia", "Zillennia")
        )
    };
    let dead = a.dead.value_or(false) || a.hp <= 0.;
    let Some(o) = o.filter(|o| o.current) else {
        return format!("{subject} {}.", if dead { "lies still" } else { "is here" });
    };
    let h = o.hazards.as_ref();
    let has = |f: fn(&HazardSnapshot) -> &Field<bool>| h.is_some_and(|h| f(h).value_or(false));
    let ichor = has(|h| &h.ichor);
    let tile = o.tile.as_deref().unwrap_or("");
    let liquid = ichor || matches!(tile, "water" | "shallow-water");
    let terrain = if o.door_open.is_some() {
        "the threshold"
    } else if o.grate {
        "the grate"
    } else if ichor {
        "ichor"
    } else if o.bridge && tile == "floor" {
        "a bridge"
    } else if has(|h| &h.slime) {
        "green slime"
    } else if has(|h| &h.mud) {
        "mud"
    } else if has(|h| &h.roots) {
        "fungal roots"
    } else if o.hellfire {
        "Hellfire"
    } else if o.ice {
        "ice"
    } else if o.puddles {
        "a puddle"
    } else if o.snow && matches!(tile, "floor" | "grass") {
        "snow"
    } else if tile == "floor" {
        match o.substrate.as_deref() {
            Some("tiled") => "stone tiles",
            Some("rock") => "the rock",
            Some("earth") => "the soil",
            Some("sand") => "sand",
            Some("soot") => "soot-covered ground",
            Some("ash") => "ash",
            _ => "the ground",
        }
    } else {
        match tile {
            "grass" => "grass",
            "water" => "water",
            "shallow-water" => "shallow water",
            "chasm" => "a chasm",
            "up" | "down" => "the stairs",
            "pit" => "the bottomless pit",
            _ => "",
        }
    };
    if terrain.is_empty() {
        return format!("{subject} {}.", if dead { "lies still" } else { "is here" });
    }
    let verb = if dead {
        if liquid {
            "lies in"
        } else {
            "lies on"
        }
    } else if travel::flying(r, a) {
        if active(a, "Phase Through") || active(a, "Float Away") {
            "floats above"
        } else {
            "flies over"
        }
    } else if a.abilities.iter().any(|s| s == "enemy:beast:rooted") {
        "grows from"
    } else if tile == "water" && !ichor && cond(a, "underwater") > 0. {
        return format!("{subject} lurks beneath the water.");
    } else if (!a.dead.value_or(false) && tile == "water" && !ichor && !travel::flying(r, a))
        || (ichor && (crate_has_aquatic(a) || definition(a).is_some_and(|d| d.ichor_family)))
        || (tile == "shallow-water" && !ichor && crate_has_aquatic(a))
    {
        "swims through"
    } else if liquid {
        "wades through"
    } else {
        "stands on"
    };
    format!("{subject} {verb} {terrain}.")
}
fn crate_has_aquatic(a: &Actor) -> bool {
    has(a, "enemy:beast:aquatic")
}
fn compact_condition(s: &statuses_ui::Status) -> String {
    let mut name = statuses_ui::display_name(&s.name).to_string();
    if let Some((stat, n)) = name.split_once(' ') {
        if let Ok(n) = n.parse::<f64>() {
            let labels = match stat {
                "offense" => Some(("Stronger attacks", "Weaker attacks")),
                "defense" => Some(("Fortified", "More vulnerable")),
                "accuracy" => Some(("Steadier aim", "Impaired aim")),
                "dodge" => Some(("More evasive", "Less evasive")),
                "dr" => Some(("Reinforced armor", "Weakened armor")),
                "perception" => Some(("Keen senses", "Dulled senses")),
                "stealth" => Some(("Better concealed", "More conspicuous")),
                "maxEnergy" => Some(("Expanded Energy", "Reduced Energy")),
                "maxHp" => Some(("Bolstered Health", "Reduced Health")),
                _ => None,
            };
            if let Some((positive, negative)) = labels {
                name = if n > 0. { positive } else { negative }.into()
            }
        }
    }
    if let Some(d) = s.duration {
        format!("{name} ({d}t)")
    } else {
        name
    }
}
fn heart(r: &Run, a: &Actor) -> String {
    if a.heartless.value_or(false) {
        return "Heart: none".into();
    }
    let Ok(i) = corpse_core::inventory::heart_for_revision(a, 0., r.simulation_revision.value_or(29.)) else {
        return "Heart reward is not yet supported natively.".into();
    };
    let blessings = i.exp.value_or(0.);
    let mut parts = vec![format!(
        "{blessings} {}",
        if blessings == 1. {
            "blessing"
        } else {
            "blessings"
        }
    )];
    if let Some(b) = i.bonus.as_ref() {
        let mut values = stat_values(b).to_vec();
        if let Some(d) = definition(a) {
            let names = [
                ("Offense", "offense"),
                ("Defense", "defense"),
                ("Accuracy", "accuracy"),
                ("Dodge", "dodge"),
                ("damage reduction", "dr"),
                ("Perception", "perception"),
                ("maximum Energy", "maxEnergy"),
                ("maximum Health", "maxHp"),
            ];
            values.sort_by_key(|(name, _)| {
                d.heart_order
                    .iter()
                    .position(|k| names.iter().any(|(label, key)| label == name && *key == k))
                    .unwrap_or(usize::MAX)
            });
        }
        for (name, v) in values {
            if let Some(v) = v.as_ref().filter(|&&n| n != 0.) {
                let (name,v)=crate::stat_presentation::bonus(name,*v);
                parts.push(format!("{}{v} {name}", if v >= 0. { "+" } else { "" }));
            }
        }
    }
    parts.push(format!("{} Energy", i.energy.value_or(0.)));
    format!("Heart: {}", parts.join(" · "))
}
fn stat_values(b: &PartialStats) -> [(&str, &Field<f64>); 8] {
    [
        ("Offense", &b.offense),
        ("Defense", &b.defense),
        ("Accuracy", &b.accuracy),
        ("Dodge", &b.dodge),
        ("damage reduction", &b.dr),
        ("Perception", &b.perception),
        ("maximum Energy", &b.max_energy),
        ("maximum Health", &b.max_hp),
    ]
}
/// Internal observed-actor formatter. Caller must already admit the actor by
/// perception. `tile` permission remains independent from actor observation.
fn creature(r: &Run, a: &Actor, tile: Option<&vision::ObservedCell>) -> CreatureReport {
    let shown=crate::creature_presentation::project(r,a);
    let view=shown.as_ref().map(|a|crate::creature_presentation::projected_run(r,a));
    let r=view.as_ref().unwrap_or(r);
    let a=shown.as_ref().unwrap_or(a);
    creature_presented(r,a,tile,shown.is_some())
}
fn creature_presented(r:&Run,a:&Actor,tile:Option<&vision::ObservedCell>, projected:bool)->CreatureReport {
    let (your, their) = attacks(r, a);
    let owner = owner(r, a);
    let stats = combat::effective_stats(r, owner);
    let title = creature_title(r, a);
    let mut behavior: String = if corpse_core::dreamwalking::is_nightmare(a) {
        "In mind".into()
    } else if corpse_core::dreamwalking::is_body(a) {
        "Sleeping".into()
    } else if a.dead.value_or(false) || a.hp <= 0. {
        "Dead".into()
    } else if ai::sleeping(a) {
        "Sleeping".into()
    } else {
        let mut c = a.state.chars();
        c.next().map_or(String::new(), |f| {
            f.to_uppercase().collect::<String>() + c.as_str()
        })
    };
    if !projected && vision::directly_visible_actor(r,a) {
        if let Some(feeding)=corpse_core::creature_feeding::description(r,a) {
            behavior.push_str(&format!(" · {feeding}"));
        }
    }
    let illumination = tile
        .filter(|o| o.current)
        .and_then(|o| o.illumination)
        .map_or("unknown".into(), |n| js_round(n).to_string());
    let (abilities, ability_details) = crate::creature_abilities_ui::describe(r, a);
    CreatureReport {
        title,
        description: flavor(a),
        scene: creature_scene(r, a, tile),
        behavior: if corpse_core::dreamwalking::is_nightmare(a) { behavior }
            else if let Some(range)=crate::perception_ui::range_label(r,a) {
                let modifier=tile.and_then(|o|crate::perception_ui::tile_label(r,o)).map_or(String::new(),|s|format!(" · {s}"));
                format!("{behavior} · {range}{modifier}")
            }
            else { format!("{behavior} · Illumination {illumination}") },
        resources: format!(
            "{}Health {}/{} · Energy {}/{}",
            if owner.id != a.id {
                "Shared with you · "
            } else {
                ""
            },
            owner.hp.ceil(),
            owner.max_hp,
            owner.energy.ceil(),
            if ability(owner, "Boundless Hunger") {
                "∞".into()
            } else {
                stats.max_energy.ceil().to_string()
            }
        ),
        conditions: statuses_ui::observe(r, a)
            .iter()
            .map(compact_condition)
            .collect::<Vec<_>>()
            .join(" · "),
        your_attack: if !projected && corpse_core::ahvamitt::is_actor(a)
            && corpse_core::dreamwalking::same_plane(r, &r.player, a) {
            "Attack him to continue his game; he teleports away instead of taking ordinary attack damage.".into()
        } else { format_attack(&your, true) },
        their_attack: if !projected && corpse_core::ahvamitt::is_actor(a) {
            "Ahvamitt does not move or attack.".into()
        } else { format_attack(&their, false) },
        abilities,
        ability_details,
        heart: heart(r, a),
        hover_resources: hover_resources(r, a),
        hover_available: !a.dead.value_or(false),
        bow_shot: String::new(),
    }
}
fn hover_resources(r: &Run, a: &Actor) -> Vec<HoverResource> {
    if half_here::is_clone(a) || corpse_core::dreamwalking::is_body(a) {
        return vec![HoverResource {
            label: "Shares your Health and Energy".into(),
            ..Default::default()
        }];
    }
    let maximum = combat::effective_stats(r, a).max_energy;
    let boundless = ability(a, "Boundless Hunger");
    vec![
        if has(a, "enemy:beast:sphinx") {
            HoverResource {
                label: "Health · Invulnerable".into(),
                ..Default::default()
            }
        } else {
            HoverResource {
                label: "Health".into(),
                value: Some(a.hp),
                maximum: Some(a.max_hp),
                reference: a.max_hp,
                color: [255, 48, 85],
            }
        },
        HoverResource {
            label: "Energy".into(),
            value: Some(a.energy),
            maximum: (!boundless).then_some(maximum),
            reference: if boundless { 100. } else { maximum },
            color: [36, 223, 255],
        },
    ]
}
fn item_magic(i: &Item) -> bool {
    corpse_core::inventory::is_magical(i)
}
fn remembered_items<'a>(
    r: &'a Run,
    x: usize,
    y: usize,
    observation: &vision::TerrainObservation,
) -> Vec<(&'a Item, bool)> {
    let depth = r.depth.to_string();
    let width = observation.width;
    let mut ids = HashSet::new();
    let mut result = Vec::new();
    // Source deduplicates globally by identity, before filtering the chosen cell.
    if let Some(ground) = r.ground.get(&depth) {
        for g in ground.iter().filter(|g| crate::dream_ui::item_visible(r, &g.item)) {
            if observation
                .cells
                .get(g.y as usize * width + g.x as usize)
                .is_some_and(|o| o.current)
            {
                ids.insert(g.item.id.to_bits());
            }
        }
    }
    let sensitivity = half_here::origins(r)
        .iter()
        .any(|a| !a.dead.value_or(false) && cond(a, "magicSensitive") > 0.);
    if let Some(entries) = r.detected_items.as_ref().and_then(|m| m.get(&depth)) {
        for magical in [true, false] {
            for g in entries.iter().filter(|g| crate::dream_ui::item_visible(r, &g.item)) {
                if item_magic(&g.item) != magical {
                    continue;
                }
                if observation
                    .cells
                    .get(g.y as usize * width + g.x as usize)
                    .is_some_and(|o| !o.current)
                    && ids.insert(g.item.id.to_bits())
                    && g.x == x as f64
                    && g.y == y as f64
                {
                    result.push((&g.item, magical && sensitivity));
                }
            }
        }
    }
    if let Some(entries) = r.remembered_items.as_ref().and_then(|m| m.get(&depth)) {
        for g in entries.iter().filter(|g| crate::dream_ui::item_visible(r, &g.item)) {
            if observation
                .cells
                .get(g.y as usize * width + g.x as usize)
                .is_some_and(|o| !o.current)
                && ids.insert(g.item.id.to_bits())
                && g.x == x as f64
                && g.y == y as f64
            {
                result.push((&g.item, false));
            }
        }
    }
    result
}
/// O(ground entries + observed alias queries), O(matching items) references.
/// Caller must admit the observed cell before consulting live ground records.
fn observed_ground_at<'a>(r: &'a Run, observation: &vision::TerrainObservation, x: usize, y: usize) -> Vec<&'a corpse_core::GroundItem> {
    let Some(f) = r.floor() else { return Vec::new(); };
    if !observation.cells.get(y*f.width()+x).is_some_and(|o|o.current) { return Vec::new(); }
    let mate=crate::portal_ui::observed_item_mate(r,observation,y*f.width()+x);
    let aliases=crate::non_euclidean_ui::observed_item_aliases(r,observation,y*f.width()+x);
    r.ground.get(&r.depth.to_string()).into_iter().flat_map(|items|items.iter()).filter(|g|
        ((g.x==x as f64&&g.y==y as f64)
            ||mate.is_some_and(|m|g.x==(m%f.width())as f64&&g.y==(m/f.width())as f64)
            ||aliases.contains(&(g.y as usize*f.width()+g.x as usize)))
            &&crate::dream_ui::item_visible(r,&g.item)).collect()
}
pub fn hover_available(report: &Report) -> bool {
    !report.item_hover.is_empty() || report.creature.as_ref().is_some_and(|c|c.hover_available)
}
pub fn observe_tile(r: &Run, x: usize, y: usize) -> Report {
    let mut report=observe_tile_inner(r,x,y,None);
    if crate::hallucination_ui::active(r){question_report(&mut report);}
    report
}
/// Reuse only terrain observed from this exact unchanged world's dream projection.
/// Dimensions are checked defensively; freshness belongs to the caller, not tick.
pub fn observe_tile_with_terrain(r: &Run, x: usize, y: usize, terrain: &vision::TerrainObservation) -> Report {
    let mut report=observe_tile_inner(r,x,y,Some(terrain));
    if crate::hallucination_ui::active(r){question_report(&mut report);}
    report
}
fn observe_tile_inner(r: &Run, x: usize, y: usize, terrain: Option<&vision::TerrainObservation>) -> Report {
    let _timing = corpse_core::timing::span("desktop.inspect");
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
    let unknown = || Report {
        title: "The unknown".into(),
        scene: "You cannot make out what lies here.".into(),
        lines: vec!["You cannot make out what lies here.".into()],
        ..Default::default()
    };
    let Some(f) = r.floor() else { return unknown() };
    if x >= f.width() || y >= f.height() {
        return unknown();
    }
    let owned;
    let observation = match terrain.filter(|o| o.width == f.width() && o.height == f.height() && o.cells.len() == f.tiles.len()) {
        Some(o) => o,
        None => { owned = vision::observe_terrain(r); let Some(o) = owned.as_ref() else { return unknown(); }; o }
    };
    let o = &observation.cells[y * f.width() + x];
    let portal_alias = crate::portal_ui::observed_alias_with_terrain(r, x, y, observation);
    let pocket_alias = crate::non_euclidean_ui::observed_alias_with_terrain(r, x, y, observation);
    let alias = portal_alias.or(pocket_alias);
    let observed: Vec<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| ((a.x == x as f64 && a.y == y as f64) || alias.is_some_and(|other| a.id == other.id)) && vision::visible_actor(r, a))
        .collect();
    if let Some(a) = observed.iter().copied()
        .min_by_key(|a| (a.dead.value_or(false), !corpse_core::dreamwalking::is_nightmare(a)))
    {
        let decorate=|mut detail:CreatureReport,phase:usize| {
        if alias.is_some_and(|other| a.id == other.id) {
            let text = if portal_alias.is_some() { " This creature occupies the linked portal space at the other end." }
                else { " This already visible creature also occupies this linked Non-Euclidean space." };
            detail.description.push_str(text); detail.scene.push_str(text);
        }
        let overlapping: Vec<_> = observed.iter().copied()
            .filter(|other| other.id != a.id && !other.dead.value_or(false)
                && corpse_core::dreamwalking::dream_actor(r, other) != corpse_core::dreamwalking::dream_actor(r, a))
            .map(|other| {
                let shown = if crate::hallucination_ui::active(r)&&crate::hallucination_ui::eligible(other) {crate::creature_presentation::project_kind(r,other,crate::hallucination_ui::kind(r,other,phase))}else{crate::creature_presentation::project(r, other)};
                creature_title(r, shown.as_ref().unwrap_or(other))
            }).collect();
        if !overlapping.is_empty() {
            let text = format!(" Overlapping in the {} plane: {}. They share this square but cannot touch one another.",
                if corpse_core::dreamwalking::dream_actor(r, a) { "normal" } else { "dream" },
                overlapping.join(", "));
            detail.description.push_str(&text);
            detail.scene.push_str(&text);
        }
        if r.player.x == x as f64 && r.player.y == y as f64
            && !corpse_core::dreamwalking::same_plane(r, &r.player, a) {
            detail.description.push_str(" Your dream projection overlaps this normal-plane creature without touching it.");
        }
        if let Some(text) = crate::non_euclidean_ui::description(r, &observation, x, y) {
            detail.description.push_str(&format!(" {text}")); detail.scene.push_str(&format!(" {text}"));
        }
        if corpse_core::revision50::enabled(r) {
            for g in observed_ground_at(r,observation,x,y) {
                detail.description.push_str(&format!("\n{}",pack::describe_item(r,&g.item)));
            }
        }
        detail.report()
        };
        if crate::hallucination_ui::active(r)&&crate::hallucination_ui::eligible(a) {
            // One detached scratch Run, one admitted terrain observation; no Run retained.
            let mut scratch=r.clone();
            let variants=std::array::from_fn(|phase| {
                let shown=crate::creature_presentation::project_kind(r,a,crate::hallucination_ui::kind(r,a,phase)).unwrap();
                if let Some(slot)=scratch.actors_at_mut(r.depth).iter_mut().find(|v|v.id==a.id){*slot=shown.clone();}
                let mut report=decorate(creature_presented(&scratch,&shown,Some(o),true),phase);
                question_report(&mut report);report
            });
            let mut report=variants[0].clone();report.hallucination=Some(std::sync::Arc::new(variants));return report;
        }
        return decorate(creature(r,a,Some(o)),0);
    }
    let mut report = unknown();
    if o.current || o.remembered {
        let (name, text) = terrain_copy(f, o);
        report = Report {
            title: terrain_report_title(name, o.current),
            scene: text.clone(),
            lines: vec![text],
            ..Default::default()
        };
        if let Some(text) = crate::portal_ui::description(r, x, y) {
            report.title = text.split('.').next().unwrap_or("Portal").into();
            report.scene.push_str(&format!(" {text}")); report.lines.push(text);
        }
        if let Some(text) = crate::non_euclidean_ui::description(r, &observation, x, y) {
            report.title = if text.starts_with("Speed-up area.") { "Speed-up area" } else { "Non-Euclidean space" }.into();
            report.scene = format!("{text} {}",report.scene); report.lines.insert(0,text);
        }
        if o.current {
            if let Some(countdown) = crate::observe::observed_epidite_countdown(r, o, f.width()) {
                let timer = format!("{countdown} turns until judgement");
                report.scene = format!("{timer}. {}",report.scene); report.lines.insert(0,timer);
            }
            if let Some(h) = o.hazards.as_ref() {
                if let Some(feature) = h.feature.as_ref() {
                    report.title = feature.name.clone();
                    report.lines.push(feature.description.clone());
                    report.scene.push_str(&format!(" {}", feature.description));
                }
                if let Some(source) = h.light_source.as_ref() {
                    let (name, text) = if source == "beacon" {
                        ("Magical beacon", "A magical beacon glows here.")
                    } else if o.tile.as_deref() == Some("wall") {
                        ("Wall torch", "A torch set into the wall flickers yellow.")
                    } else {
                        ("Brazier", "A brazier burns here.")
                    };
                    report.title = name.into();
                    report.lines.push(text.into());
                    report.scene.push_str(&format!(" {text}"));
                }
            }
            let player = r.player.x == x as f64 && r.player.y == y as f64;
            if let Some(label)=crate::perception_ui::tile_label(r,o) { report.lines.push(label); }
            if player && !corpse_core::revision51_perception::enabled(r) {
                report.lines.push(format!(
                    "Illumination {}.",
                    js_round(o.illumination.unwrap_or(0.))
                ));
            }
            if let Some(ground) = r.ground.get(&r.depth.to_string()) {
                let mate = crate::portal_ui::observed_item_mate(r, &observation, y*f.width()+x);
                let aliases = crate::non_euclidean_ui::observed_item_aliases(r, &observation, y*f.width()+x);
                for g in ground.iter().filter(|g| ((g.x == x as f64 && g.y == y as f64)
                    || mate.is_some_and(|m| g.x == (m%f.width()) as f64 && g.y == (m/f.width()) as f64)
                    || aliases.contains(&(g.y as usize*f.width()+g.x as usize)))
                    && crate::dream_ui::item_visible(r, &g.item)) {
                    let linked = g.x != x as f64 || g.y != y as f64;
                    let place = if linked && mate.is_some() { "through the linked portal" } else if linked { "in the linked Non-Euclidean space" } else { match o.tile.as_deref() {
                        Some("grass") => "upon the grass",
                        Some("water" | "shallow-water") => "in the water",
                        Some("up" | "down") => "on the stairs",
                        _ => "upon the ground",
                    }};
                    let identity = format!("{} rests {place}.", pack::displayed_name(r, &g.item));
                    report.scene.push_str(&format!(" {identity}"));
                    let description=pack::describe_item(r,&g.item);
                    if corpse_core::revision50::enabled(r) { report.item_hover.push(description.clone()); }
                    report.lines.push(format!("{identity}\n{description}"));
                }
            }
            if player {
                report.title = "Your vessel".into();
                report.lines.push(format!(
                    "Health {:.1}/{} · Energy {}",
                    r.player.hp, r.player.max_hp, r.player.energy
                ));
                let statuses = statuses_ui::observe(r, &r.player)
                    .iter()
                    .map(|s| {
                        format!(
                            "{}: {}",
                            statuses_ui::display_name(&s.name),
                            s.duration
                                .map_or("ongoing".into(), |d| format!("{d} turns"))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                if !statuses.is_empty() {
                    report.lines.push(statuses);
                }
                if let Some(g) = o.hazards.as_ref().and_then(|h| h.gas.as_ref()) {
                    report.lines.push(gas_exposure(r, &r.player, g));
                }
            }
            if ability(&r.player, "Ascend")
                && r.depth > 1.
                && r.floors
                    .iter()
                    .find(|f| f.depth == r.depth - 1.)
                    .is_some_and(|f| {
                        x < f.width() && y < f.height() && f.tiles[y * f.width() + x] == "chasm"
                    })
            {
                report.lines.push("Chasm above; Ascend here.".into());
            }
        }
    }
    if !o.current {
        let items = remembered_items(r, x, y, &observation);
        if !items.is_empty() {
            let magical = items.iter().any(|(_, m)| *m);
            let ground = if o.remembered {
                report.lines.clone()
            } else {
                vec![]
            };
            report.title = if magical {
                "Magic-sensed items"
            } else {
                "Remembered items"
            }
            .into();
            report.lines = vec![if magical {
                "You sense magic here; the surroundings remain out of sight."
            } else {
                "You last saw these items here; they may have moved."
            }
            .into()];
            report.scene = format!(
                "{} {} here.",
                if magical { "You sense" } else { "You remember" },
                items
                    .iter()
                    .map(|(i, _)| pack::displayed_name(r, i))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if corpse_core::revision50::enabled(r) { report.item_hover.push(report.lines[0].clone()); }
            for (i, _) in items {
                let description=pack::describe_item(r,i);
                if corpse_core::revision50::enabled(r) { report.item_hover.push(description.clone()); }
                report.lines.push(description);
            }
            report.lines.extend(ground);
        }
    }
    if r.actors_at(r.depth).iter().any(|a| {
        !vision::visible_actor(r, a)
            && lifecycle::audible_banshee(r, a)
            && cond(a, "monsterAudibleX") == x as f64
            && cond(a, "monsterAudibleY") == y as f64
    }) {
        report.title = "Banshee shriek".into();
        report.scene =
            "A banshee’s shriek came from here. Nothing else is revealed by the sound.".into();
        report.lines = vec![report.scene.clone()];
        report.item_hover.clear();
    }
    report
}
/// Bow targeting enriches an already-observed creature report only. No hidden
/// collision query participates: source bow hitChance ignores pathClear, while
/// the separately gated UI trace supplies obstruction/unknown prose.
pub fn observe_bow_tile(r: &Run, x: usize, y: usize, bow_id: f64) -> Report {
    observe_bow_tile_inner(r,x,y,bow_id,None)
}
/// Only the initial inspection terrain is reused; bow identity/trace logic is unchanged.
/// The supplied terrain must describe this exact unchanged world's dream projection.
pub fn observe_bow_tile_with_terrain(r: &Run, x: usize, y: usize, bow_id: f64, terrain: &vision::TerrainObservation) -> Report {
    observe_bow_tile_inner(r,x,y,bow_id,Some(terrain))
}
fn observe_bow_tile_inner(r: &Run, x: usize, y: usize, bow_id: f64, terrain: Option<&vision::TerrainObservation>) -> Report {
    let dream_view=crate::dream_ui::observation_run(r);let r=dream_view.as_ref();
    let mut report=terrain.map_or_else(||observe_tile(r,x,y),|o|observe_tile_with_terrain(r,x,y,o));
    if report.creature.is_some(){
      if let Some(actor)=r.actors_at(r.depth).iter().filter(|a|a.x==x as f64&&a.y==y as f64&&vision::visible_actor(r,a)&&corpse_core::dreamwalking::same_plane(r,&r.player,a)).min_by_key(|a|(a.dead.value_or(false),!corpse_core::dreamwalking::is_nightmare(a))){
       if let Some(bow)=r.inventory.iter().find(|i|i.id==bow_id){
        let trace=crate::targeting_ui::observe(r,Some(crate::targeting_ui::Point{x,y}));
        if let Some(cache)=report.hallucination.as_mut(){
          let mut scratch=r.clone();
          for (phase,entry) in std::sync::Arc::make_mut(cache).iter_mut().enumerate(){
            let shown=crate::creature_presentation::project_kind(r,actor,crate::hallucination_ui::kind(r,actor,phase)).unwrap();
            if let Some(slot)=scratch.actors_at_mut(r.depth).iter_mut().find(|a|a.id==actor.id){*slot=shown.clone();}
            if let Some(c)=entry.creature.as_mut(){c.bow_shot=bow_text_presented(&scratch,bow,&shown,&trace);crate::hallucination_ui::question(&mut c.bow_shot);}
          }
          report.creature=cache[0].creature.clone();
        }else if let Some(c)=report.creature.as_mut(){c.bow_shot=bow_text(r,bow,actor,&trace);}
       }
      }
    }report
}
fn bow_text(r: &Run, bow: &Item, a: &Actor, trace: &crate::targeting_ui::Trace) -> String {
    let shown=crate::creature_presentation::project(r,a);
    let view=shown.as_ref().map(|a|crate::creature_presentation::projected_run(r,a));
    let r=view.as_ref().unwrap_or(r);
    let a=shown.as_ref().unwrap_or(a);
    bow_text_presented(r,bow,a,trace)
}
fn bow_text_presented(r:&Run,bow:&Item,a:&Actor,trace:&crate::targeting_ui::Trace)->String {
    if let Some(reason) = corpse_core::projectiles::blocker(
        r,
        &corpse_core::Command::Shoot {
            id: bow.id,
            x: a.x,
            y: a.y,
        },
    ) {
        return format!("Bow shot unavailable: {reason}");
    }
    match trace.stop_reason {
        crate::targeting_ui::StopReason::Terrain|crate::targeting_ui::StopReason::Creature=>return "Bow shot: blocked by visible terrain or another visible creature. The arrow stops at the first obstruction.".into(),
        crate::targeting_ui::StopReason::Unknown=>return "Bow shot: the full projectile path is not currently visible.".into(),
        _=>{}
    }
    let mut view = combat_view(r);
    let mut actor = a.clone();
    if actor.id != view.player.id && !matches!(actor.state.as_str(), "ally" | "neutral" | "hunting")
    {
        actor.state = "hunting".into();
        if let Some(entry) = view
            .actors_at_mut(r.depth)
            .iter_mut()
            .find(|b| b.id == a.id)
        {
            *entry = actor.clone();
        }
    }
    let base = corpse_core::engine::player_weapon_strike_base(&view, bow);
    let extra = if corpse_core::revision50::enabled(&view) { 0. }
        else { corpse_core::crafting::preview_fury_bonus(&view, view.player.energy).0 };
    let range = (view.player.x - actor.x).hypot(view.player.y - actor.y);
    let mut shot = strike_preview_ranged(&view, &view.player, &actor, base, 1., extra, range, 0.5);
    if corpse_core::divine_reactions::nightmare_absorbs(&view, &actor, Some(&view.player)) {
        shot.minimum = 0.;
        shot.maximum = 0.;
    }
    format!(
        "Bow shot: {:.1}% hit · {}–{} damage · uses 1 arrow.",
        shot.chance * 100.,
        shot.minimum,
        shot.maximum
    )
}
#[derive(Clone)]
struct PaintLine {
    text: String,
    color: [u8; 3],
    size: u16,
    disclosure: bool,
}
pub fn bounds(profile: &crate::appearance_profile::Profile) -> (i32, i32, i32, i32) {
    modal_ui::inspection_bounds(profile).tuple()
}
fn role_line(p: &Painter, role: Role, size: u16, baseline: i32) -> i32 {
    let mut base = RoleStyle::baseline(size as f64);
    base.line_height = baseline as f64 / size as f64;
    let style = typography_ui::resolve(&p.profile, role, base);
    (style.font_size * style.line_height).round().max(1.) as i32
}
fn content_top(p: &Painter) -> i32 {
    78.max(42 + role_line(p, Role::Headings, 21, 28))
}
fn role_lines(p: &Painter, role: Role, text: &str, width: i32, size: u16) -> Vec<String> {
    p.wrap_text_role(role, text, width.max(1), size)
}
fn paint_prose_role(
    p: &mut Painter,
    role: Role,
    text: &str,
    x: i32,
    y: i32,
    size: u16,
    color: [u8; 3],
) {
    // The source inspection interface uses the bundled monospace reading face,
    // including unchanged defaults; the manual's serif prose is separate.
    p.text_role(role, text, x, y, size, color)
}

fn ui_color(p: &Painter, color: [u8; 3]) -> [u8; 3] {
    if color == INK {
        p.profile.rgb("ui.colors.primary")
    } else if color == MUTED {
        p.profile.rgb("ui.colors.secondary")
    } else if color == GOLD {
        p.profile.rgb("ui.colors.headings")
    } else {
        color
    }
}
fn wrapped(report: &Report, p: &Painter) -> Vec<PaintLine> {
    let mut rows = Vec::new();
    let mut add = |text: &str, color: [u8; 3], size: u16, space: bool| {
        if text.is_empty() {
            return;
        };
        for text in role_lines(p, Role::Primary, text, bounds(&p.profile).2 - 32, size) {
            rows.push(PaintLine {
                text,
                color,
                size,
                disclosure: false,
            });
        }
        if space {
            rows.push(PaintLine {
                text: String::new(),
                color,
                size,
                disclosure: false,
            });
        }
    };
    if let Some(c) = &report.creature {
        add(&c.description, MUTED, 15, false);
        add(&c.behavior, MUTED, 15, false);
        add(&c.resources, INK, 15, false);
        add(&c.conditions, [193, 138, 255], 14, true);
        add("Your attacks", [36, 223, 255], 15, false);
        add(&c.your_attack, [36, 223, 255], 15, true);
        add("Enemy attacks", [255, 82, 120], 15, false);
        add(&c.their_attack, [255, 82, 120], 15, true);
        add(&c.abilities, MUTED, 15, true);
        add(&c.heart, GOLD, 14, false);
        if !c.ability_details.is_empty() && c.ability_details != c.abilities {
            rows.push(PaintLine {
                text: format!(
                    "{} Full mechanics",
                    if report.details_open { "▾" } else { "▸" }
                ),
                color: MUTED,
                size: 15,
                disclosure: true,
            });
            if report.details_open {
                for text in role_lines(
                    p,
                    Role::Primary,
                    &c.ability_details,
                    bounds(&p.profile).2 - 32,
                    15,
                ) {
                    rows.push(PaintLine {
                        text,
                        color: MUTED,
                        size: 15,
                        disclosure: false,
                    });
                }
            }
        }
    } else {
        for text in &report.lines {
            add(text, INK, 15, true);
        }
    }
    rows
}
pub fn scroll_line_count(report: &Report, p: &Painter) -> usize {
    wrapped(report, p).len().saturating_sub(
        ((bounds(&p.profile).3 - content_top(p) - 16).max(0) / role_line(p, Role::Primary, 15, 21))
            .max(1) as usize,
    )
}
pub fn toggle_details(report: &mut Report) -> bool {
    if report
        .creature
        .as_ref()
        .is_some_and(|c| !c.ability_details.is_empty() && c.ability_details != c.abilities)
    {
        report.details_open = !report.details_open;
        true
    } else {
        false
    }
}
/// Drawer owns its hit testing. Parent consumes every mouse input while open.
pub fn click(report: &mut Report, p: &Painter, scroll: usize, x: i32, y: i32) -> bool {
    let (bx, by, w, h) = bounds(&p.profile);
    if x < bx || x >= bx + w || y < by + content_top(p) || y >= by + h - 12 {
        return false;
    }
    let row = scroll + ((y - by - content_top(p)) / role_line(p, Role::Primary, 15, 21)) as usize;
    if wrapped(report, p).get(row).is_some_and(|r| r.disclosure) {
        return toggle_details(report);
    }
    false
}
pub fn close_hit(x: i32, y: i32) -> bool {
    let (bx, by, w, _) = BOUNDS;
    x >= bx + w - 40 && x < bx + w - 8 && y >= by + 8 && y < by + 36
}
pub fn close_hit_profile(p: &Painter, x: i32, y: i32) -> bool {
    let (bx, by, w, _) = bounds(&p.profile);
    x >= bx + w - 40 && x < bx + w - 8 && y >= by + 8 && y < by + 36
}
pub fn paint(report: &Report, p: &mut Painter, scroll: usize) {
    let (x, y, w, h) = bounds(&p.profile);
    let old = p.clip;
    p.clip = Some((x, y, w, h));
    p.rect(x, y, w, h, p.profile.rgb("ui.colors.panel"));
    modal_ui::border(p, Rect(x, y, w, h), p.profile.rgb("ui.colors.border"));
    p.text_role(
        Role::Primary,
        "Inspect",
        x + 13,
        y + 12,
        16,
        p.profile.rgb("ui.colors.primary"),
    );
    p.text("×", x + w - 30, y + 10, 19, MUTED);
    p.text_role(
        Role::Headings,
        &report.title,
        x + 13,
        y + 42,
        21,
        p.profile.rgb("ui.colors.headings"),
    );
    p.clip = Some((
        x + 12,
        y + content_top(p) - 1,
        (w - 24).max(0),
        (h - content_top(p) - 11).max(0),
    ));
    for (n, line) in wrapped(report, p)
        .iter()
        .skip(scroll)
        .take(((h - content_top(p) - 16).max(0) / role_line(p, Role::Primary, 15, 21)) as usize + 1)
        .enumerate()
    {
        paint_prose_role(
            p,
            Role::Primary,
            &line.text,
            x + 14,
            y + content_top(p) + n as i32 * role_line(p, Role::Primary, 15, 21),
            line.size,
            ui_color(p, line.color),
        )
    }
    p.clip = old;
}
/// Exact source enemyHoverPlacement geometry, with its viewport supplied by
/// the desktop integration. The tooltip never creates a hit region.
fn hover_bounds(
    tile: (i32, i32, i32, i32),
    height: i32,
    viewport: (i32, i32),
) -> (i32, i32, i32, i32) {
    let (left, top, right, bottom) = tile;
    let (vw, vh) = viewport;
    let w = 520.min((vw - 16).max(0));
    let h = height.max(0).min(620.min((vh - 16).max(0)));
    let below = bottom + 5;
    let y = if below + h <= vh - 8 {
        below
    } else {
        top - 5 - h
    };
    (
        ((left + right - w) / 2).clamp(8, (vw - 8 - w).max(8)),
        y.clamp(8, (vh - 8 - h).max(8)),
        w,
        h,
    )
}
/// Point-anchor convenience. Prefer paint_hover_at with the actual source tile
/// rectangle so movement across that tile never shifts the tooltip.
pub fn paint_hover(report: &Report, p: &mut Painter, x: i32, y: i32) {
    paint_hover_at(report, p, (x, y, x, y))
}
pub fn paint_hover_at(report: &Report, p: &mut Painter, tile: (i32, i32, i32, i32)) {
    let Some(c) = report.creature.as_ref().filter(|c| c.hover_available) else {
        paint_item_hover(report,p,tile);
        return;
    };
    let hover_width = modal_ui::hover_bounds_painter(p, tile, 0).2;
    let line_height = role_line(p, Role::Tooltips, 15, 21);
    let mut rows: Vec<(String, [u8; 3], u16, i32)> = vec![];
    let mut offset = 36.max(12 + role_line(p, Role::Tooltips, 18, 24));
    let mut add = |text: &str, color: [u8; 3], size: u16, gap: i32| {
        if text.is_empty() {
            return;
        }
        for line in role_lines(p, Role::Tooltips, text, hover_width - 30, size) {
            rows.push((line, color, size, offset));
            offset += line_height;
        }
        offset += gap;
    };
    add(&c.description, MUTED, 15, 4);
    add(&c.behavior, MUTED, 15, 4);
    // Resource meters are an observed detached projection, never parsed from prose.
    let resource_y = offset + 4;
    offset += if c.hover_resources.is_empty() { 0 } else { 41 };
    let mut add = |text: &str, color: [u8; 3], size: u16, gap: i32| {
        if text.is_empty() {
            return;
        }
        for line in role_lines(p, Role::Tooltips, text, hover_width - 30, size) {
            rows.push((line, color, size, offset));
            offset += line_height;
        }
        offset += gap;
    };
    if c.hover_resources.is_empty() {
        add(&c.resources, INK, 15, 4)
    }
    add(&c.conditions, [193, 138, 255], 14, 7);
    add("Your attacks", [36, 223, 255], 15, 2);
    add(&c.your_attack, [36, 223, 255], 15, 7);
    add("Enemy attacks", [255, 82, 120], 15, 2);
    add(&c.their_attack, [255, 82, 120], 15, 7);
    add(&c.abilities, MUTED, 15, 4);
    add(&c.heart, GOLD, 14, 0);
    add(&c.bow_shot, INK, 15, 7);
    // The native map column is the overlay viewport, preserving the right rail.
    let (x, y, w, h) = modal_ui::hover_bounds_painter(p, tile, offset + 12).tuple();
    let old = p.clip;
    p.clip = Some((x, y, w, h));
    // Source #151923f5 composited over the already-painted scene.
    // O(clipped native rectangle area), O(1) extra memory; same logical preclip.
    let left=x.max(0); let top=y.max(0);
    p.blend_rect(left,top,(x+w).min(crate::render::WIDTH as i32)-left,
        (y+h).min(crate::render::HEIGHT as i32)-top,p.profile.rgb("ui.colors.tooltip"),245);
    modal_ui::border(p, Rect(x, y, w, h), p.profile.rgb("ui.colors.border"));
    paint_prose_role(
        p,
        Role::Tooltips,
        &c.title,
        x + 14,
        y + 12,
        18,
        p.profile.rgb("ui.colors.primary"),
    );
    for (text, color, size, yy) in rows {
        paint_prose_role(
            p,
            Role::Tooltips,
            &text,
            x + 14,
            y + yy,
            size,
            ui_color(p, color),
        )
    }
    let cell = (w - 38) / 2;
    for (i, r) in c.hover_resources.iter().enumerate() {
        let xx = x + 14 + i as i32 * (cell + 10);
        let yy = y + resource_y;
        if let Some(value) = r.value {
            p.text(&r.label, xx, yy, 14, r.color);
            let count = format!(
                "{} / {}",
                value.ceil(),
                r.maximum.map_or("∞".into(), |n| n.ceil().to_string())
            );
            p.text(&count, xx + cell - 110, yy, 14, INK);
            p.rect(xx, yy + 20, cell, 5, [36, 41, 53]);
            let fraction = if r.reference.is_finite() && r.reference > 0. {
                (value / r.reference).clamp(0., 1.)
            } else {
                1.
            };
            p.rect(
                xx,
                yy + 20,
                (cell as f64 * fraction).round() as i32,
                5,
                r.color,
            );
        } else {
            p.text(&r.label, xx, yy, 14, MUTED)
        }
    }
    p.clip = old;
}
/// Detached item text uses the existing tooltip geometry and typography.
/// O(text wrapping/font work + clipped tooltip pixels), no world observation.
fn paint_item_hover(report: &Report, p: &mut Painter, tile: (i32,i32,i32,i32)) {
    if report.item_hover.is_empty() { return; }
    let width=modal_ui::hover_bounds_painter(p,tile,0).2;
    let line_height=role_line(p,Role::Tooltips,15,21);
    let mut rows=Vec::new();
    for block in &report.item_hover {
        for paragraph in block.lines() {
            rows.extend(role_lines(p,Role::Tooltips,paragraph,width-28,15));
        }
        rows.push(String::new());
    }
    rows.pop();
    let (x,y,w,h)=modal_ui::hover_bounds_painter(p,tile,rows.len()as i32*line_height+24).tuple();
    let old=p.clip;p.clip=Some((x,y,w,h));
    let left=x.max(0);let top=y.max(0);
    p.blend_rect(left,top,(x+w).min(crate::render::WIDTH as i32)-left,
        (y+h).min(crate::render::HEIGHT as i32)-top,p.profile.rgb("ui.colors.tooltip"),245);
    modal_ui::border(p,Rect(x,y,w,h),p.profile.rgb("ui.colors.border"));
    for (index,line) in rows.iter().enumerate() {
        paint_prose_role(p,Role::Tooltips,line,x+14,y+12+index as i32*line_height,15,p.profile.rgb("ui.colors.primary"));
    }
    p.clip=old;
}
fn js_round(n: f64) -> f64 {
    (n + 0.5).floor() + 0.
}
fn gas_exposure(r: &Run, a: &Actor, g: &corpse_core::GasPresence) -> String {
    let prefix = format!(
        "{} present (volume {}{})",
        gas_label(&g.r#type),
        g.concentration,
        if g.concentration == 0. {
            "; lingering gas layer"
        } else {
            ""
        }
    );
    let corrosive = matches!(g.r#type.as_str(), "caustic" | "breather");
    let immunity = if has(a, "enemy:beast:gas-immunity") {
        Some("Gas Immunity")
    } else if corrosive && ability(a, "Entropy") {
        Some("Entropy")
    } else if (corrosive || g.r#type=="hallucinogenic") && has(a, "enemy:pale:gas-immunity") {
        Some(if g.r#type=="hallucinogenic" {"Pale hallucination-gas immunity"} else {"Pale caustic-gas immunity"})
    } else {
        None
    };
    if let Some(reason) = immunity {
        return format!("{prefix}: immune ({reason}).");
    }
    if corrosive {
        if combat::nearest_damage_enabled(r) {
            let raw = a.max_hp * (1. / 15.) * (1. + cond(a, "gasVulnerable").max(0.));
            let kind = if g.r#type == "caustic" { "caustic" } else { "breatherGas" };
            return format!("{prefix}: {} HP per normal turn while present, after damage resistance.",
                combat::damage_packet(r, a, raw, kind, true));
        }
        return format!(
            "{prefix}: {} HP per normal turn while present, before damage prevention.",
            (a.max_hp / 15. * (1. + cond(a, "gasVulnerable").max(0.))).ceil()
        );
    }
    match g.r#type.as_str() {
        "paralysis" => format!("{prefix}: refreshes paralysis to 2 turns while present."),
        "confusion" => format!("{prefix}: refreshes confusion to 8 turns while present."),
        "hallucinogenic" => format!("{prefix}: refreshes hallucinations to 2 turns while present; perceived creature identities change, not their true statistics."),
        kind => format!(
            "{prefix}: harmless on contact{}.",
            if kind == "steam" {
                " and nonflammable"
            } else {
                "; dangerous if ignited"
            }
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Run {
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()
    }
    fn merge(target: &mut serde_json::Value, patch: &serde_json::Value) {
        for (k, v) in patch.as_object().unwrap() {
            target[k] = v.clone();
        }
    }
    fn cell(f: &Floor, at: usize, current: bool, tick: f64) -> vision::ObservedCell {
        let array = |v: &Field<Vec<String>>| {
            v.as_ref()
                .and_then(|v| v.get(at))
                .cloned()
                .filter(|s| !s.is_empty())
        };
        vision::ObservedCell {
            index: at,
            current,
            remembered: true,
            knowledge: if current { "direct" } else { "remembered" }.into(),
            direct: current,
            tile: if current {
                Some(f.tiles[at].clone())
            } else {
                array(&f.remembered_tiles)
            },
            substrate: if current {
                array(&f.substrates)
            } else {
                array(&f.remembered_substrates)
            },
            door_open: if current {
                f.doors
                    .as_ref()
                    .and_then(|m| m.get(&at.to_string()))
                    .map(|d| d.open)
            } else {
                f.remembered_doors
                    .as_ref()
                    .and_then(|m| m.get(&at.to_string()))
                    .map(|d| d.open)
            },
            biome: corpse_core::pocket_biomes::at(f,at).map(|b|b.theme.clone()),
            flowers: corpse_core::pocket_biomes::flower_at(f,at) && if current {f.tiles[at]=="grass"} else {f.remembered_tiles.as_ref().and_then(|v|v.get(at)).is_some_and(|t|t=="grass")},
            snow: if current {
                f.snow.as_ref()
            } else {
                f.remembered_snow.as_ref()
            }
            .and_then(|v| v.get(at))
            .copied()
            .unwrap_or(false),
            ice: if current { f.ice.as_ref() } else { f.remembered_ice.as_ref() }
                .and_then(|v|v.get(at)).copied().unwrap_or(false),
            puddles: if current { f.puddles.as_ref() } else { f.remembered_puddles.as_ref() }
                .and_then(|v|v.get(at)).copied().unwrap_or(false),
            hellfire: if current { f.hellfire.as_ref() } else { f.remembered_hellfire.as_ref() }
                .and_then(|v|v.get(at)).copied().unwrap_or(false),
            grate: f.grates.as_ref().is_some_and(|v| v.contains(&(at as f64))),
            bridge: f
                .generation_bridges
                .as_ref()
                .is_some_and(|v| v.contains(&(at as f64))),
            entrance: false,
            chance: 100.,
            direct_chance: 100.,
            illumination: current.then_some(21.6),
            ambient: None,
            distance: 1.,
            hazards: if current {
                Some(corpse_core::hazard_memory::observed_hazards_at(f, at, tick))
            } else {
                f.remembered_hazards
                    .as_ref()
                    .and_then(|m| m.get(&at.to_string()))
                    .cloned()
            },
        }
    }
    #[test]
    fn current_plain_tree_title_is_hidden_without_changing_terrain_copy() {
        let r = base();
        let mut f = r.floor().unwrap().clone();
        let at = 0;
        f.tiles[at] = "tree".into();
        let mut o = cell(&f, at, true, r.tick);
        o.hazards = None; o.biome = None; o.flowers = false;
        o.snow = false; o.ice = false; o.puddles = false; o.hellfire = false;
        let (name, text) = terrain_copy(&f, &o);
        assert_eq!(name, "Tree");
        assert_eq!(text, "A tree rises from the ground.");
        assert_eq!(terrain_report_title(name, true), "");
        assert_eq!(terrain_report_title("Tree".into(), false), "Tree");
        assert_eq!(terrain_report_title("Wall".into(), true), "");
        assert_eq!(terrain_report_title("Portal".into(), true), "Portal");
        for name in ["Ice", "Puddle", "Hellfire", "Bloody wall", "Rock wall", "Stone wall", "Wall", "Bare ground", "Stone floor", "Rock floor", "Earthen floor", "Sand", "Soot", "Ash", "Grass", "Tree", "Deep water", "Shallow water", "Chasm", "Upstairs", "Downstairs", "Bottomless pit", "Open door", "Closed door", "Grate", "Bridge"] {
            assert_eq!(terrain_report_title(name.into(), true), "", "{name}");
            assert_eq!(terrain_report_title(name.into(), false), name, "{name}");
        }
        for name in ["Speed-up area", "Unclear memory", "Non-Euclidean space", "Brazier", "Magical beacon"] {
            assert_eq!(terrain_report_title(name.into(), true), name);
        }
    }
    #[test]
    fn source_58_terrain_reports() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/inspection-ui-v48.json")).unwrap();
        let r = base();
        for case in fixtures["terrain"].as_array().unwrap() {
            let mut f = serde_json::to_value(r.floor().unwrap()).unwrap();
            merge(&mut f, &case["patch"]);
            let f: Floor =
                serde_json::from_value(f).unwrap_or_else(|e| panic!("{} {e}", case["name"]));
            let o = cell(
                &f,
                case["at"].as_u64().unwrap() as usize,
                case["current"].as_bool().unwrap(),
                r.tick,
            );
            let (name, description) = terrain_copy(&f, &o);
            assert_eq!(
                name,
                case["expected"]["name"].as_str().unwrap(),
                "{}",
                case["name"]
            );
            assert_eq!(
                description,
                case["expected"]["description"].as_str().unwrap(),
                "{}",
                case["name"]
            );
        }
    }
    #[test]
    fn source_creature_flavor_scene_resources_conditions_and_forecasts() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/inspection-ui-v48.json")).unwrap();
        for case in fixtures["creatures"].as_array().unwrap() {
            let mut source = base();
            source.inventory.clear();
            let mut value = serde_json::to_value(source).unwrap();
            merge(&mut value, &case["patch"]);
            let mut r: Run = serde_json::from_value(value).unwrap();
            let a: Actor = serde_json::from_value(case["actor"].clone()).unwrap();
            *r.actors_at_mut(r.depth) = vec![a.clone()];
            let before = r.clone();
            let f = r.floor().unwrap();
            let o = cell(
                f,
                a.y as usize * f.width() + a.x as usize,
                case["tileObserved"].as_bool().unwrap(),
                r.tick,
            );
            let got = creature(&r, &a, Some(&o));
            let e = &case["expected"];
            for (key, actual) in [
                ("title", got.title),
                ("description", got.description),
                ("scene", got.scene),
                ("behavior", got.behavior),
                ("resources", got.resources),
                ("conditions", got.conditions),
                ("yourAttack", got.your_attack),
                ("theirAttack", got.their_attack),
                ("abilities", got.abilities),
                ("abilityDetails", got.ability_details),
                ("heart", got.heart),
            ] {
                let expected=e[key].as_str().unwrap();
                let expected=if key=="description" {
                    // Preserve the frozen fixture; apply only approved grammar corrections.
                    expected.replace("whom has", "who has").replace("It's head", "Its head")
                }else if key=="conditions" {
                    expected.replace("Slow: acts every other turn","Slow")
                        .replace("Slowed: acts every other turn","Slowed")
                        .replace("Lucky: next attack","Lucky").replace("Unlucky: next attack","Unlucky")
                        .replace("Distracted: next attack","Distracted").replace("Stuck: next move escapes","Stuck")
                        .replace("Charging moonbeam: next normal action","Charging moonbeam")
                        .replace("Riptide: rooted","Riptide")
                        .replace("Squirming helplessly: outside habitat","Squirming helplessly")
                }else if key=="heart" {
                    // Presentation units changed, not the frozen heart reward.
                    // Compute this oracle from fixture numbers, independently
                    // of the live stat-presentation helpers.
                    expected.split(" · ").map(|part| {
                        if let Some(n)=part.strip_suffix(" Defense") {
                            format!("{:+} HP",n.parse::<f64>().unwrap()*6.)
                        }else{part.replace(" Offense"," Damage")}
                    }).collect::<Vec<_>>().join(" · ")
                }else{expected.to_string()};
                assert_eq!(actual, expected, "{} {key}", case["name"]);
            }
            assert_eq!(r, before);
        }
    }
    #[test]
    fn source_gas_exposure_copy() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/inspection-ui-v48.json")).unwrap();
        for c in cases["gas"].as_array().unwrap() {
            let a: Actor = serde_json::from_value(c["actor"].clone()).unwrap();
            let gas = serde_json::from_value(c["presence"].clone()).unwrap();
            // Internal gas key remains breather; the display name follows Gasser.
            let expected = c["expected"].as_str().unwrap().replace("Breather gas", "Gasser gas");
            assert_eq!(gas_exposure(&base(), &a, &gas), expected);
        }
    }
    fn hidden_scene() -> (Run, usize, usize) {
        let mut r = base();
        r.player.conditions.clear();
        r.player.abilities.clear();
        r.player.active.clear();
        r.actors_at_mut(r.depth).clear();
        let f = r.floor_mut().unwrap();
        let x = f.width() - 2;
        let y = f.height() - 2;
        let at = y * f.width() + x;
        f.seen.fill(false);
        f.seen[at] = true;
        f.remembered_tiles = vec!["floor".into(); f.tiles.len()].into();
        f.remembered_substrates = vec!["earth".into(); f.tiles.len()].into();
        f.remembered_hazards = Default::default();
        f.lights.fill(-100.);
        assert!(!vision::observe_terrain(&r).unwrap().cells[at].current);
        (r, x, y)
    }
    #[test]
    fn r50_item_hover_ground_names_and_effects_are_known_without_identification() {
        let mut r = base();
        r.simulation_revision=50.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        r.actors=Default::default();r.ground.clear();r.known_potions.clear();
        r.player.x=10.;r.player.y=10.;
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
        f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;
        for effect in corpse_core::revision50_potions::ORDINARY.into_iter().chain(["Veridian Sap"]) {
            let item=Item{id:900.,kind:"potion".into(),name:"Unidentified potion".into(),
                effect:effect.to_string().into(),identified:false.into(),..Default::default()};
            *r.ground_at_mut(r.depth)=vec![corpse_core::GroundItem{x:11.,y:10.,item,..Default::default()}];
            let before=serde_json::to_vec(&r).unwrap();
            let report=observe_tile(&r,11,10);let text=report.lines.join("\n");
            assert!(text.contains(corpse_core::revision50_item_knowledge::potion_name(effect).unwrap()),"{effect}: {text}");
            assert!(text.contains(&corpse_core::revision50_item_knowledge::potion_effect_description(effect,false).unwrap()));
            assert!(!text.contains("Unidentified")&&!text.contains("Identified when used"));
            assert!(hover_available(&report));assert!(report.item_hover.join("\n").contains(&corpse_core::revision50_item_knowledge::potion_effect_description(effect,false).unwrap()));
            let view=crate::observe::observe(&r);
            assert!(view.cells.iter().any(|c|c.x==11&&c.y==10&&c.label==corpse_core::revision50_item_knowledge::potion_name(effect).unwrap()));
            assert_eq!(observe_tile(&r,11,10),report);
            assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        }
    }
    #[test]
    fn r50_item_hover_ground_painter_is_versioned_and_leaves_no_clip() {
        let mut r=base();r.simulation_revision=50.0.into();r.actors=Default::default();r.ground.clear();
        r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.x=10.;r.player.y=10.;
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.environment=Field::Missing;
        let item=Item{id:900.,kind:"potion".into(),name:"Unidentified potion".into(),effect:"healing".to_string().into(),..Default::default()};
        r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:11.,y:10.,item,..Default::default()});
        let report=observe_tile(&r,11,10);let mut p=Painter::new();p.clear([20,30,40]);let blank=p.pixels.clone();
        paint_hover(&report,&mut p,300,300);assert_ne!(p.pixels,blank);assert_eq!(p.clip,None);
        r.simulation_revision=49.0.into();let legacy=observe_tile(&r,11,10);assert!(!hover_available(&legacy));
        p.clear([20,30,40]);paint_hover(&legacy,&mut p,300,300);assert_eq!(p.pixels,blank);
    }
    #[test]
    fn r50_item_hover_remembered_properties_do_not_reveal_live_hidden_world() {
        let (mut r,x,y)=hidden_scene();r.simulation_revision=50.0.into();
        let remembered=Item{id:900.,kind:"potion".into(),name:"Unidentified potion".into(),
            effect:"healing".to_string().into(),identified:false.into(),..Default::default()};
        r.remembered_items.get_or_insert_with(Default::default).insert(r.depth.to_string(),
            vec![corpse_core::RunRememberedItemsEntryEntry{x:x as f64,y:y as f64,item:remembered.clone()}]);
        let before=observe_tile(&r,x,y);let text=before.lines.join("\n");
        assert_eq!(before.title,"Remembered items");assert!(text.contains("Potion of Healing")&&text.contains("50 HP"));
        assert!(hover_available(&before));assert!(before.item_hover[0].contains("may have moved"));
        let mut hidden=remembered;hidden.effect="discord".to_string().into();
        r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:x as f64,y:y as f64,item:hidden,..Default::default()});
        assert_eq!(observe_tile(&r,x,y),before);
        r.remembered_items=Field::Missing;
        let text=observe_tile(&r,x,y).lines.join("\n");assert!(!text.contains("Potion of Discord")&&!text.contains("Potion of Healing"));
    }
    #[test]
    fn hidden_live_terrain_items_actors_never_enrich_memory() {
        let (mut r, x, y) = hidden_scene();
        let remembered = corpse_core::equipment::make_equipment("Ring", 900.).unwrap();
        r.remembered_items
            .get_or_insert_with(Default::default)
            .insert(
                r.depth.to_string(),
                vec![corpse_core::RunRememberedItemsEntryEntry {
                    x: x as f64,
                    y: y as f64,
                    item: remembered.clone(),
                }],
            );
        let before = observe_tile(&r, x, y);
        assert_eq!(before.title, "Remembered items");
        assert!(before.scene.contains("Ring"));
        let original = r.clone();
        assert_eq!(observe_tile(&r, x, y), before);
        assert_eq!(r, original);
        let mut hidden = r.player.clone();
        hidden.id = 950.;
        hidden.x = x as f64;
        hidden.y = y as f64;
        hidden.name = "Hidden secret creature".into();
        hidden.hp = 999.;
        r.actors_at_mut(r.depth).push(hidden);
        let f = r.floor_mut().unwrap();
        let at = y * f.width() + x;
        f.tiles[at] = "water".into();
        f.substrates = vec!["sand".into(); f.tiles.len()].into();
        f.slime = vec![5.; f.tiles.len()].into();
        let mut new_item = remembered.clone();
        new_item.name = "Hidden secret item".into();
        new_item.enchantment = 99.0.into();
        r.ground_at_mut(r.depth).push(corpse_core::GroundItem {
            x: x as f64,
            y: y as f64,
            item: new_item,
            ..Default::default()
        });
        assert_eq!(observe_tile(&r, x, y), before);
        r.floor_mut().unwrap().remembered_tiles = Field::Missing;
        r.remembered_items = Field::Missing;
        let memory = observe_tile(&r, x, y);
        assert_eq!(memory.title, "Unclear memory");
        assert_eq!(
            memory.scene,
            "You have been here before, but cannot recall the ground."
        );
    }
    #[test]
    fn detected_items_keep_snapshot_identity_and_per_item_deduplication() {
        let (mut r, x, y) = hidden_scene();
        r.player.conditions.insert("magicSensitive".into(), 4.);
        let i = Item {
            id: 900.,
            name: "Hidden identity".into(),
            kind: "potion".into(),
            effect: "healing".to_string().into(),
            magic_nature: "good".to_string().into(),
            ..Default::default()
        };
        let j = Item {
            id: 901.,
            name: "Old heart".into(),
            kind: "heart".into(),
            ..Default::default()
        };
        r.detected_items
            .get_or_insert_with(Default::default)
            .insert(
                r.depth.to_string(),
                vec![
                    corpse_core::RunDetectedItemsEntryEntry {
                        x: x as f64,
                        y: y as f64,
                        item: i.clone(),
                    },
                    corpse_core::RunDetectedItemsEntryEntry {
                        x: x as f64,
                        y: y as f64,
                        item: j.clone(),
                    },
                ],
            );
        r.remembered_items
            .get_or_insert_with(Default::default)
            .insert(
                r.depth.to_string(),
                vec![corpse_core::RunRememberedItemsEntryEntry {
                    x: x as f64,
                    y: y as f64,
                    item: i,
                }],
            );
        let report = observe_tile(&r, x, y);
        assert_eq!(report.title, "Magic-sensed items");
        assert!(report.scene.contains("Old heart"));
        assert!(!report.scene.contains("healing"));
        assert_eq!(report.lines.len(), 4);
        r.player.conditions.clear();
        assert_eq!(observe_tile(&r, x, y).title, "Remembered items");
    }
    #[test]
    fn sound_only_marker_replaces_all_prior_scene_information() {
        let (mut r, x, y) = hidden_scene();
        let mut a = r.player.clone();
        a.id = 999.;
        a.name = "Banshee".into();
        a.creature_kind = "banshee".to_string().into();
        a.x = x as f64;
        a.y = y as f64;
        a.conditions.insert("monsterAudibleX".into(), x as f64);
        a.conditions.insert("monsterAudibleY".into(), y as f64);
        a.conditions
            .insert("monsterAudibleUntil".into(), r.tick + 1.);
        r.actors_at_mut(r.depth).push(a);
        let report = observe_tile(&r, x, y);
        assert_eq!(report.title, "Banshee shriek");
        assert_eq!(
            report.lines,
            vec!["A banshee’s shriek came from here. Nothing else is revealed by the sound."]
        );
        assert!(report.creature.is_none());
    }
    #[test]
    fn drawer_paint_bounds_scroll_disclosure_and_repeat_open() {
        let mut p = Painter::new();
        let mut report = CreatureReport {
            title: "Observed creature".into(),
            description: "Authored flavor.".into(),
            behavior: "Hunting · Illumination 20".into(),
            resources: "Health 40/40 · Energy 50/50".into(),
            your_attack: "You cannot damage them.".into(),
            their_attack: "They cannot damage you.".into(),
            abilities: "An ability.".into(),
            ability_details: "A full explanation. ".repeat(100),
            heart: "Heart: none".into(),
            ..Default::default()
        }
        .report();
        let lines = wrapped(&report, &p);
        let row = lines.iter().position(|l| l.disclosure).unwrap();
        let (x, y, w, h) = BOUNDS;
        assert!(!click(&mut report, &p, 0, x - 1, y + 78 + row as i32 * 21));
        assert!(click(&mut report, &p, 0, x + 20, y + 78 + row as i32 * 21));
        assert!(report.details_open);
        assert!(scroll_line_count(&report, &p) > 0);
        assert!(close_hit(x + w - 20, y + 20));
        assert!(!close_hit(x + w, y + 20));
        for _ in 0..10 {
            p.rect(
                0,
                0,
                crate::render::WIDTH as i32,
                crate::render::HEIGHT as i32,
                [93, 31, 113],
            );
            paint(&report, &mut p, 0);
            for (px, py) in [
                (x - 1, y),
                (x + w, y),
                (x, y - 1),
                (x, y + h),
                (1000, 300),
                (0, 748),
            ] {
                let at = (py as usize * crate::render::WIDTH + px as usize) * 4;
                assert_eq!(&p.pixels[at..at + 3], &[93, 31, 113]);
            }
            assert_eq!(p.clip, None);
            toggle_details(&mut report);
        }
    }
}

// Source combat projections: private actor copies, no resolver, RNG, log or
// lifecycle calls. Adjacency is an explicit inspection policy, never movement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Strike {
    chance: f64,
    minimum: f64,
    maximum: f64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Offhand {
    after_hit: Option<Strike>,
    after_miss: Option<Strike>,
    sequences: f64,
    intervening_effects: bool,
    stops_on_hit: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Attack {
    strike: Strike,
    defender_hp: f64,
    defender_max_hp: f64,
    reason: Option<String>,
    offhand: Option<Offhand>,
}
fn allied(r: &Run, a: &Actor) -> bool {
    a.id == r.player.id || a.state == "ally"
}
fn hostile(r: &Run, a: &Actor, b: &Actor) -> bool {
    a.id != b.id && allied(r, a) != allied(r, b)
}
fn is_active(r: &Run, a: &Actor, name: &str) -> bool {
    active(owner(r, a), name)
}
fn surprise(r: &Run, a: &Actor, b: &Actor) -> bool {
    allied(r, a) && matches!(b.state.as_str(), "sleeping" | "wandering")
}
fn angel_ready(r: &Run, a: &Actor) -> bool {
    let round = r.tick
        + if r.resolving_tick.value_or(false) {
            0.
        } else {
            1.
        };
    ability(a, "The Number of Blood")
        && round % 5. == 0.
        && owner(r, a).conditions.get("angelUsedRound") != Some(&round)
}
fn strike_damage(
    r: &Run,
    a: &Actor,
    b: &Actor,
    base: f64,
    variance: f64,
    extra: f64,
    apply_surrender: bool,
) -> f64 {
    let mut n = base + extra;
    if surprise(r, a, b) {
        n *= if ability(a, "Silent Strike") { 6. } else { 3. }
    } else if cond(b, "paralyzed") > 0. {
        n *= 3.
    }
    if ability(a, "Drown")
        && r.floor().is_some_and(|f| {
            f.tiles
                .get(a.y as usize * f.width() + a.x as usize)
                .is_some_and(|s| matches!(s.as_str(), "water" | "shallow-water"))
        })
    {
        n *= 3.
    }
    if angel_ready(r, a) {
        n *= 2.
    }
    if corpse_core::divine_combat::blood_weapon_ready(r, a) {
        n *= 2.
    }
    let defense = combat::effective_stats(r, b);
    let pool = owner(r, b);
    let surrender = if apply_surrender
        && is_active(r, b, "Never Surrender")
        && pool.energy >= 8.
        && pool.conditions.get("surrenderTick") != Some(&r.tick)
    {
        defense.defense
    } else {
        0.
    };
    let mitigation = if has(a, "enemy:beast:armor-piercing") {
        0.
    } else {
        defense.dr + combat::adaptive_armor_dr(pool, a) + surrender
    };
    if has(b, "enemy:beast:sphinx") {
        0.
    } else {
        combat::tuned_damage_for_run(r, n * variance - mitigation, Some("attack"))
    }
}
fn strike_preview(r: &Run, a: &Actor, b: &Actor, base: f64, multiplier: f64, extra: f64) -> Strike {
    strike_preview_ranged(r, a, b, base, multiplier, extra, 0., 1.)
}
fn strike_preview_ranged(
    r: &Run,
    a: &Actor,
    b: &Actor,
    base: f64,
    multiplier: f64,
    extra: f64,
    range: f64,
    miss_multiplier: f64,
) -> Strike {
    let stats = combat::effective_stats(r, a);
    let mut accuracy = stats.accuracy;
    if ability(a, "Shine") {
        accuracy += combat::integer_stat(vision::light(r, b));
    }
    let mut chance =
        combat::hit_chance(accuracy, combat::effective_stats(r, b).dodge, range) * multiplier;
    let guaranteed = (surprise(r, a, b) && !(a.id == r.player.id && range > 0.))
        || angel_ready(r, a)
        || corpse_core::divine_combat::blood_weapon_ready(r, a);
    if guaranteed {
        chance = 1.
    }
    if !corpse_core::revision50::enabled(r) && is_active(r, b, "Missed Me")
        && owner(r, b).reactions.value_or(0.) > 0.
        && cond(owner(r, b), "reacted") == 0.
    {
        chance *= 0.5;
    }
    if miss_multiplier != 1. {
        chance = 1. - (1. - chance.clamp(0., 1.)) * miss_multiplier;
    }
    if cond(a, "unlucky") != 0. && !guaranteed {
        chance = 0.
    }
    if cond(a, "lucky") != 0. {
        chance = 1.
    }
    if cond(a, "distracted") != 0. {
        chance = 0.
    }
    if cond(b, "paralyzed") > 0. {
        chance = 1.
    }
    if hostile(r, a, b)
        && (is_active(r, b, "Turning the Wheel")
            || (range > 0. && (a.x - b.x).hypot(a.y - b.y) > 5. && is_active(r, b, "Not There")))
    {
        chance = 0.
    }
    if definition(a).is_some_and(|d| d.id == "hemogoblin-mech")
        && !a
            .monster_state
            .as_ref()
            .is_some_and(|s| s.pilot.as_ref().is_some())
    {
        chance = 0.
    }
    Strike {
        chance: chance.clamp(0., 1.),
        minimum: strike_damage(r, a, b, base, 0.75, extra, true),
        maximum: strike_damage(r, a, b, base, 1.25, extra, true),
    }
}
fn weapon_hands(r: &Run) -> (Option<&Item>, Option<&Item>) {
    if corpse_core::revision50::enabled(r) { return (None, None); }
    let find = |slot: &str| {
        r.inventory.iter().find(|i| {
            i.kind == "weapon"
                && !corpse_core::projectiles::is_bow(i)
                && i.equipped.as_ref().is_some_and(|s| s == slot)
        })
    };
    (find("hand1"), find("hand2"))
}
fn weapon_base(item: Option<&Item>, offense: f64) -> f64 {
    item.map_or(30., |i| i.damage.value_or(30.) + i.enchantment.value_or(0.))
        + offense * item.map_or(1., |i| i.offense_multiplier.value_or(1.))
}
fn ordinary_base(r: &Run, a: &Actor, offense: f64) -> f64 {
    if corpse_core::revision50::enabled(r)
        && (owner(r, a).id == r.player.id
            || (r.simulation_revision.value_or(29.) < 51. && cond(a, "miniMe") != 0.))
    { return corpse_core::revision50_player_attack::with_offense_for_run(r, offense); }
    // R51 independent copies preview their stored baseline, including old snapshots.
    if owner(r, a).id == r.player.id {
        30. + offense
    } else {
        a.conditions.get("weaponDamage").copied().unwrap_or(10.)
            + offense
                * a.conditions
                    .get("weaponOffenseMultiplier")
                    .copied()
                    .unwrap_or(1.)
    }
}
fn preparation(r: &Run) -> (f64, f64, f64) {
    if corpse_core::revision50::enabled(r) { return (r.player.energy, 0., 1.); }
    let assault = is_active(r, &r.player, "Unyielding Assault") && r.player.energy >= 10.;
    let available = r.player.energy - if assault { 10. } else { 0. };
    let (fury, spent) = corpse_core::crafting::preview_fury_bonus(r, available);
    let energy = available - spent;
    (energy, fury, if assault { 2. } else { 1. })
}
fn blocked(r: &Run, a: &Actor, b: &Actor) -> Option<&'static str> {
    if !corpse_core::dreamwalking::same_plane(r, a, b) { return Some("different-plane"); }
    if corpse_core::dreamwalking::is_body(a) { return Some("sleeping-body"); }
    let pa = owner(r, a);
    let pb = owner(r, b);
    if pa.dead.value_or(false) || pa.hp <= 0. {
        return Some("attacker-dead");
    }
    if pb.dead.value_or(false) || pb.hp <= 0. {
        return Some("defender-dead");
    }
    if a.dead.value_or(false) || a.hp <= 0. {
        return Some("attacker-dead");
    }
    if half_here::is_clone(a) {
        return Some("nonattacking-clone");
    }
    if half_here::is_clone(b) && !r.actors_at(r.depth).iter().any(|p| p.id == b.id) {
        return Some("removed-clone");
    }
    if r.floor()
        .is_some_and(|f| !travel::habitat_valid(f, a, a.x, a.y))
    {
        return Some("stranded");
    }
    if b.dead.value_or(false) || b.hp <= 0. {
        return Some("defender-dead");
    }
    if definition(a).is_some_and(|d| d.id == "hemogoblin-mech")
        && !a
            .monster_state
            .as_ref()
            .is_some_and(|s| s.pilot.as_ref().is_some())
    {
        return Some("inactive-mech");
    }
    if has(a, "enemy:beast:hold-only") {
        return Some("harmless");
    }
    if has(a, "enemy:beast:rooted") {
        return Some("rooted");
    }
    if hostile(r, a, b) && is_active(r, b, "Turning the Wheel") {
        return Some("protected");
    }
    if r.status != "playing" {
        return Some("not-playing");
    }
    if cond(a, "paralyzed") > 0. {
        return Some("paralyzed");
    }
    if a.id == r.player.id {
        if cond(a, "riptide") != 0. && cond(a, "forcedMove") == 0. {
            return Some("immobilized");
        }
        if !travel::passable(r, a, b.x, b.y)
            || (cond(a, "forcedMove") == 0.
                && (has(a, "enemy:beast:rooted")
                    || r.floor().is_some_and(|f| {
                        !travel::habitat_valid(f, a, a.x, a.y)
                            || !travel::habitat_valid(f, a, b.x, b.y)
                    })))
        {
            return Some("inaccessible");
        }
    }
    None
}
/// Deliberately incomplete read-only combat context. Exact queried fields are
/// carried; replay/history/archive/log payloads are never cloned for a hover.
/// Must never be handed to a simulation resolver or persisted.
fn combat_view(r: &Run) -> Run {
    let mut view = Run {
        player: r.player.clone(),
        dreamwalking: r.dreamwalking.clone(),
        simulation_revision: r.simulation_revision.clone(),
        inventory: r.inventory.clone(),
        floors: r.floors.clone(),
        depth: r.depth,
        tick: r.tick,
        status: r.status.clone(),
        resolving_tick: r.resolving_tick.clone(),
        ..Run::default()
    };
    if let Some(actors) = r.actors.get(&r.depth.to_string()) {
        view.actors.insert(r.depth.to_string(), actors.clone());
    }
    view
}
fn project_attack(r: &Run, a: &Actor, b: &Actor) -> Attack {
    let pool = owner(r, b);
    let mut out = Attack {
        defender_hp: pool.hp.max(0.),
        defender_max_hp: pool.max_hp,
        ..Default::default()
    };
    if let Some(reason) = blocked(r, a, b) {
        out.reason = Some(reason.into());
        return out;
    }
    if corpse_core::revision50::enabled(r)
        && corpse_core::abilities::endpoint_active(r, b, "isheilah-missed-me")
        && owner(r, b).reactions.value_or(0.) > 0.
    {
        // A legal retreat replaces the attack; portal/forced-movement arrival
        // effects belong to simulation. Do not invent a probabilistic forecast.
        out.reason = Some("missed-me-retreat".into());
        return out;
    }
    let strike = if a.id == r.player.id {
        let mut view = combat_view(r);
        let (energy, fury, _) = preparation(r);
        view.player.energy = energy;
        let defender = if b.id == r.player.id { &view.player } else { b };
        let mut strike = strike_preview(
            &view,
            &view.player,
            defender,
            if corpse_core::revision50::enabled(r) {
                corpse_core::revision50_player_attack::base(r)
            } else {
                weapon_base(weapon_hands(r).0, combat::effective_stats(r, &r.player).offense)
            },
            1.,
            fury,
        );
        if corpse_core::divine_reactions::nightmare_absorbs(&view, defender, Some(&view.player)) {
            strike.minimum = 0.;
            strike.maximum = 0.;
        }
        strike
    } else {
        strike_preview(
            r,
            a,
            b,
            ordinary_base(r, a, combat::effective_stats(r, a).offense),
            1.,
            0.,
        )
    };
    out.strike = strike;
    if corpse_core::divine_reactions::nightmare_absorbs(r, b, Some(a)) {
        out.strike.minimum = 0.;
        out.strike.maximum = 0.;
    }
    out.offhand = offhand(r, a, b, strike.chance);
    out
}
fn offhand(r: &Run, a: &Actor, b: &Actor, chance: f64) -> Option<Offhand> {
    if corpse_core::revision50::enabled(r) { return None; }
    let player = a.id == r.player.id;
    let (main, second) = weapon_hands(r);
    if if player {
        second.is_none()
    } else {
        cond(a, "miniMe") == 0. || cond(a, "offhandDamage") <= 0.
    } {
        return None;
    }
    let offense = combat::effective_stats(r, a).offense;
    let primary = if player {
        weapon_base(main, offense)
    } else {
        ordinary_base(r, a, offense)
    };
    let secondary = if player {
        weapon_base(second, offense)
    } else {
        cond(a, "offhandDamage")
            + offense
                * a.conditions
                    .get("offhandOffenseMultiplier")
                    .copied()
                    .unwrap_or(1.)
    };
    let (energy, fury, sequences) = if player {
        preparation(r)
    } else {
        (a.energy, 0., 1.)
    };
    let mut initial = combat_view(r);
    if player {
        initial.player.energy = energy;
    }
    let ia = engine::actor(&initial, a.id).unwrap_or(a);
    let ib = engine::actor(&initial, b.id).unwrap_or(b);
    let target = owner(&initial, ib);
    let absorbed = corpse_core::divine_reactions::nightmare_absorbs(&initial, ib, Some(ia));
    let low = if absorbed {
        0.
    } else {
        strike_damage(&initial, ia, ib, primary, 0.75, fury, true)
    };
    let high = if absorbed {
        0.
    } else {
        strike_damage(&initial, ia, ib, primary, 1.25, fury, true)
    };
    let revive = chance > 0.
        && high >= target.hp
        && ((is_active(&initial, ib, "Rage at the Dying of the Light")
            && target.energy >= cond(target, "rageCost"))
            || ability(target, "Overflowing")
            || (is_active(&initial, ib, "You Shall Never Be Rid of Me")
                && cond(target, "pelianUsed") == 0.));
    let stops = chance > 0. && low >= target.hp && !revive;
    let intervening = revive
        || (is_active(&initial, ia, "Nova")
            && energy >= corpse_core::abilities::nova_attack_cost(initial.simulation_revision.value_or(29.)))
        || (chance > 0.
            && is_active(&initial, ia, "Erase Metaphysical")
            && energy >= 10.
            && !target.abilities.is_empty())
        || (is_active(&initial, ib, "Never Without a Fight") && target.energy >= 10.)
        || (chance > 0. && ability(ib, "I Hit Back") && cond(target, "hitBack") == 0.)
        || (chance < 1.
            && is_active(&initial, ib, "Are You Sure About That?")
            && target.reactions.value_or(0.) > 0.)
        || (chance > 0. && high > 0. && is_active(&initial, ib, "Float Away"))
        || is_active(&initial, ia, "Bonded By Unseen Threads")
        || is_active(&initial, ib, "Bonded By Unseen Threads");
    if intervening && !(stops && chance == 1.) {
        return Some(Offhand {
            sequences,
            intervening_effects: true,
            ..Default::default()
        });
    }
    let branch = |hit: bool, variance: f64| -> Strike {
        let mut view = combat_view(r);
        if player {
            view.player.energy = energy;
        }
        let aa = engine::actor(&view, a.id).unwrap_or(a).clone();
        let bb = engine::actor(&view, b.id).unwrap_or(b).clone();
        let owner_id = half_here::owner_id(&view, &aa);
        let target_id = half_here::owner_id(&view, &bb);
        let main_damage = if hit {
            strike_damage(&view, &aa, &bb, primary, variance, fury, true)
        } else {
            0.
        };
        let ordinary = if hit {
            strike_damage(&view, &aa, &bb, primary, variance, fury, false)
        } else {
            0.
        };
        let absorbed = hit
            && main_damage > 0.
            && corpse_core::divine_reactions::nightmare_absorbs(&view, &bb, Some(&aa));
        let angel = angel_ready(&view, &aa);
        let blood = corpse_core::divine_combat::blood_weapon_ready(&view, &aa);
        let miss = is_active(&view, &bb, "Missed Me")
            && owner(&view, &bb).reactions.value_or(0.) > 0.
            && cond(owner(&view, &bb), "reacted") == 0.;
        let surrender = hit
            && ordinary > 0.
            && is_active(&view, &bb, "Never Surrender")
            && owner(&view, &bb).conditions.get("surrenderTick") != Some(&r.tick)
            && owner(&view, &bb).energy >= 8.;
        if let Some(aa) = engine::actor_mut(&mut view, a.id) {
            for key in ["lucky", "unlucky", "distracted"] {
                aa.conditions.shift_remove(key);
            }
        }
        if miss {
            let t = engine::actor_mut(&mut view, target_id).unwrap();
            t.reactions = (t.reactions.value_or(0.) - 1.).into();
            t.conditions.insert("reacted".into(), 1.);
        }
        if angel {
            let round = r.tick
                + if r.resolving_tick.value_or(false) {
                    0.
                } else {
                    1.
                };
            let o = engine::actor_mut(&mut view, owner_id).unwrap();
            o.conditions.insert("angelUsed".into(), 1.);
            o.conditions.insert("angelUsedRound".into(), round);
        }
        if blood {
            engine::actor_mut(&mut view, owner_id)
                .unwrap()
                .conditions
                .shift_remove("bloodWeaponReady");
        }
        if hit {
            if surrender {
                let t = engine::actor_mut(&mut view, target_id).unwrap();
                t.energy -= 8.;
                t.conditions.insert("surrenderTick".into(), r.tick);
            }
            engine::actor_mut(&mut view, b.id)
                .unwrap()
                .conditions
                .shift_remove("paralyzed");
            let t = engine::actor_mut(&mut view, target_id).unwrap();
            if absorbed {
                t.nightmares = (t.nightmares.value_or(0.) - 1.).into();
            } else {
                let rem = t.hp % 1.;
                t.hp = (if rem == 0. { 1. } else { rem }).max(t.hp - main_damage);
            }
            let hp = t.hp;
            if target_id != b.id {
                engine::actor_mut(&mut view, b.id).unwrap().hp = hp;
            }
            let bb = engine::actor_mut(&mut view, b.id).unwrap();
            if ability(&aa, "Beat to Death") {
                let hits = cond(bb, "beatToDeathHits");
                bb.conditions.insert("beatToDeathHits".into(), hits + 1.);
            }
            if bb.state == "sleeping" {
                bb.state = "wandering".into()
            }
            if bb.state != "ally" && !has(bb, "enemy:beast:sphinx") {
                bb.state = "hunting".into()
            }
        }
        let aa = engine::actor(&view, a.id).unwrap();
        let bb = engine::actor(&view, b.id).unwrap();
        let mut result = strike_preview(&view, aa, bb, secondary, 0.5, 0.);
        if corpse_core::divine_reactions::nightmare_absorbs(&view, bb, Some(aa)) {
            result.minimum = 0.;
            result.maximum = 0.;
        }
        result
    };
    let mut hit_branches = Vec::new();
    if chance > 0. && !stops {
        hit_branches.push(branch(true, 0.75));
        hit_branches.push(branch(true, 1.25));
        if is_active(&initial, ib, "Never Surrender")
            && target.energy >= 8.
            && target.conditions.get("surrenderTick") != Some(&r.tick)
        {
            let activates = |v| strike_damage(&initial, ia, ib, primary, v, fury, false) > 0.;
            let low_activates = activates(0.75);
            if low_activates != activates(1.25) {
                let (mut l, mut h) = (0.75, 1.25);
                for _ in 0..48 {
                    let mid = (l + h) / 2.;
                    if activates(mid) == low_activates {
                        l = mid
                    } else {
                        h = mid
                    }
                }
                hit_branches.push(branch(true, l));
                hit_branches.push(branch(true, h));
            }
        }
    }
    Some(Offhand {
        after_hit: hit_branches.first().map(|first| Strike {
            chance: first.chance,
            minimum: hit_branches
                .iter()
                .map(|s| s.minimum)
                .fold(f64::INFINITY, f64::min),
            maximum: hit_branches
                .iter()
                .map(|s| s.maximum)
                .fold(f64::NEG_INFINITY, f64::max),
        }),
        after_miss: (chance < 1.).then(|| branch(false, 0.75)),
        sequences,
        intervening_effects: false,
        stops_on_hit: stops,
    })
}
fn attacks(r: &Run, a: &Actor) -> (Attack, Attack) {
    if matches!(a.state.as_str(), "ally" | "neutral" | "hunting") || a.id == r.player.id {
        return (
            project_attack(r, &r.player, a),
            project_attack(r, a, &r.player),
        );
    }
    let mut view = combat_view(r);
    let mut projected = a.clone();
    projected.state = "hunting".into();
    if let Some(entry) = view
        .actors_at_mut(r.depth)
        .iter_mut()
        .find(|b| b.id == a.id)
    {
        *entry = projected.clone();
    }
    (
        project_attack(&view, &view.player, &projected),
        project_attack(&view, &projected, &view.player),
    )
}
fn format_attack(a: &Attack, yours: bool) -> String {
    if let Some(reason) = a.reason.as_deref() {
        let pair = match reason {
            "different-plane" => ("They are in another plane; your projection cannot touch them.", "They are in another plane and cannot touch your projection."),
            "missed-me-retreat" => (
                "They have a Missed Me reaction available and may retreat instead of taking this melee attack. If no legal retreat exists, the attack proceeds.",
                "You have a Missed Me reaction available and may retreat instead of taking this melee attack. If no legal retreat exists, the attack proceeds.",
            ),
            "sleeping-body" => ("Your sleeping body cannot attack.", "Your sleeping body cannot attack."),
            "out-of-reach" => ("They are out of reach.", "You are out of their reach."),
            "inactive-mech" => (
                "You cannot attack until piloted.",
                "They cannot attack until piloted.",
            ),
            "attacker-dead" => (
                "You cannot attack while dead.",
                "They cannot attack while dead.",
            ),
            "defender-dead" => ("They are already dead.", "You are already dead."),
            "paralyzed" => (
                "You cannot attack while paralyzed.",
                "They cannot attack while paralyzed.",
            ),
            "immobilized" => (
                "You cannot attack while held in place by Riptide.",
                "They cannot attack while held in place by Riptide.",
            ),
            "stranded" => (
                "You cannot attack outside your habitat.",
                "They cannot attack outside their habitat.",
            ),
            "nonattacking-clone" => ("This body cannot attack.", "This body cannot attack."),
            "removed-clone" => (
                "This body is no longer present.",
                "This body is no longer present.",
            ),
            "harmless" => (
                "You have no ordinary damaging attack.",
                "They have no ordinary damaging attack.",
            ),
            "rooted" => (
                "You cannot make an ordinary attack while rooted.",
                "They cannot make an ordinary attack while rooted.",
            ),
            "protected" => (
                "You cannot currently attack them through their protection.",
                "They cannot currently attack you through your protection.",
            ),
            "blocked-projectile" => (
                "Your projectile is blocked.",
                "Their projectile is blocked.",
            ),
            "inaccessible" => (
                "You cannot reach them from this terrain.",
                "They cannot reach you from their terrain.",
            ),
            "not-playing" => (
                "You cannot attack after the descent has ended.",
                "They cannot attack after the descent has ended.",
            ),
            _ => (
                "You cannot currently attack them.",
                "They cannot currently attack you.",
            ),
        };
        return if yours { pair.0 } else { pair.1 }.into();
    }
    let percent = |n: f64| (n / a.defender_max_hp * 100.).max(0.).round();
    let hit = (a.strike.chance.clamp(0., 1.) * 100.).round();
    if let Some(off) = &a.offhand {
        let strike = |s: Strike| {
            format!(
                "{}% hit · {}–{}% health/hit",
                (s.chance.clamp(0., 1.) * 100.).round(),
                percent(s.minimum),
                percent(s.maximum)
            )
        };
        let hits = if a.strike.maximum > 0. && a.strike.chance > 0. {
            (a.defender_hp / a.strike.maximum).ceil()
        } else {
            0.
        };
        let main = format!(
            "Main hand: {}{}",
            strike(a.strike),
            if hits > 0. {
                format!(
                    " · best: {hits} main {}",
                    if hits == 1. { "hit" } else { "hits" }
                )
            } else {
                String::new()
            }
        );
        let h = off.after_hit.map(strike);
        let m = off.after_miss.map(strike);
        let second = if off.intervening_effects {
            "Offhand: extra strike; hit chance and damage depend on intervening effects.".into()
        } else if off.stops_on_hit {
            m.as_ref()
                .map_or("Offhand: no strike; main hand is lethal.".into(), |s| {
                    format!("Offhand only after main miss: {s}")
                })
        } else if h.is_some() && m.is_some() && h != m {
            format!(
                "Offhand after main hit: {}\nOffhand after main miss: {}",
                h.unwrap(),
                m.clone().unwrap()
            )
        } else {
            format!("Offhand: {}", h.or(m.clone()).unwrap_or_default())
        };
        return format!(
            "{main}\n{second}{}{}",
            if off.intervening_effects || (off.stops_on_hit && m.is_none()) {
                ""
            } else {
                "\nOffhand requires both to survive."
            },
            if off.sequences > 1. {
                "\nUnyielding Assault: two sequences; first shown."
            } else {
                ""
            }
        );
    }
    if a.strike.maximum <= 0. {
        return if yours {
            "You cannot damage them."
        } else {
            "They cannot damage you."
        }
        .into();
    }
    let hits = (a.defender_hp.max(0.) / a.strike.maximum).ceil();
    let count = format!("{hits} {}", if hits == 1. { "hit" } else { "hits" });
    let range = format!(
        "{}–{}%",
        percent(a.strike.minimum),
        percent(a.strike.maximum)
    );
    if yours {
        format!("You hit {hit}% of the time and deal {range} of their health per hit. At best, you could kill them in {count}.")
    } else {
        format!("They hit you {hit}% of the time and deal {range} of your health per hit. At best, they could kill you in {count}.")
    }
}

#[cfg(test)]
mod hover_tests {
    use super::*;
    #[test]
    fn revision50_missed_me_forecast_is_conditional_retreat_not_halved_odds() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 50.0.into(); r.inventory.clear(); r.player.conditions.clear();
        r.player.abilities = vec!["isheilah-missed-me".into()]; r.player.active = r.player.abilities.clone();
        r.player.reactions = 1.0.into(); r.player.stats.dodge = 0.0;
        let enemy = Actor { id: 991.0, x: r.player.x + 1.0, y: r.player.y,
            hp: 100.0, max_hp: 100.0, state: "hunting".into(), ..Default::default() };
        r.actors_at_mut(r.depth).clear(); r.actors_at_mut(r.depth).push(enemy.clone());
        let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into());
        f.environment = Field::Missing; f.doors = Field::Missing;
        let before = serde_json::to_vec(&r).unwrap();
        assert_eq!(strike_preview(&r, &enemy, &r.player, 30.0, 1.0, 0.0).chance, 0.65);
        let forecast = project_attack(&r, &enemy, &r.player);
        assert_eq!(forecast.reason.as_deref(), Some("missed-me-retreat"));
        let text = format_attack(&forecast, false);
        assert!(text.contains("reaction available")); assert!(text.contains("If no legal retreat exists"));
        assert!(!text.contains('%'));
        assert_eq!(serde_json::to_vec(&r).unwrap(), before);
        r.player.reactions = 0.0.into();
        assert_ne!(project_attack(&r, &enemy, &r.player).reason.as_deref(), Some("missed-me-retreat"));
        r.player.reactions = 1.0.into(); r.simulation_revision = 46.0.into();
        assert_eq!(strike_preview(&r, &enemy, &r.player, 30.0, 1.0, 0.0).chance, 0.325);
    }
    #[test]
    fn revision50_melee_previews_share_greatsword_base_but_bow_stays_explicit() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 50.0.into(); r.inventory.clear();
        r.player.stats.offense = 20.0; r.player.conditions.clear();
        for (id, slot) in [(901.0, "hand1"), (902.0, "hand2")] {
            r.inventory.push(Item { id, kind: "weapon".into(), damage: 999.0.into(),
                equipped: slot.to_string().into(), ..Default::default() });
        }
        let mut copy = r.player.clone(); copy.id = 903.0;
        copy.conditions.insert("miniMe".into(), 1.0);
        copy.conditions.insert("offhandDamage".into(), 999.0);
        let before = serde_json::to_vec(&r).unwrap();
        assert_eq!(weapon_hands(&r), (None, None));
        assert_eq!(ordinary_base(&r, &r.player, 20.0), 85.0);
        assert_eq!(ordinary_base(&r, &copy, 20.0), 85.0);
        assert_eq!(preparation(&r), (r.player.energy, 0.0, 1.0));
        assert!(offhand(&r, &r.player, &copy, 1.0).is_none());
        assert!(offhand(&r, &copy, &r.player, 1.0).is_none());
        let bow = Item { kind: "weapon".into(), damage: 15.0.into(),
            offense_multiplier: 0.5.into(), ranged_weapon: "bow".to_string().into(),
            ..Default::default() };
        assert_eq!(weapon_base(Some(&bow), 20.0), 25.0);
        assert_eq!(serde_json::to_vec(&r).unwrap(), before);
        r.simulation_revision = 46.0.into();
        assert!(weapon_hands(&r).0.is_some());
        assert!(weapon_hands(&r).1.is_some());
        assert_eq!(ordinary_base(&r, &r.player, 20.0), 50.0);
    }
    #[test]
    fn nova_intervening_offhand_preview_uses_revision_funding_threshold() {
        for (revision, energy, intervenes) in [(44.,29.,false),(44.,30.,true),(45.,39.,false),(45.,40.,true)] {
            let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            r.simulation_revision = revision.into(); r.inventory.clear();
            r.player.abilities = vec!["anavani-nova".into()]; r.player.active = r.player.abilities.clone();
            r.player.conditions.clear(); r.player.energy = energy; r.player.stats.offense = 0.;
            r.inventory.push(Item {id:500.,name:"QA offhand".into(),kind:"weapon".into(),hands:1.0.into(),
                damage:0.0.into(),equipped:"hand2".to_string().into(),..Default::default()});
            let mut b = r.player.clone(); b.id = 1000.; b.x += 1.; b.state = "hunting".into();
            b.abilities.clear(); b.active.clear(); b.hp = 1000000.; b.max_hp = b.hp;
            *r.actors_at_mut(r.depth) = vec![b.clone()];
            let before = serde_json::to_value(&r).unwrap();
            let forecast = offhand(&r, &r.player, &b, 0.5).unwrap();
            assert_eq!(forecast.intervening_effects, intervenes, "revision={revision}, energy={energy}");
            assert_eq!(serde_json::to_value(&r).unwrap(), before);
        }
    }
    #[test]
    fn source_hover_geometry_80_cases() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/hover-ui-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let t = &c["tile"];
            let v = &c["viewport"];
            let e = &c["expected"];
            let n = |v: &serde_json::Value, k: &str| v[k].as_i64().unwrap() as i32;
            let got = hover_bounds(
                (n(t, "left"), n(t, "top"), n(t, "right"), n(t, "bottom")),
                n(c, "height"),
                (n(v, "width"), n(v, "height")),
            );
            assert_eq!(got.0, e["left"].as_f64().unwrap().floor() as i32);
            assert_eq!(got.1, e["top"].as_f64().unwrap().floor() as i32);
            assert_eq!(got.2, n(e, "width"));
            assert_eq!(got.3, n(c, "height").min(n(e, "maxHeight")));
        }
    }
    #[test]
    fn resource_projection_preserves_clone_sphinx_and_infinity() {
        let mut r = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        let mut a = r.player.clone();
        a.id = 777.;
        a.hp = 2.25;
        a.energy = 23.5;
        a.abilities = vec![
            "enemy:beast:sphinx".into(),
            "grisha-boundless-hunger".into(),
        ];
        a.active.clear();
        r.inventory.clear();
        let meters = hover_resources(&r, &a);
        assert_eq!(meters[0].label, "Health · Invulnerable");
        assert_eq!(meters[0].value, None);
        assert_eq!(meters[1].maximum, None);
        assert_eq!(meters[1].reference, 100.);
        assert_eq!(meters[1].value, Some(23.5));
        a.conditions.insert("halfHereClone".into(), 1.);
        a.conditions.insert("halfHereOwner".into(), r.player.id);
        let shared = hover_resources(&r, &a);
        assert_eq!(shared.len(), 1);
        assert_eq!(shared[0].label, "Shares your Health and Energy");
    }
    #[test]
    fn hover_is_detached_bounded_and_has_no_disclosure_hitbox() {
        let mut p = Painter::new();
        p.rect(
            0,
            0,
            crate::render::WIDTH as i32,
            crate::render::HEIGHT as i32,
            [93, 31, 113],
        );
        let original = p.pixels.clone();
        paint_hover(&Report::default(), &mut p, 500, 400);
        assert_eq!(p.pixels, original);
        let report = CreatureReport {
            title: "Observed creature".into(),
            hover_available: true,
            description: "Authored flavor. ".repeat(200),
            ability_details: "FULL PRIVATE DISCLOSURE".repeat(100),
            hover_resources: vec![HoverResource {
                label: "Energy".into(),
                value: Some(50.),
                maximum: None,
                reference: 100.,
                color: [36, 223, 255],
            }],
            ..Default::default()
        }
        .report();
        let before = report.clone();
        for point in [(10, 130), (900, 690), (600, 400)] {
            paint_hover(&report, &mut p, point.0, point.1);
            assert_eq!(p.clip, None);
            for x in 974..crate::render::WIDTH {
                for y in 0..crate::render::HEIGHT {
                    let at = (y * crate::render::WIDTH + x) * 4;
                    assert_eq!(&p.pixels[at..at + 3], &[93, 31, 113]);
                }
            }
        }
        assert_eq!(report, before);
        let mut dead = report.clone();
        dead.creature.as_mut().unwrap().hover_available = false;
        let pixels = p.pixels.clone();
        paint_hover(&dead, &mut p, 100, 100);
        assert_eq!(p.pixels, pixels);
    }
}
#[cfg(test)]
mod bow_tests {
    use super::*;
    #[test]
    fn source_141_bow_hover_forecasts() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/targeting-ui-v48.json")).unwrap();
        for c in cases["bows"].as_array().unwrap() {
            let r: Run = serde_json::from_value(c["run"].clone())
                .unwrap_or_else(|e| panic!("{}: {e}", c["name"]));
            let before = r.clone();
            let a = r
                .actors_at(r.depth)
                .iter()
                .find(|a| a.id == c["actor"].as_f64().unwrap())
                .unwrap();
            let bow = r
                .inventory
                .iter()
                .find(|i| i.id == c["bow"].as_f64().unwrap())
                .unwrap();
            let trace = serde_json::from_value(c["trace"].clone()).unwrap();
            assert_eq!(
                bow_text(&r, bow, a, &trace),
                c["expected"].as_str().unwrap(),
                "{}",
                c["name"]
            );
            assert_eq!(r, before);
        }
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn authored_inspector_scroll_disclosure_and_hover_share_resolved_geometry() {
        let mut p = Painter::new();
        let mut report = CreatureReport {
            title: "Observed creature".into(),
            description: "A detailed observed creature description with several wrapped lines. "
                .repeat(4),
            abilities: "One observed ability.".into(),
            ability_details:
                "Complete mechanics remain reachable through disclosure and scrolling. ".repeat(40),
            hover_available: true,
            ..Default::default()
        }
        .report();
        for appearance in [
            json!({}),
            json!({"ui":{"layout":{"rightSidebarWidth":600,"sidebarWidthPercent":40,"chronicleHeight":260,"tooltipMaxWidth":240,"borderWidth":4}},"typography":{"primary":{"fontSize":24,"lineHeight":1.8},"headings":{"fontSize":30},"tooltips":{"fontSize":24,"lineHeight":1.8}}}),
        ] {
            p.profile = crate::appearance_profile::Profile::from_value(appearance).unwrap();
            report.details_open = false;
            let (bx, by, w, h) = bounds(&p.profile);
            assert!(close_hit_profile(&p, bx + w - 20, by + 20));
            assert!(!close_hit_profile(&p, bx + w, by + 20));
            let rows = wrapped(&report, &p);
            let disclosure = rows.iter().position(|r| r.disclosure).unwrap();
            let line = role_line(&p, Role::Primary, 15, 21);
            let scroll = disclosure;
            assert!(click(
                &mut report,
                &p,
                scroll,
                bx + 20,
                by + content_top(&p) + line / 2
            ));
            assert!(report.details_open);
            assert!(scroll_line_count(&report, &p) > 0);
            p.clear([93, 31, 113]);
            let visible_scroll = scroll.min(scroll_line_count(&report, &p));
            paint(&report, &mut p, visible_scroll);
            for (x, y) in [(bx - 1, by), (bx + w, by), (bx, by - 1), (bx, by + h)] {
                let at = (y as usize * crate::render::WIDTH + x as usize) * 4;
                assert_eq!(&p.pixels[at..at + 3], &[93, 31, 113]);
            }
            if let Ok(dir) = std::env::var("CORPSE_MODAL_SCREENSHOTS") {
                std::fs::create_dir_all(&dir).unwrap();
                let file = std::fs::File::create(std::path::Path::new(&dir).join(
                    if p.profile == crate::appearance_profile::Profile::default() {
                        "inspect-default.png"
                    } else {
                        "inspect-authored.png"
                    },
                ))
                .unwrap();
                let mut e = png::Encoder::new(
                    file,
                    crate::render::WIDTH as u32,
                    crate::render::HEIGHT as u32,
                );
                e.set_color(png::ColorType::Rgba);
                e.set_depth(png::BitDepth::Eight);
                e.write_header()
                    .unwrap()
                    .write_image_data(&p.pixels)
                    .unwrap();
            }
            let tile = (710, 680, 730, 700);
            let hover = modal_ui::hover_bounds_painter(&p, tile, 1000);
            assert!(hover.2 <= p.profile.number("ui.layout.tooltipMaxWidth") as i32);
            let original = report.clone();
            paint_hover_at(&report, &mut p, tile);
            assert_eq!(report, original);
        }
    }
}

#[cfg(test)]
mod supplied_terrain_tests {
    use super::*;
    fn assert_parity(r: &Run) {
        let before=serde_json::to_vec(r).unwrap();
        let view=crate::observe::observe(r);
        let terrain=view.terrain_snapshot.as_deref().unwrap();
        for (x,y) in [(10,10),(11,10),(12,10),(13,10),(14,10),(60,20),(usize::MAX,0)] {
            assert_eq!(observe_tile_with_terrain(r,x,y,terrain),observe_tile(r,x,y),"tile {x},{y}");
            assert_eq!(observe_bow_tile_with_terrain(r,x,y,0.,terrain),observe_bow_tile(r,x,y,0.),"bow tile {x},{y}");
        }
        let mut invalid=terrain.clone(); invalid.width+=1;
        assert_eq!(observe_tile_with_terrain(r,11,10,&invalid),observe_tile(r,11,10));
        assert_eq!(serde_json::to_vec(r).unwrap(),before);
    }
    #[test]
    fn supplied_terrain_matches_fresh_reports_including_hallucination_and_same_tick_changes() {
        let mut r=crate::non_euclidean_ui::tests::scene();
        r.simulation_revision=43.0.into();
        r.actors_at_mut(r.depth).push(corpse_core::population::create_actor_revision("rat",900.,13.,10.,false,43.).unwrap());
        assert_parity(&r);
        // Same-tick changes require a fresh descriptor; tick alone is not a key.
        let tick=r.tick;
        r.player.conditions.insert("hallucinating".into(),100.);
        r.player.conditions.insert("telepathic".into(),100.);
        assert_parity(&r);
        r.player.conditions.clear();
        let f=r.floor_mut().unwrap();let w=f.width();
        for y in 1..f.height()-1 {f.tiles[y*w+20]="wall".into();}
        f.seen.fill(false);f.remembered_tiles=Field::Missing;
        assert_parity(&r);
        assert_eq!(r.tick,tick);
    }
    #[test]
    fn supplied_terrain_matches_bow_forecast_fixture_and_hallucinated_variants() {
        let cases:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/targeting-ui-v48.json")).unwrap();
        for c in cases["bows"].as_array().unwrap() {
            let mut r:Run=serde_json::from_value(c["run"].clone()).unwrap();
            let id=c["actor"].as_f64().unwrap();let bow=c["bow"].as_f64().unwrap();
            let a=r.actors_at(r.depth).iter().find(|a|a.id==id).unwrap();let (x,y)=(a.x as usize,a.y as usize);
            for hallucinating in [false,true] {
                if hallucinating {r.simulation_revision=43.0.into();r.player.conditions.insert("hallucinating".into(),100.);}
                let before=serde_json::to_vec(&r).unwrap();let view=crate::observe::observe(&r);let terrain=view.terrain_snapshot.as_deref().unwrap();
                assert_eq!(observe_bow_tile_with_terrain(&r,x,y,bow,terrain),observe_bow_tile(&r,x,y,bow),"{}",c["name"]);
                assert_eq!(serde_json::to_vec(&r).unwrap(),before);
            }
        }
    }
    #[test]
    fn supplied_terrain_matches_dream_projection_and_half_here() {
        let mut r=crate::non_euclidean_ui::tests::scene();
        r.simulation_revision=43.0.into();
        let mut body=r.player.clone();body.id=901.;body.conditions.insert("phantasmiiBody".into(),1.);
        body.conditions.insert("halfHereOwner".into(),r.player.id);r.actors_at_mut(r.depth).push(body);
        r.dreamwalking=corpse_core::dreamwalking::State {active:true,depth:r.depth,body_id:Some(901.),anchor_x:r.player.x,anchor_y:r.player.y,..Default::default()}.into();
        corpse_core::dreamwalking::ensure_layer(&mut r);
        assert_parity(&r);
        let mut clone=r.player.clone();clone.id=902.;clone.x=14.;clone.y=10.;clone.conditions.insert("halfHereClone".into(),1.);clone.conditions.insert("halfHereOwner".into(),r.player.id);
        r.actors_at_mut(r.depth).push(clone);
        assert_parity(&r);
    }
}

#[cfg(test)]
mod revision51_rounding_tests {
    use super::*;
    #[test]
    fn revision51_previews_use_reduced_player_base_and_stored_copy_baselines() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.inventory.clear();r.player.conditions.clear();
        r.player.abilities.clear();r.player.active.clear();r.player.stats.offense=20.;
        let fresh=corpse_core::mini_me::preview(&r,7.,7.);
        assert_eq!(ordinary_base(&r,&r.player,20.),71.5);
        assert_eq!(ordinary_base(&r,&fresh,20.),71.5);
        let mut saved=fresh.clone();saved.conditions.insert("weaponDamage".into(),45.);
        assert_eq!(ordinary_base(&r,&saved,20.),85.);
        let enemy=Actor{id:901.,stats:corpse_core::Stats{offense:20.,..Default::default()},..Default::default()};
        assert_eq!(ordinary_base(&r,&enemy,20.),30.);
        let bow=Item{kind:"weapon".into(),damage:15.0.into(),enchantment:3.0.into(),
            offense_multiplier:0.5.into(),ranged_weapon:"bow".to_string().into(),..Default::default()};
        assert_eq!(corpse_core::engine::player_weapon_strike_base(&r,&bow),23.5);
    }
    #[test]
    fn revision51_attack_and_gas_previews_share_packet_rounding() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.inventory.clear();
        let a=Actor{id:901.,hp:100.,max_hp:100.,state:"hunting".into(),..Default::default()};
        let b=Actor{id:902.,hp:100.,max_hp:100.,state:"hunting".into(),stats:corpse_core::Stats{dr:10.,..Default::default()},..Default::default()};
        *r.actors_at_mut(r.depth)=vec![a.clone(),b.clone()];
        assert_eq!(strike_damage(&r,&a,&b,30.,0.75,0.,false),6.);
        r.simulation_revision=50.0.into();assert_eq!(strike_damage(&r,&a,&b,30.,0.75,0.,false),7.);
        r.simulation_revision=51.0.into();r.player.max_hp=100.;
        r.inventory.push(Item{id:800.,name:"Ring of Corrosion Resistance".into(),kind:"ring".into(),effect:corpse_core::revision50_rings::CORROSION_RESISTANCE.to_string().into(),..Default::default()});
        let gas=corpse_core::GasPresence{r#type:"caustic".into(),..Default::default()};
        let before=r.clone();assert!(gas_exposure(&r,&r.player,&gas).contains("3 HP per normal turn"));assert_eq!(r,before);
    }
}

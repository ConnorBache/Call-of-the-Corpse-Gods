//! Boundary-only semantic validation transcribed from v48 persistence validators.
//! Missing optional properties remain absent; malformed values are never repaired here.
use crate::{Run, storage::StorageError};
use serde_json::Value;
type Result<T> = std::result::Result<T, StorageError>;
fn fail(p: &str) -> StorageError {
    StorageError(format!("Invalid or incompatible save: {p}"))
}
fn req(ok: bool, p: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(fail(p)) }
}
fn num(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite())
}
fn integer(v: &Value) -> bool {
    num(v).is_some_and(|n| n.fract() == 0.0)
}
fn safe(v: &Value) -> bool {
    num(v).is_some_and(|n| n.fract() == 0.0 && n.abs() <= 9007199254740991.0)
}
fn nonnegative(v: &Value) -> bool {
    num(v).is_some_and(|n| n >= 0.0)
}
fn count(v: &Value) -> bool {
    safe(v) && nonnegative(v)
}
fn cell(v: &Value, n: usize) -> bool {
    integer(v) && num(v).is_some_and(|v| v >= 0.0 && v < n as f64)
}
fn one(v: &Value, choices: &[&str]) -> bool {
    v.as_str().is_some_and(|s| choices.contains(&s))
}
fn array<'a>(v: &'a Value, p: &str) -> Result<&'a Vec<Value>> {
    v.as_array().ok_or_else(|| fail(p))
}
fn object<'a>(v: &'a Value, p: &str) -> Result<&'a serde_json::Map<String, Value>> {
    v.as_object().ok_or_else(|| fail(p))
}
fn optional(v: &Value, k: &str, f: impl FnOnce(&Value) -> bool) -> bool {
    v.get(k).is_none_or(f)
}
fn text(v: &Value, max: usize) -> bool {
    v.as_str().is_some_and(|s| {
        !s.trim().is_empty()
            && s.encode_utf16().count() <= max
            && !s.chars().any(|c| (c as u32) < 32)
            && !["__proto__", "prototype", "constructor"].contains(&s)
    })
}
fn coordinate(v: &Value, w: usize, h: usize) -> bool {
    cell(&v["x"], w) && cell(&v["y"], h)
}
fn decimal_key(k: &str) -> bool {
    !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) && (k == "0" || !k.starts_with('0'))
}
/// R52 extends only the player's meter map. Removed/dormant meters can survive
/// until the next turn, so validate identity syntax/catalog, not live ownership.
/// O(A * L) catalog lookup, bounded L <= 128; no user-authored meter labels.
fn resource_meter_key(key: &str, revision: f64, player: bool) -> bool {
    if ["hp", "energy", "blessings"].contains(&key) { return true; }
    if revision < 52. || !player { return false; }
    if let Some(id) = key.strip_prefix("ability:") {
        return !id.is_empty() && id.len() <= 128
            && id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'-' | b':'))
            && (crate::progression::definition_for_revision(revision, id).is_some()
                || crate::abilities::copied_trait_supported(id));
    }
    key.strip_prefix("amulet:").is_some_and(|id| id.len() <= 16 && decimal_key(id)
        && id.parse::<u64>().is_ok_and(|id| id <= 9_007_199_254_740_991))
}
fn sparse(v: &Value, n: usize, p: &str, f: impl Fn(&Value) -> bool) -> Result<()> {
    for (k, x) in object(v, p)? {
        req(
            decimal_key(k) && k.parse::<usize>().is_ok_and(|i| i < n) && f(x),
            p,
        )?;
    }
    Ok(())
}
fn cell_list(v: &Value, n: usize, p: &str) -> Result<()> {
    let a = array(v, p)?;
    req(a.len() <= n && a.iter().all(|x| cell(x, n)), p)
}
const GAS: [&str; 11] = [
    "bloodMist",
    "hallucinogenic",
    "deathGas",
    "caustic",
    "breather",
    "paralysis",
    "confusion",
    "flammable",
    "explosive",
    "hydrogen",
    "steam",
];
const FIRE: [&str; 8] = [
    "plain",
    "gas",
    "explosion",
    "gas-explosion",
    "creature",
    "embers",
    "brazier",
    "ichor",
];
pub fn validate(run: &Run) -> Result<()> {
    crate::revision50_amulets::validate_run(run).map_err(StorageError)?;
    crate::revision50_control_amulets::validate_terrain(run).map_err(StorageError)?;
    req(crate::sub_biomes_validation::validate_run(run), "sub-biome terrain layers")?;
    req(crate::portals_generation::validate_run(run), "portal metadata")?;
    req(crate::non_euclidean_generation::validate_run(run), "non-Euclidean metadata")?;
    crate::dreamwalking::validate(run).map_err(StorageError)?;
    crate::timeweave::validate_run(run).map_err(StorageError)?;
    crate::playtest::validate_profile(run).map_err(StorageError)?;
    crate::finale::validate(run).map_err(StorageError)?;
    crate::blood_mist::validate_fire_rules(run).map_err(StorageError)?;
    crate::half_here::validate_lock_policy(run).map_err(StorageError)?;
    crate::screamer_rules::validate_range(run).map_err(StorageError)?;
    crate::death_gas_rules::validate(run).map_err(StorageError)?;
    crate::minotaur_rules::validate(run).map_err(StorageError)?;
    crate::borge::validate(run).map_err(StorageError)?;
    for f in &run.floors {
        req(crate::pocket_biomes::validate(f), "pocket biome metadata")?;
        req(crate::entrance::validate(f) || crate::entrance::metadata_cell(f)
            .is_some_and(|at| crate::finale::collapsed(run, f.depth, at)), "entrance identity")?;
        req(
            crate::generation::validate_floor_generation(f),
            "generation diagnostics",
        )?;
        req(
            crate::population::validate_encounter_metadata(&f.encounters),
            "encounter metadata",
        )?;
    }
    let v = serde_json::to_value(run)?;
    validate_value(&v)
}
pub fn validate_value(v: &Value) -> Result<()> {
    let revision = num(&v["simulationRevision"]).unwrap_or(29.0);
    req(optional(v, "fastMode", |flag| flag.is_boolean() && (!flag.as_bool().unwrap() || revision == 52. && v["mode"] == "Avatar" && v["dev"] == false)), "Fast mode flag")?;
    req(optional(v, "compressedDepths", |flag| flag.is_boolean() && (!flag.as_bool().unwrap() || v["fastMode"] == true)), "Dungeon route flag")?;
    if let Some(phase)=v.get("fastActionPhase").filter(|v|!v.is_null()) {
        req(revision>=52. && one(phase,&["haste","normal"]) && v["phase"]=="normal", "Fast action phase")?;
    }
    req(revision_ability_fields(v, revision), "revision-specific ability, reserve or trail")?;
    req(
        optional(v, "veridianSapHarvested", Value::is_boolean),
        "Veridian sap flag",
    )?;
    req(optional(v, "compositeBowGenerated", Value::is_boolean), "Composite bow generation flag")?;
    req(
        optional(v, "divineRevision", |x| num(x) == Some(1.0))
            && optional(v, "enemyHealthRevision", |x| num(x) == Some(1.0)),
        "revision",
    )?;
    if let Some(grants) = v.get("generatedAbilityGrants") {
        let a = array(grants, "generated grants")?;
        req(
            a.iter().all(|id| generated_ability(id, revision)) && a.iter().enumerate().all(|(i, x)| !a[..i].contains(x)),
            "generated ability grant",
        )?;
    }
    validate_pilot_ids(v)?;
    let floors = array(&v["floors"], "floors")?;
    let inventory = array(&v["inventory"], "inventory")?;
    // Only R50 trinkets escape the defensive legacy entry bound. Every carried
    // ring/amulet counts regardless of historical equipped flags. Decode once to
    // reuse runtime classification and enforce all four independent quantity caps;
    // rejection never drops, rewrites or silently stashes an over-limit item.
    if revision >= 50.0 {
        let items: Vec<crate::Item> = serde_json::from_value(v["inventory"].clone())?;
        let abilities: Vec<String> = serde_json::from_value(v["player"]["abilities"].clone())?;
        req(items.iter().filter(|i| !crate::revision50_inventory::unlimited(i)).count() <= 100,
            "inventory capacity")?;
        req(crate::revision50_inventory::finite_categories_fit_revision(&items, &abilities, revision),
            "Revision 50 backpack category capacity")?;
    } else {
        req(inventory.len() <= 100, "inventory capacity")?;
    }
    if let Some(id) = v.get("runId") {
        req(
            id.as_str()
                .is_some_and(|s| !s.is_empty() && s.encode_utf16().count() <= 200),
            "runId",
        )?;
    }
    if let Some(kinds) = v.get("logKinds") {
        let a = array(kinds, "logKinds")?;
        req(
            a.len() == array(&v["log"], "log")?.len()
                && a.iter()
                    .all(|x| one(x, &["player", "enemy", "environment", "unknown"])),
            "logKinds",
        )?;
    }
    if let Some(a) = v.get("logArchive") {
        req(
            num(&a["version"]) == Some(1.0) && a["incomplete"].is_boolean(),
            "logArchive",
        )?;
        let messages = array(&a["messages"], "logArchive.messages")?;
        req(messages.iter().all(Value::is_string), "archive messages")?;
        req(
            array(&a["events"], "archive events")?
                .iter()
                .all(|x| count(x) && num(x).unwrap() < (messages.len() * 4) as f64),
            "archive event code",
        )?;
    }
    if let Some(h) = v.get("resourceHistory") {
        req(
            count(&h["tick"])
                && integer(&h["depth"])
                && num(&h["depth"]).is_some_and(|n| (1.0..=40.0).contains(&n)),
            "resource history clock",
        )?;
        for (id, meters) in object(&h["actors"], "resource actors")? {
            req(
                !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()),
                "resource actor id",
            )?;
            for (k, m) in object(meters, "resource meters")? {
                req(
                    resource_meter_key(k, revision,
                        num(&v["player"]["id"]).is_some_and(|player| *id == player.to_string()))
                        && ["start", "max", "loss"].iter().all(|k| nonnegative(&m[*k]))
                        && optional(m, "displayReference", Value::is_boolean),
                    "resource meter",
                )?;
            }
        }
    }
    if let Some(s) = v.get("runStats") {
        req(
            num(&s["version"]) == Some(1.0)
                && count(&s["blessingsReceived"])
                && s["blessingsComplete"].is_boolean()
                && s["heartsComplete"].is_boolean(),
            "run statistics",
        )?;
        for (k, n) in object(&s["heartsEaten"], "hearts eaten")? {
            req(
                text(&Value::String(k.clone()), 200) && count(n) && num(n).unwrap() > 0.0,
                "heart count",
            )?;
        }
        if let Some(d) = s.get("death") {
            req(
                text(&d["reason"], 512)
                    && count(&d["tick"])
                    && integer(&d["depth"])
                    && num(&d["depth"]).is_some_and(|n| (1.0..=40.0).contains(&n))
                    && d["unpreventable"].is_boolean()
                    && optional(d, "sourceName", |v| text(v, 200))
                    && optional(d, "cause", |v| text(v, 200)),
                "run death",
            )?;
        }
    }
    if let Some(dream) = v.get("dreamwalking") {
        for f in array(&dream["floors"], "dream floors")? {
            let n = array(&f["tiles"], "dream tiles")?.len();
            let w = if n == 2291 {79} else {(n as f64).sqrt() as usize};
            let h = n / w.max(1);
            floor(f, n, w, h)?;
        }
    }
    for f in floors {
        let tiles = array(&f["tiles"], "tiles")?;
        let n = tiles.len();
        let w = if n == 2291 {
            79
        } else {
            (n as f64).sqrt() as usize
        };
        let h = n / w.max(1);
        req(
            optional(f, "width", |v| num(v) == Some(w as f64))
                && optional(f, "height", |v| num(v) == Some(h as f64)),
            "floor metadata geometry",
        )?;
        floor(f, n, w, h)?;
        let depth = num(&f["depth"]).ok_or_else(|| fail("floor depth"))?.to_string();
        for actor in array(&v["actors"][&depth], "actors floor")? {
            req(
                coordinate(actor, w, h)
                    && num(&actor["hp"]).is_some()
                    && num(&actor["energy"]).is_some(),
                "actor coordinate/resources",
            )?;
            actor_state(actor, revision)?;
        }
        for g in array(&v["ground"][&depth], "ground floor")? {
            req(coordinate(g, w, h), "ground coordinate")?;
            item(&g["item"], true, revision)?;
        }
    }
    let depth = num(&v["depth"]).ok_or_else(|| fail("depth"))? as usize;
    let f = floors
        .iter().find(|f| num(&f["depth"]) == Some(depth as f64))
        .ok_or_else(|| fail("player floor"))?;
    let n = array(&f["tiles"], "tiles")?.len();
    let w = if n == 2291 {
        79
    } else {
        (n as f64).sqrt() as usize
    };
    req(
        coordinate(&v["player"], w, n / w.max(1)),
        "player coordinate",
    )?;
    actor_state(&v["player"], revision)?;
    for i in inventory {
        item(i, true, revision)?;
    }
    for k in ["detectedItems", "rememberedItems"] {
        if let Some(memory) = v.get(k) {
            for (d, entries) in object(memory, k)? {
                let dep = d.parse::<usize>().ok().filter(|n| (1..=40).contains(n));
                req(
                    dep.is_some_and(|n| n.to_string() == *d && floors.iter().any(|f| num(&f["depth"]) == Some(n as f64))),
                    k,
                )?;
                let f = floors.iter().find(|f| num(&f["depth"]) == Some(dep.unwrap() as f64)).unwrap();
                let n = array(&f["tiles"], "tiles")?.len();
                let w = if n == 2291 {
                    79
                } else {
                    (n as f64).sqrt() as usize
                };
                let entries = array(entries, k)?;
                req(entries.len() <= n, k)?;
                for e in entries {
                    req(coordinate(e, w, n / w), k)?;
                    item(&e["item"], false, revision)?;
                }
            }
        }
    }
    if let Some(natures) = v.get("knownPotionNatures") {
        req(
            object(natures, "potion natures")?
                .values()
                .all(|x| one(x, &["good", "bad"])),
            "potion nature",
        )?;
    }
    for raw in array(&v["history"], "history")? {
        let frame: Value =
            serde_json::from_str(raw.as_str().ok_or_else(|| fail("history string"))?)?;
        for group in array(&frame["groups"], "history groups")? {
            for patch in array(group, "history group")? {
                let path = array(&patch["path"], "history path")?;
                req(
                    !path.is_empty()
                        && path.iter().all(|k| {
                            (k.is_string() || k.is_number())
                                && !k.as_str().is_some_and(|s| {
                                    ["__proto__", "prototype", "constructor"].contains(&s)
                                })
                        }),
                    "history path key",
                )?;
                if let Some(n) = patch.get("length") {
                    let allowed = path.len() == 2
                        && path[0] == "logArchive"
                        && one(&path[1], &["events", "messages"])
                        && num(n).is_some_and(|n| {
                            n <= v["logArchive"][path[1].as_str().unwrap()]
                                .as_array()
                                .map_or(0, Vec::len) as f64
                        });
                    req(
                        count(n) && (num(n).unwrap() <= 100000.0 || allowed),
                        "history array length",
                    )?;
                }
            }
        }
    }
    Ok(())
}
fn actor_state(a: &Value, revision: f64) -> Result<()> {
    catalog_actor(a, revision)?;
    if let Some(h) = a.get("adaptiveHearts") {
        for (k, v) in object(h, "adaptive hearts")? {
            req(
                text(&Value::String(k.clone()), 200)
                    && integer(v)
                    && num(v).is_some_and(|n| (0.0..=5.0).contains(&n)),
                "adaptive heart entry",
            )?;
        }
    }
    req(
        optional(&a["stats"], "maxHp", |v| num(v).is_some()),
        "stats maxHp",
    )?;
    for k in ["equipmentAbilities", "abilityOrder"] {
        if let Some(v) = a.get(k) {
            let a = array(v, k)?;
            req(
                a.iter().all(Value::is_string)
                    && a.iter().enumerate().all(|(i, v)| !a[..i].contains(v)),
                k,
            )?;
        }
    }
    if let Some(m) = a.get("monsterState") {
        object(m, "monster state")?;
        if let Some(c) = m.get("consumedCorpseIds") {
            let c = array(c, "consumed corpses")?;
            req(
                c.len() <= 100000
                    && c.iter().all(count)
                    && c.iter().enumerate().all(|(i, v)| !c[..i].contains(v)),
                "consumed corpse id",
            )?;
        }
        if let Some(items) = m.get("carriedItems") {
            let a = array(items, "carried items")?;
            req(a.len() <= 100, "carried item capacity")?;
            for i in a {
                req(
                    safe(&i["id"])
                        && i["name"].is_string()
                        && one(
                            &i["kind"],
                            &[
                                "potion", "heart", "weapon", "armor", "ring", "amulet", "tool", "food",
                                "ammo",
                            ],
                        )
                        && i["enhancements"].is_array()
                        && optional(i, "magicNature", |v| one(v, &["good", "bad", "bizarre"])),
                    "carried item",
                )?;
            }
        }
        if let Some(p) = m.get("pilot") {
            req(
                p.is_object()
                    && p["monsterState"].get("pilot").is_none()
                    && a["creatureKind"] == "hemogoblin-mech"
                    && p["creatureKind"] == "hemogoblin-inventor"
                    && safe(&p["id"])
                    && integer(&p["x"])
                    && integer(&p["y"])
                    && ["hp", "maxHp", "energy"]
                        .iter()
                        .all(|k| num(&p[*k]).is_some()),
                "nested pilot",
            )?;
            actor_state(p, revision)?;
        }
    }
    Ok(())
}
fn item(i: &Value, quantity: bool, revision: f64) -> Result<()> {
    catalog_item(i, revision)?;
    req(optional(i,"amuletTier",|v|revision==52. && i["kind"]=="amulet" && integer(v)
        && num(v).is_some_and(|n|(1. ..=9.).contains(&n))
        && i["effect"].as_str().is_some_and(|e|["lightning","firebolt","acid","iceblast","floods","poison","roots","blinking","beckoning","discord","obstruction","teleportation"].contains(&e))),"R52 amulet tier")?;
    req(optional(i, "amuletState", |state| i["kind"] == "amulet"
        && state.is_object() && state.as_object().unwrap().keys().all(|k|["instance_id","energy","ready_at_half_tick"].contains(&k.as_str()))
        && count(&state["instance_id"]) && num(&state["instance_id"]) == num(&i["id"])
        && nonnegative(&state["energy"]) && num(&state["energy"]).is_some_and(|n|n<=50.0)
        && optional(state,"ready_at_half_tick",|v|v.as_u64().is_some())), "R50 amulet state")?;
    req(optional(i,"blessed",Value::is_boolean) && (i.get("blessed").is_none() || i["kind"]=="heart"),"Blessed heart flag")?;
    if quantity {
        if let Some(q) = i.get("quantity") {
            let stack = i["kind"] == "heart"
                && i["name"].as_str().is_some_and(|s| has_word(s, "Jackal"))
                || i["kind"] == "ammo" && i["effect"] == "arrows"
                || i["kind"] == "quest" && i["name"] == crate::phantasmii_generation::SHARD_NAME;
            req(
                count(q) && num(q).unwrap() >= 1.0 && (num(q) == Some(1.0) || stack),
                "item quantity",
            )?;
        }
    }
    req(
        optional(i, "rangedWeapon", |v| {
            i["kind"] == "weapon" && one(v, &["bow", "composite-bow"])
        }) && optional(i, "potionNature", |v| one(v, &["good", "bad"]))
            && optional(i, "magicNature", |v| one(v, &["good", "bad", "bizarre"]))
            && optional(i, "heartCreatureType", |v| {
                i["kind"] == "heart" && text(v, 200)
            })
            && optional(i, "offenseMultiplier", nonnegative),
        "item optional data",
    )?;
    req(optional(i, "sourceActorId", count) && optional(i, "borgeDepth", |v| count(v) && (1.0..=40.0).contains(&num(v).unwrap())), "Borge heart provenance")?;
    if i.get("sourceActorId").is_some() || i.get("borgeDepth").is_some() {
        req(i["kind"] == "heart" && i.get("sourceActorId").is_some() && i.get("borgeDepth").is_some() && i.get("quantity").is_none_or(|q| num(q) == Some(1.0)), "Borge unique heart")?;
    }
    if i["kind"] == "ammo" {
        req(
            i["effect"] == "arrows"
                && i.get("equipped").is_none_or(|v| v.as_str() == Some(""))
                && array(&i["enhancements"], "enhancements")?.is_empty(),
            "arrow equipment",
        )?;
    }
    if let Some(b) = i.get("baseBonus") {
        req(
            object(b, "base bonus")?.values().all(|v| num(v).is_some()),
            "base bonus value",
        )?;
    }
    req(
        optional(&i["bonus"], "maxHp", |v| num(v).is_some()),
        "bonus maxHp",
    )?;
    Ok(())
}
fn floor(f: &Value, n: usize, w: usize, h: usize) -> Result<()> {
    extra_floor(f, n, w, h)?;
    for k in [
        "currents",
        "rememberedCurrents",
        "rememberedIchor",
        "mud",
        "roots",
        "substrates",
        "rememberedTiles",
        "rememberedSubstrates",
        "slime",
        "razorleaves",
        "blood",
        "waterBlood",
        "bloodyWalls",
        "ice",
        "puddles",
        "hellfire",
        "rememberedIce",
        "rememberedPuddles",
        "rememberedHellfire",
        "snow",
        "rememberedSnow",
        "ichor",
    ] {
        if let Some(v) = f.get(k) {
            let a = array(v, k)?;
            req(a.len() == n, k)?;
            for x in a {
                req(
                    match k {
                        "substrates" | "rememberedSubstrates" => {
                            one(x, &["tiled", "rock", "earth", "sand", "soot", "ash"])
                        }
                        "rememberedTiles" => one(x, &crate::storage::TILES),
                        "currents" | "rememberedCurrents" => {
                            integer(x) && num(x).is_some_and(|n| (0.0..=8.0).contains(&n))
                        }
                        "rememberedIchor" => nonnegative(x) && num(x).unwrap() <= 255.0,
                        "waterBlood" => nonnegative(x) && num(x).unwrap() <= 100.0,
                        "blood" | "bloodyWalls" | "snow" | "rememberedSnow" | "ice" | "puddles" | "hellfire" | "rememberedIce" | "rememberedPuddles" | "rememberedHellfire" => x.is_boolean(),
                        _ => nonnegative(x),
                    },
                    k,
                )?;
            }
        }
    }
    req(optional(f, "monsterEntryTick", count), "monster entry tick")?;
    for k in ["grates", "permanentFire", "kelpieBloodSources", "bloodRiverSources"] {
        if let Some(v) = f.get(k) {
            cell_list(v, n, k)?;
        }
    }
    for k in ["doors", "rememberedDoors"] {
        if let Some(v) = f.get(k) {
            sparse(v, n, k, |d| {
                d["open"].is_boolean() && optional(d, "burning", nonnegative)
            })?;
        }
    }
    if let Some(v) = f.get("bulwarks") {
        sparse(v, n, "bulwarks", count)?;
    }
    if let Some(v) = f.get("lightSources") {
        let a = array(v, "lightSources")?;
        req(a.len() <= n, "light sources size")?;
        for s in a {
            req(
                coordinate(s, w, h)
                    && nonnegative(&s["power"])
                    && nonnegative(&s["falloff"])
                    && optional(s, "kind", |v| one(v, &["torch", "brazier", "beacon"])),
                "light source",
            )?;
        }
    }
    if let Some(v) = f.get("vents") {
        let a = array(v, "vents")?;
        req(a.len() <= n, "vent size")?;
        for s in a {
            req(
                cell(&s["at"], n)
                    && one(&s["gas"], &GAS)
                    && count(&s["countdown"])
                    && one(&s["phase"], &["waiting", "clearing"])
                    && optional(s, "amount", nonnegative),
                "vent",
            )?;
            if let Some(p) = s.get("pulse") {
                sparse(p, n, "vent pulse", nonnegative)?;
            }
        }
    }
    if let Some(v) = f.get("fungalRoots") {
        for r in array(v, "fungal roots")? {
            req(
                count(&r["fungusId"])
                    && count(&r["corpseId"])
                    && r["active"].is_boolean()
                    && r["color"].is_string(),
                "fungal root",
            )?;
            cell_list(&r["path"], n, "root path")?;
            if let Some(growth) = r.get("growth") {
                object(growth, "root growth")?;
                req(count(&growth["grown"]) && num(&growth["grown"]).is_some_and(|grown|
                    grown >= 1. && grown <= r["path"].as_array().unwrap().len() as f64)
                    && count(&growth["turns"]) && num(&growth["turns"]).is_some_and(|turns| turns < 3.),
                    "root growth")?;
            }
        }
    }
    if let Some(e) = f.get("environment") {
        object(e, "environment")?;
        if let Some(v)=e.get("bloodMistTurns") {
            let a=array(v,"blood mist duration")?;
            req(a.len()==n && a.iter().all(|v|integer(v) && num(v).is_some_and(|n|(0.0..=5.0).contains(&n))),"blood mist duration")?;
        }
        for key in GAS.into_iter().chain(["fire"]) {
            if let Some(v) = e.get(key) {
                let a = array(v, key)?;
                req(a.len() == n && a.iter().all(nonnegative), key)?;
            }
            if let Some(v) = e.get("provenance").and_then(|p| p.get(key)) {
                let owner = array(&v["owner"], "provenance owner")?;
                let player = array(&v["player"], "provenance player")?;
                req(
                    owner.len() == n
                        && owner.iter().all(|v| safe(v) && num(v).unwrap() >= -2.0)
                        && player.len() == n
                        && player.iter().all(nonnegative),
                    "provenance",
                )?;
            }
        }
        for group in [
            &["gasType", "gasVolume", "gasOwner", "gasPlayer"][..],
            &["fireKind", "fireFresh"][..],
        ] {
            if group.iter().any(|k| e.get(*k).is_some()) {
                for k in group {
                    let a = array(&e[*k], k)?;
                    req(a.len() == n, k)?;
                    for v in a {
                        req(
                            match *k {
                                "gasType" => v.is_null() || one(v, &GAS),
                                "gasVolume" => {
                                    integer(v)
                                        && num(v).is_some_and(|n| (0.0..=65535.0).contains(&n))
                                }
                                "gasOwner" => safe(v) && num(v).unwrap() >= -2.0,
                                "gasPlayer" => nonnegative(v),
                                "fireKind" => v.is_null() || one(v, &FIRE),
                                _ => v.is_boolean(),
                            },
                            k,
                        )?;
                    }
                }
            }
        }
        if e.get("fireReaction").is_some() || e.get("nextFireReaction").is_some() {
            let next = &e["nextFireReaction"];
            let a = array(&e["fireReaction"], "fire reactions")?;
            req(
                count(next)
                    && num(next).unwrap() >= 1.0
                    && a.len() == n
                    && a.iter()
                        .all(|v| count(v) && num(v).unwrap() < num(next).unwrap()),
                "fire reaction allocation",
            )?;
        }
    }
    Ok(())
}

fn has_word(s: &str, word: &str) -> bool {
    s.match_indices(word).any(|(i, _)| {
        let chars = s.as_bytes();
        let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        (i == 0 || !is_word(chars[i - 1]))
            && (i + word.len() == chars.len() || !is_word(chars[i + word.len()]))
    })
}
fn abilities() -> &'static Value {
    static V: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/abilities-v48.json"))
            .expect("frozen abilities")
    })
}
fn identities() -> &'static Value {
    static V: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/validation-identities-v48.json"))
            .expect("frozen identities")
    })
}
fn ability(v: &Value, revision: f64) -> bool {
    if revision >= 50.0 {
        return v.as_str().is_some_and(|id| crate::progression::definition_for_revision(revision, id).is_some());
    }
    v.as_str() == Some(crate::progression::WEAVE_ID) || (v.is_string()
        && abilities()
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == *v))
}
/// Generated-grant history is provenance, not currently equipped ownership.
/// Keep recognized pre-replacement IDs in draft/imported R50 history; future
/// selections still use the current catalog. Unknown IDs are never admitted.
fn generated_ability(v: &Value, revision: f64) -> bool {
    ability(v, revision) || revision >= 50.0 && ability(v, 46.0)
}
fn creature(v: &Value) -> bool {
    let Some(s) = v.as_str() else { return false };
    let s = s
        .strip_prefix("Mercuria’s ")
        .or_else(|| s.strip_prefix("Mercuria's "))
        .unwrap_or(s);
    if ["vent-fungus", "Vent Fungus", "night-crawler", "Night Crawler", "skin-walker", "Skin Walker", "saidribee-queen", "Saidribee Queen"].contains(&s) {
        return true;
    }
    identities()["creatures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some(s))
}
fn catalog_actor(a: &Value, revision: f64) -> Result<()> {
    req(
        optional(a, "creatureKind", |v| v == "zillennia" || v == crate::finale::BOSS_KIND || (v=="ahvamitt" && a["conditions"]["ahvamitt"]==1.0) || (v == "phantasmii-nightmare" && a["conditions"]["phantasmiiNightmare"] == 1.0) || creature(v)),
        "creature identity",
    )?;
    req(
        optional(a, "zillenniaQuality", |v| {
            a["creatureKind"] == "zillennia"
                && identities()["qualities"].as_array().unwrap().contains(v)
        }),
        "Zillennia quality",
    )?;
    if let Some(ids) = a.get("equipmentAbilities") {
        req(
            array(ids, "equipment abilities")?.iter().all(|id| ability(id, revision)),
            "equipment ability identity",
        )?;
    }
    Ok(())
}
fn catalog_item(i: &Value, revision: f64) -> Result<()> {
    req(
        optional(i, "grantedAbility", |v| {
            one(&i["kind"], &["weapon", "armor", "ring", "amulet"]) && ability(v, revision)
        }),
        "granted ability",
    )?;
    req(
        optional(i, "tinkered", |v| {
            v.is_boolean()
                && (v != &Value::Bool(true)
                    || one(&i["kind"], &["weapon", "armor", "ring", "amulet"])
                        && i.get("grantedAbility")
                            .is_some_and(|x| x.as_str().is_some_and(|s| !s.is_empty())))
        }),
        "tinkered equipment",
    )?;
    Ok(())
}
/// Boundary admission: O(A + P) expected time/space for actors and mounted pilots.
/// Preserve wire IDs/cursor; reject malformed/duplicate identities, not merely
/// low legacy allocation cursors. Actual allocation collisions fail atomically.
fn validate_pilot_ids(v: &Value) -> Result<()> {
    req(v["player"].get("monsterState").and_then(|m| m.get("pilot")).is_none(), "player cannot contain a mounted pilot")?;
    req(count(&v["nextId"]), "nextId must be a nonnegative safe integer")?;
    let mut ids = std::collections::HashSet::new();
    validate_actor_identity(&v["player"], &mut ids)?;
    for list in object(&v["actors"], "actors")?.values() {
        for actor in array(list, "actors list")? {validate_actor_identity(actor,&mut ids)?;}
    }
    Ok(())
}
/// O(1 + P) expected time/space for one actor and its mounted pilot.
fn validate_actor_identity(a: &Value, ids: &mut std::collections::HashSet<u64>) -> Result<()> {
    req(count(&a["id"]), "creature id must be a nonnegative safe integer")?;
    req(ids.insert(num(&a["id"]).unwrap() as u64), "duplicate creature id")?;
    if let Some(pilot) = a.get("monsterState").and_then(|m|m.get("pilot")) {
        req(pilot.get("monsterState").and_then(|m|m.get("pilot")).is_none(), "nested pilot depth")?;
        validate_actor_identity(pilot,ids)?;
    }
    Ok(())
}

fn color(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
    })
}
fn simple_text(v: &Value, n: usize) -> bool {
    v.as_str()
        .is_some_and(|s| !s.is_empty() && s.encode_utf16().count() <= n)
}
fn valid_hazard(v: &Value) -> bool {
    let Some(o) = v.as_object() else { return false };
    if o.keys().any(|k| {
        ![
            "gas",
            "fire",
            "permanentFire",
            "slime",
            "mud",
            "ichor",
            "razorleaves",
            "roots",
            "root",
            "blood",
            "waterBlood",
            "bloodyWall",
            "current",
            "vent",
            "bulwark",
            "lightSource",
            "feature",
        ]
        .contains(&k.as_str())
    }) {
        return false;
    }
    for k in [
        "permanentFire",
        "slime",
        "mud",
        "ichor",
        "razorleaves",
        "roots",
        "blood",
        "bloodyWall",
    ] {
        if !optional(v, k, Value::is_boolean) {
            return false;
        }
    }
    optional(v, "gas", |g| {
        g.is_object() && one(&g["type"], &GAS) && nonnegative(&g["concentration"])
    }) && optional(v, "fire", |v| one(v, &FIRE))
        && optional(v, "waterBlood", |v| nonnegative(v) && num(v).unwrap() <= 100.0)
        && optional(v, "current", |v| {
            integer(v) && num(v).is_some_and(|n| (1.0..=8.0).contains(&n))
        })
        && optional(v, "bulwark", count)
        && optional(v, "lightSource", |v| {
            one(v, &["torch", "brazier", "beacon"])
        })
        && optional(v, "root", |r| {
            r.is_object()
                && one(&r["glyph"], &["│", "─", "╲", "╱", "┼"])
                && color(&r["color"])
                && r["active"].is_boolean()
        })
        && optional(v, "vent", |r| {
            r.is_object()
                && one(&r["gas"], &GAS)
                && one(&r["phase"], &["waiting", "clearing"])
                && count(&r["countdown"])
        })
        && optional(v, "feature", |r| {
            r.is_object()
                && simple_text(&r["glyph"], 8)
                && color(&r["color"])
                && simple_text(&r["name"], 256)
                && simple_text(&r["description"], 4096)
        })
}
fn bounds(b: &Value, w: usize, h: usize) -> bool {
    ["left", "right", "top", "bottom"]
        .iter()
        .all(|k| integer(&b[*k]))
        && num(&b["left"]).is_some_and(|l| l >= 0.0 && l <= num(&b["right"]).unwrap())
        && num(&b["top"]).is_some_and(|t| t >= 0.0 && t <= num(&b["bottom"]).unwrap())
        && num(&b["right"]).is_some_and(|r| r < w as f64)
        && num(&b["bottom"]).is_some_and(|r| r < h as f64)
}
fn extra_floor(f: &Value, n: usize, w: usize, h: usize) -> Result<()> {
    if let Some(v) = f.get("rememberedHazards") {
        sparse(v, n, "remembered hazard", valid_hazard)?;
    }
    if let Some(room) = f.get("sphinxRoom") {
        req(coordinate(room, w, h), "sphinx coordinate")?;
        cell_list(&room["cells"], n, "sphinx cells")?;
        cell_list(&room["rewardCells"], n, "sphinx rewards")?;
        let a = array(&room["rewardCells"], "sphinx rewards")?;
        let cells = array(&room["cells"], "sphinx cells")?;
        req(
            a.len() == 4 && a.iter().all(|i| cells.contains(i)),
            "sphinx reward coverage",
        )?;
    }
    if let Some(families) = f.get("enemyFamilies") {
        let a = array(families, "enemy families")?;
        req(
            a.len() <= 13
                && a.iter().all(|v| {
                    one(
                        v,
                        &[
                            "pale",
                            "slime",
                            "wildlife",
                            "saidri",
                            "beetle",
                            "horn",
                            "razorwood",
                            "river",
                            "moon",
                            "obelisk",
                            "hemogoblin",
                            "undead",
                            "mythic",
                        ],
                    )
                })
                && a.iter().enumerate().all(|(i, v)| !a[..i].contains(v)),
            "enemy families",
        )?;
    }
    if let Some(setups) = f.get("setups") {
        let a = array(setups, "setups")?;
        req(a.len() <= n, "setup count")?;
        let mut owned = std::collections::HashSet::new();
        for s in a {
            req(
                count(&s["id"]) && coordinate(s, w, h),
                "setup identity/coordinate",
            )?;
            if let Some(b) = s.get("bounds") {
                req(bounds(b, w, h), "setup bounds")?;
            }
            if let Some(fp) = s.get("footprint") {
                let cells = array(fp, "setup footprint")?;
                req(
                    !cells.is_empty()
                        && cells.len() <= 169
                        && cells.iter().enumerate().all(|(i, v)| {
                            cell(v, 169)
                                && (i == 0 || num(&cells[i - 1]).unwrap() < num(v).unwrap())
                        }),
                    "setup stencil order",
                )?;
                for cell in cells {
                    let i = num(cell).unwrap() as usize;
                    let x = num(&s["x"]).unwrap() - 6.0 + (i % 13) as f64;
                    let y = num(&s["y"]).unwrap() - 6.0 + (i / 13) as f64;
                    req(
                        x >= 0.0 && x < w as f64 && y >= 0.0 && y < h as f64,
                        "setup stencil bounds",
                    )?;
                    if let Some(b) = s.get("bounds") {
                        req(
                            x >= num(&b["left"]).unwrap()
                                && x <= num(&b["right"]).unwrap()
                                && y >= num(&b["top"]).unwrap()
                                && y <= num(&b["bottom"]).unwrap(),
                            "setup footprint bounds",
                        )?;
                    }
                    req(
                        owned.insert(y as usize * w + x as usize),
                        "overlapping setup footprints",
                    )?;
                }
            }
        }
    }
    if let Some(e) = f.get("godbeastEvent") {
        req(
            one(&e["kind"], &["Mercuria", "Zillennia", "Veridian", "Borge", "Phantasmii", "Epidite", "Ahvamitt"])
                && coordinate(e, w, h)
                && bounds(&e["room"], w, h),
            "Godbeast event",
        )?;
        for k in ["well", "lever", "tap", "altar"] {
            req(
                optional(e, k, |p| coordinate(p, w, h)),
                "Godbeast coordinate",
            )?;
        }
        for k in ["activated", "rewardCreated", "tapped"] {
            req(optional(e, k, Value::is_boolean), "Godbeast flag")?;
        }
        req(optional(e,"epiditeAge",count),"Epidite clock")?;
        req(optional(e, "displayName", |v| text(v, 200)), "Godbeast display name")?;
        req(optional(e, "sacrificeActorId", count) && optional(e, "sacrificeName", |v| text(v, 200)), "Borge sacrifice")?;
        if e["kind"] == "Borge" {
            req(e.get("altar").is_some()
                && (e.get("sacrificeActorId").is_some() == e.get("sacrificeName").is_some())
                && (e.get("sacrificeActorId").is_none() || e["activated"] == true)
                && (e["rewardCreated"] != true || (e["activated"] == true && e.get("sacrificeActorId").is_some())), "Borge event state")?;
        }
        for k in ["floodAge", "nextFishAge"] {
            req(optional(e, k, count), "Godbeast age")?;
        }
        req(
            optional(e, "fishSpawned", |v| count(v) && num(v).unwrap() <= 10.0),
            "fish spawned",
        )?;
        if let Some(v) = e.get("floodFrontier") {
            cell_list(v, n, "flood frontier")?;
        }
        if let Some(v) = e.get("floodVisited") {
            let a = array(v, "flood visited")?;
            req(
                a.len() == n && a.iter().all(|v| matches!(num(v), Some(0.0 | 1.0 | 2.0))),
                "flood visited",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod creature_identity_admission_tests {
    use super::*;
    use serde_json::json;
    /// O(1) test fixture construction.
    fn run() -> Value { json!({"player":{"id":0},"actors":{"1":[{"id":4}]},"nextId":5}) }
    /// O(1) bounded admission cases; production cost is covered at its helper.
    #[test]
    fn unique_safe_ids_and_counter_are_required() {
        assert!(validate_pilot_ids(&run()).is_ok());
        for id in [0.0, -1.0, 0.5, 9007199254740992.0] {
            let mut v=run();v["actors"]["1"][0]["id"]=json!(id);
            assert!(validate_pilot_ids(&v).is_err(), "{id}");
        }
        for next in [-1.0, 4.5, 9007199254740992.0] {
            let mut v=run();v["nextId"]=json!(next);
            assert!(validate_pilot_ids(&v).is_err(), "counter {next}");
        }
        let mut v=run(); v["actors"]["2"]=json!([{"id":4}]);
        assert!(validate_pilot_ids(&v).is_err());
        let mut v=run();v["nextId"]=json!(9007199254740991u64);
        v["actors"]["1"][0]["id"]=json!(9007199254740990u64);
        assert!(validate_pilot_ids(&v).is_ok());
        v["actors"]["1"][0]["id"]=json!(9007199254740991u64);
        assert!(validate_pilot_ids(&v).is_ok());
        v["nextId"]=json!(1);assert!(validate_pilot_ids(&v).is_ok());
    }
    /// O(1) bounded nested identity and shared allocator regression cases.
    #[test]
    fn pilot_ids_are_reserved_and_legacy_cursor_preserved() {
        let mut v=run(); v["actors"]["1"][0]["monsterState"]=json!({"pilot":{"id":3}});
        assert!(validate_pilot_ids(&v).is_ok());
        v["actors"]["1"][0]["monsterState"]["pilot"]["id"]=json!(4);
        assert!(validate_pilot_ids(&v).is_err());
        v["actors"]["1"][0]["monsterState"]["pilot"]["id"]=json!(0);
        assert!(validate_pilot_ids(&v).is_err());
        let mut v=run();v["inventory"]=json!([{"id":5}]);
        assert!(validate_pilot_ids(&v).is_ok());
        v["nextId"]=json!(6);
        assert!(validate_pilot_ids(&v).is_ok());
        v["ground"]=json!({"1":[{"item":{"id":6}}]});
        assert!(validate_pilot_ids(&v).is_ok());
    }
    /// O(S) time/space for serialized fixture size S; unsupported player nesting
    /// is rejected at both save admission and dispatch before gameplay mutation.
    #[test]
    fn player_cannot_contain_a_mounted_pilot() {
        let mut value=run();
        value["player"]["monsterState"]=json!({"pilot":{"id":3}});
        assert!(validate_pilot_ids(&value).is_err());
        value["player"]["monsterState"]["pilot"]=Value::Null;
        assert!(validate_pilot_ids(&value).is_err());
        let mut state=crate::Run::default();
        state.player.monster_state=crate::Field::Value(crate::ActorMonsterState {
            pilot:Box::new(crate::Actor{id:3.,..Default::default()}).into(),
            ..Default::default()
        });
        let before=serde_json::to_value(&state).unwrap();
        let error=crate::engine::dispatch(&mut state,&crate::Command::Wait).unwrap_err();
        assert!(error.0.contains("player cannot contain a mounted pilot"));
        assert_eq!(serde_json::to_value(&state).unwrap(),before);
    }

}

/// O(S + G*A*L + G*K*L) worst-case boundary traversal, O(depth) stack.
/// S=Value nodes, G=grant/reserve IDs, A=72 catalog rows, L=ID bytes, K=max
/// additional grants on one item (prefix duplicate checks). No serialization.
/// Covers nested carried items/pilots/planes without treating arbitrary text as IDs.
fn revision_ability_fields(v: &Value, revision: f64) -> bool {
    /// O(ID bytes), exact identifier tests only.
    fn admitted(id: &str, revision: f64) -> bool {
        (id != crate::progression::WEAVE_ID || revision >= 41.0)
            && (id != "numo-revert" || revision < 41.0)
    }
    match v {
        Value::Object(map) => map.iter().all(|(key,value)| {
            // Canonical actor/item fields can also occur in initial-player,
            // copied-inventory, remembered-item, pilot and debug-stash payloads.
            // Visit them in the existing boundary traversal, never during turns.
            if key == "generatedAbilityGrants" {
                if !value.as_array().is_some_and(|ids| ids.iter().all(|id| generated_ability(id, revision))) { return false; }
            }
            if key == "equipmentAbilities" {
                if !value.as_array().is_some_and(|ids| ids.iter().all(|id| ability(id, revision))) { return false; }
            }
            if key == "grantedAbility" && !ability(value, revision) { return false; }
            if key == "additionalGrantedAbilities" {
                if revision < 50.0 || !one(&v["kind"], &["ring", "amulet"])
                    || !value.as_array().is_some_and(|ids| ids.iter().enumerate().all(|(index,id)|
                        ability(id, revision) && id != &v["grantedAbility"] && !ids[..index].contains(id))) {
                    return false;
                }
            }
            if key == "abilityReserves" {
                if revision < 50.0 || !value.as_object().is_some_and(|reserves| reserves.iter().all(|(id,state)|
                    (crate::progression::definition_for_revision(revision,id).is_some()
                        || crate::abilities::copied_trait_supported(id))
                        && state.as_object().is_some_and(|s| s.len()==2 && s.contains_key("capacity") && s.contains_key("energy"))
                        && nonnegative(&state["capacity"]) && nonnegative(&state["energy"])
                        && num(&state["energy"]) <= num(&state["capacity"]))) {
                    return false;
                }
            }
            if matches!(key.as_str(), "abilities"|"active"|"equipmentAbilities"|"abilityOrder"|"generatedAbilityGrants"|"offers") {
                if value.as_array().is_some_and(|a| a.iter().any(|x|x.as_str().is_some_and(|id|!admitted(id,revision)))) { return false; }
            }
            if key == "grantedAbility" && value.as_str().is_some_and(|id|!admitted(id,revision)) { return false; }
            if key == "timeweave" && revision < 41.0 { return false; }
            if key == "reapingLedger" && (revision < 50.0 || !value.as_object().is_some_and(|ledger|
                ledger.len()==1 && nonnegative(&value["damageCarry"]) && num(&value["damageCarry"]).is_some_and(|n| n < 20.0))) { return false; }
            if (key == "amuletState" || key == "amuletLedger") && revision < 50.0 { return false; }
            if key == "amuletTier" && !(revision==52. && v["kind"]=="amulet" && integer(value)
                && num(value).is_some_and(|n|(1. ..=9.).contains(&n))) {return false;}
            if key == "conditions" {
                if value.as_object().is_some_and(|conditions| conditions.iter().any(|(k,v)|
                    crate::balance52::condition_key(k) && (revision != 52.
                        || !v.as_f64().is_some_and(|n|crate::balance52::valid_condition(k,n))))) {return false;}
                if value.as_object().is_some_and(|conditions|conditions.iter().any(|(k,v)|
                    !v.as_f64().is_some_and(|n|crate::revision52_amulet_tiers::valid_condition(k,n,revision)))) {return false;}
                if revision==52. && value.as_object().is_some_and(|conditions|conditions.iter().any(|(k,v)|
                    crate::revision52_enemy_followups::condition_key(k)
                        && !v.as_f64().is_some_and(|n|crate::revision52_enemy_followups::valid_condition(k,n)))) {return false;}
                if value.as_object().is_some_and(|conditions| conditions.keys().any(|k| {
                    ["acquired:","unequipped:","remembered:"].iter().any(|prefix| k.strip_prefix(prefix).is_some_and(|id|!admitted(id,revision)))
                })) { return false; }
            }
            revision_ability_fields(value,revision)
        }),
        Value::Array(values) => values.iter().all(|x|revision_ability_fields(x,revision)),
        _ => true,
    }
}

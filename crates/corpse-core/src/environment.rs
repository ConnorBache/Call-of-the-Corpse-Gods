//! v48 environment state migration and exact gas propagation adapter.
use crate::propagation::{self, Dissipation, GasGrid, GridSize, RandomInt};
use crate::{EnvironmentalState, EnvironmentalStateProvenanceEntry, Field, Floor, Run};
use indexmap::IndexMap;
pub const GAS_TYPES: [&str; 10] = [
    "caustic",
    "breather",
    "paralysis",
    "confusion",
    "flammable",
    "explosive",
    "hydrogen",
    "steam",
    "hallucinogenic",
    "bloodMist",
];
fn channel<'a>(e: &'a EnvironmentalState, gas: &str) -> &'a [f64] {
    match gas {
        "bloodMist" => e.blood_mist.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        "caustic" => &e.caustic,
        "breather" => &e.breather,
        "paralysis" => &e.paralysis,
        "confusion" => e.confusion.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        "hallucinogenic" => e.hallucinogenic.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        "flammable" => e.flammable.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        "explosive" => e.explosive.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        "hydrogen" => e.hydrogen.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        "steam" => e.steam.as_ref().map(Vec::as_slice).unwrap_or(&[]),
        _ => &[],
    }
}
pub fn breather_gas(r: &Run, a: &crate::Actor) -> &'static str {
    let name = if r.simulation_revision.value_or(29.)>=50. {crate::enemy_traits::base_name(&a.name)} else {&a.name};
    let name = crate::revision50_creature_identity::legacy_name(name);
    let greater = a.creature_kind.as_ref().is_some_and(|k| k == "pale:Greater Breather")
        || (name.starts_with("Greater ") && name.ends_with("Breather"));
    if crate::water_blood::enabled(r) && greater { "hallucinogenic" } else { "caustic" }
}
pub fn fire_active(kind: Option<&str>) -> bool {
    kind.is_some_and(|k| k != "embers")
}
/// The numeric view normally agrees with the kind, but can lag the source
/// layer on arrival/import. A recorded historical prefix retains its old view.
fn burns_on_contact(r: &Run, e: &EnvironmentalState, i: usize) -> bool {
    e.fire.get(i).copied().unwrap_or(0.) > 0.
        && !(crate::ember_rules::current(r)
            && e.fire_kind.as_ref().and_then(|v| v.get(i)).and_then(Option::as_deref) == Some("embers"))
}
pub fn normalize(e: &mut EnvironmentalState) -> Result<(), String> {
    // Pilot cannot express whole-column hydration as an indexed cell write.
    if e.gas_type.as_ref().is_none() || e.gas_volume.as_ref().is_none()
        || e.gas_owner.as_ref().is_none() || e.gas_player.as_ref().is_none() {
        crate::capture_pilot::mark_unsupported("gas_primary_column_initialization");
    }
    let n = e.fire.len();
    if e.gas_type.as_ref().is_none() || e.gas_volume.as_ref().is_none() {
        let mut types = vec![None; n];
        let mut volume = vec![0.; n];
        let mut owner = vec![-1.; n];
        let mut player = vec![0.; n];
        for i in 0..n {
            let mut max = 0.;
            let mut total = 0.;
            let mut p = 0.;
            for gas in GAS_TYPES {
                let v = channel(e, gas).get(i).copied().unwrap_or(0.).max(0.);
                total += v;
                let prov = e.provenance.as_ref().and_then(|m| m.get(gas));
                p += prov.and_then(|x| x.player.get(i)).copied().unwrap_or(0.);
                if v > max {
                    max = v;
                    types[i] = Some(gas.into());
                    owner[i] = prov.and_then(|x| x.owner.get(i)).copied().unwrap_or(-1.);
                }
            }
            volume[i] = (crate::rng::to_u32(total.floor()) & 65535) as f64;
            player[i] = p.min(volume[i]);
        }
        e.gas_type = types.into();
        e.gas_volume = volume.into();
        e.gas_owner = owner.into();
        e.gas_player = player.into();
    }
    e.gas_owner.get_or_insert_with(|| vec![-1.; n]);
    e.gas_player.get_or_insert_with(|| vec![0.; n]);
    e.fire_kind.get_or_insert_with(|| {
        e.fire
            .iter()
            .map(|&v| {
                if v > 0. {
                    Some("creature".into())
                } else {
                    None
                }
            })
            .collect()
    });
    e.fire_fresh.get_or_insert_with(|| vec![false; n]);
    if e.fire_reaction.is_missing() && e.next_fire_reaction.is_missing() {
        e.fire_reaction = vec![0.; n].into();
        e.next_fire_reaction = 1f64.into();
    } else if e.fire_reaction.is_missing() || e.next_fire_reaction.is_missing() {
        return Err("Incomplete fire reaction state.".into());
    }
    Ok(())
}
pub fn refresh_gas_views(e: &mut EnvironmentalState) {
    let _timing_function = crate::timing::span("environment.gas_views");
    let n = e.fire.len();
    for gas in GAS_TYPES {
        // Optional native channel must not alter legacy save/trace bytes.
        if ((gas == "hallucinogenic" && e.hallucinogenic.is_missing()) || (gas == "bloodMist" && e.blood_mist.is_missing()))
            && !e.gas_type.as_ref().unwrap().iter().any(|g| g.as_deref() == Some(gas))
        { continue; }
        let values = (0..n)
            .map(|i| {
                if e.gas_type.as_ref().unwrap()[i].as_deref() == Some(gas) {
                    e.gas_volume.as_ref().unwrap()[i]
                } else {
                    0.
                }
            })
            .collect();
        match gas {
            "bloodMist" => e.blood_mist = Field::Value(values),
            "caustic" => e.caustic = values,
            "breather" => e.breather = values,
            "paralysis" => e.paralysis = values,
            "confusion" => e.confusion = Field::Value(values),
            "hallucinogenic" => e.hallucinogenic = Field::Value(values),
            "flammable" => e.flammable = Field::Value(values),
            "explosive" => e.explosive = Field::Value(values),
            "hydrogen" => e.hydrogen = Field::Value(values),
            "steam" => e.steam = Field::Value(values),
            _ => unreachable!(),
        }
    }
}
pub fn refresh_fire_views(e: &mut EnvironmentalState) {
    let _timing_function = crate::timing::span("environment.fire_views");
    for i in 0..e.fire.len() {
        e.fire[i] = if fire_active(e.fire_kind.as_ref().unwrap()[i].as_deref()) {
            1.
        } else {
            0.
        }
    }
}
pub fn blocks_sight(f: &Floor, i: usize) -> bool {
    f.doors
        .as_ref()
        .and_then(|d| d.get(&i.to_string()))
        .is_some_and(|d| !d.open)
}
pub fn blocks_movement(f: &Floor, i: usize) -> bool {
    blocks_sight(f, i) || f.grates.as_ref().is_some_and(|g| g.contains(&(i as f64)))
}
pub fn gas_passable(f: &Floor, i: usize) -> bool {
    f.tiles[i] != "wall" && !blocks_sight(f, i)
}
pub fn open_door(f: &mut Floor, i: usize) -> bool {
    if let Some(d) = f.doors.as_mut().and_then(|m| m.get_mut(&i.to_string())) {
        if !d.open {
            d.open = true;
            f.dirty = true;
            return true;
        }
    }
    false
}
pub fn settle_doors(f: &mut Floor, occupied: impl IntoIterator<Item = usize>) {
    let occupied: std::collections::HashSet<_> = occupied.into_iter().collect();
    if let Some(doors) = f.doors.as_mut() {
        for (i, d) in doors {
            if d.open && !occupied.contains(&i.parse().unwrap_or(usize::MAX)) {
                d.open = false;
                f.dirty = true;
            }
        }
    }
}
/// O(existing passability), no search. Revision admission does not mutate old floors.
pub fn current_destination_revision(f:&Floor,i:usize,revision:f64)->Option<usize> {
    if revision>=40.0 {crate::shore_currents::destination(f,i)} else {current_destination(f,i)}
}
pub fn current_destination(f: &Floor, i: usize) -> Option<usize> {
    let code = f.currents.as_ref()?.get(i).copied()?;
    if code.fract() != 0. || !(1.0..=8.).contains(&code) {
        return None;
    }
    let wet = |j: usize| {
        f.tiles[j] == "water"
            && f.ichor
                .as_ref()
                .and_then(|v| v.get(j))
                .copied()
                .unwrap_or(0.)
                == 0.
            && !blocks_movement(f, j)
    };
    if !wet(i) {
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
    let j = GridSize {
        width: f.width(),
        height: f.height(),
    }
    .adjacent(i, dx, dy)?;
    wet(j).then_some(j)
}
pub fn initialize(f: &mut Floor) -> Result<&mut EnvironmentalState, String> {
    let _timing_function = crate::timing::span("environment.initialize");
    let n = f.tiles.len();
    let e = f.environment.get_or_insert_with(|| EnvironmentalState {
        fire: vec![0.; n],
        caustic: vec![0.; n],
        breather: vec![0.; n],
        paralysis: vec![0.; n],
        ..Default::default()
    });
    for field in [
        &mut e.confusion,
        &mut e.flammable,
        &mut e.explosive,
        &mut e.hydrogen,
        &mut e.steam,
    ] {
        field.get_or_insert_with(|| vec![0.; n]);
    }
    normalize(e)?;
    // Hellfire is a persistent terrain source, not a spreadable fire kind.
    // Reuse brazier's non-expiring promotion behavior; neighbors ignite normally.
    if let Some(hellfire)=f.hellfire.as_ref() {
        for (i,&active) in hellfire.iter().enumerate() {
            if active { e.fire[i]=1.; e.fire_kind.as_mut().unwrap()[i]=Some("brazier".into()); }
        }
    }
    let hallucinations = e.hallucinogenic.as_ref().is_some()
        || e.gas_type.as_ref().unwrap().iter().any(|g| g.as_deref() == Some("hallucinogenic"));
    if hallucinations { e.hallucinogenic.get_or_insert_with(|| vec![0.; n]); }
    let p = e.provenance.get_or_insert_with(IndexMap::new);
    for gas in [
        "caustic",
        "breather",
        "paralysis",
        "fire",
        "confusion",
        "flammable",
        "explosive",
        "hydrogen",
        "steam",
    ] {
        p.entry(gas.into())
            .or_insert_with(|| EnvironmentalStateProvenanceEntry {
                owner: vec![-1.; n],
                player: vec![0.; n],
            });
    }
    if hallucinations {
        p.entry("hallucinogenic".into()).or_insert_with(|| EnvironmentalStateProvenanceEntry {
            owner: vec![-1.; n], player: vec![0.; n],
        });
    }
    if e.blood_mist.as_ref().is_some() || e.gas_type.as_ref().unwrap().iter().any(|g|g.as_deref()==Some("bloodMist")) {
        e.blood_mist.get_or_insert_with(||vec![0.;n]);
        e.blood_mist_turns.get_or_insert_with(||vec![5.;n]);
        e.provenance.as_mut().unwrap().entry("bloodMist".into()).or_insert_with(||EnvironmentalStateProvenanceEntry{owner:vec![-1.;n],player:vec![0.;n]});
    }
    Ok(e)
}
struct GasPass<'a> {
    // Logical physical floor identity, not an address of this possibly detached proxy.
    depth_bits: u64,
    size: GridSize,
    e: &'a mut EnvironmentalState,
    blocked: &'a [bool],
    descent: &'a [bool],
    frozen: &'a [bool],
    rng: &'a mut u32,
    old_mist_turns: Option<Vec<f64>>,
    live_mist_turns: Option<Vec<f64>>,
    next_mist_turns: Option<Vec<f64>>,
    old_volume: Vec<f64>,
    old_player: Vec<f64>,
    live_owner: Vec<f64>,
    next_owner: Vec<f64>,
    next_player: Vec<f64>,
    tracked: Vec<(usize, IndexMap<String, f64>, IndexMap<String, f64>)>,
}
impl RandomInt for GasPass<'_> {
    fn random_int(&mut self, min: u32, max: u32) -> u32 {
        if min == max {
            min
        } else {
            min + ((crate::rng::next_u32(self.rng) as f64 / 4294967296.) * ((max - min + 1) as f64))
                .floor() as u32
        }
    }
}
impl GasGrid for GasPass<'_> {
    type Gas = String;
    fn size(&self) -> GridSize {
        self.size
    }
    fn volume(&self, i: usize) -> u16 {
        self.e.gas_volume.as_ref().unwrap()[i] as u16
    }
    fn set_volume(&mut self, i: usize, v: u16) {
        crate::capture_pilot::mark_gas_write(self.depth_bits, i,
            self.e.gas_volume.as_ref().unwrap()[i].to_bits() != (v as f64).to_bits());
        self.e.gas_volume.as_mut().unwrap()[i] = v as f64
    }
    fn gas(&self, i: usize) -> Option<String> {
        self.e.gas_type.as_ref().unwrap()[i].clone()
    }
    fn set_gas(&mut self, i: usize, g: Option<String>) {
        crate::capture_pilot::mark_gas_write(self.depth_bits, i,
            self.e.gas_type.as_ref().unwrap()[i] != g);
        self.e.gas_type.as_mut().unwrap()[i] = g
    }
    fn blocks_gas(&self, i: usize) -> bool {
        self.blocked[i]
    }
    fn frozen(&self, i: usize) -> bool {
        self.frozen.get(i).copied().unwrap_or(false)
    }
    fn auto_descent(&self, i: usize) -> bool {
        self.descent[i]
    }
    fn dissipation(&self, g: Option<String>) -> Dissipation {
        match g.as_deref() {
            None | Some("explosive" | "hydrogen") => Dissipation::None,
            Some("bloodMist") => Dissipation::None,
            Some("paralysis" | "confusion" | "hallucinogenic" | "steam") => Dissipation::Quick,
            _ => Dissipation::Normal,
        }
    }
    fn on_type_change(&mut self, i: usize, _: Option<String>, _: Option<String>, source: usize) {
        self.live_owner[i] = self.live_owner[source];
        if let Some(v)=&mut self.live_mist_turns {v[i]=v[source];}
    }
    fn on_cell_resolved(&mut self, i: usize, source: usize, contributors: &[usize], output: u16) {
        if let (Some(old),Some(next))=(&self.old_mist_turns,&mut self.next_mist_turns) {
            next[i]=if self.e.gas_type.as_ref().unwrap()[i].as_deref()==Some("bloodMist") && output>0 {
                contributors.iter().map(|&j|old[j]).fold(self.live_mist_turns.as_ref().map_or(0.,|v|v[source]),f64::max)
            } else {0.};
        }
        self.next_owner[i] = self.live_owner[source];
        let total: f64 = contributors.iter().map(|&j| self.old_volume[j]).sum();
        let player: f64 = contributors.iter().map(|&j| self.old_player[j]).sum();
        self.next_player[i] = if total > 0. {
            output as f64 * (player / total).min(1.)
        } else {
            0.
        };
        if self.gas(i).is_some() && output > 0 {
            for (_, prior, next) in &mut self.tracked {
                if contributors.iter().any(|&j| {
                    prior.get(&j.to_string()).copied().unwrap_or(0.) != 0.
                        && self.old_volume[j] > 0.
                }) {
                    next.insert(i.to_string(), 1.);
                }
            }
        }
    }
}
/// Exact two-pass adapter, including live ownership and vent pulse observation.
/// O(V + V*T) time where T is tracked vents; O(V + pulse entries) extra memory.
pub fn propagate_gas(f: &mut Floor, rng: &mut u32, frozen: &[bool]) -> Result<(), String> {
    let depth_bits = f.depth.to_bits();
    let _timing_function = crate::timing::span("environment.gas_diffusion");
    let size = GridSize {
        width: f.width(),
        height: f.height(),
    };
    let blocked: Vec<_> = (0..f.tiles.len()).map(|i| !gas_passable(f, i)).collect();
    let descent: Vec<_> = f.tiles.iter().map(|t| t == "chasm" || t == "pit").collect();
    initialize(f)?;
    if !f
        .environment
        .as_ref()
        .unwrap()
        .gas_type
        .as_ref()
        .unwrap()
        .iter()
        .any(Option::is_some)
    {
        refresh_gas_views(f.environment.as_mut().unwrap());
        return Ok(());
    }
    for _ in 0..2 {
        let tracked = f
            .vents
            .as_ref()
            .map(|v| {
                v.iter()
                    .enumerate()
                    .filter_map(|(i, v)| {
                        v.pulse
                            .as_ref()
                            .filter(|p| !p.is_empty())
                            .map(|p| (i, p.clone(), IndexMap::new()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let e = f.environment.as_mut().unwrap();
        let n = e.fire.len();
        let mut pass = GasPass {
            depth_bits,
            size,
            blocked: &blocked,
            descent: &descent,
            frozen,
            rng,
            old_mist_turns: e.blood_mist_turns.as_ref().cloned(),
            live_mist_turns: e.blood_mist_turns.as_ref().cloned(),
            next_mist_turns: e.blood_mist_turns.as_ref().map(|_|vec![0.;n]),
            old_volume: e.gas_volume.as_ref().unwrap().clone(),
            old_player: e.gas_player.as_ref().unwrap().clone(),
            live_owner: e.gas_owner.as_ref().unwrap().clone(),
            next_owner: vec![-1.; n],
            next_player: vec![0.; n],
            tracked,
            e,
        };
        propagation::diffuse_gas_pass(&mut pass);
        // Pilot producer-side bulk commit check: O(N) only while collecting.
        // This compares these two already-produced columns, never an endpoint Floor.
        if crate::capture_pilot::enabled() {
            for i in 0..n {
                crate::capture_pilot::mark_gas_write(depth_bits, i,
                    pass.e.gas_owner.as_ref().unwrap()[i].to_bits() != pass.next_owner[i].to_bits());
                crate::capture_pilot::mark_gas_write(depth_bits, i,
                    pass.e.gas_player.as_ref().unwrap()[i].to_bits() != pass.next_player[i].to_bits());
            }
        }
        if let Some(next)=pass.next_mist_turns { pass.e.blood_mist_turns=next.into(); }
        pass.e.gas_owner = pass.next_owner.into();
        pass.e.gas_player = pass.next_player.into();
        let tracked = pass.tracked;
        if let Some(vents) = f.vents.as_mut() {
            for (i, _, next) in tracked {
                vents[i].pulse = next.into()
            }
        }
    }
    refresh_gas_views(f.environment.as_mut().unwrap());
    Ok(())
}
/// Fail closed before a normal turn. Wider executable kernels are not a claim
/// that all their combat, emitter, visibility, or offscreen hooks are integrated.
pub fn validate_supported(r: &Run) -> Result<(), String> {
    validate_supported_with_falls(r, &[])
}
pub fn validate_supported_with_falls(r: &Run, pending_falls: &[f64]) -> Result<(), String> {
    let _timing_function = crate::timing::span("environment.validate");
    r.floor().ok_or("Missing current floor")?;
    // Grounded occupants over gaps are legal source states (for example Remember flight expiry).
    // Actual travel and finite-flight cohort hooks decide falling; environment does not invent it.
    let _ = pending_falls;

    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if !a.dead.value_or(false)
            && (a.abilities.iter().any(|id| {
                // R50 powers have explicit turn/environment hooks. Keep their
                // admission revision- and owner-scoped; a catalog entry alone
                // must not admit an arbitrary NPC power or an older run.
                if crate::revision50::enabled(r)
                    && (id == crate::revision50_combat::MAGICKAL_CASTER
                        || (crate::half_here::owner_id(r, a) == r.player.id
                            && crate::revision50::supported(id)))
                {
                    return false;
                }
                ![
                    "enemy:beast:fungal-gas",
                    "enemy:beast:razorleaves",
                    "enemy:beast:minotaur",
                    "enemy:beast:vampire",
                    "enemy:beast:wendigo",
                    "enemy:beast:stalk",
                    "enemy:beast:sphinx",
                    "enemy:beast:yeti",
                    "enemy:beast:phoenix",
                    "enemy:beast:armor-piercing",
                    "enemy:beast:ursa-major",
                    "enemy:beast:detonate",
                    "enemy:beast:ruin-pulse",
                    "enemy:beast:silence-aura",
                    "enemy:beast:darkness-aura",
                    "enemy:beast:explosion-immunity",
                    "enemy:beast:aquatic",
                    "enemy:beast:submerge",
                    "enemy:beast:grab",
                    "enemy:beast:hold-only",
                    "enemy:beast:deep-water",
                    "enemy:beast:water-jet",
                    "enemy:beast:horn-bolt",
                    "enemy:beast:blink",
                    "enemy:beast:moon-laser",
                    "enemy:beast:bloodmancy",
                    "enemy:beast:witch",
                    "enemy:beast:saidribee-queen",
                    "enemy:beast:dryad",
                    "enemy:beast:beckon",
                    "enemy:beast:wall-seal",
                    "enemy:beast:inventor",
                    "enemy:beast:mech",
                    "enemy:beast:fire-breath",
                    "enemy:beast:lifesteal",
                    "enemy:beast:slow",
                    "enemy:pale:slow",
                    "enemy:pale:gas-immunity",
                    "enemy:pale:bite-repeat",
                    "enemy:pale:lifesteal",
                    "enemy:pale:scream",
                    "enemy:pale:spit",
                    "enemy:pale:gas-shot",
                    "enemy:pale:birth",
                    "enemy:pale:double-birth",
                    "enemy:pale:breath",
                    "enemy:pale:breath-boost",
                    "enemy:pale:gas-vulnerability",
                    "enemy:beast:fast",
                    "enemy:beast:ichor-bound",
                    "enemy:beast:ichor-siphon",
                    "enemy:beast:gas-immunity",
                    "enemy:beast:rooted",
                    "etill-phase-through",
                    "etill-rushing-torrent",
                    "etill-riptide",
                    "grisha-black-pit",
                    "dawel-reverse-entropy",
                    "grisha-entropy",
                    "anavani-star-power",
                    "anavani-ascend",
                    "etill-drown",
                    "numo-mind-read",
                    "ursa-dream-see",
                    "dawel-shadow-in-the-dark",
                    "dawel-not-there",
                    "enemy:beast:regeneration",
                    "enemy:beast:banshee",
                    "enemy:beast:ursa-minor",
                    "enemy:beast:ursa-major",
                    "enemy:beast:slime-walker",
                    "enemy:beast:slime-bound",
                    "enemy:beast:slime-trail",
                    "enemy:beast:venom-weak",
                    "enemy:beast:venom",
                    "enemy:beast:venom-strong",
                    "enemy:beast:no-revival",
                    "enemy:beast:fire-immunity",
                    "enemy:beast:everburn",
                    "enemy:beast:flight",
                ]
                .contains(&id.as_str())
                    && !(a.conditions.get("miniMe").is_some_and(|v| *v > 0.0)
                        && crate::abilities::mini_me_ability_supported(id))
                    && (crate::half_here::owner_id(r, a) != r.player.id
                        || ![
                            "shrail-unyielding-assault",
                            "pelian-never-surrender",
                            "isheilah-quick-hand",
                            "isheilah-are-you-sure-about-that",
                            "ursa-waking-nightmare",
                            "anavani-nova",
                            "morae-bonded-by-unseen-threads",
                            "dawel-unspeak",
                            "kative-mini-me",
                            "grisha-erase-metaphysical",
                            "grisha-uncast",
                            "morae-turning-the-wheel",
                            "ursa-dream-of-time-gone-by",
                            "shrail-rage-at-the-dying-of-the-light",
                            "pelian-you-shall-never-be-rid-of-me",
                            "anavani-shine",
                            "isheilah-make-slippery",
                            "etill-half-here",
                            "numo-mind-read",
                            "ursa-dream-see",
                            "dawel-shadow-in-the-dark",
                            "dawel-not-there",
                            "pelian-i-came-for-you",
                            "isheilah-missed-me",
                            "pelian-never-without-a-fight",
                            "grisha-death-swift",
                            "shrail-fight-long",
                            "shrail-i-hit-back",
                            "lichor-bleed-them",
                            "shrail-beat-to-death",
                            "lichor-hardened-blood",
                            "lichor-blood-weapon",
                            "lichor-blood-letting",
                            "lichor-overflowing",
                            "lichor-the-number-of-blood",
                            "etill-float-away",
                            "morae-fortune-teller",
                            "numo-distract",
                            "isheilah-tools-of-the-trade",
                            "grisha-boundless-hunger",
                            "kative-adaptive-armor",
                            "numo-implant-thought",
                            "numo-mind-slave",
                            "morae-sever-fate",
                            "anavani-illuminate",
                            "kative-dilute-potion",
                            "anavani-sparkle",
                            "anavani-make-kneel",
                            "ursa-sudden-sleep",
                            "morae-all-seeing-eyes",
                            "numo-revert",
                            crate::progression::WEAVE_ID,
                            "isheilah-guile-craft",
                            "shrail-fury-crafting",
                            "kative-tinker",
                            "numo-remember",
                            "dawel-silent-strike",
                            "ursa-sleep-walk",
                            "dawel-step-through-shadows",
                            "ursa-dream-craft",
                            "kative-artifice-craft",
                            "pelian-zeal-craft",
                            "kative-divine-craftsman",
                            "morae-see-the-future",
                            "pelian-bulwark-of-mind",
                            "dawel-reverse-entropy",
                            "grisha-entropy",
                            "anavani-star-power",
                            "etill-drown",
                            "anavani-ascend",
                        ]
                        .contains(&id.as_str())
                        || (a.active.contains(id)
                            && ![
                                "shrail-unyielding-assault",
                                "pelian-never-surrender",
                                "isheilah-quick-hand",
                                "isheilah-are-you-sure-about-that",
                                "ursa-waking-nightmare",
                                "anavani-nova",
                                "morae-bonded-by-unseen-threads",
                                "dawel-unspeak",
                                "kative-mini-me",
                                "grisha-erase-metaphysical",
                                "grisha-uncast",
                                "morae-turning-the-wheel",
                                "ursa-dream-of-time-gone-by",
                                "shrail-rage-at-the-dying-of-the-light",
                                "pelian-you-shall-never-be-rid-of-me",
                                "anavani-shine",
                                "isheilah-make-slippery",
                                "etill-half-here",
                                "numo-mind-read",
                                "ursa-dream-see",
                                "dawel-shadow-in-the-dark",
                                "dawel-not-there",
                                "isheilah-missed-me",
                                "pelian-never-without-a-fight",
                                "etill-float-away",
                                "dawel-reverse-entropy",
                                "grisha-entropy",
                                "anavani-star-power",
                            ]
                            .contains(&id.as_str())))
            }))
        {
            return Err(
                "Unsupported environment integration: living burning/emitter/ability traits".into(),
            );
        }
    }
    Ok(())
}
/// Deposit with already-resolved source identity (-2 player, -1 ambient).
pub fn add_gas(
    f: &mut Floor,
    i: usize,
    gas: &str,
    amount: f64,
    source: f64,
) -> Result<bool, String> {
    if i >= f.tiles.len() || !amount.is_finite() || amount <= 0. || !gas_passable(f, i) {
        return Ok(false);
    }
    if !GAS_TYPES.contains(&gas) {
        return Err(format!("Unknown gas type: {gas}"));
    }
    let depth_bits = f.depth.to_bits();
    let e = initialize(f)?;
    if gas == "hallucinogenic" {
        let n = e.fire.len();
        e.hallucinogenic.get_or_insert_with(|| vec![0.; n]);
        e.provenance.as_mut().unwrap().entry(gas.into()).or_insert_with(|| EnvironmentalStateProvenanceEntry {
            owner: vec![-1.; n], player: vec![0.; n],
        });
    }
    if gas == "bloodMist" {
        let n=e.fire.len();
        e.blood_mist.get_or_insert_with(||vec![0.;n]);
        e.blood_mist_turns.get_or_insert_with(||vec![0.;n])[i]=5.;
        e.provenance.as_mut().unwrap().entry(gas.into()).or_insert_with(||EnvironmentalStateProvenanceEntry{owner:vec![-1.;n],player:vec![0.;n]});
    }
    let added = amount.floor();
    let volume = (crate::rng::to_u32(e.gas_volume.as_ref().unwrap()[i] + added) & 65535) as f64;
    crate::capture_pilot::mark_gas_write(depth_bits, i, e.gas_type.as_ref().unwrap()[i].as_deref() != Some(gas));
    e.gas_type.as_mut().unwrap()[i] = Some(gas.into());
    crate::capture_pilot::mark_gas_write(depth_bits, i, e.gas_volume.as_ref().unwrap()[i].to_bits() != volume.to_bits());
    e.gas_volume.as_mut().unwrap()[i] = volume;
    crate::capture_pilot::mark_gas_write(depth_bits, i, e.gas_owner.as_ref().unwrap()[i].to_bits() != source.to_bits());
    e.gas_owner.as_mut().unwrap()[i] = source;
    let next_player = volume.min(e.gas_player.as_ref().unwrap()[i] + if source == -2. { added } else { 0. });
    crate::capture_pilot::mark_gas_write(depth_bits, i, e.gas_player.as_ref().unwrap()[i].to_bits() != next_player.to_bits());
    e.gas_player.as_mut().unwrap()[i] = next_player;
    let owner = e.gas_owner.as_ref().unwrap()[i];
    let player = e.gas_player.as_ref().unwrap()[i];
    let p = e.provenance.as_mut().unwrap().get_mut(gas).unwrap();
    p.owner[i] = owner;
    p.player[i] = player;
    for kind in GAS_TYPES {
        let value = if kind == gas { volume } else { 0. };
        match kind {
            "bloodMist" => {if let Some(v)=e.blood_mist.as_mut(){v[i]=value;}},
            "caustic" => e.caustic[i] = value,
            "breather" => e.breather[i] = value,
            "paralysis" => e.paralysis[i] = value,
            "confusion" => e.confusion.as_mut().unwrap()[i] = value,
            "hallucinogenic" => { if let Some(v) = e.hallucinogenic.as_mut() { v[i] = value; } },
            "flammable" => e.flammable.as_mut().unwrap()[i] = value,
            "explosive" => e.explosive.as_mut().unwrap()[i] = value,
            "hydrogen" => e.hydrogen.as_mut().unwrap()[i] = value,
            "steam" => e.steam.as_mut().unwrap()[i] = value,
            _ => unreachable!(),
        }
    }
    f.dirty = true;
    Ok(true)
}
pub fn gas_present(e: &EnvironmentalState, i: usize) -> bool {
    if let Some(g) = e.gas_type.as_ref() {
        g.get(i).is_some_and(Option::is_some)
    } else {
        GAS_TYPES
            .iter()
            .any(|gas| channel(e, gas).get(i).copied().unwrap_or(0.) > 0.)
    }
}
/// Existing single active gas, including typed zero-volume lingering gas.
/// Legacy admission chooses the greatest concentration in the configured order.
pub fn gas_kind_at(e: &EnvironmentalState, i: usize) -> Option<&str> {
    if let (Some(types), Some(_)) = (e.gas_type.as_ref(), e.gas_volume.as_ref()) {
        types.get(i).and_then(|v| v.as_deref())
    } else {
        let mut best = None;
        let mut max = 0.;
        for kind in GAS_TYPES {
            let v = channel(e, kind).get(i).copied().unwrap_or(0.).max(0.);
            if v > max {
                max = v;
                best = Some(kind)
            }
        }
        best
    }
}
/// Frozen cells deliberately do not stop vent countdowns (v48 custom contract).
/// Returns erupted vent cells for the caller's visibility-gated log/events.
pub fn tick_vents(f: &mut Floor) -> Result<Vec<usize>, String> {
    let _timing_function = crate::timing::span("environment.vents");
    let mut erupted = vec![];
    if let Some(vents) = f.vents.as_mut() {
        for (i, v) in vents.iter_mut().enumerate() {
            if v.phase != "waiting" {
                continue;
            }
            if v.countdown > 0. {
                v.countdown -= 1.
            }
            if v.countdown > 0. {
                continue;
            }
            erupted.push(i);
        }
    }
    let mut cells = vec![];
    for index in erupted {
        let (at, gas) = {
            let v = &f.vents.as_ref().unwrap()[index];
            (v.at as usize, v.gas.clone())
        };
        add_gas(f, at, &gas, 1000., -1.)?;
        let v = &mut f.vents.as_mut().unwrap()[index];
        v.amount = 1000f64.into();
        v.pulse = IndexMap::from([(at.to_string(), 1.)]).into();
        v.phase = "clearing".into();
        v.countdown = 0.;
        f.dirty = true;
        cells.push(at);
    }
    Ok(cells)
}
pub fn finish_vent_clearing(f: &mut Floor) -> Result<(), String> {
    initialize(f)?;
    let e = f.environment.as_ref().unwrap();
    if let Some(vents) = f.vents.as_mut() {
        for v in vents {
            if let Some(p) = v.pulse.as_mut() {
                p.retain(|cell, _| {
                    cell.parse::<usize>()
                        .ok()
                        .is_some_and(|i| gas_present(e, i))
                });
            }
            if v.phase == "clearing" && !gas_present(e, v.at as usize) {
                v.phase = "waiting".into();
                v.countdown = 30.;
                f.dirty = true;
            }
        }
    }
    Ok(())
}
pub fn burn_roots(f: &mut Floor, cell: usize) {
    if let Some(roots) = f.roots.as_mut() {
        roots[cell] = 0.;
    }
    if let Some(connections) = f.fungal_roots.as_mut() {
        for root in connections {
            if root.active && root.grown_path().contains(&(cell as f64)) {
                root.active = false;
            }
        }
    }
}
/// R51 discards the disconnected material but preserves severed pair history.
pub fn burn_roots_revision(f: &mut Floor, cell: usize, revision: f64) {
    burn_roots(f, cell);
    if revision >= 51. { crate::fungi::clear_orphaned_root_cells(f); }
}
pub fn corpse_rooted(f: &Floor, actors: &[crate::Actor], corpse: &crate::Actor) -> bool {
    f.fungal_roots.as_ref().is_some_and(|roots| {
        roots.iter().any(|root| {
            root.active
                && root.reaches_corpse()
                && root.corpse_id == corpse.id
                && actors.iter().any(|a| {
                    a.id == root.fungus_id
                        && !a.dead.value_or(false)
                        && a.abilities.iter().any(|x| x == "enemy:beast:fungal-gas")
                })
                && root.path.iter().all(|&i| {
                    f.roots
                        .as_ref()
                        .and_then(|v| v.get(i as usize))
                        .copied()
                        .unwrap_or(0.)
                        > 0.
                        && f.environment
                            .as_ref()
                            .and_then(|e| e.fire.get(i as usize))
                            .copied()
                            .unwrap_or(0.)
                            <= 0.
                })
        })
    })
}
/// Direct ignition preserves the source's duration compatibility parameter: it
/// validates duration but terrain lifetime is promotion-based, never a timer.
/// Resolve player provenance through the shared Half Here owner.
pub fn source_id(r: &Run, source: Option<f64>) -> f64 {
    source
        .map(|id| {
            let owner = crate::engine::actor(r, id)
                .map(|a| crate::half_here::owner_id(r, a))
                .unwrap_or(id);
            if owner == r.player.id { -2. } else { id }
        })
        .unwrap_or(-1.)
}
pub fn ignite(
    r: &mut Run,
    x: f64,
    y: f64,
    duration: Option<f64>,
    source: Option<f64>,
) -> Result<(), String> {
    let owner = source_id(r, source);
    ignite_owned(r, x, y, duration, owner)
}
fn ignite_owned(
    r: &mut Run,
    x: f64,
    y: f64,
    duration: Option<f64>,
    owner: f64,
) -> Result<(), String> {
    let f = r.floor().ok_or("Missing floor")?;
    if x < 0. || y < 0. || x >= f.width() as f64 || y >= f.height() as f64 {
        return Ok(());
    }
    let i = y as usize * f.width() + x as usize;
    let revision = r.simulation_revision.value_or(29.);
    if duration.is_none_or(|v| v.is_finite() && v > 0.)
        && crate::revision51_fire_water::extinguish_cell(r.floor_mut().unwrap(), i, revision) {
        return Ok(());
    }
    let tile = r.floor().unwrap().tiles[i].clone();
    if crate::revision50::enabled(r)&&duration.is_none_or(|v|v.is_finite()&&v>0.) {crate::terrain_runtime::melt_ice(r.floor_mut().unwrap(),i);}
    let f=r.floor().unwrap();
    if duration.is_none_or(|v|v.is_finite() && v>0.) && crate::terrain_runtime::enabled(r)
        && crate::terrain_runtime::snow_at(f,i) {
        crate::terrain_runtime::melt_snow(r.floor_mut().unwrap(),i);
        return Ok(());
    }
    let f=r.floor().unwrap();
    let ichor = f
        .ichor
        .as_ref()
        .and_then(|v| v.get(i))
        .copied()
        .unwrap_or(0.)
        > 0.;
    if (!ichor && (tile == "water" || tile == "shallow-water"))
        || ["wall", "chasm", "pit"].contains(&tile.as_str())
        || duration.is_some_and(|v| !v.is_finite() || v <= 0.)
    {
        return Ok(());
    }
    let revision = r.simulation_revision.value_or(29.);
    let f = r.floor_mut().unwrap();
    let permanent = crate::terrain_runtime::hellfire_at(f,i) || f
        .permanent_fire
        .as_ref()
        .is_some_and(|v| v.contains(&(i as f64)));
    let e = initialize(f)?;
    let was = e.fire[i] > 0.;
    e.fire_kind.as_mut().unwrap()[i] = Some(
        if permanent {
            "brazier"
        } else if ichor {
            "ichor"
        } else if tile == "grass" || tile == "tree" {
            "plain"
        } else {
            "creature"
        }
        .into(),
    );
    if !was {
        e.fire_fresh.as_mut().unwrap()[i] = true
    }
    e.fire[i] = 1.;
    let p = e.provenance.as_mut().unwrap().get_mut("fire").unwrap();
    p.owner[i] = owner;
    if owner == -2. {
        p.player[i] = 1.;
    }
    if tile == "grass" || tile == "tree" {
        f.tiles[i] = "floor".into();
        crate::ichor::enqueue_opening_revision(f, i, revision);
    }
    if let Some(v) = f.slime.as_mut() {
        v[i] = 0.;
    }
    if let Some(v) = f.razorleaves.as_mut() {
        v[i] = 0.;
    }
    if f.roots.as_ref().is_some_and(|v| v[i] > 0.) {
        burn_roots_revision(f, i, revision)
    }
    f.dirty = true;
    Ok(())
}
pub fn place_sparkle(r: &mut Run, x: f64, y: f64) -> Result<(), String> {
    let revision = r.simulation_revision.value_or(29.);
    let f = r.floor_mut().ok_or("Missing floor")?;
    let i = y as usize * f.width() + x as usize;
    // Target admission/payment already succeeded. Wet placement is legal, but
    // the flame and light immediately go out and cannot leave a relighting seed.
    if crate::revision51_fire_water::extinguish_sparkle(f, i, revision) { return Ok(()); }
    let permanent = f.permanent_fire.get_or_insert_with(Vec::new);
    if !permanent.contains(&(i as f64)) {
        permanent.push(i as f64);
    }
    let lights = f.light_sources.get_or_insert_with(Vec::new);
    if let Some(light) = lights.iter_mut().find(|s| s.x == x && s.y == y) {
        light.power = 100.;
        light.falloff = 10.;
        light.kind = Field::Value("brazier".into());
    } else {
        lights.push(crate::FloorLightSourcesEntry {
            x,
            y,
            power: 100.,
            falloff: 10.,
            kind: Field::Value("brazier".into()),
        });
    }
    let id = r.player.id;
    ignite(r, x, y, None, Some(id))?;
    // R50 Sparkle places fire on any visible square, even where ordinary
    // ignition is impossible. O(V) environment admission plus O(1) cell writes;
    // retain terrain and the same permanent-source upkeep/provenance contract.
    if crate::revision50::enabled(r) {
        let e = initialize(r.floor_mut().unwrap())?;
        if e.fire[i] <= 0.0 {
            e.fire_fresh.as_mut().unwrap()[i] = true;
        }
        e.fire[i] = 1.0;
        e.fire_kind.as_mut().unwrap()[i] = Some("brazier".into());
        let provenance = e.provenance.as_mut().unwrap().get_mut("fire").unwrap();
        provenance.owner[i] = -2.0;
        provenance.player[i] = 1.0;
    }
    r.floor_mut().unwrap().dirty = true;
    Ok(())
}
fn cond(a: &crate::Actor, k: &str) -> f64 {
    a.conditions.get(k).copied().unwrap_or(0.)
}
fn fire_immune(r: &Run, a: &crate::Actor) -> bool {
    crate::revision51_ichor_fire::immune(r.simulation_revision.value_or(29.), a)
        || crate::revision50_potions::fire_immune(a) || a.abilities
        .iter()
        .any(|s| s == "enemy:beast:fire-immunity" || s == "anavani-star-power")
}
const BURNING_TICK_DAMAGE: f64 = 4.;
/// Steam transfers heat, not a gas status; use the same protection as fire.
pub fn steam_can_scald(r: &Run, a: &crate::Actor) -> bool {
    crate::steam_rules::current(r) && !fire_immune(r, a)
}
fn gas_immune(a: &crate::Actor) -> bool {
    a.abilities.iter().any(|s| s == "enemy:beast:gas-immunity")
}
/// O(1) owner checks. Revision43 hallucinogenic exposure belongs only to the
/// player and existing shared-owner endpoints; no status transfer or new sight.
fn hallucination_affects(r: &Run, a: &crate::Actor) -> bool {
    r.simulation_revision.value_or(29.) < 43.
        || crate::half_here::owner_id(r, a) == r.player.id
}
fn source_actor(r: &Run, owner: f64, player: f64) -> Option<f64> {
    if player > 0. || owner == -2. {
        Some(r.player.id)
    } else {
        r.actors_at(r.depth)
            .iter()
            .find(|a| a.id == owner)
            .map(|a| a.id)
    }
}
/// Apply each blast once per reaction and owner pool, after the entire closure.
pub fn resolve_bursts(
    r: &mut Run,
    bursts: Vec<crate::fire_adapter::Burst>,
    shared: Option<&mut std::collections::HashSet<i64>>,
    audible: bool,
) -> Result<(), String> {
    if !crate::dreamwalking::enabled(r) {
        return resolve_bursts_inner(r, bursts, shared, audible);
    }
    // Gas closure is native to its terrain layer even when triggered outside the
    // world tick. Attribution may name a shared owner now on another plane.
    let previous = crate::dreamwalking::environment_context(r);
    r.dreamwalking.get_or_insert_with(Default::default).resolving_environment = true;
    let result = resolve_bursts_inner(r, bursts, shared, audible);
    if let Some(state) = r.dreamwalking.as_mut() { state.resolving_environment = previous; }
    result
}
fn resolve_bursts_inner(
    r: &mut Run,
    bursts: Vec<crate::fire_adapter::Burst>,
    shared: Option<&mut std::collections::HashSet<i64>>,
    audible: bool,
) -> Result<(), String> {
    let _timing_function = crate::timing::span("environment.burst_victims");
    let mut pools: std::collections::HashMap<u64, std::collections::HashSet<i64>> =
        std::collections::HashMap::new();
    let mut shared = shared;
    let mut announced = std::collections::HashSet::new();
    for burst in bursts {
        if !burst.detonates {
            continue;
        }
        if audible && announced.insert(burst.reaction) {
            crate::engine::log(r, "You hear an explosion!", "environment");
        }
        let f = r.floor().unwrap();
        let x = burst.at % f.width();
        let y = burst.at / f.width();
        explosion_event(r, x as f64, y as f64, burst.radius as f64, "shared", None);
        let origin = terrain_origin(r, x as f64, y as f64);
        let victims: Vec<_> = std::iter::once(&r.player)
            .chain(r.actors_at(r.depth))
            .filter(|a| !a.dead.value_or(false))
            .map(|a| a.id)
            .collect();
        let source = source_actor(r, burst.owner, burst.player);
        for id in victims {
            let Some(a) = crate::engine::actor(r, id) else {
                continue;
            };
            // Reject other-plane endpoints before reserving their shared HP pool.
            // Otherwise an unhittable projection can shield its physical body.
            if !crate::dreamwalking::contact_plane(r, a) { continue; }
            let owner = crate::half_here::owner_id(r, a) as i64;
            let pool = if let Some(s) = shared.as_deref_mut() {
                s
            } else {
                pools.entry(burst.reaction).or_default()
            };
            if gas_immune(a)
                || ((a.x - origin.x).powi(2) + (a.y - origin.y).powi(2)).sqrt()
                    > burst.radius as f64
                || !crate::vision::line_of_sight(r, &origin, a)
                || pool.contains(&owner)
            {
                continue;
            }
            pool.insert(owner);
            if a.abilities
                .iter()
                .any(|id| id == "anavani-nova" || id == "enemy:beast:explosion-immunity")
            {
                continue;
            }
            let amount = crate::combat::explosion_damage_for_run(r, a.max_hp);
            crate::engine::damage_environment(r, id, amount, source, "explosion", false)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub fn refresh_burning_from_fire(r: &mut Run, id: f64) -> Result<(), String> {
    let revision = r.simulation_revision.value_or(29.);
    if crate::engine::actor(r, id)
        .is_some_and(|a| crate::revision51_ichor_fire::immune(revision, a)) {
        crate::revision51_ichor_fire::extinguish(revision,
            crate::engine::actor_mut_trusted(r, id).unwrap());
        return Ok(());
    }
    if crate::engine::actor(r,id).is_some_and(crate::revision50_nightmares::is_decoy) { return Ok(()); }
    crate::terrain_runtime::snow_contact(r,id);
    let a = crate::engine::actor(r, id).ok_or("Missing actor")?;
    let f = r.floor().ok_or("Missing floor")?;
    if !crate::dreamwalking::contact_plane(r,a) || a.dead.value_or(false) || a.x < 0. || a.y < 0. {
        return Ok(());
    }
    let i = a.y as usize * f.width() + a.x as usize;
    let Some(e) = f.environment.as_ref() else {
        return Ok(());
    };
    if !burns_on_contact(r, e, i)
        || fire_immune(r, a)
        || cond(a, "underwater") > 0.
        || ((f.tiles[i] == "water" || f.tiles[i] == "shallow-water")
            && f.ichor
                .as_ref()
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(0.)
                == 0.
            && cond(a, "flying") <= 0.)
    {
        return Ok(());
    }
    if gas_immune(a)
        && !crate::corpses::is_bastidion(a)
        && e.fire_kind
            .as_ref()
            .is_some_and(|v| matches!(v[i].as_deref(), Some("gas" | "gas-explosion")))
    {
        return Ok(());
    }
    let p = e.provenance.as_ref().and_then(|p| p.get("fire"));
    let owner = p.and_then(|p| p.owner.get(i)).copied().unwrap_or(-1.);
    let player = p.and_then(|p| p.player.get(i)).copied().unwrap_or(0.);
    if crate::revision50_combat::oblivious_blocks(r,a,source_actor(r,owner,player).and_then(|id|crate::engine::actor_any(r,id))) {return Ok(());}
    let random = crate::rng::random(r);
    crate::corpses::apply_burning_run(r, id,
        revision,
        random,
        Some(owner),
        Some(player),
        None,
    );
    Ok(())
}
pub fn tick_environment(r: &mut Run) -> Result<(), String> {
    tick_environment_options(r, None)
}
pub fn tick_environment_options(r: &mut Run, black_pit_active: Option<bool>) -> Result<(), String> {
    tick_environment_options_with_falls(r, black_pit_active, &[])
}
pub fn tick_environment_options_with_falls(
    r: &mut Run,
    black_pit_active: Option<bool>,
    pending_falls: &[f64],
) -> Result<(), String> {
    let _timing_function = crate::timing::span("environment.total");
    crate::blood_mist::tick(r.floor_mut().ok_or("Missing floor")?)?;
    crate::revision50_control_amulets::expire_terrain(r);
    let _timing_environment_stage = crate::timing::span("environment.setup");
    validate_supported_with_falls(r, pending_falls)?;
    crate::corpse_currents::settle_falls(r)?;
    let native_hooks = crate::water_blood::enabled(r);
    let depth = r.depth;
    let ids: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(depth))
        .filter(|a| !a.dead.value_or(false) && !crate::revision50_nightmares::is_decoy(a) && crate::dreamwalking::contact_plane(r,a))
        .map(|a| a.id)
        .collect();
    let revision = r.simulation_revision.value_or(29.);
    for &id in &ids {
        if crate::engine::actor(r, id).is_some_and(|a| crate::revision51_ichor_fire::immune(revision, a)) {
            crate::revision51_ichor_fire::extinguish(revision,
                crate::engine::actor_mut_trusted(r, id).unwrap());
        }
        crate::terrain_runtime::snow_contact(r,id);
    }
    crate::terrain_runtime::melt_burning_snow(r);
    // Hallucination duration is a normal world-turn clock, never an actor/haste clock.
    // Decay before exposure; contact below refreshes duration, never adds to it.
    if native_hooks {
        for &id in &ids {
            let a = crate::engine::actor_mut_trusted(r, id).unwrap();
            if let Some(v) = a.conditions.get_mut("hallucinating") {
                *v = (*v - 1.).max(0.);
            }
        }
    }
    let retained_endpoint = r
        .actors_at(depth)
        .iter()
        .find(|a| !a.dead.value_or(false) && crate::half_here::is_clone(a))
        .cloned();
    if !crate::dreamwalking::context_dream(r) {for a in r.actors_slice_mut(depth) {
        crate::corpses::advance_corpse_burning(a, 1.);
    }}
    let n = r.floor().unwrap().tiles.len();
    let revision = r.simulation_revision.value_or(29.);
    {
        let f = r.floor_mut().unwrap();
        initialize(f)?;
        if native_hooks { f.water_blood.get_or_insert_with(|| vec![0.; n]); }
        crate::ichor::tick_ichor_revision(f, revision);
        f.fungal_roots.get_or_insert_with(Vec::new);
        if let Some(leaves) = f.razorleaves.as_mut() {
            leaves.fill(0.)
        }
        for i in f.permanent_fire.value_or_default() {
            let e = f.environment.as_mut().unwrap();
            e.fire[i as usize] = e.fire[i as usize].max(2.);
        }
    }
    if crate::events::enabled(r) {
        let cells:Vec<_>=r.floor().and_then(|f|f.godbeast_event.as_ref().filter(|e|e.kind=="Mercuria").and_then(|e|e.flood_frontier.as_ref()).map(|v|(f.width(),v))).map(|(w,v)|v.iter().map(|i|crate::events::EventPoint{x:(*i as usize%w)as f64,y:(*i as usize/w)as f64}).collect()).unwrap_or_default();
        crate::events::at(r,"E78","wave-ichor",crate::events::Stage::Special,&cells);
    }
    if !crate::dreamwalking::context_dream(r) {crate::ichor::tick_fish(r)?;}
    let leaf_sources = refresh_razorleaves(r, &ids);
    let vent_events = tick_vents(r.floor_mut().unwrap())?;
    for at in vent_events {
        let f = r.floor().unwrap();
        let width = f.width();
        if gas_passable(f, at) && crate::vision::visible(r, at % width, at / width) {
            let gas = f
                .vents
                .as_ref()
                .unwrap()
                .iter()
                .find(|v| v.at == at as f64)
                .unwrap()
                .gas
                .clone();
            let label = match gas.as_str() {
                "steam" | "hydrogen" => gas.clone(),
                _ => format!("{gas} gas"),
            };
            crate::engine::log(
                r,
                format!("A {label} vent erupts, releasing a dense cloud."),
                "environment",
            );
        }
    }
    drop(_timing_environment_stage);
    let _timing_environment_stage = crate::timing::span("environment.freeze_blood_fungi");
    let frozen = frozen_environment(r);
    if native_hooks { crate::water_blood::diffuse(r.floor_mut().unwrap(), &frozen); }
    if !crate::dreamwalking::context_dream(r) {crate::fungi::tick_fungi(r, &frozen)?;}
    drop(_timing_environment_stage);
    let _timing_environment_stage = crate::timing::span("environment.passive_emitters");
    let mut adapter = crate::fire_adapter::FireAdapter::new(r, frozen.clone())?;
    for &id in &ids {
        crate::terrain_runtime::snow_contact(adapter.run,id);
        let revision=adapter.run.simulation_revision.value_or(29.);
        crate::enemy_traits::initialize_enemy_traits_revision(
            crate::engine::actor_mut_trusted(adapter.run, id).unwrap(),revision,
        );
        let a = crate::engine::actor(adapter.run, id).unwrap().clone();
        let at = a.y as usize * adapter.run.floor().unwrap().width() + a.x as usize;
        if !crate::dreamwalking::is_body(&a) && a.abilities.iter().any(|id| id == "grisha-entropy")
            && !a.active.iter().any(|id| id == "grisha-entropy")
        {
            let owner = source_id(adapter.run, Some(id));
            add_gas(adapter.run.floor_mut().unwrap(), at, "caustic", 15., owner)?;
        }
        if a.abilities.iter().any(|id| id == "enemy:pale:breath") {
            let owner = source_id(adapter.run, Some(id));
            let volume = a.conditions.get("monsterGasPassiveVolume").copied().unwrap_or(35.);
            let gas = breather_gas(adapter.run, &a);
            let before=adapter.run.floor().unwrap().environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0.,|v|v[at]);
            add_gas(adapter.run.floor_mut().unwrap(), at, gas, volume, owner)?;
            let after=adapter.run.floor().unwrap().environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0.,|v|v[at]);
            if after>before {crate::animation_enemy::mark(adapter.run,&a,"E56",if gas=="hallucinogenic" {"puff-violet"}else{"puff-green"},crate::events::Stage::Special);}
        }
        if !crate::dreamwalking::is_body(&a) && a.abilities.iter().any(|id| id == "anavani-star-power") {
            if a.active.iter().any(|id| id == "anavani-star-power") {
                if !a.abilities.iter().any(|id| id == "enemy:beast:everburn")
                    || cond(&a, "burning") <= 0.
                {
                    crate::engine::actor_mut_trusted(adapter.run, id)
                        .unwrap()
                        .conditions
                        .insert("burning".into(), 0.);
                }
            } else {
                let random = crate::rng::random(adapter.run);
                crate::corpses::apply_burning_run(adapter.run, id,
                    revision,
                    random,
                    None,
                    None,
                    None,
                );
            }
        }
        let f = adapter.run.floor().unwrap();
        let i = a.y as usize * f.width() + a.x as usize;
        if crate::corpses::has_permanent_burning(&a)
            && !crate::revision51_ichor_fire::immune(revision, &a) {
            let actor = crate::engine::actor_mut_trusted(adapter.run, id).unwrap();
            let duration = cond(actor, "burning").max(1.);
            actor.conditions.insert("burning".into(), duration);
        }
        crate::terrain_runtime::snow_contact(adapter.run,id);
        let a = crate::engine::actor(adapter.run, id).unwrap().clone();
        let f = adapter.run.floor().unwrap();
        if cond(&a, "burning") > 0.
            && cond(&a, "underwater") == 0.
            && !((f.tiles[i] == "water" || f.tiles[i] == "shallow-water")
                && f.ichor
                    .as_ref()
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.)
                    == 0.
                && !is_flying_in(adapter.run, &a))
        {
            let owner = a
                .conditions
                .get("burningOwner")
                .copied()
                .unwrap_or(source_id(adapter.run, Some(id)));
            let player = a.conditions.get("burningPlayer").copied().unwrap_or(
                if source_id(adapter.run, Some(id)) == -2. {
                    1.
                } else {
                    0.
                },
            );
            adapter.ignite_creature(i, owner, player);
        }
    }
    drop(_timing_environment_stage);
    let _timing_environment_stage = crate::timing::span("environment.propagation");
    let mut rng = crate::rng::to_u32(adapter.run.rng);
    propagate_gas(adapter.run.floor_mut().unwrap(), &mut rng, &frozen)?;
    adapter.run.rng = rng as f64;
    let bursts = {
        let _timing_fire = crate::timing::span("environment.fire_advance");
        adapter.advance()?
    };
    drop(adapter);
    drop(_timing_environment_stage);
    let _timing_environment_stage = crate::timing::span("environment.damage_exposure");
    resolve_bursts(r, bursts, None, true)?;
    for &id in &ids {
        // The source living cohort retains a dismissed Half Here reference. Its owner
        // is already dead, but the gas provenance presentation still visits this cell.
        if crate::engine::actor(r, id).is_none() {
            if let Some(endpoint) = retained_endpoint.as_ref().filter(|a| a.id == id) {
                let f = r.floor_mut().unwrap();
                let i = endpoint.y as usize * f.width() + endpoint.x as usize;
                let e = f.environment.as_mut().unwrap();
                let owner = e.gas_owner.as_ref().unwrap()[i];
                let player = e.gas_player.as_ref().unwrap()[i];
                for gas in GAS_TYPES {
                    let Some(p) = e.provenance.as_mut().unwrap().get_mut(gas) else { continue; };
                    p.owner[i] = owner;
                    p.player[i] = player;
                }
            }
            continue;
        }
        if crate::engine::actor(r, id).unwrap().dead.value_or(false) {
            continue;
        }
        let a = crate::engine::actor(r, id).unwrap().clone();
        let f = r.floor().unwrap();
        let i = a.y as usize * f.width() + a.x as usize;
        let e = f.environment.as_ref().unwrap();
        let gas = e.gas_type.as_ref().unwrap()[i].clone();
        let owner = e.gas_owner.as_ref().unwrap()[i];
        let player = e.gas_player.as_ref().unwrap()[i];
        let source = source_actor(r, owner, player);
        {
            let e = r.floor_mut().unwrap().environment.as_mut().unwrap();
            for gas in GAS_TYPES {
                let Some(p) = e.provenance.as_mut().unwrap().get_mut(gas) else { continue; };
                p.owner[i] = owner;
                p.player[i] = player;
            }
        }
        if !gas_immune(&a)
            && !a
                .abilities
                .iter()
                .any(|id| id == "grisha-entropy" || id == "enemy:pale:gas-immunity")
        {
            match gas.as_deref() {
                Some("caustic" | "breather") => {
                    let raw = a.max_hp * (1. / 15.) * (1. + cond(&a, "gasVulnerable").max(0.));
                    let damage = if crate::combat::nearest_damage_enabled(r) { raw } else { raw.ceil() };
                    crate::engine::damage_environment(
                        r,
                        id,
                        damage,
                        source,
                        if gas.as_deref() == Some("caustic") {
                            "caustic"
                        } else {
                            "breatherGas"
                        },
                        true,
                    )
                    .map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
        if crate::engine::actor(r, id).is_none_or(|a| a.dead.value_or(false)) {
            continue;
        }
        if r.floor()
            .unwrap()
            .razorleaves
            .as_ref()
            .is_some_and(|v| v[i] > 0.)
            && !a.abilities.iter().any(|v| v == "enemy:beast:razorleaves")
        {
            let current = crate::engine::actor(r, id).unwrap();
            crate::engine::damage_environment(
                r,
                id,
                current.max_hp * 0.02,
                leaf_sources.get(&i).copied(),
                "razorleaves",
                true,
            )
            .map_err(|e| e.to_string())?;
        }
        if crate::engine::actor(r, id).is_none_or(|a| a.dead.value_or(false)) {
            continue;
        }
        if !gas_immune(&a) && !crate::revision50_combat::oblivious_blocks(r,&a,source.and_then(|id|crate::engine::actor_any(r,id))) {
            let hallucination_contact = hallucination_affects(r, &a);
            let five_turn_gas = crate::gas_rules::current(r);
            let paralysis_duration = crate::monster_statuses::status_duration(r, &a, if five_turn_gas { 5. } else { 2. });
            let mental_duration = match gas.as_deref() {
                Some("hallucinogenic") => crate::monster_statuses::willpower_duration(r, &a, if five_turn_gas { 5. } else { 2. }),
                Some("confusion") => crate::monster_statuses::willpower_duration(r, &a, if five_turn_gas { 5. } else { 8. }),
                _ => 0.,
            };
            let before=crate::engine::actor(r,id).unwrap();
            let prior=[cond(before,"paralyzed"),cond(before,"hallucinating"),cond(before,"confused")];
            let a = crate::engine::actor_mut_trusted(r, id).unwrap();
            if gas.as_deref() == Some("paralysis") {
                let v = cond(a, "paralyzed").max(paralysis_duration);
                a.conditions.insert("paralyzed".into(), v);
            }
            if native_hooks && hallucination_contact && gas.as_deref() == Some("hallucinogenic")
                && !a.abilities.iter().any(|id| id == "enemy:pale:gas-immunity")
            {
                let duration = if five_turn_gas { cond(a, "hallucinating").max(mental_duration) } else { mental_duration };
                a.conditions.insert("hallucinating".into(), duration);
            }
            if gas.as_deref() == Some("confusion") {
                let v = cond(a, "confused").max(mental_duration);
                a.conditions.insert("confused".into(), v);
                a.conditions.shift_remove("confusedFresh");
            }
            let after=crate::engine::actor(r,id).unwrap();
            for (key,before) in ["paralyzed","hallucinating","confused"].into_iter().zip(prior) {
                if before<=0. && cond(after,key)>0. {crate::animation_enemy::mark(r,after,"E72",if key=="paralyzed"{"bracket-violet"}else{"crossed-ring-violet"},crate::events::Stage::Special);}
            }
        }
        {
            let a = crate::engine::actor(r, id).unwrap();
            let f = r.floor().unwrap();
            if (f.tiles[i] == "water" || f.tiles[i] == "shallow-water")
                && f.ichor
                    .as_ref()
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.)
                    == 0.
                && !is_flying_in(r, a)
                && !fire_immune(r, a)
                && !a.abilities.iter().any(|s| s == "enemy:beast:everburn")
            {
                crate::engine::actor_mut_trusted(r, id)
                    .unwrap()
                    .conditions
                    .insert("burning".into(), 0.);
            }
        }
        refresh_burning_from_fire(r, id)?;
        let a = crate::engine::actor(r, id).unwrap().clone();
        if cond(&a, "burning") > 0. {
            if !fire_immune(r, &a) {
                let source = source_actor(
                    r,
                    a.conditions.get("burningOwner").copied().unwrap_or(-1.),
                    cond(&a, "burningPlayer"),
                );
                crate::engine::damage_environment(r, id, BURNING_TICK_DAMAGE, source, "fire", true)
                    .map_err(|e| e.to_string())?;
            }
            if !a.abilities.iter().any(|s| s == "enemy:beast:everburn") {
                if let Some(v) =
                    crate::engine::actor_mut_trusted(r, id).and_then(|a| a.conditions.get_mut("burning"))
                {
                    *v -= 1.;
                }
            }
        } else if gas.as_deref() == Some("steam") && steam_can_scald(r, &a) {
            // One heat packet per world tick. Existing Burning keeps its own
            // attribution and expiration; steam never adds a persistent status.
            crate::engine::damage_environment(r, id, BURNING_TICK_DAMAGE, source, "steam", true)
                .map_err(|e| e.to_string())?;
        }
    }
    drop(_timing_environment_stage);
    let _timing_environment_stage = crate::timing::span("environment.cleanup_and_items");
    for id in ids {
        let Some(a) = crate::engine::actor_mut_trusted(r, id) else {
            continue;
        };
        if cond(a, "burning") <= 0. {
            a.conditions.shift_remove("burningOwner");
            a.conditions.shift_remove("burningPlayer");
        }
        for key in ["screamWeakened", "screamEmboldened", "disoriented", "gasVulnerableTicks"] {
            if let Some(v) = a.conditions.get_mut(key) {
                if *v > 0. {
                    *v -= 1.
                }
            }
        }
        if cond(a, "gasVulnerableTicks") == 0. && a.conditions.contains_key("gasVulnerableTicks") {
            a.conditions.shift_remove("gasVulnerable");
        }
    }
    let black_pit = black_pit_active
        .unwrap_or_else(|| r.player.active.iter().any(|id| id == "grisha-black-pit"));
    let restoring:Vec<usize>=if crate::events::enabled(r) && !black_pit {
        let f=r.floor().unwrap();f.altered_chasms.as_ref().into_iter().flat_map(|m|m.iter()).filter_map(|(key,record)|key.parse::<usize>().ok().filter(|&i|record.age>=2. && f.tiles.get(i).is_some_and(|t|t=="chasm"))).collect()
    }else{vec![]};
    restore_altered_chasms_revision(r.floor_mut().unwrap(), black_pit, revision);
    if !restoring.is_empty() {
        let f=r.floor().unwrap();let w=f.width();let cells:Vec<_>=restoring.into_iter().filter(|&i|f.tiles[i]!="chasm").map(|i|crate::events::EventPoint{x:(i%w)as f64,y:(i/w)as f64}).collect();
        crate::events::at(r,"P53","unfold",crate::events::Stage::Special,&cells);
    }
    if !crate::dreamwalking::active(r) && !crate::dreamwalking::context_dream(r) {crate::inventory::float_carried_item(r)?;}
    crate::corpse_currents::drift(r);
    drift_ground_items(r)?;
    let f = r.floor_mut().unwrap();
    let e = f.environment.as_ref().unwrap();
    if e.fire.iter().any(|&v| v > 0.)
        || GAS_TYPES
            .iter()
            .any(|g| channel(e, g).iter().any(|&v| v > 0.))
    {
        f.dirty = true;
    }
    *r.metrics.entry("environmentCells".into()).or_default() += n as f64;
    Ok(())
}
pub(crate) fn legacy_channel<'a>(e: &'a EnvironmentalState, gas: &str) -> &'a [f64] {
    channel(e, gas)
}
/// Covers body-local flight sources shared by environmental contact and routing.
/// Run-aware flight includes equipped Dream Craft and the shared owner.
pub fn is_flying_in(r: &Run, a: &crate::Actor) -> bool {
    crate::travel::flying(r, a)
}
pub fn is_flying(a: &crate::Actor) -> bool {
    cond(a, "flying") > 0.
        || crate::monster_vampire::flying(a)
        || a.abilities
            .iter()
            .any(|id| id == "enemy:beast:flight" || id == "anavani-ascend")
        || a.active.iter().any(|id| id == "etill-phase-through")
}
pub fn is_swimming(r: &Run, a: &crate::Actor) -> bool {
    if a.dead.value_or(false) || a.x < 0. || a.y < 0. {
        return false;
    }
    let Some(f) = r.floor() else { return false };
    let i = a.y as usize * f.width() + a.x as usize;
    f.tiles.get(i).is_some_and(|t| t == "water")
        && f.ichor
            .as_ref()
            .and_then(|v| v.get(i))
            .copied()
            .unwrap_or(0.)
            == 0.
        && !is_flying_in(r, a)
}
/// Stable entry ordering, one drift coin per eligible still-water item.
fn drift_ground_items(r: &mut Run) -> Result<(), String> {
    let _timing_function = crate::timing::span("environment.item_drift");
    if crate::dreamwalking::context_dream(r) {return Ok(());}
    let depth = r.depth;
    let width = r.floor().unwrap().width();
    let count = r.ground.get(&depth.to_string()).map_or(0, |g| g.len());
    let mut occupied: std::collections::HashSet<usize> = r
        .ground
        .get(&depth.to_string())
        .into_iter()
        .flat_map(|v| v.iter())
        .filter(|e| !crate::dreamwalking::dream_ground_item(&e.item))
        .map(|e| e.y as usize * width + e.x as usize)
        .collect();
    for index in 0..count {
        let entry = &r.ground[&depth.to_string()][index];
        if crate::phantasmii_generation::is_shard(&entry.item) || crate::dreamwalking::dream_ground_item(&entry.item) {continue;}
        let cell = entry.y as usize * width + entry.x as usize;
        let f = r.floor().unwrap();
        let ichor = |f: &Floor, i: usize| {
            f.ichor
                .as_ref()
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(0.)
        };
        if f.tiles[cell] != "water" && ichor(f, cell) == 0. {
            continue;
        }
        if r.player.abilities.iter().any(|s| s == "etill-drown")
            && crate::half_here::origins(r)
                .iter()
                .any(|a| crate::dreamwalking::contact_plane(r,a) && (a.x - entry.x).hypot(a.y - entry.y) <= 1.5)
        {
            continue;
        }
        let to = if f
            .currents
            .as_ref()
            .and_then(|v| v.get(cell))
            .copied()
            .unwrap_or(0.)
            != 0.
            && ichor(f, cell) == 0.
        {
            current_destination_revision(f, cell, r.simulation_revision.value_or(29.0)).and_then(|to| {
                if !occupied.contains(&to) {Some(to)}
                else if r.simulation_revision.value_or(29.0)>=40.0 && crate::shore_currents::bank(f,to) {
                    crate::shore_currents::alternate_bank(f,cell,|i|!occupied.contains(&i))
                } else {None}
            })
        } else {
            if crate::rng::random(r) >= 0.25 {
                continue;
            }
            let f = r.floor().unwrap();
            let n = f.tiles.len();
            let neighbors = [
                if cell % width > 0 {
                    Some(cell - 1)
                } else {
                    None
                },
                if cell % width < width - 1 {
                    Some(cell + 1)
                } else {
                    None
                },
                cell.checked_sub(width),
                cell.checked_add(width).filter(|&i| i < n),
            ];
            let candidates: Vec<_> = neighbors
                .into_iter()
                .flatten()
                .filter(|&i| (f.tiles[i] == "water" || ichor(f, i) > 0.) && !occupied.contains(&i))
                .collect();
            if candidates.is_empty() {
                None
            } else {
                Some(candidates[(crate::rng::random(r) * candidates.len() as f64).floor() as usize])
            }
        };
        if let Some(to) = to {
            occupied.remove(&cell);
            occupied.insert(to);
            let e = &mut r.ground_at_mut(depth)[index];
            e.x = (to % width) as f64;
            e.y = (to / width) as f64;
        }
    }
    Ok(())
}
/// Pure typed-zero-aware hazard predicate; never normalizes hidden state.
pub fn creature_hazard_at(r: &Run, a: &crate::Actor, i: usize) -> bool {
    let Some(f) = r.floor() else { return false };
    let wet = matches!(
        f.tiles.get(i).map(String::as_str),
        Some("water" | "shallow-water")
    ) && f
        .ichor
        .as_ref()
        .and_then(|v| v.get(i))
        .copied()
        .unwrap_or(0.)
        == 0.;
    let has = |id: &str| a.abilities.iter().any(|s| s == id);
    if wet && !is_flying_in(r, a) && !has("etill-drown") && !has("enemy:beast:aquatic") {
        return true;
    }
    let env = f.environment.as_ref();
    let fire = env.is_some_and(|e| burns_on_contact(r, e, i))
        || (crate::terrain_runtime::enabled(r) && crate::terrain_runtime::hellfire_at(f,i))
        || f.permanent_fire
            .as_ref()
            .is_some_and(|v| v.contains(&(i as f64)));
    let gas_flame = env
        .and_then(|e| e.fire_kind.as_ref())
        .and_then(|v| v.get(i))
        .is_some_and(|v| matches!(v.as_deref(), Some("gas" | "gas-explosion")));
    let underwater = (i == a.y as usize * f.width() + a.x as usize && cond(a, "underwater") != 0.)
        || (has("enemy:beast:submerge")
            && f.tiles[i] == "water"
            && !f.ichor.as_ref().is_some_and(|v| v[i] != 0.));
    if !a.dead.value_or(false)
        && fire
        && !fire_immune(r, a)
        && !(gas_immune(a) && !crate::corpses::is_bastidion(a) && gas_flame)
        && !underwater
        && !(wet && !is_flying_in(r, a))
    {
        return true;
    }
    if f.razorleaves
        .as_ref()
        .and_then(|v| v.get(i))
        .copied()
        .unwrap_or(0.)
        > 0.
        && !has("enemy:beast:razorleaves")
    {
        return true;
    }
    let Some(e) = env else { return false };
    let gas = gas_kind_at(e, i);
    if gas == Some("steam") {
        return steam_can_scald(r, a);
    }
    if gas_immune(a) {
        return false;
    }
    match gas {
        Some("caustic" | "breather") => !has("grisha-entropy") && !has("enemy:pale:gas-immunity"),
        Some("paralysis" | "confusion") => true,
        Some("hallucinogenic") => crate::water_blood::enabled(r)
            && hallucination_affects(r, a) && !has("enemy:pale:gas-immunity"),
        _ => false,
    }
}
pub fn frozen_environment(r: &Run) -> Vec<bool> {
    let _timing_function = crate::timing::span("environment.freeze_mask");
    let f = r.floor().unwrap();
    let w = f.width();
    let h = f.height();
    let mut frozen = vec![false; f.tiles.len()];
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if !crate::dreamwalking::contact_plane(r,a) || a.dead.value_or(false) || !a.active.iter().any(|id| id == "dawel-reverse-entropy") {
            continue;
        }
        for y in (a.y as isize - 10).max(0)..=(a.y as isize + 10).min(h as isize - 1) {
            for x in (a.x as isize - 10).max(0)..=(a.x as isize + 10).min(w as isize - 1) {
                if (a.x - x as f64).hypot(a.y - y as f64) <= 10. {
                    frozen[y as usize * w + x as usize] = true;
                }
            }
        }
    }
    frozen
}
#[derive(Clone, Debug)]
pub struct ExplosionOptions {
    /// Authored immediate ability/attack blast, never propagated gas ignition.
    pub direct_impact: bool,
    pub radius: f64,
    pub source: Option<f64>,
    pub ignite_first: bool,
    pub gas_blast: bool,
    pub ignite_area: bool,
    pub audible: bool,
}
impl Default for ExplosionOptions {
    fn default() -> Self {
        Self {
            direct_impact: false,
            radius: 2.,
            source: None,
            ignite_first: false,
            gas_blast: false,
            ignite_area: true,
            audible: true,
        }
    }
}
/// Anonymous geometry belongs to the current terrain layer, never the player
/// whose real ID is zero. O(1) temporary state, no gameplay mutation or RNG.
fn terrain_origin(r: &Run, x: f64, y: f64) -> crate::Actor {
    let mut a = crate::Actor { id: -1., x, y, ..Default::default() };
    if crate::dreamwalking::enabled(r) {
        a.conditions.insert("phantasmiiPlane".into(), if crate::dreamwalking::context_dream(r) { 1. } else { 0. });
    }
    a
}
fn explosion_victims(
    r: &mut Run,
    x: f64,
    y: f64,
    options: &ExplosionOptions,
    pools: &mut std::collections::HashSet<i64>,
    shared: bool,
    source_snapshot: Option<&crate::Actor>,
) -> Result<(), String> {
    let victims: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.id)
        .collect();
    let origin = terrain_origin(r, x, y);
    for id in victims {
        let Some(a) = crate::engine::actor(r, id) else {
            continue;
        };
        let origin_actor=source_snapshot.or_else(||options.source.and_then(|id|crate::engine::actor(r,id)));
        if crate::dreamwalking::environment_context(r) {if !crate::dreamwalking::contact_plane(r,a) {continue;}} else if origin_actor.map_or(!crate::dreamwalking::contact_plane(r,a), |s|!crate::dreamwalking::same_plane(r,s,a)) {continue;}
        let owner = crate::half_here::owner_id(r, a) as i64;
        if options.gas_blast && gas_immune(a)
            || (a.x - x).hypot(a.y - y) > options.radius
            || !crate::vision::line_of_sight(r, &origin, a)
            || (shared && pools.contains(&owner))
        {
            continue;
        }
        pools.insert(owner);
        if a.abilities
            .iter()
            .any(|id| id == "anavani-nova" || id == "enemy:beast:explosion-immunity")
        {
            continue;
        }
        let damage = crate::combat::explosion_damage_for_run(r, a.max_hp);
        // Immediate blast impact only; persistent fire/gas ticks never dissolve it.
        if crate::revision50_nightmares::strike(r, id) { continue; }
        if let Some(source) = source_snapshot {
            let damage_fn = if options.direct_impact {
                crate::engine::damage_direct_with_source_snapshot
            } else { crate::engine::damage_environment_with_source_snapshot };
            damage_fn(r, id, damage, source, "explosion", false)
            .map_err(|e| e.to_string())?;
        } else {
            if options.direct_impact && options.source.is_some() {
                crate::engine::damage_direct(r, id, damage, options.source.unwrap(), "explosion", false)
            } else {
                crate::engine::damage_environment(r, id, damage, options.source, "explosion", false)
            }.map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
/// Ordinary blasts seed the same complete gas closure without advancing time.
/// Shared-owner pools suppress subsequent gas exposures, not ordinary body exposures.
pub fn explode(
    r: &mut Run,
    x: f64,
    y: f64,
    options: ExplosionOptions,
    hit_pools: Option<&mut std::collections::HashSet<i64>>,
) -> Result<(), String> {
    let snapshot = options
        .source
        .and_then(|id| crate::engine::actor(r, id))
        .cloned();
    explode_retained(r, x, y, options, hit_pools, snapshot.as_ref())
}
/// Retain attribution for a source endpoint removed by an earlier reactive callback.
pub fn explode_with_source_snapshot(
    r: &mut Run,
    x: f64,
    y: f64,
    options: ExplosionOptions,
    hit_pools: Option<&mut std::collections::HashSet<i64>>,
    source: &crate::Actor,
) -> Result<(), String> {
    explode_retained(r, x, y, options, hit_pools, Some(source))
}
fn explode_retained(
    r: &mut Run,
    x: f64,
    y: f64,
    options: ExplosionOptions,
    hit_pools: Option<&mut std::collections::HashSet<i64>>,
    source: Option<&crate::Actor>,
) -> Result<(), String> {
    let _timing_function = crate::timing::span("environment.explode");
    let owner = source
        .map(|a| {
            if crate::half_here::owner_id(r, a) == r.player.id {
                -2.
            } else {
                a.id
            }
        })
        .unwrap_or_else(|| source_id(r, options.source));
    let f = r.floor().ok_or("Missing floor")?;
    if x < 0.
        || y < 0.
        || x >= f.width() as f64
        || y >= f.height() as f64
        || !options.radius.is_finite()
        || options.radius < 0.
    {
        return Ok(());
    }
    if options.audible {
        crate::engine::log(r, "You hear an explosion!", "environment");
    }
    let animation_source=source.or_else(||options.source.and_then(|id|crate::engine::actor(r,id)));
    let animation_key=if options.direct_impact&&animation_source.is_some_and(|a|crate::abilities::endpoint_active(r,a,"anavani-nova")){"P57"}
        else if options.ignite_first&&animation_source.is_some_and(|a|a.abilities.iter().any(|s|s=="enemy:beast:detonate")){"E37"}else{"shared"};
    explosion_event(r,x,y,options.radius,animation_key,animation_source);
    let shared = options.gas_blast || hit_pools.is_some();
    let mut local = std::collections::HashSet::new();
    let pools = hit_pools.unwrap_or(&mut local);
    if !options.ignite_first {
        explosion_victims(r, x, y, &options, pools, shared, source)?;
    }
    if !options.ignite_area {
        if options.ignite_first {
            explosion_victims(r, x, y, &options, pools, shared, source)?;
        }
        return Ok(());
    }
    let f = r.floor().unwrap();
    let width = f.width();
    let height = f.height();
    let mut gas_cells = vec![];
    let origin = terrain_origin(r, x, y);
    for yy in (y - options.radius.ceil()).max(0.) as usize
        ..=(y + options.radius.ceil()).min(height as f64 - 1.) as usize
    {
        for xx in (x - options.radius.ceil()).max(0.) as usize
            ..=(x + options.radius.ceil()).min(width as f64 - 1.) as usize
        {
            let mut target = crate::Actor::default();
            target.x = xx as f64;
            target.y = yy as f64;
            if (x - xx as f64).hypot(y - yy as f64) > options.radius
                || !crate::vision::line_of_sight(r, &origin, &target)
            {
                continue;
            }
            let at = yy * width + xx;
            initialize(r.floor_mut().unwrap())?;
            let f = r.floor().unwrap();
            let e = f.environment.as_ref().unwrap();
            let explosive = e.gas_volume.as_ref().unwrap()[at] > 0.
                && matches!(
                    e.gas_type.as_ref().unwrap()[at].as_deref(),
                    Some("explosive" | "hydrogen")
                );
            let burning = e.fire_kind.as_ref().unwrap()[at].as_deref() == Some("plain")
                || ["grass", "tree"].contains(&f.tiles[at].as_str())
                || [&f.slime, &f.roots, &f.razorleaves]
                    .iter()
                    .any(|field| field.as_ref().is_some_and(|v| v[at] > 0.));
            ignite_owned(r, xx as f64, yy as f64, None, owner)?;
            if explosive {
                gas_cells.push(at)
            }
            let e = r.floor_mut().unwrap().environment.as_mut().unwrap();
            if e.fire[at] > 0.
                && !matches!(
                    e.fire_kind.as_ref().unwrap()[at].as_deref(),
                    Some("brazier" | "ichor")
                )
            {
                e.fire_kind.as_mut().unwrap()[at] = Some(
                    if explosive && burning {
                        "plain"
                    } else if options.gas_blast {
                        "gas-explosion"
                    } else {
                        "explosion"
                    }
                    .into(),
                );
            }
        }
    }
    if options.ignite_first {
        explosion_victims(r, x, y, &options, pools, shared, source)?;
    }
    if !gas_cells.is_empty() {
        let frozen = frozen_environment(r);
        let player = if owner == -2. { 1. } else { 0. };
        let mut adapter = crate::fire_adapter::FireAdapter::new(r, frozen)?;
        for at in gas_cells {
            adapter.ignite_blast(at, owner, player)
        }
        let bursts = adapter.resolve()?;
        drop(adapter);
        resolve_bursts(r, bursts, Some(pools), false)?;
    }
    Ok(())
}

/// O(R² * LOS) for a real blast radius R; detached visible footprint only.
/// Uses the very same radius and LOS predicate as damage admission, before
/// ignition/deaths alter geometry. No legacy held '+' or invented neighbors.
fn explosion_event(r: &Run, x: f64, y: f64, radius:f64,key:&str,source:Option<&crate::Actor>) {
    if crate::events::enabled(r) {
        let Some(f)=r.floor()else{return};let origin=terrain_origin(r,x,y);let mut cells=Vec::new();
        for yy in (y-radius.ceil()).max(0.)as usize..=(y+radius.ceil()).min(f.height()as f64-1.)as usize {
            for xx in (x-radius.ceil()).max(0.)as usize..=(x+radius.ceil()).min(f.width()as f64-1.)as usize {
                let mut target=origin.clone();target.x=xx as f64;target.y=yy as f64;
                if (x-target.x).hypot(y-target.y)<=radius&&crate::vision::line_of_sight(r,&origin,&target){cells.push(crate::events::EventPoint{x:target.x,y:target.y});}
            }
        }
        if let Some(a)=source.filter(|a|crate::events::actor_observed(r,a)) {crate::events::cue(r,a,key,"burst-ember",crate::events::Stage::Special,&cells);}
        else {crate::events::at(r,"shared","burst-ember",crate::events::Stage::Special,&cells);}
        return;
    }
    crate::events::emit(
        r,
        crate::events::PresentationEvent {
            kind: "burst".into(),
            from: None,
            to: crate::events::EventPoint { x, y },
            glyph: Some("人".into()),
            color: Some("#fff23a".into()),
            label: Some("Explosion".into()),
            depth: r.depth,
         animation: None, },
    );
}

/// Rebuild exact live-emitter field and preserve first source at overlapping cells.
pub fn refresh_razorleaves(r: &mut Run, ids: &[f64]) -> std::collections::HashMap<usize, f64> {
    let _timing_function = crate::timing::span("environment.razorleaves");
    let f = r.floor().unwrap();
    let w = f.width();
    let h = f.height();
    let n = f.tiles.len();
    if let Some(v) = r.floor_mut().unwrap().razorleaves.as_mut() {
        v.fill(0.)
    }
    let mut sources = std::collections::HashMap::new();
    for &id in ids {
        let Some(a) = crate::engine::actor(r, id).cloned() else {
            continue;
        };
        if !a.abilities.iter().any(|v| v == "enemy:beast:razorleaves") {
            continue;
        }
        r.floor_mut()
            .unwrap()
            .razorleaves
            .get_or_insert_with(|| vec![0.; n]);
        for y in (a.y as usize).saturating_sub(5)..=(a.y as usize + 5).min(h - 1) {
            for x in (a.x as usize).saturating_sub(5)..=(a.x as usize + 5).min(w - 1) {
                let i = y * w + x;
                let target = crate::Actor {
                    x: x as f64,
                    y: y as f64,
                    ..Default::default()
                };
                if ((a.x - target.x).powi(2) + (a.y - target.y).powi(2)).sqrt() <= 5.
                    && r.floor().unwrap().tiles[i] != "wall"
                    && crate::vision::line_of_sight(r, &a, &target)
                {
                    r.floor_mut().unwrap().razorleaves.as_mut().unwrap()[i] = 1.;
                    sources.entry(i).or_insert(id);
                }
            }
        }
    }
    sources
}

pub fn restore_altered_chasms(f: &mut Floor, active: bool) { restore_altered_chasms_revision(f, active, 29.); }
pub fn restore_altered_chasms_revision(f: &mut Floor, active: bool, revision: f64) {
    if active {
        return;
    }
    let Some(records) = f.altered_chasms.as_mut() else {
        return;
    };
    let mut done = vec![];
    for (key, record) in records.iter_mut() {
        record.age += 1.;
        if record.age >= 3. {
            done.push((key.clone(), record.tile.clone()));
        }
    }
    for (key, tile) in done {
        let i = key.parse::<usize>().expect("validated cell index");
        if f.tiles[i] == "chasm" {
            f.tiles[i] = tile;
            crate::ichor::enqueue_opening_revision(f, i, revision);
        }
        f.altered_chasms.as_mut().unwrap().shift_remove(&key);
        f.dirty = true;
    }
}

#[cfg(test)]
mod presentation_parity {
    #[test]
    fn dream_plane_physical_half_here_burst_keeps_owner_credit_without_changing_its_plane() {
        for previous in [false, true] {
            let mut r: crate::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            r.simulation_revision = 34.0.into();
            r.player.x = 20.; r.player.y = 20.; r.player.conditions.clear();
            r.player.abilities.push(crate::half_here::ID.into());
            r.player.active = vec![crate::half_here::ID.into(), "anavani-nova".into()];
            r.player.conditions.insert("halfHereId".into(), 90.);
            let mut clone = r.player.clone(); clone.id = 90.; clone.x = 12.; clone.y = 14.;
            clone.state = "ally".into(); clone.conditions.clear();
            clone.conditions.insert("halfHereClone".into(), 1.);
            clone.conditions.insert("halfHereOwner".into(), r.player.id);
            clone.conditions.insert("phantasmiiPlane".into(), 0.);
            clone.abilities.push("anavani-nova".into());
            let victim = r.actors_at_mut(1.).first_mut().unwrap();
            victim.x = 14.; victim.y = 14.; victim.hp = 100.; victim.max_hp = 100.;
            victim.conditions.clear(); victim.abilities.clear();
            let target = victim.id;
            r.actors_at_mut(1.).push(clone);
            r.dreamwalking = crate::dreamwalking::State {
                active: true, depth: 1., resolving_environment: previous, ..Default::default()
            }.into();
            let w = r.floor().unwrap().width();
            for x in 12..=14 { r.floor_mut().unwrap().tiles[14 * w + x] = "floor".into(); }
            crate::dreamwalking::ensure_layer(&mut r);
            let owner = super::source_id(&r, Some(90.));
            assert_eq!(owner, -2., "Half Here retains player attribution");
            super::resolve_bursts(&mut r, vec![crate::fire_adapter::Burst {
                at: 14 * w + 12, radius: 2, owner, player: 1., reaction: 1, detonates: true,
            }], None, false).unwrap();
            assert!(crate::engine::actor(&r, target).unwrap().hp < 100.);
            assert_eq!(crate::dreamwalking::environment_context(&r), previous);
            assert!(!crate::dreamwalking::context_dream(&r));
        }
    }
    #[test]
    fn dream_plane_projection_cannot_consume_a_physical_bodys_gas_blast_pool() {
        let mut r: crate::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.player.x = 14.; r.player.y = 14.; r.player.hp = 100.; r.player.max_hp = 100.;
        r.player.conditions.clear(); r.player.active.clear(); r.player.abilities.clear();
        let mut body = r.player.clone(); body.id = 90.; body.state = "ally".into();
        body.conditions.insert("phantasmiiBody".into(), 1.);
        body.conditions.insert("halfHereOwner".into(), r.player.id);
        r.actors_at_mut(1.).clear(); r.actors_at_mut(1.).push(body);
        r.dreamwalking = crate::dreamwalking::State {
            active: true, depth: 1., body_id: Some(90.), resolving_environment: true, ..Default::default()
        }.into();
        let w = r.floor().unwrap().width();
        for x in 12..=14 { r.floor_mut().unwrap().tiles[14 * w + x] = "floor".into(); }
        crate::dreamwalking::ensure_layer(&mut r);
        super::resolve_bursts(&mut r, vec![crate::fire_adapter::Burst {
            at: 14 * w + 12, radius: 2, owner: -1., player: 0., reaction: 1, detonates: true,
        }], None, false).unwrap();
        assert!(r.player.hp < 100.);
        assert_eq!(crate::engine::actor(&r, 90.).unwrap().hp, r.player.hp);
        assert!(crate::dreamwalking::active(&r));
    }
    #[test]
    fn dream_plane_explosion_geometry_uses_the_blast_layer_not_the_players_plane() {
        for normal_wall in [false, true] {
            for mode in 0..3 {
                let mut r: crate::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
                r.simulation_revision = 34.0.into();
                r.dreamwalking = crate::dreamwalking::State { active: true, depth: 1., ..Default::default() }.into();
                r.player.x = 20.; r.player.y = 20.;
                let f = r.floor_mut().unwrap();
                f.environment = crate::Field::Missing;
                let w = f.width();
                let wall = 14 * w + 13;
                let victim_cell = 14 * w + 14;
                for x in 10..=16 { f.tiles[14 * w + x] = "floor".into(); }
                f.tiles[wall] = if normal_wall { "wall" } else { "floor" }.into();
                crate::dreamwalking::with_plane(&mut r, true, |r| {
                    r.floor_mut().unwrap().tiles[wall] = if normal_wall { "floor" } else { "wall" }.into();
                });
                let victim = r.actors_at_mut(1.).first_mut().unwrap();
                victim.x = 14.; victim.y = 14.; victim.hp = 100.; victim.max_hp = 100.;
                victim.abilities.clear(); victim.conditions.clear();
                let id = victim.id;
                if mode == 2 {
                    r.dreamwalking.as_mut().unwrap().resolving_environment = true;
                    super::resolve_bursts(&mut r, vec![crate::fire_adapter::Burst {
                        at: 14 * w + 12, radius: 2, owner: -1., player: 0., reaction: 1, detonates: true,
                    }], None, false).unwrap();
                } else {
                    super::explode(&mut r, 12., 14., super::ExplosionOptions {
                        radius: 2., ignite_area: mode == 1, audible: false, ..Default::default()
                    }, None).unwrap();
                }
                let hp = crate::engine::actor(&r, id).unwrap().hp;
                assert_eq!(hp < 100., !normal_wall, "mode {mode}, normal wall {normal_wall}");
                if mode == 1 {
                    let burned = r.floor().unwrap().environment.as_ref()
                        .is_some_and(|e| e.fire[victim_cell] > 0.);
                    assert_eq!(burned, !normal_wall, "ignition uses physical LOS");
                }
            }
        }
    }
    #[test]
    fn explosion_presentation_matches_reference() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/explosion-v48.json")).unwrap();
        for f in fixtures.as_array().unwrap() {
            let mut r: crate::Run = serde_json::from_value(f["initial"].clone()).unwrap();
            let (x, y) = (r.player.x, r.player.y);
            let options = super::ExplosionOptions {
                source: Some(r.player.id),
                ignite_first: f["igniteFirst"].as_bool().unwrap(),
                ignite_area: f["igniteArea"].as_bool().unwrap(),
                ..Default::default()
            };
            crate::events::begin();
            if let Some(source) = f.get("sourceSnapshot") {
                let source: crate::Actor = serde_json::from_value(source.clone()).unwrap();
                let options = super::ExplosionOptions {
                    source: Some(source.id),
                    ..options
                };
                super::explode_with_source_snapshot(&mut r, x, y, options, None, &source).unwrap();
            } else {
                super::explode(&mut r, x, y, options, None).unwrap();
            }
            let actual = crate::events::finish();
            let expected: Vec<crate::events::PresentationEvent> =
                serde_json::from_value(f["events"].clone()).unwrap();
            assert_eq!(
                actual, expected,
                "{} ignition {} area {}",
                f["name"], f["igniteFirst"], f["igniteArea"]
            );
        }
    }
}

/// Pilot selected-state reader: Normal O(1), Fast O(F), plus gas-name bytes; no grid serialization.
/// Four primary columns only: legacy channel views, provenance, vents, fire and
/// RNG are deliberately outside this measurement pilot's coverage. Numeric bits
/// preserve signed zero/NaN; field presence is distinct from a null gas cell.
pub(crate) fn capture_pilot_gas_cell(r: &Run, depth_bits: u64, index: usize) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let depth = f64::from_bits(depth_bits);
    if !depth.is_finite() || depth < 1. || depth.fract() != 0. || depth > usize::MAX as f64 {
        return Err("unsupported gas floor depth".into());
    }
    let f = if crate::fast_mode::enabled(r) {
        r.floors.iter().find(|f| f.depth.to_bits() == depth_bits)
    } else {
        let slot = (depth as usize).checked_sub(1).ok_or("invalid gas floor depth")?;
        r.floors.get(slot).filter(|f| f.depth.to_bits() == depth_bits)
    }.ok_or("physical gas floor address mismatch")?;
    if index >= f.tiles.len() { return Err("gas cell outside floor".into()); }
    let e = match &f.environment {
        Field::Missing => return Ok(json!({"environment":"missing"})),
        Field::Null => return Ok(json!({"environment":"null"})),
        Field::Value(e) => e,
    };
    let kind = match &e.gas_type {
        Field::Missing => json!({"presence":"missing"}),
        Field::Null => json!({"presence":"null"}),
        Field::Value(v) => json!({"presence":"value","value":v.get(index).ok_or("gas type shape mismatch")?}),
    };
    Ok(json!({"gasType":kind,
        "gasVolume":capture_pilot_numeric_cell(&e.gas_volume,index)?,
        "gasOwner":capture_pilot_numeric_cell(&e.gas_owner,index)?,
        "gasPlayer":capture_pilot_numeric_cell(&e.gas_player,index)?}))
}
/// O(1), selected numeric cell only; no vector copy or formatting of f64 payload.
fn capture_pilot_numeric_cell(field: &Field<Vec<f64>>, index: usize) -> Result<serde_json::Value,String> {
    use serde_json::json;
    Ok(match field {
        Field::Missing => json!({"presence":"missing"}),
        Field::Null => json!({"presence":"null"}),
        Field::Value(v) => json!({"presence":"value","bits":v.get(index).ok_or("gas numeric shape mismatch")?.to_bits()}),
    })
}
/// OUTSIDE timed collection: O(all physical floor cells + emitted tuple bytes).
/// Independent oracle for stable indexed floors. Missing/relocated floors or bad
/// column shapes are unsupported (the report must be checked, not certified by
/// this oracle). No gameplay use and no writes to canonical state.
pub(crate) fn capture_pilot_gas_oracle(opening: &Run, committed: &Run) -> Vec<crate::capture_pilot::Change> {
    let mut out=Vec::new();
    for f in &committed.floors {
        let depth_bits=f.depth.to_bits();
        for index in 0..f.tiles.len() {
            if let (Ok(old),Ok(value))=(capture_pilot_gas_cell(opening,depth_bits,index),capture_pilot_gas_cell(committed,depth_bits,index)) {
                if old!=value {out.push(crate::capture_pilot::Change::Gas{depth_bits,index,value});}
            }
        }
    }
    out
}

/// Borrowed pilot field state: absent, explicit null, or one selected cell.
#[derive(Debug,PartialEq)]
pub(crate) enum PilotCell<T>{Missing,Null,Value(T)}
/// No owned strings or JSON trees; numeric cells carry exact IEEE bits.
#[derive(Debug,PartialEq)]
pub(crate) enum PilotGas<'a>{Missing,Null,Value{kind:PilotCell<Option<&'a str>>,volume:PilotCell<u64>,owner:PilotCell<u64>,player:PilotCell<u64>}}
impl PilotGas<'_>{
    /// O(gas-name bytes), materializes ONLY a changed final diagnostic tuple.
    pub(crate) fn compact(&self)->serde_json::Value{
        use serde_json::json;
        match self {Self::Missing=>json!([0]),Self::Null=>json!([1]),Self::Value{kind,volume,owner,player}=>{
            let kind=match kind{PilotCell::Missing=>json!([0]),PilotCell::Null=>json!([1]),PilotCell::Value(v)=>json!([2,v])};
            json!([2,kind,pilot_number_compact(volume),pilot_number_compact(owner),pilot_number_compact(player)])
        }}
    }
}
/// O(1), compact one selected numeric slot, exact integer bits.
fn pilot_number_compact(v:&PilotCell<u64>)->serde_json::Value{use serde_json::json;match v{PilotCell::Missing=>json!([0]),PilotCell::Null=>json!([1]),PilotCell::Value(bits)=>json!([2,bits])}}
/// O(1) lookup and borrow, no JSON/string allocation on valid reads.
fn pilot_number_borrow(v:&Field<Vec<f64>>,index:usize)->Result<PilotCell<u64>,String>{Ok(match v{Field::Missing=>PilotCell::Missing,Field::Null=>PilotCell::Null,Field::Value(v)=>PilotCell::Value(v.get(index).ok_or("gas numeric shape mismatch")?.to_bits())})}
/// Normal O(1), Fast O(F) floor lookup, then O(1) columns; string comparison O(name bytes).
/// Intentionally independent of the frozen verbose oracle's implementation.
pub(crate) fn capture_pilot_gas_borrowed(r:&Run,depth_bits:u64,index:usize)->Result<PilotGas<'_>,String>{
    let depth=f64::from_bits(depth_bits);
    if !depth.is_finite()||depth<1.||depth.fract()!=0.||depth>usize::MAX as f64{return Err("unsupported gas floor depth".into());}
    let f=if crate::fast_mode::enabled(r){
        r.floors.iter().find(|f|f.depth.to_bits()==depth_bits)
    }else{
        let slot=(depth as usize).checked_sub(1).ok_or("invalid gas floor depth")?;
        r.floors.get(slot).filter(|f|f.depth.to_bits()==depth_bits)
    }.ok_or("physical gas floor address mismatch")?;
    if index>=f.tiles.len(){return Err("gas cell outside floor".into());}
    let e=match &f.environment{Field::Missing=>return Ok(PilotGas::Missing),Field::Null=>return Ok(PilotGas::Null),Field::Value(e)=>e};
    let kind=match &e.gas_type{Field::Missing=>PilotCell::Missing,Field::Null=>PilotCell::Null,Field::Value(v)=>PilotCell::Value(v.get(index).ok_or("gas type shape mismatch")?.as_deref())};
    Ok(PilotGas::Value{kind,volume:pilot_number_borrow(&e.gas_volume,index)?,owner:pilot_number_borrow(&e.gas_owner,index)?,player:pilot_number_borrow(&e.gas_player,index)?})
}

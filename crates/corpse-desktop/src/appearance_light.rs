//! Source-v48 visual light field. Inputs are already detached knowledge only.
//! No Run, visibility query, simulation mutation, or gameplay randomness.
use crate::{appearance_profile::Profile, color::Light};
use corpse_core::vision::{ObservedCell, ObservedLightSource, TerrainObservation};
use std::collections::{HashMap, HashSet};
fn clamp(n: f64) -> f64 {
    n.clamp(0., 1.)
}
fn linear(v: u8) -> f64 {
    let v = v as f64 / 255.;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
pub fn animated(profile: &Profile) -> bool {
    profile.number("lighting.flickerStrength") > 0.
        || [
            "torch",
            "brazier",
            "fire",
            "burning-liquid",
            "magic",
            "neutral",
        ]
        .iter()
        .any(|s| profile.number(&format!("lighting.sources.{s}.flickerStrength")) > 0.)
}
pub fn flicker(x: i32, y: i32, time: f64, frequency: f64, smoothness: f64) -> f64 {
    let phase = time.max(0.) * frequency.max(0.) / 1000.;
    let tick = phase.floor();
    let fraction = phase - tick;
    let smooth = fraction * fraction * (3. - 2. * fraction);
    let blend = fraction * (1. - clamp(smoothness)) + smooth * clamp(smoothness);
    // JS bitwise hashing reduces its salt to u32; presentation time never reseeds gameplay.
    let tick = tick.rem_euclid(4_294_967_296.) as u32;
    (crate::color::cosmetic_hash(x, y, tick) * (1. - blend)
        + crate::color::cosmetic_hash(x, y, tick.wrapping_add(1)) * blend)
        * 2.
        - 1.
}
fn category<'a>(
    source: &'a ObservedLightSource,
    cells: &[Option<&ObservedCell>],
    width: usize,
) -> &'a str {
    if source.kind == "torch" || source.kind == "brazier" {
        return &source.kind;
    }
    if source.kind == "fire" {
        let burning = source.origin_known
            && source.x >= 0.
            && source.y >= 0.
            && cells
                .get(source.y as usize * width + source.x as usize)
                .and_then(|c| *c)
                .is_some_and(|c| {
                    c.current
                        && c.hazards
                            .as_ref()
                            .is_some_and(|h| h.fire.as_ref().is_some_and(|f| f == "ichor"))
                });
        return if burning { "burning-liquid" } else { "fire" };
    }
    match source.color_role.as_str() {
        "torch" => "torch",
        "fire" => "fire",
        "magic" => "magic",
        _ => "neutral",
    }
}
fn blocked(cell: &ObservedCell) -> bool {
    matches!(cell.tile.as_deref(), Some("wall" | "tree")) || cell.door_open == Some(false)
}
fn known<'a>(
    cells: &[Option<&'a ObservedCell>],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
) -> Option<&'a ObservedCell> {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return None;
    }
    cells
        .get(y as usize * width + x as usize)
        .and_then(|c| *c)
        .filter(|c| c.current && c.tile.is_some())
}
fn known_ray(
    source: &ObservedLightSource,
    x: i32,
    y: i32,
    cells: &[Option<&ObservedCell>],
    width: usize,
    height: usize,
) -> bool {
    let (mut sx, mut sy) = (source.x as i32, source.y as i32);
    let (dx, dy) = ((x - sx).abs(), (y - sy).abs());
    let step_x = if sx < x { 1 } else { -1 };
    let step_y = if sy < y { 1 } else { -1 };
    let mut error = dx - dy;
    loop {
        let Some(cell) = known(cells, width, height, sx, sy) else {
            return false;
        };
        if sx == x && sy == y {
            return true;
        }
        if (sx != source.x as i32 || sy != source.y as i32) && blocked(cell) {
            return false;
        }
        let twice = error * 2;
        if twice > -dy && twice < dx {
            for (ax, ay) in [(sx + step_x, sy), (sx, sy + step_y)] {
                if known(cells, width, height, ax, ay).is_none_or(blocked) {
                    return false;
                }
            }
        }
        if twice > -dy {
            error -= dy;
            sx += step_x;
        }
        if twice < dx {
            error += dx;
            sy += step_y;
        }
    }
}
fn curve(value: f64, curve: &str) -> f64 {
    match curve {
        "linear" => value,
        "quadratic" => value * value,
        _ => value.powi(3) * (value * (value * 6. - 15.) + 10.),
    }
}
/// Default radius: O(N + F + S) expected time and O(N + S + max footprint) memory.
/// Expanded radius: additionally O(S*N*max(width,height)) worst-case conservative rays.
/// F = detached footprint entries, S = source records, N = observed grid cells.
pub fn light_field(
    observation: &TerrainObservation,
    profile: &Profile,
    presentation_ms: f64,
) -> Vec<Light> {
    let _span=corpse_core::timing::span("desktop.observe.appearance_light");
    let n = observation.width * observation.height;
    let mut out = vec![Light::ambient(0.); n];
    let mut energy = vec![[0f32; 3]; n];
    let mut wall = vec![[0f32; 3]; n];
    let mut maximum = vec![0f32; n];
    let mut darkness = vec![0f32; n];
    let mut potion_darkness = vec![0f32; n];
    let mut cells = vec![None; n];
    // Scalar exposure and source hue are deliberately independent.
    let mut tint = vec![[0.; 3]; n];
    let mut tint_weight = vec![0.; n];
    let mut native_max = vec![0.; n];
    let mut visual_max = vec![0.; n];
    for cell in &observation.cells {
        if cell.index < n {
            cells[cell.index] = Some(cell);
            out[cell.index].ambient = (cell.ambient.unwrap_or(0.)
                + (profile.number("lighting.ambient") - 1.) * 100.)
                as f32 as f64;
            out[cell.index].illumination = cell.illumination.unwrap_or(0.);
            maximum[cell.index] = cell.ambient.unwrap_or(0.).max(0.) as f32;
            let ambient = cell.ambient.unwrap_or(0.).max(0.);
            tint[cell.index] = [ambient; 3];
            tint_weight[cell.index] = ambient;
            native_max[cell.index] = ambient;
            visual_max[cell.index] = ambient;
            out[cell.index].display = cell.direct.then(|| crate::color::light_ui::DisplayLight::neutral(
                cell.illumination.unwrap_or(0.) + (profile.number("lighting.ambient") - 1.) * 100.));
        }
    }
    let mut family = Vec::<&ObservedLightSource>::new();
    let mut family_index = HashMap::new();
    let mut sources = Vec::new();
    for source in &observation.light_sources {
        if source.color_role == "darkness" {
            for (&i, &strength) in source.cells.iter().zip(&source.strengths) {
                if i < n {
                    darkness[i] = (darkness[i] as f64 + strength) as f32;
                    if source.kind == "potion-darkness" {
                        potion_darkness[i] = (potion_darkness[i] as f64 + strength) as f32;
                    }
                }
            }
            continue;
        }
        if source.color_role != "fire" && source.color_role != "torch" {
            sources.push(source);
            continue;
        }
        if let Some(&i) = family_index.get(&source.emission_group) {
            let previous: &ObservedLightSource = family[i];
            if source.power > previous.power
                || (source.power == previous.power && source.kind != "fire")
            {
                family[i] = source;
            }
        } else {
            family_index.insert(source.emission_group, family.len());
            family.push(source);
        }
    }
    sources.extend(family);
    for source in sources {
        let prefix = format!(
            "lighting.sources.{}",
            category(source, &cells, observation.width)
        );
        let value = |field: &str| profile.number(&format!("{prefix}.{field}"));
        let color = profile.rgb(&format!("{prefix}.color")).map(linear);
        let hue_key = format!("{prefix}.color");
        let hue = if profile.is_default(&hue_key) {
            match category(source, &cells, observation.width) {
                "torch" | "brazier" | "fire" | "burning-liquid" => crate::color::light_ui::WARM,
                "neutral" => [255; 3],
                _ => profile.rgb(&hue_key),
            }
        } else { profile.rgb(&hue_key) };
        let radius = profile.number("lighting.visualRadius") * value("visualRadius");
        let falloff = profile.number("lighting.falloff") * value("falloff");
        let noise = flicker(
            source.x as i32,
            source.y as i32,
            presentation_ms,
            profile.number("lighting.flickerFrequency") * value("flickerFrequency"),
            profile.number("lighting.flickerSmoothness") * value("flickerSmoothness"),
        );
        let flicker_gain = (1.
            + (profile.number("lighting.flickerStrength") + value("flickerStrength")) * noise)
            .max(0.);
        let gain = profile.number("lighting.intensity")
            * value("intensity")
            * profile.number("lighting.glow")
            * value("glow")
            * flicker_gain;
        let source_curve = profile.text(&format!("{prefix}.curve"));
        let curve_name = if source_curve == "smootherstep" {
            profile.text("lighting.falloffCurve")
        } else {
            source_curve
        };
        let floor_response = profile.number("lighting.floorResponse") * value("floorResponse");
        let wall_response = profile.number("lighting.wallResponse") * value("wallResponse");
        let blending = profile.text("lighting.blending");
        let mut paint = |i: usize, strength: f64, expanded: bool| {
            if i >= n {
                return;
            }
            let original = clamp(strength / source.power.max(1.));
            let proportion = if expanded || radius == 1. {
                original
            } else if radius <= 0. {
                0.
            } else {
                clamp(1. - (1. - original) / radius)
            };
            let intensity = curve(proportion.powf(falloff), curve_name)
                * (source.power / 100.).clamp(0., 1.5)
                * gain;
            for channel in 0..3 {
                let contribution = color[channel] * intensity;
                let combine = |previous: f32, response: f64| {
                    let previous = previous as f64;
                    (previous
                        + contribution
                            * response
                            * if blending == "screen" {
                                1. - clamp(previous)
                            } else {
                                1.
                            }) as f32
                };
                energy[i][channel] = combine(energy[i][channel], floor_response);
                wall[i][channel] = combine(wall[i][channel], wall_response);
            }
            maximum[i] = (maximum[i] as f64).max(strength) as f32;
            // Preserve opt-in source intensity/glow/flicker/radius/falloff and
            // floor/wall response as a presentation-only departure from the
            // default scalar calibration. Compare against the same native source
            // with untouched controls; defaults produce exactly zero adjustment.
            let reference = curve(original, "smootherstep") * (source.power / 100.).clamp(0., 1.5);
            let response = if cells[i].is_some_and(|c| matches!(c.tile.as_deref(), Some("wall" | "tree"))) {
                wall_response
            } else { floor_response };
            let visual_strength = if reference > 0. {
                strength * (intensity / reference) * response
            } else { 0. };
            if !expanded { native_max[i] = native_max[i].max(strength); }
            visual_max[i] = visual_max[i].max(visual_strength);
            let weight = intensity.max(0.) * 100.;
            tint_weight[i] += weight;
            for channel in 0..3 { tint[i][channel] += hue[channel] as f64 / 255. * weight; }
        };
        let footprint: HashSet<usize> = source.cells.iter().copied().collect();
        for (&i, &strength) in source.cells.iter().zip(&source.strengths) {
            paint(i, strength, false);
        }
        // Never extend unknown/redacted source origins or raycast remembered geometry.
        if source.origin_known
            && radius > 1.
            && source.falloff > 0.
            && known(
                &cells,
                observation.width,
                observation.height,
                source.x as i32,
                source.y as i32,
            )
            .is_some()
        {
            let reach = (observation.width.max(observation.height) as f64)
                .min(source.power / source.falloff * radius);
            let left = (source.x - reach).floor().max(0.) as i32;
            let right = (source.x + reach).ceil().min(observation.width as f64 - 1.) as i32;
            let top = (source.y - reach).floor().max(0.) as i32;
            let bottom = (source.y + reach)
                .ceil()
                .min(observation.height as f64 - 1.) as i32;
            for y in top..=bottom {
                for x in left..=right {
                    let i = y as usize * observation.width + x as usize;
                    if footprint.contains(&i)
                        || known(&cells, observation.width, observation.height, x, y).is_none()
                    {
                        continue;
                    }
                    let distance = (x as f64 - source.x).hypot(y as f64 - source.y);
                    if distance >= reach
                        || !known_ray(source, x, y, &cells, observation.width, observation.height)
                    {
                        continue;
                    }
                    paint(i, source.power * (1. - distance / reach), true);
                }
            }
        }
    }
    for i in 0..n {
        let reduction = clamp(1. - darkness[i] as f64 / (maximum[i] as f64 + 100.));
        // Positive illumination historically affected glow/distance but not the
        // near material base. Apply the same bounded darkness response to that
        // base, only for the new potion mask; ordinary/aura lighting is unchanged.
        out[i].ambient_attenuation = clamp(1. - potion_darkness[i] as f64 / (maximum[i] as f64 + 100.));
        let cap = profile.number("lighting.maximumIllumination");
        for field in [&mut energy, &mut wall] {
            let peak = field[i].into_iter().fold(0f32, f32::max) as f64;
            let exposure = if cap <= 0. {
                0.
            } else if profile.text("lighting.blending") == "soft" {
                reduction / (1. + peak * (1. / cap))
            } else {
                reduction * (cap / peak.max(0.000001)).min(1.)
            };
            for channel in 0..3 {
                field[i][channel] = (field[i][channel] as f64 * exposure) as f32;
            }
        }
        let display_illumination = out[i].illumination + (profile.number("lighting.ambient") - 1.) * 100.
            + (visual_max[i] - native_max[i]);
        if let Some(display) = &mut out[i].display {
            display.multiplier = crate::color::light_ui::multiplier(display_illumination);
            if tint_weight[i] > 0. { display.tint = tint[i].map(|v| v / tint_weight[i]); }
        }
        out[i].energy = energy[i].map(|v| v as f64);
        out[i].wall_energy = wall[i].map(|v| v as f64);
    }
    out
}

//! Profile-parameterized v48 color stages. Detached values only.
use crate::{
    appearance_profile::Profile,
    color::{Context, Knowledge, Light},
    render::Rgb,
};
fn clamp(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0., 1.)
    } else {
        0.
    }
}
fn rgb(v: [f64; 3]) -> Rgb {
    v.map(|x| x.clamp(0., 255.).round() as u8)
}
pub fn hex(s: &str) -> Rgb {
    let s = s.trim_start_matches('#');
    let owned;
    if s.len() == 3 {
        owned = s.chars().flat_map(|c| [c, c]).collect::<String>();
        return hex(&owned);
    }
    let n = u32::from_str_radix(s, 16).unwrap_or(0);
    [(n >> 16) as u8, (n >> 8) as u8, n as u8]
}
pub fn cosmetic_seed(seed: &str, depth: u32) -> u32 {
    let mut n = 2166136261u32;
    for c in seed.chars() {
        let code = if c as u32 > 0xffff {
            0xd800 + ((c as u32 - 0x10000) >> 10)
        } else {
            c as u32
        };
        n = (n ^ code).wrapping_mul(16777619);
    }
    n ^ depth.wrapping_mul(0x9e3779b1)
}
pub fn cosmetic_hash(x: i32, y: i32, salt: u32) -> f64 {
    let mut v = (x.wrapping_add(37) as u32).wrapping_mul(374761393)
        ^ (y.wrapping_add(71) as u32).wrapping_mul(668265263)
        ^ salt;
    v = (v ^ (v >> 13)).wrapping_mul(1274126177);
    (v ^ (v >> 16)) as f64 / 4294967295.
}
fn smooth(v: f64) -> f64 {
    v * v * (3. - 2. * v)
}
pub fn noise(x: f64, y: f64, seed: u32, scale: f64) -> f64 {
    let left = (x / scale).floor() as i32;
    let top = (y / scale).floor() as i32;
    let fx = smooth(x / scale - left as f64);
    let fy = smooth(y / scale - top as f64);
    let upper =
        cosmetic_hash(left, top, seed) * (1. - fx) + cosmetic_hash(left + 1, top, seed) * fx;
    let lower = cosmetic_hash(left, top + 1, seed) * (1. - fx)
        + cosmetic_hash(left + 1, top + 1, seed) * fx;
    (upper * (1. - fy) + lower * fy) * 2. - 1.
}
pub fn vary(p: &Profile, fg: Rgb, bg: Rgb, tile: &str, x: f64, y: f64, seed: u32) -> (Rgb, Rgb) {
    vary_settings(
        fg,
        bg,
        tile,
        x,
        y,
        seed,
        p.number(&format!("terrain.{tile}.brightnessVariation")),
        p.number(&format!("terrain.{tile}.colorVariation")),
    )
}
pub fn vary_settings(
    fg: Rgb,
    bg: Rgb,
    tile: &str,
    x: f64,
    y: f64,
    seed: u32,
    brightness_variation: f64,
    color_variation: f64,
) -> (Rgb, Rgb) {
    let patch = noise(x, y, seed, 10.) * 0.9 + noise(x, y, seed ^ 0x51ed270b, 5.) * 0.1;
    let hue = noise(x, y, seed ^ 0x27d4eb2f, 9.);
    let water = matches!(tile, "water" | "shallow-water");
    let plant = matches!(tile, "grass" | "tree");
    let void = matches!(tile, "chasm" | "pit");
    let amp = if void {
        0.6
    } else if plant {
        2.4
    } else if water {
        2.6
    } else {
        1.4
    };
    let shift = if void {
        [1., 0.15, -0.9]
    } else if water {
        [-0.8, 0.3, 1.2]
    } else if plant {
        [-0.5, 1.1, -0.5]
    } else {
        [0.6, 0.1, -0.5]
    };
    (
        rgb(std::array::from_fn(|i| {
            fg[i] as f64
                + patch * amp * 1.4 * brightness_variation
                + hue * shift[i] * color_variation
        })),
        rgb(std::array::from_fn(|i| {
            bg[i] as f64 + patch * amp * brightness_variation + hue * shift[i] * color_variation
        })),
    )
}
pub fn to_linear(v: f64) -> f64 {
    let v = clamp(v / 255.);
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
pub fn from_linear(v: f64) -> f64 {
    let v = clamp(v);
    255. * if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn linear(c: Rgb) -> [f64; 3] {
    c.map(|v| to_linear(v as f64))
}
fn from_channels(c: [f64; 3]) -> Rgb {
    rgb(c.map(from_linear))
}
pub fn luminance(c: Rgb) -> f64 {
    let c = linear(c);
    c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
}
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
pub fn composite(bg: Rgb, fg: Rgb, t: f64) -> Rgb {
    let t = clamp(t);
    rgb(std::array::from_fn(|i| {
        bg[i] as f64 * (1. - t) + fg[i] as f64 * t
    }))
}
fn desaturate(c: Rgb, t: f64) -> Rgb {
    let gray = c[0] as f64 * 0.2126 + c[1] as f64 * 0.7152 + c[2] as f64 * 0.0722;
    rgb(c.map(|v| v as f64 * (1. - t) + gray * t))
}
pub fn memory(p: &Profile, c: Rgb) -> Rgb {
    rgb(
        desaturate(c, 1. - 0.24 * p.number("fog.rememberedSaturation"))
            .map(|v| v as f64 * 0.5 * p.number("fog.rememberedBrightness")),
    )
}
pub fn telepathic(p: &Profile, c: Rgb, bg: bool) -> Rgb {
    let luma = c[0] as f64 * 0.22 + c[1] as f64 * 0.56 + c[2] as f64 * 0.22;
    let mut c = rgb(if bg {
        [luma * 0.3, luma * 0.55, luma * 1.18 + 9.]
    } else {
        [luma * 0.52 + 20., luma * 0.76 + 27., luma * 0.95 + 45.]
    });
    if bg {
        c[1] = c[1].max(c[0].saturating_add(1));
        c[2] = c[2].max(c[1].saturating_add(1));
    }
    let c = grade(c, p.number("fog.telepathyIntensity"), 1., 1.);
    composite(
        c,
        p.rgb("fog.telepathyColor"),
        if p.text("fog.telepathyColor") == "#87b7f3" {
            0.
        } else {
            0.35
        },
    )
}
pub fn environment(
    _p: &Profile,
    c: Rgb,
    bg: bool,
    light: Light,
    emissive: bool,
    tile: &str,
    response: f64,
) -> Rgb {
    let light = light.baseline();
    let base = linear(c);
    let shade =
        1. + light.ambient.min(light.illumination).min(0.) / 100. * if bg { 0.52 } else { 0.28 };
    let void = bg && matches!(tile, "chasm" | "pit");
    let water = bg && matches!(tile, "water" | "shallow-water");
    let wall = bg && matches!(tile, "wall" | "tree");
    let ordinary = matches!(tile, "floor" | "wall");
    let strength = if void {
        0.005
    } else if water {
        0.045
    } else if wall {
        0.17
    } else if ordinary {
        if bg {
            0.13
        } else if tile == "floor" {
            0.035
        } else {
            0.095
        }
    } else if bg {
        0.42
    } else if emissive {
        0.025
    } else {
        0.14
    };
    let energy = if matches!(tile, "wall" | "tree") {
        light.wall_energy
    } else {
        light.energy
    };
    let gray = energy[0] * 0.2126 + energy[1] * 0.7152 + energy[2] * 0.0722;
    from_channels(std::array::from_fn(|i| {
        let v = base[i];
        let emission = gray * 0.08 + energy[i] * 0.92;
        let exposed = if water {
            v * 0.48
        } else if bg && tile == "wall" {
            v * 1.55
        } else if ordinary || matches!(tile, "water" | "shallow-water") {
            v
        } else if bg && !void {
            v * 1.2 / (1. + v * 0.6)
        } else if !bg && !emissive {
            v * 1.16 / (1. + v * 0.16)
        } else {
            v
        };
        exposed * if emissive { 1. } else { shade * light.ambient_attenuation }
            + emission * strength * response * if bg { 0.65 + v } else { 0.3 + v * 0.7 }
    }))
}
pub fn shadow_gamma(p: &Profile, distance: f64, light: Light, tile: &str) -> f64 {
    let light = light.baseline();
    if distance <= 3. {
        return 1.;
    }
    let energy = if matches!(tile, "wall" | "tree") {
        light.wall_energy
    } else {
        light.energy
    };
    let peak = energy.into_iter().fold(0.0f64, f64::max);
    let bright = 0.0f64.max(light.illumination).max(light.ambient).max(
        peak * 160.
            * p.value(&format!("terrain.{tile}.lightingResponse"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.),
    );
    1. + p.number("lighting.distanceDarkening")
        * smooth(clamp((distance - 3.) / 21.))
        * (1. - smooth(clamp(bright / 50.)))
}
pub fn shadow(c: Rgb, gamma: f64) -> Rgb {
    if gamma <= 1. {
        return c;
    }
    let brightness =
        clamp((c[0] as f64 * 0.2126 + c[1] as f64 * 0.7152 + c[2] as f64 * 0.0722) / 255.);
    let scale = brightness.powf(gamma - 1.);
    rgb(c.map(|v| v as f64 * scale))
}
pub fn ink_alpha(tile: &str, important: bool) -> f64 {
    if important {
        1.
    } else if matches!(tile, "water" | "shallow-water") {
        0.64
    } else if tile == "wall" {
        0.96
    } else {
        0.88
    }
}
pub fn translucent_ink(fg: Rgb, bg: Rgb, min: f64, alpha: f64) -> Rgb {
    if contrast(composite(bg, fg, alpha), bg) >= min {
        return fg;
    }
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..8 {
        let mid = (lo + hi) / 2.;
        if contrast(composite(bg, composite(fg, [255; 3], mid), alpha), bg) >= min {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    composite(fg, [255; 3], hi)
}
fn ramp(dark: Rgb, normal: Rgb, bright: Rgb, amount: f64) -> Rgb {
    if amount <= 1. {
        composite(dark, normal, amount)
    } else {
        composite(normal, bright, amount - 1.)
    }
}
fn offset(anchor: Rgb, actual: Rgb, reference: Rgb) -> Rgb {
    rgb(std::array::from_fn(|i| {
        anchor[i] as f64 + actual[i] as f64 - reference[i] as f64
    }))
}
pub fn illuminated(
    p: &Profile,
    fg: Rgb,
    bg: Rgb,
    tile: &str,
    light: Light,
    stone_floor: bool,
) -> (Rgb, Rgb) {
    let light = light.baseline();
    let energy = if tile == "wall" {
        light.wall_energy
    } else {
        light.energy
    };
    let l = light.ambient.max(
        energy.into_iter().fold(0.0f64, f64::max)
            * 160.
            * p.value(&format!("terrain.{tile}.lightingResponse"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.),
    );
    let amount = clamp(l / 100.) * 2.;
    match tile {
        "wall" => (
            offset(
                ramp(
                    p.rgb("palette.stone.wallShadow"),
                    p.rgb("terrain.wall.color"),
                    p.rgb("palette.stone.wallBright"),
                    amount,
                ),
                fg,
                hex("777987"),
            ),
            bg,
        ),
        "floor" if stone_floor => (
            fg,
            offset(
                if l < 0. {
                    composite(
                        p.rgb("palette.stone.floorUnlit"),
                        p.rgb("palette.stone.deepShadow"),
                        clamp(-l / 100.),
                    )
                } else {
                    ramp(
                        p.rgb("palette.stone.floorUnlit"),
                        p.rgb("palette.stone.floorLit"),
                        p.rgb("palette.stone.floorBright"),
                        amount,
                    )
                },
                bg,
                hex("151923"),
            ),
        ),
        "water" | "shallow-water" => (
            ramp(
                p.rgb("palette.water.dark"),
                p.rgb("terrain.water.color"),
                p.rgb("palette.water.illuminated"),
                amount,
            ),
            bg,
        ),
        _ => (fg, bg),
    }
}
pub fn map_colors(p: &Profile, fg: Rgb, bg: Rgb, c: Context<'_>) -> (Rgb, Rgb) {
    map_colors_emissive(p, fg, bg, c, false)
}
pub fn map_colors_emissive(
    p: &Profile,
    fg: Rgb,
    bg: Rgb,
    c: Context<'_>,
    emissive: bool,
) -> (Rgb, Rgb) {
    map_colors_response(
        p,
        fg,
        bg,
        c,
        emissive,
        p.value(&format!("terrain.{}.lightingResponse", c.tile))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(1.),
    )
}
pub fn map_colors_response(
    p: &Profile,
    fg: Rgb,
    bg: Rgb,
    c: Context<'_>,
    emissive: bool,
    response: f64,
) -> (Rgb, Rgb) {
    map_colors_observed(p, fg, bg, c, emissive, response, 100., false)
}
pub fn map_colors_observed(
    p: &Profile,
    mut fg: Rgb,
    mut bg: Rgb,
    c: Context<'_>,
    emissive: bool,
    response: f64,
    chance: f64,
    all_seeing: bool,
) -> (Rgb, Rgb) {
    let remembered = matches!(c.knowledge, Knowledge::Remembered | Knowledge::Layout);
    match c.knowledge {
        Knowledge::Direct => {
            fg = environment(p, fg, false, c.light, emissive, c.tile, response);
            bg = environment(p, bg, true, c.light, false, c.tile, response);
            let gamma = shadow_gamma(p, c.distance, c.light, c.tile);
            if !c.important && !emissive {
                fg = shadow(fg, gamma);
            }
            bg = shadow(bg, gamma);
        }
        Knowledge::Telepathic => {
            fg = telepathic(p, fg, false);
            bg = telepathic(p, bg, true);
        }
        _ => {
            fg = memory(p, fg);
            bg = memory(p, bg);
        }
    }
    let water = matches!(c.tile, "water" | "shallow-water");
    let alpha = ink_alpha(c.tile, c.important || emissive);
    let min = if remembered {
        1.46
    } else if c.important || emissive {
        2.81
    } else if water {
        1.83
    } else if c.tile == "floor" {
        1.5
    } else {
        1.81
    };
    fg = translucent_ink(fg, bg, min, alpha);
    if c.knowledge == Knowledge::Direct {
        fg = grade(fg, 1., 1., p.number("fog.visibleContrast"));
        bg = grade(bg, 1., 1., p.number("fog.visibleContrast"));
        if p.number("fog.transition") != 0. {
            fg = composite(
                bg,
                fg,
                1. - p.number("fog.transition") * (1. - (chance / 100.).clamp(0., 1.)),
            );
        }
    }
    if c.knowledge == Knowledge::Layout || all_seeing {
        let color = if all_seeing {
            "fog.allSeeingColor"
        } else {
            "fog.revealColor"
        };
        let intensity = if all_seeing {
            "fog.allSeeingIntensity"
        } else {
            "fog.revealIntensity"
        };
        let amount = if p.text(color) == "#87b7f3" { 0. } else { 0.3 };
        fg = grade(
            composite(fg, p.rgb(color), amount),
            p.number(intensity),
            1.,
            1.,
        );
        bg = grade(
            composite(bg, p.rgb(color), amount),
            p.number(intensity),
            1.,
            1.,
        );
    }
    (
        world(p, fg, c.knowledge == Knowledge::Direct),
        world(p, bg, c.knowledge == Knowledge::Direct),
    )
}
pub fn terrain_style(
    p: &Profile,
    fg: Rgb,
    bg: Rgb,
    c: Context<'_>,
    x: usize,
    y: usize,
    seed: u32,
    fixture: bool,
    stone_floor: bool,
) -> (Rgb, Rgb) {
    let (fg, bg) = if c.knowledge == Knowledge::Direct && !fixture {
        illuminated(p, fg, bg, c.tile, c.light, stone_floor)
    } else {
        (fg, bg)
    };
    let (fg, bg) = vary(p, fg, bg, c.tile, x as f64, y as f64, seed);
    map_colors(p, fg, bg, c)
}

pub fn grade(c: Rgb, brightness: f64, saturation: f64, contrast: f64) -> Rgb {
    if brightness == 1. && saturation == 1. && contrast == 1. {
        return c;
    }
    let gray = c[0] as f64 * 0.2126 + c[1] as f64 * 0.7152 + c[2] as f64 * 0.0722;
    rgb(c
        .map(|v| ((gray + (v as f64 - gray) * saturation - 127.5) * contrast + 127.5) * brightness))
}
pub fn world(p: &Profile, c: Rgb, visible: bool) -> Rgb {
    let tone_settings = p.global_tone();
    let channels = c.map(|v| v as f64 / 255.);
    let luma = channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
    let tone = tone_settings.shadows * (1. - luma).powi(2)
        + tone_settings.midtones * 2. * luma * (1. - luma)
        + tone_settings.highlights * luma.powi(2);
    rgb(std::array::from_fn(|i| {
        let value = (((luma + (channels[i] - luma) * tone_settings.saturation - 0.5) * tone_settings.contrast + 0.5)
            * tone_settings.exposure
            * tone
            * (1. + tone_settings.temperature * [0.15, 0., -0.15][i]))
            .max(0.)
            .powf(1. / tone_settings.gamma);
        value.clamp(
            if visible { tone_settings.minimum_brightness } else { 0. },
            tone_settings.maximum_brightness,
        ) * 255.
    }))
}
#[cfg(test)]
fn world_json_reference(p: &Profile, c: Rgb, visible: bool) -> Rgb {
    let n = |key: &str| p.value(&format!("global.{key}")).and_then(serde_json::Value::as_f64).unwrap_or(0.);
    let channels = c.map(|v| v as f64 / 255.);
    let luma = channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
    let tone = n("shadows") * (1. - luma).powi(2)
        + n("midtones") * 2. * luma * (1. - luma)
        + n("highlights") * luma.powi(2);
    rgb(std::array::from_fn(|i| {
        let value = (((luma + (channels[i] - luma) * n("saturation") - 0.5) * n("contrast") + 0.5)
            * n("exposure")
            * tone
            * (1. + n("temperature") * [0.15, 0., -0.15][i]))
            .max(0.)
            .powf(1. / n("gamma"));
        value.clamp(
            if visible { n("minimumBrightness") } else { 0. },
            n("maximumBrightness"),
        ) * 255.
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_profile_preserves_600_frozen_color_cases() {
        let p = Profile::default();
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../assets/color-parity.json")).unwrap();
        for row in cases.as_array().unwrap() {
            let ch = |key: &str| std::array::from_fn(|i| row[key][i].as_u64().unwrap() as u8);
            let tile = row["tile"].as_str().unwrap();
            let c = Context {
                tile,
                knowledge: match row["knowledge"].as_str().unwrap() {
                    "direct" => Knowledge::Direct,
                    "telepathic" => Knowledge::Telepathic,
                    "remembered" => Knowledge::Remembered,
                    _ => Knowledge::Layout,
                },
                distance: row["distance"].as_f64().unwrap(),
                light: Light::ambient(row["illumination"].as_f64().unwrap()),
                important: matches!(tile, "up" | "down" | "pit" | "chasm"),
            };
            let args = (
                ch("baseFg"),
                ch("baseBg"),
                row["seed"].as_u64().unwrap() as u32,
            );
            assert_eq!(
                terrain_style(&p, args.0, args.1, c, 11, 19, args.2, false, true),
                crate::color::terrain_style(args.0, args.1, c, 11, 19, args.2, false, true),
                "{row}"
            );
        }
    }
}

pub fn terrain_material(
    p: &Profile,
    fg: Rgb,
    bg: Rgb,
    c: Context<'_>,
    x: usize,
    y: usize,
    seed: u32,
    fixture: bool,
    stone: bool,
    material: &serde_json::Value,
    chance: f64,
    all_seeing: bool,
    illuminated_override: Option<(Rgb, Rgb)>,
) -> (Rgb, Rgb, Rgb) {
    let n = |key: &str| material[key].as_f64().unwrap_or(1.);
    let (fg, bg) = illuminated_override.unwrap_or_else(|| {
        if c.knowledge == Knowledge::Direct && !fixture {
            illuminated(p, fg, bg, c.tile, c.light, stone)
        } else { (fg, bg) }
    });
    let (fg, bg) = vary_settings(
        fg,
        bg,
        c.tile,
        x as f64,
        y as f64,
        seed,
        n("brightnessVariation"),
        n("colorVariation"),
    );
    let material_bg = bg;
    let (fg, bg) = map_colors_observed(
        p,
        grade(fg, n("brightness"), n("saturation"), n("contrast")),
        grade(bg, n("brightness"), n("saturation"), n("contrast")),
        c,
        false,
        n("lightingResponse"),
        chance,
        all_seeing,
    );
    (fg, bg, material_bg)
}

/// Same color math with immutable, already parsed material values.
pub fn terrain_material_resolved(
    p: &Profile,
    fg: Rgb,
    bg: Rgb,
    c: Context<'_>,
    x: usize,
    y: usize,
    seed: u32,
    fixture: bool,
    stone: bool,
    material: crate::appearance_profile::MaterialView<'_>,
    chance: f64,
    all_seeing: bool,
    illuminated_override: Option<(Rgb, Rgb)>,
) -> (Rgb, Rgb, Rgb) {
    let (fg, bg) = illuminated_override.unwrap_or_else(|| {
        if c.knowledge == Knowledge::Direct && !fixture {
            illuminated(p, fg, bg, c.tile, c.light, stone)
        } else { (fg, bg) }
    });
    let (fg, bg) = vary_settings(
        fg,
        bg,
        c.tile,
        x as f64,
        y as f64,
        seed,
        material.brightness_variation,
        material.color_variation,
    );
    let material_bg = bg;
    let (fg, bg) = map_colors_observed(
        p,
        grade(fg, material.brightness, material.saturation, material.contrast),
        grade(bg, material.brightness, material.saturation, material.contrast),
        c,
        false,
        material.lighting_response,
        chance,
        all_seeing,
    );
    (fg, bg, material_bg)
}

#[cfg(test)] mod global_tone_oracles {
    use super::*;
    #[test] fn compiled_global_tone_world_matches_raw_color_math(){
        let d=Profile::default();let mut ps=vec![d.clone()];
        for c in crate::appearance_profile::controls().iter().filter(|c|c.path.starts_with("global.")&&c.kind=="number") {
            for v in [c.min,c.max].into_iter().flatten(){if let Ok(p)=d.set(&c.path,serde_json::json!(v)){ps.push(p);}}
        }
        let custom=d.set("global.temperature",serde_json::json!(0.5)).unwrap().set("global.gamma",serde_json::json!(1.4)).unwrap();
        ps.push(custom.clone());ps.push(Profile::parse(&custom.json()).unwrap());ps.push(custom.reset_group("global").unwrap());
        for p in ps {for visible in [false,true]{for r in [0,1,13,64,127,192,254,255]{for g in [0,31,127,255]{for b in [0,79,255]{let rgb=[r,g,b];assert_eq!(world(&p,rgb,visible),world_json_reference(&p,rgb,visible),"{rgb:?} visible={visible}");}}}}}
    }
}

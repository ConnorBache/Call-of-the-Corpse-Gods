//! Native translation of v48 ui-color/ui-lighting/ui-tiles display math.
//! Inputs are detached knowledge only. This module cannot inspect a Run.
use crate::render::Rgb;
#[path = "light_ui.rs"]
pub mod light_ui;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Knowledge {
    Direct,
    Telepathic,
    Remembered,
    Layout,
}
#[derive(Clone, Copy, Debug)]
pub struct Light {
    /// Native display calibration only; None is the explicit frozen-v48 adapter.
    pub display: Option<light_ui::DisplayLight>,
    pub ambient: f64,
    pub illumination: f64,
    /// Presentation-only attenuation of material exposure by potion darkness.
    pub ambient_attenuation: f64,
    pub energy: [f64; 3],
    pub wall_energy: [f64; 3],
}
impl Light {
    pub fn ambient(value: f64) -> Self {
        Self {
            display: None,
            ambient: value,
            illumination: value,
            ambient_attenuation: 1.,
            energy: [0.; 3],
            wall_energy: [0.; 3],
        }
    }
    /// Author materials at their original zero-light appearance, then grade once.
    pub fn baseline(self) -> Self {
        if self.display.is_some() { Self::ambient(0.) } else { self }
    }
}
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
    corpse_core::timing::count(corpse_core::timing::Count::HexParse,1);
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
pub fn vary(fg: Rgb, bg: Rgb, tile: &str, x: f64, y: f64, seed: u32) -> (Rgb, Rgb) {
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
            fg[i] as f64 + patch * amp * 1.4 + hue * shift[i]
        })),
        rgb(std::array::from_fn(|i| {
            bg[i] as f64 + patch * amp + hue * shift[i]
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
pub fn memory(c: Rgb) -> Rgb {
    rgb(desaturate(c, 0.76).map(|v| v as f64 * 0.5))
}
pub fn telepathic(c: Rgb, bg: bool) -> Rgb {
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
    c
}
pub fn environment(c: Rgb, bg: bool, light: Light, emissive: bool, tile: &str) -> Rgb {
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
        exposed * if emissive { 1. } else { shade }
            + emission * strength * if bg { 0.65 + v } else { 0.3 + v * 0.7 }
    }))
}
pub fn shadow_gamma(distance: f64, light: Light, tile: &str) -> f64 {
    if distance <= 3. {
        return 1.;
    }
    let energy = if matches!(tile, "wall" | "tree") {
        light.wall_energy
    } else {
        light.energy
    };
    let peak = energy.into_iter().fold(0.0f64, f64::max);
    let bright = 0.0f64
        .max(light.illumination)
        .max(light.ambient)
        .max(peak * 160.);
    1. + 0.65 * smooth(clamp((distance - 3.) / 21.)) * (1. - smooth(clamp(bright / 50.)))
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
pub fn illuminated(fg: Rgb, bg: Rgb, tile: &str, light: Light, stone_floor: bool) -> (Rgb, Rgb) {
    let energy = if tile == "wall" {
        light.wall_energy
    } else {
        light.energy
    };
    let l = light
        .ambient
        .max(energy.into_iter().fold(0.0f64, f64::max) * 160.);
    let amount = clamp(l / 100.) * 2.;
    match tile {
        "wall" => (
            offset(
                ramp(hex("454957"), hex("777987"), hex("c5c3d3"), amount),
                fg,
                hex("777987"),
            ),
            bg,
        ),
        "floor" if stone_floor => (
            fg,
            offset(
                if l < 0. {
                    composite(hex("151923"), hex("0b0d15"), clamp(-l / 100.))
                } else {
                    ramp(hex("151923"), hex("242935"), hex("353344"), amount)
                },
                bg,
                hex("151923"),
            ),
        ),
        "water" | "shallow-water" => (
            ramp(hex("24638c"), hex("187dc5"), hex("39dfff"), amount),
            bg,
        ),
        _ => (fg, bg),
    }
}
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub knowledge: Knowledge,
    pub tile: &'a str,
    pub distance: f64,
    pub light: Light,
    pub important: bool,
}
pub fn map_colors(fg: Rgb, bg: Rgb, c: Context<'_>) -> (Rgb, Rgb) {
    map_colors_emissive(fg, bg, c, false)
}
pub fn map_colors_emissive(mut fg: Rgb, mut bg: Rgb, c: Context<'_>, emissive: bool) -> (Rgb, Rgb) {
    let remembered = matches!(c.knowledge, Knowledge::Remembered | Knowledge::Layout);
    match c.knowledge {
        Knowledge::Direct => {
            fg = environment(fg, false, c.light, emissive, c.tile);
            bg = environment(bg, true, c.light, false, c.tile);
            let gamma = shadow_gamma(c.distance, c.light, c.tile);
            if !c.important && !emissive {
                fg = shadow(fg, gamma);
            }
            bg = shadow(bg, gamma);
        }
        Knowledge::Telepathic => {
            fg = telepathic(fg, false);
            bg = telepathic(bg, true);
        }
        _ => {
            fg = memory(fg);
            bg = memory(bg);
        }
    }
    let water = matches!(c.tile, "water" | "shallow-water");
    let alpha = ink_alpha(c.tile, c.important);
    let min = if remembered {
        1.46
    } else if c.important {
        2.81
    } else if water {
        1.83
    } else if c.tile == "floor" {
        1.5
    } else {
        1.81
    };
    fg = translucent_ink(fg, bg, min, alpha);
    (fg, bg)
}
pub fn terrain_style(
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
        illuminated(fg, bg, c.tile, c.light, stone_floor)
    } else {
        (fg, bg)
    };
    let (fg, bg) = vary(fg, bg, c.tile, x as f64, y as f64, seed);
    map_colors(fg, bg, c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srgb_roundtrip() {
        for i in 0..=255 {
            assert_eq!(from_linear(to_linear(i as f64)).round() as u8, i);
        }
    }
    #[test]
    fn dark_blue_telepathy() {
        let c = telepathic([0; 3], true);
        assert!(c[2] > c[1] && c[1] > c[0]);
    }
    #[test]
    fn contrast_after_alpha() {
        let fg = translucent_ink([20; 3], [15; 3], 1.83, 0.64);
        assert!(contrast(composite([15; 3], fg, 0.64), [15; 3]) >= 1.83);
    }
}
#[cfg(test)]
mod parity {
    use super::*;
    #[test]
    fn source_600_material_light_memory_cases() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../assets/color-parity.json")).unwrap();
        for row in cases.as_array().unwrap() {
            let channel =
                |k: &str| -> Rgb { std::array::from_fn(|i| row[k][i].as_u64().unwrap() as u8) };
            let tile = row["tile"].as_str().unwrap();
            let knowledge = match row["knowledge"].as_str().unwrap() {
                "direct" => Knowledge::Direct,
                "telepathic" => Knowledge::Telepathic,
                "remembered" => Knowledge::Remembered,
                _ => Knowledge::Layout,
            };
            let c = Context {
                knowledge,
                tile,
                distance: row["distance"].as_f64().unwrap(),
                light: Light::ambient(row["illumination"].as_f64().unwrap()),
                important: matches!(tile, "up" | "down" | "pit" | "chasm"),
            };
            let actual = terrain_style(
                channel("baseFg"),
                channel("baseBg"),
                c,
                11,
                19,
                row["seed"].as_u64().unwrap() as u32,
                false,
                true,
            );
            assert_eq!(actual, (channel("fg"), channel("bg")), "{row}");
        }
    }
}
/// O(C + sum(source footprint sizes)), O(C + S) extra memory. Default authored
/// visual radius is one, so no expanded rays or hidden geometry are consulted.
pub fn light_field(observation: &corpse_core::vision::TerrainObservation) -> Vec<Light> {
    let n = observation.width * observation.height;
    let mut out = vec![Light::ambient(0.); n];
    let mut energy = vec![[0f32; 3]; n];
    let mut wall = vec![[0f32; 3]; n];
    let mut maximum = vec![0f32; n];
    let mut darkness = vec![0f32; n];
    for cell in &observation.cells {
        let i = cell.index;
        out[i].ambient = cell.ambient.unwrap_or(0.);
        out[i].illumination = cell.illumination.unwrap_or(0.);
        maximum[i] = out[i].ambient.max(0.) as f32;
    }
    let mut family = Vec::<&corpse_core::vision::ObservedLightSource>::new();
    let mut sources = Vec::new();
    for s in &observation.light_sources {
        if s.color_role == "darkness" {
            for (&i, &strength) in s.cells.iter().zip(&s.strengths) {
                darkness[i] = (darkness[i] as f64 + strength) as f32;
            }
            continue;
        }
        if s.color_role != "fire" && s.color_role != "torch" {
            sources.push(s);
            continue;
        }
        if let Some(old) = family
            .iter_mut()
            .find(|old| old.emission_group == s.emission_group)
        {
            if s.power > old.power || (s.power == old.power && s.kind != "fire") {
                *old = s;
            }
        } else {
            family.push(s);
        }
    }
    sources.extend(family);
    for s in sources {
        let burning = s.origin_known
            && s.kind == "fire"
            && observation
                .cells
                .get(s.y as usize * observation.width + s.x as usize)
                .and_then(|c| c.hazards.as_ref())
                .is_some_and(|h| h.fire.as_ref().is_some_and(|f| f == "ichor"));
        let c = linear(hex(if burning {
            "ff4b24"
        } else if s.kind == "torch" {
            "ffd34a"
        } else if s.kind == "brazier" || s.color_role == "fire" {
            "ff762e"
        } else if s.color_role == "magic" {
            "b64dff"
        } else if s.color_role == "torch" {
            "ffd34a"
        } else {
            "fff8e8"
        }));
        for (&i, &strength) in s.cells.iter().zip(&s.strengths) {
            let p = clamp(strength / s.power.max(1.));
            let intensity =
                p.powi(3) * (p * (p * 6. - 15.) + 10.) * (s.power / 100.).clamp(0., 1.5);
            for k in 0..3 {
                energy[i][k] = (energy[i][k] as f64 + c[k] * intensity) as f32;
                wall[i][k] = (wall[i][k] as f64 + c[k] * intensity) as f32;
            }
            maximum[i] = maximum[i].max(strength as f32);
        }
    }
    for i in 0..n {
        let reduction = clamp(1. - darkness[i] as f64 / (maximum[i] as f64 + 100.));
        for field in [&mut energy, &mut wall] {
            let peak = field[i].into_iter().fold(0f32, f32::max) as f64;
            let exposure = reduction / (1. + peak / 1.25);
            for k in 0..3 {
                field[i][k] = (field[i][k] as f64 * exposure) as f32;
            }
        }
        out[i].energy = energy[i].map(|n| n as f64);
        out[i].wall_energy = wall[i].map(|n| n as f64);
    }
    out
}
pub fn gas_field(c: Rgb) -> Rgb {
    let channels = linear(c);
    let gray = luminance(c);
    from_channels(channels.map(|v| (gray + (v - gray) * 0.94) * 0.92))
}
pub fn distinguish(c: Rgb, bg: Rgb, min: f64) -> Rgb {
    if contrast(c, bg) >= min {
        return c;
    }
    let original = linear(c);
    let target = (min * (luminance(bg) + 0.05) - 0.05).min(1.);
    let current = luminance(c);
    let amount = clamp((target - current) / (1. - current));
    from_channels(original.map(|v| v + (1. - v) * amount))
}

#[cfg(test)]
mod generated_light_parity {
    #[test]
    fn visible_source_light_field() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../assets/generated-light-parity.json")).unwrap();
        let run =
            corpse_core::startup::create_run_revision(expected["seed"].as_str().unwrap(), "Avatar", false, 30.)
                .unwrap();
        let observation = corpse_core::vision::observe_terrain(&run).unwrap();
        let actual = super::light_field(&observation);
        for row in expected["cells"].as_array().unwrap() {
            let i = row["i"].as_u64().unwrap() as usize;
            let energy: [f64; 3] = serde_json::from_value(row["energy"].clone()).unwrap();
            assert_eq!(actual[i].energy, energy, "light field cell {i}");
        }
    }
}

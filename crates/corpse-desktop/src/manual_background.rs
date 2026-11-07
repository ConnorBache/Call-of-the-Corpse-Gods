//! Deterministic, bounded, presentation-only bestiary scenery. No Run, RNG, or clock access.
use super::{signature, Entry, Theme};
use crate::render::Painter;
use std::f64::consts::TAU;
#[path = "manual_environment.rs"]
mod environment;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum Motif {
    Slime,
    Burrow,
    Shell,
    Blood,
    Wings,
    Teeth,
    Needles,
    Spores,
    Honeycomb,
    Roots,
    Regrowth,
    Void,
    Current,
    Breath,
    Ripple,
    Brood,
    Seeds,
    Claws,
    Sigil,
    Horns,
    Sand,
    Crystal,
    Gears,
    Sparks,
    Kelp,
    Frost,
    Blades,
    Flame,
    Moon,
    Mirror,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct Profile {
    motif: Motif,
    seed: u32,
    magnitude: u8,
}
impl Profile {
    fn for_entry(e: &Entry) -> Self {
        use Motif::*;
        // Explicit authored associations. Unknown future catalog entries fail coverage tests.
        let (motif, magnitude) = match e.id.as_str() {
            "blorb" => (Slime, 1),
            "glim" => (Slime, 3),
            "rat" => (Burrow, 1),
            "shellback-beetle" => (Shell, 1),
            "giant-beetle" => (Shell, 3),
            "cave-bat" => (Wings, 1),
            "hemogoblin" => (Blood, 1),
            "hemogoblin-bloodmancer" => (Blood, 3),
            "vampire" => (Blood, 4),
            "needleling" => (Needles, 1),
            "bastidion" => (Spores, 3),
            "dryad" => (Roots, 2),
            "samara" => (Seeds, 3),
            "short-saidri" => (Honeycomb, 1),
            "long-saidri" => (Honeycomb, 2),
            "saidri-guardian" => (Honeycomb, 4),
            "pale:Jackal" => (Teeth, 1),
            "pale:Biter" => (Teeth, 2),
            "pale:Greater Biter" => (Teeth, 4),
            "pale:Breather" => (Breath, 2),
            "pale:Greater Breather" => (Breath, 4),
            "pale:Spitter" => (Needles, 2),
            "pale:Greater Spitter" => (Needles, 4),
            "pale:Screamer" => (Ripple, 2),
            "pale:Greater Screamer" => (Ripple, 4),
            "pale:Birther" => (Brood, 2),
            "pale:Greater Birther" => (Brood, 4),
            "zillennia:infant" => (Void, 1),
            "zillennia:child" => (Void, 2),
            "zillennia:lesser" => (Void, 3),
            "zillennia:unqualified" => (Void, 4),
            "zillennia:mature" => (Void, 5),
            "zillennia:greater" => (Void, 6),
            "zillennia:hulking" => (Void, 7),
            "zillennia:ancient" => (Void, 8),
            "zillennia:timeless" => (Void, 9),
            "zillennia:transcendent" => (Void, 10),
            "lesser-ichor-fish" => (Current, 1),
            "ichor-fish-standard" => (Current, 2),
            "greater-ichor-fish" => (Current, 3),
            "lesser-ichor-shark" => (Current, 4),
            "ichor-shark" => (Current, 5),
            "greater-ichor-shark" => (Current, 6),
            "lesser-ichor-leviathan" => (Current, 7),
            "ichor-leviathan" => (Current, 8),
            "greater-ichor-leviathan" => (Current, 9),
            "river-drake" => (Current, 4),
            "kelpie" => (Kelp, 3),
            "cancer" => (Shell, 4),
            "bear" => (Claws, 1),
            "dire-bear" => (Claws, 3),
            "tiger" => (Claws, 2),
            "sabretooth" => (Teeth, 5),
            "wendigo" => (Claws, 4),
            "troll" => (Regrowth, 4),
            "witch" => (Sigil, 3),
            "sphinx" => (Sand, 3),
            "horn-1" => (Horns, 1),
            "horn-2" => (Horns, 2),
            "horn-3" => (Horns, 3),
            "horn-4" => (Horns, 4),
            "horn-5" => (Horns, 5),
            "horn-6" => (Horns, 6),
            "horn-7" => (Horns, 7),
            "minotaur" => (Horns, 2),
            "hemogoblin-inventor" => (Crystal, 3),
            "hemogoblin-mech" => (Gears, 4),
            "mini-me" => (Gears, 1),
            "half-here" => (Mirror, 2),
            "bombardier" => (Sparks, 3),
            "killspawn" => (Blades, 4),
            "banshee" => (Ripple, 5),
            "maulchild" => (Brood, 4),
            "phoenix" => (Flame, 2),
            "chimera" => (Flame, 3),
            "dragon" => (Flame, 5),
            "yeti" => (Frost, 4),
            "moonspawn" => (Moon, 1),
            "valta" => (Moon, 2),
            "ursa-minor" => (Moon, 3),
            "ursa-major" => (Moon, 4),
            "night-crawler" => (Blood, 3),
            "skin-walker" => (Mirror, 4),
            "saidribee-queen" => (Honeycomb, 5),
            id => panic!("Missing bestiary background profile: {id}"),
        };
        Self {
            motif,
            magnitude,
            seed: signature(&e.id, &e.family),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Descent {
    embers: usize,
    height: f64,
    speed: f64,
    heat: f64,
}
impl Descent {
    fn at(depth: usize) -> Self {
        let d = depth.min(39);
        Self {
            embers: d,
            height: 8. + d as f64 * 3.,
            speed: 0.45 + d as f64 / 50.,
            heat: d as f64 / 39.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stroke {
    a: (f64, f64),
    b: (f64, f64),
    hot: bool,
}
const MAX_STROKES: usize = 4096;
struct Scene {
    strokes: Vec<Stroke>,
}
impl Scene {
    fn line(&mut self, a: (f64, f64), b: (f64, f64), hot: bool) {
        assert!(
            self.strokes.len() < MAX_STROKES,
            "bounded background geometry"
        );
        self.strokes.push(Stroke { a, b, hot });
    }
    fn ring(&mut self, x: f64, y: f64, r: f64, squash: f64, start: f64, arc: f64) {
        for i in 0..16 {
            let a = start + arc * i as f64 / 16.;
            let b = start + arc * (i + 1) as f64 / 16.;
            self.line(
                (x + r * a.cos(), y + r * a.sin() * squash),
                (x + r * b.cos(), y + r * b.sin() * squash),
                false,
            );
        }
    }
}
fn phase(time_ms: f64) -> f64 {
    // Bound before arithmetic, including malformed QA inputs; cosmetic cycle repeats in ten minutes.
    if time_ms.is_finite() {
        time_ms.rem_euclid(600_000.) / 1000.
    } else {
        0.
    }
}
// O(K + E) bounded geometry, K=12 motifs, E<=34 infernal cinders;
// at most 4096 strokes retained. Raster costs are accounted separately in the ledger.
fn scene(profile: Profile, depth: usize, time_ms: f64, ax: i32) -> Scene {
    use Motif::*;
    let mut s = Scene {
        strokes: Vec::with_capacity(2048),
    };
    let d = Descent::at(depth);
    let t = phase(time_ms) * d.speed;
    let seed = profile.seed;
    for i in 0..12u32 {
        let h = seed.wrapping_add(i.wrapping_mul(0x9e3779b9));
        let x = if i < 8 {
            ax as f64 + 24. + (h % 352) as f64
        } else {
            20. + (h % 1240) as f64
        };
        let y = if i < 8 {
            165. + ((h >> 10) % 320) as f64
        } else {
            140. + ((h >> 10) % 590) as f64
        };
        let p = t + (h % 628) as f64 / 100.;
        let r = 12. + profile.magnitude as f64 * 3. + (p * 0.7).sin() * 3.;
        match profile.motif {
            Spores => {
                // Rooted fungal canopy: connected stalks, cap undersides, and drifting spore dust.
                let base = if i < 8 { 518. } else { 738. };
                let stalk_height = 100. + ((h >> 15) % if i < 8 { 240 } else { 400 }) as f64;
                let lean = (p * 0.25).sin() * 10.;
                for j in 0..8 {
                    let q = j as f64 / 8.;
                    let q2 = (j + 1) as f64 / 8.;
                    s.line(
                        (x + lean * q * q, base - q * stalk_height),
                        (x + lean * q2 * q2, base - q2 * stalk_height),
                        false,
                    );
                    if j % 2 == 0 {
                        let yy = base - q * stalk_height;
                        s.ring(
                            x + lean * q * q,
                            yy,
                            r * (0.5 + q),
                            0.38,
                            std::f64::consts::PI,
                            std::f64::consts::PI,
                        );
                        s.line((x - r * (0.5 + q), yy), (x + r * (0.5 + q), yy), false);
                    }
                }
                for j in 0..5 {
                    let xx = x + (p * 0.4 + j as f64).sin() * r * 1.8;
                    let yy = base - (p * 9. + j as f64 * 71.).rem_euclid(stalk_height + 30.);
                    s.line((xx, yy), (xx + 1., yy + 1.), false);
                }
            }
            Slime | Breath | Brood | Regrowth => {
                let yy = y - (p * 7.).rem_euclid(55.);
                s.ring(
                    x,
                    yy,
                    r,
                    if profile.motif == Slime { 0.5 } else { 1. },
                    0.,
                    TAU,
                );
                if matches!(profile.motif, Brood | Regrowth) {
                    s.ring(x + r, yy - r, 3., 1., 0., TAU);
                }
            }
            Burrow | Sand | Current | Kelp | Roots => {
                for j in 0..12 {
                    let a = j as f64;
                    let wave = |q: f64| (x + (q - 6.) * 9., y + (q * 0.5 + p).sin() * r);
                    let (a, b) = (wave(a), wave(a + 1.));
                    if matches!(profile.motif, Kelp | Roots) {
                        s.line(
                            (x + a.1 - y, y + a.0 - x),
                            (x + b.1 - y, y + b.0 - x),
                            false,
                        );
                    } else {
                        s.line(a, b, false);
                    }
                }
                if profile.motif == Roots {
                    s.line((x, y), (x + r, y - r), false);
                }
            }
            Moon | Ripple | Void | Mirror => {
                for j in 0..3 {
                    let rr = r + j as f64 * 9. + (p * 3.).rem_euclid(9.);
                    s.ring(
                        x,
                        y,
                        rr,
                        if profile.motif == Void {
                            0.3 + (p * 0.3).sin().abs() * 0.4
                        } else {
                            1.
                        },
                        if profile.motif == Moon { p * 0.12 } else { 0. },
                        if profile.motif == Moon {
                            TAU * 0.72
                        } else {
                            TAU
                        },
                    );
                }
                if profile.motif == Mirror {
                    s.line((x - r, y - r), (x + r, y + r), false);
                }
            }
            Blood | Flame => {
                let yy = if profile.motif == Blood {
                    y + (p * 12.).rem_euclid(60.)
                } else {
                    y - (p * 16.).rem_euclid(60.)
                };
                s.line((x, yy), (x + (p * 1.3).sin() * 8., yy - r * 1.6), false);
                s.ring(x, yy, 3. + (p.sin() + 1.) * 2., 1.4, 0., TAU);
            }
            Shell | Honeycomb | Crystal | Gears | Sigil | Frost => {
                let sides = if profile.motif == Gears { 24 } else { 6 };
                for j in 0..sides {
                    let angle = |n: usize| p * 0.09 + n as f64 * TAU / sides as f64;
                    let point = |n: usize| {
                        let a = angle(n);
                        let rr = if profile.motif == Gears && n % 2 == 0 {
                            r * 0.7
                        } else {
                            r
                        };
                        (x + rr * a.cos(), y + rr * a.sin())
                    };
                    s.line(point(j), point(j + 1), false);
                    if matches!(profile.motif, Frost | Sigil | Shell) {
                        s.line((x, y), point(j), false);
                    }
                }
                if profile.motif == Gears {
                    s.ring(x, y, r * 0.4, 1., 0., TAU);
                }
                if profile.motif == Crystal {
                    s.line((x, y - r), (x, y + r), false);
                }
            }
            Wings | Seeds => {
                let flap = (p * 1.2).sin() * r * 0.6;
                for sign in [-1., 1.] {
                    s.line((x, y), (x + sign * r, y - r + flap), false);
                    s.line(
                        (x + sign * r, y - r + flap),
                        (x + sign * r * 0.6, y + flap),
                        false,
                    );
                }
                if profile.motif == Seeds {
                    s.line((x, y), (x, y + r), false);
                }
            }
            Teeth | Needles | Claws | Blades | Horns | Sparks => {
                let n = if profile.motif == Horns {
                    profile.magnitude as usize
                } else {
                    3
                };
                for j in 0..n {
                    let xx = x + (j as f64 - (n as f64 - 1.) / 2.) * 12.;
                    let sway = (p * 0.6).sin() * 5.;
                    s.line((xx - 5., y + r), (xx + sway, y - r), false);
                    s.line((xx + sway, y - r), (xx + 5., y + r), false);
                    if profile.motif == Sparks {
                        s.line((xx + sway, y - r), (xx + 14., y - r * 1.4), false);
                    }
                }
            }
        }
    }
    // Cinders supplement the infernal architecture only in the hell band.
    for i in 0..if (29..=34).contains(&depth) {
        d.embers
    } else {
        0
    } {
        let p = t * 18. + i as f64 * 17.;
        let x = ((i * 97 + seed as usize % 101) % 1280) as f64 + (p * 0.02).sin() * 9.;
        let y = 738. - p.rem_euclid(d.height * 1.6);
        s.line((x, y), (x + 2., y - 3. - d.heat * 5.), true);
    }
    s
}
fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    std::array::from_fn(|i| (a[i] as f64 * (1. - t) + b[i] as f64 * t).round() as u8)
}
// A low-frequency field joins the motifs into scenery rather than isolated symbols.
// The 4px logical grid has <=48,320 samples; fixed trig work per sample and O(A) raster writes.
fn atmosphere(
    p: &mut Painter,
    profile: Profile,
    depth: usize,
    time_ms: f64,
    ax: i32,
    theme: &Theme,
    tint: [u8; 3],
) {
    use Motif::*;
    let d = Descent::at(depth);
    let t = phase(time_ms) * d.speed;
    let seed = (profile.seed % 1000) as f64 * 0.01;
    for y in (136..738).step_by(4) {
        let yy = y as f64;
        let descent = ((yy - 320.) / 418.).clamp(0., 1.);
        for x in (0..1280).step_by(4) {
            let xx = x as f64;
            let wave = (xx * 0.009 + seed + (yy * 0.008 + t * 0.12).sin()).sin();
            let fold = (yy * 0.012 + seed + (xx * 0.005 - t * 0.08).cos()).sin();
            let field = match profile.motif {
                Current | Kelp => (yy * 0.025 + (xx * 0.01 + t * 0.2).sin() * 2. - t * 0.35).sin(),
                Blood => (xx * 0.033 + (yy * 0.008 - t * 0.2).sin()).sin(),
                Flame | Sparks => (xx * 0.02 + (yy * 0.014 + t * 0.6).sin() * 2.).sin() * fold,
                Moon | Void | Mirror => {
                    let cx = ax as f64 + 200.;
                    let cy = 330.;
                    let radius = ((xx - cx).powi(2) + (yy - cy).powi(2)).sqrt();
                    (radius * 0.023 - t * 0.18).sin() * wave
                }
                Spores | Breath | Slime | Brood | Regrowth => {
                    wave * fold + (xx * 0.017 - yy * 0.011 + t * 0.1).sin() * 0.3
                }
                Frost => (xx * 0.013 + yy * 0.019 + t * 0.05).sin() * wave,
                _ => wave * fold,
            };
            let art_weight =
                (1. - ((xx - ax as f64 - 200.) / 220.).powi(2) - ((yy - 329.) / 205.).powi(2))
                    .clamp(0., 1.);
            let quiet = 0.035 + 0.085 * art_weight;
            let haze = (field + 1.3) / 2.6 * quiet;
            let mut color = mix(theme.paper, tint, haze);
            // Descending heat is an increasingly high, connected underglow with dark ridged masses.
            let tongues = ((xx * 0.026 + (yy * 0.015 + t * 0.35).sin() * 1.8).sin() + 1.) * 0.5;
            let heat = d.heat.powi(2) * descent.powi(2) * (0.10 + 0.25 * tongues);
            color = mix(color, [175, 37, 14], heat);
            p.rect(x, y, 4, 4, color);
        }
    }
}
pub(super) fn paint(p: &mut Painter, e: &Entry, depth: usize, time_ms: f64, theme: &Theme) {
    let profile = Profile::for_entry(e);
    let slot = profile.seed as usize % 3;
    let ax = 242 + slot as i32 * 308;
    let scene = scene(profile, depth, time_ms, ax);
    let heat = Descent::at(depth).heat;
    let species_ink = crate::color::hex(&e.color);
    let tint = mix(mix(species_ink, theme.ink, 0.5), [237, 62, 28], heat * 0.55);
    atmosphere(p, profile, depth, time_ms, ax, theme, tint);
    environment::paint(p, depth, time_ms, theme);
    let previous_clip = p.clip;
    p.clip = Some((0, 136, 1280, 602));
    for stroke in scene.strokes {
        // <=96 tiny strokes per segment; logical geometry never grows with elapsed time.
        let dx = stroke.b.0 - stroke.a.0;
        let dy = stroke.b.1 - stroke.a.1;
        let steps = (dx.abs().max(dy.abs()).ceil() as usize).clamp(1, 96);
        for i in 0..=steps {
            let x = (stroke.a.0 + dx * i as f64 / steps as f64).round() as i32;
            let y = (stroke.a.1 + dy * i as f64 / steps as f64).round() as i32;
            // Restrained texture beneath type; slightly richer around the opaque figure cutout.
            let art_weight = (1.
                - ((x as f64 - ax as f64 - 200.) / 220.).powi(2)
                - ((y as f64 - 329.) / 205.).powi(2))
            .clamp(0., 1.);
            let opacity = 0.025 + heat * 0.015 + art_weight * (0.09 + heat * 0.04);
            let color = if stroke.hot { [236, 66, 30] } else { tint };
            p.pixel(x, y, color, (opacity * 255.) as u8);
        }
    }
    p.clip = previous_clip;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_entry_has_a_distinct_semantic_profile() {
        let m = super::super::Manual::new();
        let profiles: std::collections::HashSet<_> =
            m.catalog.entries.iter().map(Profile::for_entry).collect();
        assert_eq!(profiles.len(), m.catalog.entries.len());
        assert_eq!(
            Profile::for_entry(
                m.catalog
                    .entries
                    .iter()
                    .find(|e| e.id == "bastidion")
                    .unwrap()
            )
            .motif,
            Motif::Spores
        );
        assert_eq!(
            Profile::for_entry(
                m.catalog
                    .entries
                    .iter()
                    .find(|e| e.id == "phoenix")
                    .unwrap()
            )
            .motif,
            Motif::Flame
        );
    }
    #[test]
    fn actual_compositions_are_unique_at_every_depth() {
        use std::hash::{Hash, Hasher};
        let m = super::super::Manual::new();
        for depth in 0..40 {
            let mut unique = std::collections::HashSet::new();
            for e in &m.catalog.entries {
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                for stroke in scene(Profile::for_entry(e), depth, 1700., 242).strokes {
                    for value in [stroke.a.0, stroke.a.1, stroke.b.0, stroke.b.1] {
                        value.to_bits().hash(&mut hash);
                    }
                }
                assert!(
                    unique.insert(hash.finish()),
                    "duplicate composition: {} at {}",
                    e.id,
                    depth
                );
            }
        }
    }
    #[test]
    fn raster_is_repeatable_animated_and_quiet_under_text() {
        fn luminance(c: [u8; 3]) -> f64 {
            let v = c.map(|n| {
                let n = n as f64 / 255.;
                if n <= 0.04045 {
                    n / 12.92
                } else {
                    ((n + 0.055) / 1.055).powf(2.4)
                }
            });
            v[0] * 0.2126 + v[1] * 0.7152 + v[2] * 0.0722
        }
        let m = super::super::Manual::new();
        let mut p = Painter::new();
        p.resize(320, 200);
        for id in [
            "bastidion",
            "hemogoblin",
            "yeti",
            "moonspawn",
            "zillennia:transcendent",
            "dragon",
        ] {
            let e = m.catalog.entries.iter().find(|e| e.id == id).unwrap();
            let ax = 242 + (Profile::for_entry(e).seed as usize % 3) as i32 * 308;
            for depth in 0..40 {
                let theme = Theme::at(depth);
                p.clear(theme.paper);
                paint(&mut p, e, depth, 0., &theme);
                let first = p.pixels.clone();
                p.clear(theme.paper);
                paint(&mut p, e, depth, 0., &theme);
                assert_eq!(p.pixels, first);
                p.clear(theme.paper);
                paint(&mut p, e, depth, 1700., &theme);
                assert_ne!(p.pixels, first);
                let ink = luminance(theme.ink);
                for y in 36..184usize {
                    for x in 0..320usize {
                        // Text, navigation and stats keep at least WCAG normal-text contrast.
                        if (ax..ax + 400).contains(&(x as i32 * 4)) && y * 4 < 518 {
                            continue;
                        }
                        let at = (y * 320 + x) * 4;
                        let paper = luminance([p.pixels[at], p.pixels[at + 1], p.pixels[at + 2]]);
                        let contrast = (paper.max(ink) + 0.05) / (paper.min(ink) + 0.05);
                        assert!(contrast >= 4.5, "{id} depth {depth} at {x},{y}: {contrast}");
                    }
                }
                assert_eq!(&p.pixels[..320 * 34 * 4], &first[..320 * 34 * 4]);
            }
        }
    }
    #[test]
    fn every_depth_increases_infernal_intensity() {
        for depth in 1..40 {
            let (a, b) = (Descent::at(depth - 1), Descent::at(depth));
            assert!(
                b.embers > a.embers && b.height > a.height && b.speed > a.speed && b.heat > a.heat
            );
        }
        assert_eq!(Descent::at(usize::MAX), Descent::at(39));
    }
    #[test]
    fn all_scenes_are_deterministic_animated_finite_and_bounded() {
        let m = super::super::Manual::new();
        for e in &m.catalog.entries {
            let profile = Profile::for_entry(e);
            for depth in 0..40 {
                let a = scene(profile, depth, 0., 242);
                let b = scene(profile, depth, 1700., 242);
                assert_eq!(a.strokes, scene(profile, depth, 0., 242).strokes);
                assert_ne!(a.strokes, b.strokes, "{}", e.id);
                for time in [0., 1700., 599999., f64::MAX, f64::NAN, f64::INFINITY, -1.] {
                    let s = scene(profile, depth, time, 858);
                    assert!(s.strokes.len() < MAX_STROKES);
                    for v in s.strokes {
                        for (x, y) in [v.a, v.b] {
                            assert!(
                                x.is_finite()
                                    && y.is_finite()
                                    && x.abs() < 1500.
                                    && y.abs() < 1000.
                            );
                        }
                    }
                }
            }
        }
    }
}

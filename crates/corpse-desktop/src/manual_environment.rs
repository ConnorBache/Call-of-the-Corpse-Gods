//! Depth-authored scenic silhouettes, behind all manual content. No simulation access.
use super::{mix, phase};
use crate::manual::Theme;
use crate::render::Painter;
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Cave,
    Dungeon,
    Ruin,
    Hell,
    Nightmare,
}
fn stage(depth: usize) -> Stage {
    match depth.min(39) {
        0..=9 => Stage::Cave,
        10..=23 => Stage::Dungeon,
        24..=28 => Stage::Ruin,
        29..=34 => Stage::Hell,
        _ => Stage::Nightmare,
    }
}
struct Scenery<'a> {
    p: &'a mut Painter,
    paper: [u8; 3],
    stone: [u8; 3],
    edge: [u8; 3],
    shadow: [u8; 3],
}
impl Scenery<'_> {
    // Convex/concave scan conversion: <=32 vertices, <=301 two-pixel scan rows.
    // Intersections use fixed stack storage; no per-row allocations or native-pixel trig.
    fn polygon(&mut self, points: &[(f64, f64)], color: [u8; 3]) {
        assert!(points.len() <= 32);
        let low = points
            .iter()
            .map(|p| p.1 as i32)
            .min()
            .unwrap_or(0)
            .max(136);
        let high = points
            .iter()
            .map(|p| p.1.ceil() as i32)
            .max()
            .unwrap_or(0)
            .min(738);
        for y in (low..high).step_by(2) {
            let yy = y as f64 + 0.5;
            let mut xs = [0.; 32];
            let mut n = 0;
            for i in 0..points.len() {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                if (a.1 <= yy && b.1 > yy) || (b.1 <= yy && a.1 > yy) {
                    xs[n] = a.0 + (yy - a.1) * (b.0 - a.0) / (b.1 - a.1);
                    n += 1;
                }
            }
            xs[..n].sort_by(f64::total_cmp);
            for pair in xs[..n].chunks_exact(2) {
                let x = pair[0].floor().max(0.) as i32;
                let right = pair[1].ceil().min(1280.) as i32;
                self.p.rect(x, y, right - x, 2, color);
            }
        }
    }
    fn line(&mut self, a: (f64, f64), b: (f64, f64), color: [u8; 3], width: i32) {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let n = (dx.abs().max(dy.abs()).ceil() as usize).clamp(1, 1280);
        for i in 0..=n {
            self.p.rect(
                (a.0 + dx * i as f64 / n as f64) as i32,
                (a.1 + dy * i as f64 / n as f64) as i32,
                width,
                width,
                color,
            );
        }
    }
    fn arch(&mut self, x: f64, y: f64, r: f64, bottom: f64, lean: f64, broken: bool) {
        let mut outer = Vec::with_capacity(20);
        outer.push((x - r, bottom));
        for i in 0..=16 {
            let a = PI + i as f64 * PI / 16.;
            outer.push((x + r * a.cos() + lean, y + r * a.sin()));
        }
        outer.push((x + r, bottom));
        self.polygon(&outer, self.stone);
        let mut inner = Vec::with_capacity(20);
        inner.push((x - r + 18., bottom));
        for i in 0..=16 {
            let a = PI + i as f64 * PI / 16.;
            inner.push((x + (r - 18.) * a.cos() + lean, y + (r - 18.) * a.sin() + 6.));
        }
        inner.push((x + r - 18., bottom));
        self.polygon(&inner, self.shadow);
        for i in 0..16 {
            if broken && (i == 4 || i == 5 || i == 12) {
                continue;
            }
            self.line(outer[i + 1], outer[i + 2], self.edge, 2);
            if i % 2 == 0 {
                self.line(outer[i + 1], inner[i + 1], self.edge, 1);
            }
        }
        self.line((x - r, bottom), (x - r + lean, y), self.edge, 2);
        self.line((x + r, bottom), (x + r + lean, y), self.edge, 2);
        if broken {
            self.polygon(
                &[
                    (x - 30., y - 30.),
                    (x + 15., y - 80.),
                    (x + 42., y - 58.),
                    (x + 4., y + 18.),
                ],
                self.paper,
            );
            for j in 0..4 {
                let xx = x - 70. + j as f64 * 35.;
                self.polygon(
                    &[
                        (xx, bottom),
                        (xx + 8., bottom - 20. - (j % 2) as f64 * 17.),
                        (xx + 29., bottom - 11.),
                        (xx + 35., bottom),
                    ],
                    self.stone,
                );
            }
        }
    }
}
pub(super) fn paint(p: &mut Painter, depth: usize, time_ms: f64, theme: &Theme) {
    let depth = depth.min(39);
    let stage = stage(depth);
    let t = phase(time_ms);
    let old_clip = p.clip;
    p.clip = Some((0, 136, 1280, 602));
    let deep = depth >= 24;
    let stone = mix(
        theme.paper,
        if deep { [92, 50, 56] } else { [39, 37, 31] },
        if deep { 0.48 } else { 0.17 },
    );
    let shadow = mix(theme.paper, [2, 3, 10], if deep { 0.36 } else { 0.24 });
    let edge = mix(
        theme.paper,
        if deep { theme.accent } else { theme.ink },
        if deep { 0.21 } else { 0.22 },
    );
    let mut s = Scenery {
        p,
        paper: theme.paper,
        stone,
        edge,
        shadow,
    };
    match stage {
        Stage::Cave => {
            // Overlapping ceiling strata, irregular hanging limestone and rising rock masses.
            for layer in 0..3 {
                let color = mix(theme.paper, [34, 32, 27], 0.07 + layer as f64 * 0.045);
                for i in 0..9 {
                    let x = i as f64 * 160. - 70.;
                    let h = 34. + ((i * 47 + layer * 31 + depth * 7) % 120) as f64;
                    let top = 136. + layer as f64 * 8.;
                    s.polygon(
                        &[
                            (x, top),
                            (x + 160., top),
                            (x + 143., top + 28.),
                            (x + 103., top + h * 0.4),
                            (x + 77., top + h),
                            (x + 54., top + h * 0.47),
                            (x + 12., top + 42.),
                        ],
                        color,
                    );
                }
            }
            for i in 0..11 {
                let x = i as f64 * 128. - 35.;
                let h = 35. + ((i * 57 + depth * 5) % 160) as f64;
                s.polygon(
                    &[
                        (x, 738.),
                        (x + 24., 738. - h * 0.25),
                        (x + 48., 738. - h),
                        (x + 68., 738. - h * 0.7),
                        (x + 90., 738. - h * 0.31),
                        (x + 140., 738.),
                    ],
                    stone,
                );
                s.line((x + 48., 738. - h), (x + 69., 730.), edge, 1);
            }
            // Mineral water falls slowly, an environmental animation independent of species.
            for i in 0..13 {
                let x = 31. + i as f64 * 101.;
                let y = 170. + (t * 19. + i as f64 * 47.).rem_euclid(530.);
                s.line((x, y), (x, y + 5.), mix(theme.paper, edge, 0.35), 1);
            }
        }
        Stage::Dungeon | Stage::Ruin => {
            let ruined = stage == Stage::Ruin;
            // Block courses behind deep arched bays. Missing blocks and leaning piers arrive at D25.
            for row in 0..9 {
                let y = 144. + row as f64 * 68.;
                s.line((0., y), (1280., y), mix(theme.paper, edge, 0.4), 1);
                for col in 0..9 {
                    let x = col as f64 * 164. + if row % 2 == 0 { 0. } else { 82. };
                    s.line(
                        (x, y),
                        (x + if ruined { (row % 3) as f64 * 7. } else { 0. }, y + 68.),
                        mix(theme.paper, edge, 0.4),
                        1,
                    );
                }
            }
            for i in 0..5 {
                let x = 110. + i as f64 * 265.;
                let r = 88. + if ruined { (i % 2) as f64 * 18. } else { 0. };
                let y = 310. + if ruined { ((i * 71) % 90) as f64 } else { 0. };
                let lean = if ruined {
                    (i as f64 - 2.) * (14. + (depth - 23) as f64 * 8.)
                } else {
                    0.
                };
                s.arch(x, y, r, 738., lean, ruined);
                if ruined {
                    s.line((x - 50., y - 120.), (x + 20., y + 28.), edge, 2);
                    s.line((x + 20., y + 28.), (x - 9., y + 94.), edge, 2);
                }
            }
            let glow = (t * 0.9).sin() * 0.025 + 0.08;
            for i in 0..4 {
                let x = 240 + i * 265;
                s.p.rect(x, 382, 5, 21, mix(theme.paper, [211, 99, 38], glow));
            }
            if ruined {
                // Fallen masonry and a split floor replace the orderly baseline.
                for i in 0..12 {
                    let x = i as f64 * 112.;
                    s.polygon(
                        &[
                            (x, 738.),
                            (x + 22., 690. - (i % 3) as f64 * 24.),
                            (x + 74., 701.),
                            (x + 106., 738.),
                        ],
                        stone,
                    );
                }
                s.polygon(
                    &[
                        (570., 738.),
                        (621., 625.),
                        (598., 574.),
                        (652., 499.),
                        (635., 612.),
                        (657., 653.),
                        (615., 738.),
                    ],
                    shadow,
                );
            }
        }
        Stage::Hell => {
            // Basalt horizon, a branching molten chasm, and distant infernal archways.
            for i in 0..3 {
                s.arch(
                    150. + i as f64 * 490.,
                    350.,
                    72.,
                    625.,
                    (i as f64 - 1.) * 15.,
                    true,
                );
            }
            let lava = mix(
                theme.paper,
                [222, 62, 12],
                0.30 + (depth - 29) as f64 * 0.015 + (t * 0.5).sin() * 0.018,
            );
            s.polygon(
                &[
                    (612., 390.),
                    (640., 390.),
                    (659., 490.),
                    (694., 530.),
                    (681., 585.),
                    (803., 738.),
                    (425., 738.),
                    (570., 581.),
                    (588., 501.),
                ],
                lava,
            );
            for i in 0..8 {
                let x = if i < 4 {
                    i as f64 * 145. - 50.
                } else {
                    850. + (i - 4) as f64 * 145.
                };
                let h = 90. + ((i * 73 + depth * 11) % 240) as f64;
                s.polygon(
                    &[
                        (x, 738.),
                        (x + 12., 738. - h),
                        (x + 40., 710. - h),
                        (x + 65., 738. - h),
                        (x + 81., 595.),
                        (x + 132., 738.),
                    ],
                    stone,
                );
                s.line((x + 12., 738. - h), (x + 40., 710. - h), edge, 2);
            }
            for i in 0..22 {
                let x = 47. + i as f64 * 57. + (t * 0.3 + i as f64).sin() * 8.;
                let y = 735. - (t * 23. + i as f64 * 29.).rem_euclid(340.);
                s.line(
                    (x, y),
                    (x + 1., y - 3.),
                    mix(theme.paper, [242, 108, 27], 0.24),
                    2,
                );
            }
        }
        Stage::Nightmare => {
            // Incompatible vanishing points and nested skewed gates; no single Euclidean room.
            let severity = (depth - 34) as f64 / 5.;
            let drift = (t * 0.15).sin() * 9. * severity;
            for i in (0..6).rev() {
                let r = 60. + i as f64 * 77.;
                let x = 640. + (i as f64 * 1.3).sin() * 50. * severity + drift;
                let y = 430. + (i as f64 * 0.9).cos() * 24. * severity;
                let skew = (i as f64 - 2.5) * 23. * severity;
                let pts = [
                    (x - r, y - r * 0.62),
                    (x + r, y - r * 0.62 + skew),
                    (x + r * 0.82, y + r * 0.62),
                    (x - r, y + r * 0.62 - skew),
                ];
                s.polygon(&pts, if i % 2 == 0 { shadow } else { stone });
                for j in 0..4 {
                    s.line(pts[j], pts[(j + 1) % 4], edge, 2);
                }
            }
            // Stair flights climb toward three mutually incompatible horizons.
            for side in 0..3 {
                for step in 0..(6 + (depth - 35) * 2) {
                    let q = step as f64;
                    let (x, y) = match side {
                        0 => (30. + q * 27., 700. - q * 17.),
                        1 => (930. + q * 22., 220. + q * 23.),
                        _ => (760. - q * 24., 690. - q * 15.),
                    };
                    let w = 70. - q * 2.5;
                    s.polygon(
                        &[
                            (x, y),
                            (x + w, y - 20.),
                            (x + w + 14., y - 9.),
                            (x + 14., y + 12.),
                        ],
                        stone,
                    );
                    s.line((x, y), (x + w, y - 20.), edge, 2);
                    s.line((x, y), (x + 14., y + 12.), edge, 1);
                }
            }
            // Hanging inverted pillars break the ground/ceiling relationship.
            for i in 0..4 {
                let x = 60. + i as f64 * 390.;
                s.polygon(
                    &[
                        (x, 136.),
                        (x + 45., 136.),
                        (x + 36., 270. + i as f64 * 17.),
                        (x + 12., 285. + i as f64 * 17.),
                    ],
                    stone,
                );
            }
        }
    }
    s.p.clip = old_clip;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_depth_environment_is_repeatable_animated_and_clipped() {
        let mut p = Painter::new();
        p.resize(320, 200);
        for depth in 0..40 {
            let theme = Theme::at(depth);
            p.clear(theme.paper);
            paint(&mut p, depth, 0., &theme);
            let first = p.pixels.clone();
            p.clear(theme.paper);
            paint(&mut p, depth, 0., &theme);
            assert_eq!(first, p.pixels);
            p.clear(theme.paper);
            paint(&mut p, depth, 6000., &theme);
            assert_ne!(first, p.pixels, "static environment depth {}", depth + 1);
            assert_eq!(&first[..320 * 34 * 4], &p.pixels[..320 * 34 * 4]);
            assert!(p.clip.is_none());
            for time in [f64::MAX, f64::NAN, f64::INFINITY, -1.] {
                p.clear(theme.paper);
                paint(&mut p, depth, time, &theme);
                assert_eq!(p.pixels.len(), 320 * 200 * 4);
            }
        }
    }
    #[test]
    fn requested_depth_landmarks_are_explicit() {
        assert_eq!(stage(0), Stage::Cave);
        assert_eq!(stage(14), Stage::Dungeon);
        assert_eq!(stage(24), Stage::Ruin);
        for d in 29..=34 {
            assert_eq!(stage(d), Stage::Hell);
        }
        assert_eq!(stage(39), Stage::Nightmare);
    }
}

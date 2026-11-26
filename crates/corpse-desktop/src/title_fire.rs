//! Stateless, presentation-only upward flow. No gameplay RNG or history.

use super::Picture;

const WIDTH: usize = 320;
const HEIGHT: usize = 192;
pub(super) fn elapsed_frames(frame: u32) -> f64 {
    f64::from(frame.saturating_sub(18))
}

const RAMP: [[u8; 3]; 15] = [
    [12, 9, 11], [20, 10, 13], [31, 11, 15], [46, 13, 16], [66, 15, 17],
    [86, 18, 17], [108, 23, 17], [134, 32, 17], [158, 44, 18], [181, 61, 21],
    [204, 81, 25], [221, 107, 32], [232, 137, 45], [242, 171, 64], [250, 204, 98],
];

pub(super) fn grain(x: i32, y: i64, seed: u64) -> f32 {
    let h = (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (y as u64).wrapping_mul(0xd1b5_4a32_d192_ed03) ^ seed;
    let h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((h ^ (h >> 31)) >> 40) as f32 / 16_777_215.
}

fn lerp(a: f32, b: f32, f: f32) -> f32 { a + (b - a) * f }
fn ease(t: f32) -> f32 { t * t * (3. - 2. * t) }

// Smoothly cross successive, unwrapped lattice rows. Split in f64 once per
// source row so fractional motion is still accurate after a long uptime.
#[derive(Clone, Copy)]
struct NoiseRow { y: i64, mix: f32 }
impl NoiseRow {
    fn new(y: f64) -> Self {
        let floor = y.floor();
        Self { y: floor as i64, mix: ease((y - floor) as f32) }
    }
    fn at(self, x: f32, seed: u64) -> f32 {
        let ix = x.floor() as i32;
        let fx = ease(x - ix as f32);
        lerp(
            lerp(grain(ix, self.y, seed), grain(ix + 1, self.y, seed), fx),
            lerp(grain(ix, self.y + 1, seed), grain(ix + 1, self.y + 1, seed), fx),
            self.mix,
        )
    }
}

/// Five bounded noise layers per sample, O(W H) time and RGBA output memory.
/// Art-directed edge banks frame the composition; there are no cutout tongues
/// or foreground triangles. Heat is carried upward from a common fire bed.
pub(super) fn picture(frame: u32) -> Picture {
    let time = 18. + elapsed_frames(frame) * 0.5;
    let breath_row = NoiseRow::new(time * 0.045);
    let mut rgba = vec![0; WIDTH * HEIGHT * 4];
    let columns: [(f32, f32); WIDTH] = std::array::from_fn(|x| {
        let px = x as f32;
        let x = px / (WIDTH - 1) as f32;
        let left = (-x / 0.17).exp();
        let right = (-(1. - x) / 0.175).exp();
        let vents = (-((x - 0.083) / 0.067).powi(2)).exp()
            + (-((x - 0.91) / 0.068).powi(2)).exp();
        let breath = breath_row.at(px * 0.037, 941);
        let height = (33. + 144. * left + 144. * right + 43. * vents) * (0.72 + 0.50 * breath);
        let edge_light = ((x - 0.5).abs() * 2.).powi(3);
        (height, edge_light)
    });
    for y in 0..HEIGHT {
        let rise = (HEIGHT - 1 - y) as f32;
        let lift = f64::from(rise) - time * (1.65 + 0.004 * f64::from(rise));
        let curl_row = NoiseRow::new(lift * 0.028);
        let large_row = NoiseRow::new(lift * 0.025);
        let ribbon_row = NoiseRow::new(lift * 0.042);
        let detail_row = NoiseRow::new(lift * 0.086);
        for (x, &(height, edge_light)) in columns.iter().enumerate() {
            let px = x as f32;
            let curl = curl_row.at(px * 0.029, 8191) - 0.5;
            let wx = px + curl * (17. + rise * 0.13);
            let large = large_row.at(wx * 0.062, 713);
            let ribbons = ribbon_row.at(wx * 0.119, 331);
            let small = detail_row.at(wx * 0.22, 37);
            let body = 0.19 + 0.75 * large + 0.58 * ribbons + 0.17 * small - rise / height;
            let bottom = (1. - rise / 65.).max(0.);
            let hearth = (0.27 + edge_light * 0.39) * bottom * (0.57 + 0.65 * small);
            let lick = ((ribbons - 0.53) * 1.65).max(0.) * (0.35 + 0.65 * small);
            let heat = body * (0.29 + 0.78 * edge_light) + hearth - 0.18 + lick * 0.37;
            let index = (heat.max(0.) * (RAMP.len() - 1) as f32) as usize;
            let [r, g, b] = RAMP[index.min(RAMP.len() - 1)];
            let at = (y * WIDTH + x) * 4;
            rgba[at..at + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
    Picture { w: WIDTH, h: HEIGHT, rgba }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flame_field_is_opaque_deterministic_and_not_a_still_overlay() {
        let first = picture(18);
        assert_eq!((first.w, first.h), (WIDTH, HEIGHT));
        assert_eq!(first.rgba, picture(18).rgba);
        assert!(first.rgba.chunks_exact(4).all(|px| px[3] == 255));
        let next = picture(19);
        let changed = first.rgba.chunks_exact(4)
            .zip(next.rgba.chunks_exact(4)).filter(|(a, b)| a != b).count();
        assert!(changed > WIDTH * HEIGHT / 100, "only {} changing pixels", changed);
    }

    #[test]
    fn fire_keeps_evolving_without_restarting_at_the_old_loop_boundary() {
        let energy = |a: &Picture, b: &Picture| {
            a.rgba.iter().zip(&b.rgba).map(|(a,b)| a.abs_diff(*b) as usize).sum::<usize>()
        };
        for end in [18 + 360, 18 + 720, 108_000, u32::MAX - 2] {
            let a = picture(end - 2);
            let b = picture(end - 1);
            let c = picture(end);
            let d = picture(end + 1);
            let step = energy(&b, &c);
            let adjacent = energy(&a, &b).max(energy(&c, &d));
            assert!(step > 0 && step * 2 <= adjacent * 3, "frame {end}: {step} vs {adjacent}");
            assert!(energy(&picture(end - 360), &c) > 10 * step);
        }
    }

    #[test]
    fn new_noise_tiles_join_continuously_in_both_directions() {
        for boundary in [-100_000., -1., 0., 1., 100_000.] {
            let sample = |y| NoiseRow::new(y).at(4.3, 713);
            for delta in [-0.001, 0.001] {
                assert!((sample(boundary + delta) - sample(boundary)).abs() < 0.000_01);
            }
        }
    }

    #[test]
    fn tall_outer_banks_frame_a_dark_title_lane() {
        let mut tall_banks = [false; 2];
        for frame in (18..240).step_by(17) {
            let p = picture(frame);
            let red = |x: usize, y: usize| p.rgba[(y * WIDTH + x) * 4];
            for y in 0..HEIGHT / 3 {
                tall_banks[0] |= (0..WIDTH / 10).any(|x| red(x, y) >= 108);
                tall_banks[1] |= (WIDTH * 9 / 10..WIDTH).any(|x| red(x, y) >= 108);
                assert!((WIDTH / 4..WIDTH * 3 / 4).all(|x| red(x, y) <= 46));
            }
        }
        assert_eq!(tall_banks, [true, true]);
    }
}

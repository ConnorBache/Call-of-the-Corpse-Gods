//! Stateless, presentation-only upward flow. No gameplay RNG or history.

use super::Picture;

const WIDTH: usize = 320;
const HEIGHT: usize = 192;
const RAMP: [[u8; 3]; 15] = [
    [12, 9, 11], [20, 10, 13], [31, 11, 15], [46, 13, 16], [66, 15, 17],
    [86, 18, 17], [108, 23, 17], [134, 32, 17], [158, 44, 18], [181, 61, 21],
    [204, 81, 25], [221, 107, 32], [232, 137, 45], [242, 171, 64], [250, 204, 98],
];

fn grain(x: i32, y: i32, seed: u32) -> f32 {
    let h = (x as u32).wrapping_mul(0x9e37_79b9)
        ^ (y as u32).wrapping_mul(0x85eb_ca6b) ^ seed;
    let h = (h ^ (h >> 16)).wrapping_mul(0x7feb_352d);
    let h = (h ^ (h >> 15)).wrapping_mul(0x846c_a68b);
    ((h ^ (h >> 16)) >> 8) as f32 / 16_777_215.
}

fn lerp(a: f32, b: f32, f: f32) -> f32 { a + (b - a) * f }
fn ease(t: f32) -> f32 { t * t * (3. - 2. * t) }

fn flow(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = ease(x - ix as f32);
    let fy = ease(y - iy as f32);
    lerp(
        lerp(grain(ix, iy, seed), grain(ix + 1, iy, seed), fx),
        lerp(grain(ix, iy + 1, seed), grain(ix + 1, iy + 1, seed), fx),
        fy,
    )
}

/// Five bounded noise layers per sample, O(W H) time and RGBA output memory.
/// Art-directed edge banks frame the composition; there are no cutout tongues
/// or foreground triangles. Heat is carried upward from a common fire bed.
pub(super) fn picture(frame: u32) -> Picture {
    let time = 18. + frame.saturating_sub(18) as f32 * 0.50;
    let mut rgba = vec![0; WIDTH * HEIGHT * 4];
    let columns: [(f32, f32); WIDTH] = std::array::from_fn(|x| {
        let px = x as f32;
        let x = px / (WIDTH - 1) as f32;
        let left = (-x / 0.17).exp();
        let right = (-(1. - x) / 0.175).exp();
        let vents = (-((x - 0.083) / 0.067).powi(2)).exp()
            + (-((x - 0.91) / 0.068).powi(2)).exp();
        let breath = flow(px * 0.037, time * 0.045, 941);
        let height = (33. + 144. * left + 144. * right + 43. * vents) * (0.72 + 0.50 * breath);
        let edge_light = ((x - 0.5).abs() * 2.).powi(3);
        (height, edge_light)
    });
    for y in 0..HEIGHT {
        let rise = (HEIGHT - 1 - y) as f32;
        for (x, &(height, edge_light)) in columns.iter().enumerate() {
            let px = x as f32;
            let lift = rise - time * (1.65 + 0.004 * rise);
            let curl = flow(px * 0.029, lift * 0.028, 8191) - 0.5;
            let wx = px + curl * (17. + rise * 0.13);
            let large = flow(wx * 0.062, lift * 0.025, 713);
            let ribbons = flow(wx * 0.119, lift * 0.042, 331);
            let small = flow(wx * 0.22, lift * 0.086, 37);
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

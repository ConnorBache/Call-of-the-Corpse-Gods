//! R51 displayed-sRGB illumination. Pure presentation values, never simulation.
use crate::render::Rgb;

pub const LEVELS: [f64; 9] = [-100., -75., -50., -25., 0., 25., 50., 75., 100.];
pub const MULTIPLIERS: [f64; 9] = [0.05, 0.20, 0.40, 0.65, 1., 1.50, 2.05, 2.70, 3.40];
pub const WARM: Rgb = [255, 180, 100];
pub const TINT_STRENGTH: f64 = 0.35;

/// A detached display transform, shared by software and GPU draw commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayLight {
    pub multiplier: f64,
    /// Unit-sRGB source hue, averaged separately from scalar brightness.
    pub tint: [f64; 3],
}
impl DisplayLight {
    pub fn neutral(illumination: f64) -> Self {
        Self { multiplier: multiplier(illumination), tint: [1.; 3] }
    }
    pub fn apply(self, color: Rgb) -> Rgb {
        // One common gamut scale retains material hue instead of independently
        // clipping channels to white. Zero-light neutral grading is byte-exact.
        let peak = color.into_iter().max().unwrap_or(0) as f64;
        if peak == 0. { return color; }
        let target_peak = peak * self.multiplier.max(0.);
        let knee = peak.max(220.);
        let span = 255. - knee;
        let displayed_peak = if target_peak <= knee { target_peak }
            else if span <= 0. { 255. }
            else { 255. - span / (1. + (target_peak - knee) / span) };
        let gain = displayed_peak / peak;
        let base = color.map(|v| v as f64 * gain);
        if self.tint == [1.; 3] { return rounded(base); }
        let tinted = std::array::from_fn(|i| {
            base[i] * (1. - TINT_STRENGTH + TINT_STRENGTH * self.tint[i].clamp(0., 1.))
        });
        // Normalize actual linear-sRGB luminance, not source RGB energy. Thus a
        // warm source changes material hue without adding a second exposure.
        let neutral = base.map(decode);
        let target = luma(neutral);
        let linear = tinted.map(decode);
        let normalization = target / luma(linear).max(f64::MIN_POSITIVE);
        let normalized = linear.map(|v| v * normalization);
        // Both endpoints have exactly the target luminance. Reduce tint only
        // where the gamut requires it, never dim the result to fit a channel.
        let mut tint_amount: f64 = 1.;
        for i in 0..3 {
            if normalized[i] > 1. {
                tint_amount = tint_amount.min((1. - neutral[i]).max(0.) / (normalized[i] - neutral[i]));
            }
        }
        rounded(std::array::from_fn(|i| encode(neutral[i] + tint_amount * (normalized[i] - neutral[i]))))
    }
}
fn rounded(color: [f64; 3]) -> Rgb { color.map(|v| (v + 1e-12).round().clamp(0., 255.) as u8) }
fn decode(v: f64) -> f64 {
    let v = v / 255.;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}
fn encode(v: f64) -> f64 {
    255. * if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1. / 2.4) - 0.055 }
}
fn luma(c: [f64; 3]) -> f64 { c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722 }

/// O(1) time/scratch: eight fixed monotone Hermite intervals, no table allocation.
/// C1 tails remain unclamped: exponential darkness and logarithmic brightening.
pub fn multiplier(light: f64) -> f64 {
    if light.is_nan() { return 1.; }
    if light < -100. { return 0.05 * ((light + 100.) / 10.).exp(); }
    if light > 100. { return 3.4 + 1.45 * ((light - 100.) / 50.).ln_1p(); }
    if light == 100. { return 3.4; }
    let i = ((light + 100.) / 25.).floor() as usize;
    let slope = |j: usize| (MULTIPLIERS[j + 1] - MULTIPLIERS[j]) / 25.;
    let tangent = |j: usize| {
        if j == 0 { 0.005 }
        else if j == 8 { 0.029 }
        else { 2. * slope(j - 1) * slope(j) / (slope(j - 1) + slope(j)) }
    };
    let t = (light - LEVELS[i]) / 25.;
    let t2 = t * t;
    let t3 = t2 * t;
    (2. * t3 - 3. * t2 + 1.) * MULTIPLIERS[i]
        + (t3 - 2. * t2 + t) * 25. * tangent(i)
        + (-2. * t3 + 3. * t2) * MULTIPLIERS[i + 1]
        + (t3 - t2) * 25. * tangent(i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibration_knots_floor_pixels_smooth_monotone_tails() {
        let pixels = [[2,1,2],[7,6,6],[14,11,13],[22,18,21],[34,28,32],
            [51,42,48],[70,57,66],[92,76,86],[116,95,109]];
        for ((light, expected), pixel) in LEVELS.into_iter().zip(MULTIPLIERS).zip(pixels) {
            assert!((multiplier(light) - expected).abs() < 1e-12);
            assert_eq!(DisplayLight::neutral(light).apply([34,28,32]), pixel);
            let h = 0.00001;
            let left = (multiplier(light) - multiplier(light - h)) / h;
            let right = (multiplier(light + h) - multiplier(light)) / h;
            assert!((left - right).abs() < 0.000001, "C1 at {light}");
        }
        for i in -2000..4000 { assert!(multiplier(i as f64 / 10.) < multiplier((i + 1) as f64 / 10.)); }
        assert!((multiplier(-150.) - 0.00033689734995427336).abs() < 1e-15);
        assert!((multiplier(150.) - 4.405063411811921).abs() < 1e-12);
        assert!((multiplier(200.) - 4.992987818568759).abs() < 1e-12);
        assert!(multiplier(250.) - multiplier(200.) < multiplier(200.) - multiplier(150.));
    }
    #[test]
    fn neutral_preserves_hues_and_warm_preserves_luminance() {
        for rgb in [[34,28,32],[31,70,27],[10,35,90],[45,21,16],[255,255,255],[255,128,64],[90,255,130]] {
            for level in [-50.,0.,25.,50.,80.,100.] {
                let neutral = DisplayLight::neutral(level);
                let warm = DisplayLight { tint: WARM.map(|v| v as f64 / 255.), ..neutral };
                let a = neutral.apply(rgb); let b = warm.apply(rgb);
                let la=luma(a.map(|v|decode(v as f64)));let lb=luma(b.map(|v|decode(v as f64)));
                assert!((la-lb).abs() < 0.005, "{rgb:?}/{level}: {a:?} vs {b:?}");
                if rgb[2] > rgb[0] { assert!(a[2]>a[0] && b[2]>b[0]); }
                if rgb[1] > rgb[0] { assert!(a[1]>a[0] && b[1]>b[0]); }
            }
        }
        assert_eq!(DisplayLight::neutral(0.).apply([34,28,32]),[34,28,32]);
        let extreme = DisplayLight::neutral(1000000.).apply([100,50,25]);
        assert!(extreme[0] >= 254 && extreme[1] >= 127 && extreme[2] >= 63);
        assert_ne!(DisplayLight::neutral(75.).apply([140,117,86]),DisplayLight::neutral(100.).apply([140,117,86]), "bright custom materials retain highlight headroom");
        assert_eq!(DisplayLight::neutral(0.).apply([240,230,220]),[240,230,220]);
    }
}

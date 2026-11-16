//! R51 displayed-sRGB illumination. Pure presentation values, never simulation.
use crate::render::Rgb;

pub const LEVELS: [f64; 9] = [-100., -75., -50., -25., 0., 25., 50., 75., 100.];
pub const MULTIPLIERS: [f64; 9] = [0.05, 0.15, 0.30, 0.55, 1., 1.50, 2.05, 2.70, 3.40];
pub const WARM: Rgb = [255, 180, 100];
pub const TINT_STRENGTH: f64 = 0.35;

/// A detached display transform, shared by software and GPU draw commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayLight {
    pub multiplier: f64,
    pub illumination: f64,
    /// Unit-sRGB source hue, averaged separately from scalar brightness.
    pub tint: [f64; 3],
}
impl DisplayLight {
    pub fn neutral(illumination: f64) -> Self {
        Self { multiplier: multiplier(illumination), illumination, tint: [1.; 3] }
    }
    /// Separate negative-light ink exposure. Never changes visibility or adds light.
    pub fn foreground(self, color: Rgb, creature: bool) -> Rgb {
        if self.illumination >= 0. || self.illumination.is_nan() { return self.apply(color); }
        let values=if creature {[0.40,0.55,0.70,0.85,1.]} else {[0.25,0.40,0.55,0.75,1.]};
        let retention=negative_curve(self.illumination,[0.10,0.25,0.50,0.75,1.]);
        let retention=if creature {retention+(1.-retention)*0.15} else {retention};
        let graded=Self {multiplier:negative_curve(self.illumination,values),..self}.apply_exposure(color);
        desaturate(graded,retention)
    }
    pub fn apply(self, color: Rgb) -> Rgb {
        let graded=self.apply_exposure(color);
        if self.illumination<0. {
            // Apply after source tint so a warm source cannot recolor gray darkness.
            desaturate(graded,negative_curve(self.illumination,[0.10,0.25,0.50,0.75,1.]))
        }else{graded}
    }
    fn apply_exposure(self, color:Rgb)->Rgb {
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

/// Five fixed knots, monotone cubic Hermite interpolation; zero-slope dark tail.
/// O(1) time/scratch. Beyond -100 retains readability rather than hiding admitted ink.
fn negative_curve(level:f64, values:[f64;5])->f64 {
    if level<=-100. {return values[0];}if level>=0. {return values[4];}
    let i=((level+100.)/25.).floor()as usize;let t=(level+100.-i as f64*25.)/25.;
    let slope=|j:usize|(values[j+1]-values[j])/25.;
    let tangent=|j:usize|if j==0 {0.}else if j==4 {slope(3)}else{2.*slope(j-1)*slope(j)/(slope(j-1)+slope(j))};
    (2.*t*t*t-3.*t*t+1.)*values[i]+(t*t*t-2.*t*t+t)*25.*tangent(i)
        +(-2.*t*t*t+3.*t*t)*values[i+1]+(t*t*t-t*t)*25.*tangent(i+1)
}

/// O(1) time/scratch: eight fixed monotone Hermite intervals, no table allocation.
/// C1 tails remain unclamped: exponential darkness and logarithmic brightening.
pub fn multiplier(light: f64) -> f64 {
    if light.is_nan() { return 1.; }
    if light < -100. { return 0.05 * ((light + 100.) / 12.5).exp(); }
    if light > 100. { return 3.4 + 1.45 * ((light - 100.) / 50.).ln_1p(); }
    if light == 100. { return 3.4; }
    let i = ((light + 100.) / 25.).floor() as usize;
    let slope = |j: usize| (MULTIPLIERS[j + 1] - MULTIPLIERS[j]) / 25.;
    let tangent = |j: usize| {
        if j == 0 { 0.004 }
        else if j == 8 { 0.029 }
        else if j == 4 { 2. * 0.014 * 0.020 / (0.014 + 0.020) }
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
        let pixels = [[1,1,1],[4,4,4],[9,8,9],[18,15,18],[34,28,32],
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
        assert!((multiplier(-150.) - 0.000915781944436709).abs() < 1e-15);
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

#[cfg(test)]
mod negative_role_tests {
    use super::*;
    #[test]
    fn exact_negative_knots_roles_and_desaturation() {
        for (i,level) in [-100.,-75.,-50.,-25.,0.].into_iter().enumerate() {
            let display=DisplayLight::neutral(level);
            let expected_bg=[0.05,0.15,0.30,0.55,1.][i];
            assert!((display.multiplier-expected_bg).abs()<1e-12);
            let terrain=display.foreground([100;3],false);let creature=display.foreground([100;3],true);
            assert_eq!(terrain,[[25;3],[40;3],[55;3],[75;3],[100;3]][i]);
            assert_eq!(creature,[[40;3],[55;3],[70;3],[85;3],[100;3]][i]);
        }
        let c=DisplayLight::neutral(-100.).foreground([180,80,40],false);
        assert!(c[0]-c[2]<8,"near-black terrain has muted hue: {c:?}");
        for level in [0.,1.,12.5,25.,50.,75.,100.,200.] {
            let d=DisplayLight::neutral(level);assert_eq!(d.apply([90,40,20]),d.foreground([90,40,20],false));
            assert_eq!(d.apply([90,40,20]),d.foreground([90,40,20],true));
        }
    }
}

/// O(12) bounded channel-gain search, O(1) scratch. Preserve the already-muted
/// hue while lifting foreground contrast; whiten only if its gamut cannot reach it.
pub fn foreground_contrast(ink:Rgb,bg:Rgb,target:f64)->Rgb {
    if crate::color::contrast(ink,bg)>=target {return ink;}
    let peak=ink.into_iter().max().unwrap_or(0);
    if peak==0 {return crate::color::distinguish(ink,bg,target);}
    let scale=|gain:f64|ink.map(|v|(v as f64*gain).round().min(255.)as u8);
    let(mut lo,mut hi)=(1.,255./peak as f64);
    if crate::color::contrast(scale(hi),bg)<target {return crate::color::distinguish(ink,bg,target);}
    for _ in 0..12 {let mid=(lo+hi)/2.;if crate::color::contrast(scale(mid),bg)<target {lo=mid;}else{hi=mid;}}
    scale(hi)
}

fn desaturate(color:Rgb,retention:f64)->Rgb {
    let gray=color[0]as f64*0.2126+color[1]as f64*0.7152+color[2]as f64*0.0722;
    rounded(color.map(|v|gray+(v as f64-gray)*retention))
}

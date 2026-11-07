//! Presentation-only timing for independently composited heart and vein layers.
//! No simulation state, time source, allocation, image decode, or gameplay RNG.

fn bump(t: f64, start: f64, duration: f64) -> f64 {
    let phase = (t - start) / duration;
    if !(0. ..1.).contains(&phase) { return 0.; }
    (std::f64::consts::PI * phase).sin().powi(2)
}
fn time(frame: u32) -> f64 {
    ((frame.saturating_sub(18) as u64 * 33) % 1080) as f64
}
/// Double heartbeat followed by a longer rest. Frame 18 is reduced-motion rest.
pub fn pulse(frame: u32) -> f64 {
    let t = time(frame);
    bump(t, 50., 240.) + 0.62 * bump(t, 330., 210.)
}
/// A delayed pressure front travels outward from the heart along visible veins.
/// Distance is measured in source-pixel space, normalized by artwork height.
/// Separate waves follow the two contractions, rather than blinking every vein
/// simultaneously. Reduced-motion baseline has no wave, including wraparound.
pub fn vein_pulse(frame: u32, distance: f64) -> f64 {
    if frame == 18 { return 0.; }
    let delay = distance.max(0.) * 760.;
    let t = time(frame);
    (bump(t, 170. + delay, 170.) + 0.62 * bump(t, 450. + delay, 150.)).min(1.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_frame_and_long_rest_are_still() {
        assert_eq!(pulse(18), 0.);
        assert_eq!(pulse(38), 0.);
        for d in [0., 0.1, 0.3, 0.6] { assert_eq!(vein_pulse(18, d), 0.); }
    }
    #[test]
    fn double_beat_is_bounded_and_periodic() {
        assert!(pulse(23) > 0.99);
        assert!(pulse(31) > 0.60 && pulse(31) < 0.63);
        assert_eq!(pulse(28), 0.);
        for frame in 18..2000 {
            assert!((0. ..=1.).contains(&pulse(frame)));
            assert!((pulse(frame) - pulse(frame + 360)).abs() < 1e-10);
            for d in [0., 0.2, 0.6] { assert!((0. ..=1.).contains(&vein_pulse(frame, d))); }
        }
    }
    #[test]
    fn blood_wave_follows_heart_and_reaches_distant_veins_later() {
        assert_eq!(vein_pulse(23, 0.), 0.);
        assert!(vein_pulse(26, 0.) > 0.9);
        assert_eq!(vein_pulse(26, 0.4), 0.);
        assert!(vein_pulse(35, 0.4) > 0.9);
        assert!(vein_pulse(35, 0.) < vein_pulse(35, 0.4));
    }
}

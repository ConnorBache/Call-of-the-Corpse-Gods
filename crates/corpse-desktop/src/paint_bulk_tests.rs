//! Exact scalar-reference regressions for batch primitives. No wall-clock assertions.
use super::*;
/// O(resolutions * clips * alpha cases * framebuffer area); fixed test sizes.
#[test]
fn bulk_fill_matches_scalar_for_every_alpha_and_clip() {
    let mut p=Painter::new();
    for (width,height) in [(39,23),(80,50),(192,108),(90,72)] {
        p.resize(width,height);
        for clip in [None,Some((-5,3,100,80)),Some((4,7,0,10)),Some((8,9,1,1))] {
            p.clip=clip;
            // Restrict touched region; include arbitrary destination alpha, not just opaque frames.
            for a in 0..=255u8 {
                p.pixels.fill(a.wrapping_mul(37));
                let before=p.pixels.clone();
                let c=[a,255-a,131];
                p.native_fill(-3,2,17,19,c,a);
                let actual=p.pixels.clone();
                p.pixels=before;
                for y in 2..19 {for x in -3..17 {p.native_pixel(x,y,c,a);}}
                assert_eq!(actual,p.pixels,"{width}x{height} clip={clip:?} alpha={a}");
            }
        }
    }
}
/// O(heights * 256) time, O(1) memory; validates the exact Chronicle lookup identity.
#[test]
fn fade_tables_match_original_for_every_channel_value() {
    for h in [27,108,146,195] {for y in 0..h {
        let a=crate::chronicle::fade(y as f64/h as f64);
        for background in [[0,0,0],[9,13,21],[255,127,1]] {
            for v in 0..=255u8 {
                let reference=crate::color::composite(background,[v;3],a);
                for k in 0..3 {
                    let value=(background[k] as f64*(1.-a)+v as f64*a).clamp(0.,255.).round() as u8;
                    assert_eq!(reference[k],value);
                }
            }
        }
    }}
}

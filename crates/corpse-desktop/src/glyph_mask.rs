//! Native glyph alpha coverage. No colors, display state or simulation access.

// Fourfold device-relative outline sampling avoids enlarging the tiny masks
// used by taps, pits and arrows. Only the final device-size mask is retained.
pub(super) const OVERSAMPLE: f32 = 4.;

/// Area-average a cropped alpha mask, preserving partially covered edges instead
/// of selecting a single texel. O((source_w + width) * (source_h + height)) time,
/// O(width * height) output memory. In the downsampling caller this is O(S + B)
/// for source/output pixel counts S/B; mixed-axis arbitrary resizing is not.
pub(super) fn coverage_resize(
    source: &[u8], stride: usize, left: usize, top: usize,
    source_w: usize, source_h: usize, width: usize, height: usize,
) -> Vec<u8> {
    assert!(source_w > 0 && source_h > 0 && width > 0 && height > 0);
    assert!(left + source_w <= stride && (top + source_h) * stride <= source.len());
    let mut output = Vec::with_capacity(width * height);
    let sx = source_w as f64 / width as f64;
    let sy = source_h as f64 / height as f64;
    for y in 0..height {
        let y0 = y as f64 * sy;
        let y1 = (y + 1) as f64 * sy;
        for x in 0..width {
            let x0 = x as f64 * sx;
            let x1 = (x + 1) as f64 * sx;
            let mut coverage = 0.;
            for yy in y0.floor() as usize..(y1.ceil() as usize).min(source_h) {
                let wy = (y1.min((yy + 1) as f64) - y0.max(yy as f64)).max(0.);
                for xx in x0.floor() as usize..(x1.ceil() as usize).min(source_w) {
                    let wx = (x1.min((xx + 1) as f64) - x0.max(xx as f64)).max(0.);
                    coverage += source[(top + yy) * stride + left + xx] as f64 * wx * wy;
                }
            }
            output.push((coverage / (sx * sy)).round().clamp(0., 255.) as u8);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::coverage_resize;
    #[test]
    fn area_coverage_preserves_partial_strokes_and_crop() {
        let p = [99,99,99,99,99, 99,0,255,0,99, 99,0,255,0,99, 99,99,99,99,99];
        assert_eq!(coverage_resize(&p,5,1,1,3,2,1,1),vec![85]);
        assert_eq!(coverage_resize(&p,5,1,1,3,2,3,2),vec![0,255,0,0,255,0]);
        // Every destination pixel keeps the covered fraction, rather than
        // dropping the alternating source stroke through nearest sampling.
        assert_eq!(coverage_resize(&[0,255,0,255],4,0,0,4,1,2,1),vec![128,128]);
    }
    #[test]
    fn fractional_and_mixed_axis_samples_are_symmetric() {
        assert_eq!(coverage_resize(&[0,255,0],3,0,0,3,1,2,1),vec![85,85]);
        assert_eq!(coverage_resize(&[0,255,255,0],2,0,0,2,2,1,3),vec![128,128,128]);
        assert_eq!(coverage_resize(&[128],1,0,0,1,1,3,2),vec![128;6]);
    }
    #[test]
    fn solid_coverage_has_no_fractional_seams() {
        for (sw,sh,dw,dh) in [(17,19,9,14),(9,14,17,19),(33,7,12,13),(1,1,3,5)] {
            let output=coverage_resize(&vec![255;sw*sh],sw,0,0,sw,sh,dw,dh);
            assert_eq!(output,vec![255;dw*dh]);
        }
    }
}

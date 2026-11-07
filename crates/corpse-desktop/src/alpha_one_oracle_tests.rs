//! Independent scalar oracles for exact unit-alpha specialization.
use super::*;

// Independent scalar pixel traversal retains the OLD floating conversion. Clipping
// is tested per pixel instead of borrowing the optimized row traversal.
pub(super) fn scalar_mask(
    pixels: &mut [u8], pitch: usize, bounds: (i32, i32, i32, i32),
    x: i32, y: i32, w: usize, h: usize, mask: &[u8], color: Rgb,
    alpha: f64, force_alpha: bool, italic: bool,
    tiles: Option<&tile_damage::RowSpans>,
) {
    for yy in 0..h {
        let shear = if italic {
            ((h as i32 - 1 - 2 * yy as i32) as f64 * w as f64
                / (8. * h.max(1) as f64)).round() as i32
        } else { 0 };
        for xx in 0..w {
            let px = x + xx as i32 + shear;
            let py = y + yy as i32;
            if px < bounds.0 || py < bounds.1 || px >= bounds.2 || py >= bounds.3
                || tiles.is_some_and(|t| !t.contains(px as usize, py as usize)) { continue; }
            let at = (py as usize * pitch + px as usize) * 4;
            let coverage = mask[yy * w + xx];
            let a = if force_alpha { (coverage as f64 * alpha).round() as u8 as u32 }
                else { (coverage as f64 * alpha).round() as u32 };
            for k in 0..3 {
                pixels[at+k] = ((color[k] as u32*a + pixels[at+k] as u32*(255-a))/255) as u8;
            }
            if force_alpha { pixels[at+3] = 255; }
        }
    }
}

/// Exhaustive 256 coverage × 256 destination × 256 foreground byte values.
/// Each channel uses a permutation of the same complete input sets. O(256³)
/// time, O(256²) byte storage; no timing thresholds or asset initialization.
#[test]
fn scaled_unit_opacity_matches_old_float_for_all_channel_triples() {
    let (w,h) = (256usize,256usize);
    let mask: Vec<_> = (0..w*h).map(|i| (i % w) as u8).collect();
    let mut initial = Vec::with_capacity(w*h*4);
    for y in 0..h { for x in 0..w {
        initial.extend_from_slice(&[y as u8, 255-y as u8, (y as u8).wrapping_mul(73), (x^y) as u8]);
    }}
    let bounds = (0,0,w as i32,h as i32);
    for foreground in 0..=255u8 {
        let color = [foreground, 255-foreground, foreground.wrapping_mul(73)];
        let mut expected = initial.clone();
        scalar_mask(&mut expected,w,bounds,0,0,w,h,&mask,color,1.,true,false,None);
        let mut actual = initial.clone();
        let mut stats = damage::RasterStats { enabled:true, ..Default::default() };
        blend_scaled_mask(&mut actual,w,bounds,0,0,w,h,&mask,color,1.,&mut stats,None);
        assert_eq!(actual,expected,"foreground={foreground}");
        assert_eq!(stats.pixel_visits,(w*h) as u64);
        assert_eq!(stats.pixel_writes,(w*h) as u64);
    }
}

/// Includes partial clips, disjoint native tile spans, arbitrary destination
/// alpha, zero coverage, empty bounds and the unchanged non-unit/NaN routes.
#[test]
fn scaled_mask_clips_and_non_unit_casts_match_old_scalar() {
    let (pw,ph) = (39usize,23usize);
    let initial: Vec<_> = (0..pw*ph*4).map(|i| i.wrapping_mul(37) as u8).collect();
    let (mw,mh) = (35usize,19usize);
    let mask: Vec<_> = (0..mw*mh).map(|i| i.wrapping_mul(53) as u8).collect();
    let mut tiles = tile_damage::RowSpans::default(); tiles.reset(pw,ph);
    tiles.add(damage::Rect::new(1,2,9,18)); tiles.add(damage::Rect::new(17,0,22,11)); tiles.finish();
    for alpha in [f64::NEG_INFINITY,-1.,-0.,0.,f64::MIN_POSITIVE,0.42,
        f64::from_bits(1f64.to_bits()-1),1.,f64::from_bits(1f64.to_bits()+1),2.,f64::INFINITY,
        f64::NAN,f64::from_bits(0x7ff8_0000_0000_0001)] {
        for bounds in [(0,0,39,23),(3,4,27,16),(5,5,5,9),(8,9,3,4)] {
            for tile_mode in [false,true] {
                let t = tile_mode.then_some(&tiles);
                let mut expected=initial.clone();
                scalar_mask(&mut expected,pw,bounds,-3,2,mw,mh,&mask,[19,211,87],alpha,true,false,t);
                let mut actual=initial.clone();
                let mut stats=damage::RasterStats::default();
                blend_scaled_mask(&mut actual,pw,bounds,-3,2,mw,mh,&mask,[19,211,87],alpha,&mut stats,t);
                assert_eq!(actual,expected,"alpha bits={} bounds={bounds:?} tile={tile_mode}",alpha.to_bits());
            }
        }
    }
}

/// Exercise the ACTUAL ASCII cache-hit renderer, including italic row shear,
/// sub-native scaling, logical clipping, damage clipping and native tile spans.
/// Fixed cases, O(cases * framebuffer pixels); no real font/atlas cache misses.
#[test]
fn ascii_unit_and_near_unit_opacity_preserve_rgba_and_shear() {
    let mut p=Painter::new();
    for (pw,ph) in [(320,200),(1280,800),(1920,1080),(1537,911)] {
        p.resize(pw,ph); p.set_responsive_canvas(true);
        for logical_clip in [None,Some((2,3,28,20)),Some((2,3,0,20))] {
            p.clip=logical_clip;
            for damage_mode in [false,true] {
                p.damage_clip=damage_mode.then_some(damage::Rect::new(2,1,pw-4,ph-2));
                for tile_mode in [false,true] {
                    p.tile_clip_enabled=tile_mode;
                    p.tile_mask.reset(pw,ph);
                    p.tile_mask.add(damage::Rect::new(0,0,pw/3,ph/2));
                    p.tile_mask.add(damage::Rect::new(pw/2,ph/3,pw/2,ph-ph/3));
                    p.tile_mask.finish();
                    for italic in [false,true] {
                        for alpha in [-1.,-0.,0.,0.42,f64::from_bits(1f64.to_bits()-1),1.,
                            f64::from_bits(1f64.to_bits()+1),f64::NAN] {
                            let (x,y,lw,lh)=(-2,2,37,29);
                            let (nx,ny)=(p.native_x(x as f64),p.native_y(y as f64));
                            let (w,h)=(p.native_x((x+lw) as f64)-nx,p.native_y((y+lh) as f64)-ny);
                            let mask:Vec<_>=(0..(w*h) as usize).map(|i|i.wrapping_mul(53) as u8).collect();
                            let weight=p.profile.number("typography.map.fontWeight").round().clamp(100.,900.) as u16;
                            p.map_masks.insert(('@',w,h,weight),ascii_mask::AsciiMask::new(mask.clone(),w as usize,h as usize));
                            for(i,v)in p.pixels.iter_mut().enumerate(){*v=i.wrapping_mul(37) as u8;}
                            let mut expected=p.pixels.clone();
                            scalar_mask(&mut expected,p.width,p.native_bounds(),nx,ny,w as usize,h as usize,
                                &mask,[231,13,92],alpha,false,italic,tile_mode.then_some(&p.tile_mask));
                            p.map_glyph_styled('@',x,y,lw,lh,[231,13,92],alpha,italic);
                            assert_eq!(p.pixels,expected,"{pw}x{ph} clip={logical_clip:?} damage={damage_mode} tiles={tile_mode} italic={italic} alpha bits={}",alpha.to_bits());
                        }
                    }
                }
            }
        }
    }
}

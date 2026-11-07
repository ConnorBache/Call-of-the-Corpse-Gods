use super::*;
/// Frozen v16 scalar renderer oracle, O(full destination area) time per layer.
fn reference_emblem(painter: &mut Painter, x: i32, y: i32, w: i32, h: i32, frame: u32) {
        let layers = std::sync::Arc::clone(&painter.menu_layers);
        let beat = menu_heartbeat::pulse(frame);
        let (nx, ny) = (painter.native_x(x as f64), painter.native_y(y as f64));
        let (nw, nh) = (painter.native_x((x + w) as f64) - nx, painter.native_y((y + h) as f64) - ny);
        for (layer, p) in layers.iter().enumerate() {
            for yy in 0..nh {
                for xx in 0..nw {
                    let (u, v) = (xx as f64 / nw as f64, yy as f64 / nh as f64);
                    let (px, py) = if layer == 1 && beat != 0. {
                        // Scale only the isolated heart around its anatomical center.
                        let sx = 0.505 + (u - 0.505) / (1. + 0.06 * beat);
                        let sy = 0.633 + (v - 0.633) / (1. + 0.045 * beat);
                        if !(0. ..1.).contains(&sx) || !(0. ..1.).contains(&sy) { continue; }
                        ((sx * p.w as f64) as usize, (sy * p.h as f64) as usize)
                    } else {
                        (xx as usize * p.w / nw as usize, yy as usize * p.h / nh as usize)
                    };
                    let at = (py * p.w + px) * 4;
                    let alpha = p.rgba[at + 3];
                    if alpha == 0 { continue; }
                    let mut color = [p.rgba[at], p.rgba[at + 1], p.rgba[at + 2]];
                    if layer == 0 {
                        let dx = (u - 0.505) * p.w as f64 / p.h as f64;
                        let dy = v - 0.633;
                        let wave = menu_heartbeat::vein_pulse(frame, (dx * dx + dy * dy).sqrt());
                        for channel in &mut color {
                            *channel = ((*channel as f64 * (1. + 0.48 * wave)).round().min(255.)) as u8;
                        }
                    }
                    painter.native_pixel(nx + xx, ny + yy, color, alpha);
                }
            }
        }
    }

/// O(phases * native area) oracle comparison, O(framebuffer area) scratch.
#[test]
#[ignore = "full periodic heartbeat/vein phase and resize exactness gate"]
fn full_menu_cycle_matches_frozen_scalar() {
    let mut p=Painter::new();
    for (width,height) in [(1280,800),(1920,1080),(900,720),(720,900),(320,200)] {
        p.resize(width,height);
        let h=(500. * p.logo.h as f64/p.logo.w as f64).round() as i32;
        for frame in 18..378 {
            p.clear([17,43,91]);
            reference_emblem(&mut p,676,58,500,h,frame);
            let reference=p.pixels.clone();
            p.clear([17,43,91]);p.beating_emblem(676,58,500,h,frame);
            assert!(reference==p.pixels,"{width}x{height} phase{frame}, first differing byte {:?}",reference.iter().zip(&p.pixels).position(|(a,b)|a!=b));
        }
        for clip in [Some((700,150,200,300)),Some((0,0,0,0)),None] {
            p.clip=clip;
            for (x,y,w,frame) in [(-50,-20,400,23),(20,30,300,35),(676,58,500,18)] {
                let h=(w as f64*p.logo.h as f64/p.logo.w as f64).round() as i32;
                for (i,v) in p.pixels.iter_mut().enumerate() {*v=(i*37) as u8;}
                let initial=p.pixels.clone();reference_emblem(&mut p,x,y,w,h,frame);let reference=p.pixels.clone();
                p.pixels=initial;p.beating_emblem(x,y,w,h,frame);
                assert!(reference==p.pixels,"{width}x{height} clip{clip:?} moved/size{w}, first differing byte {:?}",reference.iter().zip(&p.pixels).position(|(a,b)|a!=b));
            }
        }
        p.clip=None;
    }
}

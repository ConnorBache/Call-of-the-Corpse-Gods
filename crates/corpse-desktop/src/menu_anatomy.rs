//! Pixel-preserving anatomical layers for the authored menu emblem.
//!
//! Coordinates are registered against the 1070 × 1470 master. These are render
//! masks, not replacement artwork: every original RGBA pixel is assigned to
//! exactly one layer. Nothing is inpainted behind the bones, and the master is
//! unchanged. A small outward pulse avoids exposing invented anatomy.
use super::Picture;

pub const VEINS: usize = 0;
pub const HEART: usize = 1;
pub const FOREGROUND: usize = 2;

// Outline of the organ, including the pale surface vessels and upper lobes.
// Long arterial trunks remain in the separately pulsing vascular layer.
const HEART_OUTLINE: &[(f64, f64)] = &[
    (509., 801.), (536., 777.), (564., 766.), (587., 774.),
    (597., 807.), (594., 839.), (622., 865.), (650., 878.),
    (650., 916.), (635., 964.), (607., 1006.), (576., 1045.),
    (553., 1064.), (530., 1056.), (510., 1028.), (489., 989.),
    (464., 962.), (444., 934.), (434., 902.), (434., 873.),
    (445., 848.), (473., 829.), (495., 823.),
];

// Broad anatomical corridors; chromatic qualification below rejects the bones
// crossing them, black negative space, and the brown/gold crown.
const VEIN_CORRIDORS: &[&[(f64, f64)]] = &[
    &[(170.,440.),(210.,397.),(285.,365.),(294.,403.),(250.,431.),
      (220.,481.),(215.,539.),(235.,592.),(290.,635.),(293.,674.),
      (240.,662.),(199.,623.),(177.,567.),(167.,507.)],
    &[(374.,479.),(368.,385.),(378.,308.),(418.,244.),(463.,213.),
      (505.,212.),(509.,238.),(468.,244.),(430.,278.),(405.,335.),
      (407.,412.),(425.,475.)],
    &[(285.,508.),(303.,480.),(335.,483.),(339.,512.),(319.,537.),
      (329.,583.),(371.,636.),(445.,708.),(521.,815.),(498.,848.),
      (461.,824.),(405.,756.),(332.,693.),(269.,670.),(245.,635.),
      (286.,630.),(326.,650.),(302.,604.),(284.,556.)],
    &[(560.,820.),(610.,750.),(670.,690.),(738.,610.),(775.,530.),
      (773.,460.),(740.,407.),(696.,384.),(650.,380.),(650.,344.),
      (700.,344.),(750.,371.),(800.,430.),(817.,485.),(813.,550.),
      (780.,625.),(720.,700.),(670.,755.),(627.,812.),(625.,844.),
      (651.,850.),(706.,808.),(748.,752.),(778.,690.),(815.,650.),
      (850.,593.),(862.,547.),(851.,498.),(826.,459.),(840.,437.),
      (878.,481.),(895.,532.),(890.,592.),(861.,650.),(821.,710.),
      (796.,770.),(753.,825.),(704.,863.),(657.,888.),(608.,883.),
      (571.,861.)],
    &[(568.,822.),(635.,750.),(715.,683.),(778.,651.),(803.,686.),
      (783.,755.),(735.,815.),(681.,864.),(641.,883.),(594.,862.)],
    &[(686.,498.),(715.,495.),(745.,509.),(777.,548.),(764.,585.),
      (732.,552.),(703.,537.),(687.,521.)],
    &[(599.,697.),(620.,708.),(633.,745.),(635.,791.),(605.,814.),
      (589.,795.),(605.,765.),(600.,734.)],
];

fn in_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let (xi, yi) = polygon[current];
        let (xj, yj) = polygon[previous];
        if ((yi > y) != (yj > y)) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

/// Layer ownership at a master-space coordinate. The chromatic gate only
/// applies to long vessels; the heart retains its pale vascular detail.
fn classify(x: f64, y: f64, rgba: &[u8]) -> usize {
    let r = u32::from(rgba[0]);
    let g = u32::from(rgba[1]);
    let b = u32::from(rgba[2]);
    if in_polygon(x, y, HEART_OUTLINE) {
        // A few ivory rib-edge pixels touch the left silhouette. They are
        // foreground, unlike the pale vessels across the organ's center.
        if x < 485. && r > 30 && g * 100 >= r * 55 {
            return FOREGROUND;
        }
        return HEART;
    }
    let blood_red = r >= 12 && r * 100 > g * 225 && r * 100 > b * 180 && g * 100 <= b * 160 + 300;
    if blood_red && VEIN_CORRIDORS.iter().any(|p| in_polygon(x, y, p)) {
        VEINS
    } else {
        FOREGROUND
    }
}

/// Cached full-canvas layers, in back-to-front order: veins, heart, bones/rest.
pub fn layers(original: &Picture) -> [Picture; 3] {
    let mut result = std::array::from_fn(|_| Picture {
        w: original.w,
        h: original.h,
        rgba: vec![0; original.rgba.len()],
    });
    for y in 0..original.h {
        for x in 0..original.w {
            let i = (y * original.w + x) * 4;
            let pixel = &original.rgba[i..i + 4];
            let layer = classify(
                (x as f64 + 0.5) * 1070. / original.w as f64,
                (y as f64 + 0.5) * 1470. / original.h as f64,
                pixel,
            );
            result[layer].rgba[i..i + 4].copy_from_slice(pixel);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_pixels_are_partitioned_without_repainting() {
        let original = Picture::png(include_bytes!("../assets/menu-logo.png"));
        let separated = layers(&original);
        if let Ok(dir) = std::env::var("MENU_LAYER_QA_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            for (name, layer) in ["veins", "heart", "foreground"].iter().zip(&separated) {
                let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
                let mut encoder = png::Encoder::new(file, layer.w as u32, layer.h as u32);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&layer.rgba).unwrap();
            }
        }
        let mut counts = [0usize; 3];
        for (i, pixel) in original.rgba.chunks_exact(4).enumerate() {
            let owners: Vec<_> = separated.iter().enumerate()
                .filter(|(_, layer)| layer.rgba[i * 4 + 3] != 0)
                .collect();
            if pixel[3] != 0 {
                assert_eq!(owners.len(), 1, "pixel {i} has ambiguous ownership");
                let (owner, layer) = owners[0];
                assert_eq!(&layer.rgba[i * 4..i * 4 + 4], pixel);
                counts[owner] += 1;
            } else {
                assert!(owners.is_empty());
            }
        }
        assert!(counts[VEINS] > 10_000);
        assert!(counts[HEART] > 10_000);
        assert!(counts[FOREGROUND] > counts[HEART]);
    }

    #[test]
    fn known_authored_anatomy_stays_in_its_layer() {
        let original = Picture::png(include_bytes!("../assets/menu-logo.png"));
        let separated = layers(&original);
        for (x, y, expected) in [
            (530, 930, HEART), (185, 500, VEINS),
            (276, 890, FOREGROUND), (704, 947, FOREGROUND),
            (500, 610, FOREGROUND), (500, 185, FOREGROUND),
        ] {
            let i = (y * original.w + x) * 4;
            assert!(original.rgba[i + 3] != 0, "empty anatomy sample {x},{y}");
            assert_eq!(&separated[expected].rgba[i..i + 4], &original.rgba[i..i + 4],
                "wrong anatomical ownership at {x},{y}");
        }
    }

    #[test]
    fn bone_crossings_do_not_join_vascular_layer() {
        // Ivory and dark golden rib colors remain foreground, even inside the
        // broad vascular corridors. Fine pale vessels ON the heart stay heart.
        for p in [[224, 200, 149, 255], [82, 53, 28, 255], [10, 8, 5, 255]] {
            assert_eq!(classify(770., 704., &p), FOREGROUND);
        }
        assert_eq!(classify(550., 920., &[224, 200, 149, 255]), HEART);
        assert_eq!(classify(781., 491., &[129, 13, 13, 255]), VEINS);
    }
}

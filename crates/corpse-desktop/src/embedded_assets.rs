//! Immutable executable resources. No working-directory or installation dependency.
include!(concat!(env!("OUT_DIR"), "/creature_assets.rs"));

/// Sorted index: O(log N) comparisons and O(1) extra memory; no image decode here.
pub fn creature(id: &str) -> Option<&'static [u8]> {
    CREATURES
        .binary_search_by_key(&id, |(key, _)| *key)
        .ok()
        .map(|i| CREATURES[i].1)
}

pub fn set_window_icon(window: &mut sdl2::video::Window) -> Result<(), String> {
    let mut icon = compact_picture(include_bytes!("../assets/ui-runtime/window-icon.webp"));
    let surface = sdl2::surface::Surface::from_data(
        &mut icon.rgba,
        icon.w as u32,
        icon.h as u32,
        (icon.w * 4) as u32,
        sdl2::pixels::PixelFormatEnum::RGBA32,
    )?;
    window.set_icon(surface);
    Ok(())
}

/// Decode compact color plates with their original alpha; no color-keying or matting.
/// WebP decode is O(P) work/storage for P pixels, cached by selected manual entry.
/// A resource is capped at 4096² pixels before allocating the output buffer.
pub fn creature_picture(bytes: &[u8]) -> crate::render::Picture {
    compact_picture(bytes)
}

/// Decode an immutable compact UI or image resource without altering its pixels.
pub fn compact_picture(bytes: &[u8]) -> crate::render::Picture {
    decode_compact(bytes).expect("validated embedded WebP")
}

fn decode_compact(bytes: &[u8]) -> Result<crate::render::Picture, String> {
    if !bytes.starts_with(b"RIFF") || bytes.get(8..12) != Some(b"WEBP") {
        return Err("image resource is not WebP".into());
    }
    let mut decoder =
        image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || w > 4096 || h > 4096 || decoder.is_animated() {
        return Err("image resource dimensions/animation are unsupported".into());
    }
    let size = decoder
        .output_buffer_size()
        .ok_or("image resource is too large")?;
    let mut pixels = vec![0; size];
    decoder.read_image(&mut pixels).map_err(|e| e.to_string())?;
    let rgba = if decoder.has_alpha() {
        pixels
    } else {
        let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
        for rgb in pixels.chunks_exact(3) {
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
        rgba
    };
    Ok(crate::render::Picture {
        w: w as usize,
        h: h as usize,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lossless_ui_resources_preserve_every_rgba_byte() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for (name, bytes) in [
            (
                "menu-logo",
                include_bytes!("../assets/ui-runtime/menu-logo.webp").as_slice(),
            ),
            (
                "bestiary-ink-splash-broad",
                include_bytes!("../assets/ui-runtime/bestiary-ink-splash-broad.webp").as_slice(),
            ),
            (
                "BrogueCE-ASCII",
                include_bytes!("../assets/ui-runtime/BrogueCE-ASCII.webp").as_slice(),
            ),
            (
                "window-icon",
                include_bytes!("../assets/ui-runtime/window-icon.webp").as_slice(),
            ),
        ] {
            let original = crate::render::Picture::png(
                &std::fs::read(assets.join(format!("{name}.png"))).unwrap(),
            );
            let compact = compact_picture(bytes);
            assert_eq!((compact.w, compact.h), (original.w, original.h), "{name}");
            assert_eq!(
                compact.rgba, original.rgba,
                "all RGBA including hidden RGB: {name}"
            );
        }
    }

    #[test]
    fn malformed_creature_resources_are_rejected() {
        for bytes in [
            b"".as_slice(),
            b"\x89PNG\r\n\x1a\n",
            b"RIFF\x00\x00\x00\x00WEBP",
        ] {
            assert!(decode_compact(bytes).is_err());
        }
    }

    #[test]
    fn all_distribution_plates_preserve_dimensions_and_alpha() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let originals = std::fs::read_dir(assets.join("creatures"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|p| p.path().extension().is_some_and(|e| e == "png"))
            .count();
        assert_eq!(CREATURES.len(), originals);
        for (id, bytes) in CREATURES {
            assert_eq!(creature(id), Some(*bytes), "index lookup: {id}");
            let picture = creature_picture(bytes);
            let canonical_bytes =
                std::fs::read(assets.join("creatures-rgba").join(format!("{id}.png"))).unwrap();
            assert!(
                canonical_bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                    && canonical_bytes.get(25) == Some(&6),
                "canonical export must remain true RGBA PNG: {id}"
            );
            let canonical = crate::render::Picture::png(&canonical_bytes);
            let original = crate::render::Picture::png(
                &std::fs::read(assets.join("creatures").join(format!("{id}.png"))).unwrap(),
            );
            assert_eq!(
                (picture.w, picture.h),
                (original.w, original.h),
                "authored dimensions: {id}"
            );
            assert_eq!(
                (picture.w, picture.h),
                (canonical.w, canonical.h),
                "RGBA dimensions: {id}"
            );
            assert_eq!(picture.rgba.len(), picture.w * picture.h * 4);
            for (index, (actual, expected)) in picture
                .rgba
                .chunks_exact(4)
                .zip(canonical.rgba.chunks_exact(4))
                .enumerate()
            {
                assert_eq!(
                    actual[3], expected[3],
                    "alpha must be exact: {id}, pixel {index}"
                );
            }
            assert_eq!(
                *bytes,
                std::fs::read(assets.join("creatures-runtime").join(format!("{id}.webp"))).unwrap(),
                "embedded compact bytes: {id}"
            );
        }
        assert!(creature("not-an-authored-plate").is_none());
    }
}

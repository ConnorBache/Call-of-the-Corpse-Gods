//! Presentation-only Flesh materials. Never reads or changes simulation state.
//! Legacy v48 colors remain available to the frozen differential adapter.
use crate::{appearance_profile::Profile, render::Rgb};
use serde_json::Value;

/// The existing wall-edge rule, using only detached player-known neighbors.
/// Remembered terrain never borrows a currently observed neighbor's live identity.
pub(super) fn wall_face_neighbor(
    wall: &corpse_core::vision::ObservedCell,
    neighbor: &corpse_core::vision::ObservedCell,
) -> bool {
    (wall.current || !neighbor.current)
        && neighbor.tile.as_deref().is_some_and(|tile| !matches!(tile, "wall" | "chasm" | "pit"))
}

/// Keep the authored bone ink on exposed walls; solid backing uses the existing
/// muted earth/flesh ink. O(1): at most four known-neighbor probes, no live map,
/// allocations, new terrain, RNG, or cached classification. Grading follows later.
pub(super) fn wall_ink(
    fg: Rgb,
    cell: &corpse_core::vision::ObservedCell,
    observation: &corpse_core::vision::TerrainObservation,
    enabled: bool,
) -> Rgb {
    if !enabled || cell.tile.as_deref() != Some("wall") || cell.door_open.is_some()
        || cell.grate || cell.bridge || cell.snow || cell.ice || cell.puddles || cell.hellfire
    { return fg; }
    let (x, y) = (cell.index % observation.width, cell.index / observation.width);
    for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
        if nx < 0 || ny < 0 || nx >= observation.width as i32 || ny >= observation.height as i32 { continue; }
        if observation.cells.get(ny as usize * observation.width + nx as usize)
            .is_some_and(|neighbor| wall_face_neighbor(cell, neighbor))
        { return fg; }
    }
    [187, 124, 114]
}

/// Replace only unedited authored color channels. Glyphs and all appearance controls survive.
/// O(1) work and scratch: a fixed three-channel material patch, no RNG or geometry.
pub(super) fn material(settings: &mut Value, defaults: &Value, tile: &str, substrate: Option<&str>, fixture: bool) {
    if fixture { return; }
    let colors = match tile {
        "wall" => ["#b4a39a", "#30292a", "#d0bdb0"],
        "floor" => match substrate {
            Some("earth") => ["#bb7c72", "#381c23", "#bb7c72"],
            Some("sand") => ["#ceaa8b", "#43252a", "#ceaa8b"],
            Some("ash") => ["#b4a1aa", "#302029", "#b4a1aa"],
            Some("soot") => ["#806574", "#1e111b", "#806574"],
            _ => ["#968b87", "#231c20", "#b0a29a"],
        },
        "water" => ["#417de0", "#0a235a", "#6ea5f7"],
        "shallow-water" => ["#97d6f5", "#2c5e82", "#b7e8ff"],
        _ => return,
    };
    for (key, value) in ["color", "background", "edgeColor"].into_iter().zip(colors) {
        if settings[key] == defaults[key] { settings[key] = value.into(); }
    }
}

/// Subtle continuous vein-like ridges under existing ASCII, not additional decals.
/// O(1) per observed cell (two bounded interpolated noise samples), no retained state.
pub(super) fn veins(bg: Rgb, tile: &str, x: usize, y: usize, seed: u32, enabled: bool) -> Rgb {
    if !enabled || !matches!(tile, "wall" | "floor") { return bg; }
    let fold = crate::color::noise(x as f64, y as f64, seed ^ 0x9d4c1b71, 5.5)
        + crate::color::noise(x as f64, y as f64, seed ^ 0xa738b4d3, 2.75) * 0.24;
    let ridge = (1. - fold.abs() / 0.16).clamp(0., 1.);
    crate::color::composite(bg, [103, 31, 45], ridge * if tile == "wall" { 0.16 } else { 0.09 })
}

/// Same bounded cosmetic illumination ramp as WATER, with depth-specific blue defaults.
/// Explicit user edits to water palette anchors and color remain authoritative.
pub(super) fn water_illumination(profile: &Profile, fg: Rgb, bg: Rgb, tile: &str, light: crate::color::Light) -> (Rgb, Rgb) {
    if !profile.is_default(&format!("terrain.{tile}.color")) { return (fg, bg); }
    let anchor = |key: &str, native: Rgb| {
        if profile.is_default(key) { native } else { profile.rgb(key) }
    };
    let light = light.baseline();
    let shallow = tile == "shallow-water";
    let dark = anchor("palette.water.dark", if shallow { [76, 143, 184] } else { [28, 68, 152] });
    let bright = anchor("palette.water.illuminated", if shallow { [190, 237, 255] } else { [99, 166, 255] });
    let response = profile.number(&format!("terrain.{tile}.lightingResponse"));
    let amount = (light.ambient.max(light.energy.into_iter().fold(0.0f64, f64::max)
        * 160. * response) / 100.).clamp(0.,1.) * 2.;
    (if amount <= 1. { crate::color::composite(dark, fg, amount) }
        else { crate::color::composite(fg, bright, amount-1.) }, bg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flesh_colors_preserve_user_edits_and_semantic_glyphs() {
        let defaults = Profile::defaults().value("terrain.water").unwrap();
        let mut m = defaults.clone();
        material(&mut m, defaults, "water", None, false);
        assert_eq!(m["color"], "#417de0");
        assert_eq!(m["glyph"], "~");
        m["color"] = "#00aaff".into();
        m["background"] = "#001122".into();
        material(&mut m, defaults, "water", None, false);
        assert_eq!(m["color"], "#00aaff");
        assert_eq!(m["background"], "#001122");
        let mut fixture = defaults.clone();
        material(&mut fixture, defaults, "water", None, true);
        assert_eq!(&fixture, defaults);
    }
    #[test]
    fn water_depth_default_channels_change_without_overwriting_explicit_preferences() {
        for tile in ["water", "shallow-water"] {
            let defaults = Profile::defaults().value(&format!("terrain.{tile}")).unwrap();
            let mut custom = defaults.clone();
            for (key, value) in [("color", "#ffbb22"), ("background", "#472a11"), ("edgeColor", "#ffeebb"), ("glyph", "=")] {
                custom[key] = value.into();
            }
            let before = custom.clone(); material(&mut custom, defaults, tile, None, false);
            assert_eq!(custom, before, "{tile}: explicit channels are authoritative");
        }
    }
    #[test]
    fn water_depth_light_keeps_explicit_ink_and_palette_anchors() {
        for tile in ["water", "shallow-water"] {
            let custom = Profile::default().set(&format!("terrain.{tile}.color"), "#ffbb22".into()).unwrap();
            assert_eq!(water_illumination(&custom,[255,187,34],[4,5,6],tile,crate::color::Light::ambient(100.)),([255,187,34],[4,5,6]));
            let anchors=Profile::default().set("palette.water.illuminated","#ff8844".into()).unwrap()
                .set("palette.water.dark","#442211".into()).unwrap();
            assert_eq!(water_illumination(&anchors,[1,2,3],[4,5,6],tile,crate::color::Light::ambient(100.)).0,[255,136,68]);
            assert_eq!(water_illumination(&anchors,[1,2,3],[4,5,6],tile,crate::color::Light::ambient(0.)).0,[68,34,17]);
        }
    }
    /// O(1), restrained default separation retains rich blue water and explicit authored edits.
    #[test]
    fn bone_wall_muted_floor_and_blue_water_remain_distinct(){
        let mut colors=Vec::new();for tile in ["wall","floor","water"]{let d=Profile::defaults().value(&format!("terrain.{tile}")).unwrap();let mut m=d.clone();material(&mut m,d,tile,None,false);colors.push(m["color"].as_str().unwrap().to_owned());}
        assert_eq!(colors,vec!["#b4a39a","#968b87","#417de0"]);
        let d=Profile::defaults().value("terrain.wall").unwrap();let mut m=d.clone();m["color"]="#00aaff".into();material(&mut m,d,"wall",None,false);assert_eq!(m["color"],"#00aaff");
    }
    #[test]
    fn organic_ridges_are_stable_and_only_touch_authored_ground() {
        let base = [49, 23, 30];
        let a: Vec<_> = (0..120).map(|x| veins(base, "wall", x, 5, 71, true)).collect();
        let b: Vec<_> = (0..120).map(|x| veins(base, "wall", x, 5, 71, true)).collect();
        assert_eq!(a,b);
        assert!(a.iter().any(|c| *c != base));
        assert_eq!(veins(base,"water",3,5,71,true),base);
        assert_eq!(veins(base,"wall",3,5,71,false),base);
    }
}

/// Brogue CE chasmForeColor (7,7,15,4,4,8,0,false), percentages.
/// O(1) static cosmetic hashes, never gameplay RNG or wall-clock animation.
pub(super) fn brogue_chasm(x:usize,y:usize,seed:u32)->Rgb {
    std::array::from_fn(|channel| {
        let base=[7.,7.,15.][channel];let variation=[4.,4.,8.][channel];
        ((base+variation*crate::color::cosmetic_hash(x as i32,y as i32,seed.wrapping_add(channel as u32)))*2.55).round()as u8
    })
}
#[cfg(test)]
mod chasm_tests {
    #[test]
    fn brogue_chasm_is_static_blue_black_and_in_source_ranges() {
        let colors:Vec<_>=(0..80).map(|x|super::brogue_chasm(x,5,731)).collect();
        assert_eq!(colors,(0..80).map(|x|super::brogue_chasm(x,5,731)).collect::<Vec<_>>());
        assert!(colors.windows(2).any(|w|w[0]!=w[1]));
        for c in colors {assert!((18..=28).contains(&c[0]));assert!((18..=28).contains(&c[1]));assert!((38..=59).contains(&c[2]));}
    }
}

/// Late source-color replacement is safe only with the unedited grading pipeline.
/// Explicit global, fog or material adjustments keep the ordinary transformed result.
pub(super) fn brogue_palette_enabled(p:&Profile)->bool {
    ["global","fog","terrain.chasm.brightness","terrain.chasm.saturation",
     "terrain.chasm.contrast","terrain.chasm.opacity","terrain.chasm.colorVariation",
     "terrain.chasm.brightnessVariation","terrain.chasm.animationSpeed","terrain.chasm.lightingResponse"]
        .into_iter().all(|key|p.is_default(key))
}

/// The boundary adapter follows the same source palette only while its colors
/// and grading are unedited. O(1) fixed profile checks, no rendering/state cache.
pub(super) fn brogue_chasm_edges_enabled(p: &Profile) -> bool {
    brogue_palette_enabled(p) && ["terrain.chasm.color", "terrain.chasm.background"]
        .into_iter().all(|key| p.is_default(key))
}

#[cfg(test)]
mod chasm_edge_tests {
    use super::*;
    #[test]
    fn chasm_boundary_source_adapter_preserves_explicit_profile_controls() {
        assert!(brogue_chasm_edges_enabled(&Profile::default()));
        for (key, value) in [("terrain.chasm.color", serde_json::json!("#aa7733")),
            ("terrain.chasm.background", serde_json::json!("#123456")),
            ("terrain.chasm.opacity", serde_json::json!(0.25)),
            ("global.exposure", serde_json::json!(0.5)),
            ("fog.visibleContrast", serde_json::json!(0.5))] {
            let profile = Profile::default().set(key,value).unwrap();
            assert!(!brogue_chasm_edges_enabled(&profile), "{key}");
        }
    }
}

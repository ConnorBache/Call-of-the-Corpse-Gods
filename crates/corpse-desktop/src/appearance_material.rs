//! Pure v48 feature-color and presentation-pulse helpers over cosmetic profiles.
use crate::{appearance_profile::Profile, render::Rgb};
fn n(v: &serde_json::Value, field: &str, default: f64) -> f64 {
    v.get(field)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(default)
}
fn rgb(v: &serde_json::Value, field: &str, default: Rgb) -> Rgb {
    v.get(field)
        .and_then(serde_json::Value::as_str)
        .map(crate::color::hex)
        .unwrap_or(default)
}
pub fn feature_colors(
    profile: &Profile,
    key: &str,
    x: usize,
    y: usize,
    seed: u32,
    fallback: Option<(Rgb, Rgb)>,
) -> (Rgb, Rgb) {
    let Some(material) = profile.resolved_material(key,crate::appearance_profile::MaterialParent::None) else {
        return fallback.unwrap_or(([255; 3], [0; 3]));
    };
    let foreground = material.raw_color.unwrap_or([255;3]);
    let background = if material.defaults_exist && material.background_default && !material.color_default {
        foreground
    } else {
        material.raw_background.unwrap_or([0;3])
    };
    let (foreground, background) = fallback.unwrap_or((foreground, background));
    let (foreground, background) = crate::color_profile::vary_settings(
        foreground,
        background,
        "floor",
        x as f64,
        y as f64,
        seed,
        material.brightness_variation,
        material.color_variation,
    );
    let grade = |color| {
        crate::color_profile::grade(
            color,
            material.brightness,
            material.saturation,
            material.contrast,
        )
    };
    (grade(foreground), grade(background))
}
pub fn feature_color(profile: &Profile, key: &str, fallback: Rgb) -> Rgb {
    let path = format!("terrain.{key}");
    let (Some(material), Some(defaults)) = (profile.value(&path), Profile::defaults().value(&path))
    else {
        return fallback;
    };
    let color = if material.get("color") == defaults.get("color") {
        fallback
    } else {
        rgb(material, "color", fallback)
    };
    crate::color_profile::grade(
        color,
        n(material, "brightness", 1.),
        n(material, "saturation", 1.),
        n(material, "contrast", 1.),
    )
}
pub fn material_pulse(speed: f64, now: f64, x: i32, y: i32) -> f64 {
    if speed == 0. || speed == 1. {
        1.
    } else {
        1. + 0.08
            * (now * speed / 900.
                + crate::color::cosmetic_hash(x, y, 0) * std::f64::consts::PI * 2.)
                .sin()
    }
}
pub fn entity_pulse(speed: f64, now: f64, x: i32, y: i32) -> f64 {
    if speed == 0. {
        1.
    } else {
        0.85 + 0.15
            * (now * speed / 500.
                + crate::color::cosmetic_hash(x, y, 0) * std::f64::consts::PI * 2.)
                .sin()
    }
}
/// O(registered material/entity count); unknown future entries are inert.
pub fn animated(profile: &Profile) -> bool {
    if profile.number("path.animationSpeed") > 0.
        || profile.number("entities.defaults.animationSpeed") > 0.
    {
        return true;
    }
    if Profile::defaults().data()["terrain"]
        .as_object()
        .unwrap()
        .keys()
        .any(|key| {
            let speed = profile.number(&format!("terrain.{key}.animationSpeed"));
            speed != 0. && speed != 1.
        })
    {
        return true;
    }
    ["creatures", "items", "objects"].iter().any(|group| {
        Profile::defaults().data()["entities"][group]
            .as_object()
            .unwrap()
            .keys()
            .any(|key| profile.number(&format!("entities.{group}.{key}.animationSpeed")) > 0.)
    })
}

/// O(1), detached palette interpolation with cosmetic time only; never gameplay RNG.
/// A zero paint clock is the existing reduced-motion path; speed zero is static.
pub fn fire_ink(colors:[Rgb;4],phase:f64,speed:f64,now:f64)->Rgb{
    let position=if now<=0.||speed==0.{1.5}else{(0.5+0.5*(now*speed/340.+phase).sin())*3.};
    let low=(position.floor()as usize).min(3);let high=(low+1).min(3);
    crate::color::composite(colors[low],colors[high],position-low as f64)
}
#[cfg(test)]mod idle_fire_tests{
 use super::*;
 /// O(1), default speed moves; zero speed/reduced clock stay deterministic.
 #[test]fn default_speed_changes_and_disabled_motion_is_static(){let c=[[220,30,35],[250,75,30],[255,120,45],[255,215,85]];assert_ne!(fire_ink(c,0.4,1.,100.),fire_ink(c,0.4,1.,700.));assert_eq!(fire_ink(c,0.4,0.,100.),fire_ink(c,0.4,0.,700.));assert_eq!(fire_ink(c,0.4,1.,0.),fire_ink(c,2.4,1.,0.));assert_eq!(fire_ink(c,0.4,1.,700.),fire_ink(c,0.4,1.,700.));}
 /// O(1), authored fixed-color palette remains fixed despite the animation clock.
 #[test]fn custom_single_color_palette_is_authoritative(){let c=[[10,190,220];4];assert_eq!(fire_ink(c,2.,1.,100.),c[0]);assert_eq!(fire_ink(c,2.,1.,999.),c[0]);}
}

/// O(key bytes + bounded profile lookup), unknown gas families retain historical default1.
/// This gates only the detached overlay; nonzero speeds keep its existing waveform.
pub fn idle_overlay_enabled(profile:&Profile,key:&str)->bool {
    profile.value(&format!("terrain.{key}.animationSpeed")).and_then(|v|v.as_f64()).unwrap_or(1.)!=0.
}
/// O(bounded entity style fields), uses the same inheritance as the light object's paint.
pub fn idle_torch_enabled(profile:&Profile,source:&str)->bool {
    crate::entity_profile::resolve(profile,"objects",source,Some("light"),"*",[255,150,60]).speed!=0.
}
#[cfg(test)]mod idle_overlay_controls_tests {
 use super::*;
 /// O(fixed profile fields), zero gates each family without altering nonzero waveform.
 #[test]fn individual_zero_and_inherited_light_controls(){
  let p=Profile::defaults();
  for key in ["water","shallow-water","gas:caustic"] {
   assert!(idle_overlay_enabled(p,key));
   let stopped=p.set(&format!("terrain.{key}.animationSpeed"),serde_json::json!(0)).unwrap();assert!(!idle_overlay_enabled(&stopped,key));
   let moving=p.set(&format!("terrain.{key}.animationSpeed"),serde_json::json!(2)).unwrap();assert!(idle_overlay_enabled(&moving,key));
  }
  assert!(idle_overlay_enabled(p,"gas:hallucinogenic"));
  assert!(!idle_torch_enabled(p,"torch"));
  let moving=p.set("entities.objects.torch.animationSpeed",serde_json::json!(1)).unwrap();assert!(idle_torch_enabled(&moving,"torch"));
  let inherited=p.set("entities.objects.light.animationSpeed",serde_json::json!(1)).unwrap();assert!(idle_torch_enabled(&inherited,"torch"));
 }
}

#[cfg(test)]
fn feature_colors_json_reference(
    profile: &Profile,
    key: &str,
    x: usize,
    y: usize,
    seed: u32,
    fallback: Option<(Rgb, Rgb)>,
) -> (Rgb, Rgb) {
    let Some(material) = profile.material(key, None) else {
        return fallback.unwrap_or(([255; 3], [0; 3]));
    };
    let defaults = Profile::defaults().value(&format!("terrain.{key}"));
    let foreground = rgb(&material, "color", [255; 3]);
    let background = if defaults.is_some_and(|v| {
        material.get("background") == v.get("background") && material.get("color") != v.get("color")
    }) {
        foreground
    } else {
        rgb(&material, "background", [0; 3])
    };
    let (foreground, background) = fallback.unwrap_or((foreground, background));
    let (foreground, background) = crate::color_profile::vary_settings(
        foreground,
        background,
        "floor",
        x as f64,
        y as f64,
        seed,
        n(&material, "brightnessVariation", 1.),
        n(&material, "colorVariation", 1.),
    );
    let grade = |color| {
        crate::color_profile::grade(
            color,
            n(&material, "brightness", 1.),
            n(&material, "saturation", 1.),
            n(&material, "contrast", 1.),
        )
    };
    (grade(foreground), grade(background))
}
#[cfg(test)]mod compiled_feature_tests {use super::*;
 #[test]fn cached_feature_colors_match_raw_inheritance_and_color_cases(){let d=Profile::default();let changed=d.set("terrain.feature:fire.color",serde_json::json!("#00aaff")).unwrap();for p in [d,changed]{for key in p.data()["terrain"].as_object().unwrap().keys(){for fallback in [None,Some(([13,52,111],[0,3,7]))]{assert_eq!(feature_colors(&p,key,3,4,17,fallback),feature_colors_json_reference(&p,key,3,4,17,fallback));}}}}
}

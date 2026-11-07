//! Source entityAppearance resolution over immutable profile/catalog keys.
use crate::{appearance_profile::Profile, render::Rgb};
pub struct Entity {
    pub glyph: String,
    pub color: Rgb,
    pub background: Option<Rgb>,
    pub emphasis: f64,
    pub speed: f64,
}
pub fn resolve(
    p: &Profile,
    group: &str,
    key: &str,
    parent: Option<&str>,
    glyph: &str,
    color: Rgb,
) -> Entity {
    let defaults = Profile::defaults();
    let entry = if defaults.value(&format!("entities.{group}.{key}")).is_some() {
        p.value(&format!("entities.{group}.{key}"))
    } else {
        None
    };
    let above = parent.and_then(|key| {
        if defaults.value(&format!("entities.{group}.{key}")).is_some() {
            p.value(&format!("entities.{group}.{key}"))
        } else {
            None
        }
    });
    let explicit = entry.is_some_and(|v| v["inherited"] == false)
        || above.is_some_and(|v| v["inherited"] == false);
    let identity = entry
        .filter(|v| v["inherited"] == false)
        .or_else(|| above.filter(|v| v["inherited"] == false))
        .unwrap_or_else(|| p.value("entities.defaults").unwrap());
    let mut settings = identity.clone();
    for candidate in [above, entry].into_iter().flatten() {
        for field in ["brightness", "saturation", "emphasis", "animationSpeed"] {
            if candidate[field].as_f64().unwrap_or(0.)
                != if field == "animationSpeed" { 0. } else { 1. }
            {
                settings[field] = candidate[field].clone();
            }
        }
    }
    let changed = |field: &str| {
        p.value(&format!("entities.defaults.{field}"))
            != defaults.value(&format!("entities.defaults.{field}"))
    };
    let fg = if explicit || changed("color") {
        crate::color::hex(settings["color"].as_str().unwrap_or("#ffffff"))
    } else {
        color
    };
    let background = if explicit || changed("background") {
        settings["background"]
            .as_str()
            .filter(|s| *s != "transparent")
            .map(crate::color::hex)
    } else {
        None
    };
    Entity {
        glyph: if explicit || changed("glyph") {
            settings["glyph"].as_str().unwrap_or(glyph).into()
        } else {
            glyph.into()
        },
        color: crate::color_profile::grade(
            fg,
            settings["brightness"].as_f64().unwrap_or(1.),
            settings["saturation"].as_f64().unwrap_or(1.),
            1.,
        ),
        background,
        emphasis: settings["emphasis"].as_f64().unwrap_or(1.),
        speed: settings["animationSpeed"].as_f64().unwrap_or(0.),
    }
}

/// Source materialPulse. Inputs are cosmetic time and public tile coordinates.
pub fn material_pulse(speed: f64, now: f64, x: usize, y: usize) -> f64 {
    if speed == 0. || speed == 1. {
        1.
    } else {
        1. + 0.08
            * (now * speed / 900.
                + crate::color::cosmetic_hash(x as i32, y as i32, 0) * std::f64::consts::TAU)
                .sin()
    }
}
pub fn materials_animated(p: &Profile) -> bool {
    p.value("terrain")
        .and_then(|v| v.as_object())
        .is_some_and(|values| {
            values.values().any(|m| {
                m["animationSpeed"]
                    .as_f64()
                    .is_some_and(|v| v != 0. && v != 1.)
            })
        })
}

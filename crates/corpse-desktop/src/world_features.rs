//! Pure authored semantic material/object appearance. No simulation access.
use crate::{appearance_profile::Profile, render::Rgb};
pub struct Detail {
    pub glyph: String,
    pub color: Rgb,
    pub background: Option<(Rgb, f64)>,
    pub opacity: f64,
    pub emphasis: f64,
    pub response: Option<f64>,
}
pub fn material(
    p: &Profile,
    key: &str,
    glyph: &str,
    color: Rgb,
    x: usize,
    y: usize,
    seed: u32,
    now: f64,
) -> Detail {
    let path = format!("terrain.{key}");
    let settings = p.value(&path).unwrap();
    let defaults = Profile::defaults().value(&path).unwrap();
    let fg = if settings["color"] == defaults["color"] {
        color
    } else {
        crate::color::hex(settings["color"].as_str().unwrap())
    };
    let bg = crate::color::hex(settings["background"].as_str().unwrap());
    let (fg, _) = crate::appearance_material::feature_colors(p, key, x, y, seed, Some((fg, bg)));
    let pulse = crate::appearance_material::material_pulse(
        settings["animationSpeed"].as_f64().unwrap_or(1.),
        now,
        x as i32,
        y as i32,
    );
    let opacity = settings["opacity"].as_f64().unwrap_or(1.);
    Detail {
        glyph: if settings["glyph"] == defaults["glyph"] {
            glyph.into()
        } else {
            settings["glyph"].as_str().unwrap_or(glyph).into()
        },
        color: crate::color_profile::grade(fg, pulse, 1., 1.),
        background: (settings["background"] != defaults["background"])
            .then_some((bg, 0.25 * opacity)),
        opacity,
        emphasis: 1.,
        response: settings["lightingResponse"].as_f64(),
    }
}
/// Floor blood only recolors the existing ground punctuation. Explicit blood
/// background/glyph settings cannot turn it into a tile fill or hide the floor.
/// Blood-coloured water, bloody walls and Ichor are separate materials.
pub fn blood_dot(p:&Profile,glyph:&str,color:Rgb,x:usize,y:usize,seed:u32,now:f64)->Detail {
    let mut d=material(p,"feature:blood",glyph,color,x,y,seed,now);
    d.glyph=glyph.into();d.background=None;d
}
pub fn object(
    p: &Profile,
    key: &str,
    parent: Option<&str>,
    glyph: &str,
    color: Rgb,
    x: usize,
    y: usize,
    now: f64,
) -> Detail {
    let style = crate::entity_profile::resolve(p, "objects", key, parent, glyph, color);
    let pulse = crate::appearance_material::entity_pulse(style.speed, now, x as i32, y as i32);
    Detail {
        glyph: style.glyph,
        color: crate::color_profile::grade(style.color, pulse, 1., 1.),
        background: style.background.map(|c| (c, 0.3)),
        opacity: 1.,
        emphasis: style.emphasis,
        response: None,
    }
}
pub fn overlay(
    p: &Profile,
    key: &str,
    x: usize,
    y: usize,
    seed: u32,
    now: f64,
    alpha: f64,
) -> (Rgb, f64) {
    let m = p.value(&format!("terrain.{key}")).unwrap();
    let (_, bg) = crate::appearance_material::feature_colors(p, key, x, y, seed, None);
    let pulse = crate::appearance_material::material_pulse(
        m["animationSpeed"].as_f64().unwrap_or(1.),
        now,
        x as i32,
        y as i32,
    );
    (
        crate::color_profile::grade(bg, pulse, 1., 1.),
        alpha * m["opacity"].as_f64().unwrap_or(1.),
    )
}
pub fn tone(
    p: &Profile,
    color: Rgb,
    material_bg: Rgb,
    final_bg: Rgb,
    context: crate::color::Context<'_>,
    chance: f64,
    all_seeing: bool,
    response: Option<f64>,
    background: bool,
    important: bool,
    emissive: bool,
) -> Rgb {
    let response = response.unwrap_or_else(|| {
        p.value(&format!("terrain.{}.lightingResponse", context.tile))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.)
    });
    let ctx = crate::color::Context {
        important,
        ..context
    };
    let pair = crate::color_profile::map_colors_observed(
        p,
        color,
        if background { color } else { material_bg },
        ctx,
        emissive,
        response,
        chance,
        all_seeing,
    );
    if background {
        return pair.1;
    }
    if p.value("global") != Profile::defaults().value("global") {
        return pair.0;
    }
    crate::color::distinguish(
        pair.0,
        final_bg,
        if matches!(
            context.knowledge,
            crate::color::Knowledge::Remembered | crate::color::Knowledge::Layout
        ) {
            1.45
        } else if important || emissive {
            2.8
        } else {
            1.85
        },
    )
}

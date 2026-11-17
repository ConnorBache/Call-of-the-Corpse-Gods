//! Finish detached map colors once, after authored materials and readability.
//! No Run, RNG, visibility, LOS/LoE, or simulation values are read or changed.
use crate::{appearance_profile::Profile, color::{Light, light_ui::DisplayLight}, render::{Animation, Cell, Rgb}};

/// O(C + A + U), C cells, A animation anchors, U underlays. Fixed channel math.
pub fn finish_cells(cells: &mut [Cell], lights: &[Light], width: usize, profile: &Profile) {
    for cell in cells {
        let Some(light) = lights.get(cell.y * width + cell.x) else { continue; };
        crate::hallucination_ui::map_cell(cell, |cell| finish_cell(cell, *light, profile));
    }
}
pub fn finish_cell(cell: &mut Cell, light: Light, profile: &Profile) {
    if !cell.current && cell.entity_ink.is_none_or(|ink|ink.remembered)
        && profile.is_default("fog") && profile.is_default("global") {
        // Neutralize detached remembered decoration too, not just the main glyph.
        // Idempotent: material rebuilds may pass an already-neutral channel again.
        for color in cell.edges.iter_mut().flatten() {*color=memory_gray(*color);}
        for (_,color,_) in &mut cell.underlays {*color=memory_gray(*color);}
        if let Some((color,_))=&mut cell.background_overlay {*color=memory_gray(*color);}
    }
    let visible = cell.direct || cell.entity_ink.is_some_and(|ink| !ink.telepathic && !ink.remembered);
    let Some(display) = light.display.filter(|_| visible || cell.entity_ink.is_some()) else { return; };
    if cell.display_light.is_some() { return; }
    // Save only the source channels needed when restoring topology/actor paint
    // over a fresh animated base. Never inverse-transform rounded RGB pixels.
    cell.unlit_ink = Some((cell.fg, cell.background_overlay));
    if profile.is_default("fog") && profile.is_default("global") {
        (cell.fg, cell.bg) = crate::render::readable_cell_colors(cell);
    }
    if visible { cell.fg = display.foreground(cell.fg,cell.actor_id.is_some()); }
    cell.bg = display.apply(cell.bg);
    // Authored dark base colors, including chasms, use the same illumination
    // treatment as other terrain. Knowledge filtering above still owns memory.
    if visible && display.illumination < 0. {
        // Lift only existing foreground ink. No outline, halo, background floor,
        // extra light source, or visibility/range change is introduced.
        let rescued=crate::color::light_ui::foreground_contrast(cell.fg,cell.bg,if cell.actor_id.is_some(){4.5}else{3.});
        let t=(-display.illumination/25.).clamp(0.,1.);let blend=t*t*(3.-2.*t);
        cell.fg=crate::color::composite(cell.fg,rescued,blend);
    } else if profile.is_default("fog") && profile.is_default("global")
        && (cell.entity_ink.is_some_and(|ink| ink.important && !ink.telepathic && !ink.remembered && ink.identity_outline.is_none())
            || matches!(cell.label.as_str(), "up" | "down"))
    {
        // Important visible symbols may be rescued, ordinary terrain may not.
        cell.fg = crate::color::distinguish(cell.fg, cell.bg, 2.8);
    }
    for color in cell.edges.iter_mut().flatten() { *color = display.foreground(*color,false); }
    for (_, color, _) in &mut cell.underlays { *color = display.foreground(*color,false); }
    if let Some((color, _)) = &mut cell.background_overlay { *color = display.apply(*color); }
    for animation in &mut cell.animations {
        match animation {
            Animation::Gas { color, .. } | Animation::Reflection { color, .. } => *color = display.apply(*color),
            Animation::Fire { colors, .. } => *colors = colors.map(|color| display.foreground(color,false)),
            _ => {}
        }
    }
    cell.display_light = Some(display);
}

/// Restore only cached pre-illumination channels before animated reconstruction.
pub fn restore_source_ink(cell: &mut Cell) {
    if let Some((fg, overlay)) = cell.unlit_ink.take() { cell.fg = fg; cell.background_overlay = overlay; }
    cell.display_light = None;
}
pub fn paint_color(color: Rgb, display: Option<DisplayLight>) -> Rgb {
    display.map_or(color, |light| light.foreground(color,false))
}

/// O(C) scan of detached paint cells. Stable between world observations: the
/// immersion veil follows admitted player illumination, not cosmetic flicker.
pub fn immersion_color(view: &crate::render::View) -> Option<Rgb> {
    if !view.deep_water { return None; }
    // The canonical player layer follows any detached portal/pocket aliases.
    let display = view.cells.iter().rev().find(|cell| cell.actor_id == Some(view.player_id))
        .and_then(|cell| cell.display_light);
    Some(display.map_or([20,78,170],|light|light.apply([20,78,170])))
}

/// A single bounded veil for the approved player afflictions and immersion.
/// Reads the detached presented View, never the live simulation. O(S + C),
/// S status labels and C cells only when illumination lookup is needed; O(1) space.
/// Fixed status names here are projection identities, not formatted durations.
pub fn screen_tint(view: &crate::render::View) -> Option<Rgb> {
    let mut active = [false; 5];
    for status in &view.player_statuses {
        match status.name.as_str() {
            "Burning" => active[0] = true,
            "Poisoned" => active[1] = true,
            "Hemophilia" | "Bleeding" => active[2] = true,
            "Confused" => active[3] = true,
            "Silenced" | "Silenced by aura" => active[4] = true,
            _ => {} // In particular, Envenomed is not Poisoned.
        }
    }
    let mut sum = [0u32; 3];
    let mut count = 0u32;
    for (enabled, color) in active.into_iter().zip([[255,80,20], [25,200,60], [230,20,30], [255,255,255], [0,0,0]]) {
        if enabled {
            for i in 0..3 { sum[i] += color[i]; }
            count += 1;
        }
    }
    if view.deep_water {
        for (total, value) in sum.iter_mut().zip([20,78,170]) { *total += value; }
        count += 1;
    }
    if count == 0 { return None; }
    // Equal-weight, order-independent mean; opacity remains 34/255 regardless
    // of overlap. Related aliases share one tint, avoiding duplicate weighting.
    let color = sum.map(|v| ((v + count / 2) / count) as u8);
    let light = view.cells.iter().rev().find(|cell|cell.actor_id == Some(view.player_id))
        .and_then(|cell|cell.display_light);
    Some(light.map_or(color, |light|light.apply(color)))
}

/// Default unexplored terrain is genuinely black; explicit palette edits remain honored.
pub fn unexplored(profile:&Profile)->Rgb {
    if profile.is_default("fog.unexplored") {[0;3]} else {profile.rgb("fog.unexplored")}
}

fn memory_gray(color:Rgb)->Rgb {
    let gray=(color[0]as f64*0.2126+color[1]as f64*0.7152+color[2]as f64*0.0722).round()as u8;
    [gray;3]
}

#[cfg(test)]
#[path = "chasm_palette_tests.rs"]
mod chasm_palette_tests;

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
    let visible = cell.direct || cell.entity_ink.is_some_and(|ink| !ink.telepathic && !ink.remembered);
    let Some(display) = light.display.filter(|_| visible || cell.entity_ink.is_some()) else { return; };
    if cell.display_light.is_some() { return; }
    // Save only the source channels needed when restoring topology/actor paint
    // over a fresh animated base. Never inverse-transform rounded RGB pixels.
    cell.unlit_ink = Some((cell.fg, cell.background_overlay));
    if profile.is_default("fog") && profile.is_default("global") {
        (cell.fg, cell.bg) = crate::render::readable_cell_colors(cell);
    }
    if visible { cell.fg = display.apply(cell.fg); }
    cell.bg = display.apply(cell.bg);
    if profile.is_default("fog") && profile.is_default("global")
        && (cell.entity_ink.is_some_and(|ink| ink.important && !ink.telepathic && !ink.remembered && ink.identity_outline.is_none())
            || matches!(cell.label.as_str(), "up" | "down"))
    {
        // Important visible symbols may be rescued, ordinary terrain may not.
        cell.fg = crate::color::distinguish(cell.fg, cell.bg, 2.8);
    }
    for color in cell.edges.iter_mut().flatten() { *color = display.apply(*color); }
    for (_, color, _) in &mut cell.underlays { *color = display.apply(*color); }
    if let Some((color, _)) = &mut cell.background_overlay { *color = display.apply(*color); }
    for animation in &mut cell.animations {
        match animation {
            Animation::Gas { color, .. } | Animation::Reflection { color, .. } => *color = display.apply(*color),
            Animation::Fire { colors, .. } => *colors = colors.map(|color| display.apply(color)),
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
    display.map_or(color, |light| light.apply(color))
}

/// O(C) scan of detached paint cells. Stable between world observations: the
/// immersion veil follows admitted player illumination, not cosmetic flicker.
pub fn immersion_color(view: &crate::render::View) -> Option<Rgb> {
    if !view.deep_water { return None; }
    // The canonical player layer follows any detached portal/pocket aliases.
    let display = view.cells.iter().rev().find(|cell| cell.actor_id == Some(view.player_id))
        .and_then(|cell| cell.display_light);
    Some(paint_color([20,78,170], display))
}

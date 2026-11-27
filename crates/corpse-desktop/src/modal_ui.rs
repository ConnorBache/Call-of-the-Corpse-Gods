//! Detached modal geometry and measured text. Input and paint use identical
//! resolved rectangles; authored preferences never query or mutate a Run.
use crate::{
    appearance_profile::Profile,
    render::{Painter, HEIGHT},
    typography_ui::{self, Role, RoleStyle},
};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect(pub i32, pub i32, pub i32, pub i32);
impl Rect {
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.0 && y >= self.1 && x < self.0 + self.2 && y < self.1 + self.3
    }
    pub fn tuple(self) -> (i32, i32, i32, i32) {
        (self.0, self.1, self.2, self.3)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub map: Rect,
    pub sidebar: i32,
    pub chronicle: i32,
    pub command: i32,
    pub rail_bottom: i32,
}
impl Frame {
    pub fn from_profile(p: &Profile) -> Self {Self::for_size(p,crate::render::WIDTH as i32,HEIGHT as i32)}
    pub fn for_size(p:&Profile,width:i32,height:i32)->Self{
        let chronicle = crate::render::chronicle_height(p);
        let sidebar = crate::render::hud_left(p)+width-crate::render::WIDTH as i32;
        let command = changed_number(p, "ui.layout.buttonHeight", 42.).round() as i32 + 4;
        let top = chronicle + 12;
        let bottom = crate::render::toolbar_y(p)+height-HEIGHT as i32-inspection_strip_height(p);
        Self {
            map: Rect(
                0,
                top,
                (sidebar - crate::render::enemy_list::RESERVED_WIDTH).max(1),
                (bottom - top).max(1),
            ),
            sidebar,
            chronicle,
            command,
            rail_bottom: height - crate::render::GAME_UI_UNIT,
        }
    }
}
/// One text line with vertical padding, independent of its contents (no map reflow on hover).
pub fn inspection_strip_height(p:&Profile)->i32 {
    let style=typography_ui::resolve(p,Role::Primary,RoleStyle::baseline(15.));
    (style.font_size*style.line_height).ceil().max(style.font_size).max(18.) as i32+7
}
/// Source styles emit authored values only when different from schema defaults.
pub fn changed_number(p: &Profile, path: &str, baseline: f64) -> f64 {
    let n = p.number(path);
    if n == Profile::defaults().number(path) {
        baseline
    } else {
        n
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackLayout {
    pub bounds: Rect,
    pub list: Rect,
    pub detail: Rect,
    pub close: Rect,
    pub cancel: Rect,
    pub row: i32,
    pub button_height: i32,
    pub button_padding: i32,
}
impl PackLayout {
    /// The Pack may cover the sidebars: the measured item column takes priority,
    /// while the detail column retains readable space within the viewport.
    pub fn resolve_measured(p: &Profile, area: Rect, list_width: i32) -> Self {
        let mut out = Self::resolve(p, area);
        let margin = 12.min(area.2 / 4).max(0);
        let width = (list_width + 322 + 36).max(760).min((area.2 - 2 * margin).max(1));
        let inner = (width - 36).max(2);
        let detail = 322.min((inner - list_width).max(240.min(inner / 3)));
        let list = (inner - detail).max(1);
        let x = area.0 + (area.2 - width) / 2;
        out.bounds.0 = x; out.bounds.2 = width;
        out.list.0 = x + 12; out.list.2 = list;
        out.detail.0 = x + 24 + list; out.detail.2 = detail;
        out.close.0 = x + width - 44;
        out.cancel.0 = x + width - 92;
        out
    }
    pub fn resolve(p: &Profile, map: Rect) -> Self {
        let margin = 12.min(map.2 / 4).min(map.3 / 4).max(0);
        let w = 760.min((map.2 - 2 * margin).max(1));
        let h = 500.min((map.3 - 2 * margin).max(1));
        let x = map.0 + map.2 - margin - w;
        let y = map.1 + map.3 - margin - h;
        let inner = (w - 36).max(2);
        let left = (inner as f64 * 1.25 / 2.25).round() as i32;
        let right = (inner - left).max(1);
        let content_h = (h - 130).max(1);
        Self {
            bounds: Rect(x, y, w, h),
            list: Rect(x + 12, y + 118, left.max(1), content_h),
            detail: Rect(x + 24 + left, y + 118, right, content_h),
            close: Rect(x + w - 44, y + 4, 36.min(w), 32.min(h)),
            cancel: Rect(x + w - 92, y + 71, 80.min(w), 32.min(h)),
            row: p.number("ui.layout.inventoryRowHeight").round().max(1.) as i32,
            button_height: changed_number(p, "ui.layout.buttonHeight", 32.)
                .round()
                .max(1.) as i32,
            button_padding: changed_number(p, "ui.layout.buttonPadding", 8.)
                .round()
                .max(0.) as i32,
        }
    }
}
pub fn inspection_bounds(p: &Profile) -> Rect {
    let f = Frame::from_profile(p);
    let x = 24;
    // This drawer is painted in the original authored dialog coordinates.
    let chronicle = crate::render::chronicle_height_for(p, 108);
    let y = chronicle + 30;
    let w = 520.min((f.sidebar - 48).max(1));
    let h = 580.min((HEIGHT as i32 - chronicle - 120).max(1));
    Rect(x, y, w, h)
}
pub fn hover_bounds(p: &Profile, tile: (i32, i32, i32, i32), height: i32) -> Rect {
    let f = Frame::from_profile(p);
    hover_placement(
        tile,
        height,
        (f.map.2 - 4, HEIGHT as i32),
        p.number("ui.layout.tooltipMaxWidth").round() as i32,
    )
}
/// Passive gameplay inspector: anchor against the whole canvas, not the
/// hovered tile or the map column. Keep the footer and toolbar unobscured.
pub fn hover_bounds_painter(p:&Painter,height:i32)->Rect{
    let margin=8;
    let vw=p.logical_width().max(0);
    let map=p.frame().map;
    let bottom=(map.1+map.3).clamp(0,p.logical_height());
    let w=(p.profile.number("ui.layout.tooltipMaxWidth").round()as i32).clamp(0,(vw-2*margin).max(0));
    let h=height.clamp(0,620.min((bottom-2*margin).max(0)));
    Rect((vw-margin-w).max(0),(bottom-margin-h).max(0),w,h)
}
pub fn hover_placement(
    tile: (i32, i32, i32, i32),
    height: i32,
    viewport: (i32, i32),
    width: i32,
) -> Rect {
    let (left, top, right, bottom) = tile;
    let (vw, vh) = viewport;
    let w = width.min((vw - 16).max(0));
    let h = height.max(0).min(620.min((vh - 16).max(0)));
    let below = bottom + 5;
    let y = if below + h <= vh - 8 {
        below
    } else {
        top - 5 - h
    };
    Rect(
        ((left + right - w) / 2).clamp(8, (vw - 8 - w).max(8)),
        y.clamp(8, (vh - 8 - h).max(8)),
        w,
        h,
    )
}
/// Cached advances support input-time wrapping without retaining a Painter.
/// Configure visits observed text once, measuring each distinct scalar once.
#[derive(Clone)]
pub struct TextMetrics {
    pub role: Role,
    pub size: u16,
    pub line: i32,
    advances: HashMap<char, f64>,
}
impl TextMetrics {
    pub fn new(p: &Painter, role: Role, size: u16, baseline_line: i32) -> Self {
        let mut base = RoleStyle::baseline(size as f64);
        base.line_height = baseline_line as f64 / size as f64;
        let style = typography_ui::resolve(&p.profile, role, base);
        let mut out = Self {
            role,
            size,
            line: (style.font_size * style.line_height).round().max(1.) as i32,
            advances: HashMap::new(),
        };
        out.ensure(
            p,
            " …↑↓×∞0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz:.,;!?-_/[]()'\"",
        );
        out
    }
    pub fn ensure(&mut self, p: &Painter, text: &str) {
        for ch in text.chars() {
            self.advances
                .entry(ch)
                .or_insert_with(|| p.measure_text_role(self.role, &ch.to_string(), self.size));
        }
    }
    pub fn width(&self, text: &str) -> f64 {
        text.chars()
            .map(|c| *self.advances.get(&c).unwrap_or(&8.))
            .sum()
    }
    pub fn ellipsize(&self, text: &str, width: i32) -> String {
        if self.width(text) <= width as f64 {
            return text.into();
        }
        let mut out = String::new();
        let mut advance = self.width("…");
        for c in text.chars() {
            advance += self.advances.get(&c).copied().unwrap_or(8.);
            if advance > width.max(0) as f64 {
                break;
            }
            out.push(c);
        }
        out.push('…');
        out
    }
    pub fn wrap(&self, text: &str, width: i32) -> Vec<String> {
        wrap_measured(text, width, |s| self.width(s))
    }
}
/// Whitespace/paragraph-preserving measured wrap with scalar fallback for a
/// single long word, matching source overflow-wrap:anywhere. O(T²) worst case
/// for width callback scans; T is observed tooltip text, never save history.
pub fn wrap_measured(text: &str, width: i32, measure: impl Fn(&str) -> f64) -> Vec<String> {
    let width = width.max(1) as f64;
    let mut rows = vec![];
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.into()
            } else {
                format!("{line} {word}")
            };
            if measure(&candidate) <= width {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                rows.push(std::mem::take(&mut line));
            }
            for c in word.chars() {
                let mut candidate = line.clone();
                candidate.push(c);
                if !line.is_empty() && measure(&candidate) > width {
                    rows.push(std::mem::take(&mut line));
                }
                line.push(c);
            }
        }
        rows.push(line);
    }
    rows
}
pub fn border(p: &mut Painter, rect: Rect, color: [u8; 3]) {
    let thickness = p.profile.number("ui.layout.borderWidth").round() as i32;
    for inset in 0..thickness {
        if rect.2 > 2 * inset && rect.3 > 2 * inset {
            p.border(
                rect.0 + inset,
                rect.1 + inset,
                rect.2 - 2 * inset,
                rect.3 - 2 * inset,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn default_geometry_and_changed_css_baselines_are_preserved() {
        let p = Profile::default();
        let frame = Frame::from_profile(&p);
        assert_eq!(frame.map, Rect(0, 112, 736, 608));
        let pack = PackLayout::resolve(&p, frame.map);
        assert_eq!(pack.bounds, Rect(12, 208, 712, 500));
        assert_eq!(pack.list, Rect(24, 326, 376, 370));
        assert_eq!(pack.detail, Rect(412, 326, 300, 370));
        assert_eq!(pack.button_height, 32);
        assert_eq!(pack.button_padding, 8);
        assert_eq!(inspection_bounds(&p), Rect(24, 138, 520, 572));
        let p = p.set("ui.layout.buttonHeight", json!(54)).unwrap();
        let frame = Frame::from_profile(&p);
        assert_eq!(frame.command, 58);
        assert_eq!(frame.map.1 + frame.map.3, crate::render::toolbar_y(&p)-inspection_strip_height(&p));
        assert_eq!(PackLayout::resolve(&p, frame.map).button_height, 54);
    }
    #[test]
    fn profile_extremes_keep_fixed_tracks_and_modal_containment() {
        for sidebar in [220, 600] {
            for percent in [18, 40] {
                for height in [60, 240] {
                    let p=Profile::from_value(json!({"ui":{"layout":{"rightSidebarWidth":sidebar,"sidebarWidthPercent":percent,"chronicleHeight":height,"inventoryRowHeight":42,"mapSpacing":20,"tooltipMaxWidth":800}}})).unwrap();
                    let frame = Frame::from_profile(&p);
                    let pack = PackLayout::resolve(&p, frame.map);
                    assert!(pack.bounds.0 >= frame.map.0 && pack.bounds.1 >= frame.map.1);
                    assert!(pack.bounds.0 + pack.bounds.2 <= frame.map.0 + frame.map.2);
                    assert!(pack.bounds.1 + pack.bounds.3 <= frame.map.1 + frame.map.3);
                    assert!(pack.close.contains(pack.close.0 + 5, pack.close.1 + 5));
                    assert_eq!(pack.row, 42);
                    let hover = hover_bounds(&p, (900, 700, 920, 720), 1000);
                    assert!(hover.0 >= 8 && hover.1 >= 8);
                    assert!(hover.0 + hover.2 <= frame.map.2 - 12);
                    assert!(hover.1 + hover.3 <= HEIGHT as i32 - 8);
                }
            }
        }
    }
    #[test]
    fn compact_pack_stays_inside_frozen_bounds_and_authored_hover_placements_match() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../assets/appearance-modal-parity-v48.json"))
                .unwrap();
        assert_eq!(cases["packs"].as_array().unwrap().len(), 6);
        for case in cases["packs"].as_array().unwrap() {
            let map: [i32; 4] = serde_json::from_value(case["map"].clone()).unwrap();
            let got =
                PackLayout::resolve(Profile::defaults(), Rect(map[0], map[1], map[2], map[3]))
                    .bounds;
            let expected: [i32; 4] = serde_json::from_value(case["expected"].clone()).unwrap();
            assert!(got.2<=expected[2] && got.3<=expected[3]);
            assert_eq!(got.0+got.2,expected[0]+expected[2]);
            assert_eq!(got.1+got.3,expected[1]+expected[3]);
            assert!(got.2<=760 && got.3<=500);
        }
        assert_eq!(cases["hovers"].as_array().unwrap().len(), 36);
        for case in cases["hovers"].as_array().unwrap() {
            let tile: [i32; 4] = serde_json::from_value(case["tile"].clone()).unwrap();
            let vp: [i32; 2] = serde_json::from_value(case["viewport"].clone()).unwrap();
            let got = hover_placement(
                (tile[0], tile[1], tile[2], tile[3]),
                case["height"].as_i64().unwrap() as i32,
                (vp[0], vp[1]),
                case["width"].as_i64().unwrap() as i32,
            );
            let expected: [f64; 4] = serde_json::from_value(case["expected"].clone()).unwrap();
            for (a, b) in [got.0, got.1, got.2, got.3].into_iter().zip(expected) {
                assert!((a as f64 - b).abs() <= 0.5);
            }
        }
    }
    #[test]
    fn measured_wrapping_preserves_newlines_and_breaks_long_words() {
        let rows = wrap_measured("wide words\n012345678901234\n", 35, |s| {
            s.chars().count() as f64 * 7.
        });
        assert_eq!(rows, ["wide", "words", "01234", "56789", "01234", ""]);
        assert!(wrap_measured("x", 1, |_| 100.)[0] == "x");
    }
}

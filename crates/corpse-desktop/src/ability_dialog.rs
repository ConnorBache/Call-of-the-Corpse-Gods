//! Content-sized, observation-only ability details beside the divine-ability rail.
use super::{damage, Painter, Rgb, DIALOG_SCALE, INK, MUTED};
use crate::{ability_ui::AbilityView, typography_ui::{self, Role, RoleStyle}};

const PAD: i32 = 18;
struct Line { text: String, size: u16, color: Rgb, height: i32, gap: i32 }
struct Layout { width: i32, height: i32, body_height: i32, lines: Vec<Line>, scrolling: bool }

impl Layout {
    /// O(T²) worst-case measured wrapping / O(T) text storage, once per paint or
    /// damage query. T is detached ability prose, never world or history size.
    fn resolve(p: &Painter, a: &AbilityView) -> Self {
        let status = [a.mode.as_str(), a.state.as_str()].into_iter()
            .filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join(" · ");
        let coverage = if a.supported { "" } else { "Native execution coverage pending" };
        let sections = [(&*a.name, 21, a.color, 0), (status.as_str(), 14, MUTED, 6),
            (&*a.cost, 14, [36, 223, 255], 4), (&*a.description, 16, INK, 10),
            (coverage, 13, MUTED, 8)];
        let natural = sections.iter().map(|(s,size,_,_)|
            p.measure_text_role(Role::Tooltips, s, *size).ceil() as i32).max().unwrap_or(1);
        let available_width = ((p.hud_left() - 8).max(1) as f64 / DIALOG_SCALE).floor() as i32;
        let inner = natural.max(1).min(480).min((available_width - 2 * PAD).max(1));
        let mut lines = vec![];
        for (text, size, color, gap) in sections {
            if text.trim().is_empty() { continue; }
            let style = typography_ui::resolve(&p.profile, Role::Tooltips, RoleStyle::baseline(size as f64));
            let height = (style.font_size * style.line_height).ceil().max(style.font_size.ceil()) as i32;
            for (i, text) in p.wrap_text_role(Role::Tooltips, text, inner, size).into_iter().enumerate() {
                lines.push(Line { text, size, color, height, gap: if i == 0 && !lines.is_empty() { gap } else { 0 } });
            }
        }
        let natural_height = lines.iter().map(|line| line.height + line.gap).sum::<i32>();
        let top = top(p);
        let max_height = ((p.inspection_notice_y() - 12 - top).max(1) as f64 / DIALOG_SCALE).floor() as i32;
        let height = (natural_height + 2 * PAD).min(max_height);
        let scrolling = natural_height + 2 * PAD > height;
        let body_height = (height - 2 * PAD - if scrolling { 20 } else { 0 }).max(1);
        Self { width: inner + 2 * PAD, height, body_height, lines, scrolling }
    }
    fn first(&self, offset: usize) -> usize {
        let mut height = 0;
        let mut last_page = self.lines.len().saturating_sub(1);
        for (i, line) in self.lines.iter().enumerate().rev() {
            if height + line.height + line.gap > self.body_height && i + 1 < self.lines.len() { break; }
            height += line.height + line.gap;
            last_page = i;
        }
        offset.min(last_page)
    }
}
fn top(p: &Painter) -> i32 { (crate::ability_ui::top(&p.profile) - 28).max(8) }
fn transform(p: &Painter, width: i32) -> (f64, (i32, i32)) {
    let scale = p.scale * DIALOG_SCALE;
    let right = p.native_x(p.hud_left() as f64);
    (scale, (right - (width as f64 * scale).round() as i32, p.native_y(top(p) as f64)))
}

pub(super) fn bounds(p: &Painter, a: &AbilityView) -> Option<damage::Rect> {
    let layout = Layout::resolve(p, a);
    let (scale, (x, y)) = transform(p, layout.width);
    let right = x + (layout.width as f64 * scale).round() as i32;
    let bottom = y + (layout.height as f64 * scale).round() as i32;
    let (x, y, right, bottom) = (x.clamp(0, p.width as i32), y.clamp(0, p.height as i32),
        right.clamp(0, p.width as i32), bottom.clamp(0, p.height as i32));
    (right > x && bottom > y).then(|| damage::Rect::new(x as usize, y as usize, (right-x) as usize, (bottom-y) as usize))
}
pub(super) fn paint(p: &mut Painter, a: &AbilityView, offset: usize) {
    let layout = Layout::resolve(p, a);
    let prior = (p.scale, p.origin, p.responsive_canvas, p.clip);
    let (scale, origin) = transform(p, layout.width);
    p.scale = scale; p.origin = origin; p.responsive_canvas = false;
    p.rect(0, 0, layout.width, layout.height, [11, 13, 21]);
    p.border(0, 0, layout.width, layout.height, [53, 55, 68]);
    p.clip = Some((PAD, PAD, layout.width - 2 * PAD, layout.body_height));
    let first = layout.first(offset);
    let mut y = PAD;
    let mut last = first;
    for (i, line) in layout.lines.iter().enumerate().skip(first) {
        let gap = if i == first { 0 } else { line.gap };
        if y + gap + line.height > PAD + layout.body_height && i > first { break; }
        y += gap;
        p.text_role(Role::Tooltips, &line.text, PAD, y, line.size, line.color);
        y += line.height;
        last = i + 1;
    }
    p.clip = Some((PAD, layout.height - PAD - 18, layout.width - 2 * PAD, 18));
    if layout.scrolling {
        let arrows = format!("{}{}", if first > 0 { "↑" } else { "" }, if last < layout.lines.len() { " ↓" } else { "" });
        p.text_role(Role::Tooltips, arrows.trim(), PAD, layout.height - PAD - 18, 12, MUTED);
    }
    (p.scale, p.origin, p.responsive_canvas, p.clip) = prior;
}

#[cfg(test)]
#[path = "ability_dialog_tests.rs"]
mod tests;

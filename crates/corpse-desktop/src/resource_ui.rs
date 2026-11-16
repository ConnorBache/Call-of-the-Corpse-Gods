//! Detached player per-turn resource labels. No simulation mutation or clocks.
use corpse_core::Run;
use super::Painter;

pub fn percent_text(percent: Option<f64>) -> String {
    let Some(value) = percent.filter(|v| v.is_finite() && *v != 0.) else { return String::new(); };
    let sign = if value > 0. { "+" } else { "-" };
    let magnitude = value.abs();
    if magnitude < 0.1 { format!("{sign}<0.1%") }
    else if magnitude < 1. { format!("{sign}{magnitude:.1}%") }
    else if magnitude < 100_000. { format!("{sign}{magnitude:.0}%") }
    else { format!("{sign}{magnitude:.1e}%") }
}
pub fn change_suffix(run: &Run, key: &str, value: f64, maximum: f64) -> String {
    let label = percent_text(corpse_core::resources::player_change_percent(run,key,value,maximum));
    if label.is_empty() { label } else { format!(" - {label}") }
}
pub fn player_changes(run: &Run) -> [String; 3] {
    let energy_max = if run.player.abilities.iter().any(|id| id=="grisha-boundless-hunger") {
        f64::INFINITY
    } else { corpse_core::combat::effective_stats(run,&run.player).max_energy };
    [("hp",run.player.hp,run.player.max_hp),("energy",run.player.energy,energy_max),
        ("blessings",run.exp,corpse_core::progression::level_up_cost(run))].map(|(key,value,max)|
            percent_text(corpse_core::resources::player_change_percent(run,key,value,max)))
}
pub fn value_text(label: &str, value: f64, maximum: f64, change: &str) -> String {
    let value = if label=="Health" { crate::stat_presentation::current_health(value) } else {value};
    if change.is_empty() { format!("{value:.0} / {}",super::resource_max_label(maximum)) }
    else { format!("{value:.0}/{} - {change}",super::resource_max_label(maximum)) }
}
/// Same rounded glyph advance as Painter::text, including fallback glyphs.
/// O(G) font-metric work, no rasterization and O(1) extra space.
pub fn text_width(p: &Painter, text: &str, size: u16) -> i32 {
    let displayed=crate::ability_ui::display_text(text);let text=displayed.as_ref();
    text.chars().map(|ch| {
        let face = if p.font.lookup_glyph_index(ch)!=0 {&p.font} else {
            p.fallback.iter().find(|f|f.lookup_glyph_index(ch)!=0).unwrap_or(&p.font)
        };
        face.metrics(ch,size as f32).advance_width.round() as i32
    }).sum()
}
/// Fixed four font sizes, O(G) metric work and O(G) returned text. Unusually
/// large totals use a leading ellipsis, preserving the signed percentage tail.
pub fn fit_value(p: &Painter, text: &str, left: i32, right: i32) -> (String,u16,i32) {
    let size = (10..=13).rev().find(|size| text_width(p,text,*size)<=right-left).unwrap_or(10);
    let mut width = text_width(p,text,size);
    if width <= right-left { return (text.into(),size,right-width); }
    let ellipsis = text_width(p,"…",size);
    let mut start = 0;
    for (offset,ch) in text.char_indices() {
        width -= text_width(p,&ch.to_string(),size);
        start = offset + ch.len_utf8();
        if width + ellipsis <= right-left { break; }
    }
    let text = format!("…{}",&text[start..]);
    (text,size,(right-width-ellipsis).max(left))
}

#[cfg(test)]
#[path = "resource_ui_tests.rs"]
mod tests;

/// Content-sized ability meter: keep short names and totals on one line;
/// wrap the full name above the total only when the combined line cannot fit.
pub fn ability_bar_layout(p:&Painter,name:&str,value:&str,width:i32,size:u16)->(String,i32) {
    if text_width(p,name,size)+text_width(p,value,13)+20<=width {
        return (name.into(),size.max(13) as i32+6);
    }
    let names=p.wrap_text_role(crate::typography_ui::Role::Ability,name,(width-12).max(1),size);
    let height=(names.len() as i32+1)*(size as i32+2)+4;
    (format!("{}\n",names.join("\n")),height)
}

/// Resource labels are drawn inside the meter. Two clipped text passes switch
/// ink at the fill boundary: readable on both bright fill and dark empty track.
/// O(G + w*h) software work, O(G) fitting storage; GPU emits constant quads and
/// O(G) glyphs. Uses/restores the caller's logical clip without widening it.
pub fn paint_bar(p:&mut Painter,label:&str,value:&str,x:i32,y:i32,w:i32,h:i32,color:super::Rgb,fraction:f64,size:u16) {
    paint_bar_pending(p,label,value,x,y,w,h,color,fraction,size,0.);
}
pub fn paint_bar_pending(p:&mut Painter,label:&str,value:&str,x:i32,y:i32,w:i32,h:i32,color:super::Rgb,fraction:f64,size:u16,pending:f64) {
    let fill=(w as f64*fraction.clamp(0.,1.)).round() as i32;
    p.rect(x,y,w,h,[36,41,53]);p.rect(x,y,fill,h,color);
    let reserved=if pending>0. {(w as f64*pending).round().max(1.) as i32}else{0}.min(fill).max(0);
    let pending_left=x+fill-reserved;
    if reserved>0 {
        // Preserve the underlying resource fill: warning is strokes only.
        // Tile-style slash hatching, clipped to precisely the reserved segment.
        for row in 1..h-1 {
            for column in ((9-row.rem_euclid(9))%9..reserved).step_by(9) {
                p.rect(pending_left+column,y+row,2.min(reserved-column),1,[110,90,235]);
            }
        }
        p.border(pending_left,y,reserved,h,[110,90,235]);
    }
    let multiline=label.contains('\n');
    let start=if multiline{x+6}else{x+6+text_width(p,label,size)+8};
    let(value,value_size,value_x)=fit_value(p,value,start,x+w-6);
    let previous=p.clip;
    let fill_ink=if crate::color::contrast([8,8,12],color)>=crate::color::contrast([250,247,255],color){[8,8,12]}else{[250,247,255]};
    for (left,width,ink) in [(x,fill-reserved,fill_ink),(pending_left,reserved,fill_ink),(x+fill,w-fill,[250,247,255])] {
        let (mut l,mut t,mut r,mut b)=(left,y,left+width,y+h);
        if let Some((cx,cy,cw,ch))=previous {l=l.max(cx);t=t.max(cy);r=r.min(cx+cw);b=b.min(cy+ch);}
        if r<=l||b<=t {continue;}
        p.clip=Some((l,t,r-l,b-t));
        if multiline {for(i,line)in label.lines().enumerate(){p.text(line,x+6,y+2+i as i32*15,size,ink);}}
        else {p.text(label,x+6,y+(h-size as i32)/2-1,size,ink);}
        p.text(&value,value_x,if multiline{y+h-value_size as i32-2}else{y+(h-value_size as i32)/2-1},value_size,ink);
    }
    p.clip=previous;
}

/// Infinity uses the historical 100-point display scale, not an energy cap.
/// Hatching represents upcoming cost magnitude on the same scale, not the
/// resulting fill. Costs above the displayed range saturate. No gameplay change.
pub fn bar_fractions(value:f64,maximum:f64,upcoming:f64)->(f64,f64) {
    let scale=if maximum.is_infinite(){100.}else{maximum.max(1.)};
    let fill=(value/scale).clamp(0.,1.);
    (fill,(upcoming.max(0.)/scale).min(fill))
}

/// Read the same physical amulet pool/capacity used by its existing description.
pub fn amulet_reserve(item:&corpse_core::Item,revision:f64)->Option<(f64,f64)> {
    if revision<50. {return None;}
    let spec=corpse_core::revision50_amulet_effects::item_spec_for_revision(item,revision)?;
    Some((item.amulet_state.as_ref().map_or(spec.capacity,|s|s.energy.min(spec.capacity)).max(0.),spec.capacity))
}

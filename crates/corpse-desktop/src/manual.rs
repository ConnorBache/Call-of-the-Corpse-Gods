//! Authored encyclopedia with player-level discovery, separate from the Run.
use crate::render::{Painter, Picture};
#[path = "manual_ranges.rs"]
mod effect_ranges;
use serde::Deserialize;
use std::collections::BTreeMap;

const INDEX_LEFT: i32 = 16;
const INDEX_RIGHT: i32 = 216;
const INDEX_LABEL_PAD: i32 = 8;
const INDEX_LABEL_SIZE: u16 = 16;
const STAT_LABEL_SIZE: u16 = 16;
const GLYPH_DARK: [u8; 3] = [16, 14, 18];
const GLYPH_LIGHT: [u8; 3] = [255; 3];

fn badge_paper(ink: [u8; 3]) -> [u8; 3] {
    if theme_contrast(ink, GLYPH_DARK) >= 4.5 {
        GLYPH_DARK
    } else if theme_contrast(ink, GLYPH_LIGHT) >= 4.5 {
        GLYPH_LIGHT
    } else {
        [0; 3]
    }
}
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub glyph: String,
    pub color: String,
    pub description: String,
    #[serde(default)]
    pub natural_history: Option<String>,
    pub health: Option<f64>,
    pub stats: Option<BTreeMap<String, f64>>,
    pub mechanics: Vec<Mechanic>,
    pub sources: Vec<Source>,
    pub ordinary_depths: Vec<usize>,
    pub conditional_depths: Vec<usize>,
    pub asset: String,
    pub family: String,
    pub forms: Vec<Form>,
}
#[derive(Deserialize, Clone)]
pub struct Mechanic {
    pub name: String,
    pub text: String,
    pub cost: serde_json::Value,
}
#[derive(Deserialize, Clone)]
pub struct Source {
    pub kind: String,
    pub text: String,
}
#[derive(Deserialize, Clone)]
pub struct Form {
    pub label: String,
    pub description: String,
}
#[derive(Deserialize)]
pub struct Depth {
    pub ordinary: Vec<String>,
    pub conditional: Vec<String>,
}
#[derive(Deserialize)]
pub struct Catalog {
    pub entries: Vec<Entry>,
    pub depths: Vec<Depth>,
    pub companions: Vec<String>,
}
/// Map an already-admitted apparent actor to an authored page. O(C * L),
/// with C fixed pages and L identity length; cached catalog has O(C) storage.
/// Callers must establish visibility and apply disguises before using this.
pub fn page_id_for_actor(actor: &corpse_core::Actor) -> Option<&'static str> {
    static CATALOG: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
    let catalog=CATALOG.get_or_init(||serde_json::from_str(include_str!("../assets/manual.json")).expect("immutable bestiary catalog"));
    let kind=actor.creature_kind.as_ref().map(String::as_str);
    let identity=if kind==Some("zillennia") {
        format!("zillennia:{}",actor.zillennia_quality.as_ref().map_or("unqualified",String::as_str))
    } else if let Some(kind)=kind {kind.to_owned()}
    else {
        // Pale actors in both frozen and current saves identify through their
        // authored names, including the R50 Chomper/Gasser display aliases.
        let name=corpse_core::revision50_creature_identity::legacy_name(corpse_core::enemy_traits::base_name(&actor.name));
        if let Some(role)=name.strip_prefix("Greater Pale ") {format!("pale:Greater {role}")}
        else if let Some(role)=name.strip_prefix("Pale ") {format!("pale:{role}")}
        else {return catalog.entries.iter().find(|e|corpse_core::revision50_creature_identity::legacy_name(&e.name)==name).map(|e|e.id.as_str());}
    };
    catalog.entries.iter().find(|entry|entry.id==identity
        || (entry.kind!="zillennia" && entry.kind==identity)
        || (entry.kind=="zillennia" && entry.asset==identity))
        .map(|entry|entry.id.as_str())
}
pub struct Manual {
    pub progress: crate::bestiary_progress::Progress,
    preview: bool,
    simulation_revision: f64,
    compressed_depths: bool,
    death_gas: bool,
    pub catalog: Catalog,
    pub depth: usize,
    ordered_ids: Vec<String>,
    pub selected: usize,
    pub scroll: usize,
    art: Option<(String, Picture)>,
    void_field: Option<([u8;3],Picture)>,
}
impl Manual {
    pub fn new() -> Self {
        let mut catalog: Catalog = serde_json::from_str(include_str!("../assets/manual.json"))
            .expect("immutable bestiary catalog");
        for (id,name,text,cost) in [
            ("dragon","Flight","Flies to safety for up to 6 turns.",8),
            ("blorb","Caustic touch","Damaging melee adds caustic damage.",0),
            ("killspawn","Deep bleeding","Damaging melee adds 50% of hit damage as Bleeding over 3 turns. Stacks.",0),
        ] {
            if let Some(entry)=catalog.entries.iter_mut().find(|e|e.id==id) {
                entry.mechanics.push(Mechanic {name:name.into(),text:text.into(),cost:cost.into()});
            }
        }
        // O(N log N) once, stable and independent of discovery/run state.
        let mut entries: Vec<_> = catalog.entries.iter().collect();
        entries.sort_by_key(|e| (first_depth(e).unwrap_or(usize::MAX), e.name.to_lowercase(), e.id.clone()));
        let ordered_ids = entries.into_iter().map(|e| e.id.clone()).collect();
        let mut manual=Self {
            progress: Default::default(),
            preview: false,
            simulation_revision: 52.,
            compressed_depths: true,
            death_gas: false,
            catalog,
            depth: 0,
            ordered_ids,
            selected: 0,
            scroll: 0,
            art: None,
            void_field: None,
        };
        manual.set_death_gas(true);
        manual
    }
    /// Only the originating run's scalar revision crosses the UI boundary.
    /// Menu/inactive runs use current rules; discovery is independent of this choice.
    pub fn set_simulation_revision(&mut self, revision: Option<f64>) {
        self.simulation_revision = revision.unwrap_or(52.);
    }
    pub fn set_death_gas(&mut self, current: bool) {
        if current == self.death_gas { return; }
        self.death_gas = current;
        if let Some(entry)=self.catalog.entries.iter_mut().find(|e|e.id=="pale:Greater Breather") {
            for mechanic in &mut entry.mechanics {
                match mechanic.name.as_str() {
                    "Hallucinogenic Gas Cloud" | "Death Gas Cloud" => {
                        mechanic.name=if current {"Death Gas Cloud"}else{"Hallucinogenic Gas Cloud"}.into();
                        mechanic.text=if current {"Emits Death Gas: about 6.7% max HP per turn + Hallucinating (5 turns, refreshed in the gas)."}
                            else {"Emits hallucinogenic gas; refreshes Hallucinating on exposure."}.into();
                    }
                    "Forceful Exhalation" => mechanic.text=if current {"Exhales an extra cloud of Death Gas."}
                        else {"Exhales an extra hallucinogenic cloud."}.into(),
                    "Caustic Gas Immunity" | "Pale Gas Immunity" => {
                        mechanic.name=if current {"Pale Gas Immunity"}else{"Caustic Gas Immunity"}.into();
                        mechanic.text=if current {"Immune to Death Gas, caustic gas, and hallucinogenic gas."}
                            else {""}.into();
                    }
                    _ => (),
                }
            }
        }
    }
    pub fn set_compressed_depths(&mut self, compressed: bool) { self.compressed_depths = compressed; }
    pub fn preview_all(&mut self) { self.preview = true; }
    pub fn mark_viewed(&mut self) { if let Some(id)=self.ids().get(self.selected).cloned() {self.progress.view(&id);} }
    fn unlocked(&self,id:&str)->bool {self.preview || self.progress.seen(id)}
    pub fn ids(&self) -> &[String] { &self.ordered_ids }
    pub fn step(&mut self, delta: i32) {
        self.selected = (self.selected as i32 + delta)
            .clamp(0, self.ids().len().saturating_sub(1) as i32) as usize;
        self.depth = self.entry().and_then(first_depth).unwrap_or(40).saturating_sub(1);
        self.scroll = 0;
    }
    pub fn page(&mut self, delta: i32) { self.step(delta * 20); }
    pub fn select_id(&mut self, id: &str) -> bool {
        if let Some(i) = self.ids().iter().position(|v| v == id) {
            self.selected = i;
            self.step(0);
            true
        } else { false }
    }
    pub fn entry(&self) -> Option<&Entry> {
        let id = self.ids().get(self.selected)?;
        self.catalog.entries.iter().find(|e| &e.id == id)
    }
    pub fn click(&mut self, x: i32, y: i32) {
        if !(INDEX_LEFT..INDEX_RIGHT).contains(&x) {
            return;
        }
        if x < 210 && (80..109).contains(&y) {
            if x >= 184 {
                self.page(1);
            } else if x >= 158 {
                self.page(-1);
            }

        } else if (150..702).contains(&y) {
            let i = self.selected.saturating_sub(10) + ((y - 150) / 24) as usize;
            if i < self.ids().len() {
                self.selected = i;
                self.step(0);
            }
        } else if x < 210 && (708..734).contains(&y) {
            self.step(if x < 113 { -1 } else { 1 });
        }
        self.scroll = 0;
    }
    fn paint_storage_error(&self,p:&mut Painter,ink:[u8;3]) {
        if let Some(error)=&self.progress.error {p.prose(&crate::render::truncate(error,120),248,707,14,ink,false);}
    }
    // Decode once per selection. Artwork and authored alpha remain untouched.
    fn prepare_art(&mut self) {
        let Some(entry) = self.entry().cloned() else { return; };
        if self.art.as_ref().map(|(id, _)| id) != Some(&entry.asset) {
            self.art = crate::embedded_assets::creature(&entry.asset).map(|bytes| (
                entry.asset.clone(), trim_transparent_margin(crate::embedded_assets::creature_picture(bytes))));
        }
    }
    // Layout and stats use immutable public catalogs, never actors from a live Run.
    pub fn paint(&mut self, p: &mut Painter) {
        self.paint_at(p, 0.);
    }
    pub fn paint_at(&mut self, p: &mut Painter, _time_ms: f64) {
        // First-appearance navigation controls only the solid page color.
        // There is no selected dungeon floor: stats span the species' birth profiles.
        self.paint_page(p, self.depth as f64);
    }
    // Render-test depth override preserves the immutable roster and selected entry.
    pub fn paint_at_depth(&mut self, p: &mut Painter, _time_ms: f64, depth: usize) {
        self.paint_page(p, depth.min(39) as f64);
    }
    // Fractional render-only preview changes color, never the stat range.
    pub fn paint_at_visual_depth(&mut self, p: &mut Painter, _time_ms: f64, depth: f64) {
        let depth = if depth.is_finite() { depth.clamp(0., 39.) } else { 0. };
        self.paint_page(p, depth);
    }
    fn paint_page(&mut self, p: &mut Painter, color_depth: f64) {
        self.scroll = 0;
        let theme = Theme::at_depth(color_depth);
        let ink = theme.ink;
        let paper = theme.paper;
        p.clip = None;
        p.clear(paper);
        if self.entry().is_some_and(|e|self.unlocked(&e.id)) {self.prepare_art();} else {self.art=None;}
        p.prose_contrast_outline = false;
        // One uniform full-screen field; no scenery, noise, texture or animated backdrop.
        p.rect(16, 28, 4, 44, theme.accent);
        
        p.prose("BESTIARY", 28, 38, 25, ink, true);
        p.rect(224, 26, 1, 704, theme.rule);
        p.border(158, 80, 24, 28, ink);
        p.border(184, 80, 24, 28, ink);
        p.prose("‹", 164, 83, 18, ink, false);
        p.prose("›", 190, 83, 18, ink, false);
        p.rect(INDEX_LEFT, 139, INDEX_RIGHT - INDEX_LEFT, 1, ink);
        p.border(16, 708, 93, 26, ink);
        p.border(115, 708, 95, 26, ink);
        p.prose("‹", 28, 710, 18, ink, false);
        p.prose("›", 128, 710, 18, ink, false);
        let first = self.selected.saturating_sub(10);
        for (i, id) in self.ids().iter().enumerate().skip(first).take(23) {
            if let Some(e) = self.catalog.entries.iter().find(|e| &e.id == id) {
                let y = 153 + (i - first) as i32 * 24;
                if i == self.selected {
                    p.rect(INDEX_LEFT, y - 3, INDEX_RIGHT - INDEX_LEFT, 24, ink);
                }
                let name=if self.unlocked(id){crate::display_names::present(&e.name)}else{"".into()};
                p.prose(
                    &name,
                    INDEX_LEFT + INDEX_LABEL_PAD,
                    y,
                    INDEX_LABEL_SIZE,
                    if i == self.selected { theme.selected_ink } else { ink }, false);
            }
        }
        let Some(mut entry) = self.entry().cloned() else {
            p.prose(
                "No creature selected.",
                250,
                230,
                19,
                ink, false);
            paint_footer(p, ink);
            p.prose_contrast_outline = false;
            return;
        };
        if !self.unlocked(&entry.id) {
            paint_footer(p,ink);
            self.paint_storage_error(p,ink);
            p.prose_contrast_outline=false;
            return;
        }
        self.paint_storage_error(p,ink);
        if entry.id == "bastidion" { entry.conditional_depths = birth_depths(&entry, self.simulation_revision); }
        // Detached presentation copy: never change source spawn ranges or stat lookup depths.
        let mut displayed_entry=entry.clone();
        if self.compressed_depths {
            displayed_entry.ordinary_depths=compressed_ranges(&entry.ordinary_depths);
            displayed_entry.conditional_depths=compressed_ranges(&entry.conditional_depths);
        }
        let page=resolve_page(p,&displayed_entry,self.art.as_ref().map(|(_,a)|a));
        let layout=&page.content;
        let title=crate::display_names::present(&entry.name).to_uppercase();
        let mut title_size = 50;
        while p.prose_width(&title, title_size, true) > 990 { title_size -= 1; }
        p.prose(&format!("{} / {}", entry.family.to_uppercase(), first_depth(&displayed_entry)
            .map(|d| format!("FIRST APPEARS AT DEPTH {d:02}"))
            .unwrap_or_else(|| "COMPANION".into())),248,26,16,ink,false);
        p.prose(&title,248,48,title_size,ink,true);
        p.rect(248,106,64,4,theme.accent);
        if let Some((_,art))=&self.art {
            let (x,y,aw,ah)=page.art;
            let separate_void=entry.family=="void";
            let inset=if separate_void {theme.void_inset}else{0};
            if separate_void {
                let field=theme.void_paper;
                if self.void_field.as_ref().map(|(color,_)|*color)!=Some(field) {
                    let mut backing=crate::embedded_assets::compact_picture(include_bytes!("../assets/ui-runtime/bestiary-ink-splash-broad.webp"));
                    for pixel in backing.rgba.chunks_exact_mut(4) {pixel[..3].copy_from_slice(&field);}
                    self.void_field=Some((field,backing));
                }
                p.picture(&self.void_field.as_ref().unwrap().1,x,y,aw,ah);
            }
            let (w,h)=contain(art.w,art.h,aw-inset*2,ah-inset*2);
            p.picture(art,x+(aw-w)/2,y+(ah-h)/2,w,h);
        }
        let (ax,sy,aw,row)=page.stats;
        p.rect(ax,sy,aw,2,ink);
        p.prose("BASE STATS",ax,sy+12,16,ink,true);
        let glyph_ink = crate::color::hex(&entry.color);
        p.rect(ax+aw-32,sy+9,30,28,badge_paper(glyph_ink));
        p.map_identity(&entry.glyph,ax+aw-30,sy+10,26,26,glyph_ink);
        for (i,(label,value)) in stat_rows(&entry,self.simulation_revision).iter().enumerate() {
            let cell=aw/if row {8}else{4};
            let x=ax+(if row {i}else{i%4}) as i32*cell;
            let y=sy+45+if row {0}else{(i/4) as i32*59};
            p.prose(label,x,y,STAT_LABEL_SIZE,ink,false);
            p.prose(value,x,y+21,stat_value_size(p,value,cell-6),ink,true);
        }
        for (i,line) in layout.lines.iter().enumerate() {
            let col=usize::from(page.text_x.len()>1 && i>=layout.split);
            let y=page.text_y+layout.offsets[i]-if col==1 {layout.offsets[layout.split]} else {0};
            if line.heading {
                p.prose(&line.text,page.text_x[col],y,layout.size,ink,true);
            } else {
                paint_content_line(p,line,page.text_x[col],y,layout.size,ink);
            }
        }
        for (x,y,extra) in &page.extras {
            for (i,line) in extra.lines.iter().enumerate() {
                let ly=y+extra.offsets[i];
                if line.heading {p.prose(&line.text,*x,ly,extra.size,ink,true);} else {paint_content_line(p,line,*x,ly,extra.size,ink);}
            }
        }
        // All UI is painted after the illustration and its ink ground.
        paint_footer(p, ink);
        p.prose_contrast_outline = false;
    }
}

fn paint_footer(p: &mut Painter, ink: [u8; 3]) {
    p.rect(16, 741, 1242, 3, ink);
    p.prose("Esc to close",18,754,14,ink,false);
}

#[derive(Debug)]
struct Theme {
    paper: [u8; 3],
    ink: [u8; 3],
    accent: [u8; 3],
    rule: [u8; 3],
    selected_ink: [u8; 3],
    void_paper: [u8; 3],
    void_inset: i32,
}
fn mix_color(a: [u8;3], b: [u8;3], t: f64) -> [u8;3] {
    let t=t.clamp(0.,1.);
    std::array::from_fn(|i|(a[i] as f64+(b[i] as f64-a[i] as f64)*t).round() as u8)
}
impl Theme {
    fn at(depth: usize) -> Self { Self::at_depth(depth as f64) }
    fn at_depth(depth: f64) -> Self {
        // O(1), piecewise-linear depth colors. Fractional previews are not rounded
        // to whole floors. Only final 8-bit channel quantization remains.
        let d=if depth.is_finite() {depth.clamp(0.,39.)} else {0.};
        let paper=if d<=24. {mix_color([239,232,204],[143,28,32],d/24.)}
            else {mix_color([143,28,32],[0,0,0],(d-24.)/15.)};
        // The paper still blends continuously. Select the stronger solid ink:
        // continuous black-to-white letter fill would pass through a nearly
        // invisible midpoint, and an outline swallows the small counter shapes.
        let ink = contrasting_ink(paper);
        let selected_ink = contrasting_ink(ink);
        let accent=[216,234,27];
        let rule=mix_color(paper,ink,0.5);
        // The void backing starts as exactly the page color, then approaches
        // the established burgundy field. No luminance-triggered appearance.
        let void_progress=((d-18.)/10.).clamp(0.,1.);
        let void_paper=if d<=18. {paper}
            else {mix_color([167,79,75],[150,64,70],void_progress)};
        let void_inset=(12.*void_progress).round() as i32;
        Self {paper,ink,accent,rule,selected_ink,void_paper,void_inset}
    }
}
fn contrasting_ink(background: [u8; 3]) -> [u8; 3] {
    if theme_contrast([0; 3], background) >= theme_contrast([255; 3], background) {
        [0; 3]
    } else {
        [255; 3]
    }
}
fn relative_luminance(rgb: [u8;3]) -> f64 {
    let c = rgb.map(|v| { let s=v as f64/255.; if s<=0.04045 {s/12.92} else {((s+0.055)/1.055).powf(2.4)} });
    c[0]*0.2126+c[1]*0.7152+c[2]*0.0722
}
/// O(D), D <= 40 authored depths; lists remain sorted and duplicate-free.
fn compressed_ranges(depths:&[usize])->Vec<usize> {
    let mut mapped:Vec<_>=(1..=20).filter(|d| depths.contains(&(corpse_core::fast_mode::content_depth(*d) as usize))).map(|d|d as usize).collect();
    mapped.sort_unstable();mapped.dedup();mapped
}
fn first_depth(entry: &Entry) -> Option<usize> {
    entry.ordinary_depths.iter().chain(&entry.conditional_depths).copied().min()
}

#[cfg(test)]
fn signature(id: &str, family: &str) -> u32 {
    id.bytes()
        .chain(family.bytes())
        .fold(2166136261u32, |hash, b| {
            (hash ^ b as u32).wrapping_mul(16777619)
        })
}
fn contain(w: usize, h: usize, max_w: i32, max_h: i32) -> (i32, i32) {
    let scale = (max_w as f64 / w.max(1) as f64).min(max_h as f64 / h.max(1) as f64);
    (
        (w as f64 * scale).round().max(1.) as i32,
        (h as f64 * scale).round().max(1.) as i32,
    )
}
// Trim only fully transparent margin from offline-reviewed/frozen RGBA assets. Every occupied RGBA pixel
// is copied verbatim; this reveals silhouette proportions instead of square asset canvases.
// Cold selection O(P) scan/copy, O(P) output storage; no work on warm frames.
fn trim_transparent_margin(art:Picture)->Picture {
    let (mut x0,mut y0,mut x1,mut y1)=(art.w,art.h,0,0);
    for (i,pixel) in art.rgba.chunks_exact(4).enumerate() {
        if pixel[3]>0 {let x=i%art.w;let y=i/art.w;x0=x0.min(x);y0=y0.min(y);x1=x1.max(x+1);y1=y1.max(y+1);}
    }
    if x1<=x0 || y1<=y0 {return art;}
    let mut rgba=Vec::with_capacity((x1-x0)*(y1-y0)*4);
    for y in y0..y1 {rgba.extend_from_slice(&art.rgba[(y*art.w+x0)*4..(y*art.w+x1)*4]);}
    Picture {w:x1-x0,h:y1-y0,rgba}
}
const STAT_LABELS: [&str; 7] = ["HP", "Energy", "Accuracy", "Damage", "Dodge", "DR", "Perception"];

/// The current catalog owns R52 species reference rows, not the Bestiary's
/// historical appearance metadata. Birth contexts share their canonical species.
fn birth_depths(entry: &Entry, revision: f64) -> Vec<usize> {
    if revision == 52. {
        let key = corpse_core::balance52::canonical_kind(&entry.id);
        let depths: Vec<_> = corpse_core::balance52::profiles().iter()
            .filter(|r| r.kind == key).map(|r| r.floor as usize).collect();
        if !depths.is_empty() { return depths; }
    }
    if entry.id.starts_with("zillennia:") {
        return corpse_core::population::catalog().zillennia.iter().enumerate()
            .filter(|(_, a)| a.zillennia_quality.as_ref().is_some_and(|q| entry.id == format!("zillennia:{q}")))
            .map(|(i, _)| i + 1).collect();
    }
    if entry.id == "bastidion" {
        if revision >= 51. {
            // Cold-only immutable admission cache. Pool construction/scan costs
            // are documented separately; no geometry or hidden Run is inspected.
            static R51: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
            static R52: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
            return (if revision == 52. { &R52 } else { &R51 }).get_or_init(|| {
                fungus_setup_depths(revision).into_iter().filter(|&depth|
                    corpse_core::fungus_feasts::victim_pools(depth as u32, revision)
                        .is_ok_and(|pools| pools.iter().all(|pool| !pool.is_empty()))).collect()
            }).clone();
        }
        return fungus_setup_depths(revision);
    }
    let population = corpse_core::population::catalog();
    if corpse_core::ordinary_roster::species_revision(revision).iter().any(|s| s.id == entry.id)
        || population.pairs.values().any(|id| id == &entry.id) {
        return (1..=40).filter(|&depth| corpse_core::ordinary_roster::candidates_revision(depth, true, revision)
            .is_ok_and(|recipes| recipes.iter().any(|r| r.kinds.contains(&entry.id))))
            .map(|d| d as usize).collect();
    }
    if entry.id.contains("ichor-") {
        return (1..=40).filter(|&depth| corpse_core::ichor::eligible_fish_kinds(depth, revision).contains(&entry.id.as_str()))
            .map(|d| d as usize).collect();
    }
    // Remaining fixed special births have their authored source floors; no
    // depth-dependent profile is inferred when an authoritative birth is absent.
    let mut depths: Vec<_> = entry.ordinary_depths.iter().chain(&entry.conditional_depths)
        .copied().filter(|d| (1..=40).contains(d)).collect();
    depths.sort_unstable(); depths.dedup(); depths
}
fn fungus_setup_depths(revision: f64) -> Vec<usize> {
    (1..=40).filter(|&depth| corpse_core::generation::setup_definitions().iter().any(|setup|
        // Setup24 stamps its fungus directly; frozen metadata has no kinds.
        (setup.id == 24 || setup.kinds.iter().any(|kind| corpse_core::fungus_feasts::is_fungus(kind)))
            && corpse_core::generation::setup_eligible_at_depth_revision(setup, depth, revision).unwrap_or(false)))
        .map(|d| d as usize).collect()
}

/// Reconstruct only a public, unwounded birth template. The explicit special
/// cases mirror startup/pocket births; context offspring are not natural actors.
fn birth_actor(entry: &Entry, depth: usize, revision: f64) -> Option<corpse_core::Actor> {
    if entry.id.starts_with("zillennia:") {
        return corpse_core::population::zillennia_revision(depth as u32, revision).ok()
            .filter(|a| a.zillennia_quality.as_ref().is_some_and(|q| entry.id == format!("zillennia:{q}")));
    }
    if entry.id == "bastidion" {
        return corpse_core::population::fungus_revision(depth as u32, revision).ok();
    }
    let (birth_revision, profile_key) = match entry.id.as_str() {
        // Keep the Sphinx chamber body; add only the R52 Accuracy/Dodge baselines below.
        "sphinx" => (29., entry.id.as_str()),
        // The Queen has one public birth source: a Nest, introduced in revision33.
        "saidribee-queen" if revision >= 33. => (33., "nest:saidribee-queen"),
        "saidribee-queen" => return None,
        _ => (revision, entry.id.as_str()),
    };
    let mut actor = corpse_core::population::create_actor_revision(&entry.id, 0., 0., 0., false, birth_revision).ok()?;
    if entry.id == "sphinx" {
        actor.stats.accuracy=corpse_core::combat::creature_birth_accuracy(revision,actor.stats.accuracy);
        actor.stats.dodge=corpse_core::combat::creature_birth_dodge(revision,actor.stats.dodge);
    }
    corpse_core::balance52::apply_new_actor_kind(&mut actor, profile_key, depth as f64, revision);
    Some(actor)
}
fn actor_stat_values(actor: &corpse_core::Actor, revision: f64) -> [f64; 7] {
    let stats = &actor.stats;
    [actor.max_hp, stats.max_energy, stats.accuracy,
        crate::stat_presentation::enemy_damage_revision(actor, revision), stats.dodge, stats.dr, stats.perception]
}
/// After floor admission, O(D * (A + T + R)): D <= 40 floors, A cloned actor
/// fields, T legacy tuning and R current birth rows. O(A + D + S) scratch, S = 7.
/// Floor/recipe and cold-cache helper costs: docs/revision52-bestiary-stats.md.
/// No live actors, RNG, discovery changes or combat calibration from page colors.
fn stat_rows(entry: &Entry, revision: f64) -> Vec<(&'static str, String)> {
    let depths = birth_depths(entry, revision);
    let mut bounds: Option<([f64; 7], [f64; 7])> = None;
    let mut uses_energy = false;
    // No public birth is not proof of incapability: character-dependent pages
    // retain Varies. A supported no-depth template may still omit unused Energy.
    let unknown_energy = depths.is_empty() && birth_actor(entry, 1, revision).is_none();
    if depths.is_empty() {
        uses_energy = birth_actor(entry, 1, revision).is_some_and(|a|
            crate::monster_status_ui::energy_capable(revision, &a));
    }
    for depth in depths {
        let Some(actor) = birth_actor(entry, depth, revision) else {
            // Unsupported/character-dependent entries must not borrow stale JSON stats.
            return STAT_LABELS.into_iter().map(|label| (label, "Varies".into())).collect();
        };
        uses_energy |= crate::monster_status_ui::energy_capable(revision, &actor);
        let values = actor_stat_values(&actor, revision);
        if let Some((low, high)) = bounds.as_mut() {
            for i in 0..7 { low[i] = low[i].min(values[i]); high[i] = high[i].max(values[i]); }
        } else { bounds = Some((values, values)); }
    }
    STAT_LABELS.into_iter().enumerate().filter(|(_, label)|
        *label != "Energy" || uses_energy || unknown_energy).map(|(i, label)| {
        let value = match bounds {
            Some((low, high)) if low[i] == high[i] => display_stat_number(low[i]),
            Some((low, high)) => format!("{}–{}", display_stat_number(low[i]), display_stat_number(high[i])),
            None => "Varies".into(),
        };
        (label, value)
    }).collect()
}
// Legacy templates can retain fractional stats. Round only the rendered text,
// never actor values or gameplay, and do not print repeating binary fractions.
fn display_stat_number(value: f64) -> String {
    crate::stat_presentation::whole_number(value)
}
fn stat_value_size(p: &Painter, value: &str, width: i32) -> u16 {
    (12..=24).rev().find(|size| p.prose_width(value, *size, true) <= width).unwrap_or(12)
}
// Remove only complete, byte-identical leading sentences already shown in APPEARANCE.
// A suffix can be reconstructed losslessly with its shared authored prefix.
fn form_suffix<'a>(base: &str, form: &'a str) -> &'a str {
    let mut shared = 0;
    for (at, _) in base.match_indices(". ") {
        let end = at + 2;
        if form.starts_with(&base[..end]) { shared = end; } else { break; }
    }
    &form[shared..]
}
fn all_biomes_source(source: &Source) -> bool {
    source.kind == "terrain" && source.text == "Found in special locations in these regions: Dungeon, Cave, Meadow, Forest, Archipelago, Mountain."
}
fn redundant_source(source: &Source) -> bool {
    all_biomes_source(source)
        || (source.kind == "ordinary" && source.text == "May appear in roaming groups.")
}
fn mechanic_heading(mechanic: &Mechanic) -> &str {
    &mechanic.name
}
struct ContentLine {
    text: String,
    heading: bool,
    emphasis: Vec<std::ops::Range<usize>>,
}
struct ContentLayout {
    lines: Vec<ContentLine>,
    offsets: Vec<i32>,
    split: usize,
    size: u16,
    height: i32,
}
// At most five font candidates: O(C) font-metric work and O(C) temporary memory,
// where C is authored selected-entry characters; partitioning never drops a line.
const LATIN_PLACEHOLDER: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.";
fn natural_history(entry: &Entry) -> &str {
    entry.natural_history.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or(LATIN_PLACEHOLDER)
}
// Match only the setting name, not anatomical uses such as "its flesh".
// ASCII case folding preserves UTF-8 byte offsets. O(C) time and temporary memory.
fn setting_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    text.to_ascii_lowercase().match_indices("the flesh").filter_map(|(start, _)| {
        let end = start + "the Flesh".len();
        let boundary = |c: char| !c.is_alphanumeric() && c != '_';
        if text[..start].chars().next_back().is_none_or(boundary)
            && text[end..].chars().next().is_none_or(boundary) {
            Some(start..end)
        } else { None }
    }).collect()
}
fn body_lines(p: &Painter, body: &str, width: i32, size: u16) -> Vec<ContentLine> {
    let mut body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let ranges = setting_ranges(&body);
    for range in &ranges {
        body.replace_range(range.start + 4..range.end, "Flesh");
    }
    let mut offset = 0;
    let mut first_range = 0;
    p.prose_lines(&body, width, size).into_iter().map(|text| {
        let end = offset + text.len();
        while first_range < ranges.len() && ranges[first_range].end <= offset { first_range += 1; }
        let emphasis = ranges[first_range..].iter().take_while(|range| range.start < end).filter_map(|range| {
            let start = range.start.max(offset);
            let stop = range.end.min(end);
            (start < stop).then(|| start-offset..stop-offset)
        }).collect();
        offset = end + 1;
        ContentLine { text, heading: false, emphasis }
    }).collect()
}
fn paint_content_line(p: &mut Painter, line: &ContentLine, x: i32, y: i32, size: u16, ink: [u8; 3]) {
    let mut start = 0;
    let mut cursor = x;
    for range in &line.emphasis {
        let normal = &line.text[start..range.start];
        p.prose(normal, cursor, y, size, ink, false);
        cursor += p.prose_width(normal, size, false);
        let bold = &line.text[range.clone()];
        p.prose(bold, cursor, y, size, ink, true);
        cursor += p.prose_width(bold, size, true);
        start = range.end;
    }
    p.prose(&line.text[start..], cursor, y, size, ink, false);
}
struct PageLayout {
    art:(i32,i32,i32,i32),
    stats:(i32,i32,i32,bool),
    text_x:Vec<i32>,
    text_y:i32,
    content:ContentLayout,
    extras:Vec<(i32,i32,ContentLayout)>,
}
fn resolve_page(p:&Painter,entry:&Entry,art:Option<&Picture>)->PageLayout {
    let tall=art.is_some_and(|a|a.h*10>a.w*14);
    let wide=art.is_some_and(|a|a.w*10>a.h*17);
    if wide {
        let content=fit_content_in(p,entry,484,true,300);
        if content.height<=300 && content.size>=18 {
            return PageLayout {art:(248,120,1010,204),stats:(248,330,1010,true),text_x:vec![248,774],text_y:430,content,extras:vec![]};
        }
    }
    let (mut ax,mut aw,mut tx,mut tw)=if tall {(858,400,248,560)} else {(248,500,798,460)};
    let mut content=fit_content_in(p,entry,tw,false,610);
    if content.size<18 {
        ax=248;aw=370;tx=658;tw=600;
        content=fit_content_in(p,entry,tw,false,610);
    }
    if content.size<18 && entry.mechanics.len()==3 && entry.forms.is_empty() {
        let narrative=fit_content_part(p,entry,600,false,460,-2);
        let mut extras=vec![(248,594,fit_content_part(p,entry,370,false,140,-3))];
        for i in 0..3 {extras.push((658+i as i32*204,594,fit_content_part(p,entry,192,false,140,i as isize)));}
        if narrative.size>=18 && narrative.height<=460 && extras.iter().all(|(_,_,l)|l.size>=18 && l.height<=140) {
            return PageLayout {art:(248,128,370,292),stats:(248,440,370,false),text_x:vec![658],text_y:124,content:narrative,extras};
        }
    }
    if content.height>610 || content.size<18 {
        let content = fit_content_in(p, entry, 288, true, 610);
        if content.lines.get(content.split).is_some_and(|line| !line.heading) {
            let narrative = fit_content_part(p, entry, 355, false, 610, -2);
            let reference = fit_content_part(p, entry, 288, false, 610, -4);
            if [&narrative, &reference].iter().all(|part| part.size >= 18 && part.height <= 610) {
                return PageLayout {art:(248,128,325,418),stats:(248,566,325,false),text_x:vec![595],text_y:124,content:narrative,extras:vec![(970,124,reference)]};
            }
        }
        return PageLayout {art:(248,128,380,418),stats:(248,566,380,false),text_x:vec![660,970],text_y:124,content,extras:vec![]};
    }
    PageLayout {art:(ax,128,aw,418),stats:(ax,566,aw,false),text_x:vec![tx],text_y:124,content,extras:vec![]}
}
#[cfg(test)]
fn fit_content(p: &Painter, entry: &Entry) -> ContentLayout { fit_content_in(p,entry,288,true,610) }
fn fit_content_in(p:&Painter,entry:&Entry,width:i32,columns:bool,max_height:i32)->ContentLayout {fit_content_part(p,entry,width,columns,max_height,-1)}
fn fit_content_part(p:&Painter,entry:&Entry,width:i32,columns:bool,max_height:i32,part:isize)->ContentLayout {
    let mut fallback = None;
    for size in (16..=20).rev() {
        let mut lines = Vec::new();
        let mut section = |heading: &str, body: String| {
            // Wrap long ability names with the actual display metrics, never squeeze them.
            for text in p.prose_lines(heading, width, size) {
                lines.push(ContentLine { text, heading: true, emphasis: vec![] });
            }
            lines.extend(body_lines(p, &body, width, size));
        };
        if part == -1 || part == -2 {
        section("APPEARANCE", entry.description.clone());
        section("NATURAL HISTORY", natural_history(entry).to_owned());
        for form in &entry.forms {
            section(&form.label.to_uppercase(), form_suffix(&entry.description, &form.description).to_owned());
        }
        }
        for (index,mechanic) in entry.mechanics.iter().enumerate() {
            if part != -1 && part != -4 && part != index as isize {continue;}
            let cost = if mechanic.cost.as_str() == Some("all") {
                " · all NRG".into()
            } else if mechanic.cost.as_f64().unwrap_or(0.) > 0. {
                format!(" · {} NRG", mechanic.cost)
            } else {
                String::new()
            };
            section(
                &format!("{}{}", crate::ability_ui::display_text(mechanic_heading(mechanic)), cost),
                effect_ranges::page_text(entry, mechanic).to_owned(),
            );
        }
        let mut sources = Vec::new();
        if !entry.ordinary_depths.is_empty() {
            sources.push(format!(
                "Roaming depths: {}.",
                ranges(&entry.ordinary_depths)
            ));
        }
        if !entry.conditional_depths.is_empty() {
            sources.push(format!(
                "Special depths: {}.",
                ranges(&entry.conditional_depths)
            ));
        }
        sources.extend(entry.sources.iter().filter(|s| !redundant_source(s)).map(|s| s.text.clone()));
        if !sources.is_empty() && (part == -1 || part == -3 || part == -4) {
            section("WHERE IT APPEARS", sources.join(" "));
        }
        let mut offsets = vec![0];
        for (i,line) in lines.iter().enumerate() {
            if i>0 && line.heading && !lines[i-1].heading {
                *offsets.last_mut().unwrap()+=if line.text=="NATURAL HISTORY" || line.text=="WHERE IT APPEARS" {14}else{9};
            }
            offsets.push(offsets.last().unwrap()+if line.heading {size as i32+7}else{size as i32+4});
        }
        let total = *offsets.last().unwrap();
        let section_split = (1..lines.len())
            .filter(|&i| lines[i].heading)
            .min_by_key(|&i| offsets[i].max(total - offsets[i]));
        let split = if !columns { lines.len() } else { section_split
            .filter(|&i| offsets[i].max(total - offsets[i]) <= max_height)
            .unwrap_or_else(|| {
                (1..lines.len())
                    .filter(|&i| !lines[i - 1].heading)
                    .min_by_key(|&i| offsets[i].max(total - offsets[i]))
                    .unwrap_or(lines.len())
            }) };
        let height = offsets[split].max(total - offsets[split]);
        let layout = ContentLayout {
            lines,
            offsets,
            split,
            size,
            height,
        };
        if height <= max_height {
            let starts_at_heading = layout.lines.get(layout.split).is_none_or(|line| line.heading);
            if !columns || starts_at_heading && (size >= 18 || fallback.is_none()) {
                return layout;
            }
            if fallback.is_none() { fallback = Some(layout); }
        } else if size == 16 && fallback.is_none() {
            return layout;
        }
    }
    fallback.expect("at least the minimum-size layout")
}

pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}
fn ranges(values: &[usize]) -> String {
    if values.is_empty() {
        return "none".into();
    }
    let mut groups = Vec::new();
    let (mut start, mut last) = (values[0], values[0]);
    for &v in &values[1..] {
        if v == last + 1 {
            last = v;
        } else {
            groups.push(if start == last {
                start.to_string()
            } else {
                format!("{start}-{last}")
            });
            start = v;
            last = v;
        }
    }
    groups.push(if start == last {
        start.to_string()
    } else {
        format!("{start}-{last}")
    });
    groups.join(", ")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "opt-in optimized paint timing; not a correctness assertion"]
    fn benchmark_animated_manual_warm_frames() {
        for (width,height) in [(1280,800),(1920,1080)] {
            let mut p=Painter::new();
            p.resize(width,height);
            for (id,depth) in [("blorb",0),("wendigo",14),("bastidion",39),("zillennia:transcendent",39)] {
                let mut m=Manual::new(); m.preview_all();
                assert!(m.select_id(id));
                m.paint_at_depth(&mut p,0.,depth);
                m.paint_at_depth(&mut p,83.,depth);
                let start=std::time::Instant::now();
                for frame in 0..24 {
                    m.paint_at_depth(&mut p,frame as f64*83.,depth);
                    std::hint::black_box(&p.pixels);
                }
                eprintln!("MANUAL_WARM {width}x{height} {id} depth={} mean_ms={:.3} frames=24",depth+1,start.elapsed().as_secs_f64()*1000./24.);
            }
        }
    }
    #[test]
    fn clean_art_pages_have_no_animated_decal_pass() {
        let source = include_str!("manual_direction.rs");
        assert!(!source.contains("Painter"));
        assert!(!source.contains("paint_interaction"));
        let mut m = Manual::new(); m.preview_all();
        let mut p = Painter::new();
        for id in ["dragon", "bastidion", "dryad"] {
            assert!(m.select_id(id));
            m.paint_at(&mut p, 0.);
            let first = p.pixels.clone();
            m.paint_at(&mut p, 1537.);
            assert_eq!(first, p.pixels, "{id}: unexpected animated overlay");
        }
    }
    #[test]
    fn artwork_never_overlaps_title_navigation_or_footer() {
        for id in ["pale:Screamer","river-drake","zillennia:timeless"] {
            let mut m=Manual::new(); m.preview_all(); assert!(m.select_id(id));
            let mut p=Painter::new(); m.paint(&mut p);
            assert!(!p.prose_contrast_outline);
            let with_art=p.pixels.clone();
            for px in m.art.as_mut().unwrap().1.rgba.chunks_exact_mut(4) { px[3]=0; }
            m.paint(&mut p);
            for y in 0..800 { for x in 0..1280 {
                if x<240 || y<120 || y>=550 {
                    let at=(y*crate::render::WIDTH+x)*4;
                    assert_eq!(&with_art[at..at+4],&p.pixels[at..at+4],"{id}: art outside protected stage at {x},{y}");
                }
            }}
        }
    }
    #[test]
    fn all_actual_layouts_keep_prose_in_page_and_headings_with_body() {
        let mut m=Manual::new(); m.preview_all(); let p=Painter::new();
        for e in m.catalog.entries.clone() {
            assert!(m.select_id(&e.id));m.prepare_art();
            let page=resolve_page(&p,&e,m.art.as_ref().map(|(_,a)|a));
            let layout=&page.content;
            assert!(page.text_y+layout.height<=734,"{}: actual bottom {}",e.id,page.text_y+layout.height);
            assert!(layout.size>=18,"{}: small body {}",e.id,layout.size);
            let (x,y,w,h)=page.art;
            assert!(x>=248 && y>=120 && x+w<=1258 && y+h<=550,"{}: art stage",e.id);
            let (sx,sy,sw,row)=page.stats;
            assert!(sx>=248 && sx+sw<=1258 && sy+if row {88}else{147}<=734);
            for (i,line) in layout.lines.iter().enumerate() {
                let col=usize::from(page.text_x.len()>1 && i>=layout.split);
                let ly=page.text_y+layout.offsets[i]-if col==1 {layout.offsets[layout.split]}else{0};
                let lx=page.text_x[col];let lw=p.prose_width(&line.text,layout.size,line.heading);
                assert!(lx+lw<=1258,"{}: prose right edge",e.id);
                assert!(ly>=y+h || ly+layout.size as i32<=y || lx>=x+w || lx+lw<=x,"{}: prose/art intersection",e.id);
            }
            if page.text_x.len()>1 && layout.split<layout.lines.len() {
                assert!(layout.lines[layout.split].heading, "{}: column starts mid-section", e.id);
                assert!(!layout.lines[layout.split-1].heading,"{}: orphan heading",e.id);
            }
            for (ex,ey,extra) in &page.extras {
                assert!(extra.size>=18 && ey+extra.height<=734,"{}: extra content bounds",e.id);
                for (i,line) in extra.lines.iter().enumerate() {
                    let ly=ey+extra.offsets[i];
                    let lw=p.prose_width(&line.text,extra.size,line.heading);
                    assert!(ex+lw<=1258,"{}: extra right edge",e.id);
                    assert!(ly>=y+h || ly+extra.size as i32<=y || *ex>=x+w || ex+lw<=x,"{}: extra/art intersection",e.id);
                }
            }
            eprintln!("LAYOUT {} columns={} extras={} size={} height={} art={:?}",e.id,page.text_x.len(),page.extras.len(),layout.size,layout.height,page.art);
        }
    }
    #[test]
    fn transparent_trim_preserves_every_occupied_pixel() {
        let m=Manual::new();
        for e in &m.catalog.entries {
            let art=crate::embedded_assets::creature_picture(crate::embedded_assets::creature(&e.asset).unwrap());
            let pixels:Vec<_>=art.rgba.chunks_exact(4).filter(|p|p[3]>0).map(|p|p.to_vec()).collect();
            let trimmed=trim_transparent_margin(art);
            let actual:Vec<_>=trimmed.rgba.chunks_exact(4).filter(|p|p[3]>0).map(|p|p.to_vec()).collect();
            assert_eq!(pixels,actual,"{}: occupied art changed",e.id);
        }
    }
    #[test]
    fn all_authored_entries_and_art_exist() {
        let m = Manual::new();
        assert_eq!(m.catalog.entries.len(), crate::embedded_assets::CREATURES.len() - 2);
        assert_eq!(m.catalog.depths.len(), 40);
        for e in &m.catalog.entries {
            assert!(
                crate::embedded_assets::creature(&e.asset).is_some(),
                "{}",
                e.id
            );
        }
    }
    #[test]
    fn summoned_duplicates_are_excluded_only_from_bestiary() {
        let mut m = Manual::new(); m.preview_all();
        assert!(m.catalog.companions.is_empty());
        for id in ["half-here", "mini-me"] {
            assert!(!m.select_id(id), "{id} must not be a bestiary entry");
            assert!(crate::embedded_assets::creature(id).is_some(), "{id} art must remain available");
        }
        for entry in &m.catalog.entries {
            assert!(entry.natural_history.as_ref().is_some_and(|history| !history.trim().is_empty()));
            assert_ne!(natural_history(entry), LATIN_PLACEHOLDER);
        }
    }
    #[test]
    fn every_entry_fits_without_discarding_authored_text() {
        let mut m = Manual::new(); m.preview_all();
        let p = Painter::new();
        for e in &m.catalog.entries.clone() {
            assert!(m.select_id(&e.id));m.prepare_art();
            let page=resolve_page(&p,e,m.art.as_ref().map(|(_,a)|a));
            let layout=page.content;
            assert!(
                layout.height <= 610,
                "{}: {}px at {}pt",
                e.id,
                layout.height,
                layout.size
            );
            assert!(layout.size >= 16);
            let body = layout
                .lines
                .iter()
                .chain(page.extras.iter().flat_map(|(_,_,extra)|extra.lines.iter()))
                .filter(|l| !l.heading)
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            assert!(body.contains(&e.description));
            assert!(body.contains(&natural_history(e).split_whitespace().collect::<Vec<_>>().join(" ")), "{}: missing natural history", e.id);
            for form in &e.forms {
                let suffix = form_suffix(&e.description, &form.description);
                assert!(body.contains(suffix), "{}", e.id);
                let prefix = &form.description[..form.description.len()-suffix.len()];
                assert!(e.description.starts_with(prefix));
                assert_eq!(format!("{prefix}{suffix}"), form.description);
            }
            for mechanic in &e.mechanics {
                assert!(body.contains(effect_ranges::page_text(e, mechanic)), "{}", e.id);
            }
            for source in &e.sources {
                assert!(redundant_source(source) || body.contains(&source.text), "{}", e.id);
            }
            let labels = stat_rows(e, 52.).iter().map(|r| r.0).collect::<Vec<_>>();
            assert_eq!(labels.iter().copied().filter(|s| *s != "Energy").collect::<Vec<_>>(),
                ["HP", "Accuracy", "Damage", "Dodge", "DR", "Perception"]);
        }
    }
    #[test]
    fn aspects_and_depth_styles_are_deterministic() {
        assert_eq!(contain(800, 400, 330, 354), (330, 165));
        assert_eq!(contain(400, 800, 330, 354), (177, 354));
        for depth in 1..40 {
            assert_ne!(Theme::at(depth - 1).paper, Theme::at(depth).paper);
        }
        assert_eq!(signature("wolf", "wild"), signature("wolf", "wild"));
    }
    #[test]
    fn trimming_preserves_every_authored_rgba_pixel_including_black() {
        let rgba=vec![0,0,0,0, 0,0,0,255, 5,2,1,127, 0,0,0,0];
        let art=trim_transparent_margin(Picture {w:4,h:1,rgba});
        assert_eq!((art.w,art.h),(2,1));
        assert_eq!(art.rgba,vec![0,0,0,255,5,2,1,127]);
    }
    #[test]
    fn selected_art_consumes_png_alpha_without_runtime_keying() {
        for (id,page) in [("phoenix","phoenix"),("zillennia-timeless","zillennia:timeless"),("vampire","vampire")] {
            let decoded=crate::embedded_assets::creature_picture(crate::embedded_assets::creature(id).unwrap());
            let expected=trim_transparent_margin(decoded);
            let mut manual=Manual::new(); manual.preview_all();
            assert!(manual.select_id(page));
            manual.prepare_art();
            let actual=&manual.art.as_ref().unwrap().1;
            assert_eq!((actual.w,actual.h),(expected.w,expected.h),"{id}");
            assert_eq!(actual.rgba,expected.rgba,"alpha and RGB must not be re-keyed: {id}");
        }
    }
    fn display_values(actor: &corpse_core::Actor, revision: f64) -> [f64; 7] {
        let base = actor.conditions.get("weaponDamage").copied().unwrap_or(10.)
            + actor.stats.offense * actor.conditions.get("weaponOffenseMultiplier").copied().unwrap_or(1.);
        let damage = if revision >= 51. { (base * 0.5).max(0.).round() }
            else { (base * 0.5).ceil().max(0.) };
        [actor.max_hp, actor.stats.max_energy, actor.stats.accuracy, damage,
            actor.stats.dodge, actor.stats.dr, actor.stats.perception]
    }
    fn test_floor(depth: usize) -> corpse_core::Floor {
        corpse_core::Floor { depth: depth as f64, width: 5.0.into(), height: 5.0.into(),
            domain: "Dungeon".into(), tiles: vec!["floor".into(); 25], ..Default::default() }
    }
    fn assert_ranges(entry: &Entry, actors: &[corpse_core::Actor], revision: f64) {
        assert!(!actors.is_empty());
        let values: Vec<_> = actors.iter().map(|a| display_values(a, revision)).collect();
        let rows = stat_rows(entry, revision);
        let energy = actors.iter().any(|a| crate::monster_status_ui::energy_capable(revision, a));
        assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(),
            STAT_LABELS.into_iter().filter(|s| *s != "Energy" || energy).collect::<Vec<_>>());
        for (label, actual) in rows {
            let i = STAT_LABELS.iter().position(|s| *s == label).unwrap();
            let low = values.iter().map(|v| v[i]).fold(f64::INFINITY, f64::min);
            let high = values.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max);
            let expected = if low == high { display_stat_number(low) }
                else { format!("{}–{}", display_stat_number(low), display_stat_number(high)) };
            assert_eq!(actual, expected, "{} revision{revision} {label}", entry.id);
        }
    }
    #[test]
    fn monster_energy_rows_use_public_revisioned_capabilities() {
        let manual=Manual::new();
        for (id, expected) in [("rat",false),("cave-bat",false),("vampire",false),
            ("chimera",false),("hemogoblin-mech",false),("horn-2",false),("bastidion",false),
            ("witch",true),("dryad",true),("bombardier",true),("horn-1",true),("dragon",true)] {
            let entry=manual.catalog.entries.iter().find(|e|e.id==id).unwrap();
            assert_eq!(stat_rows(entry,52.).iter().any(|r|r.0=="Energy"),expected,"{id}");
        }
        let dragon=manual.catalog.entries.iter().find(|e|e.id=="dragon").unwrap();
        assert!(!stat_rows(dragon,51.).iter().any(|r|r.0=="Energy"));
        assert!(stat_rows(dragon,52.).iter().any(|r|r.0=="Energy"));
    }
    #[test]
    fn r52_bestiary_natural_and_legacy_stats_match_population_births() {
        let manual = Manual::new();
        for revision in [29., 33., 42., 50., 51., 52.] {
            for entry in manual.catalog.entries.iter().filter(|e| !e.id.starts_with("zillennia:")
                && !["sphinx", "saidribee-queen"].contains(&e.id.as_str())) {
                let mut actors = Vec::new();
                for depth in birth_depths(entry, revision) {
                    let mut floor = test_floor(depth); let mut next_id = 1.;
                    let mut rng = corpse_core::rng::GenerationRng::new(1);
                    let expected = corpse_core::population::create_population_actor_revision(
                        &mut next_id, &mut floor, &entry.id, 12, false, &mut rng, revision).ok();
                    let actual = birth_actor(entry, depth, revision);
                    assert_eq!(actual.is_some(), expected.is_some(), "{} r{revision}", entry.id);
                    if let (Some(actual), Some(expected)) = (actual, expected) {
                        assert_eq!(actor_stat_values(&actual, revision), display_values(&expected, revision),
                            "{} floor{depth} r{revision}", entry.id);
                        actors.push(expected);
                    }
                }
                if actors.is_empty() { assert!(stat_rows(entry, revision).iter().all(|r| r.1 == "Varies")); }
                else { assert_ranges(entry, &actors, revision); }
            }
        }
        let rat = manual.catalog.entries.iter().find(|e| e.id == "rat").unwrap();
        assert_eq!(stat_rows(rat, 52.)[0].1, "40");
        assert_eq!(stat_rows(rat, 52.).iter().find(|r| r.0 == "Damage").unwrap().1, "14");
        assert_ne!(stat_rows(rat, 52.), stat_rows(rat, 51.));
        assert_eq!(actor_stat_values(&birth_actor(rat, 1, 52.).unwrap(),52.), actor_stat_values(&birth_actor(rat, 3, 52.).unwrap(),52.));
    }
    #[test]
    fn r52_bestiary_fungus_and_zillennia_use_real_depth_templates() {
        let manual = Manual::new();
        for revision in [29., 33., 51., 52.] {
            let fungus = manual.catalog.entries.iter().find(|e| e.id == "bastidion").unwrap();
            let actors: Vec<_> = birth_depths(fungus, revision).iter()
                .map(|&d| {
corpse_core::population::fungus_revision(d as u32, revision).unwrap()
                }).collect();
            assert_ranges(fungus, &actors, revision);
            assert_eq!(stat_rows(fungus, revision)[0].1, if revision == 52. { "154" } else if revision >= 51. { "154–574" } else { "154–622" });
            for entry in manual.catalog.entries.iter().filter(|e| e.id.starts_with("zillennia:")) {
                let actors: Vec<_> = entry.conditional_depths.iter().map(|&depth| {
                    let actor = corpse_core::population::zillennia_revision(depth as u32, revision).unwrap();
                    assert_eq!(entry.id, format!("zillennia:{}", actor.zillennia_quality.as_ref().unwrap()));
                    actor
                }).collect();
                assert_ranges(entry, &actors, revision);
            }
        }
    }
    #[test]
    fn r52_bestiary_fractional_legacy_numbers_are_concise_display_only() {
        assert_eq!(display_stat_number(-50. / 3.), "-17");
        assert_eq!(display_stat_number(1000.), "1000");
        assert_eq!(display_stat_number(1.25), "1");
        assert_eq!(display_stat_number(-0.00001), "0");
        assert_eq!(display_stat_number(3.5), "4");
    }
    #[test]
    fn r52_bestiary_fungus_setup_metadata_matches_its_executable_stencil() {
        let setup = corpse_core::generation::setup_definitions().iter().find(|s| s.id == 24).unwrap();
        assert!(setup.kinds.is_empty());
        let mut floor = corpse_core::Floor { depth: 1., width: 29.0.into(), height: 29.0.into(),
            tiles: vec!["floor".into(); 29*29], ..Default::default() };
        corpse_core::generation::stamp_setup(&mut floor, setup.id, 14, 14, &setup.kinds).unwrap();
        assert!(floor.setup_spawns.as_ref().unwrap().iter().any(|s| corpse_core::fungus_feasts::is_fungus(&s.kind)));
        assert_eq!(fungus_setup_depths(52.), (1..=40).collect::<Vec<_>>());
    }
    #[test]
    fn concise_bestiary_preserves_signature_threats_and_shared_labels() {
        let mut manual = Manual::new();
        let page = |manual: &Manual, id: &str, ability: &str| {
            let e = manual.catalog.entries.iter().find(|e| e.id == id).unwrap();
            let m = e.mechanics.iter().find(|m| m.name == ability).unwrap();
            effect_ranges::page_text(e, m).to_owned()
        };
        assert_eq!(page(&manual, "cave-bat", "Fast"), "");
        assert!(page(&manual, "pale:Greater Breather", "Death Gas Cloud").contains("about 6.7% max HP"));
        manual.set_death_gas(false);
        assert!(page(&manual, "pale:Greater Breather", "Hallucinogenic Gas Cloud").contains("Hallucinating"));
        manual.set_death_gas(true);
        assert!(page(&manual, "bombardier", "Reservoir Explosion").contains("50% max HP"));
        assert!(page(&manual, "banshee", "Banshee Shriek").contains("50% current HP + 100"));
        assert!(page(&manual, "samara", "Razorleaves").contains("2% max HP"));
        assert!(page(&manual, "zillennia:transcendent", "Ground Erasure").contains("loose items"));
        for e in manual.catalog.entries.iter().filter(|e|e.kind == "zillennia") {
            assert!(page(&manual, &e.id, "Ruin Pulse").contains("within 3 tiles"));
            assert!(page(&manual, &e.id, "Mouth of Silence").contains("within 3 tiles"));
        }
        for (id, ability, damage) in [("needleling","Weak Venom",1),("long-saidri","Venom",2),("saidri-guardian","Guardian Venom",4)] {
            assert!(page(&manual, id, ability).contains(&format!("+{damage} damage per turn")));
        }
    }
    #[test]
    fn concise_bestiary_native_examples() {
        let Ok(directory) = std::env::var("CORPSE_BESTIARY_COPY_REVIEW") else { return; };
        std::fs::create_dir_all(&directory).unwrap();
        let mut manual = Manual::new(); manual.preview_all();
        let mut p = Painter::new();
        p.try_resize(1280, 800).unwrap();
        for id in ["witch", "bombardier", "pale:Greater Breather", "zillennia:transcendent", "banshee", "valta"] {
            assert!(manual.select_id(id));
            manual.paint(&mut p);
            let file = std::fs::File::create(format!("{directory}/bestiary-{}.png", id.replace(':', "-"))).unwrap();
            let mut encoder = png::Encoder::new(file, p.width as u32, p.height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
    #[test]
    fn r52_bestiary_ranges_ignore_stale_manual_floors_and_fungus_requires_both_feasts() {
        let manual = Manual::new();
        for id in ["rat", "bastidion", "zillennia:child", "saidribee-queen"] {
            let mut entry = manual.catalog.entries.iter().find(|e| e.id == id).unwrap().clone();
            let rows = stat_rows(&entry, 52.); let depths = birth_depths(&entry, 52.);
            entry.ordinary_depths = vec![40]; entry.conditional_depths.clear();
            assert_eq!(stat_rows(&entry, 52.), rows, "{id}"); assert_eq!(birth_depths(&entry, 52.), depths);
        }
        let fungus = manual.catalog.entries.iter().find(|e| e.id == "bastidion").unwrap();
        for revision in [51., 52.] {
            let depths = birth_depths(fungus, revision);
            assert_eq!(depths, (1..=36).collect::<Vec<_>>());
            for depth in 1..=40 {
                let pools = corpse_core::fungus_feasts::victim_pools(depth, revision).unwrap();
                assert_eq!(depths.contains(&(depth as usize)), pools.iter().all(|p| !p.is_empty()));
                assert!(pools.iter().flatten().all(|k| !k.starts_with("zillennia") && k != "sphinx"));
            }
        }
    }
    #[test]
    fn r52_bestiary_context_only_entries_match_their_actual_public_birth_path() {
        let manual = Manual::new();
        let queen = manual.catalog.entries.iter().find(|e| e.id == "saidribee-queen").unwrap();
        for revision in [33., 42., 50., 51., 52.] {
            let mut queens = Vec::new();
            for &depth in &queen.conditional_depths {
                let mut floor = test_floor(depth);
                floor.pocket_biomes = vec![corpse_core::pocket_biomes::PocketBiome {
                    theme: "saidribee-nest".into(), queen_cell: Some(12), honey_cell: Some(13), ..Default::default()
                }].into();
                let mut actors = Vec::new(); let mut ground = Vec::new(); let mut next_id = 1.;
                corpse_core::pocket_biomes::populate_revision(&mut floor, &mut actors, &mut ground, &mut next_id, revision).unwrap();
                assert_eq!(actors.len(), 1);
                assert_eq!(actor_stat_values(&birth_actor(queen, depth, revision).unwrap(), revision), display_values(&actors[0], revision));
                queens.push(actors.remove(0));
            }
            assert_ranges(queen, &queens, revision);
        }
        assert_eq!(stat_rows(queen, 52.)[0].1, "430");
        assert!(stat_rows(queen, 29.).iter().all(|r| r.1 == "Varies"));
        let sphinx = manual.catalog.entries.iter().find(|e| e.id == "sphinx").unwrap();
        let guardian = corpse_core::population::create_actor("sphinx", 0., 0., 0., false).unwrap();
        for revision in [29., 33., 42., 50., 51., 52.] {
            let mut expected=guardian.clone();
            if revision >= 52. { expected.stats.accuracy += 15.; expected.stats.dodge = (10. + expected.stats.dodge).max(10.); }
            assert_ranges(sphinx, &[expected], revision);
        }
        // Do not invent a profile for a character-dependent or unsupported page.
        let mut variable = queen.clone(); variable.id = "character-dependent".into();
        variable.stats = None; variable.health = None;
        assert!(stat_rows(&variable, 52.).iter().all(|r| r.1 == "Varies"));
        variable.conditional_depths.clear();
        assert!(stat_rows(&variable, 52.).iter().all(|r| r.1 == "Varies"));
    }
    #[test]
    fn r52_bestiary_natural_and_context_births_share_species_profiles() {
        let manual = Manual::new();
        for (kind, context) in [("short-saidri", "queen:short-saidri"),
            ("saidri-guardian", "nest:saidri-guardian"), ("pale:Jackal", "birther:pale:Jackal"),
            ("night-crawler", "witch:night-crawler")] {
            let entry = manual.catalog.entries.iter().find(|e| e.id == kind).unwrap();
            let profiles: Vec<_> = corpse_core::balance52::profiles().iter().filter(|r| r.kind == kind).collect();
            let actors: Vec<_> = profiles.iter().map(|r| birth_actor(entry, r.floor as usize, 52.).unwrap()).collect();
            assert_ranges(entry, &actors, 52.);
            let contextual = corpse_core::balance52::profile(context, 30.).unwrap();
            let natural = birth_actor(entry, 30, 52.).unwrap();
            assert_eq!((natural.stats.offense, natural.max_hp, natural.stats.accuracy),
                (contextual.offense, 100. + 6. * contextual.defense, contextual.accuracy + 15.));
        }
    }
    #[test]
    fn r52_bestiary_ichor_stats_match_event_births() {
        let manual = Manual::new();
        for revision in [29., 33., 42., 51., 52.] {
            for depth in 4..=40 {
                let kinds = corpse_core::ichor::eligible_fish_kinds(depth, revision);
                for (i, kind) in kinds.iter().enumerate() {
                    let entry = manual.catalog.entries.iter().find(|e| e.id == *kind).unwrap();
                    let fish = corpse_core::ichor::make_fish_revision(0., 0., 0., depth as f64,
                        (i as f64 + 0.5) / kinds.len() as f64, revision);
                    assert_eq!(fish.creature_kind.as_ref().unwrap(), kind);
                    assert_eq!(actor_stat_values(&birth_actor(entry, depth as usize, revision).unwrap(), revision),
                        display_values(&fish, revision), "{kind} r{revision}");
                }
            }
        }
    }
    #[test]
    fn r52_bestiary_revision_changes_preserve_discovery_and_no_active_defaults_current() {
        let mut manual = Manual::new(); manual.progress.discover(["rat"]);
        assert!(manual.select_id("rat"));
        let selected = manual.selected; let color_depth = manual.depth; let ids = manual.ids().to_vec();
        assert_eq!(manual.simulation_revision, 52.);
        let current = stat_rows(manual.entry().unwrap(), manual.simulation_revision);
        manual.set_simulation_revision(Some(29.));
        assert_ne!(stat_rows(manual.entry().unwrap(), manual.simulation_revision), current);
        manual.set_simulation_revision(None);
        assert_eq!(stat_rows(manual.entry().unwrap(), manual.simulation_revision), current);
        assert!(manual.progress.seen("rat") && manual.progress.unread("rat"));
        assert_eq!((manual.selected, manual.depth), (selected, color_depth)); assert_eq!(manual.ids(), ids);
    }
    #[test]
    fn r52_bestiary_rebalanced_spell_copy_has_no_stale_fixed_coefficients() {
        let manual = Manual::new(); let mut checked = 0;
        for entry in &manual.catalog.entries {
            if corpse_core::balance52::profiles().iter().any(|r| r.kind == entry.id
                && (r.heal_amount.is_some() || r.special_damage.is_some())) {
                // Static entries may show geometric limits and timing, but the
                // damage/heal packet varies by spawn profile rather than the reader's HP.
                let name = if entry.id == "dryad" { "Healing Grove" } else { "Moonbeam" };
                let mechanic = entry.mechanics.iter().find(|m| m.name == name).unwrap();
                let copy = effect_ranges::page_text(entry, mechanic);
                let range = if entry.id == "dryad" { "6 tiles" } else { "16-tile" };
                assert!(copy.contains(range), "{}: {copy}", entry.id);
                for packet in corpse_core::balance52::profiles().iter()
                    .filter(|r| r.kind == entry.id)
                    .filter_map(|r| r.heal_amount.or(r.special_damage))
                {
                    assert!(!copy.contains(&format!("{packet} HP")), "{}: {copy}", entry.id);
                    assert!(!copy.contains(&format!("{packet} damage")), "{}: {copy}", entry.id);
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 2); // Dryad healing and Valta moonbeam; costs remain authored.
    }
    #[test]
    fn r52_bestiary_range_labels_fit_every_authored_page_and_capture_aspects() {
        let mut manual = Manual::new(); manual.preview_all(); let mut p = Painter::new();
        for id in manual.ids().to_vec() {
            manual.select_id(&id); manual.prepare_art(); let entry = manual.entry().unwrap();
            let page = resolve_page(&p, entry, manual.art.as_ref().map(|(_, a)| a));
            let cell = page.stats.2 / if page.stats.3 {8} else {4};
            for revision in [29., 33., 42., 50., 51., 52.] {
                for (_, value) in stat_rows(entry, revision) {
                    let size = stat_value_size(&p, &value, cell - 6);
                    assert!(p.prose_width(&value, size, true) <= cell - 6, "{id}: {value}");
                }
            }
        }
        let directory = std::env::var("CORPSE_BESTIARY_REVIEW").ok();
        if let Some(directory) = &directory { std::fs::create_dir_all(directory).unwrap(); }
        for (aspect, width, height) in [("wide",1920,1080), ("portrait",900,1200)] {
            p.try_resize(width,height).unwrap(); p.set_responsive_canvas(false);
            for (id, revision) in [("rat",52.), ("dragon",52.), ("bastidion",52.), ("saidribee-queen",52.), ("rat",51.)] {
                manual.select_id(id); manual.set_simulation_revision(Some(revision)); manual.paint(&mut p);
                if let Some(directory) = &directory {
                    let file = std::fs::File::create(format!("{directory}/bestiary-stats-{aspect}-{id}-r{revision}.png")).unwrap();
                    let mut encoder = png::Encoder::new(file,p.width as u32,p.height as u32);
                    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
                }
            }
        }
    }
    #[test]
    fn concise_labels_keep_authored_effects_and_unique_sources() {
        let manual = Manual::new();
        for entry in &manual.catalog.entries {
            for mechanic in &entry.mechanics {
                let heading = mechanic_heading(mechanic);
                if heading != mechanic.name {
                    assert!(matches!(heading, "Fast" | "Slow"));
                    assert!(mechanic.name.starts_with(&format!("{heading} (")));
                    assert!(!mechanic.text.is_empty());
                }
            }
            for source in &entry.sources {
                if source.kind != "ordinary" && source.kind != "terrain" {
                    assert!(!redundant_source(source));
                }
            }
        }
        assert!(!redundant_source(&Source {
            kind: "ordinary".into(),
            text: "May appear in groups in large, connected pools.".into(),
        }));
    }
    #[test]
    fn mouse_controls_cover_pages_and_creature_navigation() {
        let mut m = Manual::new(); m.preview_all();
        m.click(195, 90); assert_eq!(m.selected, 20);
        m.click(167, 90); assert_eq!(m.selected, 0);
        m.click(130, 720); assert_eq!(m.selected, 1);
        m.click(30, 720); assert_eq!(m.selected, 0);
        m.click(25, 125); assert_eq!(m.selected, 0);
    }
    #[test]
    fn roster_is_unique_and_sorted_by_first_appearance() {
        let m=Manual::new();
        assert_eq!(m.ids().len(),m.catalog.entries.len());
        assert_eq!(m.ids().iter().collect::<std::collections::HashSet<_>>().len(),m.catalog.entries.len());
        let keys:Vec<_>=m.ids().iter().map(|id| {
            let e=m.catalog.entries.iter().find(|e| &e.id==id).unwrap();
            (first_depth(e).unwrap_or(usize::MAX),e.name.to_lowercase(),e.id.clone())
        }).collect();
        assert!(keys.windows(2).all(|w| w[0]<=w[1]));
        assert!(keys.iter().all(|k|k.0!=usize::MAX));
        assert_eq!(m.ids().last().map(String::as_str),Some("dragon"));
    }
    #[test]
    fn all_bestiary_entries_preserve_owner_supplied_natural_history() {
        let m=Manual::new();
        for e in &m.catalog.entries {
            if e.id=="saidribee-queen" {
                assert_eq!(natural_history(e),corpse_core::new_creatures::QUEEN_HISTORY);
                continue; // Approved factual reference copy, not a verbatim owner quote.
            }
            let expected = match e.id.as_str() {
                "bastidion" => "A common fungus in the Flesh. It spreads roots to corpses, which are in no short supply. Though death has perished, a Corpse Fungus's roots can still paralyze a corpse, until the roots are slashed or burned away.",
                "blorb" => "These small creatures eat refuse and feces from larger animals. Their slime helps to break it down, and though they're known to be hostile, they aren't known to be particularly dangerous.",
                "cave-bat" => "Small flying mammals which populate the high-caves of the Flesh. They suckle from bleeding walls, and from dead creatures.",
                "rat" => "An ordinary rat, but grown fat on magickal blood. The essence of gods forces them to spring up faster.",
                "hemogoblin" => "Ordinary goblins, corrupted by the Flesh. Their skin has melted away, leaving only blood which they track across the floor.",
                "needleling" => "Creatures created by the spawn of Kative to spread disease. Despite their appearance, they are almost purely mechanical, their biological components being just a facade, with the exception of their heart.",
                "pale:Biter" => "The most aggressive of the ancient pales. Despite their large mouths, pales do not starve, and are known to survive centuries without food, water or oxygen.",
                "pale:Jackal" => "The only pale which can die of age. They are birthed as a defense mechanism of other pales, but within 10 years are often driven irrational by death psychosis.",
                "shellback-beetle" => "A huge beetle, most notable for its near impenetrable shell, and slow, lumbering pace. It eats foliage in the shallowest depths of the Flesh.",
                "short-saidri" => "The floating worker of a Saidribee colony. They pollinate and hunt throughout the Flesh, to bring down to their hive queen deeper underneath.",
                "dryad" => "Descended from the God-Trees, these creatures serve to heal and assist those creatures native to the Flesh, and are hostile to those outside.",
                "hemogoblin-bloodmancer" => "A goblin magus, rendered of its flesh. Having been born of Hemogoblins in the Flesh, this creature was born with magickal abilities.",
                "pale:Breather" => "A disgusting creature of pale origin. It breaths out a gas which similar to vomit and death as a way to defend its kin.",
                "ichor-fish-standard" | "lesser-ichor-fish" | "greater-ichor-fish" => "A piranha made of gruesome god-metal. It is a creature alive only to serve its godbeast master, and for no other reason.",
                "zillennia:child" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This one is smaller than average, but still of incredible eldritch power.",
                "zillennia:infant" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This one, dangerous as it is, is of diminutive size.",
                "pale:Screamer" => "A creature of the pale species, most noticeable for its large mouth and piercing scream, which emboldens its fellows.",
                "pale:Spitter" => "An ancient low beast. Its phallic appendage can spew acid and gas to subdue its enemies.",
                "pale:Birther" => "A hideous low beast, which spawns other creatures from its vagina like head. Its large midsection deflates as it births its spawn.",
                "giant-beetle" => "A predatory beetle the size of a horse, with giant wings. It hunts humanoids which wander into the Flesh.",
                "glim" => "A creature made of adhesive, flammable slime. It glides through the slime it places on the ground, unable to move otherwise.",
                "bear" => "A mammalian beast which roams the Flesh, eating living creatures and carcasses alike.",
                "witch" => "A former human, long since corrupted by the power of the Flesh, and defiled by eldritch creatures. Magick now flows through her blood, allowing her to bring forth great power, and birth terrible creatures.",
                "samara" => "A tree with razor-sharp twirling seeds. Their living areas are often vacated due to the natural hazard they create.",
                "zillennia:lesser" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This creature is large as an elephant, and yet is still small for its species.",
                "horn-1" => "A beautiful creature from another reality which delights in mischief and death.",
                "horn-2" => "A two-horned creature from another reality which wishes to gore all living things.",
                "horn-4" => "A four-horned creature from another reality which wishes to gore all living things.",
                "horn-6" => "A six-horned creature from another reality which wishes to gore all living things.",
                "lesser-ichor-shark" | "ichor-shark" | "greater-ichor-shark" => "A huge, terrifying fish with rows of sharp, dagger like teeth. Its body is made from the same toxic god-liquid in which it swims.",
                "sphinx" => "A keeper of secrets and riddles. Sphinxes stand as timeless sentinels over their treasure, and only through guile can they be plundered.",
                "hemogoblin-mech" => "The result of true ingenuity attached to malevolent power and purpose.",
                "hemogoblin-inventor" => "A goblin, rended of its flesh, but not its inventive habits. It creates mechanisms of malice with which to end human life.",
                "tiger" => "A feline-predator, which stalks its prey quietly.",
                "long-saidri" => "A roaming guardian of Saidribee colonies. Its legs support a larger body, which allow for more venom and muscle.",
                "bombardier" => "A beetle that explodes when startled. It is unclear whether this adaptation is biological, or corruptive in nature.",
                "zillennia:unqualified" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. As large as a house, and as all consuming as a black hole.",
                "kelpie" => "A carnivorous horse made of seaweed and vile magick. Kelpie hiding places are only spotted through close observation of the murky red water that follows their feeding.",
                "wendigo" => "A human who starved to death, ressurected by the blood of the Flesh. Now he hungers endlessly.",
                "pale:Greater Biter" => "A grown pale. Large and hungry, it shares the maw of a crocodile, and an unmatched malice.",
                "pale:Greater Spitter" => "A long, spindly pale, like an imitation of a human, but with a long phallic trunk used to spew terrible projectiles.",
                "zillennia:mature" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This creature is large, larger than the space it takes up. You see it only through the inability to see around it, as it consumes everything that comes into contact, be it sound, light or flesh.",
                "troll" => "A humanoid grown from the blood walls of the Flesh. Its hands are the size of small men, and its mouth full of teeth with which to devour the uncorrupted.",
                "pale:Greater Breather" => "A lowbeast biped which emits the smell of death. That is to say, those that smell the stench of the Greater Gasser have the death of their loved ones brought to the forefront of their mind until the hallucinations become too intense, and the soul evaporates.",
                "pale:Greater Screamer" => "A colossal maw allows the scream of this creature to echo around, disorienting its pray. Its colossal vocal chords are as thick as steel cables.",
                "pale:Greater Birther" => "A large biped which brings forth other lowbeasts. It deflates as it births its spawn, and replenishes itself only with the blood of the Flesh.",
                "dire-bear" => "A huge bear, large enough to fit a human torso in its mouth with room to spare. It roams the Flesh, eating monsters and people alike.",
                "zillennia:greater" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This one is gargantuan, even larger than average, and consumes all it touches without discrimination.",
                "river-drake" => "A reptilian which conquers the ecosystems of lakes and rivers.",
                "horn-3" => "A beautiful, three-horned horse of magickal power from another dimension. Its playful nature often results in the death of its victims.",
                "killspawn" => "A demon made of metal, spawned fully formed from the God-Obelisks: The Borge. These creatures delight in death, and naught else.",
                "zillennia:hulking" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This one is of gargantuan, almost incomprehensible size. To kill it, almost unfathomable.",
                "lesser-ichor-leviathan" => "An incredibly large, whale like predator created by Mercuria the Machine-Godbeast, all to patrol the endless depths of Ichor.",
                "sabretooth" => "A pre-historic tiger, revived to defend the Flesh. Its bite is powerful, and its fangs long.",
                "banshee" => "Women under duress who flee to the Flesh for comfort and salvation have their body stripped from them, and replaced with a construct. Their screams of anguish can be fatal to those mortals who hear.",
                "vampire" => "Human nobles who perform terrible sacrifices can curry favor with the Flesh, earning themselves a terrible reward. Leech-vampirism changes their mind, just as well as their body, leaving them as one of the most dangerous monsters of the Flesh.",
                "maulchild" => "A demon of innocent persuasion. It comes with others, and craves affection, holding its victim still for its fellows' carving knives.",
                "horn-5" => "A beautiful horse with five horns from another dimension. It carries with it a malevolent aura, and a desire to inflict suffering.",
                "zillennia:ancient" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. Born early in that millenium of godly intercourse, this creature is older than  humanity. It carries with it no desires, or fears. Just an almost unending sea of nothingness.",
                "zillennia:timeless" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. This one does not change. It was created when those two gods locked eyes with each other, and their horrible romance began, before humanity had yet breathed. It cares not for the whims of time. It is likely it is unaware that its parents lie long dead, and that it is a relic of a different era.",
                "zillennia:transcendent" => "Grisha and Dawel, in their final, death-like embrace, brought forth a creature known as a Zillennia, which is made of nothing and desires nothingness abound. Many spawned, large and small. It came into existence before its birth. This creature has never not existed, and yet it is composed of nothing, and so has never existed either. It shall eventually consume all it comes into contact with, as it wanders across the Flesh.",
                "ichor-leviathan" => "An incredibly large, whale like predator created by Mercuria the Machine-Godbeast, all to patrol the endless depths of Ichor.",
                "phoenix" => "A beautiful and noble creature, long-since corrupted by the Flesh and used as a terrible monster to protect its adminstrator.",
                "valta" => "A moonhaunted of Anavani, one of her companions on the moon. Created for its beauty and its fierce protectiveness.",
                "moonspawn" => "A child of Anavani, created to patrol her final resting place upon the moon. Now it dwells down here, this place undeserving of its beauty.",
                "yeti" => "A creature from the high mountains, now brought low, its habitat mirrored in the places it appears.",
                "saidri-guardian" => "A guardian of the Saidribee Nest Queen. Fiercely protective, and incredibly dangerous.",
                "cancer" => "A monster of myth. Woven into the stars, this crab has the strength to crush almost anything.",
                "greater-ichor-leviathan" => "An incredibly large, whale like predator created by Mercuria the Machine-Godbeast, all to patrol the endless depths of Ichor.",
                "chimera" => "A corrupted abomination. Multiple animals, mutated into one despicable creature.",
                "horn-7" => "The most powerful of the extra-dimensional corn monsters. Beautiful and dangerous, this creature kills with its thoughts.",
                "ursa-major" => "Anavani's bear, plucked from his constellation to protect her along the moon. How this fantastic creature finds itself here, no one knows.",
                "ursa-minor" => "She prowls to feed her mate after having been plucked down from her eternal watch in the sky.",
                "minotaur" => "A creature of the Labyrinth. Now prowls the Flesh, recklessly charging forward at intruders.",
                "dragon" => "The most dangerous of reptiles, a legendary predator found only in the depths of most magickal caves. And of course, in the Flesh, from whence they are spawned.",
                "night-crawler" => corpse_core::new_creatures::NIGHT_HISTORY,
                "skin-walker" => corpse_core::new_creatures::SKIN_HISTORY,
                _ => panic!("Unexpected bestiary entry without authored history: {}", e.id),
            };
            assert_eq!(natural_history(e), expected, "{}", e.id);
        }
        let mut e=m.catalog.entries[0].clone();e.natural_history=Some("Owner-authored history.".into());
        assert_eq!(natural_history(&e),"Owner-authored history.");
    }
    #[test]
    fn setting_name_is_capitalized_and_bold_even_across_wrapped_lines() {
        let p = Painter::new();
        for size in 16..=20 {
            // The bundled regular and ExtraBold monospace faces have identical advances.
            for c in "the Flesh".chars() {
                assert_eq!(p.prose_width(&c.to_string(), size, false), p.prose_width(&c.to_string(), size, true));
            }
            let lines = body_lines(&p, "its flesh, THE flesh. the flesh! other flesh", 60, size);
            let bold = lines.iter().flat_map(|line| line.emphasis.iter().map(|r| line.text[r.clone()].to_owned())).collect::<Vec<_>>().join(" ");
            assert_eq!(bold, "THE Flesh the Flesh");
            assert!(lines.iter().all(|line| p.prose_width(&line.text, size, false) <= 60));
        }
        let m = Manual::new();
        for e in &m.catalog.entries {
            let layout = fit_content(&p, e);
            for line in &layout.lines {
                for range in &line.emphasis {
                    assert!(!line.text[range.clone()].contains("flesh"));
                }
            }
        }
        assert!(setting_ranges("other flesh; its flesh; the fleshy skin").is_empty());
    }
    #[test]
    fn setting_bold_changes_actual_pixels_without_changing_layout() {
        let mut regular = Painter::new();
        let mut bold = Painter::new();
        let line = body_lines(&bold, "the Flesh", 288, 20).remove(0);
        regular.prose(&line.text, 20, 20, 20, [255,255,255], false);
        paint_content_line(&mut bold, &line, 20, 20, 20, [255,255,255]);
        assert_ne!(regular.pixels, bold.pixels);
    }
    #[test]
    fn background_darkens_monotonically_with_readable_text() {
        assert_eq!(Theme::at(0).paper,[239,232,204]);
        assert_eq!(Theme::at(24).paper,[143,28,32]);
        assert_eq!(Theme::at(39).paper,[0,0,0]);
        let mut previous=1.;
        for d in 0..40 {
            let t=Theme::at(d);let l=relative_luminance(t.paper);assert!(l<previous);previous=l;
            assert!(theme_contrast(t.ink,t.paper)>=4.5, "depth {}: ink contrast",d+1);
        }
    }
    #[test]
    fn fractional_visual_floors_interpolate_without_tier_switches() {
        for d in [0.5f64, 4.5, 12.5, 23.5, 24.5, 32.5, 38.5] {
            let low = Theme::at_depth(d.floor()).paper;
            let high = Theme::at_depth(d.ceil()).paper;
            let middle = Theme::at_depth(d).paper;
            for channel in 0..3 {
                assert!(middle[channel] >= high[channel] && middle[channel] <= low[channel]);
                assert!((middle[channel] as i16 * 2 - low[channel] as i16 - high[channel] as i16).abs() <= 1);
            }
            assert_ne!(middle,low); assert_ne!(middle,high);
        }
        let before=Theme::at_depth(24.-0.0001).paper;
        let after=Theme::at_depth(24.+0.0001).paper;
        assert_eq!(before,after,"tier anchor must not produce a discontinuity");
        assert_eq!(Theme::at_depth(-2.).paper,Theme::at(0).paper);
        assert_eq!(Theme::at_depth(99.).paper,Theme::at(39).paper);
    }
    #[test]
    fn first_appearance_controls_solid_page_color_not_combat_reference() {
        let mut m=Manual::new(); m.preview_all(); let roster=m.ids().to_vec();
        let mut p=Painter::new();
        for id in ["hemogoblin", "bear", "dragon"] {
            assert!(m.select_id(id));
            let first=m.depth;
            m.paint_at(&mut p,0.);
            for y in [100,500,730] {
                let at=(y*p.width+1278)*4;
                assert_eq!(&p.pixels[at..at+3],&Theme::at(first).paper,
                    "{id}: page must remain spatially solid");
            }
            assert_eq!(m.depth,first);
            assert_eq!(m.ids(),roster);
        }
    }
    #[test]
    fn whole_roster_visual_palette_never_backtracks() {
        let mut m=Manual::new(); m.preview_all(); let ids=m.ids().to_vec();
        // Corpse Fungus is alphabetized under C; Blorb is now the first depth-1 entry.
        assert_eq!(ids.first().map(String::as_str),Some("blorb"));
        let mut previous=1.;
        for id in ids {
            assert!(m.select_id(&id));
            let current=relative_luminance(Theme::at(m.depth).paper);
            assert!(current<=previous,"{id}: later page must not lighten");
            previous=current;
        }
        assert!(m.select_id("bastidion"));
        assert_eq!(Theme::at(m.depth).paper,[239,232,204]);
    }
    #[test]
    fn export_visual_background_review() {
        let Ok(directory)=std::env::var("CORPSE_FLESH_REVIEW") else {return;};
        std::fs::create_dir_all(&directory).unwrap();
        let mut m=Manual::new(); m.preview_all(); let mut p=Painter::new();
        for id in ["bastidion", "hemogoblin", "bear", "wendigo", "dragon"] {
            assert!(m.select_id(id)); m.paint_at(&mut p,0.);
            let file=std::fs::File::create(format!("{directory}/bestiary-visual-{id}.png")).unwrap();
            let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
            encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
    #[test]
    fn export_solid_visual_palette_map() {
        let Ok(directory)=std::env::var("CORPSE_FLESH_REVIEW") else {return;};
        std::fs::create_dir_all(&directory).unwrap();
        let mut p=Painter::new(); p.clear([20,17,22]);
        p.prose("SOLID PAGE COLOR BY VISUAL FLOOR",20,14,26,[239,232,204],true);
        p.prose("Colors blend between floor anchors; each bestiary page itself stays solid.",20,48,18,[239,232,204],false);
        for floor in 1..=40 {
            let theme=Theme::at(floor-1);
            let x=20+((floor-1)%8) as i32*157;
            let y=94+((floor-1)/8) as i32*134;
            p.rect(x,y,145,120,theme.paper);
            p.prose(&format!("FLOOR {floor:02}"),x+10,y+45,20,theme.ink,true);
        }
        let file=std::fs::File::create(format!("{directory}/bestiary-palette-map.png")).unwrap();
        let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
    #[test]
    fn depth_windows() {
        assert_eq!(ranges(&[1, 2, 3, 5, 7, 8]), "1-3, 5, 7-8");
    }
}

#[cfg(test)]
mod discovery_privacy_tests {
    use super::*;
    #[test]
    fn locked_page_pixels_do_not_depend_on_secret_content() {
        let mut manual=Manual::new();let mut painter=Painter::new();
        manual.paint_at_depth(&mut painter,0.,0);let original=painter.pixels.clone();
        for entry in &mut manual.catalog.entries {entry.name="SECRET NAME".into();entry.description="SECRET LORE".into();entry.asset="missing-secret-art".into();entry.family="SECRET FAMILY".into();entry.health=Some(999999.);entry.natural_history=Some("SECRET HISTORY".into());}
        manual.paint_at_depth(&mut painter,0.,0);assert_eq!(painter.pixels,original);assert!(manual.art.is_none());
    }
    #[test]
    fn r52_bestiary_revision_and_stale_stat_changes_cannot_reveal_locked_pages() {
        let mut manual = Manual::new(); let mut painter = Painter::new();
        manual.select_id("rat"); manual.paint(&mut painter); let locked = painter.pixels.clone();
        for revision in [29., 33., 51., 52.] {
            manual.set_simulation_revision(Some(revision)); manual.paint(&mut painter);
            assert_eq!(painter.pixels, locked); assert!(manual.art.is_none());
        }
        let rat = manual.catalog.entries.iter_mut().find(|e| e.id == "rat").unwrap();
        rat.health = Some(999999.); rat.stats.as_mut().unwrap().insert("offense".into(),999999.);
        manual.paint(&mut painter); assert_eq!(painter.pixels,locked);
        assert!(!manual.progress.seen("rat") && !manual.progress.has_unread());
    }
    #[test]
    fn r52_bestiary_revision_and_render_preserve_persisted_discovery_bytes() {
        let path = std::env::temp_dir().join(format!("r52-bestiary-stat-record-{}.json", std::process::id()));
        let mut manual = Manual::new();
        manual.progress = crate::bestiary_progress::Progress::load(path.clone());
        manual.progress.discover(["rat", "dragon"]); manual.select_id("rat");
        let bytes = std::fs::read(&path).unwrap(); let mut painter = Painter::new();
        for revision in [29., 51., 52.] {
            manual.set_simulation_revision(Some(revision)); manual.paint(&mut painter);
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        let reloaded = crate::bestiary_progress::Progress::load(path.clone());
        assert!(reloaded.seen("rat") && reloaded.unread("rat") && reloaded.seen("dragon") && reloaded.unread("dragon"));
        manual.mark_viewed();
        let reloaded = crate::bestiary_progress::Progress::load(path.clone());
        assert!(reloaded.seen("rat") && !reloaded.unread("rat") && reloaded.unread("dragon"));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn known_page_changes_pixels_but_unread_marker_does_not() {
        let mut manual=Manual::new();let id=manual.ids()[0].clone();let mut painter=Painter::new();manual.paint(&mut painter);let locked=painter.pixels.clone();
        manual.progress.discover([id.as_str()]);manual.paint(&mut painter);assert_ne!(painter.pixels,locked);assert!(manual.progress.unread(&id));
        let known=painter.pixels.clone();
        manual.mark_viewed();assert!(!manual.progress.unread(&id));assert!(manual.progress.seen(&id));
        manual.paint(&mut painter);assert_eq!(painter.pixels,known);
    }
    #[test]
    fn discovery_transition_wide_and_portrait_capture() {
        let directory=std::env::var("CORPSE_BESTIARY_REVIEW").ok();
        if let Some(directory)=&directory {std::fs::create_dir_all(directory).unwrap();}
        for (label,width,height) in [("wide",1920,1080),("portrait",900,1200)] {
            let mut manual=Manual::new();assert!(manual.select_id("blorb"));
            let mut painter=Painter::new();painter.try_resize(width,height).unwrap();painter.set_responsive_canvas(false);
            let mut previous=None;
            for state in ["unknown","unread","read"] {
                if state=="unread" {manual.progress.discover(["blorb"]);}
                if state=="read" {manual.mark_viewed();}
                manual.paint(&mut painter);
                if let Some(prior)=previous {
                    if state=="read" {assert_eq!(painter.pixels,prior,"{label}: clearing unread must not change the page");}
                    else {assert_ne!(painter.pixels,prior,"{label}: discovering a creature must reveal its page");}
                }
                previous=Some(painter.pixels.clone());
                assert_eq!(manual.progress.unread("blorb"),state=="unread");
                if state=="unknown" {assert!(manual.art.is_none());}
                if let Some(directory)=&directory {
                    let file=std::fs::File::create(format!("{directory}/bestiary-{label}-{state}.png")).unwrap();
                    let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);
                    encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
                }
            }
        }
    }

}

fn theme_contrast(a:[u8;3],b:[u8;3])->f64 {
    let (a,b)=(relative_luminance(a),relative_luminance(b));
    (a.max(b)+0.05)/(a.min(b)+0.05)
}
#[cfg(test)]
#[path="bestiary_theme_tests.rs"]
mod bestiary_theme_tests;

#[cfg(test)]
mod compressed_depth_tests {
    #[test]
    fn bestiary_ranges_only_show_the_twenty_physical_floors() {
        assert_eq!(super::compressed_ranges(&(1..=40).collect::<Vec<_>>()),(1..=20).collect::<Vec<_>>());
        assert_eq!(super::compressed_ranges(&[5,10,15,20,25,30,35,40]),vec![3,5,8,10,13,15,18,20]);
        assert!(super::compressed_ranges(&[2,4,9,19,39]).is_empty());
    }
}

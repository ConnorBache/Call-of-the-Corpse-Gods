//! Observation-only painter. Optional GPU recording emits native quads, while
//! the unchanged software path remains the reference and headless exporter.
use std::collections::HashMap;
#[path = "resource_ui.rs"]
pub mod resource_ui;
#[path="ability_indicator_ui.rs"]
mod ability_indicator_ui;
#[path="ability_dialog.rs"]
mod ability_dialog;
#[path="pack_occlusion.rs"]
mod pack_occlusion;
#[path="chronicle_fade_cache.rs"]
mod chronicle_fade_cache;
#[path = "render_background.rs"]
mod render_background;
#[path = "ascii_mask.rs"]
mod ascii_mask;
#[path = "glyph_mask.rs"]
mod glyph_mask;
#[path = "glyph_ink.rs"]
mod glyph_ink;
#[path = "menu_heartbeat.rs"]
mod menu_heartbeat;
#[path = "menu_anatomy.rs"]
mod menu_anatomy;
#[path = "menu_raster.rs"]
mod menu_raster;
#[path = "enemy_list.rs"]
pub mod enemy_list;
#[path = "render_damage.rs"]
pub mod damage;
#[path="tile_damage.rs"]
pub mod tile_damage;
pub use damage::RetainedGame;
pub const WIDTH: usize = 1280;
pub const HEIGHT: usize = 800; // Authored menu and dialog sheets.
pub const GAME_HEIGHT: usize = 720;
pub const GAME_UI_UNIT: i32 = 8;
const GAME_RESOURCE_WIDTH: i32 = crate::game_hud::RAIL_WIDTH;

/// O(1): fit the gameplay baseline or authored sheet without stretching.
pub(crate) fn canvas_scale(width: usize, height: usize, gameplay: bool) -> f64 {
    let reference_height = if gameplay { GAME_HEIGHT } else { HEIGHT };
    (width as f64 / WIDTH as f64).min(height as f64 / reference_height as f64)
}
/// R50 follow-up: 4.5% smaller than the R46 88% sheets (70.56% of full area).
/// The shared input/damage transform and authored content remain unchanged.
pub const DIALOG_SCALE: f64 = 0.84;
pub type Rgb = [u8; 3];
pub const GOLD: Rgb = [248, 209, 0];
pub const INK: Rgb = [233, 229, 239];
pub const MUTED: Rgb = [119, 115, 132];
const CORPSE_CROSS: Rgb = [245, 54, 64];
#[derive(Clone)]
pub struct Picture {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}
impl Picture {
    pub fn png(data: &[u8]) -> Self {
        let mut decoder = png::Decoder::new(data);
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().expect("authored PNG");
        let mut bytes = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut bytes).expect("PNG pixels");
        let channels = info.color_type.samples();
        let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
        for p in bytes[..info.buffer_size()].chunks(channels) {
            match channels {
                4 => rgba.extend_from_slice(p),
                3 => rgba.extend_from_slice(&[p[0], p[1], p[2], 255]),
                2 => rgba.extend_from_slice(&[p[0], p[0], p[0], p[1]]),
                _ => rgba.extend_from_slice(&[p[0], p[0], p[0], 255]),
            }
        }
        Self {
            w: info.width as usize,
            h: info.height as usize,
            rgba,
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Animation {
    Fire { phase: f64, speed:f64, colors:[Rgb;4] },
    Water { phase: f64, speed: f64 },
    Gas { phase: f64, color: Rgb },
    Reflection { phase: f64, color: Rgb },
    Razorleaves { phase: f64, speed: f64, color: Rgb, opacity: f64 },
    WardPulse { speed: f64 },
    Torch,
}
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct EntityInk {
    /// Exact default identity ink with a small neutral readability outline.
    pub identity_outline: Option<Rgb>,
    /// Designer-authored black Dire Bear ink never lifts for contrast.
    pub plain_black: bool,
    pub color: Rgb,
    pub player: bool,
    pub telepathic: bool,
    pub important: bool,
    pub remembered: bool,
    pub detected: bool,
}
#[derive(Default, Clone, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Cell {
    /// Already-final map calibration; late native readability must not undo it.
    pub display_light: Option<crate::color::light_ui::DisplayLight>,
    /// Exact source ink/overlay for detached animated reconstruction.
    pub unlit_ink: Option<(Rgb, Option<(Rgb, f64)>)>,
    /// Six detached actor-only cosmetic variants; each cached entry has None.
    pub hallucination: Option<std::sync::Arc<[Cell;6]>>,
    /// Cached visible Timeweave direction; paint never reads the Run or trail index.
    pub timeweave: Option<(i8, i8)>,
    /// Exact known original duel perimeter; persistent state, no animation loop.
    pub duel_edges:[bool;4],
    /// Italic treatment of the ASCII portal glyph, not a different character.
    pub italic: bool,
    /// Ground-cover ink alignment only; backgrounds and overlays keep tile bounds.
    pub bottom_aligned: bool,
    /// Detached pocket marker, retained during animated terrain reconstruction.
    pub non_euclidean: bool,
    /// Authored live terrain ink; never inferred from hidden floor modifications.
    pub cogworks: bool,
    /// Projection and nightmares retain their own ink during Dreamwalking.
    pub dream_entity: bool,
    pub fire: bool,
    pub underlays: Vec<(String, Rgb, f64)>,
    pub material_background: Option<Rgb>,
    pub entity_ink: Option<EntityInk>,
    pub background_overlay: Option<(Rgb, f64)>,
    pub emphasis: f64,
    pub pulse_speed: f64,
    pub fire_flicker: bool,
    pub x: usize,
    pub y: usize,
    pub glyph: String,
    pub label: String,
    pub edges: [Option<Rgb>; 4],
    pub animations: Vec<Animation>,
    pub current: bool,
    pub direct: bool,
    pub actor_id: Option<f64>,
    /// Detached ground-item identity, used only to stage a recovered projectile.
    pub ground_item_id: Option<u64>,
    pub controllable_ally: bool,
    pub heartless: bool,
    /// Detached death marker; living sleeping bodies remain unmarked.
    pub corpse: bool,
    pub superscript: String,
    /// Detached phase number on the real Avatar.
    pub number_of_blood: Option<u8>,
    pub subscript: String,
    pub arrow: String,
    pub fg: Rgb,
    pub bg: Rgb,
}
impl Cell {
    pub(crate) fn marker_spill(&self) -> usize {
        (self.superscript.chars().count()+usize::from(self.number_of_blood.is_some()))
            .max(self.subscript.chars().count()).max(self.arrow.chars().count())
    }
}
#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct Creature {
    pub has_heart: Option<bool>,
    pub perception_range: Option<String>,
    pub identity_outline: Option<Rgb>,
    /// Six detached sidebar variants; never retains a Run or hidden actor.
    pub hallucination: Option<std::sync::Arc<[Creature;6]>>,
    pub shared_resources: bool,
    pub invulnerable: bool,
    pub id: f64,
    pub name: String,
    pub glyph: String,
    pub color: Rgb,
    pub state: String,
    pub distance: f64,
    pub hp: f64,
    pub max_hp: f64,
    pub energy: f64,
    pub max_energy: f64,
    pub uses_energy: bool,
    pub statuses: Vec<crate::statuses_ui::Status>,
    pub meters: Vec<crate::statuses_ui::Meter>,
}
#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct View {
    pub ability_overlays: crate::persistent_ability_overlays::Overlays,
    /// Detached known-only R51 creature-sight contours, selected by two-way hover.
    pub perception_boundaries: Vec<crate::perception_ui::Boundary>,
    pub hallucination_cells: Vec<usize>,
    pub hallucination_phase: usize,
    pub dreamwalking: bool,
    /// Canonical swimming state of the current player body, detached from Run.
    pub deep_water: bool,
    pub terrain_snapshot: Option<std::sync::Arc<corpse_core::vision::TerrainObservation>>,
    pub terrain_count: usize,
    pub cosmetic_seed: u32,
    pub fire_seed: u32,
    pub path: Vec<(usize, usize)>,
    /// Replanned only on world observation; pointer updates reuse this route.
    pub pursuit_path: Vec<(usize, usize)>,
    pub target_path: Vec<(usize, usize)>,
    pub target_area: Vec<(usize, usize)>,
    pub target_impact: Option<(usize, usize)>,
    pub target_color: Option<Rgb>,
    pub selected_ability: Option<String>,
    pub cogworks_mode: u8,
    /// Possible exits, not a selected or promised movement/projectile outcome.
    pub possible_exits: Vec<(usize, usize)>,
    pub hover: Option<(usize, usize)>,
    pub player_id: f64,
    pub player_statuses: Vec<crate::statuses_ui::Status>,
    pub player_status_offset: usize,
    pub selected_actor: Option<f64>,
    pub player_meters: Vec<crate::statuses_ui::Meter>,
    pub pack_hud: crate::game_hud::PackHud,
    pub provenance: String,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
    pub hp: f64,
    pub max_hp: f64,
    /// Detached per-turn Health, Energy and Blessings change labels.
    pub resource_changes: [String; 3],
    pub pending_energy_loss: f64,
    /// Effective display contributions: nominal melee Damage, HP, DR, Accuracy, Dodge.
    pub player_stats: [f64;5],
    pub energy: f64,
    pub max_energy: f64,
    pub turn: f64,
    pub depth: f64,
    pub display_depth: f64,
    pub max_depth: u32,
    pub log: Vec<String>,
    pub chronicle_offset: usize,
    pub log_groups: Vec<crate::chronicle::Group>,
    pub creatures: Vec<Creature>,
    pub ground_entities: Vec<enemy_list::GroundEntity>,
    pub inventory: Vec<String>,
    pub dead: bool,
    pub state: String,
    pub domain: String,
    pub mode: String,
    pub gods: String,
    pub blessings: f64,
    pub blessing_goal: f64,
    pub offers: Vec<crate::ability_ui::AbilityView>,
    pub level: f64,
    pub god_labels: Vec<(String, Rgb)>,
    pub abilities: Vec<crate::ability_ui::AbilityView>,
    pub total_abilities: usize,
}
impl View {
    pub fn measure_conditions(&mut self, meters: &mut crate::statuses_ui::ConditionMeters) {
        self.player_meters = meters.measure(self.player_id, &self.player_statuses);
        for a in &mut self.creatures {
            a.meters = meters.measure(a.id, &a.statuses);
            if let Some(cache)=a.hallucination.as_mut(){
                for variant in std::sync::Arc::make_mut(cache).iter_mut(){
                    variant.meters=variant.statuses.iter().map(|status|a.meters.iter().find(|m|m.name==status.name).cloned().unwrap_or_else(||crate::statuses_ui::Meter{
                        name:status.name.clone(),duration:status.duration,reference:status.duration.unwrap_or(1.).max(1.),reference_known:status.duration.is_none(),percent:100.
                    })).collect();
                }
            }
        }
    }
}
/// O(clipped mask area) time, O(1) extra space. Preserve text's historical alpha
/// behavior; native-pixel callers instead force alpha even for zero coverage.
fn blend_mask(pixels: &mut [u8], pitch: usize, bounds: (i32,i32,i32,i32),
    x: i32, y: i32, w: usize, h: usize, mask: &[u8], color: Rgb, force_alpha: bool, stats:&mut damage::RasterStats, tile_mask:Option<&tile_damage::RowSpans>) {
    #[cfg(test)]
    let mut _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Mask);
    let (l,t,r,b) = bounds;
    let (l,t,r,b) = (l.max(x),t.max(y),r.min(x+w as i32),b.min(y+h as i32));
    if l >= r || t >= b { return; }
    for py in t..b {
        for (l,r) in tile_damage::row_segments(tile_mask,py,l,r){
        stats.pixels((r-l)as usize);
            #[cfg(test)]
            _paint_profile.pixels((r-l) as usize);
        let src = ((py-y) as usize*w+(l-x) as usize)..((py-y) as usize*w+(r-x) as usize);
        let dest = &mut pixels[(py as usize*pitch+l as usize)*4..(py as usize*pitch+r as usize)*4];
        for (pixel,&alpha) in dest.chunks_exact_mut(4).zip(&mask[src]) {
            let a = alpha as u32;
            for k in 0..3 { pixel[k] = ((color[k] as u32*a+pixel[k] as u32*(255-a))/255) as u8; }
            if force_alpha { pixel[3]=255; }
        }
        }
    }
}
/// O(clipped mask area) time, O(1) space. Coverage scaling retains f64 rounding
/// and saturating u8 conversion used by compound glyph native_pixel calls.
fn blend_scaled_mask(pixels: &mut [u8], pitch: usize, bounds: (i32,i32,i32,i32),
    x: i32, y: i32, w: usize, h: usize, mask: &[u8], color: Rgb, alpha: f64, stats:&mut damage::RasterStats, tile_mask:Option<&tile_damage::RowSpans>) {
    #[cfg(test)]
    let mut _paint_profile = paint_profile_bench::span(if alpha == 1. { paint_profile_bench::Kind::ScaledMaskUnit } else { paint_profile_bench::Kind::ScaledMaskOther });
    // Byte coverage is already exact at unit opacity. Preserve forced alpha;
    // other opacity values retain their historical rounding and saturation.
    if alpha == 1. {
        blend_mask(pixels,pitch,bounds,x,y,w,h,mask,color,true,stats,tile_mask);
        return;
    }
    let (l,t,r,b)=bounds;
    let (l,t,r,b)=(l.max(x),t.max(y),r.min(x+w as i32),b.min(y+h as i32));
    if l>=r || t>=b {return;}
    for py in t..b {
        for (l,r) in tile_damage::row_segments(tile_mask,py,l,r){
        stats.pixels((r-l)as usize);
            #[cfg(test)]
            _paint_profile.pixels((r-l) as usize);
        let src=((py-y) as usize*w+(l-x) as usize)..((py-y) as usize*w+(r-x) as usize);
        let dest=&mut pixels[(py as usize*pitch+l as usize)*4..(py as usize*pitch+r as usize)*4];
        for (pixel,&coverage) in dest.chunks_exact_mut(4).zip(&mask[src]) {
            let a=(coverage as f64*alpha).round() as u8 as u32;
            for k in 0..3 {pixel[k]=((color[k] as u32*a+pixel[k] as u32*(255-a))/255) as u8;}
            pixel[3]=255;
        }
        }
    }
}
pub struct ResizePlan { width:usize, height:usize, pixels:Vec<u8> }
pub const MAX_FRAMEBUFFER_PIXELS:usize=33_554_432;
pub struct Painter {
    /// Painter-only Firebolt staging; canonical View and inspection stay current.
    pub firebolt_terrain: Option<crate::staged_firebolt::PaintStage>,
    pub gpu: Option<crate::gpu_render::Frame>,
    responsive_canvas:bool,
    pub tile_mask: tile_damage::RowSpans,
    pub tile_clip_enabled: bool,
    pub tile_cell_bounds: Vec<Option<damage::Rect>>,
    /// Native damage scissor, independent of nested logical UI clips.
    pub damage_clip: Option<damage::Rect>,
    pub paint_cell_indices: Option<Vec<usize>>,
    pub raster_stats: damage::RasterStats,
    /// Observation-only pointer/keyboard selection, never input dispatch.
    pub toolbar_focus: Option<usize>,
    pub bestiary_unread: bool,
    pub enemy_list_offset: usize,
    pub item_rail_offset: usize,
    pub enemy_hover: Option<u64>,
    pub ground_hover: Option<(usize,usize)>,
    pub ability_hover: Option<usize>,
    pub chronicle_hover: bool,
    pub prose_contrast_outline: bool,
    /// Temporary call-scoped backing rectangle, never retained scene metadata.
    pack_map_occlusion: Option<damage::Rect>,
    #[cfg(test)]
    pack_occlusion_stats: pack_occlusion::Stats,
    /// Same-binary background algorithm control; absent from production.
    #[cfg(test)]
    background_fused_enabled: bool,
    chronicle_fade_cache: chronicle_fade_cache::FadeCache,
    #[cfg(test)]
    chronicle_fade_cache_enabled: bool,
    pub profile: crate::appearance_profile::Profile,
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
    scale: f64,
    origin: (i32, i32),
    font: fontdue::Font,
    serif: fontdue::Font,
    title_font: fontdue::Font,
    pub clip: Option<(i32, i32, i32, i32)>,
    fallback: Vec<fontdue::Font>,
    glyphs: HashMap<(char, u16), (fontdue::Metrics, Vec<u8>)>,
    styled_glyphs: HashMap<(char, u16, usize, u16), (fontdue::Metrics, Vec<u8>)>,
    identity_masks: HashMap<(String, i32, i32, u32, bool, u64, u64), (i32, i32, usize, usize, Vec<u8>, Option<glyph_ink::Bounds>)>,
    prose_glyphs: HashMap<(char, u32, bool), (fontdue::Metrics, std::sync::Arc<[u8]>)>,
    map_masks: HashMap<(char, i32, i32, u16), ascii_mask::AsciiMask>,
    /// Same-binary algorithm control; never a production setting.
    #[cfg(test)]
    ascii_sparse_enabled: bool,
    atlas: Picture,
    pub logo: Picture,
    menu_layers: std::sync::Arc<[Picture; 3]>,
    menu_raster: Option<menu_raster::Raster>,
}
impl Painter {
    /// Asset/font decode and layer extraction plus O(P) framebuffer; empty new caches O(1).
    pub fn new() -> Self {
        let logo = crate::embedded_assets::compact_picture(include_bytes!("../assets/ui-runtime/menu-logo.webp"));
        let menu_layers = std::sync::Arc::new(menu_anatomy::layers(&logo));
        Self {
            firebolt_terrain: None,
            gpu: None,
            responsive_canvas:false,
            tile_mask:tile_damage::RowSpans::default(),tile_clip_enabled:false,tile_cell_bounds:Vec::new(),
            damage_clip: None,
            paint_cell_indices: None,
            raster_stats: damage::RasterStats::default(),
            toolbar_focus: None,
            bestiary_unread: false,
            enemy_list_offset: 0,
            item_rail_offset: 0,
            enemy_hover: None,
            ground_hover: None,
            ability_hover: None,
            chronicle_hover: false,
            prose_contrast_outline: false,
            pack_map_occlusion: None,
            #[cfg(test)]
            pack_occlusion_stats: pack_occlusion::Stats::default(),
            #[cfg(test)]
            background_fused_enabled: true,
            chronicle_fade_cache: chronicle_fade_cache::FadeCache::default(),
            #[cfg(test)]
            chronicle_fade_cache_enabled: true,
            profile: crate::appearance_profile::Profile::default(),
            pixels: vec![0; WIDTH * HEIGHT * 4],
            width: WIDTH,
            height: HEIGHT,
            scale: 1.,
            origin: (0, 0),
            serif: fontdue::Font::from_bytes(
                include_bytes!("../assets/Inconsolata-Regular.ttf") as &[u8],
                fontdue::FontSettings::default(),
            )
            .unwrap(),
            title_font: fontdue::Font::from_bytes(
                include_bytes!("../assets/Inconsolata-ExtraBold.ttf") as &[u8],
                fontdue::FontSettings::default(),
            )
            .unwrap(),
            clip: None,
            font: fontdue::Font::from_bytes(
                include_bytes!("../assets/IBMPlexMono-Regular.ttf") as &[u8],
                fontdue::FontSettings::default(),
            )
            .unwrap(),
            fallback: vec![
                include_bytes!("../assets/Inconsolata-Variable.ttf") as &[u8],
                include_bytes!("../assets/CorpseGlyphMono.ttf") as &[u8],
                include_bytes!("../assets/CorpseGlyphSymbols.ttf") as &[u8],
                include_bytes!("../assets/CorpseGlyphCJK.otf") as &[u8],
                // Existing licensed face supplies Greek Ω absent from the subset glyph fonts.
                // Append only, preserving all authored face indices and ASCII atlas behavior.
                include_bytes!("../assets/LiberationSerif-Regular.ttf") as &[u8],
            ]
            .into_iter()
            .map(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).unwrap())
            .collect(),
            glyphs: HashMap::new(),
            styled_glyphs: HashMap::new(),
            identity_masks: HashMap::new(),
            prose_glyphs: HashMap::new(),
            map_masks: HashMap::new(),
            #[cfg(test)]
            ascii_sparse_enabled: true,
            atlas: crate::embedded_assets::compact_picture(include_bytes!("../assets/ui-runtime/BrogueCE-ASCII.webp")),
            logo,
            menu_layers,
            menu_raster: None,
        }
    }
    pub fn clear(&mut self, c: Rgb) {
        if let Some(gpu)=self.gpu.as_mut() { gpu.quads.clear();gpu.clear=Some(c);return; }
        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Clear);
        // Clear retains its historical independence from logical clips, but may
        // never erase pixels outside the compositor's native damage rectangle.
        let bounds=damage::Rect::new(0,0,self.width,self.height);
        let Some(area)=self.damage_clip.unwrap_or(bounds).intersect(bounds) else{return;};
        for y in area.y..area.bottom() {
            for (l,r) in tile_damage::row_segments(self.tile_clip_enabled.then_some(&self.tile_mask),y as i32,area.x as i32,area.right()as i32){
            self.raster_stats.pixels((r-l)as usize);
            #[cfg(test)]
            _paint_profile.pixels((r-l) as usize);
            for p in self.pixels[(y*self.width+l as usize)*4..(y*self.width+r as usize)*4].chunks_exact_mut(4) {
                p.copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
            }
        }
    }
    /// O(W*H) allocation before mutation. Same-size is O(1); bounded arithmetic
    /// and fallible reserve preserve the previous drawable on rejected sizes.
    pub fn prepare_resize(&self,width:usize,height:usize)->Result<Option<ResizePlan>,String>{
        if width==0||height==0{return Ok(None)}
        if (width,height)==(self.width,self.height){return Ok(None)}
        let count=width.checked_mul(height).filter(|&n|n<=MAX_FRAMEBUFFER_PIXELS).ok_or("Window exceeds the 33,554,432-pixel framebuffer budget; resize smaller")?;
        // The taller reference has the smaller scale and bounds both canvases.
        let scale=canvas_scale(width,height,false);
        if width>i32::MAX as usize||height>i32::MAX as usize||width as f64/scale>i32::MAX as f64/4.||height as f64/scale>i32::MAX as f64/4.{return Err("Window aspect exceeds drawing coordinate bounds".into())}
        let bytes=count.checked_mul(4).ok_or("Framebuffer byte count overflow")?;
        let mut pixels=Vec::new();pixels.try_reserve_exact(bytes).map_err(|_|"Cannot allocate window framebuffer; resize smaller")?;pixels.resize(bytes,0);
        Ok(Some(ResizePlan{width,height,pixels}))
    }
    pub fn commit_resize(&mut self,plan:ResizePlan){
        if let Some(gpu)=self.gpu.as_mut(){gpu.invalidate();}
        self.width=plan.width;self.height=plan.height;self.pixels=plan.pixels;
        self.set_responsive_canvas(self.responsive_canvas);
        self.chronicle_fade_cache.invalidate();
        self.damage_clip=None;self.glyphs.clear();self.styled_glyphs.clear();self.map_masks.clear();self.prose_glyphs.clear();self.identity_masks.clear();self.menu_raster=None;
    }
    pub fn try_resize(&mut self,width:usize,height:usize)->Result<bool,String>{if let Some(plan)=self.prepare_resize(width,height)?{self.commit_resize(plan);Ok(true)}else{Ok(false)}}
    /// Compatibility helper for bounded renderer fixtures. Live SDL uses prepare /
    /// commit so texture allocation failure also preserves the old framebuffer.
    pub fn resize(&mut self,width:usize,height:usize)->bool{self.try_resize(width,height).expect("Invalid renderer fixture size")}
    pub fn set_responsive_canvas(&mut self, responsive: bool) {
        self.responsive_canvas = responsive;
        (self.scale, self.origin) = self.canvas_transform(responsive);
    }
    fn canvas_transform(&self, gameplay: bool) -> (f64, (i32, i32)) {
        let scale = canvas_scale(self.width, self.height, gameplay);
        let origin = if gameplay { (0, 0) } else { self.sheet_origin(scale) };
        (scale, origin)
    }
    fn sheet_origin(&self, scale: f64) -> (i32, i32) {
        (((self.width as f64-WIDTH as f64*scale)/2.).round() as i32,
         ((self.height as f64-HEIGHT as f64*scale)/2.).round() as i32)
    }
    /// O(1) compact modal transform. All authored content and scroll ranges stay
    /// intact. Input and damage use the exact same transform, including HiDPI.
    fn sheet_transform(&self)->(f64,(i32,i32)) {
        let scale=canvas_scale(self.width,self.height,false)*DIALOG_SCALE;
        (scale,self.sheet_origin(scale))
    }
    pub fn centered_sheet<T>(&mut self,paint:impl FnOnce(&mut Self)->T)->T {
        let prior=(self.scale,self.origin,self.responsive_canvas);let(scale,origin)=self.sheet_transform();
        self.scale=scale;self.origin=origin;self.responsive_canvas=false;
        let result=paint(self);self.scale=prior.0;self.origin=prior.1;self.responsive_canvas=prior.2;result
    }
    pub fn logical_point_for_sheet(&self,x:i32,y:i32,window:(u32,u32))->(i32,i32) {
        let(scale,origin)=self.sheet_transform();let px=x as f64*self.width as f64/window.0.max(1)as f64;let py=y as f64*self.height as f64/window.1.max(1)as f64;
        (((px-origin.0 as f64)/scale).floor()as i32,((py-origin.1 as f64)/scale).floor()as i32)
    }
    pub fn sheet_damage_rect(&self,x:i32,y:i32,w:i32,h:i32)->Option<damage::Rect> {
        if w<=0||h<=0{return None;}let(scale,origin)=self.sheet_transform();
        let nx=|x:i32|(origin.0+(x as f64*scale).round()as i32).clamp(0,self.width as i32);
        let ny=|y:i32|(origin.1+(y as f64*scale).round()as i32).clamp(0,self.height as i32);
        let(l,t,r,b)=(nx(x),ny(y),nx(x.saturating_add(w)),ny(y.saturating_add(h)));
        (r>l&&b>t).then(||damage::Rect::new(l as usize,t as usize,(r-l)as usize,(b-t)as usize))
    }
    pub fn logical_point_for_canvas(&self,x:i32,y:i32,window:(u32,u32),responsive:bool)->(i32,i32){
        let (scale,origin)=self.canvas_transform(responsive);
        let px=x as f64*self.width as f64/window.0.max(1)as f64;let py=y as f64*self.height as f64/window.1.max(1)as f64;
        (((px-origin.0 as f64)/scale).floor()as i32,((py-origin.1 as f64)/scale).floor()as i32)
    }
    pub fn logical_width(&self)->i32{if self.responsive_canvas{(self.width as f64/self.scale).ceil()as i32}else{WIDTH as i32}}
    pub fn logical_height(&self)->i32{if self.responsive_canvas{(self.height as f64/self.scale).ceil()as i32}else{HEIGHT as i32}}
    pub fn frame(&self)->crate::modal_ui::Frame{crate::modal_ui::Frame::for_size(&self.profile,self.logical_width(),self.logical_height())}
    pub fn hud_left(&self)->i32{hud_left(&self.profile)+self.logical_width()-WIDTH as i32}
    pub fn toolbar_y(&self)->i32{self.hud_tracks().button(0).1}
    pub fn hud_tracks(&self)->crate::game_hud::Tracks{crate::game_hud::Tracks::resolve(self)}
    pub fn toolbar_action(&self,x:i32,y:i32)->Option<usize>{
        let tracks=self.hud_tracks();
        if tracks.hearts.contains(x,y) { return Some(crate::game_hud::HEARTS); }
        if tracks.potions.contains(x,y) { return Some(crate::game_hud::POTIONS); }
        TOOLBAR_VISIBLE_ACTIONS.iter().enumerate()
            .find_map(|(slot,id)|tracks.button(slot).contains(x,y).then_some(*id))
    }
    pub fn game_toolbar_rect(&self, slot: usize) -> (i32, i32) {
        let r=self.hud_tracks().button(slot);(r.0,r.2)
    }
    pub fn game_ability_top(&self)->i32 {
        self.hud_tracks().divine.1+crate::game_hud::RAIL_INSET
    }
    pub fn game_ability_row(&self)->i32 {
        (crate::ability_ui::row(&self.profile)+18).max(54)
    }
    pub fn ability_capacity(&self) -> usize {
        let d=self.hud_tracks().divine;
        ((d.1+d.3-self.game_ability_top())/self.game_ability_row()).max(0) as usize
    }
    /// O(C+A) over detached visible cells/creatures; no world or hidden actor reads.
    pub fn enemy_at_map_point(&self,v:&View,x:i32,y:i32)->Option<u64>{
        let(cw,ch,ox,oy)=self.map_layout(v.width,v.height);
        if x<ox||y<oy||x>=ox+cw*v.width as i32||y>=oy+ch*v.height as i32{return None;}
        let point=(((x-ox)/cw)as usize,((y-oy)/ch)as usize);
        let id=v.cells.iter().rev().find(|c|(c.x,c.y)==point&&c.actor_id.is_some())?.actor_id?;
        v.creatures.iter().any(|a|a.id==id).then_some(id.to_bits())
    }
    pub fn ability_inspect_at(&self,count:usize,x:i32,y:i32)->Option<usize>{
        let top=self.game_ability_top();
        let row=self.game_ability_row();
        let n=count.min(self.ability_capacity());
        let left=self.hud_left();
        let bounds=crate::modal_ui::Rect(left+12,top,(self.logical_width()-left-26).max(0),row*n as i32);
        (bounds.contains(x,y)&&row>0).then(||((y-top)/row) as usize)
    }
    pub fn chronicle_hit_rect(&self)->crate::modal_ui::Rect{let r=self.hud_tracks().chronicle;crate::modal_ui::Rect(8,8,r.2-16,r.3-16)}
    fn native_x(&self, x: f64) -> i32 {
        self.origin.0 + (x * self.scale).round() as i32
    }
    fn native_y(&self, y: f64) -> i32 {
        self.origin.1 + (y * self.scale).round() as i32
    }
    /// SDL mouse coordinates are window points; convert through drawable pixels for HiDPI.
    pub fn logical_point(&self, x: i32, y: i32, window: (u32, u32)) -> (i32, i32) {
        let px = x as f64 * self.width as f64 / window.0.max(1) as f64;
        let py = y as f64 * self.height as f64 / window.1.max(1) as f64;
        (
            ((px - self.origin.0 as f64) / self.scale).floor() as i32,
            ((py - self.origin.1 as f64) / self.scale).floor() as i32,
        )
    }
    /// O(1) conservative logical bounds cull; caller includes glyph/outline spill.
    /// O(1) logical-to-native conservative rectangle, clipped to framebuffer.
    pub fn logical_damage_rect(&self,x:i32,y:i32,w:i32,h:i32)->Option<damage::Rect>{
        if w<=0||h<=0{return None;}
        let l=self.native_x(x as f64).max(0).min(self.width as i32);
        let t=self.native_y(y as f64).max(0).min(self.height as i32);
        let r=self.native_x(x.saturating_add(w) as f64).max(0).min(self.width as i32);
        let b=self.native_y(y.saturating_add(h) as f64).max(0).min(self.height as i32);
        (r>l&&b>t).then(||damage::Rect::new(l as usize,t as usize,(r-l)as usize,(b-t)as usize))
    }
    /// O(1) interaction footprint. Rounded corner marks can extend a logical
    /// pixel, and rect() guarantees a native pixel even below scale one.
    /// Both retained paths must restore these strokes when targeting ends.
    pub fn interaction_damage_rect(&self,x:i32,y:i32,w:i32,h:i32)->Option<damage::Rect>{
        let r=self.logical_damage_rect(x-2,y-2,w+4,h+4)?;
        let l=r.x.saturating_sub(1);let t=r.y.saturating_sub(1);
        let right=r.right().saturating_add(1).min(self.width);
        let bottom=r.bottom().saturating_add(1).min(self.height);
        Some(damage::Rect::new(l,t,right-l,bottom-t))
    }
    /// O(text bytes) conservative full glyph/outline footprint. Long text is not
    /// clipped to its tile: include the entire contrast-dependent line footprint.
    pub fn cell_damage_rect(&self,c:&Cell,w:usize,h:usize)->Option<damage::Rect>{
        let(cw,ch,ox,oy)=self.map_layout(w,h);let x=ox+c.x as i32*cw;let y=oy+c.y as i32*ch;
        let spill=c.marker_spill();
        let extra=(spill as i32).saturating_mul(ch.max(7));
        let emphasis=c.emphasis.max(1.);
        let gx=((cw as f64*(emphasis-1.)/2.).ceil() as i32).max(0)
            +(self.profile.number("typography.map.alignX").abs()*cw as f64/15.).ceil() as i32+3;
        let gy=((ch as f64*(emphasis-1.)/2.).ceil() as i32).max(0)
            +(self.profile.number("typography.map.alignY").abs()*ch as f64/15.).ceil() as i32+3;
        self.logical_damage_rect(x-gx-extra,y-gy,cw+2*gx+extra.saturating_mul(2),ch+2*gy+(ch/2).max(7))
    }
    pub fn intersects_damage(&self,x:i32,y:i32,w:i32,h:i32)->bool {
        if self.tile_clip_enabled{return self.logical_damage_rect(x,y,w,h).is_some_and(|r|self.tile_mask.intersects(r));}
        let Some(d)=self.damage_clip else{return true;};
        let l=self.native_x(x as f64);let t=self.native_y(y as f64);
        let r=self.native_x((x+w) as f64);let b=self.native_y((y+h) as f64);
        r>d.x as i32&&b>d.y as i32&&l<d.right() as i32&&t<d.bottom() as i32
    }
    fn inside_native_clip(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && x < self.width as i32
            && y < self.height as i32
            && (!self.tile_clip_enabled || self.tile_mask.contains(x as usize,y as usize))
            && self.damage_clip.is_none_or(|r| r.contains(x as usize,y as usize))
            && self.clip.is_none_or(|(cx, cy, cw, ch)| {
                let (left, top) = (self.native_x(cx as f64), self.native_y(cy as f64));
                cw > 0
                    && ch > 0
                    && x >= left
                    && y >= top
                    && x < self.native_x((cx + cw) as f64).max(left + 1)
                    && y < self.native_y((cy + ch) as f64).max(top + 1)
            })
    }
    /// O(1) time/space for a sparse native pixel; exact clip and alpha semantics.
    fn native_pixel(&mut self, x: i32, y: i32, c: Rgb, a: u8) {
        if !self.inside_native_clip(x, y) {
            return;
        }
        if let Some(gpu)=self.gpu.as_mut(){gpu.fill((x,y,x+1,y+1),c,a);return;}
        self.raster_stats.pixels(1);
        let p = (y as usize * self.width + x as usize) * 4;
        if a == 255 {
            self.pixels[p..p+4].copy_from_slice(&[c[0],c[1],c[2],255]);
            return;
        }
        for k in 0..3 {
            self.pixels[p + k] = ((c[k] as u32 * a as u32
                + self.pixels[p + k] as u32 * (255 - a as u32))
                / 255) as u8;
        }
        self.pixels[p + 3] = 255;
    }
    /// O(1) time/space. Resolve logical clip once per batch, with legacy minimum strokes.
    fn native_bounds(&self) -> (i32, i32, i32, i32) {
        let (mut l, mut t, mut r, mut b) = (0, 0, self.width as i32, self.height as i32);
        if let Some((x, y, w, h)) = self.clip {
            if w <= 0 || h <= 0 { return (0, 0, 0, 0); }
            let (cx, cy) = (self.native_x(x as f64), self.native_y(y as f64));
            l = l.max(cx); t = t.max(cy);
            r = r.min(self.native_x((x + w) as f64).max(cx + 1));
            b = b.min(self.native_y((y + h) as f64).max(cy + 1));
        }
        if let Some(d)=self.damage_clip {
            l=l.max(d.x as i32);t=t.max(d.y as i32);
            r=r.min(d.right() as i32);b=b.min(d.bottom() as i32);
        }
        (l, t, r, b)
    }
    /// O(clipped area) time, O(1) extra space; no per-pixel coordinate/clip dispatch.
    /// Even alpha zero sets destination alpha to 255, exactly like native_pixel.
    fn native_fill(&mut self, left: i32, top: i32, right: i32, bottom: i32, c: Rgb, a: u8) {
        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(if a == 255 { paint_profile_bench::Kind::FillOpaque } else { paint_profile_bench::Kind::FillAlpha });
        let (l, t, r, b) = self.native_bounds();
        let (l, t, r, b) = (l.max(left), t.max(top), r.min(right), b.min(bottom));
        if l >= r || t >= b { return; }
        if let Some(gpu)=self.gpu.as_mut(){gpu.fill((l,t,r,b),c,a);return;}
        let width = self.width;
        let ink = [c[0], c[1], c[2], 255];
        let a = a as u32;
        let inverse = 255 - a;
        let source = c.map(|v| v as u32 * a);
        for y in t as usize..b as usize {
            for (l,r)in tile_damage::row_segments(self.tile_clip_enabled.then_some(&self.tile_mask),y as i32,l,r){
            self.raster_stats.pixels((r-l)as usize);
            #[cfg(test)]
            _paint_profile.pixels((r-l) as usize);
            let row = &mut self.pixels[(y * width + l as usize) * 4..(y * width + r as usize) * 4];
            if a == 255 {
                for pixel in row.chunks_exact_mut(4) { pixel.copy_from_slice(&ink); }
            } else {
                for pixel in row.chunks_exact_mut(4) {
                    for k in 0..3 { pixel[k] = ((source[k] + pixel[k] as u32 * inverse) / 255) as u8; }
                    pixel[3] = 255;
                }
            }
            }
        }
    }
    /// Ending tint covers the full drawable, including side panels and letterbox.
    pub fn ending_fade(&mut self, color: Rgb, alpha: u8) {
        if alpha == 0 { return; }
        let clips = (self.tile_clip_enabled, self.clip, self.damage_clip);
        self.tile_clip_enabled = false;
        self.clip = None;
        self.damage_clip = None;
        self.native_fill(0, 0, self.width as i32, self.height as i32, color, alpha);
        (self.tile_clip_enabled, self.clip, self.damage_clip) = clips;
    }
    /// O(native tile area) time and O(1) scratch for three straight strokes.
    /// Equal screen-space axes keep diagonals at 45 degrees, irrespective of tile aspect.
    fn timeweave_arrow(&mut self, x:i32, y:i32, cw:i32, ch:i32, direction:(i8,i8)) {
        self.timeweave_arrow_ink(x,y,cw,ch,direction,[54,211,193]);
    }
    fn timeweave_arrow_ink(&mut self, x:i32, y:i32, cw:i32, ch:i32, (dx,dy):(i8,i8), color:Rgb) {
        if cw<3 || ch<3 || !(-1..=1).contains(&dx) || !(-1..=1).contains(&dy) || (dx==0 && dy==0) {return;}
        let norm=((dx as f64).powi(2)+(dy as f64).powi(2)).sqrt();
        let (ux,uy)=(dx as f64/norm,dy as f64/norm);
        let length=cw.min(ch) as f64*0.36;
        let (cx,cy)=(x as f64+cw as f64/2.,y as f64+ch as f64/2.);
        let tip=(cx+ux*length,cy+uy*length);
        let tail=(cx-ux*length,cy-uy*length);
        let head=length*0.65;
        let back=(tip.0-ux*head,tip.1-uy*head);
        let strokes=[(tail,tip),(tip,(back.0-uy*head*0.65,back.1+ux*head*0.65)),
            (tip,(back.0+uy*head*0.65,back.1-ux*head*0.65))];
        for py in self.native_y(y as f64)..self.native_y((y+ch) as f64) {
            for px in self.native_x(x as f64)..self.native_x((x+cw) as f64) {
                let mut coverage=0f64;
                for (a,b) in strokes {
                    let a=(self.native_x(a.0) as f64,self.native_y(a.1) as f64);
                    let b=(self.native_x(b.0) as f64,self.native_y(b.1) as f64);
                    let (vx,vy)=(b.0-a.0,b.1-a.1);
                    let t=(((px as f64+0.5-a.0)*vx+(py as f64+0.5-a.1)*vy)/(vx*vx+vy*vy).max(0.001)).clamp(0.,1.);
                    let distance=((px as f64+0.5-a.0-t*vx).powi(2)+(py as f64+0.5-a.1-t*vy).powi(2)).sqrt();
                    coverage=coverage.max((self.scale*0.5+0.5-distance).clamp(0.,1.));
                }
                if coverage>0. {self.native_pixel(px,py,color,(220.*coverage).round() as u8);}
            }
        }
    }
    /// Move the canonical water glyph gently inside its tile, never redraw it as
    /// a deforming curve. O(glyph raster coverage), cached mask; O(1) scratch.
    fn water_wave(&mut self,x:i32,y:i32,w:i32,h:i32,ink:Rgb,phase:f64) {
        if w<=0 || h<=0 { return; }
        let padx=(w/8).max(1);let pady=(h/10).max(1);
        let dx=(phase.sin()*padx as f64).round() as i32;
        let dy=((phase*0.53).sin()*pady as f64).round() as i32;
        self.map_identity("~",x+padx+dx,y+pady+dy,(w-2*padx).max(1),(h-2*pady).max(1),ink);
    }
    /// O(native pixel footprint) time, O(1) extra space. No minimum-width expansion.
    pub fn pixel(&mut self, x: i32, y: i32, c: Rgb, a: u8) {
        self.native_fill(self.native_x(x as f64), self.native_y(y as f64),
            self.native_x((x + 1) as f64), self.native_y((y + 1) as f64), c, a);
    }
    /// O(clipped native area) time, O(1) extra space; opaque contiguous-row writes.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        if w <= 0 || h <= 0 { return; }
        let (left, top) = (self.native_x(x as f64), self.native_y(y as f64));
        self.native_fill(left, top, self.native_x((x + w) as f64).max(left + 1),
            self.native_y((y + h) as f64).max(top + 1), c, 255);
    }
    /// One-native-pixel alpha line, so thin tethers survive downscaled windows.
    pub fn faint_line(&mut self,x0:i32,y0:i32,x1:i32,y1:i32,color:Rgb,alpha:u8){
        let(mut x,mut y)=(self.native_x(x0 as f64),self.native_y(y0 as f64));
        let(tx,ty)=(self.native_x(x1 as f64),self.native_y(y1 as f64));
        let dx=(tx-x).abs();let dy=-(ty-y).abs();let sx=if x<tx{1}else{-1};let sy=if y<ty{1}else{-1};let mut error=dx+dy;
        loop {self.native_fill(x,y,x+1,y+1,color,alpha);if x==tx&&y==ty{break;}
            let e=2*error;if e>=dy{error+=dy;x+=sx;}if e<=dx{error+=dx;y+=sy;}}
    }
    /// O(clipped native area), O(1) space; batches adjacent logical pixel footprints.
    /// Unlike opaque rect, zero-size subpixel footprints do not get a minimum stroke.
    pub fn blend_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb, a: u8) {
        if w <= 0 || h <= 0 { return; }
        self.native_fill(self.native_x(x as f64),self.native_y(y as f64),
            self.native_x((x+w) as f64),self.native_y((y+h) as f64),c,a);
    }
    /// O(native perimeter pixels), O(1) scratch. Keep minimum-one-pixel
    /// strokes inside the rounded panel footprint, including its own clip.
    pub fn border(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        if w <= 0 || h <= 0 { return; }
        let (left, top) = (self.native_x(x as f64), self.native_y(y as f64));
        let right = self.native_x((x + w) as f64).max(left + 1);
        let bottom = self.native_y((y + h) as f64).max(top + 1);
        let inner_left = self.native_x((x + 1) as f64).clamp(left + 1, right);
        let inner_top = self.native_y((y + 1) as f64).clamp(top + 1, bottom);
        let inner_right = self.native_x((x + w - 1) as f64).clamp(left, right - 1);
        let inner_bottom = self.native_y((y + h - 1) as f64).clamp(top, bottom - 1);
        self.native_fill(left, top, right, inner_top, c, 255);
        self.native_fill(left, inner_bottom, right, bottom, c, 255);
        self.native_fill(left, top, inner_left, bottom, c, 255);
        self.native_fill(inner_right, top, right, bottom, c, 255);
    }
    /// Warm O(G metric work + mask pixels); cold adds font rasterization; cached mask rows.
    // Keep a two-digit identity rank within its tile, rather than the next actor.
    fn superscript_x(&self, value: &str, x: i32, cw: i32, ch: i32) -> i32 {
        if value == "10" {
            let size = (ch / 2).max(7) as f32;
            let width: i32 = value.chars().map(|c| self.font.metrics(c, size).advance_width.round() as i32).sum();
            x + (cw - width).max(0)
        } else { x + cw * 2 / 3 }
    }
    fn number_of_blood_superscript(&mut self, cell: &Cell, x: i32, y: i32, cw: i32, ch: i32, avatar_ink: Rgb) {
        let Some(phase)=cell.number_of_blood else {return};
        if !(1..=5).contains(&phase) {return;}
        let size=(ch/2).max(7) as u16;
        let mut offset=x+cw-1;
        if !cell.superscript.is_empty() {
            let width:i32=cell.superscript.chars().map(|c|self.font.metrics(c,size as f32).advance_width.round() as i32).sum();
            offset=offset.max(self.superscript_x(&cell.superscript,x,cw,ch)+width+1);
        }
        // A tiny pixel digit stays legible at the smaller native map sizes
        // where an anti-aliased font can lose the entire small glyph.
        let pixels:[[u8;5];5]=[
            [2,6,2,2,7], [7,1,7,4,7], [7,1,7,1,7],
            [5,5,7,1,1], [7,4,7,1,7],
        ];
        let unit=(ch/12).max(1);
        let color=if phase==5 {[255,30,59]}else{avatar_ink};
        for (row,bits) in pixels[phase as usize-1].into_iter().enumerate() {
            for col in 0..3 {
                if bits & (4>>col) != 0 {
                    self.rect(offset+col*unit,y+row as i32*unit,unit,unit,color);
                }
            }
        }
    }
    pub fn text(&mut self, s: &str, x: i32, y: i32, size: u16, c: Rgb) {
        let displayed=crate::ability_ui::display_text(s);let s=displayed.as_ref();
        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Text);
        let mut cursor = x;
        let raster_size = (size as f64 * self.scale).max(1.).round() as u16;
        for ch in s.chars() {
            let key = (ch, raster_size);
            if !self.glyphs.contains_key(&key) {
                #[cfg(test)]
                _paint_profile.cache_miss();
                let face = if self.font.lookup_glyph_index(ch) != 0 {
                    &self.font
                } else {
                    self.fallback
                        .iter()
                        .find(|f| f.lookup_glyph_index(ch) != 0)
                        .unwrap_or(&self.font)
                };
                self.glyphs
                    .insert(key, face.rasterize(ch, raster_size as f32));
            }
            let bounds = self.native_bounds();
            let (m, bitmap) = self.glyphs.get(&key).unwrap();
            let m = *m;
            let gx = self.native_x(cursor as f64) + m.xmin;
            let gy = self.native_y((y + size as i32) as f64) - m.height as i32 - m.ymin;
            if let Some(gpu)=self.gpu.as_mut(){gpu.mask(crate::gpu_render::MaskKey::Text(ch,raster_size),bounds,gx,gy,m.width,m.height,bitmap,c,1.,false);}else{
            blend_mask(&mut self.pixels, self.width, bounds, gx, gy, m.width, m.height,
                bitmap, c, false, &mut self.raster_stats, self.tile_clip_enabled.then_some(&self.tile_mask));
            }

            let face = if self.font.lookup_glyph_index(ch) != 0 {
                &self.font
            } else {
                self.fallback
                    .iter()
                    .find(|f| f.lookup_glyph_index(ch) != 0)
                    .unwrap_or(&self.font)
            };
            cursor += face.metrics(ch, size as f32).advance_width.round() as i32;
        }
    }
    pub fn measure_text_role(
        &self,
        role: crate::typography_ui::Role,
        s: &str,
        baseline_size: u16,
    ) -> f64 {
        let displayed=crate::ability_ui::display_text(s);let s=displayed.as_ref();
        #[cfg(test)]
        let _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::TextMetrics);
        use crate::typography_ui::{Face, RoleStyle};
        let overrides = crate::typography_ui::overrides(&self.profile, role);
        let style = overrides.apply(RoleStyle::baseline(baseline_size as f64));
        let size = style.font_size.round().clamp(1., 128.) as f32;
        let candidates = crate::typography_ui::font_candidates(&style.font_family, false);
        let mut line = 0f64;
        let mut width = 0f64;
        for ch in s.chars() {
            if ch == '\n' {
                width = width.max(line);
                line = 0.;
                continue;
            }
            let face = candidates
                .faces
                .iter()
                .find_map(|f| {
                    let i = match f {
                        Face::IbmPlexMono => 0,
                        Face::Inconsolata => 1,
                        Face::CorpseGlyphMono => 2,
                        Face::CorpseGlyphSymbols => 3,
                        Face::CorpseGlyphCjk => 4,
                        Face::BrogueAscii => 0,
                    };
                    let f = if i == 0 {
                        &self.font
                    } else {
                        &self.fallback[i - 1]
                    };
                    (f.lookup_glyph_index(ch) != 0).then_some(f)
                })
                .unwrap_or(&self.font);
            let advance = face.metrics(ch, size).advance_width as f64;
            line += if !overrides.any() {
                advance.round()
            } else {
                advance + style.letter_spacing
            };
        }
        width.max(line)
    }
    /// Measured ellipsis uses O(n log n) metric work, O(n) temporary characters.
    pub fn fit_text_role(
        &self,
        role: crate::typography_ui::Role,
        text: &str,
        width: i32,
        size: u16,
    ) -> String {
        let displayed=crate::ability_ui::display_text(text);let text=displayed.as_ref();
        if self.measure_text_role(role, text, size) <= width as f64 {
            return text.to_string();
        }
        if self.measure_text_role(role, "…", size) > width as f64 {
            return String::new();
        }
        let chars: Vec<char> = text.chars().collect();
        let (mut low, mut high) = (0, chars.len());
        while low < high {
            let mid = (low + high + 1) / 2;
            let candidate = chars[..mid].iter().collect::<String>() + "…";
            if self.measure_text_role(role, &candidate, size) <= width as f64 {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        chars[..low].iter().collect::<String>() + "…"
    }
    pub fn wrap_text_role(
        &self,
        role: crate::typography_ui::Role,
        s: &str,
        width: i32,
        baseline_size: u16,
    ) -> Vec<String> {
        let displayed=crate::ability_ui::display_text(s);let s=displayed.as_ref();
        let mut output = Vec::new();
        for paragraph in s.split('\n') {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let proposed = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty()
                    && self.measure_text_role(role, &proposed, baseline_size) > width.max(1) as f64
                {
                    output.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                // Long identifiers are split by measured glyph advances rather than
                // overflowing the clipping region or changing panel geometry.
                if self.measure_text_role(role, word, baseline_size) > width.max(1) as f64 {
                    for ch in word.chars() {
                        let proposed = format!("{line}{ch}");
                        if !line.is_empty() && self.measure_text_role(role, &proposed, baseline_size) > width.max(1) as f64 {
                            output.push(std::mem::take(&mut line));
                        }
                        line.push(ch);
                    }
                } else { line.push_str(word); }
            }
            output.push(line);
        }
        output
    }
    /// Warm O(G metric work + mask pixels); cold adds rasterization/weight work; bounded cache.
    pub fn text_role(
        &mut self,
        role: crate::typography_ui::Role,
        s: &str,
        x: i32,
        y: i32,
        size: u16,
        color: Rgb,
    ) {
        let displayed=crate::ability_ui::display_text(s);let s=displayed.as_ref();
        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::TextRole);
        use crate::typography_ui::{Face, RoleStyle};
        let overrides = crate::typography_ui::overrides(&self.profile, role);
        if !overrides.any() {
            self.text(s, x, y, size, color);
            return;
        }
        let style = overrides.apply(RoleStyle::baseline(size as f64));
        let size = style.font_size.round().clamp(1., 128.) as u16;
        let raster_size = (size as f64 * self.scale).max(1.).round() as u16;
        let candidates = crate::typography_ui::font_candidates(&style.font_family, false);
        let faces: Vec<usize> = candidates
            .faces
            .iter()
            .map(|f| match f {
                Face::IbmPlexMono => 0,
                Face::Inconsolata => 1,
                Face::CorpseGlyphMono => 2,
                Face::CorpseGlyphSymbols => 3,
                Face::CorpseGlyphCjk => 4,
                Face::BrogueAscii => 0,
            })
            .collect();
        let mut cursor = x as f64;
        let mut top = y;
        for ch in s.chars() {
            if ch == '\n' {
                cursor = x as f64;
                top += (size as f64 * style.line_height).round() as i32;
                continue;
            }
            let face = faces
                .iter()
                .copied()
                .find(|i| {
                    let f = if *i == 0 {
                        &self.font
                    } else {
                        &self.fallback[*i - 1]
                    };
                    f.lookup_glyph_index(ch) != 0
                })
                .unwrap_or(0);
            let weight = style.font_weight.round().clamp(100., 900.) as u16;
            let key = (ch, raster_size, face, weight);
            if !self.styled_glyphs.contains_key(&key) {
                #[cfg(test)]
                _paint_profile.cache_miss();
                if self.styled_glyphs.len() >= 2048 {
                    self.styled_glyphs.clear();
                }
                let f = if face == 0 {
                    &self.font
                } else {
                    &self.fallback[face - 1]
                };
                let (m, mut pixels) = f.rasterize(ch, raster_size as f32);
                if weight != 400 {
                    for a in &mut pixels {
                        *a = ((*a as f64 / 255.).powf(400. / weight as f64) * 255.).round() as u8;
                    }
                }
                self.styled_glyphs.insert(key, (m, pixels));
            }
            let bounds = self.native_bounds();
            let (m, bitmap) = self.styled_glyphs.get(&key).unwrap();
            let m = *m;
            let gx = self.native_x(cursor) + m.xmin;
            let gy = self.native_y((top + size as i32) as f64) - m.height as i32 - m.ymin;
            if let Some(gpu)=self.gpu.as_mut(){gpu.mask(crate::gpu_render::MaskKey::Styled(ch,raster_size,face,weight),bounds,gx,gy,m.width,m.height,bitmap,color,1.,false);}else{
            blend_mask(&mut self.pixels, self.width, bounds, gx, gy, m.width, m.height,
                bitmap, color, false, &mut self.raster_stats, self.tile_clip_enabled.then_some(&self.tile_mask));
            }

            let f = if face == 0 {
                &self.font
            } else {
                &self.fallback[face - 1]
            };
            cursor += f.metrics(ch, size as f32).advance_width as f64 + style.letter_spacing;
        }
    }
    /// Expected O(1) warm lookup; cold font rasterization O(glyph outline + bitmap area),
    /// O(bitmap area) retained per key, capped at 2048 keys. Exact f32 size bits avoid
    /// rounding fractional device sizes differently from the original prose renderer.
    fn prose_raster(&mut self, ch: char, size: f32, title: bool) -> (fontdue::Metrics, std::sync::Arc<[u8]>) {
        let key=(ch,size.to_bits(),title);
        if !self.prose_glyphs.contains_key(&key) {
            if self.prose_glyphs.len()>=2048 { self.prose_glyphs.clear(); }
            let face=if title {&self.title_font} else {&self.serif};
            let (m,b)=face.rasterize(ch,size);
            self.prose_glyphs.insert(key,(m,b.into()));
        }
        self.prose_glyphs[&key].clone()
    }
    /// Warm O(G metric work + mask pixels), cold adds rasterization; outlines add O(mask pixels * radius²).
    pub fn prose(&mut self, s: &str, x: i32, y: i32, size: u16, c: Rgb, title: bool) {
        if self.prose_contrast_outline {if let Some(gpu)=self.gpu.as_mut(){gpu.unsupported=Some("Contrast-sampled prose requires software painting");return;}}

        #[cfg(test)]
        let _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Prose);
        if !self.prose_contrast_outline {
        let mut cursor = x;
        for ch in s.chars() {
            let face = if title { &self.title_font } else { &self.serif };
            let advance = face.metrics(ch, size as f32).advance_width.round() as i32;
            let (m, b) = self.prose_raster(ch, (size as f64 * self.scale).max(1.) as f32, title);
            let gx = self.native_x(cursor as f64) + m.xmin;
            let gy = self.native_y((y + size as i32) as f64) - m.height as i32 - m.ymin;
            let bounds = self.native_bounds();
            if let Some(gpu)=self.gpu.as_mut(){gpu.mask(crate::gpu_render::MaskKey::Prose(ch,((size as f64*self.scale).max(1.)as f32).to_bits(),title),bounds,gx,gy,m.width,m.height,&b,c,1.,false);}else{
            blend_mask(&mut self.pixels, self.width, bounds, gx, gy, m.width, m.height, &b, c, true, &mut self.raster_stats, self.tile_clip_enabled.then_some(&self.tile_mask));
            }
            cursor += advance;
        }
                return;
        }

        let mut cursor = x;
        let mut glyphs = Vec::new();
        for ch in s.chars() {
            let face = if title { &self.title_font } else { &self.serif };
            let advance = face.metrics(ch, size as f32).advance_width.round() as i32;
            let (m, bitmap) = self.prose_raster(ch, (size as f64 * self.scale).max(1.) as f32, title);
            let gx = self.native_x(cursor as f64) + m.xmin;
            let gy = self.native_y((y + size as i32) as f64) - m.height as i32 - m.ymin;
            glyphs.push((gx, gy, m, bitmap));
            cursor += advance;
        }
        let dark = c.iter().map(|&v| v as u32).sum::<u32>() < 384;
        // Choose contrast for the complete line, never individual letters or pixels.
        let outline = self.prose_contrast_outline && glyphs.iter().any(|(gx,gy,m,b)| {
            b.iter().enumerate().any(|(i,&alpha)| {
                let x = gx + (i % m.width) as i32;
                let y = gy + (i / m.width) as i32;
                if alpha < 96 || x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return false; }
                let at = (y as usize*self.width+x as usize)*4;
                let l = 0.2126*self.pixels[at] as f64+0.7152*self.pixels[at+1] as f64+0.0722*self.pixels[at+2] as f64;
                if dark { l < 125. } else { l > 140. }
            })
        });
        let draw_color = c;
        if outline {
            let halo = if dark { [255,255,255] } else { [0,0,0] };
            let r = self.scale.ceil().max(1.) as i32;
            for (gx,gy,m,b) in &glyphs { for yy in 0..m.height { for xx in 0..m.width {
                let alpha = b[yy*m.width+xx];
                if alpha == 0 { continue; }
                for dy in -r..=r { for dx in -r..=r {
                    self.native_pixel(gx+xx as i32+dx,gy+yy as i32+dy,halo,alpha);
                }}
            }}}
        }
        let bounds = self.native_bounds();
        for (gx,gy,m,b) in &glyphs {
            blend_mask(&mut self.pixels, self.width, bounds, *gx, *gy, m.width, m.height, b, draw_color, true, &mut self.raster_stats, self.tile_clip_enabled.then_some(&self.tile_mask));
        }
    }
    /// Measured with exactly the rounded per-glyph advances used by prose.
    pub fn prose_width(&self, text: &str, size: u16, title: bool) -> i32 {
        let face = if title { &self.title_font } else { &self.serif };
        text.chars().map(|c| face.metrics(c, size as f32).advance_width.round() as i32).sum()
    }
    pub fn prose_lines(&self, text: &str, width: i32, size: u16) -> Vec<String> {
        let mut lines = vec![];
        let mut line = String::new();
        let mut w = 0.;
        for word in text.split_whitespace() {
            let ww: f32 = word
                .chars()
                .map(|c| self.serif.metrics(c, size as f32).advance_width.round())
                .sum();
            let gap = self.serif.metrics(' ', size as f32).advance_width.round();
            if !line.is_empty() && w + gap + ww > width as f32 {
                lines.push(std::mem::take(&mut line));
                w = 0.;
            }
            if !line.is_empty() {
                line.push(' ');
                w += gap;
            }
            line.push_str(word);
            w += ww;
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }
    pub fn center(&mut self, s: &str, y: i32, size: u16, c: Rgb) {
        let w = s
            .chars()
            .map(|ch| self.font.metrics(ch, size as f32).advance_width.round() as i32)
            .sum::<i32>();
        self.text(s, (WIDTH as i32 - w) / 2, y, size, c);
    }
    /// O(clipped destination area) time, O(1) extra space. Nearest-neighbor sampling
    /// and integer blend rounding are unchanged; clip/row/source-y work is hoisted.
    pub fn picture(&mut self, p: &Picture, x: i32, y: i32, w: i32, h: i32) {
        if let Some(gpu)=self.gpu.as_mut(){gpu.unsupported=Some("Picture painting requires software painting");return;}

        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Picture);
        let (nx, ny) = (self.native_x(x as f64), self.native_y(y as f64));
        let (w,h) = (self.native_x((x+w) as f64)-nx,self.native_y((y+h) as f64)-ny);
        if w <= 0 || h <= 0 { return; }
        let (l,t,r,b) = self.native_bounds();
        let (l,t,r,b) = (l.max(nx),t.max(ny),r.min(nx+w),b.min(ny+h));
        if l >= r || t >= b { return; }
        let step=p.w/w as usize;
        let remainder=p.w%w as usize;
        for yy in t..b {
            for(l,r)in tile_damage::row_segments(self.tile_clip_enabled.then_some(&self.tile_mask),yy,l,r){
            self.raster_stats.pixels((r-l)as usize);
            #[cfg(test)]
            _paint_profile.pixels((r-l) as usize);
            let initial=(l-nx)as usize*p.w;
            let source_row = (yy-ny) as usize*p.h/h as usize*p.w*4;
            let row = &mut self.pixels[(yy as usize*self.width+l as usize)*4..(yy as usize*self.width+r as usize)*4];
            let mut sx=initial/w as usize;
            let mut error=initial%w as usize;
            for pixel in row.chunks_exact_mut(4) {
                let at = source_row + sx*4;
                let a = p.rgba[at+3] as u32;
                if a==255 {
                    pixel.copy_from_slice(&p.rgba[at..at+4]);
                } else {
                    if a!=0 {for k in 0..3 {pixel[k]=((p.rgba[at+k] as u32*a+pixel[k] as u32*(255-a))/255) as u8;}}
                    // Original native_pixel forces opaque destination even for alpha zero.
                    pixel[3]=255;
                }
                sx+=step; error+=remainder;
                if error>=w as usize {sx+=1;error-=w as usize;}
            }
            }
        }
    }
    /// Cold O(source + destination area); warm O(occupied static samples + dimensions
    /// + transformed heart bounding-box area). Retains only one native-size cache.
    /// Original source pixels/layer order, clipping and forward sampling are unchanged.
    fn beating_emblem(&mut self, x: i32, y: i32, w: i32, h: i32, frame: u32) {
        if let Some(gpu)=self.gpu.as_mut(){gpu.unsupported=Some("Layered emblem requires software painting");return;}

        let (nx,ny)=(self.native_x(x as f64),self.native_y(y as f64));
        let (nw,nh)=(self.native_x((x+w) as f64)-nx,self.native_y((y+h) as f64)-ny);
        let bounds=self.native_bounds();
        if nw<=0||nh<=0||bounds.0>=bounds.2||bounds.1>=bounds.3 {return;}
        if self.menu_raster.as_ref().is_some_and(|r|r.size!=(nw,nh)) {
            self.menu_raster=None; // Release the prior resolution before allocating its replacement.
        }
        let raster=self.menu_raster.get_or_insert_with(||menu_raster::Raster::new(&self.menu_layers,nw,nh));
        if self.tile_clip_enabled {
            for y in bounds.1..bounds.3{for(l,r)in tile_damage::row_segments(Some(&self.tile_mask),y,bounds.0,bounds.2){raster.paint(&mut self.pixels,self.width,(nx,ny),(l,y,r,y+1),&self.menu_layers,frame);}}
        }else{raster.paint(&mut self.pixels,self.width,(nx,ny),bounds,&self.menu_layers,frame);}
    }
    // Rasterize each glyph/size once; unit opacity reuses exact nonzero row runs.
    pub fn map_glyph(&mut self, ch: char, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        self.map_glyph_alpha(ch, x, y, w, h, c, 1.);
    }
    fn map_glyph_alpha(&mut self, ch: char, x: i32, y: i32, w: i32, h: i32, c: Rgb, alpha: f64) {
        self.map_glyph_styled(ch, x, y, w, h, c, alpha, false);
    }
    /// Unit opacity: O(clipped rows + tile segments * log(row runs) + clipped runs
    /// + nonzero pixels). Other opacity stays O(clipped mask area). Cold adds atlas
    /// resampling, O(mask area) run construction and O(height + runs) metadata.
    /// Draw at the comma's actual ink baseline, not the bottom of its atlas box.
    /// The clip keeps both streams inside this tile even while entering/exiting.
    fn falling_razorleaves(&mut self, (x,y,w,h): (i32,i32,i32,i32), phase: f64, time: f64, color: Rgb, opacity: f64) {
        let previous = self.clip;
        let (l,t,r,b) = previous.map_or((x,y,x+w,y+h), |(cx,cy,cw,ch)| {
            (x.max(cx), y.max(cy), (x+w).min(cx+cw), (y+h).min(cy+ch))
        });
        if l>=r || t>=b {return;}
        self.clip=Some((l,t,r-l,b-t));
        let gw=(w*4/5).max(1);
        let gh=(h*9/10).max(1);
        for (lane, offset) in [(0.28,0.), (0.72,0.50)] {
            let p=(time/1180. + phase + offset).rem_euclid(1.);
            let drift=(p*std::f64::consts::TAU+lane*4.).sin();
            let gx=x+((lane+drift*0.16)*w as f64).round() as i32-gw/2;
            let baseline=y-1+(p*(h+4) as f64).round() as i32;
            let italic=(p*std::f64::consts::TAU*2.+lane).sin()>0.;
            self.map_glyph_placed(',',gx,y,gw,gh,color,opacity.clamp(0.,1.),italic,
                Some(self.native_y(baseline as f64)));
        }
        self.clip=previous;
    }
    fn map_glyph_styled(&mut self, ch: char, x: i32, y: i32, w: i32, h: i32, c: Rgb, alpha: f64, italic: bool) {
        self.map_glyph_placed(ch, x, y, w, h, c, alpha, italic, None);
    }
    /// Bottom is a native-pixel ink baseline, not a moved tile or a new mask.
    fn map_glyph_placed(&mut self, ch: char, x: i32, y: i32, w: i32, h: i32, c: Rgb, alpha: f64, italic: bool, ink_bottom: Option<i32>) -> Option<glyph_ink::Bounds> {
        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(if alpha == 1. { paint_profile_bench::Kind::AsciiUnit } else { paint_profile_bench::Kind::AsciiScaled });
        let code = ch as usize;
        if !(32..=126).contains(&code) {
            self.text(&ch.to_string(), x, y, h as u16, c);
            return None;
        }
        if ch == ' ' {
            return None;
        }
        let (nx, ny) = (self.native_x(x as f64), self.native_y(y as f64));
        let (w, h) = (
            self.native_x((x + w) as f64) - nx,
            self.native_y((y + h) as f64) - ny,
        );
        let (x, y) = (nx, ny);
        if w <= 0 || h <= 0 {
            return None;
        }
        let weight = self
            .profile
            .number("typography.map.fontWeight")
            .round()
            .clamp(100., 900.);
        let key = (ch, w, h, weight as u16);
        if !self.map_masks.contains_key(&key) {
                #[cfg(test)]
                _paint_profile.cache_miss();
            let sx = (code - 32) % 16 * 128;
            let sy = (code - 32) / 16 * 232;
            let mut mask = vec![0; (w * h) as usize];
            for yy in 0..h {
                for xx in 0..w {
                    let left = xx as f64 * 128. / w as f64;
                    let right = (xx + 1) as f64 * 128. / w as f64;
                    let top = yy as f64 * 232. / h as f64;
                    let bottom = (yy + 1) as f64 * 232. / h as f64;
                    let mut value = 0.;
                    for iy in top.floor() as usize..bottom.ceil() as usize {
                        let oy = bottom.min((iy + 1) as f64) - top.max(iy as f64);
                        for ix in left.floor() as usize..right.ceil() as usize {
                            let ox = right.min((ix + 1) as f64) - left.max(ix as f64);
                            let alpha = self.atlas.rgba
                                [((sy + iy) * self.atlas.w + sx + ix) * 4 + 3]
                                as f64
                                / 255.;
                            value += alpha * alpha * ox * oy;
                        }
                    }
                    mask[(yy * w + xx) as usize] = (255.
                        * (value / ((right - left) * (bottom - top)))
                            .sqrt()
                            .powf(400. / weight))
                    .round() as u8;
                }
            }
            if self.map_masks.len() > 512 {
                self.map_masks.clear();
            }
            self.map_masks.insert(key, ascii_mask::AsciiMask::new(mask,w as usize,h as usize));
        }
        let (left,top,right,bottom) = self.native_bounds();
        let cached = &self.map_masks[&key];
        let y = ink_bottom.map_or(y, |bottom| bottom - cached.ink_bottom as i32);
        let mask = &cached.coverage;
        let ink_bounds = if italic { cached.italic_ink_bounds } else { cached.ink_bounds }
            .map(|bounds| bounds.translated(x, y));
        if let Some(gpu)=self.gpu.as_mut(){gpu.mask(crate::gpu_render::MaskKey::Ascii(ch,w,h,weight as u16,italic),(left,top,right,bottom),x,y,w as usize,h as usize,mask,c,alpha,italic);return ink_bounds;}
        #[cfg(test)]
        let sparse = alpha == 1. && self.ascii_sparse_enabled;
        #[cfg(not(test))]
        let sparse = alpha == 1.;
        for yy in (top-y).max(0)..(bottom-y).min(h) {
            let shear = if italic { ((h - 1 - 2 * yy) as f64 * w as f64 / (8. * h.max(1) as f64)).round() as i32 } else { 0 };
            let l=(left-x-shear).max(0); let r=(right-x-shear).min(w);
            if l>=r {continue;}
            for(nl,nr)in tile_damage::row_segments(self.tile_clip_enabled.then_some(&self.tile_mask),y+yy,x+l+shear,x+r+shear){
                let l=nl-x-shear;let r=nr-x-shear;
                if sparse {
                    #[cfg(test)]
                    _paint_profile.ascii_coverage((r-l) as usize,0,0);
                    // Zero coverage leaves all RGBA bytes unchanged ONLY for this
                    // ASCII path. Forced-alpha text/compound paths remain untouched.
                    for (l,r) in cached.row_runs(yy as usize,l,r) {
                        self.raster_stats.pixels((r-l) as usize);
                        #[cfg(test)]
                        { _paint_profile.pixels((r-l) as usize);
                          _paint_profile.ascii_coverage(0,(r-l) as usize,1); }
                        let row=&mut self.pixels[((y+yy) as usize*self.width+(x+l+shear) as usize)*4
                            ..((y+yy) as usize*self.width+(x+r+shear) as usize)*4];
                        for (pixel,&coverage) in row.chunks_exact_mut(4).zip(&mask[(yy*w+l) as usize..(yy*w+r) as usize]) {
                            let a=coverage as u32;
                            for k in 0..3 {pixel[k]=((c[k] as u32*a+pixel[k] as u32*(255-a))/255) as u8;}
                        }
                    }
                } else {
                    self.raster_stats.pixels((r-l) as usize);
                    #[cfg(test)]
                    { _paint_profile.pixels((r-l) as usize);
                      if alpha == 1. && _paint_profile.enabled() {
                          let (nonzero,runs)=cached.row_runs(yy as usize,l,r)
                              .fold((0,0),|(pixels,runs),(l,r)|(pixels+(r-l) as usize,runs+1));
                          _paint_profile.ascii_coverage((r-l) as usize,nonzero,runs);
                      }
                    }
                    let row=&mut self.pixels[((y+yy) as usize*self.width+(x+l+shear) as usize)*4
                        ..((y+yy) as usize*self.width+(x+r+shear) as usize)*4];
                    // Keep the pre-sparse unit-alpha loop as the test control.
                    // Non-unit values retain their historical floating conversion.
                    if alpha == 1. {
                        for (pixel,&coverage) in row.chunks_exact_mut(4).zip(&mask[(yy*w+l) as usize..(yy*w+r) as usize]) {
                            let a=coverage as u32;
                            for k in 0..3 {pixel[k]=((c[k] as u32*a+pixel[k] as u32*(255-a))/255) as u8;}
                        }
                    } else {
                        for (pixel,&coverage) in row.chunks_exact_mut(4).zip(&mask[(yy*w+l) as usize..(yy*w+r) as usize]) {
                            let a=(coverage as f64*alpha).round() as u32;
                            for k in 0..3 {pixel[k]=((c[k] as u32*a+pixel[k] as u32*(255-a))/255) as u8;}
                        }
                    }
                }
            }
        }
        ink_bounds
    }

    /// Four fixed one-logical-pixel offsets, O(glyph raster coverage), no tile fill.
    pub fn map_identity_outline(&mut self,glyph:&str,x:i32,y:i32,w:i32,h:i32,ink:Rgb) {
        for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {self.map_identity(glyph,x+dx,y+dy,w,h,ink);}
    }
    pub fn map_identity(&mut self, glyph: &str, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        self.map_identity_aligned(glyph, x, y, w, h, c, false);
    }
    fn ground_number(&mut self,glyph:&str,x:i32,y:i32,cw:i32,ch:i32,gw:i32,gh:i32,ink:Rgb) -> Option<glyph_ink::Bounds> {
        let (nw,nh)=((gw*2/3).max(1),(gh*2/3).max(1));
        self.map_identity_aligned(glyph,x+(cw-nw)/2,y+ch-nh,nw,nh,ink,true)
    }
    fn map_identity_aligned(&mut self, glyph: &str, x: i32, y: i32, w: i32, h: i32, c: Rgb, bottom_aligned: bool) -> Option<glyph_ink::Bounds> {
        let scale = self.profile.number("typography.map.glyphScale")
            * self.profile.number("typography.map.fontSize")
            / 17.;
        let aspect = (self.profile.number("typography.map.tileHeight")
            / self.profile.number("typography.map.tileWidth"))
            / (24. / 15.);
        let mut horizontal = scale * aspect.min(1.);
        let mut vertical = horizontal * self.profile.number("typography.map.lineHeight") / aspect;
        let align_x = self.profile.number("typography.map.alignX");
        let align_y = self.profile.number("typography.map.alignY");
        if scale != 1. || aspect != 1. || vertical != 1. || align_x != 0. || align_y != 0. {
            let fit = 1f64
                .min((14. - 2. * align_x.abs()).max(1.) / (15. * horizontal))
                .min((14. - 2. * align_y.abs()).max(1.) / (15. * vertical));
            horizontal *= fit;
            vertical *= fit;
        }
        let (ww, hh) = (
            (w as f64 * horizontal).round().max(1.) as i32,
            (h as f64 * vertical).round().max(1.) as i32,
        );
        let dx = (self.profile.number("typography.map.alignX") * w as f64 / 15.).round() as i32;
        let dy = (self.profile.number("typography.map.alignY") * h as f64 / 15.).round() as i32;
        // One logical pixel of lower padding. Explicit typography offsets retain
        // their usual effect; glyph positioning never changes the cell rectangle.
        let bottom = bottom_aligned.then(|| self.native_y((y + (h + hh) / 2 + dy - 1) as f64));
        self.map_identity_alpha_placed(
            glyph,
            x + (w - ww) / 2 + dx,
            y + (h - hh) / 2 + dy,
            ww,
            hh,
            c,
            1.,
            bottom,
        )
    }
    /// Warm expected O(key bytes + clipped mask area); cold adds glyph raster + source mask work.
    fn map_identity_alpha(
        &mut self,
        glyph: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        c: Rgb,
        alpha: f64,
    ) {
        self.map_identity_alpha_placed(glyph, x, y, w, h, c, alpha, None);
    }
    fn map_identity_alpha_placed(&mut self, glyph: &str, x: i32, y: i32, w: i32, h: i32, c: Rgb, alpha: f64, ink_bottom: Option<i32>) -> Option<glyph_ink::Bounds> {
        #[cfg(test)]
        let mut _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Compound);
        if glyph.chars().count() == 1
            && glyph.is_ascii()
            && self
                .profile
                .text("typography.map.fontFamily")
                .split(',')
                .next()
                .is_some_and(|f| f.contains("Brogue CE ASCII"))
        {
            let gw = (13.09 * w as f64 / 15.).round().max(1.) as i32;
            let gh = (17. * h as f64 / 15.)
                .min(gw as f64 * 232. / 128.)
                .round()
                .max(1.) as i32;
            let gx = (x as f64 + w as f64 / 2. - gw as f64 / 2.).round() as i32;
            let gy = (y as f64 + h as f64 * 8. / 15. - gh as f64 / 2.).round() as i32;
            return self.map_glyph_placed(glyph.chars().next().unwrap(), gx, gy, gw, gh, c, alpha, false, ink_bottom);
        }
        // Fit the complete compound glyph using actual font metrics; preserve every character.
        // Rasterize a 4x device-cell outline, then integrate alpha coverage at the
        // final size. Never nearest-enlarge a small punctuation mask or discard
        // thin diagonals by point sampling. Cold O(outlines + source + output
        // pixels); only the final native mask remains in the bounded cache.
        let source_scale = (self.scale * (w.max(h) as f64 / 15.).max(1.)).max(1.) as f32
            * glyph_mask::OVERSAMPLE;
        let (nx, ny) = (self.native_x(x as f64), self.native_y(y as f64));
        let (w, h) = (
            self.native_x((x + w) as f64) - nx,
            self.native_y((y + h) as f64) - ny,
        );
        let (x, y) = (nx, ny);
        if w <= 0 || h <= 0 {
            return None;
        }
        let primary_ibm = self.profile.text("typography.map.fontFamily").split(',')
            .next().unwrap_or("").contains("IBM Plex Mono");
        let spacing=self.profile.number("typography.map.letterSpacing");
        let key=(glyph.to_string(),w,h,source_scale.to_bits(),primary_ibm,spacing.to_bits(),self.scale.to_bits());
        if !self.identity_masks.contains_key(&key) {
                #[cfg(test)]
                _paint_profile.cache_miss();
        let (dx,dy,dw,dh,output) = if glyph == "·" {
            // The ember is the authored middle dot, not a full-cell symbol.
            // Rasterize its existing outline at final device size. Cropping a
            // 2px dot and fitting it to the tile magnified it into a pixel block.
            // Cold O(outline + ink pixels), no intermediate bitmap resampling.
            let face = if primary_ibm && self.font.lookup_glyph_index('·') != 0 {
                &self.font
            } else {
                self.fallback.iter().find(|f| f.lookup_glyph_index('·') != 0)
                    .unwrap_or(&self.font)
            };
            let size = (17. * (w as f64 / 15.).min(h as f64 / 24.)) as f32;
            let (metrics, mask) = face.rasterize('·', size);
            ((w - metrics.width as i32) / 2, (h - metrics.height as i32) / 2,
                metrics.width, metrics.height, mask)
        } else {
        let size = if glyph.chars().count() > 1 {
            13.0
        } else {
            17.0
        };
        let mut glyphs = Vec::new();
        let mut advance = 0f32;
        for ch in glyph.chars() {
            let face = if primary_ibm && self.font.lookup_glyph_index(ch) != 0
            {
                &self.font
            } else {
                self.fallback
                    .iter()
                    .find(|f| f.lookup_glyph_index(ch) != 0)
                    .unwrap_or(&self.font)
            };
            let (m, b) = face.rasterize(ch, size * source_scale);
            glyphs.push((advance, m, b));
            advance += m.advance_width
                + self.profile.number("typography.map.letterSpacing") as f32 * source_scale;
        }
        let pad = (2. * source_scale).round() as i32;
        let sw = advance.ceil().max(1.) as usize + 2 * pad as usize;
        let sh = (26. * source_scale).ceil() as usize;
        let mut mask = vec![0u8; sw * sh];
        for (at, m, b) in glyphs {
            for yy in 0..m.height {
                for xx in 0..m.width {
                    let px = pad + at.round() as i32 + m.xmin + xx as i32;
                    let py =
                        (19. * source_scale).round() as i32 - m.height as i32 - m.ymin + yy as i32;
                    if px >= 0 && py >= 0 && (px as usize) < sw && (py as usize) < sh {
                        let p = py as usize * sw + px as usize;
                        mask[p] = mask[p].max(b[yy * m.width + xx]);
                    }
                }
            }
        }
        let top = (0..sh)
            .find(|yy| mask[yy * sw..(yy + 1) * sw].iter().any(|a| *a > 0))
            .unwrap_or(0);
        let bottom = (0..sh)
            .rev()
            .find(|yy| mask[yy * sw..(yy + 1) * sw].iter().any(|a| *a > 0))
            .unwrap_or(sh - 1)
            + 1;
        let left = (0..sw)
            .find(|xx| (top..bottom).any(|yy| mask[yy * sw + xx] > 0))
            .unwrap_or(0);
        let right = (0..sw)
            .rev()
            .find(|xx| (top..bottom).any(|yy| mask[yy * sw + xx] > 0))
            .unwrap_or(sw - 1)
            + 1;
        let max_h = if glyph.chars().count() > 1 {
            h * 3 / 4
        } else {
            h - (2. * self.scale).round() as i32
        };
        let scale = (w as f32 / (right - left) as f32).min(max_h as f32 / (bottom - top) as f32);
        let dw = ((right - left) as f32 * scale).round().max(1.) as i32;
        let dh = ((bottom - top) as f32 * scale).round().max(1.) as i32;
        let dx = (w - dw) / 2;
        let dy = (h - dh) / 2 - if glyph.chars().count() > 1 { h / 8 } else { 0 };
        let output = glyph_mask::coverage_resize(&mask, sw, left, top,
            right - left, bottom - top, dw as usize, dh as usize);
        (dx,dy,dw as usize,dh as usize,output)
        };
        if self.identity_masks.len()>=512 {self.identity_masks.clear();}
        let ink_bounds = glyph_ink::Bounds::from_mask(&output, dw);
        self.identity_masks.insert(key.clone(),(dx,dy,dw,dh,output,ink_bounds));
        }
        let bounds=self.native_bounds();
        let (dx,dy,dw,dh,mask,ink_bounds)=&self.identity_masks[&key];
        let y=ink_bottom.map_or(y,|bottom|bottom-*dh as i32-*dy);
        let ink_bounds = ink_bounds.map(|bounds| bounds.translated(x+dx, y+dy));
        if let Some(gpu)=self.gpu.as_mut(){gpu.mask(crate::gpu_render::MaskKey::Identity(key.0.clone(),key.1,key.2,key.3,key.4,key.5,key.6),bounds,x+dx,y+dy,*dw,*dh,mask,c,alpha,false);return ink_bounds;}
        blend_scaled_mask(&mut self.pixels,self.width,bounds,x+dx,y+dy,*dw,*dh,mask,c,alpha, &mut self.raster_stats, self.tile_clip_enabled.then_some(&self.tile_mask));
        ink_bounds
    }
    /// O(P) native bounding pixels with fixed16 coverage samples; O(1) scratch.
    /// Same authored marker geometry/colors; no actor/world/visibility queries.
    fn heartless_marker(&mut self, glyph: &str, x: i32, y: i32, w: i32, h: i32, ink: Option<(Rgb, Rgb)>) {
        let compound = glyph.chars().count() > 1;
        let (dy, radius) = if compound {
            (5.5, 0.65)
        } else if glyph == "^" {
            (0.75, 1.)
        } else if glyph == "y" {
            (0.25, 0.9)
        } else {
            (
                if glyph.chars().all(|c| c.is_ascii_lowercase()) {
                    1.
                } else {
                    -0.5
                },
                1.6,
            )
        };
        let cx = x as f64 + w as f64 / 2.;
        let cy = y as f64 + h as f64 / 2. + dy * h as f64 / 15.;
        let r = (radius * w as f64 / 15.).max(0.65);
        let center = (self.origin.0 as f64 + cx * self.scale,
            self.origin.1 as f64 + cy * self.scale);
        self.heartless_ring(x,y,w,h,center,r+0.8,(r-0.55).max(0.),ink);
    }
    fn corpse_heartless_marker(&mut self, center: (f64, f64), x: i32, y: i32, w: i32, h: i32, ink: Option<(Rgb,Rgb)>) {
        let outer = (1.3 * w as f64 / 15.).max(0.65) + 0.8;
        // Keep a readable hole even on the narrowest map tiles; a solid
        // center would read as a dot instead of the requested little o.
        self.heartless_ring(x,y,w,h,center,outer,outer*0.58,ink);
    }
    fn heartless_ring(&mut self, x: i32, y: i32, w: i32, h: i32, (nx, ny): (f64, f64), outer: f64, inner: f64, ink: Option<(Rgb,Rgb)>) {
        // Sample the authored ring at drawable resolution, not logical pixel
        // blocks. Fixed 4x4 area coverage keeps tiny holes readable at fractional DPI.
        let (rim, hole) = ink.unwrap_or(([240, 223, 190], [24, 23, 21]));
        let outer = outer * self.scale;
        let inner = inner * self.scale;
        let left = ((nx-outer).floor() as i32).max(self.native_x(x as f64));
        let right = ((nx+outer).ceil() as i32).min(self.native_x((x+w) as f64));
        let top = ((ny-outer).floor() as i32).max(self.native_y(y as f64));
        let bottom = ((ny+outer).ceil() as i32).min(self.native_y((y+h) as f64));
        for yy in top..bottom { for xx in left..right {
            let (mut covered, mut dark) = (0u32, 0u32);
            for sy in 0..4 { for sx in 0..4 {
                let dx=xx as f64+(sx as f64+0.5)/4.-nx;
                let dy=yy as f64+(sy as f64+0.5)/4.-ny;
                let d2=dx*dx+dy*dy;
                if d2<outer*outer {covered+=1;if d2<inner*inner {dark+=1;}}
            }}
            if covered>0 {
                let color=std::array::from_fn(|k| ((hole[k] as u32*dark
                    +rim[k] as u32*(covered-dark)+covered/2)/covered) as u8);
                self.native_pixel(xx,yy,color,((covered*255+8)/16) as u8);
            }
        }}
    }
    /// O(tile native pixels * 16 fixed coverage samples), O(1) scratch.
    /// The X overlays the retained creature glyph, like the heartless ring.
    /// Both renderers consume the same clipped native-pixel commands.
    fn corpse_marker(&mut self, glyph_bounds:Option<glyph_ink::Bounds>, x:i32, y:i32, w:i32, h:i32, ink:Option<(Rgb,Rgb)>) -> (f64, f64) {
        let (cross,outline)=ink.unwrap_or((CORPSE_CROSS,[24,23,21]));
        // Actual ink, after typography and device-pixel rounding. Do not re-center
        // a clipped fragment or include outlines, badges, shadows or terrain.
        let (cx,cy)=glyph_bounds.map_or_else(|| (
            self.origin.0 as f64+(x as f64+w as f64/2.)*self.scale,
            self.origin.1 as f64+(y as f64+h as f64/2.)*self.scale,
        ), glyph_ink::Bounds::center); // Preserve death indication for blank saved glyphs.
        let half=w.min(h) as f64*0.30*self.scale;
        let (left,right,top,bottom)=(cx-half,cx+half,cy-half,cy+half);
        let inner=(w as f64/15.*0.48*self.scale).max(0.2);
        let outer=inner+(0.28*self.scale).max(0.2);
        let bounds=(self.native_x(x as f64),self.native_y(y as f64),self.native_x((x+w)as f64),self.native_y((y+h)as f64));
        let distance=|px:f64,py:f64,ax:f64,ay:f64,bx:f64,by:f64| {
            let dx=bx-ax;let dy=by-ay;let t=(((px-ax)*dx+(py-ay)*dy)/(dx*dx+dy*dy).max(f64::EPSILON)).clamp(0.,1.);
            ((px-ax-t*dx).powi(2)+(py-ay-t*dy).powi(2)).sqrt()
        };
        for yy in ((top-outer).floor()as i32).max(bounds.1)..((bottom+outer).ceil()as i32).min(bounds.3) {
            for xx in ((left-outer).floor()as i32).max(bounds.0)..((right+outer).ceil()as i32).min(bounds.2) {
                let(mut covered,mut light)=(0u32,0u32);
                for sy in 0..4 {for sx in 0..4 {
                    let px=xx as f64+(sx as f64+0.5)/4.;let py=yy as f64+(sy as f64+0.5)/4.;
                    let d=distance(px,py,left,top,right,bottom).min(distance(px,py,left,bottom,right,top));
                    if d<outer {covered+=1;if d<inner {light+=1;}}
                }}
                if covered>0 {let c=std::array::from_fn(|k|((cross[k]as u32*light+outline[k]as u32*(covered-light)+covered/2)/covered)as u8);self.native_pixel(xx,yy,c,((covered*255+8)/16)as u8);}
            }
        }
        // The optional heartless ring overlays the very same native center.
        (cx, cy)
    }
    // Composite onto the already painted background, before glyphs/edges/overlays.
    // No dark backing, replacement background, or simulation/visibility reads.
    /// O(clipped native area) time, O(1) extra space; one uniform-alpha batch.
    fn path_tint(&mut self, x: i32, y: i32, w: i32, h: i32, selected: bool) {
        let alpha = if selected { 62 } else { 28 };
        self.native_fill(self.native_x(x as f64), self.native_y(y as f64),
            self.native_x((x + w) as f64), self.native_y((y + h) as f64), [255; 3], alpha);
    }
    // Sixteen cosmetic particles, rasterized at drawable resolution rather than upscaled
    // with the low-resolution pixel-art flame field. O(16 * max(1, scale)^2), O(1) extra memory.
    fn menu_embers(&mut self, frame: u32) {
        let width = self.scale.round().max(1.) as i32;
        for i in 0..16 {
            let clock = title_fire::elapsed_frames(frame) / (310. + hash(i, 3) * 140.) + hash(i, 4);
            let birth = clock.floor() as i64;
            let life = clock.fract();
            let x = f64::from(title_fire::grain(i, birth, 8)) * 52.
                + if i % 2 != 0 { 266. } else { 0. };
            let rise = life * (90. + f64::from(title_fire::grain(i, birth, 17)) * 65.);
            // Extinguish before respawning; a new ember has its own origin and height.
            let alpha = (255. * (life / 0.10).min((1. - life) / 0.16).min(1.)) as u8;
            let (x, y) = (
                self.native_x(x * 4.),
                self.native_y((179. - rise) * 800. / 180.),
            );
            let color = [[186, 62, 24], [216, 88, 29], [236, 126, 41]][i as usize % 3];
            for dy in 0..width * if i % 3 == 0 { 2 } else { 1 } {
                for dx in 0..width {
                    self.native_pixel(x + dx, y + dy, color, alpha);
                }
            }
        }
    }
    // Even-odd native scan conversion for the fixed title contours, including
    // counters. O(H*E log E + A), E <= 32 edges; O(E) fixed scratch storage.
    pub(crate) fn menu_shape(&mut self, contours: &[&[(i32, i32)]], x: i32, y: i32,
        w: i32, h: i32, color: Rgb, worn: Option<usize>) {
        let nx = self.native_x(x as f64);
        let ny = self.native_y(y as f64);
        let nw = self.native_x((x + w) as f64) - nx;
        let nh = self.native_y((y + h) as f64) - ny;
        if nw <= 0 || nh <= 0 { return; }
        for row in 0..nh {
            let sy = (row as f64 + 0.5) * 100. / nh as f64;
            let mut hits = [0f64; 32];
            let mut count = 0;
            for contour in contours {
                for i in 0..contour.len() {
                    let (ax, ay) = contour[i];
                    let (bx, by) = contour[(i + 1) % contour.len()];
                    if (ay as f64 <= sy && sy < by as f64) || (by as f64 <= sy && sy < ay as f64) {
                        hits[count] = ax as f64 + (sy - ay as f64) * (bx - ax) as f64 / (by - ay) as f64;
                        count += 1;
                    }
                }
            }
            hits[..count].sort_by(|a, b| a.total_cmp(b));
            for span in hits[..count].chunks_exact(2) {
                let start = (span[0] * nw as f64 / 100.).ceil() as i32;
                let end = (span[1] * nw as f64 / 100.).ceil() as i32;
                if worn.is_none() {
                    self.native_fill(nx+start,ny+row,nx+end,ny+row+1,color,255);
                    continue;
                }
                let mut run_start=start;
                for col in start..end {
                    let sx = col as f64 * 100. / nw as f64;
                    let cut = worn.is_some_and(|seed| {
                        let cut_y = 16. + (seed * 17 % 63) as f64;
                        (sy - (cut_y + sx * 0.035)).abs() < 0.65 && sx < 18. + (seed * 13 % 24) as f64
                            || (sy - (78. - sx * 0.07)).abs() < 0.45 && sx > 79.
                    });
                    if cut {
                        if run_start<col {self.native_fill(nx+run_start,ny+row,nx+col,ny+row+1,color,255);}
                        run_start=col+1;
                    }
                }
                if run_start<end {self.native_fill(nx+run_start,ny+row,nx+end,ny+row+1,color,255);}
            }
        }
    }
    pub fn menu(&mut self, frame: u32, selected: usize, can_continue: bool, _mode: &str) {
        // Fire and embers use elapsed menu time; the emblem keeps its own pulse.
        let fire = hellfire(frame);
        // Fill the drawable including widescreen wings, with no panel or letterbox bar.
        let left = -(self.origin.0 as f64 / self.scale).ceil() as i32;
        let top = -(self.origin.1 as f64 / self.scale).ceil() as i32;
        let wide = (self.width as f64 / self.scale).ceil() as i32;
        let high = (self.height as f64 / self.scale).ceil() as i32;
        self.picture(&fire, left, top, wide, high);
        self.menu_embers(frame);
        let bone = [238, 229, 207];
        let blood = [197, 37, 54];
        let art_w = 217;
        let art_h = (art_w as f64 * self.logo.h as f64 / self.logo.w as f64).round() as i32;
        self.beating_emblem(785, 43, art_w, art_h, frame);
        crate::menu_ui::lettering(self, "CALL OF THE", 130, 97, 252, 28, bone, false);
        crate::menu_ui::lettering(self, "CORPSE", 124, 143, 405, 95, bone, true);
        crate::menu_ui::lettering(self, "GODS", 124, 253, 358, 116, blood, true);
        for (i, item) in crate::menu_ui::ITEMS.iter().enumerate() {
            let disabled = i == 1 && !can_continue;
            let active = selected == i;
            let text_y = item.y + (item.h - item.size as i32) / 2 - 2;
            if i == 0 {
                self.rect(item.x, item.y, item.w, item.h, if active { blood } else { [156, 24, 45] });
                let width: i32 = item.label.chars().map(|ch| self.font.metrics(ch, item.size as f32).advance_width.round() as i32).sum();
                self.text(item.label, item.x + (item.w - width) / 2, text_y, item.size, bone);
                continue;
            }
            if active {
                self.rect(item.x, item.y, item.w, item.h, if disabled { [74, 70, 70] } else { [34, 23, 24] });
                if !disabled { self.rect(item.x, item.y, 3, item.h, bone); }
            }
            let text_x = item.x + 15;
            if !active {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    self.text(item.label, text_x + dx, text_y + dy, item.size, [8, 6, 9]);
                }
            }
            self.text(item.label, text_x, text_y, item.size, if disabled { [135, 119, 122] } else { bone });
        }
    }
    /// Fixed text and bounds; O(covered drawable pixels + glyph raster work).
    pub fn menu_fast_mode(&mut self, _enabled: bool, _selected: bool) {}
    /// O(seed glyphs), measured UTF-8 tail fitting the raw menu font, no character splitting.
    pub(crate) fn menu_seed_tail(&self,seed:&str)->String{
        let mut width=0;let mut tail=Vec::new();
        for ch in seed.chars().rev(){
            let face=if self.font.lookup_glyph_index(ch)!=0{&self.font}else{self.fallback.iter().find(|f|f.lookup_glyph_index(ch)!=0).unwrap_or(&self.font)};
            let next=face.metrics(ch,crate::menu_ui::SEED_SIZE as f32).advance_width.round() as i32;
            if width+next>crate::menu_ui::SEED_FIELD.2-36{break;}width+=next;tail.push(ch);
        }tail.into_iter().rev().collect()
    }
    /// O(seed glyphs + input pixels), overlay only; heart/ribs render unchanged.
    pub fn menu_seed(&mut self,seed:&str,focused:bool,busy:bool){
        let (x, y, w, h) = crate::menu_ui::SEED_FIELD;
        self.text("Seed (optional)", crate::menu_ui::START_X + 4, y + 9, 19, [199, 185, 168]);
        self.rect(x,y,w,h,[17,11,15]);
        self.border(x,y,w,h,if focused {[238,229,207]} else {[111,91,86]});
        let label=if busy {"Preparing…".to_owned()} else if seed.is_empty() {"Random seed".to_owned()} else {self.menu_seed_tail(seed)};
        self.text(&label,x+16,y+8,crate::menu_ui::SEED_SIZE,if seed.is_empty()||busy {[164,150,145]} else {[238,229,207]});
        if focused&&!busy {self.rect(x+w-13,y+10,2,21,[238,229,207]);}
    }

    pub fn import_save(&mut self, notice: &str) {
        self.clear([5, 4, 12]);
        self.text("Import save", 284, 195, 35, GOLD);
        self.text("[Esc] Main menu", 284, 340, 16, MUTED);
        self.text(&truncate(notice, 76), 284, 410, 15, [240, 130, 110]);
    }
    pub fn start(&mut self, seed: &str, _mode: &str, focus: bool, playtest: Option<(&str, bool)>, notice: &str) {
        self.clear([8, 8, 8]);
        self.prose(if playtest.is_some() { "DEBUG floor testing" } else { "Descend?" }, 284, 175, if playtest.is_some() { 38 } else { 48 }, INK, true);
        if playtest.is_some() {
            self.text("Actual prior hearts + loot; choose every earned ability. No survival simulated.", 284, 239, 15, MUTED);
            self.text("Full HP/energy. No event/summon extras. No autosave or high scores.", 284, 262, 15, MUTED);
        }
        if playtest.is_some() { self.text(&crate::debug_floor_ui::god_names(seed),284,284,14,GOLD); }
        self.text("Seed", 284, 308, 18, INK);
        self.text("Reroll", 892, 308, 16, INK);
        self.rect(284, 340, 710, 52, [16, 16, 16]);
        self.border(284, 340, 710, 52, if focus { INK } else { [75, 75, 75] });
        self.text(&truncate(seed, 62), 298, 355, 17, INK);
        self.text("Vessel", 284, 420, 18, INK);
        self.text("Avatar", 470, 427, 20, INK);
        self.text("Gods selected on the main menu", 620, 431, 14, MUTED);
        if let Some((floor, focus)) = playtest {
            self.text("Starting floor", 284, 485, 18, INK);
            self.rect(450, 475, 544, 42, [16, 16, 16]);
            self.border(450, 475, 544, 42, if focus { INK } else { [75, 75, 75] });
            self.text(&format!("{floor}  / 20"), 470, 485, 19, INK);
        }
        let button_y = if playtest.is_some() { 548 } else { 505 };
        self.rect(284, button_y, 710, 55, INK);
        self.text(if playtest.is_some() { "Reconstruct prior floors / choose abilities" } else { "Descend" }, 305, button_y + 15, 22, [8, 8, 8]);
        self.text("Back", 284, 615, 18, INK);
        self.text(&truncate(notice, 80), 284, 657, 15, [240, 130, 110]);
    }
    pub fn history(&mut self, lines: &[String], offset: usize) {
        self.rect(350, 125, 918, 579, [11, 13, 21]);
        self.border(350, 125, 918, 579, [53, 55, 68]);
        self.text("Full event history", 374, 146, 27, [255, 217, 53]);
        let start = lines.len().saturating_sub(18 + offset);
        for (i, line) in lines.iter().skip(start).take(18).enumerate() {
            self.text(
                &truncate(line, 89),
                374,
                224 + i as i32 * 24,
                16,
                crate::chronicle::color(line),
            );
        }
    }
    // H198: floor/plane mechanics remain immediate; no departure-tile tint.
    pub fn plane_crossfade(&mut self,_before:&View,_age:f64){}
    pub fn scene_crossfade(&mut self,_before:&View,_age:f64,_ascend:bool){}
    /// Existing O(map cells + effect tiles log effect tiles) projection; raster O(painted pixels).
    pub fn effects(&mut self, v: &View, cues: &[crate::effects::Cue], now: f64, reduced: bool) {
        if cues.is_empty() {
            return;
        }
        let mut known = vec![false; v.width * v.height];
        let mut direct_cells = known.clone();
        let mut display_lights = vec![None; known.len()];
        let mut actors = vec![None; known.len()];
        for c in &v.cells {
            let i = c.y * v.width + c.x;
            if i >= known.len() {
                continue;
            }
            known[i] |= c.current;
            direct_cells[i] |= c.direct;
            if c.direct { display_lights[i] = c.display_light; }
            if c.actor_id.is_some() {
                actors[i] = Some(c);
            }
        }
        let mut tiles = std::collections::BTreeMap::new();
        for cue in cues {
            if cue.event.depth != v.depth {
                continue;
            }
            if let Some(a)=cue.event.animation.as_ref().filter(|a|a.style.starts_with("moving-actor")&&a.style!=corpse_core::events::COMMANDMENT_SLIDE) {
                if cues.iter().any(|next|next.started>cue.started&&next.started<=now&&next.event.animation.as_ref().is_some_and(|b|b.action==a.action&&b.style.starts_with("moving-actor"))){continue;}
            }
            for tile in crate::effects::tiles(cue, now, reduced) {
                if tile.x < 0 || tile.y < 0 || tile.x >= v.width as i32 || tile.y >= v.height as i32
                {
                    continue;
                }
                let i = tile.y as usize * v.width + tile.x as usize;
                if !known[i] && !cue.event.animation.as_ref().is_some_and(|a|a.surface.as_deref()==Some("audible")) {
                    continue;
                }
                if tiles
                    .get(&i)
                    .is_none_or(|old: &crate::effects::Tile| tile.light >= old.light)
                {
                    tiles.insert(i, tile);
                }
            }
        }
        let (cw, ch, ox, oy) = self.map_layout(v.width, v.height);
        for cue in cues {if cue.event.depth==v.depth {if let Some(a)=cue.event.animation.as_ref().filter(|a|a.style=="thread-grow") {
            let age=now-cue.started;
            if age>=0.&&age<crate::effects::duration(&cue.event) {
                let count=if reduced{a.cells.len()}else{(age/32.).floor() as usize+1};
                let cells=a.cells.iter().take(count).filter(|c|c.x>=0.&&c.y>=0.&&c.x<v.width as f64&&c.y<v.height as f64&&known[c.y as usize*v.width+c.x as usize]).map(|c|(c.x as usize,c.y as usize)).collect::<Vec<_>>();
                let color=if a.proposal=="P08"{[255,211,64]}else{[255,48,64]};
                crate::persistent_ability_overlays::paint_thread(self,(cw,ch,ox,oy),&cells,color,110);
                // The new end is visible immediately at a portal exit, even
                // before the next cell gives the thread a line segment there.
                if !reduced {
                    if let Some(&(x,y))=cells.last() {
                        self.blend_rect(ox+x as i32*cw+cw/2-1,oy+y as i32*ch+ch/2-1,2,2,color,110);
                    }
                }
            }
        }}}
        // Suppress every previous step's snapshot glyph until the push completes.
        let moving_actions:std::collections::BTreeSet<_>=cues.iter().filter_map(|c|{
            let a=c.event.animation.as_ref()?;
            (a.style.starts_with("moving-actor")&&a.style!=corpse_core::events::COMMANDMENT_SLIDE&&now>=c.started&&now<c.started+crate::effects::duration(&c.event)).then_some(a.action)
        }).collect();
        for cue in cues {if let Some(a)=cue.event.animation.as_ref().filter(|a|a.style.starts_with("moving-actor")&&a.style!=corpse_core::events::COMMANDMENT_SLIDE&&moving_actions.contains(&a.action)) {
            if let Some(origin)=a.cells.first() {if origin.x>=0.&&origin.y>=0.&&origin.x<v.width as f64&&origin.y<v.height as f64 {
                let i=origin.y as usize*v.width+origin.x as usize;
                if let Some(Some(cell))=actors.get(i) {self.rect(ox+origin.x as i32*cw,oy+origin.y as i32*ch,cw,ch,cell.bg);actors[i]=None;}
            }}
        }}
        for tile in tiles.values() {
            let x = ox + tile.x * cw;
            let y = oy + tile.y * ch;
            let index = tile.y as usize * v.width + tile.x as usize;
            let direct = direct_cells[index];
            let display = display_lights[index];
            let field = tile.color.map(|c| (c as f64 * tile.field_strength).round() as u8);
            let field = if tile.field_strength>=1. {field} else if direct {
                display.map_or(field,|light|light.apply(field))
            } else {
                crate::color::telepathic(field, true)
            };
            if tile.glyph.is_none() { self.blend_rect(x,y,cw,ch,field,(tile.light*255.).round() as u8); }
            let actor = actors[index];
            if let Some(c) = actor {
                let glyph_bounds=self.map_identity_aligned(&c.glyph, x, y, cw, ch, c.fg, false);
                let marker = if v.dreamwalking && !c.dream_entity { Some((c.fg,c.bg)) }
                    else { display.map(|light| (light.foreground(if c.corpse {CORPSE_CROSS} else {[240,223,190]},true),light.apply([24,23,21]))) };
                if c.corpse {
                    let center = self.corpse_marker(glyph_bounds,x,y,cw,ch,marker);
                    if c.heartless {
                        let ring = if v.dreamwalking && !c.dream_entity { Some((c.fg,c.bg)) }
                            else { display.map(|light| (light.foreground([240,223,190],true),light.apply([24,23,21]))) };
                        self.corpse_heartless_marker(center,x,y,cw,ch,ring);
                    }
                } else if c.heartless {
                    self.heartless_marker(&c.glyph, x, y, cw, ch,marker);
                }
                self.text(
                    &c.superscript,
                    self.superscript_x(&c.superscript, x, cw, ch),
                    y - 2,
                    (ch / 2).max(7) as u16,
                    crate::illumination_ui::paint_color([255, 224, 160], display),
                );
                self.number_of_blood_superscript(c,x,y,cw,ch,c.fg);
            }
            // A traveling glyph is the only foreground animation primitive.
            // Local feedback preserves the entity's canonical identity above light.
            if let Some(g) = &tile.glyph {
                let color = if tile.field_strength>=1. {tile.color} else if direct { crate::illumination_ui::paint_color(tile.color, display) }
                    else { crate::color::telepathic(tile.color, false) };
                self.map_identity_alpha(g, x, y, cw, ch, color, tile.light);
            }
        }
        // Sequence has already removed this actor's original layer; unlike
        // tile-based effects, drawing the glyph never repaints its background.
        if !reduced {
            for cue in cues.iter().filter(|c| !c.static_marks && c.event.depth==v.depth) {
                let Some((x,y))=crate::divine_animation_shapes::commandment_pose(&cue.event,now-cue.started) else {continue};
                if x<0. || y<0. || x>=v.width as f64 || y>=v.height as f64 {continue;}
                let Some(glyph)=cue.event.glyph.as_deref() else {continue};
                let ink=cue.event.color.as_deref().map_or(INK,crate::color::hex);
                let index=y.round() as usize*v.width+x.round() as usize;
                let ink=display_lights.get(index).copied().flatten().map_or(ink,|light|light.foreground(ink,true));
                self.map_identity(glyph,ox+(x*cw as f64).round() as i32,oy+(y*ch as f64).round() as i32,cw,ch,ink);
            }
        }
    }
    /// O(viewport pixels + bounded offers/text raster work); O(1) dimmer temporary space.
    pub fn blessing_choices(&mut self, offers: &[crate::ability_ui::AbilityView], selected: usize) {self.centered_sheet(|p|p.blessing_choices_sheet(offers,selected));}
    fn blessing_choices_sheet(&mut self, offers: &[crate::ability_ui::AbilityView], selected: usize) {
        // O(native viewport area), O(1) space: adjacent logical pixel footprints tile exactly.
        self.native_fill(0,0,self.width as i32,self.height as i32,[5,4,12],232);
        self.rect(345, 70, 590, 660, [11, 13, 21]);
        self.border(345, 70, 590, 660, [53, 55, 68]);
        for (i, a) in offers.iter().take(3).enumerate() {
            let y = 180 + i as i32 * 170;
            self.rect(
                373,
                y,
                534,
                158,
                if i == selected {
                    [36, 41, 53]
                } else {
                    [11, 13, 21]
                },
            );
            self.border(373, y, 534, 158, [53, 55, 68]);
            self.rect(373, y, 3, 158, a.color);
            self.text(&truncate(&a.name, 41), 389, y + 12, 20, a.color);
            let mut line = String::new();
            let mut lines = Vec::new();
            for word in a.description.split_whitespace() {
                if line.chars().count() + word.chars().count() + 1 > 54 {
                    lines.push(line);
                    line = String::new();
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            lines.push(line);
            self.clip = Some((389, y + 42, 501, 106));
            for (j, line) in lines.iter().enumerate() {
                self.text(line, 389, y + 42 + j as i32 * 21, 15, MUTED);
            }
            self.clip = None;
        }
    }
    pub fn hazard_prompt(&mut self, message: &str) {self.centered_sheet(|p|p.hazard_prompt_sheet(message));}
    fn hazard_prompt_sheet(&mut self, message: &str) {
        self.rect(310, 290, 660, 220, [11, 13, 21]);
        self.border(310, 290, 660, 220, [255, 217, 53]);
        let mut line = String::new();
        let mut lines = Vec::new();
        for word in message.split_whitespace() {
            if line.len() + word.len() > 58 {
                lines.push(line);
                line = String::new();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
        for (i, line) in lines.iter().enumerate() {
            self.text(line, 334, 316 + i as i32 * 27, 18, INK);
        }
        self.text("[Y] Yes    [N] No", 334, 468, 18, GOLD);
    }
    pub fn ability_detail(&mut self, a: &crate::ability_ui::AbilityView, offset: usize) {
        ability_dialog::paint(self, a, offset);
    }
    pub fn ability_detail_damage(&self, a: &crate::ability_ui::AbilityView) -> Option<damage::Rect> {
        ability_dialog::bounds(self, a)
    }
    pub fn inspection_notice_y(&self) -> i32 {
        {let r=self.hud_tracks().inspector; r.1+(r.3-13)/2}
    }
    pub fn hud_damage(&self)->impl Iterator<Item=damage::Rect>+'_ {
        let t=self.hud_tracks();
        [t.resources,t.nav,t.depth,t.stats,t.gods,t.rings,t.hearts,t.potions,t.divine,t.items].into_iter().filter_map(|r|
            self.logical_damage_rect(r.0,r.1,r.2,r.3))
    }
    pub fn footer_damage(&self) -> Option<damage::Rect> {
        let top = self.frame().map.1+self.frame().map.3;
        self.logical_damage_rect(0,top,self.logical_width(),self.logical_height()-top)
    }
    /// O(T log T) worst-case fitting, O(T) temporary strings; no truncation if both fit.
    /// Read-only anchor at the already displayed Quick Hand reaction cost.
    pub fn reaction_action_anchor(&self,v:&View,style:&str)->(i32,i32,i32,i32){
        if let Some((i,a))=v.abilities.iter().take(self.ability_capacity()).enumerate().find(|(_,a)|a.id==if style=="reaction-spent-missed-me"{"isheilah-missed-me"}else{"isheilah-quick-hand"}) {
            let left=self.hud_left();let rail=self.logical_width()-left-24;
            let _=a;
            let x=left+12+rail-66;
            return (x,self.game_ability_top()+i as i32*self.game_ability_row()+34,14,16)
        }
        (self.hud_left()+12,self.hud_tracks().divine.1+2,20,20)
    }
    fn ability_metadata(&self,state:&str,cost:&str,left:i32,right:i32)->(String,String,i32){
        let role=crate::typography_ui::Role::Secondary;
        let gap=if state.is_empty()||cost.is_empty(){0}else{8};
        let total=(right-left).max(0);
        let sw=self.measure_text_role(role,state,10).ceil()as i32;
        let state=self.fit_text_role(role,state,sw.min(total),10);
        let sw=self.measure_text_role(role,&state,10).ceil()as i32;
        let cost=self.fit_text_role(role,cost,(total-sw-gap).max(0),10);
        let cw=self.measure_text_role(role,&cost,10).ceil()as i32;
        (state,cost,right-cw)
    }
    pub fn condition_meter(&mut self, m: &crate::statuses_ui::Meter, x: i32, y: i32, w: i32) {
        self.condition_meter_tinted(m,x,y,w,[193,138,255]);
    }
    pub fn condition_meter_tinted(&mut self, m: &crate::statuses_ui::Meter, x: i32, y: i32, w: i32, tint:Rgb) {
        let label = self.condition_meter_label(m,w);
        self.text_role(
            crate::typography_ui::Role::Secondary,
            &label,
            x,
            y,
            13,
            tint,
        );
        if m.duration.is_some() {
            self.rect(x, y + 18, w, 3, [36, 41, 53]);
            self.rect(
                x,
                y + 18,
                (w as f64 * m.percent / 100.).clamp(0., w as f64) as i32,
                3,
                tint,
            );
            if !m.reference_known {
                for dx in (0..w).step_by(5) {
                    self.rect(x + dx, y + 17, 2, 1, tint);
                }
            }
        }
    }
    fn condition_meter_label(&self,m:&crate::statuses_ui::Meter,w:i32)->String {
        let name=crate::statuses_ui::display_name(&m.name);
        let suffix=m.duration.map_or_else(String::new,|n|format!(" {n}"));
        let role=crate::typography_ui::Role::Secondary;
        let available=(w-self.measure_text_role(role,&suffix,13).ceil() as i32).max(0);
        format!("{}{suffix}",self.fit_text_role(role,name,available,13))
    }
    pub fn creature_summary(&mut self, a: &Creature, x: i32, y: i32, w: i32, compact: bool) {
        self.text_role(
            crate::typography_ui::Role::Secondary,
            &truncate(
                &format!(
                    "{} {}",
                    a.glyph, a.name
                ),
                if compact { 33 } else { 68 },
            ),
            x,
            y,
            if compact { 12 } else { 18 },
            a.color,
        );
        let bar_y = y + if compact { 22 } else { 32 };
        if a.shared_resources {
            return;
        }
        let half = if a.uses_energy { (w - 10) / 2 } else { w };
        for (i, (name, value, max, color)) in [
            ("Health", a.hp, a.max_hp, [255, 48, 85]),
            ("Energy", a.energy, a.max_energy, [36, 223, 255]),
        ]
        .into_iter()
        .take(if a.uses_energy { 2 } else { 1 })
        .enumerate()
        {
            let xx = x + i as i32 * (half + 10);
            if i == 0 && a.invulnerable {
                self.text_role(
                    crate::typography_ui::Role::Secondary,
                    "Health · Invulnerable",
                    xx,
                    bar_y,
                    12,
                    MUTED,
                );
                continue;
            }
            self.text_role(
                crate::typography_ui::Role::Secondary,
                &format!("{name} {:.0}/{}", if name == "Health" {crate::stat_presentation::current_health(value)} else {value}, resource_max_label(max)),
                xx,
                bar_y,
                12,
                INK,
            );
            self.rect(xx, bar_y + 17, half, 4, [36, 41, 53]);
            self.rect(
                xx,
                bar_y + 17,
                (half as f64
                    * (value / if max.is_infinite() { 100. } else { max.max(1.) }).clamp(0., 1.))
                    as i32,
                4,
                color,
            );
        }
    }
    /// Shared measured wrapping keeps dialog painting and selection aligned.
    fn creature_condition_layout(&self, a: &Creature) -> (Vec<String>, i32, i32) {
        let role = crate::typography_ui::Role::Secondary;
        let text = crate::statuses_ui::summary(&a.statuses);
        let lines = if text.is_empty() { Vec::new() } else { self.wrap_text_role(role, &text, 588, 13) };
        let style = crate::typography_ui::resolve(&self.profile, role,
            crate::typography_ui::RoleStyle::baseline(13.));
        let step = (style.font_size * style.line_height).ceil().max(15.) as i32;
        let height = 76 + lines.len() as i32 * step;
        (lines, step, height)
    }
    pub fn creatures(&mut self, view: &View, selected: Option<f64>, offset: usize) {self.centered_sheet(|p|p.creatures_sheet(view,selected,offset));}
    fn creatures_sheet(&mut self, view: &View, selected: Option<f64>, offset: usize) {
        self.rect(337, 125, 636, 579, [11, 13, 21]);
        self.border(337, 125, 636, 579, [53, 55, 68]);
        self.text("Creatures", 361, 146, 28, GOLD);
        self.clip = Some((349, 190, 612, 466));
        let mut y = 196;
        for a in view.creatures.iter().skip(offset) {
            let (lines, step, h) = self.creature_condition_layout(a);
            if selected == Some(a.id) {
                self.rect(349, y - 4, 612, h - 6, [28, 32, 46]);
            }
            self.creature_summary(a, 361, y, 588, false);
            for (i, line) in lines.iter().enumerate() {
                self.text_role(crate::typography_ui::Role::Secondary, line,
                    361, y + 58 + i as i32 * step, 13, [193, 138, 255]);
            }
            y += h;
            if y >= 656 {
                break;
            }
        }
        if view.creatures.is_empty() {
            self.text("No creatures in sight", 361, 204, 18, MUTED);
        }
        self.clip = None;

    }
    pub fn game(&mut self, v: &View, notice: &str, inventory: bool) {
        self.game_at(v, notice, inventory, 0.);
    }
    fn interaction_mark(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        route: bool,
        destination: bool,
        target: bool,
    ) {
        let marks: &[(f64, f64, f64, f64)] = if route && target {
            &[(0., 6., 2., 3.), (13., 6., 2., 3.)]
        } else if route {
            &[
                (1., 1., 13., 1.),
                (1., 13., 13., 1.),
                (1., 2., 1., 11.),
                (13., 2., 1., 11.),
            ]
        } else {
            &[
                (0., 0., 4., 1.),
                (0., 0., 1., 4.),
                (11., 0., 4., 1.),
                (14., 0., 1., 4.),
                (0., 14., 4., 1.),
                (0., 11., 1., 4.),
                (11., 14., 4., 1.),
                (14., 11., 1., 4.),
            ]
        };
        // Movement uses the cell tint alone; targeting retains distinct warm side ticks.
        if route && !target {
            return;
        }
        let fg = if target {
            [230, 155, 131]
        } else {
            [238, 238, 238]
        };
        for &(dx, dy, ww, hh) in marks {
            self.rect(
                x + (dx * w as f64 / 15.).round() as i32,
                y + (dy * h as f64 / 15.).round() as i32,
                (ww * w as f64 / 15.).round().max(1.) as i32,
                (hh * h as f64 / 15.).round().max(1.) as i32,
                fg,
            );
        }
        if destination {
            self.rect(x + w / 3, y + h - 1, (w / 3).max(1), 1, fg);
        }
    }
    pub fn map_layout(&self, w: usize, h: usize) -> (i32, i32, i32, i32) {
        map_layout_frame(w,h,&self.profile,self.frame())
    }
    /// Existing per-cell/layout/projection work plus O(P) raster; fade O(H*(256 + row width)).
    pub fn game_at(&mut self, v: &View, notice: &str, _inventory: bool, now: f64) {
        #[cfg(test)]
        {self.pack_occlusion_stats=pack_occlusion::Stats::default();}
        #[cfg(test)]
        let _paint_profile = paint_profile_bench::span(paint_profile_bench::Kind::Game);
    let _timing = corpse_core::timing::span(if self.gpu.is_some(){"desktop.gpu.commands"}else{"desktop.software_render"});
        #[cfg(test)]
        let _paint_background = paint_profile_bench::span(paint_profile_bench::Kind::Background);
        self.set_responsive_canvas(true);
        let frame = self.frame();
        let hud=self.hud_tracks();
        let map_top = frame.map.1;
        self.clear_with_rect(
            game_color(&self.profile, "ui.colors.background"),
            (0, map_top,
                self.frame().map.2,
                (self.frame().map.1 + self.frame().map.3 - map_top).max(0)),
            crate::illumination_ui::unexplored(&self.profile),
        );
        #[cfg(test)]
        drop(_paint_background);
        // Reference composition: top Chronicle, entire floor left, resource/ability rail right.

        let hovered_enemy=self.enemy_hover.and_then(|id|v.cells.iter()
            .find(|c|c.actor_id.is_some_and(|actor|actor.to_bits()==id)).map(|c|(c.x,c.y)));
        let left = self.hud_left();
        let rail_width = self.logical_width() - left - 24;
        let map_right=self.frame().map.2;
        let panel_bottom=self.frame().map.1+self.frame().map.3;
        let rail_bottom=frame.rail_bottom;
        // Measured wrapping is O(L²) worst-case metric work for bounded recent log text.
        // Raster fade is O(A) over the native chronicle rectangle, with no copied frame.
        if self.intersects_damage(0,0,hud.chronicle.2,hud.chronicle.3) {
        crate::game_hud::panel(self,hud.chronicle);
        #[cfg(test)]
        let _paint_chronicle = paint_profile_bench::span(paint_profile_bench::Kind::Chronicle);
        let mut lines = Vec::new();
        let size = crate::modal_ui::changed_number(&self.profile,"typography.chronicle.fontSize",14.).round() as u16;
        let line_height = (size as f64 * self.profile.number("typography.chronicle.lineHeight"))
            .round().max(1.) as i32;
        for group in &v.log_groups {
            let message = if group.count > 1 {
                format!("{}  ×{}", group.message, group.count)
            } else { group.message.clone() };
            for line in self.wrap_text_role(crate::typography_ui::Role::Chronicle,
                &message, hud.chronicle.2 - 24, size) {
                lines.push((line, crate::chronicle::color(&group.message)));
            }
        }
        let end = lines.len().saturating_sub(v.chronicle_offset.min(lines.len().saturating_sub(4)));
        let height = hud.chronicle.3-16;
        let start = end.saturating_sub(((height + line_height - 1) / line_height) as usize);
        let bottom = 8 + height;
        self.clip = Some((12, 8, hud.chronicle.2 - 24, height));
        for (i, (line, color)) in lines[start..end].iter().enumerate() {
            self.text_role(crate::typography_ui::Role::Chronicle, line, 12,
                bottom - (end - start - i) as i32 * line_height, size,
                if *color == INK { game_color(&self.profile, "ui.colors.chronicle") } else { *color });
        }
        self.clip = None;
        let background = [14,13,22];
        let (left_px, right_px) = (self.native_x(12.).max(0), self.native_x((hud.chronicle.2 - 12) as f64).min(self.width as i32));
        let (top_px, bottom_px) = (self.native_y(8.).max(0), self.native_y(bottom as f64).min(self.height as i32));
        self.paint_chronicle_fade(background,(left_px,top_px,right_px,bottom_px));
        if self.chronicle_hover {
            let r=self.chronicle_hit_rect();
            self.border(r.0,r.1,r.2,r.3,[174,158,107]);
        }
        } // Chronicle damage section
        let (cw, ch, ox, oy) = self.map_layout(v.width, v.height);
        let map_damage=self.paint_cell_indices.as_ref().is_some_and(|v|!v.is_empty()) || self.intersects_damage(ox-4,oy-4,cw*v.width as i32+8,ch*v.height as i32+ch+8);
        #[cfg(test)]
        let _paint_terrain = paint_profile_bench::span(paint_profile_bench::Kind::TerrainPreparation);
        // The staged cache changes only at fire departures, item arrivals, or a new observation.
        // Arc clones keep per-frame preparation O(1); actor layers stay current.
        let staged_cells=self.firebolt_terrain.as_ref().map(|stage|stage.cells.clone());
        let source_cells=staged_cells.as_deref().unwrap_or(&v.cells);
        let terrain_snapshot=self.firebolt_terrain.as_ref().map(|stage|stage.snapshot.clone())
            .or_else(||v.terrain_snapshot.clone());
        let animated_cells = if map_damage && (crate::appearance_light::animated(&self.profile)
            || crate::appearance_material::animated(&self.profile))
        {
            terrain_snapshot.as_ref().map(|observation| {
                let (mut cells, backgrounds, lights) =
                    crate::observe::terrain_frame_baseline(observation, &self.profile, v.cosmetic_seed, now);
                // Countdown is observed once per accepted turn, never recomputed
                // from render time or from hidden live world state.
                for (cell, original) in cells.iter_mut().zip(source_cells.iter().take(v.terrain_count)) {
                    let mut original = original.clone();
                    crate::illumination_ui::restore_source_ink(&mut original);
                    let original = &original;
                    cell.subscript = original.subscript.clone();
                    cell.timeweave = original.timeweave;
                    cell.duel_edges = original.duel_edges;
                    if original.non_euclidean {
                        crate::non_euclidean_ui::restore_marker(cell, original);
                    }
                    if original.italic {
                        cell.italic = true;
                        cell.glyph = original.glyph.clone();
                        cell.label = original.label.clone();
                        cell.fg = original.fg;
                    }
                    crate::cogworks_ui::restore_marker(cell, original);
                }
                let mut material = vec![[0, 0, 0]; v.width * v.height];
                for c in &cells {
                    if let Some(bg) = c.material_background {
                        material[c.y * v.width + c.x] = bg;
                    }
                }
                for original in source_cells.iter().skip(v.terrain_count) {
                    let mut cell = original.clone();
                    crate::hallucination_ui::map_cell(&mut cell, crate::illumination_ui::restore_source_ink);
                    let i = cell.y * v.width + cell.x;
                    if let Some(bg) = backgrounds.get(i) {
                        cell.bg = *bg;
                    }
                    if cell.fire {
                        cell = crate::observe::fire_cell(
                            &self.profile,
                            &observation.cells[i],
                            observation.all_seeing,
                            lights[i],
                            material[i],
                            cell.bg,
                            v.width,
                            v.cosmetic_seed,
                            v.fire_seed,
                            now,
                        );
                    }
                    if let Some(observed) = observation.cells.get(i) {
                        crate::hallucination_ui::map_cell(&mut cell, |cell| {
                            cell.bg = backgrounds[i];
                            if let Some(ink) = cell.entity_ink {
                                cell.fg = crate::observe::entity_ink(
                                    &self.profile, observed, observation.all_seeing,
                                    lights[i], material[i], cell.bg, ink);
                            }
                        });
                    }
                    cells.push(cell);
                }
                crate::illumination_ui::finish_cells(&mut cells, &lights, v.width, &self.profile);
                cells
            })
        } else {
            None
        };
        #[cfg(test)]
        drop(_paint_terrain);
        let painted_cells = animated_cells.as_ref().unwrap_or(source_cells);
        #[cfg(test)]
        let _paint_dream = paint_profile_bench::span(paint_profile_bench::Kind::DreamPreparation);
        // O(C + G) transform/copy of detached cells, including G glyph/label bytes;
        // no terrain queries or simulation RNG. Cosmetic time only changes blue ink.
        let dream_cells = (map_damage && v.dreamwalking).then(|| painted_cells.iter().cloned().map(|mut cell| {
            crate::dream_ui::override_cell(&mut cell, now, v.cosmetic_seed);
            cell
        }).collect::<Vec<_>>());
        #[cfg(test)]
        drop(_paint_dream);
        let painted_cells = dream_cells.as_ref().unwrap_or(painted_cells);
        #[cfg(test)]
        let _paint_map = paint_profile_bench::span(paint_profile_bench::Kind::Map);
        // Expected O(P + T + C) membership work; tint raster work is bounded by painted cell area.
        let path: std::collections::HashSet<_> = v.path.iter().copied().collect();
        let target_path: std::collections::HashSet<_> = v.target_path.iter().copied().collect();
        let target_area: std::collections::HashSet<_> = v.target_area.iter().copied().collect();
        let native_contrast = self.profile.value("fog") == crate::appearance_profile::Profile::defaults().value("fog")
            && self.profile.value("global") == crate::appearance_profile::Profile::defaults().value("global");
        let indices=self.paint_cell_indices.take();
        let cell_indices:Box<dyn Iterator<Item=usize>>=if let Some(indices)=indices{Box::new(indices.into_iter())}else{Box::new(0..painted_cells.len())};
        let mut blood_counter=None;
        let mut razorleaves=Vec::new();
        for index in cell_indices {
            let Some(cell)=painted_cells.get(index) else{continue;};
            if self.raster_stats.enabled{self.raster_stats.cells_considered+=1;}
            // Full painting has no damage scissor: do not compute retained-only
            // glyph/selection footprints. Raster primitives still clip to the frame.
            if self.tile_clip_enabled && self.tile_cell_bounds.get(index).copied().flatten().is_none_or(|r|!self.tile_mask.intersects(r)){continue;}
            if let Some(d)=self.damage_clip {
                if self.cell_damage_rect(cell,v.width,v.height).is_none_or(|r|d.intersect(r).is_none()){continue;}
            }
            let x = ox + cell.x as i32 * cw;
            let y = oy + cell.y as i32 * ch;
            if self.pack_hides_plain_cell(cell,x,y,cw,ch) {continue;}
            if self.raster_stats.enabled{self.raster_stats.cells_painted+=1;}
            let (cell_fg, cell_bg) = if native_contrast && !v.dreamwalking { readable_cell_colors(cell) } else { (cell.fg, cell.bg) };
            self.rect(x, y, cw, ch, cell_bg);
            if let Some((color, alpha)) = cell.background_overlay {
                let inset_x = (cw as f64 / 15.).round().max(1.) as i32;
                let inset_y = (ch as f64 / 15.).round().max(1.) as i32;
                self.rect(
                    x + inset_x,
                    y + inset_y,
                    (cw - 2 * inset_x).max(1),
                    (ch - 2 * inset_y).max(1),
                    crate::color::composite(cell_bg, color, alpha),
                );
            }
            let point = (cell.x, cell.y);
            if path.contains(&point) || target_path.contains(&point) || target_area.contains(&point) || v.target_impact==Some(point) || v.hover == Some(point) || (hovered_enemy == Some(point) || self.ground_hover == Some(point)) {
                let selected = v.hover == Some(point) || (hovered_enemy == Some(point) || self.ground_hover == Some(point))
                    || (v.target_path.is_empty() && v.path.last() == Some(&point));
                if target_area.contains(&point) || v.target_impact==Some(point) {
                    self.native_fill(self.native_x(x as f64),self.native_y(y as f64),
                        self.native_x((x+cw) as f64),self.native_y((y+ch) as f64),[244,127,101],if v.target_impact==Some(point){94}else{48});
                } else if let Some(color)=v.target_color.filter(|_|target_path.contains(&point)) {
                    self.native_fill(self.native_x(x as f64),self.native_y(y as f64),
                        self.native_x((x+cw) as f64),self.native_y((y+ch) as f64),color,if selected {80}else{52});
                } else { self.path_tint(x, y, cw, ch, selected); }
            }
            for (direction, edge) in cell.edges.iter().enumerate() {
                if let Some(c) = edge {
                    match direction {
                        0 => self.rect(x + cw - 1, y, 1, ch, *c),
                        1 => self.rect(x, y + ch - 1, cw, 1, *c),
                        2 => self.rect(x, y, 1, ch, *c),
                        _ => self.rect(x, y, cw, 1, *c),
                    }
                }
            }
            for (direction,edge) in cell.duel_edges.iter().enumerate(){if *edge{
                let ink=crate::illumination_ui::paint_color([235,196,72],cell.display_light);
                match direction {0=>self.rect(x+cw-1,y,1,ch,ink),1=>self.rect(x,y+ch-1,cw,1,ink),2=>self.rect(x,y,1,ch,ink),_=>self.rect(x,y,cw,1,ink)}
            }}
            for (glyph, color, opacity) in &cell.underlays {
                self.map_identity(
                    glyph,
                    x,
                    y,
                    cw,
                    ch,
                    crate::color::composite(cell_bg, *color, *opacity),
                );
            }
            if let Some(direction) = cell.timeweave {
                self.timeweave_arrow_ink(x, y, cw, ch, direction, crate::illumination_ui::paint_color([54,211,193],cell.display_light));
            }
            let mut ink = cell_fg;
            for animation in &cell.animations {
                match *animation {
                    Animation::Fire { phase,speed,colors } =>
                        ink=crate::appearance_material::fire_ink(colors,phase,speed,now),
                    Animation::WardPulse { speed } if now > 0. => {
                        // Pulse the ward's glyph, not the underlying tile or actors.
                        let wave = (now * speed * std::f64::consts::TAU / 1200.).cos();
                        ink = crate::color::composite(cell_bg, cell_fg, 0.76 + 0.24 * wave);
                    }
                    _ => {}
                }
            }
            if cell.fire_flicker && now > 0. {
                ink=crate::color_profile::grade(ink,crate::entity_profile::fire_flicker(now,cell.x as i32,cell.y as i32),1.,1.);
            }
            if cell.pulse_speed != 0. && now > 0. {
                ink = crate::color_profile::grade(
                    ink,
                    0.85 + 0.15
                        * (now * cell.pulse_speed / 500.
                            + crate::color::cosmetic_hash(cell.x as i32, cell.y as i32, 0)
                                * std::f64::consts::TAU)
                            .sin(),
                    1.,
                    1.,
                );
            }
            let emphasis = if cell.emphasis > 0. {
                cell.emphasis
            } else {
                1.
            };
            let (gw, gh) = (
                (cw as f64 * emphasis).round().max(1.) as i32,
                (ch as f64 * emphasis).round().max(1.) as i32,
            );
            if cell.entity_ink.is_some_and(|entity| entity.player) {
                // Preserve the terrain, light and features beneath the player.
                ink = crate::color::distinguish(ink, cell_bg, 7.);
            }
            if !v.dreamwalking {
                if let Some(outline)=cell.entity_ink.and_then(|entity|entity.identity_outline) {
                    self.map_identity_outline(&cell.glyph,x+(cw-gw)/2,y+(ch-gh)/2,gw,gh,crate::illumination_ui::paint_color(outline,cell.display_light));
                }
            }
            let water_phase=cell.animations.iter().find_map(|a|if let Animation::Water{phase,speed}=a {Some((*phase,*speed))}else{None});
            let glyph_bounds = if cell.glyph=="~" && now>0. && water_phase.is_some() {
                let(phase,speed)=water_phase.unwrap();
                self.water_wave(x+(cw-gw)/2,y+(ch-gh)/2,gw,gh,ink,now*speed/900.+phase);
                None
            } else if cell.italic && cell.glyph == "0" {
                self.map_glyph_placed('0', x + (cw - gw) / 2 + gw / 8,
                    y + (ch - gh) / 2, (gw * 3 / 4).max(1), gh, ink, 1., true, None)
            } else if cell.non_euclidean && cell.bottom_aligned && cell.glyph.parse::<usize>().is_ok() {
                // A small ground marker, not an actor-sized identity. Keep its baseline
                // at the same lower tile padding as grass, including multi-digit links.
                self.ground_number(&cell.glyph,x,y,cw,ch,gw,gh,ink)
            } else {
                self.map_identity_aligned(&cell.glyph, x + (cw - gw) / 2,
                    y + (ch - gh) / 2, gw, gh, ink,
                    cell.bottom_aligned && cell.glyph == "\"")
            };
            for animation in &cell.animations {
                if let Animation::Razorleaves { phase, speed, color, opacity } = *animation {
                    razorleaves.push(((x,y,cw,ch),phase,now*speed,color,opacity));
                }
            }
            if now > 0. {
                for animation in &cell.animations {
                    match *animation {
                        Animation::Water { .. } | Animation::Razorleaves { .. } | Animation::WardPulse { .. } => {}
                        Animation::Gas { phase, color } => {
                            let pulse = 0.5 + 0.5 * (now / 1900. + phase).sin();
                            self.blend_rect(x,y,cw,ch,color,((0.012+pulse*0.018)*255.) as u8);
                        }
                        Animation::Reflection { phase, color } => {
                            let pulse = 0.5
                                + 0.5
                                    * (now / 670. + phase).sin()
                                    * (now / 1300. + phase * 0.3).sin();
                            self.blend_rect(x,y,cw,ch,color,((0.008+pulse*0.018)*255.) as u8);
                        }
                        Animation::Torch => {
                            let p = 0.5
                                + 0.5
                                    * (now / 300. + cell.x as f64 * 3. + cell.y as f64 * 7.).sin();
                            let c = crate::illumination_ui::paint_color(crate::color::composite([255, 118, 46], [255, 211, 74], p),cell.display_light);
                            self.map_identity(&cell.glyph, x, y, cw, ch, c);
                        }
                        Animation::Fire { .. } => {}
                    }
                }
            }
            let marker_ink = if v.dreamwalking && !cell.dream_entity { Some((cell.fg,cell.bg)) }
                else { cell.display_light.map(|light| (light.foreground(if cell.corpse {CORPSE_CROSS} else {[240,223,190]},true),light.apply([24,23,21]))) };
            if cell.corpse {
                let center = self.corpse_marker(glyph_bounds,x,y,cw,ch,marker_ink);
                if cell.heartless {
                    let ring = if v.dreamwalking && !cell.dream_entity { Some((cell.fg,cell.bg)) }
                        else { cell.display_light.map(|light| (light.foreground([240,223,190],true),light.apply([24,23,21]))) };
                    self.corpse_heartless_marker(center,x,y,cw,ch,ring);
                }
            } else if cell.heartless {
                self.heartless_marker(&cell.glyph, x, y, cw, ch,marker_ink);
            }
            if !cell.superscript.is_empty() {
                self.text(
                    &cell.superscript,
                    self.superscript_x(&cell.superscript, x, cw, ch),
                    y - 2,
                    (ch / 2).max(7) as u16,
                    if v.dreamwalking && !cell.dream_entity { cell.fg } else { crate::illumination_ui::paint_color([255, 224, 160], cell.display_light) },
                );
            }
            if cell.number_of_blood.is_some() {blood_counter=Some((cell,x,y,ink));}
            if !cell.subscript.is_empty() {
                let size = (ch / 2).max(7) as u16;
                let width: i32 = cell.subscript.chars()
                    .map(|ch| self.font.metrics(ch, size as f32).advance_width.round() as i32).sum();
                let xx = x + cw - width;
                let yy = y + ch - size as i32;
                self.rect(xx - 1, yy - 1, width + 2, size as i32 + 2, cell_bg);
                self.text(&cell.subscript, xx, yy, size, cell_fg);
            }
            if !cell.arrow.is_empty() {
                self.text(
                    &cell.arrow,
                    x + cw / 2,
                    y + ch / 2,
                    (ch / 2).max(7) as u16,
                    cell.fg,
                );
            }
        }
        // Airborne leaves stay visible over occupants without replacing their glyphs.
        for (rect, phase, time, color, opacity) in razorleaves {
            self.falling_razorleaves(rect,phase,time,color,opacity);
        }
        // The right edge can extend over another cell. Paint after every
        // map layer so a neighboring tile background never erases the digit.
        if let Some((cell,x,y,ink))=blood_counter {
            self.number_of_blood_superscript(cell,x,y,cw,ch,ink);
        }
        #[cfg(test)]
        drop(_paint_map);
        crate::game_hud::paint_resources(self,v,hud,now);
        crate::game_hud::paint_nav(self,hud);
        crate::game_hud::paint_depth(self,v,hud);
        crate::game_hud::panel(self,hud.divine);
        crate::game_hud::scroll_mark(self,hud.divine,v.total_abilities,self.ability_capacity(),v.total_abilities.saturating_sub(v.abilities.len()));
        crate::game_hud::paint_item_rail(self,v,hud);
        if self.intersects_damage(map_right,0,left-map_right,self.logical_height()) {
            #[cfg(test)]
            let _paint_enemies = paint_profile_bench::span(paint_profile_bench::Kind::Enemies);
            let enemy_layout=enemy_list::Layout::resolve_frame(&self.profile, ox,self.frame());
            enemy_list::paint(self,v,enemy_layout);
        }
        if self.intersects_damage(left,0,self.logical_width()-left,self.logical_height()) {
        #[cfg(test)]
        let _paint_abilities = paint_profile_bench::span(paint_profile_bench::Kind::HudAbilities);
        for (i, a) in v
            .abilities
            .iter()
            .take(self.ability_capacity())
            .enumerate()
        {
            let y = self.game_ability_top()
                + i as i32 * self.game_ability_row();
            if self.ability_hover == Some(i) {
                self.rect(left + 12, y, rail_width, self.game_ability_row(), [39,44,61]);
                self.border(left + 12, y, rail_width, self.game_ability_row(), [174,158,107]);
            }
            let selecting=v.selected_ability.as_deref()==Some(a.id.as_str());
            let ragelight_cost=match a.indicator {
                Some(crate::ability_ui::CustomIndicator::RagelightCost(cost))=>Some(cost),
                _=>None,
            };
            let passive=ragelight_cost.is_some()||(a.mode.eq_ignore_ascii_case("passive")&&!selecting);
            // Ragelight has no activation orb, but still spends its own reserve.
            let reserve=if passive && ragelight_cost.is_none() {None}else{a.reserve};
            let cogworks=a.id=="kative-cogworks";
            let orb_width=if passive{0}else if cogworks{50}else{18};
            let name_x=left+12+if cogworks{0}else{orb_width};
            let bar_width=if matches!(a.indicator,Some(crate::ability_ui::CustomIndicator::FutureFloors{..})){140.min(rail_width/2)}else{rail_width/3};
            let bar_width=bar_width.max(66).min(rail_width);
            let bar_x=left+12+rail_width-bar_width;
            let meter_y=y+32;
            let ragelight_box=ragelight_cost.map(|cost| {
                let width=ability_indicator_ui::ragelight_width(self,cost);
                (bar_x-width-6,width)
            });
            let name_width=if cogworks{rail_width}else{rail_width-8-orb_width};
            let names=self.wrap_text_role(crate::typography_ui::Role::Ability,&a.name,name_width,13);
            self.clip=Some((left+12,y,rail_width,self.game_ability_row()));
            for(line,name)in names.iter().take(2).enumerate(){
                self.text(name,name_x,y+3+line as i32*15,13,a.color);
            }
            if let Some((box_x,box_width))=ragelight_box {
                ability_indicator_ui::paint(self,a.indicator.unwrap(),box_x,meter_y,box_width);
            }
            if let Some(indicator)=a.indicator.filter(|_|ragelight_box.is_none()) {
                ability_indicator_ui::paint(self,indicator,bar_x,meter_y,bar_width);
            } else if let Some((count,until))=a.fortunes {
                let box_w=((bar_width-6)/3).max(18);
                for i in 0..3 {
                    let bx=bar_x+i* (box_w+3);
                    if i<count as i32 {self.rect(bx,meter_y,box_w,19,[184,119,255]);}
                    self.border(bx,meter_y,box_w,19,[184,119,255]);
                    if count<3&&i==count as i32 {
                        let text=until.to_string();let tw=resource_ui::text_width(self,&text,12);
                        self.text(&text,bx+(box_w-tw)/2,meter_y+2,12,[232,220,255]);
                    }
                }
            } else if let Some((current,maximum))=reserve {
                let value=format!("{current:.0}/{maximum:.0}");
                resource_ui::paint_bar_pending(self,"",&value,bar_x,meter_y,bar_width,19,
                    game_color(&self.profile,"ui.colors.energy"),current/maximum.max(1.),11,a.pending_reserve_loss/maximum.max(1.));
            }
            if !passive {
                for orb in 0..if cogworks{3}else{1} {
                    let chosen=if cogworks{orb+1==v.cogworks_mode as i32}else{a.active};
                    let targeting=selecting&&(!cogworks||chosen);
                    let cx=left+19+orb*16;let cy=if cogworks{y+40}else{y+11};
                    for dy in -5i32..=5 {for dx in -5i32..=5 {let d=dx*dx+dy*dy;if d<=25&&(targeting||chosen||d>=15){self.rect(cx+dx,cy+dy,1,1,if targeting{[255,155,56]}else if chosen{[100,255,114]}else{[110,110,125]});}}}
                }
            }
            self.clip=None;
        }
        } // HUD damage section
        if map_damage && (!v.path.is_empty()||!v.target_path.is_empty()||!v.target_area.is_empty()||v.target_impact.is_some()||!v.possible_exits.is_empty()||v.hover.is_some()) {
        #[cfg(test)]
        let _paint_interactions = paint_profile_bench::span(paint_profile_bench::Kind::Interactions);
        let known: std::collections::HashSet<_> =
            painted_cells.iter().map(|c| (c.x, c.y)).collect();
        for &(tx, ty) in &v.path {
            if known.contains(&(tx, ty)) {
                self.interaction_mark(
                    ox + tx as i32 * cw,
                    oy + ty as i32 * ch,
                    cw,
                    ch,
                    true,
                    false,
                    false,
                );
            }
        }
        if let Some((tx, ty)) = v.hover {
            if known.contains(&(tx, ty)) {
                self.interaction_mark(
                    ox + tx as i32 * cw,
                    oy + ty as i32 * ch,
                    cw,
                    ch,
                    false,
                    v.path.last() == Some(&(tx, ty)),
                    !v.target_path.is_empty(),
                );
            }
        }
        for &(tx, ty) in &v.target_path {
            if known.contains(&(tx, ty)) {
                self.interaction_mark(
                    ox + tx as i32 * cw,
                    oy + ty as i32 * ch,
                    cw,
                    ch,
                    true,
                    false,
                    true,
                );
            }
        }
        for &(tx,ty) in &v.target_area {
            if known.contains(&(tx,ty)) {
                let (x,y)=(ox+tx as i32*cw,oy+ty as i32*ch);
                self.rect(x+cw/3,y+ch-2,(cw/3).max(1),1,[244,127,101]);
            }
        }
        if let Some((tx,ty))=v.target_impact.filter(|p|known.contains(p)) {
            self.interaction_mark(ox+tx as i32*cw,oy+ty as i32*ch,cw,ch,false,true,true);
        }
        // Hollow cyan corners distinguish possible arrivals from the solid path.
        for &(tx, ty) in &v.possible_exits {
            if known.contains(&(tx, ty)) {
                let (x, y) = (ox + tx as i32 * cw, oy + ty as i32 * ch);
                let color = crate::non_euclidean_ui::INK;
                self.rect(x + 1, y + 1, (cw / 3).max(2), 1, color);
                self.rect(x + 1, y + 1, 1, (ch / 3).max(2), color);
                self.rect(x + cw - (cw / 3).max(2) - 1, y + ch - 2, (cw / 3).max(2), 1, color);
                self.rect(x + cw - 2, y + ch - (ch / 3).max(2) - 1, 1, (ch / 3).max(2), color);
            }
        }
        } // interaction marks
        if map_damage { crate::persistent_ability_overlays::paint(self,v,now); crate::perception_ui::paint(self,v); }
        crate::game_hud::paint_footer(self,v,hud,notice);
        let edge=[70,63,81];
        self.border(0,map_top,map_right,panel_bottom-map_top,edge);
        self.border(map_right,map_top,left-map_right,rail_bottom-map_top,edge);
        if let Some(veil) = crate::illumination_ui::screen_tint(v) {
            // Rebuilt base pixels only: clipped/tile paths tint their disjoint
            // repainted regions once, never previously tinted retained pixels.
            // O(selected native pixels), O(1) scratch; GPU records one alpha quad.
            self.native_fill(0, 0, self.width as i32, self.height as i32, veil, 34);
        }
    }
}
/// Keep free/passive metadata once while retaining meaningful counters and prices.
fn ability_cost_text(cost: &str, state: &str) -> String {
    let complete = cost.trim();
    if complete.eq_ignore_ascii_case(state.trim()) { return String::new(); }
    complete.to_owned()
}
pub fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
pub fn map_layout(w: usize, h: usize) -> (i32, i32, i32, i32) {
    map_layout_profile(w, h, crate::appearance_profile::Profile::defaults())
}
pub fn hud_left(p: &crate::appearance_profile::Profile) -> i32 {
    let max = p.number("ui.layout.rightSidebarWidth");
    if max == 440. && p.number("ui.layout.sidebarWidthPercent") == 23. {
        return WIDTH as i32 - GAME_RESOURCE_WIDTH;
    }
    (WIDTH as f64
        - (WIDTH as f64 * p.number("ui.layout.sidebarWidthPercent") / 100.)
            .clamp(max.min(290.), max))
    .floor() as i32
}
pub fn chronicle_height(p: &crate::appearance_profile::Profile) -> i32 {
    chronicle_height_for(p, crate::game_hud::HEADER - 12)
}
pub(crate) fn chronicle_height_for(p: &crate::appearance_profile::Profile, baseline: i32) -> i32 {
    let explicit = p.number("ui.layout.chronicleHeight");
    if explicit == 108.
        && (p.number("typography.chronicle.fontSize") != 18.
            || p.number("typography.chronicle.lineHeight") != 4. / 3.)
    {
        (p.number("typography.chronicle.fontSize")
            * p.number("typography.chronicle.lineHeight")
            * 4.5)
            .round() as i32
    } else if explicit == 108. {
        baseline
    } else {
        explicit.round() as i32
    }
}
/// O(1): preserve fitted tile aspect/size and align map to playfield left edge.
pub fn map_layout_profile(
    w: usize,
    h: usize,
    p: &crate::appearance_profile::Profile,
) -> (i32, i32, i32, i32) {
    map_layout_frame(w,h,p,crate::modal_ui::Frame::from_profile(p))
}
pub fn map_layout_frame(w:usize,h:usize,p:&crate::appearance_profile::Profile,frame:crate::modal_ui::Frame)->(i32,i32,i32,i32){
    let (tw,th)=(p.number("typography.map.tileWidth"),p.number("typography.map.tileHeight"));
    let stock = (tw,th)==(15.,24.);
    let (tile_w,tile_h) = crate::game_hud::MAP_TILE;
    // Stock geometry is measured on the 16:9 HUD, with no dead gutter.
    // Leave custom appearance profiles' tile shape and margin semantics intact.
    let aspect = if stock {f64::from(tile_h)/f64::from(tile_w)} else {th/tw};
    let area_w = (frame.map.2 - if stock {0} else {24}).max(1);
    let top = frame.map.1 + if stock {0} else {2};
    let area_h = (frame.map.3 - if stock {0} else {4}).max(1);
    let mut cw = ((area_w as f64 / w.max(1) as f64).min(area_h as f64 / (h.max(1) as f64 * aspect)))
        .floor()
        .max(1.) as i32;
    // Solve round(cw*aspect) <= floor(area_h/h) in O(1), including authored
    // sub-unit aspect ratios where reducing width by one is insufficient.
    let max_ch=(area_h/h.max(1) as i32).max(1);
    let rounded_height_width=(((max_ch as f64+0.5)/aspect).ceil()-1.).max(1.) as i32;
    cw=cw.min(rounded_height_width);
    let ch = (cw as f64 * aspect).round().max(1.) as i32;
    (
        cw,
        ch,
        frame.map.0,
        top + (area_h - ch * h as i32) / 2,
    )
}

fn hash(x: i32, y: i32) -> f64 {
    let h = (x.wrapping_add(197) as u32).wrapping_mul(374761393)
        ^ (y.wrapping_add(71) as u32).wrapping_mul(668265263);
    let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h as f64 / 4294967296.0
}
#[path = "title_fire.rs"]
mod title_fire;

pub fn hellfire(frame: u32) -> Picture {
    title_fire::picture(frame)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cosmetic_frames_preserve_complete_hud_and_chronicle() {
        let raw = include_str!("../../../fixtures/avatar-room-v1.json");
        let engine = corpse_core::temporal::Engine::load(raw).unwrap();
        let before = engine.save().unwrap();
        let mut view = crate::observe::observe(&engine.run);
        let mut painter = Painter::new();
        painter.game_at(&view, "", false, 0.);
        let baseline = painter.pixels.clone();
        let static_pixel = |x: usize, y: usize| (x >= 985 && y < 705) || (y < 110) || y >= 748;
        for time in [16., 83., 166., 420., 900.] {
            view.path = vec![(12, 14), (13, 14)];
            view.hover = Some((13, 14));
            painter.game_at(&view, "", false, time);
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    if static_pixel(x, y) {
                        let at = (y * WIDTH + x) * 4;
                        assert_eq!(
                            &painter.pixels[at..at + 4],
                            &baseline[at..at + 4],
                            "frame {time}, pixel {x},{y}"
                        );
                    }
                }
            }
        }
        assert_eq!(engine.save().unwrap(), before);
    }
    #[test]
    fn toolbar_hits_match_painted_rectangles() {
        let profile = crate::appearance_profile::Profile::default();
        for (slot, action) in TOOLBAR_VISIBLE_ACTIONS.into_iter().enumerate() {
            let (x, w) = toolbar_rect(slot);
            assert_eq!(toolbar_action(x, 748), Some(action));
            assert_eq!(toolbar_action(x + w - 1, 787), Some(action));
            assert_eq!(toolbar_action(x + w / 2, 765), Some(action));
            assert_eq!(toolbar_action_profile(&profile, x, toolbar_y(&profile)), Some(action));
            assert_eq!(toolbar_action_profile(&profile, x + w - 1, 787), Some(action));
        }
        for x in 12..1268 {
            assert!(toolbar_action(x, 760).is_some());
            assert_ne!(toolbar_action(x, 760), Some(8));
        }
        assert_eq!(toolbar_action(1267, 760), Some(9));
        assert_eq!(toolbar_action(11, 760), None);
        assert_eq!(toolbar_action(1268, 760), None);
        assert_eq!(toolbar_action(100, 788), None);
    }
    #[test]
    fn blessing_hits_exclude_button_gaps() {
        for i in 0..3 {
            assert_eq!(blessing_choice_at(389, 180 + i * 170), Some(i as usize));
            assert_eq!(blessing_choice_at(906, 337 + i * 170), Some(i as usize));
            assert_eq!(blessing_choice_at(389, 338 + i * 170), None);
        }
        assert_eq!(blessing_choice_at(907, 180), None);
    }
    #[test]
    fn downscaled_thin_rectangles_survive_with_clipping() {
        let mut p = Painter::new();
        p.resize(1180, 812);
        // Find exactly the rounding phase that used to erase a one-unit stroke.
        let y = (0..800)
            .find(|y| p.native_y(*y as f64) == p.native_y((*y + 1) as f64))
            .unwrap();
        let x = (0..1280)
            .find(|x| p.native_x(*x as f64) == p.native_x((*x + 1) as f64))
            .unwrap();
        p.clear([0, 0, 0]);
        p.clip = Some((20, y, 30, 1));
        p.rect(0, y, 100, 1, [255, 255, 255]);
        let row = p.native_y(y as f64) as usize;
        let left = p.native_x(20.) as usize;
        let right = p.native_x(50.) as usize;
        for xx in 0..p.width {
            assert_eq!(
                p.pixels[(row * p.width + xx) * 4],
                if (left..right).contains(&xx) { 255 } else { 0 }
            );
        }
        p.clear([0, 0, 0]);
        p.clip = Some((x, 20, 1, 30));
        p.rect(x, 0, 1, 100, [255, 255, 255]);
        let col = p.native_x(x as f64) as usize;
        for yy in 0..p.height {
            let visible = (p.native_y(20.) as usize..p.native_y(50.) as usize).contains(&yy);
            assert_eq!(
                p.pixels[(yy * p.width + col) * 4],
                if visible { 255 } else { 0 }
            );
        }
        let before = p.pixels.clone();
        p.rect(x, 20, 0, 30, [255, 0, 0]);
        p.rect(x, 20, 1, -1, [255, 0, 0]);
        assert_eq!(before, p.pixels);
    }
    #[test]
    fn native_resolution_rasterizes_fonts_and_ascii_at_device_size() {
        let mut p = Painter::new();
        assert!(p.resize(2560, 1600));
        p.clear([0, 0, 0]);
        p.text("A", 20, 20, 16, [255, 255, 255]);
        assert!(p.glyphs.contains_key(&('A', 32)));
        assert!(!p.glyphs.contains_key(&('A', 16)));
        p.map_glyph('@', 50, 50, 15, 24, [255, 255, 255]);
        assert!(p
            .map_masks
            .keys()
            .any(|k| k.0 == '@' && k.1 == 30 && k.2 == 48));
        assert_eq!(p.pixels.len(), 2560 * 1600 * 4);
        p.map_identity("@³", 100, 100, 15, 24, [255, 255, 255]);
        // Native compound outlines contain sub-logical-pixel coverage; enlarged
        // low-resolution masks would repeat each 2×2 block exactly.
        assert!((200..248)
            .step_by(2)
            .any(|y| (200..230).step_by(2).any(|x| {
                let at = (y * p.width + x) * 4;
                p.pixels[at] != p.pixels[at + 4] || p.pixels[at] != p.pixels[at + p.width * 4]
            })));
        assert!(!p.resize(2560, 1600));
        assert!(!p.glyphs.is_empty());
        assert!(p.resize(1920, 1080));
        assert!(p.glyphs.is_empty());
        assert!(p.map_masks.is_empty());
        assert_eq!(p.logical_point(960, 540, (1920, 1080)), (640, 400));
        assert_eq!(p.logical_point(480, 270, (960, 540)), (640, 400));
        assert!(p.logical_point(0, 0, (1920, 1080)).0 < 0);
        let (cw, ch, ox, oy) = map_layout(79, 29);
        assert!(p.native_x((ox + cw * 79) as f64) <= p.width as i32);
        assert!(p.native_y((oy + ch * 29) as f64) <= p.height as i32);
        p.clear([0, 0, 0]);
        p.clip = Some((20, 20, 10, 10));
        p.rect(0, 0, WIDTH as i32, HEIGHT as i32, [255, 255, 255]);
        for y in 0..p.height {
            for x in 0..p.width {
                if p.pixels[(y * p.width + x) * 4] != 0 {
                    assert!(p.inside_native_clip(x as i32, y as i32));
                }
            }
        }
    }
    #[test]
    fn ability_summary_uses_measured_ellipsis_without_dropping_payment_details() {
        let p = Painter::new();
        let role = crate::typography_ui::Role::Secondary;
        let result = p.fit_text_role(role, "Long reaction cost with details", 100, 12);
        assert!(result.ends_with('…'));
        assert!(p.measure_text_role(role, &result, 12) <= 100.);
        assert_eq!(
            ability_cost_text("1/1 reactions · 1 reaction/first attack", "PASSIVE"),
            "1/1 reactions · 1 reaction/first attack"
        );
        assert_eq!(
            ability_cost_text("3/3 fortunes · 1 fortune", "ACTIVE"),
            "3/3 fortunes · 1 fortune"
        );
        assert_eq!(
            ability_cost_text("3/5 charges · 6 NRG", "ON"),
            "3/5 charges · 6 NRG"
        );
    }
    #[test]
    fn entrance_omega_has_a_real_bundled_glyph_not_missing_character_box() {
        let mut p = Painter::new();
        assert!(p.fallback.iter().any(|font| font.lookup_glyph_index('Ω') != 0));
        p.clear([8,25,82]); p.map_identity("Ω",100,100,60,96,[76,170,255]);
        let omega=p.pixels.clone();
        p.clear([8,25,82]); p.map_identity("\u{10ffff}",100,100,60,96,[76,170,255]);
        assert_ne!(omega,p.pixels,"Omega must not use the missing-glyph box");
    }
    #[test]
    fn floor_fits() {
        let (w, h, x, y) = map_layout(79, 29);
        let frame = crate::modal_ui::Frame::from_profile(crate::appearance_profile::Profile::defaults());
        assert_eq!(x, frame.map.0);
        assert!(y >= frame.map.1);
        assert!(
            (h as f64 / w as f64 - 1.5).abs() <= 0.5 / w as f64,
            "default tile aspect survives integer raster fit"
        );
        assert!(frame.map.contains(x + w * 79 - 1, y + h * 29 - 1));
    }
    #[test]
    fn path_tints_are_white_restrained_and_preserve_glyph_ink() {
        let engine = corpse_core::temporal::Engine::load(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        let mut view = crate::observe::observe(&engine.run);
        view.terrain_snapshot = None;
        view.cells = vec![
            Cell {
                x: 1,
                y: 1,
                bg: [40, 60, 80],
                fg: [255, 255, 255],
                glyph: "@".into(),
                ..Cell::default()
            },
            Cell {
                x: 2,
                y: 1,
                bg: [40, 60, 80],
                ..Cell::default()
            },
        ];
        let mut p = Painter::new();
        let (cw, ch, ox, oy) = p.map_layout(view.width, view.height);
        p.game(&view, "", false);
        let baseline = p.pixels.clone();
        let at = |x: usize, y: usize| (y * WIDTH + x) * 4;
        let first = at((ox + cw + cw / 2) as usize, (oy + ch) as usize);
        let second = at((ox + 2 * cw + cw / 2) as usize, (oy + ch) as usize);
        view.path = vec![(1, 1), (2, 1), (3, 1)];
        view.hover = Some((2, 1));
        p.game(&view, "", false);
        assert!(p.pixels[first] > baseline[first] && p.pixels[first] < baseline[first] + 30);
        assert!(p.pixels[second] > p.pixels[first]);
        for y in oy + ch..oy + 2 * ch {
            for x in ox + cw..ox + 2 * cw {
                let i = at(x as usize, y as usize);
                for k in 0..3 {
                    assert!(
                        p.pixels[i + k] >= baseline[i + k],
                        "path must never darken glyphs/background"
                    );
                }
            }
        }
        // Even deliberately injected preview coordinates cannot expose unobserved cells.
        for y in oy + ch..oy + 2 * ch {
            for x in ox + 3 * cw..ox + 4 * cw {
                let i = at(x as usize, y as usize);
                assert_eq!(&p.pixels[i..i + 4], &baseline[i..i + 4]);
            }
        }
        view.path.clear();
        view.target_path = vec![(1, 1), (2, 1), (3, 1)];
        p.game(&view, "", false);
        assert!(p.pixels[first] > baseline[first]);
        assert!(p.pixels[second] > p.pixels[first]);
        let unknown = at((ox + 3 * cw + cw / 2) as usize, (oy + ch + ch / 2) as usize);
        assert_eq!(
            &p.pixels[unknown..unknown + 4],
            &baseline[unknown..unknown + 4]
        );
        assert_eq!(
            engine.save().unwrap(),
            corpse_core::temporal::Engine::load(include_str!(
                "../../../fixtures/avatar-room-v1.json"
            ))
            .unwrap()
            .save()
            .unwrap()
        );
    }
    #[test]
    fn embers_are_crisp_deterministic_drawable_resolution_particles() {
        let mut p = Painter::new();
        for (w, h) in [(1280, 800), (2560, 1600), (1920, 1080)] {
            p.resize(w, h);
            p.clear([0, 0, 0]);
            p.menu_embers(18);
            let first = p.pixels.clone();
            let lit = first.chunks_exact(4).filter(|px| px[0] > 0).count();
            let width = p.scale.round().max(1.) as usize;
            assert_eq!(
                lit,
                22 * width * width,
                "sixteen crisp 1- or 2-pixel-high particles"
            );
            assert!(
                first
                    .chunks_exact(4)
                    .all(|px| px[0] == 0 || [186, 216, 236].contains(&px[0])),
                "no interpolated halo"
            );
            p.clear([0, 0, 0]);
            p.menu_embers(18);
            assert_eq!(first, p.pixels);
            p.clear([0, 0, 0]);
            p.menu_embers(19);
            assert_ne!(first, p.pixels);
        }
    }
    #[test]
    fn fire_is_deterministic_animated() {
        assert_eq!(hellfire(18).rgba, hellfire(18).rgba);
        assert_ne!(hellfire(18).rgba, hellfire(19).rgba);
    }
    #[test]
    fn title_fire_is_not_reset_when_the_heartbeat_repeats() {
        let mut p = Painter::new();
        p.resize(640, 400);
        p.menu(18, 0, false, "Avatar");
        let first = p.pixels.clone();
        p.menu(378, 0, false, "Avatar");
        assert_ne!(p.pixels, first);
        p.menu(18, 0, false, "Avatar");
        assert_eq!(p.pixels, first, "reduced-motion frame stays deterministic");
    }
    #[test]
    fn continue_disabled_never_receives_active_menu_highlight() {
        let mut p = Painter::new();
        p.menu(18, 1, false, "Avatar");
        let r = &crate::menu_ui::ITEMS[1];
        let at = ((r.y + 2) as usize * p.width + (r.x + 2) as usize) * 4;
        // Hover feedback is gray while unavailable, never the enabled bone inversion.
        assert_eq!(&p.pixels[at..at + 3], &[74, 70, 70]);
        p.menu(18, 1, true, "Avatar");
        assert_eq!(&p.pixels[at..at + 3], &[238, 229, 207]);
    }
    #[test]
    fn paint_menu_manual() {
        let mut p = Painter::new();
        p.menu(18, 0, false, "Avatar");
        assert_eq!(p.pixels.len(), 1280 * 800 * 4);
        let a = p.pixels.clone();
        crate::manual::Manual::new().paint(&mut p);
        assert_ne!(p.pixels, a);
    }
}

// Stable command IDs are independent of their visible toolbar slots.
const NAV_LEN: usize = crate::game_hud::NAV.len();
pub const TOOLBAR_VISIBLE_ACTIONS: [usize; NAV_LEN] = {
    let [a, b, c] = crate::game_hud::NAV;
    [a.0, b.0, c.0]
};
pub const TOOLBAR_ACTIONS: [&str; NAV_LEN] = {
    let [a, b, c] = crate::game_hud::NAV;
    [a.1, b.1, c.1]
};
pub const TOOLBAR_LABELS: [&str; NAV_LEN] = TOOLBAR_ACTIONS;
/// Equal slots follow the active canvas, not the original 1280px sheet.
pub fn toolbar_rect_for_width(i:usize,width:i32)->(i32,i32){
    let usable=(width-24).max(0);
    let count=TOOLBAR_VISIBLE_ACTIONS.len() as i32;
    let left=12+usable*i as i32/count;
    let right=12+usable*(i as i32+1)/count;
    (left,right-left)
}
pub fn toolbar_rect(i: usize) -> (i32, i32) {toolbar_rect_for_width(i,WIDTH as i32)}
pub fn toolbar_y(p: &crate::appearance_profile::Profile) -> i32 {
    748 - (p.number("ui.layout.buttonHeight").round() as i32 - 42)
}
pub fn toolbar_action_profile(
    p: &crate::appearance_profile::Profile,
    x: i32,
    y: i32,
) -> Option<usize> {
    if !(toolbar_y(p)..788).contains(&y) {
        return None;
    }
    TOOLBAR_VISIBLE_ACTIONS.iter().enumerate().find_map(|(slot, action)| {
        let (left, w) = toolbar_rect(slot);
        (x >= left && x < left + w).then_some(*action)
    })
}
pub fn toolbar_action(x: i32, y: i32) -> Option<usize> {
    if !(748..788).contains(&y) {
        return None;
    }
    TOOLBAR_VISIBLE_ACTIONS.iter().enumerate().find_map(|(slot, action)| {
        let (left, w) = toolbar_rect(slot);
        (x >= left && x < left + w).then_some(*action)
    })
}

pub fn creature_at(p: &Painter, view: &View, offset: usize, x: i32, y: i32) -> Option<f64> {
    if !(337..973).contains(&x) || !(190..656).contains(&y) {
        return None;
    }
    let mut top = 196;
    for a in view.creatures.iter().skip(offset) {
        let (_, _, h) = p.creature_condition_layout(a);
        if y >= top - 4 && y < top + h - 4 {
            return Some(a.id);
        }
        top += h;
    }
    None
}

pub fn blessing_choice_at(x: i32, y: i32) -> Option<usize> {
    if !(373..907).contains(&x) {
        return None;
    }
    (0..3).find(|i| (180 + *i as i32 * 170..338 + *i as i32 * 170).contains(&y))
}

fn resource_max_label(max: f64) -> String {
    if max.is_infinite() {
        "∞".into()
    } else {
        format!("{max:.0}")
    }
}

/// Native UI palette defaults; explicitly authored appearance values still win.
pub(crate) fn game_color(p: &crate::appearance_profile::Profile, path: &str) -> Rgb {
    if p.value(path) != crate::appearance_profile::Profile::defaults().value(path) { return p.rgb(path); }
    match path {
        "ui.colors.primary" => INK,
        "ui.colors.chronicle" => [198, 191, 207],
        "ui.colors.secondary" | "ui.colors.muted" | "ui.colors.disabled" => MUTED,
        "ui.colors.health" => [255, 36, 87],
        "ui.colors.energy" => [17, 217, 238],
        "ui.colors.blessing" | "ui.colors.headings" => GOLD,
        "ui.colors.background" => [8, 6, 14],
        "ui.colors.hover" => [32, 26, 42],
        _ => p.rgb(path),
    }
}
/// O(1) cosmetic transform of already-redacted observations. No world access,
/// no live lighting/geometry lookup, and never creates a cell in unknown space.
pub(crate) fn readable_cell_colors(cell: &Cell) -> (Rgb, Rgb) {
    if cell.display_light.is_some() { return (cell.fg, cell.bg); }
    if let Some(entity) = cell.entity_ink {
        let fg = if entity.important && !entity.remembered && !entity.telepathic && !entity.plain_black && entity.identity_outline.is_none() {
            crate::color::distinguish(cell.fg, cell.bg, if entity.player { 5. } else { 3.8 })
        } else { cell.fg };
        return (fg, cell.bg);
    }
    // Known chasm terrain keeps its authored dark ink and black background in
    // memory too. Entity ink was handled above; never lift a void to memory gray.
    if cell.fire || cell.label.is_empty() || cell.label == "chasm" { return (cell.fg, cell.bg); }
    if cell.current {
        let contrast = if cell.label == "wall" { 2.5 } else if cell.label == "floor" { 1.7 } else { 2.1 };
        (crate::color::distinguish(cell.fg, cell.bg, contrast), cell.bg)
    } else {
        let gray = |c: Rgb, saturation: f64| {
            let luma = c[0] as f64 * 0.2126 + c[1] as f64 * 0.7152 + c[2] as f64 * 0.0722;
            c.map(|v| (luma + (v as f64 - luma) * saturation).round().clamp(0., 255.) as u8)
        };
        let wall = cell.label == "wall";
        let bg = gray(cell.bg, 0.).map(|v| v.max(if wall { 13 } else { 7 }));
        let fg = gray(cell.fg, 0.);
        let floor: u8 = if wall { 70 } else if cell.label == "floor" { 54 } else { 64 };
        let peak = *fg.iter().max().unwrap_or(&0);
        let lift = floor.saturating_sub(peak);
        (fg.map(|v| v.saturating_add(lift)), bg)
    }
}

#[cfg(test)]
mod gameplay_hierarchy_tests {
    use super::*;
    #[test]
    fn revision50_status_meter_labels_retain_turns_and_legacy_copy() {
        let p=Painter::new();
        for (name,compact) in [("Discord: hostile to all","Discord"),("Rooted","Rooted"),("Corroded","Corroded"),("Fire Immunity","Fire immune")] {
            for turns in [3.,6.,20.,999.] {
                let m=crate::statuses_ui::Meter{name:name.into(),duration:Some(turns),reference:turns,reference_known:true,percent:100.};
                let label=p.condition_meter_label(&m,124);
                assert_eq!(label,format!("{compact} {turns}"));
                assert!(p.measure_text_role(crate::typography_ui::Role::Secondary,&label,13)<=124.);
            }
        }
        let m=crate::statuses_ui::Meter{name:"Long legacy condition".into(),duration:Some(5.),reference:5.,reference_known:true,percent:100.};
        assert_eq!(p.condition_meter_label(&m,124),"Long legacy … 5");
        assert!(p.measure_text_role(crate::typography_ui::Role::Secondary,&p.condition_meter_label(&m,124),13)<=124.);
    }
    fn scene() -> View {
        let e = corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        crate::observe::observe(&e.run)
    }
    #[test]
    fn long_domain_heading_and_passive_metadata_fit_fixed_allocations() {
        let p = Painter::new();
        let width = WIDTH as i32 - 80 - (hud_left(&p.profile) + 12) - 10;
        let title = "Archipelago · 1/40";
        assert_eq!(p.fit_text_role(crate::typography_ui::Role::Headings, title, width, 13), title);
        assert!(p.measure_text_role(crate::typography_ui::Role::Headings, title, 13) <= width as f64);
        assert_eq!(ability_cost_text("Passive", "PASSIVE"), "");
        assert_eq!(ability_cost_text("Free", "FREE"), "");
        assert_eq!(ability_cost_text("Free", "OFF"), "Free");
        assert_eq!(ability_cost_text("1/1 reactions · 1 reaction/first attack", "PASSIVE"), "1/1 reactions · 1 reaction/first attack");
    }
    #[test]
    fn memory_preserves_geometry_without_revealing_black_space() {
        let wall = Cell { label: "wall".into(), fg: [24, 23, 26], bg: [2, 2, 3], ..Cell::default() };
        let floor = Cell { label: "floor".into(), ..wall.clone() };
        let water = Cell { label: "water".into(), fg: [25, 40, 60], ..wall.clone() };
        let (wall_fg, wall_bg) = readable_cell_colors(&wall);
        let (floor_fg, _) = readable_cell_colors(&floor);
        let (water_fg, _) = readable_cell_colors(&water);
        assert!(wall_fg[0] >= 70 && floor_fg[0] >= 54);
        assert!(wall_fg[0] > floor_fg[0]);
        assert_eq!(water_fg[2],water_fg[0]);
        assert!(crate::color::contrast(wall_fg, wall_bg) > 2.);
        assert_eq!(readable_cell_colors(&Cell::default()), ([0; 3], [0; 3]));
        // No clock/neighbor/current world state enters the remembered transform.
        let mut altered = wall.clone();
        altered.direct = true;
        altered.actor_id = Some(99.);
        assert_eq!(readable_cell_colors(&wall), readable_cell_colors(&altered));
    }
    #[test]
    fn sidebar_content_never_moves_map_or_toolbar() {
        let mut view = scene();
        let mut painter = Painter::new();
        painter.game_at(&view, "", false, 0.);
        let before = painter.pixels.clone();
        let layout = painter.map_layout(view.width, view.height);
        view.hp = 2.;
        view.max_hp = 99999.;
        view.total_abilities = 90;
        view.abilities = vec![crate::ability_ui::AbilityView { reserve:None,indicator:None,fortunes:None,pending_reserve_loss:0.,
            id: "test".into(), name: "Unyielding Assault".into(), description: "Details".repeat(100),
            color: [255, 70, 150], cost: "3/5 charges · 6 NRG".into(), state: "Unavailable".into(),
            active: false, mode: "Passive".into(), supported: true,
        }; 90];
        view.creatures.clear();
        painter.game_at(&view, "", false, 0.);
        assert_eq!(painter.map_layout(view.width, view.height), layout);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                // The Chronicle ends at the HUD boundary. The HUD's top rows
                // legitimately change here (creature count and Health).
                if (x >= layout.2 as usize && x < painter.frame().map.2 as usize)
                    || (x < painter.frame().map.2 as usize && y < chronicle_height(&painter.profile) as usize)
                    || y >= 748
                {
                    let i = (y * WIDTH + x) * 4;
                    assert_eq!(&before[i..i + 4], &painter.pixels[i..i + 4], "unrelated pixel {x},{y}");
                }
            }
        }
        assert_eq!(painter.fit_text_role(crate::typography_ui::Role::Ability, "Unyielding Assault",
            WIDTH as i32 - hud_left(&painter.profile) - 38, 15), "Unyielding Assault");
    }
    #[test]
    fn measured_wrap_bounds_long_tokens_and_chronicle_fade_is_continuous() {
        let p = Painter::new();
        let role = crate::typography_ui::Role::Chronicle;
        let lines = p.wrap_text_role(role, "A strike lands. Then an exceptionallylongunbrokentokenends.", 130, 18);
        assert!(lines.len() > 2);
        for line in lines { assert!(p.measure_text_role(role, &line, 18) <= 130.); }
        assert_eq!(chronicle_height(&p.profile), 108);
        for i in 1..100 {
            assert!(crate::chronicle::fade(i as f64 / 100.) > crate::chronicle::fade((i - 1) as f64 / 100.));
            assert!(crate::chronicle::fade(i as f64 / 100.) - crate::chronicle::fade((i - 1) as f64 / 100.) < 0.011);
        }
    }
    #[test]
    fn hidden_world_edits_cannot_change_painted_frame() {
        use corpse_core::model::{Run, Floor, Field, Actor};
        use std::sync::Arc;
        let mut run = Run::default();
        run.depth = 1.; run.player.x = 1.; run.player.y = 1.; run.player.glyph = "@".into();
        let mut floor = Floor::default();
        floor.depth = 1.; floor.width = Field::Value(79.); floor.height = Field::Value(29.);
        floor.tiles = vec!["wall".into(); 79 * 29];
        floor.lights = vec![-100.; 79 * 29]; floor.seen = vec![false; 79 * 29];
        run.floors = vec![Arc::new(floor)];
        let mut painter = Painter::new();
        painter.game_at(&crate::observe::observe(&run), "", false, 0.);
        let before = painter.pixels.clone();
        Arc::make_mut(&mut run.floors[0]).tiles[20 * 79 + 70] = "water".into();
        let mut enemy = Actor::default(); enemy.x = 70.; enemy.y = 20.; enemy.glyph = "Z".into();
        run.actors.insert("1".into(), Arc::new(vec![enemy]));
        painter.game_at(&crate::observe::observe(&run), "", false, 0.);
        assert!(painter.pixels == before, "hidden-world edits changed the displayed frame");
    }
    #[test]
    fn export_gameplay_hierarchy_review() {
        let Ok(directory) = std::env::var("CORPSE_GAME_UI_REVIEW") else { return; };
        std::fs::create_dir_all(&directory).unwrap();
        let mut view = scene();
        view.log_groups = crate::chronicle::compact(&[
            "You step through the dark water.".into(), "The wall remembers an ancient fire.".into(),
            "You strike the Screamling.".into(), "You kill the Screamling.".into(),
            "A blessing stirs. The ancient passages remain visible beyond the reach of your light.".into(),
        ], &[]);
        view.abilities = vec![crate::ability_ui::AbilityView { reserve:None,indicator:None,fortunes:None,pending_reserve_loss:0.,
            id: "test".into(), name: "Unyielding Assault".into(), description: String::new(),
            color: [255, 70, 150], cost: "1/1 reactions · 1 reaction/first attack".into(), state: String::new(),
            active: false, mode: "Passive".into(), supported: true,
        }];
        view.total_abilities = 1;
        let mut painter = Painter::new(); painter.toolbar_focus = Some(4);
        for (w, h) in [(1280, 800), (1920, 1080)] {
            painter.resize(w, h); painter.game_at(&view, "", false, 0.);
            let file = std::fs::File::create(format!("{directory}/gameplay-hierarchy-{w}x{h}.png")).unwrap();
            let mut encoder = png::Encoder::new(file, w as u32, h as u32);
            encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        }
    }
    #[test]
    fn layered_heartbeat_preserves_rest_image_and_stationary_foreground() {
        let mut p = Painter::new();
        let logo = p.logo.clone();
        let layers = std::sync::Arc::clone(&p.menu_layers);
        for (w, h) in [(960, 540), (1280, 800), (1537, 961)] {
            p.resize(w, h);
            p.clear([7, 9, 11]);
            let art_h = (500. * logo.h as f64 / logo.w as f64).round() as i32;
            p.picture(&logo, 676, 58, 500, art_h);
            let baseline = p.pixels.clone();
            p.clear([7, 9, 11]);
            p.beating_emblem(676, 58, 500, art_h, 18);
            assert_eq!(baseline, p.pixels, "rest must exactly reconstruct original art");
            for frame in [23, 26, 31, 35] {
                p.clear([7, 9, 11]);
                p.beating_emblem(676, 58, 500, art_h, frame);
                assert_ne!(baseline, p.pixels, "heart or traveling blood wave must change pixels");
                let nx = p.native_x(676.);
                let ny = p.native_y(58.);
                let nw = p.native_x(1176.) - nx;
                let nh = p.native_y((58 + art_h) as f64) - ny;
                for yy in 0..nh {
                    for xx in 0..nw {
                        let (sx, sy) = (xx as usize * logo.w / nw as usize, yy as usize * logo.h / nh as usize);
                        let foreground_alpha = layers[2].rgba[(sy * logo.w + sx) * 4 + 3];
                        if foreground_alpha > 0 {
                            let at = (((ny + yy) as usize * w) + (nx + xx) as usize) * 4;
                            // Only transmission from animated layers BEHIND the fixed
                            // foreground can vary: bounded by its exact transparency.
                            // This also covers authored bone alpha251–253, not just255.
                            for channel in 0..3 {
                                assert!(baseline[at + channel].abs_diff(p.pixels[at + channel]) <= 255 - foreground_alpha,
                                    "ribs/skull/crown sampling must stay fixed");
                            }
                        }
                    }
                }
            }
        }
    }

}
#[cfg(test)]
#[path = "paint_parity_tests.rs"]
mod paint_parity_tests;
#[cfg(test)]
#[path = "paint_bulk_tests.rs"]
mod paint_bulk_tests;

#[cfg(test)]
#[path = "menu_cache_tests.rs"]
mod menu_cache_tests;

/// O(1) Chronicle click and hover use the same painted logical region.
pub fn chronicle_hit_rect(p:&crate::appearance_profile::Profile)->crate::modal_ui::Rect {
    crate::modal_ui::Rect(12,12,(crate::modal_ui::Frame::from_profile(p).map.2-24).max(0),chronicle_height(p).max(0))
}

#[cfg(test)]
mod heartless_native_tests {
    use super::*;
    /// O(drawable pixels + glyph variants), optional renderer-only visual evidence.
    #[test]
    fn native_heartless_marker_is_antialiased_and_cell_clipped_at_three_scales() {
        for scale in [1.,1.5,2.] {
            let mut p=Painter::new();p.resize((1280.*scale) as usize,(800.*scale) as usize);
            p.clear([5,6,7]);
            for (i,glyph) in ["b","^","y","B","bb"].iter().enumerate() {
                p.map_identity(glyph,40+i as i32*32,40,12,19,[230,204,86]);
            }
            let before=p.pixels.clone();
            for (i,glyph) in ["b","^","y","B","bb"].iter().enumerate() {
                let x=40+i as i32*32; p.heartless_marker(glyph,x,40,12,19,None);
            }
            let mut partial=0;
            for y in 0..p.height {for x in 0..p.width {
                let at=(y*p.width+x)*4;
                if p.pixels[at..at+4]!=before[at..at+4] {
                    assert!([40,72,104,136,168].iter().any(|&left|
                        x as i32>=p.native_x(left as f64)&&(x as i32)<p.native_x((left+12) as f64)
                        &&y as i32>=p.native_y(40.)&&(y as i32)<p.native_y(59.)));
                    let c=&p.pixels[at..at+3];
                    if c!=[240,223,190]&&c!=[24,23,21] {partial+=1;}
                }
            }}
            assert!(partial>0,"native edge coverage missing at scale {scale}");
            if let Ok(dir)=std::env::var("HEARTLESS_SCREENSHOT_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                let file=std::fs::File::create(format!("{dir}/heartless-native-{scale}.png")).unwrap();
                let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
                encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod timeweave_paint_tests {
    use super::*;
    /// O(native framebuffer pixels) test scan; renderer itself paints only bounded cell strokes.
    #[test]
    fn cached_teal_arrows_are_directional_clipped_and_scale_consistent() {
        for scale in [1.,1.5,2.] {
            let mut p=Painter::new();p.resize((WIDTH as f64*scale) as usize,(HEIGHT as f64*scale) as usize);
            let mut images=Vec::new();
            for direction in [(-1,-1),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)] {
                p.clear([0,0,0]);p.timeweave_arrow(100,100,12,19,direction);
                let mut colored=0;
                for y in 0..p.height {for x in 0..p.width {
                    let rgb=&p.pixels[(y*p.width+x)*4..][..3];
                    if rgb!=[0,0,0] {
                        colored+=1;assert!(rgb[1]>rgb[0] && rgb[2]>rgb[0]);
                        assert!((x as i32)>=p.native_x(100.) && (x as i32)<p.native_x(112.));
                        assert!((y as i32)>=p.native_y(100.) && (y as i32)<p.native_y(119.));
                    }
                }}
                assert!(colored>0);
                let mut crop=Vec::new();
                for yy in p.native_y(100.)..p.native_y(119.) {
                    let a=(yy as usize*p.width+p.native_x(100.) as usize)*4;
                    let b=(yy as usize*p.width+p.native_x(112.) as usize)*4;
                    crop.extend_from_slice(&p.pixels[a..b]);
                }
                images.push(crop);
            }
            for i in 0..images.len(){for j in 0..i{assert_ne!(images[i],images[j]);}}
            p.clear([0,0,0]);p.timeweave_arrow(100,100,12,19,(0,0));assert!(p.pixels.chunks_exact(4).all(|c|c[..3]==[0,0,0]));
            if let Ok(dir)=std::env::var("CORPSE_TIMEWEAVE_UI_REVIEW") {
                p.clear([24,21,28]);
                for (i,direction) in [(-1,-1),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)].into_iter().enumerate(){
                    let x=100+i as i32*20;p.timeweave_arrow(x,100,12,19,direction);p.map_identity(".",x,100,12,19,[150,139,135]);
                }
                let file=std::fs::File::create(format!("{dir}/timeweave-arrows-{scale}.png")).unwrap();
                let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod full_paint_no_scissor_tests {
    use super::*;
    #[test]
    fn full_frame_without_damage_bounds_matches_explicit_full_scissor() {
        let e=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut view=crate::observe::observe(&e.run);
        let mut actor=view.cells[view.cells.len()/2].clone();
        actor.actor_id=Some(987654.);actor.glyph="b".into();actor.heartless=true;
        actor.superscript="long overhang".into();actor.subscript="3".into();
        view.cells.push(actor);view.selected_actor=Some(987654.);
        for (w,h) in [(1280,800),(1920,1200),(2560,1600)] {
            let mut full=Painter::new();let mut scissored=Painter::new();
            full.resize(w,h);scissored.resize(w,h);
            scissored.damage_clip=Some(damage::Rect::new(0,0,w,h));
            full.game_at(&view,"Full repaint",false,1234.);
            scissored.game_at(&view,"Full repaint",false,1234.);
            assert_eq!(full.pixels,scissored.pixels,"full frame parity at {w}x{h}");
        }
    }
}

#[cfg(test)]
#[path="responsive_geometry_tests.rs"]
mod responsive_geometry_tests;

#[cfg(test)]
#[path="resolution_bench.rs"]
mod resolution_bench;

#[cfg(test)]
#[path="paint_profile_bench.rs"]
mod paint_profile_bench;

#[cfg(test)]
#[path="alpha_one_oracle_tests.rs"]
mod alpha_one_oracle_tests;

#[cfg(test)]
#[path="sparse_ascii_tests.rs"]
mod sparse_ascii_tests;

#[cfg(test)]
#[path="corpse_marker_tests.rs"]
mod corpse_marker_tests;

#[cfg(test)]
#[path="compact_dialog_tests.rs"]
mod compact_dialog_tests;

#[cfg(test)]
#[path="visual_requests_revision50_tests.rs"]
mod visual_requests_revision50_tests;

#[cfg(test)]
#[path="creature_identity_tests.rs"]
pub(crate) mod creature_identity_tests;

#[cfg(test)]
#[path="quiet_ui_tests.rs"]
mod quiet_ui_tests;

#[cfg(test)]
#[path="ember_glyph_tests.rs"]
mod ember_glyph_tests;

#[cfg(test)]
#[path="grass_amulet_render_tests.rs"]
mod grass_amulet_render_tests;

#[cfg(test)]
mod ground_number_paint_tests {
    use super::*;
    fn bounds(p:&Painter)->(usize,usize,usize,usize) {
        let points:Vec<_>=p.pixels.chunks_exact(4).enumerate().filter(|(_,c)|c[..3]!=[0,0,0]).map(|(i,_)|(i%p.width,i/p.width)).collect();
        (points.iter().map(|p|p.0).min().unwrap(),points.iter().map(|p|p.1).min().unwrap(),
         points.iter().map(|p|p.0).max().unwrap(),points.iter().map(|p|p.1).max().unwrap())
    }
    #[test]
    fn numbers_are_smaller_and_lower_at_native_scales() {
        for scale in [1.,1.5,2.] {
            let mut p=Painter::new();p.resize((WIDTH as f64*scale)as usize,(HEIGHT as f64*scale)as usize);
            for glyph in ["1","12","128"] {
                p.clear([0,0,0]);p.map_identity(glyph,100,100,15,24,[255,255,255]);let large=bounds(&p);
                p.clear([0,0,0]);p.ground_number(glyph,100,100,15,24,15,24,[255,255,255]);let small=bounds(&p);
                assert!(small.3-small.1<large.3-large.1,"{glyph} scale {scale}");
                assert!(small.1>large.1);assert!(small.3>=large.3);
                assert!(small.0>=p.native_x(100.)as usize && small.2<p.native_x(115.)as usize);
                assert!(small.3<p.native_y(124.)as usize);
            }
            if let Ok(dir)=std::env::var("CORPSE_TIMEWEAVE_UI_REVIEW") {
                p.clear([24,21,28]);
                for (i,glyph) in ["1","2","12","128"].into_iter().enumerate() {
                    let x=100+i as i32*22;p.map_identity(glyph,x,70,15,24,[95,211,211]);
                    p.ground_number(glyph,x,100,15,24,15,24,[95,211,211]);
                }
                p.map_identity_aligned("\"",195,100,15,24,[120,199,107],true);
                let file=std::fs::File::create(format!("{dir}/ground-numbers-{scale}.png")).unwrap();
                let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod linked_space_repaint_tests {
    use super::*;
    #[test]
    fn moving_player_and_ground_numbers_match_full_and_tile_repaints() {
        let mut run=crate::non_euclidean_ui::tests::scene();run.simulation_revision=52.0.into();
        run.player.x=11.;let mut old=crate::observe::observe(&run);
        let (mut retained_p,mut tile,mut full)=(Painter::new(),Painter::new(),Painter::new());
        let mut retained=RetainedGame::default();let mut plan=tile_damage::TilePlanner::default();
        retained.update(&mut retained_p,&old,"",0.,true,true).unwrap();
        plan.replace_view(None,&old,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&old,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        for x in [13.,12.,13.,10.] {
            run.player.x=x;let view=crate::observe::observe(&run);
            full.game_at(&view,"",false,0.);retained.update(&mut retained_p,&view,"",0.,true,false).unwrap();
            assert!(retained_p.pixels==full.pixels,"retained at {x}");
            plan.replace_view(Some(&old),&view,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&view,"",0.);tile.tile_clip_enabled=false;
            assert!(tile.pixels==full.pixels,"tile at {x}");plan.clear_frame();old=view;
        }
        if let Ok(dir)=std::env::var("CORPSE_TIMEWEAVE_UI_REVIEW") {
            let file=std::fs::File::create(format!("{dir}/linked-space-full-frame.png")).unwrap();
            let mut encoder=png::Encoder::new(file,full.width as u32,full.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&full.pixels).unwrap();
        }
    }
}

#[cfg(test)]
#[path="glyph_mask_tests.rs"]
mod glyph_mask_tests;

#[cfg(test)]
mod zero_health_render_tests {
    use super::*;
    fn number_pixels(hp: f64) -> Vec<u8> {
        let r: corpse_core::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut v = crate::observe::observe(&r);v.hp=hp;v.max_hp=100.;
        let mut p=Painter::new();p.game_at(&v,"",false,0.);
        let left=(p.logical_width()-138) as usize;
        (76..91).flat_map(|y|p.pixels[(y*p.width+left)*4..(y*p.width+left+120)*4].to_vec()).collect()
    }
    #[test]
    fn zero_health_player_hud_distinguishes_a_quarter_hp_from_death() {
        assert_eq!(number_pixels(0.25),number_pixels(1.));
        assert_ne!(number_pixels(0.25),number_pixels(0.));
    }
}

#[cfg(test)]
mod fast_menu_layout_tests {
    use super::*;
    /// Opt-in native Painter evidence only. O(F * P + glyph rasterization) time,
    /// O(P) comparison memory for F=9 exported frames at three fixed sizes.
    /// No SDL window, game state, gameplay RNG, or product CLI behavior is added.
    #[test]
    fn fast_menu_native_scaled_captures() {
        use crate::avatar_menu::{Selection, GODS, MODE};
        use sdl2::keyboard::{Keycode, Mod};
        let Ok(directory) = std::env::var("COTCG_FAST_MENU_EXPORT_DIR") else { return; };
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        let mut painter = Painter::new();
        let mut selection = Selection::default();
        let mut seed = "03-09-11:fast-menu-review".to_string();
        selection.sync_seed(&seed);
        let original_seed = seed.clone();
        let original_gods = selection.gods;
        assert_eq!(original_gods, [3, 9, 11]);
        let mut frames = Vec::new();
        let paint_menu = |painter: &mut Painter, selection: &Selection, seed: &str, fast| {
            painter.menu(18, crate::menu_ui::FAST_INDEX, false, MODE);
            painter.menu_seed(seed, false, false);
            painter.menu_fast_mode(fast, true);
            selection.paint_slots(painter, crate::menu_ui::FAST_INDEX);
        };
        let write_frame = |painter: &Painter, name: &str| {
            // Exclusive evidence creation prevents a later invocation silently
            // replacing frames whose source/binary identity was already bound.
            let file = std::fs::OpenOptions::new().write(true).create_new(true)
                .open(directory.join(name)).unwrap();
            let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        };
        for (width, height) in [(1280, 800), (1920, 1200), (960, 1200)] {
            painter.resize(width, height);
            let controls = [crate::menu_ui::FAST_RECT, crate::menu_ui::SEED_RECT,
                crate::avatar_menu::CANCEL].into_iter()
                .chain((0..3).map(crate::avatar_menu::slot_rect))
                .chain((0..12).map(crate::avatar_menu::card_rect))
                .chain(crate::menu_ui::ITEMS.iter().map(|item| (item.x,item.y,item.w,item.h)));
            for (x,y,w,h) in controls {
                assert!(painter.native_x(x as f64) >= 0 && painter.native_y(y as f64) >= 0);
                assert!(painter.native_x((x+w) as f64) <= width as i32);
                assert!(painter.native_y((y+h) as f64) <= height as i32);
            }
            let (fx,fy,fw,fh) = crate::menu_ui::FAST_RECT;
            let fast_bounds = (painter.native_x(fx as f64),painter.native_y(fy as f64),
                painter.native_x((fx+fw) as f64),painter.native_y((fy+fh) as f64));
            for dpi in [1,2] {
                let native = (painter.native_x((fx+fw/2) as f64)/dpi,
                    painter.native_y((fy+fh/2) as f64)/dpi);
                let logical = painter.logical_point_for_canvas(native.0,native.1,
                    ((width/dpi as usize) as u32,(height/dpi as usize) as u32),false);
                assert!(crate::menu_ui::fast_at(logical.0,logical.1));
            }
            let mut off_frame = Vec::new();
            let mut on_frame = Vec::new();
            for fast in [false, true] {
                paint_menu(&mut painter, &selection, &seed, fast);
                assert_eq!((painter.width, painter.height), (width, height));
                assert_eq!(selection.gods, original_gods);
                assert_eq!(seed, original_seed);
                let state = if fast { "on" } else { "off" };
                let name = format!("avatar-fast-{state}-{width}x{height}.png");
                write_frame(&painter, &name);
                frames.push(serde_json::json!({"file":name,"width":width,"height":height,
                    "screen":"Avatar menu","fast":fast,"selectedControl":"Fast mode",
                    "fastNativeBounds":fast_bounds}));
                if fast { on_frame = painter.pixels.clone(); }
                else { off_frame = painter.pixels.clone(); }
            }
            assert_eq!(off_frame, on_frame, "Removed mode flag must not change the menu");
            for (index,(off,on)) in off_frame.chunks_exact(4).zip(on_frame.chunks_exact(4)).enumerate() {
                if off != on {
                    let (x,y) = ((index%width) as i32,(index/width) as i32);
                    assert!(x>=fast_bounds.0 && x<fast_bounds.2 && y>=fast_bounds.1 && y<fast_bounds.3,
                        "Fast toggle changed pixels outside its actual native bounds");
                }
            }
            selection.open(0);
            selection.paint_picker(&mut painter);
            for id in 0..GODS.len() {
                let (x,y,_,h) = crate::avatar_menu::card_rect(id);
                let px = painter.native_x((x+4) as f64) as usize;
                let py = painter.native_y((y+h-3) as f64) as usize;
                let offset = (py*width+px)*4;
                let occupied = selection.gods.iter().position(|&god| god == id).is_some_and(|slot| slot != 0);
                assert_eq!(&painter.pixels[offset..offset+3], &crate::avatar_menu::card_stripe(crate::ability_ui::god_color(id), occupied));
            }
            let picker_frame = painter.pixels.clone();
            let name = format!("avatar-god-picker-{width}x{height}.png");
            write_frame(&painter, &name);
            frames.push(serde_json::json!({"file":name,"width":width,"height":height,
                "screen":"God picker","fast":true,"pickerSlot":0}));
            selection.key(Keycode::Escape, Mod::NOMOD, &mut seed);
            assert!(!selection.active());
            assert_eq!(selection.gods, original_gods);
            assert_eq!(seed, original_seed);
            paint_menu(&mut painter, &selection, &seed, true);
            assert_eq!(painter.pixels, on_frame, "Cancel restores the same selected Fast menu");
            selection.open(0);
            selection.paint_picker(&mut painter);
            assert_eq!(painter.pixels, picker_frame, "Reopening retains picker selection and layout");
            selection.close();
        }
        let gods: Vec<_> = GODS.iter().enumerate().map(|(id, god)| serde_json::json!({
            "id":id,"name":god.name,"alias":god.alias,"domain":god.domain,
            "color":crate::ability_ui::god_color(id)
        })).collect();
        let evidence = serde_json::json!({
            "verification":"production native Painter PNGs, not live SDL input or Windows runtime",
            "seed":seed,"selectedGods":original_gods,"gods":gods,"frames":frames,
            "cancelReopen":"production Selection Escape path and exact full framebuffer restoration",
            "fastSemantics":"presentation OFF/ON only; gameplay verified by separate source-bound core tests"
        });
        let file = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(directory.join("fast-menu-evidence.json")).unwrap();
        serde_json::to_writer_pretty(file, &evidence).unwrap();
        println!("FAST_MENU_CAPTURE frames=9 sizes=1280x800,1920x1200,960x1200 live_sdl=false");
    }
    #[test]
    fn fast_menu_labels_fit_their_fixed_columns_at_supported_scales() {
        let mut p=Painter::new();
        for (width,height) in [(640,480),(960,540),(1280,800),(1920,1080),(2560,1600)] {
            p.resize(width,height);
            for (text,size,available) in [
                ("Fast mode: OFF",18,170),
                ("Fast mode: ON",18,170),
                ("Floors 1, 3, ... 39, 40; hearts grant",12,359),
                ("double blessings and stats. New descents only.",12,359),
            ] {
                // Same baseline advances as Painter::text; drawable scaling preserves fit.
                let advance:i32=text.chars().map(|c|p.font.metrics(c,size as f32).advance_width.round() as i32).sum();
                assert!(advance<=available,"{text} requires {advance}, available {available}");
                assert!(advance as f64*p.scale<=available as f64*p.scale);
            }
        }
    }
}

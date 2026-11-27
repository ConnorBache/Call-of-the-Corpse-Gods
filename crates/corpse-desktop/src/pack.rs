//! Measured Pack projection. UI grouping is never simulation stacking.
//!
//! The owner must refresh after engine dispatch, discard this view when replacing
//! a session, and consume all input while the drawer is open. Only `Order` edits
//! cosmetic Run data; all gameplay leaves through the temporal engine as Command.
use crate::render::{Painter, GOLD, INK, MUTED};
use crate::{
    appearance_profile::Profile,
    modal_ui::{self, PackLayout, Rect, TextMetrics},
    typography_ui::Role,
};
use corpse_core::inventory::{
    displayed_item_magic_nature as magic_nature, potion_appearance, potion_nature,
};
use corpse_core::{
    abilities, combat, crafting, equipment, inventory, progression, projectiles, Command, Field,
    Item, PartialStats, Run,
};
use sdl2::keyboard::{Keycode, Mod};

pub const BOUNDS: (i32, i32, i32, i32) = (213, 210, 760, 500);
const LIST: Rect = Rect(225, 328, 402, 370);
const DETAIL: Rect = Rect(639, 328, 322, 370);
const CLOSE: Rect = Rect(929, 214, 36, 32);
const CANCEL: Rect = Rect(881, 281, 80, 32);
const ROW: i32 = 30;
const EDGE: [u8; 3] = [53, 55, 68];
const PANEL: [u8; 3] = [11, 13, 21];
const SELECTED: [u8; 3] = [53, 51, 68];
const INVALID: [u8; 3] = [255, 184, 199];

#[derive(Clone, Debug, PartialEq)]
pub enum PackTarget {
    Throw(f64),
    Shoot(f64),
    UseAt(f64),
}
impl PackTarget {
    /// UI admission precedes live target validation: never disclose an unseen occupant.
    pub fn target_blocker(&self, run: &Run, x: f64, y: f64) -> Option<&'static str> {
        let Self::UseAt(id) = *self else { return None; };
        if run.simulation_revision.value_or(29.)>=50. {
            if let Some(item)=run.inventory.iter().find(|i|i.id==id).filter(|i|corpse_core::revision50_amulet_effects::is_amulet(i)) {
                return corpse_core::revision50_amulet_effects::target_blocker_ui(run,item,x,y);
            }
        }
        if !run.actors_at(run.depth).iter().any(|a| a.x==x && a.y==y
            && !a.dead.value_or(false) && a.hp>0. && corpse_core::vision::visible_actor(run,a)
            && corpse_core::dreamwalking::same_plane(run,&run.player,a)) {
            return Some("Choose a visible adjacent living creature.");
        }
        let Some(item)=run.inventory.iter().find(|i|i.id==id) else { return Some("That item is no longer in your pack."); };
        corpse_core::borge::use_blocker(run,item,x,y)
    }
    pub fn is_amulet(&self,run:&Run)->bool {
        matches!(self,Self::UseAt(id) if run.simulation_revision.value_or(29.)>=50.
            && run.inventory.iter().any(|i|i.id==*id && corpse_core::revision50_amulet_effects::is_amulet(i)))
    }
    /// Known-only effect footprint; never inspect an unobserved actor or blocker.
    pub fn application_preview(&self,run:&Run,hover:Option<(usize,usize)>)->Vec<(usize,usize)> {
        let Some((x,y))=hover else{return vec![];};
        if let Self::UseAt(id)=self {
            if self.is_amulet(run) {
                return run.inventory.iter().find(|i|i.id==*id).map_or_else(Vec::new,|i|
                    corpse_core::revision50_amulet_effects::preview(run,i,x as f64,y as f64));
            }
        }
        if self.target_blocker(run,x as f64,y as f64).is_none(){vec![(x,y)]}else{vec![]}
    }
    /// The same detached result feeds mouse and keyboard aiming. Painting never
    /// re-enters simulation or asks live collision questions.
    pub fn aim_preview(&self,run:&Run,hover:Option<(usize,usize)>)->corpse_core::revision50_amulet_effects::AimPreview {
        let mut out=corpse_core::revision50_amulet_effects::AimPreview::default();
        let Some((x,y))=hover else{return out};
        if let Self::UseAt(id)=self {
            if self.is_amulet(run) {
                return run.inventory.iter().find(|i|i.id==*id).map_or(out,|item|
                    corpse_core::revision50_amulet_effects::aim_preview(run,item,x as f64,y as f64));
            }
            out.area=self.application_preview(run,hover);return out;
        }
        let trace=crate::targeting_ui::observe(run,Some(crate::targeting_ui::Point{x,y}));
        out.delivery=trace.path.into_iter().map(|p|(p.x,p.y)).collect();
        if matches!(trace.stop_reason,crate::targeting_ui::StopReason::Target|crate::targeting_ui::StopReason::Creature|crate::targeting_ui::StopReason::Terrain) {
            out.impact=out.delivery.last().copied();
        }
        out.possible_exits=trace.possible_exits.into_iter().map(|p|(p.x,p.y)).collect();out
    }
    pub fn command(&self, x: f64, y: f64) -> Command {
        match *self {
            Self::Throw(id) => Command::Throw { id, x, y },
            Self::Shoot(id) => Command::Shoot { id, x, y },
            Self::UseAt(id) => Command::UseAt { id, x, y },
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum PackAction {
    /// Still consumed by the modal owner, including clicks outside its bounds.
    None,
    Close,
    Command(Command),
    Target(PackTarget),
    /// Persist `inventory_display_order`; no command, RNG, or turn is involved.
    Order(Vec<f64>),
    Notice(String),
}
#[derive(Clone, Debug, PartialEq)]
enum Request {
    Craft { heart: f64, craft: String },
    Tinker { ability: String },
}
#[derive(Clone)]
enum ButtonAction {
    Command(Command),
    Target(PackTarget),
    Craft { heart: f64, craft: String },
}
#[derive(Clone)]
struct Button {
    label: String,
    action: ButtonAction,
    blocker: Option<String>,
    tip: String,
}
#[derive(Clone)]
struct Group {
    item: Item,
    items: Vec<Item>,
    count: f64,
    name: String,
    description: String,
    actions: Vec<Button>,
    section: usize,
}
#[derive(Clone, PartialEq)]
struct Stamp {
    revision: f64,
    category_summary: String,
    category_headers: Vec<String>,
    run_id: Field<String>,
    seed: String,
    player: f64,
    tick: f64,
    depth: f64,
    phase: Field<String>,
    inventory: Vec<Item>,
    abilities: Vec<String>,
    energy: f64,
    ability_reserves: Vec<(String,f64,f64)>,
    resource_history: Vec<(String, corpse_core::ResourceMeterHistory)>,
    status: String,
    offers: Vec<String>,
    blocked: bool,
    dreamwalking: bool,
    teleport_cooldown: u32,
    tinker_blocker: Option<String>,
}
impl Stamp {
    fn observe(run: &Run) -> Self {
        let category_headers = category_headers(run);
        Self {
            category_summary: category_headers.join(" · "),
            category_headers,
            revision: run.simulation_revision.value_or(29.),
            run_id: run.run_id.clone(),
            seed: run.seed.clone(),
            player: run.player.id,
            tick: run.tick,
            depth: run.depth,
            phase: run.phase.clone(),
            inventory: run.inventory.clone(),
            abilities: run.player.abilities.clone(),
            energy: run.player.energy,
            ability_reserves: if run.simulation_revision.value_or(29.)>=50. {
                run.player.ability_reserves.as_ref().into_iter().flat_map(|m|m.iter())
                    .map(|(id,s)|(id.clone(),s.energy,s.capacity)).collect()
            } else {Vec::new()},
            resource_history: run.resource_history.as_ref().and_then(|h|h.actors.get(&run.player.id.to_string())).into_iter().flat_map(|m|m.iter()).map(|(key,value)|(key.clone(),value.clone())).collect(),
            status: run.status.clone(),
            dreamwalking: corpse_core::dreamwalking::active(run),
            teleport_cooldown: corpse_core::ahvamitt::cooldown(run),
            offers: run.offers.clone(),
            tinker_blocker: run
                .player
                .abilities
                .iter()
                .any(|id| id == "kative-tinker")
                .then(|| tinker_blocker(run))
                .flatten(),
            blocked: run
                .player
                .conditions
                .get("deathResolutionBlocked")
                .is_some_and(|n| *n != 0.),
        }
    }
    fn playable(&self) -> bool {
        self.status == "playing" && self.offers.is_empty() && !self.blocked
    }
}
/// Observation is O(I² + I*A + G log G + T) time / O(I + T) extra memory, where
/// I physical item records (unbounded for R50 trinkets), A the fixed ability
/// catalog, G <= I display groups, and T projected tooltip text. Typed equality
/// preserves optional identity fields except obsolete R50 jewelry equip flags;
/// no JSON or gameplay RNG is used for grouping.
pub struct PackView {
    layout: PackLayout,
    content_metrics_dirty: bool,
    paint_epoch: u64,
    #[cfg(test)]
    configure_rebuilds: usize,
    #[cfg(test)]
    refresh_count: usize,
    metric_signature: Option<(
        crate::typography_ui::Overrides,
        crate::typography_ui::Overrides,
    )>,
    text_metrics: Option<TextMetrics>,
    minimum_list_width: i32,
    button_metrics: Option<TextMetrics>,
    groups: Vec<Group>,
    stamp: Stamp,
    slots: f64,
    capacity: f64,
    selected: Option<f64>,
    request: Option<Request>,
    list_scroll: i32,
    detail_scroll: i32,
    hover_at: Option<(i32, i32)>,
    read_only: bool,
    pending: bool,
    focus_action: Option<usize>,
    quick_kind: Option<&'static str>,
    quick_order: Vec<f64>,
}
/// Small owned UI descriptor; item groups and canonical Run are never cloned here.
#[derive(Clone,Debug,PartialEq)]
pub struct PackPaintState {
    pub bounds:Rect,epoch:u64,selected:Option<u64>,request:Option<Request>,list_scroll:i32,detail_scroll:i32,read_only:bool,pending:bool,focus_action:Option<usize>,hover:Vec<Rect>,tooltip:Option<PackTooltip>,rows:Vec<Rect>,detail:Rect,header:Rect,
}
#[derive(Clone,Debug,PartialEq)]
struct PackTooltip{bounds:Rect,lines:Vec<String>,line_height:i32}
fn intersect_rect(a:Rect,b:Rect)->Option<Rect>{let x=a.0.max(b.0);let y=a.1.max(b.1);let right=(a.0+a.2).min(b.0+b.2);let bottom=(a.1+a.3).min(b.1+b.3);(x<right&&y<bottom).then(||Rect(x,y,right-x,bottom-y))}
impl PackPaintState {
    /// O(hover regions + tooltip text). Returns old/new logical footprints, not pixel diffs.
    pub fn changes(&self,old:&Self)->Vec<Rect>{
        if self.bounds!=old.bounds||self.epoch!=old.epoch||self.selected!=old.selected||self.request!=old.request||self.list_scroll!=old.list_scroll||self.detail_scroll!=old.detail_scroll||self.read_only!=old.read_only||self.pending!=old.pending||self.focus_action!=old.focus_action{return vec![old.bounds,self.bounds];}
        let mut out=Vec::new();
        if self.hover!=old.hover{out.extend_from_slice(&old.hover);out.extend_from_slice(&self.hover);}
        if self.tooltip!=old.tooltip{if let Some(t)=&old.tooltip{out.push(t.bounds);}if let Some(t)=&self.tooltip{out.push(t.bounds);}}
        self.expand_damage(&mut out);out
    }
    /// O(G*R + G), G visible rows and R input rectangles. Complete text areas are
    /// restored before adaptive contrast reads them; tooltip can connect rows.
    pub fn expand_damage(&self,regions:&mut Vec<Rect>){
        if regions.is_empty(){return;}let original=regions.clone();
        let candidates=self.rows.iter().copied().chain([self.detail,self.header]).collect::<Vec<_>>();
        let mut selected=candidates.iter().map(|c|original.iter().any(|r|intersect_rect(*c,*r).is_some())).collect::<Vec<_>>();
        let tip=self.tooltip.as_ref().filter(|t|original.iter().any(|r|intersect_rect(t.bounds,*r).is_some())||candidates.iter().zip(&selected).any(|(r,s)|*s&&intersect_rect(t.bounds,*r).is_some()));
        if let Some(t)=tip{regions.push(t.bounds);for(i,r)in candidates.iter().enumerate(){selected[i]|=intersect_rect(t.bounds,*r).is_some();}}
        regions.extend(candidates.into_iter().zip(selected).filter_map(|(r,s)|s.then_some(r)));
    }
}
impl PackView {
    pub fn observe(run: &Run) -> Self {
        Self {
            layout: PackLayout::resolve(
                Profile::defaults(),
                modal_ui::Frame::from_profile(Profile::defaults()).map,
            ),
            content_metrics_dirty: true,
            paint_epoch: 0,
            #[cfg(test)]
            configure_rebuilds: 0,
            #[cfg(test)]
            refresh_count: 0,
            metric_signature: None,
            text_metrics: None,
            minimum_list_width: 402,
            button_metrics: None,
            groups: display_groups(run),
            quick_kind: None,
            quick_order: Vec::new(),
            stamp: Stamp::observe(run),
            slots: pack_limited_occupancy(run),
            capacity: pack_capacity(run),
            selected: None,
            request: None,
            list_scroll: 0,
            detail_scroll: 0,
            hover_at: None,
            read_only: false,
            pending: false,
            focus_action: None,
        }
    }
    /// Warm O(fixed layout/style fields), no group/text scan when unchanged.
    /// Content/style/layout invalidation costs O(group label bytes + selected detail wrapping).
    /// Detached metrics never observe or rebuild simulation state.
    pub fn configure(&mut self, p: &Painter, map: (i32, i32, i32, i32)) {
        let signature = (
            crate::typography_ui::overrides(&p.profile, Role::Inventory),
            crate::typography_ui::overrides(&p.profile, Role::Primary),
        );
        let changed = self.metric_signature.as_ref() != Some(&signature);
        let area = Rect(0, map.1, p.logical_width(), map.3);
        let mut layout = PackLayout::resolve_measured(&p.profile, area, self.minimum_list_width);
        // Row height is content-derived; geometry/style/content invalidate it together.
        layout.row = self.layout.row;
        if !changed && !self.content_metrics_dirty && self.layout==layout && self.text_metrics.is_some() && self.button_metrics.is_some(){return;}
        let _timing=corpse_core::timing::span("desktop.pack.configure_rebuild");
        #[cfg(test)]{self.configure_rebuilds+=1;}
        self.paint_epoch=self.paint_epoch.wrapping_add(1);
        self.layout=layout;
        if changed || self.text_metrics.is_none() {
            self.text_metrics = Some(TextMetrics::new(p, Role::Inventory, 14, 18));
            self.button_metrics = Some(TextMetrics::new(p, Role::Primary, 14, 18));
        }
        self.metric_signature = Some(signature);
        self.content_metrics_dirty=false;
        for g in &self.groups {
            self.text_metrics.as_mut().unwrap().ensure(p, &g.name);
            self.text_metrics
                .as_mut()
                .unwrap()
                .ensure(p, &g.description);
            for b in &g.actions {
                self.button_metrics.as_mut().unwrap().ensure(p, &b.label);
            }
        }
        // O(A*E + label bytes), only on content/style/geometry invalidation.
        // Reserve room even before the longest legal Tinkered amulet is acquired.
        let row_labels: Vec<_> = self.groups.iter().map(|g| self.row_label(g)).collect();
        let metrics = self.text_metrics.as_mut().unwrap();
        let longest = longest_tinkered_amulet_name(p, self.stamp.revision);
        metrics.ensure(p, &longest);
        let mut width = metrics.width(&longest);
        for label in &row_labels { metrics.ensure(p, label); width = width.max(metrics.width(label)); }
        self.minimum_list_width = width.ceil() as i32 + 58 + 100 + 8 + 4;
        self.layout = PackLayout::resolve_measured(&p.profile, area, self.minimum_list_width);
        // At very narrow viewports or large user fonts, wrap instead of clipping.
        let name_width = (self.layout.list.2 - 58 - 100 - 8 - 4).max(1);
        let lines = row_labels.iter().map(|label| metrics.wrap(label, name_width).len()).max().unwrap_or(1);
        self.layout.row = self.layout.row.max(lines as i32 * metrics.line + 12);
        self.list_scroll = self.list_scroll.clamp(0, self.list_max_scroll());
        self.detail_scroll = self.detail_scroll.clamp(0, self.detail_max_scroll());
    }
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        self.layout.bounds.tuple()
    }
    fn wrap_detail(&self, text: &str) -> Vec<String> {
        self.text_metrics.as_ref().map_or_else(
            || wrap(text, 44),
            |m| m.wrap(text, (self.layout.detail.2 - 14).max(1)),
        )
    }
    fn detail_line_height(&self) -> i32 {
        self.text_metrics.as_ref().map_or(18, |m| m.line)
    }
    fn button_height(&self) -> i32 {
        self.layout
            .button_height
            .max(self.button_metrics.as_ref().map_or(26, |m| m.line) + 6)
    }
    /// O(I² + I*A + G log G + T), explicit content refresh; never called by hover redraw.
    pub fn refresh(&mut self, run: &Run) {
        let _timing=corpse_core::timing::span("desktop.pack.refresh");
        self.paint_epoch=self.paint_epoch.wrapping_add(1);
        self.content_metrics_dirty=true;
        #[cfg(test)]{self.refresh_count+=1;}
        let stamp = Stamp::observe(run);
        if stamp != self.stamp {
            self.request = None;
        }
        self.stamp = stamp;
        self.groups = display_groups(run);
        if let Some(kind)=self.quick_kind {
            self.quick_order=self.groups.iter().flat_map(|g|g.items.iter().map(|i|i.id)).collect();
            self.groups.retain(|g|g.item.kind==kind);
        }
        self.slots = pack_limited_occupancy(run);
        self.capacity = pack_capacity(run);
        if !self.groups.iter().any(|g| Some(g.item.id) == self.selected) {
            self.selected = None;
            self.detail_scroll = 0;
        }
        self.list_scroll = self.list_scroll.clamp(0, self.list_max_scroll());
        self.detail_scroll = self.detail_scroll.clamp(0, self.detail_max_scroll());
        self.pending = false;
        if self
            .focus_action
            .is_some_and(|i| self.selected_group().is_none_or(|g| i >= g.actions.len()))
        {
            self.focus_action = None;
        }
    }
    pub fn focus_kind(&mut self,run:&Run,kind:Option<&'static str>){
        self.quick_kind=kind;
        self.quick_order.clear();
        self.cancel_selection();
        self.selected=None;
        self.list_scroll=0;
        self.detail_scroll=0;
        self.refresh(run);
        if kind.is_some(){
            if let Some(group)=self.groups.first(){self.selected=Some(group.item.id);self.reveal_selected();}
        }
    }
    pub fn display_order(&self) -> Vec<f64> {
        if self.quick_kind.is_some(){return self.quick_order.clone();}
        self.groups
            .iter()
            .flat_map(|g| g.items.iter().map(|i| i.id))
            .collect()
    }
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
        if read_only {
            self.cancel_selection();
        }
    }
    pub fn cancel_selection(&mut self) {
        self.request = None;
        self.focus_action = None;
        self.hover_at = None;
    }
    /// O(1), pointer-only state; identical motion needs no redraw, no content observation.
    pub fn hover(&mut self, x: i32, y: i32)->bool {
        let changed=self.hover_at!=Some((x,y));self.hover_at=Some((x,y));changed
    }
    pub fn begin_tinker(&mut self, run: &Run, ability: &str) -> PackAction {
        self.refresh(run);
        if self.read_only
            || !self.stamp.playable()
            || !run.player.abilities.iter().any(|a| a == ability)
        {
            return PackAction::Notice(
                "That action is unavailable. Choose it again when ready.".into(),
            );
        }
        if !abilities::supported(ability) {
            return PackAction::Notice("Tinker is not implemented in this native build.".into());
        }
        if let Some(reason) = &self.stamp.tinker_blocker {
            return PackAction::Notice(reason.clone());
        }
        self.request = Some(Request::Tinker {
            ability: ability.into(),
        });
        self.focus_action = None;
        self.select_first_eligible();
        PackAction::None
    }
    fn selected_group(&self) -> Option<&Group> {
        self.groups
            .iter()
            .find(|g| Some(g.item.id) == self.selected)
    }
    fn selected_index(&self) -> Option<usize> {
        self.groups
            .iter()
            .position(|g| Some(g.item.id) == self.selected)
    }
    fn select_first_eligible(&mut self) {
        if let Some(g) = self
            .groups
            .iter()
            .find(|g| g.items.iter().any(|i| self.target_blocker(i).is_none()))
        {
            self.selected = Some(g.item.id);
            self.detail_scroll = 0;
            self.reveal_selected();
        }
    }
    fn target_blocker(&self, item: &Item) -> Option<String> {
        match self.request.as_ref()? {
            Request::Tinker { .. } => {
                if item.tinkered.value_or(false) {
                    Some(
                        "You've already tinkered with this item."
                            .into(),
                    )
                } else if self.stamp.revision < 50. && item.granted_ability.as_ref().is_some_and(|s| !s.is_empty()) {
                    Some("This item already grants an ability; Tinker cannot replace it.".into())
                } else if !corpse_core::tinker::target_eligible_for_revision(item, self.stamp.revision) {
                    Some(if self.stamp.revision >= 50. { "Choose a ring or amulet that has not been tinkered." } else { "Choose an unblessed weapon, armor, ring or amulet." }.into())
                } else {
                    None
                }
            }
            Request::Craft { .. } => {
                if !equipment::equippable(item) {
                    return Some("Choose an equippable item for this heart craft.".into());
                }
                let limit = if self
                    .stamp
                    .abilities
                    .iter()
                    .any(|s| s == "kative-divine-craftsman")
                {
                    2
                } else {
                    1
                };
                (item.enhancements.len() >= limit).then(|| {
                    format!(
                        "Invalid. This item already has {limit} enhancement{}.",
                        if limit == 1 { "" } else { "s" }
                    )
                })
            }
        }
    }
    fn target_member<'a>(&self, g: &'a Group) -> &'a Item {
        g.items
            .iter()
            .find(|i| self.target_blocker(i).is_none())
            .unwrap_or(&g.item)
    }
    fn select_target(&mut self, index: usize) -> PackAction {
        if self.pending || self.read_only {
            return PackAction::None;
        }
        let group = &self.groups[index];
        self.selected = Some(group.item.id);
        self.detail_scroll = 0;
        let item = self.target_member(group);
        if let Some(blocker) = self.target_blocker(item) {
            return PackAction::Notice(blocker);
        }
        let id = item.id;
        let command = match self.request.take().unwrap() {
            Request::Tinker { ability } => Command::Ability {
                id: ability,
                target: id.into(),
                x: Field::Missing,
                y: Field::Missing,
            },
            Request::Craft { heart, craft } => Command::Craft {
                heart,
                item: id,
                craft,
            },
        };
        self.pending = true;
        PackAction::Command(command)
    }
    fn activate(&mut self, index: usize) -> PackAction {
        if self.pending || self.read_only || self.request.is_some() || !self.stamp.playable() {
            return PackAction::None;
        }
        let Some(button) = self
            .selected_group()
            .and_then(|g| g.actions.get(index))
            .cloned()
        else {
            return PackAction::None;
        };
        if let Some(blocker) = button.blocker {
            return PackAction::Notice(blocker);
        }
        match button.action {
            ButtonAction::Command(c) => {
                self.pending = true;
                PackAction::Command(c)
            }
            ButtonAction::Target(t) => {
                self.pending = true;
                PackAction::Target(t)
            }
            ButtonAction::Craft { heart, craft } => {
                if !self.groups.iter().any(|g| equipment::equippable(&g.item)) {
                    return PackAction::Notice(
                        "Invalid: no equippable item in your inventory.".into(),
                    );
                }
                self.request = Some(Request::Craft { heart, craft });
                self.focus_action = None;
                self.select_first_eligible();
                PackAction::None
            }
        }
    }
    fn reorder_neighbor(&self, index: usize, delta: isize) -> Option<usize> {
        let other = index.checked_add_signed(delta)?;
        (self.groups.get(index)?.section == self.groups.get(other)?.section).then_some(other)
    }
    fn move_group(&mut self, index: usize, delta: isize) -> PackAction {
        if self.pending || self.read_only || self.request.is_some() || self.quick_kind.is_some() {
            return PackAction::None;
        }
        let Some(other) = self.reorder_neighbor(index, delta) else { return PackAction::None; };
        self.groups.swap(index, other);
        self.paint_epoch=self.paint_epoch.wrapping_add(1);
        PackAction::Order(self.display_order())
    }
    pub fn click(&mut self, x: i32, y: i32) -> PackAction {
        if self.layout.close.contains(x, y) {
            self.cancel_selection();
            return PackAction::Close;
        }
        if self.request.is_some() && self.layout.cancel.contains(x, y) {
            self.cancel_selection();
            return PackAction::None;
        }
        if self.pending {
            return PackAction::None;
        }
        if self.layout.list.contains(x, y) {
            for (index, yy) in self.row_positions() {
                let yy = yy - self.list_scroll;
                if Rect(self.layout.list.0, yy, self.layout.list.2, self.layout.row).contains(x, y)
                {
                    if self.request.is_some() {
                        return self.select_target(index);
                    }
                    if !self.read_only && x >= self.layout.list.0 + self.layout.list.2 - 52 {
                        return self.move_group(
                            index,
                            if x < self.layout.list.0 + self.layout.list.2 - 26 {
                                -1
                            } else {
                                1
                            },
                        );
                    }
                    let id = self.groups[index].item.id;
                    self.selected = if self.selected == Some(id) {
                        None
                    } else {
                        Some(id)
                    };
                    self.detail_scroll = 0;
                    self.focus_action = None;
                    return PackAction::None;
                }
            }
        }
        if self.layout.detail.contains(x, y) && self.request.is_none() && !self.read_only {
            for (index, rect) in self.button_rects() {
                if rect.contains(x, y) {
                    return self.activate(index);
                }
            }
        }
        PackAction::None
    }
    /// Test-only label resolution; O(A) selected action labels, no mutation.
    /// Keyboard injection still activates the actual public UI path.
    #[cfg(test)]
    pub(crate) fn keyboard_tabs_for_action_label(&self,label:&str)->Option<usize>{
        self.selected_group()?.actions.iter().position(|b|b.label==label).map(|i|i+1)
    }
    pub fn key(&mut self, key: Keycode, modifiers: Mod) -> PackAction {
        if key == Keycode::Escape {
            if self.request.is_some() {
                self.cancel_selection();
                return PackAction::None;
            }
            return PackAction::Close;
        }
        if key == Keycode::I {
            self.cancel_selection();
            return PackAction::Close;
        }
        if self.pending {
            return PackAction::None;
        }
        match key {
            Keycode::Up | Keycode::Down | Keycode::Home | Keycode::End => {
                if self.groups.is_empty() {
                    return PackAction::None;
                }
                let i = match key {
                    Keycode::Home => 0,
                    Keycode::End => self.groups.len() - 1,
                    Keycode::Up => self.selected_index().unwrap_or(1).saturating_sub(1),
                    _ => self
                        .selected_index()
                        .map_or(0, |i| (i + 1).min(self.groups.len() - 1)),
                };
                self.selected = Some(self.groups[i].item.id);
                self.detail_scroll = 0;
                self.focus_action = None;
                self.reveal_selected();
            }
            Keycode::PageUp => self.list_scroll = (self.list_scroll - self.layout.list.3).max(0),
            Keycode::PageDown => {
                self.list_scroll =
                    (self.list_scroll + self.layout.list.3).min(self.list_max_scroll())
            }
            Keycode::Tab if !self.read_only && self.request.is_none() => {
                let len = self.selected_group().map_or(0, |g| g.actions.len());
                if len > 0 {
                    self.focus_action =
                        Some(if modifiers.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                            self.focus_action.map_or(len - 1, |i| (i + len - 1) % len)
                        } else {
                            self.focus_action.map_or(0, |i| (i + 1) % len)
                        });
                    if let Some((_, rect)) = self
                        .button_rects()
                        .into_iter()
                        .find(|(i, _)| Some(*i) == self.focus_action)
                    {
                        if rect.1 < self.layout.detail.1 {
                            self.detail_scroll =
                                (self.detail_scroll - (self.layout.detail.1 - rect.1)).max(0);
                        } else if rect.1 + rect.3 > self.layout.detail.1 + self.layout.detail.3 {
                            self.detail_scroll +=
                                rect.1 + rect.3 - self.layout.detail.1 - self.layout.detail.3;
                        }
                    }
                }
            }
            Keycode::Return | Keycode::KpEnter | Keycode::Space => {
                if let Some(index) = self.selected_index() {
                    if self.request.is_some() {
                        return self.select_target(index);
                    }
                }
                if let Some(index) = self.focus_action {
                    return self.activate(index);
                }
            }
            _ => {}
        }
        PackAction::None
    }
    pub fn scroll(&mut self, dy: i32, x: i32, y: i32) {
        if self.layout.list.contains(x, y) {
            self.list_scroll =
                (self.list_scroll - dy * self.layout.row * 3).clamp(0, self.list_max_scroll());
        } else if self.layout.detail.contains(x, y) {
            self.detail_scroll = (self.detail_scroll - dy * self.detail_line_height() * 3)
                .clamp(0, self.detail_max_scroll());
        }
        self.hover_at = None;
    }
    /// O(G + S) time and O(S) memory: five R50 sections or two legacy sections.
    /// Unsupported direct projections stay visible without claiming trinket space.
    fn section_positions(&self) -> Vec<(usize, i32)> {
        if let Some(kind)=self.quick_kind {
            let section=if self.stamp.revision<50. {1} else if kind=="heart" {4} else {3};
            return vec![(section,self.layout.list.1)];
        }
        let count = if self.stamp.revision >= 50. { self.stamp.category_headers.len() } else { 2 };
        let mut y = self.layout.list.1;
        let mut group = 0;
        let mut sections = Vec::with_capacity(count);
        for section in 0..count {
            sections.push((section, y));
            y += 24;
            while group < self.groups.len() && self.groups[group].section == section {
                y += self.layout.row;
                group += 1;
            }
        }
        sections
    }
    fn row_positions(&self) -> Vec<(usize, i32)> {
        if self.quick_kind.is_some(){
            return self.groups.iter().enumerate().map(|(index,_)|
                (index,self.layout.list.1+24+index as i32*self.layout.row)).collect();
        }
        let sections = self.section_positions();
        let mut offsets = vec![0; sections.len()];
        self.groups.iter().enumerate().map(|(index, group)| {
            let y = sections[group.section].1 + 24 + offsets[group.section];
            offsets[group.section] += self.layout.row;
            (index, y)
        }).collect()
    }
    fn list_max_scroll(&self) -> i32 {
        let height = if self.quick_kind.is_some(){24+self.layout.row*self.groups.len() as i32} else if self.stamp.revision >= 50. {
            self.stamp.category_headers.len() as i32 * 24 + self.layout.row * self.groups.len() as i32
        } else {
            48 + self.layout.row * self.groups.len().max(20) as i32
        };
        (height - self.layout.list.3).max(0)
    }
    fn reveal_selected(&mut self) {
        if let Some((_, y)) = self
            .row_positions()
            .into_iter()
            .find(|(i, _)| Some(self.groups[*i].item.id) == self.selected)
        {
            if y - self.list_scroll < self.layout.list.1 {
                self.list_scroll = y - self.layout.list.1;
            } else if y + self.layout.row - self.list_scroll
                > self.layout.list.1 + self.layout.list.3
            {
                self.list_scroll = y + self.layout.row - self.layout.list.1 - self.layout.list.3;
            }
        }
    }
    fn detail_lines(&self) -> Vec<(String, [u8; 3])> {
        let Some(g) = self.selected_group() else {
            return vec![];
        };
        let mut lines = vec![];
        lines.extend(self.wrap_detail(&g.name).into_iter().map(|s| (s, INK)));
        lines.push((String::new(), MUTED));
        for (index,s) in g.description.split('\n').enumerate() {
            // The selected pane already paints the canonical jewelry title.
            // Keep it in the complete description for detached tooltips/ground.
            if matches!(g.item.kind.as_str(),"ring"|"amulet")
                && index == 0 && s == g.name { continue; }
            lines.extend(self.wrap_detail(s).into_iter().map(|s| (s, MUTED)));
        }
        if g.count > 1. {
            let text = if self.stamp.revision >= 50. && corpse_core::revision50_inventory::category(&g.item)
                == Some(corpse_core::revision50_inventory::Category::Trinket) {
                format!("{} trinket stack{} · unlimited capacity", g.items.len(), if g.items.len()==1 {""} else {"s"})
            } else if self.stamp.revision >= 50. {
                format!("{} {} slots · actions use one", g.count, g.item.kind)
            } else if inventory::is_arrow(&g.item) {
                "1 bag slot · actions use one arrow".to_string()
            } else {
                format!("{} bag slots · actions use one", g.count)
            };
            lines.push((String::new(), MUTED));
            lines.extend(self.wrap_detail(&text).into_iter().map(|s| (s, MUTED)));
        }
        if self.request.is_some() {
            let blocker = self.target_blocker(self.target_member(g));
            lines.push((String::new(), MUTED));
            lines.extend(
                self.wrap_detail(blocker.as_deref().unwrap_or(""))
                    .into_iter()
                    .map(|s| (s, if blocker.is_some() { INVALID } else { GOLD })),
            );
        }
        lines
    }
    fn button_rects(&self) -> Vec<(usize, Rect)> {
        if self.read_only || self.request.is_some() {
            return vec![];
        }
        let Some(g) = self.selected_group() else {
            return vec![];
        };
        let mut x = self.layout.detail.0 + 10;
        let mut y = self.layout.detail.1
            + self.detail_lines().len() as i32 * self.detail_line_height()
            + 12
            - self.detail_scroll;
        let mut result = vec![];
        for (i, b) in g.actions.iter().enumerate() {
            let width = (self
                .button_metrics
                .as_ref()
                .map_or(b.label.chars().count() as f64 * 8., |m| m.width(&b.label))
                .ceil() as i32
                + self.layout.button_padding * 2)
                .min((self.layout.detail.2 - 10).max(1));
            if x + width > self.layout.detail.0 + self.layout.detail.2 {
                x = self.layout.detail.0 + 10;
                y += self.button_height() + 5;
            }
            result.push((i, Rect(x, y, width, self.button_height())));
            x += width + 5;
        }
        result
    }
    fn detail_max_scroll(&self) -> i32 {
        let content = self.button_rects().last().map_or(
            self.detail_lines().len() as i32 * self.detail_line_height(),
            |(_, r)| r.1 + r.3 + self.detail_scroll - self.layout.detail.1 + 8,
        );
        (content - self.layout.detail.3).max(0)
    }
    fn row_label(&self, g: &Group) -> String {
        format!("{}{}{}", g.name,
            if g.count>1. {format!(" ×{}",g.count)} else {String::new()},
            if self.stamp.revision<50. && crafting::is_equipped(&g.item) {
                format!(" [{}]",g.item.equipped.as_ref().unwrap())
            } else {String::new()})
    }
    fn capacity_heading(&self) -> String {
        if self.stamp.revision >= 50. { String::new() }
        else { format!("{} / {} slots", self.slots, self.capacity) }
    }
    fn hint(&self) -> String {
        match &self.request {
            Some(Request::Tinker { .. }) if self.stamp.revision >= 50. => "Tinker: choose an untinkered ring or amulet.".into(),
            Some(Request::Tinker { .. }) => "Tinker: click an unblessed weapon, armor, ring or amulet in your pack or equipped items.".into(),
            Some(Request::Craft { heart, craft }) => format!("{}: click equipment to craft using {}.", ability_name(craft), self.stamp.inventory.iter().find(|i| i.id == *heart).map_or("your heart", |i| &i.name)),
            None => String::new(),
        }
    }
    fn tooltip(&self) -> Option<String> {
        let (x, y) = self.hover_at?;
        if self.request.is_some()
            && Rect(
                self.layout.list.0,
                self.layout.bounds.1 + 68,
                self.layout.cancel.0 - self.layout.list.0 - 8,
                42,
            )
            .contains(x, y)
        {
            return Some(self.hint());
        }
        if self.layout.list.contains(x, y) {
            for (i, yy) in self.row_positions() {
                if Rect(
                    self.layout.list.0,
                    yy - self.list_scroll,
                    self.layout.list.2,
                    self.layout.row,
                )
                .contains(x, y)
                {
                    let g = &self.groups[i];
                    if self.request.is_some() {
                        let hint = self.target_blocker(self.target_member(g))
                            .unwrap_or_else(|| "Select this item".into());
                        return Some(if self.stamp.revision >= 50. {
                            format!("{}\n{hint}", g.description)
                        } else { hint });
                    }
                    if !self.read_only && x >= self.layout.list.0 + self.layout.list.2 - 52 {
                        let delta = if x < self.layout.list.0 + self.layout.list.2 - 26 { -1 } else { 1 };
                        if self.stamp.revision >= 50. && self.reorder_neighbor(i, delta).is_none() {
                            return Some("Items stay within their category (no turn).".into());
                        }
                        return Some(format!(
                            "Move {} {} in your pack (no turn)",
                            g.name,
                            if x < self.layout.list.0 + self.layout.list.2 - 26 {
                                "up"
                            } else {
                                "down"
                            }
                        ));
                    }
                    return Some(if self.stamp.revision >= 50. { g.description.clone() } else { g.name.clone() });
                }
            }
        }
        if self.layout.detail.contains(x, y) {
            for (i, rect) in self.button_rects() {
                if rect.contains(x, y) {
                    let b = &self.selected_group()?.actions[i];
                    return Some(b.blocker.clone().unwrap_or_else(|| b.tip.clone()));
                }
            }
        }
        None
    }
    /// O(I + tooltip text), detached visual descriptor. No item/world observation.
    pub fn paint_state(&self,p:&Painter)->PackPaintState {
        let mut hover=Vec::new();
        if let Some((x,y))=self.hover_at {
            for(i,yy)in self.row_positions(){let r=Rect(self.layout.list.0,yy-self.list_scroll,self.layout.list.2,self.layout.row);if Some(self.groups[i].item.id)!=self.selected&&r.contains(x,y)&&self.layout.list.contains(x,y){if let Some(r)=intersect_rect(r,self.layout.list){hover.push(r);}}}
            if self.layout.detail.contains(x,y){for(i,r)in self.button_rects(){if self.focus_action!=Some(i)&&r.contains(x,y){if let Some(r)=intersect_rect(r,self.layout.detail){hover.push(r);}}}}
        }
        let tooltip=self.tooltip().filter(|s|!s.is_empty()).zip(self.hover_at).map(|(text,(x,y))|{
            let width=(p.profile.number("ui.layout.tooltipMaxWidth").round()as i32).min((self.layout.bounds.2-8).max(1));
            let lines=p.wrap_text_role(Role::Tooltips,&text,(width-16).max(1),13);
            let line_height=TextMetrics::new(p,Role::Tooltips,13,18).line;
            let height=(lines.len()as i32*line_height+12).min((self.layout.bounds.3-8).max(1));
            let tx=x.clamp(self.layout.bounds.0+4,(self.layout.bounds.0+self.layout.bounds.2-width-4).max(self.layout.bounds.0+4));
            let ty=(y+24).min(self.layout.bounds.1+self.layout.bounds.3-height-4).max(self.layout.bounds.1+4);
            PackTooltip{bounds:Rect(tx,ty,width,height),lines,line_height}
        });
        let positions=self.row_positions();
        let mut rows=positions.into_iter().filter_map(|(_,y)|intersect_rect(Rect(self.layout.list.0,y-self.list_scroll,self.layout.list.2,self.layout.row),self.layout.list)).collect::<Vec<_>>();
        for (_,y) in self.section_positions(){if let Some(r)=intersect_rect(Rect(self.layout.list.0,y-self.list_scroll,self.layout.list.2,24),self.layout.list){rows.push(r);}}
        let header=Rect(self.layout.bounds.0,self.layout.bounds.1,self.layout.bounds.2,(self.layout.list.1-self.layout.bounds.1).max(0));
        PackPaintState{rows,detail:self.layout.detail,header,bounds:self.layout.bounds,epoch:self.paint_epoch,selected:self.selected.map(f64::to_bits),request:self.request.clone(),list_scroll:self.list_scroll,detail_scroll:self.detail_scroll,read_only:self.read_only,pending:self.pending,focus_action:self.focus_action,hover,tooltip}
    }
    /// Inventory action feedback uses the actual visible matching item row. A
    /// consumed or scrolled-away item falls back to the Pack heading, not a map cue.
    pub fn paint_action_marks(&self,p:&mut Painter,cues:&[crate::effects::Cue],now:f64){
        for (id,mark,color,alpha) in crate::effects::inventory_marks(cues,now){
            if id.is_none()&&mark.style!="pack-capacity" {continue}
            let row=id.and_then(|id|self.row_positions().into_iter().find(|(i,_)|self.groups[*i].item.id==id))
                .map(|(_,y)|y-self.list_scroll).filter(|y|*y>=self.layout.list.1&&*y+self.layout.row<=self.layout.list.1+self.layout.list.3);
            let (x,y,w,h)=row.map_or((self.layout.bounds.0+85,self.layout.bounds.1+12,22,22),|y|(self.layout.list.0+2,y,22,self.layout.row));
            crate::action_marks::paint(p,&mark,x,y,w,h,color,alpha);
        }
    }
    pub fn paint(&self,p:&mut Painter){let state=self.paint_state(p);self.paint_prepared(p,&state);}
    /// Paints the same full logical composition through the independent native damage scissor.
    /// Prepared tooltip layout is reused, so damage discovery does not wrap it twice.
    pub fn paint_prepared(&self, p: &mut Painter,state:&PackPaintState) {
        let old_clip = p.clip;
        p.clip = Some(self.layout.bounds.tuple());
        p.rect(
            self.layout.bounds.0,
            self.layout.bounds.1,
            self.layout.bounds.2,
            self.layout.bounds.3,
            p.profile.rgb("ui.colors.panel"),
        );
        modal_ui::border(p, self.layout.bounds, p.profile.rgb("ui.colors.border"));
        p.text_role(
            Role::Headings,
            match self.quick_kind {Some("heart")=>"Hearts",Some("potion")=>"Potions",_=>"Pack"},
            self.layout.list.0,
            self.layout.bounds.1 + 11,
            18,
            p.profile.rgb("ui.colors.headings"),
        );
        p.text_role(
            Role::Primary,
            "×",
            self.layout.close.0 + 10,
            self.layout.close.1 + 1,
            24,
            p.profile.rgb("ui.colors.primary"),
        );
        p.text(
            &self.capacity_heading(),
            self.layout.list.0,
            self.layout.bounds.1 + 52,
            14,
            p.profile.rgb("ui.colors.secondary"),
        );
        if self.request.is_some() {
            let hint_width=(self.layout.cancel.0-self.layout.list.0-8).max(1);
            let hint=p.fit_text_role(Role::Primary,&self.hint(),hint_width,13);
            p.text_role(Role::Primary,&hint,self.layout.list.0,self.layout.bounds.1+74,13,p.profile.rgb("ui.colors.headings"));
            p.border(
                self.layout.cancel.0,
                self.layout.cancel.1,
                self.layout.cancel.2,
                self.layout.cancel.3,
                p.profile.rgb("ui.colors.border"),
            );
            p.text_role(
                Role::Primary,
                "Cancel",
                self.layout.cancel.0 + 12,
                self.layout.cancel.1 + 7,
                14,
                p.profile.rgb("ui.colors.primary"),
            );
        }
        if self.request.is_none() && self.quick_kind.is_none() && self.stamp.revision >= 50. {
            let summary = p.fit_text_role(Role::Primary, &self.stamp.category_summary, self.layout.bounds.2 - 24, 13);
            p.text_role(Role::Primary, &summary, self.layout.list.0, self.layout.bounds.1 + 74, 13, p.profile.rgb("ui.colors.secondary"));
        }
        p.clip = Some(self.layout.list.tuple());
        for (section, y) in self.section_positions() {
            if section > 0 {
                p.rect(self.layout.list.0, y - self.list_scroll, self.layout.list.2, 1,
                    p.profile.rgb("ui.colors.border"));
            }
            let label = if self.stamp.revision >= 50. {
                self.stamp.category_headers[section].as_str()
            } else if section == 0 { "Equipped" } else { "In your pack" };
            p.text(label, self.layout.list.0, y - self.list_scroll + if section == 0 { 3 } else { 4 },
                13, p.profile.rgb("ui.colors.secondary"));
        }
        let rows = self.row_positions();
        for (index, yy) in &rows {
            p.clip = Some(self.layout.list.tuple());
            let g = &self.groups[*index];
            let y = *yy - self.list_scroll;
            if y + self.layout.row <= self.layout.list.1
                || y >= self.layout.list.1 + self.layout.list.3
            {
                continue;
            }
            if Some(g.item.id) != self.selected
                && self.hover_at.is_some_and(|(x, yy)| {
                    Rect(self.layout.list.0, y, self.layout.list.2, self.layout.row).contains(x, yy)
                })
            {
                p.rect(
                    self.layout.list.0,
                    y,
                    self.layout.list.2,
                    self.layout.row,
                    p.profile.rgb("ui.colors.hover"),
                );
            }
            if Some(g.item.id) == self.selected {
                p.rect(
                    self.layout.list.0,
                    y,
                    self.layout.list.2,
                    self.layout.row,
                    p.profile.rgb("ui.colors.selected"),
                );
            }
            let valid =
                self.request.is_some() && self.target_blocker(self.target_member(g)).is_none();
            let label = self.row_label(g);
            p.clip = Some((
                self.layout.list.0,
                y.max(self.layout.list.1),
                self.layout.list.2,
                (y + self.layout.row).min(self.layout.list.1 + self.layout.list.3)
                    - y.max(self.layout.list.1),
            ));
            let reserve=crate::render::resource_ui::amulet_reserve(&g.item,self.stamp.revision);
            let right_pad=if self.read_only||self.request.is_some(){6}else{58};
            let meter_width=if reserve.is_some(){((self.layout.list.2-right_pad)/3).clamp(60,100)}else{0};
            let available = (self.layout.list.2-right_pad-if reserve.is_some(){meter_width+8}else{0}-4).max(1);
            let lines = self.text_metrics.as_ref().map_or_else(|| vec![label.clone()], |m| m.wrap(&label, available));
            for (line_index, line) in lines.iter().enumerate() {
                p.text_role(Role::Inventory, line, self.layout.list.0 + 2,
                    y + 6 + line_index as i32 * self.detail_line_height(), 14,
                    if valid { p.profile.rgb("ui.colors.headings") } else { p.profile.rgb("ui.colors.primary") });
            }
            if let Some((energy,capacity))=reserve {
                crate::render::resource_ui::paint_bar(p,"",&format!("{energy:.0}/{capacity:.0}"),
                    self.layout.list.0+self.layout.list.2-right_pad-meter_width,y+4,meter_width,19,
                    p.profile.rgb("ui.colors.energy"),energy/capacity.max(1.),11);
            }
            if !self.read_only && self.request.is_none() {
                let up = self.reorder_neighbor(*index, -1).is_some();
                let down = self.reorder_neighbor(*index, 1).is_some();
                p.text(
                    "↑",
                    self.layout.list.0 + self.layout.list.2 - 45,
                    y + 4,
                    17,
                    if up {
                        p.profile.rgb("ui.colors.primary")
                    } else {
                        p.profile.rgb("ui.colors.border")
                    },
                );
                p.text(
                    "↓",
                    self.layout.list.0 + self.layout.list.2 - 19,
                    y + 4,
                    17,
                    if down {
                        p.profile.rgb("ui.colors.primary")
                    } else {
                        p.profile.rgb("ui.colors.border")
                    },
                );
            }
        }
        p.clip = Some(self.layout.list.tuple());
        // Legacy empty-slot decoration remains versioned; R50 has no global bag cap.
        if self.stamp.revision < 50. {
            let carried_y = self.section_positions()[1].1;
            let bottom = rows.last().map_or(carried_y + 24, |(_, y)| y + self.layout.row)
                .max(carried_y + 24);
            for n in self.groups.len()..20 {
                let y = bottom + (n - self.groups.len()) as i32 * self.layout.row - self.list_scroll;
                p.rect(self.layout.list.0 + 2, y + self.layout.row - 1,
                    self.layout.list.2 - 4, 1, [21, 25, 35]);
            }
        }
        draw_scrollbar(
            p,
            self.layout.list,
            self.list_scroll,
            self.list_max_scroll(),
        );
        p.clip = Some(self.layout.detail.tuple());
        p.rect(
            self.layout.detail.0,
            self.layout.detail.1,
            1,
            self.layout.detail.3,
            p.profile.rgb("ui.colors.border"),
        );
        for (i, (line, color)) in self.detail_lines().iter().enumerate() {
            p.text_role(
                Role::Inventory,
                line,
                self.layout.detail.0 + 10,
                self.layout.detail.1 + i as i32 * self.detail_line_height() - self.detail_scroll,
                14,
                if *color == INK {
                    p.profile.rgb("ui.colors.primary")
                } else if *color == MUTED {
                    p.profile.rgb("ui.colors.secondary")
                } else if *color == GOLD {
                    p.profile.rgb("ui.colors.headings")
                } else {
                    *color
                },
            );
        }
        if let Some(g) = self.selected_group() {
            for (index, rect) in self.button_rects() {
                let b = &g.actions[index];
                p.rect(
                    rect.0,
                    rect.1,
                    rect.2,
                    rect.3,
                    if self.focus_action == Some(index) {
                        p.profile.rgb("ui.colors.selected")
                    } else if self.hover_at.is_some_and(|(x, y)| rect.contains(x, y)) {
                        p.profile.rgb("ui.colors.hover")
                    } else {
                        p.profile.rgb("ui.colors.button")
                    },
                );
                modal_ui::border(
                    p,
                    rect,
                    if self.focus_action == Some(index) {
                        p.profile.rgb("ui.colors.headings")
                    } else {
                        p.profile.rgb("ui.colors.border")
                    },
                );
                p.text_role(
                    Role::Primary,
                    &b.label,
                    rect.0 + self.layout.button_padding,
                    rect.1 + 7,
                    14,
                    if b.blocker.is_some() || self.pending || !self.stamp.playable() {
                        p.profile.rgb("ui.colors.disabled")
                    } else {
                        p.profile.rgb("ui.colors.primary")
                    },
                );
            }
        }
        draw_scrollbar(
            p,
            self.layout.detail,
            self.detail_scroll,
            self.detail_max_scroll(),
        );
        p.clip = Some(self.layout.bounds.tuple());
        if let Some(tip)=&state.tooltip {
            let Rect(tx,ty,width,height)=tip.bounds;
            p.clip=Some(tip.bounds.tuple());
            p.rect(tx,ty,width,height,p.profile.rgb("ui.colors.tooltip"));
            modal_ui::border(p,tip.bounds,p.profile.rgb("ui.colors.border"));
            for(i,line)in tip.lines.iter().enumerate(){p.text_role(Role::Tooltips,line,tx+8,ty+6+i as i32*tip.line_height,13,p.profile.rgb("ui.colors.primary"));}
        }
        p.clip = old_clip;
    }
}
fn draw_scrollbar(p: &mut Painter, rect: Rect, offset: i32, maximum: i32) {
    if maximum <= 0 {
        return;
    }
    let h = (rect.3 * rect.3 / (rect.3 + maximum)).max(18);
    let y = rect.1 + offset * (rect.3 - h) / maximum;
    p.rect(
        rect.0 + rect.2 - 3,
        y,
        2,
        h,
        p.profile.rgb("ui.colors.border"),
    );
}
fn tinker_blocker(r: &Run) -> Option<String> {
    if corpse_core::engine::condition(&r.player, "paralyzed") > 0. {
        Some("You are paralyzed. Wait to advance time.".into())
    } else if let Some(reason)=crate::ability_ui::payment_blocker(r,"kative-tinker",100.) {
        Some(reason)
    } else if abilities::energy_activation_silenced(r, &r.player) {
        Some(if r.simulation_revision.value_or(29.)>=43. { "Your ability is silenced and does not work." } else { "Energy-paid activation is silenced" }.into())
    } else if !r.inventory.iter().any(|i| corpse_core::tinker::target_eligible_for_revision(i, r.simulation_revision.value_or(29.))) {
        Some(if r.simulation_revision.value_or(29.) >= 50. { "No untinkered ring or amulet available" } else { "No unblessed weapon, armor, ring or amulet to tinker with" }.into())
    } else if corpse_core::tinker::god_pools(r).is_empty() {
        Some(if r.simulation_revision.value_or(29.)>=43. { "You can't think of any new ideas to tinker with..." } else { "No eligible outside-god abilities remain" }.into())
    } else {
        None
    }
}
/// Measure the actual catalog combinations, not character counts: proportional
/// fonts can make a shorter name wider. The catalog is bounded (12 effects, 72 powers).
fn longest_tinkered_amulet_name(p: &Painter, revision: f64) -> String {
    let mut longest = String::new();
    let mut width = 0.;
    for effect in ["lightning", "firebolt", "poison", "iceblast", "blinking", "beckoning", "discord", "obstruction", "roots", "teleportation"] {
        let item = Item { kind: "amulet".into(), effect: effect.to_string().into(), ..Default::default() };
        let name = corpse_core::revision50_amulet_effects::display_name(&item).unwrap();
        for a in progression::catalog_for_revision(revision).iter().filter(|a| !a.id.starts_with("kative-")) {
            let ability = match a.id.as_str() { "shrail-rage-at-the-dying-of-the-light" => "Ragelight", "pelian-you-shall-never-be-rid-of-me" => "Never Be Rid of Me", _ => &a.name };
            let label = jewelry_label(format!("Tier 4 {name} and {ability}"), "Amulet", false);
            let measured = p.measure_text_role(Role::Inventory, &label, 14);
            if measured > width { width = measured; longest = label; }
        }
    }
    longest
}
fn tinker_eligible(i: &Item) -> bool {
    corpse_core::tinker::target_eligible(i)
}
fn ability_name(id: &str) -> &str {
    progression::definition(id)
        .map_or(id, |a| a.name.as_str())
}
fn wrap(text: &str, columns: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut out = vec![];
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > columns {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        for ch in word.chars() {
            if line.chars().count() == columns {
                out.push(std::mem::take(&mut line));
            }
            line.push(ch);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}
fn ellipsize(text: &str, columns: usize) -> String {
    if text.chars().count() <= columns {
        text.into()
    } else {
        text.chars()
            .take(columns.saturating_sub(1))
            .chain(['…'])
            .collect()
    }
}

fn identity(run: &Run, i: &Item) -> Item {
    let mut key = i.clone();
    // R52 amulets are physical, separately charged instances, including old
    // missing-state items. Preserve the ID even when their effect/tier match.
    // Typed Item equality also retains the stored tier and complete reserve.
    if !(run.simulation_revision.value_or(29.) == 52.
        && corpse_core::revision52_amulet_tiers::profile(run, i).is_some()) {
        key.id = 0.;
    }
    key.quantity = Field::Missing;
    if run.simulation_revision.value_or(29.) >= 50. && matches!(i.kind.as_str(), "ring" | "amulet") {
        key.equipped = Field::Missing;
    }
    if i.kind == "potion" {
        let known = potion_known(run, i);
        key.identified = known.into();
        key.name = if known {
            format!("Potion of {}", i.effect.as_ref().map_or("", String::as_str))
        } else {
            potion_appearance(run, i.effect.as_ref().map_or("", String::as_str))
        };
        if !known {
            key.effect = key.name.clone().into();
        }
        key.potion_nature = potion_nature(run, i).map_or(Field::Missing, |s| s.to_string().into());
        key.magic_nature = magic_nature(run, i).map_or(Field::Missing, |s| s.to_string().into());
    }
    key
}
fn display_groups(run: &Run) -> Vec<Group> {
    // One current-loadout stat query for all damage descriptions in this rebuild.
    let offense = combat::effective_stats(run, &run.player).offense;
    let mut groups: Vec<Group> = vec![];
    let mut identities: Vec<Item> = vec![];
    for item in &run.inventory {
        let key = identity(run, item);
        let prior = if run.simulation_revision.value_or(29.) < 50. && crafting::is_equipped(item) {
            None
        } else {
            identities.iter().position(|i| *i == key)
        };
        if let Some(index) = prior {
            groups[index].items.push(item.clone());
            groups[index].count += item.quantity.value_or(1.);
        } else {
            identities.push(key);
            groups.push(Group {
                item: item.clone(),
                items: vec![item.clone()],
                count: item.quantity.value_or(1.),
                name: displayed_name(run, item),
                description: describe_item_with_offense(run, item, offense),
                actions: item_actions(run, item),
                section: item_section(run.simulation_revision.value_or(29.), item),
            });
        }
    }
    let order = run
        .inventory_display_order
        .as_ref()
        .map_or(&[][..], Vec::as_slice);
    let rank = |g: &Group| {
        g.items
            .iter()
            .map(|item| {
                order
                    .iter()
                    .position(|id| *id == item.id)
                    .unwrap_or_else(|| {
                        order.len() + run.inventory.iter().position(|i| i.id == item.id).unwrap()
                    })
            })
            .min()
            .unwrap()
    };
    let mut ranked: Vec<_> = groups
        .into_iter()
        .map(|g| {
            let key = (g.section, rank(&g));
            (key, g)
        })
        .collect();
    ranked.sort_by_key(|(key, _)| *key);
    ranked.into_iter().map(|(_, g)| g).collect()
}
/// Shortcut admission uses the same current item, blocker and target as the Pack.
pub fn quick_item_action(run:&Run,id:f64)->PackAction {
    let Some(item)=run.inventory.iter().find(|i|i.id==id&&crate::game_hud::is_quick_item(run,i)) else {
        return PackAction::Notice("That item is no longer available.".into());
    };
    for button in item_actions(run,item) {
        let shard=corpse_core::borge::is_shard(item);
        if !(matches!(button.label.as_str(),"Apply"|"Teleport")||shard&&button.label=="Throw"){continue}
        if let Some(blocker)=button.blocker {return PackAction::Notice(blocker);}
        match button.action {
            ButtonAction::Target(target)=>return PackAction::Target(target),
            ButtonAction::Command(command)=>return PackAction::Command(command),
            _=>{}
        }
    }
    PackAction::Notice("This item has no available action.".into())
}
fn item_actions(run: &Run, item: &Item) -> Vec<Button> {
    let mut buttons = vec![];
    let mut push = |label: &str, action: ButtonAction, blocker: Option<String>, tip: String| {
        buttons.push(Button {
            label: label.into(),
            action,
            blocker,
            tip,
        });
    };
    if corpse_core::ahvamitt::is_mask(item) {
        let cooldown=corpse_core::ahvamitt::cooldown(run);
        push("Teleport", ButtonAction::Command(Command::Ability {
            id:corpse_core::ahvamitt::TELEPORT_ID.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing,
        }), (cooldown>0).then(||format!("Teleport is ready in {cooldown} turns.")),
            "Teleport to a random standable spot; 100-turn cooldown.".into());
    }
    if run.simulation_revision.value_or(29.)>=50. && corpse_core::revision50_amulet_effects::is_amulet(item)
        && !corpse_core::revision52_amulet_tiers::retired(run,item) {
        let blocker=corpse_core::revision50_amulets::plan_item_activation(run,item.id,
            corpse_core::revision50_amulet_effects::item_spec_for_revision(item,run.simulation_revision.value_or(29.)).as_ref()).err()
            .map(corpse_core::revision50_amulet_effects::admission_message);
        push("Apply",ButtonAction::Target(PackTarget::UseAt(item.id)),blocker,
            "Choose a target for this amulet.".into());
    }
    if corpse_core::borge::is_shard(item) && !corpse_core::borge::weapon_enabled(run) {
        push("Use on…", ButtonAction::Target(PackTarget::UseAt(item.id)), None,
            "Choose an adjacent living creature. The shard is consumed only on success.".into());
    }
    if matches!(item.kind.as_str(), "heart" | "potion" | "tool" | "food") && !corpse_core::ahvamitt::is_mask(item)
        && !corpse_core::revision50_potions::retired(run,item) {
        let label = match item.kind.as_str() {
            "tool" if corpse_core::phantasmii_generation::is_dagger(item) => "Apply",
            "heart" | "food" => "Eat",
            "tool" => "Use",
            _ => "Drink",
        };
        push(
            label,
            ButtonAction::Command(Command::Use { id: item.id }),
            (item.kind == "tool" && !corpse_core::phantasmii_generation::is_dagger(item)
                && item.effect.as_ref().is_none_or(|e| e != "tree-tap"))
                .then(|| "This tool has no supported use.".into()),
            String::new(),
        );
    } else if equipment::equippable(item) && run.simulation_revision.value_or(29.) < 50. {
        let label = if item.kind == "weapon" {
            if item.hands.value_or(1.) == 2. {
                "Both hands"
            } else {
                "Main hand"
            }
        } else if item.equipped.as_ref().is_some_and(|s| !s.is_empty()) {
            "Equipped"
        } else {
            "Equip"
        };
        let slot = match item.kind.as_str() {
            "weapon" => "hand1",
            "ring" => "ring1",
            other => other,
        };
        push(
            label,
            ButtonAction::Command(Command::Equip {
                id: item.id,
                slot: slot.into(),
            }),
            None,
            String::new(),
        );
    }
    if run.simulation_revision.value_or(29.) < 50. && equipment::equippable(item)
        && ((item.kind == "weapon" && item.hands.value_or(1.) != 2.) || item.kind == "ring")
    {
        push(
            if item.kind == "weapon" {
                "Offhand"
            } else {
                "Ring 2"
            },
            ButtonAction::Command(Command::Equip {
                id: item.id,
                slot: if item.kind == "weapon" {
                    "hand2"
                } else {
                    "ring2"
                }
                .into(),
            }),
            None,
            String::new(),
        );
    }
    if crafting::is_equipped(item) && run.simulation_revision.value_or(29.) < 50. {
        push(
            "Unequip",
            ButtonAction::Command(Command::Unequip { id: item.id }),
            None,
            String::new(),
        );
    }
    push(
        "Drop",
        ButtonAction::Command(Command::Drop { id: item.id }),
        None,
        String::new(),
    );
    if (!corpse_core::borge::is_shard(item) || corpse_core::borge::weapon_enabled(run))
        && !corpse_core::revision50_potions::retired(run,item) {
    push(
        "Throw",
        ButtonAction::Target(PackTarget::Throw(item.id)),
        None,
        String::new(),
    );
    }
    if item.kind == "heart" && run.simulation_revision.value_or(29.) < 50. {
        for craft in run
            .player
            .abilities
            .iter()
            .filter(|a| crafting::craft_name(a).is_some())
        {
            push(
                ability_name(craft),
                ButtonAction::Craft {
                    heart: item.id,
                    craft: craft.clone(),
                },
                (!crafting::supported(craft)).then(|| {
                    format!(
                        "{} is not implemented in this native build.",
                        ability_name(craft)
                    )
                }),
                String::new(),
            );
        }
    }
    if projectiles::is_bow(item) {
        let blocker = projectiles::blocker(
            run,
            &Command::Shoot {
                id: item.id,
                x: run.player.x,
                y: run.player.y,
            },
        );
        buttons.insert(
            0,
            Button {
                label: "Apply".into(),
                action: ButtonAction::Target(PackTarget::Shoot(item.id)),
                blocker,
                tip: format!("Fire one arrow ({} available). {}", arrow_count(run), if run.simulation_revision.value_or(29.) >= 50. { "No equipping required." } else { "This bow occupies both hands." }),
            },
        );
    }
    for button in &mut buttons {
        let blocker = match &button.action {
            ButtonAction::Command(command) => corpse_core::dreamwalking::blocker(run, command),
            ButtonAction::Target(target) => corpse_core::dreamwalking::blocker(run,
                &target.command(run.player.x, run.player.y)),
            ButtonAction::Craft { .. } if corpse_core::dreamwalking::active(run) =>
                Some("You cannot use consumable items while Dreamwalking."),
            _ => None,
        };
        if let Some(reason) = blocker { button.blocker = Some(reason.into()); }
    }
    buttons
}
fn arrow_count(run: &Run) -> f64 {
    run.inventory
        .iter()
        .filter(|i| inventory::is_arrow(i))
        .map(inventory::quantity)
        .fold(0., |sum, n| sum + n)
}
fn potion_known(run: &Run, i: &Item) -> bool {
    corpse_core::revision50::enabled(run)
        || i.identified.value_or(false)
        || run
            .known_potions
            .iter()
            .any(|s| Some(s) == i.effect.as_ref())
}
fn jewelry_kind(i: &Item) -> Option<&'static str> {
    match i.kind.as_str() { "ring" => Some("Ring"), "amulet" => Some("Amulet"), _ => None }
}

// Rewrite only the projected label. Model names are stable lookup/save keys.
fn jewelry_label(name: String, kind: &str, floor: bool) -> String {
    let body = name.strip_prefix("Retired ").unwrap_or(&name);
    let body = body.strip_prefix("Tier ")
        .and_then(|s|s.split_once(' '))
        .filter(|(tier,_)|tier.bytes().all(|c|c.is_ascii_digit()))
        .map_or(body, |(_,label)|label);
    let of = match kind { "Ring" => "Ring of ", _ => "Amulet of " };
    let Some(effect) = body.strip_prefix(of) else { return name; };
    let (effect, grant) = effect.split_once(" and ").map_or((effect,None),|(effect,grant)|(effect,Some(grant)));
    if floor { return format!("{effect} {kind}"); }
    let prefix = &name[..name.len()-body.len()];
    grant.map_or_else(||format!("{prefix}{effect} {kind}"),|g|format!("{prefix}{effect} {kind} and {g}"))
}

pub(crate) fn ground_name(run: &Run, i: &Item) -> String {
    let Some(kind) = jewelry_kind(i) else {
        return if corpse_core::revision50::enabled(run){displayed_name(run,i)}else{i.name.clone()};
    };
    let catalog_name = if corpse_core::revision50::enabled(run) {
        corpse_core::revision50_rings::display_name(i)
            .or_else(||corpse_core::revision50_amulet_effects::display_name(i))
    } else { None };
    jewelry_label(catalog_name.unwrap_or(&i.name).to_owned(), kind, true)
}

pub(crate) fn ground_description(run: &Run, i: &Item) -> String {
    if jewelry_kind(i).is_some() {ground_name(run,i)}else{describe_item(run,i)}
}

pub(crate) fn displayed_name(run: &Run, i: &Item) -> String {
    let name = if corpse_core::revision50::enabled(run) {
        corpse_core::revision50_item_knowledge::display_name(run, i)
    } else if i.kind == "potion" {
        if potion_known(run, i) {
            format!(
                "Potion of {}{}",
                if run.simulation_revision.value_or(29.)>=33. && i.effect.as_ref().is_some_and(|e|e=="Veridian Sap") {
                    "Verdania Sap"
                } else { i.effect.as_ref().map_or("", String::as_str) },
                if i.diluted.value_or(false) {
                    " (diluted)"
                } else {
                    ""
                }
            )
        } else {
            potion_appearance(run, i.effect.as_ref().map_or("", String::as_str))
        }
    } else {
        i.name.clone()
    };
    let name = if let Some(kind) = jewelry_kind(i) { jewelry_label(name,kind,false) } else { name };
    let name = name.replace("Potion of Offense", "Potion of Damage").replace("Potion of Defense", "Potion of HP").replace("Potion of offense", "Potion of Damage").replace("Potion of defense", "Potion of HP");
    let name = if !corpse_core::revision50::enabled(run) && marked_heart(i) { format!("{name} (marked)") } else { name };
    match magic_nature(run, i) {
        Some(nature) if !nature.is_empty() => format!("{name} ({nature} magic)"),
        _ => name,
    }
}
fn stat_values(stats: &PartialStats) -> [(&str, &Field<f64>); 8] {
    [
        ("offense", &stats.offense),
        ("defense", &stats.defense),
        ("accuracy", &stats.accuracy),
        ("dodge", &stats.dodge),
        ("dr", &stats.dr),
        ("perception", &stats.perception),
        ("maxEnergy", &stats.max_energy),
        ("maxHp", &stats.max_hp),
    ]
}
fn bonuses(stats: Option<&PartialStats>, base: Option<&PartialStats>) -> String {
    let Some(stats) = stats else {
        return String::new();
    };
    stat_values(stats)
        .iter()
        .enumerate()
        .filter_map(|(n, (name, field))| {
            field.as_ref().map(|value| {
                let value = base
                    .and_then(|b| stat_values(b)[n].1.as_ref())
                    .map_or(combat::integer_stat(*value), |b| {
                        combat::integer_stat(*b) + combat::integer_stat(*value - *b)
                    });
                let (name,value)=crate::stat_presentation::bonus(name,value);
                format!("{}{value} {name}", if value >= 0. { "+" } else { "" })
            })
        })
        .collect::<Vec<_>>()
        .join(", ")
}
fn equipped_place(i: &Item) -> &str {
    if i.kind == "weapon" && i.hands.value_or(1.) == 2. {
        return "Both hands";
    }
    match i.equipped.as_ref().map(String::as_str) {
        Some("hand1") => "Main hand",
        Some("hand2") => "Offhand",
        Some("body" | "armor" | "amulet") => "Worn",
        Some("ring" | "ring1") => "Ring finger",
        Some("ring2") => "Second ring finger",
        _ => "Equipped",
    }
}
fn marked_heart(i: &Item) -> bool {
    i.kind=="heart" && i.source_actor_id.as_ref().is_some() && i.borge_depth.as_ref().is_some()
}
/// Pure preview from shared current-loadout Offense: constant range arithmetic,
/// plus O(I+enhancements) first-strike/funding lookups. No RNG or target scan.
fn weapon_damage_preview(run: &Run, item: &Item, shot: bool, offense: f64) -> String {
    let base = corpse_core::engine::player_weapon_strike_base_for_revision(item, offense, run.simulation_revision.value_or(29.));
    let low = combat::ordinary_strike_damage_for_run(run, base, 0.75, 0., false, false);
    let high = combat::ordinary_strike_damage_for_run(run, base, 1.25, 0., false, false);
    let mut text = format!("{}{}: {low}–{high} before defenses and ability bonuses.",
        if shot { "Shot damage" } else { "Damage" },
        if crafting::is_equipped(item) { "" } else { " with current stats" });
    let main = run.inventory.iter().find(|i| i.kind == "weapon" && !projectiles::is_bow(i)
        && i.equipped.as_ref().is_some_and(|s|s=="hand1")).is_some_and(|i|i.id==item.id);
    if run.simulation_revision.value_or(29.) >= 43. && ((shot && crafting::is_equipped(item)) || (!shot && main)) {
        append_fury_preview(&mut text, run, base, !shot, "Fury first strike");
    }
    text
}
/// O(I + enhancements) funding forecast; never spends energy or toggles a craft.
fn append_fury_preview(text: &mut String, run: &Run, base: f64, melee: bool, label: &str) {
    let assault = melee && run.player.active.iter().any(|a| a==abilities::ASSAULT) && run.player.energy>=10.;
    let (bonus, _) = crafting::preview_fury_bonus(run, run.player.energy-if assault{10.}else{0.});
    if bonus>0. {
        text.push_str(&format!("\n{label}: {}–{} before defenses and other ability bonuses.",
            combat::ordinary_strike_damage_for_run(run, base+bonus, 0.75, 0., false, false),
            combat::ordinary_strike_damage_for_run(run, base+bonus, 1.25, 0., false, false)));
    }
}
pub(crate) fn describe_item(run: &Run, i: &Item) -> String {
    let offense = if i.kind=="weapon" { combat::effective_stats(run, &run.player).offense } else { 0. };
    describe_item_with_offense(run, i, offense)
}
fn describe_item_with_offense(run: &Run, i: &Item, offense: f64) -> String {
    if run.simulation_revision.value_or(29.) >= 52. && projectiles::is_bow(i) {
        return format!("Apply to fire an arrow at a visible target. {} arrows available.", arrow_count(run));
    }
    if corpse_core::ahvamitt::is_mask(i) {
        return format!("{}\nGrants Teleport while in your inventory.", corpse_core::revision50_item_knowledge::display_name(run, i));
    }
    if corpse_core::phantasmii_generation::is_dagger(i) {
        return format!("{}\n{}",
            i.name, corpse_core::phantasmii_generation::DAGGER_DESCRIPTION);
    }
    if corpse_core::phantasmii_generation::is_shard(i) {
        let mut text = format!("{} ×{}", i.name, i.quantity.value_or(1.));
        if corpse_core::phantasmii_generation::is_dream_shard(i) {
            text.push_str(if corpse_core::phantasmii_blessing_rules::current(run) {
                "\nCollect three dream shards to complete the ritual."
            } else { "\nCollect three to complete the ritual." });
        }
        if corpse_core::phantasmii_blessing_rules::current(run) {
            text.push('\n');
            text.push_str(&corpse_core::phantasmii_generation::blessing_description(run));
        }
        return text;
    }
    if corpse_core::borge::is_shard(i) {
        if run.simulation_revision.value_or(29.) >= 50. {
            return format!("{}\nThrow to permanently kill a creature. Consumed after a successful hit.", i.name);
        }
        if run.simulation_revision.value_or(29.) >= 43. {
            return format!("{}\n{}\nPermanently kill a creature and destroy the shard.", i.name,
                if crafting::is_equipped(i) { equipped_place(i) } else { "Unequipped" });
        }
        if corpse_core::borge::weapon_enabled(run) {
            return format!("{}\n{}\n{}", i.name,
                if crafting::is_equipped(i) { equipped_place(i) } else { "Unequipped" },
                corpse_core::borge::SHARD_DESCRIPTION);
        }
        return format!("{}\n{}\n{}\nUse on an adjacent living creature, or equip in either hand to use on your next melee target. No accuracy roll; prevents revival. Consumed only after a successful use.",
            i.name, if crafting::is_equipped(i) { equipped_place(i) } else { "Unequipped" }, corpse_core::borge::SHARD_DESCRIPTION);
    }

    if corpse_core::royal_honey::is_honey(i) {return if run.simulation_revision.value_or(29.) >= 43. { "9999 health, 9999 energy" } else { "Eat once to fully restore current health and energy to their effective maxima. No other effect." }.into();}
    if i.kind == "potion" {
        return describe_potion(run, i);
    }
    let stats = bonuses(i.bonus.as_ref(), i.base_bonus.as_ref());
    if i.kind == "heart" {
        let rewards = inventory::effective_heart_rewards(run, i);
        let stats = if corpse_core::fast_mode::enabled(run) {
            bonuses(rewards.bonus.as_ref(), None)
        } else { stats };
        let mut text = format!(
            "{}{} <heart>\n{}\n{} blessings · {} energy",
            displayed_name(run, i),
            if i.quantity.value_or(1.) > 1. {
                format!(" ×{}", i.quantity.value_or(1.))
            } else {
                String::new()
            },
            if stats.is_empty() {
                "No permanent stat increase"
            } else {
                &stats
            },
            rewards.blessings,
            i.energy.value_or(0.)
        );
        if marked_heart(i) {
            text.push_str(&format!("\nBorge-marked sacrifice: offer this exact heart at the Borge altar on depth {}.", i.borge_depth.value_or(0.)));
            if corpse_core::borge::throw_offering_enabled(run) {
                text.push_str(" You can throw it onto the altar to leave the shard there.");
            }
        }
        if run
            .player
            .abilities
            .iter()
            .any(|a| a == "kative-adaptive-armor")
        {
            if let Some(kind) = inventory::heart_creature_type(i) {
                let prior = run
                    .player
                    .adaptive_hearts
                    .as_ref()
                    .and_then(|m| m.get(&kind))
                    .copied()
                    .unwrap_or(0.);
                let kind = corpse_core::revision50_creature_identity::name_for_revision(&kind, run.simulation_revision.value_or(29.));
                text.push_str(&format!(
                    "\nAdaptive Armor: +{} DR against {kind}",
                    (3. - prior.floor().max(0.)).max(0.)
                ));
            }
        }
        return text;
    }
    if inventory::is_arrow(i) {
        return format!("{} ×{}", i.name, i.quantity.value_or(1.));
    }
    let mut lines = vec![
        displayed_name(run, i),
        format!(
            "{}{}",
            titlecase(&i.kind),
            if !corpse_core::revision50::enabled(run) && crafting::is_equipped(i) {
                format!(" · {}", equipped_place(i))
            } else if !corpse_core::revision50::enabled(run) && equipment::equippable(i) {
                " · Unequipped".into()
            } else {
                String::new()
            }
        ),
    ];
    // Primary item effects precede optional Tinker grants so a long tooltip
    // starts with what this item does, even when its full details need scrolling.
    let named_jewelry = append_named_jewelry(run, i, &mut lines);
    if i.kind == "weapon" && !projectiles::is_bow(i) {
        if let Some(slot) = i.equipped.as_ref().filter(|s| matches!(s.as_str(), "hand1" | "hand2")) {
            let first = run.inventory.iter().find(|j| {
                j.kind == "weapon" && !projectiles::is_bow(j) && j.equipped.as_ref() == Some(slot)
            });
            if first.is_some_and(|j| j.id == i.id) {
                let repeat = run.player.active.iter().any(|a| a == abilities::ASSAULT)
                    && run.player.energy >= 10.;
                lines.push(format!("{}{}{}",
                    if slot == "hand2" { "Offhand: extra strike · ½ normal hit chance" } else { "Main strike · normal hit chance" },
                    if repeat { " · twice with Unyielding Assault" } else { "" },
                    if slot == "hand2" { " Only if both survive the main strike." } else { "" }));
            }
        }
        lines.push(format!("{}-handed", i.hands.value_or(1.)));
        lines.push(weapon_damage_preview(run, i, false, offense));
        if i.enchantment.value_or(0.) != 0. {
            lines.push(format!("Enchantment: {:+}", i.enchantment.value_or(0.)));
        }
        let base = projectiles::player_throw_base_for_revision(i, offense, run.simulation_revision.value_or(29.));
        let mut thrown = format!("Thrown damage with current stats: {}–{} before defenses and ability bonuses.",
            combat::ordinary_strike_damage_for_run(run, base, 0.75, 0., false, false),
            combat::ordinary_strike_damage_for_run(run, base, 1.25, 0., false, false));
        if run.simulation_revision.value_or(29.)>=43. { append_fury_preview(&mut thrown, run, base, false, "Fury throw"); }
        lines.push(thrown);
    }
    if projectiles::is_bow(i) {
        lines.extend([format!("{} Apply to fire at a visible target. Uses one arrow; {} available.", if run.simulation_revision.value_or(29.) >= 50. { "No equipping required." } else { "2-handed · Equip in both hands;" }, arrow_count(run)), "Uses your existing unarmed attack in melee.".into(), weapon_damage_preview(run, i, true, offense), "Half the miss chance of a thrown weapon; guaranteed hits and misses still apply. Walls, trees, closed doors, and the first creature stop arrows; grates let them pass.".into(), "Arrows vanish on a hit. Misses leave recoverable arrows, which may scatter or fall.".into()]);
    }
    if !stats.is_empty() {
        lines.push(format!(
            "{}: {stats}",
            if corpse_core::revision50::enabled(run) && matches!(i.kind.as_str(), "ring" | "amulet") {
                "Bonuses"
            } else if equipment::equippable(i) {
                "When equipped"
            } else {
                "Inactive equipment bonuses"
            }
        ));
    }
    for id in i.granted_ability.as_ref().filter(|s| !s.is_empty()).into_iter().chain(i.additional_granted_abilities.iter().filter(|_| run.simulation_revision.value_or(29.) >= 50.)) {
        lines.push(format!(
            "{} {}.",
            if corpse_core::revision50::enabled(run) && matches!(i.kind.as_str(), "ring" | "amulet") {
                "Grants"
            } else if equipment::equippable(i) {
                "While equipped: grants"
            } else {
                "Inactive equipment grant:"
            },
            crate::ability_ui::display_text(progression::definition_for_revision(run.simulation_revision.value_or(29.), id).map_or(id.as_str(), |a| a.name.as_str()))
        ));
    }
    if i.tinkered.value_or(false) {
        lines.push("You've already tinkered with this item.".into());
    }
    for e in &i.enhancements {
        let name = crafting::craft_name(&e.craft).unwrap_or(&e.craft);
        let text = match name {
            "Fury" => if run.simulation_revision.value_or(29.)>=43. {
                "Empowers your first strike for 10 energy."
            } else {
                "Toggle: 10 energy per attack; +10 first-strike damage before defenses and the final ×0.5 scale."
            }
            "Zeal" => "Spend 15 energy to restore 25% of maximum health, without taking an action.",
            "Guile" => "+5 carrying capacity.",
            "Dream" => "Always flying; +50% maximum energy.",
            "Artifice" => "Adds these heart bonuses while equipped.",
            _ => "",
        };
        lines.push(format!(
            "{name} craft ({}): {text}",
            if equipment::equippable(i) {
                "equipped only"
            } else {
                "inactive on pack-only items"
            }
        ));
        lines.push(bonuses(Some(&e.bonus), None));
    }
    if !named_jewelry {
        if let Some(effect) = i.effect.as_ref().filter(|s| !s.is_empty()) {
            lines.push(if effect == "tree-tap" {
                format!("Apply beside {} to collect its one gift of sap. The tap breaks when used.",
                    if run.simulation_revision.value_or(29.)>=33. {"Verdania"} else {"Veridian"})
            } else {
                effect.clone()
            });
        }
    }
    lines
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
fn append_named_jewelry(run: &Run, i: &Item, lines: &mut Vec<String>) -> bool {
    if !corpse_core::revision50::enabled(run) { return false; }
    if let Some(description) = corpse_core::revision50_rings::description_for_run(run,i) {
        lines.push(description.into());
        return true;
    }
    if let Some(description) = corpse_core::revision52_amulet_tiers::description(run, i) {
        lines.push(description);
        if corpse_core::revision52_amulet_tiers::retired(run,i) {return true;}
        let spec=corpse_core::revision50_amulet_effects::item_spec_for_revision(i,run.simulation_revision.value_or(29.)).unwrap();
        let energy=i.amulet_state.as_ref().map_or(spec.capacity,|s|s.energy.min(spec.capacity));
        let base=spec.activation_cost;
        let cost=(base-if run.player.abilities.iter().any(|a|a=="kative-appropriate"){10.}else{0.}).max(0.);
        let change = if run.inventory.iter().any(|owned| owned.id==i.id) {
            crate::render::resource_ui::change_suffix(run,
                &corpse_core::resources::amulet_key(i.id),energy,spec.capacity)
        } else { String::new() };
        lines.push(format!("Amulet energy: {energy}/{}{change} · Activation: {cost} energy.",spec.capacity));
        if let Some(details) = corpse_core::revision52_amulet_tiers::details(run,i) { lines.push(details); }
        return true;
    }
    false
}
fn titlecase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
fn describe_potion(run: &Run, i: &Item) -> String {
    if corpse_core::revision50_potions::retired(run,i) { return inventory::potion_description(run,i); }
    if corpse_core::revision50::enabled(run) {
        return format!("{}\nDrink to affect yourself, or throw to apply at the impact point.", inventory::potion_description(run, i));
    }
    let effect = i.effect.as_ref().map_or("", String::as_str);
    let diluted = i.diluted.value_or(false);
    let strength = if diluted { 0.5 } else { 1. };
    let identity = inventory::potion_description(run, i).replace("offense", "Damage").replace("defense", "HP");
    let identity = if run.simulation_revision.value_or(29.)>=33. && effect=="Veridian Sap" && potion_known(run,i) {
        identity.replace("Veridian Sap","Verdania Sap")
    } else {identity};
    if !potion_known(run, i) {
        return identity;
    }
    let detail=match effect {
        "detect magic"=>format!("Senses potions and magical equipment for {} turns. Remembers their good, bad, or bizarre magic and last positions afterward. Mixed magic is bizarre. Ordinary gear, hearts, and tools are not detected.",(20f64*strength).ceil()),
        "healing"=>format!("Restores {} health.",50.*strength),
        "energy"=>format!("Restores {} energy.",50.*strength),
        "incineration"=>format!("Ignites the impact tile; a creature there burns for a random {}–{} turns, taking 4 HP per normal turn. Fire exposure renews the 6–10-turn burn.",(6f64*strength).ceil(),(10f64*strength).ceil()),
        "caustic gas"=>"Releases caustic gas.".into(),
        "paralysis"=>format!("Releases paralysis gas; a directly affected creature is paralyzed for {} turns.",(5f64*strength).ceil()),
        "telepathy"=>format!("Senses living creatures through walls across this floor for {} turns. Reveals terrain and occupants within 2 squares of each sensed creature.",(30f64*strength).ceil()),
        "levitation"=>format!("Grants flight for {} turns, avoiding chasms, currents, and deep-water item loss. Fire and gas still affect you. If flight ends over a chasm, you fall.",(20f64*strength).ceil()),
        "speed"=>format!("Grants one extra movement step per normal turn for {} turns, without an extra attack.",(10f64*strength).ceil()),
        "confusion"=>"Releases confusion gas.".into(),
        "invisibility"=>format!("Hides you from ordinary sight for {} turns, even while attacking. Telepathy still reveals you.",(20f64*strength).ceil()),
        "descent"=>format!("Collapses terrain within a circular radius of {} squares. Grounded creatures fall one floor, taking fall damage unless they land in deep water. Spares stairs and fixed fixtures. Has no effect on floor 40.",if diluted{1}else{2}),
        "offense"|"defense"|"accuracy"|"dodge"|"dr"|"perception"=>{let (label,value)=crate::stat_presentation::bonus(effect,combat::integer_stat(if effect=="dr" {2.*strength}else{5.*strength}));format!("Grants +{value} {label} for {} turns. Reusing keeps the stronger bonus and longer duration, including diluted potions; they do not stack.",(20f64*strength).ceil())},
        _=>String::new(),
    };
    [
        identity,
        detail,
        "Drink to use on yourself. Throw to break at a target.".into(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    /// O(initial projection + 100 fixed-field checks), repeated hover never refreshes content/metrics.
    #[test]
    fn hover_configuration_reuses_content_and_rebuilds_only_on_invalidation(){
        let mut r=run();let canonical=serde_json::to_vec(&r).unwrap();let mut p=Painter::new();let map=(0,0,1000,800);let mut v=PackView::observe(&r);v.configure(&p,map);assert_eq!(v.configure_rebuilds,1);assert_eq!(v.refresh_count,0);
        assert!(v.hover(100,300));assert!(!v.hover(100,300));
        for n in 0..100{v.hover(100+n,300);v.configure(&p,map);}
        assert_eq!(v.configure_rebuilds,1);assert_eq!(v.refresh_count,0);assert_eq!(serde_json::to_vec(&r).unwrap(),canonical);
        v.configure(&p,(0,0,640,480));assert_eq!(v.configure_rebuilds,2);
        p.profile=p.profile.set("typography.inventory.fontSize",serde_json::json!(24)).unwrap();v.configure(&p,(0,0,640,480));assert_eq!(v.configure_rebuilds,3);
        r.inventory.push(gear(9999.,"Ring"));v.refresh(&r);v.configure(&p,(0,0,640,480));assert_eq!(v.refresh_count,1);assert_eq!(v.configure_rebuilds,4);assert!(v.groups.iter().any(|g|g.items.iter().any(|i|i.id==9999.)));
        v.scroll(-100,v.layout.list.0+3,v.layout.list.1+3);v.configure(&p,(0,0,640,480));assert_eq!(v.configure_rebuilds,4);assert!(v.list_scroll<=v.list_max_scroll());
        let index=0;let y=v.row_positions()[index].1-v.list_scroll;let _=v.click(v.layout.list.0+10,y+10);v.configure(&p,(0,0,640,480));assert_eq!(v.configure_rebuilds,4);
    }
    /// O(pack pixels + glyphs), cached warm configuration paints identically to forced fresh metrics scan.
    #[test]
    fn warm_hover_paint_matches_forced_configuration(){let r=run();let mut p=Painter::new();let map=(0,0,1000,800);let mut v=PackView::observe(&r);v.configure(&p,map);v.hover(v.layout.list.0+10,v.row_positions()[0].1+5);p.clear([1,2,3]);v.paint(&mut p);let expected=p.pixels.clone();v.content_metrics_dirty=true;v.configure(&p,map);p.clear([1,2,3]);v.paint(&mut p);assert_eq!(p.pixels,expected);}

    fn run() -> Run {
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()
    }
    #[test]
    fn fast_heart_description_matches_awarded_rewards_without_mutating_item() {
        let mut r = run(); r.simulation_revision = 52.0.into(); r.fast_mode = true.into();
        let i = Item { id: 9999., name: "Rat heart".into(), kind: "heart".into(), exp: 0.25.into(),
            energy: 3.5.into(), bonus: PartialStats { offense: 0.125.into(), defense: 0.25.into(),
                accuracy: 0.375.into(), dodge: 0.625.into(), dr: 0.75.into(), perception: 1.125.into(),
                max_energy: 1.25.into(), max_hp: 1.375.into() }.into(), ..Default::default() };
        let before = i.clone(); let text = describe_item(&r, &i);
        for value in ["+0 Damage", "+0 HP", "+0 accuracy", "+2 dodge", "+2 dr",
            "+2 perception", "+2 maxEnergy", "+2 maxHp", "0.5 blessings · 3.5 energy"] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
        assert_eq!(i, before);
        r.fast_mode = Field::Missing;
        assert!(describe_item(&r, &i).contains("0.25 blessings · 3.5 energy"));
    }
    fn gear(id: f64, name: &str) -> Item {
        equipment::make_equipment(name, id).unwrap()
    }
    fn heart(id: f64) -> Item {
        Item {
            id,
            kind: "heart".into(),
            name: "Rat heart".into(),
            glyph: ";".into(),
            bonus: PartialStats {
                offense: 1.0.into(),
                ..Default::default()
            }
            .into(),
            exp: 1.0.into(),
            energy: 2.0.into(),
            ..Default::default()
        }
    }
    fn click_row(view: &mut PackView, index: usize) -> PackAction {
        let y = view.row_positions()[index].1 - view.list_scroll;
        view.click(LIST.0 + 10, y + 10)
    }
    fn click_action(view: &mut PackView, name: &str) -> PackAction {
        let index = view
            .selected_group()
            .unwrap()
            .actions
            .iter()
            .position(|a| a.label == name)
            .unwrap();
        let rect = view
            .button_rects()
            .into_iter()
            .find(|(i, _)| *i == index)
            .unwrap()
            .1;
        if rect.1 + rect.3 > DETAIL.1 + DETAIL.3 {
            view.detail_scroll += rect.1 + rect.3 - DETAIL.1 - DETAIL.3;
        }
        let rect = view
            .button_rects()
            .into_iter()
            .find(|(i, _)| *i == index)
            .unwrap()
            .1;
        view.click(rect.0 + 3, rect.1 + 3)
    }
    #[test]
    fn ahvamitt_ui_mask_uses_teleport_command_and_no_unsupported_apply_action() {
        let mut r=run();r.simulation_revision=34.0.into();r.offers.clear();
        let mask=corpse_core::ahvamitt::mask(301.);r.inventory=vec![mask.clone()];
        let buttons=item_actions(&r,&mask);
        let teleport=buttons.iter().find(|b|b.label=="Teleport").unwrap();assert!(teleport.blocker.is_none());
        assert!(matches!(&teleport.action,ButtonAction::Command(Command::Ability{id,..}) if id==corpse_core::ahvamitt::TELEPORT_ID));
        assert!(buttons.iter().all(|b|b.label!="Use"&&b.label!="Apply"&&b.label!="Equip"));
        assert!(describe_item(&r,&mask).contains("Grants Teleport while in your inventory."));
        let mut view=PackView::observe(&r);let before=view.stamp.clone();
        r.player.conditions.insert("ahvamittTeleportReadyAt".into(),r.tick+100.);
        view.refresh(&r);assert!(view.stamp!=before);assert_eq!(view.stamp.teleport_cooldown,100);
        assert!(item_actions(&r,&mask).iter().find(|b|b.label=="Teleport").unwrap().blocker.as_ref().unwrap().contains("100 turns"));
        r.player.conditions.shift_remove("ahvamittTeleportReadyAt");
        r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,..Default::default()}.into();
        assert!(item_actions(&r,&mask).iter().find(|b|b.label=="Teleport").unwrap().blocker.is_none());
    }
    #[test]
    fn dream_ui_dagger_apply_and_pack_refresh_restrict_consumables_not_equipment() {
        let mut r=run();r.simulation_revision=34.0.into();r.offers.clear();
        let dagger=corpse_core::phantasmii_generation::dagger(290.);
        assert!(item_actions(&r,&dagger).iter().any(|b|b.label=="Apply" && b.blocker.is_none()));
        assert!(describe_item(&r,&dagger).contains(corpse_core::phantasmii_generation::DAGGER_DESCRIPTION));
        let equipment=gear(291.,"Shortsword");r.inventory=vec![dagger.clone(),equipment.clone()];
        let mut view=PackView::observe(&r);let before=view.stamp.clone();
        r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,..Default::default()}.into();
        view.refresh(&r);assert!(view.stamp!=before);assert!(view.stamp.dreamwalking);
        assert!(item_actions(&r,&dagger).iter().all(|b|b.blocker.is_some()));
        let gear_actions=item_actions(&r,&equipment);
        assert!(gear_actions.iter().any(|b|b.label=="Main hand" && b.blocker.is_none()));
        assert!(gear_actions.iter().find(|b|b.label=="Drop").unwrap().blocker.is_some());
        assert!(gear_actions.iter().find(|b|b.label=="Throw").unwrap().blocker.is_some());
    }
    #[test]
    fn verdania_labels_are_revision_gated_without_changing_sap_effect() {
        let mut r=run();let tap=Item{id:1.,kind:"tool".into(),name:"Tree tap".into(),effect:"tree-tap".to_string().into(),..Default::default()};
        let sap=Item{id:2.,kind:"potion".into(),name:"Veridian Sap".into(),effect:"Veridian Sap".to_string().into(),identified:true.into(),..Default::default()};
        r.simulation_revision=32.0.into();assert!(describe_item(&r,&tap).contains("beside Veridian"));assert!(displayed_name(&r,&sap).contains("Veridian Sap"));
        r.simulation_revision=33.0.into();assert!(describe_item(&r,&tap).contains("beside Verdania"));assert!(displayed_name(&r,&sap).contains("Verdania Sap"));assert!(!describe_item(&r,&sap).contains("Veridian Sap"));assert_eq!(sap.effect.as_ref().unwrap(),"Veridian Sap");
    }
    #[test]
    fn borge_shard_has_targeted_use_either_hand_and_no_throw() {
        let mut r=run();r.simulation_revision=33.0.into();
        let shard=corpse_core::borge::shard(201.);r.inventory=vec![shard.clone()];
        let actions=item_actions(&r,&shard);
        for name in ["Use on…","Main hand","Offhand","Drop"] {assert!(actions.iter().any(|a|a.label==name));}
        assert!(!actions.iter().any(|a|a.label=="Throw"));
        let text=describe_item(&r,&shard);assert!(text.contains(corpse_core::borge::SHARD_DESCRIPTION));assert!(!text.contains("damage per hit"));
        let before=r.clone();let mut view=PackView::observe(&r);click_row(&mut view,0);
        for _ in 0..3 {assert_eq!(click_action(&mut view,"Use on…"),PackAction::Target(PackTarget::UseAt(201.)));view.refresh(&r);}
        assert_eq!(PackTarget::UseAt(201.).command(3.,4.),Command::UseAt{id:201.,x:3.,y:4.});
        assert_eq!(r,before,"opening/retargeting never spends the shard or a turn");
        assert_eq!(view.key(Keycode::Escape,Mod::NOMOD),PackAction::Close);
        assert!(PackTarget::UseAt(201.).target_blocker(&r,r.player.x,r.player.y).is_some());
    }
    #[test]
    fn borge_shard_r39_weapon_actions_preserve_legacy_actions() {
        let mut r=run();r.offers.clear();
        let shard=corpse_core::borge::shard(201.);r.inventory=vec![shard.clone()];
        for revision in [33.,34.,35.,36.,37.,38.,39.] {
            r.simulation_revision=revision.into();
            let before=r.clone();let actions=item_actions(&r,&shard);
            for name in ["Main hand","Offhand","Drop"] {assert!(actions.iter().any(|a|a.label==name));}
            assert_eq!(actions.iter().any(|a|a.label=="Use on…"),revision<39.);
            assert_eq!(actions.iter().any(|a|a.label=="Throw"),revision>=39.);
            let text=describe_item(&r,&shard);
            assert!(text.contains(corpse_core::borge::SHARD_DESCRIPTION));
            assert!(!text.contains("damage per hit"));
            assert_eq!(text.contains("No accuracy roll"),revision<39.);
            if revision>=39. {
                assert_eq!(text,format!("{}\nUnequipped\n{}",corpse_core::borge::SHARD_NAME,corpse_core::borge::SHARD_DESCRIPTION));
                assert!(actions.iter().all(|a|!matches!(a.action,ButtonAction::Target(PackTarget::UseAt(_)))));
            }
            assert_eq!(r,before,"action and description projection are pure");
        }
    }
    #[test]
    fn borge_shard_r39_throw_mouse_keyboard_cancel_and_equipment_refresh() {
        let mut r=run();r.simulation_revision=39.0.into();r.offers.clear();
        r.inventory=vec![corpse_core::borge::shard(201.)];
        let before=r.clone();let mut view=PackView::observe(&r);click_row(&mut view,0);
        assert_eq!(click_action(&mut view,"Main hand"),PackAction::Command(Command::Equip{id:201.,slot:"hand1".into()}));
        view.refresh(&r);
        assert_eq!(click_action(&mut view,"Offhand"),PackAction::Command(Command::Equip{id:201.,slot:"hand2".into()}));
        view.refresh(&r);
        for _ in 0..3 {
            assert_eq!(click_action(&mut view,"Throw"),PackAction::Target(PackTarget::Throw(201.)));
            view.refresh(&r);
            assert_eq!(view.key(Keycode::Escape,Mod::NOMOD),PackAction::Close);
        }
        view.cancel_selection();
        let index=view.selected_group().unwrap().actions.iter().position(|a|a.label=="Throw").unwrap();
        for _ in 0..=index {assert_eq!(view.key(Keycode::Tab,Mod::NOMOD),PackAction::None);}
        assert_eq!(view.key(Keycode::Return,Mod::NOMOD),PackAction::Target(PackTarget::Throw(201.)));
        assert_eq!(PackTarget::Throw(201.).command(3.,4.),Command::Throw{id:201.,x:3.,y:4.});
        assert_eq!(r,before,"opening, aiming, canceling and selecting do not mutate run");
        r.inventory[0].equipped="hand2".to_string().into();view.refresh(&r);
        assert!(view.selected_group().unwrap().actions.iter().any(|a|a.label=="Unequip"));
        assert!(view.selected_group().unwrap().actions.iter().any(|a|a.label=="Throw"));
        assert!(!view.selected_group().unwrap().actions.iter().any(|a|a.label=="Use on…"));
        r.inventory.clear();view.refresh(&r);assert!(view.groups.is_empty());assert!(view.selected.is_none());
        assert_eq!(view.key(Keycode::Return,Mod::NOMOD),PackAction::None);
    }
    #[test]
    fn borge_shard_r39_ordinary_projectile_preview_is_pure_and_dream_weapon_exception_holds() {
        let mut r=run();r.simulation_revision=39.0.into();r.offers.clear();
        let shard=corpse_core::borge::shard(201.);r.inventory=vec![shard.clone()];
        let before=r.clone();let target=Some(crate::targeting_ui::Point{x:3,y:4});
        let first=crate::targeting_ui::observe(&r,target);
        for _ in 0..3 {assert_eq!(crate::targeting_ui::observe(&r,target),first);}
        assert_eq!(r,before);
        r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,..Default::default()}.into();
        let actions=item_actions(&r,&shard);
        assert!(actions.iter().find(|a|a.label=="Throw").unwrap().blocker.is_none());
        assert!(corpse_core::dreamwalking::dream_actor(&r,&r.player));
        let dream_before=r.clone();let mut view=PackView::observe(&r);click_row(&mut view,0);
        assert_eq!(click_action(&mut view,"Throw"),PackAction::Target(PackTarget::Throw(201.)));
        assert_eq!(PackTarget::Throw(201.).command(3.,4.),Command::Throw{id:201.,x:3.,y:4.});
        assert_eq!(r,dream_before,"dream weapon targeting is read-only");
        let consumable=heart(202.);r.inventory.push(consumable.clone());
        assert!(item_actions(&r,&consumable).iter().find(|a|a.label=="Throw").unwrap().blocker.is_some());
        for name in ["Main hand","Offhand"] {assert!(actions.iter().find(|a|a.label==name).unwrap().blocker.is_none());}
        assert!(!actions.iter().any(|a|a.label=="Use on…"));
    }
    #[test]
    fn borge_heart_provenance_is_visible_and_refreshes_without_changing_reward() {
        for revision in [29.,52.] {
        let mut r=run();r.simulation_revision=revision.into();let mut h=heart(201.);r.inventory=vec![h.clone()];let mut view=PackView::observe(&r);
        assert!(!displayed_name(&r,&h).contains("marked"));
        h.source_actor_id=44.0.into();h.borge_depth=12.0.into();r.inventory[0]=h.clone();view.refresh(&r);
        assert!(displayed_name(&r,&h).contains("(marked)"));assert!(describe_item(&r,&h).contains("exact heart"));assert!(describe_item(&r,&h).contains("depth 12"));
        assert_eq!(h.bonus,heart(201.).bonus);assert_eq!(h.exp,heart(201.).exp);assert_eq!(h.energy,heart(201.).energy);
        assert!(view.groups[0].name.contains("(marked)"));
        assert_eq!(view.groups[0].name.matches("(marked)").count(),1);
        assert!(view.row_label(&view.groups[0]).contains("(marked)"));
        assert!(describe_item(&r,&h).lines().next().unwrap().contains("(marked)"));
        }
    }
    #[test]
    fn source_grouping_and_descriptions_64_cases() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/pack-ui-v48.json")).unwrap();
        assert_eq!(cases["cases"].as_array().unwrap().len(), 64);
        for case in cases["cases"].as_array().unwrap() {
            let mut raw = serde_json::to_value(run()).unwrap();
            raw["inventory"] = case["inventory"].clone();
            for (key, value) in case["changes"].as_object().unwrap() {
                raw[key] = value.clone();
            }
            let r: Run = serde_json::from_value(raw).unwrap();
            if r.inventory.iter().any(|item| item.kind == "weapon") {
                assert_eq!(combat::effective_stats(&r, &r.player).offense, 0.,
                    "literal numeric-copy oracle requires the frozen zero-Offense fixture");
            }
            let view = PackView::observe(&r);
            assert_eq!(
                view.slots,
                case["slots"].as_f64().unwrap(),
                "{} slots",
                case["name"]
            );
            assert_eq!(
                view.capacity,
                case["capacity"].as_f64().unwrap(),
                "{} capacity",
                case["name"]
            );
            let actual:Vec<_>=view.groups.iter().map(|g|serde_json::json!({"ids":g.items.iter().map(|i|i.id).collect::<Vec<_>>(),"count":g.count,"name":g.name})).collect();
            let expected: Vec<_> = case["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| serde_json::json!({"ids":g["ids"],"count":g["count"],"name":g["name"].as_str().unwrap().replace("Potion of offense","Potion of Damage").replace("Potion of defense","Potion of HP")}))
                .collect();
            assert!(
                corpse_core::storage::canonical_equal(
                    &serde_json::json!(actual),
                    &serde_json::json!(expected)
                ),
                "{} grouping",
                case["name"]
            );
            for (g, expected) in view.groups.iter().zip(case["groups"].as_array().unwrap()) {
                // PartialStats has a fixed typed key order, unlike JS insertion
                // order. Content is compared with stat-list token order ignored.
                assert_eq!(
                    normalize_stat_lists(&g.description),
                    normalize_stat_lists(&requested_copy_over_frozen_pack_description(expected["description"].as_str().unwrap())),
                    "{}: {}",
                    case["name"],
                    g.name
                );
            }
        }
    }
    // The 64-case source fixture remains immutable: grouping, inventory order,
    // grants and all unrelated prose still compare in full. These explicit
    // designer-requested presentation replacements are independent of the live
    // description/preview helpers. Its weapon fixtures all have zero Offense;
    // literal rounded ranges pin the new numeric copy rather than its formula.
    fn requested_copy_over_frozen_pack_description(old: &str) -> String {
        // Approved arrow-copy cut retains only its name and quantity.
        if old.starts_with("Arrows ×") { return old.lines().next().unwrap().to_string(); }
        let equipped = old.contains("Weapon · Main hand")
            || old.contains("Weapon · Offhand") || old.contains("Weapon · Both hands");
        let suffix = if equipped { "" } else { " with current stats" };
        let mut text = old.to_string();
        for (old_line, hands, range) in [
            ("1-handed · 30 base damage · ×1 Offense", "1-handed", "12–19"),
            ("1-handed · 40 base damage · ×1 Offense", "1-handed", "15–25"),
            ("2-handed · 45 base damage · ×2 Offense", "2-handed", "17–29"),
        ] {
            text = text.replace(old_line, &format!("{hands}\nDamage{suffix}: {range} before defenses and ability bonuses."));
        }
        text = text.replace("\nFinal damage varies with Offense, defenses, and abilities, then scales ×0.5 and rounds to a whole number.", "");
        text = text.replace("\n12–19 damage per hit before enemy defenses and strike bonuses. Only if both survive the main strike.", " Only if both survive the main strike.");
        text = text.replace("\n12–19 damage per hit before enemy defenses and strike bonuses.", "");
        let thrown = if old.contains("2-handed · 45 base damage") { "0–0" }
            else if old.contains("1-handed · 40 base damage") { "8–13" }
            else { "6–10" };
        text = text.replace("Thrown: half weapon damage for one-handed weapons; otherwise Offense damage.",
            &format!("Thrown damage with current stats: {thrown} before defenses and ability bonuses."));
        text = text.replace("Thrown weapon: full attack damage when thrown.",
            "Thrown damage with current stats: 12–19 before defenses and ability bonuses.");
        for (base, range) in [(15, "6–10"), (30, "12–19")] {
            text = text.replace(&format!("Shots: {base} base damage + ×0.5 Offense, before variance, defenses, and whole-number rounding."),
                &format!("Shot damage{suffix}: {range} before defenses and ability bonuses."));
        }
        text = text.replace("\nWhen you use a potion, mark it diluted. You can use it again at half efficacy. Then it's actually gone.", "");
        text = text.replace(", up to maximum health.", ".");
        text = text.replace(", up to your energy limit.", ".");
        text = text.replace("Already tinkered; cannot be tinkered again.", "You've already tinkered with this item.");
        text = text.replace("Releases gas that deals 1/15 of maximum HP per normal turn. Gas vulnerability applies before rounding up; damage prevention applies afterward. Dilution reduces the gas volume, not its damage.", "Releases caustic gas.");
        // Literal old/new bonus-unit oracle, independent of presentation helpers.
        for (old,new) in [("offense","Damage"),("defense","HP")] {
            text=text.replace(&format!("Potion of {old}"), &format!("Potion of {new}"));
            text=text.replace(&format!(", {old},"), &format!(", {new},"));
        }
        for n in -200..=200 {
            text=text.replace(&format!("{n:+} defense"), &format!("{:+} HP",n*6));
            text=text.replace(&format!("{n:+} offense"), &format!("{n:+} Damage"));
        }
        text.replace("Releases gas that confuses creatures for 8 turns, refreshed by exposure. Movement goes in a random direction; bumping an attackable enemy attacks it. Dilution reduces gas volume.", "Releases confusion gas.")
    }
    fn normalize_stat_lists(text: &str) -> String {
        text.lines()
            .map(|line| {
                if !line.contains(", ")
                    || !line.split(", ").all(|part| {
                        part.trim_start_matches("When equipped: ")
                            .trim_start_matches("Inactive equipment bonuses: ")
                            .starts_with(['+', '-'])
                    })
                {
                    return line.to_string();
                }
                let (prefix, rest) = line.split_once(": ").map_or(("", line), |(a, b)| (a, b));
                let mut stats = rest.split(", ").collect::<Vec<_>>();
                stats.sort();
                format!("{prefix}|{}", stats.join(", "))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn visually_stacked_items_keep_original_member_and_slots() {
        let mut r = run();
        r.inventory = vec![
            gear(201., "Ring"),
            gear(202., "Ring"),
            heart(203.),
            heart(204.),
            equipment::make_arrows(205., 500.),
        ];
        r.inventory[3].quantity = 3.0.into();
        r.inventory_display_order = vec![204., 202., 205., 201., 203.].into();
        let mut view = PackView::observe(&r);
        assert_eq!(view.groups.len(), 3);
        assert_eq!(view.slots, 7.);
        assert_eq!(view.groups[0].item.id, 203.);
        assert_eq!(view.groups[0].count, 4.);
        click_row(&mut view, 0);
        assert_eq!(
            click_action(&mut view, "Eat"),
            PackAction::Command(Command::Use { id: 203. })
        );
        assert_eq!(
            click_action(&mut view, "Eat"),
            PackAction::None,
            "duplicate activation waits for refresh"
        );
    }
    #[test]
    fn equipment_selection_and_slot_commands_follow_source() {
        let mut r = run();
        r.inventory = vec![
            gear(201., "Shortsword"),
            gear(202., "Ring"),
            gear(203., "Greatsword"),
        ];
        let mut v = PackView::observe(&r);
        click_row(&mut v, 0);
        assert_eq!(
            click_action(&mut v, "Offhand"),
            PackAction::Command(Command::Equip {
                id: 201.,
                slot: "hand2".into()
            })
        );
        corpse_core::inventory::command(
            &mut r,
            &Command::Equip {
                id: 201.,
                slot: "hand2".into(),
            },
        )
        .unwrap();
        v.refresh(&r);
        assert!(crafting::is_equipped(&v.groups[0].item));
        assert_eq!(v.selected, Some(201.));
        assert_eq!(
            click_action(&mut v, "Unequip"),
            PackAction::Command(Command::Unequip { id: 201. })
        );
        v.refresh(&r);
        click_row(&mut v, 1);
        assert_eq!(
            click_action(&mut v, "Ring 2"),
            PackAction::Command(Command::Equip {
                id: 202.,
                slot: "ring2".into()
            })
        );
        v.refresh(&r);
        click_row(&mut v, 2);
        assert!(!v
            .selected_group()
            .unwrap()
            .actions
            .iter()
            .any(|a| a.label == "Offhand"));
        assert_eq!(
            click_action(&mut v, "Both hands"),
            PackAction::Command(Command::Equip {
                id: 203.,
                slot: "hand1".into()
            })
        );
    }
    #[test]
    fn bow_apply_is_gated_by_equipment_and_arrows() {
        let mut r = run();
        r.inventory = vec![gear(201., "Bow"), equipment::make_arrows(202., 2.)];
        let mut v = PackView::observe(&r);
        click_row(&mut v, 0);
        assert_eq!(
            click_action(&mut v, "Apply"),
            PackAction::Notice("Equip this bow in both hands before firing.".into())
        );
        r.inventory[0].equipped = "hand1".to_string().into();
        v.refresh(&r);
        assert_eq!(
            click_action(&mut v, "Apply"),
            PackAction::Target(PackTarget::Shoot(201.))
        );
        assert_eq!(
            PackTarget::Shoot(201.).command(3., 4.),
            Command::Shoot {
                id: 201.,
                x: 3.,
                y: 4.
            }
        );
        r.inventory.pop();
        v.refresh(&r);
        assert_eq!(
            click_action(&mut v, "Apply"),
            PackAction::Notice("You have no arrows.".into())
        );
    }
    #[test]
    fn crafting_selection_cancel_stale_and_valid_paths() {
        let mut r = run();
        r.inventory = vec![heart(201.), gear(202., "Ring")];
        r.player.abilities.push("isheilah-guile-craft".into());
        let mut v = PackView::observe(&r);
        click_row(&mut v, 0);
        assert_eq!(click_action(&mut v, "Guile Craft"), PackAction::None);
        assert!(v.request.is_some());
        assert!(matches!(click_row(&mut v, 0), PackAction::Notice(_)));
        assert!(v.request.is_some());
        assert_eq!(v.key(Keycode::Escape, Mod::NOMOD), PackAction::None);
        assert!(v.request.is_none());
        assert_eq!(v.key(Keycode::Escape, Mod::NOMOD), PackAction::Close);
        v.selected = Some(201.);
        click_action(&mut v, "Guile Craft");
        r.tick += 1.;
        v.refresh(&r);
        assert!(v.request.is_none());
        v.selected = Some(201.);
        click_action(&mut v, "Guile Craft");
        assert_eq!(
            click_row(&mut v, 1),
            PackAction::Command(Command::Craft {
                heart: 201.,
                item: 202.,
                craft: "isheilah-guile-craft".into()
            })
        );
        assert!(v.request.is_none());
        assert_eq!(click_row(&mut v, 1), PackAction::None);
    }
    #[test]
    fn reorder_never_crosses_sections_or_changes_simulation() {
        let mut r = run();
        r.inventory = vec![
            gear(201., "Ring"),
            gear(202., "Amulet"),
            gear(203., "Shortsword"),
        ];
        r.inventory[2].equipped = "hand1".to_string().into();
        let before = r.clone();
        let mut v = PackView::observe(&r);
        assert_eq!(v.move_group(0, 1), PackAction::None);
        assert_eq!(
            v.move_group(1, 1),
            PackAction::Order(vec![203., 202., 201.])
        );
        assert_eq!(r, before);
        r.inventory_display_order = v.display_order().into();
        v.refresh(&r);
        assert_eq!(v.display_order(), vec![203., 202., 201.]);
    }
    #[test]
    fn read_only_pack_consumes_inputs_without_actions() {
        let mut r = run();
        r.inventory = vec![heart(201.), gear(202., "Bow")];
        let mut v = PackView::observe(&r);
        v.set_read_only(true);
        click_row(&mut v, 0);
        assert!(v.button_rects().is_empty());
        assert_eq!(v.key(Keycode::Return, Mod::NOMOD), PackAction::None);
        assert_eq!(v.key(Keycode::D, Mod::NOMOD), PackAction::None);
        assert_eq!(v.click(0, 0), PackAction::None);
        assert_eq!(v.move_group(0, 1), PackAction::None);
    }
    #[test]
    fn paint_and_repeated_open_close_are_fixed_nonmutating_and_clipped() {
        let mut r = run();
        r.inventory = vec![
            heart(201.),
            gear(202., "Bow"),
            equipment::make_arrows(203., 100.),
        ];
        let before = r.clone();
        let mut v = PackView::observe(&r);
        let mut painter = Painter::new();
        for _ in 0..4 {
            painter.clear([93, 31, 113]);
            v.paint(&mut painter);
            assert_eq!(v.key(Keycode::Escape, Mod::NOMOD), PackAction::Close);
            v.refresh(&r);
            // The resolved responsive modal, not the obsolete fixed BOUNDS,
            // defines the paint scissor. Check the entire exterior, not samples.
            let Rect(bx,by,bw,bh)=v.layout.bounds;
            for y in 0..painter.height { for x in 0..painter.width {
                if (bx..bx+bw).contains(&(x as i32))&&(by..by+bh).contains(&(y as i32)){continue;}
                let at=(y as usize*painter.width as usize+x as usize)*4;
                assert_eq!(&painter.pixels[at..at+3],&[93,31,113],"outside modal at {x},{y}");
            }}
            assert_eq!(painter.clip, None);
        }
        assert_eq!(r, before);
        click_row(&mut v, 1);
        painter.game(&crate::observe::observe(&r), "", false);
        v.paint(&mut painter);
        if std::env::var_os("PACK_TEST_PNG").is_some() {
            let file = std::fs::File::create("/tmp/native-pack-test.png").unwrap();
            let mut png = png::Encoder::new(
                file,
                crate::render::WIDTH as u32,
                crate::render::HEIGHT as u32,
            );
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header()
                .unwrap()
                .write_image_data(&painter.pixels)
                .unwrap();
        }
    }
    #[test]
    fn supported_tinker_targets_once_without_spending_during_selection() {
        let mut r = run();
        r.inventory = vec![gear(201., "Ring")];
        r.player.abilities.push("kative-tinker".into());
        r.player.energy = 150.;
        let before = r.clone();
        let mut v = PackView::observe(&r);
        assert_eq!(v.begin_tinker(&r, "kative-tinker"), PackAction::None);
        assert!(v.request.is_some());
        assert_eq!(r, before);
        assert_eq!(v.key(Keycode::Escape, Mod::NOMOD), PackAction::None);
        assert!(v.request.is_none());
        assert_eq!(v.begin_tinker(&r, "kative-tinker"), PackAction::None);
        assert_eq!(
            v.key(Keycode::Return, Mod::NOMOD),
            PackAction::Command(Command::Ability {
                id: "kative-tinker".into(),
                target: 201.0.into(),
                x: Field::Missing,
                y: Field::Missing
            })
        );
        assert_eq!(v.key(Keycode::Return, Mod::NOMOD), PackAction::None);
        assert_eq!(r, before);
        r.player.energy = 99.;
        v.refresh(&r);
        assert_eq!(
            v.begin_tinker(&r, "kative-tinker"),
            PackAction::Notice("Needs 100 energy".into())
        );
    }
    #[test]
    fn tinker_availability_and_session_invalidation_follow_source() {
        let mut r = run();
        r.inventory = vec![gear(201., "Ring")];
        r.player.abilities.push("kative-tinker".into());
        r.player.energy = 150.;
        let mut v = PackView::observe(&r);
        assert_eq!(v.begin_tinker(&r, "kative-tinker"), PackAction::None);
        r.player.conditions.insert("silenced".into(), 3.);
        v.refresh(&r);
        assert!(v.request.is_none());
        assert_eq!(
            v.begin_tinker(&r, "kative-tinker"),
            PackAction::Notice("Energy-paid activation is silenced".into())
        );
        r.player.conditions.insert("paralyzed".into(), 2.);
        assert_eq!(
            v.begin_tinker(&r, "kative-tinker"),
            PackAction::Notice("You are paralyzed. Wait to advance time.".into())
        );
        r.player.conditions.clear();
        r.gods = (0..12).map(|n| n as f64).collect();
        assert_eq!(
            v.begin_tinker(&r, "kative-tinker"),
            PackAction::Notice("No eligible outside-god abilities remain".into())
        );
        r.inventory[0].tinkered = true.into();
        assert_eq!(
            v.begin_tinker(&r, "kative-tinker"),
            PackAction::Notice("No unblessed weapon, armor, ring or amulet to tinker with".into())
        );
    }
    #[test]
    fn available_throw_does_not_disclose_unknown_potion_contents() {
        let mut r = run();
        r.inventory = vec![Item {
            id: 201.,
            name: "unknown".into(),
            kind: "potion".into(),
            glyph: "!".into(),
            effect: "incineration".to_string().into(),
            ..Default::default()
        }];
        r.known_potions.clear();
        let v = PackView::observe(&r);
        let use_button = v.groups[0]
            .actions
            .iter()
            .find(|b| b.label == "Drink")
            .unwrap();
        assert!(use_button.blocker.is_none());
        let throw = v.groups[0]
            .actions
            .iter()
            .find(|b| b.label == "Throw")
            .unwrap();
        assert_eq!(throw.blocker.as_deref(), None);
    }
    #[test]
    fn use_command_runs_through_temporal_engine_and_refreshes_selection() {
        let mut r = run();
        r.inventory = vec![heart(201.), heart(202.)];
        r.next_id = 1000.;
        let mut engine = corpse_core::temporal::Engine::new(r).unwrap();
        let mut v = PackView::observe(&engine.run);
        click_row(&mut v, 0);
        let PackAction::Command(command) = click_action(&mut v, "Eat") else {
            panic!("missing use command")
        };
        let tick = engine.run.tick;
        let offense = engine.run.player.stats.offense;
        let outcome = engine.dispatch(&command).unwrap();
        assert!(outcome.changed);
        assert!(outcome.took_turn);
        assert_eq!(engine.run.tick, tick + 1.);
        assert_eq!(engine.run.player.stats.offense, offense + 1.);
        assert!(!engine.run.inventory.iter().any(|i| i.id == 201.));
        assert!(engine.run.inventory.iter().any(|i| i.id == 202.));
        v.refresh(&engine.run);
        assert_eq!(v.selected, None);
        assert_eq!(v.slots, 1.);
        assert_eq!(v.groups[0].item.id, 202.);
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn custom_profile_reflows_input_rows_buttons_and_paint_without_run_changes() {
        let mut run: Run =
            serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.inventory = (0..30)
            .map(|i| {
                let mut item = equipment::make_equipment("Ring", 400. + i as f64).unwrap();
                item.name = format!("Observed ring {i} with a long detached label");
                item
            })
            .collect();
        let before = run.clone();
        let mut view = PackView::observe(&run);
        let mut p = Painter::new();
        for appearance in [
            json!({}),
            json!({"ui":{"layout":{"rightSidebarWidth":600,"sidebarWidthPercent":40,"chronicleHeight":260,"inventoryRowHeight":24,"buttonHeight":54,"buttonPadding":16,"borderWidth":4},"colors":{"panel":"#123456","selected":"#654321","button":"#223344"}},"typography":{"inventory":{"fontSize":32,"lineHeight":2},"primary":{"fontSize":22},"tooltips":{"fontSize":22}}}),
        ] {
            p.profile = Profile::from_value(appearance).unwrap();
            // Each profile starts a fresh pack: prior Home/scroll state must not
            // offset the first-row click after a large font reflow.
            view = PackView::observe(&run);
            let frame = modal_ui::Frame::from_profile(&p.profile);
            view.configure(&p, frame.map.tuple());
            let layout = view.layout;
            assert_eq!(view.bounds(), layout.bounds.tuple());
            assert!(layout.row >= p.profile.number("ui.layout.inventoryRowHeight") as i32);
            let (_, first) = view.row_positions()[0];
            assert_eq!(view.click(layout.list.0 + 8, first + 2), PackAction::None);
            assert!(view.selected.is_some());
            let lines = view.detail_lines();
            assert!(!lines.is_empty());
            for (line, _) in lines {
                assert!(
                    view.text_metrics.as_ref().unwrap().width(&line)
                        <= (layout.detail.2 - 14).max(1) as f64 + 40.
                );
            }
            for _ in 0..4 {
                view.key(Keycode::Tab, Mod::NOMOD);
            }
            if let Some(index) = view.focus_action {
                let (_, button) = view
                    .button_rects()
                    .into_iter()
                    .find(|(i, _)| *i == index)
                    .unwrap();
                assert!(
                    button.0 >= layout.detail.0
                        && button.0 + button.2 <= layout.detail.0 + layout.detail.2
                );
                assert!(button.1 + button.3 <= layout.detail.1 + layout.detail.3);
            }
            view.scroll(-100, layout.list.0 + 4, layout.list.1 + 4);
            assert_eq!(view.list_scroll, view.list_max_scroll());
            view.key(Keycode::Home, Mod::NOMOD);
            assert!(view.list_scroll < view.list_max_scroll());
            p.clear([93, 31, 113]);
            view.paint(&mut p);
            for (x, y) in [
                (layout.bounds.0 - 1, layout.bounds.1),
                (layout.bounds.0 + layout.bounds.2, layout.bounds.1),
                (layout.bounds.0, layout.bounds.1 - 1),
                (layout.bounds.0, layout.bounds.1 + layout.bounds.3),
            ] {
                let at = (y as usize * crate::render::WIDTH + x as usize) * 4;
                assert_eq!(&p.pixels[at..at + 3], &[93, 31, 113]);
            }
            assert_eq!(
                view.click(layout.close.0 + 5, layout.close.1 + 5),
                PackAction::Close
            );
            assert_eq!(run, before);
            if let Ok(dir) = std::env::var("CORPSE_MODAL_SCREENSHOTS") {
                std::fs::create_dir_all(&dir).unwrap();
                let file = std::fs::File::create(std::path::Path::new(&dir).join(
                    if p.profile == Profile::default() {
                        "pack-default.png"
                    } else {
                        "pack-authored.png"
                    },
                ))
                .unwrap();
                let mut e = png::Encoder::new(
                    file,
                    crate::render::WIDTH as u32,
                    crate::render::HEIGHT as u32,
                );
                e.set_color(png::ColorType::Rgba);
                e.set_depth(png::BitDepth::Eight);
                e.write_header()
                    .unwrap()
                    .write_image_data(&p.pixels)
                    .unwrap();
            }
        }
    }
}

#[cfg(test)]
mod completed_engine_action_tests {
    use super::*;
    #[test]
    fn potions_expose_the_implemented_throw_target_action() {
        let run: Run =
            serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for effect in [
            "healing",
            "energy",
            "incineration",
            "caustic gas",
            "paralysis",
            "offense",
            "defense",
            "accuracy",
            "dodge",
            "detect magic",
            "telepathy",
            "levitation",
            "speed",
            "confusion",
            "invisibility",
            "descent",
            "Veridian Sap",
        ] {
            let item = Item {
                id: 123.,
                kind: "potion".into(),
                name: format!("Potion of {effect}"),
                effect: effect.to_string().into(),
                ..Default::default()
            };
            let buttons = item_actions(&run, &item);
            let button = buttons.iter().find(|b| b.label == "Throw").unwrap();
            assert!(button.blocker.is_none(), "{effect}");
            assert!(matches!(
                button.action,
                ButtonAction::Target(PackTarget::Throw(123.))
            ));
        }
    }
    #[test]
    fn tree_tap_exposes_use_without_admitting_unknown_tools() {
        let run: Run =
            serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for (effect, allowed) in [("tree-tap", true), ("unknown-tool", false)] {
            let item = Item {
                id: 124.,
                kind: "tool".into(),
                name: "Tree tap".into(),
                effect: effect.to_string().into(),
                ..Default::default()
            };
            let buttons = item_actions(&run, &item);
            let button = buttons.iter().find(|b| b.label == "Use").unwrap();
            assert_eq!(button.blocker.is_none(), allowed);
            assert!(matches!(
                button.action,
                ButtonAction::Command(Command::Use { id: 124. })
            ));
        }
    }
}

include!("retained_pack_tests.rs");

#[cfg(test)]
mod revision43_copy_tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=43.0.into();r.inventory.clear();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.stats.offense=20.;r
    }
    #[test]
    fn requested_short_item_copy_and_legacy_mechanical_descriptions() {
        let mut r=room();let honey=corpse_core::royal_honey::item(301.);
        assert_eq!(describe_item(&r,&honey),"9999 health, 9999 energy");
        let shard=corpse_core::borge::shard(302.);
        assert_eq!(describe_item(&r,&shard),format!("{}\nUnequipped\nPermanently kill a creature and destroy the shard.",shard.name));
        let fragment=corpse_core::phantasmii_generation::shard(303.);
        assert_eq!(describe_item(&r,&fragment),format!("{} ×1\nCollect three to complete the ritual.",fragment.name));
        let mask=corpse_core::ahvamitt::mask(304.);
        assert_eq!(item_actions(&r,&mask).iter().find(|b|b.label=="Teleport").unwrap().tip,"Teleport to a random standable spot; 100-turn cooldown.");
        for (effect,line) in [("caustic gas","Releases caustic gas."),("confusion","Releases confusion gas.")] {
            let potion=Item{id:305.,kind:"potion".into(),effect:effect.to_string().into(),identified:true.into(),..Default::default()};
            assert!(describe_item(&r,&potion).lines().any(|s|s==line));
            assert!(!describe_item(&r,&potion).contains("Dilution reduces"));
        }
        r.simulation_revision=42.0.into();
        assert!(describe_item(&r,&honey).contains("fully restore"));
        assert!(describe_item(&r,&shard).contains(corpse_core::borge::SHARD_DESCRIPTION));
    }
    #[test]
    fn silence_and_exhausted_tinker_ui_show_requested_messages() {
        let mut r=room();r.player.energy=150.;r.player.conditions.insert("silenced".into(),1.);
        assert_eq!(tinker_blocker(&r).as_deref(),Some("Your ability is silenced and does not work."));
        r.player.conditions.clear();r.gods=(0..12).map(|g|g as f64).collect();
        r.inventory.push(Item{id:309.,name:"Ring".into(),kind:"ring".into(),..Default::default()});
        assert_eq!(tinker_blocker(&r).as_deref(),Some("You can't think of any new ideas to tinker with..."));
    }
    #[test]
    fn tinkered_blocker_and_description_use_same_requested_copy() {
        let r=room();let item=Item{id:306.,name:"Sword".into(),kind:"weapon".into(),tinkered:true.into(),..Default::default()};
        let mut view=PackView::observe(&r);view.request=Some(Request::Tinker{ability:"yasmui-tinker".into()});
        assert_eq!(view.target_blocker(&item).as_deref(),Some("You've already tinkered with this item."));
        assert!(describe_item(&r,&item).lines().any(|s|s=="You've already tinkered with this item."));
    }
    #[test]
    fn funded_fury_first_strike_respects_assault_offhand_and_free_refresh() {
        let mut r=room();r.player.energy=10.;
        let mut item=Item{id:308.,name:"Sword".into(),kind:"weapon".into(),hands:1.0.into(),damage:30.0.into(),enchantment:2.0.into(),offense_multiplier:1.5.into(),equipped:"hand1".to_string().into(),..Default::default()};
        item.enhancements.push(corpse_core::ItemEnhancementsEntry{craft:"Fury".into(),active:true.into(),..Default::default()});
        r.inventory.push(item.clone());let before=serde_json::to_vec(&r).unwrap();
        let mut view=PackView::observe(&r);
        assert!(view.groups[0].description.contains("Empowers your first strike for 10 energy."));
        assert!(view.groups[0].description.contains("Fury first strike: 66–110"));
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        let tick=r.tick;
        r.player.active.push(abilities::ASSAULT.into());view.refresh(&r);
        assert!(!view.groups[0].description.contains("Fury first strike:"));
        assert!(view.groups[0].description.contains("Fury throw:"));
        r.player.stats.offense=30.;view.refresh(&r);
        assert!(view.groups[0].description.contains("Damage: 29–49"));assert_eq!(r.tick,tick);
        r.player.active.clear();r.inventory[0].equipped="hand2".to_string().into();view.refresh(&r);
        assert!(!view.groups[0].description.contains("Fury first strike:"));
        r.inventory[0].enhancements[0].active=false.into();view.refresh(&r);
        assert!(!view.groups[0].description.contains("Fury throw:"));
        r.simulation_revision=42.0.into();
        assert!(describe_item(&r,&r.inventory[0]).contains("+10 first-strike damage"));
    }
    #[test]
    fn damage_preview_is_numeric_current_stats_and_uses_shared_hit_math() {
        let r=room();let before=serde_json::to_vec(&r).unwrap();
        let item=Item{id:307.,name:"Sword".into(),kind:"weapon".into(),hands:1.0.into(),damage:30.0.into(),enchantment:2.0.into(),offense_multiplier:1.5.into(),..Default::default()};
        let text=describe_item(&r,&item);
        assert!(text.contains("Damage with current stats: 24–39 before defenses and ability bonuses."));
        assert!(text.contains("Thrown damage with current stats: 12–20 before defenses and ability bonuses."));
        for removed in ["base damage","×1.5 Offense","Final damage varies","scales ×0.5","half weapon damage"]{assert!(!text.contains(removed),"{removed}");}
        let mut bow=item.clone();bow.ranged_weapon="bow".to_string().into();bow.hands=2.0.into();
        assert!(describe_item(&r,&bow).contains("Shot damage with current stats: 24–39 before defenses and ability bonuses."));
        let mut zero=item.clone();zero.damage=(-32.0).into();
        assert!(weapon_damage_preview(&r,&zero,false,20.).contains("0–0"));
        let mut infinite=item.clone();infinite.damage=f64::INFINITY.into();
        assert!(weapon_damage_preview(&r,&infinite,false,20.).contains("inf–inf"));
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}

fn pack_limited_occupancy(run: &Run) -> f64 {
    if run.simulation_revision.value_or(29.) >= 50. { corpse_core::revision50_inventory::limited_occupancy(run) }
    else { inventory::occupancy(run) }
}
fn pack_capacity(run: &Run) -> f64 {
    if run.simulation_revision.value_or(29.) >= 50. { corpse_core::revision50_inventory::capacity(run).total() }
    else { crafting::capacity(&run.inventory) }
}

fn item_section(revision: f64, item: &Item) -> usize {
    if revision < 50. { return usize::from(!crafting::is_equipped(item)); }
    use corpse_core::revision50_inventory::{category, Category};
    match category(item) {
        Some(Category::Trinket) => 0,
        Some(Category::Amulet) => 1,
        Some(Category::Ring) => 2,
        Some(Category::Potion) => 3,
        Some(Category::Heart) => 4,
        None => 5,
    }
}
/// O(I + B) category projection; trinkets count stacks, finite categories count units.
/// Unknown items in direct/debug projections are shown separately, never admitted as trinkets.
fn category_headers(run: &Run) -> Vec<String> {
    if run.simulation_revision.value_or(29.) < 50. { return Vec::new(); }
    let c = corpse_core::revision50_inventory::capacity(run);
    let mut used = [0.; 6];
    let mut unsupported = false;
    for item in &run.inventory {
        let section = item_section(50., item);
        unsupported |= section == 5;
        used[section] += if section == 0 { 1. } else { inventory::quantity(item) };
    }
    let mut headers = vec![
        format!("Trinkets {} · unlimited", used[0]),
        format!("Amulets {}/{}", used[1], c.amulets),
        format!("Rings {}/{}", used[2], c.rings),
        format!("Potions {}/{}", used[3], c.potions),
        format!("Hearts {}/{}", used[4], c.hearts),
    ];
    if unsupported { headers.push(format!("Unsupported items {}", used[5])); }
    headers
}
#[cfg(test)]
fn category_summary(run: &Run) -> String {
    category_headers(run).join(" · ")
}
#[cfg(test)]
#[path="revision50_item_knowledge_tests.rs"]
mod revision50_item_knowledge_tests;

#[cfg(test)]
#[path="revision52_amulet_ui_tests.rs"]
mod revision52_amulet_ui_tests;

#[cfg(test)]
#[path="revision50_jewelry_capture_tests.rs"]
mod revision50_jewelry_capture_tests;

#[cfg(test)]
mod revision50_pack_tests {
    use super::*;
    fn run() -> Run { let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap(); r.simulation_revision = 50.0.into(); r }
    #[test] fn category_capacity_and_equipment_actions_are_versioned() {
        let mut r=run(); r.player.abilities.clear(); assert_eq!(pack_capacity(&r),20.);
        let weapon=Item { id:900., kind:"weapon".into(), ..Default::default() };
        assert!(!item_actions(&r,&weapon).iter().any(|b| matches!(b.action,ButtonAction::Command(Command::Equip{..}))));
        for kind in ["ring", "amulet"] {
            for equipped in [Field::Missing, "ring1".to_string().into()] {
                let item=Item { id:901., kind:kind.into(), equipped, ..Default::default() };
                assert!(!item_actions(&r,&item).iter().any(|b| matches!(b.action,
                    ButtonAction::Command(Command::Equip{..}|Command::Unequip{..}))));
                let text=describe_item(&r,&item);
                assert!(text.lines().any(|line|line==titlecase(kind)));
                assert!(!text.contains("while carried"));
            }
        }
        r.player.abilities=vec!["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into(),"kative-appropriate".into()];
        assert_eq!(pack_capacity(&r),43.);
        r.simulation_revision=49.0.into();
        let ring=Item { id:901., kind:"ring".into(), ..Default::default() };
        assert!(item_actions(&r,&ring).iter().any(|b|b.label=="Equip"));
        assert!(describe_item(&r,&ring).contains("Unequipped"));
    }
    #[test] fn carried_jewelry_rows_omit_redundant_activity_and_capacity_explanation() {
        let mut r=run();r.inventory.clear();r.player.abilities.clear();
        r.inventory.push(Item{id:900.,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),..Default::default()});
        r.inventory.push(corpse_core::revision50_amulet_effects::make_amulet(corpse_core::revision50_amulet_effects::Effect::Firebolt,901.).unwrap());
        let view=PackView::observe(&r);
        assert!(view.capacity_heading().is_empty());
        for g in &view.groups {assert_eq!(view.row_label(g),g.name);assert!(!view.row_label(g).contains("active"));}
        assert!(view.stamp.category_summary.contains("Rings 1/5"));
        assert!(view.stamp.category_summary.contains("Amulets 1/5"));
    }
    #[test] fn five_empty_category_sections_keep_independent_limits() {
        let mut r=run();r.inventory.clear();r.player.abilities.clear();
        let view=PackView::observe(&r);
        assert_eq!(view.stamp.category_headers,vec!["Trinkets 0 · unlimited","Amulets 0/5","Rings 0/5","Potions 0/5","Hearts 0/5"]);
        assert_eq!(view.section_positions(),(0..5).map(|i|(i,view.layout.list.1+i as i32*24)).collect::<Vec<_>>());
        assert!(view.row_positions().is_empty());assert_eq!(view.list_max_scroll(),0);
        assert!(view.capacity_heading().is_empty());
        assert!(!view.stamp.category_summary.contains("shared"));
    }
    #[test] fn category_counts_include_all_carried_jewelry_and_stack_units() {
        let mut r=run();r.inventory.clear();r.player.abilities.clear();
        for (id,kind,quantity) in [(900.,"ring",2.),(901.,"ring",3.),(902.,"amulet",5.),(903.,"potion",5.),(904.,"heart",5.)] {
            r.inventory.push(Item{id,kind:kind.into(),name:kind.into(),quantity:quantity.into(),..Default::default()});
        }
        r.inventory[0].equipped="ring1".to_string().into();
        r.inventory.push(equipment::make_arrows(905.,500.));
        r.inventory.push(equipment::make_equipment("Bow",906.).unwrap());
        let before=r.clone();let mut view=PackView::observe(&r);
        assert_eq!(view.stamp.category_headers,vec!["Trinkets 2 · unlimited","Amulets 5/5","Rings 5/5","Potions 5/5","Hearts 5/5"]);
        assert_eq!(view.slots,20.);assert_eq!(view.capacity,20.);
        let rings=view.groups.iter().find(|g|g.item.kind=="ring").unwrap();
        assert_eq!(rings.items.len(),2);assert_eq!(rings.count,5.);
        view.selected=Some(rings.item.id);
        let text=view.detail_lines().into_iter().map(|(s,_)|s).collect::<Vec<_>>().join(" ");
        assert!(text.contains("5 ring slots"));assert!(!text.contains("no backpack space"));
        assert_eq!(r,before);
        r.player.abilities=vec!["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into(),"kative-appropriate".into()];
        view.refresh(&r);
        assert_eq!(view.stamp.category_headers,vec!["Trinkets 2 · unlimited","Amulets 5/10","Rings 5/13","Potions 5/15","Hearts 5/5"]);
        assert!(view.capacity_heading().is_empty());
    }
    #[test] fn category_order_and_reordering_never_cross_sections() {
        let mut r=run();r.inventory.clear();r.player.abilities.clear();
        for (id,kind) in [(2.,"ring"),(5.,"heart"),(4.,"potion"),(3.,"amulet"),(1.,"trinket"),(7.,"trinket"),(6.,"ring")] {
            r.inventory.push(Item{id,kind:kind.into(),name:format!("Item {id}"),..Default::default()});
        }
        r.inventory[0].equipped="ring1".to_string().into();
        r.inventory_display_order=vec![5.,4.,2.,1.,3.,7.,6.].into();
        let before=r.clone();let mut view=PackView::observe(&r);
        assert_eq!(view.display_order(),vec![1.,7.,3.,2.,6.,4.,5.]);
        assert_eq!(view.reorder_neighbor(1,1),None);
        assert_eq!(view.move_group(1,1),PackAction::None);
        let y=view.row_positions()[1].1+4;
        view.hover(view.layout.list.0+view.layout.list.2-10,y);
        assert_eq!(view.tooltip().as_deref(),Some("Items stay within their category (no turn)."));
        assert_eq!(view.move_group(0,1),PackAction::Order(vec![7.,1.,3.,2.,6.,4.,5.]));
        assert_eq!(view.move_group(3,1),PackAction::Order(vec![7.,1.,3.,6.,2.,4.,5.]));
        assert_eq!(r,before);
        r.inventory_display_order=view.display_order().into();view.refresh(&r);
        assert_eq!(view.display_order(),vec![7.,1.,3.,6.,2.,4.,5.]);
        assert_eq!(view.groups.iter().map(|g|g.section).collect::<Vec<_>>(),vec![0,0,1,2,2,3,4]);
    }
    #[test] fn category_geometry_keeps_empty_headers_and_scrolls_to_last_item() {
        let mut r=run();r.inventory.clear();r.player.abilities.clear();
        for id in 1..=20 {r.inventory.push(Item{id:id as f64,kind:"trinket".into(),name:format!("Trinket {id}"),..Default::default()});}
        r.inventory.push(Item{id:30.,kind:"heart".into(),name:"Rat heart".into(),..Default::default()});
        let before=r.clone();let mut view=PackView::observe(&r);
        let p=Painter::new();view.configure(&p,p.frame().map.tuple());
        assert_eq!(view.section_positions().len(),5);
        assert_eq!(view.click(view.layout.list.0+10,view.layout.list.1+5),PackAction::None);
        assert!(view.selected.is_none(),"a section header is not an item");
        view.key(Keycode::End,Mod::NOMOD);assert_eq!(view.selected,Some(30.));
        let y=view.row_positions().last().unwrap().1-view.list_scroll;
        assert!(y>=view.layout.list.1&&y+view.layout.row<=view.layout.list.1+view.layout.list.3);
        view.scroll(-1000,view.layout.list.0+5,view.layout.list.1+5);
        assert_eq!(view.list_scroll,view.list_max_scroll());
        view.key(Keycode::Home,Mod::NOMOD);assert_eq!(view.selected,Some(1.));
        assert!(view.list_scroll<view.list_max_scroll());assert_eq!(r,before);
    }
    #[test] fn unsupported_direct_projection_items_are_not_hidden_or_called_trinkets() {
        let mut r=run();r.player.abilities.clear();r.inventory=vec![equipment::make_equipment("Shortsword",900.).unwrap()];
        let view=PackView::observe(&r);
        assert_eq!(view.stamp.category_headers[0],"Trinkets 0 · unlimited");
        assert_eq!(view.stamp.category_headers[5],"Unsupported items 1");
        assert_eq!(view.groups[0].section,5);assert_eq!(view.row_positions().len(),1);
    }
    #[test] fn unlimited_trinkets_are_finite_in_ui_and_keep_special_actions() {
        let mut r=run();r.player.abilities.clear();r.inventory.clear();
        for id in 1..=150 {r.inventory.push(corpse_core::equipment::make_equipment("Bow",id as f64).unwrap());}
        r.inventory.push(corpse_core::equipment::make_arrows(151.,500.));
        let mask=corpse_core::ahvamitt::mask(152.);r.inventory.push(mask.clone());
        assert_eq!(pack_limited_occupancy(&r),0.);assert_eq!(pack_capacity(&r),20.);
        assert!(category_summary(&r).contains("Trinkets 152 · unlimited"));
        assert!(!category_summary(&r).contains("inf"));
        assert!(describe_item(&r,&mask).starts_with("Masque of the Trickster\n"));
        assert!(item_actions(&r,&mask).iter().any(|b|b.label=="Teleport"));
        let shard=corpse_core::borge::shard(153.);
        let actions=item_actions(&r,&shard);
        assert!(actions.iter().any(|b|b.label=="Throw"));
        assert!(!actions.iter().any(|b|matches!(b.label.as_str(),"Apply"|"Wear"|"Main hand"|"Use on…")));
        assert!(!describe_item(&r,&shard).contains("Unequipped"));
        assert!(describe_item(&r,&shard).contains("Throw"));
    }
    #[test] fn all_item_grants_have_revision50_names_and_copy() {
        let r=run(); let i=Item { kind:"ring".into(),granted_ability:"shrail-execute".to_string().into(), additional_granted_abilities:vec!["kative-appropriate".into()],..Default::default() };
        let text=describe_item(&r,&i);assert!(text.contains("Execute"));assert!(text.contains("Appropriate"));assert!(!text.contains("energy costs from amulets by 10"));
    }
    #[test] fn carried_jewelry_grant_and_bonus_copy_preserves_legacy() {
        let mut r=run();
        for kind in ["ring", "amulet"] {
            for equipped in [Field::Missing, "ring1".to_string().into()] {
                let item=Item{kind:kind.into(),equipped,
                    bonus:PartialStats{offense:1.0.into(),..Default::default()}.into(),
                    granted_ability:"shrail-execute".to_string().into(),
                    additional_granted_abilities:vec!["kative-appropriate".into()],..Default::default()};
                r.simulation_revision=50.0.into();
                let text=describe_item(&r,&item);
                assert!(text.contains("Grants Execute."));
                assert!(text.contains("Grants Appropriate."));
                assert!(text.contains("Bonuses: +1 Damage"));
                assert!(!text.contains("While carried"));
                assert!(!text.contains("While equipped")&&!text.contains("When equipped"));
                r.simulation_revision=49.0.into();
                // Execute replaced Unyielding Assault at R50; use the real legacy identity.
                let mut legacy_item=item.clone();
                legacy_item.granted_ability="shrail-unyielding-assault".to_string().into();
                let text=describe_item(&r,&legacy_item);
                assert!(text.contains("While equipped: grants Unyielding Assault."));
                assert!(text.contains("When equipped:"));
                assert!(!text.contains("While carried"));
            }
        }
    }
    #[test] fn reserve_changes_invalidate_pack_but_do_not_duplicate_ability_details() {
        let mut r=run();r.player.energy=0.;
        let id="numo-implant-thought";
        let item=Item{kind:"ring".into(),granted_ability:id.to_string().into(),..Default::default()};
        let item_text=describe_item(&r,&item);assert!(item_text.contains("Grants Implant Thought."));assert!(!item_text.contains("Reserve"));
        let before=Stamp::observe(&r);
        corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,id,31.,16.).unwrap();
        assert!(before!=Stamp::observe(&r));
        assert_eq!(describe_item(&r,&item),item_text);
        assert!(crate::ability_ui::observe(&r,id).cost.contains("Reserve 16/31"));
    }
    #[test] fn authored_ring_effect_tooltips_are_versioned_and_keep_item_grants() {
        let mut r=run();
        for (effect,fragment) in [("stealth","1.2"),("light","10 illumination"),("regeneration","Doubles your natural HP regeneration.")] {
            let i=Item{kind:"ring".into(),name:format!("Ring of {effect}"),effect:effect.to_string().into(),
                granted_ability:"shrail-execute".to_string().into(),..Default::default()};
            let text=describe_item(&r,&i);
            assert!(text.contains(fragment),"{effect}: {text}");
            assert!(!text.contains("Duplicate copies")&&!text.contains("Different ring powers"));
            assert!(text.contains("Grants Execute."));
            r.simulation_revision=46.0.into();
            assert!(!describe_item(&r,&i).contains(fragment));
            r.simulation_revision=50.0.into();
        }
    }

    #[test] fn ring_names_follow_vampirism_and_ice_immunity_clarifications() {
        let mut r=run();
        for (effect,expected) in [("transference","Vampirism Ring"),("vampirism","Vampirism Ring"),("ice-immunity","Ice Immunity Ring")] {
            let i=Item{kind:"ring".into(),name:"Old display label".into(),effect:effect.to_string().into(),..Default::default()};
            assert_eq!(displayed_name(&r,&i),expected);
            assert!(describe_item(&r,&i).starts_with(expected));
            r.simulation_revision=46.0.into();assert_eq!(displayed_name(&r,&i),"Old display label");
            r.simulation_revision=50.0.into();
        }
    }

    #[test] fn amulet_apply_energy_cost_names_and_preview_follow_current_state() {
        use corpse_core::revision50_amulet_effects::{self as amulets,Effect};
        let mut r=run();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        r.inventory.clear();r.actors=Default::default();r.player.x=10.;r.player.y=10.;r.player.energy=100.;
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
        f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;
        let item=amulets::make_amulet(Effect::Lightning,901.).unwrap();assert!(item.equipped.is_missing());
        r.inventory.push(item);
        let target=PackTarget::UseAt(901.);
        assert!(target.is_amulet(&r));assert!(target.target_blocker(&r,12.,10.).is_none());
        assert_eq!(target.command(12.,10.),Command::UseAt{id:901.,x:12.,y:10.});
        assert!(!target.application_preview(&r,Some((12,10))).is_empty());
        assert!(target.application_preview(&r,None).is_empty());
        assert!(item_actions(&r,&r.inventory[0]).iter().any(|b|b.label=="Apply" && b.blocker.is_none()));
        let text=describe_item(&r,&r.inventory[0]);assert!(text.starts_with("Lightning Amulet"));
        assert!(text.contains("50/50"));assert!(text.contains("Activation: 25 energy"));assert!(!text.contains("No cooldown"));
        let stamp=Stamp::observe(&r);r.inventory[0].amulet_state.as_mut().unwrap().energy=25.;
        assert!(stamp!=Stamp::observe(&r));assert!(describe_item(&r,&r.inventory[0]).contains("25/50"));
        r.player.abilities.push("kative-appropriate".into());
        assert!(describe_item(&r,&r.inventory[0]).contains("Activation: 15 energy"));
        r.simulation_revision=46.0.into();assert!(!target.is_amulet(&r));
        assert!(!item_actions(&r,&r.inventory[0]).iter().any(|b|b.label=="Apply"));
    }

}

#[cfg(test)] mod revision51_stat_unit_tests {
    use super::*;
    #[test] fn heart_and_equipment_bonuses_use_display_units() {
        let stats=PartialStats{offense:2.0.into(),defense:3.0.into(),..Default::default()};
        assert_eq!(bonuses(Some(&stats),None),"+2 Damage, +18 HP");
        let base=PartialStats{defense:1.2.into(),..Default::default()};
        let enchantment=PartialStats{defense:1.8.into(),..Default::default()};
        assert_eq!(bonuses(Some(&enchantment),Some(&base)),"+12 HP");
    }
}

#[cfg(test)]
mod revision51_damage_preview_tests {
    use super::*;
    #[test]
    fn revision51_bow_and_throw_previews_keep_bonuses_unscaled() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.inventory.clear();r.player.active.clear();
        r.player.abilities.clear();r.player.conditions.clear();r.player.stats.offense=20.;
        let bow=Item{kind:"weapon".into(),damage:15.0.into(),enchantment:3.0.into(),
            offense_multiplier:0.5.into(),ranged_weapon:"bow".to_string().into(),..Default::default()};
        assert!(weapon_damage_preview(&r,&bow,true,20.).contains("9–15 before defenses"));
        let hatchet=Item{kind:"weapon".into(),name:"Hatchet".into(),damage:30.0.into(),
            enchantment:3.0.into(),offense_multiplier:1.5.into(),hands:1.0.into(),thrown:true.into(),..Default::default()};
        assert!(describe_item(&r,&hatchet).contains("Thrown damage with current stats: 20–34 before defenses"));
        r.simulation_revision=50.0.into();
        assert!(weapon_damage_preview(&r,&bow,true,20.).contains("11–18 before defenses"));
        assert!(describe_item(&r,&hatchet).contains("Thrown damage with current stats: 24–40 before defenses"));
    }
    #[test]
    fn revision51_weapon_range_uses_nearest_not_ceiling() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.inventory.clear();r.player.active.clear();
        let bow=Item{kind:"weapon".into(),damage:15.0.into(),..Default::default()};
        assert!(weapon_damage_preview(&r,&bow,true,0.).contains("4–7 before defenses"));
        r.simulation_revision=50.0.into();assert!(weapon_damage_preview(&r,&bow,true,0.).contains("6–10 before defenses"));
    }
}

#[cfg(test)]
mod quiet_pack_copy_tests {
    use super::*;
    #[test]
    fn quiet_pack_has_no_static_jewelry_or_cancel_tutorial_line() {
        let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.inventory.clear();
        let mut v=PackView::observe(&r);let mut p=Painter::new();
        for request in [false,true] {
            if request {v.request=Some(Request::Tinker{ability:"kative-tinker".into()});}
            v.paint(&mut p);
            let color=p.profile.rgb("ui.colors.panel");
            for y in v.layout.bounds.1+93..v.layout.bounds.1+104 {
                for x in v.layout.list.0..v.layout.cancel.0-8 {
                    let at=(y as usize*p.width+x as usize)*4;
                    assert_eq!(&p.pixels[at..at+3],&color,"tutorial band should be empty");
                }
            }
            if request {
                assert_eq!(v.hint(),"Tinker: choose an untinkered ring or amulet.");
                v.hover_at=Some((v.layout.list.0+4,v.layout.bounds.1+75));
                assert_eq!(v.tooltip().as_deref(),Some("Tinker: choose an untinkered ring or amulet."));
            }
        }
        let item=corpse_core::revision50_amulet_effects::make_amulet(corpse_core::revision50_amulet_effects::Effect::Lightning,901.).unwrap();
        r.inventory.push(item.clone());
        let buttons=item_actions(&r,&item);
        assert_eq!(buttons.iter().find(|b|b.label=="Apply").unwrap().tip,"Choose a target for this amulet.");
    }
}

#[cfg(test)]
mod released_floods_pack_tests {
    use super::*;
    #[test]
    fn legacy_floods_apply_keeps_targeting_and_current_retired_items_have_no_apply() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.player.energy=100.;r.player.abilities.clear();r.player.active.clear();r.inventory.clear();
        let mut item=Item{id:901.,kind:"amulet".into(),effect:"floods".to_string().into(),..Default::default()};
        corpse_core::revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();r.inventory.push(item.clone());
        for revision in [50.,51.,52.] {
            r.simulation_revision=revision.into();r.amulet_rules_from=if revision==52. {Field::Null}else{Field::Missing};let buttons=item_actions(&r,&item);let apply=buttons.iter().find(|b|b.label=="Apply").unwrap();assert!(apply.blocker.is_none());
            assert!(matches!(apply.action,ButtonAction::Target(PackTarget::UseAt(901.))));
            assert_eq!(apply.tip,"Choose a target for this amulet.");
            for (width,height) in [(1280,800),(960,1200)] {
                let mut painter=Painter::new();painter.resize(width,height);painter.set_responsive_canvas(true);
                let mut view=PackView::observe(&r);view.configure(&painter,painter.frame().map.tuple());view.selected=Some(901.);
                let expected=PackAction::Target(PackTarget::UseAt(901.));
                for _ in 0..view.keyboard_tabs_for_action_label("Apply").unwrap(){view.key(Keycode::Tab,Mod::NOMOD);}
                assert_eq!(view.key(Keycode::Return,Mod::NOMOD),expected);
                // A completed/canceled action refreshes the real pending-action guard.
                view.refresh(&r);view.configure(&painter,painter.frame().map.tuple());view.focus_action=None;
                let index=view.selected_group().unwrap().actions.iter().position(|a|a.label=="Apply").unwrap();
                let rect=view.button_rects().into_iter().find(|(i,_)|*i==index).unwrap().1;
                if rect.1+rect.3>view.layout.detail.1+view.layout.detail.3 {view.detail_scroll+=rect.1+rect.3-view.layout.detail.1-view.layout.detail.3;}
                let rect=view.button_rects().into_iter().find(|(i,_)|*i==index).unwrap().1;
                assert_eq!(view.click(rect.0+3,rect.1+3),expected);
            }
        }
        r.simulation_revision=52.0.into();r.amulet_rules_from=Field::Missing;
        for effect in ["acid","floods"] {
            item.effect=effect.to_string().into();r.inventory=vec![item.clone()];
            assert!(!item_actions(&r,&item).iter().any(|b|b.label=="Apply"));
            let mut lines=Vec::new();assert!(append_named_jewelry(&r,&item,&mut lines));
            assert_eq!(lines,vec![corpse_core::revision52_amulet_tiers::RETIRED_MESSAGE]);
            assert!(corpse_core::revision50_item_knowledge::display_name(&r,&item).starts_with("Retired"));
        }
    }
}

#[cfg(test)]
mod revision52_composite_accuracy_tests {
    use super::*;
    #[test]
    fn current_bow_description_is_only_requested_text() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.inventory.clear();
        for count in [0.,1.,17.] {
            r.inventory.clear();
            if count>0. {r.inventory.push(Item{id:951.,kind:"ammo".into(),effect:"arrows".to_string().into(),quantity:count.into(),..Default::default()});}
            for name in ["Bow","Composite bow"] {
                let mut item=corpse_core::equipment::make_equipment(name,950.).unwrap();
                item.tinkered=true.into(); item.granted_ability="shrail-execute".to_string().into();
                r.simulation_revision=52.0.into();
                assert_eq!(describe_item(&r,&item),format!("Apply to fire an arrow at a visible target. {count} arrows available."));
                r.simulation_revision=51.0.into();
                assert!(describe_item(&r,&item).contains("Half the miss chance of a thrown weapon"));
            }
        }
    }
}

#[cfg(test)]mod tinkered_name_ui_tests {
 use super::*;
 #[test]fn pack_name_and_inspect_title_use_same_grant_suffix(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  for (kind,effect,name)in[("ring","light","Ring of Light"),("amulet","lightning","Amulet of Lightning")] {
   let i=Item{id:900.,kind:kind.into(),effect:effect.to_string().into(),name:name.into(),tinkered:true.into(),granted_ability:"shrail-execute".to_string().into(),..Default::default()};
   let shown=displayed_name(&r,&i);assert!(shown.ends_with(" and Execute"));assert!(describe_item(&r,&i).starts_with(&shown));assert_eq!(shown.matches(" and Execute").count(),1);
  }
 }
}

#[cfg(test)]mod insufficient_energy_pack_tests {
 use super::*;
 #[test]fn amulet_apply_blocker_is_exact_short_copy(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.energy=0.;r.player.abilities.clear();r.player.active.clear();
  let item=Item{id:991.,kind:"amulet".into(),name:"Amulet of Lightning".into(),effect:"lightning".to_string().into(),amulet_state:corpse_core::revision50_amulets::AmuletState{instance_id:991,energy:0.,ready_at_half_tick:0}.into(),..Default::default()};r.inventory=vec![item.clone()];
  let buttons=item_actions(&r,&item);let apply=buttons.iter().find(|b|b.label=="Apply").unwrap();assert_eq!(apply.blocker.as_deref(),Some("Insufficient Energy"));
 }
}

#[cfg(test)] mod arrow_copy_cuts_tests {
    use super::*;
    #[test] fn arrow_description_keeps_only_name_and_quantity() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for revision in [49.,50.,51.,52.] {
            r.simulation_revision=revision.into();
            for count in [1.,17.,500.] {
                let item=equipment::make_arrows(900.,count);
                assert_eq!(describe_item(&r,&item),format!("{} ×{}",item.name,count));
            }
        }
    }
}

#[cfg(test)]
mod widest_amulet_tests {
    use super::*;
    #[test]
    fn widest_tinkered_amulet_fits_default_pack_and_narrow_views_wrap() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision = 52.0.into();
        let mut p = Painter::new();
        let longest = longest_tinkered_amulet_name(&p, 52.);
        eprintln!("Widest amulet: {} ({} px)", longest, p.measure_text_role(Role::Inventory, &longest, 14));
        assert!(longest.starts_with("Tier 4 ") && longest.contains(" Amulet and "));
        let mut view = PackView::observe(&run);
        view.configure(&p, p.frame().map.tuple());
        assert!(view.layout.bounds.2 > 760);
        let available = view.layout.list.2 - 58 - 100 - 8 - 4;
        assert!(view.text_metrics.as_ref().unwrap().width(&longest) <= available as f64);
        assert!(view.layout.bounds.0 >= 0 && view.layout.bounds.0 + view.layout.bounds.2 <= p.logical_width());
        // Carry the maximum name to exercise wrapping at a deliberately large font.
        let mut group = display_groups(&run).into_iter().next().unwrap();
        group.name = longest;
        view.groups = vec![group];
        view.content_metrics_dirty = true;
        p.profile = Profile::from_value(serde_json::json!({"typography":{"inventory":{"fontSize":32}}})).unwrap();
        view.configure(&p, p.frame().map.tuple());
        let metrics = view.text_metrics.as_ref().unwrap();
        let lines = metrics.wrap(&view.groups[0].name, (view.layout.list.2 - 170).max(1));
        assert!(lines.len() > 1);
        assert!(view.layout.row >= lines.len() as i32 * metrics.line + 12);
        let rebuilds = view.configure_rebuilds;
        view.configure(&p, p.frame().map.tuple());
        assert_eq!(view.configure_rebuilds, rebuilds);
    }
}

#[cfg(test)]
mod retired_potion_ui_tests {
    use super::*;
    #[test]
    fn retired_potions_keep_identity_and_drop_but_not_effect_actions_or_copy() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.potion_rules_from=Field::Missing;
        for effect in ["cleansing","flooding","repulsion"] {
            let item=Item{id:903.,kind:"potion".into(),name:format!("Potion of {effect}"),effect:effect.to_string().into(),diluted:true.into(),quantity:2.0.into(),..Default::default()};
            let before=item.clone();
            let actions=item_actions(&r,&item);
            assert!(!actions.iter().any(|a|matches!(a.label.as_str(),"Drink"|"Apply"|"Throw")));
            assert!(actions.iter().any(|a|a.label=="Drop"));
            assert!(displayed_name(&r,&item).starts_with("Retired Potion"));
            assert!(describe_potion(&r,&item).ends_with(corpse_core::revision50_potions::RETIRED_MESSAGE));
            assert!(!describe_potion(&r,&item).contains("Drink to"));
            assert_eq!(item,before);
            r.potion_rules_from=Field::Null;
            let actions=item_actions(&r,&item);
            assert!(actions.iter().any(|a|a.label=="Drink"));
            assert!(actions.iter().any(|a|a.label=="Throw"));
            assert!(!describe_potion(&r,&item).contains("retired"));
            assert!(!displayed_name(&r,&item).starts_with("Retired"));
            r.potion_rules_from=Field::Missing;
        }
    }
}

#[cfg(test)]
mod current_ring_ui_tests {
    use super::*;
    #[test]
    fn retired_rings_show_no_active_effect_copy_and_keep_drop() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.ring_rules_from=0usize.into();
        for effect in ["fire-resistance","poison-resistance","sustain-strength","ice-immunity","corrosion-resistance"] {
            let item=Item{id:903.,kind:"ring".into(),effect:effect.to_string().into(),..Default::default()};
            let before=item.clone();let mut text=vec![];
            assert!(append_named_jewelry(&r,&item,&mut text));assert_eq!(text,vec!["Retired ring."]);
            let name=displayed_name(&r,&item);assert!(name.starts_with("Retired ") && name.ends_with(" Ring"));
            assert!(item_actions(&r,&item).iter().any(|a|a.label=="Drop"));assert_eq!(item,before);
            r.ring_rules_from=Field::Null;text.clear();assert!(append_named_jewelry(&r,&item,&mut text));assert_ne!(text,vec!["Retired ring."]);
            r.ring_rules_from=0usize.into();
        }
    }
}

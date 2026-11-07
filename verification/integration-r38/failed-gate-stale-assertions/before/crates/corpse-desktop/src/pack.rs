//! Fixed-geometry Pack projection. UI grouping is never simulation stacking.
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

pub const BOUNDS: (i32, i32, i32, i32) = (73, 131, 900, 579);
const LIST: Rect = Rect(85, 249, 480, 449);
const DETAIL: Rect = Rect(577, 249, 384, 449);
const CLOSE: Rect = Rect(929, 135, 36, 32);
const CANCEL: Rect = Rect(881, 202, 80, 32);
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
        if !run.actors_at(run.depth).iter().any(|a| a.x==x && a.y==y
            && !a.dead.value_or(false) && a.hp>0. && corpse_core::vision::visible_actor(run,a)
            && corpse_core::dreamwalking::same_plane(run,&run.player,a)) {
            return Some("Choose a visible adjacent living creature.");
        }
        let Some(item)=run.inventory.iter().find(|i|i.id==id) else { return Some("That item is no longer in your pack."); };
        corpse_core::borge::use_blocker(run,item,x,y)
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
}
#[derive(Clone, PartialEq)]
struct Stamp {
    run_id: Field<String>,
    seed: String,
    player: f64,
    tick: f64,
    depth: f64,
    phase: Field<String>,
    inventory: Vec<Item>,
    abilities: Vec<String>,
    energy: f64,
    status: String,
    offers: Vec<String>,
    blocked: bool,
    dreamwalking: bool,
    teleport_cooldown: u32,
    tinker_blocker: Option<String>,
}
impl Stamp {
    fn observe(run: &Run) -> Self {
        Self {
            run_id: run.run_id.clone(),
            seed: run.seed.clone(),
            player: run.player.id,
            tick: run.tick,
            depth: run.depth,
            phase: run.phase.clone(),
            inventory: run.inventory.clone(),
            abilities: run.player.abilities.clone(),
            energy: run.player.energy,
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
/// I <= 100 physical item records, A is the fixed ability catalog, G <= I display
/// groups, and T is projected tooltip text. Typed equality preserves every
/// optional identity field; no JSON or gameplay RNG is used for grouping.
pub struct PackView {
    layout: PackLayout,
    metric_signature: Option<(
        crate::typography_ui::Overrides,
        crate::typography_ui::Overrides,
    )>,
    text_metrics: Option<TextMetrics>,
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
}
impl PackView {
    pub fn observe(run: &Run) -> Self {
        Self {
            layout: PackLayout::resolve(
                Profile::defaults(),
                modal_ui::Frame::from_profile(Profile::defaults()).map,
            ),
            metric_signature: None,
            text_metrics: None,
            button_metrics: None,
            groups: display_groups(run),
            stamp: Stamp::observe(run),
            slots: inventory::occupancy(run) + 0.,
            capacity: crafting::capacity(&run.inventory),
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
    /// Configure before input and paint. Cached text metrics contain only
    /// detached observed labels; changing layout never rebuilds simulation.
    pub fn configure(&mut self, p: &Painter, map: (i32, i32, i32, i32)) {
        let signature = (
            crate::typography_ui::overrides(&p.profile, Role::Inventory),
            crate::typography_ui::overrides(&p.profile, Role::Primary),
        );
        let changed = self.metric_signature.as_ref() != Some(&signature);
        self.layout = PackLayout::resolve(&p.profile, Rect(map.0, map.1, map.2, map.3));
        if changed || self.text_metrics.is_none() {
            self.text_metrics = Some(TextMetrics::new(p, Role::Inventory, 14, 18));
            self.button_metrics = Some(TextMetrics::new(p, Role::Primary, 14, 18));
        }
        self.metric_signature = Some(signature);
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
    pub fn refresh(&mut self, run: &Run) {
        let stamp = Stamp::observe(run);
        if stamp != self.stamp {
            self.request = None;
        }
        self.stamp = stamp;
        self.groups = display_groups(run);
        self.slots = inventory::occupancy(run) + 0.;
        self.capacity = crafting::capacity(&run.inventory);
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
    pub fn display_order(&self) -> Vec<f64> {
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
    pub fn hover(&mut self, x: i32, y: i32) {
        self.hover_at = Some((x, y));
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
                        "This item has already been tinkered. Each item can be tinkered once."
                            .into(),
                    )
                } else if item.granted_ability.as_ref().is_some_and(|s| !s.is_empty()) {
                    Some("This item already grants an ability; Tinker cannot replace it.".into())
                } else if !tinker_eligible(item) {
                    Some("Choose an unblessed weapon, armor, ring or amulet.".into())
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
    fn move_group(&mut self, index: usize, delta: isize) -> PackAction {
        if self.pending || self.read_only || self.request.is_some() {
            return PackAction::None;
        }
        let Some(other) = index
            .checked_add_signed(delta)
            .filter(|i| *i < self.groups.len())
        else {
            return PackAction::None;
        };
        if crafting::is_equipped(&self.groups[index].item)
            != crafting::is_equipped(&self.groups[other].item)
        {
            return PackAction::None;
        }
        self.groups.swap(index, other);
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
    fn row_positions(&self) -> Vec<(usize, i32)> {
        let mut y = self.layout.list.1 + 24;
        let mut carried = false;
        let mut rows = Vec::with_capacity(self.groups.len());
        for (i, g) in self.groups.iter().enumerate() {
            if !carried && !crafting::is_equipped(&g.item) {
                y += 24;
                carried = true;
            }
            rows.push((i, y));
            y += self.layout.row;
        }
        rows
    }
    fn list_max_scroll(&self) -> i32 {
        (48 + self.layout.row * self.groups.len().max(20) as i32 - self.layout.list.3).max(0)
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
            return vec![("Select an item to inspect it.".into(), MUTED)];
        };
        let mut lines = vec![];
        lines.extend(self.wrap_detail(&g.name).into_iter().map(|s| (s, INK)));
        lines.push((String::new(), MUTED));
        for s in g.description.split('\n') {
            lines.extend(self.wrap_detail(s).into_iter().map(|s| (s, MUTED)));
        }
        if g.count > 1. {
            let text = if inventory::is_arrow(&g.item) {
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
                self.wrap_detail(blocker.as_deref().unwrap_or("Click to select"))
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
    fn hint(&self) -> String {
        match &self.request {
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
            return Some(format!(
                "{} Esc to cancel. Nothing is spent until you choose a valid item.",
                self.hint()
            ));
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
                        return Some(
                            self.target_blocker(self.target_member(g))
                                .unwrap_or_else(|| "Select this item".into()),
                        );
                    }
                    if !self.read_only && x >= self.layout.list.0 + self.layout.list.2 - 52 {
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
                    return Some(g.name.clone());
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
    pub fn paint(&self, p: &mut Painter) {
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
            "Pack",
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
            &format!("{} / {} slots", self.slots, self.capacity),
            self.layout.list.0,
            self.layout.bounds.1 + 52,
            14,
            p.profile.rgb("ui.colors.secondary"),
        );
        if self.request.is_some() {
            p.text(
                &ellipsize(&self.hint(), 94),
                self.layout.list.0,
                self.layout.bounds.1 + 74,
                13,
                p.profile.rgb("ui.colors.headings"),
            );
            p.text(
                "Esc to cancel. Nothing is spent until you choose a valid item.",
                self.layout.list.0,
                self.layout.bounds.1 + 91,
                13,
                p.profile.rgb("ui.colors.headings"),
            );
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
        p.clip = Some(self.layout.list.tuple());
        p.text(
            "Equipped",
            self.layout.list.0,
            self.layout.list.1 - self.list_scroll + 3,
            13,
            p.profile.rgb("ui.colors.secondary"),
        );
        let rows = self.row_positions();
        let carried_y = rows
            .iter()
            .find(|(i, _)| !crafting::is_equipped(&self.groups[*i].item))
            .map_or(
                self.layout.list.1 + 24 + self.groups.len() as i32 * self.layout.row,
                |(_, y)| y - 24,
            );
        p.rect(
            self.layout.list.0,
            carried_y - self.list_scroll,
            self.layout.list.2,
            1,
            p.profile.rgb("ui.colors.border"),
        );
        p.text(
            "In your pack",
            self.layout.list.0,
            carried_y - self.list_scroll + 4,
            13,
            p.profile.rgb("ui.colors.secondary"),
        );
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
            let label = format!(
                "{}{}{}",
                g.name,
                if g.count > 1. {
                    format!(" ×{}", g.count)
                } else {
                    String::new()
                },
                if crafting::is_equipped(&g.item) {
                    format!(" [{}]", g.item.equipped.as_ref().unwrap())
                } else {
                    String::new()
                }
            );
            p.clip = Some((
                self.layout.list.0,
                y.max(self.layout.list.1),
                self.layout.list.2,
                (y + self.layout.row).min(self.layout.list.1 + self.layout.list.3)
                    - y.max(self.layout.list.1),
            ));
            let width = if self.read_only || self.request.is_some() {
                57
            } else {
                51
            };
            p.text_role(
                Role::Inventory,
                &self.text_metrics.as_ref().map_or_else(
                    || ellipsize(&label, width),
                    |m| {
                        m.ellipsize(
                            &label,
                            self.layout.list.2
                                - if self.read_only || self.request.is_some() {
                                    6
                                } else {
                                    58
                                },
                        )
                    },
                ),
                self.layout.list.0 + 2,
                y + 6,
                14,
                if valid {
                    p.profile.rgb("ui.colors.headings")
                } else {
                    p.profile.rgb("ui.colors.primary")
                },
            );
            if !self.read_only && self.request.is_none() {
                let equipped = crafting::is_equipped(&g.item);
                let up =
                    *index > 0 && crafting::is_equipped(&self.groups[*index - 1].item) == equipped;
                let down = *index + 1 < self.groups.len()
                    && crafting::is_equipped(&self.groups[*index + 1].item) == equipped;
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
        // The same twenty authored presentation positions exist for an empty bag.
        let bottom = rows
            .last()
            .map_or(carried_y + 24, |(_, y)| y + self.layout.row)
            .max(carried_y + 24);
        for n in self.groups.len()..20 {
            let y = bottom + (n - self.groups.len()) as i32 * self.layout.row - self.list_scroll;
            p.rect(
                self.layout.list.0 + 2,
                y + self.layout.row - 1,
                self.layout.list.2 - 4,
                1,
                [21, 25, 35],
            );
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
        if let (Some(text), Some((x, y))) = (self.tooltip(), self.hover_at) {
            if !text.is_empty() {
                let width = p.profile.number("ui.layout.tooltipMaxWidth").round() as i32;
                let width = width.min((self.layout.bounds.2 - 8).max(1));
                let lines = p.wrap_text_role(Role::Tooltips, &text, (width - 16).max(1), 13);
                let metrics = TextMetrics::new(p, Role::Tooltips, 13, 18);
                let height =
                    (lines.len() as i32 * metrics.line + 12).min((self.layout.bounds.3 - 8).max(1));
                let tx = x.clamp(
                    self.layout.bounds.0 + 4,
                    (self.layout.bounds.0 + self.layout.bounds.2 - width - 4)
                        .max(self.layout.bounds.0 + 4),
                );
                let ty = (y + 24)
                    .min(self.layout.bounds.1 + self.layout.bounds.3 - height - 4)
                    .max(self.layout.bounds.1 + 4);
                p.clip = Some((tx, ty, width, height));
                p.rect(tx, ty, width, height, p.profile.rgb("ui.colors.tooltip"));
                modal_ui::border(
                    p,
                    Rect(tx, ty, width, height),
                    p.profile.rgb("ui.colors.border"),
                );
                for (i, line) in lines.iter().enumerate() {
                    p.text_role(
                        Role::Tooltips,
                        line,
                        tx + 8,
                        ty + 6 + i as i32 * metrics.line,
                        13,
                        p.profile.rgb("ui.colors.primary"),
                    );
                }
            }
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
    } else if r.player.energy < 100. {
        Some("Needs 100 energy".into())
    } else if abilities::energy_activation_silenced(r, &r.player) {
        Some("Energy-paid activation is silenced".into())
    } else if !r.inventory.iter().any(tinker_eligible) {
        Some("No unblessed weapon, armor, ring or amulet to tinker with".into())
    } else if corpse_core::tinker::god_pools(r).is_empty() {
        Some("No eligible outside-god abilities remain".into())
    } else {
        None
    }
}
fn tinker_eligible(i: &Item) -> bool {
    corpse_core::tinker::target_eligible(i)
}
fn ability_name(id: &str) -> &str {
    progression::catalog()
        .iter()
        .find(|a| a.id == id)
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
    key.id = 0.;
    key.quantity = Field::Missing;
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
    let mut groups: Vec<Group> = vec![];
    let mut identities: Vec<Item> = vec![];
    for item in &run.inventory {
        let key = identity(run, item);
        let prior = if crafting::is_equipped(item) {
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
                description: describe_item(run, item),
                actions: item_actions(run, item),
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
            let key = (!crafting::is_equipped(&g.item), rank(&g));
            (key, g)
        })
        .collect();
    ranked.sort_by_key(|(key, _)| *key);
    ranked.into_iter().map(|(_, g)| g).collect()
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
            "Granted while carried. Teleport to a random standable spot; 100-turn cooldown.".into());
    }
    if corpse_core::borge::is_shard(item) {
        push("Use on…", ButtonAction::Target(PackTarget::UseAt(item.id)), None,
            "Choose an adjacent living creature. The shard is consumed only on success.".into());
    }
    if matches!(item.kind.as_str(), "heart" | "potion" | "tool" | "food") && !corpse_core::ahvamitt::is_mask(item) {
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
    } else if equipment::equippable(item) {
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
    if equipment::equippable(item)
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
    if crafting::is_equipped(item) {
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
    if !corpse_core::borge::is_shard(item) {
    push(
        "Throw",
        ButtonAction::Target(PackTarget::Throw(item.id)),
        None,
        String::new(),
    );
    }
    if item.kind == "heart" {
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
                tip: format!(
                    "Fire one arrow ({} available). This bow occupies both hands.",
                    arrow_count(run)
                ),
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
    i.identified.value_or(false)
        || run
            .known_potions
            .iter()
            .any(|s| Some(s) == i.effect.as_ref())
}
pub(crate) fn displayed_name(run: &Run, i: &Item) -> String {
    let name = if i.kind == "potion" {
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
    let name = if marked_heart(i) { format!("{name} · Borge-marked") } else { name };
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
pub(crate) fn describe_item(run: &Run, i: &Item) -> String {
    if corpse_core::ahvamitt::is_mask(i) {
        return format!("{}\nGrants Teleport while in your inventory.", i.name);
    }
    if corpse_core::phantasmii_generation::is_dagger(i) {
        return format!("{}\n{}\nApply while standing on the Altar of the Phantasmii to enter Dreamwalking.",
            i.name, corpse_core::phantasmii_generation::DAGGER_DESCRIPTION);
    }
    if corpse_core::phantasmii_generation::is_shard(i) {
        return format!("{} ×{}\nA fragment found in the dream plane. Collect three to complete the ritual and learn Dreamwalk.",
            i.name, i.quantity.value_or(1.));
    }
    if corpse_core::borge::is_shard(i) {
        return format!("{}\n{}\n{}\nUse on an adjacent living creature, or equip in either hand to use on your next melee target. No accuracy roll; prevents revival. Consumed only after a successful use.",
            i.name, if crafting::is_equipped(i) { equipped_place(i) } else { "Unequipped" }, corpse_core::borge::SHARD_DESCRIPTION);
    }

    if corpse_core::royal_honey::is_honey(i) {return "Eat once to fully restore current health and energy to their effective maxima. No other effect.".into();}
    if i.kind == "potion" {
        return describe_potion(run, i);
    }
    let stats = bonuses(i.bonus.as_ref(), i.base_bonus.as_ref());
    if i.kind == "heart" {
        let mut text = format!(
            "{}{} <heart>\n{}\n{} blessings · {} energy",
            i.name,
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
            i.exp.value_or(0.),
            i.energy.value_or(0.)
        );
        if marked_heart(i) {
            text.push_str(&format!("\nBorge-marked sacrifice: offer this exact heart at the Borge altar on depth {}.", i.borge_depth.value_or(0.)));
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
                text.push_str(&format!(
                    "\nAdaptive Armor: +{} DR against {kind}",
                    (3. - prior.floor().max(0.)).max(0.)
                ));
            }
        }
        return text;
    }
    if inventory::is_arrow(i) {
        return format!(
            "{} ×{}\nBow ammunition; unlimited arrows share one bag slot.\nA shot consumes one arrow on a hit. Misses and empty shots leave it recoverable. Drop and Throw use one arrow.",
            i.name,
            i.quantity.value_or(1.)
        );
    }
    let mut lines = vec![
        displayed_name(run, i),
        format!(
            "{}{}",
            titlecase(&i.kind),
            if crafting::is_equipped(i) {
                format!(" · {}", equipped_place(i))
            } else if equipment::equippable(i) {
                " · Unequipped".into()
            } else {
                String::new()
            }
        ),
    ];
    if i.kind == "weapon" && !projectiles::is_bow(i) {
        if let Some(slot) = i
            .equipped
            .as_ref()
            .filter(|s| matches!(s.as_str(), "hand1" | "hand2"))
        {
            let first = run.inventory.iter().find(|j| {
                j.kind == "weapon" && !projectiles::is_bow(j) && j.equipped.as_ref() == Some(slot)
            });
            if first.is_some_and(|j| j.id == i.id) {
                let base = corpse_core::engine::player_weapon_strike_base(run, i);
                let repeat = run.player.active.iter().any(|a| a == abilities::ASSAULT)
                    && run.player.energy >= 10.;
                lines.push(format!(
                    "{}{}\n{}–{} damage per hit before enemy defenses and strike bonuses.{}",
                    if slot == "hand2" {
                        "Offhand: extra strike · ½ normal hit chance"
                    } else {
                        "Main strike · normal hit chance"
                    },
                    if repeat {
                        " · twice with Unyielding Assault"
                    } else {
                        ""
                    },
                    combat::tuned_damage(base * 0.75, Some("attack")),
                    combat::tuned_damage(base * 1.25, Some("attack")),
                    if slot == "hand2" {
                        " Only if both survive the main strike."
                    } else {
                        ""
                    }
                ));
            }
        }
        lines.push(format!(
            "{}-handed · {} base damage · ×{} Offense{}",
            i.hands.value_or(1.),
            i.damage.value_or(0.),
            i.offense_multiplier.value_or(1.),
            if i.enchantment.value_or(0.) != 0. {
                format!(
                    " · {}{} enchantment",
                    if i.enchantment.value_or(0.) > 0. {
                        "+"
                    } else {
                        ""
                    },
                    i.enchantment.value_or(0.)
                )
            } else {
                String::new()
            }
        ));
        if i.equipped.as_ref().is_none_or(|s| s.is_empty()) {
            lines.push("Final damage varies with Offense, defenses, and abilities, then scales ×0.5 and rounds to a whole number.".into());
        }
        lines.push(
            if i.thrown.value_or(false) {
                "Thrown weapon: full attack damage when thrown."
            } else {
                "Thrown: half weapon damage for one-handed weapons; otherwise Offense damage."
            }
            .into(),
        );
    }
    if projectiles::is_bow(i) {
        lines.extend([format!("2-handed · Equip in both hands; Apply to fire at a visible target. Uses one arrow; {} available.", arrow_count(run)), "Uses your existing unarmed attack in melee.".into(), format!("Shots: {} base damage + ×{} Offense, before variance, defenses, and whole-number rounding.",i.damage.value_or(0.),i.offense_multiplier.value_or(1.)), "Half the miss chance of a thrown weapon; guaranteed hits and misses still apply. Walls, trees, closed doors, and the first creature stop arrows; grates let them pass.".into(), "Arrows vanish on a hit. Misses leave recoverable arrows, which may scatter or fall.".into()]);
    }
    if !stats.is_empty() {
        lines.push(format!(
            "{}: {stats}",
            if equipment::equippable(i) {
                "When equipped"
            } else {
                "Inactive equipment bonuses"
            }
        ));
    }
    if let Some(id) = i.granted_ability.as_ref().filter(|s| !s.is_empty()) {
        lines.push(format!(
            "{} {}.",
            if equipment::equippable(i) {
                "While equipped: grants"
            } else {
                "Inactive equipment grant:"
            },
            ability_name(id)
        ));
        if let Some(a) = progression::catalog().iter().find(|a| &a.id == id) {
            lines.push(a.description.clone());
        }
    }
    if i.tinkered.value_or(false) {
        lines.push("Already tinkered; cannot be tinkered again.".into());
    }
    for e in &i.enhancements {
        let name = crafting::craft_name(&e.craft).unwrap_or(&e.craft);
        let text = match name {
            "Fury" => {
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
    if let Some(effect) = i.effect.as_ref().filter(|s| !s.is_empty()) {
        lines.push(if effect == "tree-tap" {
            format!("Apply beside {} to collect its one gift of sap. The tap breaks when used.",
                if run.simulation_revision.value_or(29.)>=33. {"Verdania"} else {"Veridian"})
        } else {
            effect.clone()
        });
    }
    lines
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
fn titlecase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
fn describe_potion(run: &Run, i: &Item) -> String {
    let effect = i.effect.as_ref().map_or("", String::as_str);
    let diluted = i.diluted.value_or(false);
    let strength = if diluted { 0.5 } else { 1. };
    let identity = inventory::potion_description(run, i);
    let identity = if run.simulation_revision.value_or(29.)>=33. && effect=="Veridian Sap" && potion_known(run,i) {
        identity.replace("Veridian Sap","Verdania Sap")
    } else {identity};
    if !potion_known(run, i) {
        return identity;
    }
    let detail=match effect {
        "detect magic"=>format!("Senses potions and magical equipment for {} turns. Remembers their good, bad, or bizarre magic and last positions afterward. Mixed magic is bizarre. Ordinary gear, hearts, and tools are not detected.",(20f64*strength).ceil()),
        "healing"=>format!("Restores {} health, up to maximum health.",50.*strength),
        "energy"=>format!("Restores {} energy, up to your energy limit.",50.*strength),
        "incineration"=>format!("Ignites the impact tile; a creature there burns for a random {}–{} turns, taking 4 HP per normal turn. Fire exposure renews the 6–10-turn burn.",(6f64*strength).ceil(),(10f64*strength).ceil()),
        "caustic gas"=>"Releases gas that deals 1/15 of maximum HP per normal turn. Gas vulnerability applies before rounding up; damage prevention applies afterward. Dilution reduces the gas volume, not its damage.".into(),
        "paralysis"=>format!("Releases paralysis gas; a directly affected creature is paralyzed for {} turns.",(5f64*strength).ceil()),
        "telepathy"=>format!("Senses living creatures through walls across this floor for {} turns. Reveals terrain and occupants within 2 squares of each sensed creature.",(30f64*strength).ceil()),
        "levitation"=>format!("Grants flight for {} turns, avoiding chasms, currents, and deep-water item loss. Fire and gas still affect you. If flight ends over a chasm, you fall.",(20f64*strength).ceil()),
        "speed"=>format!("Grants one extra movement step per normal turn for {} turns, without an extra attack.",(10f64*strength).ceil()),
        "confusion"=>"Releases gas that confuses creatures for 8 turns, refreshed by exposure. Movement goes in a random direction; bumping an attackable enemy attacks it. Dilution reduces gas volume.".into(),
        "invisibility"=>format!("Hides you from ordinary sight for {} turns, even while attacking. Telepathy still reveals you.",(20f64*strength).ceil()),
        "descent"=>format!("Collapses terrain within a circular radius of {} squares. Grounded creatures fall one floor, taking fall damage unless they land in deep water. Spares stairs and fixed fixtures. Has no effect on floor 40.",if diluted{1}else{2}),
        "offense"|"defense"|"accuracy"|"dodge"|"dr"|"perception"=>format!("Grants +{} {effect} for {} turns. Reusing keeps the stronger bonus and longer duration, including diluted potions; they do not stack.",combat::integer_stat(if effect=="dr" {2.*strength}else{5.*strength}),(20f64*strength).ceil()),
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
    fn run() -> Run {
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()
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
        assert!(describe_item(&r,&mask).contains("while carried"));
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
    fn borge_heart_provenance_is_visible_and_refreshes_without_changing_reward() {
        let mut r=run();let mut h=heart(201.);r.inventory=vec![h.clone()];let mut view=PackView::observe(&r);
        assert!(!displayed_name(&r,&h).contains("Borge"));
        h.source_actor_id=44.0.into();h.borge_depth=12.0.into();r.inventory[0]=h.clone();view.refresh(&r);
        assert!(displayed_name(&r,&h).contains("Borge-marked"));assert!(describe_item(&r,&h).contains("exact heart"));assert!(describe_item(&r,&h).contains("depth 12"));
        assert_eq!(h.bonus,heart(201.).bonus);assert_eq!(h.exp,heart(201.).exp);assert_eq!(h.energy,heart(201.).energy);
        assert!(view.groups[0].name.contains("Borge-marked"));
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
                .map(|g| serde_json::json!({"ids":g["ids"],"count":g["count"],"name":g["name"]}))
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
                    normalize_stat_lists(expected["description"].as_str().unwrap()),
                    "{}: {}",
                    case["name"],
                    g.name
                );
            }
        }
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
            for (x, y) in [
                (BOUNDS.0 - 1, BOUNDS.1),
                (BOUNDS.0, BOUNDS.1 - 1),
                (BOUNDS.0 + BOUNDS.2, BOUNDS.1),
                (BOUNDS.0, BOUNDS.1 + BOUNDS.3),
                (1100, 350),
            ] {
                let at = (y as usize * crate::render::WIDTH + x as usize) * 4;
                assert_eq!(&painter.pixels[at..at + 3], &[93, 31, 113]);
            }
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
            let frame = modal_ui::Frame::from_profile(&p.profile);
            view.configure(&p, frame.map.tuple());
            let layout = view.layout;
            assert_eq!(view.bounds(), layout.bounds.tuple());
            assert_eq!(
                layout.row,
                p.profile.number("ui.layout.inventoryRowHeight") as i32
            );
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

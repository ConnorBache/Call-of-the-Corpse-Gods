//! Pure v48 UI-role overrides. Authored edits override per-consumer baselines;
//! schema defaults are not a substitute for the source CSS cascade.
use crate::appearance_profile::Profile;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Primary,
    Secondary,
    Headings,
    Chronicle,
    Ability,
    Tooltips,
    Inventory,
    Hotkeys,
}
impl Role {
    pub const ALL: [Self; 8] = [
        Self::Primary,
        Self::Secondary,
        Self::Headings,
        Self::Chronicle,
        Self::Ability,
        Self::Tooltips,
        Self::Inventory,
        Self::Hotkeys,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Headings => "headings",
            Self::Chronicle => "chronicle",
            Self::Ability => "ability",
            Self::Tooltips => "tooltips",
            Self::Inventory => "inventory",
            Self::Hotkeys => "hotkeys",
        }
    }
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.key() == key)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RoleStyle {
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: f64,
    pub letter_spacing: f64,
    pub line_height: f64,
}
impl RoleStyle {
    /// A convenient native monospace baseline; callers preserve their actual
    /// authored heading weight/line-height/family by setting those fields.
    pub fn baseline(size: f64) -> Self {
        Self {
            font_family: Profile::defaults()
                .text("typography.primary.fontFamily")
                .into(),
            font_size: size,
            font_weight: 400.,
            letter_spacing: 0.,
            line_height: 1.4,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<f64>,
    pub letter_spacing: Option<f64>,
    pub line_height: Option<f64>,
}
impl Overrides {
    pub fn apply(&self, mut base: RoleStyle) -> RoleStyle {
        if let Some(v) = &self.font_family {
            base.font_family = v.clone();
        }
        if let Some(v) = self.font_size {
            base.font_size = v;
        }
        if let Some(v) = self.font_weight {
            base.font_weight = v;
        }
        if let Some(v) = self.letter_spacing {
            base.letter_spacing = v;
        }
        if let Some(v) = self.line_height {
            base.line_height = v;
        }
        base
    }
    pub fn any(&self) -> bool {
        self.font_family.is_some()
            || self.font_size.is_some()
            || self.font_weight.is_some()
            || self.letter_spacing.is_some()
            || self.line_height.is_some()
    }
}
pub fn overrides(profile: &Profile, role: Role) -> Overrides {
    let path = |field: &str| format!("typography.{}.{}", role.key(), field);
    let number = |field: &str| {
        let p = path(field);
        let v = profile.number(&p);
        (v != Profile::defaults().number(&p)).then_some(v)
    };
    let family = path("fontFamily");
    Overrides {
        font_family: (profile.text(&family) != Profile::defaults().text(&family))
            .then(|| profile.text(&family).to_owned()),
        font_size: number("fontSize"),
        font_weight: number("fontWeight"),
        letter_spacing: number("letterSpacing"),
        line_height: number("lineHeight"),
    }
}
pub fn resolve(profile: &Profile, role: Role, baseline: RoleStyle) -> RoleStyle {
    overrides(profile, role).apply(baseline)
}
/// Source chronicle's four-and-a-half-line reserved track. Explicit authored
/// chronicleHeight wins over typography's derived height. Default values retain
/// the consumer's baseline (including compact-window CSS defaults).
pub fn chronicle_metrics(
    profile: &Profile,
    baseline_line: f64,
    baseline_height: f64,
) -> (f64, f64) {
    let d = Profile::defaults();
    let size = profile.number("typography.chronicle.fontSize");
    let line = profile.number("typography.chronicle.lineHeight");
    let edited = size != d.number("typography.chronicle.fontSize")
        || line != d.number("typography.chronicle.lineHeight");
    let height = profile.number("ui.layout.chronicleHeight");
    let height_edited = height != d.number("ui.layout.chronicleHeight");
    (
        if edited { size * line } else { baseline_line },
        if height_edited {
            height
        } else if edited {
            size * line * 4.5
        } else {
            baseline_height
        },
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    IbmPlexMono,
    Inconsolata,
    CorpseGlyphMono,
    CorpseGlyphSymbols,
    CorpseGlyphCjk,
    BrogueAscii,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontCandidates {
    pub faces: Vec<Face>,
    pub unavailable: Vec<String>,
}
/// Parse the validated CSS-family list without platform-font discovery. The
/// caller picks the first face that contains each glyph. Brogue is ASCII-only
/// artwork, available only to the map painter, never a UI text-font claim.
/// Unsupported names survive in Profile and are explicitly reported here.
pub fn font_candidates(family: &str, for_map: bool) -> FontCandidates {
    let mut result = FontCandidates {
        faces: vec![],
        unavailable: vec![],
    };
    for name in family
        .split(',')
        .map(|s| s.trim().trim_matches(['\'', '"']).trim())
        .filter(|s| !s.is_empty())
    {
        let face = match name.to_ascii_lowercase().as_str() {
            "ibm plex mono" | "monospace" => Some(Face::IbmPlexMono),
            "inconsolata" => Some(Face::Inconsolata),
            "corpse glyph mono" => Some(Face::CorpseGlyphMono),
            "corpse glyph symbols" => Some(Face::CorpseGlyphSymbols),
            "corpse glyph cjk" => Some(Face::CorpseGlyphCjk),
            "brogue ce ascii" if for_map => Some(Face::BrogueAscii),
            _ => None,
        };
        if let Some(face) = face {
            if !result.faces.contains(&face) {
                result.faces.push(face);
            }
        } else if !result.unavailable.iter().any(|s| s == name) {
            result.unavailable.push(name.into());
        }
    }
    // Native deterministic last-resort faces, not a claim to emulate every
    // system generic family. Always preserve the caller's known-family order.
    for face in [
        Face::IbmPlexMono,
        Face::Inconsolata,
        Face::CorpseGlyphMono,
        Face::CorpseGlyphSymbols,
        Face::CorpseGlyphCjk,
    ] {
        if !result.faces.contains(&face) {
            result.faces.push(face);
        }
    }
    result
}

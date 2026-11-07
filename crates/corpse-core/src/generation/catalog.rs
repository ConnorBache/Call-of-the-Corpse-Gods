//! Immutable frozen source data. Parsing metadata does not imply runtime ability support.
use serde::Deserialize;
use std::sync::OnceLock;
#[derive(Debug, Deserialize)]
pub struct GenerationCreature {
    pub id: String,
    pub name: String,
    pub abilities: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct GenerationBestiary {
    pub bestiary: Vec<GenerationCreature>,
    pub events: Vec<GenerationCreature>,
    pub families: Vec<String>,
}
pub fn generation_bestiary() -> &'static GenerationBestiary {
    static C: OnceLock<GenerationBestiary> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/generation-bestiary-v48.json"))
            .expect("frozen bestiary metadata")
    })
}
pub fn generation_creature(kind: &str) -> Option<&'static GenerationCreature> {
    let original = kind
        .strip_prefix("Mercuria’s ")
        .or_else(|| kind.strip_prefix("Mercuria's "))
        .unwrap_or(kind);
    let name = if original == "vent-fungus" || original == "Vent Fungus" {
        "bastidion"
    } else {
        original
    };
    generation_bestiary()
        .bestiary
        .iter()
        .chain(&generation_bestiary().events)
        .find(|d| d.id == name || d.name == name)
}

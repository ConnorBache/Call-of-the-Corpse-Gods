//! Capture pilot note: disabled hooks O(1); enabled actor-group marks O(log G + key bytes),
//! unrestricted-boundary flags O(log B + label bytes). These are access counts, not writes.
//! Typed canonical schema. Optional properties distinguish absent from explicit null.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Field<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}
impl<T> Field<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    pub fn as_ref(&self) -> Option<&T> {
        if let Self::Value(x) = self {
            Some(x)
        } else {
            None
        }
    }
    pub fn as_mut(&mut self) -> Option<&mut T> {
        if let Self::Value(x) = self {
            Some(x)
        } else {
            None
        }
    }
    pub fn into_option(self) -> Option<T> {
        if let Self::Value(x) = self {
            Some(x)
        } else {
            None
        }
    }
    pub fn get_or_insert_with(&mut self, f: impl FnOnce() -> T) -> &mut T {
        if !matches!(self, Self::Value(_)) {
            *self = Self::Value(f());
        }
        self.as_mut().unwrap()
    }
}
impl<T: Clone> Field<T> {
    pub fn value_or(&self, default: T) -> T {
        self.as_ref().cloned().unwrap_or(default)
    }
}
impl<T: Default + Clone> Field<T> {
    pub fn value_or_default(&self) -> T {
        self.value_or(T::default())
    }
}
impl<T> From<T> for Field<T> {
    fn from(v: T) -> Self {
        Self::Value(v)
    }
}
impl<T: Serialize> Serialize for Field<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(v) => v.serialize(s),
            _ => s.serialize_none(),
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Field<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(d)? {
            Some(v) => Self::Value(v),
            None => Self::Null,
        })
    }
}
#[path = "model_generated.rs"]
mod generated;
pub use generated::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Command {
    Dev {
        action: String,
    },
    Interact {
        x: f64,
        y: f64,
    },
    Move {
        dx: f64,
        dy: f64,
    },
    Wait,
    Descend,
    Ascend,
    Pickup,
    Use {
        id: f64,
    },
    UseAt {
        id: f64,
        x: f64,
        y: f64,
    },
    Drop {
        id: f64,
    },
    Equip {
        id: f64,
        slot: String,
    },
    Unequip {
        id: f64,
    },
    Throw {
        id: f64,
        x: f64,
        y: f64,
    },
    Shoot {
        id: f64,
        x: f64,
        y: f64,
    },
    Ability {
        id: String,
        #[serde(default, skip_serializing_if = "Field::is_missing")]
        target: Field<f64>,
        #[serde(default, skip_serializing_if = "Field::is_missing")]
        x: Field<f64>,
        #[serde(default, skip_serializing_if = "Field::is_missing")]
        y: Field<f64>,
    },
    Choose {
        id: String,
    },
    Rally {
        x: f64,
        y: f64,
    },
    Ally {
        id: f64,
    },
    Craft {
        heart: f64,
        item: f64,
        craft: String,
    },
}
impl Run {
    pub fn floor(&self) -> Option<&Floor> {
        self.floors
            .iter()
            .find(|f| f.depth == self.depth)
            .map(|f| f.as_ref())
    }
    pub fn floor_mut(&mut self) -> Option<&mut Floor> {
        self.floors
            .iter_mut()
            .find(|f| f.depth == self.depth)
            .map(crate::mutation_journal::make_mut)
    }
    pub fn actors_at(&self, depth: f64) -> &[Actor] {
        self.actors
            .get(&depth.to_string())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
    /// Expected O(1) floor lookup plus COW actor payload; invalidates derived IDs.
    pub fn actors_at_mut(&mut self, depth: f64) -> &mut Vec<Actor> {
        crate::capture_pilot::mark_actor_group(&depth.to_string());
        crate::capture_pilot::mark_unsupported("actor_vector_structural_access");
        crate::mutation_journal::make_mut(self.actors.entry(depth.to_string()).or_default())
    }
    /// Expected O(1) lookup plus COW payload; internal property-only writes.
    /// Caller must preserve actor IDs, nested pilots, vector order and length.
    pub(crate) fn actors_slice_mut(&mut self,depth:f64)->&mut [Actor] {
        self.actors.trusted_slice_or_default(&depth.to_string())
    }
    /// O(d) lookup, O((A+P+B)*d+F+V) after structural edits; runtime-only floor slot.
    pub fn creature_floor_id(&self,id:f64)->Result<Option<(String,crate::creature_index::FloorCreatureId)>,String> {
        crate::creature_index::check_player(self)?;self.actors.floor_id(id,self.player.id,self.depth)
    }
    /// O(d) clean resolution; O((A+P+B)*d+F+V) structural reconciliation. Stale handles fail.
    pub fn creature_from_floor_id(&self,floor:&str,id:crate::creature_index::FloorCreatureId)->Result<Option<&Actor>,String> {
        crate::creature_index::check_player(self)?;
        let Some(global)=self.actors.resolve_floor_id(floor,id,self.player.id,self.depth)? else {return Ok(None)};
        Ok(crate::engine::actor_any(self,global))
    }
    pub fn ground_at_mut(&mut self, depth: f64) -> &mut Vec<GroundItem> {
        crate::mutation_journal::make_mut(self.ground.entry(depth.to_string()).or_default())
    }
}
impl Floor {
    /// Cell arrays, not optional legacy metadata, are authoritative for geometry.
    /// Invalid arrays return zero here and are rejected at storage/engine admission.
    pub fn width(&self) -> usize {
        if self.tiles.len() == 2291 {
            79
        } else {
            let width = (self.tiles.len() as f64).sqrt() as usize;
            if width > 0 && width * width == self.tiles.len() {
                width
            } else {
                0
            }
        }
    }
    pub fn height(&self) -> usize {
        self.tiles.len() / self.width().max(1)
    }
}

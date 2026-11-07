//! Bounded final-value save operations, not gameplay commands or a world journal.
use super::{Engine, Receipt, Stamp, MAX_PENDING_COMMANDS};
use corpse_core::{Field, storage::StorageError};
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "ids", rename_all = "camelCase", deny_unknown_fields)]
pub enum OrderValue {
    Missing,
    Null,
    Ids(Vec<f64>),
}
impl From<Field<Vec<f64>>> for OrderValue {
    fn from(value: Field<Vec<f64>>) -> Self {
        match value { Field::Missing => Self::Missing, Field::Null => Self::Null, Field::Value(ids) => Self::Ids(ids) }
    }
}
impl From<OrderValue> for Field<Vec<f64>> {
    fn from(value: OrderValue) -> Self {
        match value { OrderValue::Missing => Self::Missing, OrderValue::Null => Self::Null, OrderValue::Ids(ids) => Self::Value(ids) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "text", rename_all = "camelCase", deny_unknown_fields)]
pub enum RunIdValue {
    Missing,
    Null,
    Text(String),
}
impl From<Field<String>> for RunIdValue {
    fn from(value: Field<String>) -> Self {
        match value { Field::Missing => Self::Missing, Field::Null => Self::Null, Field::Value(text) => Self::Text(text) }
    }
}
impl From<RunIdValue> for Field<String> {
    fn from(value: RunIdValue) -> Self {
        match value { RunIdValue::Missing => Self::Missing, RunIdValue::Null => Self::Null, RunIdValue::Text(text) => Self::Value(text) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Operation {
    Command { command: corpse_core::Command, after: Stamp },
    InventoryDisplayOrder { value: OrderValue, after: Stamp },
    RunId { value: RunIdValue, after: Stamp },
}
impl From<Receipt> for Operation {
    fn from(receipt: Receipt) -> Self { Self::Command { command: receipt.command, after: receipt.after } }
}
impl Operation {
    /// O(selected ID/string bytes), never scans the world or inventory membership.
    /// Removed/duplicate IDs are preserved here; legacy load normalization is separate.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::InventoryDisplayOrder { value: OrderValue::Ids(ids), .. } => {
                if ids.iter().any(|id| !id.is_finite()) {
                    return Err("Inventory display order contains a non-finite ID".into());
                }
            }
            Self::RunId { value: RunIdValue::Null, .. } => {
                return Err("Null run identity is not admitted".into());
            }
            Self::RunId { value: RunIdValue::Text(text), .. } if !valid_run_id(text) => {
                return Err("Invalid run identity metadata".into());
            }
            _ => {}
        }
        Ok(())
    }
    /// O(command execution) or O(selected payload move), followed by a cheap stamp.
    /// Metadata changes no replay ledger, RNG, tick, temporal policy or turn count.
    pub fn apply(self, engine: &mut corpse_core::temporal::Engine) -> Result<(), String> {
        self.validate()?;
        let after = match self {
            Self::Command { command, after } => {
                engine.dispatch(&command).map_err(|e| e.to_string())?;
                after
            }
            Self::InventoryDisplayOrder { value, after } => {
                engine.run.inventory_display_order = value.into();
                after
            }
            Self::RunId { value, after } => {
                engine.run.run_id = value.into();
                after
            }
        };
        if !after.verify(engine) { return Err("Save operation replay diverged".into()); }
        Ok(())
    }
}
fn valid_run_id(text: &str) -> bool {
    !text.is_empty() && text.encode_utf16().count() <= 200 && !text.chars().any(|c| (c as u32) < 32)
}
fn same_order(a: &Field<Vec<f64>>, b: &Field<Vec<f64>>) -> bool {
    match (a, b) {
        (Field::Missing, Field::Missing) | (Field::Null, Field::Null) => true,
        (Field::Value(a), Field::Value(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a,b)| a.to_bits() == b.to_bits()),
        _ => false,
    }
}
impl Engine {
    /// O(selected payload bytes) preflight. No mutation or epoch change on refusal.
    fn admit_metadata(operation: &Operation) -> Result<(), String> {
        operation.validate()?;
        let encoded = serde_json::to_vec(operation).map_err(|e| e.to_string())?;
        if encoded.len() > super::super::incremental_log::MAX_BATCH_PAYLOAD_BYTES - 64 {
            return Err("Metadata operation exceeds save batch capacity".into());
        }
        Ok(())
    }
    /// Marks unknown BEFORE canonical assignment/potential queue allocation.
    /// Prior unknown/overflow state is never cleared by a known metadata write.
    fn begin_metadata(&mut self) -> bool {
        let tracking = self.tracking.get_mut().unwrap_or_else(|e| e.into_inner());
        Self::advance(tracking);
        let prior = tracking.checkpoint_required;
        tracking.checkpoint_required = true;
        prior
    }
    fn finish_metadata(&mut self, operation: Operation, prior_unknown: bool) {
        let tracking = self.tracking.get_mut().unwrap_or_else(|e| e.into_inner());
        if !prior_unknown && tracking.operations.len() < MAX_PENDING_COMMANDS {
            tracking.operations.push(operation);
            tracking.checkpoint_required = false;
        }
    }
    /// O(I) exact field comparison/capture/encoding; no Run diff or mutable escape.
    pub fn set_inventory_display_order(&mut self, desired: Field<Vec<f64>>) -> Result<bool, String> {
        if same_order(&self.core.run.inventory_display_order, &desired) { return Ok(false); }
        let operation = Operation::InventoryDisplayOrder { value: desired.clone().into(), after: Stamp::capture(&self.core) };
        Self::admit_metadata(&operation)?;
        let prior = self.begin_metadata();
        self.core.run.inventory_display_order = desired;
        self.finish_metadata(operation, prior);
        Ok(true)
    }
    /// O(ID bytes). Missing/Null/Text remain distinct, including serialized records.
    pub fn set_run_id(&mut self, desired: Field<String>) -> Result<bool, String> {
        if self.core.run.run_id == desired { return Ok(false); }
        let operation = Operation::RunId { value: desired.clone().into(), after: Stamp::capture(&self.core) };
        Self::admit_metadata(&operation)?;
        let prior = self.begin_metadata();
        self.core.run.run_id = desired;
        self.finish_metadata(operation, prior);
        Ok(true)
    }
    /// Same identity validation/generator contract as core::profile::ensure_run_id.
    /// Existing valid identity neither invokes the generator nor advances the token.
    pub fn ensure_run_id(&mut self, create_id: impl FnOnce() -> String) -> Result<String, StorageError> {
        let invalid = || StorageError("Invalid descent profile. Existing records were not changed.".into());
        if let Some(id) = self.core.run.run_id.as_ref() {
            if !valid_run_id(id) { return Err(invalid()); }
            return Ok(id.clone());
        }
        if !self.core.run.run_id.is_missing() { return Err(invalid()); }
        let id = create_id();
        if !valid_run_id(&id) { return Err(invalid()); }
        self.set_run_id(Field::Value(id.clone())).map_err(StorageError)?;
        Ok(id)
    }
}

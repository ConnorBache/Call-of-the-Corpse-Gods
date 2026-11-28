//! R52 command boundary for the shared ground-item/corpse footprint.
use crate::{Field, Run};

pub fn current(r:&Run)->bool {
    r.simulation_revision.value_or(29.)>=52. && matches!(r.remains_rules_from,Field::Value(_))
}
pub(crate) fn validate(r:&Run)->Result<(),String> {
    if !r.remains_rules_from.is_missing() &&
        !(r.simulation_revision.value_or(29.)>=52. &&
          matches!(r.remains_rules_from,Field::Value(n) if n<=r.replay.len())){
        return Err("Invalid remains rules command boundary".into());
    }
    Ok(())
}
pub fn admit(r:&mut Run)->Result<(),String>{
    validate(r)?;
    if r.simulation_revision.value_or(29.)>=52. && r.remains_rules_from.is_missing(){
        r.remains_rules_from=r.replay.len().into();
        crate::corpse_placement::separate_overlaps(r)?;
    }
    Ok(())
}
pub(crate) fn prepare_replay(r:&mut Run){
    r.remains_rules_from=Field::Missing;
}
pub(crate) fn apply_replay(r:&mut Run,from:&Field<usize>,cursor:usize)->Result<(),String>{
    if !current(r) && from.as_ref()==Some(&cursor) {
        r.remains_rules_from=cursor.into();
        crate::corpse_placement::separate_overlaps(r)?;
    }
    Ok(())
}

#[cfg(test)]
#[path="remains_rules_tests.rs"]
mod tests;

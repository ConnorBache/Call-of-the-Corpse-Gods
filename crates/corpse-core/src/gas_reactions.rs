//! Persisted fire reaction identity union; geometry never creates identities.
use std::collections::HashMap;
const MAX_SAFE: u64 = 9007199254740991;
#[derive(Default)]
pub struct GasReactions {
    parents: HashMap<u64, u64>,
    ranks: HashMap<u64, u32>,
    minimums: HashMap<u64, u64>,
    pub next: u64,
}
impl GasReactions {
    pub fn new(next: u64) -> Self {
        Self {
            next,
            ..Default::default()
        }
    }
    fn root(&mut self, mut id: u64) -> Result<u64, String> {
        if id == 0 || id >= self.next || id > MAX_SAFE {
            return Err("Invalid gas reaction identity.".into());
        }
        if !self.parents.contains_key(&id) {
            self.parents.insert(id, id);
            self.ranks.insert(id, 0);
            self.minimums.insert(id, id);
            return Ok(id);
        }
        let mut top = id;
        while self.parents[&top] != top {
            top = self.parents[&top]
        }
        while id != top {
            let next = self.parents[&id];
            self.parents.insert(id, top);
            id = next
        }
        Ok(top)
    }
    pub fn find(&mut self, id: u64) -> Result<u64, String> {
        if id == 0 {
            return Ok(0);
        }
        let root = self.root(id)?;
        Ok(self.minimums[&root])
    }
    pub fn merge(&mut self, a: u64, b: u64) -> Result<u64, String> {
        if a == 0 {
            return self.find(b);
        }
        if b == 0 {
            return self.find(a);
        }
        let mut left = self.root(a)?;
        let mut right = self.root(b)?;
        if left == right {
            return Ok(self.minimums[&left]);
        }
        let minimum = self.minimums[&left].min(self.minimums[&right]);
        if self.ranks[&left] < self.ranks[&right] {
            std::mem::swap(&mut left, &mut right)
        }
        self.parents.insert(right, left);
        if self.ranks[&left] == self.ranks[&right] {
            *self.ranks.get_mut(&left).unwrap() += 1;
        }
        self.minimums.insert(left, minimum);
        Ok(minimum)
    }
    pub fn allocate(&mut self) -> Result<u64, String> {
        let id = self.next;
        if id < 1 || id >= MAX_SAFE {
            return Err("Gas reaction identity space exhausted.".into());
        }
        self.next += 1;
        self.root(id)?;
        Ok(id)
    }
    pub fn finish(&mut self, ids: &mut [u64], active: &[bool]) -> Result<(), String> {
        for (i, id) in ids.iter_mut().enumerate() {
            *id = if active[i] { self.find(*id)? } else { 0 }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimum_identity_survives_rank_and_zero() {
        let mut r = GasReactions::new(1);
        let ids: Vec<_> = (0..8).map(|_| r.allocate().unwrap()).collect();
        r.merge(ids[6], ids[7]).unwrap();
        r.merge(ids[4], ids[5]).unwrap();
        r.merge(ids[4], ids[6]).unwrap();
        assert_eq!(r.merge(ids[0], ids[7]).unwrap(), 1);
        assert_eq!(r.find(ids[5]).unwrap(), 1);
        assert_eq!(r.merge(0, 8).unwrap(), 1);
        assert!(r.find(9).is_err());
    }
    #[test]
    fn expiration_is_not_a_bridge() {
        let mut r = GasReactions::new(3);
        let mut ids = [1, 2, 1];
        r.finish(&mut ids, &[true, false, true]).unwrap();
        assert_eq!(ids, [1, 0, 1]);
    }
}

//! Explicit wrapping equivalents of JavaScript bitwise arithmetic.
use crate::Run;
pub fn seed_hash(s: &str) -> u32 {
    let mut h = 2166136261u32;
    for c in s.chars() {
        let code = if c as u32 > 0xffff {
            0xd800 + ((c as u32 - 0x10000) >> 10)
        } else {
            c as u32
        };
        h = (h ^ code).wrapping_mul(16777619);
    }
    if h == 0 { 1 } else { h }
}
pub fn to_u32(n: f64) -> u32 {
    if !n.is_finite() || n == 0.0 {
        0
    } else {
        n.trunc().rem_euclid(4294967296.0) as u32
    }
}
pub fn next_u32(state: &mut u32) -> u32 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    *state = s;
    s
}
pub fn random(run: &mut Run) -> f64 {
    let mut state = to_u32(run.rng);
    let n = next_u32(&mut state);
    run.rng = state as f64;
    n as f64 / 4294967296.0
}
pub fn mix(seed: u32, depth: u32) -> u32 {
    let mut h = seed ^ depth.wrapping_mul(0x9e3779b1);
    h = (h ^ (h >> 16)).wrapping_mul(0x85ebca6b);
    h ^ (h >> 13)
}
#[derive(Debug, Clone, Copy)]
pub struct GenerationRng {
    pub state: u32,
}
impl GenerationRng {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }
    pub fn random(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b79f5);
        let s = self.state;
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        (t ^ (t >> 14)) as f64 / 4294967296.0
    }
    pub fn integer(&mut self, min: i32, max: i32) -> i32 {
        min + (self.random() * (max - min + 1) as f64).floor() as i32
    }
}

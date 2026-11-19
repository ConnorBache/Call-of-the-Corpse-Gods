//! Policy 1: uniform complete Avatar tuples, never per-god weighted rerolls.
//! Eligibility is an audited rules table, not a substring search of UI prose.
use crate::{progression, rng};
pub const POLICY: u8 = 1;
pub const DIRECT_COMBAT: &[&str] = &[
 "shrail-rage-at-the-dying-of-the-light", "shrail-execute", "shrail-fight-long",
 "pelian-you-shall-never-be-rid-of-me", "pelian-fated-duel", "pelian-never-surrender", "pelian-bulwark-of-mind",
 "isheilah-missed-me", "isheilah-are-you-sure-about-that",
 "ursa-sudden-sleep", "ursa-oblivious",
 "lichor-overflowing", "lichor-bleed-them", "lichor-blood-letting", "lichor-hardened-blood", "lichor-blood-weapon", "lichor-the-number-of-blood",
 "dawel-silent-strike", "dawel-not-there", "grisha-witch-hatred", "anavani-nova",
 "kative-mini-me", "kative-clay-army", "morae-fortune-teller", "morae-bonded-by-unseen-threads", "morae-turning-the-wheel",
];
/// Actual positive energy debits: activation, upkeep, proc or Hanged damage.
/// No HP/blood/reaction/fortune costs, energy gain or amulet discounts qualify.
/// Ragelight qualifies after its free rescue; Unspeak when the target has a paid
/// ability. Current Futuresight is free and is deliberately absent.
pub const ENERGY_SPENDERS: &[&str] = &[
 "shrail-rage-at-the-dying-of-the-light", "shrail-execute",
 "pelian-i-came-for-you", "pelian-fated-duel", "pelian-never-surrender", "pelian-bulwark-of-mind", "pelian-relentless-pursuit",
 "isheilah-missed-me", "isheilah-make-slippery",
 "numo-implant-thought", "numo-distract", "numo-mind-read", "numo-mind-slave", "numo-remember",
 "etill-phase-through", "etill-rushing-torrent", "etill-riptide",
 "ursa-sudden-sleep", "ursa-dream-of-time-gone-by", "ursa-dream-see",
 "dawel-unspeak", "dawel-shadow-in-the-dark", "dawel-not-there",
 "grisha-erase-metaphysical", "grisha-black-pit",
 "anavani-ascend", "anavani-nova", "anavani-illuminate", "anavani-commandment",
 "kative-cogworks", "kative-mini-me", "kative-clay-army", "kative-tinker",
 "morae-bonded-by-unseen-threads", "morae-turning-the-wheel", "morae-sever-fate", "morae-all-seeing-eyes",
];
pub fn direct_combat(id: &str) -> bool { DIRECT_COMBAT.contains(&id) }
pub fn energy_spender(id: &str) -> bool { ENERGY_SPENDERS.contains(&id) }
pub fn valid(tuple: &[&str; 3]) -> bool {
 tuple.iter().any(|id| direct_combat(id)) && tuple.iter().any(|id| energy_spender(id))
}
/// Fixed catalog order within each selected god; selected god order is retained.
/// O(A + 6^3 * (D+E)) time and at most 216 three-reference tuples of scratch.
pub fn valid_tuples(gods: &[f64]) -> Result<Vec<[&'static str; 3]>, String> {
 if gods.len()!=3 || gods.iter().any(|g| !g.is_finite() || g.fract()!=0. || !(0.0..12.0).contains(g))
     || gods[0]==gods[1] || gods[0]==gods[2] || gods[1]==gods[2] { return Err("Starter policy requires three distinct god IDs".into()); }
 let catalog=progression::catalog_for_revision(52.);
 let pools:Vec<Vec<_>>=gods.iter().map(|g|catalog.iter().filter(|a|a.god_id==*g).map(|a|a.id.as_str()).collect()).collect();
 if pools.iter().any(|p|p.len()!=6) {return Err("Starter policy 1 requires six current blessings per god".into());}
 let mut tuples=Vec::new();
 for &a in &pools[0] {for &b in &pools[1] {for &c in &pools[2] {let t=[a,b,c];if valid(&t){tuples.push(t);}}}}
 if tuples.is_empty(){return Err("Selected gods have no valid starter tuple".into());}
 Ok(tuples)
}
/// Xorshift's nonzero cycle has 2^32-1 outputs, not 2^32. Convert to a zero-based
/// population, discard its short remainder, then reduce. At most bound-1 states
/// are rejected in the entire cycle; thus bound draws guarantee acceptance.
fn uniform_index(state: &mut u32, bound: usize) -> Result<usize,String> {
 if *state==0 || bound==0 || bound>216 {return Err("Invalid starter random domain".into());}
 let bound=bound as u32;let limit=u32::MAX-u32::MAX%bound;
 for _ in 0..bound {let sample=rng::next_u32(state)-1;if sample<limit{return Ok((sample%bound)as usize);}}
 Err("Starter rejection bound exceeded".into())
}
pub fn choose(seed: &str,gods: &[f64]) -> Result<Vec<String>,String> {
 let tuples=valid_tuples(gods)?;
 let mut state=rng::seed_hash(&format!("avatar-starters-v1:{seed}"));
 let index=uniform_index(&mut state,tuples.len())?;
 Ok(tuples[index].iter().map(|id|(*id).to_owned()).collect())
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn all_220_trios_have_complete_uniform_candidate_domains() {
  let catalog=progression::catalog_for_revision(52.);assert_eq!(catalog.len(),72);
  assert_eq!(DIRECT_COMBAT.len(),26);assert_eq!(ENERGY_SPENDERS.len(),37);
  for set in [DIRECT_COMBAT,ENERGY_SPENDERS] {let unique:std::collections::HashSet<_>=set.iter().collect();assert_eq!(unique.len(),set.len());assert!(set.iter().all(|id|catalog.iter().any(|a|a.id==*id)));}
  let mut count=0;let mut low=216;let mut high=0;let mut triple_combat=0;
  for a in 0..12 {for b in a+1..12 {for c in b+1..12 {
   count+=1;let gods=[a as f64,b as f64,c as f64];let tuples=valid_tuples(&gods).unwrap();
   low=low.min(tuples.len());high=high.max(tuples.len());
   let unique:std::collections::HashSet<_>=tuples.iter().collect();assert_eq!(unique.len(),tuples.len());
   let pools:Vec<_>=gods.iter().map(|g|catalog.iter().filter(|v|v.god_id==*g).collect::<Vec<_>>()).collect();
   let mut expected=0;for x in &pools[0]{for y in &pools[1]{for z in &pools[2]{let t=[x.id.as_str(),y.id.as_str(),z.id.as_str()];assert_eq!(tuples.contains(&t),valid(&t));expected+=usize::from(valid(&t));}}}
   assert_eq!(expected,tuples.len());triple_combat+=tuples.iter().filter(|t|t.iter().all(|id|direct_combat(id))).count();
   // Every ordering keeps the blessing in its selected god's slot.
   for order in [[a,b,c],[a,c,b],[b,a,c],[b,c,a],[c,a,b],[c,b,a]] {
    let gods=order.map(|v|v as f64);let domain=valid_tuples(&gods).unwrap();assert_eq!(domain.len(),expected);
    for t in domain {for i in 0..3 {assert_eq!(catalog.iter().find(|v|v.id==t[i]).unwrap().god_id,gods[i]);}}
   }
  }}}
  assert_eq!((count,low,high,triple_combat),(220,33,210,1747));
 }
 #[test] fn unbiased_domain_has_equal_residue_counts_and_bounded_rejection() {
  for n in 1..=216u32 {let accepted=u32::MAX-u32::MAX%n;assert_eq!(accepted%n,0);assert!(u32::MAX-accepted<n);
   for i in 0..n {assert_eq!((accepted-n+i)%n,i);}
   for seed in 1..=100u32 {assert!(uniform_index(&mut seed.clone(),n as usize).unwrap()<n as usize);}
  }
  assert!(uniform_index(&mut 0,216).is_err());
 }
 #[test] fn maximum_output_is_rejected_using_nonzero_domain_cardinality() {
  fn undo_left(y:u32,n:u32)->u32 {let mut x=y;for _ in 0..32{x=y^(x<<n);}x}
  fn undo_right(y:u32,n:u32)->u32 {let mut x=y;for _ in 0..32{x=y^(x>>n);}x}
  let seed=undo_left(undo_right(undo_left(u32::MAX,5),17),13);
  let mut proof=seed;assert_eq!(rng::next_u32(&mut proof),u32::MAX);
  let mut state=seed;let i=uniform_index(&mut state,2).unwrap();
  assert_ne!(state,u32::MAX);assert_eq!(i,((state-1)%2)as usize);
 }
 #[test] fn aliases_and_real_debits_not_energy_words_define_eligibility() {
  assert!(direct_combat(crate::hanged::ID)&&energy_spender(crate::hanged::ID));
  assert_eq!(progression::definition_for_revision(52.,crate::hanged::ID).unwrap().name,"Hanged");
  for id in ["morae-see-the-future","grisha-boundless-hunger","kative-appropriate","morae-fortune-teller","lichor-blood-weapon","isheilah-quick-hand"] {assert!(!energy_spender(id),"{id}");}
  assert!(energy_spender("pelian-fated-duel"));assert!(energy_spender("dawel-unspeak"));
 }
 #[test] fn policy_keeps_global_rng_and_non_avatar_history_unchanged() {
  for seed in ["0-1-2:starter-domain","3-4-8:minimum","1-3-6:maximum","canonical seed"] {
   let old=crate::startup::new_run_shell_revision(seed,"Avatar",false,52.).unwrap();
   let new=crate::startup::new_run_shell_revision_policy(seed,"Avatar",false,52.,&crate::Field::Value(POLICY)).unwrap();
   assert_eq!(old.seed,new.seed);assert_eq!(old.rng,new.rng);assert_eq!(old.gods,new.gods);
   assert_eq!(choose(&new.seed,&new.gods).unwrap(),new.player.abilities);
   let ids:Vec<_>=new.player.ability_reserves.as_ref().unwrap().keys().cloned().collect();assert_eq!(ids,new.player.abilities);
   assert!(valid(&[&new.player.abilities[0],&new.player.abilities[1],&new.player.abilities[2]]));
  }
  for mode in ["Blessed","Unblessed"] {let old=crate::startup::new_run_shell_revision("0-1-2:other",mode,false,52.).unwrap();let same=crate::startup::new_run_shell_revision_policy("0-1-2:other",mode,false,52.,&crate::Field::Missing).unwrap();assert_eq!(serde_json::to_value(old).unwrap(),serde_json::to_value(same).unwrap());}
 }
}

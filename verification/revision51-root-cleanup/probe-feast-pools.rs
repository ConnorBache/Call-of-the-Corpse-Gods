fn main() {
    for depth in 1..=40 {
        let pools = corpse_core::fungus_feasts::victim_pools(depth,51.).unwrap();
        println!("{depth}: {}",pools[1].join(", "));
    }
    for depth in [14,15,16] {
        let kinds: std::collections::BTreeSet<_> = corpse_core::ordinary_roster::candidates_revision(depth,true,51.).unwrap().iter().flat_map(|r|r.kinds.iter().map(String::as_str)).collect();
        println!("water-inclusive floor {depth}: {kinds:?}");
    }
}

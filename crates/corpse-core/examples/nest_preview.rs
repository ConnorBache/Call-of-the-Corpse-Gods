//! Reproducible real generated-world inspection artifact, never shipped as a run.
fn main()->Result<(),Box<dyn std::error::Error>> {
    let output=std::env::args().nth(1).ok_or("output path")?;
    let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:nest-preview","Unblessed",true,33.0)?;
    let d=corpse_core::pocket_biomes::nest_depth(corpse_core::rng::seed_hash(&r.seed));
    corpse_core::startup::ensure_floors(&mut r,d)?;r.depth=d as f64;
    let f=r.floor().unwrap();let at=f.tiles.iter().position(|t|t=="up").unwrap();let w=f.width();r.player.x=(at%w)as f64;r.player.y=(at/w)as f64;
    r.player.conditions.insert("allSeeingReveal".into(),1.0);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    let f=r.floor_mut().unwrap();f.seen.fill(true);f.lights.fill(35.0);f.remembered_tiles=f.tiles.clone().into();
    std::fs::write(&output,corpse_core::storage::serialize(&r)?)?;
    eprintln!("Real generated {} floor{}: {:?}",r.seed,d,r.floor().unwrap().pocket_biomes.as_ref().map(|bs|bs.iter().map(|b|(&b.theme,b.cells.len(),&b.rooms)).collect::<Vec<_>>()));Ok(())
}

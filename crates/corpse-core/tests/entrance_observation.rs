//! The entrance marker follows known terrain, never hidden live terrain mutations.
use corpse_core::{Field, storage, vision};
#[test]
fn remembered_entrance_does_not_reveal_hidden_tile_changes() {
    let mut run=storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.player.x=1.; run.player.y=1.; run.player.stats.perception=0.;
    let f=run.floor_mut().unwrap();
    let at=(f.height()-2)*f.width()+f.width()/2;
    f.entrance=corpse_core::entrance::Entrance {x:(f.width()/2) as f64,y:(f.height()-2) as f64}.into();
    f.tiles[at]="up".into(); f.lights.fill(-100.); f.seen.fill(false); f.seen[at]=true;
    f.remembered_tiles=f.tiles.clone().into();
    let before=vision::observe_terrain(&run).unwrap();
    assert!(!before.cells[at].current);
    assert!(before.cells[at].entrance);
    assert_eq!(before.cells[at].tile.as_deref(),Some("up"));
    run.floor_mut().unwrap().tiles[at]="water".into();
    let after=vision::observe_terrain(&run).unwrap();
    assert_eq!(after.cells[at].entrance,before.cells[at].entrance);
    assert_eq!(after.cells[at].tile,before.cells[at].tile);
    run.floor_mut().unwrap().seen[at]=false;
    run.floor_mut().unwrap().remembered_tiles=Field::Missing;
    let unknown=vision::observe_terrain(&run).unwrap();
    assert!(!unknown.cells[at].entrance);
    assert!(unknown.cells[at].tile.is_none());
}

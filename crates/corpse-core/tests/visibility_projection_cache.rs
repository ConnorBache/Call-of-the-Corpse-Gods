use corpse_core::{Actor,Floor,Run};
use corpse_core::vision::{self,VisibilityProjection};
use std::sync::Arc;

/// O(V+A) deterministic adversarial synthetic map, no gameplay RNG.
fn room(case: usize) -> Run {
    let mut r=Run{depth:10.,simulation_revision:39.0.into(),..Default::default()};
    r.player=Actor{id:0.,x:2.,y:2.,hp:100.,..Default::default()};
    let mut f=Floor{depth:10.,tiles:(0..225).map(|i|if (i*17+case)%29==0{"wall"}else if (i+case)%13==0{"water"}else{"floor"}.into()).collect(),lights:(0..225).map(|i|-90.+((i*7+case)%190) as f64).collect(),seen:vec![false;225],..Default::default()};
    f.tiles[32]="floor".into();
    f.portals=(0..(case%8)).map(|i|corpse_core::portals_generation::PortalPair{a:40+i*2,b:180+i*2,color_index:i as u8}).collect::<Vec<_>>().into();
    r.floors.push(Arc::new(f));
    for i in 1..=32 {
        let mut a=Actor{id:i as f64,x:((i*3+case)%15)as f64,y:((i*7+case)%15)as f64,hp:100.,state:if i%3==0{"sleeping"}else{"hunting"}.into(),..Default::default()};
        if i%4==0{a.abilities.push("anavani-shine".into());if i%8==0{a.active.push("anavani-shine".into());}}
        if i%7==0{a.abilities.push("enemy:beast:darkness-aura".into());}
        if i%5==0{a.abilities.push("dawel-shadow-in-the-dark".into());a.active.push("dawel-shadow-in-the-dark".into());}
        if i%6==0{a.conditions.insert("stealth".into(),17.5);a.conditions.insert("submergedStealth".into(),5.5);}
        if i%9==0{a.conditions.insert("invisible".into(),1.);}
        if i%11==0{a.perfect=true.into();}
        if i%13==0{a.dead=true.into();}
        r.actors_at_mut(10.).push(a);
    }
    r
}

/// O(A*(scalar visibility cost + projected cost)) comparison; no mutation.
fn compare(r:&Run) {
    let projection=VisibilityProjection::new(r);
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        assert_eq!(projection.directly_visible_actor(a),vision::directly_visible_actor(r,a),"direct id={} pos={},{}",a.id,a.x,a.y);
        assert_eq!(projection.visible_actor(a),vision::visible_actor(r,a),"visible id={} pos={},{}",a.id,a.x,a.y);
    }
}

/// O(cases*edits*A*scalar visibility cost): catches cache reuse and fresh origin errors.
#[test]
fn projected_actor_visibility_matches_scalar_special_cases() {
    for case in 0..16 {
        let mut r=room(case);compare(&r);
        r.player.x=12.;r.player.y=12.;compare(&r);
        r.player.abilities.push("anavani-shine".into());r.player.active.push("anavani-shine".into());compare(&r);
        r.floor_mut().unwrap().tiles[80]="tree".into();compare(&r);
        r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),10.);compare(&r);
        r.player.conditions.shift_remove("allSeeingReveal");
        corpse_core::dreamwalking::ensure_layer(&mut r);
        r.actors_at_mut(10.)[10].conditions.insert("phantasmiiPlane".into(),1.);compare(&r);
        r.actors_at_mut(10.)[11].conditions.insert("phantasmiiNightmare".into(),1.);compare(&r);
        r.player.conditions.insert("telepathic".into(),1.);compare(&r);
        compare(&r.clone());
    }
}

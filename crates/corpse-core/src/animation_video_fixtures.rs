//! Opt-in actual gameplay fixtures for native animation review captures.
use crate::*;
fn room()->Run{let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=12.;r.player.y=12.;r.player.hp=100.;r.player.max_hp=100.;r.player.energy=1000.;r.player.stats.max_energy=1000.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_reserves=Field::Missing;r.inventory.clear();r.offers.clear();r.actors_at_mut(r.depth).clear();r.ground_at_mut(r.depth).clear();r.next_id=1000.;let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.environment=Field::Missing;f.doors=Field::Missing;f.godbeast_event=Field::Missing;let mut a=population::create_actor_revision("rat",900.,18.,12.,false,52.).unwrap();a.hp=10.;a.max_hp=100.;a.conditions.clear();a.abilities.clear();a.state="sleeping".into();r.actors_at_mut(r.depth).push(a);r}
fn grant(r:&mut Run,id:&str){r.player.abilities=vec![id.into()];revision50_ability_energy::initialize_reserve(&mut r.player,id,10.,10.).unwrap();}
fn export(name:&str,before:&Run,after:&Run,events:Vec<events::PresentationEvent>,frames:Vec<events::PresentationFrame>){let dir=std::env::var("CORPSE_ANIMATION_EXPORT_DIR").unwrap();std::fs::create_dir_all(&dir).unwrap();assert!(!events.is_empty(),"missing receipt {name}");let frames:Vec<_>=frames.iter().filter_map(|f|f.observation.downcast_ref::<Run>().map(|r|serde_json::json!({"action":f.action,"run":r}))).collect();std::fs::write(format!("{dir}/{name}.json"),serde_json::to_vec(&serde_json::json!({"before":before,"after":after,"events":events,"frames":frames})).unwrap()).unwrap();}
fn record(name:&str,mut r:Run,f:impl FnOnce(&mut Run)){let before=r.clone();let observer=events::PresentationObserver::install(|r|std::rc::Rc::new(r.clone()));events::begin();f(&mut r);events::checkpoint(&r);let e=events::finish();export(name,&before,&r,e,observer.take());}
#[test]#[ignore="explicit actual animation capture fixtures"]fn export_new_animation_fixtures(){
 for (name,id)in[("ragelight",divine_rescues::RAGE),("never-be-rid-of-me",divine_rescues::PELIAN),("overflowing-blood-mist",divine_rescues::OVERFLOWING)]{let mut r=room();grant(&mut r,id);r.player.active.push(id.into());r.player.hp=-1.;r.player.blood=1.0.into();record(name,r,|r|{assert!(divine_rescues::try_rescue(r,r.player.id,false));});}
 {let mut r=room();grant(&mut r,revision50_combat::EXECUTE);r.player.active=r.player.abilities.clone();record("execute",r,|r|{let a=r.player.clone();let b=r.actors_at(r.depth)[0].clone();assert!(revision50_combat::execute_crossing(r,&a,&b,25.,25.,24.));});}
 for(name,id,x,y)in[("i-came-for-you","pelian-i-came-for-you",18.,12.),("fated-duel","pelian-fated-duel",18.,12.),("mind-slave","numo-mind-slave",18.,12.),("half-here","etill-half-here",15.,15.),("riptide","etill-riptide",14.,15.),("reverse-entropy","dawel-reverse-entropy",12.,12.),("bonded-by-unseen-threads","morae-bonded-by-unseen-threads",18.,12.),("waking-nightmares","ursa-waking-nightmares",15.,15.)]{let mut r=room();grant(&mut r,id);if id=="pelian-fated-duel"{r.actors_at_mut(r.depth)[0].x=13.;}r.player.nightmares=3.0.into();let before=r.clone();let observer=events::PresentationObserver::install(|r|std::rc::Rc::new(r.clone()));let out=engine::dispatch(&mut r,&Command::Ability{id:id.into(),target:900.0.into(),x:x.into(),y:y.into()}).unwrap();export(name,&before,&r,out.events,observer.take());}
 {let mut r=room();grant(&mut r,"kative-mini-me");record("mini-me",r,|r|{let a=mini_me::preview(r,15.,12.);mini_me::commit(r,a).unwrap();});}
 {let mut r=room();grant(&mut r,revision50_terrain::CLAY_ARMY);record("clay-army",r,|r|{assert!(revision50_terrain::activate_clay(r,13.,12.).unwrap());});}
 {let mut r=room();grant(&mut r,revision50_terrain::COGWORKS);revision50_terrain::select_mode(&mut r,2.);record("cogworks",r,|r|{assert!(revision50_terrain::activate_cogworks(r,20.,12.).unwrap());});}
 {let mut r=room();grant(&mut r,revision50_combat::COMMANDMENT);r.player.active=r.player.abilities.clone();r.actors_at_mut(r.depth)[0].x=13.;let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[12*w+17]="wall".into();record("commandment",r,|r|{let a=r.player.clone();let b=r.actors_at(r.depth)[0].clone();revision50_combat::after_attack(r,&a,&b,true).unwrap();});}
 {let mut r=room();grant(&mut r,"isheilah-make-slippery");r.player.active=r.player.abilities.clone();r.player.conditions.insert("slipperyTarget".into(),900.);record("make-slippery",r,|r|{engine::move_actor(r,900.,1.,0.,false).unwrap();});}
 {let mut r=room();grant(&mut r,"numo-implant-thought");let mut target=r.actors_at(r.depth)[0].clone();target.id=901.;target.x=19.;r.actors_at_mut(r.depth).push(target);record("implant-thought",r,|r|{abilities::activate(r,"numo-implant-thought",&900.0.into(),&19.0.into(),&12.0.into()).unwrap();});}
 {let mut r=room();grant(&mut r,revision50_combat::WITCH_HATRED);r.actors_at_mut(r.depth)[0].abilities.push(revision50_combat::MAGICKAL_CASTER.into());record("witch-hatred",r,|r|{let a=r.player.clone();let b=r.actors_at(r.depth)[0].clone();assert!(revision50_combat::lethal_hit(r,&a,&b));});}
 {let mut r=room();grant(&mut r,"grisha-death-swift");record("death-swift",r,|r|{engine::damage_direct(r,900.,1000.,r.player.id,"attack",true).unwrap();});}
 {let mut r=room();grant(&mut r,"grisha-erase-metaphysical");r.player.active=r.player.abilities.clone();r.player.stats.accuracy=10000.;r.actors_at_mut(r.depth)[0].x=13.;r.actors_at_mut(r.depth)[0].hp=1000.;r.actors_at_mut(r.depth)[0].abilities.push("enemy:beast:fast".into());record("erase-metaphysical",r,|r|{engine::attack(r,r.player.id,900.,1.).unwrap();});}
 {let mut r=room();grant(&mut r,"morae-see-the-future");let before=r.clone();let mut e=temporal::Engine::new(r).unwrap();let out=e.dispatch(&Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();export("see-the-future",&before,&e.run,out.events,vec![]);}

}
#[test]#[ignore="explicit actual spell capture fixtures"]fn export_amulet_animation_fixtures(){
 use revision50_amulet_effects::{make_amulet,Effect};
 for name in ["iceblast","blinking","beckoning","discord","roots","obstruction","teleportation"]{let mut r=room();let mut item=make_amulet(Effect::Lightning,800.).unwrap();item.effect=name.to_string().into();item.name=format!("Amulet of {name}");r.inventory.push(item);let before=r.clone();let observer=events::PresentationObserver::install(|r|std::rc::Rc::new(r.clone()));let(x,y)=if name=="blinking"{(16.,15.)}else if name=="obstruction"{(16.,12.)}else{(18.,12.)};let out=engine::dispatch(&mut r,&Command::UseAt{id:800.,x,y}).unwrap();export(&format!("amulet-{name}"),&before,&r,out.events,observer.take());}
}

fn beckon_scene(cast_by_player: bool) -> (Run, Command, EventCoords) {
    let mut r = room();
    r.beckoning_rules_from = 0usize.into();
    r.player.x = 12.;
    r.player.y = 12.;
    r.tick = 30.;
    r.floor_mut().unwrap().monster_entry_tick = 0.0.into();
    let w = r.floor().unwrap().width();
    let floor = r.floor_mut().unwrap();
    for y in 5..20 {
        for x in 7..32 {
            if y == 5 || y == 19 || x == 7 || x == 31 {
                floor.tiles[y*w+x] = "wall".into();
            }
        }
    }
    let (x,y) = (28.,12.);
    let (key, target, caster);
    let command = if cast_by_player {
        r.actors_at_mut(r.depth)[0].x = x;
        r.actors_at_mut(r.depth)[0].y = y;
        let mut item = Item{id:800.,kind:"amulet".into(),effect:"beckoning".to_string().into(),..Default::default()};
        revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();
        revision52_amulet_tiers::initialize_birth(&mut item,52.,1).unwrap();
        r.inventory.push(item);
        key = "A07"; target = (x,y); caster = (r.player.x,r.player.y);
        Command::UseAt { id:800., x, y }
    } else {
        r.actors_at_mut(r.depth).clear();
        let mut horn = population::create_actor_revision("horn-3",900.,x,y,false,52.).unwrap();
        horn.state = "hunting".into();
        horn.energy = 100.;
        horn.stats.perception = 300.;
        horn.abilities = vec!["enemy:beast:beckon".into()];
        r.actors_at_mut(r.depth).push(horn);
        key = "E39"; target = (r.player.x,r.player.y); caster = (x,y);
        Command::Wait
    };
    let target_id=if cast_by_player {900.} else {r.player.id};
    (r,command,EventCoords { key, target, caster, target_id })
}

struct EventCoords { key: &'static str, target: (f64,f64), caster: (f64,f64), target_id: f64 }

fn assert_beckon_direction(out: &[events::PresentationEvent], coords: EventCoords) {
    let hits: Vec<_> = out.iter().filter_map(|e|e.animation.as_ref())
        .filter(|a|a.proposal==coords.key).collect();
    assert_eq!(hits.len(),2,"missing or additional Beckon receipts: {:?}",out);
    let outbound=hits[0];
    let beam=hits[1];
    assert_eq!(outbound.style,animation_shapes::BECKON_OUTBOUND);
    assert_eq!(beam.style,animation_shapes::BECKON_BEAM);
    assert_eq!(outbound.action,beam.action);
    assert_eq!(outbound.hide_actor,None);
    assert_eq!(beam.hide_actor,Some(coords.target_id));
    assert_eq!((outbound.delivery,beam.delivery),(events::Delivery::Magic,events::Delivery::Magic));
    assert!(outbound.cells.iter().eq(beam.cells.iter().rev()));
    let coords_of = |p:&events::EventPoint| (p.x,p.y);
    assert_eq!(beam.cells.first().map(coords_of),Some(coords.target));
    assert_eq!(beam.cells.last().map(coords_of),Some(coords.caster));
    let step = (coords.caster.0-coords.target.0).signum();
    assert!(beam.cells.windows(2).all(|p|p[1].x-p[0].x==step));
}

#[test]
fn beckon_round_trip_hides_only_the_pulled_actor_on_return() {
    for cast_by_player in [true,false] {
        let (mut r, cmd, coords)=beckon_scene(cast_by_player);
        let out=engine::dispatch(&mut r,&cmd).unwrap();
        assert_beckon_direction(&out.events, coords);
    }
}

#[test]
#[ignore = "explicit Beckoning animation review fixtures"]
fn export_beckon_round_trip() {
    for cast_by_player in [true,false] {
        let (mut r, cmd, coords)=beckon_scene(cast_by_player);
        let before=r.clone();
        let observer=events::PresentationObserver::install(|r|std::rc::Rc::new(r.clone()));
        let out=engine::dispatch(&mut r,&cmd).unwrap();
        assert_beckon_direction(&out.events, coords);
        export(if cast_by_player {"avatar-beckons-rat"} else {"tricorn-beckons-avatar"},
            &before,&r,out.events,observer.take());
    }
}

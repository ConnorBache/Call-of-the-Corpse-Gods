use corpse_core::{Run,engine,population,revision50_combat as c};
const CASTERS:[&str;12]=["witch","hemogoblin-bloodmancer","dryad","horn-1","horn-2","horn-3","horn-4","horn-5","horn-6","horn-7","saidribee-queen","valta"];
fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.player.x=12.;r.player.y=14.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.energy=100.;r.next_id=1000.;r.actors_at_mut(r.depth).clear();let f=r.floor_mut().unwrap();
    f.tiles.fill("floor".into());f.doors=Default::default();f.environment=Default::default();f.razorleaves=Default::default();r
}
#[test] fn every_approved_kind_is_tagged_only_on_revision50_births() {
    for kind in CASTERS {
        let old=population::create_actor_revision(kind,50.,13.,14.,false,46.).unwrap();
        assert!(!c::magickal_caster(&old),"{kind}");
        let mut current=population::create_actor_revision(kind,50.,13.,14.,false,50.).unwrap();
        assert!(c::magickal_caster(&current),"{kind}");
        c::mark_new_caster(&mut current,kind,50.);
        assert_eq!(current.abilities.iter().filter(|id|id.as_str()==c::MAGICKAL_CASTER).count(),1);
        current.abilities.retain(|id|id!=c::MAGICKAL_CASTER);assert_eq!(current,old,"birth profile changed: {kind}");
    }
    for kind in population::catalog().actors.keys().filter(|k|!CASTERS.contains(&k.as_str())) {
        let current=population::create_actor_revision(kind,50.,13.,14.,false,50.).unwrap();
        assert!(!c::magickal_caster(&current),"unapproved caster: {kind}");
    }
    for kind in ["night-crawler","skin-walker"] {assert!(!c::magickal_caster(&population::create_actor_revision(kind,50.,13.,14.,false,50.).unwrap()));}
}
#[test] fn ordinary_population_births_and_nest_queen_use_explicit_revision() {
    let mut r=room(50.);let mut next=1000.;let mut rng=population::population_random("caster-births");
    for kind in CASTERS {
        let a=population::create_population_actor_revision(&mut next,r.floor_mut().unwrap(),kind,100,false,&mut rng,50.).unwrap();
        assert!(c::magickal_caster(&a),"{kind}");
    }
    r.floor_mut().unwrap().pocket_biomes=vec![corpse_core::pocket_biomes::PocketBiome{
        theme:"saidribee-nest".into(),queen_cell:Some(100),honey_cell:Some(101),
        inhabitants:(102..108).collect(),..Default::default()}].into();
    let mut f=r.floor().unwrap().clone();let mut old_floor=f.clone();let(mut actors,mut ground,mut next)=(vec![],vec![],1000.);
    corpse_core::pocket_biomes::populate_revision(&mut f,&mut actors,&mut ground,&mut next,50.).unwrap();
    let(mut old,mut old_ground,mut old_next)=(vec![],vec![],1000.);
    corpse_core::pocket_biomes::populate(&mut old_floor,&mut old,&mut old_ground,&mut old_next).unwrap();
    assert!(c::magickal_caster(&actors[0]));assert!(actors[1..].iter().all(|a|!c::magickal_caster(a)));
    assert!(actors[1..].iter().all(|a|a.color==corpse_core::revision50_creature_identity::SAIDRIBEE_YELLOW));
    for a in &mut old {corpse_core::revision50_creature_identity::apply_actor(a);}
    actors[0].abilities.retain(|id|id!=c::MAGICKAL_CASTER);assert_eq!(actors,old);assert_eq!(ground,old_ground);assert_eq!(next,old_next);
}
#[test] fn summons_keep_noncasters_and_existing_newborn_profiles() {
    for kind in ["witch","saidribee-queen"] {
        let summon=|revision| {
            let mut r=room(revision);let mut source=population::create_actor_revision(kind,50.,14.,14.,false,revision).unwrap();
            source.energy=100.;source.state="hunting".into();r.actors_at_mut(r.depth).push(source);
            let player=r.player.id;
            let cast=if kind=="witch" {corpse_core::witch::summon(&mut r,50.,player)} else {corpse_core::saidribee_queen::summon(&mut r,50.,player)};
            assert!(cast.unwrap(),"{kind}");let child=engine::actor(&r,1000.).unwrap();assert!(!c::magickal_caster(child));
            (child.clone(),r.rng)
        };
        let current=summon(50.);let mut old=summon(46.);
        if kind=="saidribee-queen" {assert_eq!(current.0.color,corpse_core::revision50_creature_identity::SAIDRIBEE_YELLOW);}
        corpse_core::revision50_creature_identity::apply_actor(&mut old.0);
        assert_eq!(current,old,"noncosmetic birth profile or RNG changed: {kind}");
    }
}
#[test] fn authored_trait_drives_telepathy_and_landed_terminal_strike() {
    for kind in CASTERS {
        let mut r=room(50.);r.player.abilities.push(c::WITCH_HATRED.into());r.player.stats.accuracy=1000.;
        let mut target=population::create_actor_revision(kind,50.,13.,14.,false,50.).unwrap();
        target.state="hunting".into();target.hp=10000.;target.max_hp=10000.;target.stats.dr=100000.;target.stats.dodge=0.;
        target.conditions.insert("invisible".into(),10.);r.actors_at_mut(r.depth).push(target);
        assert!(corpse_core::vision::visible_actor(&r,engine::actor(&r,50.).unwrap()),"{kind}");
        assert!(!corpse_core::vision::directly_visible_actor(&r,engine::actor(&r,50.).unwrap()),"{kind}");
        engine::attack(&mut r,0.,50.,1.).unwrap();
        assert!(engine::actor(&r,50.).is_none_or(|a|a.dead.value_or(false)||a.hp<=0.),"{kind}");assert_eq!(r.player.energy,100.);
    }
}

use super::*;
use corpse_core::{Run, population};

fn actor(kind: &str) -> Actor {
    population::create_actor_revision(kind, 900., 12., 10., false, 52.).unwrap()
}
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into(); r.player.x=10.; r.player.y=10.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear(); r.inventory.clear();
    r.player.stats.perception=100.; r.actors_at_mut(r.depth).clear();
    let f=r.floor_mut().unwrap(); let w=f.width(); let h=f.height();
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x]="floor".into(); } }
    f.lights=vec![100.;w*h]; f.seen=vec![true;w*h];
    f.environment=corpse_core::Field::Missing; f.doors=corpse_core::Field::Missing;
    r
}
#[test]
fn paid_species_are_not_inferred_from_default_pools_or_free_specials() {
    for kind in ["bombardier", "hemogoblin-bloodmancer", "witch", "dryad", "horn-1",
        "horn-3", "horn-5", "horn-7", "river-drake", "valta", "moonspawn", "dragon", "saidribee-queen"] {
        let mut a=actor(kind); let before=a.clone();
        assert!(energy_capable(52., &a), "{kind}"); assert_eq!(a,before);
        a.energy=0.; a.state="sleeping".into(); a.conditions.insert("silenced".into(),5.);
        a.conditions.insert("paralyzed".into(),3.);
        assert!(energy_capable(52., &a), "temporarily unavailable {kind}");
    }
    for kind in ["rat", "cave-bat", "bear", "dire-bear", "tiger", "horn-2", "horn-4",
        "horn-6", "vampire", "hemogoblin-mech", "hemogoblin-inventor", "chimera", "phoenix",
        "troll", "banshee", "maulchild", "kelpie", "bastidion", "sphinx"] {
        let mut a=actor(kind); a.energy=10000.; a.stats.max_energy=10000.;
        assert!(!energy_capable(52., &a), "unused pool {kind}");
    }
    assert!(!energy_capable(51., &actor("dragon")));
    assert!(!energy_capable(32., &actor("saidribee-queen")));
    for a in &population::catalog().zillennia { assert!(energy_capable(52., a)); }
}
#[test]
fn pale_costs_and_erased_or_acquired_traits_follow_executable_branches() {
    for role in ["Breather", "Biter", "Spitter", "Birther", "Screamer"] {
        let mut a=actor("rat"); a.name=format!("Pale {role}"); a.creature_kind=corpse_core::Field::Missing;
        a.conditions.clear(); a.abilities.clear();
        assert!(energy_capable(52., &a), "legacy inferred {role}");
        corpse_core::enemy_traits::initialize_enemy_traits_revision(&mut a,52.);
        assert!(energy_capable(52., &a));
        a.abilities.clear(); assert!(!energy_capable(52., &a), "erased {role}");
    }
    let mut a=actor("rat"); a.abilities.push("enemy:pale:double-birth".into());
    assert!(!energy_capable(52., &a));
    a.abilities.push("enemy:pale:birth".into()); assert!(energy_capable(52., &a));
    a.abilities=vec!["enemy:beast:water-jet".into(), "enemy:beast:hold-only".into()];
    assert!(!energy_capable(52., &a));
}
#[test]
fn copied_active_powers_and_passive_hanged_count_but_unused_player_actions_do_not() {
    let mut r=room(); r.player.abilities=vec![corpse_core::hanged::ID.into(), "numo-mind-read".into()];
    r.player.active=vec!["numo-mind-read".into()];
    let copy=corpse_core::mini_me::preview(&r,12.,10.);
    assert!(!energy_capable(52., &copy), "R50+ Mini-Me copies stats, not powers");
    let mut a=actor("rat");
    a.abilities=vec!["grisha-boundless-hunger".into(), "numo-mind-slave".into(), "ursa-sleep-walk".into()];
    assert!(!energy_capable(52., &a));
    a.abilities.push(corpse_core::hanged::ID.into());
    assert!(energy_capable(52., &a)); assert!(!energy_capable(51., &a));
    a.abilities.clear();
    for id in ["numo-mind-read", "anavani-nova", "pelian-never-surrender", "grisha-erase-metaphysical",
        "pelian-never-without-a-fight", "shrail-rage-at-the-dying-of-the-light", "shrail-execute", "anavani-commandment"] {
        a.active=vec![id.into()]; assert!(energy_capable(52., &a), "{id}");
    }
    a.active=vec!["shrail-execute".into()]; assert!(!energy_capable(49., &a));
    let mut vampire=actor("vampire"); vampire.conditions.insert("monsterVampireBat".into(),1.);
    assert!(!energy_capable(52., &vampire), "free transformation");
}
#[test]
fn visible_resources_and_comma_conditions_are_read_only_and_consistent() {
    let mut r=room();
    let mut bat=actor("cave-bat"); bat.id=901.; bat.x=13.; bat.state="sleeping".into();
    let mut witch=actor("witch"); witch.id=902.; witch.x=14.; witch.energy=0.;
    *r.actors_at_mut(r.depth)=vec![actor("rat"),bat,witch];
    let before=serde_json::to_string(&r).unwrap(); let view=crate::observe::observe(&r);
    for (id,uses) in [(900.,false),(901.,false),(902.,true)] {
        let a=view.creatures.iter().find(|a|a.id==id).unwrap(); assert_eq!(a.uses_energy,uses);
        let report=crate::inspection_ui::observe_tile(&r, a.id as usize-888,10).creature.unwrap();
        assert_eq!(report.resources.contains("Energy"),uses);
        assert_eq!(report.hover_resources.iter().any(|r|r.label=="Energy"),uses);
        if id==901. { assert!(report.conditions.starts_with("Sleeping, Flying"),"{}",report.conditions); }
    }
    assert_eq!(serde_json::to_string(&r).unwrap(),before);
    assert_eq!(crate::statuses_ui::summary(&[]), "");
    let states=vec![crate::statuses_ui::Status{name:"Sleeping".into(),duration:None},
        crate::statuses_ui::Status{name:"Flying".into(),duration:None}];
    assert_eq!(crate::statuses_ui::summary(&states),"Sleeping, Flying");
}
#[test]
fn hidden_remembered_sensed_and_projected_bodies_keep_existing_admission() {
    let mut r=room(); let mut hidden=actor("witch"); hidden.x=25.;
    r.actors_at_mut(r.depth).push(hidden); let w=r.floor().unwrap().width(); let h=r.floor().unwrap().height();
    for y in 1..h-1 {r.floor_mut().unwrap().tiles[y*w+20]="wall".into();}
    r.player.stats.perception=0.;
    let report=crate::inspection_ui::observe_tile(&r,25,10);
    assert!(report.creature.is_none()); assert!(crate::observe::observe(&r).creatures.is_empty());
    r.actors_at_mut(r.depth)[0].energy=23.; r.actors_at_mut(r.depth)[0].abilities.clear();
    assert_eq!(crate::inspection_ui::observe_tile(&r,25,10),report);
    r.player.conditions.insert("telepathic".into(),10.);
    let row=crate::observe::observe(&r).creatures.into_iter().find(|a|a.id==900.).unwrap();
    assert_eq!(row.state,"sensed"); assert!(!row.uses_energy, "erased powers stay erased when sensed");
    let mut original=actor("witch"); original.active.push("anavani-nova".into());
    let shown=crate::creature_presentation::project_kind(&r,&original,"rat").unwrap();
    assert!(!energy_capable(52., &shown), "appearance never leaks hidden caster powers");
    let shown=crate::creature_presentation::project_kind(&r,&actor("rat"),"witch").unwrap();
    assert!(energy_capable(52., &shown));
    r.player.conditions.insert("hallucinating".into(),10.);
    let view=crate::observe::observe(&r);
    for row in &view.creatures {
        if let Some(variants)=&row.hallucination { for (phase,variant) in variants.iter().enumerate() {
            let a=&r.actors_at(r.depth)[0];
            let shown=crate::creature_presentation::project_kind(&r,a,crate::hallucination_ui::kind(&r,a,phase)).unwrap();
            assert_eq!(variant.uses_energy,energy_capable(52.,&shown));
        } }
    }
}
#[test]
fn dryad_capability_is_backed_by_real_paid_healing() {
    let mut r=room(); let mut dryad=actor("dryad"); dryad.state="ally".into(); dryad.energy=100.;
    r.player.hp=(r.player.max_hp-1.).max(1.); r.actors_at_mut(r.depth).push(dryad);
    assert!(energy_capable(52., &r.actors_at(r.depth)[0]));
    assert!(corpse_core::monster_spells::heal_others(&mut r,900.).unwrap());
    assert_eq!(r.actors_at(r.depth)[0].energy,95.);
}
fn capture(p:&crate::render::Painter, label:&str) {
    let Ok(dir)=std::env::var("CORPSE_MONSTER_STATUS_REVIEW") else {return;};
    std::fs::create_dir_all(&dir).unwrap();
    let file=std::fs::File::create(format!("{dir}/{label}.png")).unwrap();
    let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}
#[test]
fn native_status_capture_and_dialog_rows_use_the_same_wrapping() {
    let mut r=room();
    let mut bat=actor("cave-bat"); bat.id=901.;bat.x=13.;bat.state="sleeping".into();
    let mut witch=actor("witch");witch.id=902.;witch.x=14.;
    *r.actors_at_mut(r.depth)=vec![bat,witch]; let mut view=crate::observe::observe(&r);
    for (w,h) in [(1280,800),(960,720),(1920,1080)] {
        let mut p=crate::render::Painter::new(); p.resize(w,h); p.set_responsive_canvas(true);
        p.game_at(&view,"",false,0.);capture(&p,&format!("monster-status-rail-{w}x{h}"));
        p.creatures(&view,None,0);capture(&p,&format!("monster-status-dialog-{w}x{h}"));
    }
    let mut p=crate::render::Painter::new();
    view.creatures[0].statuses=(0..22).map(|i|crate::statuses_ui::Status {
        name:format!("Condition {i} with a long but complete description"),duration:Some(i as f64+1.)}).collect();
    p.creatures(&view,None,0);
    assert_eq!(crate::render::creature_at(&p,&view,0,361,210),Some(view.creatures[0].id));
    // A long card occupies the visible sheet; it must not select the next row.
    assert_eq!(crate::render::creature_at(&p,&view,0,361,500),Some(view.creatures[0].id));
    capture(&p,"monster-status-long-dialog");
    let mut manual=crate::manual::Manual::new();manual.preview_all();
    for id in ["rat","witch","dragon"] {assert!(manual.select_id(id));manual.paint_at(&mut p,0.);capture(&p,&format!("monster-status-bestiary-{id}"));}
}

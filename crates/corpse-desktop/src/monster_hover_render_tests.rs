//! Isolated playable states exercised via the normal hover observation and Painter.
use super::*;
use corpse_core::{Actor, Field, Floor, Run, population};
use std::{collections::BTreeSet, sync::Arc};

fn room() -> Run {
    let (w, h) = (79, 29);
    let mut r = corpse_core::startup::new_run_shell_revision("0-1-2:enemy-hover-qa", "Avatar", false, 52.).unwrap();
    r.player.x = 10.;
    r.player.y = 10.;
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    r.inventory.clear();
    r.actors.clear();
    r.depth = 1.;
    r.tick = 0.;
    r.banshee_shriek_from = 0usize.into();
    r.death_gas_rules_from = 0usize.into();
    r.gas_rules_from = 0usize.into();
    r.screamer_range_from = 0usize.into();
    r.floors = vec![Arc::new(Floor {
        depth: 1., width: (w as f64).into(), height: (h as f64).into(),
        tiles: vec!["floor".into(); w*h], lights: vec![100.; w*h],
        seen: vec![true; w*h], domain: "Dungeon".into(), ..Default::default()
    })];
    corpse_core::lifecycle::floor_entered(&mut r);
    r
}
fn spawn(r: &mut Run, kind: &str, x: usize) -> Actor {
    let mut a = if kind == "flesh-administrator" {
        corpse_core::finale::administrator_revision(900., x as f64, 10., 52.)
    } else {
        population::create_actor_revision(kind, 900., x as f64, 10., false, 52.).unwrap()
    };
    corpse_core::balance52::apply_new_actor_kind(&mut a, kind, r.depth, 52.);
    a.state = "hunting".into();
    *r.actors_at_mut(r.depth) = vec![a.clone()];
    a
}
fn observed(r: &Run, x: usize) -> Report {
    let before = r.clone();
    let report = observe_tile(r, x, 10);
    assert_eq!(r, &before, "hover never changes state");
    assert!(report.creature.as_ref().is_some_and(|c| c.creature_card.is_some()), "{x}: {}", report.title);
    report
}
fn copy(report: &Report) -> String {
    report.creature.as_ref().unwrap().creature_card.as_ref().unwrap().mechanics.join(" ")
}

#[test]
fn live_monster_hover_countdown_recovery_disguise_and_matchup() {
    let mut r = room();
    let banshee = spawn(&mut r,"banshee",12);
    r.tick = 27.;
    r.player.hp = 40.;
    let card = observed(&r,12);
    let text = copy(&card);
    assert!(text.contains("Shriek in 3t"), "{text}");
    assert!(text.contains("300% of your current HP"), "{text}");
    assert_eq!(text.matches("Shriek in").count(), 1);
    r.player.hp = 100.;
    assert!(copy(&observed(&r,12)).contains("150% of your current HP"));
    assert_eq!(card.creature.unwrap().description, flavor(&banshee));
    r.tick = 0.;
    let id = r.player.id;

    // Enter the charged and recovery states through the real AI callbacks.
    spawn(&mut r,"valta",15);
    r.actors_at_mut(r.depth)[0].energy = 100.;
    assert!(corpse_core::monster_moon::special_action(&mut r,900.,id,false).unwrap());
    let v = observed(&r,15);
    let text = copy(&v);
    assert!(text.contains("Moonbeam charged"), "{text}");
    assert!(text.contains("16 tiles") && text.contains("HP/hit"), "{text}");
    assert!(!text.contains("20 NRG"));
    assert!(!v.creature.unwrap().creature_card.unwrap().conditions.to_lowercase().contains("moonbeam"));

    spawn(&mut r,"pale:Spitter",14);
    r.actors_at_mut(r.depth)[0].energy = 100.;
    assert!(corpse_core::ai::enemy_action_options(&mut r,900.,id,false).unwrap());
    let s = observed(&r,14);
    let text = copy(&s);
    assert_eq!(text.matches("recovering").count(), 1);
    assert!(text.contains("12 tiles") && text.contains("Caustic Gas at impact"), "{text}");
    assert!(!s.creature.unwrap().creature_card.unwrap().conditions.to_lowercase().contains("recovery"));

    spawn(&mut r,"skin-walker",14);
    let disguised = observed(&r,14);
    assert_ne!(disguised.title, "Skin Walker");
    assert!(!copy(&disguised).contains("Mimics"));
    r.player.x = 12.;
    let revealed = observed(&r,14);
    assert_eq!(revealed.title, "Skin Walker");
    assert!(copy(&revealed).contains("Mimics weaker creatures"));

    spawn(&mut r,"flesh-administrator",13);
    let a = observed(&r,13);
    let c = a.creature.unwrap();
    assert!(c.creature_card.as_ref().unwrap().administrator);
    assert!(c.creature_card.unwrap().mechanics.iter().any(|s|s=="Killing her starts the collapse."));
}

#[test]
fn live_monster_hover_roster_and_damage_follow_through() {
    let mut r = room();
    let keys: BTreeSet<_> = population::catalog().actors.keys().cloned()
        .chain(corpse_core::new_creatures::templates().iter().filter_map(|a| a.creature_kind.as_ref().cloned())).collect();
    assert!(keys.len() > 60);
    for key in keys {
        let a = spawn(&mut r,&key,11);
        let c = creature_presented(&r,&a,None,false);
        assert_eq!(c.description, flavor(&a), "{key}");
        let rows = c.creature_card.unwrap().mechanics;
        assert!(rows.iter().all(|s| !s.contains("undescribed ability") && !s.contains("This is not Haste")), "{key}: {rows:?}");
    }
    let mut quality = BTreeSet::new();
    for depth in 1..=40 {
        let mut a = population::zillennia_revision(depth,52.).unwrap();
        let q = a.zillennia_quality.as_ref().unwrap().clone();
        if !quality.insert(q.clone()) { continue; }
        a.x=11.; a.y=10.; a.id=900.;
        corpse_core::balance52::apply_new_actor_kind(&mut a,&format!("zillennia:{q}"),depth as f64,52.);
        *r.actors_at_mut(1.)=vec![a.clone()];
        r.player.conditions.insert("telepathic".into(), 10.);
        let report = observed(&r,11);
        let damage = corpse_core::monster_ruin::ruin_damage_revision(&a,52.);
        let expectation = crate::creature_abilities_ui::hover::percent(
            combat::damage_packet(&r, &r.player, damage, "attack", false), r.player.max_hp);
        let text = copy(&report);
        assert!(text.contains(&format!("Ruin Pulse: {expectation}")) && text.contains("Silences"), "{q}: {text}");
        assert_eq!(report.creature.unwrap().description, flavor(&a));
    }
    assert_eq!(quality.len(),10);
    r.player.max_hp=100.;
    for (kind,n) in [("needleling",5),("long-saidri",10),("saidri-guardian",20)] {
        spawn(&mut r,kind,11);
        let text = copy(&observed(&r,11));
        assert!(text.contains(&format!("{n}% HP over 5t from Envenomed")), "{kind}: {text}");
    }
    for (kind, needles) in [
        ("pale:Greater Breather", &["Death Gas", "7% max HP/turn", "Hallucinating 5t", "double caustic"][..]),
        ("pale:Screamer", &["10 tiles", "+5 Damage", "+20 Perception", "Rallies"][..]),
        ("bombardier", &["5 tiles", "50% max HP", "ignites", "all NRG"][..]),
        ("samara", &["5 tiles", "2% max HP at turn end", "Burning lasts"][..]),
        ("witch", &["Poison 40% HP/20t", "Night Crawler or Skin Walker", "10t cooldown"][..]),
    ] {
        spawn(&mut r,kind,11);
        let text = copy(&observed(&r,11));
        for needle in needles { assert!(text.contains(needle), "{kind} expected {needle}: {text}"); }
    }
}

#[test]
fn live_monster_hover_native_capture() {
    let mut r = room();
    let mut painter = Painter::new();
    let out = std::env::var_os("COG_MONSTER_HOVER_CAPTURE").map(std::path::PathBuf::from);
    if let Some(path) = &out { std::fs::create_dir_all(path).unwrap(); }
    for (kind,name,x) in [
        ("banshee","banshee",12), ("valta","valta",15), ("pale:Spitter","spitter",14),
        ("witch","witch",13), ("bastidion","fungus",12), ("pale:Greater Breather","greater-gasser",12),
        ("pale:Greater Screamer","screamer",12), ("needleling","needleling",12), ("samara","samara",12),
        ("bombardier","bombardier",12), ("flesh-administrator","administrator",13),
    ] {
        r.tick = 0.;
        r.player.energy = 100.;
        r.player.hp = 100.;
        r.player.max_hp = 100.;
        r.player.conditions.clear();
        spawn(&mut r,kind,x);
        let id = r.player.id;
        if name=="banshee" { r.tick=27.; }
        if name=="valta" {
            r.actors_at_mut(r.depth)[0].energy = 100.;
            assert!(corpse_core::monster_moon::special_action(&mut r,900.,id,false).unwrap());
        }
        if name=="spitter" {
            r.actors_at_mut(r.depth)[0].energy = 100.;
            assert!(corpse_core::ai::enemy_action_options(&mut r,900.,id,false).unwrap());
        }
        let report = observed(&r,x);
        let view = crate::observe::observe(&r);
        for (width,height) in [(1280,800),(1024,600)] {
            painter.resize(width,height);
            painter.set_responsive_canvas(true);
            painter.game_at(&view, "", false, 0.);
            let baseline = painter.pixels.clone();
            for before in [true,false] {
                painter.pixels.copy_from_slice(&baseline);
                let mut shot = report.clone();
                if before { shot.creature.as_mut().unwrap().creature_card=None; }
                paint_hover(&shot,&mut painter);
                assert_ne!(painter.pixels, baseline, "{name}: hover renders");
                if let Some(dir)=&out {
                    save_card(&painter, &baseline, dir, &format!("{name}-{width}-{}", if before {"before"} else {"after"}));
                }
            }
        }
    }
}

fn save_card(p: &Painter, original: &[u8], path: &std::path::Path, name: &str) {
    use std::io::Write;
    let (width,height)=(p.width,p.height);
    let (mut left,mut top,mut right,mut bottom) = (width,height,0,0);
    for i in 0..width*height {
        let offset=i*4;
        if p.pixels[offset..offset+4] != original[offset..offset+4] {
            left=left.min(i%width);right=right.max(i%width+1);
            top=top.min(i/width);bottom=bottom.max(i/width+1);
        }
    }
    let mut f=std::io::BufWriter::new(std::fs::File::create(path.join(format!("{name}.ppm"))).unwrap());
    write!(f,"P6\n{} {}\n255\n",right-left,bottom-top).unwrap();
    for y in top..bottom {
        for x in left..right {
            let i=(y*width+x)*4;
            f.write_all(&p.pixels[i..i+3]).unwrap();
        }
    }
}

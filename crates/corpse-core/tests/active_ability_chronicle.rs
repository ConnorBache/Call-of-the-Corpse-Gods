use corpse_core::{Command, Field, Run, engine, population};

fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();
    r.player.x=10.;r.player.y=10.;r.player.stats.perception=300.;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.phase="normal".to_string().into();r.offers.clear();
    r.actors_at_mut(r.depth).clear();r.ground_at_mut(r.depth).clear();r.inventory.clear();
    r.player.ability_reserves=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.ability_order=Field::Missing;
    let f=r.floor_mut().unwrap();
    f.tiles.fill("floor".into());f.lights.fill(100.);
    f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;
    f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;
    r
}

#[test]
fn committed_free_toggle_logs_activation_and_deactivation() {
    let mut r=room();
    let id="shrail-execute";
    r.player.abilities.push(id.into());
    let cmd=Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    for (active, message) in [(true,"You activate Execute."),(false,"You deactivate Execute.")] {
        assert!(!engine::dispatch(&mut r,&cmd).unwrap().took_turn);
        assert_eq!(r.player.active.iter().any(|s|s==id),active);
        assert!(r.log.iter().any(|s|s==message));
        assert!(r.log_archive.as_ref().unwrap().messages.iter().any(|s|s==message));
    }
}

#[test]
fn committed_enemy_cast_has_its_own_line_and_failed_cast_does_not() {
    for (kind,label) in [("horn-1","Hornlight"),("river-drake","Water Jet")] {
        let mut r=room();
        let id=900.;
        let mut a=population::create_actor_revision(kind,id,14.,10.,false,52.).unwrap();
        a.state="hunting".into();
        a.energy=0.;
        r.actors_at_mut(r.depth).push(a);
        let player=r.player.id;
        let act=|r:&mut Run| {
            if kind=="horn-1"{corpse_core::monster_horn::special_action(r,id,player)}
            else{corpse_core::monster_grab::water_jet(r,id,player)}
        };
        let success=format!(" uses {label}.");
        assert!(!act(&mut r).unwrap());
        assert!(!r.log.iter().any(|m|m.ends_with(&success)));
        engine::actor_mut(&mut r,id).unwrap().energy=100.;
        assert!(act(&mut r).unwrap());
        assert_eq!(r.log.iter().filter(|m|m.ends_with(&success)).count(),1);
        assert!(r.log_archive.as_ref().unwrap().messages.iter().any(|m|m.ends_with(&success)));
    }
}

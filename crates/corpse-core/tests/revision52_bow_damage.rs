//! R52 bows use the innate Greatsword damage base, preserving ranged behavior.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, combat, engine,
    projectiles, revision50_player_attack as melee, rng, storage, temporal::Snapshot};
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.rng = 123456.; r.next_id = 1000.;
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 1000.; r.player.max_hp = 1000.;
    r.player.base_max_hp = 1000.0.into(); r.player.energy = 100.;
    r.player.stats = Stats { offense: 20., accuracy: 1000., perception: 100., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.inventory.clear();
    // Keep the required empty actor blocks for every generated fixture floor.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into();
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.environment = Field::Missing; f.doors = Field::Missing; f.non_euclidean = Field::Missing;
    f.portals = Field::Missing; f.currents = Field::Missing;
    r.actors_at_mut(r.depth).push(Actor { id: 901., name: "Target".into(),
        x: 11., y: 10., hp: 1000., max_hp: 1000., base_max_hp: 1000.0.into(),
        state: "hunting".into(), heartless: true.into(),
        stats: Stats { accuracy: 1000., ..Default::default() }, ..Default::default() });
    r
}
fn bow() -> Item {
    Item { id: 950., name: "Bow".into(), kind: "weapon".into(), damage: 15.0.into(),
        enchantment: 3.0.into(), offense_multiplier: 0.5.into(),
        ranged_weapon: "bow".to_string().into(), hands: 2.0.into(),
        equipped: "hand1".to_string().into(), ..Default::default() }
}
fn variance(r: &Run) -> (f64, f64) {
    let mut predicted = r.clone(); rng::random(&mut predicted);
    let factor = 0.75 + rng::random(&mut predicted) * 0.5;
    (factor, predicted.rng)
}
fn hp(r: &Run) -> f64 { engine::actor(r, 901.).unwrap().hp }


#[test]
fn both_bows_match_live_melee_and_ignore_legacy_item_damage_fields() {
    for name in ["Bow", "Composite bow"] {
        let mut item=corpse_core::equipment::make_equipment(name, 950.).unwrap();
        for offense in [-5.,0.,1.5,20.,100.] {
            for enchantment in [-4.,0.,7.] {
                item.enchantment=enchantment.into();
                let mut r=room(52.); r.player.stats.offense=offense;
                assert_eq!(engine::player_weapon_strike_base_for_revision(&item,offense,52.),
                    melee::with_offense_for_run(&r,offense));
                // A stale save's item values cannot reintroduce the obsolete formula.
                item.damage=999.0.into(); item.offense_multiplier=99.0.into();
                assert_eq!(engine::player_weapon_strike_base(&r,&item),melee::base(&r));
            }
        }
    }
}

#[test]
fn committed_shots_apply_bow_tier_after_melee_damage_armor_and_rounding() {
    for name in ["Bow", "Composite bow"] { for phase in ["normal","haste"] {
        for offense in [0.,20.] { for dr in [0.,10.,30.5,30.7,80.] { for paralyzed in [false,true] {
            let mut shot=room(52.); shot.phase=phase.to_string().into();
            shot.player.stats.offense=offense; shot.player.conditions.insert("lucky".into(),1.);
            engine::actor_mut(&mut shot,901.).unwrap().stats.dr=dr;
            if paralyzed { engine::actor_mut(&mut shot,901.).unwrap().conditions.insert("paralyzed".into(),1.); }
            let mut item=corpse_core::equipment::make_equipment(name,950.).unwrap();
            item.equipped="hand1".to_string().into(); item.enchantment=7.0.into();
            shot.inventory.push(item);
            shot.inventory.push(Item{id:951.,name:"Arrow".into(),kind:"ammo".into(),
                effect:"arrows".to_string().into(),quantity:2.0.into(),..Default::default()});
            let mut sword=shot.clone(); let (v,_)=variance(&shot);
            let scale=if name=="Composite bow" {2.} else {1.};
            let expected=scale*combat::ordinary_strike_damage_for_run(&shot,18.9+offense,v,combat::integer_stat(dr),false,paralyzed);
            assert_eq!(projectiles::command(&mut shot,&Command::Shoot{id:950.,x:11.,y:10.}).unwrap(),Some(true));
            engine::player_attack(&mut sword,901.).unwrap();
            assert_eq!(1000.-hp(&shot),expected,"{name}, {phase}, offense {offense}, DR {dr}, paralysis {paralyzed}");
            assert_eq!(1000.-hp(&shot),scale*(1000.-hp(&sword)));
            // Doubling the rounded packet preserves zero hits and RNG consumption.
            assert_eq!(shot.rng,sword.rng);
            assert_eq!(shot.inventory.iter().find(|i|i.id==951.).unwrap().quantity.value_or(0.),1.);
        }}}
    }}
}

#[test]
fn save_and_rewind_keep_parity_without_rewriting_inventory() {
    let mut r=room(52.); let item=bow(); r.inventory.push(item.clone());
    let snapshot=Snapshot::capture(&r);
    let loaded=storage::deserialize_compatible(&storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(loaded.inventory,r.inventory);
    assert_eq!(engine::player_weapon_strike_base(&loaded,&item),melee::base(&loaded));
    r.player.stats.offense=100.; snapshot.restore(&mut r).unwrap();
    assert_eq!(engine::player_weapon_strike_base(&r,&item),melee::base(&r));
    for revision in [50.,51.] {
        assert_eq!(engine::player_weapon_strike_base_for_revision(&item,20.,revision),
            if revision==50. {28.} else {23.5});
    }
}

#[test]
fn committed_bows_combine_damage_parity_with_normal_accuracy() {
    for name in ["Bow", "Composite bow"] { for range in [1.,4.,10.] {
        for seed in [1.,1973.,8192.,12345.,50000.,100000.,123456789.] {
            let mut r=room(52.); r.rng=seed; r.player.stats.accuracy=20.;
            let target=engine::actor_mut(&mut r,901.).unwrap();
            target.x=10.+range; target.stats.dodge=20.;
            let mut bow=corpse_core::equipment::make_equipment(name,950.).unwrap();
            bow.equipped="hand1".to_string().into(); r.inventory.push(bow);
            r.inventory.push(Item{id:951.,kind:"ammo".into(),effect:"arrows".to_string().into(),
                quantity:2.0.into(),..Default::default()});
            let chance=0.5; // Current normal Accuracy/(2*Dodge), independent of distance.
            let mut forecast=r.clone(); let hit=rng::random(&mut forecast)<chance;
            assert_eq!(projectiles::command(&mut r,&Command::Shoot{id:950.,x:10.+range,y:10.}).unwrap(),Some(true));
            assert_eq!(hp(&r)<1000.,hit,"{name}, range {range}, seed {seed}");
        }
    }}
}

#[test]
fn composite_doubles_silent_strike_modifier_but_not_immunity_or_overkill() {
    for case in ["silent", "immune", "overkill"] {
        let mut r=room(52.);r.player.conditions.insert("lucky".into(),1.);
        r.player.abilities.push("dawel-silent-strike".into());
        let target=engine::actor_mut(&mut r,901.).unwrap();target.stats.dr=10.;
        if case=="immune" {target.abilities.push("enemy:beast:sphinx".into());}
        if case=="overkill" {target.hp=1.;}
        let bow=corpse_core::equipment::make_equipment("Composite bow",950.).unwrap();
        r.inventory.push(bow);r.inventory.push(Item{id:951.,kind:"ammo".into(),
            effect:"arrows".to_string().into(),quantity:2.0.into(),..Default::default()});
        let mut sword=r.clone();let before=hp(&r);
        projectiles::command(&mut r,&Command::Shoot{id:950.,x:11.,y:10.}).unwrap();
        engine::player_attack(&mut sword,901.).unwrap();
        assert_eq!(before-hp(&r),(2.*(before-hp(&sword))).min(before),"{case}");
    }
}

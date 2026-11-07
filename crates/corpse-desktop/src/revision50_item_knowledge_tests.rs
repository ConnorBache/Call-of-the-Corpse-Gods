//! Read-only R50 item hover and dismissal coverage; no release admission.
use super::*;
fn run() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into(); r.inventory.clear(); r.actors=Default::default();
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.known_potions.clear(); r.known_potion_natures=Field::Missing;
    r
}
fn potion(effect:&str)->Item {
    Item{id:900.,kind:"potion".into(),name:"Unidentified potion".into(),glyph:"!".into(),
        effect:effect.to_string().into(),identified:false.into(),..Default::default()}
}
fn hover_first(view:&mut PackView) {
    let y=view.row_positions()[0].1-view.list_scroll+4;
    view.hover(view.layout.list.0+12,y);
}
#[test]
fn r50_item_hover_all_potions_reveal_effect_before_clicking() {
    let mut r=run(); let p=Painter::new();
    for effect in corpse_core::revision50_potions::ORDINARY.into_iter().chain(["Veridian Sap"]) {
        r.inventory=vec![potion(effect)]; let before=serde_json::to_vec(&r).unwrap();
        let mut view=PackView::observe(&r); view.configure(&p,p.frame().map.tuple()); hover_first(&mut view);
        assert!(view.selected.is_none());
        let text=view.tooltip().unwrap();
        assert!(text.contains(corpse_core::revision50_item_knowledge::potion_name(effect).unwrap()));
        assert!(text.contains(&corpse_core::revision50_item_knowledge::potion_effect_description(effect,false).unwrap()));
        assert!(!text.contains("Might be")&&!text.contains("Identified when used")&&!text.contains("Unidentified"));
        for _ in 0..3 { hover_first(&mut view); assert_eq!(view.tooltip().as_deref(),Some(text.as_str())); }
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}
#[test]
fn r50_item_hover_jewelry_hearts_tools_and_grants_keep_effect_details() {
    let mut r=run(); let p=Painter::new();
    let samples=[
        (Item{kind:"ring".into(),name:"stale ring".into(),effect:"corrosion-resistance".to_string().into(),..Default::default()}, "corrosion penalties to DR"),
        (Item{kind:"amulet".into(),name:"stale amulet".into(),effect:"lightning".to_string().into(),..Default::default()}, "50 lightning damage"),
        (Item{kind:"heart".into(),name:"Rat heart".into(),exp:2.0.into(),energy:3.0.into(),..Default::default()}, "2 blessings · 3 energy"),
        (Item{kind:"tool".into(),name:"Tree tap".into(),effect:"tree-tap".to_string().into(),..Default::default()}, "collect its one gift of sap"),
    ];
    for (mut item,fragment) in samples {item.id=900.;r.inventory=vec![item];
        let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
        assert!(view.tooltip().unwrap().contains(fragment));
    }
    r.inventory[0]=Item{id:900.,kind:"ring".into(),name:"stale ring".into(),effect:"light".to_string().into(),
        granted_ability:"shrail-execute".to_string().into(),..Default::default()};
    let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
    let text=view.tooltip().unwrap();assert!(text.contains("Ring of Light")&&text.contains("Execute"));
    assert!(text.find("Adds 10 illumination").unwrap()<text.find("Execute").unwrap());
}
#[test]
fn r50_item_hover_cancel_reopen_refresh_and_scroll_are_read_only() {
    let mut r=run();r.inventory=vec![potion("healing")];let before=serde_json::to_vec(&r).unwrap();
    let mut p=Painter::new();let base=crate::observe::observe(&r);p.game_at(&base,"",false,0.);let baseline=p.pixels.clone();
    for _ in 0..3 {
        let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
        assert!(view.tooltip().unwrap().contains("50 HP"));view.paint(&mut p);
        view.cancel_selection();assert!(view.tooltip().is_none());hover_first(&mut view);
        let row=view.row_positions()[0].1-view.list_scroll+4;
        assert_eq!(view.click(view.layout.list.0+12,row),PackAction::None);
        let throw=view.selected_group().unwrap().actions.iter().position(|b|b.label=="Throw").unwrap();
        assert_eq!(view.activate(throw),PackAction::Target(PackTarget::Throw(900.)));
        assert_eq!(view.activate(throw),PackAction::None,"repeat activation is suppressed");
        assert_eq!(view.key(Keycode::Escape,Mod::NOMOD),PackAction::Close);
        p.game_at(&base,"",false,0.);assert_eq!(p.pixels,baseline);
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
    let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
    view.scroll(-1,view.layout.list.0+12,view.layout.list.1+12);assert!(view.tooltip().is_none());
    r.inventory[0]=potion("energy");view.refresh(&r);view.configure(&p,p.frame().map.tuple());
    // Scrolling keeps its viewport across refresh; return the item into view.
    view.scroll(1,view.layout.list.0+12,view.layout.list.1+12);hover_first(&mut view);
    assert!(view.tooltip().unwrap().contains("50 energy"));assert!(!view.tooltip().unwrap().contains("50 HP"));
}
#[test]
fn r50_item_hover_legacy_unknown_potion_stays_unknown() {
    let mut r=run();r.simulation_revision=49.0.into();r.inventory=vec![potion("healing")];
    let p=Painter::new();let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
    assert_eq!(view.tooltip().unwrap(),potion_appearance(&r,"healing"));
    assert!(describe_item(&r,&r.inventory[0]).contains("Identified when used"));
}
#[test]
fn r50_item_hover_carried_jewelry_is_active_without_equipment_controls() {
    let mut r=run();
    for (kind,effect) in [("ring","light"),("amulet","lightning")] {
        for equipped in [Field::Missing,"ring1".to_string().into()] {
            let item=Item{id:900.,kind:kind.into(),name:"Old draft label".into(),effect:effect.to_string().into(),
                equipped,..Default::default()};
            let actions=item_actions(&r,&item);
            assert!(!actions.iter().any(|b|matches!(b.label.as_str(),"Wear"|"Take off"|"Equip"|"Equipped"|"Unequip"|"Ring 2")));
            assert!(!actions.iter().any(|b|matches!(b.action,ButtonAction::Command(Command::Equip{..}|Command::Unequip{..}))));
            let text=describe_item(&r,&item);
            assert_eq!(text.lines().nth(1),Some(titlecase(kind).as_str()));
            assert!(!text.contains("while carried"));
            assert!(!text.contains("Ring finger")&&!text.contains(" · Worn")&&!text.contains("Unequipped"));
        }
    }
    r.simulation_revision=49.0.into();
    let item=Item{id:900.,kind:"ring".into(),name:"Ring".into(),equipped:"ring1".to_string().into(),..Default::default()};
    assert!(describe_item(&r,&item).contains("Ring finger"));
    assert!(item_actions(&r,&item).iter().any(|b|b.label=="Equipped"));
    assert!(item_actions(&r,&item).iter().any(|b|b.label=="Unequip"));
}

#[test]
fn r51_all_ring_descriptions_are_only_names_types_and_effects() {
    let mut r=run();r.simulation_revision=51.0.into();
    let effects=[
        ("stealth","Negative illumination on your tile is multiplied by 1.2."),
        ("light","Adds 10 illumination to surrounding tiles in your line of sight."),
        ("regeneration","Doubles your natural HP regeneration."),
        ("reaping","Restores 1 energy to each amulet for every 20 direct damage you deal to enemies."),
        ("vampirism","Melee attacks heal you for 5% of the damage dealt."),
        ("transference","Melee attacks heal you for 5% of the damage dealt."),
        ("flight","Grants flight."),
        ("fire-resistance","Halves fire and burning damage."),
        ("poison-resistance","Halves poison and venom damage."),
        ("sustain-strength","Halves all temporary reductions to your stats."),
        ("willpower","Halves the duration of confusion, hallucination, and enemy curses."),
        ("protection","Grants 4 DR."),
        ("slaying","Grants +4 Damage."),
        ("ice-immunity","Prevents cold damage, cold-induced slowing, and slipping on ice."),
        ("corrosion-resistance","Halves acid, caustic gas, and ichor damage, and corrosion penalties to DR."),
    ];
    let p=Painter::new();
    for (effect,description) in effects {
        r.inventory=vec![Item{id:900.,kind:"ring".into(),effect:effect.to_string().into(),..Default::default()}];
        let before=serde_json::to_vec(&r).unwrap();
        let expected=format!("{}\nRing\n{description}",displayed_name(&r,&r.inventory[0]));
        assert_eq!(describe_item(&r,&r.inventory[0]),expected,"{effect}");
        let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
        assert_eq!(view.tooltip().as_deref(),Some(expected.as_str()),"{effect} hover");
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}

#[test]
fn r51_all_amulet_descriptions_keep_effects_costs_and_targeting() {
    let mut r=run();r.simulation_revision=51.0.into();
    let effects=[
        ("lightning",25,"Fires a piercing bolt for 50 lightning damage.","Unlimited range."),
        ("firebolt",20,"Fires a bolt for 80 fire damage, igniting creatures and flammable terrain.","Unlimited range."),
        ("poison",15,"Fires a projectile that poisons its target for 6 damage per turn for 20 turns.","Unlimited range."),
        ("iceblast",30,"Deals 30 cold damage and slows creatures for 3 turns in a 3×3 area, leaving ice.","Targets a visible area."),
        ("acid",20,"Fires a projectile that deals 40 damage and reduces the target’s DR by 10 for 6 turns.","Range: 8 tiles."),
        ("blinking",20,"Teleports you to a visible, accessible square within 10 tiles.",""),
        ("beckoning",15,"Pulls a targeted creature next to you.","Targets a visible creature within 8 tiles with a clear path to you."),
        ("discord",30,"Makes a creature and all other creatures hostile to each other for 6 turns.","Targets a visible creature within 8 tiles."),
        ("obstruction",25,"Creates a three-tile wall perpendicular to your aim for 8 turns.","Targets visible open ground along a cardinal or diagonal aim."),
        ("roots",20,"Roots a creature in place for 4 turns, dealing 3 damage each turn.","Targets a grounded, visible creature within 6 tiles. Fire, teleporting, or forced movement breaks the roots."),
        ("teleportation",50,"Teleports another creature to a random valid square on the current floor.","Targets a visible creature within 8 tiles."),
        ("floods",30,"Sends a 3×5 wave that deals 20 damage, pushes creatures up to 2 tiles, extinguishes fire, and leaves shallow water for 12 turns.",""),
    ];
    let p=Painter::new();
    for (effect,cost,description,targeting) in effects {
        let mut item=Item{id:900.,kind:"amulet".into(),effect:effect.to_string().into(),..Default::default()};
        corpse_core::revision50_amulets::initialize_item(&mut item).unwrap();
        r.inventory=vec![item];let before=serde_json::to_vec(&r).unwrap();
        let mut expected=format!("{}\nAmulet\n{description}\nAmulet energy: 50/50 · Activation: {cost} energy.",displayed_name(&r,&r.inventory[0]));
        if !targeting.is_empty() {expected.push('\n');expected.push_str(targeting);}
        assert_eq!(describe_item(&r,&r.inventory[0]),expected,"{effect}");
        let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());hover_first(&mut view);
        assert_eq!(view.tooltip().as_deref(),Some(expected.as_str()),"{effect} hover");
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}

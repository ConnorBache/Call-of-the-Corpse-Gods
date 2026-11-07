// R51 display-only identities; saved items and terrain remain unchanged.
#[cfg(test)]
mod grass_amulet_observe_tests {
    use super::*;
    use crate::appearance_profile::Profile;
    use serde_json::json;

    fn observation() -> corpse_core::vision::TerrainObservation {
        let run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut o = corpse_core::vision::observe_terrain(&run).unwrap();
        o.light_sources.clear();
        for c in &mut o.cells {
            c.current=true;c.direct=true;c.remembered=true;c.knowledge="direct".into();
            c.chance=100.;c.direct_chance=100.;c.distance=0.;c.ambient=Some(0.);c.illumination=Some(0.);
            c.tile=Some("floor".into());c.substrate=None;c.door_open=None;c.grate=false;c.bridge=false;
            c.snow=false;c.ice=false;c.puddles=false;c.hellfire=false;c.flowers=false;c.biome=None;
            c.entrance=false;c.hazards=None;
        }
        o
    }

    #[test]
    fn grass_amulet_grass_alignment_is_knowledge_safe_and_keeps_other_terrain() {
        let mut o=observation();let p=Profile::default();
        for (at,tile) in [(200,"grass"),(201,"floor"),(202,"tree"),(203,"water"),(204,"wall")] {o.cells[at].tile=Some(tile.into());}
        o.cells[205].tile=Some("grass".into());o.cells[205].door_open=Some(false);
        o.cells[206].tile=Some("grass".into());o.cells[206].snow=true;
        o.cells[207].tile=Some("grass".into());o.cells[207].flowers=true;
        o.cells[208].tile=Some("grass".into());o.cells[208].current=false;o.cells[208].direct=false;o.cells[208].knowledge="remembered".into();
        o.cells[209].tile=None;o.cells[209].current=false;o.cells[209].direct=false;
        let original=o.clone();
        let prepared=ground_compiled::prepare(&p,&o.cells[200],true);
        assert_eq!(prepared.glyph,"\"");assert_eq!(prepared.fg,[120,199,107]);
        let (cells,_,_)=terrain_frame(&o,&p,17,0.);
        let get=|at|cells.iter().find(|c|c.y*o.width+c.x==at).unwrap();
        assert!(get(200).bottom_aligned);assert!(get(208).bottom_aligned);
        assert!(get(200).fg[1]>get(200).fg[0]&&get(200).fg[1]>get(200).fg[2]);
        for at in 201..208 {assert!(!get(at).bottom_aligned,"unrelated terrain {at}");}
        assert!(!cells.iter().any(|c|c.y*o.width+c.x==209));
        let custom=p.set("terrain.grass.glyph",json!("g")).unwrap().set("terrain.grass.color",json!("#aabbcc")).unwrap();
        let prepared=ground_compiled::prepare(&custom,&o.cells[200],true);
        assert_eq!(prepared.glyph,"g");assert_eq!(prepared.fg,[170,187,204]);
        let (custom_cells,_,_)=terrain_frame(&o,&custom,17,0.);
        let grass=custom_cells.iter().find(|c|c.y*o.width+c.x==200).unwrap();
        assert_eq!(grass.glyph,"g");assert!(!grass.bottom_aligned);
        assert_eq!(format!("{o:?}"),format!("{original:?}"));
    }

    #[test]
    fn grass_amulet_item_identity_overrides_and_legacy_profiles_are_preserved() {
        let item=corpse_core::Item{kind:"amulet".into(),name:"Amulet".into(),glyph:"\"".into(),..Default::default()};
        let original=item.clone();let p=Profile::default();
        assert_eq!(item_glyph(&item),"?");
        for kind in ["Amulet","kind:amulet"] {assert_eq!(p.text(&format!("entities.items.{kind}.glyph")),"?");}
        let style=|p:&Profile|crate::entity_profile::resolve(p,"items","Amulet",Some("kind:amulet"),&item_glyph(&item),item_color(&item.kind));
        assert_eq!(style(&p).glyph,"?");
        for path in ["entities.items.kind:amulet.glyph","entities.items.Amulet.glyph","entities.defaults.glyph"] {
            let custom=p.set(path,json!("A")).unwrap();assert_eq!(style(&custom).glyph,"A");
            assert_eq!(style(&Profile::parse(&custom.json()).unwrap()).glyph,"A");
        }
        let mut legacy=p.data().clone();
        for kind in ["Amulet","kind:amulet"] {legacy["entities"]["items"][kind]["glyph"]=json!("\"");}
        assert_eq!(style(&Profile::from_value(legacy.clone()).unwrap()).glyph,"?","old inherited identities adopt the new display");
        legacy["entities"]["items"]["Amulet"]["inherited"]=json!(false);
        assert_eq!(style(&Profile::from_value(legacy).unwrap()).glyph,"\"","explicit old appearance overrides survive");
        let color_only=p.set("entities.items.Amulet.color",json!("#aabbcc")).unwrap();assert_eq!(style(&color_only).glyph,"?");
        for (kind,stored,shown) in [("ring","=","o"),("heart",";",";"),("potion","!","!"),("armor","[","["),("tool","T","T")] {
            let item=corpse_core::Item{kind:kind.into(),glyph:stored.into(),..Default::default()};assert_eq!(item_glyph(&item),shown);
        }
        assert_eq!(item,original);
    }

    #[test]
    fn grass_amulet_map_projection_uses_question_mark_without_changing_saved_item() {
        let mut run=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.actors.clear();run.ground.clear();
        let item=corpse_core::Item{id:900.,kind:"amulet".into(),name:"Amulet".into(),glyph:"\"".into(),..Default::default()};
        let (x,y)=(run.player.x+1.,run.player.y);
        run.ground.insert(run.depth.to_string(),std::sync::Arc::new(vec![corpse_core::GroundItem{x,y,item,..Default::default()}]));
        let before=run.clone();let p=Profile::default();
        for (profile,expected) in [(p.clone(),"?"),(p.set("entities.items.kind:amulet.glyph",json!("A")).unwrap(),"A")] {
            let view=observe_with_profile(&run,&profile);
            let cell=view.cells.iter().find(|c|c.x==x as usize&&c.y==y as usize&&c.entity_ink.is_some()).unwrap();
            assert_eq!(cell.glyph,expected);assert!(!cell.bottom_aligned);
        }
        assert_eq!(run,before);
    }
}

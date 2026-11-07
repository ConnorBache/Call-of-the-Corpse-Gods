#[cfg(test)]
mod compiled_ground_oracles {
    use super::*;
    use crate::appearance_profile::Profile;
    use corpse_core::vision::{ObservedCell,TerrainObservation};
    fn observation()->TerrainObservation{
        let tiles=["floor","grass","wall","water","shallow-water","tree","chasm","pit","up","down"];
        let substrates=["tiled","rock","earth","sand","ash","soot","unknown-substrate"];
        let biomes=["saidribee-nest","flower-grove","mountainous","corn-dimension"];
        let cells=(0..64).map(|i|{
            let current=i%5!=0;let direct=current&&i%7!=0;let hidden=i%11==0;
            let mut h=corpse_core::HazardSnapshot::default();h.slime=(i%13==0).into();h.mud=(i%17==0).into();h.blood=(i%9==0).into();h.ichor=(i%19==0).into();h.bloody_wall=(i%4==0).into();if i%7==0{h.water_blood=0.4.into();}if i%6==0{h.fire="normal".to_string().into();}if i%8==0{h.current=3.0.into();}
            ObservedCell{index:i,current,remembered:true,knowledge:if hidden{"layout"}else if direct{"direct"}else if current{"telepathic"}else{"remembered"}.into(),direct,tile:(!hidden).then(||tiles[i%tiles.len()].into()),substrate:Some(substrates[i%substrates.len()].into()),door_open:(i%17==1).then_some(i%2==0),snow:i%13==1,ice:i%19==1,puddles:i%23==1,hellfire:i%29==1,biome:(i%3==0).then(||biomes[i%4].into()),flowers:i%7==2,grate:i%17==2,bridge:i%17==3,entrance:i==31,chance:if current{90.}else{0.},direct_chance:if direct{85.}else{0.},illumination:current.then_some(75.),ambient:current.then_some(45.),distance:((i%8)as f64-3.).hypot((i/8)as f64-3.),hazards:Some(h)}
        }).collect();TerrainObservation{all_seeing:false,width:8,height:8,cells,light_sources:vec![]}
    }
    fn profiles()->Vec<Profile>{let d=Profile::default();let a=d.set("terrain.floor.color",serde_json::json!("#00aaff")).unwrap().set("terrain.floor.opacity",serde_json::json!(0.4)).unwrap();let upper=d.text("terrain.ground:tiled.color").to_ascii_uppercase();vec![d.clone(),a.clone(),d.set("terrain.ground:tiled.color",upper.into()).unwrap(),a.reset_group("terrain").unwrap(),Profile::parse(&a.json()).unwrap(),a.set("terrain.water.color",serde_json::json!("#187DC5")).unwrap(),d.set("global.temperature",serde_json::json!(0.5)).unwrap().set("global.gamma",serde_json::json!(1.4)).unwrap()]}
    #[test]
    fn prepared_ground_matches_exact_json_reference_every_cell(){
        let o=observation();for p in profiles(){for flesh in [false,true]{for cell in o.cells.iter().filter(|c|c.tile.is_some()){
            let g=ground_compiled::prepare(&p,cell,flesh);let old=ground_profile(&p,cell,flesh);
            assert_eq!((&g.glyph,g.fg,g.bg,g.fixture,g.stone),(&old.0,old.1,old.2,old.3,old.4),"cell {} flesh {}",cell.index,flesh);
        }}}
    }
    #[test]
    fn compiled_full_terrain_descriptors_and_pixels_match_json_reference(){
        let o=observation();let run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for p in profiles(){for flesh in [false,true]{for now in [0.,731.]{
            let a=terrain_frame_palette(&o,&p,17,now,flesh);let b=terrain_frame_palette_json_reference(&o,&p,17,now,flesh);
            assert_eq!(format!("{:?}",&a),format!("{:?}",&b),"full Cell/background/light oracle");
            let mut va=observe_with_profile(&run,&p);let mut vb=observe_with_profile(&run,&p);
            va.width=8;va.height=8;va.cells=a.0;va.terrain_count=va.cells.len();va.terrain_snapshot=None;
            vb.width=8;vb.height=8;vb.cells=b.0;vb.terrain_count=vb.cells.len();vb.terrain_snapshot=None;
            let mut pa=crate::render::Painter::new();let mut pb=crate::render::Painter::new();pa.profile=p.clone();pb.profile=p.clone();
            pa.game_at(&va,"",false,now);pb.game_at(&vb,"",false,now);assert_eq!(pa.pixels,pb.pixels);
        }}}
    }
}

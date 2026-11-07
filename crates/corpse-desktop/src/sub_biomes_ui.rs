//! Presentation regressions for the revision-37 terrain layers.
#[cfg(test)]
mod tests {
    use corpse_core::{Field, Run};
    fn scene() -> Run {
        let mut r = crate::non_euclidean_ui::tests::scene();
        r.simulation_revision = 37.0.into();
        let f = r.floor_mut().unwrap();
        f.non_euclidean = Field::Missing;
        let n = f.tiles.len();
        f.ice = vec![false;n].into(); f.snow = vec![false;n].into();
        f.puddles = vec![false;n].into(); f.hellfire = vec![false;n].into();
        r
    }
    #[test]
    fn r37_terrain_observation_and_inspection_are_detached() {
        let mut r=scene(); let w=r.floor().unwrap().width();
        let f=r.floor_mut().unwrap();
        f.ice.as_mut().unwrap()[10*w+11]=true;
        f.puddles.as_mut().unwrap()[10*w+12]=true;
        f.hellfire.as_mut().unwrap()[10*w+13]=true;
        let before=r.clone();
        let o=corpse_core::vision::observe_terrain(&r).unwrap();
        assert!(o.cells[10*w+11].ice); assert!(o.cells[10*w+12].puddles); assert!(o.cells[10*w+13].hellfire);
        assert_eq!(crate::inspection_ui::observe_tile(&r,11,10).title,"Ice");
        let puddle=crate::inspection_ui::observe_tile(&r,12,10);
        assert_eq!(puddle.title,"Puddle"); assert!(puddle.scene.contains("a puddle sits on the ground."),"{}",puddle.scene);
        assert_eq!(crate::inspection_ui::observe_tile(&r,13,10).title,"Hellfire");
        let v=crate::observe::observe(&r);
        assert!(v.cells.iter().any(|c|c.x==11&&c.y==10&&c.label=="Ice"&&c.glyph=="_"));
        assert!(v.cells.iter().any(|c|c.x==12&&c.y==10&&c.label=="Puddle"&&c.glyph=="~"));
        assert!(v.cells.iter().any(|c|c.x==13&&c.y==10&&c.label=="Hellfire"&&c.fire));
        assert_eq!(r,before);
    }
    #[test]
    fn r37_melted_snow_puddle_is_described_even_over_ice() {
        let mut r=scene();let w=r.floor().unwrap().width();let at=10*w+11;
        let f=r.floor_mut().unwrap();f.ice.as_mut().unwrap()[at]=true;f.snow.as_mut().unwrap()[at]=true;
        assert!(corpse_core::terrain_runtime::melt_snow(r.floor_mut().unwrap(),at));
        let report=crate::inspection_ui::observe_tile(&r,11,10);
        assert_eq!(report.title,"Ice");assert!(report.scene.contains("a puddle sits on the ground."));
    }
    #[test]
    fn r37_terrain_remembers_last_seen_layers_without_hidden_updates() {
        let mut r=scene(); let w=r.floor().unwrap().width(); let at=10*w+11;
        r.floor_mut().unwrap().ice.as_mut().unwrap()[at]=true;
        corpse_core::engine::remember(&mut r);
        let f=r.floor_mut().unwrap();
        f.ice.as_mut().unwrap()[at]=false; f.puddles.as_mut().unwrap()[at]=true;
        for y in 1..f.height()-1 {f.tiles[y*w+20]="wall".into();}
        r.player.x=65.;r.player.y=20.;
        let o=corpse_core::vision::observe_terrain(&r).unwrap();
        assert!(!o.cells[at].current);assert!(o.cells[at].ice);assert!(!o.cells[at].puddles);
        assert_eq!(crate::inspection_ui::observe_tile(&r,11,10).title,"Ice");
        let f=r.floor_mut().unwrap();f.seen[at]=false;
        let o=corpse_core::vision::observe_terrain(&r).unwrap();
        assert!(!o.cells[at].ice&&!o.cells[at].puddles&&!o.cells[at].hellfire);
    }
    #[test]
    fn r37_hellfire_palette_is_distinct_and_cosmetic() {
        let mut r=scene(); let w=r.floor().unwrap().width();let at=10*w+11;
        r.floor_mut().unwrap().hellfire.as_mut().unwrap()[at]=true;
        let o=corpse_core::vision::observe_terrain(&r).unwrap();let mut ordinary=o.cells[at].clone();ordinary.hellfire=false;
        let p=crate::appearance_profile::Profile::defaults();
        let (_,bg,lights)=crate::observe::terrain_frame(&o,p,17,0.);
        let before=r.clone();let mut different=false;
        for now in [0.,100.,200.,500.,1000.] {
            let hell=crate::observe::fire_cell(p,&o.cells[at],o.all_seeing,lights[at],[9,3,7],bg[at],w,17,19,now);
            let fire=crate::observe::fire_cell(p,&ordinary,o.all_seeing,lights[at],[9,3,7],bg[at],w,17,19,now);
            different|=hell.fg!=fire.fg;
        }
        assert!(different);assert_eq!(r,before);
    }
    fn save(p: &crate::render::Painter, path: &std::path::Path) {
        let f=std::fs::File::create(path).unwrap(); let mut encoder=png::Encoder::new(f,p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
    #[test]
    fn r37_sub_biomes_actual_renderer_controlled_capture() {
        let mut r=scene(); let f=r.floor_mut().unwrap();let w=f.width();
        for y in 5..19 {for x in 5..26 {
            let at=y*w+x;
            if (x as i32-15).pow(2)+(y as i32-12).pow(2)<80 {
                f.snow.as_mut().unwrap()[at]=true;
                if (x+y)%4!=0 {f.ice.as_mut().unwrap()[at]=true;}
            }
        }}
        for y in 7..18 {f.hellfire.as_mut().unwrap()[y*w+36]=true;}
        for x in 28..33 {f.puddles.as_mut().unwrap()[14*w+x]=true;}
        r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
        let mut v=crate::observe::observe(&r);v.provenance="CONTROLLED r37 terrain fixture; not generated gameplay".into();
        let before=r.clone();let mut p=crate::render::Painter::new();p.game_at(&v,"Ice, snow, puddles, Hellfire",false,824.);
        if let Ok(dir)=std::env::var("CORPSE_SUB_BIOMES_UI_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();save(&p,&std::path::Path::new(&dir).join("sub-biomes-terrain-controlled.png"));
            std::fs::write(std::path::Path::new(&dir).join("sub-biomes-terrain-controlled.json"),serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        }
        assert_eq!(r,before);
    }
    #[test]
    fn r37_sub_biomes_actual_renderer_generated_capture() {
        let Ok(dir)=std::env::var("CORPSE_SUB_BIOMES_GENERATED_REVIEW") else {return;};
        let primary=std::env::var("CORPSE_SUB_BIOMES_REVIEW_SEED").unwrap_or_else(|_|"sub-biomes-review-r37".into());
        let corn=std::env::var("CORPSE_SUB_BIOMES_CORN_REVIEW_SEED").unwrap_or_else(|_|"sub-biome-review-1".into());
        for (seed,themes) in [(primary,vec!["mountainous","hellfire"]),(corn,vec!["corn-dimension"])] {
        let mut r=corpse_core::startup::create_run_revision(&seed,"Unblessed",true,37.).unwrap();
        corpse_core::startup::ensure_floors(&mut r,40).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        for theme in themes {
            let (depth,at,w)=r.floors.iter().filter(|f|theme!="mountainous"||f.depth==22.).find_map(|f| {
                let at=if theme=="hellfire" {f.hellfire.as_ref()?.iter().position(|b|*b)?}
                    else if theme=="mountainous" {f.ice.as_ref()?.iter().position(|b|*b)?}
                    else {*f.pocket_biomes.as_ref()?.iter().find(|b|b.theme==theme)?.cells.first()?};
                Some((f.depth,at,f.width()))
            }).unwrap_or_else(||panic!("capture seed {seed} has no {theme}"));
            println!("Generated capture seed={seed} theme={theme} floor={depth}");
            r.depth=depth;r.offers.clear();
            let f=r.floor().unwrap();
            let stand=f.tiles.iter().enumerate().find(|(i,t)|t.as_str()=="up"&&corpse_core::non_euclidean_generation::group_at(f,*i).is_none()).map(|(i,_)|i).unwrap_or(at);
            r.player.x=(stand%w)as f64;r.player.y=(stand/w)as f64;
            r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),depth);
            let mut v=crate::observe::observe(&r);
            v.provenance=format!("GENERATION QA r37 floor {depth}; full reveal; level-1 observer; no turns");
            v.hover=Some((at%w,at/w));
            let report=crate::inspection_ui::observe_tile(&r,at%w,at/w);
            let mut p=crate::render::Painter::new();p.game_at(&v,&report.scene,false,824.);
            save(&p,&std::path::Path::new(&dir).join(format!("sub-biomes-{theme}-floor{}-generated-debug.png",depth as u32)));
            std::fs::write(std::path::Path::new(&dir).join(format!("sub-biomes-{theme}-generated-debug.json")),serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        }
        }
    }

}

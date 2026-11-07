//! Exact-output tests, separate from lead-controlled performance measurements.
use super::*;

#[test]
fn sparse_ascii_runs_reconstruct_coverage_and_clip_without_zeros() {
    let (w,h)=(37usize,19usize);
    for pattern in 0..5 {
        let coverage:Vec<_>=(0..w*h).map(|i|match pattern {
            0=>0, 1=>255, 2=>if i%2==0 {0}else{1},
            3=>if i/w%3==0 || i%w<7 || i%w>28 {0}else{(i%255+1) as u8},
            _=>i.wrapping_mul(53) as u8,
        }).collect();
        let cache=ascii_mask::AsciiMask::new(coverage.clone(),w,h);
        assert_eq!(cache.coverage,coverage);
        assert_eq!(cache.row_offsets.len(),h+1);
        assert_eq!(cache.row_offsets[0],0);
        assert_eq!(cache.row_offsets[h] as usize,cache.runs.len());
        for yy in 0..h {
            for (left,right) in [(0,w as i32),(3,32),(5,5)] {
                let mut visited=vec![false;w];let mut previous_end=None;
                for (l,r) in cache.row_runs(yy,left,right) {
                    assert!(left<=l && l<r && r<=right);
                    if let Some(end)=previous_end {assert!(end<l,"runs must be maximal and ordered");}
                    previous_end=Some(r);
                    for x in l..r {assert!(!visited[x as usize]);visited[x as usize]=true;}
                }
                for x in 0..w {assert_eq!(visited[x],(left..right).contains(&(x as i32))&&coverage[yy*w+x]!=0);}
            }
        }
    }
}

#[test]
fn sparse_ascii_matches_old_scalar_with_joint_clips_negative_origins_and_alpha() {
    let mut p=Painter::new();p.resize(1537,911);p.set_responsive_canvas(true);
    // Exercise a negative native origin as well as negative logical coordinates.
    p.origin=(-7,-11);
    let initial:Vec<_>=(0..p.pixels.len()).map(|i|i.wrapping_mul(37) as u8).collect();
    for (x,y) in [(-2,-3),(9,8)] {
        let(lw,lh)=(57,43);
        let(nx,ny)=(p.native_x(x as f64),p.native_y(y as f64));
        let(w,h)=(p.native_x((x+lw) as f64)-nx,p.native_y((y+lh) as f64)-ny);
        // Contains all byte coverage values plus long zero spans and empty rows.
        let mask:Vec<_>=(0..(w*h) as usize).map(|i|if i/(w as usize)%5==0 || i%11<7 {0}else{i.wrapping_mul(53) as u8}).collect();
        let weight=p.profile.number("typography.map.fontWeight").round().clamp(100.,900.) as u16;
        p.map_masks.insert(('@',w,h,weight),ascii_mask::AsciiMask::new(mask.clone(),w as usize,h as usize));
        for italic in [false,true] {
            for clip_mode in 0..3 {
                p.clip=match clip_mode {0=>None,1=>Some((2,3,49,37)),_=>Some((2,3,0,37))};
                p.damage_clip=(clip_mode!=0).then_some(damage::Rect::new(3,2,51,39));
                p.tile_clip_enabled=clip_mode!=0;p.tile_mask.reset(p.width,p.height);
                p.tile_mask.add(damage::Rect::new(0,0,11,45));
                p.tile_mask.add(damage::Rect::new(16,6,16,37));
                p.tile_mask.add(damage::Rect::new(35,0,43,31));p.tile_mask.finish();
                for alpha in [1.,f64::from_bits(1f64.to_bits()-1),0.42] {
                    let mut expected=initial.clone();
                    alpha_one_oracle_tests::scalar_mask(&mut expected,p.width,p.native_bounds(),nx,ny,w as usize,h as usize,
                        &mask,[231,13,92],alpha,false,italic,p.tile_clip_enabled.then_some(&p.tile_mask));
                    let mut full=0u64;let mut nonzero=0u64;let bounds=p.native_bounds();
                    // Independent scalar clipping/count oracle, not the cached runs.
                    for yy in 0..h {for xx in 0..w {
                        let shear=if italic {((h-1-2*yy) as f64*w as f64/(8.*h.max(1) as f64)).round() as i32}else{0};
                        let(px,py)=(nx+xx+shear,ny+yy);
                        if px<bounds.0||px>=bounds.2||py<bounds.1||py>=bounds.3
                            ||p.tile_clip_enabled&&!p.tile_mask.contains(px as usize,py as usize){continue;}
                        full+=1;nonzero+=u64::from(mask[(yy*w+xx) as usize]!=0);
                    }}
                    for sparse in [false,true] {
                        p.pixels.copy_from_slice(&initial);p.ascii_sparse_enabled=sparse;
                        p.raster_stats=damage::RasterStats{enabled:true,..Default::default()};
                        p.map_glyph_styled('@',x,y,lw,lh,[231,13,92],alpha,italic);
                        assert_eq!(p.pixels,expected,"origin=({x},{y}) italic={italic} clips={clip_mode} alpha={alpha} sparse={sparse}");
                        assert_eq!(p.raster_stats.pixel_visits,if sparse&&alpha==1. {nonzero}else{full});
                        assert!(p.pixels.chunks_exact(4).zip(initial.chunks_exact(4)).all(|(a,b)|a[3]==b[3]));
                    }
                }
            }
        }
    }
}

#[test]
fn sparse_ascii_full_frames_and_clock_transitions_match_control() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let saved=engine.save().unwrap();let mut view=crate::observe::observe(&engine.run);
    view.measure_conditions(&mut crate::statuses_ui::ConditionMeters::default());
    let mut full=Painter::new();let mut sparse=Painter::new();full.ascii_sparse_enabled=false;
    full.background_fused_enabled=false;sparse.background_fused_enabled=false;
    for(w,h)in [(1537,911),(3840,2160)] {
        full.resize(w,h);sparse.resize(w,h);
        let mut pack_full=crate::pack::PackView::observe(&engine.run);
        let mut pack_sparse=crate::pack::PackView::observe(&engine.run);
        for clock in [1000.,1125.] {
            full.game_at(&view,"",false,clock);sparse.game_at(&view,"",false,clock);
            assert_eq!(full.pixels,sparse.pixels,"game {w}x{h} clock={clock}");
            assert!(!sparse.map_masks.is_empty());
            pack_full.configure(&full,full.frame().map.tuple());
            pack_sparse.configure(&sparse,sparse.frame().map.tuple());
            pack_full.paint(&mut full);pack_sparse.paint(&mut sparse);
            assert_eq!(full.pixels,sparse.pixels,"Pack {w}x{h} clock={clock}");
        }
    }
    assert_eq!(saved,engine.save().unwrap(),"painting cannot mutate run/RNG/replay");
    sparse.resize(1920,1080);assert!(sparse.map_masks.is_empty(),"runs share cache lifetime");
}

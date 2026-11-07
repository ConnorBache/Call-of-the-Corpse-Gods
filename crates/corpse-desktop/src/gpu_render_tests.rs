use super::*;
use crate::render::Painter;
use serde_json::json;

#[test]
fn clipped_quad_uses_matching_source_pixels_and_caches_masks(){
    let mut f=Frame::default();let key=MaskKey::Text('x',10);
    f.mask(key.clone(),(5,7,11,13),3,4,8,10,&vec![170;80],[1,2,3],1.,false);
    assert_eq!(f.quads[0].rect,(5,7,11,13));assert_eq!(f.quads[0].mask,Some((0,(2,3,8,9))));
    f.begin();f.mask(key,(0,0,100,100),5,7,8,10,&vec![170;80],[3,4,5],0.5,false);
    assert_eq!(f.masks.len(),1);assert_eq!(f.quads.len(),1);assert_eq!(f.quads[0].color,[3,4,5,128]);
}

#[test]
#[ignore="Explicit isolated SDL process: SDL_VIDEODRIVER=offscreen or dummy, --test-threads=1"]
fn gpu_sdl_native_quad_parity_and_fallback(){
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();
    let window=video.window("GPU verification",1280,800).hidden().build().unwrap();
    let (mut canvas,accelerated)=create_canvas(window,false).unwrap();
    let creator=canvas.texture_creator();let mut renderer=Renderer::new(&creator,2048);
    let mut expected=Painter::new();let mut p=Painter::new();p.gpu=Some(Frame::default());
    fn scene(p:&mut Painter){
        p.clear([11,13,21]);p.rect(14,17,500,250,[30,44,71]);p.blend_rect(17,35,121,74,[240,130,50],129);
        p.text("GPU glyphs: Welcome Ω 123!?",20,40,19,[238,224,150]);
        for (i,ch) in ['@','#','A','&','.','~','%','+'].iter().enumerate(){p.map_glyph(*ch,20+i as i32*35,95,24,38,[100,230,194]);}
        p.map_identity("Ω",72,155,21,27,[240,210,80]);p.map_identity("II",120,155,23,28,[210,170,90]);
        p.clip=Some((50,195,135,27));p.text("Clipped outlines and pixels",20,190,22,[230,235,250]);p.clip=None;
        p.prose("Native prose",20,235,18,[240,190,150],false);
    }
    scene(&mut expected);scene(&mut p);
    assert!(p.pixels.iter().all(|&b|b==0),"GPU path must not CPU-paint a framebuffer");
    renderer.draw(&mut canvas,p.gpu.as_ref().unwrap()).unwrap();
    let actual=canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap();
    let mut max=0u8;let mut sum=0usize;let mut different=0usize;
    for (a,b) in actual.chunks_exact(4).zip(expected.pixels.chunks_exact(4)) {for k in 0..3{let d=a[k].abs_diff(b[k]);max=max.max(d);sum+=d as usize;if d>0{different+=1;}}}
    eprintln!("{}",json!({"renderer":canvas.info().name,"accelerated_api":accelerated,"max_channel_delta":max,"sum_channel_delta":sum,"different_channels":different,"stats":format!("{:?}",renderer.stats)}));
    assert!(max<=3,"GPU blending must preserve appearance (rounding tolerance 3), got {max}");
    assert!(renderer.stats.uploaded_mask_bytes>0);assert!(renderer.stats.batches<=3);
    renderer.draw(&mut canvas,p.gpu.as_ref().unwrap()).unwrap();assert_eq!(renderer.stats.uploaded_mask_bytes,0);
    renderer.reset();renderer.draw(&mut canvas,p.gpu.as_ref().unwrap()).unwrap();assert!(renderer.stats.uploaded_mask_bytes>0);
    drop(renderer);drop(creator);drop(canvas);
    let window=video.window("Forced software",31,19).hidden().build().unwrap();
    let (canvas,accelerated)=create_canvas(window,true).unwrap();assert!(!accelerated);assert_eq!(canvas.info().name,"software");
}

#[test]
fn gameplay_gpu_command_capture_preserves_cpu_pixels_and_simulation(){
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before=serde_json::to_string(&engine.run).unwrap();
    let mut view=crate::observe::observe(&engine.run);view.path=vec![(12,14),(13,14)];view.hover=Some((13,14));
    let mut p=Painter::new();p.gpu=Some(Frame::default());p.pixels.fill(17);
    for (w,h) in [(1280,800),(1920,1080),(1537,911),(960,1200),(3840,2160)] {
        p.resize(w,h);p.pixels.fill(17);p.gpu.as_mut().unwrap().begin();
        p.game_at(&view,"GPU verification",false,420.);
        let mut pack=crate::pack::PackView::observe(&engine.run);pack.configure(&p,p.frame().map.tuple());pack.paint(&mut p);
        let report=crate::inspection_ui::observe_tile(&engine.run,engine.run.player.x as usize,engine.run.player.y as usize);
        crate::inspection_ui::paint_hover(&report,&mut p,300,300);
        assert!(p.pixels.iter().all(|&b|b==17),"GPU path CPU-painted at {w}x{h}");
        assert!(!p.gpu.as_ref().unwrap().quads.is_empty());
    }
    assert_eq!(serde_json::to_string(&engine.run).unwrap(),before);
}

#[test]
#[ignore="Isolated native SDL matrix; software OpenGL is functional evidence, not hardware performance"]
fn gpu_sdl_gameplay_visual_matrix(){
    let input=std::env::var("CORPSE_GPU_INPUT").ok().map(|p|std::fs::read_to_string(p).unwrap());
    let (engine,_)=crate::session_store::decode(input.as_deref().unwrap_or(include_str!("../../../fixtures/avatar-room-v1.json"))).unwrap();
    let mut view=crate::observe::observe(&engine.run);view.path=vec![(12,14),(13,14)];view.hover=Some((13,14));
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();
    let mut rows=vec![];let mut driver=String::new();let mut accelerated_api=false;
    let output=std::env::var("CORPSE_GPU_OUTPUT").ok();
    if let Some(dir)=&output{std::fs::create_dir_all(dir).unwrap();}
    let mut cpu=Painter::new();let mut gpu=Painter::new();gpu.gpu=Some(Frame::default());
    let report=crate::inspection_ui::observe_tile(&engine.run,engine.run.player.x as usize,engine.run.player.y as usize);
    let baseline_cells=view.cells.clone();let baseline_terrain=view.terrain_snapshot.clone();
    for (w,h) in [(1280,800),(1920,1080),(3840,2160),(1537,911),(960,1200)] {
        // SDL offscreen allocates its EGL pbuffer only at window creation.
        // Its SetWindowSize updates reported dimensions without resizing that
        // surface, so native image comparisons must create a fresh window.
        let window=video.window("GPU scene matrix",w as u32,h as u32).hidden().build().unwrap();
        let (mut canvas,accelerated)=create_canvas(window,false).unwrap();driver=canvas.info().name.to_owned();accelerated_api=accelerated;
        let creator=canvas.texture_creator();let mut renderer=Renderer::new(&creator,2048);
        assert_eq!(canvas.output_size().unwrap(),(w as u32,h as u32));
        cpu.resize(w,h);gpu.resize(w,h);
        for scene in ["game","pack","hover","styled","shutter","animated","corpse-markers","blood-dots"] {
            view.cells=baseline_cells.clone();view.terrain_snapshot=baseline_terrain.clone();
            if scene=="blood-dots" {
                if let Some(snapshot)=baseline_terrain.as_ref() {
                    let mut observed=(**snapshot).clone();
                    for c in &mut observed.cells {if c.current && c.tile.as_deref()==Some("floor") {c.hazards=Some(serde_json::from_value(json!({"blood":true})).unwrap());}}
                    let(mut cells,_,_)=crate::observe::terrain_frame(&observed,crate::appearance_profile::Profile::defaults(),view.cosmetic_seed,0.);
                    assert!(cells.iter().zip(baseline_cells.iter()).any(|(a,b)|a.fg!=b.fg),"blood scene must actually recolor floor dots");
                    cells.extend(baseline_cells.iter().skip(view.terrain_count).cloned());view.cells=cells;
                    view.terrain_snapshot=Some(std::sync::Arc::new(observed));
                }
            }
            if scene=="corpse-markers" {for cell in &mut view.cells {if cell.actor_id.is_some(){cell.corpse=true;cell.heartless=false;}}}
            if scene=="animated" {for cell in &mut view.cells{cell.italic=true;cell.animations=vec![crate::render::Animation::Gas{phase:0.7,color:[18,233,91]},crate::render::Animation::Reflection{phase:0.4,color:[219,45,192]}];}}
            cpu.profile=crate::appearance_profile::Profile::default();gpu.profile=cpu.profile.clone();
            if scene=="styled" {
                cpu.profile=cpu.profile.set("typography.chronicle.fontWeight",json!(700)).unwrap();
                cpu.profile=cpu.profile.set("typography.map.fontFamily",json!("IBM Plex Mono")).unwrap();
                cpu.profile=cpu.profile.set("typography.map.letterSpacing",json!(0.7)).unwrap();gpu.profile=cpu.profile.clone();
            }
            fn paint(p:&mut Painter,view:&crate::render::View,scene:&str,run:&corpse_core::model::Run,report:&crate::inspection_ui::Report){
                if scene=="pack" && p.gpu.is_some(){
                    let mut pack=crate::pack::PackView::observe(run);pack.configure(p,p.frame().map.tuple());
                    p.game_at_behind_pack(view,"GPU scene verification",420.,pack.bounds());pack.paint(p);return;
                }
                p.game_at(view,"GPU scene verification",false,420.);
                match scene {
                    "pack"=>{let mut pack=crate::pack::PackView::observe(run);pack.configure(p,p.frame().map.tuple());pack.paint(p);},
                    "hover"=>crate::inspection_ui::paint_hover(report,p,300,300),
                    "shutter"=>{let mut shutter=crate::chronicle_overlay::Shutter::default();shutter.open(0.,false);crate::chronicle_overlay::paint(p,&vec!["A remembered event. Ω 123".into();25],&shutter,110.,false);},
                    _=>{}
                }
            }
            let start=std::time::Instant::now();paint(&mut cpu,&view,scene,&engine.run,&report);let cpu_ns=start.elapsed().as_nanos();
            gpu.pixels.fill(19);gpu.gpu.as_mut().unwrap().begin();let start=std::time::Instant::now();paint(&mut gpu,&view,scene,&engine.run,&report);let commands_ns=start.elapsed().as_nanos();
            assert!(gpu.pixels.iter().all(|&b|b==19),"unexpected CPU raster for {scene}");
            renderer.draw(&mut canvas,gpu.gpu.as_ref().unwrap()).unwrap();let actual=canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap();
            let mut max=0u8;let mut sum=0u64;let mut over3=0usize;
            for (a,b) in actual.chunks_exact(4).zip(cpu.pixels.chunks_exact(4)){for k in 0..3{let d=a[k].abs_diff(b[k]);max=max.max(d);sum+=d as u64;if d>3{over3+=1;}}}
            let row=json!({"scene":scene,"width":w,"height":h,"max_channel_delta":max,"mean_channel_delta":sum as f64/(w*h*3)as f64,"channels_over_3":over3,"cpu_cold_paint_ns":cpu_ns,"cpu_gpu_cold_command_ns":commands_ns,"quads":renderer.stats.quads,"batches":renderer.stats.batches,"uploaded_bytes":renderer.stats.uploaded_mask_bytes});eprintln!("{row}");rows.push(row);
            if let Some(dir)=&output {
                for (label,pixels) in [("gpu",actual.as_slice()),("cpu",cpu.pixels.as_slice())]{
                    let file=std::fs::File::create(format!("{dir}/{scene}-{w}x{h}-{label}.png")).unwrap();let mut enc=png::Encoder::new(file,w as u32,h as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(pixels).unwrap();
                }
            }
            assert!(max<=8,"{scene} {w}x{h}: unexpected image difference {max}");
        }
    }
    if let Some(dir)=output{std::fs::write(format!("{dir}/matrix.json"),serde_json::to_vec_pretty(&json!({"sdl_renderer":driver,"accelerated_api":accelerated_api,"hardware_verified":false,"fresh_window_per_resolution":true,"rows":rows})).unwrap()).unwrap();}
}

#[test]
#[ignore="Isolated SDL process exercises atlas batching and renderer replacement"]
fn gpu_sdl_atlas_order_batch_boundary_and_failure_recovery(){
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();
    let window=video.window("GPU fallback verification",64,64).hidden().build().unwrap();
    let (mut canvas,_)=create_canvas(window,false).unwrap();let creator=canvas.texture_creator();
    // A tiny atlas forces page0 -> page1 -> page0 without enormous fixtures.
    let mut renderer=Renderer::new(&creator,32);let mut f=Frame::default();f.clear=Some([0,0,0]);
    for (ch,color) in [('a',[250,10,20]),('b',[10,250,20]),('a',[10,20,250])] {
        f.mask(MaskKey::Text(ch,22),(0,0,64,64),3,4,22,22,&vec![255;22*22],color,1.,false);
    }
    renderer.draw(&mut canvas,&f).unwrap();assert_eq!(renderer.stats.atlas_pages,2);assert_eq!(renderer.stats.batches,3);
    let bytes=canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap();assert_eq!(&bytes[(8*64+8)*4..(8*64+8)*4+3],&[10,20,250]);
    f.begin();f.clear=Some([0,0,0]);for n in 0..17000{f.fill((1,1,2,2),if n==16999{[13,28,91]}else{[1,2,3]},255);}
    renderer.draw(&mut canvas,&f).unwrap();assert_eq!(renderer.stats.batches,2);
    let bytes=canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap();assert_eq!(&bytes[(64+1)*4..(64+1)*4+3],&[13,28,91]);
    // Force the real atlas admission error, then perform the same ownership
    // sequence as main: destroy textures/context, retain window, create software.
    f.begin();f.mask(MaskKey::Text('X',65),(0,0,64,64),0,0,40,40,&vec![255;1600],[255,255,255],1.,false);
    assert!(renderer.draw(&mut canvas,&f).unwrap_err().contains("exceeds"));
    drop(renderer);drop(creator);let window=canvas.window().clone();drop(canvas);
    let (mut canvas,accelerated)=create_canvas(window,true).unwrap();assert!(!accelerated);assert_eq!(canvas.info().name,"software");
    let creator=canvas.texture_creator();let mut texture=creator.create_texture_streaming(PixelFormatEnum::RGBA32,64,64).unwrap();
    let pixels=[17,29,41,255].repeat(64*64);texture.update(None,&pixels,64*4).unwrap();canvas.copy(&texture,None,None).unwrap();
    assert_eq!(canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap(),pixels);
}

#[test]
fn unsupported_gpu_primitive_requests_fallback_before_painting(){
    let mut p=Painter::new();p.gpu=Some(Frame::default());p.pixels.fill(33);
    p.picture(&crate::render::Picture{w:1,h:1,rgba:vec![255;4]},0,0,5,5);
    assert!(p.gpu.as_ref().unwrap().unsupported.is_some());assert!(p.pixels.iter().all(|&b|b==33));
    p.gpu.as_mut().unwrap().begin();p.prose_contrast_outline=true;p.prose("Ink",1,1,12,[22,33,44],false);
    assert!(p.gpu.as_ref().unwrap().unsupported.is_some());assert!(p.pixels.iter().all(|&b|b==33));
}

#[test]
#[ignore="Isolated native SDL fractional-alpha/italic sampling oracle"]
fn gpu_sdl_italic_fractional_alpha_oracle(){
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();
    let window=video.window("Italic/alpha oracle",64,64).hidden().build().unwrap();
    let (mut canvas,_)=create_canvas(window,false).unwrap();let creator=canvas.texture_creator();let mut renderer=Renderer::new(&creator,2048);
    let (w,h)=(13usize,17usize);let mask=(0..w*h).map(|n|((n*37)%256)as u8).collect::<Vec<_>>();
    for alpha in [1.,0.42,0.025] {for italic in [false,true]{
        let mut frame=Frame::default();frame.clear=Some([17,29,41]);let bounds=(6,7,32,31);
        frame.mask(MaskKey::Ascii('Q',w as i32,h as i32,400,italic),bounds,5,6,w,h,&mask,[218,133,251],alpha,italic);
        renderer.reset();renderer.draw(&mut canvas,&frame).unwrap();let actual=canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap();
        let mut expected=[17,29,41,255].repeat(64*64);
        for yy in 0..h{for xx in 0..w{
            let shift=if italic{((h as i32-1-2*yy as i32)as f64*w as f64/(8.*h as f64)).round()as i32}else{0};
            let (x,y)=(5+xx as i32+shift,6+yy as i32);if x<bounds.0||x>=bounds.2||y<bounds.1||y>=bounds.3{continue;}
            let a=(mask[yy*w+xx]as f64*alpha).round()as u32;let at=(y as usize*64+x as usize)*4;
            for k in 0..3{expected[at+k]=(([218,133,251][k]as u32*a+expected[at+k]as u32*(255-a))/255)as u8;}
        }}
        let max=actual.iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
        assert!(max<=3,"alpha={alpha} italic={italic} max={max}");
    }}
}

#[test]
fn native_setup_errors_remain_inside_the_recovery_result(){
    assert!(checked_sdl(0,"test").is_ok());
    assert!(checked_sdl(-1,"injected native setup failure").unwrap_err().starts_with("injected native setup failure:"));
}

#[test]
#[ignore="Isolated SDL parity for R50 outlined default identities; SDL_VIDEODRIVER=offscreen --test-threads=1"]
fn revision50_creature_identity_gpu_native_parity() {
    let r=crate::render::creature_identity_tests::room();let view=crate::observe::observe(&r);
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();
    for (w,h) in [(1280,800),(1537,911),(2560,1600)] {
        let window=video.window("Creature identity parity",w,h).hidden().build().unwrap();
        let (mut canvas,_)=create_canvas(window,false).unwrap();let creator=canvas.texture_creator();let mut renderer=Renderer::new(&creator,2048);
        let mut cpu=Painter::new();let mut gpu=Painter::new();cpu.resize(w as usize,h as usize);gpu.resize(w as usize,h as usize);gpu.gpu=Some(Frame::default());
        cpu.game_at(&view,"",false,0.);gpu.game_at(&view,"",false,0.);
        renderer.draw(&mut canvas,gpu.gpu.as_ref().unwrap()).unwrap();let actual=canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap();
        let maximum=actual.chunks_exact(4).zip(cpu.pixels.chunks_exact(4)).flat_map(|(a,b)|(0..3).map(move|k|a[k].abs_diff(b[k]))).max().unwrap();
        assert!(maximum<=3,"identity CPU/GPU channel delta {maximum} at {w}x{h}");
    }
}

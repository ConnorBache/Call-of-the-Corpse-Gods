// One actual two-command trace with naturally observed light/fire animations.
// It supplements, rather than relabels, the zero-animation ordinary room rows.
fn animated_weave_room()->Run {
    let mut r=weave_room();
    let f=r.floor_mut().unwrap();let w=f.width();
    // Neither source nor destination (10,10)/(11,10) is hazardous or occupied.
    // Nonflammable floor separates the distant permanent fire from both steps.
    f.tiles[7*w+10]="wall".into();
    f.light_sources=vec![corpse_core::FloorLightSourcesEntry{x:10.,y:7.,kind:"torch".to_string().into(),power:75.,falloff:10.}].into();
    f.lights.fill(0.);
    f.permanent_fire=vec![(10*w+14)as f64].into();
    r
}
fn animation_counts(v:&render::View)->(usize,usize,usize,usize){
    let mut out=(0,0,0,0);for c in &v.cells{for a in &c.animations{out.0+=1;match a{render::Animation::Fire{..}=>out.1+=1,render::Animation::Torch=>out.2+=1,render::Animation::Reflection{..}=>out.3+=1,_=>{}}}}out
}
#[test]
#[ignore="lead-controlled actual Move/freeback with observed animations; explicit overlay required"]
fn timeweave_action_animated_move_and_freeback(){
    let options=Options::read();let dir=Sandbox::new(&options.root,"animated-weave");let mut rows=Vec::new();
    for repeat in 1..=options.repeats {
        let mut engine=temporal::Engine::new(animated_weave_room()).unwrap();
        let path=dir.0.join(format!("animated-weave-{repeat}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);
        let initial=animation_counts(ui.game.as_ref().unwrap());assert!(initial.0>0,"fixture must genuinely observe animations before timed command");assert!(initial.1+initial.2>0,"authored fire/torch must be visible, not only reflected light");
        for(step,(label,dx,delta))in [("animated-weave-forward",1.,1.),("animated-weave-free-back",-1.,0.)].into_iter().enumerate(){
            let before=animation_counts(ui.game.as_ref().unwrap());let old_rng=engine.run.rng;let(x,y)=(engine.run.player.x,engine.run.player.y);
            let mut row=sample(&options,&mut engine,&mut auto,&path,&mut ui,label,repeat,step+1,None,&Command::Move{dx,dy:0.},delta);
            assert_eq!((engine.run.player.x,engine.run.player.y),(x+dx,y));assert_eq!(engine.run.timeweave.as_ref().unwrap().len(),1);if delta==0.{assert_eq!(engine.run.rng,old_rng);}
            let after=animation_counts(ui.game.as_ref().unwrap());assert!(after.0>0,"actual command must retain observed animations");
            row["observed_animations_before"]=serde_json::json!({"all":before.0,"fire":before.1,"torch":before.2,"reflection":before.3});row["observed_animations_after"]=serde_json::json!({"all":after.0,"fire":after.1,"torch":after.2,"reflection":after.3});row["actual_effect_asserted"]=true.into();row["fixture_authored_safe_sources"]=true.into();rows.push(row);
        }
    }
    report("timeweave-action-animated-move-freeback",&options,rows);
}

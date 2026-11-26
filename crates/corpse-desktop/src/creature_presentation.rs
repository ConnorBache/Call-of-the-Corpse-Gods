//! Detached appearance projection. Visibility is established by callers before
//! applying disguises/illusions; true actors and gameplay RNG are never mutated.
use corpse_core::{Actor,Run};
/// O(A + B*R + S + L + T): bodies/rays, copied actor, seed bytes and tuning rows.
/// Selection is bounded deterministic hashing, independent of frame time.
pub fn project(r:&Run,a:&Actor)->Option<Actor> {
    if r.simulation_revision.value_or(29.)>=43. && a.id==r.player.id{return None;}
    if corpse_core::dreamwalking::is_nightmare(a) || corpse_core::dreamwalking::is_body(a) { return None; }
    let original=corpse_core::actor_presentation::apparent_kind(r,a)?;
    let kind=if crate::hallucination_ui::affects(r,a) {
        crate::hallucination_ui::kind(r,a,0)
    }else{original};
    project_kind(r,a,kind)
}
/// Detached projection for one already-admitted identity; no gameplay RNG.
pub fn project_kind(r:&Run,a:&Actor,kind:&str)->Option<Actor> {
    let template=corpse_core::population::create_actor_revision(kind,a.id,a.x,a.y,false,if r.simulation_revision.value_or(29.)>=50. {50.} else {33.}).ok()?;
    let mut shown=a.clone();
    shown.name=template.name;shown.creature_kind=template.creature_kind;
    shown.glyph=template.glyph;shown.color=template.color;shown.abilities=template.abilities;
    shown.active.clear();shown.monster_state=corpse_core::Field::Missing;
    shown.hp=(a.hp/a.max_hp.max(1.)).clamp(0.,1.)*template.max_hp;
    shown.max_hp=template.max_hp;
    shown.energy=(a.energy/a.stats.max_energy.max(1.)).clamp(0.,1.)*template.stats.max_energy;
    shown.stats=template.stats;
    shown.conditions.retain(|k,_| !k.starts_with("skinWalker") && !k.starts_with("witchSummoned"));
    Some(shown)
}
/// Detached, lazy lookup of actual heart tiles on the observed floor. Marks
/// depend on physical co-location, not pickup history or the heart's source.
pub struct CorpseHearts<'a> {
    run: &'a Run,
    tiles: std::cell::OnceCell<std::collections::HashSet<(usize, usize, bool)>>,
}
impl<'a> CorpseHearts<'a> {
    pub fn new(run: &'a Run) -> Self {
        Self { run, tiles: std::cell::OnceCell::new() }
    }

    pub fn heartless(&self, a: &Actor) -> bool {
        let tiles = self.tiles.get_or_init(|| self.run.ground.get(&self.run.depth.to_string())
            .into_iter().flat_map(|items| items.iter())
            .filter(|entry| entry.item.kind == "heart" && crate::dream_ui::item_visible(self.run, &entry.item))
            .map(|entry| (entry.x as usize, entry.y as usize, corpse_core::dreamwalking::dream_item(self.run, &entry.item)))
            .collect());
        !tiles.contains(&(a.x as usize, a.y as usize, corpse_core::dreamwalking::dream_actor(self.run, a)))
    }
}

pub fn corpse_name(name: &str, heartless: bool) -> String {
    format!("{}{name} corpse", if heartless { "heartless " } else { "" })
}
/// Isolated combat/heart/ability inspection must not re-fetch the hidden actor by ID.
/// Run clone shares floor cells but copies metadata N; replacing one actor block
/// costs O(N + A*S), never mutating simulation state.
pub fn projected_run(r:&Run,shown:&Actor)->Run {
    let mut view=r.clone();
    if let Some(a)=view.actors_at_mut(r.depth).iter_mut().find(|a|a.id==shown.id) {*a=shown.clone();}
    view
}
#[cfg(test)]
mod tests {
    use super::*;
    fn room()->Run {
        let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:appearance33","Unblessed",false,33.).unwrap();
        r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let w=79;let h=29;let mut tiles=vec!["wall".into();w*h];for y in 1..h-1 {for x in 1..w-1 {tiles[y*w+x]="floor".into();}}
        r.floors=vec![std::sync::Arc::new(corpse_core::Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()})];
        let a=corpse_core::population::create_actor_revision("skin-walker",4.,5.,5.,false,33.).unwrap();
        r.actors.clear();r.actors.insert("1".into(),std::sync::Arc::new(vec![a]));r
    }
    #[test]
    fn borge_mark_uses_visible_projected_identity_without_fog_leak() {
        let mut r=room();r.floor_mut().unwrap().godbeast_event=corpse_core::GodbeastEvent {
            kind:"Borge".into(),sacrifice_actor_id:4.0.into(),activated:true.into(),..Default::default()
        }.into();
        let name=project(&r,&r.actors_at(1.)[0]).unwrap().name;let expected=format!("{name} · Borge-marked");
        let before=r.clone();let view=crate::observe::observe(&r);
        assert_eq!(view.cells.iter().find(|c|c.actor_id==Some(4.)).unwrap().label,expected);
        assert_eq!(view.creatures.iter().find(|c|c.id==4.).unwrap().name,expected);
        let report=crate::inspection_ui::observe_tile(&r,5,5);assert_eq!(report.title,expected);
        assert!(!serde_json::to_string(&report).unwrap().contains("Skin Walker"));assert_eq!(r,before);
        r.player.stats.perception=0.;for y in 1..28 {r.floor_mut().unwrap().tiles[y*79+4]="wall".into();}
        assert!(!crate::observe::observe(&r).creatures.iter().any(|c|c.id==4.));assert!(crate::inspection_ui::observe_tile(&r,5,5).creature.is_none());
        r.inventory=vec![corpse_core::borge::shard(201.)];let before=r.clone();
        assert_eq!(crate::pack::PackTarget::UseAt(201.).target_blocker(&r,5.,5.),Some("Choose a visible adjacent living creature."));assert_eq!(r,before);
    }
    #[test]
    fn v33_disguise_map_list_hover_inspector_and_rng_are_consistent() {
        let mut r=room();let before=r.clone();let a=&r.actors_at(1.)[0];let shown=project(&r,a).unwrap();
        let view=crate::observe::observe(&r);let cell=view.cells.iter().find(|c|c.actor_id==Some(4.)).unwrap();
        assert_eq!(cell.glyph,shown.glyph);assert_eq!(cell.label,shown.name);
        let creature=view.creatures.iter().find(|c|c.id==4.).unwrap();assert_eq!(creature.name,shown.name);assert_eq!(creature.max_hp,shown.max_hp);
        let report=crate::inspection_ui::observe_tile(&r,5,5);let serialized=serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("Skin Walker"));assert!(!serialized.contains("skin-walker"));assert!(!serialized.contains("376"));
        assert_eq!(report.title,shown.name);assert_eq!(r,before,"all observations are read-only");
        r.player.x=3.;r.player.y=3.;let view=crate::observe::observe(&r);assert_eq!(view.creatures.iter().find(|c|c.id==4.).unwrap().name,"Skin Walker");
        assert_eq!(crate::inspection_ui::observe_tile(&r,5,5).title,"Skin Walker");
        r.player.x=2.;r.player.y=2.;assert_eq!(crate::inspection_ui::observe_tile(&r,5,5).title,shown.name);
    }
    #[test]
    fn v33_telepathy_keeps_disguise_and_fog_does_not_admit_hidden_actor() {
        let mut r=room();r.player.conditions.insert("telepathic".into(),10.);
        r.floor_mut().unwrap().tiles[4*79+4]="wall".into();let a=&r.actors_at(1.)[0];let shown=project(&r,a).unwrap();
        assert_eq!(crate::inspection_ui::observe_tile(&r,5,5).title,shown.name);
        r.player.conditions.clear();r.player.stats.perception=0.;r.actors_at_mut(1.)[0].x=60.;r.actors_at_mut(1.)[0].y=20.;for y in 1..28 {r.floor_mut().unwrap().tiles[y*79+20]="wall".into();}
        assert!(!crate::observe::observe(&r).creatures.iter().any(|c|c.id==4.));
        assert!(crate::inspection_ui::observe_tile(&r,60,20).creature.is_none());
        r.player.conditions.insert("hallucinating".into(),2.);
        let report=crate::inspection_ui::observe_tile(&r,60,20);
        let labels=crate::observe::observe(&r).cells.iter().map(|c|(c.x,c.y,c.glyph.clone(),c.label.clone(),c.fg,c.bg)).collect::<Vec<_>>();
        let hidden=&mut r.actors_at_mut(1.)[0];hidden.name="Hidden different creature".into();hidden.glyph="X".into();hidden.color="#00ff00".into();hidden.stats.offense=9999.;hidden.hp=999.;
        assert_eq!(crate::inspection_ui::observe_tile(&r,60,20),report);
        assert_eq!(crate::observe::observe(&r).cells.iter().map(|c|(c.x,c.y,c.glyph.clone(),c.label.clone(),c.fg,c.bg)).collect::<Vec<_>>(),labels);
    }
    #[test]
    fn v33_hallucinations_are_repeatable_per_tick_and_do_not_reveal_hidden_state() {
        let mut r=room();r.player.conditions.insert("hallucinating".into(),2.);let before=r.clone();
        let a=&r.actors_at(1.)[0];let one=project(&r,a).unwrap();let two=project(&r,a).unwrap();assert_eq!(one,two);
        let report=crate::inspection_ui::observe_tile(&r,5,5);assert_eq!(report.title,one.name);assert_eq!(r,before);
        r.tick+=1.;assert_ne!(project(&r,&r.actors_at(1.)[0]).unwrap().name,one.name);
    }
    #[test]
    fn v33_hallucination_in_half_here_body_changes_shared_observation_only() {
        let mut r=room();r.actors_at_mut(1.)[0]=corpse_core::population::create_actor_revision("rat",4.,5.,5.,false,33.).unwrap();
        assert!(project(&r,&r.actors_at(1.)[0]).is_none());
        r.player.abilities.push(corpse_core::half_here::ID.into());r.player.active.push(corpse_core::half_here::ID.into());r.player.conditions.insert("halfHereId".into(),9.);
        let mut body=r.player.clone();body.id=9.;body.conditions.insert("halfHereClone".into(),1.);body.conditions.insert("halfHereOwner".into(),r.player.id);body.conditions.insert("hallucinating".into(),2.);r.actors_at_mut(1.).push(body);
        let before=r.clone();assert!(project(&r,&r.actors_at(1.)[0]).is_some());assert_eq!(r,before);assert_eq!(corpse_core::engine::condition(&r.player,"hallucinating"),0.);
    }
    #[test]
    fn v33_hallucinogenic_cloud_has_actual_gas_field_tint() {
        let mut r=room();corpse_core::environment::initialize(r.floor_mut().unwrap()).unwrap();
        let plain=crate::observe::observe(&r);let before=plain.cells.iter().find(|c|c.x==4&&c.y==2).unwrap().bg;
        corpse_core::environment::add_gas(r.floor_mut().unwrap(),2*79+4,"hallucinogenic",100.,0.).unwrap();
        let view=crate::observe::observe(&r);let cloud=view.cells.iter().find(|c|c.x==4&&c.y==2).unwrap();
        assert_ne!(cloud.bg,before);assert!(cloud.bg[0]>cloud.bg[1] && cloud.bg[2]>cloud.bg[1],"purple hallucination tint {:?}",cloud.bg);
    }
    #[test]
    fn v33_clean_water_blood_and_ichor_are_distinct_without_state_changes() {
        let mut r=room();let w=79;for x in 3..=5 {r.floor_mut().unwrap().tiles[2*w+x]="water".into();}
        let mut blood=vec![0.;w*29];blood[2*w+4]=3.;blood[2*w+5]=100.;r.floor_mut().unwrap().water_blood=blood.into();
        let before=r.clone();let view=crate::observe::observe(&r);
        let c=|x|view.cells.iter().find(|c|c.x==x&&c.y==2).unwrap();
        assert_ne!((c(3).fg,c(3).bg),(c(4).fg,c(4).bg));assert_ne!((c(4).fg,c(4).bg),(c(5).fg,c(5).bg));
        assert_eq!(c(3).glyph,c(4).glyph);let report=crate::inspection_ui::observe_tile(&r,4,2);assert!(report.lines.iter().any(|s|s.contains("blood clouds")));assert_eq!(r,before);
    }
}

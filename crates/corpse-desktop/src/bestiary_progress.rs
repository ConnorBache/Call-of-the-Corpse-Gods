//! Player-level discovery only: never part of a deterministic Run or replay.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::{Path, PathBuf}};
pub const ADDED: &str = "Bestiary Pages Added!";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record { version: u32, seen: BTreeSet<String>, unread: BTreeSet<String> }
impl Default for Record { fn default()->Self { Self { version:1, seen:BTreeSet::new(), unread:BTreeSet::new() } } }
pub struct Progress { record:Record, path:Option<PathBuf>, blocked:bool, pub error:Option<String>, reported_error:Option<String> }
impl Default for Progress { fn default()->Self { Self { record:Record::default(),path:None,blocked:false,error:None,reported_error:None } } }
impl Progress {
    pub fn path_for_profile(profile:&Path)->PathBuf {
        let mut name=profile.file_name().unwrap_or_default().to_os_string();name.push(".bestiary.json");profile.with_file_name(name)
    }
    pub fn load(path:PathBuf)->Self {
        let mut result=Self {path:Some(path.clone()),..Self::default()};
        let loaded=(|| {
            match std::fs::metadata(&path) {
                Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(Record::default()),
                Err(e)=>return Err(e.to_string()),
                Ok(m) if m.len()>1024*1024=>return Err("Bestiary record exceeds 1 MiB".into()),
                _=>{}
            }
            let record:Record=serde_json::from_slice(&std::fs::read(&path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            if record.version!=1 || !record.unread.is_subset(&record.seen) {return Err("Invalid Bestiary version or unread records".into());}
            Ok(record)
        })();
        match loaded {Ok(record)=>result.record=record,Err(e)=>{result.blocked=true;result.error=Some(format!("Bestiary could not load {}: {e}. Original file preserved.",path.display()));}}
        result
    }
    /// Surface each distinct failure once without erasing another operation's notice.
    pub fn report_error(&mut self,notice:&mut String) {
        if self.error!=self.reported_error {
            if let Some(error)=&self.error {
                if !notice.is_empty() {notice.push_str(" | ");}
                notice.push_str(error);
            }
            self.reported_error=self.error.clone();
        }
    }
    pub fn seen(&self,id:&str)->bool {self.record.seen.contains(id)}
    pub fn unread(&self,id:&str)->bool {self.record.unread.contains(id)}
    pub fn has_unread(&self)->bool {!self.record.unread.is_empty()}
    fn save(&mut self) {
        if self.blocked {return;}
        let Some(path)=self.path.as_ref() else{return;};
        let result=(|| {
            if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
            crate::appearance_store::atomic_write(path,&serde_json::to_string_pretty(&self.record).map_err(|e|e.to_string())?)
        })();
        self.error=result.err().map(|e:String|format!("Bestiary could not save {}: {e}. Progress is only in memory.",path.display()));
    }
    /// O(K log S) set updates plus O(S) atomic serialization on changes only.
    pub fn discover<'a>(&mut self,ids:impl IntoIterator<Item=&'a str>)->bool {
        let mut changed=false;
        for id in ids {if self.record.seen.insert(id.to_owned()) {self.record.unread.insert(id.to_owned());changed=true;}}
        if changed {self.save();} changed
    }
    pub fn view(&mut self,id:&str) {if self.record.unread.remove(id) {self.save();}}
}
/// Admit identities only from rows already rendered as directly visible. Disguises
/// unlock their apparent page; hallucinations/sensing never disclose true species.
/// O(A + V log V + V*C), C bounded authored catalog, without Run/RNG mutation.
pub fn observed_ids(run:&corpse_core::Run,view:&crate::render::View,known:&[String])->Vec<String> {
    let rows:BTreeSet<_>=view.creatures.iter().filter(|c|c.state!="sensed" && c.state!="In mind" && c.state!="Sleeping" && c.hallucination.is_none()).map(|c|c.id.to_bits()).collect();
    run.actors_at(run.depth).iter().filter(|a| rows.contains(&a.id.to_bits())).filter_map(|a| {
        let shown=crate::creature_presentation::project(run,a);
        let a=shown.as_ref().unwrap_or(a);
        crate::manual::page_id_for_actor(a).filter(|id|known.iter().any(|known|known.as_str()==*id)).map(str::to_owned)
    }).collect()
}
/// Presentation-only Chronicle inserts, anchored to canonical archive positions.
#[derive(Default)]
pub struct Notices { identity:Option<String>, events:Vec<usize> }
impl Notices {
    fn count(run:&corpse_core::Run)->usize {run.log_archive.as_ref().map_or(run.log.len(),|a|a.events.len())}
    pub fn observe(&mut self,run:&corpse_core::Run,added:bool) {
        let identity=run.run_id.as_ref().cloned().unwrap_or_else(||run.seed.clone());
        if self.identity.as_ref()!=Some(&identity) || self.events.last().is_some_and(|at|*at>Self::count(run)) {self.events.clear();self.identity=Some(identity);}
        if added {self.events.push(Self::count(run));}
    }
    pub fn apply(&self,run:&corpse_core::Run,view:&mut crate::render::View) {
        let count=Self::count(run);let base=count.saturating_sub(view.log.len());
        let mut kinds=run.log_kinds.as_ref().cloned().unwrap_or_else(||vec!["unknown".into();view.log.len()]);
        for &at in self.events.iter().rev().filter(|&&at|at>=base&&at<=count) {let i=(at-base).min(view.log.len());view.log.insert(i,ADDED.into());kinds.insert(i.min(kinds.len()),"bestiary".into());}
        view.log_groups=crate::chronicle::compact(&view.log,&kinds);
    }
    pub fn history(&self,run:&corpse_core::Run)->Vec<String> {
        crate::observe::history_with_notices(run,&self.events)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn path(label:&str)->PathBuf {static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);std::env::temp_dir().join(format!("bestiary-{}-{}-{}.json",std::process::id(),label,NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)))}
    #[test]
    fn durable_discovery_unread_and_duplicate_batches() {
        let path=path("durable");let mut progress=Progress::load(path.clone());
        assert!(!progress.has_unread());assert!(progress.discover(["blorb","vampire","blorb"]));
        assert!(!progress.discover(["blorb","vampire"]));
        let mut restored=Progress::load(path.clone());assert!(restored.seen("blorb"));assert!(restored.unread("vampire"));
        restored.view("blorb");restored.view("unknown");
        let restored=Progress::load(path.clone());assert!(restored.seen("blorb"));assert!(!restored.unread("blorb"));assert!(restored.unread("vampire"));std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn malformed_record_is_never_overwritten() {
        let path=path("malformed");std::fs::write(&path,"not json").unwrap();let mut progress=Progress::load(path.clone());
        assert!(progress.error.is_some());assert!(progress.discover(["blorb"]));progress.view("blorb");
        assert_eq!(std::fs::read_to_string(&path).unwrap(),"not json");std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn failed_write_keeps_memory_and_reports_error() {
        let dir=path("directory");std::fs::create_dir(&dir).unwrap();let mut progress=Progress {path:Some(dir.clone()),..Progress::default()};
        assert!(progress.discover(["blorb"]));assert!(progress.seen("blorb"));assert!(progress.error.is_some());std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn explicit_profiles_are_isolated() {assert_eq!(Progress::path_for_profile(Path::new("saves/alice.json")),Path::new("saves/alice.json.bestiary.json"));assert_ne!(Progress::path_for_profile(Path::new("alice.json")),Progress::path_for_profile(Path::new("bob.json")));}
    #[test]
    fn observed_rows_only_no_hidden_actor_or_rng_mutation() {
        let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:bestiary","Unblessed",false,50.).unwrap();
        r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let w=79;let h=29;let mut tiles=vec!["wall".into();w*h];for y in 1..h-1 {for x in 1..w-1 {tiles[y*w+x]="floor".into();}}
        r.floors=vec![std::sync::Arc::new(corpse_core::Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()})];
        let a=corpse_core::population::create_actor_revision("blorb",4.,5.,5.,false,50.).unwrap();r.actors.clear();r.actors.insert("1".into(),std::sync::Arc::new(vec![a]));
        let known=vec!["blorb".into()];let before=r.clone();let mut view=crate::observe::observe(&r);
        assert_eq!(observed_ids(&r,&view,&known),known);assert_eq!(r,before);
        view.creatures[0].state="sensed".into();assert!(observed_ids(&r,&view,&known).is_empty());
        view.creatures.clear();assert!(observed_ids(&r,&view,&known).is_empty());assert_eq!(r,before);
    }
    #[test]
    fn notification_once_per_discovery_and_no_run_mutation() {
        let r=corpse_core::startup::new_run_shell_revision("0-5-11:notice","Unblessed",false,50.).unwrap();let before=r.clone();
        let mut notices=Notices::default();notices.observe(&r,true);notices.observe(&r,false);
        let mut view=crate::observe::observe(&r);notices.apply(&r,&mut view);assert_eq!(view.log.iter().filter(|m|m.as_str()==ADDED).count(),1);
        assert_eq!(notices.history(&r).iter().filter(|m|m.as_str()==ADDED).count(),1);assert_eq!(r,before);
    }
    fn admitted_view(run:&corpse_core::Run)->crate::render::View {
        let mut view=crate::observe::observe(run);
        view.creatures=vec![crate::render::Creature {
            perception_range:None,identity_outline:None,hallucination:None,shared_resources:false,invulnerable:false,
            id:4.,name:"Admitted apparent actor".into(),glyph:"?".into(),color:[255;3],state:"wandering".into(),
            distance:3.,hp:10.,max_hp:10.,energy:10.,max_energy:10.,uses_energy:false,statuses:vec![],meters:vec![],
        }];view
    }
    #[test]
    fn every_population_template_has_expected_authored_page_or_explicit_no_page() {
        let manual=crate::manual::Manual::new();let known=manual.ids();
        let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:all-pages","Unblessed",false,50.).unwrap();
        let mut view=admitted_view(&r);let population=corpse_core::population::catalog();
        let mut fixtures:Vec<(corpse_core::Actor,Option<String>)>=population.actors.iter().map(|(id,actor)| {
            let expected=if ["ichor-fish","pale:Greater Jackal"].contains(&id.as_str()) {None}else{Some(id.clone())};
            (actor.clone(),expected)
        }).collect();
        fixtures.extend(population.fungi.iter().map(|a|(a.clone(),Some("bastidion".into()))));
        fixtures.extend(population.zillennia.iter().map(|a|(a.clone(),Some(format!("zillennia:{}",a.zillennia_quality.as_ref().unwrap())))));
        fixtures.extend(corpse_core::new_creatures::templates().iter().map(|a|(a.clone(),a.creature_kind.as_ref().cloned())));
        for (mut actor,expected) in fixtures {
            // Test canonical identity mapping for all templates, including pre-R50 Pale names.
            assert_eq!(crate::manual::page_id_for_actor(&actor),expected.as_deref(),"{}",actor.name);
            corpse_core::revision50_creature_identity::apply_actor(&mut actor);
            assert_eq!(crate::manual::page_id_for_actor(&actor),expected.as_deref(),"renamed {}",actor.name);
            actor.id=4.;actor.x=50.;actor.y=20.;let name=actor.name.clone();
            r.actors.insert(r.depth.to_string(),std::sync::Arc::new(vec![actor.clone()]));
            // Temporarily use revision32 for the identity adapter test: disguise
            // projection is separately exercised below at current revision50.
            r.simulation_revision=32.0.into();
            let before=r.clone();let actual=observed_ids(&r,&view,known);
            assert_eq!(actual,expected.clone().into_iter().collect::<Vec<_>>(),"observed {name}");
            let row=view.creatures.pop().unwrap();assert!(observed_ids(&r,&view,known).is_empty(),"hidden {name}");view.creatures.push(row);
            assert_eq!(r,before);
        }
    }
    #[test]
    fn quality_alias_and_vampire_bat_map_to_the_authored_species_page() {
        let mut actor=corpse_core::population::catalog().zillennia[32].clone();
        actor.creature_kind="zillennia-timeless".to_owned().into();
        assert_eq!(crate::manual::page_id_for_actor(&actor),Some("zillennia:timeless"));
        let mut vampire=corpse_core::population::create_actor_revision("vampire",4.,5.,5.,false,50.).unwrap();
        vampire.conditions.insert("monsterVampireBat".into(),1.);
        assert!(corpse_core::monster_vampire::flying(&vampire));
        assert_eq!(crate::manual::page_id_for_actor(&vampire),Some("vampire"));
        let mut run=corpse_core::startup::new_run_shell_revision("0-5-11:bat-page","Unblessed",false,50.).unwrap();
        let view=admitted_view(&run);run.actors.insert(run.depth.to_string(),std::sync::Arc::new(vec![vampire]));
        assert_eq!(observed_ids(&run,&view,crate::manual::Manual::new().ids()),["vampire"]);
    }
    #[test]
    fn disguised_actor_unlocks_only_apparent_page_and_hallucinations_unlock_nothing() {
        let mut run=corpse_core::startup::new_run_shell_revision("0-5-11:disguised-page","Unblessed",false,50.).unwrap();
        let mut view=admitted_view(&run);let known=crate::manual::Manual::new().ids().to_vec();
        let mut actor=corpse_core::population::create_actor_revision("skin-walker",4.,60.,25.,false,50.).unwrap();
        actor.conditions.insert("skinWalkerDisguise".into(),0.);
        run.player.x=2.;run.player.y=2.;run.actors.insert(run.depth.to_string(),std::sync::Arc::new(vec![actor]));
        let before=run.clone();assert_eq!(observed_ids(&run,&view,&known),["rat"]);
        let row=view.creatures[0].clone();view.creatures[0].hallucination=Some(std::sync::Arc::new(std::array::from_fn(|_|row.clone())));
        assert!(observed_ids(&run,&view,&known).is_empty());assert_eq!(run,before);
    }

    #[test]
    fn error_notice_is_once_per_failure_and_preserves_other_notices() {
        let mut progress=Progress::default();progress.error=Some("Bestiary write failed".into());
        let mut notice="Save file changed".to_owned();progress.report_error(&mut notice);
        assert_eq!(notice,"Save file changed | Bestiary write failed");
        notice="A later notice".into();progress.report_error(&mut notice);assert_eq!(notice,"A later notice");
        progress.error=None;progress.report_error(&mut notice);assert_eq!(notice,"A later notice");
        progress.error=Some("Bestiary write failed".into());progress.report_error(&mut notice);
        assert_eq!(notice,"A later notice | Bestiary write failed");
    }

}

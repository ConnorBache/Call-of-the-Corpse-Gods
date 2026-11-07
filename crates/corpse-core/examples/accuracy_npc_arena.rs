//! Analysis-only native ally-versus-hostile duels. Both combatants are unmodified
//! species constructors except explicit candidate stats. The player is sealed in
//! a separate cell and only Waits; no artificial HP, Dodge or Accuracy is used.
//! O(T*C*D) native dispatch cost, T trials, C<=256 commands, D full validation,
//! pathfinding, visibility and environmental work; fixed81 cells, O(T) outcomes.
//! No claim that two naturally friendly hostiles ordinarily fight each other.
use corpse_core::{Actor,ActorLastSeen,Command,Field,Floor,Run,combat,engine,population,startup};
use serde::Deserialize;
use serde_json::{Value,json};
use std::{io::{self,BufRead,Write},sync::Arc};
#[derive(Default,Deserialize)]
struct Patch {accuracy:Option<f64>,dodge:Option<f64>,offense:Option<f64>,max_hp:Option<f64>}
#[derive(Deserialize)]
struct Request {left:String,right:String,left_patch:Option<Patch>,right_patch:Option<Patch>,#[serde(default)]reverse:bool,#[serde(default)]packet_probe:bool,#[serde(default="trials")]trials:usize}
fn trials()->usize{64}
fn patch(a:&mut Actor,p:&Option<Patch>){if let Some(p)=p{if let Some(n)=p.accuracy{a.stats.accuracy=n;}if let Some(n)=p.dodge{a.stats.dodge=n;}if let Some(n)=p.offense{a.stats.offense=n;}if let Some(n)=p.max_hp{a.hp=n;a.max_hp=n;}}}
fn live(a:Option<&Actor>)->bool{a.is_some_and(|a|a.hp>0.&&!a.dead.value_or(false))}
fn scene(q:&Request,seed:u32)->Result<Run,String>{
 let mut r=startup::new_run_shell_revision("0-5-11:accuracy-npc33","Unblessed",false,33.)?;
 r.player.x=1.;r.player.y=1.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;r.rng=seed as f64;
 let w=9;let mut tiles=vec!["wall".into();w*w];tiles[10]="floor".into();for y in 1..8{for x in 3..8{tiles[y*w+x]="floor".into();}}
 let mut f=Floor{depth:1.,width:9.0.into(),height:9.0.into(),domain:"Dungeon".into(),tiles,lights:vec![100.;w*w],seen:vec![true;w*w],..Default::default()};corpse_core::environment::initialize(&mut f)?;r.floors=vec![Arc::new(f)];
 let mut a=population::create_actor_revision(&q.left,4.,4.,3.,false,33.)?;let mut b=population::create_actor_revision(&q.right,5.,5.,3.,false,33.)?;
 patch(&mut a,&q.left_patch);patch(&mut b,&q.right_patch);
 a.state=if q.reverse||q.packet_probe{"hunting"}else{"ally"}.into();b.state=if q.reverse&&!q.packet_probe{"ally"}else{"hunting"}.into();
 a.conditions.insert("targetId".into(),5.);b.conditions.insert("targetId".into(),4.);a.last_seen=ActorLastSeen{x:5.,y:3.}.into();b.last_seen=ActorLastSeen{x:4.,y:3.}.into();
 r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![a,b]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=6.;Ok(r)
}
fn run(q:Request)->Result<Value,String>{
 if q.trials==0||q.trials>4096{return Err("trials must be1..4096".into());}
 let mut rows=Vec::new();let mut probabilities=None;
 for i in 0..q.trials{
  let mut r=scene(&q,corpse_core::rng::seed_hash(&format!("npc33:{}:{}:{}:{i}",q.left,q.right,q.reverse)))?;
  let a=engine::actor(&r,4.).unwrap();let b=engine::actor(&r,5.).unwrap();let hp=[a.max_hp,b.max_hp];
  probabilities=Some(json!({"left_to_right":combat::hit_chance(a.stats.accuracy,b.stats.dodge,0.),"right_to_left":combat::hit_chance(b.stats.accuracy,a.stats.dodge,0.),"left_to_right_range4":combat::hit_chance(a.stats.accuracy,b.stats.dodge,4.),"right_to_left_range4":combat::hit_chance(b.stats.accuracy,a.stats.dodge,4.)}));
  let mut damage=[0.,0.];let mut commands=0;
  while commands<256&&live(engine::actor(&r,4.))&&live(engine::actor(&r,5.)){
   corpse_core::combat_audit::begin();
   if q.packet_probe{
    // Controlled reciprocal packets, not an AI hostility rule or world-time model.
    let ids=if (i%2==0)^q.reverse{[4.,5.]}else{[5.,4.]};
    engine::attack(&mut r,ids[0],ids[1],1.).map_err(|e|e.to_string())?;
    if live(engine::actor(&r,ids[1])){engine::attack(&mut r,ids[1],ids[0],1.).map_err(|e|e.to_string())?;}
   }else{engine::dispatch(&mut r,&Command::Wait).map_err(|e|e.to_string())?;}
   for p in corpse_core::combat_audit::take(){if p.victim==4.{damage[0]+=p.amount;}else if p.victim==5.{damage[1]+=p.amount;}}
   commands+=1;
  }
  let l=live(engine::actor(&r,4.));let rr=live(engine::actor(&r,5.));rows.push(json!({"left_won":l&&!rr,"right_won":rr&&!l,"censored":l&&rr,"commands":commands,"left_gross_fraction":damage[0]/hp[0],"right_gross_fraction":damage[1]/hp[1],"player_hp":r.player.hp}));
 }
 let count=|key:&str|rows.iter().filter(|r|r[key].as_bool()==Some(true)).count();let mean=|key:&str|rows.iter().map(|r|r[key].as_f64().unwrap()).sum::<f64>()/rows.len()as f64;
 Ok(json!({"left":q.left,"right":q.right,"reverse":q.reverse,"mode":if q.packet_probe{"hostile_reciprocal_packet_probe"}else{"native_ally_duel"},"trials":q.trials,"opening_probability":probabilities,"left_wins":count("left_won"),"right_wins":count("right_won"),"censored":count("censored"),"mean_commands":mean("commands"),"mean_left_gross_fraction":mean("left_gross_fraction"),"mean_right_gross_fraction":mean("right_gross_fraction"),"player_untouched":rows.iter().all(|r|r["player_hp"]==100.)}))
}
fn main(){for line in io::stdin().lock().lines(){let result=line.map_err(|e|e.to_string()).and_then(|l|serde_json::from_str::<Request>(&l).map_err(|e|e.to_string())).and_then(run).unwrap_or_else(|e|json!({"error":e}));println!("{result}");io::stdout().flush().unwrap();}}

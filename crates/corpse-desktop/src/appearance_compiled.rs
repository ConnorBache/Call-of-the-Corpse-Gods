//! Immutable derived appearance data. JSON remains authoritative at edit boundaries.
//! Compile O(BD + PD + 3*terrain material bytes/fields); see docs for depth/path bounds.
//! Retained O(scalar
//! path/text bytes + resolved material bytes). No coordinates, light, Run or RNG.
use crate::render::Rgb;
use serde_json::Value;
use std::collections::HashMap;

/// Immutable parsed global controls. Same absent/wrong-type fallback as number():0.
#[derive(Clone,Copy,Debug,Default)]
pub struct GlobalTone {pub shadows:f64,pub midtones:f64,pub highlights:f64,pub saturation:f64,pub contrast:f64,pub exposure:f64,pub temperature:f64,pub gamma:f64,pub minimum_brightness:f64,pub maximum_brightness:f64}
impl GlobalTone { fn from_raw(raw:&Value)->Self {let n=|key:&str|super::get(raw,&format!("global.{key}")).and_then(Value::as_f64).unwrap_or(0.); Self{shadows:n("shadows"),midtones:n("midtones"),highlights:n("highlights"),saturation:n("saturation"),contrast:n("contrast"),exposure:n("exposure"),temperature:n("temperature"),gamma:n("gamma"),minimum_brightness:n("minimumBrightness"),maximum_brightness:n("maximumBrightness")}}}
#[derive(Clone,Debug)]
enum Scalar {Number(f64),Text{value:String,rgb:Rgb},Bool(bool)}
#[derive(Clone,Copy,Debug)]
pub enum MaterialParent {None,Floor,Door}
#[derive(Clone,Debug)]
struct Material {
    glyph:Option<String>,color:Option<Rgb>,background:Option<Rgb>,edge:Option<Rgb>,edge_nonempty:bool,
    raw_color:Option<Rgb>,raw_background:Option<Rgb>,
    brightness:f64,saturation:f64,contrast:f64,opacity:f64,color_variation:f64,brightness_variation:f64,animation_speed:f64,lighting_response:f64,
    color_default:bool,background_default:bool,edge_default:bool,defaults_exist:bool,
}
/// Cheap borrowed pre-light settings. Per-cell overrides change only this Copy view.
#[derive(Clone,Copy,Debug)]
pub struct MaterialView<'a> {
    pub glyph:Option<&'a str>,pub color:Option<Rgb>,pub background:Option<Rgb>,pub edge:Option<Rgb>,pub edge_nonempty:bool,
    pub raw_color:Option<Rgb>,pub raw_background:Option<Rgb>,
    pub brightness:f64,pub saturation:f64,pub contrast:f64,pub opacity:f64,pub color_variation:f64,pub brightness_variation:f64,pub animation_speed:f64,pub lighting_response:f64,
    pub color_default:bool,pub background_default:bool,pub edge_default:bool,pub defaults_exist:bool,
}
impl Default for MaterialView<'_> {
    fn default()->Self{Self{glyph:None,color:None,background:None,edge:None,edge_nonempty:false,raw_color:None,raw_background:None,brightness:1.,saturation:1.,contrast:1.,opacity:1.,color_variation:1.,brightness_variation:1.,animation_speed:1.,lighting_response:1.,color_default:true,background_default:true,edge_default:true,defaults_exist:false}}
}
fn strict_hex(s:&str)->Rgb {
    if s.len()==7&&s.starts_with('#'){let n=u32::from_str_radix(&s[1..],16).unwrap_or(0xffffff);[(n>>16)as u8,(n>>8)as u8,n as u8]}else{[255;3]}
}
impl Material {
    fn from_value(v:&Value,defaults:Option<&Value>)->Self {
        let n=|k|v.get(k).and_then(Value::as_f64).unwrap_or(1.);
        let text=|k|v.get(k).and_then(Value::as_str);
        // Compare ORIGINAL resolved JSON BEFORE parsing colors/numbers. Hex case
        // and spelling remain significant to custom/default override semantics.
        let absent=Value::Null;let d=defaults.unwrap_or(&absent);
        Self{glyph:text("glyph").map(str::to_owned),color:text("color").map(strict_hex),background:text("background").map(strict_hex),edge:text("edgeColor").map(strict_hex),edge_nonempty:text("edgeColor").is_some_and(|s|!s.is_empty()),raw_color:text("color").map(crate::color::hex),raw_background:text("background").map(crate::color::hex),brightness:n("brightness"),saturation:n("saturation"),contrast:n("contrast"),opacity:n("opacity"),color_variation:n("colorVariation"),brightness_variation:n("brightnessVariation"),animation_speed:n("animationSpeed"),lighting_response:n("lightingResponse"),color_default:v["color"]==d["color"],background_default:v["background"]==d["background"],edge_default:v["edgeColor"]==d["edgeColor"],defaults_exist:defaults.is_some()}
    }
    fn view(&self)->MaterialView<'_>{MaterialView{glyph:self.glyph.as_deref(),color:self.color,background:self.background,edge:self.edge,edge_nonempty:self.edge_nonempty,raw_color:self.raw_color,raw_background:self.raw_background,brightness:self.brightness,saturation:self.saturation,contrast:self.contrast,opacity:self.opacity,color_variation:self.color_variation,brightness_variation:self.brightness_variation,animation_speed:self.animation_speed,lighting_response:self.lighting_response,color_default:self.color_default,background_default:self.background_default,edge_default:self.edge_default,defaults_exist:self.defaults_exist}}
}
#[derive(Debug,Default)]
pub struct Compiled {pub global_tone:GlobalTone,scalars:HashMap<String,Scalar>,equal:HashMap<String,bool>,materials:[HashMap<String,Material>;3]}
/// Mirrors Profile::material exactly using RAW trees, never Profile::defaults or
/// another compiled accessor. This avoids recursive default OnceLock initialization.
fn resolve(raw:&Value,defaults:&Value,key:&str,parent:Option<&str>)->Option<Value>{
    let own=super::get(raw,&format!("terrain.{key}"))?;
    let Some(parent)=parent.filter(|p|*p!=key) else{return Some(own.clone())};
    let(Some(base),Some(above),Some(parent_base))=(super::get(defaults,&format!("terrain.{key}")),super::get(raw,&format!("terrain.{parent}")),super::get(defaults,&format!("terrain.{parent}")))else{return Some(own.clone())};
    let mut result=own.clone();for(field,default)in base.as_object()?{if own.get(field)==Some(default)&&above.get(field)!=parent_base.get(field){if let Some(v)=above.get(field){result[field]=v.clone();}}}Some(result)
}
impl Compiled {
    pub fn build(raw:&Value,defaults:&Value)->Self {
        fn visit(v:&Value,path:&mut String,out:&mut HashMap<String,Scalar>){
            match v {
                Value::Object(m)=>for(k,v)in m{if k.contains('.')||matches!(k.as_str(),"__proto__"|"constructor"|"prototype"){continue}let old=path.len();if old>0{path.push('.')}path.push_str(k);visit(v,path,out);path.truncate(old);},
                Value::String(s)=>{out.insert(path.clone(),Scalar::Text{value:s.clone(),rgb:crate::color::hex(s)});},
                Value::Number(n)=>if let Some(n)=n.as_f64(){out.insert(path.clone(),Scalar::Number(n));},
                Value::Bool(b)=>{out.insert(path.clone(),Scalar::Bool(*b));},_=>{}
            }
        }
        let mut result=Self::default();result.global_tone=GlobalTone::from_raw(raw);visit(raw,&mut String::new(),&mut result.scalars);
        fn equality(v:&Value,path:&mut String,defaults:&Value,out:&mut HashMap<String,bool>){
            if !path.is_empty(){out.insert(path.clone(),super::get(defaults,path)==Some(v));}
            if let Value::Object(m)=v{for(k,v)in m{if k.contains('.')||matches!(k.as_str(),"__proto__"|"constructor"|"prototype"){continue}let old=path.len();if old>0{path.push('.')}path.push_str(k);equality(v,path,defaults,out);path.truncate(old);}}
        }
        equality(raw,&mut String::new(),defaults,&mut result.equal);
        if let Some(terrain)=raw.get("terrain").and_then(Value::as_object){for key in terrain.keys(){for(index,parent)in [None,Some("floor"),Some("fixture:door")].into_iter().enumerate(){if let Some(v)=resolve(raw,defaults,key,parent){let d=resolve(defaults,defaults,key,parent);result.materials[index].insert(key.clone(),Material::from_value(&v,d.as_ref()));}}}}
        result
    }
    pub fn is_default(&self,p:&str)->bool{corpse_core::timing::count(corpse_core::timing::Count::ProfileResolvedScalar,1);self.equal.get(p).copied().unwrap_or(true)}
    pub fn number(&self,p:&str)->f64{corpse_core::timing::count(corpse_core::timing::Count::ProfileResolvedScalar,1);match self.scalars.get(p){Some(Scalar::Number(v))=>*v,_=>0.}}
    pub fn boolean(&self,p:&str)->bool{corpse_core::timing::count(corpse_core::timing::Count::ProfileResolvedScalar,1);matches!(self.scalars.get(p),Some(Scalar::Bool(true)))}
    pub fn text(&self,p:&str)->&str{corpse_core::timing::count(corpse_core::timing::Count::ProfileResolvedScalar,1);match self.scalars.get(p){Some(Scalar::Text{value,..})=>value,_=>""}}
    pub fn rgb(&self,p:&str)->Rgb{corpse_core::timing::count(corpse_core::timing::Count::ProfileResolvedScalar,1);match self.scalars.get(p){Some(Scalar::Text{rgb,..})=>*rgb,_=>[0;3]}}
    pub fn material(&self,key:&str,parent:MaterialParent)->Option<MaterialView<'_>>{corpse_core::timing::count(corpse_core::timing::Count::ProfileResolvedMaterial,1);self.materials[match parent{MaterialParent::None=>0,MaterialParent::Floor=>1,MaterialParent::Door=>2}].get(key).map(Material::view)}
    /// Logical retained payload/capacity accounting, not allocator/RSS measurement.
    pub fn storage_stats(&self)->serde_json::Value{
        let text_bytes:usize=self.scalars.iter().map(|(k,v)|k.capacity()+match v{Scalar::Text{value,..}=>value.capacity(),_=>0}).sum();
        let material_bytes:usize=self.materials.iter().flat_map(|m|m.iter()).map(|(k,v)|k.capacity()+v.glyph.as_ref().map_or(0,String::capacity)).sum();
        serde_json::json!({"global_tone_bytes":std::mem::size_of::<GlobalTone>(),"equality_entries":self.equal.len(),"equality_key_capacity_bytes":self.equal.keys().map(String::capacity).sum::<usize>(),"scalar_entries":self.scalars.len(),"scalar_capacity":self.scalars.capacity(),"scalar_owned_string_capacity_bytes":text_bytes,"material_entries":self.materials.iter().map(HashMap::len).sum::<usize>(),"material_capacity":self.materials.iter().map(HashMap::capacity).sum::<usize>(),"material_owned_string_capacity_bytes":material_bytes,"scalar_entry_size":std::mem::size_of::<(String,Scalar)>(),"material_entry_size":std::mem::size_of::<(String,Material)>(),"scope":"logical entry sizes/capacities only; hash control bytes, allocator rounding and Arc overhead excluded; not RSS"})
    }
}

#[cfg(test)] mod global_tone_raw_tests {
    use super::*;
    #[test] fn compiled_global_tone_missing_wrong_type_and_signed_zero(){
        for raw in [serde_json::json!({}),serde_json::json!({"global":{"gamma":"wrong","shadows":null,"exposure":-0.0}})] {
            let t=GlobalTone::from_raw(&raw);assert_eq!(t.gamma.to_bits(),0.0f64.to_bits());assert_eq!(t.shadows.to_bits(),0.0f64.to_bits());
            assert_eq!(t.exposure.to_bits(),super::super::get(&raw,"global.exposure").and_then(Value::as_f64).unwrap_or(0.).to_bits());
        }
    }
}

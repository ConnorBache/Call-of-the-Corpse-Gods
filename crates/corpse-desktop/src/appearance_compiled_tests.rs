#[cfg(test)]
mod compiled_tests {
    use super::*;
    fn variants()->Vec<Profile>{
        let d=Profile::default();let changed=d.set("terrain.floor.opacity",serde_json::json!(0.37)).unwrap().set("terrain.floor.color",serde_json::json!("#00aaff")).unwrap();
        let upper=d.text("terrain.ground:tiled.color").to_ascii_uppercase();assert_ne!(upper,d.text("terrain.ground:tiled.color"));
        vec![d.clone(),changed.clone(),d.set("terrain.ground:tiled.color",upper.into()).unwrap(),changed.reset_group("terrain").unwrap(),Profile::parse(&changed.json()).unwrap(),changed.named("Cache clone name")]
    }
    fn paths(v:&Value,p:&str,out:&mut Vec<String>){if !p.is_empty(){out.push(p.into())}if let Value::Object(m)=v{for(k,v)in m{if !k.contains('.')&&!forbidden(k){paths(v,&if p.is_empty(){k.clone()}else{format!("{p}.{k}")},out)}}}}
    #[test]
    fn compiled_scalars_exact_raw_lookup_and_default_equality(){
        for p in variants(){let mut keys=vec![];paths(p.data(),"",&mut keys);keys.push("missing.path".into());
            for key in keys{let v=get(p.data(),&key);assert_eq!(p.number(&key).to_bits(),v.and_then(Value::as_f64).unwrap_or(0.).to_bits(),"{key}");assert_eq!(p.text(&key),v.and_then(Value::as_str).unwrap_or(""));assert_eq!(p.boolean(&key),v.and_then(Value::as_bool).unwrap_or(false));assert_eq!(p.rgb(&key),crate::color::hex(v.and_then(Value::as_str).unwrap_or("")));assert_eq!(p.is_default(&key),v==get(Profile::defaults().data(),&key),"{key}");}
            assert_eq!(Profile::parse(&p.json()).unwrap().data(),p.data());
        }
    }
    #[test]
    fn compiled_material_exact_inheritance_and_case_sensitive_customization(){
        for p in variants(){for key in p.data()["terrain"].as_object().unwrap().keys(){for(parent,id)in [(None,MaterialParent::None),(Some("floor"),MaterialParent::Floor),(Some("fixture:door"),MaterialParent::Door)]{
            let v=p.material(key,parent);let got=p.resolved_material(key,id);assert_eq!(v.is_some(),got.is_some());
            if let(Some(v),Some(g))=(v,got){let d=Profile::defaults().material(key,parent).unwrap_or(Value::Null);
                assert_eq!(g.glyph,v["glyph"].as_str());assert_eq!(g.opacity.to_bits(),v["opacity"].as_f64().unwrap_or(1.).to_bits());assert_eq!(g.animation_speed.to_bits(),v["animationSpeed"].as_f64().unwrap_or(1.).to_bits());
                assert_eq!(g.color_default,v["color"]==d["color"]);assert_eq!(g.background_default,v["background"]==d["background"]);assert_eq!(g.edge_default,v["edgeColor"]==d["edgeColor"]);
            }
        }}}
        let d=Profile::default();let path="terrain.ground:tiled.color";let p=d.set(path,d.text(path).to_ascii_uppercase().into()).unwrap();assert_eq!(p.rgb(path),d.rgb(path));assert!(!p.is_default(path));assert!(!p.resolved_material("ground:tiled",MaterialParent::Floor).unwrap().color_default);
    }
    #[test]
    fn compiled_clone_edit_reset_named_never_stale(){
        let p=Profile::default();let c=p.clone();assert!(std::sync::Arc::ptr_eq(&p.1,&c.1));
        let changed=c.set("terrain.floor.opacity",serde_json::json!(0.21)).unwrap();assert!(!std::sync::Arc::ptr_eq(&p.1,&changed.1));assert_eq!(changed.resolved_material("ground:tiled",MaterialParent::Floor).unwrap().opacity,0.21);assert_eq!(p.number("terrain.floor.opacity"),1.);
        let reset=changed.reset("terrain.floor.opacity").unwrap();assert_eq!(reset.data(),p.data());assert_eq!(reset.number("terrain.floor.opacity"),1.);
        let named=p.named("Different cached name");assert_eq!(named.text("presetName"),"Different cached name");assert_ne!(named.text("presetName"),p.text("presetName"));
        let entity=controls().iter().find(|c|c.path.starts_with("entities.")&&c.path.ends_with(".color")&&c.path.split('.').count()==4).unwrap();
        let changed=p.set(&entity.path,serde_json::json!("#00aaff")).unwrap();let reset=changed.reset(&entity.path).unwrap();let base=entity.path.rsplit_once('.').unwrap().0;let inherited=format!("{base}.inherited");assert_eq!(reset.boolean(&inherited),get(reset.data(),&inherited).and_then(Value::as_bool).unwrap_or(false));
    }
    #[test]
    #[ignore="lead-controlled construction/retained-data diagnostic, outside frame timing"]
    fn compiled_appearance_construction_audit(){
        use std::time::Instant;let raw=Profile::defaults().data().clone();let mut rows=vec![];
        for _ in 0..20{let input=raw.clone();let t=Instant::now();let c=compiled::Compiled::build(&input,&raw);let compile_ns=t.elapsed().as_nanos();let stats=c.storage_stats();drop(c);
            let input=raw.clone();let t=Instant::now();let p=Profile::from_value(input).unwrap();let validated_constructor_ns=t.elapsed().as_nanos();rows.push(serde_json::json!({"compile_ns":compile_ns,"validated_constructor_ns":validated_constructor_ns,"stats":stats,"raw_json_bytes":p.json().len()}));}
        println!("COMPILED_APPEARANCE_AUDIT {}",serde_json::to_string(&rows).unwrap());
    }
}

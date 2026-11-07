use super::*;
fn original_illumination_in_plane(r:&Run,target:Point,exclude:Option<f64>,dream:bool)->f64 {
    let _timing_function = crate::timing::span("vision.lighting");
    let Some(f) = crate::dreamwalking::floor_in_plane(r,dream).or_else(||r.floor()) else { return -100.0 };
    if !bounds(f, target) {
        return -100.0;
    }
    let mut result = f
        .lights
        .get(target.y as usize * f.width() + target.x as usize)
        .copied()
        .unwrap_or(0.0)
        .max(-100.0);
    if let Some(e) = f.environment.as_ref() {
        for y in (target.y - 7).max(0)..=(target.y + 7).min(f.height() as i32 - 1) {
            for x in (target.x - 7).max(0)..=(target.x + 7).min(f.width() as i32 - 1) {
                if e.fire
                    .get(y as usize * f.width() + x as usize)
                    .copied()
                    .unwrap_or(0.0)
                    <= 0.0
                {
                    continue;
                }
                let source = Point { x, y };
                let amount = 80.0 - 10.0 * distance(source, target);
                if amount > 0.0 && amount > result && floor_ray(f, source, target) {
                    result = amount;
                }
            }
        }
    }
    if let Some(sources) = f.light_sources.as_ref() {
        for s in sources {
            if !s.power.is_finite() || !s.falloff.is_finite() || s.falloff <= 0.0 {
                continue;
            }
            let source = Point {
                x: s.x as i32,
                y: s.y as i32,
            };
            let amount = s.power - 10.0 * distance(source, target);
            if amount > 0.0 && amount > result && floor_ray(f, source, target) {
                result = amount;
            }
        }
    }
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if (crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r,a)!=dream) || crate::dreamwalking::is_body(a) || a.dead.value_or(false)
            || !has(a, "anavani-shine")
            || (exclude == Some(a.id) && active(a, "anavani-shine"))
        {
            continue;
        }
        let amount = 100.0 - 10.0 * distance(point(a), target);
        if amount > 0.0 && amount > result && floor_ray(f, point(a), target) {
            result = amount;
        }
    }
    result = result.clamp(-100.0, 100.0);
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if (crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r,a)!=dream) || a.dead.value_or(false) || a.hp <= 0.0 || !has(a, "enemy:beast:darkness-aura") {
            continue;
        }
        let amount = (100.0
            - 10.0 * ((a.x - target.x as f64).abs() + (a.y - target.y as f64).abs()))
        .max(0.0);
        if amount > 0.0 && floor_ray(f, point(a), target) {
            result -= amount;
        }
    }
    result.clamp(-100.0, 100.0)
}

/// O(V+A) deterministic room; emitters cover integer/fractional/cast boundaries.
fn culling_room(case:usize)->Run {
    let mut r=Run{depth:1.,simulation_revision:39.0.into(),..Default::default()};
    r.player=Actor{id:0.,x:10.,y:10.,hp:100.,..Default::default()};
    let mut f=Floor{depth:1.,tiles:(0..441).map(|i|if (i*7+case)%31==0{"wall"}else{"floor"}.into()).collect(),lights:(0..441).map(|i|match (i+case)%7{0=>f64::NAN,1=>f64::INFINITY,2=>-100.,3=>100.,_=>-45.25}).collect(),..Default::default()};
    f.light_sources=vec![crate::FloorLightSourcesEntry{x:8.,y:8.,power:110.125,falloff:10.,..Default::default()}].into();
    let mut env=crate::EnvironmentalState::default();env.fire=vec![0.;441];env.fire[7*21+7]=1.;f.environment=env.into();
    r.floors.push(std::sync::Arc::new(f));
    for i in 1..=24 {
        let mut a=Actor{id:i as f64,x:((i*7+case)%23)as f64-1.125,y:((i*13+case)%23)as f64-1.875,hp:100.,..Default::default()};
        if i%3==0{a.abilities.push("anavani-shine".into());if i%2==0{a.active.push("anavani-shine".into());}}
        if i%4==0{a.abilities.push("enemy:beast:darkness-aura".into());}
        if i%5==0{a.conditions.insert("phantasmiiPlane".into(),1.);}
        if i%7==0{a.conditions.insert("phantasmiiBody".into(),1.);}
        if i%11==0{a.dead=true.into();}
        if i==3{a.x=f64::NAN;} // Shine's cast is zero, not a fractional-distance cull.
        if i==6{a.x=f64::INFINITY;} // positive cast extreme remains safe in legacy oracle.
        if i==8{a.x=f64::NEG_INFINITY;} // darkness-only legacy rejects before integer ray.
        if i==12{a.y=f64::NAN;} // both Shine (cast zero) and darkness (zero amount).
        if i==16{a.hp=f64::NAN;}
        r.actors_at_mut(1.).push(a);
    }
    r
}

/// O(cases*planes*V*exclusions*original lighting cost); exact bitwise differential.
#[test]
fn scalar_source_culls_match_frozen_original() {
    for case in 0..8 {
        let mut r=culling_room(case);
        crate::dreamwalking::ensure_layer(&mut r);
        for dream in [false,true] {
            for y in 0..21 {for x in 0..21 {
                let target=Point{x,y};
                for exclude in [None,Some(0.),Some(3.),Some(6.),Some(12.)] {
                    assert_eq!(illumination_in_plane(&r,target,exclude,dream).to_bits(),original_illumination_in_plane(&r,target,exclude,dream).to_bits(),"case={case} point={target:?} exclude={exclude:?} dream={dream}");
                }
            }}
        }
    }
}

/// O(A*B+V) synthetic benchmark setup, independent of measured scalar queries.
fn benchmark_world(crowded:bool)->Run {
    let mut r=Run{depth:1.,simulation_revision:39.0.into(),..Default::default()};
    r.player=Actor{id:0.,x:10.,y:10.,hp:100.,..Default::default()};
    r.floors.push(std::sync::Arc::new(Floor{depth:1.,tiles:vec!["floor".into();6561],lights:vec![-50.;6561],..Default::default()}));
    for i in 1..=148 {
        let near=crowded||i<17;
        let mut a=Actor{id:i as f64,x:if near{(i%9+6)as f64}else{(i%41+30)as f64},y:if near{(i%7+7)as f64}else{(i%37+31)as f64},hp:100.,abilities:(0..8).map(|j|format!("test-non-light-{j}")).collect(),..Default::default()};
        if i==1{a.abilities.push("anavani-shine".into());}
        if i==2{a.abilities.push("enemy:beast:darkness-aura".into());}
        r.actors_at_mut(1.).push(a);
    }
    r
}

/// O(iterations*scalar-light-cost); manual quiet-window microbenchmark, no RNG.
#[test]
#[ignore = "manual release-only quiet-window microbenchmark"]
fn scalar_light_culling_microbenchmark() {
    use std::{hint::black_box,time::Instant};
    for crowded in [false,true] {
        let r=benchmark_world(crowded);let target=Point{x:10,y:10};
        assert_eq!(illumination_in_plane(&r,target,None,false).to_bits(),original_illumination_in_plane(&r,target,None,false).to_bits());
        for sample in 0..9 {
            for optimized in if sample%2==0{[false,true]}else{[true,false]} {
                let started=Instant::now();
                for _ in 0..4096 {
                    let value=if optimized{illumination_in_plane(black_box(&r),black_box(target),None,false)}else{original_illumination_in_plane(black_box(&r),black_box(target),None,false)};
                    black_box(value);
                }
                println!("scalar-light,crowded={crowded},optimized={optimized},sample={sample},ns_per_query={:.3}",started.elapsed().as_nanos()as f64/4096.);
            }
        }
    }
}

// Frozen v16 scalar detection and target modifier oracle.
fn original_light(r: &Run, a: &Actor) -> f64 {
    if has(a, "enemy:beast:darkness-aura") {
        return -100.0;
    }
    let mut l = original_illumination_in_plane(r, point(a), Some(a.id), crate::dreamwalking::context_dream(r));
    if has(a, "dawel-shadow-in-the-dark") {
        if active(a, "dawel-shadow-in-the-dark") {
            l -= 20.0;
        }
        if l < 0.0 {
            l *= 2.0;
        }
    }
    l.clamp(-100.0, 100.0)
}
fn original_actor_chance(r: &Run, observer: &Actor, target: &Actor, perception: f64) -> f64 {
    let Some(f) = crate::dreamwalking::floor_for(r,observer).or_else(||r.floor()) else { return 0.0 };
    if !bounds(f, point(observer)) || !bounds(f, point(target)) {
        return 0.0;
    }
    let underwater = f.tiles[observer.y as usize * f.width() + observer.x as usize] == "water"
        && f.tiles[target.y as usize * f.width() + target.x as usize] == "water";
    sight_chance(
        perception,
        optical_route(r,observer,point(target)).map_or(f64::INFINITY,|p|p.distance),
        condition(target, "stealth"),
        condition(target, "submergedStealth"),
        if crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r,observer)!=crate::dreamwalking::context_dream(r) {original_illumination_in_plane(r,point(target),Some(target.id),crate::dreamwalking::dream_actor(r,observer))}else{original_light(r,target)},
        underwater,
        line_of_sight(r, observer, target),
        condition(target, "invisible") > 0.0,
    )
}
fn original_detection_chance(r: &Run, observer: &Actor, target: &Actor) -> f64 {
    let _timing_function = crate::timing::span("vision.detection");
    if !crate::dreamwalking::same_plane(r,observer,target) {return 0.;}
    if !target.dead.value_or(false) && condition(observer, "telepathic") > 0.0 {
        return 100.0;
    }
    if condition(target, "invisible") > 0.0 {
        return 0.0;
    }
    let mut p = crate::combat::effective_stats(r, observer).perception;
    if observer.state == "hunting" {
        p *= 2.0;
    }
    if sleeping(observer) {
        p *= 0.5;
    }
    let base = original_actor_chance(r, observer, target, p);
    if observer.id == r.player.id || observer.state == "ally" {
        return base;
    }
    if base == 0.0 && (!line_of_sight(r, observer, target) || original_light(r, target) <= -100.0) {
        return 0.0;
    }
    if observer.state == "hunting" {
        if condition(observer, "targetId") == target.id
            && distance(point(observer), point(target)) < 5.0
        {
            return 100.0;
        }
        100.0 - (100.0 - base) / 3.0
    } else {
        base / 2.0
    }
}

/// O(A²*original detection cost) fixed-bit checks, including public chance API.
fn compare_detection(r:&Run) {
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        for b in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
            assert_eq!(detection_chance(r,a,b).to_bits(),original_detection_chance(r,a,b).to_bits(),"observer={} target={} context={}",a.id,b.id,crate::dreamwalking::context_dream(r));
            assert_eq!(actor_chance(r,a,b,300.).to_bits(),original_actor_chance(r,a,b,300.).to_bits(),"public chance observer={} target={}",a.id,b.id);
        }
    }
}

/// O(cases*edits*A²*scalar detection cost): same-call reuse exactness oracle.
#[test]
fn detection_sample_matches_frozen_original() {
    for case in 0..4 {
        let mut r=culling_room(case);
        for a in r.actors_at_mut(1.).iter_mut() {
            let id=a.id as usize;
            a.stats.perception=[0.,25.,300.,3000.][id%4];
            a.state=["hunting","sleeping","ally","wandering"][id%4].into();
            a.conditions.insert("targetId".into(),((id+1)%25)as f64);
            if id%7==0{a.conditions.insert("invisible".into(),1.);}
            if id%9==0{a.conditions.insert("telepathic".into(),1.);}
            if id%5==0{a.abilities.push("dawel-shadow-in-the-dark".into());a.active.push("dawel-shadow-in-the-dark".into());}
            a.conditions.insert("stealth".into(),((id%5)*11)as f64);
        }
        compare_detection(&r);
        crate::dreamwalking::ensure_layer(&mut r);compare_detection(&r);
        r.dreamwalking.as_mut().unwrap().context_dream=true;compare_detection(&r);
        // A missing floor or out-of-bounds endpoint cannot have a graph route.
        r.floors.clear();r.dreamwalking=crate::Field::Missing;compare_detection(&r);
    }
}

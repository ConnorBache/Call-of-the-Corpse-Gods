use corpse_core::{
    Run, storage,
    temporal::{History, Snapshot, revert},
};
use std::{hint::black_box, time::Instant};
fn measure(mut f: impl FnMut() -> usize, n: usize) -> serde_json::Value {
    let mut a = Vec::new();
    for i in 0..n + 5 {
        let t = Instant::now();
        black_box(f());
        if i >= 5 {
            a.push(t.elapsed().as_secs_f64() * 1e6);
        }
    }
    a.sort_by(f64::total_cmp);
    serde_json::json!({"samples":n,"median_us":a[n/2],"p95_us":a[n*95/100]})
}
fn main() {
    let mut results = vec![];
    for floors in [10, 20, 40] {
        let raw =
            std::fs::read_to_string(format!("fixtures/generated/floors-{floors}.json")).unwrap();
        let original: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let mut run = storage::from_canonical(&raw).unwrap();
        assert!(
            storage::canonical_equal(&original, &storage::canonical_value(&run).unwrap()),
            "full generated canonical roundtrip failed"
        );
        let mut cases = vec![];
        cases.push(serde_json::json!({"operation":"parse","timing":measure(||{let r:Run=serde_json::from_str(black_box(&raw)).unwrap();black_box(r.floors.len())},20)}));
        cases.push(serde_json::json!({"operation":"snapshot","timing":measure(||black_box(Snapshot::capture(&run)).state.floors.len(),1000)}));
        cases.push(serde_json::json!({"operation":"snapshot-and-touch-floor","timing":measure(||{let before=Snapshot::capture(&run);let f=std::sync::Arc::make_mut(&mut run.floors[0]);f.seen[0]=!f.seen[0];black_box(before).state.floors.len()},100)}));
        cases.push(serde_json::json!({"operation":"snapshot-history-export","timing":measure(||{let mut r=run.clone();let before=Snapshot::capture(&r);r.tick+=1.0;r.player.x+=1.0;let mut h=History::default();h.record_completed(before);h.export(&r).unwrap().history[0].len()},10)}));
        cases.push(serde_json::json!({"operation":"snapshot-history-undo","timing":measure(||{let mut r=run.clone();r.player.conditions.insert("revertCharges".into(),5.0);r.player.energy=100.0;let before=Snapshot::capture(&r);r.tick+=1.0;r.player.x+=1.0;let mut h=History::default();h.record_completed(before);revert(&mut r,&mut h).unwrap();r.tick as usize},10)}));
        results.push(serde_json::json!({"floors":floors,"bytes":raw.len(),"full_schema_roundtrip":true,"cases":cases}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"seed":"0-1-2:native-port-baseline","results":results})
        )
        .unwrap()
    );
}

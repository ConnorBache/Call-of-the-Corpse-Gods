//! Data-contract tests only. These do not prove runtime execution of any blessing.
use serde_json::Value;
use std::collections::BTreeSet;

fn catalog() -> Vec<Value> {
    serde_json::from_str(include_str!("../catalog/abilities-v50.json")).unwrap()
}

#[test]
fn revision50_preserves_all_72_names_and_authorized_descriptions() {
    let source = include_str!("../../../docs/revision50-request.txt");
    let roster = source.split("# FULL ABILITY ROSTER\n").nth(1).unwrap()
        .split("# IMPORTANT UNRESOLVED").next().unwrap();
    let mut expected: Vec<(String, String)> = Vec::new();
    for line in roster.lines() {
        let header = line.split_once("\\. ").filter(|(n, _)| n.parse::<usize>().is_ok());
        if let Some((_, name)) = header {
            expected.push((name.into(), String::new()));
        } else if !line.starts_with("## ") {
            if let Some((_, description)) = expected.last_mut() {
                description.push_str(line);
                description.push('\n');
            }
        }
    }
    let entries = catalog();
    let amendments: Value = serde_json::from_str(include_str!("../catalog/ability-description-amendments-v50.json")).unwrap();
    // The designer restored four omitted costs after the first nine amendments.
    assert_eq!(amendments.as_object().unwrap().len(), 13);
    for (id, cost) in [
        ("pelian-i-came-for-you", "Spend 10 energy"),
        ("numo-mind-read", "spend 1 energy per turn"),
        ("ursa-dream-see", "spend 1 energy per turn"),
        ("morae-bonded-by-unseen-threads", "Spend 15 energy on activation"),
    ] {
        assert!(amendments[id].as_str().unwrap().contains(cost), "{id}");
    }
    assert!(amendments["morae-bonded-by-unseen-threads"].as_str().unwrap().contains("Spend 8 energy per turn"));
    assert_eq!(entries.len(), 72);
    assert_eq!(expected.len(), 72);
    for (index, (entry, (name, description))) in entries.iter().zip(expected).enumerate() {
        assert_eq!(entry["name"].as_str().unwrap(), name, "slot {index}");
        let authored = amendments.get(entry["id"].as_str().unwrap()).and_then(Value::as_str).unwrap_or(description.trim());
        assert_eq!(entry["description"].as_str().unwrap(), authored, "{name}");
        assert_eq!(entry["godId"].as_u64().unwrap(), (index / 6) as u64);
    }
}

#[test]
fn revision50_identity_map_is_complete_unique_and_keeps_old_catalog() {
    let legacy: Vec<Value> = serde_json::from_str(include_str!("../catalog/abilities-v48.json")).unwrap();
    let entries = catalog();
    let map: Vec<Value> = serde_json::from_str(include_str!("../catalog/ability-identities-v50.json")).unwrap();
    assert_eq!(map.len(), 72);
    let ids: BTreeSet<_> = entries.iter().map(|a| a["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), 72);
    for (i, row) in map.iter().enumerate() {
        assert_eq!(row["legacyId"], legacy[i]["id"]);
        assert_eq!(row["revision50Id"], entries[i]["id"]);
        assert_eq!(row["name"], entries[i]["name"]);
        assert_eq!(row["slot"].as_u64().unwrap(), (i % 6 + 1) as u64);
        assert_eq!(row["godId"], entries[i]["godId"]);
        assert!(!entries[i]["flags"].as_array().unwrap().iter().any(|v| v == "craft"));
    }
    assert!(legacy.iter().any(|a| a["id"] == "shrail-unyielding-assault"));
    assert!(!ids.contains("shrail-unyielding-assault"));
    assert!(ids.contains("shrail-execute"));
    assert!(ids.contains("dawel-step-through-shadows"));
    assert!(ids.contains("grisha-boundless-hunger"));
    assert!(ids.contains("kative-cogworks"));
    assert!(ids.contains("kative-tinker"));
}

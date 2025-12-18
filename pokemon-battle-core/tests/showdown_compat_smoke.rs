use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ShowdownCompatCase {
    id: String,
    formatid: String,
    #[allow(dead_code)]
    seed: [u32; 4],
    p1: PlayerCase,
    p2: PlayerCase,
    log: Vec<String>,
    events: Events,
}

#[derive(Debug, Deserialize)]
struct PlayerCase {
    name: String,
    team: String,
}

#[derive(Debug, Deserialize)]
struct Events {
    #[allow(dead_code)]
    damage: Vec<DamageEvent>,
    #[allow(dead_code)]
    status: Vec<StatusEvent>,
    win: Option<String>,
    tie: bool,
}

#[derive(Debug, Deserialize)]
struct DamageEvent {
    #[allow(dead_code)]
    target: String,
    #[allow(dead_code)]
    hp: String,
    #[allow(dead_code)]
    details: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct StatusEvent {
    #[allow(dead_code)]
    target: String,
    #[allow(dead_code)]
    status: String,
    #[allow(dead_code)]
    details: Vec<String>,
}

fn cases_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("showdown_compat")
        .join("cases")
}

#[test]
fn showdown_compat_cases_are_valid_json() {
    let dir = cases_dir();
    if !dir.exists() {
        return;
    }
    let mut found = 0usize;
    for entry in fs::read_dir(&dir).expect("read_dir failed") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read case");
        let case: ShowdownCompatCase =
            serde_json::from_str(&content).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
        assert!(!case.id.trim().is_empty());
        assert!(!case.formatid.trim().is_empty());
        assert!(!case.p1.name.trim().is_empty());
        assert!(!case.p2.name.trim().is_empty());
        assert!(!case.p1.team.trim().is_empty());
        assert!(!case.p2.team.trim().is_empty());
        assert!(!case.log.is_empty());
        assert!(
            case.events.tie || case.events.win.is_some(),
            "case must end in win or tie: {}",
            case.id
        );
        found += 1;
    }
    assert!(found > 0, "no cases found in {}", dir.display());
}

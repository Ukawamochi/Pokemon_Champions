// Showdownとの差分解析ツール（V2）
//
// 目的:
// - Showdown側ログとRust側ログ（どちらもJSON）を比較し、差分をHTMLで出力する
// - CI用途として、重大な差分がある場合は終了コードを非0にできる
//
// 使い方（例）:
// - `diff_analyzer --showdown showdown_log.json --rust rust_log.json --out report.html`
// - `diff_analyzer --showdown showdown.json --rust rust.json --out report.html --fail-on-diff`
//
// 想定ログ形式（ゆるく対応）:
// - ルートが `{ "turns": [...] }` / `{ "log": [...] }` / `[ ... ]` のどれでもOK
// - turns: `{ "turn": 1, "events": [...] }` を推奨（eventsは `{ "kind": "...", ... }`）
// - log: Showdownプロトコル行（SIM-PROTOCOL.md）をそのまま格納した配列を想定
//
// NOTE:
// - 本ファイルは `tools/` 配下の単体ツールとして置く（pokemon-showdownは編集しない）
// - 依存は serde_json のみを想定（ワークスペースで既に利用）

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

#[derive(Clone, Debug)]
struct Args {
    showdown_path: PathBuf,
    rust_path: PathBuf,
    out_path: PathBuf,
    fail_on_diff: bool,
    max_turns: Option<usize>,
}

fn usage() -> &'static str {
    "Usage:\n  diff_analyzer --showdown <showdown.json> --rust <rust.json> --out <report.html> [--fail-on-diff] [--max-turns N]\n"
}

fn parse_args() -> Result<Args, String> {
    let mut showdown_path: Option<PathBuf> = None;
    let mut rust_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut fail_on_diff = false;
    let mut max_turns: Option<usize> = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--showdown" => {
                showdown_path = it.next().map(PathBuf::from);
            }
            "--rust" => {
                rust_path = it.next().map(PathBuf::from);
            }
            "--out" => {
                out_path = it.next().map(PathBuf::from);
            }
            "--fail-on-diff" => {
                fail_on_diff = true;
            }
            "--max-turns" => {
                let raw = it.next().ok_or_else(|| "--max-turns requires a value".to_string())?;
                max_turns = Some(
                    raw.parse::<usize>()
                        .map_err(|_| format!("invalid --max-turns value: {raw}"))?,
                );
            }
            "--help" | "-h" => {
                return Err(usage().to_string());
            }
            other => {
                return Err(format!("unknown arg: {other}\n\n{usage}", usage = usage()));
            }
        }
    }

    let showdown_path =
        showdown_path.ok_or_else(|| format!("missing --showdown\n\n{}", usage()))?;
    let rust_path = rust_path.ok_or_else(|| format!("missing --rust\n\n{}", usage()))?;
    let out_path = out_path.ok_or_else(|| format!("missing --out\n\n{}", usage()))?;

    Ok(Args {
        showdown_path,
        rust_path,
        out_path,
        fail_on_diff,
        max_turns,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum EventKey {
    Damage { target: String, source: String, move_id: String },
    Heal { target: String },
    Status { target: String, status: String },
    Move { source: String, move_id: String, target: String },
    Switch { side: String, to: String },
    Weather { weather: String },
    Field { field: String },
    Message { text: String },
    Unknown { kind: String, fingerprint: String },
}

#[derive(Clone, Debug)]
struct Event {
    key: EventKey,
    data: Value,
}

#[derive(Clone, Debug)]
struct TurnLog {
    turn: u32,
    events: Vec<Event>,
}

#[derive(Clone, Debug)]
struct BattleLog {
    turns: Vec<TurnLog>,
    winner: Option<String>,
    seed: Option<String>,
}

fn read_json(path: &PathBuf) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read failed: {path:?}: {e}"))?;
    serde_json::from_str::<Value>(&text).map_err(|e| format!("json parse failed: {path:?}: {e}"))
}

fn as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn get_str(obj: &Value, key: &str) -> Option<String> {
    obj.get(key).and_then(as_string)
}

fn get_u32(obj: &Value, key: &str) -> Option<u32> {
    obj.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

fn normalize_id(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn parse_log_line(line: &str) -> Option<Vec<String>> {
    if !line.starts_with('|') {
        return None;
    }
    let parts = line.split('|').skip(1).map(|s| s.trim()).collect::<Vec<_>>();
    if parts.is_empty() || parts[0].is_empty() {
        return None;
    }
    Some(parts.into_iter().map(|s| s.to_string()).collect())
}

fn side_from_ident(ident: &str) -> String {
    // "p1a: Pikachu" -> "p1", "p2a: Gyarados" -> "p2"
    let trimmed = ident.trim();
    if trimmed.len() >= 2 && (trimmed.starts_with("p1") || trimmed.starts_with("p2")) {
        trimmed.chars().take(2).collect()
    } else {
        "?".to_string()
    }
}

fn parse_event(raw: &Value) -> Event {
    let kind = raw
        .get("kind")
        .or_else(|| raw.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let kind_norm = normalize_id(kind);

    let key = match kind_norm.as_str() {
        "move" => {
            let source = get_str(raw, "source").unwrap_or_else(|| "?".to_string());
            let target = get_str(raw, "target").unwrap_or_else(|| "?".to_string());
            let move_id = get_str(raw, "move")
                .map(|s| normalize_id(&s))
                .unwrap_or_else(|| "?".to_string());
            EventKey::Move {
                source,
                move_id,
                target,
            }
        }
        "damage" => {
            let target = get_str(raw, "target").unwrap_or_else(|| "?".to_string());
            let source = get_str(raw, "source").unwrap_or_else(|| "?".to_string());
            let move_id = get_str(raw, "move")
                .map(|s| normalize_id(&s))
                .unwrap_or_else(|| "?".to_string());
            EventKey::Damage {
                target,
                source,
                move_id,
            }
        }
        "heal" => {
            let target = get_str(raw, "target").unwrap_or_else(|| "?".to_string());
            EventKey::Heal { target }
        }
        "status" => {
            let target = get_str(raw, "target").unwrap_or_else(|| "?".to_string());
            let status = get_str(raw, "status").unwrap_or_else(|| "?".to_string());
            EventKey::Status { target, status }
        }
        "switch" => {
            let side = get_str(raw, "side").unwrap_or_else(|| "?".to_string());
            let to = get_str(raw, "to").unwrap_or_else(|| "?".to_string());
            EventKey::Switch { side, to }
        }
        "weather" => {
            let weather = get_str(raw, "weather").unwrap_or_else(|| "?".to_string());
            EventKey::Weather { weather }
        }
        "field" => {
            let field = get_str(raw, "field").unwrap_or_else(|| "?".to_string());
            EventKey::Field { field }
        }
        "message" | "log" => {
            let text = get_str(raw, "text").unwrap_or_else(|| raw.to_string());
            EventKey::Message { text }
        }
        _ => {
            let fingerprint = raw
                .get("target")
                .and_then(as_string)
                .or_else(|| raw.get("move").and_then(as_string))
                .or_else(|| raw.get("status").and_then(as_string))
                .unwrap_or_else(|| raw.to_string());
            EventKey::Unknown {
                kind: kind_norm,
                fingerprint,
            }
        }
    };

    Event {
        key,
        data: raw.clone(),
    }
}

fn parse_log_array_to_turns(log: &[Value]) -> (Vec<TurnLog>, Option<String>, bool) {
    let mut turns: Vec<TurnLog> = Vec::new();
    let mut current_turn: Option<TurnLog> = None;

    let mut winner: Option<String> = None;
    let mut tie = false;

    // Track most recent move in the current turn so that damage can be attributed.
    let mut last_move_source: Option<String> = None;
    let mut last_move_target: Option<String> = None;
    let mut last_move_id: Option<String> = None;

    let push_turn = |turn: Option<TurnLog>, turns: &mut Vec<TurnLog>| {
        if let Some(t) = turn {
            if t.turn != 0 || !t.events.is_empty() {
                turns.push(t);
            }
        }
    };

    for raw in log {
        let line = match raw {
            Value::String(s) => s.as_str(),
            _ => {
                eprintln!("warning: log entry is not a string: {raw}");
                continue;
            }
        };
        let parts = match parse_log_line(line) {
            Some(p) => p,
            None => continue,
        };
        let kind = normalize_id(&parts[0]);

        if kind == "turn" {
            if let Some(n) = parts.get(1).and_then(|s| s.parse::<u32>().ok()) {
                push_turn(current_turn.take(), &mut turns);
                current_turn = Some(TurnLog {
                    turn: n,
                    events: Vec::new(),
                });
                last_move_source = None;
                last_move_target = None;
                last_move_id = None;
            }
            continue;
        }

        if kind == "win" {
            if let Some(w) = parts.get(1).cloned() {
                winner = Some(w);
            }
            continue;
        }
        if kind == "tie" {
            tie = true;
            continue;
        }

        let turn = current_turn.get_or_insert_with(|| TurnLog {
            turn: 0,
            events: Vec::new(),
        });

        match kind.as_str() {
            "move" => {
                // |move|source|move|target
                let source = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let mv = parts.get(2).cloned().unwrap_or_else(|| "?".to_string());
                let target = parts.get(3).cloned().unwrap_or_else(|| "?".to_string());
                last_move_source = Some(source.clone());
                last_move_target = Some(target.clone());
                last_move_id = Some(mv.clone());
                let ev = json!({ "kind": "move", "source": source, "move": mv, "target": target });
                turn.events.push(parse_event(&ev));
            }
            "-damage" => {
                // |-damage|target|hp
                let target = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let hp = parts.get(2).cloned().unwrap_or_else(|| "?".to_string());
                let source = last_move_source.clone().unwrap_or_else(|| "?".to_string());
                let mv = last_move_id.clone().unwrap_or_else(|| "?".to_string());
                let ev = json!({
                    "kind": "damage",
                    "target": target,
                    "hp": hp,
                    "source": source,
                    "move": mv
                });
                turn.events.push(parse_event(&ev));
            }
            "-heal" => {
                // |-heal|target|hp
                let target = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let hp = parts.get(2).cloned().unwrap_or_else(|| "?".to_string());
                let ev = json!({ "kind": "heal", "target": target, "hp": hp });
                turn.events.push(parse_event(&ev));
            }
            "-status" => {
                // |-status|target|status
                let target = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let status = parts.get(2).cloned().unwrap_or_else(|| "?".to_string());
                let ev = json!({ "kind": "status", "target": target, "status": status });
                turn.events.push(parse_event(&ev));
            }
            "switch" => {
                // |switch|pokemon|details|hp
                let pokemon = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let details = parts.get(2).cloned().unwrap_or_else(|| "?".to_string());
                let side = side_from_ident(&pokemon);
                let ev = json!({ "kind": "switch", "side": side, "to": details, "pokemon": pokemon, "hp": parts.get(3).cloned().unwrap_or_default() });
                turn.events.push(parse_event(&ev));
            }
            "-weather" => {
                // |-weather|Sandstorm
                let weather = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let ev = json!({ "kind": "weather", "weather": weather });
                turn.events.push(parse_event(&ev));
            }
            "-fieldstart" | "-fieldend" => {
                // |-fieldstart|move: Electric Terrain
                let field = parts.get(1).cloned().unwrap_or_else(|| "?".to_string());
                let ev = json!({ "kind": "field", "field": field, "action": parts[0].clone() });
                turn.events.push(parse_event(&ev));
            }
            _ => {
                // Ignore other protocol lines for now.
                let _ = last_move_target;
            }
        }
    }

    push_turn(current_turn.take(), &mut turns);
    (turns, winner, tie)
}

fn parse_turn(raw: &Value, idx: usize) -> TurnLog {
    let turn = get_u32(raw, "turn").unwrap_or_else(|| (idx + 1) as u32);
    let events = raw
        .get("events")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(parse_event).collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);
    TurnLog { turn, events }
}

fn parse_battle_log(root: &Value) -> BattleLog {
    let winner_from_field = get_str(root, "winner");
    let seed = root.get("seed").map(|v| v.to_string());

    if let Some(log_array) = root.get("log").and_then(|v| v.as_array()) {
        let (turns, winner_from_log, tie) = parse_log_array_to_turns(log_array);
        let winner = winner_from_log.or(winner_from_field).or_else(|| {
            if tie {
                None
            } else {
                None
            }
        });
        return BattleLog { turns, winner, seed };
    }

    let turns_val = root.get("turns").cloned().unwrap_or(Value::Null);
    let turns = match turns_val {
        Value::Array(arr) => arr
            .iter()
            .enumerate()
            .map(|(i, v)| parse_turn(v, i))
            .collect(),
        _ => Vec::new(),
    };

    BattleLog {
        turns,
        winner: winner_from_field,
        seed,
    }
}

#[derive(Clone, Debug)]
struct TurnDiff {
    turn: u32,
    missing_in_rust: Vec<Event>,
    extra_in_rust: Vec<Event>,
    mismatched: Vec<(Event, Event, Vec<String>)>,
}

fn compare_events(showdown: &[Event], rust: &[Event]) -> (Vec<Event>, Vec<Event>, Vec<(Event, Event, Vec<String>)>) {
    let mut s_map: BTreeMap<EventKey, Vec<Event>> = BTreeMap::new();
    let mut r_map: BTreeMap<EventKey, Vec<Event>> = BTreeMap::new();

    for ev in showdown {
        s_map.entry(ev.key.clone()).or_default().push(ev.clone());
    }
    for ev in rust {
        r_map.entry(ev.key.clone()).or_default().push(ev.clone());
    }

    let keys: BTreeSet<EventKey> = s_map
        .keys()
        .cloned()
        .chain(r_map.keys().cloned())
        .collect();

    let mut missing_in_rust = Vec::new();
    let mut extra_in_rust = Vec::new();
    let mut mismatched = Vec::new();

    for key in keys {
        let mut s_list = s_map.remove(&key).unwrap_or_default();
        let mut r_list = r_map.remove(&key).unwrap_or_default();

        // same key but possibly different payload (e.g., damage amount)
        while !s_list.is_empty() && !r_list.is_empty() {
            let s = s_list.remove(0);
            let r = r_list.remove(0);
            let reasons = diff_reason(&s, &r);
            if !reasons.is_empty() {
                mismatched.push((s, r, reasons));
            }
        }
        for s in s_list {
            missing_in_rust.push(s);
        }
        for r in r_list {
            extra_in_rust.push(r);
        }
    }

    (missing_in_rust, extra_in_rust, mismatched)
}

fn diff_reason(showdown: &Event, rust: &Event) -> Vec<String> {
    let mut reasons = Vec::new();
    if let (Some(s), Some(r)) = (showdown.data.get("amount"), rust.data.get("amount")) {
        if s != r {
            reasons.push(format!("damage amount differs (showdown={s}, rust={r})"));
        }
    }
    if let (Some(s), Some(r)) = (showdown.data.get("hp"), rust.data.get("hp")) {
        if s != r {
            reasons.push(format!("remaining hp differs (showdown={s}, rust={r})"));
        }
    }
    if let (Some(s), Some(r)) = (showdown.data.get("status"), rust.data.get("status")) {
        if s != r {
            reasons.push(format!("status differs (showdown={s}, rust={r})"));
        }
    }
    reasons
}

fn compare_logs(showdown: &BattleLog, rust: &BattleLog, max_turns: Option<usize>) -> Vec<TurnDiff> {
    let mut diffs = Vec::new();
    let max_turn = max_turns
        .unwrap_or_else(|| showdown.turns.len().max(rust.turns.len()))
        .min(showdown.turns.len().max(rust.turns.len()));

    for idx in 0..max_turn {
        let s_turn = showdown.turns.get(idx);
        let r_turn = rust.turns.get(idx);
        let turn_num = s_turn
            .map(|t| t.turn)
            .or_else(|| r_turn.map(|t| t.turn))
            .unwrap_or((idx + 1) as u32);

        let s_events = s_turn.map(|t| t.events.as_slice()).unwrap_or(&[]);
        let r_events = r_turn.map(|t| t.events.as_slice()).unwrap_or(&[]);

        let (missing_in_rust, extra_in_rust, mismatched) = compare_events(s_events, r_events);
        if !missing_in_rust.is_empty() || !extra_in_rust.is_empty() || !mismatched.is_empty() {
            diffs.push(TurnDiff {
                turn: turn_num,
                missing_in_rust,
                extra_in_rust,
                mismatched,
            });
        }
    }

    diffs
}

fn summarize_causes(diffs: &[TurnDiff]) -> Vec<String> {
    let mut damage_mismatches = 0usize;
    let mut status_mismatches = 0usize;
    let mut missing = 0usize;
    let mut extra = 0usize;

    for d in diffs {
        missing += d.missing_in_rust.len();
        extra += d.extra_in_rust.len();
        for (_, _, reasons) in &d.mismatched {
            for r in reasons {
                if r.contains("damage amount") || r.contains("remaining hp") {
                    damage_mismatches += 1;
                }
                if r.contains("status differs") {
                    status_mismatches += 1;
                }
            }
        }
    }

    let mut hints = Vec::new();
    if damage_mismatches > 0 {
        hints.push("推定原因: ダメージ計算式/補正順序/丸めのズレ".to_string());
    }
    if status_mismatches > 0 {
        hints.push("推定原因: 状態異常付与条件/免疫/優先度（Substitute/Protect等）の違い".to_string());
    }
    if missing > 0 || extra > 0 {
        hints.push("推定原因: 乱数シード差/処理順序差（速度/優先度/ターン終了処理）の影響".to_string());
    }
    if hints.is_empty() {
        hints.push("差分原因の推定: 重大な差分は検出されませんでした".to_string());
    }
    hints
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_html(showdown: &BattleLog, rust: &BattleLog, diffs: &[TurnDiff]) -> String {
    let summary = summarize_causes(diffs);
    let header = json!({
        "showdown": {
            "winner": showdown.winner,
            "seed": showdown.seed,
            "turns": showdown.turns.len()
        },
        "rust": {
            "winner": rust.winner,
            "seed": rust.seed,
            "turns": rust.turns.len()
        },
        "diff_turns": diffs.len()
    });

    let mut body = String::new();
    body.push_str("<!doctype html><html><head><meta charset=\"utf-8\"/>");
    body.push_str("<title>Showdown Diff Report</title>");
    body.push_str(
        "<style>
body{font-family:system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin:24px;}
code,pre{font-family:ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;}
.ok{color:#0a7;}
.bad{color:#c33;}
.card{border:1px solid #ddd; border-radius:10px; padding:12px 14px; margin:12px 0;}
.turn{margin-top:18px;}
.grid{display:grid; grid-template-columns:1fr 1fr; gap:12px;}
.tag{display:inline-block; padding:2px 8px; border-radius:999px; font-size:12px; background:#eee;}
.tag.bad{background:#fee; color:#a00; border:1px solid #fbb;}
.tag.ok{background:#efe; color:#060; border:1px solid #bfb;}
details > summary{cursor:pointer;}
</style>",
    );
    body.push_str("</head><body>");
    body.push_str("<h1>Showdown Diff Report</h1>");
    body.push_str("<div class=\"card\"><h2>Summary</h2>");
    body.push_str("<pre>");
    body.push_str(&html_escape(&serde_json::to_string_pretty(&header).unwrap_or_default()));
    body.push_str("</pre>");
    for h in summary {
        body.push_str("<div>");
        body.push_str(&html_escape(&h));
        body.push_str("</div>");
    }
    body.push_str("</div>");

    if diffs.is_empty() {
        body.push_str("<div class=\"card\"><span class=\"tag ok\">NO DIFF</span> 差分は検出されませんでした。</div>");
    } else {
        body.push_str("<div class=\"card\"><span class=\"tag bad\">DIFF</span> 差分が検出されました。</div>");
    }

    for d in diffs {
        body.push_str(&format!("<div class=\"turn\"><h2>Turn {}</h2>", d.turn));
        body.push_str("<div class=\"grid\">");

        body.push_str("<div class=\"card\"><h3>Missing in Rust</h3>");
        if d.missing_in_rust.is_empty() {
            body.push_str("<span class=\"tag ok\">none</span>");
        } else {
            for ev in &d.missing_in_rust {
                body.push_str("<details><summary>");
                body.push_str(&html_escape(&format!("{:?}", ev.key)));
                body.push_str("</summary><pre>");
                body.push_str(&html_escape(&serde_json::to_string_pretty(&ev.data).unwrap_or_default()));
                body.push_str("</pre></details>");
            }
        }
        body.push_str("</div>");

        body.push_str("<div class=\"card\"><h3>Extra in Rust</h3>");
        if d.extra_in_rust.is_empty() {
            body.push_str("<span class=\"tag ok\">none</span>");
        } else {
            for ev in &d.extra_in_rust {
                body.push_str("<details><summary>");
                body.push_str(&html_escape(&format!("{:?}", ev.key)));
                body.push_str("</summary><pre>");
                body.push_str(&html_escape(&serde_json::to_string_pretty(&ev.data).unwrap_or_default()));
                body.push_str("</pre></details>");
            }
        }
        body.push_str("</div>");

        body.push_str("</div>");

        body.push_str("<div class=\"card\"><h3>Mismatched payload</h3>");
        if d.mismatched.is_empty() {
            body.push_str("<span class=\"tag ok\">none</span>");
        } else {
            for (s, r, reasons) in &d.mismatched {
                body.push_str("<details><summary>");
                body.push_str(&html_escape(&format!("{:?}", s.key)));
                body.push_str("</summary>");
                body.push_str("<div>");
                for reason in reasons {
                    body.push_str("<div class=\"bad\">");
                    body.push_str(&html_escape(reason));
                    body.push_str("</div>");
                }
                body.push_str("</div>");
                body.push_str("<div class=\"grid\">");
                body.push_str("<div><h4>Showdown</h4><pre>");
                body.push_str(&html_escape(&serde_json::to_string_pretty(&s.data).unwrap_or_default()));
                body.push_str("</pre></div>");
                body.push_str("<div><h4>Rust</h4><pre>");
                body.push_str(&html_escape(&serde_json::to_string_pretty(&r.data).unwrap_or_default()));
                body.push_str("</pre></div>");
                body.push_str("</div>");
                body.push_str("</details>");
            }
        }
        body.push_str("</div>");

        body.push_str("</div>");
    }

    body.push_str("</body></html>");
    body
}

pub fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };

    let showdown_root = match read_json(&args.showdown_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let rust_root = match read_json(&args.rust_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    let showdown = parse_battle_log(&showdown_root);
    let rust = parse_battle_log(&rust_root);
    let diffs = compare_logs(&showdown, &rust, args.max_turns);

    let html = render_html(&showdown, &rust, &diffs);
    if let Err(e) = fs::write(&args.out_path, html) {
        eprintln!("write failed: {:?}: {e}", args.out_path);
        std::process::exit(2);
    }

    if !diffs.is_empty() {
        eprintln!(
            "Diff detected: {} turn(s). Report: {:?}",
            diffs.len(),
            args.out_path
        );
        if args.fail_on_diff {
            std::process::exit(1);
        }
    } else {
        println!("No diff. Report: {:?}", args.out_path);
    }
}

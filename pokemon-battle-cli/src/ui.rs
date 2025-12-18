use anyhow::Result;
use pokemon_battle_core::battle::{
    BattleResult, BattleView, BattlerView, MoveEventView, MoveOutcome, MoveView, PlayerAction, Side,
    SideView, StatStagesView, StatusEventView, TeamMemberView,
};
use pokemon_battle_core::model::{Pokemon, StatusCondition};
use pokemon_battle_core::types::type_effectiveness;
use std::collections::HashSet;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

pub fn render(
    view: &BattleView,
    human_side: Side,
    human_candidates: &[Pokemon],
    opponent_candidates: &[Pokemon],
) {
    clear_screen();
    let (you, opp) = if matches!(human_side, Side::A) {
        (&view.side_a, &view.side_b)
    } else {
        (&view.side_b, &view.side_a)
    };

    if let Some(weather) = &view.weather {
        println!("天候: {:?} (残りターン: {})", weather, view.weather_turns);
        println!();
    }
    print_party_section(you, opp, human_candidates, opponent_candidates);
    println!();
    println!("=== 場 ===");
    print_field_section(you, opp);
}

pub fn prompt_action(view: &BattleView, side: Side, force_switch: bool) -> Result<PlayerAction> {
    let side_view = match side {
        Side::A => &view.side_a,
        Side::B => &view.side_b,
    };
    let target_types = match side {
        Side::A => &view.side_b.active.types,
        Side::B => &view.side_a.active.types,
    };
    let switch_targets: Vec<usize> = side_view
        .team
        .iter()
        .filter(|m| !m.is_active && !m.is_fainted)
        .map(|m| m.index)
        .collect();

    loop {
        if force_switch {
            if switch_targets.is_empty() {
                println!("交代可能なポケモンがいません。");
                return Ok(PlayerAction::Move(0));
            }
            println!("交代先を番号で選んでください:");
            list_switch_options(side_view);
            let input = read_line()?;
            if let Ok(idx) = input.trim().parse::<usize>() {
                if switch_targets.contains(&idx) {
                    return Ok(PlayerAction::Switch(idx));
                }
            }
            println!("無効な入力です。もう一度入力してください。");
            continue;
        }

        println!("行動を選択してください:");
        print_moves(&side_view.active.moves, target_types);
        println!(" 0: 交代");
        let input = read_line()?;
        let trimmed = input.trim();
        if trimmed == "0" {
            if switch_targets.is_empty() {
                println!("交代できるポケモンがいません。");
                continue;
            }
            list_switch_options(side_view);
            let input = read_line()?;
            if let Ok(idx) = input.trim().parse::<usize>() {
                if switch_targets.contains(&idx) {
                    return Ok(PlayerAction::Switch(idx));
                }
            }
            println!("無効な入力です。もう一度入力してください。");
            continue;
        }

        if let Ok(choice) = trimmed.parse::<usize>() {
            if choice == 0 || choice > side_view.active.moves.len() {
                println!("範囲外の番号です。");
                continue;
            }
            let mv_idx = choice - 1;
            if side_view.active.moves.get(mv_idx).map(|m| m.remaining_pp <= 0).unwrap_or(true) {
                println!("PPが足りません。");
                continue;
            }
            return Ok(PlayerAction::Move(mv_idx));
        }
        println!("無効な入力です。もう一度入力してください。");
    }
}

pub fn print_result(result: BattleResult, human_side: Side) {
    let outcome_text = match result {
        BattleResult::AWins if matches!(human_side, Side::A) => "あなたの勝ち！",
        BattleResult::BWins if matches!(human_side, Side::B) => "あなたの勝ち！",
        BattleResult::AWins | BattleResult::BWins => "あなたの負け…",
        BattleResult::Tie => "引き分け",
    };
    println!();
    println!("=== 結果: {} ===", outcome_text);
}

fn print_moves(moves: &[MoveView], target_types: &[String]) {
    for (i, mv) in moves.iter().enumerate() {
        let status = if mv.remaining_pp <= 0 { "PPなし" } else { "" };
        let effectiveness = type_effectiveness(&mv.move_type, target_types);
        let effectiveness_text = effectiveness_label(effectiveness);
        println!(
            " {:>2}: {:<16} {:<10} 命中 {:>5.1} 威力 {:<3} PP {}/{} {} {}",
            i + 1,
            mv.name,
            mv.move_type,
            mv.accuracy,
            mv.power,
            mv.remaining_pp,
            mv.max_pp,
            status,
            effectiveness_text
        );
    }
}

fn print_party_section(
    you: &SideView,
    opp: &SideView,
    human_candidates: &[Pokemon],
    opponent_candidates: &[Pokemon],
) {
    println!("=== 手持ち ===");
    println!(
        "{:<46} | {:<46}",
        "あなたの手持ち",
        "相手の手持ち"
    );
    println!("{}", "-".repeat(46) + " | " + &"-".repeat(46));
    let rows = human_candidates.len().max(opponent_candidates.len()).max(3);
    let selected_names: HashSet<String> =
        opp.team.iter().map(|member| member.name.clone()).collect();
    for idx in 0..rows {
        let left = if idx < you.team.len() {
            format_own_party_entry(&you.team[idx], idx)
        } else {
            "".to_string()
        };
        let right = if idx < opponent_candidates.len() {
            format_opponent_party_entry(
                &opponent_candidates[idx],
                idx,
                selected_names.contains(&opponent_candidates[idx].name),
            )
        } else {
            "".to_string()
        };
        println!("{:<46} | {:<46}", left, right);
    }
}

fn print_field_section(you: &SideView, opp: &SideView) {
    println!("{}", "-".repeat(46) + " | " + &"-".repeat(46));
    let human_line = format_active_summary("あなた", &you.active, true);
    let opponent_line = format_active_summary("相手", &opp.active, false);
    println!("{:<46} | {:<46}", human_line, opponent_line);
}

fn format_own_party_entry(member: &TeamMemberView, idx: usize) -> String {
    let marker = if member.is_active { "▶" } else { " " };
    format!(
        "{}{:>2}: {:<12} [{} / {}] {}",
        marker,
        idx + 1,
        member.name,
        member.hp.max(0),
        member.max_hp,
        format_status(&member.status)
    )
}

fn format_opponent_party_entry(mon: &Pokemon, idx: usize, selected: bool) -> String {
    let marker = if selected { "▶" } else { " " };
    format!("{}{:>2}: {:<18}", marker, idx + 1, mon.name)
}

fn format_active_summary(label: &str, active: &BattlerView, show_numbers: bool) -> String {
    let bar = hp_bar(active.hp, active.max_hp);
    let status = format_status(&active.status);
    if show_numbers {
        let hp_text = format!("{} / {}", active.hp.max(0), active.max_hp);
        format!("{}: {} {} {} {}", label, active.name, bar, hp_text, status)
    } else {
        format!("{}: {} {} {}", label, active.name, bar, status)
    }
}

pub fn describe_faints(before: &BattleView, after: &BattleView, human_side: Side) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(faint_lines_for_side(
        &before.side_a,
        &after.side_a,
        matches!(human_side, Side::A),
    ));
    lines.extend(faint_lines_for_side(
        &before.side_b,
        &after.side_b,
        matches!(human_side, Side::B),
    ));
    lines
}

pub fn print_turn_summary(
    action_lines: &[String],
    status_lines: &[String],
    faint_lines: &[String],
) {
    if action_lines.is_empty() && status_lines.is_empty() && faint_lines.is_empty() {
        return;
    }
    println!();
    for line in action_lines {
        println!("{}", line);
    }
    for line in status_lines {
        println!("{}", line);
    }
    for line in faint_lines {
        println!("{}", line);
    }
}

pub fn wait_for_next_turn() {
    thread::sleep(Duration::from_secs(2));
}

pub fn prompt_team_selection(
    role_label: &str,
    team: &[Pokemon],
    opponent: &[Pokemon],
    count: usize,
) -> Result<Vec<usize>> {
    if team.len() < count {
        anyhow::bail!("{}のチームに十分なポケモンがいません", role_label);
    }
    println!(
        "{}のチームから{}体を選んでください (例: 1 3 5)。番号はスペースまたはカンマで区切れます。",
        role_label, count
    );
    print_selection_table(team, opponent);
    loop {
        print!("選出番号: ");
        let input = read_line()?;
        let replaced = input.replace(',', " ");
        let mut picks = Vec::new();
        let mut invalid = false;
        for token in replaced.split_whitespace() {
            let num = match token.parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    invalid = true;
                    break;
                }
            };
            if num == 0 || num > team.len() {
                println!("番号は1〜{}で指定してください。", team.len());
                invalid = true;
                break;
            }
            let idx = num - 1;
            if picks.contains(&idx) {
                println!("同じ番号を複数回選ぶことはできません。");
                invalid = true;
                break;
            }
            picks.push(idx);
        }
        if invalid {
            continue;
        }
        if picks.len() != count {
            println!("{}体ちょうど選択してください。現在: {}", count, picks.len());
            continue;
        }
        return Ok(picks);
    }
}

pub fn print_selection_summary(role_label: &str, party: &[Pokemon]) {
    if party.is_empty() {
        return;
    }
    let names: Vec<String> = party.iter().map(|p| p.name.clone()).collect();
    println!("{}の選出: {}", role_label, names.join(" / "));
}

fn effectiveness_label(multiplier: f32) -> &'static str {
    const EPS: f32 = 0.01;
    if (multiplier - 0.0).abs() < EPS {
        "こうかがない..."
    } else if multiplier > 1.0 + EPS {
        "こうかはばつぐん！"
    } else if multiplier < 1.0 - EPS {
        "いまひとつ..."
    } else {
        "こうかあり"
    }
}

fn faint_lines_for_side(before: &SideView, after: &SideView, is_human: bool) -> Vec<String> {
    before
        .team
        .iter()
        .zip(after.team.iter())
        .filter_map(|(prev, current)| {
            if !prev.is_fainted && current.is_fainted {
                let prefix = if is_human { "" } else { "相手の" };
                Some(format!("{}{} は たおれた！", prefix, current.name))
            } else {
                None
            }
        })
        .collect()
}

pub fn format_move_events(events: &[MoveEventView], human_side: Side) -> Vec<String> {
    let mut lines = Vec::new();
    for event in events {
        let prefix = if event.side == human_side {
            ""
        } else {
            "相手の"
        };
        lines.push(format!(
            "{}{} は {} を つかった！",
            prefix, event.pokemon, event.move_name
        ));
        match &event.outcome {
            MoveOutcome::Missed => lines.push("しかし あたらなかった！".to_string()),
            MoveOutcome::Protected => lines.push("しかし まもられた！".to_string()),
            MoveOutcome::NoEffect { effectiveness } => {
                lines.push(effectiveness_label(*effectiveness).to_string())
            }
            MoveOutcome::Hit { effectiveness, .. } => {
                lines.push(effectiveness_label(*effectiveness).to_string())
            }
            MoveOutcome::StatusOnly => {}
        }
    }
    lines
}

pub fn format_status_events(events: &[StatusEventView], human_side: Side) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            let prefix = if event.side == human_side {
                ""
            } else {
                "相手の"
            };
            format!("{}{} {}", prefix, event.pokemon, event.message)
        })
        .collect()
}

fn print_selection_table(team: &[Pokemon], opponent: &[Pokemon]) {
    let width = 46;
    println!(
        "{:<width$} | {}",
        "あなたの候補",
        "相手の候補",
        width = width
    );
    println!("{}", "-".repeat(width) + " | " + &"-".repeat(width));
    let rows = team.len().max(opponent.len());
    for idx in 0..rows {
        let left = team
            .get(idx)
            .map(|mon| format_candidate(mon, idx))
            .unwrap_or_else(|| " ".repeat(width));
        let right = opponent
            .get(idx)
            .map(|mon| format_candidate(mon, idx))
            .unwrap_or_else(|| " ".repeat(width));
        println!("{:<width$} | {}", left, right, width = width);
    }
    println!("{}", "-".repeat(width) + " | " + &"-".repeat(width));
}

fn format_candidate(mon: &Pokemon, idx: usize) -> String {
    let types = if mon.types.is_empty() {
        "-".to_string()
    } else {
        mon.types.join("/")
    };
    format!(
        "{:>2}: {:<12} タイプ:{:<12} HP:{}",
        idx + 1,
        mon.name,
        types,
        mon.stats.hp
    )
}

fn list_switch_options(side: &SideView) {
    for member in side.team.iter().filter(|m| !m.is_active && !m.is_fainted) {
        let hp_bar = hp_bar(member.hp, member.max_hp);
        let status = format_status(&member.status);
        println!(
            " {:>2}: {:<12} [{}] {} {}",
            member.index,
            member.name,
            hp_bar,
            status,
            if member.is_fainted { "(瀕死)" } else { "" }
        );
    }
}

fn print_active(hp: i32, max_hp: i32, status: &Option<StatusCondition>, stages: &StatStagesView, name: &str) {
    let bar = hp_bar(hp, max_hp);
    let status = format_status(status);
    let stage_text = format_stages(stages);
    println!("{} [{}] {} {}", name, bar, status, stage_text);
}

fn print_team(team: &[TeamMemberView]) {
    for member in team {
        let hp_bar = hp_bar(member.hp, member.max_hp);
        let status = format_status(&member.status);
        let marker = if member.is_active { ">" } else { " " };
        let faint = if member.is_fainted { " (瀕死)" } else { "" };
        println!(
            "{} {:<12} [{}] {}{}",
            marker, member.name, hp_bar, status, faint
        );
    }
}

fn describe_side(side: &SideView) -> String {
    let mut parts = Vec::new();
    if side.hazards.stealth_rock {
        parts.push("ステロ");
    }
    if side.hazards.spikes > 0 {
        parts.push(match side.hazards.spikes {
            1 => "1層スパイク",
            2 => "2層スパイク",
            _ => "3層スパイク",
        });
    }
    if side.hazards.toxic_spikes > 0 {
        parts.push(match side.hazards.toxic_spikes {
            1 => "どくびし1層",
            _ => "どくびし2層",
        });
    }
    if side.screens.reflect > 0 {
        parts.push("リフレクター");
    }
    if side.screens.light_screen > 0 {
        parts.push("ひかりのかべ");
    }
    if parts.is_empty() {
        "なし".to_string()
    } else {
        parts.join(" / ")
    }
}

fn hp_bar(hp: i32, max_hp: i32) -> String {
    let width = 20usize;
    let hp_clamped = hp.max(0) as f32;
    let max = max_hp.max(1) as f32;
    let filled = ((hp_clamped / max) * width as f32).round() as usize;
    let filled = filled.min(width);
    let mut bar = String::new();
    bar.push('[');
    for _ in 0..filled {
        bar.push('=');
    }
    for _ in filled..width {
        bar.push('.');
    }
    bar.push(']');
    bar
}

fn format_status(status: &Option<StatusCondition>) -> String {
    match status {
        None => "OK".to_string(),
        Some(StatusCondition::Burn) => "BRN".to_string(),
        Some(StatusCondition::Paralysis) => "PAR".to_string(),
        Some(StatusCondition::Sleep) => "SLP".to_string(),
        Some(StatusCondition::Poison) => "PSN".to_string(),
        Some(StatusCondition::Toxic) => "TOX".to_string(),
        Some(StatusCondition::Freeze) => "FRZ".to_string(),
    }
}

fn format_stages(stages: &StatStagesView) -> String {
    let mut parts = Vec::new();
    let push = |parts: &mut Vec<String>, label: &str, val: i8| {
        if val != 0 {
            parts.push(format!("{} {:+}", label, val));
        }
    };
    push(&mut parts, "Atk", stages.atk);
    push(&mut parts, "Def", stages.def);
    push(&mut parts, "SpA", stages.spa);
    push(&mut parts, "SpD", stages.spd);
    push(&mut parts, "Spe", stages.spe);
    push(&mut parts, "Acc", stages.acc);
    push(&mut parts, "Eva", stages.eva);
    if parts.is_empty() {
        "能力変化: なし".to_string()
    } else {
        format!("能力変化: {}", parts.join(" "))
    }
}

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

fn read_line() -> Result<String> {
    let mut buf = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

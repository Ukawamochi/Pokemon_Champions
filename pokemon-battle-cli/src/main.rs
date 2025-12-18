use anyhow::{anyhow, Context};
use pokemon_battle_core::battle_logger::{showdown_ident, BattleLogger};
use pokemon_battle_core::data::moves::{get_move, normalize_move_name};
use pokemon_battle_core::data::species::POKEDEX;
use pokemon_battle_core::i18n::translate_pokemon;
use pokemon_battle_core::parser::parse_showdown_team;
use pokemon_battle_core::sim::battle::{execute_turn, Action, BattleState};
use pokemon_battle_core::sim::{run_team_battle as sim_run_team_battle, BattleResult, RandomAI};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;

#[derive(Deserialize)]
struct TeamsJson {
    team_a: String,
    team_b: String,
}

#[derive(Debug, Deserialize)]
struct ShowdownCompatCase {
    #[serde(default)]
    id: String,
    #[serde(default)]
    formatid: String,
    #[serde(default)]
    seed: [u32; 4],
    p1: PlayerCase,
    p2: PlayerCase,
    #[serde(default)]
    events: CaseEvents,
}

#[derive(Debug, Deserialize)]
struct PlayerCase {
    name: String,
    team: String,
}

#[derive(Debug, Default, Deserialize)]
struct CaseEvents {
    win: Option<String>,
    #[serde(default)]
    tie: bool,
}

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("check-species") => {
            let name = args
                .next()
                .ok_or_else(|| anyhow!("Usage: cargo run -- check-species <species>"))?;
            check_species(&name)
        }
        Some("check-move") => {
            let name = args
                .next()
                .ok_or_else(|| anyhow!("Usage: cargo run -- check-move <move>"))?;
            check_move(&name)
        }
        Some("list-species") => list_species(),
        Some("test-parse") => {
            let path = args.next().unwrap_or_else(|| "teams.json".to_string());
            test_parse(&path)
        }
        Some("run-case") => {
            let mut case_path: Option<String> = None;
            let mut out_path: Option<String> = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--case" => case_path = args.next(),
                    "--log-json" => out_path = args.next(),
                    other => return Err(anyhow!("Unknown arg '{}' for run-case", other)),
                }
            }
            let case_path = case_path.ok_or_else(|| anyhow!("Usage: cargo run -- run-case --case <case.json> --log-json <out.json>"))?;
            let out_path = out_path.ok_or_else(|| anyhow!("Usage: cargo run -- run-case --case <case.json> --log-json <out.json>"))?;
            run_case(&case_path, &out_path)
        }
        Some(cmd) => Err(anyhow!("Unknown command '{}'", cmd)),
        None => run_default_battle(),
    }
}

fn seed_to_u64(seed: [u32; 4]) -> u64 {
    ((seed[0] as u64) << 48) ^ ((seed[1] as u64) << 32) ^ ((seed[2] as u64) << 16) ^ (seed[3] as u64)
}

fn run_case(case_path: &str, out_path: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(case_path).with_context(|| format!("failed to read {}", case_path))?;
    let case: ShowdownCompatCase =
        serde_json::from_str(&content).map_err(|e| anyhow!("failed to parse case json {}: {}", case_path, e))?;

    let mut p1_team = parse_showdown_team(&case.p1.team)?;
    let mut p2_team = parse_showdown_team(&case.p2.team)?;
    if p1_team.is_empty() || p2_team.is_empty() {
        return Err(anyhow!("each team must contain at least one Pokémon"));
    }
    let p1 = p1_team.remove(0);
    let p2 = p2_team.remove(0);

    let mut state = BattleState::new(p1, p2);
    let formatid = if case.formatid.trim().is_empty() {
        "gen9customgame".to_string()
    } else {
        case.formatid.clone()
    };
    state.logger = Some(BattleLogger::new_with_format(formatid.clone()));

    if let Some(logger) = state.logger.as_mut() {
        let p1_ident = showdown_ident(0, &state.pokemon_a.species);
        let p2_ident = showdown_ident(1, &state.pokemon_b.species);
        logger.log_switch(&p1_ident, &state.pokemon_a.species, state.pokemon_a.current_hp, state.pokemon_a.stats.hp);
        logger.log_switch(&p2_ident, &state.pokemon_b.species, state.pokemon_b.current_hp, state.pokemon_b.stats.hp);
    }

    if state.pokemon_a.moves.is_empty() || state.pokemon_b.moves.is_empty() {
        return Err(anyhow!("both active Pokémon must have at least one move"));
    }

    let mut rng = SmallRng::seed_from_u64(seed_to_u64(case.seed));
    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);

    if let Some(logger) = state.logger.as_mut() {
        if case.events.tie {
            logger.log_tie();
        } else if let Some(winner) = case.events.win.as_deref() {
            logger.log_win(winner);
        }
    }

    let log_json = if let Some(logger) = &state.logger {
        json!({
            "id": case.id,
            "formatid": formatid,
            "seed": case.seed,
            "log": logger.log_lines(),
        })
    } else {
        json!({})
    };

    fs::write(out_path, serde_json::to_string_pretty(&log_json)? + "\n")
        .with_context(|| format!("failed to write {}", out_path))?;
    Ok(())
}

fn run_default_battle() -> anyhow::Result<()> {
    let content =
        fs::read_to_string("teams.json").context("failed to read teams.json in project root")?;
    let teams: TeamsJson =
        serde_json::from_str(&content).map_err(|e| anyhow!("failed to parse teams.json: {}", e))?;
    let mut rng = SmallRng::seed_from_u64(0xBADC0DE);
    let team_a = parse_showdown_team(&teams.team_a)?;
    let team_b = parse_showdown_team(&teams.team_b)?;
    if team_a.is_empty() || team_b.is_empty() {
        return Err(anyhow!("each team must contain at least one Pokémon"));
    }
    let selected_a = select_three(team_a, &mut rng);
    let selected_b = select_three(team_b, &mut rng);
    println!("選抜されたチームAのポケモン:");
    for p in &selected_a {
        println!("  {}", translate_pokemon(&p.species));
    }
    println!("選抜されたチームBのポケモン:");
    for p in &selected_b {
        println!("  {}", translate_pokemon(&p.species));
    }
    println!("\n=== ポケモンバトル (3vs3) ===");
    let winner = run_team_battle(selected_a, selected_b)?;
    println!("\n勝者: {}", winner);
    Ok(())
}

fn run_team_battle(
    team_a: Vec<pokemon_battle_core::sim::Pokemon>,
    team_b: Vec<pokemon_battle_core::sim::Pokemon>,
) -> anyhow::Result<&'static str> {
    if team_a.is_empty() || team_b.is_empty() {
        return Err(anyhow!("Teams must have at least one Pokémon"));
    }
    let mut ai_a = RandomAI::new(0);
    let mut ai_b = RandomAI::new(1);
    let result = sim_run_team_battle(team_a, team_b, &mut ai_a, &mut ai_b);
    let winner = match result {
        BattleResult::TeamAWins => "チームA",
        BattleResult::TeamBWins => "チームB",
        BattleResult::Draw => "引き分け",
    };
    Ok(winner)
}

fn select_three(
    mut team: Vec<pokemon_battle_core::sim::Pokemon>,
    rng: &mut SmallRng,
) -> Vec<pokemon_battle_core::sim::Pokemon> {
    let count = team.len().min(3);
    team.shuffle(rng);
    team.into_iter().take(count).collect()
}

fn check_species(name: &str) -> anyhow::Result<()> {
    let id = normalize_species_id(name);
    let data = POKEDEX
        .get(id.as_str())
        .ok_or_else(|| anyhow!("Species '{}' not found in POKEDEX", name))?;
    println!(
        "Found species: {} (#{}) Types: {}{}{}",
        data.name,
        data.num,
        data.types[0],
        if data.types[1].is_empty() { "" } else { " / " },
        data.types[1]
    );
    println!(
        "Base stats - HP: {}, Atk: {}, Def: {}, SpA: {}, SpD: {}, Spe: {}",
        data.base_stats.hp,
        data.base_stats.atk,
        data.base_stats.def,
        data.base_stats.spa,
        data.base_stats.spd,
        data.base_stats.spe
    );
    Ok(())
}

fn check_move(name: &str) -> anyhow::Result<()> {
    let normalized = normalize_move_name(name);
    let data = get_move(normalized.as_str()).ok_or_else(|| anyhow!("Move '{}' not found", name))?;
    println!(
        "Found move: {} (type: {}, category: {:?}, power: {:?}, priority: {})",
        data.name, data.move_type, data.category, data.base_power, data.priority
    );
    Ok(())
}

fn list_species() -> anyhow::Result<()> {
    let mut entries: Vec<_> = POKEDEX.entries().collect();
    entries.sort_by_key(|(id, _)| *id);
    for (id, data) in entries {
        println!("{} ({})", data.name, id);
    }
    Ok(())
}

fn test_parse(path: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(path).with_context(|| format!("failed to read {}", path))?;
    let teams: TeamsJson =
        serde_json::from_str(&content).map_err(|e| anyhow!("failed to parse {}: {}", path, e))?;
    let team_a = parse_showdown_team(&teams.team_a)?;
    let team_b = parse_showdown_team(&teams.team_b)?;
    println!(
        "Parsed {} Pokémon from team_a and {} from team_b in {}",
        team_a.len(),
        team_b.len(),
        path
    );
    Ok(())
}

fn normalize_species_id(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

mod ui;

use anyhow::Context;
use pokemon_battle_matrix::battle::{Battle, BattleOptions, BattlePolicy, PlayerAction, Side};
use pokemon_battle_matrix::{load_teams, model::{Pokemon, TeamsFile}, MctsMode, MctsParams};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

struct CliOptions {
    teams_path: PathBuf,
    seed: u64,
    human_side: Side,
    policy: BattlePolicy,
}

fn main() -> anyhow::Result<()> {
    let opts = parse_args()?;
    let teams = load_teams(&opts.teams_path).context("チームデータの読み込みに失敗しました")?;
    run_game(opts, teams)
}

fn parse_args() -> anyhow::Result<CliOptions> {
    let mut teams_path = PathBuf::from("../teams.json");
    let mut seed = 0u64;
    let mut human_side = Side::A;
    let mut policy = BattlePolicy::Random;
    let mut mcts_params = MctsParams::default();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--teams" => {
                teams_path = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow::anyhow!("--teams の後にパスを指定してください"))?;
            }
            "--seed" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--seed の後に数値を指定してください"))?;
                seed = val.parse()?;
            }
            "--human-side" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--human-side の後に A または B を指定してください"))?;
                human_side = parse_side(&val)
                    .ok_or_else(|| anyhow::anyhow!("--human-side は A または B を指定してください"))?;
            }
            "--policy" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--policy の後に random または mcts を指定してください"))?;
                policy = match val.to_ascii_lowercase().as_str() {
                    "random" => BattlePolicy::Random,
                    "mcts" => BattlePolicy::Mcts(mcts_params.clone()),
                    other => anyhow::bail!("--policy は random または mcts を指定してください (指定: {other})"),
                };
            }
            "--mcts-iters" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--mcts-iters の後に数値を指定してください"))?;
                mcts_params.iterations = Some(val.parse()?);
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--mcts-ms" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--mcts-ms の後にミリ秒を指定してください"))?;
                mcts_params.time_budget = Some(Duration::from_millis(val.parse()?));
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--rollout-horizon" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--rollout-horizon の後に数値を指定してください"))?;
                mcts_params.rollout_horizon = val.parse()?;
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--uct-c" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--uct-c の後に実数を指定してください"))?;
                mcts_params.exploration_constant = val.parse()?;
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--mcts-mode" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--mcts-mode の後に joint または myaction を指定してください"))?;
                mcts_params.mode = match val.to_ascii_lowercase().as_str() {
                    "joint" => MctsMode::Joint,
                    "myaction" | "my_action" => MctsMode::MyActionOnly,
                    other => anyhow::bail!("--mcts-mode は joint または myaction を指定してください (指定: {other})"),
                };
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                print_usage();
                anyhow::bail!("不明な引数: {}", other);
            }
        }
    }

    Ok(CliOptions {
        teams_path,
        seed,
        human_side,
        policy,
    })
}

fn run_game(opts: CliOptions, teams: TeamsFile) -> anyhow::Result<()> {
    let party_size = 3;
    let mut selection_rng = SmallRng::seed_from_u64(opts.seed ^ 0xD1B5_C0DE);
    let (selected_a, selected_b) =
        prepare_parties(
            &teams.team_a,
            &teams.team_b,
            opts.human_side,
            party_size,
            &mut selection_rng,
        )?;

    let mut battle = Battle::new_with_options(
        &selected_a,
        &selected_b,
        opts.seed,
        BattleOptions {
            auto_switch_on_faint: false,
        },
    );
    let mut ai_seed_rng = SmallRng::seed_from_u64(opts.seed ^ 0x9E37_79B9);
    let human = opts.human_side;
    let ai = match human {
        Side::A => Side::B,
        Side::B => Side::A,
    };

    let (human_candidates, opponent_candidates) = match human {
        Side::A => (&teams.team_a, &teams.team_b),
        Side::B => (&teams.team_b, &teams.team_a),
    };

    loop {
        if let Some(result) = battle.outcome() {
            ui::render(
                &battle.view(),
                human,
                human_candidates,
                opponent_candidates,
            );
            ui::print_result(result, human);
            break;
        }

        if battle.needs_switch(human) {
            let view = battle.view();
            ui::render(
                &view,
                human,
                human_candidates,
                opponent_candidates,
            );
            let action = ui::prompt_action(&view, human, true)?;
            if let PlayerAction::Switch(idx) = action {
                battle.manual_switch(human, idx);
            }
            continue;
        }

        if battle.needs_switch(ai) {
            if let Some(idx) = choose_ai_switch(&battle, ai) {
                battle.manual_switch(ai, idx);
            }
            continue;
        }

        let view = battle.view();
        ui::render(
            &view,
            human,
            human_candidates,
            opponent_candidates,
        );
        let human_action = ui::prompt_action(&view, human, false)?;
        let ai_seed = ai_seed_rng.gen::<u64>();
        let ai_action = choose_ai_action(&battle, ai, &opts.policy, ai_seed);

        let (a_action, b_action) = match human {
            Side::A => (Some(human_action), ai_action),
            Side::B => (ai_action, Some(human_action)),
        };

        battle.run_turn_with_actions(a_action, b_action);
        let move_lines = ui::format_move_events(battle.last_turn_move_events(), human);
        let status_lines = ui::format_status_events(battle.last_turn_status_events(), human);
        let updated_view = battle.view();
        let faint_lines = ui::describe_faints(&view, &updated_view, human);
        ui::print_turn_summary(&move_lines, &status_lines, &faint_lines);
        ui::wait_for_next_turn();
    }

    Ok(())
}

fn choose_ai_switch(battle: &Battle, side: Side) -> Option<usize> {
    battle.available_switches(side).into_iter().next()
}

fn choose_ai_action(battle: &Battle, side: Side, policy: &BattlePolicy, seed: u64) -> Option<PlayerAction> {
    if battle.needs_switch(side) {
        return choose_ai_switch(battle, side).map(PlayerAction::Switch);
    }
    battle.select_action(side, policy, seed)
}

fn parse_side(s: &str) -> Option<Side> {
    match s.to_ascii_lowercase().as_str() {
        "a" => Some(Side::A),
        "b" => Some(Side::B),
        _ => None,
    }
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run -p pokemon-battle-cli -- [--teams ../teams.json] [--seed N] [--human-side A|B] \
[--policy random|mcts] [--mcts-iters N] [--mcts-ms MS] [--rollout-horizon H] [--uct-c C] [--mcts-mode joint|myaction]"
    );
}

fn prepare_parties(
    team_a: &[Pokemon],
    team_b: &[Pokemon],
    human_side: Side,
    count: usize,
    rng: &mut SmallRng,
) -> anyhow::Result<(Vec<Pokemon>, Vec<Pokemon>)> {
    match human_side {
        Side::A => {
            let picks = ui::prompt_team_selection("あなた", team_a, team_b, count)?;
            let human_party = clone_selected(team_a, &picks);
            ui::print_selectiummary("あなた", &human_party);
            let ai_party = select_ai_party(team_b, count, rng)?;
            println!("相手も{}体のポケモンを選出しました。", ai_party.len());
            Ok((human_party, ai_party))
        }
        Side::B => {
            let picks = ui::prompt_team_selection("あなた", team_b, team_a, count)?;
            let human_party = clone_selected(team_b, &picks);
            ui::print_selection_summary("あなた", &human_party);
            let ai_party = select_ai_party(team_a, count, rng)?;
            println!("相手も{}体のポケモンを選出しました。", ai_party.len());
            Ok((ai_party, human_party))
        }
    }
}

fn clone_selected(team: &[Pokemon], indexes: &[usize]) -> Vec<Pokemon> {
    indexes
        .iter()
        .map(|&idx| {
            team.get(idx)
                .cloned()
                .expect("invalid team index during selection")
        })
        .collect()
}

fn select_ai_party(team: &[Pokemon], count: usize, rng: &mut SmallRng) -> anyhow::Result<Vec<Pokemon>> {
    if team.len() < count {
        anyhow::bail!("相手チームのポケモンが{}体未満です", count);
    }
    let mut indices: Vec<usize> = (0..team.len()).collect();
    indices.shuffle(rng);
    indices.truncate(count);
    Ok(indices.into_iter().map(|i| team[i].clone()).collect())
}

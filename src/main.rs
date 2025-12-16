use pokemon_battle_matrix::battle::BattlePolicy;
use pokemon_battle_matrix::{run, CliOptions, MctsMode, MctsParams};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

fn usage() -> ! {
    eprintln!(
        "Usage: cargo run --release -- [--teams teams.json] [--sims-per-cell N] [--seed SEED] [--output matrix.csv] \
--policy random|mcts [--mcts-iters N] [--mcts-ms MS] [--rollout-horizon H] [--uct-c C] [--mcts-mode joint|myaction]"
    );
    std::process::exit(1);
}

fn parse_args() -> anyhow::Result<CliOptions> {
    let mut teams_path = PathBuf::from("teams.json");
    let mut sims_per_cell = 100usize;
    let mut seed = 0u64;
    let mut output_path = PathBuf::from("matrix.csv");
    let mut policy = BattlePolicy::Random;
    let mut mcts_params = MctsParams::default();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--teams" => {
                teams_path = args.next().map(PathBuf::from).ok_or_else(|| {
                    anyhow::anyhow!("--teams requires a path (e.g. --teams teams.json)")
                })?;
            }
            "--sims-per-cell" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--sims-per-cell requires a number"))?;
                sims_per_cell = val.parse()?;
            }
            "--seed" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--seed requires a number"))?;
                seed = val.parse()?;
            }
            "--output" => {
                output_path = args.next().map(PathBuf::from).ok_or_else(|| {
                    anyhow::anyhow!("--output requires a path (e.g. --output matrix.csv)")
                })?;
            }
            "--policy" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--policy requires random or mcts"))?;
                policy = match val.to_ascii_lowercase().as_str() {
                    "random" => BattlePolicy::Random,
                    "mcts" => BattlePolicy::Mcts(mcts_params.clone()),
                    other => anyhow::bail!("Unknown policy {other} (use random or mcts)"),
                };
            }
            "--mcts-iters" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--mcts-iters requires a number"))?;
                mcts_params.iterations = Some(val.parse()?);
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--mcts-ms" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--mcts-ms requires milliseconds"))?;
                mcts_params.time_budget = Some(Duration::from_millis(val.parse()?));
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--rollout-horizon" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--rollout-horizon requires a number"))?;
                mcts_params.rollout_horizon = val.parse()?;
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--uct-c" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--uct-c requires a float"))?;
                mcts_params.exploration_constant = val.parse()?;
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--mcts-mode" => {
                let val = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--mcts-mode requires joint or myaction"))?;
                mcts_params.mode = match val.to_ascii_lowercase().as_str() {
                    "joint" => MctsMode::Joint,
                    "myaction" | "my_action" => MctsMode::MyActionOnly,
                    other => anyhow::bail!("--mcts-mode must be joint or myaction, got {other}"),
                };
                if matches!(policy, BattlePolicy::Mcts(_)) {
                    policy = BattlePolicy::Mcts(mcts_params.clone());
                }
            }
            "--help" | "-h" => usage(),
            other => return Err(anyhow::anyhow!("Unknown argument {other}")),
        }
    }

    Ok(CliOptions {
        teams_path,
        sims_per_cell,
        seed,
        output_path,
        policy,
    })
}

fn main() -> anyhow::Result<()> {
    let opts = parse_args()?;
    run(opts)
}

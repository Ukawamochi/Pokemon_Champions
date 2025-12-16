use pokemon_battle_matrix::{run, CliOptions};
use std::env;
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!(
        "Usage: cargo run --release -- [--teams teams.json] [--sims-per-cell N] [--seed SEED] [--output matrix.csv]"
    );
    std::process::exit(1);
}

fn parse_args() -> anyhow::Result<CliOptions> {
    let mut teams_path = PathBuf::from("teams.json");
    let mut sims_per_cell = 100usize;
    let mut seed = 0u64;
    let mut output_path = PathBuf::from("matrix.csv");

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--teams" => {
                teams_path = args.next().map(PathBuf::from).ok_or_else(|| {
                    anyhow::anyhow!("--teams requires a path (e.g. --teams teams.json)")
                })?;
            }
            "--sims-per-cell" => {
                let val = args.next().ok_or_else(|| anyhow::anyhow!("--sims-per-cell requires a number"))?;
                sims_per_cell = val.parse()?;
            }
            "--seed" => {
                let val = args.next().ok_or_else(|| anyhow::anyhow!("--seed requires a number"))?;
                seed = val.parse()?;
            }
            "--output" => {
                output_path = args.next().map(PathBuf::from).ok_or_else(|| {
                    anyhow::anyhow!("--output requires a path (e.g. --output matrix.csv)")
                })?;
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
    })
}

fn main() -> anyhow::Result<()> {
    let opts = parse_args()?;
    run(opts)
}

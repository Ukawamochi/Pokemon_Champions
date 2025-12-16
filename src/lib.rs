pub mod battle;
pub mod matrix;
pub mod model;
pub mod types;

use crate::matrix::{compute_matrix, validate_team_sizes};
use crate::model::TeamsFile;
use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CliOptions {
    pub teams_path: PathBuf,
    pub sims_per_cell: usize,
    pub seed: u64,
    pub output_path: PathBuf,
}

pub fn load_teams(path: &Path) -> anyhow::Result<TeamsFile> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read teams file at {}", path.display()))?;
    let parsed: TeamsFile = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse JSON from {}", path.display()))?;
    validate_team_sizes(&parsed)?;
    Ok(parsed)
}

pub fn run(opts: CliOptions) -> anyhow::Result<()> {
    if opts.sims_per_cell == 0 {
        anyhow::bail!("--sims-per-cell must be > 0");
    }
    let teams = load_teams(&opts.teams_path)?;
    let matrix = compute_matrix(&teams, opts.sims_per_cell, opts.seed);
    matrix::write_csv(&matrix, &opts.output_path)?;
    println!(
        "Wrote {}x{} matrix to {}",
        matrix.len(),
        matrix.get(0).map(|r| r.len()).unwrap_or(0),
        opts.output_path.display()
    );
    Ok(())
}

use crate::battle::{simulate_battle, BattleResult};
use crate::model::{Pokemon, TeamsFile};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

fn choose3_indices(len: usize) -> Vec<[usize; 3]> {
    let mut combos = Vec::new();
    for i in 0..len {
        for j in (i + 1)..len {
            for k in (j + 1)..len {
                combos.push([i, j, k]);
            }
        }
    }
    combos
}

fn selection_from_indices(team: &[Pokemon], indices: &[usize; 3]) -> Vec<Pokemon> {
    indices.iter().map(|&idx| team[idx].clone()).collect()
}

pub fn compute_matrix(teams: &TeamsFile, sims_per_cell: usize, seed: u64) -> Vec<Vec<f64>> {
    let combos_a = choose3_indices(teams.team_a.len());
    let combos_b = choose3_indices(teams.team_b.len());
    let selections_a: Vec<Vec<Pokemon>> = combos_a
        .iter()
        .map(|idx| selection_from_indices(&teams.team_a, idx))
        .collect();
    let selections_b: Vec<Vec<Pokemon>> = combos_b
        .iter()
        .map(|idx| selection_from_indices(&teams.team_b, idx))
        .collect();
    let tasks: Vec<(usize, usize)> = (0..selections_a.len())
        .flat_map(|a| (0..selections_b.len()).map(move |b| (a, b)))
        .collect();
    let cell_results: Vec<CellResult> = tasks
        .par_iter()
        .map(|(a_idx, b_idx)| {
            let mut cell_rng =
                SmallRng::seed_from_u64(seed ^ ((*a_idx as u64) << 32) ^ (*b_idx as u64));
            let a_sel = &selections_a[*a_idx];
            let b_sel = &selections_b[*b_idx];
            let mut a_wins = 0u64;
            let mut ties = 0u64;
            for _ in 0..sims_per_cell {
                let battle_seed = cell_rng.gen();
                match simulate_battle(a_sel, b_sel, battle_seed) {
                    BattleResult::AWins => a_wins += 1,
                    BattleResult::BWins => {}
                    BattleResult::Tie => ties += 1,
                }
            }
            let total = sims_per_cell as f64;
            let win_rate = (a_wins as f64 + 0.5 * ties as f64) / total;
            CellResult {
                a_idx: *a_idx,
                b_idx: *b_idx,
                win_rate,
            }
        })
        .collect();

    let mut matrix = vec![vec![0.0; selections_b.len()]; selections_a.len()];
    for cell in cell_results {
        matrix[cell.a_idx][cell.b_idx] = cell.win_rate;
    }
    matrix
}

pub fn write_csv(matrix: &[Vec<f64>], path: &std::path::Path) -> anyhow::Result<()> {
    let mut out = String::new();
    for (row_idx, row) in matrix.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            if col_idx > 0 {
                out.push(',');
            }
            out.push_str(&format!("{value:.4}"));
        }
        if row_idx + 1 < matrix.len() {
            out.push('\n');
        }
    }
    std::fs::write(path, out)?;
    Ok(())
}

struct CellResult {
    a_idx: usize,
    b_idx: usize,
    win_rate: f64,
}

pub fn validate_team_sizes(teams: &TeamsFile) -> anyhow::Result<()> {
    if teams.team_a.len() != 6 || teams.team_b.len() != 6 {
        anyhow::bail!("Expected exactly 6 Pokemon per team");
    }
    Ok(())
}

//! High-level battle engine wrapper for step-based simulations.

use crate::sim::battle::{
    apply_end_of_turn_effects, apply_on_entry_abilities, execute_turn, Action, BattleResult,
    BattleState,
};
use crate::sim::Pokemon;
use rand::rngs::SmallRng;
use rand::SeedableRng;

/// Player identifier for selecting actions and observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Player {
    A,
    B,
}

/// Result of a single engine step.
#[derive(Clone, Debug)]
pub struct StepResult {
    /// Event log derived from the state transition.
    pub events: Vec<String>,
    /// Reward for player A.
    pub reward_a: f32,
    /// Reward for player B.
    pub reward_b: f32,
    /// State snapshot before the step.
    pub before: BattleState,
    /// State snapshot after the step.
    pub after: BattleState,
    /// Terminal outcome if the battle ended.
    pub outcome: Option<BattleResult>,
}

/// Step-based battle engine for external callers (e.g., RL loops).
pub struct BattleEngine {
    state: BattleState,
    rng: SmallRng,
}

impl BattleEngine {
    /// Create a new engine from two teams.
    ///
    /// `team_a`/`team_b` must each contain at least one Pokémon.
    pub fn new(team_a: &[Pokemon], team_b: &[Pokemon], seed: u64) -> Self {
        assert!(!team_a.is_empty(), "team_a must contain at least one Pokemon");
        assert!(!team_b.is_empty(), "team_b must contain at least one Pokemon");
        let mut team_a = team_a.to_vec();
        let mut team_b = team_b.to_vec();
        let pokemon_a = team_a.remove(0);
        let pokemon_b = team_b.remove(0);
        let mut state = BattleState::new_with_bench(pokemon_a, pokemon_b, team_a, team_b);
        apply_on_entry_abilities(&mut state);
        let rng = SmallRng::seed_from_u64(seed);
        Self { state, rng }
    }

    /// Advance the battle by one turn using the provided actions.
    pub fn step(&mut self, action_a: Action, action_b: Action) -> StepResult {
        if let Some(outcome) = battle_outcome(&self.state) {
            let snapshot = self.state.clone();
            let (reward_a, reward_b) = outcome_rewards(Some(outcome));
            return StepResult {
                events: vec![format!("terminal: {:?}", outcome)],
                reward_a,
                reward_b,
                before: snapshot.clone(),
                after: snapshot,
                outcome: Some(outcome),
            };
        }

        let before = self.state.clone();
        reset_turn_flags(&mut self.state);
        execute_turn(&mut self.state, action_a, action_b, &mut self.rng);
        apply_end_of_turn_effects(&mut self.state, &mut self.rng);
        self.state.turn = self.state.turn.saturating_add(1);

        let outcome = battle_outcome(&self.state);
        let (reward_a, reward_b) = outcome_rewards(outcome);
        let events = build_events(&before, &self.state, outcome);
        StepResult {
            events,
            reward_a,
            reward_b,
            before,
            after: self.state.clone(),
            outcome,
        }
    }

    /// Returns true if the current state is terminal.
    pub fn is_terminal(&self) -> bool {
        battle_outcome(&self.state).is_some()
    }

    /// List legal actions for a player given the current state.
    pub fn legal_actions(&self, player: Player) -> Vec<Action> {
        match player {
            Player::A => actions_for(&self.state.pokemon_a, &self.state.bench_a),
            Player::B => actions_for(&self.state.pokemon_b, &self.state.bench_b),
        }
    }

    /// Access the internal battle state.
    pub fn state(&self) -> &BattleState {
        &self.state
    }
}

fn actions_for(active: &Pokemon, bench: &[Pokemon]) -> Vec<Action> {
    let mut actions: Vec<Action> = active
        .moves
        .iter()
        .enumerate()
        .map(|(idx, _)| Action::Move(idx))
        .collect();
    for (idx, candidate) in bench.iter().enumerate() {
        if !candidate.is_fainted() {
            actions.push(Action::Switch(idx));
        }
    }
    actions
}

fn reset_turn_flags(state: &mut BattleState) {
    state.pokemon_a.protect_active = false;
    state.pokemon_b.protect_active = false;
    state.pokemon_a.kings_shield_active = false;
    state.pokemon_b.kings_shield_active = false;
    state.pokemon_a.roosted = false;
    state.pokemon_b.roosted = false;
    state.pokemon_a.semi_invulnerable = false;
    state.pokemon_b.semi_invulnerable = false;
}

fn battle_outcome(state: &BattleState) -> Option<BattleResult> {
    let a_available = side_has_available(&state.pokemon_a, &state.bench_a);
    let b_available = side_has_available(&state.pokemon_b, &state.bench_b);
    match (a_available, b_available) {
        (false, false) => Some(BattleResult::Draw),
        (false, true) => Some(BattleResult::TeamBWins),
        (true, false) => Some(BattleResult::TeamAWins),
        (true, true) => None,
    }
}

fn side_has_available(active: &Pokemon, bench: &[Pokemon]) -> bool {
    if !active.is_fainted() {
        return true;
    }
    bench.iter().any(|pokemon| !pokemon.is_fainted())
}

fn outcome_rewards(outcome: Option<BattleResult>) -> (f32, f32) {
    match outcome {
        Some(BattleResult::TeamAWins) => (1.0, -1.0),
        Some(BattleResult::TeamBWins) => (-1.0, 1.0),
        Some(BattleResult::Draw) | None => (0.0, 0.0),
    }
}

fn build_events(before: &BattleState, after: &BattleState, outcome: Option<BattleResult>) -> Vec<String> {
    let mut events = Vec::new();
    if before.turn != after.turn {
        events.push(format!("turn {} -> {}", before.turn, after.turn));
    }
    append_active_events(&mut events, "side_a", &before.pokemon_a, &after.pokemon_a);
    append_active_events(&mut events, "side_b", &before.pokemon_b, &after.pokemon_b);
    if let Some(result) = outcome {
        events.push(format!("outcome: {:?}", result));
    }
    events
}

fn append_active_events(events: &mut Vec<String>, label: &str, before: &Pokemon, after: &Pokemon) {
    if before.species != after.species {
        events.push(format!("{}_switch", label));
    }
    if before.current_hp != after.current_hp {
        events.push(format!(
            "{}_hp {} -> {}",
            label, before.current_hp, after.current_hp
        ));
    }
    if before.status != after.status {
        events.push(format!("{}_status {:?} -> {:?}", label, before.status, after.status));
    }
}

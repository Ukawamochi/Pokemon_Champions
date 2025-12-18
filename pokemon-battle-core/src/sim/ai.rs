use crate::sim::battle::{Action, BattleState};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

pub trait BattleAI {
    fn choose_action(&mut self, state: &BattleState, valid_actions: &[Action]) -> Action;
}

pub struct RandomAI {
    rng: SmallRng,
}

impl RandomAI {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}

impl BattleAI for RandomAI {
    fn choose_action(&mut self, _state: &BattleState, valid_actions: &[Action]) -> Action {
        *valid_actions
            .choose(&mut self.rng)
            .unwrap_or(&Action::Move(0))
    }
}

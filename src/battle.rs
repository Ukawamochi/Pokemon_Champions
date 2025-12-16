use crate::model::{Move, MoveCategory, Pokemon};
use crate::types::type_effectiveness;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Side {
    A,
    B,
}

impl Side {
    fn opponent(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BattleResult {
    AWins,
    BWins,
    Tie,
}

#[derive(Clone)]
struct Battler {
    pokemon: Pokemon,
    current_hp: i32,
}

impl Battler {
    fn is_fainted(&self) -> bool {
        self.current_hp <= 0
    }
}

pub struct Battle {
    team_a: Vec<Battler>,
    team_b: Vec<Battler>,
    active_a: usize,
    active_b: usize,
    // Ref: pokemon-showdown/sim/battle.ts: Battle uses a shared PRNG (Battle.prng).
    rng: SmallRng,
}

impl Battle {
    pub fn new(team_a: &[Pokemon], team_b: &[Pokemon], seed: u64) -> Self {
        let mut a = Vec::new();
        for p in team_a {
            a.push(Battler {
                pokemon: p.clone(),
                current_hp: p.initial_hp(),
            });
        }
        let mut b = Vec::new();
        for p in team_b {
            b.push(Battler {
                pokemon: p.clone(),
                current_hp: p.initial_hp(),
            });
        }
        Battle {
            team_a: a,
            team_b: b,
            active_a: 0,
            active_b: 0,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    fn team(&self, side: Side) -> &Vec<Battler> {
        match side {
            Side::A => &self.team_a,
            Side::B => &self.team_b,
        }
    }

    fn team_mut(&mut self, side: Side) -> &mut Vec<Battler> {
        match side {
            Side::A => &mut self.team_a,
            Side::B => &mut self.team_b,
        }
    }

    fn active_index(&self, side: Side) -> usize {
        match side {
            Side::A => self.active_a,
            Side::B => self.active_b,
        }
    }

    fn set_active_index(&mut self, side: Side, idx: usize) {
        match side {
            Side::A => self.active_a = idx,
            Side::B => self.active_b = idx,
        }
    }

    fn active(&self, side: Side) -> &Battler {
        let idx = self.active_index(side);
        &self.team(side)[idx]
    }

    fn active_mut(&mut self, side: Side) -> &mut Battler {
        let idx = self.active_index(side);
        &mut self.team_mut(side)[idx]
    }

    fn send_next(&mut self, side: Side) {
        let team = self.team(side);
        let next = team
            .iter()
            .enumerate()
            .find(|(_, p)| !p.is_fainted())
            .map(|(idx, _)| idx);
        if let Some(idx) = next {
            self.set_active_index(side, idx);
        }
    }

    fn alive_count(&self, side: Side) -> usize {
        self.team(side).iter().filter(|p| !p.is_fainted()).count()
    }

    fn choose_action(&mut self, side: Side) -> Option<usize> {
        let moves = &self.active(side).pokemon.moves;
        if moves.is_empty() {
            return None;
        }
        Some(self.rng.gen_range(0..moves.len()))
    }

    fn planned_actions(&mut self, a_move: Option<usize>, b_move: Option<usize>) -> Vec<PlannedAction> {
        let mut actions = Vec::new();
        if let Some(m) = a_move {
            let battler = self.active(Side::A);
            let mv = &battler.pokemon.moves[m];
            actions.push(PlannedAction {
                side: Side::A,
                move_index: m,
                priority: mv.priority,
                speed: battler.pokemon.stats.spe,
                // Ref: pokemon-showdown/sim/battle.ts: comparePriority/speed and then PRNG tie-breaker via this.random().
                tie_break: self.rng.gen(),
            });
        }
        if let Some(m) = b_move {
            let battler = self.active(Side::B);
            let mv = &battler.pokemon.moves[m];
            actions.push(PlannedAction {
                side: Side::B,
                move_index: m,
                priority: mv.priority,
                speed: battler.pokemon.stats.spe,
                tie_break: self.rng.gen(),
            });
        }
        actions.sort_by(|lhs, rhs| {
            rhs.priority
                .cmp(&lhs.priority)
                .then_with(|| rhs.speed.cmp(&lhs.speed))
                .then_with(|| lhs.tie_break.cmp(&rhs.tie_break))
        });
        actions
    }

    fn run_turn(&mut self) {
        let a_move = self.choose_action(Side::A);
        let b_move = self.choose_action(Side::B);
        let actions = self.planned_actions(a_move, b_move);
        for action in actions {
            if self.alive_count(action.side) == 0 || self.alive_count(action.side.opponent()) == 0 {
                return;
            }
            if self.active(action.side).is_fainted() {
                continue;
            }
            self.execute_move(action.side, action.move_index);
        }
    }

    fn execute_move(&mut self, side: Side, move_idx: usize) {
        let move_def = self.active(side).pokemon.moves[move_idx].clone();
        if matches!(move_def.category, MoveCategory::Status) || move_def.power == 0 {
            return;
        }
        if !roll_accuracy(&move_def, &mut self.rng) {
            return;
        }
        let damage = compute_damage(
            &self.active(side).pokemon,
            &self.active(side.opponent()).pokemon,
            &move_def,
            &mut self.rng,
        );
        if damage == 0 {
            return;
        }
        {
            let target = self.active_mut(side.opponent());
            target.current_hp -= damage as i32;
        }
        if self.active(side.opponent()).is_fainted() {
            self.send_next(side.opponent());
        }
    }
}

#[derive(Clone)]
struct PlannedAction {
    side: Side,
    move_index: usize,
    priority: i32,
    speed: u32,
    tie_break: u64,
}

pub fn simulate_battle(team_a: &[Pokemon], team_b: &[Pokemon], seed: u64) -> BattleResult {
    let mut battle = Battle::new(team_a, team_b, seed);
    // Ref: pokemon-showdown/sim/battle.ts: turn loop until one side has no Pokemon left.
    for _turn in 0..500 {
        if battle.alive_count(Side::A) == 0 && battle.alive_count(Side::B) == 0 {
            return BattleResult::Tie;
        }
        if battle.alive_count(Side::A) == 0 {
            return BattleResult::BWins;
        }
        if battle.alive_count(Side::B) == 0 {
            return BattleResult::AWins;
        }
        battle.run_turn();
    }
    BattleResult::Tie
}

fn roll_accuracy(move_def: &Move, rng: &mut SmallRng) -> bool {
    // Ref: pokemon-showdown/sim/battle.ts: tryMoveHit uses randomChance(move.accuracy, 100).
    if move_def.accuracy >= 100.0 {
        return true;
    }
    let roll = rng.gen_range(0.0..100.0);
    roll < move_def.accuracy
}

pub fn compute_damage(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_def: &Move,
    rng: &mut SmallRng,
) -> u32 {
    // Ref: pokemon-showdown/sim/damage.ts: getDamage formula simplified to level scaling + STAB/type/random.
    if move_def.power == 0 || matches!(move_def.category, MoveCategory::Status) {
        return 0;
    }
    let atk = match move_def.category {
        MoveCategory::Physical => attacker.stats.atk as f32,
        MoveCategory::Special => attacker.stats.spa as f32,
        MoveCategory::Status => return 0,
    };
    let def = match move_def.category {
        MoveCategory::Physical => defender.stats.def as f32,
        MoveCategory::Special => defender.stats.spd as f32,
        MoveCategory::Status => return 0,
    };
    if def == 0.0 {
        return 0;
    }
    let level = 50.0;
    let mut base = (((2.0 * level / 5.0 + 2.0) * move_def.power as f32 * atk / def) / 50.0) + 2.0;
    let stab = if attacker
        .types
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&move_def.move_type))
    {
        1.5
    } else {
        1.0
    };
    let type_mod = type_effectiveness(&move_def.move_type, &defender.types);
    if type_mod == 0.0 {
        return 0;
    }
    let rand_mod = (rng.gen_range(85..=100) as f32) / 100.0; // PS randomDamage roll.
    base *= stab * type_mod * rand_mod;
    base.floor().max(1.0) as u32
}

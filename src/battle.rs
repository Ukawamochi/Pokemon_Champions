use crate::items::{ItemEffect, ITEM_TABLE};
use crate::mcts::MctsParams;
use crate::model::{HazardMove, Move, MoveCategory, Pokemon, StatBoosts, StatusCondition, Weather};
use crate::types::type_effectiveness;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::cmp::min;

// 参考: pokemon-showdown/sim/battle.ts, pokemon-showdown/sim/pokemon.ts, pokemon-showdown/sim/damage.ts など。

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Side {
    A,
    B,
}

impl Side {
    pub fn opponent(self) -> Side {
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

#[derive(Clone, Debug)]
pub struct BattleOptions {
    pub auto_switch_on_faint: bool,
}

impl Default for BattleOptions {
    fn default() -> Self {
        Self {
            auto_switch_on_faint: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimulationOptions {
    pub policy_a: BattlePolicy,
    pub policy_b: BattlePolicy,
    pub battle: BattleOptions,
}

impl Default for SimulationOptions {
    fn default() -> Self {
        Self {
            policy_a: BattlePolicy::Random,
            policy_b: BattlePolicy::Random,
            battle: BattleOptions::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum BattlePolicy {
    Random,
    Mcts(MctsParams),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PlayerAction {
    Move(usize),
    Switch(usize),
}

#[derive(Clone, Default)]
struct StatStages {
    atk: i8,
    def: i8,
    spa: i8,
    spd: i8,
    spe: i8,
    acc: i8,
    eva: i8,
}

#[derive(Clone, Debug)]
pub struct StatStagesView {
    pub atk: i8,
    pub def: i8,
    pub spa: i8,
    pub spd: i8,
    pub spe: i8,
    pub acc: i8,
    pub eva: i8,
}

#[derive(Clone, Debug)]
pub struct MoveView {
    pub name: String,
    pub move_type: String,
    pub category: MoveCategory,
    pub power: u32,
    pub accuracy: f32,
    pub priority: i32,
    pub remaining_pp: i32,
    pub max_pp: u8,
}

#[derive(Clone, Debug)]
pub struct BattlerView {
    pub index: usize,
    pub name: String,
    pub types: Vec<String>,
    pub hp: i32,
    pub max_hp: i32,
    pub status: Option<StatusCondition>,
    pub stat_stages: StatStagesView,
    pub moves: Vec<MoveView>,
    pub item: Option<String>,
    pub ability: Option<String>,
    pub is_fainted: bool,
}

#[derive(Clone, Debug)]
pub struct TeamMemberView {
    pub index: usize,
    pub name: String,
    pub types: Vec<String>,
    pub hp: i32,
    pub max_hp: i32,
    pub status: Option<StatusCondition>,
    pub is_active: bool,
    pub is_fainted: bool,
}

#[derive(Clone, Debug)]
pub struct HazardsView {
    pub stealth_rock: bool,
    pub spikes: u8,
    pub toxic_spikes: u8,
}

#[derive(Clone, Debug)]
pub struct ScreensView {
    pub reflect: u8,
    pub light_screen: u8,
}

#[derive(Clone, Debug)]
pub struct SideView {
    pub active: BattlerView,
    pub team: Vec<TeamMemberView>,
    pub hazards: HazardsView,
    pub screens: ScreensView,
}

#[derive(Clone, Debug)]
pub struct BattleView {
    pub side_a: SideView,
    pub side_b: SideView,
    pub weather: Option<Weather>,
    pub weather_turns: u8,
}

#[derive(Clone, Debug)]
pub enum MoveOutcome {
    Missed,
    Protected,
    NoEffect { effectiveness: f32 },
    Hit { effectiveness: f32, damage: u32 },
    StatusOnly,
}

#[derive(Clone, Debug)]
pub struct MoveEventView {
    pub side: Side,
    pub pokemon: String,
    pub move_name: String,
    pub outcome: MoveOutcome,
}

#[derive(Clone, Debug)]
pub struct StatusEventView {
    pub side: Side,
    pub pokemon: String,
    pub message: &'static str,
}

#[derive(Clone)]
struct Battler {
    pokemon: Pokemon,
    current_hp: i32,
    status: Option<StatusCondition>,
    sleep_turns: u8,
    toxic_counter: u8,
    stat_stages: StatStages,
    move_pp: Vec<i32>,
    last_move: Option<usize>,
    choice_lock: Option<usize>,
    protecting: bool,
    sash_used: bool,
    berry_used: bool,
}

impl Battler {
    fn new(p: &Pokemon) -> Self {
        Self {
            pokemon: p.clone(),
            current_hp: p.initial_hp(),
            status: None,
            sleep_turns: 0,
            toxic_counter: 0,
            stat_stages: StatStages::default(),
            move_pp: p.moves.iter().map(|m| m.pp as i32).collect(),
            last_move: None,
            choice_lock: None,
            protecting: false,
            sash_used: false,
            berry_used: false,
        }
    }

    fn is_fainted(&self) -> bool {
        self.current_hp <= 0
    }

    fn max_hp(&self) -> i32 {
        self.pokemon.stats.hp as i32
    }

    fn heal(&mut self, amount: i32) {
        self.current_hp = min(self.current_hp + amount, self.max_hp());
    }

    fn apply_boosts(&mut self, boosts: &StatBoosts) {
        self.stat_stages.atk = clamp_stage(self.stat_stages.atk + boosts.atk);
        self.stat_stages.def = clamp_stage(self.stat_stages.def + boosts.def);
        self.stat_stages.spa = clamp_stage(self.stat_stages.spa + boosts.spa);
        self.stat_stages.spd = clamp_stage(self.stat_stages.spd + boosts.spd);
        self.stat_stages.spe = clamp_stage(self.stat_stages.spe + boosts.spe);
        self.stat_stages.acc = clamp_stage(self.stat_stages.acc + boosts.acc);
        self.stat_stages.eva = clamp_stage(self.stat_stages.eva + boosts.eva);
    }
}

fn clamp_stage(v: i8) -> i8 {
    v.max(-6).min(6)
}

#[derive(Default, Clone)]
struct Hazards {
    stealth_rock: bool,
    spikes: u8,
    toxic_spikes: u8,
}

#[derive(Default, Clone)]
struct Screens {
    reflect: u8,
    light_screen: u8,
}

#[derive(Default, Clone)]
struct WeatherState {
    current: Option<Weather>,
    turns: u8,
}

#[derive(Default, Clone)]
struct SideState {
    hazards: Hazards,
    screens: Screens,
}

#[derive(Clone)]
pub struct Battle {
    team_a: Vec<Battler>,
    team_b: Vec<Battler>,
    active_a: usize,
    active_b: usize,
    // 参考: pokemon-showdown/sim/battle.ts: Battle は共有 PRNG (Battle.prng) を用いる。
    rng: SmallRng,
    weather: WeatherState,
    side_state: [SideState; 2],
    trick_room: bool,
    trick_room_turns: u8,
    options: BattleOptions,
    last_turn_move_events: Vec<MoveEventView>,
    last_turn_status_events: Vec<StatusEventView>,
}

impl Battle {
    pub fn new(team_a: &[Pokemon], team_b: &[Pokemon], seed: u64) -> Self {
        Self::new_with_options(team_a, team_b, seed, BattleOptions::default())
    }

    pub fn new_with_options(
        team_a: &[Pokemon],
        team_b: &[Pokemon],
        seed: u64,
        options: BattleOptions,
    ) -> Self {
        let mut a = Vec::new();
        for p in team_a {
            a.push(Battler::new(p));
        }
        let mut b = Vec::new();
        for p in team_b {
            b.push(Battler::new(p));
        }
        Battle {
            team_a: a,
            team_b: b,
            active_a: 0,
            active_b: 0,
            rng: SmallRng::seed_from_u64(seed),
            weather: WeatherState::default(),
            side_state: [SideState::default(), SideState::default()],
            trick_room: false,
            trick_room_turns: 0,
            options,
            last_turn_move_events: Vec::new(),
            last_turn_status_events: Vec::new(),
        }
    }

    pub fn view(&self) -> BattleView {
        BattleView {
            side_a: self.side_view(Side::A),
            side_b: self.side_view(Side::B),
            weather: self.weather.current.clone(),
            weather_turns: self.weather.turns,
        }
    }

    pub fn last_turn_move_events(&self) -> &[MoveEventView] {
        &self.last_turn_move_events
    }

    pub fn last_turn_status_events(&self) -> &[StatusEventView] {
        &self.last_turn_status_events
    }

    pub fn needs_switch(&self, side: Side) -> bool {
        self.active(side).is_fainted()
            && self
                .team(side)
                .iter()
                .enumerate()
                .any(|(idx, b)| idx != self.active_index(side) && !b.is_fainted())
    }

    pub fn available_switches(&self, side: Side) -> Vec<usize> {
        self.team(side)
            .iter()
            .enumerate()
            .filter(|(idx, b)| *idx != self.active_index(side) && !b.is_fainted())
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn manual_switch(&mut self, side: Side, target_idx: usize) {
        self.switch_to(side, target_idx);
    }

    pub fn random_action(&mut self, side: Side) -> Option<PlayerAction> {
        self.choose_action(side)
    }

    pub fn outcome(&self) -> Option<BattleResult> {
        let a_alive = self.alive_count(Side::A);
        let b_alive = self.alive_count(Side::B);
        if a_alive == 0 && b_alive == 0 {
            Some(BattleResult::Tie)
        } else if a_alive == 0 {
            Some(BattleResult::BWins)
        } else if b_alive == 0 {
            Some(BattleResult::AWins)
        } else {
            None
        }
    }

    fn side_view(&self, side: Side) -> SideView {
        let active_idx = self.active_index(side);
        let team = self
            .team(side)
            .iter()
            .enumerate()
            .map(|(idx, b)| TeamMemberView {
                index: idx,
                name: b.pokemon.name.clone(),
                types: b.pokemon.types.clone(),
                hp: b.current_hp,
                max_hp: b.max_hp(),
                status: b.status.clone(),
                is_active: idx == active_idx,
                is_fainted: b.is_fainted(),
            })
            .collect();
        let active = self.battler_view(side, active_idx);
        SideView {
            active,
            team,
            hazards: HazardsView {
                stealth_rock: self.side_state(side).hazards.stealth_rock,
                spikes: self.side_state(side).hazards.spikes,
                toxic_spikes: self.side_state(side).hazards.toxic_spikes,
            },
            screens: ScreensView {
                reflect: self.side_state(side).screens.reflect,
                light_screen: self.side_state(side).screens.light_screen,
            },
        }
    }

    fn battler_view(&self, side: Side, idx: usize) -> BattlerView {
        let b = &self.team(side)[idx];
        let moves = b
            .pokemon
            .moves
            .iter()
            .enumerate()
            .map(|(i, mv)| MoveView {
                name: mv.name.clone(),
                move_type: mv.move_type.clone(),
                category: mv.category.clone(),
                power: mv.power,
                accuracy: mv.accuracy,
                priority: mv.priority,
                remaining_pp: b.move_pp.get(i).copied().unwrap_or(0),
                max_pp: mv.pp,
            })
            .collect();
        BattlerView {
            index: idx,
            name: b.pokemon.name.clone(),
            types: b.pokemon.types.clone(),
            hp: b.current_hp,
            max_hp: b.max_hp(),
            status: b.status.clone(),
            stat_stages: self.stat_stages_view(&b.stat_stages),
            moves,
            item: b.pokemon.item.clone(),
            ability: b.pokemon.ability.clone(),
            is_fainted: b.is_fainted(),
        }
    }

    fn stat_stages_view(&self, stages: &StatStages) -> StatStagesView {
        StatStagesView {
            atk: stages.atk,
            def: stages.def,
            spa: stages.spa,
            spd: stages.spd,
            spe: stages.spe,
            acc: stages.acc,
            eva: stages.eva,
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

    fn side_state(&self, side: Side) -> &SideState {
        &self.side_state[match side {
            Side::A => 0,
            Side::B => 1,
        }]
    }

    fn side_state_mut(&mut self, side: Side) -> &mut SideState {
        &mut self.side_state[match side {
            Side::A => 0,
            Side::B => 1,
        }]
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
            self.switch_to(side, idx);
        }
    }

    fn switch_to(&mut self, side: Side, target_idx: usize) {
        if target_idx >= self.team(side).len() {
            return;
        }
        if self.team(side)[target_idx].is_fainted() {
            return;
        }
        if self.active_index(side) == target_idx {
            return;
        }
        let current_idx = self.active_index(side);
        if let Some(outgoing) = self.team_mut(side).get_mut(current_idx) {
            outgoing.choice_lock = None;
            outgoing.last_move = None;
        }
        self.set_active_index(side, target_idx);
        if let Some(active) = self.team_mut(side).get_mut(target_idx) {
            active.protecting = false;
        }
        self.apply_hazards_on_switch(side);
    }

    fn alive_count(&self, side: Side) -> usize {
        self.team(side).iter().filter(|p| !p.is_fainted()).count()
    }

    pub fn legal_actions(&self, side: Side) -> Vec<PlayerAction> {
        let battler = self.active(side);
        let moves = &battler.pokemon.moves;
        if moves.is_empty() {
            return Vec::new();
        }
        let usable: Vec<usize> = moves
            .iter()
            .enumerate()
            .filter(|(i, _)| battler.move_pp.get(*i).copied().unwrap_or(0) > 0)
            .map(|(i, _)| i)
            .collect();
        if usable.is_empty() {
            return Vec::new();
        }
        if let Some(lock) = battler.choice_lock {
            if battler.move_pp.get(lock).copied().unwrap_or(0) > 0 {
                return vec![PlayerAction::Move(lock)];
            }
        }
        usable.into_iter().map(PlayerAction::Move).collect()
    }

    fn random_action_with_rng(&self, side: Side, rng: &mut SmallRng) -> Option<PlayerAction> {
        let actions = self.legal_actions(side);
        if actions.is_empty() {
            return None;
        }
        actions.choose(rng).cloned()
    }

    fn choose_action(&mut self, side: Side) -> Option<PlayerAction> {
        let actions = self.legal_actions(side);
        if actions.is_empty() {
            return None;
        }
        actions.choose(&mut self.rng).cloned()
    }

    fn planned_actions(
        &mut self,
        a_action: Option<PlayerAction>,
        b_action: Option<PlayerAction>,
    ) -> Vec<PlannedAction> {
        let mut actions = Vec::new();

        if let Some(action) = a_action.clone() {
            self.push_action(Side::A, action, &mut actions);
        }
        if let Some(action) = b_action.clone() {
            self.push_action(Side::B, action, &mut actions);
        }

        if a_action.is_none() && self.alive_count(Side::A) > 0 {
            self.push_move_action(Side::A, None, &mut actions);
        }
        if b_action.is_none() && self.alive_count(Side::B) > 0 {
            self.push_move_action(Side::B, None, &mut actions);
        }

        actions.sort_by(|lhs, rhs| {
            rhs.priority_value
                .partial_cmp(&lhs.priority_value)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| rhs.speed.cmp(&lhs.speed))
                .then_with(|| lhs.tie_break.cmp(&rhs.tie_break))
        });
        actions
    }

    fn push_action(&mut self, side: Side, action: PlayerAction, actions: &mut Vec<PlannedAction>) {
        match action {
            PlayerAction::Move(idx) => self.push_move_action(side, Some(idx), actions),
            PlayerAction::Switch(target_idx) => self.push_switch_action(side, target_idx, actions),
        }
    }

    fn push_move_action(
        &mut self,
        side: Side,
        idx: Option<usize>,
        actions: &mut Vec<PlannedAction>,
    ) {
        let (mv, original) = self.resolve_move(side, idx);
        let frac = self.fractional_priority(side, &mv);
        actions.push(PlannedAction {
            side,
            move_index: original,
            switch_target: None,
            move_def: mv.clone(),
            priority_value: mv.priority as f32 + frac,
            speed: self.calc_effective_speed(side),
            tie_break: self.rng.gen(),
            battler_slot: self.active_index(side),
        });
    }

    fn push_switch_action(
        &mut self,
        side: Side,
        target_idx: usize,
        actions: &mut Vec<PlannedAction>,
    ) {
        actions.push(PlannedAction {
            side,
            move_index: None,
            switch_target: Some(target_idx),
            move_def: struggle_move(),
            priority_value: 1000.0, // この簡略化では交代を最優先で処理する。
            speed: self.calc_effective_speed(side),
            tie_break: self.rng.gen(),
            battler_slot: self.active_index(side),
        });
    }

    fn calc_effective_speed(&self, side: Side) -> u32 {
        let b = self.active(side);
        let mut speed = b.pokemon.stats.spe as f32;
        speed *= stage_modifier(b.stat_stages.spe);
        if matches!(b.status, Some(StatusCondition::Paralysis)) {
            speed *= 0.25;
        }
        let eff = item_effect(b);
        if let Some(m) = eff.speed_mult {
            speed *= m;
        }
        // 参考: pokemon-showdown/sim/pokemon.ts getActionSpeed の Trick Room 反転。
        if self.trick_room {
            speed = 10000.0 - speed;
        }
        speed as u32
    }

    #[allow(dead_code)]
    fn run_turn(&mut self) {
        let a_action = self.choose_action(Side::A);
        let b_action = self.choose_action(Side::B);
        self.run_turn_with_actions(a_action, b_action);
    }

    pub fn run_turn_with_actions(
        &mut self,
        a_action: Option<PlayerAction>,
        b_action: Option<PlayerAction>,
    ) {
        for side in [Side::A, Side::B] {
            self.active_mut(side).protecting = false;
        }
        let actions = self.planned_actions(a_action, b_action);
        self.last_turn_move_events.clear();
        self.last_turn_status_events.clear();
        for action in actions {
            if self.alive_count(action.side) == 0 || self.alive_count(action.side.opponent()) == 0 {
                return;
            }
            if action.switch_target.is_none() {
                if self.active_index(action.side) != action.battler_slot {
                    continue;
                }
                if self.active(action.side).is_fainted() {
                    continue;
                }
            }
            self.execute_action(&action);
        }
        self.end_of_turn();
    }

    pub fn select_action(
        &self,
        side: Side,
        policy: &BattlePolicy,
        seed: u64,
    ) -> Option<PlayerAction> {
        match policy {
            BattlePolicy::Random => {
                let mut rng = SmallRng::seed_from_u64(seed);
                self.random_action_with_rng(side, &mut rng)
            }
            BattlePolicy::Mcts(params) => crate::mcts::mcts_action(self, side, params, seed),
        }
    }

    fn run_turn_with_policies(&mut self, options: &SimulationOptions) {
        let seed_a = self.rng.gen::<u64>();
        let seed_b = self.rng.gen::<u64>();
        let a_action = self.select_action(Side::A, &options.policy_a, seed_a);
        let b_action = self.select_action(Side::B, &options.policy_b, seed_b);
        self.run_turn_with_actions(a_action, b_action);
    }

    pub fn clone_with_rng_seed(&self, seed: u64) -> Self {
        let mut cloned = self.clone();
        cloned.rng = SmallRng::seed_from_u64(seed);
        cloned
    }

    pub fn terminal_result(&self) -> Option<BattleResult> {
        let alive_a = self.alive_count(Side::A);
        let alive_b = self.alive_count(Side::B);
        if alive_a == 0 && alive_b == 0 {
            Some(BattleResult::Tie)
        } else if alive_a == 0 {
            Some(BattleResult::BWins)
        } else if alive_b == 0 {
            Some(BattleResult::AWins)
        } else {
            None
        }
    }

    fn move_pp_mut(&mut self, side: Side, idx: usize) -> Option<&mut i32> {
        let active_idx = self.active_index(side);
        self.team_mut(side)
            .get_mut(active_idx)
            .and_then(|b| b.move_pp.get_mut(idx))
    }

    fn execute_action(&mut self, action: &PlannedAction) {
        let side = action.side;
        let target_side = side.opponent();
        let move_def = action.move_def.clone();

        if let Some(target_idx) = action.switch_target {
            self.switch_to(side, target_idx);
            return;
        }

        if let Some(idx) = action.move_index {
            if !self.consume_pp(side, idx) {
                return;
            }
        }

        let pokemon_name = self.active(side).pokemon.name.clone();
        let move_name = move_def.name.clone();
        let mut event = MoveEventView {
            side,
            pokemon: pokemon_name,
            move_name,
            outcome: MoveOutcome::Missed,
        };

        if self.blocked_by_status(side) {
            event.outcome = MoveOutcome::Missed;
            self.last_turn_move_events.push(event);
            return;
        }

        if move_def.protect {
            self.active_mut(side).protecting = true;
            event.outcome = MoveOutcome::Protected;
            self.last_turn_move_events.push(event);
            return;
        }

        if matches!(move_def.category, MoveCategory::Status)
            && move_def.power == 0
            && move_def.status.is_none()
            && move_def.boosts.is_none()
            && move_def.hazard.is_none()
            && move_def.set_weather.is_none()
        {
            event.outcome = MoveOutcome::StatusOnly;
            self.last_turn_move_events.push(event);
            return;
        }

        if self.active(target_side).protecting {
            event.outcome = MoveOutcome::Protected;
            self.last_turn_move_events.push(event);
            return;
        }

        if !roll_accuracy(&move_def, &mut self.rng) {
            event.outcome = MoveOutcome::Missed;
            self.last_turn_move_events.push(event);
            return;
        }

        if move_def.trick_room {
            self.trick_room = !self.trick_room;
            self.trick_room_turns = 5;
            event.outcome = MoveOutcome::StatusOnly;
            self.last_turn_move_events.push(event);
            return;
        }

        let effectiveness =
            type_effectiveness(&move_def.move_type, &self.active(target_side).pokemon.types);
        let multihit = move_def
            .multihit
            .as_ref()
            .map(|m| {
                let min_h = if m.min_hits == 0 { 1 } else { m.min_hits };
                self.rng.gen_range(min_h..=m.max_hits.max(min_h))
            })
            .unwrap_or(1);

        let mut total_damage = 0u32;
        for _ in 0..multihit {
            if self.active(target_side).is_fainted() {
                break;
            }
            let damage = self.compute_damage(side, target_side, &move_def);
            if damage == 0 {
                continue;
            }
            let applied = self.apply_damage(target_side, damage);
            total_damage = total_damage.saturating_add(applied);
            if self.active(target_side).is_fainted() {
                break;
            }
        }

        if total_damage > 0 {
            self.apply_recoil_and_drain(side, target_side, &move_def, total_damage);
            self.apply_secondary(side, target_side, &move_def);
            self.apply_stat_boosts(side, target_side, &move_def);
            event.outcome = MoveOutcome::Hit {
                effectiveness,
                damage: total_damage,
            };
        } else if matches!(move_def.category, MoveCategory::Status) && move_def.power == 0 {
            event.outcome = MoveOutcome::StatusOnly;
        } else {
            event.outcome = MoveOutcome::NoEffect { effectiveness };
        }

        if let Some(w) = move_def.set_weather.clone() {
            self.set_weather(w);
        }

        if let Some(h) = move_def.hazard.clone() {
            self.set_hazard(target_side, h);
        }

        if move_def.switch_after && !self.active(side).is_fainted() {
            self.send_next(side);
        }

        if let Some(idx) = action.move_index {
            self.active_mut(side).last_move = Some(idx);
            if is_choice_item(self.active(side)) {
                self.active_mut(side).choice_lock = Some(idx);
            }
        }

        self.last_turn_move_events.push(event);
    }

    fn resolve_move(&self, side: Side, idx: Option<usize>) -> (Move, Option<usize>) {
        if let Some(i) = idx {
            if let Some(mv) = self.active(side).pokemon.moves.get(i) {
                return (mv.clone(), Some(i));
            }
        }
        (struggle_move(), None)
    }

    fn fractional_priority(&mut self, side: Side, move_def: &Move) -> f32 {
        let mut frac: f32 = 0.0;
        // 参考: pokemon-showdown/data/items.ts Quick Claw/Custap Berry/Lagging Tail/Full Incense の fractionalPriority。
        if move_def.priority <= 0 {
            if has_item(self.active(side), "Quick Claw") {
                if self.rng.gen_ratio(1, 5) {
                    frac = frac.max(0.1);
                }
            }
            let custap = has_item(self.active(side), "Custap Berry");
            if custap && !self.active(side).berry_used {
                if self.active(side).current_hp * 4 <= self.active(side).max_hp() {
                    self.active_mut(side).berry_used = true;
                    frac = frac.max(0.1);
                }
            }
        }
        if has_item(self.active(side), "Lagging Tail")
            || has_item(self.active(side), "Full Incense")
        {
            frac = -0.1;
        }
        frac
    }

    fn consume_pp(&mut self, side: Side, move_idx: usize) -> bool {
        if let Some(pp) = self.move_pp_mut(side, move_idx) {
            if *pp > 0 {
                *pp -= 1;
                return true;
            }
        }
        false
    }

    fn blocked_by_status(&mut self, side: Side) -> bool {
        let status = self.active(side).status.clone();
        match status {
            Some(StatusCondition::Sleep) => {
                let b = self.active_mut(side);
                if b.sleep_turns > 0 {
                    b.sleep_turns -= 1;
                    return true;
                }
                // 目覚める
                let b = self.active_mut(side);
                b.status = None;
                false
            }
            Some(StatusCondition::Paralysis) => {
                let roll: f32 = self.rng.gen();
                roll < 0.25
            }
            Some(StatusCondition::Freeze) => {
                let roll: f32 = self.rng.gen();
                if roll < 0.2 {
                    let b = self.active_mut(side);
                    b.status = None;
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    fn compute_damage(&mut self, side: Side, target_side: Side, move_def: &Move) -> u32 {
        if move_def.power == 0 {
            return 0;
        }
        let attacker = self.active(side).clone();
        let defender = self.active(target_side).clone();

        if is_ground_immune(&defender, move_def) {
            return 0;
        }

        let mut atk_stat = match move_def.category {
            MoveCategory::Physical => attacker.pokemon.stats.atk as f32,
            MoveCategory::Special => attacker.pokemon.stats.spa as f32,
            MoveCategory::Status => return 0,
        };
        let mut def_stat = match move_def.category {
            MoveCategory::Physical => defender.pokemon.stats.def as f32,
            MoveCategory::Special => defender.pokemon.stats.spd as f32,
            MoveCategory::Status => return 0,
        };

        let crit = roll_crit(move_def.crit_rate, &mut self.rng);

        let atk_stage = stage_modifier(attacker.stat_stages.atk);
        let spa_stage = stage_modifier(attacker.stat_stages.spa);
        let def_stage = stage_modifier(defender.stat_stages.def);
        let spd_stage = stage_modifier(defender.stat_stages.spd);

        match move_def.category {
            MoveCategory::Physical => {
                atk_stat *= if crit { 1.0 } else { atk_stage };
                def_stat *= if crit { 1.0 } else { def_stage };
            }
            MoveCategory::Special => {
                atk_stat *= if crit { 1.0 } else { spa_stage };
                def_stat *= if crit { 1.0 } else { spd_stage };
            }
            MoveCategory::Status => {}
        }

        if matches!(move_def.category, MoveCategory::Physical)
            && matches!(attacker.status, Some(StatusCondition::Burn))
            && !has_ability(&attacker, "Guts")
        {
            atk_stat *= 0.5;
        }

        let atk_item = item_effect(&attacker);
        if let Some(stat) = atk_item.choice_stat {
            if stat == "atk" && matches!(move_def.category, MoveCategory::Physical) {
                atk_stat *= 1.5;
            }
            if stat == "spa" && matches!(move_def.category, MoveCategory::Special) {
                atk_stat *= 1.5;
            }
        }
        if atk_item.atk_mult.is_some() && matches!(move_def.category, MoveCategory::Physical) {
            atk_stat *= atk_item.atk_mult.unwrap_or(1.0);
        }
        if atk_item.spa_mult.is_some() && matches!(move_def.category, MoveCategory::Special) {
            atk_stat *= atk_item.spa_mult.unwrap_or(1.0);
        }

        let level = 50.0;
        let mut base = (((2.0 * level / 5.0 + 2.0) * move_def.power as f32 * atk_stat / def_stat)
            / 50.0)
            + 2.0;

        let stab = stab_modifier(
            &attacker.pokemon.types,
            &move_def.move_type,
            has_ability(&attacker, "Adaptability"),
        );
        let type_mod = type_effectiveness(&move_def.move_type, &defender.pokemon.types);
        if type_mod == 0.0 {
            return 0;
        }

        let weather_mod = weather_modifier(self.weather.current.as_ref(), &move_def.move_type);

        let screen_mod = screen_modifier(&move_def.category, self.side_state(target_side));

        let rand_mod = (self.rng.gen_range(85..=100) as f32) / 100.0; // 参考: pokemon-showdown/sim/damage.ts randomDamage。

        base *= stab * type_mod * weather_mod * screen_mod * rand_mod;

        if atk_item.life_orb {
            base *= 1.3;
        }
        if crit {
            base *= 1.5; // 参考: pokemon-showdown/sim/damage.ts の criticalModifier。
        }

        base.floor().max(1.0) as u32
    }

    fn apply_damage(&mut self, target_side: Side, damage: u32) -> u32 {
        let target = self.active_mut(target_side);
        let mut actual = damage as i32;
        let full = target.current_hp == target.max_hp();
        if full
            && (item_effect(target).sash_like || has_ability(target, "Sturdy"))
            && !target.sash_used
        {
            if actual >= target.current_hp {
                actual = target.current_hp - 1;
                target.sash_used = true;
            }
        }
        target.current_hp -= actual;
        actual.max(1) as u32
    }

    fn apply_recoil_and_drain(
        &mut self,
        side: Side,
        target_side: Side,
        move_def: &Move,
        dealt: u32,
    ) {
        let atk_item = item_effect(self.active(side));
        if let Some((num, den)) = move_def.recoil {
            let amount = if move_def.name.eq_ignore_ascii_case("struggle") {
                ((self.active(side).max_hp() as f32) * 0.25).ceil() as i32
            } else {
                ((dealt * num as u32) as f32 / den as f32).ceil() as i32
            };
            let self_battler = self.active_mut(side);
            self_battler.current_hp -= amount;
        }
        if atk_item.life_orb {
            let amount = ((self.active(side).max_hp() as f32) * 0.1).ceil() as i32;
            let self_battler = self.active_mut(side);
            self_battler.current_hp -= amount;
        }
        if let Some((num, den)) = move_def.drain {
            let amount = ((dealt * num as u32) as f32 / den as f32).ceil() as i32;
            let self_battler = self.active_mut(side);
            self_battler.heal(amount);
        }
        if has_item(self.active(target_side), "Rocky Helmet") && !self.active(side).is_fainted() {
            let amount = ((self.active(side).max_hp() as f32) * 0.16).ceil() as i32;
            let self_battler = self.active_mut(side);
            self_battler.current_hp -= amount;
        }
    }

    fn apply_secondary(&mut self, actor: Side, target_side: Side, move_def: &Move) {
        if let Some(sec) = move_def.secondary.as_ref() {
            let roll: f32 = self.rng.gen_range(0.0..100.0);
            if roll <= sec.chance {
                if let Some(status) = sec.status.as_ref() {
                    self.set_status(target_side, status.clone());
                }
                if let Some(boosts) = sec.boosts.as_ref() {
                    self.active_mut(target_side).apply_boosts(boosts);
                }
                if let Some(self_boosts) = sec.self_boosts.as_ref() {
                    self.active_mut(actor).apply_boosts(self_boosts);
                }
            }
        }
        if let Some(status) = move_def.status.as_ref() {
            let chance = move_def.status_chance.unwrap_or(100.0);
            let roll: f32 = self.rng.gen_range(0.0..100.0);
            if roll < chance {
                self.set_status(target_side, status.clone());
            }
        }
    }

    fn apply_stat_boosts(&mut self, side: Side, target_side: Side, move_def: &Move) {
        if let Some(b) = move_def.boosts.as_ref() {
            self.active_mut(target_side).apply_boosts(b);
        }
        if let Some(b) = move_def.self_boosts.as_ref() {
            self.active_mut(side).apply_boosts(b);
        }
    }

    fn set_status(&mut self, side: Side, status: StatusCondition) {
        let can_set = self.active(side).status.is_none();
        if !can_set {
            return;
        }
        let sleep_turns = if matches!(status, StatusCondition::Sleep) {
            Some(self.rng.gen_range(1..=3))
        } else {
            None
        };
        let pokemon_name = {
            let target = self.active_mut(side);
            let name = target.pokemon.name.clone();
            target.status = Some(status.clone());
            if let Some(turns) = sleep_turns {
                target.sleep_turns = turns;
            }
            if matches!(status, StatusCondition::Toxic) {
                target.toxic_counter = 0;
            }
            name
        };
        self.push_status_event(side, &pokemon_name, &status);
    }

    fn push_status_event(&mut self, side: Side, pokemon_name: &str, status: &StatusCondition) {
        if let Some(message) = Self::status_message(status) {
            self.last_turn_status_events.push(StatusEventView {
                side,
                pokemon: pokemon_name.to_string(),
                message,
            });
        }
    }

    fn status_message(status: &StatusCondition) -> Option<&'static str> {
        match status {
            StatusCondition::Sleep => Some("は ねむって しまった！"),
            StatusCondition::Poison => Some("は どくに かかった！"),
            StatusCondition::Toxic => Some("は もうどくに かかった！"),
            _ => None,
        }
    }

    fn set_weather(&mut self, weather: Weather) {
        self.weather.current = Some(weather);
        self.weather.turns = 5;
    }

    fn set_hazard(&mut self, side: Side, hazard: HazardMove) {
        let hazards = &mut self.side_state_mut(side).hazards;
        match hazard {
            HazardMove::Stealthrock => hazards.stealth_rock = true,
            HazardMove::Spikes => hazards.spikes = (hazards.spikes + 1).min(3),
            HazardMove::Toxicspikes => hazards.toxic_spikes = (hazards.toxic_spikes + 1).min(2),
        }
    }

    fn apply_hazards_on_switch(&mut self, side: Side) {
        let hazards = self.side_state(side.opponent()).hazards.clone();
        if has_item(self.active(side), "Heavy-Duty Boots") {
            return;
        }
        let mut status_event: Option<(StatusCondition, String)> = None;
        {
            let target = self.active_mut(side);
            if hazards.stealth_rock {
                let mod_ = type_effectiveness("rock", &target.pokemon.types);
                let dmg = ((target.max_hp() as f32) * 0.125 * mod_) as i32;
                target.current_hp -= dmg.max(1);
            }
            if hazards.spikes > 0 && is_grounded(target) {
                let frac = match hazards.spikes {
                    1 => 0.125,
                    2 => 1.0 / 6.0,
                    _ => 0.25,
                };
                let dmg = ((target.max_hp() as f32) * frac).ceil() as i32;
                target.current_hp -= dmg.max(1);
            }
            if hazards.toxic_spikes > 0 && is_grounded(target) {
                let status = if hazards.toxic_spikes >= 2 {
                    StatusCondition::Toxic
                } else {
                    StatusCondition::Poison
                };
                if target.status.is_none() {
                    target.status = Some(status.clone());
                    status_event = Some((status, target.pokemon.name.clone()));
                }
            }
        }
        if let Some((status, name)) = status_event {
            self.push_status_event(side, &name, &status);
        }
    }

    fn end_of_turn(&mut self) {
        for side in [Side::A, Side::B] {
            let mut residual = Vec::new();
            {
                let b = self.active(side);
                if matches!(b.status, Some(StatusCondition::Burn)) {
                    residual.push((side, (b.max_hp() as f32 * 0.0625) as i32));
                }
                if matches!(b.status, Some(StatusCondition::Poison)) {
                    residual.push((side, (b.max_hp() as f32 * 0.0625) as i32));
                }
                if matches!(b.status, Some(StatusCondition::Toxic)) {
                    residual.push((
                        side,
                        (b.max_hp() as f32 * 0.0625 * (b.toxic_counter.max(1) as f32)) as i32,
                    ));
                }
            }
            for (s, dmg) in residual {
                let target = self.active_mut(s);
                target.current_hp -= dmg;
                if matches!(target.status, Some(StatusCondition::Toxic)) {
                    target.toxic_counter = target.toxic_counter.saturating_add(1);
                }
            }
            if !self.active(side).is_fainted() {
                if has_item(self.active(side), "Leftovers") {
                    let heal = ((self.active(side).max_hp() as f32) * 0.0625).ceil() as i32;
                    self.active_mut(side).heal(heal);
                }
                if !self.active(side).berry_used && has_item(self.active(side), "Sitrus Berry") {
                    if self.active(side).current_hp * 2 <= self.active(side).max_hp() {
                        let heal = ((self.active(side).max_hp() as f32) * 0.25).ceil() as i32;
                        let b = self.active_mut(side);
                        b.heal(heal);
                        b.berry_used = true;
                    }
                }
            }
        }
        if let Some(weather) = self.weather.current.clone() {
            for side in [Side::A, Side::B] {
                let target = self.active_mut(side);
                if target.is_fainted() {
                    continue;
                }
                match weather {
                    Weather::Sand => {
                        if !(target.pokemon.types.iter().any(|t| {
                            matches_ignore_ascii(t, "rock")
                                || matches_ignore_ascii(t, "ground")
                                || matches_ignore_ascii(t, "steel")
                        })) {
                            let dmg = ((target.max_hp() as f32) * 0.0625).ceil() as i32;
                            target.current_hp -= dmg;
                        }
                    }
                    Weather::Hail | Weather::Snow => {
                        if !target
                            .pokemon
                            .types
                            .iter()
                            .any(|t| matches_ignore_ascii(t, "ice"))
                        {
                            let dmg = ((target.max_hp() as f32) * 0.0625).ceil() as i32;
                            target.current_hp -= dmg;
                        }
                    }
                    _ => {}
                }
            }
            if self.weather.turns > 0 {
                self.weather.turns -= 1;
                if self.weather.turns == 0 {
                    self.weather.current = None;
                }
            }
        }
        if self.trick_room {
            if self.trick_room_turns > 0 {
                self.trick_room_turns -= 1;
                if self.trick_room_turns == 0 {
                    self.trick_room = false;
                }
            }
        }
        for side in [Side::A, Side::B] {
            if self.active(side).is_fainted() && self.options.auto_switch_on_faint {
                self.send_next(side);
            }
        }
    }
}

#[derive(Clone)]
struct PlannedAction {
    side: Side,
    move_index: Option<usize>,
    switch_target: Option<usize>,
    move_def: Move,
    priority_value: f32,
    speed: u32,
    tie_break: u64,
    battler_slot: usize,
}

pub fn simulate_battle(team_a: &[Pokemon], team_b: &[Pokemon], seed: u64) -> BattleResult {
    simulate_battle_with_options(team_a, team_b, seed, &SimulationOptions::default())
}

pub fn simulate_battle_with_options(
    team_a: &[Pokemon],
    team_b: &[Pokemon],
    seed: u64,
    options: &SimulationOptions,
) -> BattleResult {
    let mut battle = Battle::new_with_options(team_a, team_b, seed, options.battle.clone());
    // 参考: pokemon-showdown/sim/battle.ts: どちらかの手持ちが尽きるまでターンを回す。
    for _turn in 0..500 {
        if let Some(result) = battle.terminal_result() {
            return result;
        }
        battle.run_turn_with_policies(options);
    }
    BattleResult::Tie
}

pub fn compute_damage_preview(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_def: &Move,
    seed: u64,
) -> u32 {
    let mut battle = Battle::new(&[attacker.clone()], &[defender.clone()], seed);
    battle.compute_damage(Side::A, Side::B, move_def)
}

fn roll_accuracy(move_def: &Move, rng: &mut SmallRng) -> bool {
    // 参考: pokemon-showdown/sim/battle.ts: tryMoveHit は randomChance(move.accuracy, 100) を用いる。
    if move_def.accuracy >= 100.0 {
        return true;
    }
    let roll = rng.gen_range(0.0..100.0);
    roll < move_def.accuracy
}

pub fn sample_accuracy_hits(move_def: &Move, seed: u64, trials: usize) -> usize {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut hits = 0usize;
    for _ in 0..trials {
        if roll_accuracy(move_def, &mut rng) {
            hits += 1;
        }
    }
    hits
}

fn hp_totals(battle: &Battle, side: Side) -> (i32, i32) {
    let (mut current, mut max_hp) = (0i32, 0i32);
    let team = battle.team(side);
    for b in team {
        current += b.current_hp.max(0);
        max_hp += b.max_hp();
    }
    (current, max_hp.max(1))
}

pub fn evaluate_state(state: &Battle, perspective: Side) -> f32 {
    if let Some(result) = state.terminal_result() {
        return match result {
            BattleResult::AWins if matches!(perspective, Side::A) => 1.0,
            BattleResult::BWins if matches!(perspective, Side::B) => 1.0,
            BattleResult::Tie => 0.5,
            _ => 0.0,
        };
    }
    let my_alive = state.alive_count(perspective) as i32;
    let opp_alive = state.alive_count(perspective.opponent()) as i32;
    let alive_diff = (my_alive - opp_alive).clamp(-3, 3) as f32 / 3.0;

    let (my_hp, my_max) = hp_totals(state, perspective);
    let (opp_hp, opp_max) = hp_totals(state, perspective.opponent());
    let my_frac = (my_hp as f32 / my_max as f32).clamp(0.0, 1.0);
    let opp_frac = (opp_hp as f32 / opp_max as f32).clamp(0.0, 1.0);
    let hp_frac_diff = (my_frac - opp_frac).clamp(-1.0, 1.0);

    let score = 0.5 + 0.4 * alive_diff + 0.1 * hp_frac_diff;
    score.clamp(0.0, 1.0)
}

fn stage_modifier(stage: i8) -> f32 {
    if stage >= 0 {
        (2.0 + stage as f32) / 2.0
    } else {
        2.0 / (2.0 + (-stage) as f32)
    }
}

fn stab_modifier(types: &[String], move_type: &str, adaptability: bool) -> f32 {
    if types.iter().any(|t| matches_ignore_ascii(t, move_type)) {
        if adaptability {
            2.0
        } else {
            1.5
        }
    } else {
        1.0
    }
}

fn matches_ignore_ascii(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn has_item(b: &Battler, name: &str) -> bool {
    b.pokemon
        .item
        .as_ref()
        .map(|i| i.eq_ignore_ascii_case(name))
        .unwrap_or(false)
}

fn has_ability(b: &Battler, name: &str) -> bool {
    b.pokemon
        .ability
        .as_ref()
        .map(|i| i.eq_ignore_ascii_case(name))
        .unwrap_or(false)
}

fn weather_modifier(weather: Option<&Weather>, move_type: &str) -> f32 {
    match weather {
        Some(Weather::Rain) => {
            if move_type.eq_ignore_ascii_case("water") {
                1.5
            } else if move_type.eq_ignore_ascii_case("fire") {
                0.5
            } else {
                1.0
            }
        }
        Some(Weather::Sun) => {
            if move_type.eq_ignore_ascii_case("fire") {
                1.5
            } else if move_type.eq_ignore_ascii_case("water") {
                0.5
            } else {
                1.0
            }
        }
        _ => 1.0,
    }
}

fn screen_modifier(category: &MoveCategory, state: &SideState) -> f32 {
    match category {
        MoveCategory::Physical if state.screens.reflect > 0 => 0.5,
        MoveCategory::Special if state.screens.light_screen > 0 => 0.5,
        _ => 1.0,
    }
}

fn roll_crit(crit_rate: u8, rng: &mut SmallRng) -> bool {
    // 参考: pokemon-showdown/sim/damage.ts crit level: 1/24, 1/8, 1/2, 1。
    let level = crit_rate.min(3);
    let chance = match level {
        0 => 1.0 / 24.0,
        1 => 1.0 / 8.0,
        2 => 0.5,
        _ => 1.0,
    };
    rng.gen::<f32>() < chance
}

fn is_ground_immune(b: &Battler, move_def: &Move) -> bool {
    if !move_def.move_type.eq_ignore_ascii_case("ground") {
        return false;
    }
    if has_ability(b, "Levitate") {
        return true;
    }
    b.pokemon
        .types
        .iter()
        .any(|t| t.eq_ignore_ascii_case("flying"))
}

fn is_grounded(b: &Battler) -> bool {
    !(has_ability(b, "Levitate")
        || b.pokemon
            .types
            .iter()
            .any(|t| t.eq_ignore_ascii_case("flying")))
}

fn is_choice_item(b: &Battler) -> bool {
    ["Choice Band", "Choice Specs", "Choice Scarf"]
        .iter()
        .any(|name| has_item(b, name))
}

fn normalize_item_id(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn item_effect(b: &Battler) -> ItemEffect {
    if let Some(item) = b.pokemon.item.as_ref() {
        let id = normalize_item_id(item);
        if let Some(eff) = ITEM_TABLE.get(id.as_str()) {
            return *eff;
        }
    }
    ItemEffect::default()
}

fn struggle_move() -> Move {
    Move {
        name: "Struggle".to_string(),
        move_type: "typeless".to_string(),
        category: MoveCategory::Physical,
        power: 50,
        accuracy: 100.0,
        priority: 0,
        pp: 1,
        crit_rate: 0,
        secondary: None,
        recoil: Some((1, 4)), // 本来は最大HPの 1/4。簡略化でダメージ割合に近似。
        drain: None,
        boosts: None,
        self_boosts: None,
        status: None,
        status_chance: None,
        set_weather: None,
        hazard: None,
        protect: false,
        switch_after: false,
        multihit: None,
        trick_room: false,
        extras: Default::default(),
    }
}

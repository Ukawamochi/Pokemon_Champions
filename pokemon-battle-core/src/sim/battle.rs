use crate::data::moves::{get_move, MoveCategory};
use crate::data::types::{effectiveness_dual, Type};
use crate::battle_logger::{showdown_ident, BattleLogger};
use crate::i18n::{translate_item, translate_move, translate_pokemon};
use crate::sim::ai::BattleAI;
use crate::sim::abilities::misc_abilities::{
    apply_contact_damage_abilities, apply_effect_spore, poison_heal_amount, speed_multiplier,
    try_absorb_water_move,
};
use crate::sim::abilities::status_abilities::{apply_download, apply_intimidate, apply_trace, DownloadBoost};
use crate::sim::damage::{
    ability_attack_modifier, ability_defense_modifier, calculate_damage, calculate_damage_with_modifiers,
    chain_modifier, item_type_boost, DamageModifiers, is_stab,
};
use crate::sim::faint_handler::{apply_aftermath_if_applicable, prevent_ko_if_applicable, KoPrevention};
use crate::sim::items::battle_items;
use crate::sim::moves::attacking::{
    apply_drain, apply_recoil_damage, calculate_multihit_count, calculate_variable_power, get_move_priority,
    handle_charging_move, handle_ohko_move,
};
use crate::sim::moves::flags::{bypasses_protect, bypasses_substitute, check_ability_immunity, is_contact_move};
use crate::sim::moves::secondary::{
    apply_secondary_effect_with_update, secondary_effects_from_move, self_effect_from_move,
};
use crate::sim::moves::status::handle_status_move;
use crate::sim::pokemon::{Pokemon, Status};
use crate::sim::switching::{self, SwitchKind};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Weather {
    Sun,
    Rain,
    Sand,
    Hail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldEffect {
    Reflect,
    LightScreen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Grassy,
    Electric,
    Psychic,
    Misty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Move(usize),
    Switch(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattleResult {
    TeamAWins,
    TeamBWins,
    Draw,
}

#[derive(Clone, Debug)]
pub struct BattleState {
    pub pokemon_a: Pokemon,
    pub pokemon_b: Pokemon,
    pub bench_a: Vec<Pokemon>,
    pub bench_b: Vec<Pokemon>,
    pub logger: Option<BattleLogger>,
    pub turn: u32,
    pub weather: Option<Weather>,
    pub weather_turns: u8,
    pub field_effects: Vec<FieldEffect>,
    pub field: Option<Field>,
    pub field_turns: u8,
    pub trick_room_turns: u8,
    pub side_a: SideConditions,
    pub side_b: SideConditions,
}

impl BattleState {
    pub fn new(pokemon_a: Pokemon, pokemon_b: Pokemon) -> Self {
        Self {
            pokemon_a,
            pokemon_b,
            bench_a: Vec::new(),
            bench_b: Vec::new(),
            logger: None,
            turn: 0,
            weather: None,
            weather_turns: 0,
            field_effects: Vec::new(),
            field: None,
            field_turns: 0,
            trick_room_turns: 0,
            side_a: SideConditions::default(),
            side_b: SideConditions::default(),
        }
    }

    pub fn new_with_bench(
        pokemon_a: Pokemon,
        pokemon_b: Pokemon,
        bench_a: Vec<Pokemon>,
        bench_b: Vec<Pokemon>,
    ) -> Self {
        Self {
            pokemon_a,
            pokemon_b,
            bench_a,
            bench_b,
            logger: None,
            turn: 0,
            weather: None,
            weather_turns: 0,
            field_effects: Vec::new(),
            field: None,
            field_turns: 0,
            trick_room_turns: 0,
            side_a: SideConditions::default(),
            side_b: SideConditions::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SideConditions {
    pub stealth_rock: bool,
    pub spikes: u8,
    pub toxic_spikes: u8,
    pub sticky_web: bool,
    pub reflect_turns: u8,
    pub light_screen_turns: u8,
    pub mist_turns: u8,
    pub safeguard_turns: u8,
    pub tailwind_turns: u8,
    pub lucky_chant_turns: u8,
    pub aurora_veil_turns: u8,
    pub wish_turns: u8,
    pub wish_heal: u16,
    pub healing_wish_pending: bool,
}

#[derive(Default)]
pub(crate) struct EnvUpdate {
    pub(crate) weather: Option<Weather>,
    pub(crate) field: Option<Field>,
    pub(crate) trick_room_turns: Option<u8>,
    pub(crate) hazard: Option<HazardUpdate>,
    pub(crate) screen: Option<ScreenUpdate>,
    pub(crate) side_condition: Option<SideConditionUpdate>,
    pub(crate) wish: Option<WishUpdate>,
    pub(crate) healing_wish: Option<usize>,
    pub(crate) court_change: bool,
    pub(crate) force_switch: Option<usize>,
    pub(crate) clear_hazards: Option<HazardClear>,
    pub(crate) clear_screens: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum HazardKind {
    StealthRock,
    Spikes,
    ToxicSpikes,
    StickyWeb,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HazardUpdate {
    pub(crate) target: usize,
    pub(crate) kind: HazardKind,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScreenUpdate {
    pub(crate) target: usize,
    pub(crate) kind: FieldEffect,
    pub(crate) turns: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SideConditionKind {
    Mist,
    Safeguard,
    Tailwind,
    LuckyChant,
    AuroraVeil,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SideConditionUpdate {
    pub(crate) target: usize,
    pub(crate) kind: SideConditionKind,
    pub(crate) turns: u8,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum HazardClear {
    Side(usize),
    Both,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WishUpdate {
    pub(crate) target: usize,
    pub(crate) heal: u16,
}

fn valid_actions(pokemon: &Pokemon, bench: &[Pokemon]) -> Vec<Action> {
    let mut actions: Vec<Action> = pokemon
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

pub fn run_battle(
    pokemon_a: Pokemon,
    pokemon_b: Pokemon,
    ai_a: &mut dyn BattleAI,
    ai_b: &mut dyn BattleAI,
) -> BattleResult {
    let mut state = BattleState::new(pokemon_a, pokemon_b);
    run_battle_with_state(&mut state, ai_a, ai_b)
}

pub fn run_team_battle(
    mut team_a: Vec<Pokemon>,
    mut team_b: Vec<Pokemon>,
    ai_a: &mut dyn BattleAI,
    ai_b: &mut dyn BattleAI,
) -> BattleResult {
    if team_a.is_empty() || team_b.is_empty() {
        return BattleResult::Draw;
    }
    let pokemon_a = team_a.remove(0);
    let pokemon_b = team_b.remove(0);
    let mut state = BattleState::new_with_bench(pokemon_a, pokemon_b, team_a, team_b);
    run_battle_with_state(&mut state, ai_a, ai_b)
}

fn run_battle_with_state(
    state: &mut BattleState,
    ai_a: &mut dyn BattleAI,
    ai_b: &mut dyn BattleAI,
) -> BattleResult {
    apply_on_entry_abilities(state);
    let mut rng = SmallRng::seed_from_u64(0xBADC0DE);
    for _ in 0..500 {
        if !side_has_available(&state.pokemon_a, &state.bench_a)
            && !side_has_available(&state.pokemon_b, &state.bench_b)
        {
            return BattleResult::Draw;
        }
        if !side_has_available(&state.pokemon_a, &state.bench_a) {
            return BattleResult::TeamBWins;
        }
        if !side_has_available(&state.pokemon_b, &state.bench_b) {
            return BattleResult::TeamAWins;
        }
        if state.pokemon_a.is_fainted() {
            if let Some(idx) = switching::pick_random_switch(&state.bench_a, &mut rng) {
                perform_switch(state, 0, idx, SwitchKind::Forced, &mut rng);
            }
        }
        if state.pokemon_b.is_fainted() {
            if let Some(idx) = switching::pick_random_switch(&state.bench_b, &mut rng) {
                perform_switch(state, 1, idx, SwitchKind::Forced, &mut rng);
            }
        }
        apply_start_of_turn_effects(state, &mut rng);
        if state.pokemon_a.is_fainted() {
            if let Some(idx) = switching::pick_random_switch(&state.bench_a, &mut rng) {
                perform_switch(state, 0, idx, SwitchKind::Forced, &mut rng);
            }
        }
        if state.pokemon_b.is_fainted() {
            if let Some(idx) = switching::pick_random_switch(&state.bench_b, &mut rng) {
                perform_switch(state, 1, idx, SwitchKind::Forced, &mut rng);
            }
        }
        if !side_has_available(&state.pokemon_a, &state.bench_a)
            && !side_has_available(&state.pokemon_b, &state.bench_b)
        {
            return BattleResult::Draw;
        }
        if !side_has_available(&state.pokemon_a, &state.bench_a) {
            return BattleResult::TeamBWins;
        }
        if !side_has_available(&state.pokemon_b, &state.bench_b) {
            return BattleResult::TeamAWins;
        }
        state.pokemon_a.protect_active = false;
        state.pokemon_b.protect_active = false;
        state.pokemon_a.endure_active = false;
        state.pokemon_b.endure_active = false;
        state.pokemon_a.magic_coat_active = false;
        state.pokemon_b.magic_coat_active = false;
        state.pokemon_a.kings_shield_active = false;
        state.pokemon_b.kings_shield_active = false;
        state.pokemon_a.roosted = false;
        state.pokemon_b.roosted = false;
        state.pokemon_a.semi_invulnerable = false;
        state.pokemon_b.semi_invulnerable = false;
        if let Some(logger) = state.logger.as_mut() {
            logger.log_turn((state.turn + 1) as usize);
        }
        println!("Turn {}:", state.turn + 1);
        let actions_a = valid_actions(&state.pokemon_a, &state.bench_a);
        let actions_b = valid_actions(&state.pokemon_b, &state.bench_b);
        if actions_a.is_empty() && actions_b.is_empty() {
            return BattleResult::Draw;
        }
        let action_a = ai_a.choose_action(state, &actions_a);
        let action_b = ai_b.choose_action(state, &actions_b);
        execute_turn(state, action_a, action_b, &mut rng);
        apply_end_of_turn_effects(state, &mut rng);
        handle_simultaneous_faints(state, &mut rng);
        state.turn += 1;
    }
    BattleResult::Draw
}

pub fn execute_turn(
    state: &mut BattleState,
    action_a: Action,
    action_b: Action,
    rng: &mut SmallRng,
) {
    let (a_first, b_first) =
        determine_order(
            &state.pokemon_a,
            action_a,
            &state.pokemon_b,
            action_b,
            state.trick_room_turns > 0,
            state.weather,
            state.field,
            rng,
        );
    if a_first {
        resolve_action(state, 0, action_a, action_b, 1, rng);
        if !state.pokemon_b.is_fainted() {
            resolve_action(state, 1, action_b, action_a, 0, rng);
        }
    } else if b_first {
        resolve_action(state, 1, action_b, action_a, 0, rng);
        if !state.pokemon_a.is_fainted() {
            resolve_action(state, 0, action_a, action_b, 1, rng);
        }
    }
    handle_simultaneous_faints(state, rng);
}

fn handle_simultaneous_faints(state: &mut BattleState, rng: &mut SmallRng) {
    let a_fainted = state.pokemon_a.is_fainted();
    let b_fainted = state.pokemon_b.is_fainted();
    if !a_fainted && !b_fainted {
        return;
    }
    let order_a_first = if a_fainted && b_fainted {
        let spe_a = effective_speed(&state.pokemon_a, state.weather);
        let spe_b = effective_speed(&state.pokemon_b, state.weather);
        if spe_a == spe_b {
            rng.gen_bool(0.5)
        } else {
            spe_a > spe_b
        }
    } else {
        a_fainted
    };

    for side_idx in if order_a_first { [0usize, 1usize] } else { [1usize, 0usize] } {
        let (active, bench) = if side_idx == 0 {
            (&state.pokemon_a, &state.bench_a)
        } else {
            (&state.pokemon_b, &state.bench_b)
        };
        if !active.is_fainted() {
            continue;
        }
        if let Some(idx) = switching::pick_random_switch(bench, rng) {
            perform_switch(state, side_idx, idx, SwitchKind::Forced, rng);
        }
    }
}

pub fn determine_order(
    pokemon_a: &Pokemon,
    action_a: Action,
    pokemon_b: &Pokemon,
    action_b: Action,
    trick_room_active: bool,
    weather: Option<Weather>,
    field: Option<Field>,
    rng: &mut SmallRng,
) -> (bool, bool) {
    let priority_a = action_priority(action_a, pokemon_a, field);
    let priority_b = action_priority(action_b, pokemon_b, field);
    if priority_a != priority_b {
        let a_first = priority_a > priority_b;
        return (a_first, !a_first);
    }
    let spe_a = effective_speed(pokemon_a, weather);
    let spe_b = effective_speed(pokemon_b, weather);
    if spe_a != spe_b {
        let a_first = if trick_room_active {
            spe_a < spe_b
        } else {
            spe_a > spe_b
        };
        return (a_first, !a_first);
    }
    let coin = rng.gen_bool(0.5);
    (coin, !coin)
}

fn action_priority(action: Action, pokemon: &Pokemon, field: Option<Field>) -> i8 {
    match action {
        Action::Move(idx) => pokemon
            .moves
            .get(idx)
            .and_then(|name| get_move(name.as_str()))
            .map(|mv| get_move_priority(mv, pokemon, field))
            .unwrap_or(0),
        Action::Switch(_) => 6,
    }
}

fn is_attack_action(action: Action, pokemon: &Pokemon) -> bool {
    if let Some(charging) = pokemon.charging_move.as_deref() {
        if let Some(data) = get_move(charging) {
            return !matches!(data.category, MoveCategory::Status);
        }
    }
    if pokemon.encore_turns > 0 {
        if let Some(encore) = pokemon.encore_move.as_deref() {
            if let Some(data) = get_move(encore) {
                return !matches!(data.category, MoveCategory::Status);
            }
        }
    }
    match action {
        Action::Move(idx) => pokemon
            .moves
            .get(idx)
            .and_then(|name| get_move(name.as_str()))
            .map(|mv| !matches!(mv.category, MoveCategory::Status))
            .unwrap_or(false),
        Action::Switch(_) => false,
    }
}

pub(crate) const STAGE_ATK: usize = 0;
pub(crate) const STAGE_DEF: usize = 1;
pub(crate) const STAGE_SPA: usize = 2;
pub(crate) const STAGE_SPD: usize = 3;
pub(crate) const STAGE_SPE: usize = 4;

fn effective_speed(pokemon: &Pokemon, weather: Option<Weather>) -> u16 {
    let mut spe = apply_stage_multiplier(pokemon.stats.spe, pokemon.stat_stages[STAGE_SPE]);
    if matches!(pokemon.status, Some(Status::Paralysis)) && !pokemon.has_ability("Quick Feet") {
        spe = ((spe as f32) * 0.5).floor() as u16;
    }
    let speed_mod = speed_multiplier(
        pokemon,
        matches!(weather, Some(Weather::Rain)),
        matches!(weather, Some(Weather::Sun)),
    );
    let weather_ability_mod = crate::sim::weather_field::weather_speed_multiplier(pokemon, weather);
    let item_mod = battle_items::speed_modifier(pokemon);
    ((spe as f32) * speed_mod * weather_ability_mod * item_mod).floor() as u16
}

fn stage_multiplier(stage: i8) -> f32 {
    if stage >= 0 {
        (2 + stage as i32) as f32 / 2.0
    } else {
        2.0 / (2 - stage as i32) as f32
    }
}

fn accuracy_multiplier(stage: i8) -> f32 {
    if stage >= 0 {
        (3 + stage as i32) as f32 / 3.0
    } else {
        3.0 / (3 - stage as i32) as f32
    }
}

fn apply_stage_multiplier(base: u16, stage: i8) -> u16 {
    let value = (base as f32) * stage_multiplier(stage);
    value.floor().max(1.0) as u16
}

pub(crate) fn apply_stage_change(pokemon: &mut Pokemon, name: &str, stat: usize, delta: i8) -> bool {
    let current = pokemon.stat_stages[stat];
    let mut next = current.saturating_add(delta);
    next = next.clamp(-6, 6);
    if next == current {
        return false;
    }
    pokemon.stat_stages[stat] = next;
    let stat_name = match stat {
        STAGE_ATK => "こうげき",
        STAGE_DEF => "ぼうぎょ",
        STAGE_SPA => "とくこう",
        STAGE_SPD => "とくぼう",
        STAGE_SPE => "すばやさ",
        _ => "のうりょく",
    };
    let direction = if delta > 0 { "あがった" } else { "さがった" };
    println!("  {}の{}が{}！", name, stat_name, direction);
    true
}

pub(crate) fn apply_accuracy_change(pokemon: &mut Pokemon, name: &str, delta: i8) -> bool {
    let current = pokemon.accuracy_stage;
    let next = current.saturating_add(delta).clamp(-6, 6);
    if next == current {
        return false;
    }
    pokemon.accuracy_stage = next;
    let direction = if delta > 0 { "あがった" } else { "さがった" };
    println!("  {}のめいちゅうが{}！", name, direction);
    true
}

pub(crate) fn apply_evasion_change(pokemon: &mut Pokemon, name: &str, delta: i8) -> bool {
    let current = pokemon.evasion_stage;
    let next = current.saturating_add(delta).clamp(-6, 6);
    if next == current {
        return false;
    }
    pokemon.evasion_stage = next;
    let direction = if delta > 0 { "あがった" } else { "さがった" };
    println!("  {}のかいひが{}！", name, direction);
    true
}

pub(crate) fn reset_stat_stages(pokemon: &mut Pokemon, name: &str) {
    pokemon.stat_stages = [0; 6];
    pokemon.accuracy_stage = 0;
    pokemon.evasion_stage = 0;
    println!("  {}ののうりょくへんかが もとにもどった！", name);
}

pub(crate) fn heal_hp(pokemon: &mut Pokemon, name: &str, ratio: f32) {
    let max_hp = pokemon.stats.hp;
    if pokemon.current_hp >= max_hp {
        println!("  しかし こうかがなかった！");
        return;
    }
    let amount = ((max_hp as f32) * ratio).floor() as u16;
    pokemon.current_hp = (pokemon.current_hp + amount).min(max_hp);
    println!(
        "  {}はHPをかいふくした！ (HP: {}/{})",
        name, pokemon.current_hp, max_hp
    );
}

fn effective_types(pokemon: &Pokemon) -> [Type; 2] {
    if !pokemon.roosted {
        return pokemon.types;
    }
    let mut t0 = pokemon.types[0];
    let mut t1 = pokemon.types[1];
    if t0 == Type::Flying {
        t0 = t1;
        t1 = Type::Normal;
    } else if t1 == Type::Flying {
        t1 = Type::Normal;
    }
    [t0, t1]
}

fn item_id(pokemon: &Pokemon) -> Option<String> {
    crate::sim::items::consumable::item_id(pokemon)
}

pub(crate) fn has_item(pokemon: &Pokemon, item: &str) -> bool {
    crate::sim::items::consumable::has_item(pokemon, item)
}

fn has_consumable_item(pokemon: &Pokemon, item: &str) -> bool {
    !pokemon.item_consumed && has_item(pokemon, item)
}

fn consume_item(pokemon: &mut Pokemon) {
    pokemon.item_consumed = true;
}

pub(crate) fn apply_on_entry_abilities(state: &mut BattleState) {
    apply_on_entry_ability_for_side(state, 0, true);
    apply_on_entry_ability_for_side(state, 1, true);
}

fn apply_on_entry_ability_for_side(state: &mut BattleState, side_idx: usize, allow_trace: bool) {
    let ability = if side_idx == 0 {
        state.pokemon_a.ability.clone()
    } else {
        state.pokemon_b.ability.clone()
    };
    apply_on_entry_ability_effects(state, side_idx, ability.as_str(), allow_trace);
}

fn apply_on_entry_ability_effects(
    state: &mut BattleState,
    side_idx: usize,
    ability: &str,
    allow_trace: bool,
) {
    apply_field_ability(state, ability);
    apply_weather_ability(state, ability);

    let (user, foe) = if side_idx == 0 {
        (&mut state.pokemon_a, &mut state.pokemon_b)
    } else {
        (&mut state.pokemon_b, &mut state.pokemon_a)
    };
    let user_name = translate_pokemon(&user.species);
    let foe_name = translate_pokemon(&foe.species);

    if ability.eq_ignore_ascii_case("Intimidate") && !foe.is_fainted() {
        if apply_intimidate(foe) {
            println!("  {}のこうげきがさがった！", foe_name);
        } else {
            println!("  しかし こうかがなかった！");
        }
    }

    if ability.eq_ignore_ascii_case("Download") && !user.is_fainted() {
        match apply_download(user, foe) {
            Some(DownloadBoost::Attack) => println!("  {}のこうげきがあがった！", user_name),
            Some(DownloadBoost::SpAttack) => println!("  {}のとくこうがあがった！", user_name),
            None => println!("  しかし こうかがなかった！"),
        }
    }

    if allow_trace && ability.eq_ignore_ascii_case("Trace") && !user.is_fainted() {
        if let Some(traced) = apply_trace(user, foe) {
            println!("  {}は{}をトレースした！", user_name, traced);
            apply_on_entry_ability_effects(state, side_idx, traced.as_str(), false);
        } else {
            println!("  しかし こうかがなかった！");
        }
    }
}

fn apply_field_ability(state: &mut BattleState, ability: &str) {
    if ability.eq_ignore_ascii_case("Grassy Surge") {
        state.field = Some(Field::Grassy);
        state.field_turns = 5;
        println!("  グラスフィールドが展開された！");
    }
    if ability.eq_ignore_ascii_case("Electric Surge") {
        state.field = Some(Field::Electric);
        state.field_turns = 5;
        println!("  エレキフィールドが展開された！");
    }
    if ability.eq_ignore_ascii_case("Psychic Surge") {
        state.field = Some(Field::Psychic);
        state.field_turns = 5;
        println!("  サイコフィールドが展開された！");
    }
    if ability.eq_ignore_ascii_case("Misty Surge") {
        state.field = Some(Field::Misty);
        state.field_turns = 5;
        println!("  ミストフィールドが展開された！");
    }
}

fn apply_weather_ability(state: &mut BattleState, ability: &str) {
    if ability.eq_ignore_ascii_case("Drought") {
        state.weather = Some(Weather::Sun);
        state.weather_turns = 5;
        println!("  ひざしがつよくなった！");
    }
    if ability.eq_ignore_ascii_case("Drizzle") {
        state.weather = Some(Weather::Rain);
        state.weather_turns = 5;
        println!("  あめがふりはじめた！");
    }
    if ability.eq_ignore_ascii_case("Sand Stream") {
        state.weather = Some(Weather::Sand);
        state.weather_turns = 5;
        println!("  すなあらしがふきはじめた！");
    }
    if ability.eq_ignore_ascii_case("Snow Warning") {
        state.weather = Some(Weather::Hail);
        state.weather_turns = 5;
        println!("  あられがふりはじめた！");
    }
}

fn side_conditions_mut(state: &mut BattleState, side_idx: usize) -> &mut SideConditions {
    if side_idx == 0 {
        &mut state.side_a
    } else {
        &mut state.side_b
    }
}

fn bench_mut(state: &mut BattleState, side_idx: usize) -> &mut Vec<Pokemon> {
    if side_idx == 0 {
        &mut state.bench_a
    } else {
        &mut state.bench_b
    }
}

fn reset_on_switch(pokemon: &mut Pokemon) {
    pokemon.stat_stages = [0; 6];
    pokemon.accuracy_stage = 0;
    pokemon.evasion_stage = 0;
    pokemon.protect_active = false;
    pokemon.kings_shield_active = false;
    pokemon.endure_active = false;
    pokemon.roosted = false;
    pokemon.semi_invulnerable = false;
    pokemon.flinched = false;
    pokemon.confusion_turns = 0;
    switching::clear_trap(pokemon);
    pokemon.protect_counter = 0;
    pokemon.substitute_hp = 0;
    pokemon.destiny_bond = false;
    pokemon.charging_move = None;
    pokemon.taunt_turns = 0;
    pokemon.encore_turns = 0;
    pokemon.encore_move = None;
    battle_items::clear_choice_lock(pokemon);
    if matches!(pokemon.status, Some(Status::Poison)) && pokemon.toxic_counter > 0 {
        // PS: tox stage resets on switch
        pokemon.toxic_counter = 1;
    }
    if pokemon.stance_blade {
        swap_stance_stats(pokemon);
        pokemon.stance_blade = false;
    }
}

fn apply_entry_hazards(
    pokemon: &mut Pokemon,
    side: &mut SideConditions,
    field: Option<Field>,
    rng: &mut SmallRng,
) {
    let name = translate_pokemon(&pokemon.species);
    if side.stealth_rock {
        let types = effective_types(pokemon);
        let effectiveness = effectiveness_dual(Type::Rock, types[0], types[1]);
        if effectiveness > 0.0 {
            let ratio = effectiveness / 8.0;
            let dmg = ((pokemon.stats.hp as f32) * ratio).floor().max(1.0) as u16;
            pokemon.take_damage(dmg);
            println!(
                "  {}はステルスロックのダメージをうけた！ (HP: {}/{})",
                name, pokemon.current_hp, pokemon.stats.hp
            );
        }
    }
    if side.spikes > 0 && is_grounded(pokemon) {
        let ratio = match side.spikes {
            1 => 1.0 / 8.0,
            2 => 1.0 / 6.0,
            _ => 1.0 / 4.0,
        };
        let dmg = ((pokemon.stats.hp as f32) * ratio).floor().max(1.0) as u16;
        pokemon.take_damage(dmg);
        println!(
            "  {}はまきびしのダメージをうけた！ (HP: {}/{})",
            name, pokemon.current_hp, pokemon.stats.hp
        );
    }
    if side.toxic_spikes > 0 && is_grounded(pokemon) {
        let poison_type = pokemon.types[0] == Type::Poison || pokemon.types[1] == Type::Poison;
        if poison_type {
            side.toxic_spikes = 0;
            println!("  どくびしがきれいに かたづけられた！");
        } else {
            let toxic = side.toxic_spikes >= 2;
            let status = if toxic { Status::Poison } else { Status::Poison };
            if apply_status_with_field(pokemon, status, toxic, field, rng) {
                println!("  {}は{}！", name, format_status(status));
            }
        }
    }
    if side.sticky_web && is_grounded(pokemon) {
        if !apply_stage_change(pokemon, &name, STAGE_SPE, -1) {
            println!("  しかし こうかがなかった！");
        }
    }
}

fn perform_switch(
    state: &mut BattleState,
    side_idx: usize,
    bench_idx: usize,
    kind: SwitchKind,
    rng: &mut SmallRng,
) -> bool {
    let do_log = state.logger.is_some();
    let mut pending_switch_log: Option<(String, String, u16, u16)> = None;
    {
        let (active, bench, side) = if side_idx == 0 {
            (&mut state.pokemon_a, &mut state.bench_a, &mut state.side_a)
        } else {
            (&mut state.pokemon_b, &mut state.bench_b, &mut state.side_b)
        };
        if !switching::can_switch(active, kind) {
            println!("  しかし こうかがなかった！");
            return false;
        }
        if bench_idx >= bench.len() || bench[bench_idx].is_fainted() {
            return false;
        }
        let outgoing_name = translate_pokemon(&active.species);
        reset_on_switch(active);
        std::mem::swap(active, &mut bench[bench_idx]);
        let incoming_name = translate_pokemon(&active.species);
        println!("  {}は {}に交代した！", outgoing_name, incoming_name);
        if do_log {
            pending_switch_log = Some((
                showdown_ident(side_idx, &active.species),
                active.species.clone(),
                active.current_hp,
                active.stats.hp,
            ));
        }
        apply_entry_hazards(active, side, state.field, rng);
        if side.healing_wish_pending && !active.is_fainted() {
            side.healing_wish_pending = false;
            active.current_hp = active.stats.hp;
            active.clear_status();
            println!("  {}は いやしのねがいで かいふくした！", incoming_name);
        }
    }
    if do_log {
        if let (Some(logger), Some((ident, species, hp, max_hp))) =
            (state.logger.as_mut(), pending_switch_log)
        {
            logger.log_switch(&ident, &species, hp, max_hp);
        }
    }
    apply_on_entry_ability_for_side(state, side_idx, true);
    true
}

fn side_has_available(active: &Pokemon, bench: &[Pokemon]) -> bool {
    if !active.is_fainted() {
        return true;
    }
    bench.iter().any(|pokemon| !pokemon.is_fainted())
}

fn apply_env_update(state: &mut BattleState, update: EnvUpdate, rng: &mut SmallRng) {
    if update.court_change {
        std::mem::swap(&mut state.side_a, &mut state.side_b);
        println!("  コートチェンジ！");
    }
    if let Some(weather) = update.weather {
        state.weather = Some(weather);
        state.weather_turns = 5;
    }
    if let Some(field) = update.field {
        state.field = Some(field);
        state.field_turns = 5;
    }
    if let Some(turns) = update.trick_room_turns {
        state.trick_room_turns = turns;
    }
    if let Some(hazard) = update.hazard {
        let side = side_conditions_mut(state, hazard.target);
        match hazard.kind {
            HazardKind::StealthRock => {
                if side.stealth_rock {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.stealth_rock = true;
                    println!("  ステルスロックが しかけられた！");
                }
            }
            HazardKind::Spikes => {
                if side.spikes >= 3 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.spikes = (side.spikes + 1).min(3);
                    println!("  まきびしが しかけられた！");
                }
            }
            HazardKind::ToxicSpikes => {
                if side.toxic_spikes >= 2 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.toxic_spikes = (side.toxic_spikes + 1).min(2);
                    println!("  どくびしが しかけられた！");
                }
            }
            HazardKind::StickyWeb => {
                if side.sticky_web {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.sticky_web = true;
                    println!("  ねばねばネットが しかけられた！");
                }
            }
        }
    }
    if let Some(wish) = update.wish {
        let side = side_conditions_mut(state, wish.target);
        side.wish_turns = 2;
        side.wish_heal = wish.heal.max(1);
        println!("  ねがいごとが となえられた！");
    }
    if let Some(target) = update.healing_wish {
        let side = side_conditions_mut(state, target);
        side.healing_wish_pending = true;
        println!("  いやしのねがいが こめられた！");
    }
    if let Some(screen) = update.screen {
        let side = side_conditions_mut(state, screen.target);
        match screen.kind {
            FieldEffect::Reflect => {
                if side.reflect_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.reflect_turns = screen.turns;
                    println!("  リフレクターが はられた！");
                }
            }
            FieldEffect::LightScreen => {
                if side.light_screen_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.light_screen_turns = screen.turns;
                    println!("  ひかりのかべが はられた！");
                }
            }
        }
    }
    if let Some(side_update) = update.side_condition {
        let side = side_conditions_mut(state, side_update.target);
        match side_update.kind {
            SideConditionKind::Mist => {
                if side.mist_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.mist_turns = side_update.turns.max(1);
                    println!("  しろいきりが かかった！");
                }
            }
            SideConditionKind::Safeguard => {
                if side.safeguard_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.safeguard_turns = side_update.turns.max(1);
                    println!("  しんぴのベールに つつまれた！");
                }
            }
            SideConditionKind::Tailwind => {
                if side.tailwind_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.tailwind_turns = side_update.turns.max(1);
                    println!("  おいかぜが ふきはじめた！");
                }
            }
            SideConditionKind::LuckyChant => {
                if side.lucky_chant_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.lucky_chant_turns = side_update.turns.max(1);
                    println!("  おまじないを となえた！");
                }
            }
            SideConditionKind::AuroraVeil => {
                if side.aurora_veil_turns > 0 {
                    println!("  しかし こうかがなかった！");
                } else {
                    side.aurora_veil_turns = side_update.turns.max(1);
                    println!("  オーロラベールが はられた！");
                }
            }
        }
    }
    if let Some(clear) = update.clear_hazards {
        match clear {
            HazardClear::Side(idx) => {
                let side = side_conditions_mut(state, idx);
                side.stealth_rock = false;
                side.spikes = 0;
                side.toxic_spikes = 0;
                side.sticky_web = false;
                println!("  しかけられていた わざが とりのぞかれた！");
            }
            HazardClear::Both => {
                clear_hazards(&mut state.side_a);
                clear_hazards(&mut state.side_b);
                println!("  しかけられていた わざが ぜんぶ とりのぞかれた！");
            }
        }
    }
    if update.clear_screens {
        state.side_a.reflect_turns = 0;
        state.side_a.light_screen_turns = 0;
        state.side_a.aurora_veil_turns = 0;
        state.side_b.reflect_turns = 0;
        state.side_b.light_screen_turns = 0;
        state.side_b.aurora_veil_turns = 0;
        state.field_effects.clear();
        println!("  バリアが かきけされた！");
    }
    if let Some(target_side) = update.force_switch {
        let bench = bench_mut(state, target_side);
        if let Some(idx) = switching::pick_random_switch(bench, rng) {
            perform_switch(state, target_side, idx, SwitchKind::Forced, rng);
        } else {
            println!("  しかし こうかがなかった！");
        }
    }
}

fn clear_hazards(side: &mut SideConditions) {
    side.stealth_rock = false;
    side.spikes = 0;
    side.toxic_spikes = 0;
    side.sticky_web = false;
}

fn is_grounded(pokemon: &Pokemon) -> bool {
    if pokemon.roosted {
        return true;
    }
    if pokemon.telekinesis_turns > 0 {
        return false;
    }
    !(pokemon.types[0] == Type::Flying || pokemon.types[1] == Type::Flying)
}

fn targets_opponent_pokemon(target: &str) -> bool {
    matches!(
        target,
        "normal" | "adjacentFoe" | "allAdjacentFoes" | "randomNormal" | "any"
    )
}

fn apply_contact_abilities(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    move_data: &crate::data::moves::MoveData,
    field: Option<Field>,
    rng: &mut SmallRng,
) {
    if !is_contact_move(move_data) {
        return;
    }
    let attacker_ja = translate_pokemon(&attacker.species);
    if defender.has_ability("Poison Point") {
        if rng.gen_bool(0.3) {
            if apply_status_with_field(attacker, Status::Poison, false, field, rng) {
                println!("  {}は{}！", attacker_ja, format_status(Status::Poison));
            }
        }
    }
    apply_contact_damage_abilities(attacker, defender);
    apply_effect_spore(attacker, defender, field, rng);
    if has_item(defender, "rockyhelmet") {
        let dmg = (attacker.stats.hp as u32 / 6).max(1) as u16;
        attacker.take_damage(dmg);
        println!(
            "  {}は{}のダメージをうけた！ (HP: {}/{})",
            attacker_ja, dmg, attacker.current_hp, attacker.stats.hp
        );
        if attacker.is_fainted() {
            println!("  {}はたおれた！", attacker_ja);
        }
    }
}

fn ability_damage_modifier(attacker: &Pokemon, move_type: Type) -> f32 {
    let low_hp = attacker.current_hp * 3 <= attacker.stats.hp;
    if !low_hp {
        return 1.0;
    }
    if attacker.has_ability("Blaze") && move_type == Type::Fire {
        return 1.5;
    }
    if attacker.has_ability("Torrent") && move_type == Type::Water {
        return 1.5;
    }
    if attacker.has_ability("Overgrow") && move_type == Type::Grass {
        return 1.5;
    }
    if attacker.has_ability("Swarm") && move_type == Type::Bug {
        return 1.5;
    }
    1.0
}

fn item_damage_modifier(attacker: &Pokemon, type_effectiveness: f32) -> f32 {
    battle_items::base_power_modifier(attacker, type_effectiveness)
}

fn chain_modifiers(modifiers: &[f32]) -> f32 {
    let mut combined = 1.0;
    for &modifier in modifiers {
        combined = chain_modifier(combined, modifier);
    }
    combined
}

fn apply_libero(attacker: &mut Pokemon, move_type: Type) {
    if attacker.has_ability("Libero") {
        attacker.types = [move_type, move_type];
    }
}

fn apply_stance_change(attacker: &mut Pokemon, normalized_move: &str, category: MoveCategory) {
    if !attacker.has_ability("Stance Change") {
        return;
    }
    let to_blade = !matches!(category, MoveCategory::Status) && normalized_move != "kingsshield";
    let to_shield = normalized_move == "kingsshield";
    if to_blade && !attacker.stance_blade {
        swap_stance_stats(attacker);
        attacker.stance_blade = true;
    } else if to_shield && attacker.stance_blade {
        swap_stance_stats(attacker);
        attacker.stance_blade = false;
    }
}

fn swap_stance_stats(pokemon: &mut Pokemon) {
    std::mem::swap(&mut pokemon.stats.atk, &mut pokemon.stats.def);
    std::mem::swap(&mut pokemon.stats.spa, &mut pokemon.stats.spd);
}

fn resolve_action(
    state: &mut BattleState,
    attacker_idx: usize,
    action: Action,
    defender_action: Action,
    defender_idx: usize,
    rng: &mut SmallRng,
) {
    match action {
        Action::Move(idx) => {
            crate::sim::moves::execute_move_state(state, attacker_idx, idx, defender_action, defender_idx, rng);
        }
        Action::Switch(idx) => {
            perform_switch(state, attacker_idx, idx, SwitchKind::Voluntary, rng);
        }
    }
}

fn passes_accuracy(
    accuracy: Option<f32>,
    attacker: &Pokemon,
    defender: &Pokemon,
    rng: &mut SmallRng,
) -> bool {
    if attacker.has_ability("No Guard") || defender.has_ability("No Guard") {
        return true;
    }
    match accuracy {
        None => true,
        Some(acc) => {
            let acc_mod = accuracy_multiplier(attacker.accuracy_stage);
            let eva_mod = accuracy_multiplier(defender.evasion_stage);
            let mut final_acc = acc * acc_mod / eva_mod;
            if attacker.has_ability("Compound Eyes") {
                final_acc *= 1.3;
            }
            let final_acc = final_acc.clamp(0.0, 100.0);
            rng.gen_range(0.0..100.0) < final_acc
        }
    }
}

fn effective_accuracy(move_data: &crate::data::moves::MoveData, weather: Option<Weather>) -> Option<f32> {
    crate::sim::weather_field::effective_accuracy(move_data, weather)
}

fn is_charging_move(normalized_move: &str) -> bool {
    matches!(normalized_move, "solarbeam" | "fly" | "bounce" | "phantomforce")
}

#[allow(dead_code)]
fn is_semi_invulnerable_move(normalized_move: &str) -> bool {
    matches!(normalized_move, "fly" | "bounce" | "phantomforce")
}

fn is_ohko_move(normalized_move: &str) -> bool {
    matches!(normalized_move, "fissure" | "guillotine" | "horndrill" | "sheercold")
}

fn move_hit_count(
    move_data: &crate::data::moves::MoveData,
    normalized_move: &str,
    rng: &mut SmallRng,
) -> u8 {
    let _ = normalized_move;
    calculate_multihit_count(move_data, rng)
}

fn critical_stage(move_data: &crate::data::moves::MoveData) -> u8 {
    move_data
        .crit_ratio
        .map(|ratio| ratio.saturating_sub(1))
        .unwrap_or(0)
        .min(3)
}

fn roll_critical(stage: u8, rng: &mut SmallRng) -> bool {
    let chance = match stage {
        0 => 1.0 / 24.0,
        1 => 1.0 / 8.0,
        2 => 0.5,
        _ => 1.0,
    };
    rng.gen_bool(chance)
}

fn fixed_damage(normalized_move: &str, attacker: &Pokemon, defender: &Pokemon) -> Option<u16> {
    match normalized_move {
        "seismictoss" | "nightshade" => Some(attacker.level as u16),
        "endeavor" => {
            if defender.current_hp > attacker.current_hp {
                Some(defender.current_hp - attacker.current_hp)
            } else {
                Some(0)
            }
        }
        "horndrill" => Some(defender.current_hp),
        _ => None,
    }
}

pub(crate) fn apply_status_with_field(
    target: &mut Pokemon,
    status: Status,
    toxic: bool,
    field: Option<Field>,
    rng: &mut SmallRng,
) -> bool {
    if is_grounded(target) {
        match field {
            Some(Field::Electric) => {
                if matches!(status, Status::Sleep) {
                    return false;
                }
            }
            Some(Field::Misty) => {
                return false;
            }
            _ => {}
        }
    }
    if toxic {
        target.apply_toxic(rng)
    } else {
        target.apply_status(status, rng)
    }
}

pub(crate) fn screen_turns(attacker: &Pokemon) -> u8 {
    if has_item(attacker, "lightclay") {
        8
    } else {
        5
    }
}

fn screen_damage_modifier(
    reflect_turns: u8,
    light_screen_turns: u8,
    aurora_veil_turns: u8,
    category: MoveCategory,
    is_crit: bool,
) -> f32 {
    if is_crit {
        return 1.0;
    }
    if aurora_veil_turns > 0 {
        return 0.5;
    }
    match category {
        MoveCategory::Physical => {
            if reflect_turns > 0 {
                0.5
            } else {
                1.0
            }
        }
        MoveCategory::Special => {
            if light_screen_turns > 0 {
                0.5
            } else {
                1.0
            }
        }
        MoveCategory::Status => 1.0,
    }
}

fn burn_damage_modifier(attacker: &Pokemon, category: MoveCategory, move_id: &str) -> f32 {
    if matches!(category, MoveCategory::Physical)
        && matches!(attacker.status, Some(Status::Burn))
        && !attacker.has_ability("Guts")
        && move_id != "facade"
    {
        0.5
    } else {
        1.0
    }
}

fn field_damage_modifier(field: Option<Field>, attacker: &Pokemon, defender: &Pokemon, move_type: Type, move_id: &str) -> f32 {
    crate::sim::weather_field::field_damage_modifier(field, attacker, defender, move_type, move_id)
}

fn weather_damage_modifier(weather: Option<Weather>, move_type: Type) -> f32 {
    crate::sim::weather_field::weather_damage_modifier(weather, move_type)
}

#[allow(dead_code)]
fn map_status(id: &str) -> Option<(Status, bool)> {
    match id {
        "brn" => Some((Status::Burn, false)),
        "par" => Some((Status::Paralysis, false)),
        "psn" => Some((Status::Poison, false)),
        "tox" => Some((Status::Poison, true)),
        "slp" => Some((Status::Sleep, false)),
        "frz" => Some((Status::Freeze, false)),
        _ => None,
    }
}

fn can_act(pokemon: &mut Pokemon, rng: &mut SmallRng) -> bool {
    if pokemon.flinched {
        pokemon.flinched = false;
        return false;
    }
    if pokemon.confusion_turns > 0 {
        pokemon.confusion_turns = pokemon.confusion_turns.saturating_sub(1);
        if rng.gen_bool(1.0 / 3.0) {
            let atk = apply_stage_multiplier(pokemon.stats.atk, pokemon.stat_stages[STAGE_ATK]);
            let def = apply_stage_multiplier(pokemon.stats.def, pokemon.stat_stages[STAGE_DEF]);
            let random_factor = rng.gen_range(85..=100) as f32 / 100.0;
            let dmg = calculate_damage(pokemon.level, atk, def, 40, 1.0, false, random_factor, 1.0);
            pokemon.take_damage(dmg);
            println!(
                "  {}はこんらんしてじぶんを こうげきした！ (HP: {}/{})",
                translate_pokemon(&pokemon.species),
                pokemon.current_hp,
                pokemon.stats.hp
            );
            if pokemon.is_fainted() {
                println!("  {}はたおれた！", translate_pokemon(&pokemon.species));
            }
            return false;
        }
    }
    match pokemon.status {
        Some(Status::Sleep) => {
            if pokemon.sleep_turns == 0 {
                pokemon.clear_status();
                return true;
            }
            false
        }
        Some(Status::Freeze) => {
            if rng.gen_bool(0.2) {
                pokemon.clear_status();
                true
            } else {
                false
            }
        }
        Some(Status::Paralysis) => !rng.gen_bool(0.25),
        _ => true,
    }
}

fn apply_start_of_turn_effects(state: &mut BattleState, rng: &mut SmallRng) {
    let field = state.field;
    for pokemon in [&mut state.pokemon_a, &mut state.pokemon_b] {
        if pokemon.is_fainted() {
            continue;
        }
        pokemon.flinched = false;
        if pokemon.is_fainted() {
            continue;
        }
        if has_item(pokemon, "flameorb") && pokemon.status.is_none() {
            if apply_status_with_field(pokemon, Status::Burn, false, field, rng) {
                println!(
                    "  {}は{}のこうかで {}！",
                    translate_pokemon(&pokemon.species),
                    translate_item("Flame Orb"),
                    format_status(Status::Burn)
                );
            }
        }
    }
}

pub(crate) fn apply_end_of_turn_effects(state: &mut BattleState, rng: &mut SmallRng) {
    let weather = state.weather;
    let field = state.field;
    for pokemon in [&mut state.pokemon_a, &mut state.pokemon_b] {
        if pokemon.is_fainted() {
            continue;
        }
        match pokemon.status {
            Some(Status::Burn) => {
                let dmg = (pokemon.stats.hp as u32 / 16).max(1) as u16;
                pokemon.take_damage(dmg);
                println!(
                    "  {}はやけどでダメージをうけた！ (HP: {}/{})",
                    translate_pokemon(&pokemon.species),
                    pokemon.current_hp,
                    pokemon.stats.hp
                );
            }
            Some(Status::Poison) => {
                if let Some(heal) = poison_heal_amount(pokemon) {
                    pokemon.current_hp = (pokemon.current_hp + heal).min(pokemon.stats.hp);
                    println!(
                        "  {}はポイズンヒールで たいりょくをかいふくした！ (HP: {}/{})",
                        translate_pokemon(&pokemon.species),
                        pokemon.current_hp,
                        pokemon.stats.hp
                    );
                } else {
                    let dmg = if pokemon.toxic_counter > 0 {
                        let dmg = (pokemon.stats.hp as u32 * pokemon.toxic_counter as u32 / 16).max(1) as u16;
                        pokemon.toxic_counter = pokemon.toxic_counter.saturating_add(1).min(15); // PS: statusState.stage max 15
                        dmg
                    } else {
                        (pokemon.stats.hp as u32 / 8).max(1) as u16
                    };
                    pokemon.take_damage(dmg);
                    println!(
                        "  {}はどくでダメージをうけた！ (HP: {}/{})",
                        translate_pokemon(&pokemon.species),
                        pokemon.current_hp,
                        pokemon.stats.hp
                    );
                }
            }
            Some(Status::Sleep) => {
                // PS: statusState.time
                if pokemon.sleep_turns > 0 {
                    pokemon.sleep_turns = pokemon.sleep_turns.saturating_sub(1);
                    if pokemon.sleep_turns == 0 {
                        pokemon.clear_status();
                    }
                }
            }
            _ => {}
        }
        if pokemon.taunt_turns > 0 {
            pokemon.taunt_turns = pokemon.taunt_turns.saturating_sub(1);
            if pokemon.taunt_turns == 0 {
                println!(
                    "  {}のちょうはつが とけた！",
                    translate_pokemon(&pokemon.species)
                );
            }
        }
        if pokemon.encore_turns > 0 {
            pokemon.encore_turns = pokemon.encore_turns.saturating_sub(1);
            if pokemon.encore_turns == 0 {
                pokemon.encore_move = None;
                println!(
                    "  {}のアンコールが とけた！",
                    translate_pokemon(&pokemon.species)
                );
            }
        }
        if pokemon.telekinesis_turns > 0 {
            pokemon.telekinesis_turns = pokemon.telekinesis_turns.saturating_sub(1);
            if pokemon.telekinesis_turns == 0 {
                println!(
                    "  {}は もとにもどった！",
                    translate_pokemon(&pokemon.species)
                );
            }
        }
        if pokemon.perish_count > 0 {
            pokemon.perish_count = pokemon.perish_count.saturating_sub(1);
            if pokemon.perish_count == 0 && !pokemon.is_fainted() {
                pokemon.current_hp = 0;
                println!(
                    "  {}はほろびのうたで たおれた！",
                    translate_pokemon(&pokemon.species)
                );
                continue;
            }
        }
        if let Some(effect) = battle_items::end_of_turn_effect(pokemon) {
            match effect {
                battle_items::EndOfTurnEffect::Heal { amount, item_id } => {
                    pokemon.current_hp = (pokemon.current_hp + amount).min(pokemon.stats.hp);
                    let item_name = match item_id {
                        "leftovers" => translate_item("Leftovers"),
                        "blacksludge" => translate_item("Black Sludge"),
                        _ => item_id.to_string(),
                    };
                    println!(
                        "  {}は{}で たいりょくをかいふくした！ (HP: {}/{})",
                        translate_pokemon(&pokemon.species),
                        item_name,
                        pokemon.current_hp,
                        pokemon.stats.hp
                    );
                }
                battle_items::EndOfTurnEffect::Damage { amount, item_id } => {
                    pokemon.take_damage(amount);
                    let item_name = match item_id {
                        "blacksludge" => translate_item("Black Sludge"),
                        _ => item_id.to_string(),
                    };
                    println!(
                        "  {}は{}で ダメージをうけた！ (HP: {}/{})",
                        translate_pokemon(&pokemon.species),
                        item_name,
                        pokemon.current_hp,
                        pokemon.stats.hp
                    );
                    if pokemon.is_fainted() {
                        println!("  {}はたおれた！", translate_pokemon(&pokemon.species));
                    }
                }
            }
        }
        if let Some(Field::Grassy) = field {
            if is_grounded(pokemon) {
                let heal = (pokemon.stats.hp as u32 / 16).max(1) as u16;
                if pokemon.current_hp < pokemon.stats.hp {
                    pokemon.current_hp = (pokemon.current_hp + heal).min(pokemon.stats.hp);
                    println!(
                        "  {}はグラスフィールドでかいふくした！ (HP: {}/{})",
                        translate_pokemon(&pokemon.species),
                        pokemon.current_hp,
                        pokemon.stats.hp
                    );
                }
            }
        }
        if let Some((dmg, kind)) = crate::sim::weather_field::weather_residual_damage(pokemon, weather) {
            pokemon.take_damage(dmg);
            let msg = match kind {
                Weather::Sand => "すなあらし",
                Weather::Hail => "あられ",
                _ => "てんこう",
            };
            println!(
                "  {}は{}でダメージをうけた！ (HP: {}/{})",
                translate_pokemon(&pokemon.species),
                msg,
                pokemon.current_hp,
                pokemon.stats.hp
            );
            if pokemon.is_fainted() {
                println!("  {}はたおれた！", translate_pokemon(&pokemon.species));
                continue;
            }
        }
        let _ = rng;
    }
    apply_wish(&mut state.side_a, &mut state.pokemon_a);
    apply_wish(&mut state.side_b, &mut state.pokemon_b);
    if state.field_turns > 0 {
        state.field_turns = state.field_turns.saturating_sub(1);
        if state.field_turns == 0 {
            state.field = None;
        }
    }
    if state.weather_turns > 0 {
        state.weather_turns = state.weather_turns.saturating_sub(1);
        if state.weather_turns == 0 {
            state.weather = None;
        }
    }
    if state.trick_room_turns > 0 {
        state.trick_room_turns = state.trick_room_turns.saturating_sub(1);
        if state.trick_room_turns == 0 {
            println!("  トリックルームが もとにもどった！");
        }
    }
    crate::sim::moves::decrement_side_conditions(&mut state.side_a);
    crate::sim::moves::decrement_side_conditions(&mut state.side_b);
}

fn apply_wish(side: &mut SideConditions, pokemon: &mut Pokemon) {
    if side.wish_turns == 0 {
        return;
    }
    side.wish_turns = side.wish_turns.saturating_sub(1);
    if side.wish_turns == 0 && !pokemon.is_fainted() {
        let heal = side.wish_heal.max(1);
        if pokemon.current_hp < pokemon.stats.hp {
            pokemon.current_hp = (pokemon.current_hp + heal).min(pokemon.stats.hp);
            println!(
                "  {}はねがいごとで たいりょくをかいふくした！ (HP: {}/{})",
                translate_pokemon(&pokemon.species),
                pokemon.current_hp,
                pokemon.stats.hp
            );
        }
        side.wish_heal = 0;
    }
}

pub(crate) fn execute_move_impl(
    state: &mut BattleState,
    attacker_idx: usize,
    move_idx: usize,
    defender_action: Action,
    defender_idx: usize,
    rng: &mut SmallRng,
) {
    let weather = state.weather;
    let field = state.field;
    let trick_room_turns = state.trick_room_turns;
    let (defender_reflect_turns, defender_light_screen_turns, defender_aurora_veil_turns) =
        if defender_idx == 0 {
            (
                state.side_a.reflect_turns,
                state.side_a.light_screen_turns,
                state.side_a.aurora_veil_turns,
            )
    } else {
            (
                state.side_b.reflect_turns,
                state.side_b.light_screen_turns,
                state.side_b.aurora_veil_turns,
            )
    };
    let mut env_update = EnvUpdate::default();
    let mut pending_force_switch: Option<usize> = None;
    let mut pending_clear_hazards: Option<HazardClear> = None;
    let mut pending_pivot_switch: Option<usize> = None;
    let mut status_move_used = false;
    let do_log = state.logger.is_some();
    let mut pending_move_log: Option<(String, String, String)> = None;
    let mut pending_damage_logs: Vec<(String, u16, u16)> = Vec::new();
    {
        let (attacker, defender) = if attacker_idx == 0 {
            (&mut state.pokemon_a, &mut state.pokemon_b)
        } else {
            (&mut state.pokemon_b, &mut state.pokemon_a)
        };
        if attacker.is_fainted() || defender.is_fainted() {
            return;
        }
        let mut resolved_idx = move_idx;
        if !attacker.item_consumed && item_id(attacker).as_deref().is_some_and(battle_items::is_choice_item_id) {
            if let Some(locked) = attacker.choice_lock_move.clone() {
                if let Some((idx, _)) = attacker
                    .moves
                    .iter()
                    .enumerate()
                    .find(|(_, name)| name.as_str() == locked)
                {
                    resolved_idx = idx;
                }
            }
        }
        if attacker.encore_turns > 0 {
            if let Some(encore) = attacker.encore_move.clone() {
                if let Some((idx, _)) = attacker
                    .moves
                    .iter()
                    .enumerate()
                    .find(|(_, name)| name.as_str() == encore)
                {
                    resolved_idx = idx;
                } else {
                    attacker.encore_turns = 0;
                    attacker.encore_move = None;
                }
            }
        }
        let move_name = match attacker.moves.get(resolved_idx) {
            Some(name) => name,
            None => return,
        };
        let mut move_data = match get_move(move_name.as_str()) {
            Some(data) => data,
            None => {
                eprintln!("Warning: Move '{}' not found", move_name);
                return;
            }
        };
        let mut normalized = crate::data::moves::normalize_move_name(move_data.name);
        if let Some(charging) = attacker.charging_move.clone() {
            if charging != normalized {
                if let Some(data) = get_move(charging.as_str()) {
                    move_data = data;
                    normalized = charging;
                }
            }
        }
        let attacker_ja = translate_pokemon(&attacker.species);
        let bypass_substitute = bypasses_substitute(&move_data);
        let bypass_protect = bypasses_protect(&move_data);
        let targets_opponent = targets_opponent_pokemon(move_data.target);
        if !matches!(normalized.as_str(), "protect" | "kingsshield" | "detect" | "endure") {
            attacker.protect_counter = 0;
        }
        apply_stance_change(attacker, normalized.as_str(), move_data.category);
        if attacker.destiny_bond && normalized != "destinybond" {
            attacker.destiny_bond = false;
        }
        let is_second_turn = attacker.charging_move.as_deref() == Some(normalized.as_str());
        if matches!(move_data.category, MoveCategory::Status) && attacker.taunt_turns > 0 {
            println!("  {}はちょうはつされて へんかわざがだせない！", attacker_ja);
            return;
        }
        if !can_act(attacker, rng) {
            if matches!(normalized.as_str(), "protect" | "kingsshield" | "detect" | "endure") {
                attacker.protect_counter = 0;
            }
            if attacker.charging_move.is_some() {
                attacker.charging_move = None;
            }
            println!("  {}はうまくうごけなかった！", translate_pokemon(&attacker.species));
            return;
        }
        let move_ja = translate_move(move_data.name);
        println!("  {}は{}をつかった！", attacker_ja, move_ja);
        if do_log {
            pending_move_log = Some((
                showdown_ident(attacker_idx, &attacker.species),
                move_data.name.to_string(),
                showdown_ident(defender_idx, &defender.species),
            ));
        }
        battle_items::set_choice_lock_move(attacker, normalized.as_str());
        attacker.last_move = Some(normalized.clone());
        if targets_opponent && check_ability_immunity(defender, &move_data) {
            if is_second_turn {
                attacker.charging_move = None;
            }
            println!("  しかし こうかがなかった！");
            return;
        }
        if normalized == "suckerpunch" && !is_attack_action(defender_action, defender) {
            if is_second_turn {
                attacker.charging_move = None;
            }
            println!("  しかし うまくきまらなかった！");
            return;
        }
        if defender.semi_invulnerable && !matches!(move_data.category, MoveCategory::Status) {
            if is_second_turn {
                attacker.charging_move = None;
            }
            println!("  しかし あたらなかった！");
            return;
        }
        if is_charging_move(normalized.as_str()) && !is_second_turn {
            let mut skip_charge = false;
            if normalized == "solarbeam" && matches!(weather, Some(Weather::Sun)) {
                skip_charge = true;
            }
            if has_consumable_item(attacker, "powerherb") {
                consume_item(attacker);
                skip_charge = true;
                println!(
                    "  {}の{}が こうかをあらわした！",
                    attacker_ja,
                    translate_item("Power Herb")
                );
            }
            if !skip_charge {
                handle_charging_move(attacker, normalized.as_str());
                println!("  {}はちからをためている！", attacker_ja);
                return;
            }
        }
        let is_ohko = is_ohko_move(normalized.as_str());
        let mut ohko_damage = None;
        if is_ohko {
            ohko_damage = handle_ohko_move(attacker, defender, normalized.as_str(), rng);
            if ohko_damage.is_none() {
                if is_second_turn {
                    attacker.charging_move = None;
                }
                println!("  しかし あたらなかった！");
                return;
            }
        } else {
            let acc = effective_accuracy(&move_data, weather);
            if !passes_accuracy(acc, attacker, defender, rng) {
                if is_second_turn {
                    attacker.charging_move = None;
                }
                println!("  しかし あたらなかった！");
                return;
            }
        }
        if matches!(move_data.category, MoveCategory::Status) {
            if defender.substitute_hp > 0 && !bypass_substitute && targets_opponent {
                if is_second_turn {
                    attacker.charging_move = None;
                }
                println!("  しかし みがわりが まもっている！");
                return;
            }
            if targets_opponent && defender.magic_coat_active {
                let defender_ja = translate_pokemon(&defender.species);
                println!("  {}は マジックコートで はねかえした！", defender_ja);
                env_update = handle_status_move(
                    defender,
                    attacker,
                    &move_data,
                    field,
                    weather,
                    trick_room_turns,
                    attacker_idx,
                    rng,
                );
            } else {
                env_update = handle_status_move(
                    attacker,
                    defender,
                    &move_data,
                    field,
                    weather,
                    trick_room_turns,
                    defender_idx,
                    rng,
                );
            }
            status_move_used = true;
        } else {
            if field == Some(Field::Psychic)
                && is_grounded(defender)
                && move_data.priority > 0
            {
                if is_second_turn {
                    attacker.charging_move = None;
                }
                println!("  サイコフィールドのちからで うまくきまらなかった！");
                return;
            }
            if defender.protect_active && !bypass_protect {
                if defender.kings_shield_active && is_contact_move(&move_data) {
                    let attacker_ja = translate_pokemon(&attacker.species);
                    if !apply_stage_change(attacker, &attacker_ja, STAGE_ATK, -2) {
                        println!("  しかし こうかがなかった！");
                    }
                }
                if is_second_turn {
                    attacker.charging_move = None;
                }
                println!("  しかし まもられた！");
                return;
            }
            let move_type = parse_type(move_data.move_type);
            apply_libero(attacker, move_type);
            let defender_ja = translate_pokemon(&defender.species);
            if defender.substitute_hp == 0 || bypass_substitute {
                if let Some(absorb) = try_absorb_water_move(defender, move_type) {
                    if is_second_turn {
                        attacker.charging_move = None;
                    }
                    println!(
                        "  {}は{}で たいりょくをかいふくした！ (HP: {}/{})",
                        defender_ja,
                        absorb.kind.display_name(),
                        defender.current_hp,
                        defender.stats.hp
                    );
                    return;
                }
            }
            let mut power = calculate_variable_power(&move_data, attacker, defender, weather, field);
            if attacker.charge_active && move_type == Type::Electric {
                power = power.saturating_mul(2).max(1);
                attacker.charge_active = false;
            }
            if power == 0
                && ohko_damage.is_none()
                && fixed_damage(normalized.as_str(), attacker, defender).is_none()
            {
                if is_second_turn {
                    attacker.charging_move = None;
                }
                return;
            }
            if normalized.as_str() == "solarbeam" {
                match weather {
                    Some(Weather::Rain) | Some(Weather::Sand) | Some(Weather::Hail) => {
                        power = (power / 2).max(1);
                    }
                    _ => {}
                }
            }
            if matches!(defender.status, Some(Status::Freeze)) && move_type == Type::Fire {
                defender.clear_status();
                println!("  {}のこおりがとけた！", translate_pokemon(&defender.species));
            }
            let defender_types = effective_types(defender);
            let type_effectiveness =
                effectiveness_dual(move_type, defender_types[0], defender_types[1]);
            let ability_mod = ability_damage_modifier(attacker, move_type);
            let item_mod = item_damage_modifier(attacker, type_effectiveness);
            let weather_mod = weather_damage_modifier(weather, move_type);
            let field_mod = field_damage_modifier(field, attacker, defender, move_type, normalized.as_str());
            let burn_mod = burn_damage_modifier(attacker, move_data.category, normalized.as_str());
            if type_effectiveness == 0.0 {
                if is_second_turn {
                    attacker.charging_move = None;
                }
                println!("  しかし こうかがないようだ！");
                return;
            }
            let is_sandstorm = matches!(weather, Some(Weather::Sand));
            let attacker_ability_mod =
                ability_attack_modifier(attacker, &move_data, move_type, is_sandstorm);
            let defender_ability_mod =
                ability_defense_modifier(defender, &move_data, type_effectiveness);
            let type_item_mod = attacker
                .item
                .as_deref()
                .map(|item| item_type_boost(item, move_type))
                .unwrap_or(1.0);
            let base_final_mod =
                chain_modifiers(&[ability_mod, attacker_ability_mod, defender_ability_mod, item_mod, type_item_mod, field_mod]);
            let stab = is_stab(move_type, attacker.types);
            let hits = move_hit_count(&move_data, normalized.as_str(), rng);
            let crit_stage = critical_stage(&move_data);
            let mut total_damage: u16 = 0;
            let mut damage_to_target: u16 = 0;
            for _ in 0..hits {
                if attacker.is_fainted() || defender.is_fainted() {
                    break;
                }
                let is_crit = roll_critical(crit_stage, rng);
                let attacker_stat = match move_data.category {
                    MoveCategory::Physical => {
                        let stage = if is_crit {
                            attacker.stat_stages[STAGE_ATK].max(0)
                        } else {
                            attacker.stat_stages[STAGE_ATK]
                        };
                        let atk = apply_stage_multiplier(attacker.stats.atk, stage);
                        let item_mod = battle_items::attack_stat_modifier(attacker, MoveCategory::Physical);
                        (((atk as f32) * item_mod).floor() as u16).max(1)
                    }
                    MoveCategory::Special => {
                        let stage = if is_crit {
                            attacker.stat_stages[STAGE_SPA].max(0)
                        } else {
                            attacker.stat_stages[STAGE_SPA]
                        };
                        let spa = apply_stage_multiplier(attacker.stats.spa, stage);
                        let item_mod = battle_items::attack_stat_modifier(attacker, MoveCategory::Special);
                        (((spa as f32) * item_mod).floor() as u16).max(1)
                    }
                    MoveCategory::Status => return,
                };
                let defender_stat = match move_data.category {
                    MoveCategory::Physical => {
                        let stage = if is_crit {
                            defender.stat_stages[STAGE_DEF].min(0)
                        } else {
                            defender.stat_stages[STAGE_DEF]
                        };
                        let mut def = apply_stage_multiplier(defender.stats.def, stage);
                        if defender.status.is_some()
                            && defender.ability.eq_ignore_ascii_case("Marvel Scale")
                        {
                            def = ((def as f32) * 1.5).floor() as u16;
                        }
                        def.max(1)
                    }
                    MoveCategory::Special => {
                        let stage = if is_crit {
                            defender.stat_stages[STAGE_SPD].min(0)
                        } else {
                            defender.stat_stages[STAGE_SPD]
                        };
                        let mut spd = apply_stage_multiplier(defender.stats.spd, stage);
                        if has_item(defender, "assaultvest") {
                            spd = ((spd as f32) * 1.5).floor() as u16;
                        }
                        if weather == Some(Weather::Sand)
                            && (defender.types[0] == Type::Rock || defender.types[1] == Type::Rock)
                        {
                            spd = ((spd as f32) * 1.5).floor() as u16;
                        }
                        spd.max(1)
                    }
                    MoveCategory::Status => return,
                };
                let random_factor = rng.gen_range(85..=100) as f32 / 100.0;
                let crit_mod = if is_crit { 1.5 } else { 1.0 };
                let screen_mod = screen_damage_modifier(
                    defender_reflect_turns,
                    defender_light_screen_turns,
                    defender_aurora_veil_turns,
                    move_data.category,
                    is_crit,
                );
                let final_mod = chain_modifier(base_final_mod, screen_mod);
                let fixed = ohko_damage.or_else(|| fixed_damage(normalized.as_str(), attacker, defender));
                let mut damage = if let Some(fixed) = fixed {
                    fixed
                } else {
                    calculate_damage_with_modifiers(
                        attacker.level,
                        attacker_stat,
                        defender_stat,
                        power,
                        type_effectiveness,
                        stab,
                        random_factor,
                        DamageModifiers {
                            weather: weather_mod,
                            crit: crit_mod,
                            burn: burn_mod,
                            final_modifier: final_mod,
                        },
                    )
                };
                if damage == 0 {
                    if fixed.is_some() {
                        println!("  しかし こうかがないようだ！");
                        break;
                    }
                    continue;
                }
                if defender.substitute_hp > 0 && !bypass_substitute {
                    let sub_damage = damage.min(defender.substitute_hp);
                    defender.substitute_hp = defender.substitute_hp.saturating_sub(damage);
                    total_damage = total_damage.saturating_add(damage);
                    println!(
                        "  {}のみがわりは{}のダメージをうけた！",
                        defender_ja, sub_damage
                    );
                    if defender.substitute_hp == 0 {
                        println!("  {}のみがわりは こわれた！", defender_ja);
                    }
                    continue;
                }
                if crate::sim::items::consumable::try_consume_resist_berry(
                    defender,
                    move_type,
                    type_effectiveness,
                ) {
                    damage = (damage / 2).max(1);
                    println!(
                        "  {}のきのみが こうかをあらわした！",
                        defender_ja
                    );
                }
                let (final_damage, prevention) = prevent_ko_if_applicable(defender, damage);
                if let Some(prevention) = prevention {
                    match prevention {
                        KoPrevention::Endure => println!("  {}はこらえている！", defender_ja),
                        KoPrevention::Sturdy => println!("  {}はがんじょうで もちこたえた！", defender_ja),
                        KoPrevention::FocusSash => println!("  {}はきあいのタスキで もちこたえた！", defender_ja),
                    }
                }
                defender.take_damage(final_damage);
                total_damage = total_damage.saturating_add(final_damage);
                damage_to_target = damage_to_target.saturating_add(final_damage);
                println!(
                    "  {}は{}のダメージをうけた！ (HP: {}/{})",
                    defender_ja, final_damage, defender.current_hp, defender.stats.hp
                );
                if do_log {
                    pending_damage_logs.push((
                        showdown_ident(defender_idx, &defender.species),
                        defender.current_hp,
                        defender.stats.hp,
                    ));
                }
                if is_crit {
                    println!("  きゅうしょにあたった！");
                }
                for effect in secondary_effects_from_move(normalized.as_str(), &move_data) {
                    let applied = apply_secondary_effect_with_update(
                        attacker,
                        defender,
                        &effect,
                        field,
                        attacker_idx,
                        defender_idx,
                        &mut env_update,
                        rng,
                    );
                    if applied {
                        if let Some(status) = effect.status {
                            if status != Status::Flinch {
                                let (target_name, target_status) = if effect.target_self {
                                    (&attacker_ja, attacker.status)
                                } else {
                                    (&defender_ja, defender.status)
                                };
                                if target_status == Some(status) {
                                    println!("  {}は{}！", target_name, format_status(status));
                                }
                            }
                        }
                    }
                }
                apply_contact_abilities(attacker, defender, &move_data, field, rng);
                if defender.is_fainted() {
                    println!("  {}はたおれた！", defender_ja);
                    if let Some(dmg) = apply_aftermath_if_applicable(attacker, defender, &move_data) {
                        println!(
                            "  {}はゆうばくで{}のダメージをうけた！ (HP: {}/{})",
                            attacker_ja, dmg, attacker.current_hp, attacker.stats.hp
                        );
                    }
                    if defender.destiny_bond && !attacker.is_fainted() {
                        defender.destiny_bond = false;
                        attacker.take_damage(attacker.current_hp);
                        println!("  {}はみちづれになった！", attacker_ja);
                    }
                    break;
                }
                if attacker.is_fainted() {
                    break;
                }
            }
            if is_second_turn {
                attacker.charging_move = None;
            }
            if total_damage > 0 {
                if let Some(effect) = self_effect_from_move(normalized.as_str(), &move_data) {
                    let applied = apply_secondary_effect_with_update(
                        attacker,
                        defender,
                        &effect,
                        field,
                        attacker_idx,
                        defender_idx,
                        &mut env_update,
                        rng,
                    );
                    if applied {
                        if let Some(status) = effect.status {
                            if attacker.status == Some(status) {
                                println!("  {}は{}！", attacker_ja, format_status(status));
                            }
                        }
                    }
                }
            }
            if total_damage > 0 {
                if normalized.as_str() == "rapidspin" {
                    pending_clear_hazards = Some(HazardClear::Side(attacker_idx));
                }
                if let Some(drain) = move_data.drain {
                    if damage_to_target > 0 {
                        apply_drain(attacker, damage_to_target, drain);
                        println!(
                            "  {}はHPをすいとった！ (HP: {}/{})",
                            attacker_ja, attacker.current_hp, attacker.stats.hp
                        );
                    }
                }
                if let Some(recoil) = move_data.recoil {
                    let hp_before = attacker.current_hp;
                    apply_recoil_damage(attacker, total_damage, recoil);
                    let _recoil = hp_before.saturating_sub(attacker.current_hp);
                    println!(
                        "  {}ははんどうをうけた！ (HP: {}/{})",
                        attacker_ja, attacker.current_hp, attacker.stats.hp
                    );
                    if attacker.is_fainted() {
                        println!("  {}はたおれた！", attacker_ja);
                    }
                }
                if has_item(attacker, "lifeorb") {
                    let recoil = (attacker.stats.hp as u32 / 10).max(1) as u16;
                    attacker.take_damage(recoil);
                    println!(
                        "  {}は{}のはんどうをうけた！ (HP: {}/{})",
                        attacker_ja,
                        translate_item("Life Orb"),
                        attacker.current_hp,
                        attacker.stats.hp
                    );
                    if attacker.is_fainted() {
                        println!("  {}はたおれた！", attacker_ja);
                    }
                }
            }
            if !defender.is_fainted() {
                if let Some(_heal) = crate::sim::items::consumable::try_consume_sitrus_berry(defender) {
                    println!(
                        "  {}は{}で たいりょくをかいふくした！ (HP: {}/{})",
                        defender_ja,
                        translate_item("Sitrus Berry"),
                        defender.current_hp,
                        defender.stats.hp
                    );
                }
            }
            if normalized.as_str() == "clearsmog" && !defender.is_fainted() && damage_to_target > 0 {
                reset_stat_stages(defender, &defender_ja);
            }
            if !defender.is_fainted()
                && damage_to_target > 0
                && matches!(normalized.as_str(), "dragontail" | "circlethrow")
            {
                pending_force_switch = Some(defender_idx);
            }
            if total_damage > 0
                && !attacker.is_fainted()
                && switching::is_pivot_move(normalized.as_str())
            {
                pending_pivot_switch = Some(attacker_idx);
            }
        }
    }
    if do_log {
        if let Some(logger) = state.logger.as_mut() {
            if let Some((src, mv, tgt)) = pending_move_log {
                logger.log_move(&src, &mv, &tgt);
            }
            for (tgt, hp, max_hp) in pending_damage_logs {
                logger.log_damage(&tgt, hp, max_hp);
            }
        }
    }
    if status_move_used {
        apply_env_update(state, env_update, rng);
        return;
    }
    if let Some(target) = pending_force_switch {
        let update = EnvUpdate {
            force_switch: Some(target),
            ..EnvUpdate::default()
        };
        apply_env_update(state, update, rng);
    }
    if let Some(clear) = pending_clear_hazards {
        let update = EnvUpdate {
            clear_hazards: Some(clear),
            ..EnvUpdate::default()
        };
        apply_env_update(state, update, rng);
    }
    if let Some(side_idx) = pending_pivot_switch {
        let bench_idx = {
            let bench = bench_mut(state, side_idx);
            switching::pick_random_switch(bench, rng)
        };
        if let Some(idx) = bench_idx {
            perform_switch(state, side_idx, idx, SwitchKind::Pivot, rng);
        }
    }
}

fn parse_type(name: &str) -> Type {
    match name.to_ascii_lowercase().as_str() {
        "normal" => Type::Normal,
        "fire" => Type::Fire,
        "water" => Type::Water,
        "electric" => Type::Electric,
        "grass" => Type::Grass,
        "ice" => Type::Ice,
        "fighting" => Type::Fighting,
        "poison" => Type::Poison,
        "ground" => Type::Ground,
        "flying" => Type::Flying,
        "psychic" => Type::Psychic,
        "bug" => Type::Bug,
        "rock" => Type::Rock,
        "ghost" => Type::Ghost,
        "dragon" => Type::Dragon,
        "dark" => Type::Dark,
        "steel" => Type::Steel,
        "fairy" => Type::Fairy,
        _ => Type::Normal,
    }
}

pub(crate) fn format_status(status: crate::sim::pokemon::Status) -> &'static str {
    match status {
        crate::sim::pokemon::Status::Burn => "やけどをおった",
        crate::sim::pokemon::Status::Paralysis => "まひした",
        crate::sim::pokemon::Status::Poison => "どくをうけた",
        crate::sim::pokemon::Status::Sleep => "ねむってしまった",
        crate::sim::pokemon::Status::Freeze => "こおってしまった",
        crate::sim::pokemon::Status::Flinch => "ひるんだ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ai::RandomAI;

    fn make_pokemon(moves: Vec<String>) -> Pokemon {
        Pokemon::new(
            "charizard",
            50,
            [0; 6],
            [0; 6],
            crate::sim::stats::Nature::Hardy,
            moves,
            "Blaze",
            None,
        )
        .expect("species exists")
    }

    #[test]
    fn test_damage_application() {
        let mut attacker = make_pokemon(vec!["thunderbolt".to_string()]);
        attacker.stats.spa = 120;
        attacker.stats.atk = 84;
        let defender = make_pokemon(vec!["flamethrower".to_string()]);
        let mut state = BattleState::new(attacker, defender);
        let mut rng = SmallRng::seed_from_u64(42);
        execute_move_impl(&mut state, 0, 0, Action::Move(0), 1, &mut rng);
        assert!(state.pokemon_b.current_hp < state.pokemon_b.stats.hp);
    }

    #[test]
    fn test_u_turn_pivots_after_hit() {
        let mut attacker = make_pokemon(vec!["uturn".to_string()]);
        attacker.stats.atk = 200;
        let defender = make_pokemon(vec!["tackle".to_string()]);
        let mut state = BattleState::new(attacker, defender);
        let bench = Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [0; 6],
            crate::sim::stats::Nature::Hardy,
            vec!["tackle".to_string()],
            "Static",
            None,
        )
        .expect("species exists");
        state.bench_a.push(bench);
        let mut rng = SmallRng::seed_from_u64(5);

        execute_move_impl(&mut state, 0, 0, Action::Move(0), 1, &mut rng);

        assert_eq!(state.pokemon_a.species.to_ascii_lowercase(), "pikachu");
    }

    #[test]
    fn test_simultaneous_faints_trigger_double_replacement() {
        let mut a = make_pokemon(vec!["tackle".to_string()]);
        let mut b = make_pokemon(vec!["tackle".to_string()]);
        a.current_hp = 0;
        b.current_hp = 0;
        let mut state = BattleState::new(a, b);
        let bench_a = Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [0; 6],
            crate::sim::stats::Nature::Hardy,
            vec!["tackle".to_string()],
            "Static",
            None,
        )
        .expect("species exists");
        let bench_b = Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [0; 6],
            crate::sim::stats::Nature::Hardy,
            vec!["tackle".to_string()],
            "Static",
            None,
        )
        .expect("species exists");
        state.bench_a.push(bench_a);
        state.bench_b.push(bench_b);
        let mut rng = SmallRng::seed_from_u64(9);

        handle_simultaneous_faints(&mut state, &mut rng);

        assert!(!state.pokemon_a.is_fainted());
        assert!(!state.pokemon_b.is_fainted());
    }

    #[test]
    fn test_faint_detection() {
        let mut attacker = make_pokemon(vec!["thunderbolt".to_string()]);
        attacker.stats.spa = 200;
        attacker.stats.atk = 84;
        let mut defender = make_pokemon(vec!["flamethrower".to_string()]);
        defender.current_hp = 10;
        let mut state = BattleState::new(attacker, defender);
        let mut rng = SmallRng::seed_from_u64(1);
        execute_move_impl(&mut state, 0, 0, Action::Move(0), 1, &mut rng);
        assert!(state.pokemon_b.is_fainted());
    }

    #[test]
    fn test_speed_order() {
        let attacker = make_pokemon(vec!["thunderbolt".to_string()]);
        let mut faster = make_pokemon(vec!["flamethrower".to_string()]);
        faster.stats.spe = 200;
        let (a_first, _) = determine_order(
            &attacker,
            Action::Move(0),
            &faster,
            Action::Move(0),
            false,
            None,
            None,
            &mut SmallRng::seed_from_u64(0),
        );
        assert!(!a_first);
    }

    #[test]
    fn test_slow_start_speed_halves() {
        let mut slow = Pokemon::new(
            "charizard",
            50,
            [0; 6],
            [0; 6],
            crate::sim::stats::Nature::Hardy,
            vec!["tackle".to_string()],
            "Slow Start",
            None,
        )
        .expect("species exists");
        let mut normal = make_pokemon(vec!["tackle".to_string()]);
        slow.stats.spe = 100;
        normal.stats.spe = 80;
        let (a_first, _) = determine_order(
            &slow,
            Action::Move(0),
            &normal,
            Action::Move(0),
            false,
            None,
            None,
            &mut SmallRng::seed_from_u64(2),
        );
        assert!(!a_first);
    }

    #[test]
    fn test_quick_attack_priority() {
        let slower = make_pokemon(vec!["quickattack".to_string()]);
        let mut faster = make_pokemon(vec!["tackle".to_string()]);
        faster.stats.spe = 200;
        let (a_first, _) = determine_order(
            &slower,
            Action::Move(0),
            &faster,
            Action::Move(0),
            false,
            None,
            None,
            &mut SmallRng::seed_from_u64(1),
        );
        assert!(a_first);
    }

    #[test]
    fn test_substitute_creation_consumes_hp() {
        let user = make_pokemon(vec!["substitute".to_string()]);
        let opponent = make_pokemon(vec!["tackle".to_string()]);
        let max_hp = user.stats.hp;
        let mut state = BattleState::new(user, opponent);
        let mut rng = SmallRng::seed_from_u64(2);

        execute_move_impl(&mut state, 0, 0, Action::Move(0), 1, &mut rng);

        let expected = (max_hp as u32 / 4).max(1) as u16;
        assert_eq!(state.pokemon_a.substitute_hp, expected);
        assert_eq!(state.pokemon_a.current_hp, max_hp - expected);
    }

    #[test]
    fn test_substitute_blocks_damage() {
        let attacker = make_pokemon(vec!["seismictoss".to_string()]);
        let mut defender = make_pokemon(vec!["tackle".to_string()]);
        defender.substitute_hp = 10;
        let hp_before = defender.current_hp;
        let mut state = BattleState::new(attacker, defender);
        let mut rng = SmallRng::seed_from_u64(3);

        execute_move_impl(&mut state, 0, 0, Action::Move(0), 1, &mut rng);

        assert_eq!(state.pokemon_b.current_hp, hp_before);
        assert_eq!(state.pokemon_b.substitute_hp, 0);
    }

    #[test]
    fn test_sleep_turns_tick_down_at_end_of_turn() {
        let mut sleeper = make_pokemon(vec!["thunderbolt".to_string()]);
        sleeper.status = Some(Status::Sleep);
        sleeper.sleep_turns = 2;
        let opponent = make_pokemon(vec!["flamethrower".to_string()]);
        let mut state = BattleState::new(sleeper, opponent);
        let mut rng = SmallRng::seed_from_u64(7);

        apply_end_of_turn_effects(&mut state, &mut rng);
        assert_eq!(state.pokemon_a.sleep_turns, 1);
        assert!(matches!(state.pokemon_a.status, Some(Status::Sleep)));

        apply_end_of_turn_effects(&mut state, &mut rng);
        assert_eq!(state.pokemon_a.sleep_turns, 0);
        assert!(state.pokemon_a.status.is_none());
    }

    #[test]
    fn test_toxic_stage_resets_on_switch() {
        let mut pokemon = make_pokemon(vec!["thunderbolt".to_string()]);
        pokemon.status = Some(Status::Poison);
        pokemon.toxic_counter = 5;

        reset_on_switch(&mut pokemon);

        assert_eq!(pokemon.toxic_counter, 1);
        assert!(matches!(pokemon.status, Some(Status::Poison)));
    }

    #[test]
    fn test_battle_loop() {
        let base_moves = vec!["thunderbolt".to_string()];
        let battles = 10;
        for seed in 0..battles {
            let mut ai_a = RandomAI::new(seed);
            let mut ai_b = RandomAI::new(seed + 1);
            let pokemon_a = make_pokemon(base_moves.clone());
            let pokemon_b = make_pokemon(base_moves.clone());
            let result = run_battle(pokemon_a, pokemon_b, &mut ai_a, &mut ai_b);
            assert!(matches!(
                result,
                BattleResult::TeamAWins | BattleResult::TeamBWins | BattleResult::Draw
            ));
        }
    }
}

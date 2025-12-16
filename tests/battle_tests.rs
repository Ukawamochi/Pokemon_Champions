use pokemon_battle_matrix::battle::{
    compute_damage_preview, sample_accuracy_hits, simulate_battle, BattleResult,
};
use pokemon_battle_matrix::model::{Move, MoveCategory, Pokemon, Stats};
use std::collections::HashMap;

fn make_move(
    name: &str,
    move_type: &str,
    category: MoveCategory,
    power: u32,
    accuracy: f32,
    priority: i32,
) -> Move {
    Move {
        name: name.to_string(),
        move_type: move_type.to_string(),
        category,
        power,
        accuracy,
        priority,
        pp: 10,
        crit_rate: 0,
        secondary: None,
        recoil: None,
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
        extras: HashMap::new(),
    }
}

fn make_mon(name: &str, types: &[&str], speed: u32, mv: Move) -> Pokemon {
    Pokemon {
        name: name.to_string(),
        types: types.iter().map(|t| t.to_string()).collect(),
        stats: Stats {
            hp: 100,
            atk: 120,
            def: 80,
            spa: 120,
            spd: 80,
            spe: speed,
        },
        moves: vec![mv],
        item: None,
        ability: None,
        extras: HashMap::new(),
    }
}

#[test]
fn priority_beats_speed() {
    let priority_move = make_move(
        "Quick Blow",
        "normal",
        MoveCategory::Physical,
        200,
        100.0,
        1,
    );
    let normal_move = make_move(
        "Heavy Swing",
        "normal",
        MoveCategory::Physical,
        200,
        100.0,
        0,
    );
    let slow_with_priority = make_mon("Slowmon", &["normal"], 50, priority_move);
    let fast_no_priority = make_mon("Fastmon", &["normal"], 100, normal_move);
    let result = simulate_battle(&[slow_with_priority], &[fast_no_priority], 1);
    assert_eq!(result, BattleResult::AWins);
}

#[test]
fn speed_beats_lower_speed_when_priority_equal() {
    let strong_move = make_move("Strike", "normal", MoveCategory::Physical, 200, 100.0, 0);
    let fast = make_mon("Fast", &["normal"], 120, strong_move.clone());
    let slow = make_mon("Slow", &["normal"], 60, strong_move);
    let result = simulate_battle(&[fast], &[slow], 2);
    assert_eq!(result, BattleResult::AWins);
}

#[test]
fn speed_ties_use_rng() {
    let strong_move = make_move("Strike", "normal", MoveCategory::Physical, 200, 100.0, 0);
    let a = make_mon("MonoA", &["normal"], 80, strong_move.clone());
    let b = make_mon("MonoB", &["normal"], 80, strong_move);
    let mut a_wins = 0;
    let mut b_wins = 0;
    for seed in 0..10 {
        match simulate_battle(&[a.clone()], &[b.clone()], seed) {
            BattleResult::AWins => a_wins += 1,
            BattleResult::BWins => b_wins += 1,
            BattleResult::Tie => {}
        }
    }
    assert!(
        a_wins > 0 && b_wins > 0,
        "tie-breaker should allow either side to move first"
    );
}

#[test]
fn accuracy_rolls_respect_probability() {
    let shaky = make_move("Shaky", "normal", MoveCategory::Physical, 50, 50.0, 0);
    let hits = sample_accuracy_hits(&shaky, 42, 1000);
    let rate = hits as f64 / 1000.0;
    assert!(
        (rate - 0.5).abs() < 0.1,
        "expected hit rate near 0.5, got {rate}"
    );
}

#[test]
fn stab_and_type_effectiveness_affect_damage() {
    let fire_move = make_move("Flame", "fire", MoveCategory::Special, 90, 100.0, 0);
    let neutral_move = make_move("Neutral", "normal", MoveCategory::Special, 90, 100.0, 0);
    let attacker = make_mon("Blaze", &["fire"], 80, fire_move.clone());
    let target_grass = make_mon("Leafy", &["grass"], 80, neutral_move.clone());
    let target_water = make_mon("Splash", &["water"], 80, neutral_move.clone());

    let damage_fire_grass = compute_damage_preview(&attacker, &target_grass, &fire_move, 7);
    let damage_fire_water = compute_damage_preview(&attacker, &target_water, &fire_move, 7);
    let damage_neutral_grass = compute_damage_preview(&attacker, &target_grass, &neutral_move, 8);

    assert!(damage_fire_grass > damage_fire_water);
    assert!(damage_fire_grass > damage_neutral_grass);
}

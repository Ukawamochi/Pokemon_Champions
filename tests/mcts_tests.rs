use pokemon_battle_matrix::battle::{
    simulate_battle_with_options, Battle, BattleOptions, BattlePolicy, PlayerAction, Side,
    SimulationOptions,
};
use pokemon_battle_matrix::model::{Move, MoveCategory, Pokemon, Stats};
use pokemon_battle_matrix::{MctsMode, MctsParams};
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

fn make_mon(name: &str, types: &[&str], stats: Stats, moves: Vec<Move>) -> Pokemon {
    Pokemon {
        name: name.to_string(),
        types: types.iter().map(|t| t.to_string()).collect(),
        stats,
        moves,
        item: None,
        ability: None,
        extras: HashMap::new(),
    }
}

fn default_stats(hp: u32, spe: u32) -> Stats {
    Stats {
        hp,
        atk: 120,
        def: 80,
        spa: 120,
        spd: 80,
        spe,
    }
}

fn mcts_params(iterations: usize, horizon: usize) -> MctsParams {
    MctsParams {
        iterations: Some(iterations),
        time_budget: None,
        rollout_horizon: horizon,
        exploration_constant: 1.2,
        mode: MctsMode::Joint,
    }
}

#[test]
fn mcts_is_deterministic_for_same_seed() {
    let strong = make_move("Strike", "normal", MoveCategory::Physical, 90, 100.0, 0);
    let a = make_mon(
        "Alpha",
        &["normal"],
        default_stats(80, 80),
        vec![strong.clone()],
    );
    let b = make_mon("Beta", &["normal"], default_stats(80, 70), vec![strong]);
    let battle = Battle::new(&[a], &[b], 7);
    let params = mcts_params(50, 0);

    let action1 = pokemon_battle_matrix::mcts::mcts_action(&battle, Side::A, &params, 999);
    let action2 = pokemon_battle_matrix::mcts::mcts_action(&battle, Side::A, &params, 999);

    assert_eq!(action1, action2, "同じシードなら同じ手を選ぶべき");
}

#[test]
fn mcts_prefers_finishing_move() {
    // 相手HPが低いので、Move0を使えば即勝利になる局面を用意する
    let finisher = make_move("Finisher", "normal", MoveCategory::Physical, 200, 100.0, 0);
    let stall = make_move("Wait", "normal", MoveCategory::Status, 0, 100.0, 0);
    let attacker = make_mon(
        "Closer",
        &["normal"],
        default_stats(40, 80),
        vec![finisher.clone(), stall],
    );
    let target = make_mon("Foe", &["normal"], default_stats(20, 50), vec![finisher]);
    let battle = Battle::new(&[attacker], &[target], 11);
    let params = mcts_params(120, 1);

    let action = pokemon_battle_matrix::mcts::mcts_action(&battle, Side::A, &params, 1234);
    assert_eq!(
        action,
        Some(PlayerAction::Move(0)),
        "勝ち筋の技を優先してほしい"
    );
}

#[test]
fn mcts_outperforms_random_in_simple_matchup() {
    // Aは草に抜群の炎技(0)と弱いノーマル技(1)を持つ。MCTSなら安定して炎を選びやすい。
    let fire = make_move("Flame", "fire", MoveCategory::Special, 110, 100.0, 0);
    let weak = make_move("Tap", "normal", MoveCategory::Special, 30, 100.0, 0);
    let tackle = make_move("Tackle", "normal", MoveCategory::Physical, 60, 100.0, 0);

    let attacker = make_mon("Blaze", &["fire"], default_stats(90, 90), vec![fire, weak]);
    let defender = make_mon(
        "Sprout",
        &["grass"],
        default_stats(90, 80),
        vec![tackle.clone(), tackle],
    );

    let mut mcts_wins = 0;
    let mut random_wins = 0;
    let params = mcts_params(80, 4);
    let sim_mcts = SimulationOptions {
        policy_a: BattlePolicy::Mcts(params),
        policy_b: BattlePolicy::Random,
        battle: BattleOptions::default(),
    };
    let sim_random = SimulationOptions {
        policy_a: BattlePolicy::Random,
        policy_b: BattlePolicy::Random,
        battle: BattleOptions::default(),
    };

    for seed in 0..10 {
        let mcts_result =
            simulate_battle_with_options(&[attacker.clone()], &[defender.clone()], seed, &sim_mcts);
        if matches!(
            mcts_result,
            pokemon_battle_matrix::battle::BattleResult::AWins
        ) {
            mcts_wins += 1;
        }

        let random_result = simulate_battle_with_options(
            &[attacker.clone()],
            &[defender.clone()],
            seed,
            &sim_random,
        );
        if matches!(
            random_result,
            pokemon_battle_matrix::battle::BattleResult::AWins
        ) {
            random_wins += 1;
        }
    }

    assert!(
        mcts_wins >= random_wins && mcts_wins >= 6,
        "MCTS はランダムより多く勝つことを期待 (mcts={mcts_wins}, random={random_wins})"
    );
}

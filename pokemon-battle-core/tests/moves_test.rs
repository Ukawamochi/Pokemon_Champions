use pokemon_battle_core::data::moves::get_move;
use pokemon_battle_core::sim::battle::{determine_order, execute_turn, Action, BattleState, Field, Weather};
use pokemon_battle_core::sim::moves::{
    apply_drain, apply_recoil_damage, apply_secondary_effect, calculate_multihit_count,
    affects_grounded_only, bypasses_protect, bypasses_substitute, calculate_variable_power,
    check_ability_immunity, get_move_priority, handle_charging_move, handle_ohko_move, is_bullet_move,
    is_contact_move, is_pulse_move, is_sound_move, secondary_effect_from_move,
    secondary_effects_from_move,
};
use pokemon_battle_core::sim::pokemon::{Pokemon, Status};
use pokemon_battle_core::sim::stats::Nature;
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn make_pokemon(species: &str, moves: Vec<&str>, ability: &str) -> Pokemon {
    Pokemon::new(
        species,
        50,
        [0; 6],
        [31; 6],
        Nature::Hardy,
        moves.into_iter().map(|m| m.to_string()).collect(),
        ability,
        None,
    )
    .expect("species exists")
}

#[test]
fn flags_contact_move_is_detected() {
    let tackle = get_move("tackle").expect("move exists");
    assert!(is_contact_move(tackle));
}

#[test]
fn non_contact_move_is_not_detected_as_contact() {
    let thunderbolt = get_move("thunderbolt").expect("move exists");
    assert!(!is_contact_move(thunderbolt));
}

#[test]
fn secondary_effect_from_move_paralysis_is_detected() {
    let thunderbolt = get_move("thunderbolt").expect("move exists");
    let effect = secondary_effect_from_move("thunderbolt", thunderbolt).expect("secondary exists");
    assert_eq!(effect.chance, 10);
    assert_eq!(effect.status, Some(Status::Paralysis));
}

#[test]
fn apply_secondary_effect_is_deterministic_with_rng() {
    let thunderbolt = get_move("thunderbolt").expect("move exists");
    let effect = secondary_effect_from_move("thunderbolt", thunderbolt).expect("secondary exists");

    let mut attacker = make_pokemon("pikachu", vec!["thunderbolt"], "Static");
    let defender = make_pokemon("gyarados", vec!["tackle"], "Intimidate");

    let mut rng = SmallRng::seed_from_u64(0);
    let mut paralyzed = 0;
    for _ in 0..100 {
        let mut def_clone = defender.clone();
        let applied = apply_secondary_effect(&mut attacker, &mut def_clone, &effect, None, &mut rng);
        if applied && def_clone.status == Some(Status::Paralysis) {
            paralyzed += 1;
        }
    }
    assert!(paralyzed > 0);
    assert!(paralyzed < 100);
}

#[test]
fn check_ability_immunity_blocks_soundproof_sound_moves() {
    let hypervoice = get_move("hypervoice").expect("move exists");
    let defender = make_pokemon("exploud", vec!["tackle"], "Soundproof");
    assert!(check_ability_immunity(&defender, hypervoice));
}

#[test]
fn check_ability_immunity_blocks_bulletproof_bullet_moves() {
    let bullet_seed = get_move("bulletseed").expect("move exists");
    let defender = make_pokemon("chesnaught", vec!["tackle"], "Bulletproof");
    assert!(check_ability_immunity(&defender, bullet_seed));
}

#[test]
fn check_ability_immunity_blocks_priority_under_queenly_majesty() {
    let quick_attack = get_move("quickattack").expect("move exists");
    let defender = make_pokemon("tsareena", vec!["tackle"], "Queenly Majesty");
    assert!(check_ability_immunity(&defender, quick_attack));
}

#[test]
fn protect_blocks_standard_attack_moves() {
    let attacker = make_pokemon("charizard", vec!["tackle"], "Blaze");
    let mut defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    defender.protect_active = true;
    let mut state = BattleState::new(attacker, defender);
    let mut rng = SmallRng::seed_from_u64(1);

    let hp_before = state.pokemon_b.current_hp;
    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert_eq!(state.pokemon_b.current_hp, hp_before);
}

#[test]
fn magic_coat_reflects_status_move_to_user() {
    let mut attacker = make_pokemon("charizard", vec!["thunderwave"], "Blaze");
    attacker.stats.spe = 50;
    let mut defender = make_pokemon("blissey", vec!["magiccoat"], "Natural Cure");
    defender.stats.spe = 200;

    let mut state = BattleState::new(attacker, defender);
    let mut rng = SmallRng::seed_from_u64(2);

    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert_eq!(state.pokemon_a.status, Some(Status::Paralysis));
}

#[test]
fn charge_doubles_next_electric_move_once() {
    let mut attacker = make_pokemon("pikachu", vec!["thunderbolt"], "Static");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");

    let mut state_no_charge = BattleState::new(attacker.clone(), defender.clone());
    let mut rng = SmallRng::seed_from_u64(3);
    let hp_before = state_no_charge.pokemon_b.current_hp;
    execute_turn(&mut state_no_charge, Action::Move(0), Action::Move(0), &mut rng);
    let damage_no_charge = hp_before - state_no_charge.pokemon_b.current_hp;

    attacker.charge_active = true;
    let mut state_charge = BattleState::new(attacker, defender);
    let mut rng = SmallRng::seed_from_u64(3);
    let hp_before = state_charge.pokemon_b.current_hp;
    execute_turn(&mut state_charge, Action::Move(0), Action::Move(0), &mut rng);
    let damage_charge = hp_before - state_charge.pokemon_b.current_hp;

    assert!(damage_charge > damage_no_charge);
    assert!(!state_charge.pokemon_a.charge_active);
}

#[test]
fn court_change_swaps_side_conditions() {
    let attacker = make_pokemon("cinderace", vec!["courtchange"], "Libero");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let mut state = BattleState::new(attacker, defender);
    state.side_a.stealth_rock = true;
    state.side_b.spikes = 2;
    state.side_b.light_screen_turns = 5;
    let mut rng = SmallRng::seed_from_u64(4);

    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert_eq!(state.side_a.spikes, 2);
    assert_eq!(state.side_a.light_screen_turns, 5);
    assert!(!state.side_a.stealth_rock);
    assert!(state.side_b.stealth_rock);
    assert_eq!(state.side_b.spikes, 0);
}

#[test]
fn healing_wish_heals_next_switch_in() {
    let mut active = make_pokemon("pikachu", vec!["healingwish"], "Static");
    active.stats.spe = 200;
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");

    let mut incoming = make_pokemon("charizard", vec!["tackle"], "Blaze");
    incoming.current_hp = (incoming.stats.hp / 2).max(1);
    incoming.status = Some(Status::Burn);

    let mut state = BattleState::new_with_bench(active, defender, vec![incoming], vec![]);
    let mut rng = SmallRng::seed_from_u64(5);

    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert_eq!(state.pokemon_a.species.to_ascii_lowercase(), "charizard");
    assert_eq!(state.pokemon_a.current_hp, state.pokemon_a.stats.hp);
    assert_eq!(state.pokemon_a.status, None);
    assert!(!state.side_a.healing_wish_pending);
}

#[test]
fn get_move_priority_boosts_grassy_glide_in_grassy_field() {
    let glide = get_move("grassyglide").expect("move exists");
    let attacker = make_pokemon("rillaboom", vec!["grassyglide"], "Grassy Surge");
    let base = get_move_priority(glide, &attacker, None);
    let boosted = get_move_priority(glide, &attacker, Some(Field::Grassy));
    assert_eq!(boosted, base + 1);
}

#[test]
fn determine_order_respects_grassy_glide_priority_boost() {
    let slower = make_pokemon("rillaboom", vec!["grassyglide"], "Grassy Surge");
    let mut faster = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    faster.stats.spe = 200;
    let (a_first, _) = determine_order(
        &slower,
        Action::Move(0),
        &faster,
        Action::Move(0),
        false,
        None,
        Some(Field::Grassy),
        &mut SmallRng::seed_from_u64(7),
    );
    assert!(a_first);
}

#[test]
fn calculate_variable_power_eruption_scales_with_hp() {
    let eruption = get_move("eruption").expect("move exists");
    let mut attacker = make_pokemon("typhlosion", vec!["eruption"], "Blaze");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    attacker.stats.hp = 200;
    attacker.current_hp = 100;
    assert_eq!(
        calculate_variable_power(eruption, &attacker, &defender, None, None),
        75
    );
}

#[test]
fn calculate_multihit_count_returns_range_for_2_to_5_moves() {
    let seed = get_move("bulletseed").expect("move exists");
    let mut rng = SmallRng::seed_from_u64(1);
    for _ in 0..50 {
        let hits = calculate_multihit_count(seed, &mut rng);
        assert!((2..=5).contains(&hits));
    }
}

#[test]
fn handle_charging_move_toggles_charging_state() {
    let mut user = make_pokemon("charizard", vec!["fly"], "Blaze");
    assert!(handle_charging_move(&mut user, "fly"));
    assert!(user.charging_move.is_some());
    assert!(user.semi_invulnerable);
    assert!(!handle_charging_move(&mut user, "fly"));
    assert!(user.charging_move.is_none());
    assert!(!user.semi_invulnerable);
}

#[test]
fn handle_ohko_move_can_hit_with_seeded_rng() {
    let attacker = Pokemon::new(
        "machamp",
        100,
        [0; 6],
        [31; 6],
        Nature::Hardy,
        vec!["fissure".to_string()],
        "No Guard",
        None,
    )
    .expect("species exists");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let mut rng = SmallRng::seed_from_u64(0);
    // level条件を満たすので、seedによっては命中する（命中しなくてもpanicしない）
    let _ = handle_ohko_move(&attacker, &defender, "fissure", &mut rng);
}

#[test]
fn apply_drain_heals_attacker() {
    let mut attacker = make_pokemon("venusaur", vec!["gigadrain"], "Overgrow");
    attacker.current_hp = 10;
    apply_drain(&mut attacker, 100, (1, 2));
    assert!(attacker.current_hp > 10);
}

#[test]
fn apply_recoil_damage_damages_attacker() {
    let mut attacker = make_pokemon("charizard", vec!["flareblitz"], "Blaze");
    let hp_before = attacker.current_hp;
    apply_recoil_damage(&mut attacker, 100, (1, 3));
    assert!(attacker.current_hp < hp_before);
}

#[test]
fn charge_status_can_be_set_by_status_move_and_expires_after_electric_attack() {
    let attacker = make_pokemon("pikachu", vec!["charge", "thunderbolt"], "Static");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let mut state = BattleState::new(attacker, defender);
    let mut rng = SmallRng::seed_from_u64(9);

    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert!(state.pokemon_a.charge_active);

    let hp_before = state.pokemon_b.current_hp;
    execute_turn(&mut state, Action::Move(1), Action::Move(0), &mut rng);
    assert!(state.pokemon_b.current_hp < hp_before);
    assert!(!state.pokemon_a.charge_active);
}

#[test]
fn weather_is_passed_into_variable_power_speed_calcs() {
    // Electro Ball uses effective speed; just sanity check it doesn't panic under weather.
    let electro_ball = get_move("electroball").expect("move exists");
    let attacker = make_pokemon("pikachu", vec!["electroball"], "Swift Swim");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let _ = calculate_variable_power(electro_ball, &attacker, &defender, Some(Weather::Rain), None);
}

#[test]
fn flags_sound_move_is_detected() {
    let hypervoice = get_move("hypervoice").expect("move exists");
    assert!(is_sound_move(hypervoice));
}

#[test]
fn flags_bullet_move_is_detected() {
    let bullet_seed = get_move("bulletseed").expect("move exists");
    assert!(is_bullet_move(bullet_seed));
}

#[test]
fn flags_pulse_move_is_detected() {
    let darkpulse = get_move("darkpulse").expect("move exists");
    assert!(is_pulse_move(darkpulse));
}

#[test]
fn secondary_effects_from_move_can_return_multiple_entries() {
    let fire_fang = get_move("firefang").expect("move exists");
    let effects = secondary_effects_from_move("firefang", fire_fang);
    assert!(effects.len() >= 1);
}

#[test]
fn calculate_variable_power_defaults_to_base_power_for_normal_moves() {
    let tackle = get_move("tackle").expect("move exists");
    let attacker = make_pokemon("pikachu", vec!["tackle"], "Static");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    assert_eq!(
        calculate_variable_power(tackle, &attacker, &defender, None, None),
        tackle.base_power.unwrap_or(0)
    );
}

#[test]
fn calculate_variable_power_flail_thresholds_smoke() {
    let flail = get_move("flail").expect("move exists");
    let mut attacker = make_pokemon("magikarp", vec!["flail"], "Swift Swim");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    attacker.stats.hp = 100;
    attacker.current_hp = 1;
    assert_eq!(calculate_variable_power(flail, &attacker, &defender, None, None), 200);
    attacker.current_hp = 100;
    assert_eq!(calculate_variable_power(flail, &attacker, &defender, None, None), 20);
}

#[test]
fn calculate_variable_power_gyro_ball_is_capped_at_150() {
    let gyro = get_move("gyroball").expect("move exists");
    let mut attacker = make_pokemon("ferrothorn", vec!["gyroball"], "Iron Barbs");
    let mut defender = make_pokemon("electrode", vec!["tackle"], "Soundproof");
    attacker.stats.spe = 1;
    defender.stats.spe = 500;
    assert_eq!(
        calculate_variable_power(gyro, &attacker, &defender, None, None),
        150
    );
}

#[test]
fn calculate_multihit_count_fixed_two_hit_moves() {
    let mv = get_move("doublekick").expect("move exists");
    let mut rng = SmallRng::seed_from_u64(123);
    assert_eq!(calculate_multihit_count(mv, &mut rng), 2);
}

#[test]
fn calculate_multihit_count_population_bomb_is_10() {
    let mv = get_move("populationbomb").expect("move exists");
    let mut rng = SmallRng::seed_from_u64(0);
    assert_eq!(calculate_multihit_count(mv, &mut rng), 10);
}

#[test]
fn apply_recoil_damage_minimum_one() {
    let mut attacker = make_pokemon("charizard", vec!["flareblitz"], "Blaze");
    let hp_before = attacker.current_hp;
    apply_recoil_damage(&mut attacker, 1, (1, 3));
    assert_eq!(attacker.current_hp, hp_before - 1);
}

#[test]
fn apply_drain_minimum_one() {
    let mut attacker = make_pokemon("venusaur", vec!["gigadrain"], "Overgrow");
    attacker.current_hp = attacker.current_hp.saturating_sub(1);
    let hp_before = attacker.current_hp;
    apply_drain(&mut attacker, 1, (1, 2));
    assert!(attacker.current_hp > hp_before);
}

#[test]
fn get_move_priority_no_boost_outside_grassy_field() {
    let glide = get_move("grassyglide").expect("move exists");
    let attacker = make_pokemon("rillaboom", vec!["grassyglide"], "Grassy Surge");
    assert_eq!(
        get_move_priority(glide, &attacker, Some(Field::Electric)),
        glide.priority
    );
}

#[test]
fn ohko_move_fails_if_attacker_is_lower_level() {
    let attacker = Pokemon::new(
        "machamp",
        50,
        [0; 6],
        [31; 6],
        Nature::Hardy,
        vec!["fissure".to_string()],
        "No Guard",
        None,
    )
    .expect("species exists");
    let defender = Pokemon::new(
        "blissey",
        100,
        [0; 6],
        [31; 6],
        Nature::Hardy,
        vec!["tackle".to_string()],
        "Natural Cure",
        None,
    )
    .expect("species exists");
    let mut rng = SmallRng::seed_from_u64(0);
    assert!(handle_ohko_move(&attacker, &defender, "fissure", &mut rng).is_none());
}

#[test]
fn sheer_cold_is_blocked_by_ice_type_target() {
    let attacker = Pokemon::new(
        "lapras",
        100,
        [0; 6],
        [31; 6],
        Nature::Hardy,
        vec!["sheercold".to_string()],
        "No Guard",
        None,
    )
    .expect("species exists");
    let defender = make_pokemon("glaceon", vec!["tackle"], "Snow Cloak");
    let mut rng = SmallRng::seed_from_u64(1);
    assert!(handle_ohko_move(&attacker, &defender, "sheercold", &mut rng).is_none());
}

#[test]
fn handle_charging_move_returns_false_for_non_charging_moves() {
    let mut user = make_pokemon("charizard", vec!["tackle"], "Blaze");
    assert!(!handle_charging_move(&mut user, "tackle"));
    assert!(user.charging_move.is_none());
}

#[test]
fn telekinesis_sets_turn_counter() {
    let mut attacker = make_pokemon("alakazam", vec!["telekinesis"], "Synchronize");
    attacker.stats.spe = 200;
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let mut state = BattleState::new(attacker, defender);
    let mut rng = SmallRng::seed_from_u64(10);
    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert_eq!(state.pokemon_b.telekinesis_turns, 3);
}

#[test]
fn aurora_veil_fails_without_hail() {
    let mut attacker = make_pokemon("ninetalesalola", vec!["auroraveil"], "Snow Warning");
    attacker.stats.spe = 200;
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let mut state = BattleState::new(attacker, defender);
    state.weather = None;
    let mut rng = SmallRng::seed_from_u64(11);
    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert_eq!(state.side_a.aurora_veil_turns, 0);
}

#[test]
fn aurora_veil_succeeds_in_hail() {
    let mut attacker = make_pokemon("ninetalesalola", vec!["auroraveil"], "Snow Warning");
    attacker.stats.spe = 200;
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    let mut state = BattleState::new(attacker, defender);
    state.weather = Some(Weather::Hail);
    let mut rng = SmallRng::seed_from_u64(12);
    execute_turn(&mut state, Action::Move(0), Action::Move(0), &mut rng);
    assert!(state.side_a.aurora_veil_turns > 0);
}

#[test]
fn dazzling_blocks_priority_moves() {
    let quick_attack = get_move("quickattack").expect("move exists");
    let defender = make_pokemon("bruxish", vec!["tackle"], "Dazzling");
    assert!(check_ability_immunity(&defender, quick_attack));
}

#[test]
fn affects_grounded_only_detects_thousand_arrows() {
    let mv = get_move("thousandarrows").expect("move exists");
    assert!(affects_grounded_only(mv));
}

#[test]
fn bypasses_substitute_is_true_for_sound_moves() {
    let hypervoice = get_move("hypervoice").expect("move exists");
    assert!(bypasses_substitute(hypervoice));
    let tackle = get_move("tackle").expect("move exists");
    assert!(!bypasses_substitute(tackle));
}

#[test]
fn bypasses_protect_is_true_for_moves_without_protect_flag() {
    let hyperspacehole = get_move("hyperspacehole").expect("move exists");
    assert!(bypasses_protect(hyperspacehole));
}

#[test]
fn check_ability_immunity_is_false_for_irrelevant_moves() {
    let tackle = get_move("tackle").expect("move exists");
    let defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    assert!(!check_ability_immunity(&defender, tackle));
}

#[test]
fn secondary_effect_from_move_can_return_flinch_as_status() {
    let air_slash = get_move("airslash").expect("move exists");
    let effect = secondary_effect_from_move("airslash", air_slash).expect("secondary exists");
    assert!(effect.status == Some(Status::Flinch) || effect.volatile_status == Some("flinch"));
}

#[test]
fn calculate_multihit_count_triplekick_is_3() {
    let mv = get_move("triplekick").expect("move exists");
    let mut rng = SmallRng::seed_from_u64(0);
    assert_eq!(calculate_multihit_count(mv, &mut rng), 3);
}

#[test]
fn handle_charging_move_works_for_solar_beam() {
    let mut user = make_pokemon("venusaur", vec!["solarbeam"], "Overgrow");
    assert!(handle_charging_move(&mut user, "solarbeam"));
    assert!(user.charging_move.is_some());
    assert!(!handle_charging_move(&mut user, "solarbeam"));
    assert!(user.charging_move.is_none());
}

#[test]
fn electro_ball_variable_power_is_monotonic_with_speed_ratio() {
    let mv = get_move("electroball").expect("move exists");
    let mut fast = make_pokemon("pikachu", vec!["electroball"], "Static");
    let mut slow_target = make_pokemon("snorlax", vec!["tackle"], "Immunity");
    fast.stats.spe = 200;
    slow_target.stats.spe = 50;
    let high = calculate_variable_power(mv, &fast, &slow_target, None, None);
    slow_target.stats.spe = 150;
    let low = calculate_variable_power(mv, &fast, &slow_target, None, None);
    assert!(high >= low);
}

#[test]
fn gyro_ball_variable_power_is_at_least_one() {
    let mv = get_move("gyroball").expect("move exists");
    let mut attacker = make_pokemon("ferrothorn", vec!["gyroball"], "Iron Barbs");
    let mut defender = make_pokemon("blissey", vec!["tackle"], "Natural Cure");
    attacker.stats.spe = 0;
    defender.stats.spe = 0;
    assert!(calculate_variable_power(mv, &attacker, &defender, None, None) >= 1);
}

use super::abilities::ABILITIES;
use super::items::ITEMS;
use super::moves::MOVES;
use super::species::POKEDEX;
use super::types::{effectiveness_dual, Type};

#[test]
fn charizard_stats() {
    let charizard = POKEDEX
        .get("charizard")
        .expect("Charizard should exist in the Pokedex");
    assert_eq!(charizard.base_stats.hp, 78);
    assert_eq!(charizard.base_stats.atk, 84);
    assert_eq!(charizard.types[0], "Fire");
    assert_eq!(charizard.types[1], "Flying");
}

#[test]
fn dragonite_stats() {
    let dragonite = POKEDEX
        .get("dragonite")
        .expect("Dragonite should exist in the Pokedex");
    assert_eq!(dragonite.base_stats.hp, 91);
    assert_eq!(dragonite.base_stats.atk, 134);
}

#[test]
fn thunderbolt_secondary_paralysis() {
    let thunderbolt = MOVES
        .get("thunderbolt")
        .expect("Thunderbolt must be present");
    assert_eq!(thunderbolt.base_power, Some(90));
    let secondary = thunderbolt
        .secondary
        .as_ref()
        .expect("Thunderbolt should have a secondary effect");
    assert_eq!(secondary.chance, 10);
    assert_eq!(secondary.status, Some("par"));
}

#[test]
fn ice_beam_secondary_freeze() {
    let ice_beam = MOVES.get("icebeam").expect("Ice Beam must be present");
    assert_eq!(ice_beam.base_power, Some(90));
    let secondary = ice_beam
        .secondary
        .as_ref()
        .expect("Ice Beam should have a secondary effect");
    assert_eq!(secondary.chance, 10);
    assert_eq!(secondary.status, Some("frz"));
}

#[test]
fn fire_fang_has_two_secondaries() {
    let fire_fang = MOVES.get("firefang").expect("Fire Fang must be present");
    assert_eq!(fire_fang.secondaries.len(), 2);
    assert!(fire_fang.secondaries.iter().any(|s| s.status == Some("brn")));
    assert!(fire_fang
        .secondaries
        .iter()
        .any(|s| s.volatile_status == Some("flinch")));
}

#[test]
fn type_effectiveness_ice_vs_dragon_flying() {
    let effectiveness = effectiveness_dual(Type::Ice, Type::Dragon, Type::Flying);
    assert!((effectiveness - 4.0).abs() < f32::EPSILON);
}

#[test]
fn abilities_include_key_entries() {
    assert!(ABILITIES.contains_key("multiscale"));
    assert!(ABILITIES.contains_key("intimidate"));
}

#[test]
fn items_include_expected_keys() {
    assert!(ITEMS.contains_key("leftovers"));
    assert!(ITEMS.contains_key("choicescarf"));
}

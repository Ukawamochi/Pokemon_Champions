use crate::data::types::Type;
use crate::data::moves::MoveData;
use crate::sim::pokemon::Pokemon;

#[derive(Clone, Copy, Debug)]
pub struct DamageModifiers {
    pub weather: f32,
    pub crit: f32,
    pub burn: f32,
    pub final_modifier: f32,
}

impl Default for DamageModifiers {
    fn default() -> Self {
        Self {
            weather: 1.0,
            crit: 1.0,
            burn: 1.0,
            final_modifier: 1.0,
        }
    }
}

pub(crate) fn chain_modifier(previous: f32, next: f32) -> f32 {
    // Showdown: battle.ts#L2272-L2287 (chain)
    let previous = (previous * 4096.0).floor() as u64;
    let next = (next * 4096.0).floor() as u64;
    let chained = (previous * next + 2048) >> 12;
    chained as f32 / 4096.0
}

fn apply_modifier(value: u32, modifier: f32) -> u32 {
    // Showdown: battle.ts#L2302-L2313 (modify)
    if modifier <= 0.0 {
        return 0;
    }
    let modifier = (modifier * 4096.0).floor() as u64;
    let value = value as u64;
    ((value * modifier + 2048 - 1) / 4096) as u32
}

fn apply_random_factor(value: u32, random_factor: f32) -> u32 {
    // Showdown: battle.ts#L2354-L2359 (randomizer)
    let mut percent = (random_factor * 100.0).round() as i32;
    percent = percent.clamp(85, 100);
    value.saturating_mul(percent as u32) / 100
}

fn type_effectiveness_steps(type_effectiveness: f32) -> i8 {
    if type_effectiveness == 4.0 {
        2
    } else if type_effectiveness == 2.0 {
        1
    } else if type_effectiveness == 1.0 {
        0
    } else if type_effectiveness == 0.5 {
        -1
    } else if type_effectiveness == 0.25 {
        -2
    } else {
        (type_effectiveness.ln() / 2.0_f32.ln()).round() as i8
    }
}

fn apply_type_effectiveness(value: u32, type_effectiveness: f32) -> u32 {
    if type_effectiveness == 0.0 {
        return 0;
    }
    let steps = type_effectiveness_steps(type_effectiveness);
    if steps > 0 {
        value.saturating_mul(1u32 << steps as u32)
    } else if steps < 0 {
        value / (1u32 << (-steps) as u32)
    } else {
        value
    }
}

fn compute_base_damage(
    attacker_level: u8,
    attacker_stat: u16,
    defender_stat: u16,
    move_power: u16,
) -> u32 {
    // Showdown: battle-actions.ts#L1715-L1716 (baseDamage)
    let level = attacker_level as u32;
    let attack = attacker_stat as u32;
    let defense = defender_stat.max(1) as u32;
    let base_power = move_power as u32;
    let mut base_damage = 2 * level / 5 + 2;
    base_damage = base_damage.saturating_mul(base_power);
    base_damage = base_damage.saturating_mul(attack);
    base_damage /= defense;
    base_damage /= 50;
    base_damage
}

pub fn calculate_damage_with_modifiers(
    attacker_level: u8,
    attacker_atk_or_spa: u16,
    defender_def_or_spd: u16,
    move_power: u16,
    type_effectiveness: f32,
    stab: bool,
    random_factor: f32,
    modifiers: DamageModifiers,
) -> u16 {
    if type_effectiveness == 0.0 {
        return 0;
    }
    let mut base_damage = compute_base_damage(
        attacker_level,
        attacker_atk_or_spa,
        defender_def_or_spd,
        move_power,
    );
    // Showdown: battle-actions.ts#L1729
    base_damage = base_damage.saturating_add(2);
    // Showdown: battle-actions.ts#L1743-L1744
    base_damage = apply_modifier(base_damage, modifiers.weather);
    // Showdown: battle-actions.ts#L1746-L1749
    if (modifiers.crit - 1.0).abs() > f32::EPSILON {
        base_damage = ((base_damage as f32) * modifiers.crit).floor() as u32;
    }
    // Showdown: battle-actions.ts#L1752-L1753
    base_damage = apply_random_factor(base_damage, random_factor);
    // Showdown: battle-actions.ts#L1755-L1791
    if stab {
        base_damage = apply_modifier(base_damage, 1.5);
    }
    // Showdown: battle-actions.ts#L1793-L1809
    base_damage = apply_type_effectiveness(base_damage, type_effectiveness);
    // Showdown: battle-actions.ts#L1814-L1817
    base_damage = apply_modifier(base_damage, modifiers.burn);
    // Showdown: battle-actions.ts#L1823-L1824
    base_damage = apply_modifier(base_damage, modifiers.final_modifier);
    // Showdown: battle-actions.ts#L1831-L1835
    if base_damage == 0 {
        return 1;
    }
    (base_damage & 0xFFFF) as u16
}

pub fn calculate_damage(
    attacker_level: u8,
    attacker_atk_or_spa: u16,
    defender_def_or_spd: u16,
    move_power: u16,
    type_effectiveness: f32,
    stab: bool,
    random_factor: f32,
    other_modifiers: f32,
) -> u16 {
    calculate_damage_with_modifiers(
        attacker_level,
        attacker_atk_or_spa,
        defender_def_or_spd,
        move_power,
        type_effectiveness,
        stab,
        random_factor,
        DamageModifiers {
            final_modifier: other_modifiers,
            ..DamageModifiers::default()
        },
    )
}

pub fn damage_range(base_damage: u16) -> Vec<u16> {
    (0..16)
        .map(|i| {
            let factor = 0.85 + (i as f32) * 0.01;
            let value = (base_damage as f32 * factor).floor();
            value as u16
        })
        .collect()
}

pub fn is_stab(move_type: Type, pokemon_types: [Type; 2]) -> bool {
    pokemon_types.iter().any(|t| *t == move_type)
}

pub fn ability_attack_modifier(
    pokemon: &Pokemon,
    move_data: &MoveData,
    move_type: Type,
    is_sandstorm: bool,
) -> f32 {
    crate::sim::abilities::damage_modifiers::attacker_damage_modifier(
        pokemon,
        move_data,
        move_type,
        is_sandstorm,
    )
}

pub fn ability_defense_modifier(pokemon: &Pokemon, move_data: &MoveData, type_effectiveness: f32) -> f32 {
    crate::sim::abilities::damage_modifiers::defender_damage_modifier(pokemon, move_data, type_effectiveness)
}

pub fn item_type_boost(item: &str, move_type: Type) -> f32 {
    crate::sim::items::type_items::item_type_boost(item, move_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::types::{effectiveness_against, effectiveness_dual, Type};
    use crate::sim::stats::{Nature, StatsSet};
    use crate::data::moves::get_move;
    use crate::sim::pokemon::{Pokemon, Status};

    #[test]
    fn test_damage_calc_thunderbolt() {
        let damage = calculate_damage(50, 120, 100, 90, 1.0, false, 1.0, 1.0);
        assert_eq!(damage, 49);
    }

    #[test]
    fn test_stab_bonus() {
        let base = calculate_damage(50, 120, 100, 90, 1.0, false, 1.0, 1.0);
        let with_stab = calculate_damage(50, 120, 100, 90, 1.0, true, 1.0, 1.0);
        assert_eq!(with_stab, 73);
        assert!(with_stab > base);
    }

    #[test]
    fn test_type_effectiveness() {
        let base = calculate_damage(50, 120, 100, 90, 1.0, false, 1.0, 1.0);
        let doubled = calculate_damage(50, 120, 100, 90, 2.0, false, 1.0, 1.0);
        assert_eq!(doubled, base * 2);
    }

    #[test]
    fn test_damage_range() {
        let range = damage_range(200);
        assert_eq!(range.len(), 16);
        assert_eq!(range[0], 170); // 200 * 0.85
        assert_eq!(range[15], 200);
        assert_eq!(range[1] - range[0], 2);
    }

    #[test]
    fn test_is_stab() {
        let types = [Type::Electric, Type::Flying];
        assert!(is_stab(Type::Electric, types));
        assert!(!is_stab(Type::Fire, types));
    }

    #[test]
    fn test_effectiveness_against() {
        assert_eq!(
            effectiveness_against(Type::Ice, Type::Dragon),
            2.0
        );
    }

    #[test]
    fn test_showdown_damage_garchomp_earthquake_heatran() {
        let evs = [0; 6];
        let ivs = [31; 6];
        let attacker = StatsSet::from_species("garchomp", 50, evs, ivs, Nature::Hardy)
            .expect("garchomp stats");
        let defender = StatsSet::from_species("heatran", 50, evs, ivs, Nature::Hardy)
            .expect("heatran stats");
        let type_effectiveness = effectiveness_dual(Type::Ground, Type::Fire, Type::Steel);
        let max_damage = calculate_damage_with_modifiers(
            50,
            attacker.atk,
            defender.def,
            100,
            type_effectiveness,
            true,
            1.0,
            DamageModifiers::default(),
        );
        let min_damage = calculate_damage_with_modifiers(
            50,
            attacker.atk,
            defender.def,
            100,
            type_effectiveness,
            true,
            0.85,
            DamageModifiers::default(),
        );
        assert_eq!(max_damage, 324);
        assert_eq!(min_damage, 268);
    }

    #[test]
    fn test_showdown_damage_pikachu_thunderbolt_gyarados() {
        let evs = [0; 6];
        let ivs = [31; 6];
        let attacker = StatsSet::from_species("pikachu", 100, evs, ivs, Nature::Hardy)
            .expect("pikachu stats");
        let defender = StatsSet::from_species("gyarados", 100, evs, ivs, Nature::Hardy)
            .expect("gyarados stats");
        let type_effectiveness = effectiveness_dual(Type::Electric, Type::Water, Type::Flying);
        let max_damage = calculate_damage_with_modifiers(
            100,
            attacker.spa,
            defender.spd,
            90,
            type_effectiveness,
            true,
            1.0,
            DamageModifiers::default(),
        );
        let min_damage = calculate_damage_with_modifiers(
            100,
            attacker.spa,
            defender.spd,
            90,
            type_effectiveness,
            true,
            0.85,
            DamageModifiers::default(),
        );
        assert_eq!(max_damage, 268);
        assert_eq!(min_damage, 228);
    }

    fn make_test_pokemon(ability: &str) -> Pokemon {
        Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec!["tackle".to_string()],
            ability,
            None,
        )
        .expect("pikachu")
    }

    #[test]
    fn test_ability_attack_modifier_huge_power() {
        let attacker = make_test_pokemon("Huge Power");
        let tackle = get_move("tackle").expect("tackle");
        assert_eq!(
            ability_attack_modifier(&attacker, &tackle, Type::Normal, false),
            2.0
        );
    }

    #[test]
    fn test_ability_attack_modifier_iron_fist_punch() {
        let attacker = make_test_pokemon("Iron Fist");
        let move_data = get_move("firepunch").expect("firepunch");
        let modifier = ability_attack_modifier(&attacker, &move_data, Type::Fire, false);
        assert!((modifier - 1.2).abs() < 1e-6);
    }

    #[test]
    fn test_ability_attack_modifier_guts_burn() {
        let mut attacker = make_test_pokemon("Guts");
        attacker.status = Some(Status::Burn);
        let tackle = get_move("tackle").expect("tackle");
        assert_eq!(
            ability_attack_modifier(&attacker, &tackle, Type::Normal, false),
            1.5
        );
    }

    #[test]
    fn test_ability_defense_modifier_filter_super_effective() {
        let defender = make_test_pokemon("Filter");
        let move_data = get_move("tackle").expect("tackle");
        assert_eq!(
            ability_defense_modifier(&defender, &move_data, 2.0),
            0.75
        );
    }
}

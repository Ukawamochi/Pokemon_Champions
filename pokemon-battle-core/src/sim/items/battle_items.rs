use crate::data::moves::MoveCategory;
use crate::data::types::Type;
use crate::sim::pokemon::Pokemon;

// Showdown reference:
// - Life Orb / recoil application: pokemon-showdown/sim/battle-actions.ts#L983-L999
// - Expert Belt boost: pokemon-showdown/data/items.ts (expertbelt)
// - Choice items: pokemon-showdown/data/items.ts (choiceband/choicespecs/choicescarf)
// - Black Sludge: pokemon-showdown/data/items.ts (blacksludge)

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndOfTurnEffect {
    Heal { amount: u16, item_id: &'static str },
    Damage { amount: u16, item_id: &'static str },
}

pub fn is_choice_item_id(item_id: &str) -> bool {
    matches!(item_id, "choiceband" | "choicespecs" | "choicescarf")
}

pub fn speed_modifier(pokemon: &Pokemon) -> f32 {
    if pokemon.item_consumed {
        return 1.0;
    }
    match normalized_item_id(pokemon).as_deref() {
        Some("choicescarf") => 1.5,
        _ => 1.0,
    }
}

pub fn attack_stat_modifier(pokemon: &Pokemon, category: MoveCategory) -> f32 {
    if pokemon.item_consumed {
        return 1.0;
    }
    match (normalized_item_id(pokemon).as_deref(), category) {
        (Some("choiceband"), MoveCategory::Physical) => 1.5,
        (Some("choicespecs"), MoveCategory::Special) => 1.5,
        _ => 1.0,
    }
}

pub fn base_power_modifier(pokemon: &Pokemon, type_effectiveness: f32) -> f32 {
    if pokemon.item_consumed {
        return 1.0;
    }
    match normalized_item_id(pokemon).as_deref() {
        Some("lifeorb") => 1.3,
        Some("expertbelt") if type_effectiveness > 1.0 => 1.2,
        _ => 1.0,
    }
}

pub fn end_of_turn_effect(pokemon: &Pokemon) -> Option<EndOfTurnEffect> {
    if pokemon.item_consumed {
        return None;
    }
    let max_hp = pokemon.stats.hp;
    let current_hp = pokemon.current_hp;
    let item_id = normalized_item_id(pokemon)?;
    match item_id.as_str() {
        "leftovers" => {
            if current_hp >= max_hp {
                return None;
            }
            let amount = (max_hp as u32 / 16).max(1) as u16;
            Some(EndOfTurnEffect::Heal {
                amount,
                item_id: "leftovers",
            })
        }
        "blacksludge" => {
            let is_poison = pokemon.types[0] == Type::Poison || pokemon.types[1] == Type::Poison;
            if is_poison {
                if current_hp >= max_hp {
                    return None;
                }
                let amount = (max_hp as u32 / 16).max(1) as u16;
                Some(EndOfTurnEffect::Heal {
                    amount,
                    item_id: "blacksludge",
                })
            } else {
                let amount = (max_hp as u32 / 8).max(1) as u16;
                Some(EndOfTurnEffect::Damage {
                    amount,
                    item_id: "blacksludge",
                })
            }
        }
        _ => None,
    }
}

pub fn should_lock_choice_move(pokemon: &Pokemon) -> bool {
    !pokemon.item_consumed
        && normalized_item_id(pokemon).as_deref().is_some_and(is_choice_item_id)
        && pokemon.choice_lock_move.is_none()
}

pub fn set_choice_lock_move(pokemon: &mut Pokemon, move_id: &str) {
    if pokemon.item_consumed {
        return;
    }
    let Some(item_id) = normalized_item_id(pokemon) else {
        return;
    };
    if !is_choice_item_id(item_id.as_str()) {
        return;
    }
    if pokemon.choice_lock_move.is_none() {
        pokemon.choice_lock_move = Some(move_id.to_string());
    }
}

pub fn clear_choice_lock(pokemon: &mut Pokemon) {
    pokemon.choice_lock_move = None;
}

fn normalized_item_id(pokemon: &Pokemon) -> Option<String> {
    pokemon.item.as_deref().map(normalize_item_name)
}

fn normalize_item_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::stats::Nature;

    fn mk_pokemon(item: Option<&str>) -> Pokemon {
        Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec!["tackle".to_string()],
            "Static",
            item.map(|s| s.to_string()),
        )
        .unwrap()
    }

    #[test]
    fn choice_scarf_speed_modifier() {
        let mut p = mk_pokemon(Some("Choice Scarf"));
        assert_eq!(speed_modifier(&p), 1.5);
        p.item_consumed = true;
        assert_eq!(speed_modifier(&p), 1.0);
    }

    #[test]
    fn choice_band_specs_attack_modifier() {
        let p_band = mk_pokemon(Some("Choice Band"));
        assert_eq!(attack_stat_modifier(&p_band, MoveCategory::Physical), 1.5);
        assert_eq!(attack_stat_modifier(&p_band, MoveCategory::Special), 1.0);
        let p_specs = mk_pokemon(Some("Choice Specs"));
        assert_eq!(attack_stat_modifier(&p_specs, MoveCategory::Special), 1.5);
        assert_eq!(attack_stat_modifier(&p_specs, MoveCategory::Physical), 1.0);
    }

    #[test]
    fn life_orb_and_expert_belt_base_power_modifier() {
        let p_orb = mk_pokemon(Some("Life Orb"));
        assert_eq!(base_power_modifier(&p_orb, 1.0), 1.3);
        let p_belt = mk_pokemon(Some("Expert Belt"));
        assert_eq!(base_power_modifier(&p_belt, 1.0), 1.0);
        assert_eq!(base_power_modifier(&p_belt, 2.0), 1.2);
    }

    #[test]
    fn black_sludge_end_of_turn_effect() {
        let mut poison = mk_pokemon(Some("Black Sludge"));
        poison.types = [Type::Poison, Type::Poison];
        poison.current_hp = poison.stats.hp - 10;
        assert!(matches!(
            end_of_turn_effect(&poison),
            Some(EndOfTurnEffect::Heal { item_id: "blacksludge", .. })
        ));

        let mut non_poison = mk_pokemon(Some("Black Sludge"));
        non_poison.types = [Type::Electric, Type::Electric];
        assert!(matches!(
            end_of_turn_effect(&non_poison),
            Some(EndOfTurnEffect::Damage { item_id: "blacksludge", .. })
        ));
    }
}

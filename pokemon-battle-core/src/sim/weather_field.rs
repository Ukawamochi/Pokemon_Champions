use crate::data::moves::MoveData;
use crate::data::types::Type;
use crate::sim::battle::{Field, Weather};
use crate::sim::pokemon::Pokemon;

fn is_grounded(pokemon: &Pokemon) -> bool {
    if pokemon.roosted {
        return true;
    }
    !(pokemon.types[0] == Type::Flying || pokemon.types[1] == Type::Flying)
}

pub fn weather_damage_modifier(weather: Option<Weather>, move_type: Type) -> f32 {
    match weather {
        Some(Weather::Sun) => match move_type {
            Type::Fire => 1.5,
            Type::Water => 0.5,
            _ => 1.0,
        },
        Some(Weather::Rain) => match move_type {
            Type::Water => 1.5,
            Type::Fire => 0.5,
            _ => 1.0,
        },
        _ => 1.0,
    }
}

pub fn effective_accuracy(move_data: &MoveData, weather: Option<Weather>) -> Option<f32> {
    let normalized = crate::data::moves::normalize_move_name(move_data.name);
    if is_always_hit_move(normalized.as_str()) {
        return None;
    }
    match (normalized.as_str(), weather) {
        ("thunder", Some(Weather::Rain)) => Some(100.0),
        ("hurricane", Some(Weather::Rain)) => Some(100.0),
        ("thunder", Some(Weather::Sun)) => Some(50.0),
        ("hurricane", Some(Weather::Sun)) => Some(50.0),
        ("blizzard", Some(Weather::Hail)) => Some(100.0),
        _ => move_data.accuracy,
    }
}

fn is_always_hit_move(normalized_move: &str) -> bool {
    matches!(
        normalized_move,
        "aerialace"
            | "aurasphere"
            | "swift"
            | "magicalleaf"
            | "shockwave"
            | "shadowpunch"
            | "vitalthrow"
    )
}

pub fn field_damage_modifier(
    field: Option<Field>,
    attacker: &Pokemon,
    defender: &Pokemon,
    move_type: Type,
    normalized_move: &str,
) -> f32 {
    let mut modifier = 1.0;
    match field {
        Some(Field::Grassy) => {
            if is_grounded(attacker) && move_type == Type::Grass {
                modifier *= 1.3;
            }
            if is_grounded(defender)
                && move_type == Type::Ground
                && matches!(normalized_move, "earthquake" | "bulldoze" | "magnitude")
            {
                modifier *= 0.5;
            }
        }
        Some(Field::Electric) => {
            if is_grounded(attacker) && move_type == Type::Electric {
                modifier *= 1.3;
            }
        }
        Some(Field::Psychic) => {
            if is_grounded(attacker) && move_type == Type::Psychic {
                modifier *= 1.3;
            }
        }
        Some(Field::Misty) => {
            if is_grounded(defender) && move_type == Type::Dragon {
                modifier *= 0.5;
            }
        }
        None => {}
    }
    modifier
}

pub fn weather_speed_multiplier(pokemon: &Pokemon, weather: Option<Weather>) -> f32 {
    match weather {
        Some(Weather::Sand) if pokemon.has_ability("Sand Rush") => 2.0,
        Some(Weather::Hail) if pokemon.has_ability("Slush Rush") => 2.0,
        _ => 1.0,
    }
}

pub fn weather_residual_damage(pokemon: &Pokemon, weather: Option<Weather>) -> Option<(u16, Weather)> {
    let Some(weather) = weather else {
        return None;
    };
    match weather {
        Weather::Sand => {
            let immune = pokemon.types[0] == Type::Rock
                || pokemon.types[0] == Type::Ground
                || pokemon.types[0] == Type::Steel
                || pokemon.types[1] == Type::Rock
                || pokemon.types[1] == Type::Ground
                || pokemon.types[1] == Type::Steel;
            if immune {
                return None;
            }
            let dmg = (pokemon.stats.hp as u32 / 16).max(1) as u16;
            Some((dmg, Weather::Sand))
        }
        Weather::Hail => {
            let immune = pokemon.types[0] == Type::Ice || pokemon.types[1] == Type::Ice;
            if immune {
                return None;
            }
            let dmg = (pokemon.stats.hp as u32 / 16).max(1) as u16;
            Some((dmg, Weather::Hail))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::moves::get_move;
    use crate::sim::stats::Nature;

    fn make_pokemon_with_types(ability: &str, types: [Type; 2]) -> Pokemon {
        let mut pokemon = Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec!["tackle".to_string()],
            ability,
            None,
        )
        .expect("pokemon");
        pokemon.types = types;
        pokemon
    }

    #[test]
    fn thunder_hits_in_rain() {
        let thunder = get_move("thunder").expect("thunder");
        assert_eq!(effective_accuracy(&thunder, Some(Weather::Rain)), Some(100.0));
    }

    #[test]
    fn blizzard_hits_in_hail() {
        let blizzard = get_move("blizzard").expect("blizzard");
        assert_eq!(effective_accuracy(&blizzard, Some(Weather::Hail)), Some(100.0));
    }

    #[test]
    fn grassy_halves_earthquake_on_grounded_target() {
        let attacker = make_pokemon_with_types("No Ability", [Type::Normal, Type::Normal]);
        let defender = make_pokemon_with_types("No Ability", [Type::Normal, Type::Normal]);
        let modifier = field_damage_modifier(
            Some(Field::Grassy),
            &attacker,
            &defender,
            Type::Ground,
            "earthquake",
        );
        assert!((modifier - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sand_rush_doubles_speed_in_sand() {
        let pokemon = make_pokemon_with_types("Sand Rush", [Type::Normal, Type::Normal]);
        assert_eq!(weather_speed_multiplier(&pokemon, Some(Weather::Sand)), 2.0);
        assert_eq!(weather_speed_multiplier(&pokemon, Some(Weather::Rain)), 1.0);
    }

    #[test]
    fn slush_rush_doubles_speed_in_hail() {
        let pokemon = make_pokemon_with_types("Slush Rush", [Type::Normal, Type::Normal]);
        assert_eq!(weather_speed_multiplier(&pokemon, Some(Weather::Hail)), 2.0);
        assert_eq!(weather_speed_multiplier(&pokemon, Some(Weather::Sand)), 1.0);
    }
}

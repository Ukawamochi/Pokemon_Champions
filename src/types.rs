// Ref: pokemon-showdown/sim/dex-data.ts: type chart multipliers (trimmed to multipliers only).
pub fn type_effectiveness(move_type: &str, target_types: &[String]) -> f32 {
    if target_types.is_empty() {
        return 1.0;
    }
    let mut multiplier = 1.0;
    for t in target_types {
        multiplier *= single_type_effectiveness(move_type, t);
    }
    multiplier
}

fn single_type_effectiveness(attacking: &str, defending: &str) -> f32 {
    let atk = attacking.to_ascii_lowercase();
    let def = defending.to_ascii_lowercase();
    match atk.as_str() {
        "normal" => match def.as_str() {
            "rock" | "steel" => 0.5,
            "ghost" => 0.0,
            _ => 1.0,
        },
        "fire" => match def.as_str() {
            "fire" | "water" | "rock" | "dragon" => 0.5,
            "grass" | "ice" | "bug" | "steel" => 2.0,
            _ => 1.0,
        },
        "water" => match def.as_str() {
            "water" | "grass" | "dragon" => 0.5,
            "fire" | "ground" | "rock" => 2.0,
            _ => 1.0,
        },
        "electric" => match def.as_str() {
            "electric" | "grass" | "dragon" => 0.5,
            "water" | "flying" => 2.0,
            "ground" => 0.0,
            _ => 1.0,
        },
        "grass" => match def.as_str() {
            "fire" | "grass" | "poison" | "flying" | "bug" | "dragon" | "steel" => 0.5,
            "water" | "ground" | "rock" => 2.0,
            _ => 1.0,
        },
        "ice" => match def.as_str() {
            "fire" | "water" | "ice" | "steel" => 0.5,
            "grass" | "ground" | "flying" | "dragon" => 2.0,
            _ => 1.0,
        },
        "fighting" => match def.as_str() {
            "normal" | "ice" | "rock" | "dark" | "steel" => 2.0,
            "poison" | "flying" | "psychic" | "bug" | "fairy" => 0.5,
            "ghost" => 0.0,
            _ => 1.0,
        },
        "poison" => match def.as_str() {
            "grass" | "fairy" => 2.0,
            "poison" | "ground" | "rock" | "ghost" => 0.5,
            "steel" => 0.0,
            _ => 1.0,
        },
        "ground" => match def.as_str() {
            "fire" | "electric" | "poison" | "rock" | "steel" => 2.0,
            "grass" | "bug" => 0.5,
            "flying" => 0.0,
            _ => 1.0,
        },
        "flying" => match def.as_str() {
            "grass" | "fighting" | "bug" => 2.0,
            "electric" | "rock" | "steel" => 0.5,
            _ => 1.0,
        },
        "psychic" => match def.as_str() {
            "fighting" | "poison" => 2.0,
            "psychic" | "steel" => 0.5,
            "dark" => 0.0,
            _ => 1.0,
        },
        "bug" => match def.as_str() {
            "grass" | "psychic" | "dark" => 2.0,
            "fire" | "fighting" | "poison" | "flying" | "ghost" | "steel" | "fairy" => 0.5,
            _ => 1.0,
        },
        "rock" => match def.as_str() {
            "fire" | "ice" | "flying" | "bug" => 2.0,
            "fighting" | "ground" | "steel" => 0.5,
            _ => 1.0,
        },
        "ghost" => match def.as_str() {
            "ghost" | "psychic" => 2.0,
            "dark" => 0.5,
            "normal" => 0.0,
            _ => 1.0,
        },
        "dragon" => match def.as_str() {
            "dragon" => 2.0,
            "steel" => 0.5,
            "fairy" => 0.0,
            _ => 1.0,
        },
        "dark" => match def.as_str() {
            "psychic" | "ghost" => 2.0,
            "fighting" | "dark" | "fairy" => 0.5,
            _ => 1.0,
        },
        "steel" => match def.as_str() {
            "rock" | "ice" | "fairy" => 2.0,
            "fire" | "water" | "electric" | "steel" => 0.5,
            _ => 1.0,
        },
        "fairy" => match def.as_str() {
            "fighting" | "dragon" | "dark" => 2.0,
            "fire" | "poison" | "steel" => 0.5,
            _ => 1.0,
        },
        _ => 1.0,
    }
}

use crate::data::types::Type;
use crate::sim::pokemon::{Pokemon, Status};

pub fn normalize_item_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

pub fn item_id(pokemon: &Pokemon) -> Option<String> {
    pokemon.item.as_ref().map(|name| normalize_item_name(name))
}

pub fn has_item(pokemon: &Pokemon, item: &str) -> bool {
    item_id(pokemon).as_deref() == Some(item)
}

pub fn can_consume_item(pokemon: &Pokemon) -> bool {
    pokemon.item.is_some() && !pokemon.item_consumed
}

pub fn consume_item(pokemon: &mut Pokemon, item: &str) -> bool {
    if !can_consume_item(pokemon) {
        return false;
    }
    if !has_item(pokemon, item) {
        return false;
    }
    pokemon.item_consumed = true;
    true
}

pub fn restore_consumed_item(pokemon: &mut Pokemon) {
    pokemon.item_consumed = false;
}

fn ripen_multiplier(pokemon: &Pokemon) -> f32 {
    if pokemon.has_ability("Ripen") {
        1.5
    } else {
        1.0
    }
}

fn cheek_pouch_bonus_ratio(pokemon: &Pokemon) -> f32 {
    if pokemon.has_ability("Cheek Pouch") {
        1.0 / 3.0
    } else {
        0.0
    }
}

pub fn try_consume_sitrus_berry(pokemon: &mut Pokemon) -> Option<u16> {
    if pokemon.current_hp == 0 || pokemon.current_hp * 2 > pokemon.stats.hp {
        return None;
    }
    if !consume_item(pokemon, "sitrusberry") {
        return None;
    }
    let base = (pokemon.stats.hp as u32 / 4).max(1) as u16;
    let heal = ((base as f32) * ripen_multiplier(pokemon)).floor().max(1.0) as u16;
    let before = pokemon.current_hp;
    pokemon.current_hp = (pokemon.current_hp + heal).min(pokemon.stats.hp);
    let mut total = pokemon.current_hp.saturating_sub(before);
    let bonus_ratio = cheek_pouch_bonus_ratio(pokemon);
    if bonus_ratio > 0.0 && pokemon.current_hp < pokemon.stats.hp {
        let bonus = ((pokemon.stats.hp as f32) * bonus_ratio).floor().max(1.0) as u16;
        let before = pokemon.current_hp;
        pokemon.current_hp = (pokemon.current_hp + bonus).min(pokemon.stats.hp);
        total = total.saturating_add(pokemon.current_hp.saturating_sub(before));
    }
    Some(total)
}

pub fn try_consume_lum_berry(pokemon: &mut Pokemon) -> bool {
    if pokemon.current_hp == 0 {
        return false;
    }
    if pokemon.status.is_none() && !pokemon.flinched {
        return false;
    }
    if !consume_item(pokemon, "lumberry") {
        return false;
    }
    pokemon.clear_status();
    pokemon.flinched = false;
    true
}

pub fn try_consume_chesto_berry(pokemon: &mut Pokemon) -> bool {
    if pokemon.current_hp == 0 {
        return false;
    }
    if !matches!(pokemon.status, Some(Status::Sleep)) {
        return false;
    }
    if !consume_item(pokemon, "chestoberry") {
        return false;
    }
    pokemon.clear_status();
    true
}

pub fn try_consume_resist_berry(
    pokemon: &mut Pokemon,
    move_type: Type,
    type_effectiveness: f32,
) -> bool {
    if pokemon.current_hp == 0 {
        return false;
    }
    if type_effectiveness <= 1.0 {
        return false;
    }
    let berry = match move_type {
        Type::Fire => "occaberry",
        Type::Water => "passhoberry",
        Type::Electric => "wacanberry",
        Type::Grass => "rindoberry",
        Type::Ice => "yacheberry",
        Type::Fighting => "chopleberry",
        Type::Poison => "kebiaberry",
        Type::Ground => "shucaberry",
        Type::Flying => "cobaberry",
        Type::Psychic => "payapaberry",
        Type::Bug => "tangaberry",
        Type::Rock => "chartiberry",
        Type::Ghost => "kasibberry",
        Type::Dragon => "habanberry",
        Type::Dark => "colburberry",
        Type::Steel => "babiriberry",
        Type::Fairy => "roseliberry",
        _ => return false,
    };
    consume_item(pokemon, berry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::stats::Nature;

    fn make_pokemon(item: Option<&str>, ability: &str) -> Pokemon {
        Pokemon::new(
            "charizard",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec!["tackle".to_string()],
            ability,
            item.map(|s| s.to_string()),
        )
        .expect("species exists")
    }

    #[test]
    fn consume_and_restore_item() {
        let mut pokemon = make_pokemon(Some("Sitrus Berry"), "Blaze");
        assert!(can_consume_item(&pokemon));
        assert!(consume_item(&mut pokemon, "sitrusberry"));
        assert!(!can_consume_item(&pokemon));
        restore_consumed_item(&mut pokemon);
        assert!(can_consume_item(&pokemon));
    }

    #[test]
    fn sitrus_berry_heals() {
        let mut pokemon = make_pokemon(Some("Sitrus Berry"), "Blaze");
        pokemon.current_hp = pokemon.stats.hp / 2;
        let healed = try_consume_sitrus_berry(&mut pokemon).expect("should consume");
        assert!(healed > 0);
        assert!(pokemon.item_consumed);
    }
}


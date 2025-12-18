use crate::data::types::Type;

fn normalize_item_id(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

pub fn item_type_boost(item: &str, move_type: Type) -> f32 {
    let id = normalize_item_id(item);
    let boosted_type = match id.as_str() {
        "silkscarf" => Some(Type::Normal),
        "charcoal" => Some(Type::Fire),
        "mysticwater" => Some(Type::Water),
        "magnet" => Some(Type::Electric),
        "miracleseed" => Some(Type::Grass),
        "nevermeltice" => Some(Type::Ice),
        "blackbelt" => Some(Type::Fighting),
        "poisonbarb" => Some(Type::Poison),
        "softsand" => Some(Type::Ground),
        "sharpbeak" => Some(Type::Flying),
        "twistedspoon" => Some(Type::Psychic),
        "silverpowder" => Some(Type::Bug),
        "hardstone" => Some(Type::Rock),
        "spelltag" => Some(Type::Ghost),
        "dragonfang" => Some(Type::Dragon),
        "blackglasses" => Some(Type::Dark),
        "metalcoat" => Some(Type::Steel),
        "fairyfeather" => Some(Type::Fairy),
        _ => None,
    };
    if boosted_type == Some(move_type) {
        1.2
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZCrystalDef {
    pub id: &'static str,
    pub move_type: Type,
}

pub const Z_CRYSTALS: &[ZCrystalDef] = &[
    ZCrystalDef { id: "normaliumz", move_type: Type::Normal },
    ZCrystalDef { id: "firiumz", move_type: Type::Fire },
    ZCrystalDef { id: "wateriumz", move_type: Type::Water },
    ZCrystalDef { id: "electriumz", move_type: Type::Electric },
    ZCrystalDef { id: "grassiumz", move_type: Type::Grass },
    ZCrystalDef { id: "iciumz", move_type: Type::Ice },
    ZCrystalDef { id: "fightiniumz", move_type: Type::Fighting },
    ZCrystalDef { id: "poisoniumz", move_type: Type::Poison },
    ZCrystalDef { id: "groundiumz", move_type: Type::Ground },
    ZCrystalDef { id: "flyiniumz", move_type: Type::Flying },
    ZCrystalDef { id: "psychiumz", move_type: Type::Psychic },
    ZCrystalDef { id: "buginiumz", move_type: Type::Bug },
    ZCrystalDef { id: "rockiumz", move_type: Type::Rock },
    ZCrystalDef { id: "ghostiumz", move_type: Type::Ghost },
    ZCrystalDef { id: "dragoniumz", move_type: Type::Dragon },
    ZCrystalDef { id: "darkiniumz", move_type: Type::Dark },
    ZCrystalDef { id: "steeliumz", move_type: Type::Steel },
    ZCrystalDef { id: "fairiumz", move_type: Type::Fairy },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_boost_items_match_types() {
        assert_eq!(item_type_boost("Charcoal", Type::Fire), 1.2);
        assert_eq!(item_type_boost("Mystic Water", Type::Water), 1.2);
        assert_eq!(item_type_boost("Magnet", Type::Electric), 1.2);
        assert_eq!(item_type_boost("Miracle Seed", Type::Grass), 1.2);
        assert_eq!(item_type_boost("Never-Melt Ice", Type::Ice), 1.2);
        assert_eq!(item_type_boost("Black Belt", Type::Fighting), 1.2);
        assert_eq!(item_type_boost("Poison Barb", Type::Poison), 1.2);
        assert_eq!(item_type_boost("Soft Sand", Type::Ground), 1.2);
        assert_eq!(item_type_boost("Sharp Beak", Type::Flying), 1.2);
        assert_eq!(item_type_boost("Twisted Spoon", Type::Psychic), 1.2);
        assert_eq!(item_type_boost("Silver Powder", Type::Bug), 1.2);
        assert_eq!(item_type_boost("Hard Stone", Type::Rock), 1.2);
        assert_eq!(item_type_boost("Spell Tag", Type::Ghost), 1.2);
        assert_eq!(item_type_boost("Dragon Fang", Type::Dragon), 1.2);
        assert_eq!(item_type_boost("Black Glasses", Type::Dark), 1.2);
        assert_eq!(item_type_boost("Metal Coat", Type::Steel), 1.2);
        assert_eq!(item_type_boost("Silk Scarf", Type::Normal), 1.2);
        assert_eq!(item_type_boost("Fairy Feather", Type::Fairy), 1.2);
    }

    #[test]
    fn type_boost_items_do_not_boost_other_types() {
        assert_eq!(item_type_boost("Charcoal", Type::Water), 1.0);
        assert_eq!(item_type_boost("Mystic Water", Type::Fire), 1.0);
        assert_eq!(item_type_boost("No Item", Type::Fire), 1.0);
    }

    #[test]
    fn z_crystals_cover_all_types() {
        assert_eq!(Z_CRYSTALS.len(), 18);
    }
}


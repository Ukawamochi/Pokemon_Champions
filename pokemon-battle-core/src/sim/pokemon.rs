use crate::data::species::POKEDEX;
use crate::data::types::Type;
use crate::sim::abilities::status_abilities::ability_blocks_status;
use crate::sim::stats::{Nature, StatsSet};
use anyhow::{anyhow, Result};
use rand::Rng;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Burn,
    Paralysis,
    Poison,
    Sleep,
    Freeze,
    Flinch,
}

#[derive(Clone, Debug)]
pub struct Pokemon {
    pub species: String,
    pub level: u8,
    pub stats: StatsSet,
    pub current_hp: u16,
    pub substitute_hp: u16,
    pub status: Option<Status>,
    pub sleep_turns: u8, // PS: statusState.time (slp)
    pub toxic_counter: u8, // PS: statusState.stage (tox)
    pub stat_stages: [i8; 6],
    pub accuracy_stage: i8,
    pub evasion_stage: i8,
    pub protect_active: bool,
    pub protect_counter: u8,
    pub endure_active: bool,
    pub charge_active: bool,
    pub magic_coat_active: bool,
    pub telekinesis_turns: u8,
    pub kings_shield_active: bool,
    pub roosted: bool,
    pub stance_blade: bool,
    pub semi_invulnerable: bool,
    pub flinched: bool,
    pub confusion_turns: u8,
    pub trapped: bool,
    pub destiny_bond: bool,
    pub perish_count: u8,
    pub taunt_turns: u8,
    pub encore_turns: u8,
    pub encore_move: Option<String>,
    pub last_move: Option<String>,
    pub choice_lock_move: Option<String>,
    pub types: [Type; 2],
    pub moves: Vec<String>,
    pub ability: String,
    pub item: Option<String>,
    pub item_consumed: bool,
    pub charging_move: Option<String>,
}

impl Pokemon {
    pub fn new(
        species: impl Into<String>,
        level: u8,
        evs: [u8; 6],
        ivs: [u8; 6],
        nature: Nature,
        moves: Vec<String>,
        ability: impl Into<String>,
        item: Option<String>,
    ) -> Result<Self> {
        let species_str = species.into();
        let ability_str = ability.into();
        let species_id = normalize_id(species_str.as_str());
        let stats = StatsSet::from_species(species_id.as_str(), level, evs, ivs, nature)
            .ok_or_else(|| anyhow!("Species '{}' not found in POKEDEX", species_str))?;
        let types = species_types(species_id.as_str())
            .ok_or_else(|| anyhow!("Species '{}' not found in POKEDEX", species_str))?;
        Ok(Self {
            species: species_str,
            level,
            current_hp: stats.hp,
            substitute_hp: 0,
            stats,
            status: None,
            sleep_turns: 0,
            toxic_counter: 0,
            stat_stages: [0; 6],
            accuracy_stage: 0,
            evasion_stage: 0,
            protect_active: false,
            protect_counter: 0,
            endure_active: false,
            charge_active: false,
            magic_coat_active: false,
            telekinesis_turns: 0,
            kings_shield_active: false,
            roosted: false,
            stance_blade: false,
            semi_invulnerable: false,
            flinched: false,
            confusion_turns: 0,
            trapped: false,
            destiny_bond: false,
            perish_count: 0,
            taunt_turns: 0,
            encore_turns: 0,
            encore_move: None,
            last_move: None,
            choice_lock_move: None,
            types,
            moves,
            ability: ability_str,
            item,
            item_consumed: false,
            charging_move: None,
        })
    }

    pub fn take_damage(&mut self, damage: u16) {
        self.current_hp = self.current_hp.saturating_sub(damage);
    }

    pub fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }

    pub fn apply_status(&mut self, status: Status, rng: &mut impl Rng) -> bool {
        self.apply_status_internal(status, false, rng)
    }

    pub fn apply_toxic(&mut self, rng: &mut impl Rng) -> bool {
        self.apply_status_internal(Status::Poison, true, rng)
    }

    pub fn apply_confusion(&mut self, rng: &mut impl Rng) -> bool {
        if self.confusion_turns > 0 {
            return false;
        }
        // Showdown: random(2, 6) -> 2..=5 turns
        self.confusion_turns = rng.gen_range(2..=5);
        true
    }

    fn apply_status_internal(&mut self, status: Status, toxic: bool, rng: &mut impl Rng) -> bool {
        if matches!(status, Status::Flinch) {
            self.flinched = true;
            return true;
        }
        if self.status.is_some() {
            return false;
        }
        if self.is_status_immune(status) {
            return false;
        }
        match status {
            Status::Sleep => {
                // PS: random(2, 5) -> 2..=4 (1-3 turns asleep)
                self.sleep_turns = rng.gen_range(2..=4);
            }
            Status::Poison => {
                // PS: tox stage starts at 0 and increments each residual
                self.toxic_counter = if toxic { 1 } else { 0 };
            }
            _ => {}
        }
        self.status = Some(status);
        true
    }

    pub fn clear_status(&mut self) {
        self.status = None;
        self.sleep_turns = 0;
        self.toxic_counter = 0;
    }

    pub fn has_ability(&self, ability: &str) -> bool {
        self.ability.eq_ignore_ascii_case(ability)
    }

    fn is_status_immune(&self, status: Status) -> bool {
        // Type-based immunities
        if matches!(status, Status::Burn) && (self.types[0] == Type::Fire || self.types[1] == Type::Fire) {
            return true;
        }
        if matches!(status, Status::Paralysis) && (self.types[0] == Type::Electric || self.types[1] == Type::Electric) {
            return true;
        }
        if matches!(status, Status::Poison)
            && (self.types[0] == Type::Poison
                || self.types[0] == Type::Steel
                || self.types[1] == Type::Poison
                || self.types[1] == Type::Steel)
        {
            return true;
        }
        if matches!(status, Status::Freeze) && (self.types[0] == Type::Ice || self.types[1] == Type::Ice) {
            return true;
        }
        // Ability-based immunities
        if matches!(status, Status::Flinch) {
            return false;
        }
        ability_blocks_status(self, status)
    }
}

fn species_types(species: &str) -> Option<[Type; 2]> {
    let id = normalize_id(species);
    let info = POKEDEX.get(id.as_str())?;
    let primary = parse_type(info.types[0]).unwrap_or(Type::Normal);
    let secondary = parse_type(info.types[1]).unwrap_or(primary);
    Some([primary, secondary])
}

fn normalize_id(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn parse_type(name: &str) -> Option<Type> {
    match name.to_ascii_lowercase().as_str() {
        "normal" => Some(Type::Normal),
        "fire" => Some(Type::Fire),
        "water" => Some(Type::Water),
        "electric" => Some(Type::Electric),
        "grass" => Some(Type::Grass),
        "ice" => Some(Type::Ice),
        "fighting" => Some(Type::Fighting),
        "poison" => Some(Type::Poison),
        "ground" => Some(Type::Ground),
        "flying" => Some(Type::Flying),
        "psychic" => Some(Type::Psychic),
        "bug" => Some(Type::Bug),
        "rock" => Some(Type::Rock),
        "ghost" => Some(Type::Ghost),
        "dragon" => Some(Type::Dragon),
        "dark" => Some(Type::Dark),
        "steel" => Some(Type::Steel),
        "fairy" => Some(Type::Fairy),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::stats::Nature;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn pokemon_lookup_is_case_insensitive() {
        let evs = [0; 6];
        let ivs = [31; 6];
        for name in ["Charizard", "charizard", "CHARIZARD"] {
            let pokemon = Pokemon::new(
                name,
                50,
                evs,
                ivs,
                Nature::Adamant,
                vec!["Flamethrower".to_string()],
                "Blaze",
                None,
            )
            .expect("species lookup should ignore casing");
            assert_eq!(pokemon.types[0], Type::Fire);
            assert_eq!(pokemon.types[1], Type::Flying);
        }
    }

    #[test]
    fn pokemon_lookup_reports_missing_species() {
        let result = Pokemon::new(
            "NotAPokemon",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec![],
            "No Ability",
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn sleep_turns_are_in_showdown_range() {
        let mut rng = SmallRng::seed_from_u64(7);
        let mut pokemon = Pokemon::new(
            "Charizard",
            50,
            [0; 6],
            [31; 6],
            Nature::Adamant,
            vec!["Flamethrower".to_string()],
            "Blaze",
            None,
        )
        .expect("species exists");
        assert!(pokemon.apply_status(Status::Sleep, &mut rng));
        assert!((2..=4).contains(&pokemon.sleep_turns));
    }
}

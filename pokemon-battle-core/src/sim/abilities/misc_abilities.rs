use crate::data::types::Type;
use crate::i18n::translate_pokemon;
use crate::sim::battle::{apply_status_with_field, format_status, Field};
use crate::sim::pokemon::{Pokemon, Status};
use rand::rngs::SmallRng;
use rand::Rng;

// Implemented abilities (A4):
// - Rough Skin, Iron Barbs, Effect Spore
// - Water Absorb, Dry Skin, Poison Heal
// - Quick Feet, Swift Swim, Chlorophyll

#[derive(Clone, Copy, Debug)]
pub(crate) enum WaterAbsorbKind {
    WaterAbsorb,
    DrySkin,
}

impl WaterAbsorbKind {
    pub(crate) fn display_name(self) -> &'static str {
        match self {
            WaterAbsorbKind::WaterAbsorb => "ちょすい",
            WaterAbsorbKind::DrySkin => "かんそうはだ",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WaterAbsorbResult {
    pub(crate) kind: WaterAbsorbKind,
}

pub(crate) fn try_absorb_water_move(
    defender: &mut Pokemon,
    move_type: Type,
) -> Option<WaterAbsorbResult> {
    if move_type != Type::Water {
        return None;
    }
    let kind = if defender.has_ability("Water Absorb") {
        WaterAbsorbKind::WaterAbsorb
    } else if defender.has_ability("Dry Skin") {
        WaterAbsorbKind::DrySkin
    } else {
        return None;
    };
    let heal = (defender.stats.hp as u32 / 4).max(1) as u16;
    defender.current_hp = (defender.current_hp + heal).min(defender.stats.hp);
    Some(WaterAbsorbResult { kind })
}

pub(crate) fn poison_heal_amount(pokemon: &Pokemon) -> Option<u16> {
    if !pokemon.has_ability("Poison Heal") {
        return None;
    }
    if !matches!(pokemon.status, Some(Status::Poison)) {
        return None;
    }
    Some((pokemon.stats.hp as u32 / 8).max(1) as u16)
}

pub(crate) fn speed_multiplier(pokemon: &Pokemon, is_rain: bool, is_sun: bool) -> f32 {
    if pokemon.has_ability("Slow Start") {
        return 0.5;
    }
    if pokemon.has_ability("Quick Feet") && pokemon.status.is_some() {
        return 1.5;
    }
    if pokemon.has_ability("Swift Swim") && is_rain {
        return 2.0;
    }
    if pokemon.has_ability("Chlorophyll") && is_sun {
        return 2.0;
    }
    1.0
}

pub(crate) fn apply_contact_damage_abilities(attacker: &mut Pokemon, defender: &Pokemon) {
    let attacker_ja = translate_pokemon(&attacker.species);
    let mut applied = false;
    if defender.has_ability("Rough Skin") {
        applied = true;
    } else if defender.has_ability("Iron Barbs") {
        applied = true;
    }
    if !applied {
        return;
    }
    let dmg = (attacker.stats.hp as u32 / 8).max(1) as u16;
    attacker.take_damage(dmg);
    println!(
        "  {}は{}のダメージをうけた！ (HP: {}/{})",
        attacker_ja, dmg, attacker.current_hp, attacker.stats.hp
    );
    if attacker.is_fainted() {
        println!("  {}はたおれた！", attacker_ja);
    }
}

pub(crate) fn apply_effect_spore(
    attacker: &mut Pokemon,
    defender: &Pokemon,
    field: Option<Field>,
    rng: &mut SmallRng,
) {
    if !defender.has_ability("Effect Spore") {
        return;
    }
    if !rng.gen_bool(0.3) {
        return;
    }
    let status = match rng.gen_range(0..3) {
        0 => Status::Poison,
        1 => Status::Paralysis,
        _ => Status::Sleep,
    };
    if apply_status_with_field(attacker, status, false, field, rng) {
        let attacker_ja = translate_pokemon(&attacker.species);
        println!("  {}は{}！", attacker_ja, format_status(status));
    }
}

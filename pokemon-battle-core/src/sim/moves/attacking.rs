use crate::data::moves::{normalize_move_name, MoveData, MOVES};
use crate::data::types::Type;
use crate::sim::battle::{Field, Weather};
use crate::sim::items::battle_items;
use crate::sim::pokemon::Pokemon;
use crate::sim::pokemon::Status;
use crate::sim::weather_field;
use crate::sim::abilities::misc_abilities::speed_multiplier;
use rand::rngs::SmallRng;
use rand::Rng;

/// Apply recoil damage based on total damage dealt.
pub fn apply_recoil_damage(attacker: &mut Pokemon, damage_dealt: u16, recoil: (u8, u8)) {
    if damage_dealt == 0 {
        return;
    }
    let (num, den) = recoil;
    if den == 0 {
        return;
    }
    // Showdown: battle-actions.ts#L983-L986 + L1394-L1396 (calcRecoilDamage)
    let numerator = damage_dealt as u32 * num as u32;
    let recoil_damage = ((numerator + (den as u32 / 2)) / den as u32).max(1) as u16;
    attacker.take_damage(recoil_damage);
}

/// Heal the attacker based on damage dealt (drain moves).
pub fn apply_drain(attacker: &mut Pokemon, damage_dealt: u16, drain: (u8, u8)) {
    if damage_dealt == 0 {
        return;
    }
    let (num, den) = drain;
    if den == 0 {
        return;
    }
    // Showdown: battle.ts#L2124-L2132 (drain healing)
    let numerator = damage_dealt as u32 * num as u32;
    let heal = ((numerator + (den as u32 / 2)) / den as u32).max(1) as u16;
    attacker.current_hp = (attacker.current_hp + heal).min(attacker.stats.hp);
}

/// Determine the number of hits for multihit moves.
pub fn calculate_multihit_count(move_data: &MoveData, rng: &mut SmallRng) -> u8 {
    // Showdown: battle-actions.ts#L859-L877 (multihit distribution)
    if let Some((min_hits, max_hits)) = move_data.multihit {
        if min_hits == max_hits {
            return min_hits;
        }
        if min_hits == 2 && max_hits == 5 {
            let roll = rng.gen_range(0..20);
            return match roll {
                0..=6 => 2,
                7..=13 => 3,
                14..=16 => 4,
                _ => 5,
            };
        }
        return rng.gen_range(min_hits..=max_hits);
    }
    let normalized = normalize_move_name(move_data.name);
    match normalized.as_str() {
        "bonemerang"
        | "doublehit"
        | "doubleironbash"
        | "doublekick"
        | "dragondarts"
        | "dualchop"
        | "doublechop"
        | "dualwingbeat"
        | "geargrind"
        | "tachyoncutter"
        | "twinbeam"
        | "twineedle" => 2,
        "surgingstrikes" | "tripleaxel" | "tripledive" | "triplekick" => 3,
        "populationbomb" => 10,
        _ => 1,
    }
}

/// Handle the first/second turn of a charging move.
/// Returns true if the move consumes the turn to charge.
pub fn handle_charging_move(pokemon: &mut Pokemon, move_id: &str) -> bool {
    let normalized = normalize_move_name(move_id);
    if !is_charging_move(normalized.as_str()) {
        return false;
    }
    // Showdown: pokemon.ts#L814-L818 (two-turn charge checks)
    if pokemon.charging_move.as_deref() == Some(normalized.as_str()) {
        pokemon.charging_move = None;
        pokemon.semi_invulnerable = false;
        return false;
    }
    pokemon.charging_move = Some(normalized);
    pokemon.semi_invulnerable = is_semi_invulnerable_move(pokemon.charging_move.as_deref().unwrap_or(""));
    true
}

/// Resolve OHKO accuracy and damage. Returns Some(damage) when it hits.
pub fn handle_ohko_move(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_id: &str,
    rng: &mut SmallRng,
) -> Option<u16> {
    let normalized = normalize_move_name(move_id);
    let ohko_type = ohko_type(normalized.as_str())?;
    // Showdown: battle-actions.ts#L691-L703 (OHKO accuracy/eligibility)
    if attacker.level < defender.level {
        return None;
    }
    if let Some(blocked_type) = ohko_type {
        if has_type(defender, blocked_type) {
            return None;
        }
    }
    let level_diff = attacker.level.saturating_sub(defender.level) as i16;
    let mut accuracy: i16 = 30 + level_diff;
    if normalized == "sheercold" && !has_type(attacker, Type::Ice) {
        accuracy = 20 + level_diff;
    }
    let roll = rng.gen_range(0..100) as i16;
    if roll < accuracy {
        // Showdown: battle-actions.ts#L1604 (OHKO damage = target max HP)
        return Some(defender.stats.hp);
    }
    None
}

fn is_charging_move(move_id: &str) -> bool {
    MOVES
        .get(move_id)
        .map(|data| data.flags.iter().any(|flag| *flag == "charge"))
        .unwrap_or(false)
}

fn is_semi_invulnerable_move(move_id: &str) -> bool {
    matches!(
        move_id,
        "fly" | "bounce" | "dig" | "dive" | "phantomforce" | "shadowforce" | "skydrop"
    )
}

fn ohko_type(move_id: &str) -> Option<Option<Type>> {
    match move_id {
        "sheercold" => Some(Some(Type::Ice)),
        "fissure" | "guillotine" | "horndrill" => Some(None),
        _ => None,
    }
}

fn has_type(pokemon: &Pokemon, target: Type) -> bool {
    pokemon.types[0] == target || pokemon.types[1] == target
}

/// Get final move priority with context-dependent modifiers.
pub fn get_move_priority(move_data: &MoveData, _attacker: &Pokemon, field: Option<Field>) -> i8 {
    // Showdown: pokemon.ts#L892-L910 (priority modifications)
    let base_priority = move_data.priority;
    let id = normalize_move_name(move_data.name);
    if id == "grassyglide" && field == Some(Field::Grassy) {
        base_priority + 1
    } else {
        base_priority
    }
}

fn stage_multiplier(stage: i8) -> f32 {
    if stage >= 0 {
        (2 + stage as i32) as f32 / 2.0
    } else {
        2.0 / (2 - stage as i32) as f32
    }
}

fn apply_stage_multiplier(base: u16, stage: i8) -> u16 {
    ((base as f32) * stage_multiplier(stage)).floor().max(1.0) as u16
}

fn effective_speed_for_variable_power(pokemon: &Pokemon, weather: Option<Weather>) -> u16 {
    // Keep consistent with sim::battle::effective_speed (only for variable power moves).
    let mut spe = apply_stage_multiplier(pokemon.stats.spe, pokemon.stat_stages[crate::sim::battle::STAGE_SPE]);
    if matches!(pokemon.status, Some(Status::Paralysis)) && !pokemon.has_ability("Quick Feet") {
        spe = ((spe as f32) * 0.5).floor().max(1.0) as u16;
    }
    let speed_mod = speed_multiplier(
        pokemon,
        matches!(weather, Some(Weather::Rain)),
        matches!(weather, Some(Weather::Sun)),
    );
    let weather_ability_mod = weather_field::weather_speed_multiplier(pokemon, weather);
    let item_mod = battle_items::speed_modifier(pokemon);
    ((spe as f32) * speed_mod * weather_ability_mod * item_mod).floor().max(1.0) as u16
}

/// Calculate variable base power for moves whose power depends on battle state.
pub fn calculate_variable_power(
    move_data: &MoveData,
    attacker: &Pokemon,
    defender: &Pokemon,
    weather: Option<Weather>,
    _field: Option<Field>,
) -> u16 {
    // Showdown: battle-actions.ts#L1205-L1289
    let id = normalize_move_name(move_data.name);
    match id.as_str() {
        "eruption" | "waterspout" => {
            // PS: move.basePower * hp / maxhp
            let max_hp = attacker.stats.hp.max(1);
            ((150u32 * attacker.current_hp as u32) / max_hp as u32) as u16
        }
        "flail" | "reversal" => {
            // PS: ratio = max(floor(hp * 48 / maxhp), 1)
            let max_hp = attacker.stats.hp.max(1);
            let ratio = ((attacker.current_hp as u32 * 48) / (max_hp as u32)).max(1) as u32;
            if ratio < 2 {
                200
            } else if ratio < 5 {
                150
            } else if ratio < 10 {
                100
            } else if ratio < 17 {
                80
            } else if ratio < 33 {
                40
            } else {
                20
            }
        }
        "electroball" => {
            // PS: ratio = floor(userSpe / targetSpe)
            let user_spe = effective_speed_for_variable_power(attacker, weather) as u32;
            let target_spe = effective_speed_for_variable_power(defender, weather) as u32;
            let ratio = if target_spe == 0 { 0 } else { user_spe / target_spe };
            match ratio.min(4) {
                0 => 40,
                1 => 60,
                2 => 80,
                3 => 120,
                _ => 150,
            }
        }
        "gyroball" => {
            // PS: floor(25 * targetSpe / userSpe) + 1, capped at 150.
            let user_spe = effective_speed_for_variable_power(attacker, weather).max(1) as u32;
            let target_spe = effective_speed_for_variable_power(defender, weather) as u32;
            let power = ((25 * target_spe) / user_spe) + 1;
            power.min(150) as u16
        }
        _ => move_data.base_power.unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::stats::Nature;

    fn dummy_pokemon(species: &str, moves: Vec<String>) -> Pokemon {
        Pokemon::new(
            species,
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            moves,
            "",
            None,
        )
        .expect("pokemon")
    }

    #[test]
    fn grassy_glide_priority_is_boosted_in_grassy_field() {
        let glide = MOVES.get("grassyglide").expect("grassyglide");
        let attacker = dummy_pokemon("Rillaboom", vec!["grassyglide".to_string()]);
        assert_eq!(get_move_priority(glide, &attacker, None), glide.priority);
        assert_eq!(
            get_move_priority(glide, &attacker, Some(Field::Grassy)),
            glide.priority + 1
        );
    }

    #[test]
    fn variable_power_eruption_scales_with_hp() {
        let mv = MOVES.get("eruption").expect("eruption");
        let mut attacker = dummy_pokemon("Typhlosion", vec!["eruption".to_string()]);
        let defender = dummy_pokemon("Blissey", vec!["splash".to_string()]);
        attacker.stats.hp = 200;
        attacker.current_hp = 100;
        assert_eq!(
            calculate_variable_power(mv, &attacker, &defender, None, None),
            75
        );
    }

    #[test]
    fn variable_power_flail_matches_showdown_thresholds() {
        let mv = MOVES.get("flail").expect("flail");
        let mut attacker = dummy_pokemon("Magikarp", vec!["flail".to_string()]);
        let defender = dummy_pokemon("Blissey", vec!["splash".to_string()]);
        attacker.stats.hp = 100;

        attacker.current_hp = 1;
        assert_eq!(
            calculate_variable_power(mv, &attacker, &defender, None, None),
            200
        );

        attacker.current_hp = 50;
        assert_eq!(
            calculate_variable_power(mv, &attacker, &defender, None, None),
            40
        );
    }

    #[test]
    fn variable_power_electro_ball_uses_speed_ratio() {
        let mv = MOVES.get("electroball").expect("electroball");
        let mut attacker = dummy_pokemon("Pikachu", vec!["electroball".to_string()]);
        let mut defender = dummy_pokemon("Snorlax", vec!["splash".to_string()]);
        attacker.stats.spe = 200;
        defender.stats.spe = 50;
        assert_eq!(
            calculate_variable_power(mv, &attacker, &defender, None, None),
            150
        );

        attacker.stats.spe = 150;
        defender.stats.spe = 100;
        assert_eq!(
            calculate_variable_power(mv, &attacker, &defender, None, None),
            60
        );
    }

    #[test]
    fn variable_power_gyro_ball_matches_showdown_formula() {
        let mv = MOVES.get("gyroball").expect("gyroball");
        let mut attacker = dummy_pokemon("Ferrothorn", vec!["gyroball".to_string()]);
        let mut defender = dummy_pokemon("Electrode", vec!["splash".to_string()]);
        attacker.stats.spe = 50;
        defender.stats.spe = 200;
        assert_eq!(
            calculate_variable_power(mv, &attacker, &defender, None, None),
            101
        );
    }
}

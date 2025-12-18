use crate::data::moves::{normalize_move_name, MoveData, SecondaryEffect as DataSecondaryEffect};
use crate::sim::battle::{
    apply_status_with_field, EnvUpdate, Field, FieldEffect, HazardKind, HazardUpdate, ScreenUpdate, Weather,
};
use crate::sim::pokemon::{Pokemon, Status};
use crate::sim::stats::Stat;
use rand::rngs::SmallRng;
use rand::Rng;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct SecondaryEffect {
    pub chance: u8,
    pub status: Option<Status>,
    pub toxic: bool,
    pub volatile_status: Option<&'static str>,
    pub boosts: Option<BTreeMap<Stat, i8>>,
    pub target_self: bool,
    pub side_effect: Option<SideEffect>,
    pub affected_by_serene_grace: bool,
}

#[derive(Clone, Debug)]
pub enum SideEffect {
    Hazard(HazardKind),
    Screen(FieldEffect),
    Weather(Weather),
    Field(Field),
}

pub fn secondary_effect_from_move(move_id: &str, move_data: &MoveData) -> Option<SecondaryEffect> {
    let _ = normalize_move_name(move_id);
    secondary_effects_from_move(move_id, move_data).into_iter().next()
}

pub fn secondary_effects_from_move(_move_id: &str, move_data: &MoveData) -> Vec<SecondaryEffect> {
    if !move_data.secondaries.is_empty() {
        return move_data
            .secondaries
            .iter()
            .map(|secondary| effect_from_data(*secondary, false, true))
            .collect();
    }
    move_data
        .secondary
        .map(|secondary| vec![effect_from_data(secondary, false, true)])
        .unwrap_or_default()
}

pub fn self_effect_from_move(move_id: &str, move_data: &MoveData) -> Option<SecondaryEffect> {
    let _ = normalize_move_name(move_id);
    move_data
        .self_effect
        .map(|self_effect| effect_from_data(self_effect, true, false))
}

pub fn apply_secondary_effect(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    effect: &SecondaryEffect,
    field: Option<Field>,
    rng: &mut SmallRng,
) -> bool {
    let mut update = EnvUpdate::default();
    apply_secondary_effect_with_update(attacker, defender, effect, field, 0, 1, &mut update, rng)
}

pub(crate) fn apply_secondary_effect_with_update(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    effect: &SecondaryEffect,
    field: Option<Field>,
    attacker_side_idx: usize,
    defender_side_idx: usize,
    update: &mut EnvUpdate,
    rng: &mut SmallRng,
) -> bool {
    let mut chance = effect.chance;
    if effect.affected_by_serene_grace && attacker.has_ability("Serene Grace") {
        chance = chance.saturating_mul(2).min(100);
    }
    if chance == 0 {
        return false;
    }
    let roll: u8 = rng.gen_range(0..100);
    if roll >= chance {
        return false;
    }

    let target = if effect.target_self { attacker } else { defender };
    if target.is_fainted() {
        return false;
    }

    let mut applied = false;

    if let Some(status) = effect.status {
        if apply_status_with_field(target, status, effect.toxic, field, rng) {
            applied = true;
        }
    }

    if let Some(volatile) = effect.volatile_status {
        if apply_volatile_status(target, volatile, rng) {
            applied = true;
        }
    }

    if let Some(ref boosts) = effect.boosts {
        for (stat, delta) in boosts {
            if apply_stat_change(target, *stat, *delta) {
                applied = true;
            }
        }
    }

    if let Some(ref side_effect) = effect.side_effect {
        if apply_side_effect(
            side_effect,
            effect.target_self,
            attacker_side_idx,
            defender_side_idx,
            update,
        ) {
            applied = true;
        }
    }

    applied
}

fn apply_side_effect(
    effect: &SideEffect,
    target_self: bool,
    attacker_side_idx: usize,
    defender_side_idx: usize,
    update: &mut EnvUpdate,
) -> bool {
    match effect {
        SideEffect::Hazard(kind) => {
            let target = if target_self { attacker_side_idx } else { defender_side_idx };
            update.hazard = Some(HazardUpdate { target, kind: *kind });
            true
        }
        SideEffect::Screen(kind) => {
            let target = if target_self { attacker_side_idx } else { defender_side_idx };
            update.screen = Some(ScreenUpdate {
                target,
                kind: *kind,
                turns: 5,
            });
            true
        }
        SideEffect::Weather(weather) => {
            update.weather = Some(*weather);
            true
        }
        SideEffect::Field(field) => {
            update.field = Some(*field);
            true
        }
    }
}

fn apply_volatile_status(target: &mut Pokemon, volatile: &str, rng: &mut SmallRng) -> bool {
    match volatile {
        "confusion" => target.apply_confusion(rng),
        _ => false,
    }
}

fn effect_from_data(data: DataSecondaryEffect, target_self: bool, affected_by_serene_grace: bool) -> SecondaryEffect {
    let (status, toxic) = data
        .status
        .and_then(status_from_id)
        .map(|(s, t)| (Some(s), t))
        .unwrap_or((None, false));

    let volatile_status = data.volatile_status;
    let side_effect = side_effect_from_data(&data);

    SecondaryEffect {
        chance: data.chance,
        status: status.or_else(|| (volatile_status == Some("flinch")).then_some(Status::Flinch)),
        toxic,
        volatile_status,
        boosts: parse_boosts(data.boosts),
        target_self,
        side_effect,
        affected_by_serene_grace,
    }
}

fn side_effect_from_data(data: &DataSecondaryEffect) -> Option<SideEffect> {
    if let Some(side_condition) = data.side_condition {
        let id = normalize_move_name(side_condition);
        return match id.as_str() {
            "stealthrock" => Some(SideEffect::Hazard(HazardKind::StealthRock)),
            "spikes" => Some(SideEffect::Hazard(HazardKind::Spikes)),
            "toxicspikes" => Some(SideEffect::Hazard(HazardKind::ToxicSpikes)),
            "stickyweb" => Some(SideEffect::Hazard(HazardKind::StickyWeb)),
            "reflect" => Some(SideEffect::Screen(FieldEffect::Reflect)),
            "lightscreen" => Some(SideEffect::Screen(FieldEffect::LightScreen)),
            _ => None,
        };
    }
    if let Some(weather) = data.weather {
        let id = normalize_move_name(weather);
        return match id.as_str() {
            "sunnyday" | "desolateland" => Some(SideEffect::Weather(Weather::Sun)),
            "raindance" | "primordialsea" => Some(SideEffect::Weather(Weather::Rain)),
            "sandstorm" => Some(SideEffect::Weather(Weather::Sand)),
            "hail" | "snowscape" => Some(SideEffect::Weather(Weather::Hail)),
            _ => None,
        };
    }
    if let Some(terrain) = data.terrain {
        let id = normalize_move_name(terrain);
        return match id.as_str() {
            "grassyterrain" => Some(SideEffect::Field(Field::Grassy)),
            "electricterrain" => Some(SideEffect::Field(Field::Electric)),
            "psychicterrain" => Some(SideEffect::Field(Field::Psychic)),
            "mistyterrain" => Some(SideEffect::Field(Field::Misty)),
            _ => None,
        };
    }
    None
}

fn parse_boosts(boosts: &[(&'static str, i8)]) -> Option<BTreeMap<Stat, i8>> {
    if boosts.is_empty() {
        return None;
    }
    let mut map = BTreeMap::new();
    for (stat_id, amount) in boosts {
        if let Some(stat) = stat_from_id(stat_id) {
            if *amount != 0 {
                map.insert(stat, *amount);
            }
        }
    }
    (!map.is_empty()).then_some(map)
}

fn stat_from_id(id: &str) -> Option<Stat> {
    match id {
        "atk" => Some(Stat::Atk),
        "def" => Some(Stat::Def),
        "spa" => Some(Stat::Spa),
        "spd" => Some(Stat::Spd),
        "spe" => Some(Stat::Spe),
        _ => None,
    }
}

fn status_from_id(id: &str) -> Option<(Status, bool)> {
    match id {
        "brn" => Some((Status::Burn, false)),
        "par" => Some((Status::Paralysis, false)),
        "psn" => Some((Status::Poison, false)),
        "tox" => Some((Status::Poison, true)),
        "slp" => Some((Status::Sleep, false)),
        "frz" => Some((Status::Freeze, false)),
        _ => None,
    }
}

fn apply_stat_change(pokemon: &mut Pokemon, stat: Stat, delta: i8) -> bool {
    let idx = match stat {
        Stat::Atk => crate::sim::battle::STAGE_ATK,
        Stat::Def => crate::sim::battle::STAGE_DEF,
        Stat::Spa => crate::sim::battle::STAGE_SPA,
        Stat::Spd => crate::sim::battle::STAGE_SPD,
        Stat::Spe => crate::sim::battle::STAGE_SPE,
        Stat::Hp => return false,
    };
    let current = pokemon.stat_stages[idx];
    let next = current.saturating_add(delta).clamp(-6, 6);
    if next == current {
        return false;
    }
    pokemon.stat_stages[idx] = next;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::moves::get_move;
    use crate::sim::pokemon::Pokemon;
    use crate::sim::stats::Nature;
    use rand::SeedableRng;

    fn make_pokemon(species: &str) -> Pokemon {
        Pokemon::new(
            species,
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec![],
            "Serene Grace",
            None,
        )
        .expect("species exists")
    }

    #[test]
    fn secondary_effects_from_move_reads_secondaries_array() {
        let fire_fang = get_move("firefang").expect("move exists");
        let effects = secondary_effects_from_move("firefang", fire_fang);
        assert_eq!(effects.len(), 2);
        assert!(effects.iter().any(|e| e.status == Some(Status::Burn)));
        assert!(effects.iter().any(|e| e.status == Some(Status::Flinch) || e.volatile_status == Some("flinch")));
    }

    #[test]
    fn apply_secondary_effect_can_update_env_update() {
        let mut attacker = make_pokemon("togekiss");
        let mut defender = make_pokemon("blissey");
        let mut update = EnvUpdate::default();
        let mut rng = SmallRng::seed_from_u64(1);
        let effect = SecondaryEffect {
            chance: 100,
            status: None,
            toxic: false,
            volatile_status: None,
            boosts: None,
            target_self: false,
            side_effect: Some(SideEffect::Weather(Weather::Rain)),
            affected_by_serene_grace: true,
        };
        assert!(apply_secondary_effect_with_update(
            &mut attacker,
            &mut defender,
            &effect,
            None,
            0,
            1,
            &mut update,
            &mut rng
        ));
        assert_eq!(update.weather, Some(Weather::Rain));
    }
}

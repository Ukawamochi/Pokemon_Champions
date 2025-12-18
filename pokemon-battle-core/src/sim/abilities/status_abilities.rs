use crate::sim::pokemon::{Pokemon, Status};

const STAGE_ATK: usize = 0;
const STAGE_SPA: usize = 2;

pub fn ability_blocks_status(pokemon: &Pokemon, status: Status) -> bool {
    match status {
        Status::Poison => pokemon.has_ability("Immunity"),
        Status::Paralysis => pokemon.has_ability("Limber"),
        Status::Burn => pokemon.has_ability("Water Veil"),
        Status::Freeze => pokemon.has_ability("Magma Armor"),
        Status::Sleep => pokemon.has_ability("Insomnia") || pokemon.has_ability("Vital Spirit"),
        Status::Flinch => pokemon.has_ability("Inner Focus"),
    }
}

pub fn apply_intimidate(target: &mut Pokemon) -> bool {
    apply_stage_change(target, STAGE_ATK, -1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadBoost {
    Attack,
    SpAttack,
}

pub fn apply_download(user: &mut Pokemon, target: &Pokemon) -> Option<DownloadBoost> {
    let boost = if target.stats.def < target.stats.spd {
        DownloadBoost::Attack
    } else {
        DownloadBoost::SpAttack
    };
    let applied = match boost {
        DownloadBoost::Attack => apply_stage_change(user, STAGE_ATK, 1),
        DownloadBoost::SpAttack => apply_stage_change(user, STAGE_SPA, 1),
    };
    applied.then_some(boost)
}

pub fn apply_trace(user: &mut Pokemon, target: &Pokemon) -> Option<String> {
    let traced = target.ability.clone();
    if traced.is_empty() {
        return None;
    }
    if traced.eq_ignore_ascii_case("Trace") {
        return None;
    }
    user.ability = traced.clone();
    Some(traced)
}

fn apply_stage_change(pokemon: &mut Pokemon, stat: usize, delta: i8) -> bool {
    let current = pokemon.stat_stages[stat];
    let next = current.saturating_add(delta).clamp(-6, 6);
    if next == current {
        return false;
    }
    pokemon.stat_stages[stat] = next;
    true
}

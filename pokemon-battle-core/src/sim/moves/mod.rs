pub mod attacking;
pub mod flags;
pub mod secondary;
pub mod status;

use crate::data::moves::{MoveCategory, MoveData};
use crate::sim::battle::{Action, BattleState, EnvUpdate, Field, Weather};
use crate::sim::pokemon::Pokemon;
use rand::rngs::SmallRng;

pub use attacking::{
    apply_drain, apply_recoil_damage, calculate_multihit_count, calculate_variable_power,
    get_move_priority, handle_charging_move, handle_ohko_move,
};
pub use flags::{
    affects_grounded_only, bypasses_protect, bypasses_substitute, check_ability_immunity,
    is_blocked_by_bulletproof, is_blocked_by_protect, is_bullet_move, is_contact_move, is_pulse_move,
    is_sound_move, move_has_flag, FLAG_BITE, FLAG_BULLET, FLAG_CONTACT, FLAG_HEAL, FLAG_METRONOME,
    FLAG_MIRROR, FLAG_POWDER, FLAG_PROTECT, FLAG_PULSE, FLAG_PUNCH, FLAG_SOUND, FLAG_WIND,
};
pub use secondary::{
    apply_secondary_effect, secondary_effect_from_move, secondary_effects_from_move,
    self_effect_from_move, SecondaryEffect,
};
pub(crate) use status::{decrement_side_conditions, handle_status_move};

/// 技実行コンテキスト（M5）。
pub(crate) struct BattleContext<'a> {
    pub weather: Option<Weather>,
    pub field: Option<Field>,
    pub defender_action: Action,
    pub rng: &'a mut SmallRng,
    pub env_update: EnvUpdate,
}

/// 技実行結果（M5）。
pub(crate) enum MoveResult {
    Protected,
    Immune,
    Charged,
    Failed,
    Success { damage: u16 },
    Status { update: EnvUpdate },
}

/// 技実行の統合関数（M5）。
///
/// NOTE: 現状のbattle.rs実装に合わせ、env_update は MoveResult に含めて呼び出し側で適用する。
pub(crate) fn execute_move(
    move_data: &MoveData,
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    context: &mut BattleContext<'_>,
) -> MoveResult {
    // 1. まもる判定（READMEのexecute_move例に合わせる）
    if !flags::bypasses_protect(move_data)
        && defender.protect_active
        && !matches!(move_data.category, MoveCategory::Status)
    {
        return MoveResult::Protected;
    }

    // 2. 特性による無効化（M3）
    if flags::check_ability_immunity(defender, move_data) {
        return MoveResult::Immune;
    }

    // 3. 状態変化技（M2）
    if matches!(move_data.category, MoveCategory::Status) {
        let update = status::handle_status_move(
            attacker,
            defender,
            move_data,
            context.field,
            context.weather,
            0,
            1,
            context.rng,
        );
        return MoveResult::Status { update };
    }

    // 4. ダメージ計算の入口（M1/M4）
    // 実際のダメージ計算・命中/反動/吸収/多段/2ターン等は battle.rs の実装に委譲しつつ、
    // M5のAPIを提供する（後方互換のため）。
    let before = defender.current_hp;
    let move_id = crate::data::moves::normalize_move_name(move_data.name);
    let mut attacker_clone = attacker.clone();
    attacker_clone.moves = vec![move_id];
    let defender_clone = defender.clone();
    let mut dummy_state = BattleState::new(attacker_clone, defender_clone);
    dummy_state.weather = context.weather;
    dummy_state.field = context.field;
    crate::sim::battle::execute_move_impl(&mut dummy_state, 0, 0, context.defender_action, 1, context.rng);
    *attacker = dummy_state.pokemon_a;
    *defender = dummy_state.pokemon_b;
    let damage = before.saturating_sub(defender.current_hp);

    MoveResult::Success { damage }
}

/// battle.rs から技実行を呼び出すための統合エントリポイント（M5）。
pub(crate) fn execute_move_state(
    state: &mut BattleState,
    attacker_idx: usize,
    move_idx: usize,
    defender_action: Action,
    defender_idx: usize,
    rng: &mut SmallRng,
) {
    crate::sim::battle::execute_move_impl(state, attacker_idx, move_idx, defender_action, defender_idx, rng)
}

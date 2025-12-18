//! Status-move handling (M2).
//! Showdown参照:
//! - pokemon-showdown/sim/battle-actions.ts
//! - pokemon-showdown/data/moves.ts
//!
//! 実装対象（M2の追加分）:
//! - Court Change / Charge / Magic Coat / Telekinesis / Healing Wish / Lunar Dance
//! - 追加の場の状態: Mist / Safeguard / Tailwind / Lucky Chant / Aurora Veil

use crate::data::moves::{normalize_move_name, MoveData};
use crate::data::types::Type;
use crate::i18n::translate_pokemon;
use crate::sim::battle::{
    apply_stage_change, apply_status_with_field, format_status, screen_turns, EnvUpdate, Field,
    FieldEffect, HazardKind, HazardUpdate, ScreenUpdate, SideConditionKind, SideConditionUpdate,
    SideConditions, Weather, STAGE_SPD,
};
use crate::sim::pokemon::{Pokemon, Status};
use rand::rngs::SmallRng;

/// Handle status moves that alter the field, sides, or user state.
pub(crate) fn handle_status_move(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    move_data: &MoveData,
    field: Option<Field>,
    weather: Option<Weather>,
    trick_room_turns: u8,
    target_side_idx: usize,
    rng: &mut SmallRng,
) -> EnvUpdate {
    let mut update = EnvUpdate::default();
    let id = normalize_move_name(move_data.name);
    let attacker_side_idx = 1usize.saturating_sub(target_side_idx.min(1));

    match id.as_str() {
        // Defensive setup
        "magiccoat" => {
            attacker.magic_coat_active = true;
        }
        "charge" => {
            attacker.charge_active = true;
            let user = translate_pokemon(&attacker.species);
            let _ = apply_stage_change(attacker, &user, STAGE_SPD, 1);
        }
        "telekinesis" => {
            defender.telekinesis_turns = 3;
        }

        // Field / side manipulation
        "courtchange" => {
            update.court_change = true;
        }
        "healingwish" | "lunardance" => {
            update.healing_wish = Some(attacker_side_idx);
            attacker.current_hp = 0;
        }
        "trickroom" => {
            if trick_room_turns > 0 {
                update.trick_room_turns = Some(0);
            } else {
                update.trick_room_turns = Some(5);
            }
        }

        // Status
        "thunderwave" => {
            if defender.types[0] == Type::Ground || defender.types[1] == Type::Ground {
                println!("  しかし うまくきまらなかった！");
                return update;
            }
            if apply_status_with_field(defender, Status::Paralysis, false, field, rng) {
                println!(
                    "  {}は{}！",
                    translate_pokemon(&defender.species),
                    format_status(Status::Paralysis)
                );
            } else {
                println!("  しかし うまくきまらなかった！");
            }
        }

        // Substitute consumes 1/4 HP and creates a decoy.
        "substitute" => {
            let max_hp = attacker.stats.hp;
            let cost = (max_hp as u32 / 4).max(1) as u16;
            if attacker.current_hp <= cost || attacker.substitute_hp > 0 {
                println!("  しかし うまくきまらなかった！");
                return update;
            }
            attacker.current_hp = attacker.current_hp.saturating_sub(cost);
            attacker.substitute_hp = cost;
            println!("  {}はみがわりをだした！", translate_pokemon(&attacker.species));
        }

        // Screens (apply to user's side)
        "reflect" => {
            let turns = screen_turns(attacker);
            update.screen = Some(ScreenUpdate {
                target: attacker_side_idx,
                kind: FieldEffect::Reflect,
                turns,
            });
        }
        "lightscreen" => {
            let turns = screen_turns(attacker);
            update.screen = Some(ScreenUpdate {
                target: attacker_side_idx,
                kind: FieldEffect::LightScreen,
                turns,
            });
        }

        // Hazards
        "stealthrock" => {
            update.hazard = Some(HazardUpdate {
                target: target_side_idx,
                kind: HazardKind::StealthRock,
            });
        }
        "spikes" => {
            update.hazard = Some(HazardUpdate {
                target: target_side_idx,
                kind: HazardKind::Spikes,
            });
        }
        "toxicspikes" => {
            update.hazard = Some(HazardUpdate {
                target: target_side_idx,
                kind: HazardKind::ToxicSpikes,
            });
        }
        "stickyweb" => {
            update.hazard = Some(HazardUpdate {
                target: target_side_idx,
                kind: HazardKind::StickyWeb,
            });
        }

        // Side conditions (apply to user's side)
        "safeguard" => {
            update.side_condition = Some(SideConditionUpdate {
                target: attacker_side_idx,
                kind: SideConditionKind::Safeguard,
                turns: 5,
            });
        }
        "tailwind" => {
            update.side_condition = Some(SideConditionUpdate {
                target: attacker_side_idx,
                kind: SideConditionKind::Tailwind,
                turns: 4,
            });
        }
        "mist" => {
            update.side_condition = Some(SideConditionUpdate {
                target: attacker_side_idx,
                kind: SideConditionKind::Mist,
                turns: 5,
            });
        }
        "luckychant" => {
            update.side_condition = Some(SideConditionUpdate {
                target: attacker_side_idx,
                kind: SideConditionKind::LuckyChant,
                turns: 5,
            });
        }
        "auroraveil" => {
            if !matches!(weather, Some(Weather::Hail)) {
                println!("  しかし こうかがなかった！");
                return update;
            }
            update.side_condition = Some(SideConditionUpdate {
                target: attacker_side_idx,
                kind: SideConditionKind::AuroraVeil,
                turns: screen_turns(attacker),
            });
        }

        _ => {}
    }

    update
}

pub(crate) fn decrement_side_conditions(side: &mut SideConditions) {
    if side.reflect_turns > 0 {
        side.reflect_turns = side.reflect_turns.saturating_sub(1);
        if side.reflect_turns == 0 {
            println!("  リフレクターの こうかが きれた！");
        }
    }
    if side.light_screen_turns > 0 {
        side.light_screen_turns = side.light_screen_turns.saturating_sub(1);
        if side.light_screen_turns == 0 {
            println!("  ひかりのかべの こうかが きれた！");
        }
    }
    if side.mist_turns > 0 {
        side.mist_turns = side.mist_turns.saturating_sub(1);
        if side.mist_turns == 0 {
            println!("  しろいきりが きえた！");
        }
    }
    if side.safeguard_turns > 0 {
        side.safeguard_turns = side.safeguard_turns.saturating_sub(1);
        if side.safeguard_turns == 0 {
            println!("  しんぴのベールが きえた！");
        }
    }
    if side.tailwind_turns > 0 {
        side.tailwind_turns = side.tailwind_turns.saturating_sub(1);
        if side.tailwind_turns == 0 {
            println!("  おいかぜが やんだ！");
        }
    }
    if side.lucky_chant_turns > 0 {
        side.lucky_chant_turns = side.lucky_chant_turns.saturating_sub(1);
        if side.lucky_chant_turns == 0 {
            println!("  おまじないの こうかが きれた！");
        }
    }
    if side.aurora_veil_turns > 0 {
        side.aurora_veil_turns = side.aurora_veil_turns.saturating_sub(1);
        if side.aurora_veil_turns == 0 {
            println!("  オーロラベールの こうかが きれた！");
        }
    }
}

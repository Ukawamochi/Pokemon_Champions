use crate::data::moves::{normalize_move_name, MoveData};
use crate::sim::pokemon::Pokemon;

// Showdown: sim/dex-moves.ts#L1-L89 (flags)
pub const FLAG_CONTACT: &str = "contact";
pub const FLAG_SOUND: &str = "sound";
pub const FLAG_BULLET: &str = "bullet";
pub const FLAG_PULSE: &str = "pulse";
pub const FLAG_PUNCH: &str = "punch";
pub const FLAG_BITE: &str = "bite";
pub const FLAG_WIND: &str = "wind";
pub const FLAG_POWDER: &str = "powder";
pub const FLAG_PROTECT: &str = "protect";
pub const FLAG_MIRROR: &str = "mirror";
pub const FLAG_HEAL: &str = "heal";
pub const FLAG_METRONOME: &str = "metronome";
pub const FLAG_BYPASS_SUB: &str = "bypasssub";

pub fn move_has_flag(move_data: &MoveData, flag: &str) -> bool {
    move_data.flags.iter().any(|candidate| *candidate == flag)
}

pub fn is_contact_move(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_CONTACT)
}

pub fn is_sound_move(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_SOUND)
}

pub fn is_bullet_move(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_BULLET)
}

pub fn is_pulse_move(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_PULSE)
}

pub fn is_blocked_by_protect(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_PROTECT)
}

pub fn is_blocked_by_bulletproof(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_BULLET)
}

pub fn bypasses_protect(move_data: &MoveData) -> bool {
    !is_blocked_by_protect(move_data)
}

pub fn bypasses_substitute(move_data: &MoveData) -> bool {
    move_has_flag(move_data, FLAG_BYPASS_SUB) || is_sound_move(move_data)
}

pub fn affects_grounded_only(move_data: &MoveData) -> bool {
    // Placeholder for Showdown-style grounded checks; type immunities already handle most cases.
    // Showdown: battle-actions.ts#L1089-L1095 (target filtering around primary hit)
    matches!(normalize_move_name(move_data.name).as_str(), "thousandarrows")
}

pub fn check_ability_immunity(defender: &Pokemon, move_data: &MoveData) -> bool {
    // Showdown: pokemon.ts#L567-L612 (immunity checks; simplified)
    if defender.has_ability("Soundproof") && is_sound_move(move_data) {
        return true;
    }
    if defender.has_ability("Bulletproof") && is_blocked_by_bulletproof(move_data) {
        return true;
    }
    if move_data.priority > 0 && (defender.has_ability("Queenly Majesty") || defender.has_ability("Dazzling")) {
        return true;
    }
    false
}

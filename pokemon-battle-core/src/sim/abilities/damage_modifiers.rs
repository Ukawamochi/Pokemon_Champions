use crate::data::moves::{MoveCategory, MoveData};
use crate::data::types::Type;
use crate::sim::moves::flags::move_has_flag;
use crate::sim::pokemon::{Pokemon, Status};

// Showdown: battle.ts (ability damage modifiers are applied as chained modifiers)
pub(crate) fn attacker_damage_modifier(
    attacker: &Pokemon,
    move_data: &MoveData,
    move_type: Type,
    is_sandstorm: bool,
) -> f32 {
    let mut modifier = 1.0;

    // こんじょう (Guts): burn only in this project spec (physical 1.5x)
    if attacker.has_ability("Guts")
        && matches!(attacker.status, Some(Status::Burn))
        && matches!(move_data.category, MoveCategory::Physical)
    {
        modifier *= 1.5;
    }

    // てつのこぶし (Iron Fist): punch moves 1.2x
    if attacker.has_ability("Iron Fist") && move_has_flag(move_data, "punch") {
        modifier *= 1.2;
    }

    // すなのちから (Sand Force): in sandstorm, Rock/Ground/Steel moves 1.3x
    if attacker.has_ability("Sand Force")
        && is_sandstorm
        && matches!(move_type, Type::Rock | Type::Ground | Type::Steel)
    {
        modifier *= 1.3;
    }

    // ちからもち (Huge Power) / ヨガパワー (Pure Power): physical 2x
    if (attacker.has_ability("Huge Power") || attacker.has_ability("Pure Power"))
        && matches!(move_data.category, MoveCategory::Physical)
    {
        modifier *= 2.0;
    }

    // スロースタート (Slow Start): physical 0.5x (turn tracking is not implemented yet)
    if attacker.has_ability("Slow Start") && matches!(move_data.category, MoveCategory::Physical) {
        modifier *= 0.5;
    }

    modifier
}

pub(crate) fn defender_damage_modifier(
    defender: &Pokemon,
    move_data: &MoveData,
    type_effectiveness: f32,
) -> f32 {
    let mut modifier = 1.0;

    // ハードロック (Solid Rock) / フィルター (Filter): super effective damage x0.75
    if (defender.has_ability("Solid Rock") || defender.has_ability("Filter"))
        && type_effectiveness > 1.0
    {
        modifier *= 0.75;
    }

    // マルチスケイル (Multiscale): at full HP, damage x0.5
    if defender.has_ability("Multiscale") && defender.current_hp == defender.stats.hp {
        modifier *= 0.5;
    }

    // ファーコート (Fur Coat): physical damage x0.5 (approx: Defense x2)
    if defender.has_ability("Fur Coat") && matches!(move_data.category, MoveCategory::Physical) {
        modifier *= 0.5;
    }

    // かんそうはだ (Dry Skin): Fire damage x1.25
    if defender.has_ability("Dry Skin")
        && move_data.move_type.eq_ignore_ascii_case("fire")
    {
        modifier *= 1.25;
    }

    modifier
}

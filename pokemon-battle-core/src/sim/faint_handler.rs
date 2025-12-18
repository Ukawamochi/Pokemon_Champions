use crate::data::moves::MoveData;
use crate::sim::items::consumable::{can_consume_item, consume_item, has_item};
use crate::sim::moves::flags::is_contact_move;
use crate::sim::pokemon::Pokemon;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KoPrevention {
    Endure,
    Sturdy,
    FocusSash,
}

pub fn prevent_ko_if_applicable(defender: &mut Pokemon, damage: u16) -> (u16, Option<KoPrevention>) {
    if defender.current_hp == 0 || defender.current_hp == 1 {
        return (damage, None);
    }
    if damage < defender.current_hp {
        return (damage, None);
    }

    if defender.endure_active {
        return (defender.current_hp.saturating_sub(1), Some(KoPrevention::Endure));
    }

    if defender.has_ability("Sturdy") && defender.current_hp == defender.stats.hp {
        return (defender.current_hp.saturating_sub(1), Some(KoPrevention::Sturdy));
    }

    if defender.current_hp == defender.stats.hp && can_consume_item(defender) && has_item(defender, "focussash") {
        if consume_item(defender, "focussash") {
            return (defender.current_hp.saturating_sub(1), Some(KoPrevention::FocusSash));
        }
    }

    (damage, None)
}

pub fn apply_aftermath_if_applicable(
    attacker: &mut Pokemon,
    defender: &Pokemon,
    move_data: &MoveData,
) -> Option<u16> {
    if attacker.current_hp == 0 {
        return None;
    }
    if !defender.has_ability("Aftermath") {
        return None;
    }
    if !is_contact_move(move_data) {
        return None;
    }
    let dmg = (attacker.stats.hp as u32 / 4).max(1) as u16;
    attacker.take_damage(dmg);
    Some(dmg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::moves::get_move;
    use crate::sim::stats::Nature;

    fn make_pokemon(ability: &str, item: Option<&str>) -> Pokemon {
        Pokemon::new(
            "charizard",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec!["tackle".to_string()],
            ability,
            item.map(|name| name.to_string()),
        )
        .expect("species exists")
    }

    #[test]
    fn endure_prevents_ko_once() {
        let mut defender = make_pokemon("Blaze", None);
        defender.current_hp = 10;
        defender.endure_active = true;
        let (final_damage, prevention) = prevent_ko_if_applicable(&mut defender, 999);
        assert_eq!(final_damage, 9);
        assert_eq!(prevention, Some(KoPrevention::Endure));
    }

    #[test]
    fn sturdy_prevents_ko_at_full_hp() {
        let mut defender = make_pokemon("Sturdy", None);
        defender.current_hp = defender.stats.hp;
        let (final_damage, prevention) = prevent_ko_if_applicable(&mut defender, 999);
        assert_eq!(final_damage, defender.stats.hp - 1);
        assert_eq!(prevention, Some(KoPrevention::Sturdy));
    }

    #[test]
    fn focus_sash_prevents_ko_and_consumes() {
        let mut defender = make_pokemon("Blaze", Some("Focus Sash"));
        defender.current_hp = defender.stats.hp;
        let (final_damage, prevention) = prevent_ko_if_applicable(&mut defender, 999);
        assert_eq!(final_damage, defender.stats.hp - 1);
        assert_eq!(prevention, Some(KoPrevention::FocusSash));
        assert!(defender.item_consumed);
    }

    #[test]
    fn aftermath_damages_contact_attacker() {
        let move_data = get_move("tackle").expect("move exists");
        let mut attacker = make_pokemon("Blaze", None);
        let defender = make_pokemon("Aftermath", None);
        let hp_before = attacker.current_hp;
        let dmg = apply_aftermath_if_applicable(&mut attacker, &defender, &move_data).expect("should trigger");
        assert_eq!(attacker.current_hp, hp_before - dmg);
    }
}

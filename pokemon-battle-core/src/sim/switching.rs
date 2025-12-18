use crate::sim::pokemon::Pokemon;
use rand::rngs::SmallRng;
use rand::Rng;

// Showdown reference (switching / forced switch / trapping):
// - pokemon-showdown/sim/battle-actions.ts: forceSwitch / selfSwitch behavior is implemented around move resolution
// - pokemon-showdown/data/moves.ts: roar/whirlwind/uturn/voltswitch/meanlook/spiderweb definitions

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchKind {
    Voluntary,
    Forced,
    Pivot,
}

pub fn can_switch(pokemon: &Pokemon, kind: SwitchKind) -> bool {
    if pokemon.is_fainted() {
        return kind == SwitchKind::Forced;
    }
    if pokemon.trapped && kind != SwitchKind::Forced {
        return false;
    }
    true
}

pub fn is_pivot_move(move_id: &str) -> bool {
    matches!(move_id, "uturn" | "voltswitch")
}

pub fn is_trapping_move(move_id: &str) -> bool {
    matches!(move_id, "meanlook" | "spiderweb")
}

pub fn apply_trapping_move(target: &mut Pokemon) -> bool {
    if target.trapped {
        return false;
    }
    target.trapped = true;
    true
}

pub fn clear_trap(target: &mut Pokemon) {
    target.trapped = false;
}

pub fn pick_random_switch(bench: &[Pokemon], rng: &mut SmallRng) -> Option<usize> {
    let options: Vec<usize> = bench
        .iter()
        .enumerate()
        .filter_map(|(idx, pokemon)| (!pokemon.is_fainted()).then_some(idx))
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(options[rng.gen_range(0..options.len())])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::stats::Nature;

    fn mk_pokemon() -> Pokemon {
        Pokemon::new(
            "pikachu",
            50,
            [0; 6],
            [31; 6],
            Nature::Hardy,
            vec!["tackle".to_string()],
            "Static",
            None,
        )
        .unwrap()
    }

    #[test]
    fn trapped_blocks_voluntary_and_pivot_but_not_forced() {
        let mut p = mk_pokemon();
        p.trapped = true;
        assert!(!can_switch(&p, SwitchKind::Voluntary));
        assert!(!can_switch(&p, SwitchKind::Pivot));
        assert!(can_switch(&p, SwitchKind::Forced));
    }

    #[test]
    fn pivot_and_trap_move_ids() {
        assert!(is_pivot_move("uturn"));
        assert!(is_pivot_move("voltswitch"));
        assert!(!is_pivot_move("tackle"));
        assert!(is_trapping_move("meanlook"));
        assert!(is_trapping_move("spiderweb"));
        assert!(!is_trapping_move("roar"));
    }
}

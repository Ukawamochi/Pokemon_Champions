use crate::data::species::POKEDEX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Nature {
    Hardy,
    Lonely,
    Brave,
    Adamant,
    Naughty,
    Bold,
    Docile,
    Relaxed,
    Impish,
    Lax,
    Timid,
    Hasty,
    Serious,
    Jolly,
    Naive,
    Modest,
    Mild,
    Quiet,
    Bashful,
    Rash,
    Calm,
    Gentle,
    Sassy,
    Careful,
    Quirky,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Stat {
    Hp,
    Atk,
    Def,
    Spa,
    Spd,
    Spe,
}

pub fn stat_modifier(nature: Nature, stat: Stat) -> f32 {
    match nature {
        Nature::Hardy | Nature::Docile | Nature::Serious | Nature::Bashful | Nature::Quirky => 1.0,
        Nature::Lonely => bonus(stat, Stat::Atk, Stat::Def),
        Nature::Brave => bonus(stat, Stat::Atk, Stat::Spe),
        Nature::Adamant => bonus(stat, Stat::Atk, Stat::Spa),
        Nature::Naughty => bonus(stat, Stat::Atk, Stat::Spd),
        Nature::Bold => bonus(stat, Stat::Def, Stat::Atk),
        Nature::Relaxed => bonus(stat, Stat::Def, Stat::Spe),
        Nature::Impish => bonus(stat, Stat::Def, Stat::Spa),
        Nature::Lax => bonus(stat, Stat::Def, Stat::Spd),
        Nature::Timid => bonus(stat, Stat::Spe, Stat::Atk),
        Nature::Hasty => bonus(stat, Stat::Spe, Stat::Def),
        Nature::Jolly => bonus(stat, Stat::Spe, Stat::Spa),
        Nature::Naive => bonus(stat, Stat::Spe, Stat::Spd),
        Nature::Modest => bonus(stat, Stat::Spa, Stat::Atk),
        Nature::Mild => bonus(stat, Stat::Spa, Stat::Def),
        Nature::Quiet => bonus(stat, Stat::Spa, Stat::Spe),
        Nature::Rash => bonus(stat, Stat::Spa, Stat::Spd),
        Nature::Calm => bonus(stat, Stat::Spd, Stat::Atk),
        Nature::Gentle => bonus(stat, Stat::Spd, Stat::Def),
        Nature::Sassy => bonus(stat, Stat::Spd, Stat::Spe),
        Nature::Careful => bonus(stat, Stat::Spd, Stat::Spa),
    }
}

fn bonus(stat: Stat, boosted: Stat, lowered: Stat) -> f32 {
    if stat == boosted {
        1.1
    } else if stat == lowered {
        0.9
    } else {
        1.0
    }
}

pub fn calc_hp(base: u16, iv: u8, ev: u8, level: u8) -> u16 {
    let ev_quarter = (ev / 4) as u16;
    let base_value = base * 2 + iv as u16 + ev_quarter;
    let intermediate = (base_value * level as u16) / 100;
    intermediate + level as u16 + 10
}

pub fn calc_stat(base: u16, iv: u8, ev: u8, level: u8, nature_mod: f32) -> u16 {
    let ev_quarter = (ev / 4) as u16;
    let base_value = base * 2 + iv as u16 + ev_quarter;
    let intermediate = (base_value * level as u16) / 100;
    let stat = (intermediate + 5) as f32 * nature_mod;
    stat.floor() as u16
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatsSet {
    pub hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
}

impl StatsSet {
    pub fn from_species(
        species: &str,
        level: u8,
        evs: [u8; 6],
        ivs: [u8; 6],
        nature: Nature,
    ) -> Option<Self> {
        let id = normalize_id(species);
        let data = POKEDEX.get(id.as_str())?;
        let base = data.base_stats;
        Some(Self {
            hp: calc_hp(base.hp as u16, ivs[0], evs[0], level),
            atk: calc_stat(base.atk as u16, ivs[1], evs[1], level, stat_modifier(nature, Stat::Atk)),
            def: calc_stat(base.def as u16, ivs[2], evs[2], level, stat_modifier(nature, Stat::Def)),
            spa: calc_stat(base.spa as u16, ivs[3], evs[3], level, stat_modifier(nature, Stat::Spa)),
            spd: calc_stat(base.spd as u16, ivs[4], evs[4], level, stat_modifier(nature, Stat::Spd)),
            spe: calc_stat(base.spe as u16, ivs[5], evs[5], level, stat_modifier(nature, Stat::Spe)),
        })
    }
}

fn normalize_id(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
fn test_charizard_lv50_adamant() {
    let evs = [0, 252, 0, 0, 4, 252];
    let ivs = [31; 6];
    let set = StatsSet::from_species("charizard", 50, evs, ivs, Nature::Adamant)
        .expect("Charizard data should be available");
        assert_eq!(set.hp, 153);
        assert_eq!(set.atk, 149);
        assert_eq!(set.def, 98);
        assert_eq!(set.spa, 116);
        assert_eq!(set.spd, 106);
        assert_eq!(set.spe, 152);
    }

    #[test]
    fn test_dragonite_lv50_neutral() {
        let evs = [0; 6];
        let ivs = [0; 6];
        let set = StatsSet::from_species("dragonite", 50, evs, ivs, Nature::Hardy)
            .expect("Dragonite data should be available");
        assert_eq!(set.hp, 151);
        assert_eq!(set.atk, 139);
        assert_eq!(set.def, 100);
        assert_eq!(set.spa, 105);
        assert_eq!(set.spd, 105);
        assert_eq!(set.spe, 85);
    }

    #[test]
    fn test_nature_modifiers() {
        assert!((stat_modifier(Nature::Adamant, Stat::Atk) - 1.1).abs() < f32::EPSILON);
        assert!((stat_modifier(Nature::Adamant, Stat::Spa) - 0.9).abs() < f32::EPSILON);
        assert_eq!(stat_modifier(Nature::Adamant, Stat::Def), 1.0);
    }
}

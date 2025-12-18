use crate::sim::pokemon::Pokemon;
use crate::data::moves::normalize_move_name;
use crate::sim::stats::Nature;
use anyhow::{anyhow, Context, Result};

pub fn parse_showdown_team(text: &str) -> Result<Vec<Pokemon>> {
    let mut team = Vec::new();
    for (idx, chunk) in text.split("\n\n").enumerate() {
        let trimmed = chunk.trim();
        let entry = parse_entry(trimmed).with_context(|| format!("Failed to parse team entry {}", idx + 1))?;
        if let Some(pokemon) = entry {
            team.push(pokemon);
        }
    }
    Ok(team)
}

fn parse_entry(entry: &str) -> Result<Option<Pokemon>> {
    if entry.is_empty() {
        return Ok(None);
    }
    let mut species_line = None;
    let mut ability = None;
    let mut item = None;
    let mut level = 50u8;
    let mut nature = Nature::Hardy;
    let mut evs = [0u8; 6];
    let mut ivs = [31u8; 6];
    let mut moves = Vec::new();

    for line in entry.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Ability:") {
            ability = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Level:") {
            level = rest.trim().parse().unwrap_or(level);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("EVs:") {
            parse_stat_line(rest.trim(), &mut evs);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("IVs:") {
            parse_stat_line(rest.trim(), &mut ivs);
            continue;
        }
        if trimmed.ends_with("Nature") {
            let nature_name = trimmed.trim_end_matches("Nature").trim();
            nature = parse_nature(nature_name);
            continue;
        }
        if trimmed.starts_with('-') {
            let move_name = trimmed.trim_start_matches('-').trim();
            if !move_name.is_empty() {
                moves.push(normalize_move_name(move_name));
            }
            continue;
        }
        if species_line.is_none() {
            species_line = Some(trimmed.to_string());
        }
    }

    let species_line = species_line.ok_or_else(|| anyhow!("Species line is missing"))?;
    let species_parts: Vec<&str> = species_line.split('@').map(|s| s.trim()).collect();
    let species_name = species_parts
        .get(0)
        .ok_or_else(|| anyhow!("Failed to read species name"))?
        .to_string();
    if let Some(item_str) = species_parts.get(1) {
        if !item_str.is_empty() {
            item = Some(item_str.to_string());
        }
    }

    let ability = ability.unwrap_or_else(|| "No Ability".to_string());
    let pokemon = Pokemon::new(species_name.clone(), level, evs, ivs, nature, moves, ability, item)
        .with_context(|| format!("Failed to build Pokémon '{}'", species_name))?;
    Ok(Some(pokemon))
}

fn parse_stat_line(line: &str, stats: &mut [u8; 6]) {
    for part in line.split('/') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut iter = trimmed.split_whitespace();
        if let Some(value_str) = iter.next() {
            if let Some(stat_name) = iter.next() {
                if let Ok(value) = value_str.parse::<u8>() {
                    if let Some(idx) = stat_index(stat_name) {
                        stats[idx] = value;
                    }
                }
            }
        }
    }
}

fn stat_index(name: &str) -> Option<usize> {
    match name.to_lowercase().as_str() {
        "hp" => Some(0),
        "atk" => Some(1),
        "def" => Some(2),
        "spa" | "spatk" => Some(3),
        "spd" | "spdef" => Some(4),
        "spe" => Some(5),
        _ => None,
    }
}

fn parse_nature(name: &str) -> Nature {
    match name.trim().to_lowercase().as_str() {
        "hardy" => Nature::Hardy,
        "lonely" => Nature::Lonely,
        "brave" => Nature::Brave,
        "adamant" => Nature::Adamant,
        "naughty" => Nature::Naughty,
        "bold" => Nature::Bold,
        "docile" => Nature::Docile,
        "relaxed" => Nature::Relaxed,
        "impish" => Nature::Impish,
        "lax" => Nature::Lax,
        "timid" => Nature::Timid,
        "hasty" => Nature::Hasty,
        "serious" => Nature::Serious,
        "jolly" => Nature::Jolly,
        "naive" => Nature::Naive,
        "modest" => Nature::Modest,
        "mild" => Nature::Mild,
        "quiet" => Nature::Quiet,
        "bashful" => Nature::Bashful,
        "rash" => Nature::Rash,
        "calm" => Nature::Calm,
        "gentle" => Nature::Gentle,
        "sassy" => Nature::Sassy,
        "careful" => Nature::Careful,
        "quirky" => Nature::Quirky,
        _ => Nature::Hardy,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_move_name;
    use super::parse_showdown_team;
    use anyhow::Result;

    #[test]
    fn parse_simple_export() -> Result<()> {
        let data = "\
Charizard @ Charcoal
Ability: Blaze
Level: 50
EVs: 252 Atk / 4 SpD / 252 Spe
Adamant Nature
- Flare Blitz
- Earthquake
- Dragon Claw
- Roost

Blastoise @ Leftovers
Ability: Torrent
Level: 50
EVs: 252 HP / 252 Def / 4 SpA
Modest Nature
- Hydro Pump
- Ice Beam
- Dark Pulse
- Rapid Spin
";
        let team = parse_showdown_team(data)?;
        assert_eq!(team.len(), 2);
        assert_eq!(team[0].species.to_lowercase(), "charizard");
        assert_eq!(team[0].ability, "Blaze");
        assert_eq!(team[0].moves.len(), 4);
        assert!(team[0].moves.contains(&normalize_move_name("Flare Blitz")));
        assert_eq!(team[1].item.as_deref(), Some("Leftovers"));
        assert_eq!(team[1].moves[0], normalize_move_name("Hydro Pump"));
        Ok(())
    }

    #[test]
    fn parse_minimal_defaults() -> Result<()> {
        let data = "Pikachu\n- Thunderbolt";
        let team = parse_showdown_team(data)?;
        assert_eq!(team.len(), 1);
        assert_eq!(team[0].level, 50);
        assert_eq!(team[0].moves[0], normalize_move_name("Thunderbolt"));
        assert_eq!(team[0].ability, "No Ability");
        assert!(team[0].stats.hp > 0);
        Ok(())
    }
}

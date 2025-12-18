use serde_json::json;

#[derive(Clone, Debug, Default)]
pub struct BattleLogger {
    formatid: String,
    log: Vec<String>,
}

impl BattleLogger {
    pub fn new() -> Self {
        Self {
            formatid: "gen9customgame".to_string(),
            log: Vec::new(),
        }
    }

    pub fn new_with_format(formatid: impl Into<String>) -> Self {
        Self {
            formatid: formatid.into(),
            log: Vec::new(),
        }
    }

    pub fn log_turn(&mut self, turn: usize) {
        self.log.push(format!("|turn|{turn}"));
    }

    pub fn log_move(&mut self, source: &str, move_id: &str, target: &str) {
        self.log
            .push(format!("|move|{source}|{move_id}|{target}"));
    }

    pub fn log_damage(&mut self, target: &str, hp: u16, max_hp: u16) {
        self.log.push(format!("|-damage|{target}|{hp}/{max_hp}"));
    }

    pub fn log_heal(&mut self, target: &str, hp: u16, max_hp: u16) {
        self.log.push(format!("|-heal|{target}|{hp}/{max_hp}"));
    }

    pub fn log_status(&mut self, target: &str, status: &str) {
        self.log.push(format!("|-status|{target}|{status}"));
    }

    pub fn log_switch(&mut self, pokemon: &str, species: &str, hp: u16, max_hp: u16) {
        self.log
            .push(format!("|switch|{pokemon}|{species}|{hp}/{max_hp}"));
    }

    pub fn log_win(&mut self, winner: &str) {
        self.log.push(format!("|win|{winner}"));
    }

    pub fn log_tie(&mut self) {
        self.log.push("|tie|".to_string());
    }

    pub fn log_lines(&self) -> &[String] {
        &self.log
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "formatid": self.formatid,
            "log": self.log,
        })
    }
}

pub fn showdown_ident(side_idx: usize, species: &str) -> String {
    // singles only: p1a / p2a
    let side = if side_idx == 0 { "p1a" } else { "p2a" };
    format!("{side}: {species}")
}


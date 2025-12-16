use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

fn default_accuracy() -> f32 {
    100.0
}

fn default_priority() -> i32 {
    0
}

#[derive(Debug, Clone, Deserialize)]
pub struct Move {
    pub name: String,
    #[serde(rename = "type")]
    pub move_type: String,
    pub category: MoveCategory,
    #[serde(default)]
    pub power: u32,
    #[serde(default = "default_accuracy")]
    pub accuracy: f32,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(flatten, default)]
    pub extras: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stats {
    pub hp: u32,
    pub atk: u32,
    pub def: u32,
    pub spa: u32,
    pub spd: u32,
    pub spe: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pokemon {
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    pub stats: Stats,
    #[serde(default)]
    pub moves: Vec<Move>,
    #[serde(flatten, default)]
    pub extras: HashMap<String, serde_json::Value>,
}

impl Pokemon {
    pub fn initial_hp(&self) -> i32 {
        self.stats.hp as i32
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamsFile {
    pub team_a: Vec<Pokemon>,
    pub team_b: Vec<Pokemon>,
    #[serde(flatten, default)]
    pub extras: HashMap<String, serde_json::Value>,
}

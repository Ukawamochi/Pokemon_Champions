use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusCondition {
    Burn,
    Paralysis,
    Sleep,
    Poison,
    Toxic,
    Freeze,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weather {
    Sun,
    Rain,
    Sand,
    Hail,
    Snow,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StatBoosts {
    #[serde(default)]
    pub atk: i8,
    #[serde(default)]
    pub def: i8,
    #[serde(default)]
    pub spa: i8,
    #[serde(default)]
    pub spd: i8,
    #[serde(default)]
    pub spe: i8,
    #[serde(default)]
    pub acc: i8,
    #[serde(default)]
    pub eva: i8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecondaryEffect {
    pub chance: f32,
    #[serde(default)]
    pub status: Option<StatusCondition>,
    #[serde(default)]
    pub boosts: Option<StatBoosts>,
    #[serde(default)]
    pub self_boosts: Option<StatBoosts>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HazardMove {
    Stealthrock,
    Spikes,
    Toxicspikes,
}

fn default_accuracy() -> f32 {
    100.0
}

fn default_priority() -> i32 {
    0
}

fn default_pp() -> u8 {
    10
}

fn default_crit() -> u8 {
    0
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MultiHit {
    pub min_hits: u8,
    pub max_hits: u8,
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
    #[serde(default = "default_pp")]
    pub pp: u8,
    #[serde(default = "default_crit")]
    pub crit_rate: u8,
    #[serde(default)]
    pub secondary: Option<SecondaryEffect>,
    #[serde(default)]
    pub recoil: Option<(u8, u8)>, // numerator, denominator
    #[serde(default)]
    pub drain: Option<(u8, u8)>, // numerator, denominator
    #[serde(default)]
    pub boosts: Option<StatBoosts>,
    #[serde(default)]
    pub self_boosts: Option<StatBoosts>,
    #[serde(default)]
    pub status: Option<StatusCondition>,
    #[serde(default)]
    pub status_chance: Option<f32>,
    #[serde(default)]
    pub set_weather: Option<Weather>,
    #[serde(default)]
    pub hazard: Option<HazardMove>,
    #[serde(default)]
    pub protect: bool,
    #[serde(default)]
    pub switch_after: bool,
    #[serde(default)]
    pub multihit: Option<MultiHit>,
    #[serde(default = "default_false")]
    pub trick_room: bool,
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
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub ability: Option<String>,
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

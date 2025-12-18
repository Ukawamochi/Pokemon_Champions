//! Core battle engine and data extracted from Pokemon Showdown.
//!
//! The main entry point for step-based simulations is [`engine::BattleEngine`].

pub mod data;
pub mod battle_logger;
pub mod engine;
pub mod i18n;
pub mod parser;
pub mod sim;

pub use parser::parse_showdown_team;

/// Commonly used exports for external consumers.
pub mod prelude {
    pub use crate::engine::{BattleEngine, Player, StepResult};
    pub use crate::parser::parse_showdown_team;
    pub use crate::sim::battle::{Action, BattleResult, BattleState, Field, FieldEffect, Weather};
    pub use crate::sim::Pokemon;
}

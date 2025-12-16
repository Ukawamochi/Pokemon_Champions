#[derive(Clone, Copy, Debug)]
pub enum PriorityEffect {
    QuickClaw,
    Custap,
    Negative,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemEffect {
    pub name: &'static str,
    pub priority_frac: Option<PriorityEffect>,
    pub speed_mult: Option<f32>,
    pub atk_mult: Option<f32>,
    pub spa_mult: Option<f32>,
    pub def_mult: Option<f32>,
    pub spd_mult: Option<f32>,
    pub life_orb: bool,
    pub choice_stat: Option<&'static str>,
    pub sash_like: bool,
    pub sturdy_like: bool,
}

impl Default for ItemEffect {
    fn default() -> Self {
        ItemEffect {
            name: "",
            priority_frac: None,
            speed_mult: None,
            atk_mult: None,
            spa_mult: None,
            def_mult: None,
            spd_mult: None,
            life_orb: false,
            choice_stat: None,
            sash_like: false,
            sturdy_like: false,
        }
    }
}

mod generated {
    #![allow(clippy::all)]
    use super::{ItemEffect, PriorityEffect};
    include!(env!("ITEMS_GEN"));
}

pub use generated::ITEM_TABLE;

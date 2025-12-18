use crate::sim::battle::BattleState;
use crate::sim::pokemon::Pokemon;
use rand::rngs::SmallRng;
use std::collections::HashMap;

// Showdown reference:
// - Ability schema & loading: pokemon-showdown/sim/dex-abilities.ts#L5-L129
// - Event system core: pokemon-showdown/sim/battle.ts#L465-L475 (eachEvent) + #L758-L880 (runEvent)

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AbilityTrigger {
    OnStart,          // 場に出た時
    OnDamagingHit,    // ダメージを受けた時
    OnBeforeMove,     // 技使用前
    OnAfterMove,      // 技使用後
    OnModifyAtk,      // 攻撃力補正
    OnModifyDef,      // 防御力補正
    OnWeather,        // 天候による効果
    OnStatusImmunity, // 状態異常無効化
    OnFaint,          // ひんし時
    OnSwitchIn,       // 交代時
    OnEndOfTurn,      // ターン終了時
}

impl AbilityTrigger {
    /// Showdown event id mapping (for cross-reference / debugging).
    pub const fn showdown_event(self) -> &'static str {
        match self {
            AbilityTrigger::OnStart => "Start",
            AbilityTrigger::OnDamagingHit => "DamagingHit",
            AbilityTrigger::OnBeforeMove => "BeforeMove",
            AbilityTrigger::OnAfterMove => "AfterMove",
            AbilityTrigger::OnModifyAtk => "ModifyAtk",
            AbilityTrigger::OnModifyDef => "ModifyDef",
            AbilityTrigger::OnWeather => "Weather",
            AbilityTrigger::OnStatusImmunity => "TryImmunity",
            AbilityTrigger::OnFaint => "Faint",
            AbilityTrigger::OnSwitchIn => "SwitchIn",
            AbilityTrigger::OnEndOfTurn => "Residual",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectResult {
    NoEffect,
    Applied,
    Blocked,
}

pub struct AbilityContext<'a> {
    pub pokemon: &'a mut Pokemon,
    pub opponent: &'a mut Pokemon,
    pub state: &'a mut BattleState,
    pub rng: &'a mut SmallRng,
}

pub trait AbilityEffect: Send + Sync {
    fn on_trigger(&self, trigger: AbilityTrigger, context: &mut AbilityContext<'_>) -> EffectResult;
}

pub struct AbilityRegistry {
    effects: HashMap<String, Box<dyn AbilityEffect>>,
}

impl AbilityRegistry {
    pub fn new() -> Self {
        Self {
            effects: HashMap::new(),
        }
    }

    pub fn register(&mut self, ability_id: impl Into<String>, effect: Box<dyn AbilityEffect>) {
        self.effects.insert(ability_id.into(), effect);
    }

    pub fn get(&self, ability_id: &str) -> Option<&dyn AbilityEffect> {
        self.effects.get(ability_id).map(|b| b.as_ref())
    }

    pub fn trigger(
        &self,
        ability_id: &str,
        trigger: AbilityTrigger,
        context: &mut AbilityContext<'_>,
    ) -> EffectResult {
        let Some(effect) = self.get(ability_id) else {
            return EffectResult::NoEffect;
        };
        effect.on_trigger(trigger, context)
    }
}


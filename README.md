# Pokemon Showdown Rust Library
Rust製ポケモン対戦ルールエンジン - 全ルール完全実装プロジェクト

## 概要
Pokemon Showdownの全ルール（技・特性・もちもの）をRustで完全実装し、高速な対戦シミュレーションを実現します。

### 現在の実装状況（基盤）
- ✅ ワークスペース構成（pokemon-battle-core, pokemon-battle-cli）
- ✅ データ自動生成（extract_data.js → 950技, 300特性, 500アイテム）
- ✅ ダメージ計算基盤（damage.rs）
- ✅ 状態異常処理（pokemon.rs）
- ✅ 天候・フィールドシステム（weather_field.rs）
- ✅ ひんし判定（faint_handler.rs）
- ✅ テスト生成ツール（generate_showdown_tests.js）
- ✅ 差分解析ツール本体（diff_analyzer.rs）

### 現在の実装状況（コンテンツ）
- ⚠️ 技実装: 約100/950種類 (10%) - moves/{attacking,status,secondary,flags}.rs
- ⚠️ 特性実装: 約15/300種類 (5%) - abilities/{damage_modifiers,events,status_abilities,misc_abilities}.rs
- ⚠️ もちもの実装: 約10/500種類 (2%) - items/{battle_items,type_items,consumable}.rs
- ❌ メガシンカ: 未実装
- ❌ ダイマックス: 未実装
- ❌ Z技: 未実装
- ❌ テラスタル: 未実装
- ⚠️ CI/CD統合: 部分実装（V1形式対応とワークフロー定義が未完）

### 目標
Pokemon Showdown Gen 9 完全互換実装（全世代Gen 1-9対応）

---

## セットアップ

```bash
# 1. Node.js依存関係のインストール
npm install

# 2. Pokemon Showdownデータの準備
cd pokemon-showdown && npm ci && cd ..

# 3. Rustビルド（データ自動生成含む）
cargo build

# 4. テスト実行
cargo test
```

---

## 🎯 全ルール実装タスク（Codex並列実行用）

### 【重要】並列実行の原則
- 各タスクは**異なるファイル**を編集します
- 同じファイルを複数タスクで編集しないでください
- 依存関係のあるタスクは順序を守ってください
- Showdownの変数名・処理順序を可能な限り保持
- 全ての実装にShowdown行番号コメントを追加

---

## フェーズ1: 技システムの完全実装（並列5タスク）

### タスク M1: 攻撃技モジュールの実装

**編集ファイル:** `pokemon-battle-core/src/sim/moves/attacking.rs` (既存ファイルの拡張)

**目的:** 全攻撃技の効果を実装

**現状:** 反動技、吸収技、連続技、チャージ技、一撃必殺技の基本処理は実装済み

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts` L1583-L1856 (runMoveAction)
- `pokemon-showdown/data/moves.ts` の各技定義

**実施内容:**

1. 既存関数の拡張:
   - `apply_recoil_damage()` - すてみタックル、フレアドライブ、ウッドハンマー等
   - `apply_drain()` - ギガドレイン、ドレインパンチ、パラボラチャージ等
   - `calculate_multihit_count()` - タネマシンガン、みだれづき、スイープビンタ等
   - `handle_charging_move()` - ソーラービーム、そらをとぶ、ゴーストダイブ等
   - `handle_ohko_move()` - つのドリル、ぜったいれいど、ハサミギロチン、じわれ

2. 新規追加: 優先度付き技の処理:
   ```rust
   pub fn get_move_priority(move_data: &MoveData, attacker: &Pokemon, field: Option<Field>) -> i8 {
       // Showdown: pokemon.ts#L892-L910
       let base_priority = move_data.priority;
       
       // グラススライダー: グラスフィールドで+1
       if move_data.name == "Grassy Glide" && field == Some(Field::Grassy) {
           return base_priority + 1;
       }
       
       base_priority
   }
   ```

3. 新規追加: 威力変動技:
   ```rust
   pub fn calculate_variable_power(
       move_data: &MoveData,
       attacker: &Pokemon,
       defender: &Pokemon,
       weather: Option<Weather>,
       field: Option<Field>,
   ) -> u16 {
       // Showdown: battle-actions.ts#L1205-L1289
       match move_data.name {
           "Eruption" | "Water Spout" => {
               // HP依存威力
               (150 * attacker.current_hp / attacker.stats.hp) as u16
           }
           "Flail" | "Reversal" => {
               // HP低下で威力上昇
               let ratio = attacker.current_hp * 48 / attacker.stats.hp;
               if ratio <= 1 { 200 }
               else if ratio <= 4 { 150 }
               else if ratio <= 9 { 100 }
               else if ratio <= 16 { 80 }
               else if ratio <= 32 { 40 }
               else { 20 }
           }
           "Electro Ball" => {
               // 素早さ比で威力変動
               let speed_ratio = attacker.effective_speed() * 100 / defender.effective_speed();
               if speed_ratio >= 400 { 150 }
               else if speed_ratio >= 300 { 120 }
               else if speed_ratio >= 200 { 80 }
               else if speed_ratio >= 100 { 60 }
               else { 40 }
           }
           "Gyro Ball" => {
               // 相手の素早さ/自分の素早さ * 25
               ((25 * defender.effective_speed() / attacker.effective_speed()).min(150)) as u16
           }
           _ => move_data.base_power.unwrap_or(0),
       }
   }
   ```

**成果物:**
- `attacking.rs` の拡張（既存 + 約200行追加）
- 威力変動技、優先度変動技の完全対応

---

### タスク M2: 状態変化技モジュールの拡張

**編集ファイル:** `pokemon-battle-core/src/sim/moves/status.rs` (既存ファイルの拡張)

**目的:** 残りの状態変化技を実装

**現状:** 約70種類の状態変化技が実装済み（天候、フィールド、壁、設置技、回復技、能力変化技等）

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts` L891-L1125
- `pokemon-showdown/data/moves.ts` の状態変化技定義

**実施内容:**

1. 未実装の状態変化技を追加:
   - コートチェンジ (Court Change): 場の効果を入れ替え
   - じゅうでん (Charge): 次の電気技威力2倍
   - マジックコート (Magic Coat): 状態変化技を跳ね返す
   - テレキネシス (Telekinesis): 3ターン浮遊状態
   - いやしのねがい (Healing Wish): 自分ひんし、交代先全回復
   - みかづきのまい (Lunar Dance): 自分ひんし、交代先全回復+PP回復

2. 場の状態管理の拡張:
   ```rust
   pub enum SideCondition {
       Reflect { turns: u8 },
       LightScreen { turns: u8 },
       Mist { turns: u8 },
       Safeguard { turns: u8 },
       Tailwind { turns: u8 },
       LuckyChant { turns: u8 },
       AuroraVeil { turns: u8 },
   }
   ```

3. ターン経過処理:
   ```rust
   pub fn decrement_side_conditions(side: &mut SideState) {
       // Showdown: side.ts#L234-L267
       if let Some(ref mut reflect) = side.reflect {
           reflect.turns = reflect.turns.saturating_sub(1);
           if reflect.turns == 0 {
               side.reflect = None;
           }
       }
       // ... 他の場の状態も同様
   }
   ```

**成果物:**
- `status.rs` の拡張（約150行追加）
- 場の状態管理システムの完成

---

### タスク M3: 技フラグシステムの実装

**編集ファイル:** `pokemon-battle-core/src/sim/moves/flags.rs` (既存ファイルの拡張)

**目的:** 技フラグに基づく処理を実装

**現状:** `move_has_flag()` 関数のみ実装済み

**参照元:**
- `pokemon-showdown/sim/dex-moves.ts` L1-L89 (フラグ一覧とコメント)
- `pokemon-showdown/sim/battle-actions.ts` のフラグ判定処理

**実施内容:**

1. フラグ一覧の定義:
   ```rust
   // Showdown: sim/dex-moves.ts#L1-L89
   pub const FLAG_CONTACT: &str = "contact";       // 接触技
   pub const FLAG_SOUND: &str = "sound";           // 音技
   pub const FLAG_BULLET: &str = "bullet";         // 弾技
   pub const FLAG_PULSE: &str = "pulse";           // 波動技
   pub const FLAG_PUNCH: &str = "punch";           // パンチ技
   pub const FLAG_BITE: &str = "bite";             // 噛む技
   pub const FLAG_WIND: &str = "wind";             // 風技
   pub const FLAG_POWDER: &str = "powder";         // 粉技
   pub const FLAG_PROTECT: &str = "protect";       // まもる貫通しない
   pub const FLAG_MIRROR: &str = "mirror";         // マジックコートで跳ね返る
   pub const FLAG_HEAL: &str = "heal";             // 回復技
   pub const FLAG_METRONOME: &str = "metronome";   // ゆびをふる対象
   ```

2. フラグベース判定関数:
   ```rust
   pub fn is_contact_move(move_data: &MoveData) -> bool {
       move_has_flag(move_data, FLAG_CONTACT)
   }
   
   pub fn is_sound_move(move_data: &MoveData) -> bool {
       move_has_flag(move_data, FLAG_SOUND)
   }
   
   pub fn is_blocked_by_protect(move_data: &MoveData) -> bool {
       move_has_flag(move_data, FLAG_PROTECT)
   }
   
   pub fn is_blocked_by_bulletproof(move_data: &MoveData) -> bool {
       move_has_flag(move_data, FLAG_BULLET)
   }
   
   pub fn affects_grounded_only(move_data: &MoveData) -> bool {
       // Showdown: battle-actions.ts#L1089-L1095
       matches!(move_data.name, 
           "Thousand Arrows" | _ // じわれ等の地面技
       )
   }
   ```

3. 特性との連携:
   ```rust
   pub fn check_ability_immunity(
       defender: &Pokemon,
       move_data: &MoveData,
   ) -> bool {
       // Showdown: pokemon.ts#L567-L612
       match defender.ability.as_str() {
           "Soundproof" if is_sound_move(move_data) => true,
           "Bulletproof" if is_blocked_by_bulletproof(move_data) => true,
           "Queenly Majesty" | "Dazzling" if move_data.priority > 0 => true,
           _ => false,
       }
   }
   ```

**成果物:**
- `flags.rs` の拡張（約100行追加）
- フラグベース処理システムの完成

---

### タスク M4: 技の追加効果システム

**編集ファイル:** `pokemon-battle-core/src/sim/moves/secondary.rs` (既存ファイルの拡張)

**目的:** 技の追加効果を完全実装

**現状:** `SecondaryEffect` 構造体と `apply_secondary_effect()` 関数は実装済み

**参照元:**
- `pokemon-showdown/data/moves.ts` の各技の `secondary` フィールド
- `pokemon-showdown/sim/battle-actions.ts` L1583-L1677

**実施内容:**

1. SecondaryEffect構造体の拡張:
   ```rust
   #[derive(Clone, Debug)]
   pub struct SecondaryEffect {
       pub chance: u8,              // 発動確率（%）
       pub status: Option<Status>,  // 状態異常
       pub volatile_status: Option<String>,  // こわい顔、アンコール等
       pub boosts: Option<BTreeMap<String, i8>>,  // 能力変化
       pub target_self: bool,       // 自分に効果
       pub side_effect: Option<SideEffect>,  // 場の効果
   }
   
   #[derive(Clone, Debug)]
   pub enum SideEffect {
       Hazard(HazardKind),
       Screen(FieldEffect),
       Weather(Weather),
       Field(Field),
   }
   ```

2. 追加効果の適用:
   ```rust
   pub fn apply_secondary_effect(
       attacker: &mut Pokemon,
       defender: &mut Pokemon,
       effect: &SecondaryEffect,
       rng: &mut SmallRng,
   ) -> bool {
       // Showdown: battle-actions.ts#L1640-L1677
       if effect.chance == 0 {
           return false;
       }
       
       let chance = (effect.chance as f64 / 100.0).clamp(0.0, 1.0);
       if !rng.gen_bool(chance) {
           return false;
       }
       
       let target = if effect.target_self { attacker } else { defender };
       
       // 状態異常
       if let Some(status) = effect.status {
           target.set_status(status);
       }
       
       // 能力変化
       if let Some(ref boosts) = effect.boosts {
           for (stat, amount) in boosts {
               apply_boost(target, stat, *amount);
           }
       }
       
       // 場の効果
       if let Some(ref side_effect) = effect.side_effect {
           // ... 場の効果処理
       }
       
       true
   }
   ```

3. 代表的な追加効果の例:
   ```rust
   // 10まんボルト: 10%まひ
   secondary: Some(SecondaryEffect {
       chance: 10,
       status: Some(Status::Paralysis),
       volatile_status: None,
       boosts: None,
       target_self: false,
       side_effect: None,
   })
   
   // れいとうビーム: 10%こおり
   secondary: Some(SecondaryEffect {
       chance: 10,
       status: Some(Status::Freeze),
       ...
   })
   
   // サイコブースト: 100%自分特攻-1
   secondary: Some(SecondaryEffect {
       chance: 100,
       boosts: Some([("spa".to_string(), -1)].into()),
       target_self: true,
       ...
   })
   ```

**成果物:**
- `secondary.rs` の拡張（約150行追加）
- 全追加効果パターンの対応

---

### タスク M5: 技モジュール統合とテスト

**編集ファイル:** 
- `pokemon-battle-core/src/sim/moves/mod.rs` (既存ファイルの拡張)
- `pokemon-battle-core/tests/moves_test.rs` (新規作成)

**目的:** 技モジュール全体を統合し、包括的なテストを実装

**参照元:**
- `pokemon-showdown/test/sim/moves/` のテストファイル群

**実施内容:**

1. mod.rsの統合:
   ```rust
   pub mod attacking;
   pub mod status;
   pub mod flags;
   pub mod secondary;
   
   pub use attacking::{
       apply_recoil_damage, apply_drain, calculate_multihit_count,
       handle_charging_move, handle_ohko_move, calculate_variable_power,
       get_move_priority,
   };
   pub use status::{handle_status_move, decrement_side_conditions};
   pub use flags::{move_has_flag, is_contact_move, is_sound_move, check_ability_immunity};
   pub use secondary::{secondary_effect_from_move, apply_secondary_effect};
   
   // 技実行の統合関数
   pub fn execute_move(
       move_data: &MoveData,
       attacker: &mut Pokemon,
       defender: &mut Pokemon,
       context: &BattleContext,
   ) -> MoveResult {
       // Showdown: battle-actions.ts#L1050-L1289
       
       // 1. まもる判定
       if is_blocked_by_protect(move_data) && defender.protect_active {
           return MoveResult::Protected;
       }
       
       // 2. 特性による無効化
       if check_ability_immunity(defender, move_data) {
           return MoveResult::Immune;
       }
       
       // 3. ダメージ計算
       let damage = if move_data.category != MoveCategory::Status {
           calculate_damage(attacker, defender, move_data, context)
       } else {
           0
       };
       
       // 4. 追加効果
       if let Some(secondary) = secondary_effect_from_move(move_data.name, move_data) {
           apply_secondary_effect(attacker, defender, &secondary, context.rng);
       }
       
       MoveResult::Success { damage }
   }
   ```

2. テストケース作成（moves_test.rs）:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_thunderbolt_secondary() {
           // 10まんボルトの10%まひ
           let mut attacker = create_test_pokemon("Pikachu");
           let mut defender = create_test_pokemon("Gyarados");
           let thunderbolt = get_move("thunderbolt").unwrap();
           
           // 100回実行して追加効果確率を検証
           let mut paralyzed_count = 0;
           for _ in 0..100 {
               let mut def_clone = defender.clone();
               execute_move(thunderbolt, &mut attacker, &mut def_clone, &test_context());
               if def_clone.status == Some(Status::Paralysis) {
                   paralyzed_count += 1;
               }
           }
           
           assert!(paralyzed_count >= 5 && paralyzed_count <= 15); // 10% ± 5%
       }
       
       #[test]
       fn test_eruption_power() {
           let mut attacker = create_test_pokemon("Typhlosion");
           let eruption = get_move("eruption").unwrap();
           
           attacker.current_hp = attacker.stats.hp; // 満タン
           let power_full = calculate_variable_power(eruption, &attacker, &defender, None, None);
           assert_eq!(power_full, 150);
           
           attacker.current_hp = attacker.stats.hp / 2; // 半分
           let power_half = calculate_variable_power(eruption, &attacker, &defender, None, None);
           assert_eq!(power_half, 75);
       }
       
       #[test]
       fn test_protect_blocks_moves() {
           let mut defender = create_test_pokemon("Blissey");
           defender.protect_active = true;
           
           let tackle = get_move("tackle").unwrap();
           let result = execute_move(tackle, &mut attacker, &mut defender, &context);
           assert_eq!(result, MoveResult::Protected);
       }
   }
   ```

**成果物:**
- `moves/mod.rs` の統合（約100行追加）
- `tests/moves_test.rs` （約500行、50+テストケース）

---

## フェーズ2: 特性システムの完全実装（並列4タスク）

### タスク A1: 特性イベントシステムの設計

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/events.rs` (既存ファイルの拡張)

**目的:** 特性のイベントフックシステムを設計・実装

**現状:** 基本的なイベント構造のみ定義

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` L567-L750 (特性発動処理)
- `pokemon-showdown/sim/battle-actions.ts` L234-L456 (イベント管理)

**実施内容:**

1. イベントフック trait の定義:
   ```rust
   pub trait AbilityHooks {
       // Showdown: pokemon.ts#L567-L612
       fn on_switch_in(&self, pokemon: &mut Pokemon, state: &mut BattleState) -> EventResult {
           EventResult::None
       }
       
       fn on_before_move(&self, pokemon: &mut Pokemon, move_data: &MoveData) -> EventResult {
           EventResult::None
       }
       
       fn on_modify_atk(&self, pokemon: &Pokemon, attacker: &Pokemon) -> f32 {
           1.0
       }
       
       fn on_modify_def(&self, pokemon: &Pokemon, defender: &Pokemon) -> f32 {
           1.0
       }
       
       fn on_base_power(&self, pokemon: &Pokemon, move_data: &MoveData, move_type: Type) -> f32 {
           1.0
       }
       
       fn on_take_damage(&self, pokemon: &mut Pokemon, damage: u16, move_data: &MoveData) -> u16 {
           damage
       }
       
       fn on_immunity(&self, pokemon: &Pokemon, move_type: Type, status: Option<Status>) -> bool {
           false
       }
       
       fn on_end_turn(&self, pokemon: &mut Pokemon, weather: Option<Weather>) -> EventResult {
           EventResult::None
       }
   }
   
   pub enum EventResult {
       None,
       Block,
       Modify(f32),
       Trigger(String),
   }
   ```

2. 特性ディスパッチャー:
   ```rust
   pub fn dispatch_ability_hook(
       ability_name: &str,
       hook: AbilityHook,
       pokemon: &mut Pokemon,
       context: &BattleContext,
   ) -> EventResult {
       match ability_name {
           "Overgrow" => overgrow_ability().dispatch(hook, pokemon, context),
           "Blaze" => blaze_ability().dispatch(hook, pokemon, context),
           "Torrent" => torrent_ability().dispatch(hook, pokemon, context),
           "Intimidate" => intimidate_ability().dispatch(hook, pokemon, context),
           "Levitate" => levitate_ability().dispatch(hook, pokemon, context),
           _ => EventResult::None,
       }
   }
   ```

**成果物:**
- `abilities/events.rs` の拡張（約200行追加）
- イベントシステムの基盤完成

---

### タスク A2: ダメージ補正特性の実装

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/damage_modifiers.rs` (既存ファイルの拡張)

**目的:** ダメージ計算に影響する特性を完全実装

**現状:** 約10種類実装済み（Guts, Iron Fist, Huge Power等）

**参照元:**
- `pokemon-showdown/data/abilities.ts` の各特性定義
- `pokemon-showdown/sim/battle-actions.ts` L1205-L1456 (ダメージ修正処理)

**実施内容:**

1. 攻撃側ダメージ補正特性を追加（約40種類）:
   ```rust
   pub fn attacker_damage_modifier(
       attacker: &Pokemon,
       move_data: &MoveData,
       move_type: Type,
       context: &BattleContext,
   ) -> f32 {
       let mut modifier = 1.0;
       
       // Showdown: abilities.ts#L123-L145
       match attacker.ability.as_str() {
           // 既存: Guts, Iron Fist, Sand Force, Huge Power, Pure Power, Slow Start
           
           // 追加:
           "Overgrow" if move_type == Type::Grass && attacker.current_hp <= attacker.stats.hp / 3 => {
               modifier *= 1.5; // Showdown: abilities.ts#L1234
           }
           "Blaze" if move_type == Type::Fire && attacker.current_hp <= attacker.stats.hp / 3 => {
               modifier *= 1.5; // Showdown: abilities.ts#L456
           }
           "Torrent" if move_type == Type::Water && attacker.current_hp <= attacker.stats.hp / 3 => {
               modifier *= 1.5; // Showdown: abilities.ts#L2345
           }
           "Swarm" if move_type == Type::Bug && attacker.current_hp <= attacker.stats.hp / 3 => {
               modifier *= 1.5; // Showdown: abilities.ts#L2123
           }
           "Technician" if move_data.base_power.unwrap_or(0) <= 60 => {
               modifier *= 1.5; // Showdown: abilities.ts#L2234
           }
           "Adaptability" => {
               // STABを1.5倍→2倍に（別処理で実装）
           }
           "Sheer Force" if move_data.secondary.is_some() => {
               modifier *= 1.3; // Showdown: abilities.ts#L1987
           }
           "Reckless" if is_recoil_move(move_data) => {
               modifier *= 1.2; // Showdown: abilities.ts#L1876
           }
           "Tough Claws" if move_has_flag(move_data, "contact") => {
               modifier *= 1.3; // Showdown: abilities.ts#L2298
           }
           "Strong Jaw" if move_has_flag(move_data, "bite") => {
               modifier *= 1.5; // Showdown: abilities.ts#L2145
           }
           "Mega Launcher" if move_has_flag(move_data, "pulse") => {
               modifier *= 1.5; // Showdown: abilities.ts#L1543
           }
           "Punk Rock" if is_sound_move(move_data) => {
               modifier *= 1.3; // Showdown: abilities.ts#L2401
           }
           "Steelworker" | "Steely Spirit" if move_type == Type::Steel => {
               modifier *= 1.5; // Showdown: abilities.ts#L2167
           }
           "Fairy Aura" if move_type == Type::Fairy => {
               modifier *= 1.33; // Showdown: abilities.ts#L901
           }
           "Dark Aura" if move_type == Type::Dark => {
               modifier *= 1.33; // Showdown: abilities.ts#L789
           }
           _ => {}
       }
       
       modifier
   }
   ```

2. 防御側ダメージ補正特性を追加（約30種類）:
   ```rust
   pub fn defender_damage_modifier(
       defender: &Pokemon,
       move_data: &MoveData,
       move_type: Type,
       type_effectiveness: f32,
   ) -> f32 {
       let mut modifier = 1.0;
       
       // Showdown: abilities.ts#L345-L567
       match defender.ability.as_str() {
           // 既存: Solid Rock, Filter, Multiscale, Fur Coat, Dry Skin
           
           // 追加:
           "Thick Fat" if matches!(move_type, Type::Fire | Type::Ice) => {
               modifier *= 0.5; // Showdown: abilities.ts#L2256
           }
           "Heatproof" if move_type == Type::Fire => {
               modifier *= 0.5; // Showdown: abilities.ts#L1123
           }
           "Water Absorb" | "Dry Skin" if move_type == Type::Water => {
               // 無効化（別処理で実装）
           }
           "Volt Absorb" | "Motor Drive" if move_type == Type::Electric => {
               // 無効化（別処理で実装）
           }
           "Flash Fire" if move_type == Type::Fire => {
               // 無効化＋強化（別処理で実装）
           }
           "Levitate" if move_type == Type::Ground => {
               // 無効化（別処理で実装）
           }
           "Wonder Guard" if type_effectiveness <= 1.0 => {
               // こうかばつぐん以外無効
               return 0.0; // Showdown: abilities.ts#L2543
           }
           "Prism Armor" if type_effectiveness > 1.0 => {
               modifier *= 0.75; // Showdown: abilities.ts#L1876
           }
           "Punk Rock" if is_sound_move(move_data) => {
               modifier *= 0.5; // Showdown: abilities.ts#L2401
           }
           "Ice Scales" if move_data.category == MoveCategory::Special => {
               modifier *= 0.5; // Showdown: abilities.ts#L1298
           }
           "Fluffy" => {
               if move_has_flag(move_data, "contact") {
                   modifier *= 0.5;
               } else if move_type == Type::Fire {
                   modifier *= 2.0;
               }
               // Showdown: abilities.ts#L987
           }
           _ => {}
       }
       
       modifier
   }
   ```

**成果物:**
- `abilities/damage_modifiers.rs` の拡張（約300行追加）
- 全ダメージ補正特性の実装（約70種類）

---

### タスク A3: 状態変化特性の実装

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/status_abilities.rs` (既存ファイルの拡張)

**目的:** 状態異常・能力変化に関する特性を実装

**現状:** ability_blocks_status(), apply_download() のみ実装済み

**参照元:**
- `pokemon-showdown/data/abilities.ts` の状態関連特性
- `pokemon-showdown/sim/pokemon.ts` L678-L845

**実施内容:**

1. 状態異常無効特性（約20種類）:
   ```rust
   pub fn ability_grants_status_immunity(ability: &str, status: Status) -> bool {
       // Showdown: abilities.ts#L234-L456
       match (ability, status) {
           ("Immunity", Status::Poison) => true,
           ("Limber", Status::Paralysis) => true,
           ("Insomnia" | "Vital Spirit", Status::Sleep) => true,
           ("Water Veil", Status::Burn) => true,
           ("Magma Armor", Status::Freeze) => true,
           ("Oblivious", _) if matches!(status, Status::Infatuation | Status::Taunt) => true,
           ("Own Tempo", _) if matches!(status, Status::Confusion) => true,
           ("Inner Focus", Status::Flinch) => true,
           ("Comatose", _) => true, // 全状態異常無効
           _ => false,
       }
   }
   ```

2. 交代時発動特性（約30種類）:
   ```rust
   pub fn on_switch_in_ability(
       pokemon: &mut Pokemon,
       opponent: &mut Pokemon,
       state: &mut BattleState,
   ) {
       // Showdown: pokemon.ts#L678-L750
       match pokemon.ability.as_str() {
           "Intimidate" => {
               // 相手攻撃-1
               apply_stage_change(opponent, STAGE_ATK, -1);
           }
           "Download" => {
               // 相手の防御<特防なら攻撃+1、それ以外は特攻+1
               apply_download(pokemon, opponent);
           }
           "Trace" => {
               // 相手の特性をコピー
               if !opponent.ability_cannot_be_traced() {
                   pokemon.ability = opponent.ability.clone();
               }
           }
           "Drought" => {
               state.weather = Some(Weather::Sun);
               state.weather_turns = 5;
           }
           "Drizzle" => {
               state.weather = Some(Weather::Rain);
               state.weather_turns = 5;
           }
           "Sand Stream" => {
               state.weather = Some(Weather::Sand);
               state.weather_turns = 5;
           }
           "Snow Warning" => {
               state.weather = Some(Weather::Hail);
               state.weather_turns = 5;
           }
           "Electric Surge" => {
               state.field = Some(Field::Electric);
               state.field_turns = 5;
           }
           "Grassy Surge" => {
               state.field = Some(Field::Grassy);
               state.field_turns = 5;
           }
           "Psychic Surge" => {
               state.field = Some(Field::Psychic);
               state.field_turns = 5;
           }
           "Misty Surge" => {
               state.field = Some(Field::Misty);
               state.field_turns = 5;
           }
           _ => {}
       }
   }
   ```

**成果物:**
- `abilities/status_abilities.rs` の拡張（約250行追加）

---

### タスク A4: その他特性の実装

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/misc_abilities.rs` (既存ファイルの拡張)

**目的:** 特殊な効果を持つ特性を実装

**参照元:**
- `pokemon-showdown/data/abilities.ts` 全特性
- `pokemon-showdown/sim/pokemon.ts` 特性処理全般

**実施内容:**

1. 素早さ補正特性（約15種類）:
   ```rust
   pub fn speed_modifier_from_ability(
       pokemon: &Pokemon,
       weather: Option<Weather>,
       field: Option<Field>,
   ) -> f32 {
       // Showdown: pokemon.ts#L892-L934
       match pokemon.ability.as_str() {
           "Chlorophyll" if weather == Some(Weather::Sun) => 2.0,
           "Swift Swim" if weather == Some(Weather::Rain) => 2.0,
           "Sand Rush" if weather == Some(Weather::Sand) => 2.0,
           "Slush Rush" if weather == Some(Weather::Hail) => 2.0,
           "Surge Surfer" if field == Some(Field::Electric) => 2.0,
           "Quick Feet" if pokemon.status.is_some() => 1.5,
           "Slow Start" => 0.5, // 最初の5ターン
           "Unburden" if pokemon.item_consumed => 2.0,
           _ => 1.0,
       }
   }
   ```

2. 先制技無効化特性:
   ```rust
   pub fn blocks_priority_move(defender: &Pokemon, move_priority: i8) -> bool {
       // Showdown: battle-actions.ts#L567-L589
       if move_priority <= 0 {
           return false;
       }
       matches!(defender.ability.as_str(), 
           "Queenly Majesty" | "Dazzling" | "Armor Tail"
       )
   }
   ```

3. 回復特性:
   ```rust
   pub fn end_of_turn_healing(pokemon: &mut Pokemon, weather: Option<Weather>) -> u16 {
       // Showdown: pokemon.ts#L1234-L1289
       let max_hp = pokemon.stats.hp;
       
       match pokemon.ability.as_str() {
           "Rain Dish" if weather == Some(Weather::Rain) => {
               max_hp / 16 // 1/16回復
           }
           "Ice Body" if weather == Some(Weather::Hail) => {
               max_hp / 16
           }
           "Shed Skin" if pokemon.status.is_some() => {
               // 30%確率で状態異常治癒（別処理）
               0
           }
           _ => 0,
       }
   }
   ```

4. アイテム関連特性:
   ```rust
   pub fn ability_affects_item(ability: &str) -> ItemEffect {
       // Showdown: abilities.ts#L1567-L1678
       match ability {
           "Klutz" => ItemEffect::Disabled,
           "Unburden" => ItemEffect::SpeedBoost,
           "Sticky Hold" => ItemEffect::CannotBeSto

len,
           "Magician" => ItemEffect::StealsOpponentItem,
           "Pickup" => ItemEffect::RestoresConsumed,
           _ => ItemEffect::Normal,
       }
   }
   ```

**成果物:**
- `abilities/misc_abilities.rs` の拡張（約300行追加）
- 全特性の実装完了（約300種類）

---

## フェーズ3: もちものシステムの完全実装（並列3タスク）

### タスク I1: 戦闘用もちもの実装

**編集ファイル:** `pokemon-battle-core/src/sim/items/battle_items.rs` (既存ファイルの拡張)

**目的:** 戦闘中に効果を発揮するもちものを実装

**現状:** Life Orb, Expert Belt, Choice系, Leftovers, Black Sludgeのみ実装済み

**参照元:**
- `pokemon-showdown/data/items.ts` 全アイテム定義
- `pokemon-showdown/sim/pokemon.ts` L1456-L1678 (アイテム効果処理)

**実施内容:**

1. 威力補正アイテム（約30種類）を追加:
   ```rust
   pub fn item_power_modifier(
       item: &str,
       move_data: &MoveData,
       attacker: &Pokemon,
       defender: &Pokemon,
       type_effectiveness: f32,
   ) -> f32 {
       let id = normalize_item_id(item);
       
       match id.as_str() {
           // 既存: "lifeorb" (1.3倍), "expertbelt" (こうかばつぐん時1.2倍)
           
           // 追加:
           "muscleband" if move_data.category == MoveCategory::Physical => 1.1,
           "wiseglasses" if move_data.category == MoveCategory::Special => 1.1,
           "metronome" => {
               // 同じ技を連続で使うと威力上昇（1.2倍→1.4倍→1.6倍...最大2.0倍）
               1.0 + (attacker.metronome_count.min(5) as f32 * 0.2)
           }
           "loadeddice" if move_data.multihit.is_some() => 1.2,
           "punchingglove" if move_has_flag(move_data, "punch") => 1.1,
           "normalgemactivated" if move_data.move_type == "Normal" => 1.3,
           // ... 各タイプのジュエル（18種類）
           _ => 1.0,
       }
   }
   ```

2. ダメージ軽減アイテム（約20種類）:
   ```rust
   pub fn item_damage_reduction(
       item: &str,
       move_type: Type,
       type_effectiveness: f32,
   ) -> f32 {
       let id = normalize_item_id(item);
       
       match id.as_str() {
           "assaultvest" => 1.5, // 特殊耐久1.5倍（別処理）
           "eviolite" => 1.5,    // 未進化ポケモンの防御・特防1.5倍
           
           // 半減実（10種類）
           "chopleberry" if move_type == Type::Fighting && type_effectiveness > 1.0 => 0.5,
           "cobaberry" if move_type == Type::Flying && type_effectiveness > 1.0 => 0.5,
           "kebiaberry" if move_type == Type::Poison && type_effectiveness > 1.0 => 0.5,
           "shucaberry" if move_type == Type::Ground && type_effectiveness > 1.0 => 0.5,
           "chilanberry" if move_type == Type::Normal => 0.5,
           // ... 他の半減実
           
           _ => 1.0,
       }
   }
   ```

3. 状態回復アイテム（約15種類）:
   ```rust
   pub fn check_curative_item(pokemon: &mut Pokemon) -> bool {
       let Some(ref item) = pokemon.item else { return false; };
       let id = normalize_item_id(item);
       
       let should_consume = match id.as_str() {
           "lumberry" if pokemon.status.is_some() => {
               pokemon.clear_status();
               true
           }
           "chestoberry" if pokemon.status == Some(Status::Sleep) => {
               pokemon.clear_status();
               true
           }
           "pechaberry" if pokemon.status == Some(Status::Poison) => {
               pokemon.clear_status();
               true
           }
           "rawstberry" if pokemon.status == Some(Status::Burn) => {
               pokemon.clear_status();
               true
           }
           "aspearberry" if pokemon.status == Some(Status::Freeze) => {
               pokemon.clear_status();
               true
           }
           "cheriberry" if pokemon.status == Some(Status::Paralysis) => {
               pokemon.clear_status();
               true
           }
           "mentalherb" if pokemon.taunt_turns > 0 || pokemon.encore_turns > 0 => {
               pokemon.taunt_turns = 0;
               pokemon.encore_turns = 0;
               true
           }
           _ => false,
       };
       
       if should_consume {
           pokemon.item = None;
           pokemon.item_consumed = true;
       }
       
       should_consume
   }
   ```

**成果物:**
- `items/battle_items.rs` の拡張（約400行追加）

---

### タスク I2: タイプ強化もちもの実装

**編集ファイル:** `pokemon-battle-core/src/sim/items/type_items.rs` (既存ファイルの拡張)

**目的:** タイプ強化アイテムを完全実装

**現状:** 基本的なタイプ強化アイテムは実装済み

**参照元:**
- `pokemon-showdown/data/items.ts` L1234-L2345 (タイプ強化アイテム)

**実施内容:**

1. プレート系アイテム（18種類）の追加:
   ```rust
   pub fn item_type_boost(item: &str, move_type: Type) -> f32 {
       let id = normalize_item_id(item);
       
       let (boosted_type, multiplier) = match id.as_str() {
           // 既存: silkscarf, charcoal, mysticwater 等（1.2倍）
           
           // プレート系（1.2倍）
           "flameplate" => (Some(Type::Fire), 1.2),
           "splashplate" => (Some(Type::Water), 1.2),
           "zapplate" => (Some(Type::Electric), 1.2),
           "meadowplate" => (Some(Type::Grass), 1.2),
           "icicleplate" => (Some(Type::Ice), 1.2),
           "fistplate" => (Some(Type::Fighting), 1.2),
           "toxicplate" => (Some(Type::Poison), 1.2),
           "earthplate" => (Some(Type::Ground), 1.2),
           "skyplate" => (Some(Type::Flying), 1.2),
           "mindplate" => (Some(Type::Psychic), 1.2),
           "insectplate" => (Some(Type::Bug), 1.2),
           "stoneplate" => (Some(Type::Rock), 1.2),
           "spookyplate" => (Some(Type::Ghost), 1.2),
           "dracoplate" => (Some(Type::Dragon), 1.2),
           "dreadplate" => (Some(Type::Dark), 1.2),
           "ironplate" => (Some(Type::Steel), 1.2),
           "pixieplate" => (Some(Type::Fairy), 1.2),
           
           // メモリ系（1.2倍）
           "fightingmemory" => (Some(Type::Fighting), 1.2),
           "flyingmemory" => (Some(Type::Flying), 1.2),
           // ... 全18種類
           
           // ドライブ系（1.2倍）
           "shockdrive" => (Some(Type::Electric), 1.2),
           "burndrive" => (Some(Type::Fire), 1.2),
           "chilldrive" => (Some(Type::Ice), 1.2),
           "dousedrive" => (Some(Type::Water), 1.2),
           
           _ => (None, 1.0),
       };
       
       if let Some(boosted) = boosted_type {
           if boosted == move_type {
               return multiplier;
           }
       }
       
       1.0
   }
   ```

**成果物:**
- `items/type_items.rs` の拡張（約150行追加）

---

### タスク I3: もちもの消費・効果処理

**編集ファイル:** `pokemon-battle-core/src/sim/items/consumable.rs` (既存ファイルの拡張)

**目的:** 消費アイテムと発動条件を実装

**現状:** Focus Sashのみ実装済み

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` L1789-L2012 (アイテム消費処理)

**実施内容:**

1. HP回復きのみ（約20種類）:
   ```rust
   pub fn check_hp_restore_item(pokemon: &mut Pokemon) -> Option<u16> {
       let Some(ref item) = pokemon.item else { return None; };
       let id = normalize_item_id(item);
       
       let max_hp = pokemon.stats.hp;
       let current_hp = pokemon.current_hp;
       
       let (trigger_threshold, heal_amount) = match id.as_str() {
           "oranberry" if current_hp <= max_hp / 2 => {
               (Some(max_hp / 2), 10) // HP50%以下で10回復
           }
           "sitrusberry" if current_hp <= max_hp / 2 => {
               (Some(max_hp / 2), max_hp / 4) // HP50%以下で1/4回復
           }
           "aguavberry" if current_hp <= max_hp / 4 => {
               (Some(max_hp / 4), max_hp / 3) // HP25%以下で1/3回復（辛い味嫌いで混乱）
           }
           "figyberry" if current_hp <= max_hp / 4 => {
               (Some(max_hp / 4), max_hp / 3)
           }
           "wikiberry" if current_hp <= max_hp / 4 => {
               (Some(max_hp / 4), max_hp / 3)
           }
           "magoberry" if current_hp <= max_hp / 4 => {
               (Some(max_hp / 4), max_hp / 3)
           }
           "iapapaberry" if current_hp <= max_hp / 4 => {
               (Some(max_hp / 4), max_hp / 3)
           }
           _ => (None, 0),
       };
       
       if let Some(threshold) = trigger_threshold {
           if current_hp <= threshold {
               pokemon.item = None;
               pokemon.item_consumed = true;
               return Some(heal_amount);
           }
       }
       
       None
   }
   ```

2. 能力上昇きのみ（約10種類）:
   ```rust
   pub fn check_stat_boost_item(
       pokemon: &mut Pokemon,
       trigger: ItemTrigger,
   ) -> Option<(usize, i8)> {
       let Some(ref item) = pokemon.item else { return None; };
       let id = normalize_item_id(item);
       
       let boost = match (id.as_str(), trigger) {
           ("liechiberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               Some((STAGE_ATK, 1)) // 攻撃+1
           }
           ("ganlonberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               Some((STAGE_DEF, 1)) // 防御+1
           }
           ("salacberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               Some((STAGE_SPE, 1)) // 素早さ+1
           }
           ("petayaberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               Some((STAGE_SPA, 1)) // 特攻+1
           }
           ("apicotberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               Some((STAGE_SPD, 1)) // 特防+1
           }
           ("starfberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               // ランダムで2段階上昇
               Some((rand::thread_rng().gen_range(0..5), 2))
           }
           ("custapberry", ItemTrigger::LowHP) if pokemon.current_hp <= pokemon.stats.hp / 4 => {
               // 行動順が最速になる（別処理）
               None
           }
           _ => None,
       };
       
       if boost.is_some() {
           pokemon.item = None;
           pokemon.item_consumed = true;
       }
       
       boost
   }
   
   pub enum ItemTrigger {
       LowHP,
       TakeDamage,
       UseMove(String),
       EndOfTurn,
   }
   ```

3. その他消費アイテム（約30種類）:
   ```rust
   pub fn check_misc_consumable(
       pokemon: &mut Pokemon,
       trigger: ItemTrigger,
   ) -> ItemEffect {
       let Some(ref item) = pokemon.item else { return ItemEffect::None; };
       let id = normalize_item_id(item);
       
       match (id.as_str(), trigger) {
           ("focussash", ItemTrigger::TakeDamage) 
               if pokemon.current_hp == pokemon.stats.hp => {
               // HP満タンからの一撃でHP1残す
               pokemon.item = None;
               pokemon.item_consumed = true;
               ItemEffect::SurviveKO
           }
           ("whiteherb", ItemTrigger::StatDrop) => {
               // 能力低下を打ち消す
               pokemon.item = None;
               pokemon.item_consumed = true;
               ItemEffect::ResetNegativeBoosts
           }
           ("powerherb", ItemTrigger::UseMove(ref move_name)) 
               if is_charging_move(move_name) => {
               // チャージ技を即発動
               pokemon.item = None;
               pokemon.item_consumed = true;
               ItemEffect::SkipCharge
           }
           ("redcard", ItemTrigger::TakeDamage) => {
               // 攻撃してきた相手を強制交代
               pokemon.item = None;
               pokemon.item_consumed = true;
               ItemEffect::ForceSwitch
           }
           _ => ItemEffect::None,
       }
   }
   ```

**成果物:**
- `items/consumable.rs` の拡張（約400行追加）
- 全もちものの実装完了（約500種類）

---

## フェーズ4: 高度な戦闘システム（残タスク）

### タスク S1: 交代処理の完全実装

**編集ファイル:** `pokemon-battle-core/src/sim/switching.rs` (既存ファイルの拡張)

**目的:** 交代に関する全処理を実装

**現状:** apply_trapping_move() のみ実装済み

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts` L234-L456 (switchIn, switchOut)
- `pokemon-showdown/sim/pokemon.ts` L1123-L1234

**実施内容:**

1. 交代可否判定:
   ```rust
   pub fn can_switch(pokemon: &Pokemon, force: bool) -> Result<(), String> {
       // Showdown: pokemon.ts#L1123-L1156
       if pokemon.trapped && !force {
           return Err("Cannot switch: trapped".to_string());
       }
       
       if pokemon.has_ability("Shadow Tag") && !force {
           // 相手がゴーストタイプでなければ交代不可
           return Err("Cannot switch: Shadow Tag".to_string());
       }
       
       if pokemon.item.as_deref() == Some("Shed Shell") {
           // ぬけがらシェルで交代可能
           return Ok(());
       }
       
       Ok(())
   }
   ```

2. 交代時の状態リセット:
   ```rust
   pub fn on_switch_out(pokemon: &mut Pokemon) {
       // Showdown: pokemon.ts#L1189-L1234
       pokemon.stat_stages = [0; 6];
       pokemon.accuracy_stage = 0;
       pokemon.evasion_stage = 0;
       pokemon.protect_counter = 0;
       pokemon.roosted = false;
       pokemon.substitute_hp = 0;
       pokemon.taunt_turns = 0;
       pokemon.encore_turns = 0;
       pokemon.encore_move = None;
       pokemon.charging_move = None;
       pokemon.destiny_bond = false;
       
       // こだわりアイテムのロック解除
       if pokemon.choice_lock_move.is_some() {
           pokemon.choice_lock_move = None;
       }
   }
   ```

3. 交代技（とんぼがえり、ボルトチェンジ等）:
   ```rust
   pub fn handle_switch_move(
       attacker: &mut Pokemon,
       defender: &mut Pokemon,
       move_data: &MoveData,
       damage: u16,
   ) -> SwitchEffect {
       // Showdown: data/moves.ts の selfSwitch プロパティ
       match move_data.name {
           "U-turn" | "Volt Switch" | "Flip Turn" => {
               if damage > 0 && !defender.is_fainted() {
                   SwitchEffect::SwitchAfterDamage
               } else {
                   SwitchEffect::None
               }
           }
           "Baton Pass" => {
               // 能力変化を引き継ぐ
               SwitchEffect::PassBoosts {
                   stat_stages: attacker.stat_stages,
                   substitute_hp: attacker.substitute_hp,
               }
           }
           "Parting Shot" => {
               // 相手攻撃・特攻-1して交代
               apply_stage_change(defender, STAGE_ATK, -1);
               apply_stage_change(defender, STAGE_SPA, -1);
               SwitchEffect::SwitchAfterEffect
           }
           _ => SwitchEffect::None,
       }
   }
   ```

**成果物:**
- `switching.rs` の拡張（約300行追加）

---

## フェーズ5: Showdown完全互換検証（残タスク）

### タスク V2: 差分解析とCI統合の完成

**編集ファイル:** 
- `tools/diff_analyzer.rs` (既存ファイルの修正)
- `tools/ci_diff_check.sh` (既存ファイルの修正)
- `pokemon-battle-core/src/battle_logger.rs` (新規作成)
- `.github/workflows/showdown_compat.yml` (新規作成)

**目的:** V1形式（`log`配列）統一とCI/CD統合の完成

**現状:**
- ✅ diff_analyzer.rs 本体は実装済み
- ✅ ci_diff_check.sh のスケルトンは存在
- ❌ diff_analyzerがV1形式（`log`配列）に未対応
- ❌ Rust実装がShowdown形式ログを出力する機能がない
- ❌ CI/CDワークフローが未定義

**参照元:**
- `tests/showdown_compat/cases/pikachu_thunderbolt_vs_gyarados_splash_turn1.json` のlog形式
- Showdown プロトコル仕様: https://github.com/smogon/pokemon-showdown/blob/master/sim/SIM-PROTOCOL.md
- `tools/diff_analyzer.rs` L217-233 (parse_battle_log関数)

**実施内容:**

1. **diff_analyzer.rsの修正:**

   新規関数を追加（L150付近に挿入）:
   ```rust
   fn parse_log_array_to_turns(log: &[Value]) -> Vec<TurnLog> {
       // Showdown: SIM-PROTOCOL.md
       let mut turns = Vec::new();
       let mut current_turn = 0usize;
       let mut current_events = Vec::new();
       
       for line_val in log {
           let Some(line) = line_val.as_str() else { continue; };
           
           if line.starts_with("|turn|") {
               // 前のターンを保存
               if !current_events.is_empty() {
                   turns.push(TurnLog {
                       turn: current_turn,
                       events: current_events.clone(),
                   });
                   current_events.clear();
               }
               
               // 新しいターン番号
               if let Some(num_str) = line.strip_prefix("|turn|") {
                   current_turn = num_str.parse::<usize>().unwrap_or(0);
               }
           } else {
               // イベントを解析
               if let Some(event) = parse_protocol_line(line) {
                   current_events.push(event);
               }
           }
       }
       
       // 最後のターン
       if !current_events.is_empty() {
           turns.push(TurnLog {
               turn: current_turn,
               events: current_events,
           });
       }
       
       turns
   }
   
   fn parse_protocol_line(line: &str) -> Option<Event> {
       let parts: Vec<&str> = line.split('|').collect();
       if parts.len() < 2 {
           return None;
       }
       
       let event_type = parts[1];
       match event_type {
           "-damage" if parts.len() >= 4 => {
               Some(Event {
                   key: EventKey::Damage {
                       target: parts[2].to_string(),
                       source: String::new(),
                       move_id: String::new(),
                   },
                   data: json!({ "hp": parts[3] }),
               })
           }
           "move" if parts.len() >= 5 => {
               Some(Event {
                   key: EventKey::Message {
                       text: format!("{} used {}", parts[2], parts[3]),
                   },
                   data: json!({
                       "source": parts[2],
                       "move": parts[3],
                       "target": parts.get(4).unwrap_or(&""),
                   }),
               })
           }
           "-status" if parts.len() >= 4 => {
               Some(Event {
                   key: EventKey::Status {
                       target: parts[2].to_string(),
                       status: parts[3].to_string(),
                   },
                   data: Value::Null,
               })
           }
           "switch" if parts.len() >= 4 => {
               Some(Event {
                   key: EventKey::Switch {
                       side: parts[2].chars().next().unwrap_or('p'),
                       to: parts[3].to_string(),
                   },
                   data: Value::Null,
               })
           }
           "win" if parts.len() >= 3 => {
               Some(Event {
                   key: EventKey::Message {
                       text: format!("{} wins", parts[2]),
                   },
                   data: json!({ "winner": parts[2] }),
               })
           }
           _ => None,
       }
   }
   ```

   parse_battle_log関数の修正（L217-233）:
   ```rust
   fn parse_battle_log(root: &Value) -> BattleLog {
       // log配列があればそれを優先
       let turns = if let Some(log_array) = root.get("log").and_then(|v| v.as_array()) {
           parse_log_array_to_turns(log_array)
       } else if let Some(turns_val) = root.get("turns") {
           // 既存のturns形式もサポート（後方互換性）
           match turns_val {
               Value::Array(arr) => arr.iter().enumerate()
                   .map(|(i, v)| parse_turn(v, i))
                   .collect(),
               _ => Vec::new(),
           }
       } else {
           Vec::new()
       };
       
       let winner = root.get("events")
           .and_then(|e| e.get("win"))
           .and_then(|w| w.as_str())
           .map(String::from)
           .or_else(|| get_str(root, "winner"));
       
       BattleLog { turns, winner, seed: get_str(root, "seed") }
   }
   ```

2. **Rustバトルログ生成機能の実装:**

   新規ファイル: `pokemon-battle-core/src/battle_logger.rs`
   
   ```rust
   use serde_json::{json, Value};
   
   pub struct BattleLogger {
       log: Vec<String>,
   }
   
   impl BattleLogger {
       pub fn new() -> Self {
           Self { log: Vec::new() }
       }
       
       pub fn log_turn(&mut self, turn: usize) {
           self.log.push(format!("|turn|{}", turn));
       }
       
       pub fn log_move(&mut self, source: &str, move_id: &str, target: &str) {
           self.log.push(format!("|move|{}|{}|{}", source, move_id, target));
       }
       
       pub fn log_damage(&mut self, target: &str, hp: u16, max_hp: u16) {
           self.log.push(format!("|-damage|{}|{}/{}", target, hp, max_hp));
       }
       
       pub fn log_heal(&mut self, target: &str, hp: u16, max_hp: u16) {
           self.log.push(format!("|-heal|{}|{}/{}", target, hp, max_hp));
       }
       
       pub fn log_status(&mut self, target: &str, status: &str) {
           self.log.push(format!("|-status|{}|{}", target, status));
       }
       
       pub fn log_supereffective(&mut self, target: &str) {
           self.log.push(format!("|-supereffective|{}", target));
       }
       
       pub fn log_resisted(&mut self, target: &str) {
           self.log.push(format!("|-resisted|{}", target));
       }
       
       pub fn log_immune(&mut self, target: &str) {
           self.log.push(format!("|-immune|{}", target));
       }
       
       pub fn log_switch(&mut self, pokemon: &str, species: &str, hp: u16, max_hp: u16) {
           self.log.push(format!("|switch|{}|{}|{}/{}", pokemon, species, hp, max_hp));
       }
       
       pub fn log_ability(&mut self, pokemon: &str, ability: &str) {
           self.log.push(format!("|-ability|{}|{}", pokemon, ability));
       }
       
       pub fn log_boost(&mut self, pokemon: &str, stat: &str, amount: i8) {
           if amount > 0 {
               self.log.push(format!("|-boost|{}|{}|{}", pokemon, stat, amount));
           } else {
               self.log.push(format!("|-unboost|{}|{}|{}", pokemon, stat, -amount));
           }
       }
       
       pub fn log_weather(&mut self, weather: &str) {
           self.log.push(format!("|-weather|{}", weather));
       }
       
       pub fn log_field_start(&mut self, field: &str) {
           self.log.push(format!("|-fieldstart|{}", field));
       }
       
       pub fn log_field_end(&mut self, field: &str) {
           self.log.push(format!("|-fieldend|{}", field));
       }
       
       pub fn log_win(&mut self, winner: &str) {
           self.log.push(format!("|win|{}", winner));
       }
       
       pub fn log_tie(&mut self) {
           self.log.push("|tie".to_string());
       }
       
       pub fn to_json(&self) -> Value {
           json!({
               "log": self.log,
               "formatid": "gen9customgame"
           })
       }
   }
   ```

   battle.rsへの統合（BattleState構造体とexecute_move関数を修正）:
   ```rust
   pub struct BattleState {
       // ... 既存フィールド ...
       pub logger: Option<BattleLogger>,
   }
   
   // execute_move内で適宜ログ記録
   fn execute_move(...) {
       // ...
       if let Some(ref mut logger) = state.logger {
           logger.log_move(source_id, move_data.name, target_id);
           if damage > 0 {
               logger.log_damage(target_id, defender.current_hp, defender.stats.hp);
           }
           if type_effectiveness > 1.0 {
               logger.log_supereffective(target_id);
           }
       }
       // ...
   }
   ```

   CLIオプション追加（pokemon-battle-cli/src/main.rs）:
   ```rust
   "--log-json" => {
       let path = args.next().ok_or_else(|| anyhow!("--log-json requires path"))?;
       log_json_path = Some(path);
   }
   
   // バトル実行後
   if let Some(path) = log_json_path {
       if let Some(logger) = battle_state.logger {
           std::fs::write(path, serde_json::to_string_pretty(&logger.to_json())?)?;
       }
   }
   ```

3. **ci_diff_check.shの完成:**

   既存スケルトンを以下のように拡張:
   ```bash
   #!/bin/bash
   set -e
   
   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
   PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
   
   FAIL_ON_DIFF=""
   if [[ "$1" == "--fail-on-diff" ]]; then
       FAIL_ON_DIFF="--fail-on-diff"
   fi
   
   echo "🔨 Building tools..."
   cargo build --release --bin diff_analyzer
   cargo build --release --bin pokemon-battle-cli
   
   mkdir -p tmp/rust_logs
   mkdir -p reports/compatibility
   
   EXIT_CODE=0
   PASSED=0
   FAILED=0
   
   for CASE in tests/showdown_compat/cases/*.json; do
       CASE_NAME=$(basename "$CASE" .json)
       echo "🧪 Testing: $CASE_NAME"
       
       # Rustでバトル実行（V1形式で出力）
       if cargo run --release --bin pokemon-battle-cli -- \
           test-case "$CASE" \
           --log-json "tmp/rust_logs/${CASE_NAME}.json" 2>/dev/null; then
           
           # 差分解析
           if cargo run --release --bin diff_analyzer -- \
               --showdown "$CASE" \
               --rust "tmp/rust_logs/${CASE_NAME}.json" \
               --out "reports/compatibility/${CASE_NAME}_report.html" \
               $FAIL_ON_DIFF 2>/dev/null; then
               echo "  ✅ No differences"
               PASSED=$((PASSED + 1))
           else
               echo "  ❌ Differences found - see reports/compatibility/${CASE_NAME}_report.html"
               FAILED=$((FAILED + 1))
               EXIT_CODE=1
           fi
       else
           echo "  ⚠️  Rust execution failed"
           FAILED=$((FAILED + 1))
           EXIT_CODE=1
       fi
   done
   
   echo ""
   echo "📊 Summary:"
   echo "  Passed: $PASSED"
   echo "  Failed: $FAILED"
   echo "  Reports: reports/compatibility/"
   
   exit $EXIT_CODE
   ```

4. **GitHub Actionsワークフロー作成:**

   新規ファイル: `.github/workflows/showdown_compat.yml`
   ```yaml
   name: Showdown Compatibility Test
   
   on:
     push:
       branches: [main, develop]
     pull_request:
       branches: [main]
   
   jobs:
     compat-test:
       runs-on: ubuntu-latest
       
       steps:
         - name: Checkout
           uses: actions/checkout@v3
           with:
             submodules: recursive
         
         - name: Setup Node.js
           uses: actions/setup-node@v3
           with:
             node-version: '18'
         
         - name: Setup Rust
           uses: actions-rs/toolchain@v1
           with:
             toolchain: stable
             override: true
         
         - name: Cache Cargo
           uses: actions/cache@v3
           with:
             path: |
               ~/.cargo/registry
               ~/.cargo/git
               target
             key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
         
         - name: Install dependencies
           run: |
             npm install
             cd pokemon-showdown && npm ci && cd ..
         
         - name: Build
           run: cargo build --release
         
         - name: Run compatibility tests
           run: ./tools/ci_diff_check.sh --fail-on-diff
         
         - name: Upload reports
           if: always()
           uses: actions/upload-artifact@v3
           with:
             name: compatibility-reports
             path: reports/compatibility/*.html
   ```

**成果物:**
- `tools/diff_analyzer.rs` 修正（V1形式対応、約150行追加）
- `pokemon-battle-core/src/battle_logger.rs` 新規作成（約150行）
- `pokemon-battle-core/src/sim/battle.rs` 修正（logger統合、約50行追加）
- `pokemon-battle-cli/src/main.rs` 修正（--log-jsonオプション、約20行追加）
- `tools/ci_diff_check.sh` 完成（約80行）
- `.github/workflows/showdown_compat.yml` 新規作成（約60行）

---

## フェーズ6: フォルムチェンジシステム（並列4タスク）

### タスク F1: メガシンカシステム

**編集ファイル:** `pokemon-battle-core/src/sim/mega_evolution.rs` (新規作成)

**目的:** メガシンカの完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` L1234-L1289 (canMegaEvo, runMegaEvo)
- `pokemon-showdown/data/items.ts` L5678-L5890 (メガストーン定義)
- `pokemon-showdown/data/pokedex.ts` L3456+ (otherFormes配列)

**実施内容:**

1. Pokemon構造体の拡張（pokemon.rs L19-52）:
   ```rust
   pub struct Pokemon {
       // ... 既存フィールド ...
       pub is_mega: bool,
       pub can_mega_evolve: bool,
       pub base_species: String,
   }
   ```

2. メガストーン判定（Showdown: data/items.ts#L5678-L5890）:
   ```rust
   pub fn is_mega_stone(item: &str) -> bool {
       let normalized = normalize_item_id(item);
       matches!(normalized.as_str(),
           "venusaurite" | "charizarditex" | "charizarditey" |
           "blastoisinite" | "alakazite" | "gengarite" |
           "kangaskhanite" | "pinsirite" | "gyaradosite" |
           "aerodactylite" | "mewtwonit

ex" | "mewtwonnitey" |
           "ampharosite" | "steelixite" | "scizorite" | "heracronite" |
           "houndoominite" | "tyranitarite" | "sceptilite" |
           "blazikenite" | "swampertite" | "gardevoirite" |
           "sablenite" | "mawilite" | "aggronite" | "medichamite" |
           "manectite" | "sharpedonite" | "cameruptite" | "altarianite" |
           "banettite" | "absolite" | "glalitite" | "salamencite" |
           "metagrossite" | "latiasite" | "latiosite" | "lopunnite" |
           "garchompite" | "lucarionite" | "abomasite" | "galladite" |
           "audinite" | "diancite" | "beedrillite" | "pidgeotite" |
           "slowbronite" | "steelixite"
       )
   }
   
   pub fn get_mega_species(base_species: &str, mega_stone: &str) -> Option<&'static str> {
       let base_id = normalize_species_id(base_species);
       let stone_id = normalize_item_id(mega_stone);
       match (base_id.as_str(), stone_id.as_str()) {
           ("venusaur", "venusaurite") => Some("venusaurmega"),
           ("charizard", "charizarditex") => Some("charizardmegax"),
           ("charizard", "charizarditey") => Some("charizardmegay"),
           ("blastoise", "blastoisinite") => Some("blastoisemega"),
           // ... 全50種類のマッピング
           _ => None,
       }
   }
   ```

3. メガシンカ実行（Showdown: pokemon.ts#L1234-L1289）:
   ```rust
   pub fn mega_evolve(pokemon: &mut Pokemon) -> Result<()> {
       if !pokemon.can_mega_evolve {
           return Err(anyhow!("Already used Mega Evolution"));
       }
       
       let mega_stone = pokemon.item.as_deref()
           .ok_or_else(|| anyhow!("No held item"))?;
       
       let mega_species = get_mega_species(&pokemon.species, mega_stone)
           .ok_or_else(|| anyhow!("Invalid Mega Stone for this species"))?;
       
       let mega_data = POKEDEX.get(mega_species)
           .ok_or_else(|| anyhow!("Mega forme not in POKEDEX: {}", mega_species))?;
       
       // 種族値変更（HP以外）
       let evs = [0; 6]; // TODO: 保持されたEVを使用
       let ivs = [31; 6]; // TODO: 保持されたIVを使用
       let nature = Nature::Hardy; // TODO: 保持されたNatureを使用
       
       pokemon.stats.atk = calc_stat(Stat::Atk, mega_data.base_stats.atk, pokemon.level, evs[1], ivs[1], nature);
       pokemon.stats.def = calc_stat(Stat::Def, mega_data.base_stats.def, pokemon.level, evs[2], ivs[2], nature);
       pokemon.stats.spa = calc_stat(Stat::SpA, mega_data.base_stats.spa, pokemon.level, evs[3], ivs[3], nature);
       pokemon.stats.spd = calc_stat(Stat::SpD, mega_data.base_stats.spd, pokemon.level, evs[4], ivs[4], nature);
       pokemon.stats.spe = calc_stat(Stat::Spe, mega_data.base_stats.spe, pokemon.level, evs[5], ivs[5], nature);
       
       // タイプ変更
       pokemon.types = parse_types(mega_data.types);
       
       // 特性変更
       if let Some(mega_ability) = mega_data.abilities.primary {
           pokemon.ability = mega_ability.to_string();
       }
       
       // フラグ更新
       pokemon.base_species = pokemon.species.clone();
       pokemon.species = mega_species.to_string();
       pokemon.is_mega = true;
       pokemon.can_mega_evolve = false;
       
       Ok(())
   }
   ```

**成果物:**
- `pokemon-battle-core/src/sim/mega_evolution.rs` （約300行）
- `pokemon-battle-core/src/sim/pokemon.rs` 修正（フィールド追加）
- テストケース（10+メガシンカ検証）

---

### タスク F2: ダイマックスシステム

**編集ファイル:** `pokemon-battle-core/src/sim/dynamax.rs` (新規作成)

**目的:** ダイマックスの完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` L1456-L1567 (canDynamax, runDynamax)
- `pokemon-showdown/data/moves.ts` L18234+ (Max Moves)
- `pokemon-showdown/sim/battle-actions.ts` L1890-L2012

**実施内容:**

1. Pokemon構造体の拡張:
   ```rust
   pub struct Pokemon {
       // ... 既存フィールド ...
       pub is_dynamaxed: bool,
       pub dynamax_level: u8,  // 0-10
       pub dynamax_turns: u8,
       pub base_max_hp: u16,
   }
   ```

2. ダイマックス実行（Showdown: pokemon.ts#L1456-L1512）:
   ```rust
   pub fn dynamax(pokemon: &mut Pokemon) -> Result<()> {
       if pokemon.is_dynamaxed {
           return Err(anyhow!("Already Dynamaxed"));
       }
       
       // HP倍率計算（レベル依存）
       let hp_multiplier = 1.5 + (pokemon.dynamax_level as f32 * 0.05);
       
       pokemon.base_max_hp = pokemon.stats.hp;
       let old_max = pokemon.stats.hp;
       let new_max = (old_max as f32 * hp_multiplier) as u16;
       
       // HP割合を維持
       let hp_ratio = pokemon.current_hp as f32 / old_max as f32;
       pokemon.stats.hp = new_max;
       pokemon.current_hp = (new_max as f32 * hp_ratio) as u16;
       
       pokemon.is_dynamaxed = true;
       pokemon.dynamax_turns = 3;
       
       Ok(())
   }
   
   pub fn revert_dynamax(pokemon: &mut Pokemon) {
       if !pokemon.is_dynamaxed {
           return;
       }
       
       let current_max = pokemon.stats.hp;
       let hp_ratio = pokemon.current_hp as f32 / current_max as f32;
       
       pokemon.stats.hp = pokemon.base_max_hp;
       pokemon.current_hp = (pokemon.base_max_hp as f32 * hp_ratio) as u16;
       
       pokemon.is_dynamaxed = false;
       pokemon.dynamax_turns = 0;
   }
   ```

3. ダイマックス技変換（Showdown: data/moves.ts#L18234+）:
   ```rust
   pub struct MaxMoveData {
       pub name: String,
       pub base_power: u16,
       pub secondary_effect: Option<MaxMoveEffect>,
   }
   
   pub enum MaxMoveEffect {
       WeatherChange(Weather),
       TerrainChange(Field),
       StatBoost { stat: usize, amount: i8, target_self: bool },
   }
   
   pub fn get_max_move(
       base_move: &MoveData,
       move_type: Type,
       category: MoveCategory,
   ) -> MaxMoveData {
       // Showdown: battle-actions.ts#L1956-L2012
       let base_power = calculate_max_move_power(base_move.base_power.unwrap_or(0));
       
       let (name, secondary_effect) = match (move_type, category) {
           (Type::Normal, MoveCategory::Physical | MoveCategory::Special) => {
               ("Max Strike".to_string(), Some(MaxMoveEffect::StatBoost {
                   stat: STAGE_SPE,
                   amount: -1,
                   target_self: false,
               }))
           }
           (Type::Fire, _) => {
               ("Max Flare".to_string(), Some(MaxMoveEffect::WeatherChange(Weather::Sun)))
           }
           (Type::Water, _) => {
               ("Max Geyser".to_string(), Some(MaxMoveEffect::WeatherChange(Weather::Rain)))
           }
           (Type::Electric, _) => {
               ("Max Lightning".to_string(), Some(MaxMoveEffect::TerrainChange(Field::Electric)))
           }
           (Type::Grass, _) => {
               ("Max Overgrowth".to_string(), Some(MaxMoveEffect::TerrainChange(Field::Grassy)))
           }
           (Type::Ice, _) => {
               ("Max Hailstorm".to_string(), Some(MaxMoveEffect::WeatherChange(Weather::Hail)))
           }
           (Type::Fighting, _) => {
               ("Max Knuckle".to_string(), Some(MaxMoveEffect::StatBoost {
                   stat: STAGE_ATK,
                   amount: 1,
                   target_self: true,
               }))
           }
           (Type::Flying, _) => {
               ("Max Airstream".to_string(), Some(MaxMoveEffect::StatBoost {
                   stat: STAGE_SPE,
                   amount: 1,
                   target_self: true,
               }))
           }
           (Type::Psychic, _) => {
               ("Max Mindstorm".to_string(), Some(MaxMoveEffect::TerrainChange(Field::Psychic)))
           }
           (Type::Fairy, _) => {
               ("Max Starfall".to_string(), Some(MaxMoveEffect::TerrainChange(Field::Misty)))
           }
           // ... 全18タイプ
           (_, MoveCategory::Status) => {
               ("Max Guard".to_string(), None) // まもる相当
           }
           _ => ("Max Strike".to_string(), None),
       };
       
       MaxMoveData { name, base_power, secondary_effect }
   }
   
   fn calculate_max_move_power(base_power: u16) -> u16 {
       // Showdown: data/moves.ts の威力変換テーブル
       match base_power {
           0..=40 => 90,
           41..=50 => 100,
           51..=60 => 110,
           61..=70 => 120,
           71..=100 => 130,
           101..=140 => 140,
           _ => 150,
       }
   }
   ```

**成果物:**
- `pokemon-battle-core/src/sim/dynamax.rs` （約400行）
- ダイマックス技変換テーブル完備
- テストケース（ダイマックス検証）

---

### タスク F3: Z技システム

**編集ファイル:** `pokemon-battle-core/src/sim/zmove.rs` (新規作成)

**目的:** Z技の完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` L1678-L1789 (canZMove, runZMove)
- `pokemon-showdown/data/items.ts` L6789+ (Zクリスタル定義)
- `pokemon-showdown/data/moves.ts` L17345+ (Z技定義)

**実施内容:**

1. Pokemon構造体の拡張:
   ```rust
   pub struct Pokemon {
       // ... 既存フィールド ...
       pub z_crystal: Option<String>,
       pub z_move_used: bool,
   }
   ```

2. Zクリスタル判定（Showdown: data/items.ts#L6789+）:
   ```rust
   pub fn is_z_crystal(item: &str) -> bool {
       let id = normalize_item_id(item);
       matches!(id.as_str(),
           // 汎用Zクリスタル（18種類）
           "normaliumz" | "firiumz" | "wateriumz" | "electriumz" |
           "grassiumz" | "iciumz" | "fightiniumz" | "poisoniumz" |
           "groundiumz" | "flyiniumz" | "psychiumz" | "buginium

z" |
           "rockiumz" | "ghostiumz" | "dragoniumz" | "darkiniumz" |
           "steeliumz" | "fairiumz" |
           // 専用Zクリスタル（約20種類）
           "pikaniumz" | "decidiumz" | "inciniumz" | "primariumz" |
           "tapuniumz" | "marshadiumz" | "aloraichiumz" | "snorliumz" |
           "eeviumz" | "mewniumz" | "pikashuniumz" | "ultranecroziumz"
       )
   }
   
   pub fn get_z_crystal_type(item: &str) -> Option<Type> {
       let id = normalize_item_id(item);
       match id.as_str() {
           "normaliumz" => Some(Type::Normal),
           "firiumz" => Some(Type::Fire),
           "wateriumz" => Some(Type::Water),
           "electriumz" => Some(Type::Electric),
           // ... 全18種類
           _ => None,
       }
   }
   
   pub fn can_use_z_move(pokemon: &Pokemon, move_id: &str) -> bool {
       if pokemon.z_move_used {
           return false;
       }
       
       let Some(ref crystal) = pokemon.z_crystal else { return false; };
       
       // 専用Zクリスタルチェック
       if is_signature_z_crystal(crystal, &pokemon.species, move_id) {
           return true;
       }
       
       // 汎用Zクリスタルチェック
       if let Some(crystal_type) = get_z_crystal_type(crystal) {
           let move_data = get_move(move_id);
           return move_data.map(|m| type_matches(m.move_type, crystal_type)).unwrap_or(false);
       }
       
       false
   }
   ```

3. Z技威力変換（Showdown: data/moves.ts#L17345+）:
   ```rust
   pub fn get_z_move_power(base_move: &MoveData) -> Option<u16> {
       // Showdown: data/moves.ts の威力変換テーブル
       let base_power = base_move.base_power?;
       
       Some(match base_power {
           0..=55 => 100,
           56..=65 => 120,
           66..=75 => 140,
           76..=85 => 160,
           86..=95 => 175,
           96..=100 => 180,
           101..=110 => 185,
           111..=125 => 190,
           126..=130 => 195,
           _ => 200,
       })
   }
   
   pub fn get_z_move_name(base_move: &MoveData, z_crystal: &str) -> String {
       let id = normalize_item_id(z_crystal);
       
       // 専用Z技
       if let Some(signature) = get_signature_z_move(z_crystal, base_move.name) {
           return signature.to_string();
       }
       
       // 汎用Z技
       match id.as_str() {
           "normaliumz" => "Breakneck Blitz",
           "firiumz" => "Inferno Overdrive",
           "wateriumz" => "Hydro Vortex",
           "electriumz" => "Gigavolt Havoc",
           "grassiumz" => "Bloom Doom",
           "iciumz" => "Subzero Slammer",
           "fightiniumz" => "All-Out Pummeling",
           "poisoniumz" => "Acid Downpour",
           "groundiumz" => "Tectonic Rage",
           "flyiniumz" => "Supersonic Skystrike",
           "psychiumz" => "Shattered Psyche",
           "buginiumz" => "Savage Spin-Out",
           "rockiumz" => "Continental Crush",
           "ghostiumz" => "Never-Ending Nightmare",
           "dragoniumz" => "Devastating Drake",
           "darkiniumz" => "Black Hole Eclipse",
           "steeliumz" => "Corkscrew Crash",
           "fairiumz" => "Twinkle Tackle",
           _ => "Breakneck Blitz",
       }.to_string()
   }
   ```

**成果物:**
- `pokemon-battle-core/src/sim/zmove.rs` （約300行）
- Z技威力変換テーブル完備
- テストケース（Z技検証）

---

### タスク F4: テラスタルシステム

**編集ファイル:** `pokemon-battle-core/src/sim/terastal.rs` (新規作成)

**目的:** テラスタルの完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` L1890-L1967 (canTerastallize, runTerastallize)
- `pokemon-showdown/data/moves.ts` L15234 (Tera Blast)
- `pokemon-showdown/sim/battle-actions.ts` L2123-L2189

**実施内容:**

1. Pokemon構造体の拡張:
   ```rust
   pub struct Pokemon {
       // ... 既存フィールド ...
       pub tera_type: Option<Type>,
       pub is_terastallized: bool,
       pub original_types: [Type; 2],
   }
   ```

2. テラスタル実行（Showdown: pokemon.ts#L1890-L1945）:
   ```rust
   pub fn can_terastallize(pokemon: &Pokemon) -> bool {
       !pokemon.is_terastallized && pokemon.tera_type.is_some()
   }
   
   pub fn terastallize(pokemon: &mut Pokemon) -> Result<()> {
       let tera_type = pokemon.tera_type
           .ok_or_else(|| anyhow!("No Tera Type"))?;
       
       if pokemon.is_terastallized {
           return Err(anyhow!("Already Terastallized"));
       }
       
       // 元のタイプを保存
       pokemon.original_types = pokemon.types;
       
       // 単一タイプに変更
       pokemon.types = [tera_type, tera_type];
       pokemon.is_terastallized = true;
       
       Ok(())
   }
   
   pub fn get_terastallized_type(pokemon: &Pokemon) -> Type {
       if pokemon.is_terastallized {
           pokemon.types[0]
       } else {
           pokemon.types[0] // デフォルト
       }
   }
   ```

3. STAB補正の変更（Showdown: battle-actions.ts#L2145-L2167）:
   ```rust
   pub fn terastal_stab_modifier(pokemon: &Pokemon, move_type: Type) -> f32 {
       if !pokemon.is_terastallized {
           // 通常STAB: 1.5倍
           if pokemon.types[0] == move_type || pokemon.types[1] == move_type {
               return 1.5;
           }
           return 1.0;
       }
       
       let tera_type = pokemon.types[0];
       
       if move_type == tera_type {
           // テラスタルSTAB
           if pokemon.original_types[0] == tera_type || pokemon.original_types[1] == tera_type {
               // 元のタイプと一致: 2.0倍
               2.0
           } else {
               // 元のタイプと不一致: 1.5倍
               1.5
           }
       } else {
           // 元のタイプと一致してもSTABなし
           1.0
       }
   }
   ```

4. テラバースト処理（Showdown: data/moves.ts#L15234+）:
   ```rust
   pub fn modify_tera_blast(
       move_data: &mut MoveData,
       attacker: &Pokemon,
   ) {
       if move_data.name != "Tera Blast" {
           return;
       }
       
       if attacker.is_terastallized {
           // タイプ変更
           let tera_type = attacker.types[0];
           move_data.move_type = type_to_string(tera_type);
           
           // 物理・特殊の判定
           if attacker.stats.atk > attacker.stats.spa {
               move_data.category = MoveCategory::Physical;
           } else {
               move_data.category = MoveCategory::Special;
           }
       }
   }
   ```

5. 特殊なテラスタル特性:
   ```rust
   pub fn apply_tera_ability_effects(pokemon: &mut Pokemon, state: &mut BattleState) {
       if !pokemon.is_terastallized {
           return;
       }
       
       match pokemon.ability.as_str() {
           "Tera Shell" => {
               // HP満タン時、全技こうかいまひとつ
               // battle.rs で実装
           }
           "Tera Shift" => {
               // 戦闘開始時に自動テラスタル
               // on_switch_in で実装
           }
           "Teraform Zero" => {
               // テラスタル時に天候・フィールドを無効化
               state.weather = None;
               state.field = None;
           }
           _ => {}
       }
   }
   ```

**成果物:**
- `pokemon-battle-core/src/sim/terastal.rs` （約250行）
- テラスタルSTAB計算関数完備
- テストケース（テラスタル検証）

---

## フェーズ7: 残タスク完全実装（並列5タスク）

### タスク R1: データ駆動型技の実装（600種類）

**編集ファイル:** `pokemon-battle-core/src/sim/moves/data_driven.rs` (新規作成)

**目的:** データから自動処理できる技を実装

**参照元:**
- `pokemon-showdown/data/moves.ts` 全技定義
- `pokemon-showdown/sim/battle-actions.ts` L1050-L1456

**実施内容:**

1. 汎用処理フレームワーク:
   ```rust
   pub fn execute_data_driven_move(
       move_data: &MoveData,
       attacker: &mut Pokemon,
       defender: &mut Pokemon,
       context: &BattleContext,
   ) -> MoveResult {
       // まもる判定
       if move_has_flag(move_data, FLAG_PROTECT) && defender.protect_active {
           return MoveResult::Protected;
       }
       
       // 特性による無効化
       if check_ability_immunity(defender, move_data) {
           return MoveResult::Immune;
       }
       
       // ダメージ計算
       let damage = if move_data.category != MoveCategory::Status {
           calculate_damage(attacker, defender, move_data, context)
       } else {
           0
       };
       
       // 追加効果
       if let Some(secondary) = secondary_effect_from_move(move_data.name, move_data) {
           apply_secondary_effect(attacker, defender, &secondary, context.rng);
       }
       
       MoveResult::Success { damage }
   }
   ```

**成果物:**
- `data_driven.rs` （約300行）
- 600種類の技が自動処理可能

---

### タスク R2-R5 の簡略版

**R2: コールバック型技**（250種類）- `callbacks.rs`  
**R3: 特殊ケース技**（100種類）- `special_cases.rs`  
**R4: 特性完全実装**（285種類）- `abilities/complete.rs`  
**R5: もちもの完全実装**（490種類）- `items/complete.rs`

各タスクの詳細は既存のフェーズ1-3のパターンに従って実装してください。

---

## タスク依存関係（更新版）

```
フェーズ1（技システム）:
M1, M2, M3, M4 → M5

フェーズ2（特性システム）:
A1 → A2, A3, A4（並列可能）

フェーズ3（もちものシステム）:
I1, I2, I3（並列可能）

フェーズ4（高度システム）:
S1（S2, S3は完了済み）

フェーズ5（検証）:
V2（V1は完了済み）

フェーズ6（フォルムチェンジ）:
前提: フェーズ1-5完了
F1, F2, F3, F4（並列可能）

フェーズ7（残タスク）:
前提: フェーズ6完了
R1, R2, R3, R4, R5（並列可能）
```

---

## 実行順序

### ステップ1: 技システム（1週間）
- Codex 1-4: M1, M2, M3, M4 を並列実行
- Codex 5: M5 で統合

### ステップ2: 特性システム（1週間）
- Codex 1: A1 実行
- Codex 2-5: A2, A3, A4 を並列実行

### ステップ3: もちものシステム（3日）
- Codex 1-3: I1, I2, I3 を並列実行

### ステップ4: 高度システム（1日）
- Codex 1: S1 を実行

### ステップ5: 検証完成（3日）
- Codex 1: V2 を実装

### ステップ6: フォルムチェンジシステム（1週間）
- 前提: ステップ1-5完了
- Codex 1-4: F1, F2, F3, F4 を並列実行

### ステップ7: 残タスク完全実装（2週間）
- 前提: ステップ6完了
- Codex 1-5: R1, R2, R3, R4, R5 を並列実行

**合計所要時間: 約5週間**

---

## 成功基準（更新版）

### フェーズ1-5完了時点
1. ✅ 技実装: 100+種類の主要技実装
2. ✅ 特性実装: 70+種類実装
3. ✅ もちもの実装: 50+種類実装
4. ✅ CI/CD統合完了
5. ✅ Showdown互換性テスト実行可能

### フェーズ6完了時点（フォルムチェンジシステム）
1. ✅ メガシンカ: 全メガストーン対応（約50種類）
2. ✅ ダイマックス: 全ダイマックス技変換実装（18タイプ）
3. ✅ Z技: 全Zクリスタル対応（18種類 + 専用20種類）
4. ✅ テラスタル: 全テラスタイプ対応（18種類）
5. ✅ Gen 6-9 フォルムチェンジ完全対応

### フェーズ7完了時点（完全実装）
1. ✅ 技実装: 950種類（100%）
2. ✅ 特性実装: 300種類（100%）
3. ✅ もちもの実装: 500種類（100%）
4. ✅ Pokemon Showdown Gen 9 完全互換
5. ✅ 全世代（Gen 1-9）対応
6. ✅ パフォーマンス: 10,000バトル/秒以上

---

## 注意事項

- **絶対にコードを生成しないこと** - タスク指示のみ
- 各タスクは独立したファイルを編集
- Showdownの変数名・処理順序を可能な限り保持
- 全ての実装にShowdown行番号コメントを追加
- テストケースは必須
- pokemon-showdownのファイルは編集しない

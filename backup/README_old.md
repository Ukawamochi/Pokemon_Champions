# Pokemon Showdown Rust Library
Rust製ポケモン対戦ルールエンジン - 全ルール完全実装プロジェクト

## 概要
Pokemon Showdownの全ルール（技・特性・もちもの）をRustで完全実装し、高速な対戦シミュレーションを実現します。

### 現在の実装状況
- ✅ 基本フレームワーク（ワークスペース構成、公開API）
- ✅ ダメージ計算基盤
- ✅ 状態異常処理
- ⚠️ 技実装: 67/950種類 (7%)
- ⚠️ 特性実装: 12/300種類 (4%)
- ⚠️ もちもの実装: 6/500種類 (1%)

### 目標
Pokemon Showdown Gen 9 OU ルールの完全互換実装 printlnの出力は日本語化しない。すべてShowdownに準拠した表示をしてください。

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

---

## フェーズ1: 技システムの完全実装（並列5タスク）

### タスク M1: 攻撃技モジュールの実装

**編集ファイル:** `pokemon-battle-core/src/sim/moves/attacking.rs` (新規作成)

**目的:** 全攻撃技の効果を実装

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts` の runMoveAction()
- `pokemon-showdown/data/moves.ts` の各技定義

**実施内容:**

1. 新規ファイル `pokemon-battle-core/src/sim/moves/attacking.rs` を作成

2. 攻撃技の効果ハンドラを実装:
   - 反動ダメージ技（すてみタックル、フレアドライブ等）
   - 吸収技（ギガドレイン、ドレインパンチ等）
   - 連続技（タネマシンガン、みだれづき等）
   - 2ターン技（ソーラービーム、そらをとぶ等）
   - 一撃必殺技（つのドリル、ぜったいれいど等）

3. 以下の関数を実装:
   ```
   pub fn apply_recoil_damage(attacker: &mut Pokemon, damage_dealt: u16, recoil: (u8, u8))
   pub fn apply_drain(attacker: &mut Pokemon, damage_dealt: u16, drain: (u8, u8))
   pub fn calculate_multihit_count(move_data: &MoveData, rng: &mut SmallRng) -> u8
   pub fn handle_charging_move(pokemon: &mut Pokemon, move_id: &str) -> bool
   pub fn handle_ohko_move(attacker: &Pokemon, defender: &Pokemon, move_id: &str, rng: &mut SmallRng) -> Option<u16>
   ```

4. Showdownの行番号をコメントで記載

**成果物:**
- `attacking.rs` ファイル
- `mod.rs` への統合（`pub mod attacking;`）

---

### タスク M2: 状態変化技モジュールの拡張

**編集ファイル:** `pokemon-battle-core/src/sim/moves/status.rs` (新規作成)

**目的:** 全状態変化技を実装

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts`
- `pokemon-showdown/data/moves.ts`

**実施内容:**

1. 既存の `handle_status_move()` を `status.rs` に移動・拡張

2. 以下のカテゴリの技を追加:
   - 能力変化技（全24種類の組み合わせ）
   - 回復技（ねむる、ねがいごと等）
   - 補助技（トリック、すりかえ、なげつける等）
   - フィールド技（ステルスロック、どくびし等）

3. 未実装技リストから優先度順に実装:
   - 使用頻度Top100技を優先
   - Showdownの技使用統計データを参照

**成果物:**
- `status.rs` ファイル（300行以上）
- 実装済み技リスト（コメント）

---

### タスク M3: 技フラグシステムの実装

**編集ファイル:** `pokemon-battle-core/src/sim/moves/flags.rs` (新規作成)

**目的:** 技のフラグ（contact, protect等）処理を実装

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts` の checkMoveMakesContact等

**実施内容:**

1. フラグ判定関数を実装:
   ```
   pub fn is_contact_move(move_data: &MoveData) -> bool
   pub fn is_sound_move(move_data: &MoveData) -> bool
   pub fn is_bullet_move(move_data: &MoveData) -> bool
   pub fn is_pulse_move(move_data: &MoveData) -> bool
   pub fn bypasses_protect(move_data: &MoveData) -> bool
   pub fn bypasses_substitute(move_data: &MoveData) -> bool
   ```

2. 既存の `move_has_flag()` を拡張

3. フラグによる効果発動を処理:
   - 接触技 → さめはだ、ゴツゴツメット発動
   - 音技 → みがわり貫通
   - 弾技 → ぼうだん無効化

**成果物:**
- `flags.rs` ファイル
- フラグ一覧表（コメント）

---

### タスク M4: 技の追加効果システム

**編集ファイル:** `pokemon-battle-core/src/sim/moves/secondary.rs` (新規作成)

**目的:** 技の追加効果（状態異常、能力変化等）を実装

**参照元:**
- `pokemon-showdown/sim/battle-actions.ts` の trySecondaryEffects()

**実施内容:**

1. 追加効果ハンドラを実装:
   ```
   pub struct SecondaryEffectContext {
       pub chance: u8,
       pub status: Option<Status>,
       pub stat_changes: Vec<(Stat, i8)>,
       pub target_self: bool,
   }
   
   pub fn apply_secondary_effect(
       attacker: &mut Pokemon,
       defender: &mut Pokemon,
       effect: &SecondaryEffectContext,
       rng: &mut SmallRng
   ) -> bool
   ```

2. Showdownの確率計算を完全一致:
   - SereneGrace特性（確率2倍）
   - 追加効果とメインエフェクトの独立判定

3. 主要技の追加効果実装:
   - でんじほう（100%まひ）
   - いわなだれ（30%ひるみ）
   - かみなりパンチ（10%まひ）等

**成果物:**
- `secondary.rs` ファイル
- 追加効果テストケース

---

### タスク M5: 技モジュール統合とテスト

**編集ファイル:** 
- `pokemon-battle-core/src/sim/moves/mod.rs` (新規作成)
- `pokemon-battle-core/src/sim/battle.rs` (execute_move関数の書き換え)

**目的:** M1-M4を統合し、既存battle.rsから呼び出す

**実施内容:**

1. `moves/mod.rs` を作成:
   ```
   pub mod attacking;
   pub mod status;
   pub mod flags;
   pub mod secondary;
   
   pub use attacking::*;
   pub use status::*;
   pub use flags::*;
   pub use secondary::*;
   ```

2. `battle.rs` の `execute_move()` を書き換え:
   - 技カテゴリ判定
   - 適切なモジュール関数を呼び出し
   - 既存ロジックは削除せず移行

3. テスト作成:
   - 各技カテゴリのテストケース
   - Showdown互換性テスト

**依存:** M1, M2, M3, M4完了後に実行

**成果物:**
- `moves/mod.rs`
- 更新された `battle.rs`
- テストファイル

---

## フェーズ2: 特性システムの完全実装（並列4タスク）

### タスク A1: 特性イベントシステムの設計

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/events.rs` (新規作成)

**目的:** 特性のイベントハンドラシステムを構築

**参照元:**
- `pokemon-showdown/sim/dex-abilities.ts`
- `pokemon-showdown/sim/battle.ts` のイベントシステム

**実施内容:**

1. 特性発動タイミングの列挙型定義:
   ```
   pub enum AbilityTrigger {
       OnStart,           // 場に出た時
       OnDamagingHit,     // ダメージを受けた時
       OnBeforeMove,      // 技使用前
       OnAfterMove,       // 技使用後
       OnModifyAtk,       // 攻撃力補正
       OnModifyDef,       // 防御力補正
       OnWeather,         // 天候による効果
       OnStatusImmunity,  // 状態異常無効化
       OnFaint,           // ひんし時
       OnSwitchIn,        // 交代時
       OnEndOfTurn,       // ターン終了時
   }
   ```

2. 特性効果のトレイト定義:
   ```
   pub trait AbilityEffect {
       fn on_trigger(&self, trigger: AbilityTrigger, context: &mut AbilityContext) -> EffectResult;
   }
   
   pub struct AbilityContext {
       pub pokemon: &mut Pokemon,
       pub opponent: &mut Pokemon,
       pub state: &mut BattleState,
       pub rng: &mut SmallRng,
   }
   ```

3. 特性効果の登録システム:
   ```
   pub struct AbilityRegistry {
       effects: HashMap<String, Box<dyn AbilityEffect>>,
   }
   ```

**成果物:**
- `abilities/events.rs`
- 特性システム設計ドキュメント

---

### タスク A2: ダメージ補正特性の実装

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/damage_modifiers.rs` (新規作成)

**目的:** ダメージ計算に関わる全特性を実装

**実施内容:**

1. 以下の特性を実装:
   - こんじょう（やけど時攻撃1.5倍）
   - てつのこぶし（パンチ技1.2倍）
   - すなのちから（砂嵐時特定タイプ1.3倍）
   - ちからもち（物理攻撃2倍）
   - ヨガパワー（物理攻撃2倍）
   - スロースタート（攻撃・素早さ半減）
   - ハードロック（効果抜群0.75倍）
   - フィルター（効果抜群0.75倍）
   - マルチスケイル（HP満タン時0.5倍）
   - ファーコート（物理防御2倍）

2. damage.rs に特性補正を統合:
   ```
   pub fn ability_attack_modifier(pokemon: &Pokemon, move_data: &MoveData) -> f32
   pub fn ability_defense_modifier(pokemon: &Pokemon, attacking_type: Type) -> f32
   ```

**成果物:**
- `damage_modifiers.rs`
- damage.rs への統合

---

### タスク A3: 状態変化特性の実装

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/status_abilities.rs` (新規作成)

**目的:** 状態異常・能力変化に関わる特性を実装

**実施内容:**

1. 状態異常無効化特性:
   - めんえき（どく無効）
   - じゅうなん（まひ無効）
   - みずのベール（やけど無効）
   - マグマのよろい（こおり無効）
   - ふみん、やるき（ねむり無効）

2. 場に出た時発動特性:
   - いかく（相手攻撃1段階下降）
   - ダウンロード（相手防御比較で攻撃or特攻上昇）
   - トレース（相手特性コピー）

3. 天候・フィールド設置特性:
   - すなおこし（砂嵐）
   - ひでり（晴れ）
   - あめふらし（雨）
   - ゆきふらし（霰）

**成果物:**
- `status_abilities.rs`
- pokemon.rs, battle.rs への統合

---

### タスク A4: その他特性の実装

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/misc_abilities.rs` (新規作成)

**目的:** その他の重要特性を実装

**実施内容:**

1. 接触ダメージ特性:
   - さめはだ（接触時1/8ダメージ）
   - てつのトゲ（接触時1/8ダメージ）
   - ほうし（接触時30%状態異常）

2. 回復特性:
   - ちょすい（水技で回復）
   - かんそうはだ（水技で回復、炎技でダメージ増）
   - ポイズンヒール（どく時毎ターン回復）

3. 行動制御特性:
   - はやあし（状態異常時素早さ1.5倍）
   - すいすい（雨時素早さ2倍）
   - ようりょくそ（晴れ時素早さ2倍）

**成果物:**
- `misc_abilities.rs`
- 特性実装完了リスト

---

## フェーズ3: もちものシステムの完全実装（並列3タスク）

### タスク I1: 戦闘用もちもの実装

**編集ファイル:** `pokemon-battle-core/src/sim/items/battle_items.rs` (新規作成)

**目的:** 対戦で使用される全もちものを実装

**参照元:**
- `pokemon-showdown/sim/dex-items.ts`
- `pokemon-showdown/sim/battle-actions.ts`

**実施内容:**

1. 火力強化系:
   - こだわりハチマキ（攻撃1.5倍、技固定）
   - こだわりメガネ（特攻1.5倍、技固定）
   - こだわりスカーフ（素早さ1.5倍、技固定）
   - いのちのたま（威力1.3倍、反動1/10）
   - たつじんのおび（効果抜群1.2倍）

2. 耐久系:
   - きあいのタスキ（HP満タン時瀕死回避）
   - ゴツゴツメット（接触時1/6ダメージ）
   - とつげきチョッキ（特防1.5倍、変化技使用不可）

3. 回復系:
   - たべのこし（毎ターン1/16回復）
   - くろいヘドロ（毒タイプ1/16回復）
   - オボンのみ（HP半分以下で1/4回復）

**成果物:**
- `battle_items.rs`
- もちもの効果テスト

---

### タスク I2: タイプ強化もちもの実装

**編集ファイル:** `pokemon-battle-core/src/sim/items/type_items.rs` (新規作成)

**目的:** タイプ強化系もちものを実装

**実施内容:**

1. 全18タイプの強化もちもの:
   - もくたん（炎1.2倍）
   - しんぴのしずく（水1.2倍）
   - じしゃく（電気1.2倍）
   - ... (全18種)

2. Zクリスタル対応準備（データ定義のみ）

3. damage.rs への統合:
   ```
   pub fn item_type_boost(item: &str, move_type: Type) -> f32
   ```

**成果物:**
- `type_items.rs`
- タイプブーストテスト

---

### タスク I3: もちもの消費・効果処理

**編集ファイル:** `pokemon-battle-core/src/sim/items/consumable.rs` (新規作成)

**目的:** 消費系もちものの処理を実装

**実施内容:**

1. きのみ系:
   - オボンのみ（HP1/4回復）
   - ラムのみ（状態異常治癒）
   - カゴのみ（ねむり治癒）
   - 半減きのみ（効果抜群半減）

2. もちもの消費管理:
   ```
   pub fn consume_item(pokemon: &mut Pokemon, item: &str)
   pub fn can_consume_item(pokemon: &Pokemon) -> bool
   pub fn restore_consumed_item(pokemon: &mut Pokemon) // リサイクル用
   ```

3. とくせい連動:
   - しぜんかいふく（きのみ効果1.5倍）
   - ほおぶくろ（きのみでHP追加回復）

**成果物:**
- `consumable.rs`
- pokemon.rs への統合

---

## フェーズ4: 高度な戦闘システム（並列3タスク）

### タスク S1: 交代処理の完全実装

**編集ファイル:** `pokemon-battle-core/src/sim/switching.rs` (新規作成)

**目的:** 交代に関わる全処理を実装

**実施内容:**

1. 強制交代技:
   - ほえる、ふきとばし（ランダム交代）
   - ドラゴンテール、ともえなげ（攻撃+交代）
   - とんぼがえり、ボルトチェンジ（攻撃後自分交代）

2. 交代阻止:
   - くろいまなざし、クモのす
   - アンコール中の交代制限
   - とんぼがえり後の強制交代

3. 交代時効果:
   - ステルスロック、まきびし、どくびしダメージ
   - とくせい発動（いかく、トレース等）
   - 天候・フィールド効果

**成果物:**
- `switching.rs`
- battle.rs への統合

---

### タスク S2: 天候・フィールドシステム拡張

**編集ファイル:** `pokemon-battle-core/src/sim/weather_field.rs` (新規作成)

**目的:** 天候・フィールドの全効果を実装

**実施内容:**

1. 天候効果の完全実装:
   - にほんばれ（炎1.5倍、水0.5倍、こおり溶解等）
   - あまごい（水1.5倍、炎0.5倍、かみなり必中等）
   - すなあらし（岩特防1.5倍、毎ターンダメージ等）
   - あられ（こおり以外ダメージ、ふぶき必中等）

2. フィールド効果:
   - エレキフィールド（電気1.3倍、ねむり無効）
   - グラスフィールド（草1.3倍、毎ターン回復、じしん半減）
   - サイコフィールド（エスパー1.3倍、先制技無効）
   - ミストフィールド（状態異常無効、ドラゴン半減）

3. 特性連動:
   - すなかき、すいすい（素早さ2倍）
   - ようりょくそ（素早さ2倍）
   - ゆきかき（素早さ2倍）

**成果物:**
- `weather_field.rs`
- 天候・フィールドテスト

---

### タスク S3: ひんし・勝敗判定の拡張

**編集ファイル:** `pokemon-battle-core/src/sim/faint_handler.rs` (新規作成)

**目的:** ひんし関連の特殊処理を実装

**実施内容:**

1. ひんし時発動効果:
   - みちづれ（相手も道連れ）
   - ゆうばく（相手HP1/4減少）
   - ほろびのうた（3ターン後全員ひんし）

2. ひんし回避:
   - きあいのタスキ
   - こらえる、みきり
   - がんじょう特性

3. 複数ひんし時の処理順:
   - 同時ひんし判定
   - 交代順序の決定

**成果物:**
- `faint_handler.rs`
- エッジケーステスト

---

## フェーズ5: Showdown完全互換検証（並列2タスク）

### タスク V1: 自動互換性テスト生成

**編集ファイル:** `tools/generate_showdown_tests.js` (新規作成)

**目的:** Showdownとの互換性を自動検証

**実施内容:**

1. Showdownでバトルログ生成:
   ```javascript
   // 固定seedでバトル実行
   // ダメージ値、状態変化等を記録
   // JSON形式で出力
   ```

2. Rustテストコード自動生成:
   - Showdownログを解析
   - 同一条件のテストケース生成
   - 期待値との比較

3. 1000ケース以上のテスト生成

**成果物:**
- `generate_showdown_tests.js`
- `tests/showdown_compat/` 配下にテストファイル群

---

### タスク V2: 差分解析ツール

**編集ファイル:** `tools/diff_analyzer.rs` (新規作成)

**目的:** Showdownとの差分を自動検出・報告

**実施内容:**

1. バトルログ比較ツール:
   - ターンごとのダメージ比較
   - 状態変化の差分検出
   - 勝敗の一致確認

2. 差分原因の推定:
   - ダメージ計算式のズレ
   - 乱数シードのズレ
   - 処理順序の違い

3. HTML形式のレポート生成

**成果物:**
- `diff_analyzer.rs`
- CI/CD統合スクリプト

---

## タスク依存関係

```
フェーズ1（技システム）:
M1 ─┐
M2 ─┤
M3 ─┼→ M5（統合）
M4 ─┘

フェーズ2（特性システム）:
A1 → A2, A3, A4（並列実行可能）

フェーズ3（もちものシステム）:
I1, I2, I3（並列実行可能）

フェーズ4（高度システム）:
S1, S2, S3（並列実行可能、フェーズ1-3完了後）

フェーズ5（検証）:
V1, V2（並列実行可能、全フェーズ完了後）
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

### ステップ4: 高度システム（3日）
- Codex 1-3: S1, S2, S3 を並列実行

### ステップ5: 検証（2日）
- Codex 1-2: V1, V2 を並列実行

### ステップ6: フォルムチェンジシステム（1週間）
- Codex 1-4: F1, F2, F3, F4 を並列実行

### ステップ7: 残タスク完全実装（2週間）
- Codex 1-5: 未実装技・特性・もちものを分担実装

---

## フェーズ6: フォルムチェンジシステム（並列4タスク）

### タスク F1: メガシンカシステム

**編集ファイル:** `pokemon-battle-core/src/sim/mega_evolution.rs` (新規作成)

**目的:** メガシンカの完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` の canMegaEvo(), runMegaEvo()
- `pokemon-showdown/data/items.ts` のメガストーン定義
- `pokemon-showdown/data/pokedex.ts` のメガシンカフォルム

**実施内容:**

1. `Pokemon` 構造体に以下フィールドを追加:
   ```rust
   pub is_mega: bool,
   pub can_mega_evolve: bool,
   pub mega_stone: Option<String>,
   ```

2. メガシンカ判定関数を実装:
   ```rust
   pub fn can_mega_evolve(pokemon: &Pokemon) -> bool
   pub fn get_mega_species(base_species: &str, mega_stone: &str) -> Option<&'static str>
   pub fn is_mega_stone(item: &str) -> bool
   ```

3. メガシンカ実行関数を実装:
   ```rust
   pub fn mega_evolve(pokemon: &mut Pokemon) -> Result<()>
   ```
   - 種族値変更（POKEDEX から Mega フォルムのデータ取得）
   - タイプ変更（例: Charizard-Mega-X は Fire/Dragon）
   - 特性変更（例: Mega Launcher, Aerilate 等）
   - HP以外のステータス再計算

4. メガストーンとメガシンカフォルムのマッピング:
   - Venusaurite → Venusaur-Mega
   - Charizardite X → Charizard-Mega-X
   - Charizardite Y → Charizard-Mega-Y
   - 全メガストーン（約50種類）に対応

5. バトル開始時の検証:
   - 1パーティに1体のみメガシンカ可能
   - メガストーン保持確認

**成果物:**
- `mega_evolution.rs` ファイル
- メガシンカテストケース（全メガシンカフォルムの種族値・タイプ・特性検証）

---

### タスク F2: ダイマックスシステム

**編集ファイル:** `pokemon-battle-core/src/sim/dynamax.rs` (新規作成)

**目的:** ダイマックスの完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` の canDynamax(), runDynamax()
- `pokemon-showdown/data/moves.ts` のダイマックス技変換
- `pokemon-showdown/sim/battle-actions.ts` のダイマックス処理

**実施内容:**

1. `Pokemon` 構造体に以下フィールドを追加:
   ```rust
   pub is_dynamaxed: bool,
   pub dynamax_level: u8,  // 0-10
   pub dynamax_turns: u8,  // 残りターン数
   pub base_max_hp: u16,   // ダイマックス前のHP
   ```

2. ダイマックス判定・実行関数を実装:
   ```rust
   pub fn can_dynamax(pokemon: &Pokemon, battle_rules: &BattleRules) -> bool
   pub fn dynamax(pokemon: &mut Pokemon) -> Result<()>
   pub fn revert_dynamax(pokemon: &mut Pokemon)
   ```

3. HP2倍処理:
   - 現在HP・最大HPを2倍に
   - ひんし時または3ターン経過で元に戻す
   - HP割合を維持（50%なら戻っても50%）

4. 技変換システム:
   ```rust
   pub fn get_max_move(base_move: &str, move_type: Type, category: MoveCategory) -> MaxMoveData
   ```
   - ダイアタック（Normal物理）
   - ダイサンダー（Electric特殊）
   - ダイジェット（Flying物理・特殊）→ 味方素早さ+1
   - ダイウォール（Status）→ まもる相当
   - 全18タイプ + 追加効果実装

5. ダイマックスレベルによる倍率:
   - Lv0: HP x1.5
   - Lv10: HP x2.0
   - 線形補間

6. 制約:
   - 1試合1回のみ
   - 3ターン制限
   - キョダイマックス対応（一部ポケモン専用技）

**成果物:**
- `dynamax.rs` ファイル
- ダイマックス技変換テーブル
- ダイマックステストケース

---

### タスク F3: Z技システム

**編集ファイル:** `pokemon-battle-core/src/sim/zmove.rs` (新規作成)

**目的:** Z技の完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` の canZMove(), runZMove()
- `pokemon-showdown/data/items.ts` のZクリスタル定義
- `pokemon-showdown/data/moves.ts` のZ技威力変換

**実施内容:**

1. `Pokemon` 構造体に以下フィールドを追加:
   ```rust
   pub z_crystal: Option<String>,
   pub z_move_used: bool,
   ```

2. Zクリスタル判定関数を実装:
   ```rust
   pub fn is_z_crystal(item: &str) -> bool
   pub fn get_z_crystal_type(item: &str) -> Option<Type>
   pub fn can_use_z_move(pokemon: &Pokemon, move_id: &str) -> bool
   ```

3. Z技変換関数を実装:
   ```rust
   pub fn get_z_move_power(base_move: &MoveData) -> Option<u16>
   pub fn get_z_move_name(base_move: &MoveData, z_crystal: &str) -> String
   ```
   - 威力変換テーブル:
     - 基本威力55-60 → Z技威力100
     - 基本威力65-75 → Z技威力120
     - 基本威力80-85 → Z技威力160
     - 基本威力90-95 → Z技威力175
     - 基本威力100 → Z技威力180
     - 基本威力110 → Z技威力185
     - 基本威力120-125 → Z技威力190
     - 基本威力130 → Z技威力195
     - 基本威力140+ → Z技威力200

4. 状態変化技のZ技化:
   - Z-パワー系: 攻撃+1, 特攻+1, 素早さ+1 等
   - 例: Z-つるぎのまい → 攻撃+2 + 攻撃ランク+2

5. 専用Z技:
   - Pikanium Z + Volt Tackle → Catastropika
   - Snorlium Z + Giga Impact → Pulverizing Pancake
   - 全専用Z技実装

6. Zクリスタル一覧:
   - Normalium Z, Firium Z, Waterium Z, Electrium Z 等18種類
   - 専用Zクリスタル（約20種類）

**成果物:**
- `zmove.rs` ファイル
- Z技威力変換テーブル
- Z技テストケース

---

### タスク F4: テラスタルシステム

**編集ファイル:** `pokemon-battle-core/src/sim/terastal.rs` (新規作成)

**目的:** テラスタルの完全実装

**参照元:**
- `pokemon-showdown/sim/pokemon.ts` の canTerastallize(), runTerastallize()
- `pokemon-showdown/data/moves.ts` のテラバースト
- `pokemon-showdown/sim/battle-actions.ts` のテラスタル処理

**実施内容:**

1. `Pokemon` 構造体に以下フィールドを追加:
   ```rust
   pub tera_type: Option<Type>,
   pub is_terastallized: bool,
   pub original_types: [Type; 2],  // テラスタル前のタイプ
   ```

2. テラスタル判定・実行関数を実装:
   ```rust
   pub fn can_terastallize(pokemon: &Pokemon) -> bool
   pub fn terastallize(pokemon: &mut Pokemon) -> Result<()>
   pub fn get_terastallized_type(pokemon: &Pokemon) -> Type
   ```

3. タイプ変更処理:
   - テラスタル後は単一タイプに変更
   - `pokemon.types = [tera_type, tera_type]`
   - 元のタイプは `original_types` に保存

4. STAB補正の変更:
   - 通常STAB: 1.5倍
   - テラスタルSTAB（元タイプと一致）: 2.0倍
   - テラスタルSTAB（元タイプと不一致）: 1.5倍
   - 実装例:
     ```rust
     pub fn terastal_stab_modifier(pokemon: &Pokemon, move_type: Type) -> f32
     ```

5. テラバースト（Tera Blast）の処理:
   - テラスタル時: タイプがテラスタイプに変化
   - 物理・特殊の自動判定（攻撃 > 特攻なら物理）

6. 特殊なテラスタル特性:
   - Tera Shell: テラスタル時、HP満タンなら全技こうかいまひとつ
   - Tera Shift: 戦闘開始時に自動テラスタル
   - Teraform Zero: テラスタル時に天候・フィールドを無効化

7. 制約:
   - 1試合1回のみ
   - 戦闘中永続（ひんしまで継続）

**成果物:**
- `terastal.rs` ファイル
- テラスタルSTAB計算関数
- テラスタルテストケース

---

## フェーズ7: 残タスク完全実装（並列5タスク）

### タスク R1: データ駆動型技の実装（600種類）

**編集ファイル:** `pokemon-battle-core/src/sim/moves/data_driven.rs` (新規作成)

**目的:** データから自動処理できる技を実装

**参照元:**
- `pokemon-showdown/data/moves.ts` の全技定義
- `pokemon-showdown/sim/battle-actions.ts` の基本ダメージ処理

**実施内容:**

1. 汎用処理フレームワーク:
   ```rust
   pub fn execute_data_driven_move(
       move_data: &MoveData,
       attacker: &mut Pokemon,
       defender: &mut Pokemon,
       context: &BattleContext,
   ) -> MoveResult
   ```

2. フラグベース処理（約400種類）:
   - 基本攻撃技（タイプ・威力・命中のみ）
   - 単純な追加効果技（10%まひ、10%やけど等）
   - 単純な能力変化技（攻撃+1、特攻+2等）
   - 既存の `secondary.rs` を拡張統合

3. 定型パターン技（約200種類）:
   - 連続技（2-5回攻撃）
   - 反動技（すてみタックル、フレアドライブ等）
   - 吸収技（ギガドレイン、ドレインパンチ等）
   - 優先度付き技（でんこうせっか、しんそく等）

**成果物:**
- `data_driven.rs` ファイル
- 汎用処理システム
- 600種類の技実装

---

### タスク R2: コールバック型技の実装（250種類）

**編集ファイル:** `pokemon-battle-core/src/sim/moves/callbacks.rs` (新規作成)

**目的:** 個別ロジックが必要な技を実装

**参照元:**
- `pokemon-showdown/data/moves.ts` の各種コールバック
- basePowerCallback, onTryHit, onHit 等

**実施内容:**

1. HP依存威力技:
   - eruption（ふんか）、waterspout（しおふき）: 威力 = 150 * HP割合
   - flail（じたばた）、reversal（きしかいせい）: HP低いほど高威力

2. 状況依存威力技:
   - electroball（エレキボール）: 素早さ比で威力変動
   - gyroball（ジャイロボール）: 相手の素早さ/自分の素早さ * 25
   - weatherball（ウェザーボール）: 天候で威力2倍・タイプ変化

3. 特殊な命中判定技:
   - toxic（どくどく）: 毒タイプ使用時は必中
   - blizzard（ふぶき）: あられ時必中
   - thunder（かみなり）: あめ時必中

4. フォーム変化技:
   - relicsong（いにしえのうた）: メロエッタのフォルムチェンジ
   - conversion（テクスチャー）: 技1のタイプに変化

5. 交代効果技:
   - voltswitch（ボルトチェンジ）、uturn（とんぼがえり）
   - batonpass（バトンタッチ）: 能力変化引き継ぎ

**成果物:**
- `callbacks.rs` ファイル
- 250種類の技実装

---

### タスク R3: 特殊ケース技の実装（100種類）

**編集ファイル:** `pokemon-battle-core/src/sim/moves/special_cases.rs` (新規作成)

**目的:** 極めて複雑なロジックを持つ技を実装

**参照元:**
- `pokemon-showdown/data/moves.ts` の複雑な技
- `pokemon-showdown/sim/battle-actions.ts` の特殊処理

**実施内容:**

1. カウンター系技:
   - counter（カウンター）: 受けた物理技の2倍返し
   - mirrorcoat（ミラーコート）: 受けた特殊技の2倍返し
   - metalburst（メタルバースト）: 受けたダメージの1.5倍返し

2. 一撃必殺技（既存拡張）:
   - guillotine（ハサミギロチン）、horndrill（つのドリル）
   - sheercold（ぜったいれいど）、fissure（じわれ）
   - レベル差判定・命中率処理

3. 固定ダメージ技:
   - seismictoss（ちきゅうなげ）、nightshade（ナイトヘッド）: レベル分ダメージ
   - dragonrage（りゅうのいかり）: 固定40ダメージ
   - sonicboom（ソニックブーム）: 固定20ダメージ

4. 複雑な条件付き技:
   - revenge（リベンジ）: ダメージ受けた後なら威力2倍
   - facade（からげんき）: 状態異常時威力2倍
   - hex（たたりめ）: 状態異常相手に威力2倍
   - venoshock（ベノムショック）: 毒状態相手に威力2倍

5. マルチターン技:
   - futuresight（みらいよち）、doomdesire（はめつのねがい）: 2ターン後攻撃
   - perishsong（ほろびのうた）: 3ターン後全員ひんし（既存拡張）

6. フィールド依存技:
   - risingvoltage（ライジングボルト）: エレキフィールドで威力2倍
   - grassyglide（グラススライダー）: グラスフィールドで先制
   - expandingforce（ワイドフォース）: サイコフィールドで全体攻撃

**成果物:**
- `special_cases.rs` ファイル
- 100種類の技実装

---

### タスク R4: 特性の完全実装（285種類）

**編集ファイル:** `pokemon-battle-core/src/sim/abilities/complete.rs` (新規作成)

**目的:** 全特性を実装

**実施内容:**

1. カテゴリ別実装:
   - ダメージ補正特性（残り100種類）
   - 状態変化特性（残り80種類）
   - フィールド・天候特性（残り50種類）
   - 交代時発動特性（いかく、威嚇等 30種類）
   - その他特性（残り25種類）

2. イベントフック拡張:
   ```rust
   pub trait AbilityHooks {
       fn on_switch_in(&self, pokemon: &mut Pokemon, state: &mut BattleState);
       fn on_take_damage(&self, pokemon: &mut Pokemon, damage: u16) -> u16;
       fn on_attack(&self, attacker: &mut Pokemon, defender: &Pokemon) -> f32;
       // 等
   }
   ```

**成果物:**
- `abilities/complete.rs` ファイル
- 全285種類の特性実装

---

### タスク R5: もちものの完全実装（490種類）

**編集ファイル:** `pokemon-battle-core/src/sim/items/complete.rs` (新規作成)

**目的:** 全もちものを実装

**実施内容:**

1. カテゴリ別実装:
   - 回復きのみ（オボンのみ、ラムのみ等 80種類）
   - 能力強化もちもの（こうかくレンズ、いのちのたま等 50種類）
   - タイプ強化もちもの（残り20種類）
   - メガストーン（50種類）
   - Zクリスタル（38種類）
   - 状態異常治療きのみ（カゴのみ等 10種類）
   - その他もちもの（残り242種類）

2. 消費判定システム:
   ```rust
   pub fn should_consume_item(pokemon: &Pokemon, trigger: ItemTrigger) -> bool
   pub fn consume_item(pokemon: &mut Pokemon)
   ```

3. きのみの発動タイミング:
   - HP条件（25%、50%等）
   - 状態異常治療
   - タイプ相性（半減きのみ）

**成果物:**
- `items/complete.rs` ファイル
- 全490種類のもちもの実装

---

## タスク依存関係（更新版）

```
フェーズ1-5: （既存）

フェーズ6（フォルムチェンジ）:
F1, F2, F3, F4 → 並列実行可能（各自独立ファイル）

フェーズ7（残タスク）:
R1, R2, R3 → 並列実行可能（技モジュール）
R4 → 並列実行可能（特性モジュール）
R5 → 並列実行可能（もちものモジュール）
```

---

## 成功基準（更新版）

### フェーズ1-5完了時点
1. ✅ 技実装: 800種類以上（85%）
2. ✅ 特性実装: 250種類以上（80%）
3. ✅ もちもの実装: 400種類以上（80%）
4. ✅ Showdown互換性テスト: 95%以上のケースで一致
5. ✅ パフォーマンス: 10,000バトル/秒以上

### フェーズ6完了時点（フォルムチェンジシステム）
1. ✅ メガシンカ: 全メガストーン対応（約50種類）
2. ✅ ダイマックス: 全ダイマックス技変換実装（18タイプ + キョダイマックス）
3. ✅ Z技: 全Zクリスタル対応（18種類 + 専用20種類）
4. ✅ テラスタル: 全テラスタイプ対応（18種類）
5. ✅ Gen 6-9 完全互換

### フェーズ7完了時点（完全実装）
1. ✅ 技実装: 950種類（100%）
2. ✅ 特性実装: 300種類（100%）
3. ✅ もちもの実装: 500種類（100%）
4. ✅ Pokemon Showdown Gen 9 完全互換
5. ✅ 全世代（Gen 1-9）対応

---

## 注意事項

- **絶対にコードを生成しないこと** - タスク指示のみ
- 各タスクは独立したファイルを編集
- Showdownの変数名・処理順序を可能な限り保持
- 全ての実装にShowdown行番号コメントを追加
- テストケースは必須
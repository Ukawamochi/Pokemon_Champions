# 特性イベントシステム設計（A1）

このドキュメントは、`pokemon-battle-core` の特性（Ability）を Showdown に寄せて実装するためのイベントハンドラ基盤（A1）設計です。

## 参照元（Showdown）

- `pokemon-showdown/sim/dex-abilities.ts`：特性データの読み込みと型（onStart などのイベントフック）
- `pokemon-showdown/sim/battle.ts`：`eachEvent` / `runEvent` を中心とするイベント実行モデル

## 目的

- 特性の発動タイミングを列挙し、Rust側で統一的に呼び出せるようにする
- 実装（A2/A3/A4）を「特性ごとの効果オブジェクト」に分離し、`battle.rs` の肥大化を防ぐ

## コア概念

### `AbilityTrigger`

Showdown のイベント名（例: `Start`, `DamagingHit`）と 1:1 対応させるのではなく、Rust側では用途別に意味のある粒度へ正規化します。

- `OnStart`（場に出た時）: Showdown `Start`
- `OnSwitchIn`（交代時）: Showdown `SwitchIn`
- `OnEndOfTurn`（ターン終了時）: Showdown `Residual`
- `OnDamagingHit`（ダメージを受けた時）: Showdown `DamagingHit`
- `OnBeforeMove` / `OnAfterMove`: Showdown `BeforeMove` / `AfterMove`
- `OnModifyAtk` / `OnModifyDef`: Showdown `ModifyAtk` / `ModifyDef`
- `OnWeather`: Showdown `Weather`
- `OnStatusImmunity`: Showdown `TryImmunity`
- `OnFaint`: Showdown `Faint`

### `AbilityContext`

イベント発火時に必要な mutable な参照を束ねます。

- `pokemon`: 発動主体
- `opponent`: 相手（single battle 前提）
- `state`: `BattleState`（天候・フィールド等を含む）
- `rng`: 乱数

### `EffectResult`

最小限の結果分類です。

- `NoEffect`: 何も起こさない（未登録/条件不成立含む）
- `Applied`: 何らかの効果を適用した
- `Blocked`: 行動や効果をブロックした（例: 状態異常無効化）

## 登録と発火

`AbilityRegistry` に `ability_id -> Box<dyn AbilityEffect>` で登録し、バトル側は必要なタイミングで `trigger(...)` を呼び出します。

今後の統合方針（A2以降）:

- `battle.rs` で `AbilityRegistry` を保持（もしくは静的に初期化）
- 既存の能力分岐（例: `Guts` などのハードコード）を段階的に `AbilityEffect` 実装へ移行


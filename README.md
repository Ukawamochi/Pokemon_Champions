# Pokemon Battle Matrix (Rust プロトタイプ)

6体から3体選出する固定チーム A/B について、PS（Pokémon Showdown）の挙動を最小限に再現し、20×20 の勝率行列を CSV で出力します。

## 使い方

```bash
cargo run --release -- --teams teams.json --sims-per-cell 500 --seed 1234 --output matrix.csv
```

各セルは A 勝率（引き分けは 0.5）を小数点4桁で出力します。RNG はシードで再現可能です。

## 入力形式

`teams.json`（抜粋）:

```json
{
  "teamA": [
    {
      "name": "Charizard",
      "types": ["fire", "flying"],
      "stats": { "hp": 360, "atk": 293, "def": 280, "spa": 348, "spd": 295, "spe": 328 },
      "moves": [
        { "name": "Flamethrower", "type": "fire", "category": "special", "power": 90, "accuracy": 100, "priority": 0 }
      ]
    }
  ],
  "teamB": [ ...6体... ]
}
```

各技: `name`, `type`, `category` (`physical` / `special` / `status`), `power`, `accuracy`(0–100), `priority`(整数)。追加フィールドは無視します。

## 実装しているサブセット

- 3体戦・控えは自動繰り出し。任意の交代は無し。
- 行動順: 優先度 > 素早さ > 同速は乱数。`battle.rs`のコメントに PS 参照あり（`pokemon-showdown/sim/battle.ts`）。
- 命中: 技の命中率を 1 回判定。// 参考: `pokemon-showdown/sim/battle.ts: tryMoveHit` の `randomChance(accuracy, 100)`.
- ダメージ: PS の `getDamage` を簡略化。レベル50、物理/特殊で攻防を切替、威力、STAB、タイプ相性、乱数0.85–1.00、無効なら0、最低1ダメージ。// 参考: `pokemon-showdown/sim/damage.ts`.
- タイプ相性表: PS の倍率に倣った表を使用。// 参考: `pokemon-showdown/sim/dex-data.ts`.
- 勝敗: 相手3体全滅で勝ち。同一ターン同時瀕死は引き分け扱い。
- RNG: `--seed` で固定。セルごとに決定的なシードを派生。

## 未対応（入力にあっても無視・弾く）

- 持ち物、特性、状態異常、天候、急所、副次効果、PP、反動、ランク補正、交代技、連続技、全体技、守る系、設置技、場の効果など。
- `status` カテゴリ技は何もしない。
- ターン上限 500（無限ループ防止）。到達時は引き分け。

## ファイル構成

- `Cargo.toml`: 依存（serde/serde_json/anyhow/rand/rayon）。
- `src/`: `main.rs` CLI、`lib.rs` 入口、`model.rs` データ構造、`types.rs` 相性表、`battle.rs` バトル核心（PS参照コメント付き）、`matrix.rs` 行列生成とCSV書き出し。
- `teams.json`: 入力サンプル。
- `tests/battle_tests.rs`: 行動順、命中、STAB/相性の単体テスト。

## テスト

```bash
cargo test
```

優先度/素早さ/同速乱数の順、命中確率、STAB・タイプ相性の反映を確認。

## PS 参照箇所まとめ

- 行動順の優先度・素早さ・同速乱数: `pokemon-showdown/sim/battle.ts` (`Battle.getActionSpeed`, `comparePriority` 相当)。
- 命中判定: `pokemon-showdown/sim/battle.ts` (`tryMoveHit` / `randomChance`)。
- ダメージ計算と乱数幅: `pokemon-showdown/sim/damage.ts` (`getDamage`)。
- タイプ相性表: `pokemon-showdown/sim/dex-data.ts`。

# Pokemon Battle Matrix (Rust プロトタイプ)

6体から3体選出する固定チーム A/B について、PS（Pokémon Showdown）のシングルバトル挙動を再現し、20×20 の勝率行列を CSV で出力します。

## 使い方

```bash
cargo run --release -- --teams teams.json --sims-per-cell 500 --seed 1234 --output matrix.csv
```

追加のポリシー/MCTS用フラグ:

```
--policy random|mcts        # デフォルト random。mcts を選ぶと両陣営に MCTS を適用
--mcts-iters N              # 反復回数で打ち切り
--mcts-ms MS                # ミリ秒の時間バジェットで打ち切り
--rollout-horizon H         # 0 なら終局まで、>0 なら H ターンでロールアウトを打ち切り
--uct-c C                   # UCT の探索定数
--mcts-mode joint|myaction  # 同時手 (joint; 推奨) か相手をサンプルする簡易モード
```

例:  
`cargo run --release -- --teams teams.json --sims-per-cell 50 --seed 1234 --output matrix.csv --policy mcts --mcts-ms 200 --rollout-horizon 12`

各セルは A 勝率（引き分けは 0.5）を小数点4桁で出力します。RNG はシードで再現可能です。

### ビルド前提
- Node.js 18+ と npm が必要です（`tools/extract_items.js` で items.ts を変換するため）。
- 一度だけ `npm install`（リポジトリルート、ts-node/TypeScript 用）と `cd pokemon-showdown && npm ci` を実行してください。

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

各技: `name`, `type`, `category` (`physical` / `special` / `status`), `power`, `accuracy`(0–100), `priority`(整数)。  
拡張フィールド（任意）:

- `pp`（省略時10）、`critRate`（急所段階、0〜3）
- `secondary` { `chance`, `status`, `boosts`, `selfBoosts` }
- `recoil`, `drain`, `boosts`, `selfBoosts`, `status`/`statusChance`
- `setWeather`, `hazard` (`stealthrock`/`spikes`/`toxicspikes`)
- `protect`（守る系）、`switchAfter`（ボルチェン等）、`multihit` { `minHits`, `maxHits` }
- `trickRoom` (true でトリックルームを展開/解除)
- アイテム/特性: `item`, `ability`（すべて items.ts 由来の静的テーブルに登録。効果は順次拡張中）

ポケモン: `item`, `ability` を追加で指定可能（省略可）。

## 実装しているサブセット（シングル）

- 3体戦・控えは自動繰り出し（相手瀕死・交代技時に最初の健在ポケモンを繰り出し）。
- 行動順: 優先度(＋fractional優先度) > 素早さ > 同速乱数。クイッククロウ/カスタムのみ/こうこうのしっぽ/のろいのおふだ系の fractional 優先度に対応。トリックルーム中は素早さ反転。// 参考: `pokemon-showdown/sim/battle.ts: comparePriority`, `pokemon-showdown/data/items.ts: fractionalPriority`。
- 命中: 単一判定。// 参考: `pokemon-showdown/sim/battle.ts: tryMoveHit`。
- ダメージ: レベル50、ランク補正、STAB、タイプ相性、天候補正、画面（リフレクター/光の壁）、乱数0.85–1.00、急所1.5倍、最低1。// 参考: `pokemon-showdown/sim/damage.ts`。
- 状態異常: ねむり（1–3T）、まひ（行動不可25%、素早さ1/4）、やけど（攻撃1/2、定数ダメ）、どく/もうどく（定数/蓄積ダメ）、こおり（20%で解凍）。
- 天候: 晴れ/雨の火水補正、砂・霰/雪のスリップダメージ。
- 急所: 段階0〜3で 1/24, 1/8, 1/2, 1。// 参考: `pokemon-showdown/sim/damage.ts`。
- 副次効果: `secondary`/`statusChance` で状態・ランク変化を確率適用。
- PP: 消費あり。全PP枯渇時はあばれる（Struggle）を使用。
- 反動/吸収: `recoil`, `drain` に対応。ストライク（Struggle）は最大HPの1/4反動。
- ランク補正: 攻防特防特攻素早さを ±6 段まで。
- 交代技: `switchAfter` true で技後に控えへ自動交代。
- 連続技: `multihit` (min/max) で複数回ヒット。
- 全体技: シングルなので単体扱い（威力補正なし）。
- 守る系: `protect` true でそのターンの攻撃を無効。
- 設置技: `hazard` でステルスロック/まきびし/どくびし。
- 場の効果: リフレクター/光の壁を簡易対応（ダメージ0.5倍、ターン管理簡略）。
- 持ち物: Life Orb, Choice Band/Specs/Scarf, Focus Sash, Leftovers, Rocky Helmet, Heavy-Duty Boots, Sitrus Berry, クイッククロウ、カスタムのみ、こうこうのしっぽ、のろいのおふだ、きょうせいギプス/くろいてっきゅう(素早さ半減) を実装。  
  items.ts をビルド時に `tools/extract_items.js` 経由で Rust の静的テーブルに変換し、その他のアイテムも識別は可能（効果は未実装扱い）。順次拡張予定。
- 特性: 主要なものを抜粋（Levitate, Sturdy, Guts, Adaptability）。未対応特性は無効果。
- 勝敗: 相手3体全滅で勝ち。同一ターン同時瀕死は引き分け。
- RNG: `--seed` で固定。セルごとに決定的なシードを派生。

## 未対応・簡略化（入力にあっても無効果）

- 特性/持ち物は上記リスト以外は現状無効果。items.ts 由来の静的テーブルには全アイテムが登録されるが、効果ロジックは順次実装。火傷攻撃半減や天候補正なども一部簡略化。
- 場の効果はリフレクター/光の壁のみ簡易対応。オーロラベール/しんぴのまもり等は未対応。
- 先制技の追加判定や特性・持ち物による行動順操作（でんきだま・せいしんりょく等）は未対応。
- 追加効果の詳細（ひるみ、能力ダウン／アップの優先度、複雑な連続技のヒット分布など）は簡略。
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

## 対戦モード (CLI)

`battle/` 配下に人間 vs ランダムAI の対戦CLIを追加しています。

```bash
cd battle
CARGO_NET_OFFLINE=true cargo run -- --teams ../teams.json --seed 42 --human-side A
```

AI側を MCTS にする場合の例:

```bash
cd battle
cargo run -- --teams ../teams.json --seed 42 --human-side A --policy mcts --mcts-iters 150 --rollout-horizon 8 --uct-c 1.4
```

`--human-side` でプレイヤー側（A/B）を指定できます。ターンごとに相手HPバー・状態・控えリスト、自分の技4つと交代先をターミナルに表示し、入力を受け付けます。交代が必要なときは交代先の入力を促します。

## PS 参照箇所まとめ

- 行動順の優先度・素早さ・同速乱数: `pokemon-showdown/sim/battle.ts` (`Battle.getActionSpeed`, `comparePriority` 相当)。
- 命中判定: `pokemon-showdown/sim/battle.ts` (`tryMoveHit` / `randomChance`)。
- ダメージ計算・急所・乱数幅: `pokemon-showdown/sim/damage.ts` (`getDamage`, `criticalHit`)。
- 状態異常・天候・定数ダメージ: `pokemon-showdown/sim/battle.ts` `residualEvent` 近辺。
- ステルスロック/まきびし/どくびし: `pokemon-showdown/sim/field.ts` `runSwitch`, `runEntryHazards`。
- タイプ相性表: `pokemon-showdown/sim/dex-data.ts`。
- アイテム一覧: `pokemon-showdown/data/items.ts` を `tools/extract_items.js` で変換。

# belt-agent 汎用スキル設計書

## 概要

belt-agent CLI を駆動する LLM に対して、正確なプロトコルとルールを提供する汎用スキル。パイプライン固有スキル（feature-dev, debug-flow 等の belt 移植版）がこのスキルを内部参照し、belt-agent の使い方を一貫させる。

## スコープ

### 含むもの

- belt-agent CLI の駆動ループ（`init → next → verify → step`）
- 各コマンドの JSON レスポンス解釈と次アクションの判定
- verify / validate / confirm / regate のハンドリング手順
- well-known config keys の語彙定義
- HARD-GATE: validate 検証義務

### 含まないもの

- フェーズの実行方法（サブエージェント構成、fix dispatch 戦略、TDD 等）
- 監査手法（done-criteria フォーマット、evidence plan、phase-auditor dispatch）
- セッション管理（handover）
- belt-core の内部実装
- エラー回復手順（将来必要になれば追加）

## 配置

| 項目 | 値 |
|------|-----|
| ファイル | `skills/belt-agent/SKILL.md` |
| 構成 | 単一ファイル |
| 言語 | **英語**（リポジトリ公開コンテンツは英語。`docs/plans/`, `docs/specs/` のみ日本語許容） |
| 配布 | belt リポジトリの成果物として同梱 |
| 呼び出し | パイプライン固有スキルが内部参照。直接ユーザーが呼ぶものではない |

## Belt Protocol ループ

belt-agent を駆動する LLM の基本ループ:

```
belt-agent init <pipeline.yml> [--arg key=value ...]
  ↓
loop {
  phase = belt-agent next [--run <id>]

  if phase.completed:
    break                        # パイプライン完了

  execute(phase)                 # LLM がフェーズを実行（config 参照）

  if phase.gate is not empty:
    result = belt-agent verify [--run <id>]

    while result.verdict == "FAIL":
      fix(result)                # LLM が gate 失敗を修正
      result = belt-agent verify

  if phase.confirm or phase.validate is not empty:
    belt-agent step --confirm [--run <id>]
  else:
    belt-agent step [--run <id>]
}
```

### ルール

- `next` が返す JSON でフェーズの全情報を取得（description, config, artifacts, gate, validate, confirm, regate）
- gate があるフェーズでは `verify` で PASS を得てから `step`
- gate がないフェーズ（confirm only 等）は `verify` をスキップして直接 `step`
- `step` の成否は JSON の `advanced` フィールドで判定
- `status` はいつでも呼べる（run 全体の状態確認）

## コマンドレスポンス解釈

### next

| レスポンス | アクション |
|-----------|-----------|
| `completed: true` | ループ終了。パイプライン完了 |
| `phase` が返る | フェーズ情報を読み取り、実行に入る |

### verify

| レスポンス | アクション |
|-----------|-----------|
| `verdict: "PASS"` | gate 通過。step に進む |
| `verdict: "FAIL"` | `checks` を読み、失敗した gate を修正して再 verify |

### step

| レスポンス | アクション |
|-----------|-----------|
| `advanced: true`, `to` あり | 遷移成功。次の `next` へ |
| `advanced: true`, `completed: true` | パイプライン完了 |
| `advanced: false`, `reason: "confirmation_required"` | `--confirm` が必要。validate を検証してから再実行 |

### regate

`next` が返す `regate` フィールドにフェーズ ID のリストがある場合、`verify` はそれらのフェーズの gate も再検証する。LLM は:

- regate 対象の gate が FAIL したら、**そのフェーズの修正** を行う（現フェーズではなく regate 対象フェーズ）
- 全 regate + 現フェーズの gate が PASS するまで verify → fix を繰り返す

## HARD-GATE

```
<HARD-GATE>
validate 基準が存在するフェーズでは、各基準を検証せずに
belt-agent step --confirm を実行してはならない。

validate は belt が返す「LLM が判断すべき基準のリスト」である。
belt はこの基準が実際に評価されたかを知りようがない。
--confirm フラグは「基準を確認した」という LLM の宣言であり、
未検証のまま渡すことはプロトコル違反である。
</HARD-GATE>
```

### HARD-GATE にしない制約

以下は [BELT-21](https://linear.app/neko-neko/issue/BELT-21) で belt-core Engine レベルの強制に移行予定:

- verify せずに step → belt-agent が拒否する
- gate FAIL のまま step → belt-agent が拒否する
- max_retries 超過 → belt-agent が拒否する

## Well-known Config Keys

`config` は belt がパススルーする opaque map。以下のキーのみ汎用スキルが語彙と意味を定義する。

| Key | 型 | 意味 |
|-----|-----|------|
| `config.skill` | `string` | このフェーズで invoke すべきスキル名 |

### ルール

- LLM は未知の config キーを無視してよい（前方互換性）
- パイプライン固有スキルが独自キーを自由に追加してよい（belt は干渉しない）
- dispatch の実装（どのエージェントをどう起動するか）はパイプライン固有スキルの責務

## 設計判断の根拠

### なぜ well-known config keys を最小にしたか

`config.agents`（並列 dispatch）、`config.executor`（委譲先）、`config.tdd`（TDD）、`config.options`（選択肢）等は検討したが除外した。これらは LLM の実行戦略やドメイン固有の概念であり、belt の汎用プロトコルが規定すべきレベルではない。パイプライン固有スキルが `config` に独自キーとして定義すれば済む。

### なぜ監査手法をスコープ外にしたか

既存の phase-auditor + done-criteria による監査は、独立サブエージェントによる客観的検証に大きな価値がある（トレーサビリティ照合、トートロジーテスト検出、回帰分析等）。しかしこれは belt が管掌する決定論的プロトコルではなく、ユーザーが保持するスキル側の責務。belt は `validate` で「何を確認すべきか」を返し、「誰がどう確認するか」には干渉しない。

### なぜ HARD-GATE を1つに絞ったか

verify-before-step、gate FAIL 時の step 拒否、max_retries 超過は belt-core Engine レベルで強制可能（BELT-21）。CLI が構造的に防げる違反をスキルの HARD-GATE（LLM の自律性に依存）で防ぐのは設計として弱い。CLI で制御不可能な「validate 基準の実質的検証」だけが HARD-GATE の正当な対象。

## 関連

- [BELT-20](https://linear.app/neko-neko/issue/BELT-20) — belt 再設計 MVP
- [BELT-21](https://linear.app/neko-neko/issue/BELT-21) — belt-core Engine に verify-before-step / max_retries 強制ガード追加
- `docs/specs/2026-04-06-belt-redesign.md` — belt 再設計仕様書
- `examples/feature-dev/SKILL.md` — feature-dev belt 化サンプル（本スキルの参考実装）

---
name: feature-dev
description: >-
  Quality-gated development workflow orchestrated by belt.
  Design -> Spec Review -> Plan -> Plan Review -> Execute -> Doc Audit -> Smoke Test -> Code Review -> Test Review -> Integrate.
user-invocable: true
---

# feature-dev

belt で駆動する品質ゲート付き開発ワークフロー。

## Quick Start

```bash
belt init ./pipeline.yml
belt init ./pipeline.yml --doc --smoke --e2e --codex --iterations=3
```

## Belt Protocol

belt が verification (自動検証) と状態遷移を担う。LLM が validation (妥当性確認) とフェーズ実行を担う。

```
loop {
  phase = belt next                  # 現フェーズ取得 (artifacts, gate, validate 含む)
  execute(phase)                     # フェーズ実行 (config.skill 参照)
  result = belt verify               # gate (verification) 実行

  if result.verdict == FAIL {
    fix_issues(result)               # gate 失敗を修正
    continue
  }

  if result.validate {
    run_validation(result.validate)   # validation 基準を LLM/phase-auditor が判断
  }

  if phase.confirm or result.validate {
    belt step --confirm              # 明示的遷移 (confirm or validate がある場合)
  } else {
    belt step                        # 自動遷移
  }
}
```

### Verification vs Validation

- **gate (verify)**: belt が自動実行。cargo test, file_exists, git_clean 等。PASS/FAIL を返す
- **validate**: belt は基準を返すだけ。LLM が判断し、必要なら phase-auditor agent を dispatch
- `validate:` があるフェーズは `belt step --confirm` 必須

### artifacts

`belt next` が返す `artifacts` はそのフェーズが生成すべきファイル。
LLM はこれを見て出力先を把握する。gate の file_exists と対応する。

## Phase Naming

サブパイプラインを使うフェーズは namespace 付き:

```
design/explore           ← design-exploration
design/synthesize
design/write-design
spec-review/review       ← review-cycle
spec-review/triage
spec-review/fix
plan                     ← リーフフェーズ
code-review/review       ← review-cycle (7 perspectives)
code-review/triage
code-review/fix
integrate                ← リーフフェーズ
```

## Phase Execution Rules

### config.skill

スキルを invoke する。config + args から引数を構成:

- `args.codex = true` → Codex 並列レビューを追加
- `args.iterations = 3` → N-way 投票
- `args.swarm = true` → エージェントチーム

### config.agents (探索フェーズ)

リスト内のエージェントを並列 dispatch し、結果を統合する。

### config.executor (実装フェーズ)

サブエージェントに実装を委譲。`config.tdd = true` なら TDD。

### config.options (選択フェーズ)

ユーザーに選択肢を提示。

## Regate Protocol

code-review / test-review に `regate: [execute, smoke-test]` が設定。
findings 修正後にコード変更が発生した場合:

1. `belt verify` が regate 対象の gate を自動再検証
2. regate FAIL → 修正 → 再 verify
3. regate PASS → 現フェーズの gate を検証
4. 全 PASS → validation → `belt step --confirm`

## Fix Dispatch

| Phase | 修正担当 | 戦略 |
|-------|---------|------|
| design/* | オーケストレーター | 設計書修正、探索エージェント再起動 |
| spec-review/* | オーケストレーター | 設計書修正 |
| plan | オーケストレーター | 計画書修正 |
| plan-review/* | オーケストレーター | 計画書修正 |
| execute | feature-implementer | タスク分解 → TDD 再実装 |
| doc-audit | feature-implementer | ドキュメント修正 |
| smoke-test | feature-implementer | アプリケーションバグ修正 |
| code-review/* | feature-implementer | コード修正 → regate |
| test-review/* | feature-implementer | テスト修正 → regate |
| integrate | オーケストレーター | 手動対応 |

## Handover

以下のタイミングで /handover を実行:

- confirm / validate を含むフェーズの完了後
- コンテキスト逼迫時 (即座に)

`belt status` の出力が handover の project-state を兼ねる。

## Red Flags

**Never:**
- belt verify をスキップしてフェーズ遷移
- validate 基準を検証せずに --confirm
- gate FAIL のまま次フェーズへ
- max_retries 超過を無視して継続
- impact severity high+ の findings を自動延期

**Always:**
- フェーズ遷移時に現フェーズをアナウンス
- confirm / validate フェーズ完了後に handover 検討
- 全サブエージェント結果を待機してから統合
- artifacts を確認して出力先を把握

## Linear Sync

`args.linear = true` の場合、各フェーズ完了後に linear-sync を実行。

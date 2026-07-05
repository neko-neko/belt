---
date: 2026-07-05
status: Draft
depends-on:
  - plugins/belt/skills/feature-dev/pipeline.yml
  - plugins/belt/skills/design/pipeline.yml
  - plugins/belt/skills/build/pipeline.yml
  - plugins/belt/skills/verify/pipeline.yml
  - plugins/belt/skills/handover/checkpoint.yml
  - plugins/belt-agent/skills/protocol/SKILL.md
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
---

# Sonnet-lean pipeline 設計 — intake 統合・5-stage 圧縮・/belt:goal

## 背景

2026-07-02 spec (`2026-07-02-pipeline-split-design.md`) の実測が示した状況は変わっていない:

- belt pipeline 利用は 2026-04-22〜05-07 に集中し、以後 feature-dev の起動はゼロ
- 完走 run の実測: 11.7〜27.0h
- backend/CLI repo では monkey-test / dogfood が構造的に SKIP-all(2〜8 分のセレモニー)
- 成果物規律(design/plan 文書)は定着し、10-phase の runtime 強制だけが捨てられた

7/2 の Plan A/B で stage 分割(design / diagnose / build / verify + `invoke.pipeline` 合成)は完了したが、**プロンプト層の重さは据え置き**である。今回のユーザー要求は3点:

1. **Sonnet で回る**こと(オーケストレータ含め全て)。現行はプロトコル遵守・ファイル参照連鎖・裁量指示の認知負荷が Opus 級を前提としている
2. **チケット→E2E を証跡込みで一気通貫**。現行はチケット入力の受け口がなく、triage(Linear 起票)と分断されている
3. **brainstorming / grilling の対話コスト削減**。one-question-at-a-time は1質問ごとにフルコンテキスト再読み込みが走り、10 問で 10 ターン分のコストになる

### 現行の定量

| 項目 | 現行 (feature-dev --e2e) |
|---|---|
| leaf phases | 10 |
| criteria ファイル合計 | 約 1,100 行 |
| plugins/ プロンプト総量 | 約 5,400 行 |
| レビューサブエージェント | 7 体 (spec 3 + code 4) |
| 対話型スキル | 3 回 (brainstorming / grill-me / writing-plans) |
| 1 フェーズの読み込み | SKILL.md + supplement + criteria = 200〜400 行 |

## Decisions

| # | 決定 | 根拠 |
|---|------|------|
| D1 | belt 内で書き直す。belt-agent CLI (Rust production code) は無変更 | 決定論的状態機械・resume・gate は Sonnet でこそ価値がある資産。重いのはプロンプト層 |
| D2 | /belt:goal は一括質問型。AskUserQuestion 4 問×最大 2 ラウンド | one-question-at-a-time の対話ターン爆発を排除しつつ、推定だけで進まない |
| D3 | オーケストレータ含め全て Sonnet で回る前提で書く | 裁量指示(「LLM judgment」等)を決定的ルールに置換。Opus/Fable では単に速くなる |
| D4 | チケット入力は Linear ID / URL / 自由文の汎用受け口 | triage の収集ロジックの流用。`/belt:feature-dev CLA-42` で起動できる |
| D5 | 構造は 5-stage 圧縮(案1)。leaf 10→7 (--e2e なし 6) | 3-stage 最小案は checkpoint を失いコンテキスト肥大リスク。記述のみ圧縮案はフェーズ間オーバーヘッドが残る |
| D6 | narrative notes (phase-*.md × 10) を廃止し evidence.md 1 本に追記 | 証跡の一覧性。review-pack でそのまま受け入れ判定できる形 |
| D7 | レビューア 7→3 体。dedup は機械的ルールに置換 | Sonnet で「語彙の重なりを LLM judgment」は不安定。2 ファイルマージなら決定的に書ける |
| D8 | plugin version 0.3.0 | 0.2.0 からの breaking change(スキル削除・pipeline 構造変更) |

## 新パイプライン構造 (feature-dev)

```
/belt:feature-dev <Linear ID | URL | 自由文> [--e2e] [--codex]

feature-dev/pipeline.yml (合成、現行踏襲):
  - design stage (../design/pipeline.yml)
      [1] intake   — invoke /belt:goal → goal-sheet.md
      [2] design   — 設計+テスト戦略+タスク分解 → design.md
                     (--e2e 時 scenarios.yml も)、spec-reviewer 1 体で検証
  - [3] checkpoint — ../handover/checkpoint.yml (無変更)
  - build stage (../build/pipeline.yml)
      [4] execute      — TDD 実装 (/subagent-driven-development、無変更)
      [5] code-review  — code-reviewer + quality-reviewer の 2 体並列
      [6] e2e          — when e2e。invoke /belt:verify(monkey-test + dogfood 統合の 1 パス実行に書き直し)
      [7] integrate    — /worktrunk (無変更)
```

- leaf 10→7(--e2e なし 6)。design stage の 4 leaf(design / test-scenarios / spec-review / plan)を 2 leaf に統合
- regate は消滅する(design 内 `spec-review → [test-scenarios]`、build 内 `code-review → [execute]` はいずれも統合により leaf が同居または隣接となり、gate + validate で足りる)。build 内 `code-review → [execute]` のみ維持を検討したが、evidence.md の追記時刻で改変検出が代替できるため落とす
- verify stage pipeline (`../verify/pipeline.yml`) は廃止し、build 内の 1 leaf `e2e` が `invoke: skill: /belt:verify` を呼ぶ(合成 depth が 1 段浅くなる)。/belt:verify は単体起動も従来どおり可能

## /belt:goal 仕様

`user-invocable: true`。パイプライン外で grilling / brainstorming の軽量代替として単体起動できる。

1. **入力判別**(決定的ルール): `[A-Z]+-\d+` 形式 → `linear issue view` で本文+コメント取得 / URL → WebFetch(Slack URL は slackcli)/ それ以外 → 自由文としてそのまま
2. **コードベース調査を先に行う**: 入力から触りそうなモジュールを特定し、コードで答えられる論点を潰す(grilling の原則を継承)
3. **一括質問**: 人間しか決められない論点だけを AskUserQuestion で提示。4 問×1 ラウンド、未解決が残る場合のみ 2 ラウンド目。各問に推奨案を必ず付ける
4. **出力**: `docs/features/<YYYY-MM-DD-topic>/goal-sheet.md` — Goal / In-scope / Out-of-scope / Acceptance criteria / Open risks の 5 セクション固定

対話ターン: 現行 10+ → 1〜2。

## レビューア統合 (7→3)

| 新エージェント | 統合元 | 出力 |
|---|---|---|
| spec-reviewer | feasibility + ui-design + cross-cutting-spec | findings-spec.json |
| code-reviewer | security + cross-cutting | findings-code.json |
| quality-reviewer | test + ai-antipattern | findings-quality.json |

- ui-design 観点は spec-reviewer 内で「UI 記述がなければ観点スキップ」の条件節にする(現行の early-exit エージェントを丸ごと 1 体使う無駄を排除)
- **dedup の置換**: 「file + line が一致したら severity の高い方を残す。それ以外は残す」の機械的ルール。観点優先順位テーブル・語彙重なり判定は削除
- --codex は現行どおり findings-codex.json を別枠追加(dedup 対象外)

## 証跡: evidence.md 方式

- `belt://current/notes/phase-*.md`(10 ファイル)を廃止
- `docs/features/<topic>/evidence.md` にフェーズ完了ごとオーケストレータが追記。エントリは定型:

```markdown
## <phase-id> — <UTC timestamp>
- Command: <実行したコマンド>
- Observed: <観測した出力の要約(exit code / 件数 / PASS・FAIL)>
- Artifacts: <成果物への相対リンク>
```

- gate は `file_exists: docs/features/*/evidence.md` に一本化
- 並列サブエージェントは evidence.md に書かない(オーケストレータのみが追記。競合なし)
- 最終 leaf (integrate) の validate に「evidence.md が全 leaf 分のエントリを持つ」を含める

## Sonnet プロンプト原則(plugins/belt-agent/references/authoring-principles.md として新設)

1. **1 フェーズ 1 ファイル**: criteria は pipeline.yml の `validate:` インラインリスト(3〜6 項目)へ。criteria/ ディレクトリと supplement 連鎖は廃止
2. **裁量指示の排除**: 「judge」「appropriately」「LLM judgment」を明示的 if-then に置換
3. **サブエージェントプロンプトは自己完結**: 解決済みパス・出力スキーマ・完了条件を全てプロンプト内に書く
4. **対話は一括**: AskUserQuestion で束ねる。one-question-at-a-time 禁止
5. **表より一行分岐**: 参照表・マトリクスは、条件が 3 つ以下なら if-then の箇条書きにする

AGENTS.md の Plugin Authoring セクションからこのファイルを参照する。

## スキル・エージェントの増減

| 対象 | 処置 |
|---|---|
| skills/goal | 新設 (user-invocable) |
| skills/test-scenarios | 削除(design leaf に吸収) |
| skills/monkey-test | 削除(/belt:verify に吸収) |
| skills/verify | 維持・書き直し(monkey-test + dogfood 統合の 1 パス実行。pipeline.yml は廃止し SKILL.md のみに) |
| skills/spec-review, skills/code-review | 維持・書き直し(ディスパッチ先が 1 体 / 2 体に) |
| skills/design, skills/build, skills/feature-dev | 書き直し |
| skills/bug-fix, skills/diagnose | 第 2 弾(本 spec のスコープ外。同じ原則で rca + fix-plan + fix-plan-review → 1〜2 leaf に圧縮) |
| skills/handover, skills/resume, handover/checkpoint.yml | 無変更 |
| agents/ 7 体 | 3 体に統合(上表) |
| belt-agent plugin (protocol, 5 analysis agents) | protocol SKILL.md の参照更新のみ |

## 触らないもの

- belt-agent CLI / belt-core の production code(インライン `validate:` は既存機能)
- checkpoint(handover → /clear → resume)の仕組み
- gate セマンティクス
- superpowers の brainstorming / grilling 本体(/belt:goal が代替になるだけで削除しない)

## リスクと対策

| リスク | 対策 |
|---|---|
| design 1 leaf 統合で成果物の質が低下 | goal-sheet が仕様を先に固定するため design は「どう作るか」に集中できる。spec-reviewer の validate と Acceptance criteria 突合で担保 |
| lock tests (feature_dev_refresh.rs / bug_fix_refresh.rs / review_skills_refresh.rs) が旧構造を固定 | Rust integration tests の改訂を実装計画に含める(production code は無変更)。bug-fix は第 2 弾まで旧構造のまま動くよう、旧 leaf ファイルは bug-fix が参照するものだけ残す |
| evidence.md 追記漏れ | 各 leaf の validate 先頭に「evidence.md に本 phase のエントリがある」を置く |
| /belt:test-scenarios・/belt:monkey-test 利用者の導線切れ | README / AGENTS.md のスキル一覧を同時更新。/belt:verify は維持されるため E2E 単体起動の導線は残る |

## Open Questions

1. bug-fix 第 2 弾の diagnose 圧縮粒度(rca + fix-plan を 1 leaf にするか 2 leaf にするか)— 第 2 弾の設計時に決める
2. goal-sheet の Acceptance criteria を e2e leaf の scenarios.yml 生成にどこまで直結させるか — 実装中に design leaf の記述で調整

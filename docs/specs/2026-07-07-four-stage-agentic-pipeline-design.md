---
date: 2026-07-07
status: Draft
depends-on:
  - plugins/belt/skills/feature-dev/pipeline.yml
  - plugins/belt/skills/bug-fix/pipeline.yml
  - plugins/belt/skills/design/pipeline.yml
  - plugins/belt/skills/build/pipeline.yml
  - plugins/belt/skills/diagnose/pipeline.yml
  - plugins/belt/skills/verify/SKILL.md
  - plugins/belt/skills/goal/SKILL.md
  - plugins/belt/skills/handover/checkpoint.yml
  - plugins/belt-agent/skills/protocol/SKILL.md
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
---

# 4-stage agentic pipeline 設計 — plan 分離・QA 必須化・agent ゼロベース再編・定型業務 skills

## 背景

2026-07-05 sonnet-lean rewrite (v0.3.0) で feature-dev は design → checkpoint → build の
3 ステージ合成になったが、次の課題が残る:

1. **実装計画が design.md に同居** — 設計判断 (architecture) とタスク分解 (plan) が
   1 文書・1 phase に混在し、「設計済み案件のタスク分解だけやり直す」再利用ができない。
2. **QA がオプトイン** — `--e2e` フラグ必須で、既定ルートでは browser 検証が走らない。
   e2e-report.md はテキストのみで、人間がレビューできる一次証跡 (スクショ・実行ログ) の
   保存義務がない。CLI リポジトリでは SKIPPED を自己宣言できる。
3. **confirm セレモニー** — 全 leaf phase が `confirm: true` で、2026-07-02 離脱分析の
   「SKIP 連打」要因が残存。
4. **agent の陳腐化** — 探索系 3 agent (code-explorer / code-architect / impact-analyzer)
   は使用条件が狭く出番が少ない。phase-auditor は audit-protocol の dead wiring が
   2026-07-02 検証で確認済み。
5. **定型業務の不在** — 要件定義・ドキュメント執筆という頻出業務に skill がない。

## 決定事項

| # | 決定 | 内容 |
|---|------|------|
| D1 | plan ステージ分離 | design.md は設計判断のみ。plan.md (Test Strategy + Tasks) と scenarios.yml を新設 plan ステージが持つ |
| D2 | QA 常時必須 | `--e2e` フラグ廃止。QA は feature-dev / bug-fix の必須ステージ。SKIP はユーザー承認の記録がある場合のみ |
| D3 | 証跡は対象適応 | scenario `kind: browser` → スクショ、`kind: cli` → 実コマンド実行の transcript。証跡ファイルの実在を validate で強制 |
| D4 | confirm は判断点のみ | design 承認 / plan 承認 / integrate の 3 点 + checkpoint (構造上の一時停止)。build / qa は自律進行 |
| D5 | integrate を orchestrator 直下へ | build から除去し feature-dev / bug-fix の leaf phase に。QA を通らず統合できないトポロジーを保証 |
| D6 | agent ゼロベース再編 8→6 | explorer (3→1 統合) + implementer + spec/code/quality-reviewer + qa-verifier (新設)。phase-auditor 廃止 |
| D7 | 定型業務 skill 2 本 | /belt:requirements (インタビュー型要件定義) + /belt:docs (汎用ドキュメント執筆)。いずれも pipeline.yml なし |
| D8 | belt-core 本体は無変更 | 既存 engine 機能 (invoke / with / confirm / gate / validate) のみで実現。Rust 変更は lock tests に限定 |
| D9 | 証跡バイナリは repo 非コミット | `.belt/runs/<run-id>/qa/` に生成。公開先は auto チェーン (PR コメント → Linear 添付 → ローカル保持+警告)。`belt.toml` の `qa.evidence = "pr" \| "linear" \| "local" \| "auto"` で固定可。qa-report.md (テキスト) のみ docs/features にコミット |
| D10 | PR 画像は evidence branch 方式 | orphan branch `qa-evidence` (run ごとサブディレクトリ、prune 可) に push。public repo はインライン埋め込み、private はリンクに degradation。Linear はネイティブ添付 API (private でもインライン) |
| D11 | 実行環境は scenarios.yml `setup:` | 起動コマンド / URL / 後始末を plan ステージで確定し計画承認の対象にする。qa-verifier は宣言どおり実行するだけ |
| D12 | QA 中の修正は再レビューなし | qa-verifier の再検証のみ。修正 commit 一覧を evidence.md に記録し integrate で報告 |
| D13 | 探索異常は advisory | 全観測を証跡付き記録・integrate で報告。受入基準に抵触する異常のみ FAIL に昇格 |
| D14 | integrate leaf はインライン重複 | feature-dev / bug-fix 双方に直接定義。lock test が両者の同一性を assert (共有 sub-pipeline 化しない) |

## 全体トポロジー

```
feature-dev = design → plan → pre-execute-handover → build → qa → integrate
bug-fix     = diagnose ─────→ pre-execute-handover → build → qa → integrate
```

- design / plan / build / qa は各々 pipeline.yml を持つ単体起動可能なステージ skill。
- feature-dev / bug-fix は `invoke.pipeline` で合成し、integrate のみ orchestrator 直下の
  leaf phase (`invoke.skill: /worktrunk`)。
- args は `codex: bool` のみ (`e2e` 廃止)。`with` は bare 全文字列形式
  (`codex: "args.codex"`) — expander が置換する唯一の形式。
- 展開後 leaf 数: feature-dev = 8 (design 2 + plan 1 + checkpoint 1 + build 2 + qa 1 +
  integrate 1)、bug-fix = 8 (diagnose 3 + checkpoint 1 + build 2 + qa 1 + integrate 1)。
- 合成 depth は 2 (orchestrator → stage pipeline) で expander の上限 4 内。
- cross-stage regate は engine 制約上不可のため regate は宣言しない (現行踏襲)。

### 人間のタッチポイント (feature-dev で 4 つ)

1. **design confirm** — 設計承認 (spec-review 済み design.md)
2. **plan confirm** — 計画承認。自律実行前の最終タッチ
3. **checkpoint** — `/belt:handover` → `/clear` → `/belt:resume`。構造上必須の一時停止 (現行維持)
4. **integrate confirm** — wt merge / gh pr create の選択 + 持ち越し事項の一括報告

build / qa は confirm なし。code-review の critical/high finding は自動修正し、QA の FAIL は
修正 → 再検証する。自動解決できなかった項目だけを integrate confirm 時に証跡付きで報告する。

## ステージ仕様

### design (改修: intake → design)

- intake: 現行どおり `/belt:goal` invoke。goal-sheet.md + evidence.md 生成。confirm なしに変更
  (質問バッチ自体が人間対話のため、追加 confirm は冗長)。
- design: design.md を執筆し `/belt:spec-review` (対象 design.md) を自律実行。confirm あり。
- design.md のセクションは **`## Architecture` + `## Key Decisions`** (棄却案 1 行ずつ) のみ。
  Test Strategy / Implementation Tasks は plan.md へ移動。
- scenarios.yml はこのステージでは書かない (plan へ移動)。
- args: `codex` のみ。

### plan (新設: 1 phase)

goal-sheet.md + design.md を読み、以下を執筆して `/belt:spec-review` (対象 plan.md、出力
`findings-plan.json`) を自律実行する。confirm あり。

- `docs/features/<topic>/plan.md`:
  - `## Test Strategy` — 受入基準ごとに検証テスト (level: unit/integration/qa + テスト名)
  - `## Tasks` — checkbox リスト。全タスクが対象ファイルとテスト名を持つ
- `docs/features/<topic>/scenarios.yml` — **常時執筆**。受入基準ごとに 1 scenario 以上。

validate (抜粋):
- plan.md に Test Strategy / Tasks の 2 セクションがあり、全タスクが対象ファイルを名指す
- goal-sheet.md の全受入基準が Test Strategy と scenarios.yml の双方に対応を持つ
- 全 scenario が `kind: browser | cli` を持つ
- `kind: browser` の scenario が 1 つでもあれば `setup:` (起動コマンド / URL) が宣言されている
- findings-plan.json の critical/high が解消済み (自動修正またはユーザー却下記録)

plan レビューが design.md の欠陥を指摘した場合: cross-stage regate 不可 + phase 完了後の
docs/features 手編集禁止 (mtime filter) のため、finding を「設計承認済み事項への異議」として
ユーザーに提示し、承認を得て design ステージを単体再実行する (自動 regate しない)。

### build (改修: execute → code-review)

- integrate と e2e phase を除去。args: `codex` のみ。両 phase とも **confirm なし**。
- execute: plan.md (bug run は fix-plan.md) の Tasks を implementer サブエージェントに TDD で
  dispatch。validate は「plan 文書の全 checkbox が checked / テスト・リンタ green を evidence.md
  に記録」(現行踏襲、参照先を plan.md に変更)。
- code-review: 現行 `/belt:code-review` invoke (2 agent 並列 + merge)。triage 変更:
  critical/high は**自動修正**する。修正が design/plan の承認済みスコープを変える場合、または
  2 回試行して解消しない場合のみ deferred として evidence.md に記録し integrate へ持ち越す。

### qa (新設: 1 phase、/belt:verify を置換)

- qa-verifier サブエージェント (実装者から独立) を dispatch:
  1. 環境準備 — scenarios.yml の `setup:` (起動コマンド / URL / 後始末) を宣言どおり実行。
     起動失敗は scenario FAIL と区別して報告する。
  2. scenario 再生 — `docs/features/<topic>/scenarios.yml` (bug run は rca-scenarios.yml) を
     再生。`kind: browser` は agent-browser でステップごとにスクショ撮影、`kind: cli` は
     実コマンドを実行し stdout/stderr 全文を transcript に保存。証跡は
     `.belt/runs/<run-id>/qa/` に書く (repo 非コミット、D9)。
  3. 探索パス — 変更スコープ周辺の境界値・異常系・再読み込み等をプローブ (上限 15 分)。
     全観測を証跡付きで記録。**advisory 扱い**とし、受入基準に抵触するものだけ FAIL に
     昇格する (D13)。
  4. qa-report.md 執筆 (形式は後述)。
- FAIL ループ: qa-verifier は**コードを修正しない** (独立性契約)。FAIL は main session が
  implementer に修正を dispatch → qa-verifier が該当 scenario を再検証。2 巡で解消しない
  FAIL は validate によりユーザー判断へ (「明示的に受容」のみ通過)。
- QA 中の修正に code-review は再実行しない (D12)。修正 commit 一覧を evidence.md の qa
  エントリに必ず記録し、integrate confirm の報告に含める。
- Linear id が既知かつ `qa.evidence` が `linear` の場合、qa phase 末尾で Linear issue に
  証跡を添付する (ネイティブ添付 API、private でもインライン表示)。`auto` は integrate で解決する。
- confirm なし。gate: `file_exists: docs/features/*/qa-report.md`。

validate (抜粋):
- scenarios.yml の全 scenario が qa-report.md に PASS/FAIL 行を持ち、run dir に**実在する
  証跡ファイル**を参照している
- `kind: browser` の行は 1 枚以上のスクショ、`kind: cli` の行は transcript を参照している
- setup が宣言どおり実行された (起動失敗を FAIL と混同していない)
- 全 FAIL が修正 + qa-verifier 再検証済み、またはユーザーが明示的に受容
- QA 中に修正 commit がある場合、evidence.md の qa エントリに一覧が記録されている
- Verdict: SKIPPED はユーザー承認の記録 (日時・理由) が qa-report.md にある場合のみ
- evidence.md に qa エントリがある

### integrate (orchestrator 直下 leaf)

現行 build/integrate と同一 (`invoke.skill: /worktrunk`、confirm あり) に加え:

- **証跡公開** — `gh pr create` ルートでは PR コメントに QA 結果テーブル + 証跡を添付
  (D10 の evidence branch 方式)。公開先 URL を evidence.md の integrate エントリに記録。
  wt merge ルートで Linear 添付も済んでいない場合はローカル保持を明示的に警告する。
- validate 追加: 「deferred finding・受容済み FAIL・探索 advisory の一覧をユーザーに
  提示した」「QA 証跡が公開先に添付された、またはローカル保持の警告を報告した」。
- feature-dev / bug-fix にインライン重複定義し、lock test が両者の同一性を assert する
  (D14)。

### diagnose (最小改修)

- `e2e` arg 廃止。rca-scenarios.yml は**常時執筆** (`kind` 付き)。
- confirm 統合: rca / fix-plan は confirm なし、fix-plan-review のみ confirm (診断・計画承認の
  1 点)。
- criteria/ + references/ supplement 体系の全面刷新は本 spec のスコープ外 (follow-up)。

### checkpoint / handover / resume / goal

- checkpoint.yml は無変更。
- goal: 入力解決ルールに 1 行追加 — 「ローカルパスが `requirements.md` を指す →
  読み込んで goal-sheet に凝縮する」。/belt:requirements との接続点。

## QA 証跡仕様

### 生成 (repo 非コミット、D9)

```
.belt/runs/<run-id>/qa/          # 一次証跡。repo にはコミットしない
├── <scenario-id>/
│   ├── 01-<step>.png            # browser: ステップごとのスクショ (連番、判定に必要な
│   │                            #   ステップのみ、目安 scenario あたり ≤5 枚)
│   └── transcript.txt           # cli: 実行コマンド + stdout/stderr 全文
└── exploratory/
    └── <probe>-NN.png|txt
```

`docs/features/<topic>/qa-report.md` (テキスト、repo にコミット):

```markdown
# QA report: <topic>
## Run                    — belt run id (証跡の run dir を特定するキー)
## Scenario results
| scenario | kind | result | evidence |
|----------|------|--------|----------|
| login-ok | browser | PASS | qa/login-ok/01-form.png, 02-home.png |
| init-cli | cli     | PASS | qa/init-cli/transcript.txt |
## Exploratory notes      — プローブと観測の bullet list (advisory / FAIL 昇格を明記)
## Verdict                — PASS / FAIL (一覧) / SKIPPED (ユーザー承認記録必須)
```

- evidence 列は run dir 相対の証跡ファイル名。実在しないファイル参照・証跡なし PASS は
  validate で reject — 「撮ったつもり」の捏造を構造で防ぐ。
- 人間がインライン画像でレビューする一次資料は次項の公開先 (PR コメント / Linear)。
  qa-report.md は repo 内に残る索引 + 判定記録。

### 公開 (auto チェーン + config override、D9/D10)

`belt.toml`:

```toml
[qa]
evidence = "auto"   # "pr" | "linear" | "local" | "auto"
```

- `auto` の解決順:
  1. integrate で PR 作成 → **PR コメント**に結果テーブル + 証跡を添付
  2. Linear id 既知 → **Linear issue にネイティブ添付** (auto では integrate 時に実行。`linear` 明示時のみ qa phase 末尾)
  3. どちらも不成立 → ローカル保持を明示的に警告 (整合: integrate validate)
- PR への画像は orphan branch `qa-evidence` (run ごとサブディレクトリ、append-only、
  prune 可能) に push し raw URL を埋め込む。public repo はインライン表示、private repo
  は GitHub camo の制約でインライン不可のため blob URL リンクに degradation する。
- Linear はネイティブ添付 API で Linear 自身がホストするため private でもインライン表示。
- config キーの解釈規則は qa SKILL.md に置く (SKILL.md 責務: config key 解釈)。

### scenarios.yml schema 拡張 (additive)

1. scenario 単位の `kind: browser | cli` を追加。`kind: cli` の When は実行する実コマンド、
   Then は期待する出力・exit code・生成物を書く。
2. top-level `setup:` を追加 (D11) — `start` (起動コマンド、省略可) / `url` / `teardown`。
   plan ステージで確定し計画承認の対象。qa-verifier は宣言どおり実行するだけで、推測で
   起動コマンドを叩かない。

docs/testing/cli-behavior/ 配下の CLI scenario 群 (別 schema 系統) には触れない。

## サブエージェント再編 (8 → 6)

| Agent | Plugin | 役割 | 由来 |
|-------|--------|------|------|
| explorer | belt-agent | コード調査。プロンプトで `focus: flow / patterns / impact` を指定する統合探索者。intake・design・requirements・docs から dispatch | code-explorer + code-architect + impact-analyzer 統合 |
| implementer | belt-agent | plan.md / fix-plan.md のタスクを TDD で実行。新 plan.md 形式 (Test Strategy 参照) 前提に全面改稿 | feature-implementer 改名・改稿 |
| spec-reviewer | belt | 文書レビュー。対象別観点 (requirements / goal-sheet / design.md / plan.md) を内蔵 | 改稿 |
| code-reviewer | belt | 正当性・セキュリティ・影響 (観点維持、plan.md 参照に更新) | 改稿 |
| quality-reviewer | belt | テスト網羅・AI アンチパターン (同上) | 改稿 |
| qa-verifier | belt | scenario 再生 + 証跡キャプチャ + qa-report 執筆。**コード修正禁止** | 新設 |

削除: `plugins/belt-agent/agents/{code-architect,code-explorer,impact-analyzer,feature-implementer,phase-auditor}.md`
と `plugins/belt-agent/references/audit-protocol.md` (dead wiring 確認済み)。

## review skill の改修

- `/belt:spec-review`: 対象パス引数に加え、出力ファイル名の指定を受ける
  (default `findings-spec.json`、plan phase は `findings-plan.json` を指定)。同一 run 内で
  design レビューと plan レビューの findings が衝突しないため。
- `/belt:code-review`: triage を「ユーザー却下待ち」から「critical/high 自動修正 + deferred
  記録」へ変更 (build 節参照)。`--codex` は両 skill とも現行維持。

## 定型業務 skills (belt plugin、pipeline.yml なし)

対話中心のワークフローのため belt 駆動には載せない (既存のパイプライン適合基準どおり)。

### /belt:requirements `<linear-id | url | free-text>`

1. 入力解決 — goal と同一ルール (Linear / Slack / URL / free-text)。
2. コードベース調査 — explorer を必要に応じ並列 dispatch。コードが答えられる質問は聞かない。
3. バッチ質問 — 人間にしか決められない点のみ、AskUserQuestion 最大 2 ラウンド。
4. `docs/requirements/<YYYY-MM-DD-topic>/requirements.md` を執筆:
   Background / Goals / Functional requirements / Non-functional requirements /
   Acceptance criteria / Out-of-scope / Open decisions。
5. `/belt:spec-review` (対象 requirements.md、出力先 `docs/requirements/<topic>/review/` を
   明示指定 — belt run 外のため) を実行し、finding を反映。

成果物はそのまま `/belt:feature-dev` の入力になる (goal の入力解決ルール参照)。

### /belt:docs `<お題>`

1. お題 (機能・モジュール・テーマ) を解決し、対象コードを調査 (広範囲なら explorer 並列)。
2. 配置先・文書種別 (アーキテクチャ解説 / 使い方ガイド / リファレンス) が曖昧な場合のみ
   1 回バッチ質問。
3. `docs/` 配下に新規作成 or 既存更新。既存 docs の文体・言語・構成規約に従う。
4. 既存文書との相互リンクと index (あれば) を更新する。

## 移行計画

### ファイル増減

| 操作 | 対象 |
|------|------|
| 新設 | `plugins/belt/skills/plan/{SKILL.md,pipeline.yml}`, `plugins/belt/skills/qa/{SKILL.md,pipeline.yml}`, `plugins/belt/skills/requirements/SKILL.md`, `plugins/belt/skills/docs/SKILL.md`, `plugins/belt/agents/qa-verifier.md`, `plugins/belt-agent/agents/{explorer,implementer}.md` |
| 削除 | `plugins/belt/skills/verify/`, belt-agent agents 5 ファイル, `references/audit-protocol.md` |
| 改修 | feature-dev / bug-fix / design / build / diagnose の pipeline.yml + SKILL.md, goal / spec-review / code-review SKILL.md, belt agents 3 ファイル, `references/authoring-principles.md` (evidence 形式に qa/ 証跡規約を追記) |

### lock tests (belt-core/tests、belt-core 本体は無変更)

- `feature_dev_refresh.rs` — 新 shape contract: top-level 6 phases (pipeline 委任 5 +
  integrate leaf)、args = `codex` のみ、展開 8 leaves、`when` leaf なし、regate なし、
  confirm は design/design・plan/plan・checkpoint・integrate の 4 leaf のみ。
- `bug_fix_refresh.rs` — 同様に diagnose + checkpoint + build + qa + integrate の 5 top-level。
- integrate leaf は feature-dev / bug-fix の両 pipeline.yml で**同一定義**であることを assert
  (D14 のインライン重複 drift 対策)。
- `review_skills_refresh.rs` — agent bundle 更新: belt = 3 reviewer + qa-verifier、
  belt-agent = explorer + implementer、削除 5 ファイル + audit-protocol.md の不在 lock。
- skill / agent 削除は同一 commit で tuple + docstring + orphan 参照を更新 (既存パターン)。

### バージョン・ドキュメント

- 両 plugin 0.3.0 → **0.4.0** (plugin.json 手動 bump)。breaking: `--e2e` 廃止、
  `/belt:verify` → `/belt:qa`、agent 5 削除。CHANGELOG に明記。
- README / AGENTS.md 更新 (CLAUDE.md は symlink のため `git add AGENTS.md`)。

## 非スコープ

- diagnose の criteria/ + supplement 体系刷新 (follow-up チケット化)
- belt-core engine の機能追加 (cross-stage regate 等)
- リモート `uses:` / TUI / fmt (従来どおり Future Phases)

## 検証戦略

1. `cargo test -p belt-core` — 改訂 lock tests + 既存 397+ tests green。
2. `belt lint` を全 pipeline.yml (feature-dev / bug-fix / design / plan / build / qa /
   diagnose / checkpoint) に実行し PASS。
3. `belt-agent init` → `status` で feature-dev / bug-fix の展開 leaf 数・confirm 配置を実測。
4. Dogfood — belt repo 自身の小タスクで `/belt:feature-dev` を 1 周し、run dir 配下の
   CLI transcript 証跡と qa-report.md の生成、公開先 fallback (local 警告) の動作を確認。
5. Linear 添付は linear cli / GraphQL API の upload 可否を plan 段階で実測し、不可なら
   コメント + evidence branch URL に degradation する実装へ切り替える (spec 上の想定は
   ネイティブ添付)。

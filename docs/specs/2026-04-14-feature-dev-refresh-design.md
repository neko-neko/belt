---
title: feature-dev パイプラインリフレッシュ
date: 2026-04-14
status: Draft
related_linear: BELT-20, BELT-32
supersedes:
  - docs/specs/2026-04-07-feature-dev-belt-migration.md (part)
depends-on:
  - examples/skills/feature-dev/pipeline.yml
  - examples/skills/feature-dev/SKILL.md
  - examples/criteria/*.md
---

# feature-dev パイプラインリフレッシュ Design

## 1. 背景 / 動機

### 現行 feature-dev の位置づけ

現行 `examples/skills/feature-dev/` は BELT-32 Plan B (2026-04-13, commit `c934b75`) で 19 → 10 phase に collapse 済み。多観点 multi-agent review (spec-review 4 / plan-review 3 / code-review 7 / test-review 3) + conditional phase (doc / smoke / e2e) を核とする generic example。

### リフレッシュの動機

1. **test-first への思想転換**: テストケース設計 (agent-browser E2E シナリオ含む) を writing-plans の前段に置き、実装計画がテストケースを前提にした詳細化を行えるようにする
2. **対話的 grilling の brainstorming 内吸収**: /brainstorming が既に HARD-GATE + section approval + spec self-review の反復サイクルを内蔵しているため、独立 phase としての spec-review 多観点投票は冗長
3. **exploratory + scripted の 2 段構え**: `/monkey-test` (scripted replay) + `/dogfood` (exploratory) で E2E テスト責務を分離。test-review の multi-agent レビューを実地検証に置き換える
4. **依存引数の削減**: 現行 7 args (`--e2e / --smoke / --doc / --ui / --iterations / --swarm / --linear`) から `{e2e, codex}` の 2 args へ簡素化

### 非目標 (Non-Goals)

- 現行 feature-dev の完全置換を目指すが、過渡期の共存は許容 (§17 参照)
- Linear 同期、swarm (TeamCreate) は本 pipeline のスコープ外
- CLI ツール / ライブラリ開発専用 pipeline の作成 (args.e2e を false にした汎用性で対応)
- doc-audit 相当の pipeline 組込み (将来の必要性を認めつつ、本 spec では見送り)

---

## 2. 現行 feature-dev との差分

| 項目 | 現行 (10 phase) | 新 (8 phase) | 変更理由 |
|---|---|---|---|
| design | /brainstorming | /brainstorming (+ supplement) | 据置。supplement で path override + 並列探索注入 |
| spec-review | pipeline: spec-review (4 agents) | **削除** | /brainstorming の反復 grilling で吸収 |
| plan | /writing-plans (内部で test-cases 生成) | /writing-plans (+ supplement) | test-cases 生成責務は Phase 2 に前倒し |
| plan-review | pipeline: implementation-review (3 agents) | **削除** | /grill-me 案は却下、Phase 1 の grilling で吸収 |
| execute | /subagent-driven-development | /subagent-driven-development | 据置 |
| doc-audit | /doc-audit (when: args.doc) | **削除** | 本 spec 版では見送り。将来 regate target 追加の余地あり |
| smoke-test | /smoke-test (when: args.smoke) | **削除** | /monkey-test + /dogfood に責務移行 |
| code-review | pipeline: code-review (7 agents, regate: [execute, smoke-test, doc-audit]) | /code-review (regate: [execute], max_retries: 3) | 多観点は据置、regate target を execute 単独に簡素化 |
| test-review | pipeline: test-review (3 agents, when: args.e2e) | **削除** | /monkey-test + /dogfood に責務移行 |
| - | - | **/test-scenarios (新規, Phase 2)** | test-first 思想の具象化 |
| - | - | **/monkey-test (新規, Phase 6, when: args.e2e)** | scripted E2E replay |
| - | - | **/dogfood (Phase 7, when: args.e2e)** | exploratory E2E |
| integrate | /worktrunk | /worktrunk (+ supplement) | supplement で A (merge) / B (PR) の UI を明示 |

---

## 3. 全体フロー (8 phase)

```
Feature Spec (natural language user request)
    │
    ▼
Phase 1: design ───────────── /brainstorming (+ supplement)        [always, INTERACTIVE]
    │ docs/features/<topic>/design.md
    ▼
Phase 2: test-scenarios ───── /test-scenarios (new)                [always, INTERACTIVE]
    │ docs/features/<topic>/test-strategy.md
    │ docs/features/<topic>/scenarios.yml (when: args.e2e)
    ▼
Phase 3: plan ─────────────── /writing-plans (+ supplement)        [always, AUTONOMOUS]
    │ docs/features/<topic>/plan.md
    ▼
Phase 4: execute ──────────── /subagent-driven-development         [always, AUTONOMOUS+GATE]
    │ code commits
    ▼
Phase 5: code-review ──────── /code-review (max_retries: 3)        [always, INTERACTIVE]
    │ regate: [execute]  →  fail 時 4→5 ループ、3 回で escalation
    │ .belt/runs/*/review/findings.json
    ▼
Phase 6: monkey-test ──────── /monkey-test (new) [when: args.e2e]  [AUTONOMOUS+GATE]
    │ consumes: design, test-strategy, scenarios, plan
    │ docs/features/<topic>/monkey-test-report.md + results.json
    ▼
Phase 7: dogfood ──────────── /dogfood (+ supplement) [when: e2e]  [INTERACTIVE]
    │ consumes: design, test-strategy, scenarios, monkey-test-*, plan
    │ docs/features/<topic>/dogfood-report/
    ▼
Phase 8: integrate ────────── /worktrunk (+ supplement)            [always, INTERACTIVE]
    │ ユーザー選択: (A) wt merge or (B) gh pr create
    ▼
Complete
```

---

## 4. args スキーマ

| Arg | Type | Default | 用途 |
|---|---|---|---|
| `e2e` | bool | false | Phase 6 /monkey-test と Phase 7 /dogfood を有効化。Phase 2 の scenarios.yml 生成もこのフラグ連動 |
| `codex` | bool | false | Phase 5 /code-review に Codex 並列レビューを追加 |

**削除引数** (新フローで該当 phase なし): `smoke`, `doc`, `ui`, `swarm`, `linear`, `iterations`

- `iterations` は /code-review に渡さない (pipeline 層から制御しない。/code-review は内部デフォルトで動作)
- `ui` は /dogfood + /monkey-test が UI 観点を吸収するため不要
- `swarm`, `linear` は pain-driven first-class 原則により、必要になった時点で追加

---

## 5. pipeline.yml 全体像

```yaml
name: feature-dev
version: 1
description: "Quality-gated development pipeline (8 phases)"

args:
  e2e:
    type: bool
    default: false
    description: "Enable E2E testing phases (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in code-review"

phases:
  - id: design
    description: "Generate design document via interactive brainstorming"
    invoke:
      skill: /brainstorming
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Design document with explored context and test perspectives"
    gate:
      - file_exists: "docs/features/*/design.md"
    validate: ./criteria/design.md
    confirm: true
    max_retries: 3

  - id: test-scenarios
    description: "Design comprehensive test cases and agent-browser scenarios"
    invoke:
      skill: /test-scenarios
      args:
        e2e: "args.e2e"
    consumes:
      - design_doc
    produces:
      - name: test_strategy
        path: "docs/features/*/test-strategy.md"
        description: "Human-readable test strategy (ISTQB/ISO 25010)"
      - name: scenarios
        path: "docs/features/*/scenarios.yml"
        description: "Agent-browser replay scenarios (Given/When/Then YAML)"
        when: "args.e2e"
    gate:
      - file_exists: "docs/features/*/test-strategy.md"
    validate: ./criteria/test-scenarios.md
    confirm: true
    max_retries: 3

  - id: plan
    description: "Generate implementation plan from design and test strategy"
    invoke:
      skill: /writing-plans
    consumes:
      - design_doc
      - test_strategy
    produces:
      - name: plan_doc
        path: "docs/features/*/plan.md"
        description: "Task-level implementation plan (TDD)"
    gate:
      - file_exists: "docs/features/*/plan.md"
    validate: ./criteria/plan.md
    confirm: true
    max_retries: 3

  - id: execute
    description: "Execute implementation plan via TDD subagents"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - design_doc
      - plan_doc
    validate: ../../criteria/execute.md
    confirm: true
    max_retries: 3

  - id: code-review
    description: "Multi-perspective code review"
    invoke:
      skill: /code-review
      args:
        codex: "args.codex"
    consumes:
      - design_doc
      - plan_doc
    validate: ../../criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3

  - id: monkey-test
    description: "Replay pre-defined scenarios via agent-browser"
    when: "args.e2e"
    invoke:
      skill: /monkey-test
    consumes:
      - design_doc
      - test_strategy
      - scenarios
      - plan_doc
    produces:
      - name: monkey_test_report
        path: "docs/features/*/monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/features/*/monkey-test-results.json"
    gate:
      - file_exists: "docs/features/*/monkey-test-report.md"
    validate: ./criteria/monkey-test.md
    confirm: true
    max_retries: 3

  - id: dogfood
    description: "Exploratory testing via agent-browser with feature context"
    when: "args.e2e"
    invoke:
      skill: /dogfood
    consumes:
      - design_doc
      - test_strategy
      - scenarios
      - monkey_test_report
      - monkey_test_results
      - plan_doc
    produces:
      - name: dogfood_report
        path: "docs/features/*/dogfood-report/report.md"
    gate:
      - file_exists: "docs/features/*/dogfood-report/report.md"
    validate: ./criteria/dogfood.md
    confirm: true
    max_retries: 3

  - id: integrate
    description: "Integrate changes (user chooses: wt merge or gh pr create)"
    invoke:
      skill: /worktrunk
    consumes:
      - design_doc
      - plan_doc
    validate: ./criteria/integrate.md
    confirm: true
    max_retries: 3
```

### Regate トポロジー

```
code-review  ──regate──→ [execute]
```

- Phase 5 /code-review で findings を fix commit すると、belt-agent は Phase 4 /execute の validate を再評価
- /execute が依然 PASS なら通常遷移、FAIL なら /execute 修正 → /code-review 再実行
- `max_retries: 3` on /code-review により、validate fail 3 回で escalation (pause)

---

## 6. ディレクトリ構造

```
examples/skills/feature-dev/
├── belt.toml
├── pipeline.yml
├── SKILL.md
├── criteria/
│   ├── design.md                # Phase 1 validate
│   ├── test-scenarios.md        # Phase 2 validate
│   ├── plan.md                  # Phase 3 validate
│   ├── monkey-test.md           # Phase 6 validate
│   ├── dogfood.md               # Phase 7 validate
│   └── integrate.md             # Phase 8 validate
└── references/
    ├── brainstorming-supplement.md
    ├── writing-plans-supplement.md
    ├── monkey-test-supplement.md
    ├── dogfood-supplement.md
    ├── worktrunk-supplement.md
    └── path-convention.md
```

**shared canonical** (共用): Phase 4 / 5 は `examples/criteria/execute.md` と `examples/criteria/code-review.md` を参照。既存 feature-dev と同じ。

---

## 7. パス規約 `docs/features/<topic>/`

dotfiles の supplement パターンで skill 改造なしに実現 (https://github.com/neko-neko/dotfiles 参照)。

### 構造

```
docs/features/<YYYY-MM-DD-topic>/
├── design.md                # Phase 1: /brainstorming 出力 (supplement で path override)
├── test-strategy.md         # Phase 2: /test-scenarios 出力
├── scenarios.yml            # Phase 2: /test-scenarios 出力 (when: args.e2e)
├── plan.md                  # Phase 3: /writing-plans 出力 (supplement で path override)
├── monkey-test-report.md    # Phase 6: /monkey-test 出力
├── monkey-test-results.json # Phase 6: /monkey-test 出力
└── dogfood-report/          # Phase 7: /dogfood 出力 (supplement で path override)
    ├── report.md
    ├── screenshots/
    └── videos/
```

### 命名規則

- `<YYYY-MM-DD>`: Phase 1 /brainstorming 起動日
- `<topic>`: Phase 1 /brainstorming で user と合意した kebab-case slug (例: `user-authentication`, `payment-refactor`)
- worktree branch 名 (`wt switch -c <branch>`) と一致させる (例: `feature/2026-04-14-user-authentication`)

詳細は `references/path-convention.md` に記述。

---

## 8. Supplement 設計

dotfiles パターン踏襲。SKILL.md の Phase Detail に `INVOKE 1: Read ./references/X-supplement.md / INVOKE 2: Skill` と明記。

### 8.1 `references/brainstorming-supplement.md`

**目的**: /brainstorming の挙動を feature-dev 用にカスタマイズ。

**内容 (概要)**:
- 出力 path を `docs/features/<YYYY-MM-DD-topic>/design.md` に指定
- clarifying questions 後に **並列探索エージェント** (code-explorer / code-architect / impact-analyzer) を起動
- 既存コードから **暗黙のルール** (バリデーション、条件分岐、ビジネスロジック) を抽出しユーザー確認
- design.md に **固定セクション** を要求: 前提条件 / 影響範囲 / Impact Analysis (Reverse Dependencies / Shared State / Implicit Contracts / Side Effect Risks) / Must-Verify Checklist / テスト観点 (正常系 / 境界値 / 異常値 / 状態遷移を各 1 項目以上)
- 設計書コミット前に `worktrunk:worktrunk` で `wt switch -c <branch>` 実行 + baseline テスト通過確認

dotfiles 既存実装を belt 用に簡略化して流用。

### 8.2 `references/writing-plans-supplement.md`

**目的**: /writing-plans に design + test-strategy の両方を consume させる。

**内容**:
- 入力: `docs/features/<topic>/design.md` + `docs/features/<topic>/test-strategy.md`
- 出力 path を `docs/features/<topic>/plan.md` に上書き
- test-strategy の観点を plan タスクごとの Given/When/Then に展開
- args.e2e が true なら、E2E タスクは scenarios.yml を前提にした実装タスクを含む (scenarios.yml の id を plan タスクで明示参照)

### 8.3 `references/monkey-test-supplement.md` (新規)

**目的**: /monkey-test に scenarios だけでなく design/plan を context 注入。

**内容**:
- 主入力: `docs/features/<topic>/scenarios.yml`
- 副入力 (hint として必読):
  - `design.md`: Given/When/Then の自然言語解釈に曖昧さが出た際、design のバリデーションルールで解決
  - `test-strategy.md`: 各シナリオの category/severity を test-strategy と突合し FAIL 時の重大度判定に活用
  - `plan.md`: 実装済みタスクを把握し「未実装機能のシナリオは SKIP」判定に活用
- 出力:
  - `monkey-test-report.md` (human-readable)
  - `monkey-test-results.json` (machine-readable): `[{ scenario_id, status, duration_ms, error?, screenshots[] }]`
- 完了条件: 全シナリオ実行完了 (PASS/FAIL/SKIP)、results.json が schema valid

### 8.4 `references/dogfood-supplement.md`

**目的**: /dogfood にパス上書き + 前段フェーズ成果物を探索ヒントとして注入。

**内容 (上書き)**:
- 出力 path: `docs/features/<topic>/dogfood-report/` (/dogfood デフォルト `./dogfood-output/` を上書き)
- スコープ: worktree 内で変更された範囲 (`git diff <base>..HEAD`) に集中
- severity フィルタ: critical / high のみ report.md 主要セクション、medium / low は summary

**内容 (context 注入、必読指示)**:
- `design.md`: 前提条件 / 影響範囲 / Impact Analysis の **Side Effect Risks** / **Must-Verify Checklist** を探索 target 化
- `test-strategy.md`: scripted でカバー外の非機能要件 (パフォーマンス / セキュリティ / アクセシビリティ) / 境界値 / 状態遷移を優先探索
- `scenarios.yml`: scripted 済 happy path は重複探索回避、未カバー組み合わせ探索に集中
- `monkey-test-results.json`: 既に発見済みの FAIL 項目は dogfood で「既知の問題 (monkey-test で検出済)」として区別
- `plan.md`: 実装範囲と task 構造を把握、各 task が正しく動作することを dogfood 中に確認

**探索戦略の優先度** (context に基づく):
1. design.md の Must-Verify Checklist 全項目検証 (最優先)
2. Impact Analysis の Side Effect Risks の再現試行
3. test-strategy.md でカバー外とされた非機能要件検証
4. scripted で未カバーな組み合わせ・exotic case 探索
5. UI/UX の表層バグ (タイポ / ミスアライメント / コンソールエラー等)

**report.md 構造** (既知の問題区別):
```
# Dogfood Report: <feature-name>

## Summary
- Issues found: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N (from monkey-test-results.json)
- Must-Verify Checklist: X/Y items verified

## Critical + High Issues (new findings)
## Must-Verify Checklist Verification
## Known Issues Re-encountered
## Medium / Low Issues (summary only)
```

### 8.5 `references/worktrunk-supplement.md`

**目的**: Phase 8 integrate で user に A/B 選択を提示し、選択ロジックを明文化。

**内容**:
- Phase 8 起動時、user に以下を質問:
  ```
  Select integration mode:
  (A) merge: `wt merge` to parent branch + `wt remove` worktree
  (B) PR: `gh pr create` with auto-generated body, keep worktree
  ```
- **(A) 実行**: `wt merge` → pre-merge hook (テスト / ビルド検証自動) → FF merge → `wt remove`
- **(B) 実行**: `gh pr create --title "..." --body "..."` で PR 作成。body テンプレートは以下:
  ```
  ## Summary
  (Phase 1 design.md から要約、主要機能・変更範囲)

  ## Changes
  (Phase 3 plan.md の task 一覧)

  ## Testing
  - Code Review: <findings count, severity distribution>
  - Monkey Test: <PASS/FAIL counts>  # when args.e2e
  - Dogfood: <issues count by severity> # when args.e2e

  ## Verification
  (Must-Verify Checklist from design.md の verification status)
  ```

### 8.6 `references/path-convention.md`

**目的**: `docs/features/<topic>/` 規約の SSOT。他 supplement からリンク参照される。

**内容**:
- `<topic>` slug 命名ルール (kebab-case、英数字+ハイフン、他 feature と衝突回避)
- `<YYYY-MM-DD>` の算出 (Phase 1 /brainstorming 起動日)
- worktree branch 名との対応 (`feature/<YYYY-MM-DD-topic>`)
- `docs/features/<topic>/` 配下の全ファイル一覧と生成責任 phase の対応表

---

## 9. 新規スキル仕様

### 9.1 `/test-scenarios`

**場所**: `~/.claude/skills/test-scenarios/` (global)、ないしは dotfiles 管理 (後日判断)

**frontmatter**:
```yaml
---
name: test-scenarios
description: >-
  Design comprehensive test cases from a feature design document.
  Generates human-readable test strategy (ISTQB/ISO 25010 based) and,
  when e2e mode is enabled, agent-browser-replayable scenarios in Given/When/Then YAML.
user-invocable: true
---
```

**入力**:
- `docs/features/<topic>/design.md` (feature-dev supplement が path を指示)
- args: `e2e` (bool) — scenarios.yml を生成するかのフラグ

**出力**:

1. **`test-strategy.md` (always produce)**: ISTQB テスト設計技法 + ISO 25010 品質特性評価 (人間可読 markdown)
   - 正常系 / 境界値 / 異常値 / 状態遷移の各カテゴリ最低 1 項目ずつ
   - 非機能要件 (パフォーマンス / セキュリティ / アクセシビリティ) の priority matrix
   - design.md の Must-Verify Checklist と 1:1 対応

2. **`scenarios.yml` (when args.e2e)**: agent-browser 実行用シナリオ
   ```yaml
   scenarios:
     - id: login-happy-path
       category: authentication
       severity: critical
       given: "User is not logged in, on /login page"
       when: "User enters valid email and password, clicks 'Submit'"
       then: "User is redirected to /dashboard, sees welcome message"
       preconditions:
         - "Test user exists in database"
       postconditions:
         - "Session cookie set"

     - id: login-invalid-password
       category: authentication
       severity: high
       given: "User is on /login page"
       when: "User enters valid email but wrong password, clicks 'Submit'"
       then: "Error message 'Invalid credentials' shown, remain on /login"
   ```

**思想**: 既存 `/breakdown-test` の ISTQB / ISO 25010 知識を継承。GitHub Issue テンプレ出力は排除し、belt artifact chain 専用に新設。

### 9.2 `/monkey-test`

**場所**: `~/.claude/skills/monkey-test/` (global) or dotfiles 管理

**frontmatter**:
```yaml
---
name: monkey-test
description: >-
  Replay pre-defined Given/When/Then scenarios via agent-browser.
  Designed for scripted regression testing in a feature-dev pipeline context.
  Consumes scenarios.yml and produces a human-readable report plus machine-readable results.
allowed-tools: Bash(agent-browser:*), Bash(npx agent-browser:*)
user-invocable: true
---
```

**入力**:
- `docs/features/<topic>/scenarios.yml` (必須)
- `docs/features/<topic>/design.md` (hint、supplement 経由で Read 指示)
- `docs/features/<topic>/test-strategy.md` (hint)
- `docs/features/<topic>/plan.md` (hint、未実装判定用)

**動作**:
1. scenarios.yml を Parse し、各シナリオの id / given / when / then / severity を取得
2. plan.md を参照し、未実装機能のシナリオは SKIP 判定
3. 各実行対象シナリオについて:
   - agent-browser を起動 (必要なら auth-state 復元)
   - Given → When → Then を LLM が自然言語解釈 (design.md の定義で曖昧さ解消)
   - 各ステップで screenshot、assertion で PASS/FAIL 判定
4. 全シナリオ実行後:
   - `monkey-test-report.md` を書く
   - `monkey-test-results.json` を書く (schema: `[{ scenario_id, status, duration_ms, error?, screenshots[] }]`)

**出力**:
- `monkey-test-report.md` (human-readable)
- `monkey-test-results.json` (machine-readable)

**/dogfood との差**:
- **/monkey-test**: **scripted replay** — scenarios.yml に書かれた通りに実行、検証目的
- **/dogfood**: **exploratory** — シナリオなし、LLM が自由に探索して問題発見

---

## 10. Done-Criteria / Validate

validate ファイルは feature-dev/criteria/ 配下に配置。phase-auditor (audit: required) または lite audit が判定。

### criteria/design.md (Phase 1)
```
audit: lite
- DESIGN-01: design.md に「前提条件」「影響範囲」「Impact Analysis」「Must-Verify Checklist」「テスト観点」セクションが存在
- DESIGN-02: テスト観点は正常系 / 境界値 / 異常値 / 状態遷移を各 1 項目以上
- DESIGN-03: worktree 作成済み (`.belt/runs/*/artifacts/` または git branch)
- DESIGN-04: design.md は worktree 内にコミット済み (uncommitted 変更なし)
```

### criteria/test-scenarios.md (Phase 2)
```
audit: lite
- TEST-01: test-strategy.md が生成されており ISTQB / ISO 25010 観点のセクションが揃う
- TEST-02: design.md の Must-Verify Checklist と 1:1 対応 (missing item なし)
- TEST-03: args.e2e が true の場合、scenarios.yml に最低 3 シナリオ、各に id / given / when / then / severity
- TEST-04: test-strategy の非機能要件セクションが 1 つ以上の具体的観点を含む
```

### criteria/plan.md (Phase 3)
```
audit: lite
- PLAN-01: plan.md に「Goal」「Architecture」「Tech Stack」「Task N」が存在
- PLAN-02: 各 Task が TDD step (failing test → impl → passing test → commit) を含む
- PLAN-03: test-strategy.md の観点が task に 1:1 以上で反映
- PLAN-04: placeholder (TBD / TODO / "appropriate error handling" 等) なし
- PLAN-05: args.e2e true の場合、scenarios.yml の各 id に対応する実装タスクあり
```

### criteria/monkey-test.md (Phase 6)
```
audit: lite
- MONKEY-01: monkey-test-report.md が生成
- MONKEY-02: monkey-test-results.json が schema valid
- MONKEY-03: scenarios.yml の全シナリオが results.json に PASS/FAIL/SKIP で記録
- MONKEY-04: critical/high severity の FAIL がある場合は report.md に明示
```

### criteria/dogfood.md (Phase 7)
```
audit: lite
- DOGFOOD-01: dogfood-report/report.md が生成
- DOGFOOD-02: design.md の Must-Verify Checklist 全項目の verification status が report に記載
- DOGFOOD-03: monkey-test-results.json の FAIL 項目について dogfood での再現/非再現が記載
- DOGFOOD-04: 5 件以上の new issue が well-documented (severity / repro / evidence)、または "no critical issues found" の明示
```

### criteria/integrate.md (Phase 8)
```
audit: lite
- INT-01: user が A (merge) or B (PR) を選択
- INT-02 (A 選択時): wt merge 実行、pre-merge hook 通過、worktree 削除済み
- INT-03 (B 選択時): gh pr create 成功、PR URL が report 出力
- INT-04: 全 phase の produces artifact が parent branch に含まれている (merge 時) or PR に含まれている (PR 時)
```

### shared: ../../criteria/execute.md, code-review.md

現行 `examples/criteria/execute.md` / `code-review.md` を流用 (2026-04-13 `2e560df` で canonical 版化済み)。

---

## 11. SKILL.md 構造 (authoring principle 準拠)

`docs/specs/2026-04-07-skill-md-authoring-principle.md` に準拠。SKILL.md は:

1. Config key 解釈ルール
2. Domain-specific 制約 / Red Flags
3. `references/` ポインタ

Phase Map / protocol / HARD-GATE などは記述しない (belt-agent SKILL.md が SSOT)。

```markdown
---
name: feature-dev
description: >-
  Quality-gated development pipeline (8 phases). Design → test scenarios → plan →
  execute → code review → monkey test (E2E scripted) → dogfood (E2E exploratory) →
  integrate. Web UI testing phases are conditional on --e2e.
user-invocable: true
---

# feature-dev

Belt pipeline for quality-gated development. 8 phases driven by belt-agent.

## Args

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | Enable monkey-test and dogfood phases |
| codex | bool | false | Enable Codex parallel review in code-review |

## Phase-Specific Invocation Rules

### Phase 1: design
- **INVOKE 1**: Read `./references/brainstorming-supplement.md` into context
- **INVOKE 2**: Skill tool `/brainstorming`
- The supplement injects parallel exploration (code-explorer / code-architect / impact-analyzer), implicit rules extraction, and required design sections.

### Phase 2: test-scenarios
- **INVOKE**: Skill tool `/test-scenarios` with `e2e` arg passed through.

### Phase 3: plan
- **INVOKE 1**: Read `./references/writing-plans-supplement.md`
- **INVOKE 2**: Skill tool `/writing-plans`

### Phase 4: execute
- **INVOKE**: Skill tool `/subagent-driven-development`
- Orchestrator must reconstruct tasks into self-contained implementation specs before dispatching `feature-implementer` subagents. Do not forward broad research verbatim.

### Phase 5: code-review
- **INVOKE**: Skill tool `/code-review` with `codex` passed through
- On fix commits, Phase 4 validate is re-verified per belt regate semantics.

### Phase 6: monkey-test (when e2e)
- **INVOKE 1**: Read `./references/monkey-test-supplement.md`
- **INVOKE 2**: Skill tool `/monkey-test`

### Phase 7: dogfood (when e2e)
- **INVOKE 1**: Read `./references/dogfood-supplement.md`
- **INVOKE 2**: Skill tool `/dogfood`

### Phase 8: integrate
- **INVOKE 1**: Read `./references/worktrunk-supplement.md`
- **INVOKE 2**: Prompt user for mode (A: `wt merge` / B: `gh pr create`) and execute accordingly via `/worktrunk`.

## Red Flags

- **Never skip the Phase 1 supplement load**: parallel exploration and the required design sections depend on it.
- **Never pass --iterations to /code-review**: single-pass review by design in this pipeline.
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through `references/*-supplement.md`.
- **Never populate docs/features/<topic>/ paths manually**: they are generated by phases; manual edits break belt's phase-start mtime filter.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules
- `./references/brainstorming-supplement.md` — Phase 1 overrides
- `./references/writing-plans-supplement.md` — Phase 3 overrides
- `./references/monkey-test-supplement.md` — Phase 6 context injection
- `./references/dogfood-supplement.md` — Phase 7 overrides and context injection
- `./references/worktrunk-supplement.md` — Phase 8 A/B choice logic
```

---

## 12. データフロー / Artifact Chain

```
design_doc (P1)
    ├─→ consumed by: test-scenarios (P2), plan (P3), execute (P4),
    │                 code-review (P5), monkey-test (P6), dogfood (P7), integrate (P8)

test_strategy (P2)
    ├─→ consumed by: plan (P3), monkey-test (P6), dogfood (P7)

scenarios (P2, when e2e)
    ├─→ consumed by: monkey-test (P6), dogfood (P7)

plan_doc (P3)
    ├─→ consumed by: execute (P4), code-review (P5), monkey-test (P6),
    │                 dogfood (P7), integrate (P8)

monkey_test_report + monkey_test_results (P6, when e2e)
    └─→ consumed by: dogfood (P7)

dogfood_report (P7, when e2e)
    └─→ (optional: integrate PR body 生成)
```

artifact chain が belt の declared shape で表現され、`belt lint` で静的に依存グラフを検証可能。

---

## 13. Error Handling / Regate

| Phase | max_retries | 失敗時の挙動 |
|---|---|---|
| 1 design | 3 | 3 回失敗で escalation (belt-agent pause) |
| 2 test-scenarios | 3 | 同上 |
| 3 plan | 3 | 同上 |
| 4 execute | 3 | タスク累積失敗で pause |
| 5 code-review | 3 | **regate: [execute]** により 3 回まで 4↔5 ループ、3 回目 validate fail で pause |
| 6 monkey-test | 3 | scenario FAIL があれば Phase 4/5 に戻って修正、3 回で pause |
| 7 dogfood | 3 | critical bug 発見時 pause、ユーザー判断で fix or 延期 |
| 8 integrate | 3 | merge conflict / PR 作成失敗で 3 回リトライ |

escalation 時の処理は `docs/specs/2026-04-07-belt-regate-auto-execution.md` および BELT-28 の on_escalation (pause/skip/abort) に委ねる。

---

## 14. テスト戦略 (pipeline 自身)

### Static validation

- **`belt lint`**: pipeline.yml の static 検証 (duplicate IDs, regate target 存在, artifact refs, uses references, expansion 試行)
- 新 pipeline.yml の追加により既存 lint rules が回帰していないことを CI で確認

### Integration testing

- `cargo test -p belt-core --test integration_*` に新 feature-dev pipeline を読み込むテストを追加
  - init / next / verify / step を各 phase で 1 周させる (conditional phase の skip 動作も含む)
  - regate: [execute] の semantics (execute 修正→code-review 再実行) を test
  - max_retries 3 到達時の escalation JSON shape を test

### End-to-end dogfood

- belt 自身の Linear ticket 1 つを新 feature-dev で完遂
  - 候補: BELT-28 on_escalation 実装 (中規模、web UI なしで args.e2e=false 経路を検証)
  - 候補: 別途 frontend 変更を伴うチケット (args.e2e=true 経路検証)

---

## 15. 未決事項 / 後日判断

本 spec 段階では確定させず、implementation plan で詳細化:

1. **新規スキル格納場所**: `/test-scenarios` / `/monkey-test` を global (`~/.claude/skills/`) に置くか dotfiles で管理するか
2. **supplement の逐語内容**: 本 spec は方針のみ。逐語 markdown は implementation 時に作成
3. **`<topic>` slug 合意フロー**: Phase 1 brainstorming で user に確認するタイミング / 命名 validation ルール
4. **scenarios.yml の selector 指定**: UI 要素の特定方法 (CSS selector / accessibility role / visible text) の推奨順位
5. **/worktrunk PR body テンプレート**: design + findings + reports の要約形式の詳細
6. **既存 `examples/skills/feature-dev/` の扱い**: §17 参照、差し替え / 共存 / rename から選択
7. **doc-audit 相当**: 完全削除で進めるが、将来 Phase 5 の regate target に lightweight `/doc-check` を追加する余地あり

---

## 16. 設計判断の要約

| 判断 | 採用 | 根拠 |
|---|---|---|
| 10 → 8 phase | 採用 | spec-review / plan-review / test-review / doc-audit / smoke-test 削除 |
| multi-agent review 全廃 (code-review 除く) | 採用 | Phase 3 /grill-me 案は却下し Phase 1 内部へ畳込み、test-review 相当を /monkey-test + /dogfood に移行 |
| audit phase 分離なし | 採用 | BELT-32 方針継承。validate file-ref で phase 内解決 |
| 2 新規スキル (/test-scenarios, /monkey-test) | 採用 | pain-driven first-class、既存 skill を壊さない |
| docs/features/<topic>/ 統一 | 採用 | supplement で skill 改造なしに path override (dotfiles パターン) |
| args = {e2e, codex} | 採用 | iterations / swarm / linear / smoke / doc / ui 削除 |
| regate: [execute] max_retries 3 | 採用 | BELT-24 準拠、code-review ループを既存機構で表現 |
| /monkey-test vs /dogfood 役割分離 | 採用 | scripted replay vs exploratory |
| /worktrunk A/B (merge/PR) | 採用 | 現行 integrate phase の思想継承 |
| /dogfood への前段 context 注入 | 採用 | supplement 経由で design/test-strategy/scenarios/monkey-test results/plan を必読指示 |

---

## 17. 破壊的変更 / マイグレーション

### 現行 `examples/skills/feature-dev/` の扱い

**採用方針**: **新規 pipeline で置き換え、同名で差し替え** (atomic cutover)。

理由:
- 現行 feature-dev は "belt の reference example" であり、2 つあると混乱
- BELT-32 Plan B の atomic cutover (`85efb65`) の成功例に倣う
- 旧 pipeline.yml / SKILL.md / criteria / references は git history で参照可能 (論理的失い物なし)

cutover plan:
1. 新 pipeline 一式を feature branch で完成 + test 通過
2. 単一 commit で `examples/skills/feature-dev/` 全体を書換え
3. `docs/specs/2026-04-07-feature-dev-belt-migration.md` に "superseded by 2026-04-14-feature-dev-refresh-design.md" を追記

### /brainstorming / /writing-plans / /dogfood の既存利用への影響

- **なし**。supplement 方式は skill を改造しない。他プロジェクトでの利用は現状通り。

### dotfiles 版 feature-dev との関係

- dotfiles の `claude/skills/feature-dev/` は Claude Code のみで動く実装 (belt-agent 不使用)
- 本 spec は belt-agent 前提の YAML pipeline 版
- 両者は独立。belt 版で dotfiles 版を置換する意図はないが、将来 dotfiles 版を deprecate する可能性あり (user 判断)

---

## 18. 参考資料

### Linear チケット

- **BELT-20**: belt 再設計 epic (7 核心パターン、Verification/Validation 分離)
- **BELT-24**: regate 決定論的自動実行 (regate topology + context neutrality 原則)
- **BELT-28**: on_escalation field (pause/skip/abort) — 本 pipeline の escalation 挙動に影響
- **BELT-32**: Invoker + Artifact first-class (現行 feature-dev の 19→10 collapse、本 pipeline の基盤)

### 関連 spec

- `docs/specs/2026-04-07-feature-dev-belt-migration.md` — 現行 feature-dev の主設計書 (本 spec で一部置換)
- `docs/specs/2026-04-07-belt-regate-auto-execution.md` — regate 実装基盤
- `docs/specs/2026-04-07-skill-md-authoring-principle.md` — SKILL.md authoring principle (本 spec 準拠)
- `docs/specs/2026-04-11-belt-action-data-first-class.md` — BELT-32 Plan A
- `docs/specs/2026-04-11-belt-32-plan-b-examples-migration.md` — BELT-32 Plan B (19→10 collapse)
- `docs/specs/2026-04-14-expander-with-merge-design.md` — Invoker::Pipeline.with の expander 解決

### 参考実装

- **dotfiles feature-dev**: https://github.com/neko-neko/dotfiles/tree/master/claude/skills/feature-dev
  - supplement パターンの originator
  - brainstorming-supplement.md の S1-S4 ステップを本 spec が参考

### 既存 skill ドキュメント

- `~/.claude/plugins/cache/superpowers-marketplace/superpowers/5.0.7/skills/brainstorming/SKILL.md`
- `~/.claude/plugins/cache/superpowers-marketplace/superpowers/5.0.7/skills/writing-plans/SKILL.md`
- `~/.claude/plugins/cache/superpowers-marketplace/superpowers/5.0.7/skills/subagent-driven-development/SKILL.md`
- `~/.agents/skills/dogfood/SKILL.md`
- `~/.dotfiles/claude/skills/code-review/SKILL.md`
- `~/.claude/plugins/marketplaces/worktrunk/skills/worktrunk/SKILL.md`

---

End of design document.

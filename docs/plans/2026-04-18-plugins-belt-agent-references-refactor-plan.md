# Plugins belt-agent/references Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `plugins/belt-agent/references/` を domain-neutral な type-only core に再編する。evidence-catalog を skill-local (feature-dev / bug-fix) に移設し、activity type enum を廃止し、evidence↔phase の参照方向を反転 (IoC) し、narrative / criteria-template / audit-protocol の例示を一般化する。非開発 pipeline (weekly-sync, triage, security-scan 等) が belt-agent references を脇見なしで参照できる状態にする。belt-core / belt-agent binary は touch しない。

**Architecture:** Markdown/YAML-only edits across 4 stages. Stage 1 additive (evidence-schema.md 新設 + skill-local evidence-catalog.md 2 件)、Stage 2 generalization (narrative-convention / criteria-template / audit-protocol)、Stage 3 atomic deletion + phase-auditor.md 更新 (activity_type 削除、evidence-catalog 参照更新 — spec gap を plan で fix)、Stage 4 optional IoC schema 追加 (`uses_evidence`)。最後に thought experiment として weekly-sync pipeline sketch で validation。

**Tech Stack:** Markdown (references / SKILL.md / criteria), YAML (pipeline.yml unchanged), Rust/Cargo (`cargo test --workspace` for regression), git + `/worktrunk` skill for isolation.

**Spec:** `docs/specs/2026-04-18-plugins-belt-agent-references-refactor-design.md`

**Spec Gap Notes (plan で fix する項目):**
- Spec Decision 3 の implication として phase-auditor agent (`plugins/belt-agent/agents/phase-auditor.md`) の activity_type input field と `./references/evidence-catalog.md` 参照を更新する必要があるが、spec 本文に明示タスクなし。本 plan の Task 8 で同時に fix する。spec 完了後、Future Work として phase-auditor spec の整理を follow-up ticket 化。

---

## File Structure

### Files to Create

- `plugins/belt-agent/references/evidence-schema.md` (Stage 1a, Task 3)
- `plugins/belt/skills/feature-dev/references/evidence-catalog.md` (Stage 1b, Task 4)
- `plugins/belt/skills/bug-fix/references/evidence-catalog.md` (Stage 1c, Task 5)
- `docs/plans/examples/2026-04-18-weekly-sync-pipeline-sketch.yml` + sketch fixtures (Task 11, optional validation)

### Files to Modify

- `plugins/belt-agent/references/narrative-convention.md` (Task 6: L3 header + L81+ example)
- `plugins/belt-agent/references/criteria-template.md` (Task 7 + Task 10: examples general + Stage 4 `uses_evidence` field)
- `plugins/belt-agent/references/audit-protocol.md` (Task 8: examples general — verify touch 不要の可能性)
- `plugins/belt-agent/agents/phase-auditor.md` (Task 9: activity_type 削除 + evidence-catalog 参照更新)

### Files to Delete (Stage 3)

- `plugins/belt-agent/references/evidence-catalog.md` (Task 9)

### Files Confirmed Untouched (既存動作保全)

- `plugins/belt-agent/references/_schema.md`
- `plugins/belt/skills/*/SKILL.md` (evidence-catalog 参照なしを grep 確認済み)
- `plugins/belt/skills/*/criteria/*.md` (evidence-catalog 参照なし、narrative-convention 参照は path 変更なし)
- `plugins/belt/skills/*/pipeline.yml`
- `crates/**` (Rust source、test 含めて touch しない想定、Task 2 で要確認)

---

## Task 0: Worktree 作成

**Files:** n/a (環境セットアップ)

- [ ] **Step 1: Worktree 作成**

Run:
```bash
wt create plugins-refs-refactor
```

Expected: 新 worktree 作成、cd 後の pwd が `.worktrees/plugins-refs-refactor` 等の worktree path を示す。

- [ ] **Step 2: Worktree から作業開始を確認**

Run:
```bash
git worktree list
git status
```

Expected: worktree が list に現れる。branch が新規 feature branch (例: `refactor/plugins-refs-refactor`) に切り替わっている。working tree clean。

---

## Task 1: Stage 0 事前調査 — lock test と grep baseline

**Files:** n/a (read-only 調査)

- [ ] **Step 1: 現行 workspace test の baseline pass 確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 test pass (commit 前の baseline を確認)。失敗があれば本 plan 着手前に修正。

- [ ] **Step 2: evidence-catalog への参照 grep**

Run:
```bash
grep -rn "evidence-catalog" plugins/ crates/ | grep -v "^docs/"
```

Expected output (この 2 行程度のみが参照元):
```
plugins/belt-agent/references/evidence-catalog.md:...
plugins/belt-agent/agents/phase-auditor.md:43:3. Read `./references/evidence-catalog.md`
```

この 2 箇所を Stage 3 の参照切替で処理する。新規参照元が発見されたら plan 更新。

- [ ] **Step 3: activity_type 参照 grep**

Run:
```bash
grep -rn "activity_type\|activity-type" plugins/
```

Expected:
```
plugins/belt-agent/agents/phase-auditor.md:29:- **activity_type**: One of: ...
plugins/belt-agent/agents/phase-auditor.md:64:Merge Universal Criteria... matching activity_type...
plugins/belt-agent/agents/phase-auditor.md:67:...when the activity is review-fix...
```

`phase-auditor.md` 3 箇所のみ確認。Task 9 で atomic 修正。

- [ ] **Step 4: `"feature-dev and bug-fix"` literal の grep**

Run:
```bash
grep -rn "feature-dev and bug-fix" plugins/
```

Expected:
```
plugins/belt-agent/references/narrative-convention.md:3:...feature-dev and bug-fix. belt does not parse note content...
```

narrative-convention.md L3 のみ。Task 6 で edit。

- [ ] **Step 5: E-DOC-* の参照 grep (doc-audit 対応方針確定)**

Run:
```bash
grep -rn "E-DOC-REPORT\|E-DOC-DIFF\|E-DOC-SCRIPT\|E-DOC-CHECK\|E-DOC-EXPLORATION\|E-DOC-FINDINGS\|E-DOC-VERIFY" plugins/ crates/
```

Expected:
```
plugins/belt-agent/references/evidence-catalog.md:...
```

`evidence-catalog.md` 内部のみ。Stage 3 で純 drop 決定 (外部参照なし)。

- [ ] **Step 6: lock test が evidence-catalog / references のどれかを lock しているか確認**

Run:
```bash
grep -rn "evidence-catalog\|criteria-template\|audit-protocol\|narrative-convention\|evidence-schema" crates/
```

Expected: 0 matches (既に Stage 0 で 0 matches 確認済みだが再確認)。

もし matches があれば Stage 3 の commit で test 更新を同時実行。なければ workspace test は本 plan 中ずっと無影響。

- [ ] **Step 7: commit 不要 (調査のみ)、結果を plan 内に記録**

調査結果の記録 (コメントとして)、commit 作成しない。

---

## Task 2: `plugins/belt-agent/references/evidence-schema.md` 新設 (Stage 1a)

**Files:**
- Create: `plugins/belt-agent/references/evidence-schema.md`

- [ ] **Step 1: ファイル新設**

Create `plugins/belt-agent/references/evidence-schema.md` with this exact content:

````markdown
---
name: evidence-schema
description: >-
  Schema for evidence collection and verification. Domain-neutral
  type-only core. Concrete evidence items are defined in each skill's
  own evidence-catalog.md.
---

# Evidence Schema

Evidence collection と verification の型・protocol 定義。具体の
evidence-id は各 skill 配下の `./references/evidence-catalog.md` で定義する。

## 2-Layer Model

- **Claimed (Layer 1)**: Executor が collect/store する "what happened" 記録
- **Verified (Layer 2)**: Audit Agent の独立 check による "really holds" 検証

Layer 1 は必ず phase 実行中に生成する。Layer 2 は required_capabilities が
環境で満たされる場合のみ実施、満たされない場合は Layer 1 evidence のみで
annotation 付きの audit を実施。

## Applicability Condition 記法

Evidence の applicability は observable fact (file existence, keyword
occurrence 等) ベースの述語で判定する:

- `condition: always` — 常に該当
- `condition: require_all: [<predicate>, ...]` — 全て satisfy 時のみ該当
- `condition: require_any: [<predicate>, ...]` — いずれか satisfy 時該当

述語は glob pattern / grep pattern / spec 本文中の keyword 出現判定等。
decidable で独立再現可能なものに限定する。

## if_unavailable Policy (3 種)

Evidence の required_capabilities が満たされない場合の挙動:

| Policy | 動作 |
|---|---|
| `skip_with_warning` | Evidence を除外、verdict に影響なし (警告のみ) |
| `manual_fallback` | PAUSE して user が収集する。user 提供後に再開 |
| `block` | 収集不能なら blocker FAIL、phase を通さない |

## Evidence Declaration Structure

各 evidence-id は skill-local な `evidence-catalog.md` で以下フィールドを持つ:

| Field | Required | 説明 |
|---|---|---|
| `id` | Yes | 一意識別子 (例: `E-TEST`, `E-LINT`) |
| `description` | Yes | 1 行説明 |
| `claimed` | Yes | Layer 1 記録先 path (template 含む) |
| `verified` | Yes | Layer 2 検証手順 (独立に実行可能) |
| `required_capabilities` | Yes | Layer 2 実行に必要な capability (例: `[bash]`, `[browser-automation]`) |
| `condition` | Yes | applicability 判定 (上記記法) |
| `if_unavailable` | Yes | Policy 選択 |

## Phase Reference (Inversion of Control)

各 phase の `criteria/<phase>.md` が `uses_evidence: [E-XXX]` で evidence を
**pick する**。Evidence 側から phase を指定する逆方向 (`applies_to: [...]`) は
採用しない。Activity type enum は存在しない。

```markdown
### <ID>: <criterion title>
- severity, verify_type, verification, pass_condition, fail_diagnosis_hint
- uses_evidence: [E-TEST, E-LINT]   (optional, skill-local evidence-catalog 参照)
- depends_on_artifacts: [path]       (optional, path 直接参照)
- forward_check
```

`uses_evidence` は optional field。既存 `depends_on_artifacts` との並存可能。

## 所在

- Schema (本ファイル): `plugins/belt-agent/references/evidence-schema.md`
- Concrete catalogs: `plugins/belt/skills/<skill>/references/evidence-catalog.md`
- Phase 側 pick: `plugins/belt/skills/<skill>/criteria/<phase>.md` の `uses_evidence:` field
````

- [ ] **Step 2: ファイル存在を確認**

Run:
```bash
ls -la plugins/belt-agent/references/evidence-schema.md
wc -l plugins/belt-agent/references/evidence-schema.md
```

Expected: ファイル存在、行数 50-70 行程度。

- [ ] **Step 3: 既存 test が壊れていないことを確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 test pass (追加のみなので無影響)。

- [ ] **Step 4: Commit**

Run:
```bash
git add plugins/belt-agent/references/evidence-schema.md
git commit -m "docs(plugins/refs): add evidence-schema.md as type-only core

Declares the schema for evidence declaration (2-layer model,
applicability condition grammar, if_unavailable policies,
declaration structure) and the Inversion of Control pattern
where phase criteria pick evidence via uses_evidence.
No activity type enum; concrete catalogs live under
plugins/belt/skills/<skill>/references/evidence-catalog.md."
```

---

## Task 3: `plugins/belt/skills/feature-dev/references/evidence-catalog.md` 新設 (Stage 1b)

**Files:**
- Create: `plugins/belt/skills/feature-dev/references/evidence-catalog.md`

feature-dev の phase (design, test-scenarios, spec-review, plan, pre-execute-handover, execute, code-review, monkey-test, dogfood, integrate) で使用する evidence 13 件を現 `plugins/belt-agent/references/evidence-catalog.md` から copy する。**`applies_to` フィールドは copy 時に削除** (Decision 3 per spec)。

- [ ] **Step 1: Directory 作成 + ファイル新設**

Create `plugins/belt/skills/feature-dev/references/evidence-catalog.md` with:

````markdown
---
name: evidence-catalog
description: >-
  Concrete evidence catalog for feature-dev pipeline. Conforms to
  plugins/belt-agent/references/evidence-schema.md. Evidence is picked
  by each phase's criteria/<phase>.md via uses_evidence (IoC).
---

# Evidence Catalog (feature-dev)

Concrete evidence items available to feature-dev pipeline phases. Each
phase's `criteria/<phase>.md` declares which evidence it `uses_evidence`.

Schema: [`plugins/belt-agent/references/evidence-schema.md`](../../../../belt-agent/references/evidence-schema.md)

## Evidence Layers

- **Claimed (Layer 1)**: Files the Executor collects and stores.
- **Verified (Layer 2)**: Independent checks performed by the Audit Agent.

## Universal Evidence

Universal evidence is `condition: always` and assumed collectable. When
collection is impossible (no test framework for E-TEST, no build system for
E-BUILD), the Audit Agent treats it as a blocker FAIL.

### E-TEST: Test execution log
- **condition**: always
- **claimed**: `artifacts/test-results/phase-{N}-test.log`
- **verified**: Re-run the test command independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Redirect the test command's stdout/stderr to the file.

### E-BUILD: Build log
- **condition**: always
- **claimed**: `artifacts/build/phase-{N}-build.log`
- **verified**: Re-run the build command independently and confirm exit code 0.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Redirect the build command's stdout/stderr to the file.

### E-LINT: Lint / type-check log
- **condition**: always
- **claimed**: `artifacts/lint/phase-{N}-lint.log`
- **verified**: Re-run the linter independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Redirect the linter's stdout/stderr to the file.

### E-REVIEW: Review results
- **condition**: always
- **claimed**: `artifacts/reviews/phase-{N}-review.json`
- **verified**: N/A (re-running a review is impractical).
- **required_capabilities**: []
- **if_unavailable**: block
- **collection**: Aggregate review agent output into JSON and save.

### E-DIFF: git diff snapshot
- **condition**: always
- **claimed**: `artifacts/diff/phase-{N}.diff`
- **verified**: Re-obtain `git diff` independently and confirm it matches.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: `git diff > artifacts/diff/phase-{N}.diff`

### E-TRACE: Traceability matrix
- **condition**: always
- **claimed**: `artifacts/traceability/phase-{N}-trace.md`
- **verified**: Independently verify that spec requirements map to implementation files.
- **required_capabilities**: [bash]
- **if_unavailable**: block
- **collection**: Generate a mapping table between the spec's requirement list and implementation files.

## Conditional Evidence

### E-SCREENSHOT: Screen screenshots
- **condition**:
  - require_all:
    - `glob("**/*.{html,jsx,tsx,vue,svelte}")` returns 1 or more matches
    - The spec mentions any of "screen", "page", "UI", or "component"
- **claimed**: `artifacts/smoke-test/screenshots/{screen}_{state}.png`
- **verified**: Access the same URL in a browser and confirm the page renders.
- **required_capabilities**: [browser-automation]
- **variants**: [desktop, mobile]
- **if_unavailable**: skip_with_warning
- **collection**: Capture screenshots using a browser-automation tool.

### E-SCREENSHOT-MOBILE: Mobile screenshots
- **condition**:
  - require_all:
    - E-SCREENSHOT is enabled
    - The spec mentions "responsive" or "mobile"
- **claimed**: `artifacts/smoke-test/screenshots/{screen}_{state}_mobile.png`
- **verified**: Access with a mobile viewport (≤428px wide) and confirm rendering.
- **required_capabilities**: [browser-automation]
- **if_unavailable**: skip_with_warning
- **collection**: Capture screenshots using a mobile viewport.

### E-API-LOG: API response log
- **condition**:
  - `grep -r "router\|app\.\(get\|post\|put\|delete\)\|@app\.route\|@router" **/*.{ts,js,py,go,rb}` returns 1 or more matches
- **claimed**: `artifacts/api/phase-{N}-api.log`
- **verified**: Send an HTTP request to the endpoint and confirm the response.
- **required_capabilities**: [bash]
- **if_unavailable**: skip_with_warning
- **collection**: Hit the endpoint with curl or httpie and save the result to the log.

### E-MIGRATION: DB migration log
- **condition**:
  - `glob("**/migrations/**/*")` or `glob("**/migrate/**/*")` returns 1 or more matches
- **claimed**: `artifacts/migration/phase-{N}-migration.log`
- **verified**: Connect to the DB and confirm that the migration's target tables/columns exist.
- **required_capabilities**: [database-access]
- **if_unavailable**: manual_fallback
- **collection**: Save the migration command's output to the log.

### E-PERF: Performance metrics
- **condition**:
  - The spec mentions any of "performance", "latency", or "throughput"
- **claimed**: `artifacts/perf/phase-{N}-perf.log`
- **verified**: Re-run the load test independently and compare results.
- **required_capabilities**: [bash]
- **if_unavailable**: skip_with_warning
- **collection**: Save the load-testing tool's output to the log.

### E-CONSOLE: Browser console log
- **condition**:
  - E-SCREENSHOT is enabled
- **claimed**: `artifacts/smoke-test/console.log`
- **verified**: Capture console output during page access and confirm no error-level entries.
- **required_capabilities**: [browser-automation]
- **if_unavailable**: skip_with_warning
- **collection**: Capture console logs using a browser-automation tool.

### E-DEFERRED-IMPACT: Deferred impact findings — actual harm verification
- **condition**:
  - require_all:
    - The review result (`artifacts/reviews/phase-{N}-review.json`) contains 1 or more findings with category: code-impact and user_decision: deferred
- **claimed**: `artifacts/reviews/phase-{N}-deferred-impact-verification.md`
- **verified**: Actually exercise the consumer named by each deferred finding and confirm no inconsistency.
- **required_capabilities**: [bash, browser-automation]
- **if_unavailable**: manual_fallback
- **collection**: For each deferred impact finding, record (1) a summary of the finding, (2) the result of obtaining the same metric from the paired consumer, and (3) the consistency verdict (match / mismatch).
````

- [ ] **Step 2: ファイル存在と行数確認**

Run:
```bash
ls -la plugins/belt/skills/feature-dev/references/evidence-catalog.md
wc -l plugins/belt/skills/feature-dev/references/evidence-catalog.md
```

Expected: 140-170 行程度。

- [ ] **Step 3: `applies_to` が残っていないことを確認**

Run:
```bash
grep -n "applies_to" plugins/belt/skills/feature-dev/references/evidence-catalog.md
```

Expected: 0 matches (Decision 3 per spec)。

- [ ] **Step 4: workspace test 通過確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 test pass (追加のみ)。

- [ ] **Step 5: Commit**

```bash
git add plugins/belt/skills/feature-dev/references/evidence-catalog.md
git commit -m "docs(plugins/feature-dev): add skill-local evidence-catalog.md

Migrates universal and conditional evidence from plugins/belt-agent/
references/evidence-catalog.md to feature-dev's local references layer.
Drops applies_to: [activity_type] fields per Decision 3 (activity type
enum廃止); evidence is now pickable via phase criteria's uses_evidence
(IoC). Original belt-agent-level catalog stays in place until Stage 3
(Task 9) for rollback safety."
```

---

## Task 4: `plugins/belt/skills/bug-fix/references/evidence-catalog.md` 新設 (Stage 1c)

**Files:**
- Create: `plugins/belt/skills/bug-fix/references/evidence-catalog.md`

bug-fix の phase (rca, fix-plan, fix-plan-review, execute, code-review, monkey-test, dogfood, integrate) で使う evidence 13 件を同じ抽出で作成。feature-dev との重複あり (spec Decision 2: skill-local fill-in の論理的帰結、将来 shared 層への抽出を Future Work として spec に記載済み)。

- [ ] **Step 1: ファイル新設**

Create `plugins/belt/skills/bug-fix/references/evidence-catalog.md` with **identical content to feature-dev's evidence-catalog.md** except the opening frontmatter and first line:

````markdown
---
name: evidence-catalog
description: >-
  Concrete evidence catalog for bug-fix pipeline. Conforms to
  plugins/belt-agent/references/evidence-schema.md. Evidence is picked
  by each phase's criteria/<phase>.md via uses_evidence (IoC).
---

# Evidence Catalog (bug-fix)

Concrete evidence items available to bug-fix pipeline phases. Each
phase's `criteria/<phase>.md` declares which evidence it `uses_evidence`.

Schema: [`plugins/belt-agent/references/evidence-schema.md`](../../../../belt-agent/references/evidence-schema.md)

## Evidence Layers

(同 feature-dev)

## Universal Evidence

(E-TEST, E-BUILD, E-LINT, E-REVIEW, E-DIFF, E-TRACE は feature-dev と同一内容)

## Conditional Evidence

(E-SCREENSHOT, E-SCREENSHOT-MOBILE, E-API-LOG, E-MIGRATION, E-PERF, E-CONSOLE, E-DEFERRED-IMPACT は feature-dev と同一内容)
````

**Implementation note:** 先に Task 3 で作成した `plugins/belt/skills/feature-dev/references/evidence-catalog.md` を完全 copy して以下 2 点のみ書き換える:
1. Frontmatter `description` の `feature-dev pipeline` → `bug-fix pipeline`
2. L1 見出し `# Evidence Catalog (feature-dev)` → `# Evidence Catalog (bug-fix)`
3. L3 の `feature-dev pipeline phases` → `bug-fix pipeline phases`

Run:
```bash
cp plugins/belt/skills/feature-dev/references/evidence-catalog.md \
   plugins/belt/skills/bug-fix/references/evidence-catalog.md
```

次に Edit で上記 3 点を差し替え:

```bash
# sed 等ではなく Edit tool で feature-dev → bug-fix 3 箇所を書き換える
```

- [ ] **Step 2: 書き換え後の 3 箇所を検証**

Run:
```bash
head -15 plugins/belt/skills/bug-fix/references/evidence-catalog.md
```

Expected:
```
---
name: evidence-catalog
description: >-
  Concrete evidence catalog for bug-fix pipeline. Conforms to
  plugins/belt-agent/references/evidence-schema.md. Evidence is picked
  by each phase's criteria/<phase>.md via uses_evidence (IoC).
---

# Evidence Catalog (bug-fix)

Concrete evidence items available to bug-fix pipeline phases. ...
```

- [ ] **Step 3: `feature-dev` という文字列が残っていないことを確認**

Run:
```bash
grep -n "feature-dev" plugins/belt/skills/bug-fix/references/evidence-catalog.md
```

Expected: 0 matches.

- [ ] **Step 4: `applies_to` が残っていないことを確認**

Run:
```bash
grep -n "applies_to" plugins/belt/skills/bug-fix/references/evidence-catalog.md
```

Expected: 0 matches.

- [ ] **Step 5: workspace test 通過確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 pass。

- [ ] **Step 6: Commit**

```bash
git add plugins/belt/skills/bug-fix/references/evidence-catalog.md
git commit -m "docs(plugins/bug-fix): add skill-local evidence-catalog.md

Parallel to feature-dev's skill-local evidence-catalog (Task 3), with
identical evidence content. Duplication across feature-dev and bug-fix
is tracked as Future Work in the spec (shared layer extraction if pain
emerges across more skills)."
```

---

## Task 5: 現 evidence-catalog.md の doc-audit section 削除 (Stage 1 末尾 cleanup、optional)

**Files:**
- Modify: `plugins/belt-agent/references/evidence-catalog.md` (L157-213 doc-audit section 削除)

**Rationale:** Task 1 Step 5 の grep 結果で E-DOC-* は belt plugin 外部から参照なしと確認済み。Stage 3 (Task 9) で evidence-catalog.md 全体を削除するため、本 Task は **skip 可能** だが、Stage 2 の一般化作業を clean な evidence-catalog.md に対して行えるようにするため、先に doc-audit section を落としておく。

- [ ] **Step 1: doc-audit section 削除判断**

Option A: 本 Task を skip し Stage 3 (Task 9) で evidence-catalog.md ごと削除。
Option B: 本 Task で doc-audit section のみ先に削除。

**推奨**: Option A (skip)。理由: Stage 3 でどうせ全削除、先行削除は余計な commit を生む。本 plan では Task 5 を **skip してよい** とする。

- [ ] **Step 2: skip 判定記録**

Task 5 は no-op (skip)。Task 6 へ進む。

---

## Task 6: `narrative-convention.md` 一般化 (Stage 2a)

**Files:**
- Modify: `plugins/belt-agent/references/narrative-convention.md`

現状 L3 に `"feature-dev and bug-fix"` 明言、L79-105 に feature-dev design phase example がある。両方 domain-neutral に書き換える。4 sections (Decisions / Concerns / Directives / Observations) の構造 + per-section guidance は **維持** (Decision 4)。

- [ ] **Step 1: L3 header の一般化**

Edit `plugins/belt-agent/references/narrative-convention.md`:

Change L3:
```
Convention for phase-scoped narrative notes produced by narrative-producing phases in `feature-dev` and `bug-fix`. belt does not parse note content, so this convention is owned by the SKILL layer.
```

To:
```
Convention for phase-scoped narrative notes produced by narrative-producing phases across any belt pipeline (development, data-sync, audit, triage, or other domains). belt does not parse note content, so this convention is owned by the SKILL layer.
```

- [ ] **Step 2: Example section (L79-105) の一般化**

既存 L79-105 の "Example: feature-dev design phase" block を以下に差し替え:

From (既存):
```markdown
## Example: feature-dev design phase

```markdown
---
phase: design
run_id: 01947abc-1234-7890-def0-123456789abc
---

## Decisions

- Reuse the existing belt-core narrative mechanism (2026-04-14 spec) for the context-reset capability; do not add new code to belt-core.
- Produce narrative notes only for six phases (lightweight phases excluded, per user agreement).

## Concerns

- `/clear` depends on manual user action. Without documenting "when to reset" in SKILL.md, the notes may never be consulted.

## Directives

- plan phase: keep implementation tasks at a granularity of 30 minutes or less.
- execute phase: do not quote the narrative's Decisions into commit messages (it becomes noise).

## Observations

- `narrative-convention.md` sits alongside existing references under `plugins/belt-agent/references/`.
- Criteria have been made per-plugin during the plugin migration (parity test detects drift).
```
```

To (新):
```markdown
## Example (generic phase)

```markdown
---
phase: <phase_id>
run_id: 01947abc-1234-7890-def0-123456789abc
---

## Decisions

- <Key choice made in this phase with rationale. Prefer statements
  that answer "why this choice, not the alternatives?"—future
  phases will ask.>

## Concerns

- <Unresolved risk or unverified assumption that downstream phases
  must watch. Prefer concrete leads over generic worries.>

## Directives

- <Constraint or precondition the next phase must honor. Place
  narrow, actionable rules, not broad philosophies.>

## Observations

- <Factual finding from exploration that does not fit a domain
  artifact but matters for future investigation or audit.>
```

The example above is intentionally domain-neutral; the same four sections
fit development (design, plan, execute, code-review), data-sync (scan,
analyze, approve, sync), audit (rca, fix-plan, verify), and other
workflows uniformly. See each skill's `SKILL.md` for which phases produce
narrative notes.
```

- [ ] **Step 3: `"feature-dev and bug-fix"` の grep で 0 matches 確認**

Run:
```bash
grep -n "feature-dev and bug-fix" plugins/belt-agent/references/narrative-convention.md
```

Expected: 0 matches.

- [ ] **Step 4: domain-neutral 化 grep check (spec Success Criteria 2)**

Run:
```bash
grep -E "(feature-dev|bug-fix|implementation phase|investigation phase)" plugins/belt-agent/references/narrative-convention.md
```

Expected: 0 matches (narrative-convention.md が domain-neutral 化された)。

- [ ] **Step 5: workspace test 通過確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 pass。

- [ ] **Step 6: Commit**

```bash
git add plugins/belt-agent/references/narrative-convention.md
git commit -m "docs(plugins/refs): generalize narrative-convention examples

Remove the explicit 'feature-dev and bug-fix' reference from the header
paragraph and replace the feature-dev design phase example with a
domain-neutral generic phase template. The four sections (Decisions /
Concerns / Directives / Observations) remain canonical per spec
Decision 4 (Option B-1), with the universality hypothesis supported by
the weekly-sync pilot analysis."
```

---

## Task 7: `criteria-template.md` 一般化 (Stage 2b)

**Files:**
- Modify: `plugins/belt-agent/references/criteria-template.md`

現 criteria-template.md (58 行) は構造自体が domain-neutral だが、"Human Review: 3-Point Scan" の expected time や ID Convention の例が若干 dev 寄り。主に example 部分を軽く見直す。

- [ ] **Step 1: 現状の criteria-template.md を Read**

Run: Read `plugins/belt-agent/references/criteria-template.md`

内容確認: 既に大部分は汎用。Heavy edit 不要、必要に応じて minor の文言調整。

- [ ] **Step 2: Example フィールド `fail_diagnosis_hint` の例を dev 依存からの脱却**

現状 (該当箇所不特定、Step 1 で確認後):
- "On FAIL, always state what to investigate to resolve the failure." は既に domain-neutral

**実際の edit**: 本 Task は Read 結果を見て判断。多くの場合は touch 不要。もし edit 必要なら domain-neutral に書き換え。

- [ ] **Step 3: `"feature-dev"` / `"bug-fix"` / `"implementation"` 等の dev 用語 grep**

Run:
```bash
grep -E "(feature-dev|bug-fix|implementation phase|investigation phase)" plugins/belt-agent/references/criteria-template.md
```

Expected: 0 matches (既に 0、念のため確認)。

- [ ] **Step 4: (Edit 必要時のみ) Commit**

Edit が発生した場合のみ:
```bash
git add plugins/belt-agent/references/criteria-template.md
git commit -m "docs(plugins/refs): generalize criteria-template examples

Minor wording adjustments to remove any remaining domain-specific
phrasing. Template structure and rules unchanged."
```

Edit 不要だった場合は no-op、Task 8 へ。

---

## Task 8: `audit-protocol.md` 一般化 (Stage 2c)

**Files:**
- Modify: `plugins/belt-agent/references/audit-protocol.md`

現 audit-protocol.md (107 行) は `"work phase"` 等の汎用用語で既に neutral。具体例の修正があれば軽微。

- [ ] **Step 1: `"feature-dev"` / `"bug-fix"` / `"implementation"` 等の dev 用語 grep**

Run:
```bash
grep -E "(feature-dev|bug-fix|implementation phase|investigation phase)" plugins/belt-agent/references/audit-protocol.md
```

Expected: 0 matches (既に 0、念のため確認)。

- [ ] **Step 2: Edit 不要なら no-op**

多くの場合 audit-protocol.md は touch 不要。

- [ ] **Step 3: Stage 2 全体の domain-neutral 化検証 (Success Criteria 2)**

Run:
```bash
grep -rE "(feature-dev|bug-fix|implementation phase|investigation phase)" plugins/belt-agent/references/
```

Expected: 0 matches。1 件でも残っていれば当該 file を追加 edit。

- [ ] **Step 4: Edit 発生時のみ Commit**

```bash
git add plugins/belt-agent/references/audit-protocol.md
git commit -m "docs(plugins/refs): generalize audit-protocol examples

Verified already domain-neutral (work phase wording preserved).
Stage 2 completion: plugins/belt-agent/references/ contains no
development-workflow-specific terms."
```

Edit 0 件だった場合は no-op commit 不要。

---

## Task 9: Stage 3 atomic — 現 evidence-catalog.md 削除 + phase-auditor.md 更新

**Files:**
- Delete: `plugins/belt-agent/references/evidence-catalog.md`
- Modify: `plugins/belt-agent/agents/phase-auditor.md`

このタスクは **atomic commit** で実施する。evidence-catalog.md 削除と phase-auditor.md の activity_type / evidence-catalog 参照更新を同時に行うことで、中間状態 (evidence-catalog.md がなくなったが phase-auditor.md が古い参照を持つ) を避ける。

### 9.1 phase-auditor.md の activity_type 削除 (Decision 3 per spec)

- [ ] **Step 1: L29 Input Protocol の activity_type 行を削除**

Edit `plugins/belt-agent/agents/phase-auditor.md`:

From (L26-34 周辺):
```markdown
### Required
- **criteria**: Path to a done-criteria file, or inline criteria text
- **artifacts**: Paths to deliverables to verify
- **activity_type**: One of: implementation, smoke-test, review-fix, test-fix, integration

### Optional
- **evidence_plan_path**: Path to Evidence Plan (if absent and design_doc provided, generate one)
```

To:
```markdown
### Required
- **criteria**: Path to a done-criteria file, or inline criteria text
- **artifacts**: Paths to deliverables to verify

### Optional
- **evidence_catalog_path**: Path to the skill-local `evidence-catalog.md` (if absent, skip Evidence Plan generation)
- **evidence_plan_path**: Path to Evidence Plan (if absent and both evidence_catalog_path and design_doc are provided, generate one)
```

**設計判断 note**: `activity_type` を削除した代わりに `evidence_catalog_path` を optional input として追加。orchestrator が skill dispatch 時に自 skill の `./references/evidence-catalog.md` を path 指定する設計。phase-auditor 自身は skill context を知らず、evidence の集合は input で受け取る。

- [ ] **Step 2: L43 (Evidence Plan Generation Step 3) の evidence-catalog 参照更新**

Edit `plugins/belt-agent/agents/phase-auditor.md`:

From (L39-46 周辺):
```markdown
## Evidence Plan Generation

On first invocation, if no Evidence Plan exists:

1. Check if `evidence_plan_path` points to an existing file
2. If not, read the design doc and project files (package.json, Cargo.toml, etc.)
3. Read `./references/evidence-catalog.md`
4. Evaluate each catalog entry's `condition` using Glob/Grep
```

To:
```markdown
## Evidence Plan Generation

On first invocation, if no Evidence Plan exists AND `evidence_catalog_path` is provided:

1. Check if `evidence_plan_path` points to an existing file
2. If not, read the design doc and project files (package.json, Cargo.toml, etc.)
3. Read the evidence catalog at `evidence_catalog_path` (orchestrator-supplied, skill-local)
4. Evaluate each catalog entry's `condition` using Glob/Grep
```

If `evidence_catalog_path` is absent, skip Evidence Plan generation entirely — the auditor proceeds with inline criteria only.

- [ ] **Step 3: L64 "Step 2: Compose Criteria" の activity_type 除去**

Edit `plugins/belt-agent/agents/phase-auditor.md`:

From (L63-64):
```markdown
### Step 2: Compose Criteria
Merge Universal Criteria (from file) + Evidence-derived criteria (from Evidence Plan for matching activity_type). Evidence-derived criteria are all severity: blocker, verify_type: automated.
```

To:
```markdown
### Step 2: Compose Criteria
Merge Universal Criteria (from file) + Evidence-derived criteria (from Evidence Plan, when generated). Evidence-derived criteria are all severity: blocker, verify_type: automated. When no Evidence Plan is provided, only Universal Criteria are evaluated.
```

- [ ] **Step 4: L67 "Step 2b" の activity 条件削除 (review-fix 限定分岐の廃止)**

Edit `plugins/belt-agent/agents/phase-auditor.md`:

From (L66-72 周辺):
```markdown
### Step 2b: Deferred Impact Verification
When E-DEFERRED-IMPACT is enabled in the Evidence Plan and the activity is review-fix:
1. Extract the deferred impact findings from the review results.
2. Read the file claimed by E-DEFERRED-IMPACT.
3. For each deferred finding, if the verification result does not match, add it as a dynamic severity: blocker criterion.
4. If the claimed file does not exist, report a blocker FAIL (missed collection).
```

To:
```markdown
### Step 2b: Deferred Impact Verification
When E-DEFERRED-IMPACT is enabled in the Evidence Plan:
1. Extract the deferred impact findings from the review results.
2. Read the file claimed by E-DEFERRED-IMPACT.
3. For each deferred finding, if the verification result does not match, add it as a dynamic severity: blocker criterion.
4. If the claimed file does not exist, report a blocker FAIL (missed collection).

(Previously this step was gated on `activity_type == review-fix`. After
Decision 3, the gate is simply "E-DEFERRED-IMPACT is in the Evidence
Plan" — if a skill declares it, it is relevant for that phase.)
```

- [ ] **Step 5: activity_type grep で 0 matches 確認**

Run:
```bash
grep -n "activity_type\|activity-type" plugins/belt-agent/agents/phase-auditor.md
```

Expected: 0 matches.

### 9.2 現 evidence-catalog.md 削除

- [ ] **Step 6: Path migration 最終 grep (spec Testing Strategy 項目 A)**

Run:
```bash
grep -rn "belt-agent/references/evidence-catalog" plugins/ crates/
```

Expected: 0 matches (phase-auditor.md も Step 2 で `./references/evidence-catalog.md` 参照が除去されているはず)。

1 件でも matches があれば、削除対象として追加 Edit を同 commit で実施。

- [ ] **Step 7: 現 evidence-catalog.md 削除**

Run:
```bash
git rm plugins/belt-agent/references/evidence-catalog.md
```

Expected: ファイル削除、staging 済み。

- [ ] **Step 8: workspace test 通過確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 pass。評価 test が残存していた場合は該当 test を同 commit で更新。

- [ ] **Step 9: Atomic Commit**

```bash
git add plugins/belt-agent/agents/phase-auditor.md
git commit -m "refactor(plugins): drop evidence-catalog from belt-agent layer, update phase-auditor

Stage 3 atomic: delete plugins/belt-agent/references/evidence-catalog.md
(migrated to skill-local in Tasks 3 and 4) and align phase-auditor with
spec Decision 3:
- Remove activity_type input field (enum: implementation/smoke-test/
  review-fix/test-fix/integration no longer exists).
- Replace ./references/evidence-catalog.md direct read with
  orchestrator-supplied evidence_catalog_path (skill-local).
- Ungate Step 2b (Deferred Impact) from 'activity == review-fix' — now
  purely presence-based (E-DEFERRED-IMPACT in Evidence Plan).

Spec gap acknowledged: phase-auditor updates were implicit implications
of Decision 3, not written explicitly into the spec body. Tracked as
spec gap note in the plan header; follow-up spec refresh optional."
```

---

## Task 10: `criteria-template.md` schema に `uses_evidence` 追加 (Stage 4 optional)

**Files:**
- Modify: `plugins/belt-agent/references/criteria-template.md`

Stage 4 は optional (spec より)。既存 criteria/*.md は `depends_on_artifacts` のまま温存、新規 skill のみ IoC 記法を使う。本 Task は schema 的に `uses_evidence` field を定義するのみ。

- [ ] **Step 1: File Format section への追加**

Edit `plugins/belt-agent/references/criteria-template.md` の File Format block 内:

From (既存):
```markdown
## File Format

```markdown
---
phase: {N}
name: {phase_name}
max_retries: 3
audit: required
---

## Criteria

### {ID}: {criterion title}
- **severity**: blocker | quality
- **verify_type**: automated | inspection
- **verification**:
  {Concrete steps for the Audit Agent to execute. For inspection type, numbered steps are required.}
- **pass_condition**: {No subjective terms. Use numeric thresholds or pattern matches that can be judged deterministically.}
- **fail_diagnosis_hint**: {On FAIL, what to investigate to resolve.}
- **depends_on_artifacts**: [{Paths to artifacts required for verification.}]
- **forward_check**: {Whether this is sufficient as input to the next phase. Optional.}
```
```

To (add `uses_evidence` optional field):
```markdown
## File Format

```markdown
---
phase: {N}
name: {phase_name}
max_retries: 3
audit: required
---

## Criteria

### {ID}: {criterion title}
- **severity**: blocker | quality
- **verify_type**: automated | inspection
- **verification**:
  {Concrete steps for the Audit Agent to execute. For inspection type, numbered steps are required.}
- **pass_condition**: {No subjective terms. Use numeric thresholds or pattern matches that can be judged deterministically.}
- **fail_diagnosis_hint**: {On FAIL, what to investigate to resolve.}
- **uses_evidence**: [{evidence-ids from the skill-local evidence-catalog.md, e.g., E-TEST, E-LINT. Optional.}]
- **depends_on_artifacts**: [{Paths to artifacts required for verification. Optional.}]
- **forward_check**: {Whether this is sufficient as input to the next phase. Optional.}
```
```

- [ ] **Step 2: Template Rules section へ 5 つ目の rule 追加**

Edit `plugins/belt-agent/references/criteria-template.md` の Template Rules section:

Add a new rule (5th) after existing 4 rules:
```markdown
5. **`uses_evidence` references must resolve**: If `uses_evidence` is present, each listed `E-XXX` MUST be declared in the owning skill's `./references/evidence-catalog.md`. (Currently convention-level; lint enforcement is Future Work per spec.)
```

- [ ] **Step 3: workspace test 通過確認**

Run:
```bash
cargo test --workspace
```

Expected: 全 pass。

- [ ] **Step 4: Commit**

```bash
git add plugins/belt-agent/references/criteria-template.md
git commit -m "docs(plugins/refs): add optional uses_evidence field to criteria schema

Stage 4 per spec: define the Inversion of Control syntax (uses_evidence:
[E-XXX]) for phase criteria to pick evidence from their skill-local
evidence-catalog.md. Optional field — existing criteria/*.md stay on
depends_on_artifacts unchanged (gradual migration). Lint enforcement
of uses_evidence resolution is tracked as Future Work."
```

---

## Task 11: Thought experiment sketch — weekly-sync pipeline (Testing Strategy 項目 C)

**Files:**
- Create: `docs/plans/examples/2026-04-18-weekly-sync-pipeline-sketch.yml`
- Create: `docs/plans/examples/2026-04-18-weekly-sync-evidence-catalog.md`
- Create: `docs/plans/examples/2026-04-18-weekly-sync-criteria-scan.md` (sample criteria)

Spec Testing Strategy 項目 C の validation。`evidence-schema.md` + 汎用化 narrative-convention / criteria-template / audit-protocol のみ参照し、weekly-sync pipeline (6 phase) を想定書き下しできることを手動確認する。

- [ ] **Step 1: Directory 作成**

Run:
```bash
mkdir -p docs/plans/examples
```

- [ ] **Step 2: pipeline.yml sketch 作成**

Create `docs/plans/examples/2026-04-18-weekly-sync-pipeline-sketch.yml`:

```yaml
# Thought experiment sketch for validating the refactor.
# This is NOT a deployable pipeline — it exists only to demonstrate
# that the new plugins/belt-agent/references/ layer is sufficient
# to design a non-development pipeline without leaks of dev-specific
# assumptions.

name: weekly-sync
version: 1
description: "Linear ↔ customer-facing output weekly sync (6 phases)"

args:
  from:
    type: string
    description: "Start date (ISO 8601 or YYYY-MM-DD)"
  to:
    type: string
    default: ""
    description: "End date (empty = today)"
  dry_run:
    type: bool
    default: false

phases:
  - id: setup
    description: "Validate config.md presence and schema"
    produces:
      - name: config_snapshot
        path: ".weekly-sync/config-snapshot.md"
      - name: setup_notes
        path: "belt://current/notes/phase-setup.md"
    gate:
      - file_exists: ".weekly-sync/config.md"
      - file_exists: "belt://current/notes/phase-setup.md"
    validate: ./criteria/setup.md

  - id: scan
    description: "Parallel scan Linear SSOT + output adapter"
    consumes: [config_snapshot]
    produces:
      - name: scan_result
        path: "belt://current/scan.json"
      - name: scan_notes
        path: "belt://current/notes/phase-scan.md"
    gate:
      - file_exists: "belt://current/scan.json"
      - file_exists: "belt://current/notes/phase-scan.md"
    validate: ./criteria/scan.md

  - id: analyze
    description: "Diff + sync plan generation"
    consumes: [scan_result]
    produces:
      - name: sync_plan
        path: "belt://current/sync-plan.json"
      - name: analyze_notes
        path: "belt://current/notes/phase-analyze.md"
    gate:
      - file_exists: "belt://current/sync-plan.json"
    validate: ./criteria/analyze.md

  - id: approve
    description: "Present plan, wait for user approval"
    when: "!args.dry_run"
    consumes: [sync_plan]
    confirm: true
    validate: ./criteria/approve.md

  - id: sync
    description: "Execute SSOT updates, customer document, transcribe, status sync"
    when: "!args.dry_run"
    consumes: [sync_plan]
    produces:
      - name: sync_log
        path: "belt://current/sync-log.json"
      - name: sync_notes
        path: "belt://current/notes/phase-sync.md"
    gate:
      - file_exists: "belt://current/sync-log.json"
    validate: ./criteria/sync.md

  - id: verify
    description: "Final verification + cleanup"
    when: "!args.dry_run"
    consumes: [sync_log]
    produces:
      - name: verify_report
        path: "belt://current/verify-report.md"
    gate:
      - file_exists: "belt://current/verify-report.md"
    validate: ./criteria/verify.md
```

- [ ] **Step 3: skill-local evidence-catalog.md sketch 作成**

Create `docs/plans/examples/2026-04-18-weekly-sync-evidence-catalog.md`:

```markdown
---
name: evidence-catalog
description: >-
  Concrete evidence catalog for weekly-sync pipeline (thought experiment).
  Conforms to plugins/belt-agent/references/evidence-schema.md.
---

# Evidence Catalog (weekly-sync thought experiment)

Schema: plugins/belt-agent/references/evidence-schema.md

## Evidence

### E-LINEAR-SCAN: Linear ticket scan result
- condition: always
- claimed: artifacts/scan/phase-{N}-linear.json
- verified: Re-issue linear issue list command and confirm count matches.
- required_capabilities: [linear-cli]
- if_unavailable: block
- collection: Save `linear issue list --team <team> --updated-after <from> --json` output.

### E-OUTPUT-ADAPTER-SCAN: Output adapter (GitHub Project) scan result
- condition: always
- claimed: artifacts/scan/phase-{N}-output.json
- verified: Re-issue gh project item list command and confirm count matches.
- required_capabilities: [gh-cli]
- if_unavailable: block
- collection: Save adapter's scan_existing + scan_new_external output.

### E-SYNC-PLAN: Diff analysis plan
- condition: always
- claimed: artifacts/analysis/phase-{N}-sync-plan.json
- verified: Re-run diff analysis on scan.json and confirm plan categories match.
- required_capabilities: [bash]
- if_unavailable: block
- collection: Output of analyze step (new, status-change, context-update categories).

### E-USER-APPROVAL: User approval transcript for sync plan
- condition: always
- claimed: artifacts/approval/phase-{N}-approval.md
- verified: N/A (interactive user input cannot be re-run)
- required_capabilities: []
- if_unavailable: block
- collection: Save user response ("ok" / modifications / "cancel") with timestamp.

### E-SYNC-SIDE-EFFECTS: Mutation log from sync phase
- condition: always
- claimed: artifacts/sync/phase-{N}-side-effects.json
- verified: Query Linear + output adapter and confirm mutations applied.
- required_capabilities: [linear-cli, gh-cli]
- if_unavailable: manual_fallback
- collection: Record every SSOT update, issue create, comment add, status change.

### E-CUSTOMER-DOC: Customer-facing weekly document
- condition: always
- claimed: artifacts/docs/phase-{N}-customer-doc.md
- verified: Confirm Linear document exists with title pattern `[定例 <to>]`.
- required_capabilities: [linear-cli]
- if_unavailable: block
- collection: Save generated customer document draft pre-approval and final post-approval.
```

- [ ] **Step 4: criteria/scan.md sketch 作成**

Create `docs/plans/examples/2026-04-18-weekly-sync-criteria-scan.md`:

```markdown
---
phase: 2
name: scan
max_retries: 3
audit: required
---

## Criteria

### SCAN-01: Both Linear and output-adapter scans completed without error
- severity: blocker
- verify_type: automated
- verification:
  1. Check artifacts/scan/phase-2-linear.json exists and contains a
     non-empty `tickets` array (or explicitly empty with `"tickets": []`).
  2. Check artifacts/scan/phase-2-output.json exists and contains a
     non-empty `items` array (or explicitly empty with `"items": []`).
  3. Parse both files as JSON and verify structural validity.
- pass_condition: Steps 1-3 all pass; both JSON files are valid.
- fail_diagnosis_hint: If Linear scan failed, re-run with --verbose. If
  adapter scan failed, check gh CLI authentication and project permissions.
- uses_evidence: [E-LINEAR-SCAN, E-OUTPUT-ADAPTER-SCAN]
- depends_on_artifacts: [artifacts/scan/]

### SCAN-02: Scan narrative note exists with 4 required sections
- severity: blocker
- verify_type: inspection
- verification:
  1. Read belt-agent status and locate artifact scan_notes resolved_path.
  2. Verify file exists at resolved_path.
  3. Verify frontmatter contains `phase: scan` and `run_id: <run_id>`.
  4. Verify 4 required sections exist: ## Decisions, ## Concerns,
     ## Directives, ## Observations.
  5. Verify Observations records ticket count and external link count.
- pass_condition: Steps 1-5 all pass.
- fail_diagnosis_hint: If any section is missing, re-open the note and fill
  it (at minimum `(none)` placeholder). See
  plugins/belt-agent/references/narrative-convention.md for schema.
- depends_on_artifacts: [scan_notes]
```

- [ ] **Step 5: Integrity check — evidence-schema.md で完結確認**

手動で:
1. `docs/plans/examples/2026-04-18-weekly-sync-evidence-catalog.md` の全 evidence 6 件が `plugins/belt-agent/references/evidence-schema.md` の declaration structure (id, description, claimed, verified, required_capabilities, condition, if_unavailable) に conformant
2. `docs/plans/examples/2026-04-18-weekly-sync-criteria-scan.md` の `uses_evidence: [E-LINEAR-SCAN, E-OUTPUT-ADAPTER-SCAN]` が上記 catalog に定義済み
3. narrative note の 4 sections (Decisions/Concerns/Directives/Observations) が criteria SCAN-02 で参照され、weekly-sync の scan phase に自然に fit
4. 旧 `activity_type` / `applies_to` の概念を**参照せずに**記述完結

すべて問題なければ、本 refactor の Success Criteria 3 (weekly-sync pipeline 書き下しで belt-agent references のみで完結) が PASS。

- [ ] **Step 6: Commit**

```bash
git add docs/plans/examples/2026-04-18-weekly-sync-pipeline-sketch.yml
git add docs/plans/examples/2026-04-18-weekly-sync-evidence-catalog.md
git add docs/plans/examples/2026-04-18-weekly-sync-criteria-scan.md
git commit -m "docs(plans/examples): add weekly-sync pipeline thought experiment

Validates spec Testing Strategy item C: non-development pipeline
can be designed using only plugins/belt-agent/references/ (generalized)
and spec Decision 2-4 (type-only core + IoC + canonical 4 sections).
No activity_type. No applies_to. Narrative 4 sections fit scan phase
naturally. Evidence catalog conforms to evidence-schema.md."
```

---

## Task 12: Final verification + plan drift check

**Files:** n/a (verification)

- [ ] **Step 1: 全 workspace test pass**

Run:
```bash
cargo test --workspace
```

Expected: 全 pass。

- [ ] **Step 2: workspace clippy**

Run:
```bash
cargo clippy --workspace -- -D warnings
```

Expected: no warnings (references 層の変更は Rust code に触れないはず、念のため確認)。

- [ ] **Step 3: Success Criteria 1 検証 — lock test / integration test pass**

Already confirmed via Step 1.

- [ ] **Step 4: Success Criteria 2 検証 — domain-neutral grep**

Run:
```bash
grep -rE "(feature-dev|bug-fix|implementation phase|investigation phase)" plugins/belt-agent/references/
```

Expected: 0 matches。

- [ ] **Step 5: Success Criteria 3 検証 — weekly-sync sketch review**

Task 11 Step 5 を再確認。sketch が evidence-schema.md のみで完結していることを人手確認。

- [ ] **Step 6: 参照 path 全回収検証**

Run:
```bash
grep -rn "belt-agent/references/evidence-catalog" plugins/ crates/
```

Expected: 0 matches。

- [ ] **Step 7: activity_type 全除去検証**

Run:
```bash
grep -rn "activity_type\|activity-type" plugins/
```

Expected: 0 matches。

- [ ] **Step 8: `applies_to` 全除去検証 (skill-local evidence-catalog 含む)**

Run:
```bash
grep -rn "applies_to" plugins/belt/skills/
```

Expected: 0 matches。

- [ ] **Step 9: git log 確認**

Run:
```bash
git log --oneline -15
```

Expected: Tasks 2-11 対応で 6-7 commits (Task 7/8 の一般化が no-op だった場合は少なめ)。順序:
1. evidence-schema.md 新設
2. feature-dev skill-local evidence-catalog.md
3. bug-fix skill-local evidence-catalog.md
4. narrative-convention.md 一般化
5. (criteria-template.md 一般化、touch 時のみ)
6. (audit-protocol.md 一般化、touch 時のみ)
7. evidence-catalog.md 削除 + phase-auditor.md 更新 (atomic)
8. criteria-template.md に uses_evidence 追加 (Stage 4 optional)
9. weekly-sync sketch

- [ ] **Step 10: 本 plan 自体を commit**

現在 plan は未 commit の可能性。plan 実装時は plan file も同時 commit。

Run:
```bash
git add docs/plans/2026-04-18-plugins-belt-agent-references-refactor-plan.md
git commit -m "docs(plans): add implementation plan for plugins/belt-agent/references refactor"
```

---

## Risk Mitigation (plan-scoped、spec の Risk Mitigation を補足)

| Risk | 対策 |
|---|---|
| Task 3/4 の evidence 重複が気になる | Spec Decision 2 (skill-local fill-in) の帰結。Future Work で shared 層抽出を追跡 |
| Task 9 の phase-auditor 更新が spec 本文に明示なし | 本 plan header の Spec Gap Notes で明記、follow-up spec refresh を optional で提案 |
| weekly-sync sketch の E-LINEAR-SCAN 等が actual impl で変わる可能性 | sketch は thought experiment、実装 spec は別 ticket |
| narrative-convention の section 命名 (Decisions 等) が domain によって不自然 | 本 spec では Option B-1 (canonical 維持)。pain 顕在化後の follow-up |
| phase-auditor の評価者が既存 orchestrator から `activity_type` を渡される場合 | 現状 `activity_type` を渡している orchestrator は本 plan では確認されていない (grep 結果: phase-auditor.md 内部のみ)。将来 dispatch 側で legacy kwarg が残っていた場合は subsequent follow-up で対応 |

---

## Spec Coverage Self-Check

| Spec 要件 | 実装 task |
|---|---|
| `evidence-schema.md` 新設 | Task 2 |
| feature-dev skill-local evidence-catalog.md 移設 | Task 3 |
| bug-fix skill-local evidence-catalog.md 移設 | Task 4 |
| narrative-convention.md L3/L81 一般化 | Task 6 |
| criteria-template.md 例示一般化 | Task 7 |
| audit-protocol.md 例示一般化 | Task 8 |
| 現 evidence-catalog.md 削除 + 参照切替 | Task 9 |
| Stage 4: `uses_evidence` optional field 追加 | Task 10 |
| Thought experiment: weekly-sync sketch | Task 11 |
| Success Criteria 検証 | Task 12 |
| doc-audit section 特殊 case | Task 1 Step 5 + Task 9 純 drop |
| phase-auditor.md 更新 (spec implicit implication) | Task 9 (plan で spec gap を埋める) |

---

## Execution Handoff

**Plan complete and saved to `docs/plans/2026-04-18-plugins-belt-agent-references-refactor-plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**

# Plugins belt-agent/references Refactor: Type-Only Core + IoC + Domain-Specific Split

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-18

## Summary

`plugins/belt-agent/references/` を **domain-neutral な type-only core** に再編する。開発ワークフロー固有の具体 (activity types 7 種 / evidence items 20 種 / feature-dev & bug-fix の例示) を各 skill 配下 (`plugins/belt/skills/<skill>/references/`) に移設し、belt-agent layer 本来の audience (LLM runtime、全 domain 共通) を復元する。evidence の参照方向を **Inversion of Control** (phase が evidence を pick する形) に反転して activity type 概念を廃止する。narrative-convention の 4 sections (Decisions/Concerns/Directives/Observations) は普遍性が pilot 検証で支持されたため維持し、例示のみ一般化する。

本 spec は **references layer の再編のみ** を扱う。belt-core / belt-agent binary は touch しない。weekly-sync / triage / security-scan 等の非開発 pipeline 実装は本 spec のスコープ外 (follow-up spec)。

## Background

### Problem

現行 `plugins/belt-agent/references/` (5 ファイル) は layer の責務から逸脱している:

| ファイル | 抽象度 | ハードコード度 |
|---|---|---|
| `_schema.md` | 高 (schema 定義) | なし — domain-neutral |
| `criteria-template.md` | 高 (template) | 薄い — 例示が dev 寄り |
| `audit-protocol.md` | 中 (手順) | 薄い — 用語中立 (`work phase`) |
| `narrative-convention.md` | 中〜低 | **濃い** — L3 で `"feature-dev and bug-fix"` を明言、L81 以降の例示全部が design/RCA 等開発 phase |
| `evidence-catalog.md` | 低 (具体カタログ) | **極めて濃い** — activity types 7 種 (implementation/investigation/smoke-test/review-fix/test-fix/doc-maintenance/integration) と evidence items 20 種 (E-TEST/E-BUILD/E-LINT/E-REVIEW/E-DIFF/E-SCREENSHOT/E-MIGRATION 等) が全て開発 workflow 向け |

**問題**: `plugins/belt-agent/` は本来 "agent runtime 向け汎用 layer" であるが、実際には開発 workflow 固有の内容が混入している。これからビジネススキル (triage / weekly-sync / 採用 / 法務 review 等) やセキュリティスキャン等の非開発 pipeline を belt 上で構築する際、上記 references を参照すると「開発しかサポートされていない」ように見え、ボトルネックとなる。

**痛みの発生場面** (本 spec brainstorming で確認):
- 新規 pipeline 設計時 (pipeline 作者)
- 新 skill 作成時 (skill 作者)
- 実行時の runtime orchestrator (LLM, phase-auditor)
- SSOT 読解時 (メンテナ / 学習者)

→ **全利用者場面で痛みが顕在化**

### Pilot-Driven Discovery

本 spec brainstorming で pilot 第 1 号として `weekly-sync` skill (既存 dotfiles skill) を想定し、belt パイプライン化した場合の disruption を測定した。

#### weekly-sync 想定 pipeline 構造 (6 phases)

```yaml
name: weekly-sync
args: { from, to, dry_run }
phases:
  - id: setup      # config 検証 (gate: file_exists .weekly-sync/config.md)
  - id: scan       # 並列 Linear + 出力先 scan → scan.json
  - id: analyze    # 差分 → sync-plan.json (dry-run 時はここで完了)
  - id: approve    # user 承認ゲート (confirm: true)
  - id: sync       # SSOT 更新 + Document 作成 + 転写 + ステータス同期
  - id: verify     # 検証 + 結果レポート
```

#### weekly-sync × 現行 references の disruption matrix

| reference | 適合度 | 問題 |
|---|---|---|
| `_schema.md` | ✓ 100% | schema 定義のみ、ドメイン中立 |
| `criteria-template.md` | ✓ 95% | template 構造は汎用、例示のみ dev 寄り |
| `audit-protocol.md` | ✓ 90% | phase-auditor dispatch 手順は汎用 |
| `narrative-convention.md` | △ 構造 OK / 例示 NG | 4 sections は普遍的に fit、例示が開発前提 |
| `evidence-catalog.md` | **✗ ほぼ全滅** | activity types 7 種のどれにも weekly-sync の phase が fit しない。evidence items 20 種中、E-TEST/E-BUILD/E-LINT/E-REVIEW/E-DIFF(git)/E-SCREENSHOT/E-MIGRATION 等は根本的に無関係 |

#### 重要仮説 (pilot で抽出)

1. **narrative 4 sections (Decisions/Concerns/Directives/Observations) は普遍的** — weekly-sync 6 phase 全てで自然に埋まる。思考実験 (triage / security-scan / 採用 / 法務) でも成立
2. **evidence-catalog は構造自体が開発依存** — activity type × evidence item の 2 軸とも総取り替え必要
3. **audit-protocol / criteria-template / _schema は layer 本来の責務に沿って無傷**
4. **activity type は "evidence が phase 横断で再利用されるドメインでのみ意味を持つ concept"** — feature-dev では E-TEST が 4 activity type で共有されるが、weekly-sync では evidence が phase 固有、activity type 抽象が機能しない

### Design Constraints (belt 既存原則との整合)

- **Binary Separation (原則 8)**: `plugins/belt-agent/` は agent runtime audience 向け、`plugins/belt/` は開発者 audience 向け skill 提供
- **Tiny by Constraint**: 未顕在な抽象は作らない、最小限の型化
- **Do One Thing and Do It Well**: layer ごとに責務を明確化
- **Pain-Driven First-Class** (feedback memory): 痛み顕在化で型化、未顕在は thin のまま
- **Verification Contract** (CLAUDE.md user): 独立 verification 必須、境界値 / 異常系 probe を含める

## Goals

1. 非開発 pipeline (weekly-sync 等) が `plugins/belt-agent/references/` を disruption なく参照できる状態を作る
2. 既存 feature-dev / bug-fix は挙動維持 (contents は domain-specific 層に移設するだけ)
3. Pilot-driven expansion を可能にする土台作り (pilot 2 以降で必要時に型化)

## Non-Goals

- activity type enum を belt layer に残すこと (Decision 3 で廃止)
- section 数を可変にすること (Decision 4 で canonical 4 sections 維持)
- 他 domain (security-scan / triage / 採用 / 法務) の具体 evidence-catalog 事前作成 (pain-driven 原則で必要時に追加)
- belt-core / belt-agent binary 変更 (plugins layer のみの再編)
- weekly-sync 自体を belt pipeline 化する実装 (別 spec)
- belt が narrative / evidence content を parse する方向性 (中立原則維持)
- `.belt/runs/*` state への影響 (references layer のみ)

## Success Criteria

- 現 feature-dev / bug-fix の全 lock test / integration test が pass
- `plugins/belt-agent/references/` 内に開発 workflow 固有の用語が残らない (narrative の `"feature-dev and bug-fix"` 言及、evidence-catalog の開発 activity/evidence enum が消える)
- weekly-sync のような pipeline を thought experiment として書き下した時、`plugins/belt-agent/references/` のみ参照して脇見不要になる

## Architecture

### 新 layer 構造

```
plugins/belt-agent/references/          ← domain-neutral type-only core
├── _schema.md                          ✓ 変更なし (done-criteria schema)
├── criteria-template.md                ✓ 例示を一般化
├── audit-protocol.md                   ✓ 例示を一般化、"work phase" 用語は維持
├── narrative-convention.md             ✓ L3 / L81 一般化、4 sections 維持
└── evidence-schema.md                  🆕 新設 (詳細下記)
    (evidence-catalog.md)               🗑 削除 (domain layer へ移動)

plugins/belt/skills/feature-dev/references/
└── evidence-catalog.md                 🆕 現 evidence-catalog の feature-dev 関連部分を移設

plugins/belt/skills/bug-fix/references/
└── evidence-catalog.md                 🆕 現 evidence-catalog の bug-fix 関連部分を移設
```

### `evidence-schema.md` (新設) の内容

```markdown
# Evidence Schema

Evidence collection と verification の型・protocol 定義。具体の evidence-id
は各 skill 配下の evidence-catalog.md で定義する。

## 2-Layer Model

- **Claimed (Layer 1)**: Executor が collect/store する "what happened" 記録
- **Verified (Layer 2)**: Audit Agent の独立 check による "really holds" 検証

## Applicability Condition 記法

- `condition: always` — 常に該当
- `condition: require_all: [...]` — 全て satisfy 時のみ該当
- `condition: require_any: [...]` — いずれか satisfy 時該当
- 述語は glob / grep 等の observable fact ベース

## if_unavailable Policy (3 種)

- `skip_with_warning`: evidence を除外、verdict に影響なし
- `manual_fallback`: PAUSE して user 収集待ち
- `block`: 収集不能なら即 FAIL (blocker 相当)

## Evidence Declaration Structure

各 evidence-id は skill-local な evidence-catalog.md で以下フィールドを持つ:

- `id` (例: `E-XXX`)
- `description`
- `claimed` (path template)
- `verified` (verification procedure)
- `required_capabilities`
- `condition`
- `if_unavailable`

## Phase Reference (Inversion of Control)

phase の criteria/*.md が `uses_evidence: [E-XXX]` で pick する。
evidence 側から phase を指定する逆方向 (`applies_to: [...]`) は採用しない。
activity type enum は存在しない。
```

### criteria schema 拡張 (optional field 追加のみ)

```markdown
### <ID>: <criterion title>
- severity, verify_type, verification, pass_condition, fail_diagnosis_hint  (既存)
- uses_evidence: [E-TEST, E-LINT]    🆕 optional, skill-local evidence-catalog 参照
- depends_on_artifacts: [path]        (既存、path 参照は並存可能)
- forward_check                        (既存)
```

### Layer 責務マトリクス

| Layer | 責務 | 例 |
|---|---|---|
| `plugins/belt-agent/references/` | type / schema / protocol 定義 | done-criteria schema, evidence schema, audit dispatch, narrative 4 sections 規約 |
| `plugins/belt/skills/<skill>/references/` | skill-specific 具体カタログ | 具体 evidence-id 定義、skill 固有意味解釈 |
| `plugins/belt/skills/<skill>/criteria/<phase>.md` | phase 固有 criteria | uses_evidence で catalog から pick |

### 依存方向 (一方向、循環なし)

```
criteria/<phase>.md
   ↓ (uses_evidence: [E-XXX])
skills/<skill>/references/evidence-catalog.md
   ↓ (conforms to schema)
plugins/belt-agent/references/evidence-schema.md
```

### 命名 rationale

- `evidence-schema.md` と `_schema.md` が対称 (両方 schema 定義、前者は evidence、後者は done-criteria)
- `evidence-catalog.md` の名前は **維持し場所のみ移動** — 意味は「catalog of available evidence for this skill」で変わらず、所有者が belt-agent → belt/skills/<skill> に移る

## Key Decisions

### Decision 1: Pilot-Driven Abstraction

- **採用**: 全方位汎用化を先に議論せず、pilot 1 つから concrete に抽象化パターンを抽出
- **Pilot 第 1 号**: `weekly-sync`
- **Pilot 第 2 号 (follow-up)**: `triage` or `security-scan`
- **根拠**: Pain-Driven First-Class 原則 + Tiny by Constraint + 1 pilot では抽象化パターンが見えない

### Decision 2: Type-Only Core (Option B)

- `plugins/belt-agent/references/` は schema / protocol / type の定義のみ
- 具体 (evidence-id 定義、domain 固有 activity 群) は skill-local
- **根拠**: `_schema.md` との哲学対称性、pain が未顕在な段階での具体先取り回避
- **副次効果**: `plugins/belt-agent/` layer が真に domain-neutral な audience (LLM runtime) 向けになる

### Decision 3: Inversion of Control (Option C-4)

- activity type enum (`implementation` / `investigation` / ...) を belt layer から完全削除
- 参照方向を反転:
  - 旧: `evidence-catalog` が `applies_to: [activity_type]` で phase を間接指定
  - 新: `criteria/<phase>.md` が `uses_evidence: [E-XXX]` で evidence を直接 pick
- **根拠**: belt 既存モデル (phase が declare する形) との整合、weekly-sync 観察 (activity type 抽象が実質無機能) の裏付け
- **副次効果**: evidence-catalog が read-only カタログとして再利用しやすい

### Decision 4: Narrative 4 sections 維持 (Option B-1)

- Decisions / Concerns / Directives / Observations を canonical な 4 sections として維持
- 例示 (feature-dev design phase example) のみ一般化
- **根拠**: pilot 検証 (weekly-sync 6 phase 全 fit) + 思考実験 (triage / security-scan / 採用 / 法務 で fit) により普遍性仮説支持。命名の domain mismatch はまだ痛みが顕在化していない
- **副次効果**: belt が narrative を parse しない前提と整合、弱制約 (convention level) で十分

### 判断間の整合性

- Decision 2 (type-only core) と Decision 3 (IoC) は両立的 — type-only core に IoC 記法を定義する形で両立
- Decision 4 (narrative) と Decision 2 は独立 — narrative は既に schema 的 (4 sections は type)
- Decision 1 (pilot-driven) は Decision 2-4 全体の判断時期・範囲を規律

### 破壊変更の scope

- **破壊あり**: `plugins/belt-agent/references/evidence-catalog.md` は削除、activity type enum 概念は消滅
- **破壊なし**: 既存 skill の `depends_on_artifacts` 記述、feature-dev / bug-fix の phase 構造、criteria/*.md の既存 fields
- **追加**: `uses_evidence` optional field, `evidence-schema.md` 新設, `<skill>/references/evidence-catalog.md` 移設

## Migration Plan

4 stage の段階移行で破壊変更を最小化する。

### Stage 1: 下地作り (追加のみ、破壊なし)

- `plugins/belt-agent/references/evidence-schema.md` 新設 — 既存 `evidence-catalog.md` から schema / 2-layer / applicability / policies / IoC 記法を抽出
- `plugins/belt/skills/feature-dev/references/evidence-catalog.md` 新設 — 現 evidence-catalog の feature-dev 関連 evidence を移設 (copy、まだ削除しない)
- `plugins/belt/skills/bug-fix/references/evidence-catalog.md` 新設 — bug-fix 関連部分
- 現 `plugins/belt-agent/references/evidence-catalog.md` は **この stage では残置** (rollback 容易化)
- Stage 1 内で doc-audit 関連 evidence (L157-213) の参照元調査 (grep)

#### `applies_to` フィールドの扱い (Decision 3 との整合)

現 `evidence-catalog.md` の各 evidence item には `applies_to: [activity_type]` フィールドがある。Decision 3 で activity type enum は廃止するため、skill-local への copy 時は以下のいずれかで扱う:

- **推奨**: `applies_to` フィールドを **copy 時に削除**。移設後の skill-local evidence-catalog は「この skill で収集可能な evidence のカタログ」として純粋に記述 (どの phase で使うかは phase 側が Stage 4 で `uses_evidence` で pick、または既存 `depends_on_artifacts` で path 参照)
- **代替**: `applies_to` を Stage 3 まで残し、Stage 4 で一括削除 (rollback 容易化を優先する場合)

本 spec では **推奨案** を採用。理由: Stage 1 で既に Decision 3 に整合した形で移設することで、Stage 3/4 での書換え範囲を最小化。activity type 概念を残した状態で移設するのは、後で削除する文字列を増やすだけで利得が薄い。

criteria/*.md 側の `depends_on_artifacts: [path]` は path 参照なので activity type と独立、影響なし。

### Stage 2: 一般化作業

- `narrative-convention.md` の L3 冒頭 (`"feature-dev and bug-fix"`) と L81 以降の feature-dev design example を domain-neutral に差替
- `criteria-template.md` の `fail_diagnosis_hint` 等の例示を一般化
- `audit-protocol.md` の例示を一般化 (`"work phase"` 汎用用語は維持)
- criteria/*.md の内容はこの stage でも変更しない

### Stage 3: 削除・切替

- `plugins/belt-agent/references/evidence-catalog.md` **削除**
- SKILL.md / criteria/*.md の参照 path を新 path に書き換え
  - 旧: `plugins/belt-agent/references/evidence-catalog.md`
  - 新: `./references/evidence-catalog.md` (skill-local)
- `git grep -r "belt-agent/references/evidence-catalog"` で参照回収
- Lock test / integration test が該当する場合は同 commit で更新

### Stage 4: IoC 記法採用 (gradual / optional)

- `criteria-template.md` schema に `uses_evidence: [E-XXX]` field を optional として追加
- 既存 criteria/*.md は `depends_on_artifacts: [path]` のまま温存 (migration は段階的)
- 新規 skill (weekly-sync 等) は最初から `uses_evidence` で pick

### doc-audit section の扱い (特殊 case)

現 evidence-catalog L157-213 "Doc Maintenance (specific to doc-audit)" は:
- doc-audit は **dotfiles の global skill、belt plugin ではない**
- 移設先が belt 内に存在しない

**推奨運用**: Stage 1 で `Grep -r "E-DOC-"` で参照元確認。
- 参照元が belt 内のみ → 無視して削除
- 参照元が dotfiles 等 belt 外 → dotfiles 改修は本 spec scope 外とし、一時的に `plugins/belt-agent/references/evidence-catalog-legacy.md` として Stage 3 まで残置 (dotfiles follow-up)

### Commit Strategy (見積)

- Stage 1: 2 commits (evidence-schema 新設、skill-local evidence-catalog 2 件追加)
- Stage 2: 3 commits (narrative / criteria-template / audit-protocol 各 1 commit)
- Stage 3: 1 commit (削除 + 参照切替 + test 更新 atomic)
- Stage 4: 0-1 commit (schema 追加のみ、optional)
- **合計 6-7 commits**

## Testing Strategy

### 既存 test への影響 (回帰)

| Test Layer | 影響 |
|---|---|
| Rust unit test (`crates/*/src/`) | なし |
| Rust integration test (`crates/*/tests/feature_dev_refresh.rs` 等) | Stage 3 で更新可能性 — Stage 1 の事前調査で `Grep -r "evidence-catalog"` により特定 |
| Scenarios contract (F1 実装済) | なし (references は contract 対象外) |
| docs/testing SSOT | なし |

### 追加する document-level 検証

#### A. Path migration grep check

Stage 3 実施前に:
```
Grep -r "belt-agent/references/evidence-catalog" plugins/ crates/
```
回収後、該当参照 0 件になることを commit 前に確認 (CI 化はスコープ外、手動)。

#### B. Domain-neutral 化検証

Stage 2 実施後に:
```
Grep -E "(feature-dev|bug-fix|implementation phase|investigation phase)" plugins/belt-agent/references/
```
該当 0 件 (Success Criteria 2 の自動検証)。

#### C. Thought experiment: weekly-sync pipeline 想定書き下し

本 spec の appendix として `examples/weekly-sync-pipeline-sketch.yml` (+ criteria 相当) を作成:

- `evidence-schema.md` のみを根拠に evidence-catalog.md (skill-local) + criteria/*.md が書けるか
- activity type なしで IoC 記法 (`uses_evidence`) が破綻しないか
- 4 sections narrative が全 phase で埋まるか

手動で整合確認する validation。

### 各 Stage の検証 gates

| Stage | 検証 |
|---|---|
| Stage 1 後 | `cargo test --workspace` 全 pass (追加のみ) |
| Stage 2 後 | `cargo test --workspace` 全 pass + domain-neutral 化 grep check (項目 B) |
| Stage 3 後 | `cargo test --workspace` 全 pass + path migration grep check (項目 A) |
| Stage 4 後 | `cargo test --workspace` 全 pass (schema 拡張は optional field、既存挙動無変更) |

### Adversarial Probes

| 観点 | Probe | 対処 |
|---|---|---|
| 境界 | `evidence-schema.md` に `if_unavailable` 3 種全てが spec されている | Schema 内 enum として明記 |
| 異常系 | 存在しない `uses_evidence: [E-XXX]` を criteria が参照した場合 | 本 spec では lint 化せず convention level (skill 作者責任)、Future Work で lint rule 追加 |
| idempotency | Stage 3 の削除 + 書換え commit を 2 回実行しても同結果 | 削除済み file の `rm` は no-op、grep 済み migration 書換えも idempotent |
| orphan operation | 新設 skill-local evidence-catalog が skill 削除で orphan 化 | 本 spec スコープ外 (skill lifecycle 管理問題) |
| 状態保持 | ongoing の belt run (`.belt/runs/*`) への影響 | references layer のみ、run state への影響なし |

### Out of Scope for Testing

- `evidence-schema.md` の machine-readable validation (belt は references を parse しない設計原則)
- `uses_evidence: [E-XXX]` 参照の自動 lint 化 (Stage 4 の optional、本 spec scope 外、Future Work)
- weekly-sync pipeline 実装 test (本 spec は references layer のみ)
- Cross-skill evidence reuse の enforcement (convention level のまま)

### Success Criteria 検証マッピング

| Success Criteria | 検証方法 |
|---|---|
| 現 feature-dev / bug-fix の lock test / integration test が pass | `cargo test --workspace` (各 Stage) |
| `plugins/belt-agent/references/` 内に開発固有用語が残らない | Stage 2 後の grep check (項目 B) |
| weekly-sync pipeline 書き下しで belt-agent references のみで完結 | Thought experiment sketch (項目 C) |

## Risk Mitigation

| Risk | 対策 |
|---|---|
| rename / move で lock test fail | Stage 1-2 は追加のみ、Stage 3 で lock test 更新を atomic commit |
| SKILL.md / criteria 参照 path drift | Stage 3 前に `git grep` で全参照回収、batch 書換え |
| `evidence-schema.md` が既存 `evidence-catalog.md` と矛盾 | Stage 1 で人手整合確認 |
| doc-audit 関連 evidence の移設先不在 | 参照確認 → 残置 or 削除の選択運用 |
| weekly-sync 等の新 pilot が evidence-schema だけでは書けない | 本 spec スコープ外だが Thought experiment sketch で事前検証 |

## Rollback Strategy

- Stage 1-2 は追加 / 修正のみなので `git revert <commits>` で単純 rollback 可
- Stage 3 commit は atomic、revert で復活可
- Stage 4 は optional、採用しなくても ongoing で害なし

## Future Work

1. **weekly-sync の belt pipeline 化実装**
   - 6 phase (setup/scan/analyze/approve/sync/verify) を `plugins/belt/skills/weekly-sync/` に実装
   - skill-local evidence-catalog.md の内容確定
   - 別 spec として管理

2. **Pilot 2 (triage / security-scan / 等) の belt 化**
   - 2 pilot 目で抽象化パターン再検証
   - `evidence-schema.md` 改訂判断は pain 顕在化後

3. **`uses_evidence: [E-XXX]` 参照の lint 化**
   - Stage 4 で追加された optional field の belt lint rule 化
   - skill-local evidence-catalog に定義された evidence-id のみ参照可能とする静的検証

4. **`evidence-schema.md` の machine-readable 化**
   - 現状 markdown 散文。YAML schema 化 or JSON Schema での machine validation は pain 顕在化後に検討

5. **doc-audit 関連 evidence の dotfiles 側整理**
   - Stage 3 で `evidence-catalog-legacy.md` として残置した場合の最終処理
   - dotfiles 側 doc-audit skill に同等 reference を設けるか、belt plugin に doc-audit を移植するか判断

6. **narrative 4 sections の domain-specific 意味解釈 override**
   - Option B-2 への発展 (現 B-1 で canonical 維持)
   - domain mismatch pain 顕在化時に検討

## Out of Scope

- belt-core / belt-agent binary の変更 (plugins layer のみ)
- 既存 feature-dev / bug-fix pipeline の構造変更 (contents 移設のみ)
- activity type enum 廃止に伴う既存 skill の immediate 書換え (gradual migration 許容)
- cross-skill evidence reuse の enforcement 機構 (convention level のまま)
- 他 domain (weekly-sync / security-scan / triage / 採用 / 法務) pipeline の事前設計・実装
- belt が narrative / evidence content を parse する方向性 (中立原則維持)
- `.belt/runs/*` の state 変更

## Spec Impact (他 spec への同期)

- `docs/specs/2026-04-06-belt-redesign.md`: layer 責務境界表で "references" の責務を追記 (type-only core 概念)
- `docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md`: narrative-convention.md 一般化と現 spec の example 記述との整合確認
- `docs/specs/2026-04-07-skill-md-authoring-principle.md`: SKILL.md 著作 3 責務に `./references/evidence-catalog.md` を skill-local に持つ責務を追記

## References

### 先行 spec

- [2026-04-06-belt-redesign.md](./2026-04-06-belt-redesign.md) — 3-crate / Separation by Audience
- [2026-04-07-skill-md-authoring-principle.md](./2026-04-07-skill-md-authoring-principle.md) — SKILL.md 著作原則
- [2026-04-14-belt-context-neutral-narrative-artifact.md](./2026-04-14-belt-context-neutral-narrative-artifact.md) — narrative-convention 由来
- [2026-04-08-audit-gate-pattern-design.md](../superpowers/specs/2026-04-08-audit-gate-pattern-design.md) — audit-protocol 由来
- [2026-04-16-review-skills-subagent-boundary-design.md](./2026-04-16-review-skills-subagent-boundary-design.md) — type-only core + IoC の先例

### CLAUDE.md 原則

- **Project**: Binary Separation (原則 8), Tiny by Constraint, Do One Thing and Do It Well
- **User**: Pain-Driven First-Class 実践要項、Verification Contract

### 関連 Linear

- [BELT-20](https://linear.app/neko-neko/issue/BELT-20) — belt master tracking

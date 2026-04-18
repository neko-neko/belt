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

# docs/testing/ — belt test SSOT + lock meta

This directory is the belt project's **long-term test foundation**:

- **CLI behavioral SSOT** — what belt / belt-agent / belt-core public API should do, expressed as Given/When/Then scenarios
- **Lock ledger** — what shape-lock tests (`*_refresh.rs`, `shared_*_parity.rs`) protect
- **Audit operations meta** — template + reason labels for the F2/F3 test audit

## Boundary vs `docs/features/<topic>/`

| Aspect | `docs/testing/` (this dir) | `docs/features/<topic>/` |
|---|---|---|
| Lifetime | permanent, cross-feature SSOT | per-feature, archived when feature closes |
| Consumers | F2/F3 audit + CI lock | test-scenarios skill + monkey-test + dogfood |
| Schema | scenarios.yml schema + `scope:` / `technique:` additive fields | scenarios.yml schema (UI replay契約) |
| Writers | human + feature-dev Phase 5 execute | feature-dev Phase 2 test-scenarios skill |

## Contents

- `cli-behavior/belt.yml` — belt lint CLI behavioral scenarios
- `cli-behavior/belt-agent.yml` — belt-agent CLI behavioral scenarios (F3 で拡充、F1 は stub)
- `cli-behavior/belt-core.yml` — belt-core public API behavioral scenarios (F1 で config、F2a で engine/view/lint/model/gate/error/expander/parser/uri/artifact_when の 10 module 拡充、残 shape-lock file は F2b scope)
- `lock-ledger.md` — plugin shape lock test 台帳
- `audit-template.md` — F2/F3 audit 判定手順 (v1 9 reason labels / decision tree / re-audit trigger)

## monkey-test との非互換

`docs/testing/cli-behavior/*.yml` は CLI 向けであり、`/belt:monkey-test` (agent-browser replay) で消費するものではない。これらは `docs/features/<topic>/scenarios.yml` (UI scenarios) とは別の SSOT。monkey-test SKILL.md は `docs/features/<topic>/scenarios.yml` のみを input と宣言しており、`docs/testing/cli-behavior/` は path が異なるため偶発的消費は起きない。

## Binding

`docs/testing/cli-behavior/*.yml` の全 scenario ID は `crates/*/tests/**/*.rs` 内の `/// scenario: <id>` doc-comment と `crates/belt-core/tests/scenarios_contract.rs` で機械照合される。drift があれば CI (`cargo test`) で検出される。

## Related

- CLAUDE.md: belt project overview
- `plugins/belt/skills/test-scenarios/SKILL.md`: UI scenarios.yml producer (別 path)
- `docs/features/2026-04-17-belt-test-foundation/`: F1 feature design + test strategy + plan + audit report

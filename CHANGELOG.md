# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Added

- `/belt:plan` stage skill — implementation planning (plan.md + scenarios.yml) split out of design (D1)
- `/belt:qa` stage skill — mandatory QA with human-readable evidence: browser screenshots / cli transcripts captured by the new independent `belt:qa-verifier` agent (D2, D3)
- `/belt:requirements` — interview-driven requirements definition writing docs/requirements/
- `/belt:docs` — general-purpose documentation writing under docs/
- `belt-agent:explorer` — unified codebase explorer (focus: flow | patterns | impact)
- `[qa] evidence` belt.toml key — evidence destination: pr | linear | local | auto (D9)

### Changed

- **BREAKING**: `--e2e` removed from feature-dev / bug-fix / design / diagnose / build — QA is always on (D2)
- feature-dev is now design → plan → checkpoint → build → qa → integrate; bug-fix is diagnose → checkpoint → build → qa → integrate; integrate moved out of build (D5)
- Human confirms reduced to design approval, plan approval, checkpoint, integrate (feature) / diagnosis approval, checkpoint, integrate (bug) (D4)
- `/belt:code-review` triage is autonomous in pipeline mode (critical/high auto-fixed or deferred with reason)
- `/belt:spec-review` accepts a target path and an output artifact name / directory
- `belt-agent:feature-implementer` renamed to `belt-agent:implementer`
- rca-scenarios.yml is always authored (was `--e2e` gated)

### Removed

- **BREAKING**: `/belt:verify` (replaced by `/belt:qa`)
- **BREAKING**: agents `belt-agent:code-explorer`, `belt-agent:code-architect`, `belt-agent:impact-analyzer`, `belt-agent:phase-auditor` (consolidated into `belt-agent:explorer` / retired), and `references/audit-protocol.md`

## [0.2.0] - 2026-04-17

### Changed (BREAKING)
- Plugin consolidation: 7 plugins (`belt-agents`, `feature-dev`, `bug-fix`, `code-review`, `spec-review`, `monkey-test`, `test-scenarios`) → 2 plugins (`belt`, `belt-agent`).
- Skill invocation renamed:
  - `/feature-dev:feature-dev` → `/belt:feature-dev`
  - `/bug-fix:bug-fix` → `/belt:bug-fix`
  - `/code-review:code-review` → `/belt:code-review`
  - `/spec-review:spec-review` → `/belt:spec-review`
  - `/monkey-test:monkey-test` → `/belt:monkey-test`
  - `/test-scenarios:test-scenarios` → `/belt:test-scenarios`
- Agent namespace renamed:
  - `belt-agents:<agent>` → `belt-agent:<agent>` (5 base analysis agents)
  - `code-review:<reviewer>` → `belt:<reviewer>` (4 observation reviewers)
  - `spec-review:<reviewer>` → `belt:<reviewer>` (3 observation reviewers)
- Belt Protocol skill slug: `belt-agents:belt-agent` → `belt-agent:protocol`
- Installation: `/install-plugin neko-neko/belt <plugin>` now takes 2 plugin names (`belt-agent` and `belt`) instead of 7.

## [0.1.0] - 2026-04-17

### Added
- Initial public release of belt workflow engine for LLM-driven Agent Skills.
- `belt lint` CLI: static validator for belt YAML pipelines. Detects duplicate phase IDs, unknown `regate` targets, undefined args in `when:`, missing descriptions, unresolvable `uses:` / `invoke.pipeline:` references, artifact flow violations, and sub-pipeline expansion failures.
- `belt-agent` CLI (`init` / `next` / `verify` / `regate` / `step` / `status`): runtime for agent-driven pipeline execution. All output is JSON.
- Sub-pipeline `uses:` composition with flat namespace expansion (`{parent_id}/{sub_phase_id}`).
- GateCheck (4 variants): `cmd`, `file_exists`, `git_clean`, `has_output`.
- Invoker + Artifact first-class model (BELT-32) with `invoke.skill` / `invoke.pipeline` dispatch.
- Narrative artifacts with `belt://` URI scheme (3 selectors: `belt://run/<id>/<path>`, `belt://latest/<pipeline>/<path>`, `belt://workspace/<branch>/latest/<pipeline>/<path>`).
- Cross-run inheritance via `belt-agent init --inherits-from <run_id>` — a fresh agent picks up a prior run's gated outputs without inheriting its trial-and-error trace.
- `belt.toml` config file for path resolution (BELT-22).
- Enriched `status` output (BELT-29): query-time assembly from run state, pipeline YAML, and output directories.
- 7 Claude Code plugins under `plugins/` (belt-agents, feature-dev, bug-fix, code-review, spec-review, monkey-test, test-scenarios) as working examples of belt-driven quality-gated AI development.

<!-- next-url -->
[Unreleased]: https://github.com/neko-neko/belt/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/neko-neko/belt/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/neko-neko/belt/releases/tag/v0.1.0

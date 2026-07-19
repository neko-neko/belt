# Evidence: wayfinder-layer

## intake — 2026-07-19T06:19:55Z
- Command: read `docs/requirements/2026-07-19-wayfinder-layer/requirements.md` (requirements.md path input; condensed Goals / Acceptance criteria / Out-of-scope; Open decisions → Open risks)
- Observed: 0 question rounds — scope, goal, and acceptance were fully settled by the upstream /belt:requirements run (frontier interview + spec-review, 6 findings applied); all remaining Open decisions are design-stage technical choices, none intake-level
- Artifacts: [goal-sheet.md](./goal-sheet.md)

## design — 2026-07-19T06:23:21Z
- Command: (direct authoring) — resolved requirements Open decisions into architecture; no explorer subagents (skill-authoring area already known, <10 unfamiliar files)
- Observed: 3-phase topology (map → resolve → handoff), resolve loop as max_retries=0 gate iteration, skill-driven dynamic /belt:requirements handoff, one-run-one-effort boundedness, 9 key decisions with rejected alternatives
- Artifacts: [design.md](./design.md)

## design-review — 2026-07-19T06:41:32Z
- Command: Task(belt:spec-reviewer) on design.md → findings-spec.json (codex off)
- Observed: 7 findings (HIGH 2 / MEDIUM 5), all applied after user approval. Fixes: resolve gate → linear api GraphQL scoped to map-ref with ≥1-decision-child clause; relation add → blocked-by form (was inverted); handoff gate → run-scoped belt://current/handoff-ref.json + pin --run after nested requirements; deep-HITL bound to brainstorming/domain-modeling/prototype (NFR-5); frontier scoped to map ID via GraphQL; discretionary rules tightened to checklists (ticket-vs-fog / map-index bound / cycle detection). Introduced map-ref.md as the unifying scoping key.
- Artifacts: [design.md](./design.md), findings-spec.json (run-scoped)

## plan — 2026-07-19T06:44:43Z
- Command: (direct authoring) — wrote plan.md (Test Strategy table + 5 tasks) and scenarios.yml from goal-sheet.md + design.md
- Observed: 5 build tasks (pipeline.yml / SKILL.md / belt.toml / README / manifests); QA split into 6 deterministic static scenarios (auto) + 6 pilot-* scenarios that write real Linear (require user authorization + teardown, else SKIP-with-approval); every goal-sheet AC mapped to ≥1 Test Strategy row + ≥1 scenario
- Artifacts: [plan.md](./plan.md), [scenarios.yml](./scenarios.yml)

## plan-review — 2026-07-19T07:02:36Z
- Command: Task(belt:spec-reviewer) on plan.md + scenarios.yml → findings-plan.json (codex off)
- Observed: 7 findings (HIGH 2 / MEDIUM 3 / LOW 2), all handled after user approval. Fixes: pilot-* moved out of scenarios.yml into pilot-runbook.md (belt QA has no per-scenario auth gate — F1); config-and-readme false-pass split into per-token AND greps → readme-declarations + skill-config-rounds + manifests-list-wayfinder (F2); teardown + label precondition added to pilot-runbook (F3/F7); merge/acceptance bar + required-pilot follow-up added to plan.md (F4); research phase attribution fixed to map-dispatch (F6). F5 partial: map-ref produces glob kept (reusable skill can't hardcode topic; belt convention) with the map-id-scoped cmd made authoritative. Added task T6 (pilot-runbook).
- Artifacts: [plan.md](./plan.md), [scenarios.yml](./scenarios.yml), [pilot-runbook.md](./pilot-runbook.md), findings-plan.json (run-scoped)

## execute — 2026-07-19T08:26:57Z
- Command: authored T1 `pipeline.yml` + T2 `SKILL.md` via two `belt-agent:implementer` subagents (self-contained prompts); T3 `belt.toml` / T4 README / T5 manifests edited directly; T6 pilot-runbook already in sync. Verified with `belt lint plugins/belt/skills/wayfinder/pipeline.yml` + all 8 `scenarios.yml` deterministic checks; committed cb1fd94 (triad) + f6a9c75 (README/manifests). No Rust changed (NFR-1).
- Observed: 8/8 deterministic scenarios PASS — lint-passes, pipeline-structure, command-correctness, skill-config-rounds, readme-declarations, manifests-list-wayfinder, git-hygiene, requirements-skill-untouched. `belt lint` exit 0 (`ok`, no warnings) — this is the project linter for a YAML/prose-only feature; `cargo test`/`cargo clippy` out of scope since git-hygiene confirms `crates/` is byte-untouched. Adversarial probes: both `cmd` gate block scalars pass `sh -n` shell-syntax after the `>-` YAML fold; independent verification of the documented command forms found + fixed one defect — SKILL.md documented `belt-agent status --json` (a non-existent flag, `status` emits JSON by default); `belt-agent locate --run` confirmed valid.
- Artifacts: `plugins/belt/skills/wayfinder/pipeline.yml`, `plugins/belt/skills/wayfinder/SKILL.md`, `plugins/belt/skills/wayfinder/belt.toml`, `README.md`, `plugins/belt/.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json`

## code-review — 2026-07-19T08:44:59Z
- Command: `Task(belt:code-reviewer)` + `Task(belt:quality-reviewer)` in parallel over `git diff main...HEAD`; deterministic merge → findings.json; pipeline-mode autonomous triage (critical/high fixed, medium/low recorded). Every command-form finding independently re-verified against `linear ... --help` before acting.
- Observed: 6 findings (code 0 / quality 5; 1 HIGH / 3 MEDIUM / 1 LOW). **1 HIGH fixed + committed 6cf52ea**: `linear comment create` → `linear issue comment add` (2 sites; verified via `linear issue comment --help` — no top-level `comment` verb exists). Post-fix: 8/8 deterministic scenarios PASS; `belt lint` exit 0 (`ok`). `cargo clippy`/`cargo test` N/A — zero Rust delta (git-hygiene proves `crates/` byte-untouched); the project checks for this YAML/prose feature are `belt lint` + the 8 `scenarios.yml` deterministic checks.
- Deferred (recorded per pipeline-mode critical/high-only triage; surfaced at integrate for user decision — each verified real):
  - **F2 MEDIUM** `SKILL.md:38` — `linear label create <name>` uses a positional name; CLI requires `--name <name>` (verified `linear label create --help`). One-line fix available; deferred as non-critical.
  - **F3 MEDIUM** `SKILL.md:33` — `linear label list --workspace neko-neko` misuses `--workspace` (on `label list` it is a boolean 'workspace-level only' filter, verified `linear label list --help`), so it does not target neko-neko. Deferred as non-critical.
  - **F4 MEDIUM** `SKILL.md:127` / `design.md:93` — frontier query traverses map→effort→decision (depth 2); a `wf:decision` placed directly under the map (permitted by design.md:93) is excluded. Deferred: the fix changes approved design scope; the implemented map-creation flow always parents decisions under an effort (`--parent <effort-id>`), so there is no runtime frontier gap in the authored behavior — only a design-vs-skill wording reconciliation for the user.
  - **F5 LOW** `scenarios.yml:19` — `command-correctness` is presence-only (allowlist + one negative), so wrong-but-plausible forms (like the fixed F1) pass autonomously. Enhancement: add `grep -q 'issue comment add'` positive + `! grep -q 'linear comment create'` / `! grep -Eq 'label create <'` negatives. Deferred as low.
- Artifacts: findings-code.json / findings-quality.json / findings.json (run-scoped `review/`), commit 6cf52ea

## qa — 2026-07-19T08:51:02Z
- Command: `Task(belt:qa-verifier)` replayed `docs/features/2026-07-19-wayfinder-layer/scenarios.yml` (8 `kind: cli` scenarios), one transcript per scenario under the run's `qa/` dir. No FAILs → no fix rounds (0 QA fix commits). Live pilot (P1–P6) held out of autonomous scope by design.
- Observed: **8/8 PASS** — lint-passes, pipeline-structure, command-correctness, skill-config-rounds, readme-declarations, manifests-list-wayfinder, git-hygiene, requirements-skill-untouched. Verdict PASS. Independently cross-checked: all 8 transcripts hold verbatim `when:` commands + real stdout/exit codes (spot-checked lint-passes `ok:`/exit 0 and git-hygiene grep-empty/exit 1); results match the orchestrator's own two prior independent scenario runs. qa-verifier edited no source file. `[qa] evidence` unset → `auto` → published at integrate.
- Deferred to user-driven pilot (required follow-up, recorded at integrate): AC-2..AC-6 + AC-8 live Linear behaviors in `pilot-runbook.md`. Merge bar: "authored, pilot-unverified".
- Artifacts: [qa-report.md](./qa-report.md); 8 transcripts under `.belt/runs/019f7907-8f44-7163-ab46-657e71629c1a/qa/` (not committed)

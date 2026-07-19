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

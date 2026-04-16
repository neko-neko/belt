# belt

A lightweight workflow engine for AI agents. Define deterministic state machines
in YAML, drive them idempotently from any LLM.

## Why belt?

When LLM agents control entire workflows — phase transitions, gate checks,
retry loops — they burn context on bookkeeping instead of reasoning. A 10-phase
pipeline can cost ~900 lines of prompt just to maintain structure. belt moves
the deterministic control plane into YAML: the agent calls `belt-agent next`
to receive one phase at a time, executes it, and calls `belt-agent verify`
to check gates. The pipeline definition never enters the context window.

## Example

```yaml
name: review-and-ship
version: 1

args:
  skip_e2e: { type: bool, default: false }

phases:
  - id: build
    description: "Build the project and run unit tests."
    gate:
      - cmd: "cargo build --workspace"
      - cmd: "cargo test --workspace"

  - id: review
    description: "Multi-perspective code review."
    config:
      skill: "/code-review"
    validate:
      - "No high-severity findings remain unresolved"
    confirm: true

  - id: e2e
    description: "Run E2E tests with flaky detection."
    when: "!args.skip_e2e"
    gate:
      - cmd: "npx playwright test"

  - id: ship
    description: "Push to main."
    confirm: true
```

## CLI

belt ships two binaries, separated by audience:

| Binary | Audience | Purpose |
|--------|----------|---------|
| `belt` | Pipeline authors | `belt lint <pipeline.yml>` — static validation |
| `belt-agent` | LLM / CI / scripts | `init`, `next`, `verify`, `step`, `status` — runtime |

### Agent loop

```
belt-agent init pipeline.yml --arg skip_e2e=true
loop {
  phase  = belt-agent next
  execute(phase)                    # LLM does the work
  result = belt-agent verify        # belt checks the gates
  belt-agent step [--confirm]       # advance
}
```

All `belt-agent` output is JSON. The agent never needs the full pipeline —
just the current phase.

### Status

Check pipeline state at any time:

```bash
belt-agent status                  # latest run
belt-agent status --run <run_id>   # specific run
```

Returns an enriched view assembled from run state, pipeline YAML, and output
directories — enough for a new LLM session to resume work without prior context:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    { "id": "build", "status": "completed", "verify_passed": true, "attempt": 1, "outputs": ["report.json"] },
    { "id": "review", "status": "current", "verify_passed": false, "attempt": 2, "outputs": [] },
    { "id": "test", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] },
    { "id": "deploy", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] }
  ]
}
```

## Key Concepts

- **gate** (verification) — Deterministic checks belt runs automatically:
  `cmd`, `file_exists`, `git_clean`, `has_output`. All must pass to advance.
- **validate** (validation) — Criteria belt *returns* for the LLM to judge.
  belt cannot verify these; `--confirm` is the agent's declaration that it did.
- **uses:** — Compose pipelines from reusable sub-pipelines and gate
  definitions. `uses: ./pipelines/review-cycle.yml` expands inline, namespaced
  as `review/phase-id`.
- **regate** — After a later phase passes its own gates, re-run earlier gates
  to catch regressions.
- **config** — Opaque metadata passed through to the LLM. belt doesn't
  interpret it; your skills do.

## Build

```bash
cargo build --workspace
```

Build only what you need:

```bash
cargo build -p belt          # lint tool
cargo build -p belt-agent    # agent runtime
```

## Claude Code Plugins

belt ships 7 Claude Code plugins under `plugins/` — working examples and
production tooling for quality-gated AI-driven development.

### Plugins in this repo

| Plugin | Purpose |
|---|---|
| `belt-agents` | Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + references |
| `feature-dev` | 9-phase development pipeline (design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate) |
| `bug-fix` | 8-phase debugging pipeline (rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate) |
| `code-review` | Multi-perspective code review (7 observations: quality / security / perf / test / ai-antipattern / impact / simplification) |
| `spec-review` | Multi-perspective spec review (5 observations: requirements / design-judgment / feasibility / consistency / ui-design) |
| `monkey-test` | Scripted E2E regression via agent-browser (Given/When/Then replay) |
| `test-scenarios` | Test strategy (ISTQB + ISO 25010) + Given/When/Then scenarios |

### External skill dependencies

`feature-dev` and `bug-fix` invoke skills from other plugins. `monkey-test`
requires the `agent-browser` CLI. Install them before the belt plugins that
use them:

| Dependency | Source | Required by |
|---|---|---|
| `/brainstorming` | [obra/superpowers](https://github.com/obra/superpowers) | feature-dev Phase 1 |
| `/writing-plans` | obra/superpowers | feature-dev Phase 4, bug-fix Phase 2 |
| `/subagent-driven-development` | obra/superpowers | feature-dev Phase 5, bug-fix Phase 4 |
| `/systematic-debugging` | obra/superpowers | bug-fix Phase 1 |
| `/worktrunk` | [max-sixty/worktrunk](https://github.com/max-sixty/worktrunk) | feature-dev Phase 9, bug-fix Phase 8 (integrate) |
| `agent-browser` CLI + `/agent-browser` skill | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | monkey-test (always), feature-dev Phase 7, bug-fix Phase 6 (when `--e2e`) |
| `/dogfood` | vercel-labs/agent-browser | feature-dev Phase 8, bug-fix Phase 7 (when `--e2e`) |

### Install

Install external dependencies first, then belt plugins. `skills add` supports
both GitHub shorthand (`owner/repo`) and full URLs. Use `-g` for global
install (all projects) or omit for project-local.

```bash
# 1. superpowers (/brainstorming, /writing-plans, /subagent-driven-development, /systematic-debugging)
npx skills add obra/superpowers -g -y

# 2. worktrunk (/worktrunk)
npx skills add max-sixty/worktrunk -g -y

# 3. agent-browser plugin (agent-browser CLI + /dogfood skill)
npx skills add vercel-labs/agent-browser --skill agent-browser --skill dogfood -g -y

# 4. belt plugins (all 7)
npx skills add neko-neko/belt -g -y
```

Selective install (only some belt plugins):

```bash
# Example: code-review only (no external deps)
npx skills add neko-neko/belt --skill code-review -g -y
```

Plugin discovery uses `.claude-plugin/marketplace.json` at belt repo root.

### Internal dependencies (plugin-to-plugin)

- `feature-dev` invokes `spec-review`, `code-review`, `test-scenarios`, `monkey-test`
- `bug-fix` invokes `spec-review`, `code-review`, `monkey-test`
- `feature-dev`, `bug-fix` require `belt-agents` (analysis agents referenced by criteria and supplements)
- `code-review`, `spec-review`, `monkey-test`, `test-scenarios`, `belt-agents` are standalone

### Usage

After install:

```
/feature-dev:feature-dev         # start a new feature
/bug-fix:bug-fix                 # start a bug investigation
/code-review:code-review         # standalone code review
/spec-review:spec-review         # standalone spec review
```

See each plugin's `SKILL.md` for phase details and arg reference.

## License

MIT

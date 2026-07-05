# belt

A workflow engine for LLM-driven Agent Skills. Declare deterministic state
machines in YAML, drive them idempotently from any LLM, and lint them
statically before they ever reach execution.

## Why belt?

When LLM agents control entire workflows — phase transitions, gate checks,
retry loops — they burn context on bookkeeping instead of reasoning. A 10-phase
pipeline can cost ~900 lines of prompt just to maintain structure. belt moves
the deterministic control plane into YAML: the agent calls `belt-agent next`
to receive one phase at a time, executes it, and calls `belt-agent verify`
to check gates. The pipeline definition never enters the context window.

Pipelines are statically linted with `belt lint` before any LLM run, so
structural errors — missing phase IDs, invalid gate checks, broken `uses:`
references — never reach execution.

When context grows polluted, a fresh agent can resume from a prior run's
gated outputs without inheriting its trial-and-error trace.

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

Lint it before handing it to the agent:

```
$ belt lint review-and-ship.yml
ok: review-and-ship.yml
```

If any phase id is duplicated, a gate is malformed, or a `uses:` reference is
unresolvable, lint exits non-zero with a descriptive diagnostic and the agent
is never invoked.

## CLI

belt ships two binaries, separated by audience:

| Binary | Audience | Purpose |
|--------|----------|---------|
| `belt` | Pipeline authors | `belt lint <pipeline.yml>` — static validation |
| `belt-agent` | LLM / CI / scripts | `init`, `next`, `verify`, `step`, `status` — runtime |

`belt lint` is the pipeline author's fast feedback loop: it runs in
milliseconds, catches structural errors (duplicate phase IDs, unknown `regate`
targets, undefined args referenced from `when:`, missing descriptions,
unresolvable `uses:` / `invoke.pipeline:` references, artifact flow
violations, and sub-pipeline expansion failures), and exits non-zero on any
finding — ideal for pre-commit hooks and CI.

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

## Continuity

Long LLM sessions accumulate context that pollutes reasoning. Even with
summary compaction, prior failed attempts bias the next try. Sometimes you
want a fresh agent that has seen only what matters — not a long-memory one
that has seen everything.

### Cold-start principles

belt is built on two cold-start guarantees:

- **Per-command neutrality** — every `belt-agent` call works from a cold
  start. No conversation history is required; the run state on disk is the
  source of truth.
- **Narrative artifacts** — phase outputs are deterministic files protected
  by gates, not LLM memory. A new session reads them; it does not
  reconstruct them.

### Two resumption modes

On top of those principles, belt offers two complementary ways to continue
work without carrying a polluted context:

|                 | Intra-run handover                           | Cross-run inheritance                   |
|-----------------|----------------------------------------------|-----------------------------------------|
| When            | Same run, new session                        | New run, reads prior artifacts          |
| What is carried | Resume hint + existing state.json            | Gated artifacts via `belt://` URIs      |
| Command         | `/belt:handover` → `/clear` → `/belt:resume` | `belt-agent init --inherits-from <run>` |
| Typical use     | Context bloat mid-pipeline                   | Fresh run consumes prior conclusions    |

**Intra-run handover.** When a pipeline run is mid-flight and the session's
context has grown polluted, `/belt:handover` writes a short Resume hint
(pause reason, first action, transient context) under the current run
directory. After `/clear`, `/belt:resume` reads the hint and `state.json`
and the next session picks up exactly where it left off:

```
/belt:handover
/clear
/belt:resume
```

The pipeline is never re-initialized; the resumed session continues the
current phase with a fresh context but the same run.

**Cross-run inheritance.** `belt-agent init --inherits-from <run_id>` lets
a new run consume a prior run's artifacts via `belt://` URIs:

- `belt://latest/<pipeline>/<path>` — most recent COMPLETED run on the
  current branch
- `belt://workspace/<branch>/latest/<pipeline>/<path>` — branch-scoped
  variant
- `belt://run/<run_id>/<path>` — explicit run reference

A typical use case: a long bug investigation produces `rca.md` and stops.
A fresh agent later picks up the conclusions without inheriting the
original trial-and-error trace:

```
belt-agent init bug-fix.yml --inherits-from <prior-run-id>
```

Both are `/clear` that keeps what matters —
handover keeps the run, inheritance keeps the conclusions.

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

## Install

### Shell installer (recommended)

Installs `belt` and `belt-agent` to `$HOME/.cargo/bin` (or configurable),
auto-detects platform.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/neko-neko/belt/releases/latest/download/belt-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/neko-neko/belt/releases/latest/download/belt-agent-installer.sh | sh
```

### Manual install (CI / Docker)

Pick a tarball matching your platform from the
[releases page](https://github.com/neko-neko/belt/releases):

```bash
# Example: Linux x86_64 — one tarball per binary
curl -L https://github.com/neko-neko/belt/releases/latest/download/belt-x86_64-unknown-linux-gnu.tar.xz \
  | tar -xJ -C /usr/local/bin belt
curl -L https://github.com/neko-neko/belt/releases/latest/download/belt-agent-x86_64-unknown-linux-gnu.tar.xz \
  | tar -xJ -C /usr/local/bin belt-agent
```

Replace the triple to match your platform:

| OS | Arch | Triple |
|---|---|---|
| macOS | Intel | `x86_64-apple-darwin` |
| macOS | Apple Silicon | `aarch64-apple-darwin` |
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` |

### Verify (optional)

```bash
gh release download v0.2.0 --repo neko-neko/belt --pattern '*.tar.xz'
gh attestation verify belt-x86_64-unknown-linux-gnu.tar.xz --repo neko-neko/belt
gh attestation verify belt-agent-x86_64-unknown-linux-gnu.tar.xz --repo neko-neko/belt
```

### From source

```bash
git clone https://github.com/neko-neko/belt.git && cd belt
cargo build --release --workspace
```

Build only what you need:

```bash
cargo build -p belt          # lint tool
cargo build -p belt-agent    # agent runtime
```

## Claude Code Plugins (Working Examples)

belt ships 2 Claude Code plugins under `plugins/` — working examples and
production tooling for quality-gated AI-driven development.

### Plugins in this repo

| Plugin | Purpose |
|---|---|
| `belt-agent` | Foundation: Belt Protocol driver skill + 5 analysis agents (phase-auditor, feature-implementer, code-explorer, code-architect, impact-analyzer) + shared references |
| `belt` | User-invocable pipelines and reviewer agents: `/belt:feature-dev`, `/belt:bug-fix`, `/belt:goal`, `/belt:design`, `/belt:build`, `/belt:verify`, `/belt:code-review` (2 reviewers), `/belt:spec-review` (1 reviewer), `/belt:handover`, `/belt:resume`. Requires `belt-agent` |

### External skill dependencies

The belt skills invoke skills from other plugins. `/belt:verify` requires
the `agent-browser` CLI. Install these before the belt plugins that use them:

| Dependency | Source | Required by |
|---|---|---|
| `/writing-plans` | [obra/superpowers](https://github.com/obra/superpowers) | `/belt:bug-fix` `fix-plan` phase |
| `/systematic-debugging` | obra/superpowers | `/belt:bug-fix` `rca` phase |
| `/worktrunk` | [max-sixty/worktrunk](https://github.com/max-sixty/worktrunk) | `/belt:feature-dev` `integrate` phase, `/belt:bug-fix` `integrate` phase |
| `agent-browser` CLI + `/agent-browser` skill | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | `/belt:verify` always, `/belt:feature-dev` / `/belt:bug-fix` `e2e` phase (when `--e2e`) |

### Install

belt plugins are distributed via the Claude Code plugin marketplace. Install
external plugin dependencies first, then add belt as a marketplace and install
the two belt plugins.

```
# In Claude Code:

# 1. Add external plugin dependencies
/install-plugin obra/superpowers-marketplace superpowers
/install-plugin max-sixty/worktrunk worktrunk
/install-plugin vercel-labs/agent-browser agent-browser

# 2. Add belt marketplace and install both belt plugins
/install-plugin neko-neko/belt belt-agent
/install-plugin neko-neko/belt belt
```

`belt` requires `belt-agent`; install `belt-agent` first. Plugin discovery
uses `.claude-plugin/marketplace.json` (Claude Code marketplace format) at
belt repo root.

### Usage

After install:

```
# Start a pipeline (feature-dev accepts a Linear id, URL, or free-text task)
/belt:feature-dev CLA-42 --e2e
/belt:bug-fix

# Run a single stage standalone
/belt:goal
/belt:design
/belt:build
/belt:verify

# Run a review standalone
/belt:code-review
/belt:spec-review

# Pause & resume an in-progress run
/belt:handover
/belt:resume
```

See each skill's `SKILL.md` (under `plugins/belt/skills/<skill>/`) for phase
details and arg reference. Skill tool invocations inside skills, agents,
and pipeline files are always written fully-qualified (`/belt:code-review`,
`belt-agent:phase-auditor`) — shorthand (`/code-review`) is not used.

## License

MIT

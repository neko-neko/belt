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

## License

MIT

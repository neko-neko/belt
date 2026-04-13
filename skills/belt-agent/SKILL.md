---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, invoke/artifact/validate semantics, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving `belt-agent` CLI — a deterministic state machine for pipeline execution.

## Commands

```bash
# Start a new run
belt-agent init <pipeline.yml>
belt-agent init <pipeline.yml> --arg smoke=true --arg team=BELT

# Get current phase info (or completion signal)
belt-agent next
belt-agent next --run <run_id>

# Run gate checks for current phase
belt-agent verify
belt-agent verify --run <run_id>

# Run regate checks for target phases
belt-agent regate
belt-agent regate --run <run_id>

# Advance to next phase
belt-agent step
belt-agent step --confirm
belt-agent step --run <run_id>

# Inspect full run state (enriched view)
belt-agent status
belt-agent status --run <run_id>
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → read phase.invoke → execute per variant →
verify (if gates) → regate (if targets) → step → next → ... → completed
```

## Reading `phase.invoke`

Every phase returned by `next` may carry an `invoke` field with one of four variants. Read the variant and take the matching action.

| Variant | Shape | Orchestrator action |
|---------|-------|---------------------|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as parameters. |
| `agent` | `{ agent: "name", args: { ... } }` | Dispatch a single subagent via the `Agent` tool with `subagent_type: <name>`. Pass `invoke.args` as context. |
| `agents` | `{ agents: ["a", "b", ...], iterations: N, args: { ... } }` | Dispatch each named subagent in parallel. If `iterations > 1`, run N rounds for voting. `invoke.args` carries run-time qualifiers (`ui_agent`, `codex`, `swarm`, etc.). |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |

If `invoke` is absent, the phase is a "pure checkpoint" with only `gate:`, `validate:`, or `confirm:`. Proceed directly to the verify/step loop.

## Artifact graph in `status`

`belt-agent status` returns each phase's `produces` and `consumes` as part of the enriched view.

`produces` is a list of resolved artifacts. Each entry has:

```json
{
  "name": "design_doc",
  "path": "docs/plans/*-design.md",
  "description": "Brainstormed design...",
  "exists": true,
  "resolved_path": "docs/plans/2026-04-11-feature-x-design.md"
}
```

`belt-core` resolves glob paths using the phase-start mtime filter: the matching file with the newest mtime (greater than or equal to the phase's entry timestamp) is chosen, ties broken lexicographically. For concrete paths, `exists` is a direct `std::fs::metadata` check. Use `exists: false` and `resolved_path: null` as a signal that the declared artifact is missing.

`consumes` is a list of artifact references. Each entry is either:

- A string (short form): the artifact name, resolved by lint against the most recent earlier phase that produced that name.
- An object: `{ "name": "design_doc", "from": "design" }` — explicit disambiguation when multiple earlier phases produce the same name.

Use the status output's artifact graph when you need to locate the concrete path of a prior phase's output during the current phase.

**Note: `next` and `init` emit declared artifacts, not resolved.** The `produces` array in `next`/`init` carries raw `{ name, path, description }` entries from pipeline.yml — without `exists` or `resolved_path`. Filesystem resolution (the mtime filter, glob matching) only happens in `status`. Call `belt-agent status` whenever you need the concrete path of a prior phase's output.

## Validate file semantics

Phases may use either:

- `validate: ./criteria/name.md` (scalar file reference) — read the file at that path (relative to the pipeline.yml directory) and judge the criteria defined inside it.
- `validate: /abs/path.md` — same, absolute path.
- `validate: ["criterion one", "criterion two"]` (list of inline strings) — judge each string directly.
- `validate: [{ file: "./x.md" }, "inline"]` (mixed list) — combine.

When a validate entry is a file reference, the orchestrator MUST read the file before running `step --confirm`. The file contains the actual criteria; the scalar/struct in `pipeline.yml` is just the pointer. See `examples/references/audit-protocol.md` for the expected criteria file format.

## Decision Rules

| Situation | Action |
|-----------|--------|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. Each verify invocation counts toward `max_retries`. |
| Phase has `regate` targets | After `verify` PASS, run `regate`. On FAIL, fix target phases and re-run `verify` then `regate`. |
| Phase has no `regate` targets | Skip `regate`. Go directly to `step`. |
| Phase has `validate` criteria | Verify each criterion yourself (read file if file-ref; read list if inline), then `step --confirm`. |
| `phase_attempts[phase] > max_retries` | `step` fails with `max_retries_exceeded`. Escalate per pipeline's `on_escalation` policy (BELT-28). |

Every call to `verify` increments the current phase's attempts counter regardless of verdict. `regate` is an in-place re-verification of earlier phases' gates; it does not modify any phase's attempts counter. Earlier phases' counters are never touched by operations at the current phase.

## Step Troubleshooting

When `step` returns `advanced: false`, read the `reason` field:

| `reason` | Action |
|----------|--------|
| `confirmation_required` | Phase has `validate` or `confirm`. Verify criteria, then `step --confirm`. |
| `verify_required` | Run `verify` first. |
| `regate_not_executed` | Run `regate` first. |
| `regate_failed` | Fix regate target phases. Re-run `verify` then `regate`. |
| `max_retries_exceeded` | Escalate. Pipeline author defines recovery via `on_escalation`. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML, and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    {
      "id": "build",
      "status": "completed",
      "verify_passed": true,
      "attempt": 1,
      "invoke": { "skill": "/brainstorming", "args": { "swarm": false } },
      "produces": [
        { "name": "design_doc", "path": "docs/plans/*-design.md", "exists": true, "resolved_path": "docs/plans/2026-04-11-feature-x-design.md" }
      ],
      "consumes": [],
      "outputs": ["report.json"]
    },
    {
      "id": "review",
      "status": "current",
      "verify_passed": false,
      "attempt": 2,
      "invoke": { "pipeline": "../spec-review/pipeline.yml", "with": {} },
      "produces": [],
      "consumes": ["design_doc"],
      "outputs": []
    }
  ]
}
```

Note: `produces`, `consumes`, and `invoke` are omitted from `status` JSON when empty/absent. Treat absence as equivalent to an empty array (or `null` for `invoke`).

Use `status` for context recovery or progress checks.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation. It is retained for phase-specific flags that are orthogonal to invocation identity.

Phase-level invocation identity moved to the typed `invoke:` field; do not use `config.skill`, `config.agents`, `config.criteria`, `config.audit`, or `config.reference` — these have been replaced by `invoke:`, `produces:`, `consumes:`, and file-reference `validate:` respectively.

Remaining `config` keys are skill-specific flags (e.g., `codex: true`, `ui: true`, pipeline-specific arguments). Unknown keys MAY be ignored.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase (inline or file-reference), you
MUST NOT run `belt-agent step --confirm` without verifying each criterion.

For inline `validate: ["..."]` criteria, judge each string directly.

For file-reference `validate: ./criteria/name.md` or `validate: /abs/path.md`,
you MUST Read the referenced file first, then judge each criterion defined
inside that file. The file is the authoritative source; the scalar in
pipeline.yml is just the pointer.

The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>

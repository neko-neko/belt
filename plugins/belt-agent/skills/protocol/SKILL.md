---
name: protocol
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, invoke/artifact/validate semantics, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving `belt-agent` CLI — a deterministic state
machine for pipeline execution.

## Commands

```bash
belt-agent init   <pipeline.yml> [--arg key=value ...]  # Start a new run
belt-agent next   [--run <id>]                          # Get current phase info (or completion signal)
belt-agent verify [--run <id>]                          # Run gate checks for current phase
belt-agent regate [--run <id>]                          # Run regate checks for target phases
belt-agent step   [--confirm] [--run <id>]              # Advance to next phase
belt-agent status [--run <id>]                          # Inspect full run state (enriched)
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → read phase.invoke → execute per variant →
verify (if gates) → regate (if targets) → step → next → ... → completed
```

## Reading `phase.invoke`

Every phase returned by `next` may carry an `invoke` field with one of two
variants. Read the variant and take the matching action.

| Variant | Shape | Orchestrator action |
|---|---|---|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as parameters. |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |

**`pipeline` invoke — `with` template resolution.** When a `with` entry's
value is a string of the form `"args.X"` (literal prefix `args.` followed by
a single arg identifier — no nested dotted paths), resolve it against the
parent run's `args` before calling `belt-agent init --arg X=<value>`. Literal
values (bool, number, non-template string) are passed through verbatim. If
`args.X` is absent in the parent, omit the `--arg` instead of passing `null`;
the sub-pipeline's declared default applies.

If `invoke` is absent, the phase is a "pure checkpoint" with only `gate:`,
`validate:`, or `confirm:`. Proceed directly to the verify/step loop.

## Artifact Graph in `status`

`belt-agent status` returns each phase's `produces` and `consumes` as part of
the enriched view.

`produces` entries are resolved artifacts:

```json
{
  "name": "design_doc",
  "path": "docs/plans/*-design.md",
  "description": "Brainstormed design...",
  "exists": true,
  "resolved_path": "docs/plans/2026-04-11-feature-x-design.md"
}
```

`belt-core` resolves glob paths using the phase-start mtime filter: the
matching file with the newest mtime (>= phase entry timestamp) wins, ties
broken lexicographically. For concrete paths, `exists` is a direct
`std::fs::metadata` check. The `resolved_path` field is omitted from JSON
when unresolved.

`consumes` entries are artifact references — either a string (resolved by
lint against the most recent earlier phase producing that name) or
`{ "name": "...", "from": "..." }` for explicit disambiguation.

**`next` and `init` emit declared artifacts, not resolved.** The `produces`
array in `next`/`init` carries raw `{ name, path, description }` entries from
pipeline.yml — without `exists` or `resolved_path`. Filesystem resolution
only happens in `status`. Call `belt-agent status` whenever you need the
concrete path of a prior phase's output.

## Validate File Semantics

Phases may use either:

- `validate: ./criteria/name.md` (scalar file reference, relative to pipeline.yml directory)
- `validate: /abs/path.md` (absolute path)
- `validate: ["criterion one", "criterion two"]` (inline list)
- `validate: [{ file: "./x.md" }, "inline"]` (mixed)

When a validate entry is a file reference, the orchestrator MUST read the
file before `step --confirm`. The file contains the actual criteria; the
scalar in pipeline.yml is just the pointer. See
`plugins/belt-agent/references/audit-protocol.md` for the expected
criteria file format.

## Decision Rules

| Situation | Action |
|---|---|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. Each verify invocation counts toward `max_retries`. |
| Phase has `regate` targets | After `verify` PASS, run `regate`. On FAIL, fix target phases and re-run `verify` then `regate`. |
| Phase has no `regate` targets | Skip `regate`. Go directly to `step`. |
| Phase has `validate` criteria | Verify each criterion yourself (file-ref: read file; inline: judge strings), then `step --confirm`. |
| `phase_attempts[phase] > max_retries` | `step` fails with `max_retries_exceeded`. Escalate per pipeline's `on_escalation` policy. |

Every call to `verify` increments the current phase's attempts counter
regardless of verdict. `regate` is an in-place re-verification of earlier
phases' gates; it does not modify any phase's attempts counter.

## Step Troubleshooting

When `step` returns `advanced: false`, read the `reason` field:

| `reason` | Action |
|---|---|
| `confirmation_required` | Phase has `validate` or `confirm`. Verify criteria, then `step --confirm`. |
| `verify_required` | Run `verify` first. |
| `regate_not_executed` | Run `regate` first. |
| `regate_failed` | Fix regate target phases. Re-run `verify` then `regate`. |
| `max_retries_exceeded` | Escalate. Pipeline author defines recovery via `on_escalation`. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML,
and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    {
      "id": "build",
      "status": "completed",
      "invoke": { "skill": "/brainstorming" },
      "produces": [{ "name": "design_doc", "exists": true, "resolved_path": "docs/plans/2026-04-11-feature-x-design.md" }],
      "consumes": [],
      "outputs": ["report.json"]
    },
    {
      "id": "review",
      "status": "current",
      "invoke": { "pipeline": "./nested-pipeline.yml", "with": {} },
      "consumes": ["design_doc"]
    }
  ]
}
```

`produces`, `consumes`, and `invoke` are omitted when empty/absent. Treat
absence as equivalent to an empty array (or `null` for `invoke`). Use
`status` for context recovery or progress checks.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.
Use it for phase-specific flags orthogonal to invocation identity (e.g.,
`codex: true`, `ui: true`, or pipeline-specific arguments). Unknown keys
MAY be ignored.

Phase-level invocation identity belongs in the typed `invoke:` field. Agent
dispatch and iteration loops are skill-layer concerns; `pipeline.yml`
references only `invoke.skill` or `invoke.pipeline`.

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

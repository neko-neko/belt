# Rename `debug-flow` to `bug-fix`

**Status**: Approved
**Date**: 2026-04-15

## Summary

Rename the `examples/skills/debug-flow/` skill to `examples/skills/bug-fix/`
and update all live references in the skill directory and belt-core / belt-agent
test crates. Historical spec / plan documents are intentionally excluded — they
record decisions made under the former name and must remain immutable.

## Scope

| Layer | Included | Rationale |
|-------|----------|-----------|
| A. Skill directory contents (`examples/skills/debug-flow/**`) | ✅ | Primary target |
| B. Live code references (`crates/belt-core/src/**`, `crates/belt-core/tests/**`, `crates/belt-agent/tests/**`) | ✅ | Must track the renamed entity |
| C. Historical spec / plan documents under `docs/specs/` and `docs/plans/` | ❌ | Dated records of past work; references would become dangling if renamed |
| `.claude/agent-memory/**` | ❌ | Retained knowledge snapshots; not part of this pass |

## Rename Map

| Source | Target | Context |
|--------|--------|---------|
| `debug-flow` | `bug-fix` | kebab-case — directory name, YAML pipeline `name:`, skill identifier |
| `debug_flow` | `bug_fix` | snake_case — Rust function / variable names, test file names |
| `Debug Flow` | `Bug Fix` | Title Case — document headings, prose |
| `/debug-flow` | `/bug-fix` | slash-command references in comments and docs |

## Operations

### A. Directory rename

```
examples/skills/debug-flow/  →  examples/skills/bug-fix/
```

### A-in. In-directory string replacement

Files containing references (16 occurrences across 8 files, apply all four mappings above):

- `SKILL.md` (3)
- `pipeline.yml` (1)
- `references/path-convention.md` (3)
- `references/worktrunk-supplement.md` (3)
- `references/rca-supplement.md` (2)
- `references/monkey-test-supplement.md` (2)
- `references/dogfood-supplement.md` (1)
- `references/fix-plan-supplement.md` (1)

`belt.toml` and `criteria/*.md` contain no references — excluded from this step.

### B. Live-code replacement

- `crates/belt-core/src/view.rs` — doc comment mentions
- `crates/belt-agent/tests/e2e_test.rs` — inline YAML fixture `name: debug-flow`
- `crates/belt-core/tests/debug_flow_refresh.rs`:
  - File rename: `debug_flow_refresh.rs` → `bug_fix_refresh.rs`
  - Function renames: `debug_flow_dir`, `debug_flow_pipeline_path`, `debug_flow_pipeline` → `bug_fix_dir`, `bug_fix_pipeline_path`, `bug_fix_pipeline`
  - String literals inside tests referencing `examples/skills/debug-flow` → `examples/skills/bug-fix`

## Exception Rule: Preserve Historical References

References that point to dated spec / plan filenames (e.g.
`2026-04-15-debug-flow-refresh-design.md`) MUST be preserved verbatim, even when
they appear inside live-code files in layer B. This includes:

- `crates/belt-core/src/model.rs` line 183 — `see spec 2026-04-15-debug-flow-refresh-design.md`
- `crates/belt-core/tests/artifact_when_field.rs` line 3 — same pattern
- `crates/belt-core/src/view.rs` line 236 — `Grammar (MVP, debug-flow refresh spec, 2026-04-15)` — this phrase names a historical spec by its original title; leave as-is

Rule: if the string refers to a dated spec / plan title or filename under
`docs/specs/` or `docs/plans/`, do not rewrite it.

## Verification

1. `cargo test -p belt-core --test bug_fix_refresh` passes
2. `cargo test -p belt-agent --test e2e_test` passes
3. `cargo clippy --workspace -- -D warnings` passes
4. `cd examples/skills/bug-fix && belt-agent init` parses successfully
5. Residual-string scan: within scope A + B, no occurrences of
   `debug-flow`, `debug_flow`, `Debug Flow`, `/debug-flow` remain
   other than those covered by the Exception Rule

## Non-Goals

- Renaming historical specs / plans in `docs/specs/` or `docs/plans/`
- Renaming the global dotfiles skill `~/.claude/skills/debug-flow`
  (outside this repo)
- Updating agent-memory records under `.claude/agent-memory/`

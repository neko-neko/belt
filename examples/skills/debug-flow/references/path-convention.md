# Debug Flow Path Convention

**Purpose:** SSOT for file naming and directory layout under `docs/plans/` for a debug-flow run.

## Base path

All debug-flow run outputs live under:

```
docs/plans/YYYY-MM-DD-<topic>-*
```

## Topic slug rules

- Characters: `[a-z0-9-]` (lowercase, digits, hyphens only)
- Length: 3–48 characters
- Separator: hyphen `-`
- Collision handling: on same-day topic collision, append numeric suffix `-N` (e.g., `2026-04-20-login-bug-2`)

## Branch name convention

- Format: `bugfix/<YYYY-MM-DD-topic>`
- Same topic slug as used in the paths
- Branch must be a worktree-linked branch per `worktrunk` conventions

## Artifact path table

| Artifact (logical name) | Path |
|---|---|
| `rca_report` | `docs/plans/YYYY-MM-DD-<topic>-rca-report.md` |
| `rca_scenarios` (when `--e2e`) | `docs/plans/YYYY-MM-DD-<topic>-rca-scenarios.yml` |
| `fix_plan_doc` | `docs/plans/YYYY-MM-DD-<topic>-fix-plan.md` |
| `monkey_test_report` | `docs/plans/YYYY-MM-DD-<topic>-monkey-test-report.md` |
| `monkey_test_results` | `docs/plans/YYYY-MM-DD-<topic>-monkey-test-results.json` |
| `dogfood_report` | `docs/plans/YYYY-MM-DD-<topic>-dogfood-report/report.md` (directory form) |
| `dogfood_screenshots` | `docs/plans/YYYY-MM-DD-<topic>-dogfood-report/screenshots/` |
| `dogfood_videos` | `docs/plans/YYYY-MM-DD-<topic>-dogfood-report/videos/` |

## Glob resolution

belt-agent glob resolution for `docs/plans/*-<suffix>` patterns returns files matching the glob; on ambiguity (multiple matches), the most recently modified file wins (mtime DESC). `monkey-test-supplement.md` documents this for scenarios resolution.

# Dogfood Supplement (Phase 7 override for `/dogfood`)

**Invoked by:** `SKILL.md` Phase 7 (INVOKE 1 = Read this file; INVOKE 2 = `/dogfood`). Only runs when `args.e2e=true`.

## Output path override

`/dogfood` default output is `./dogfood-output/`. Override to:

```
docs/plans/YYYY-MM-DD-<topic>-dogfood-report/
  ├── report.md
  ├── screenshots/
  └── videos/
```

Path convention: see `./path-convention.md`.

## Exploration scope

Prioritize:

1. **Impact Scope areas** from `fix_plan_doc` — specifically the files and modules modified in the fix
2. **Symmetry pairs** from RCA-08 — paired paths that may exhibit the same mechanism
3. **Root Cause mechanism re-emergence** — verify the specific mechanism described in the RCA `## Root Cause` section does not recur in adjacent code paths

## Priority: Root Cause mechanism re-verification

Include in the report an explicit statement of the form:

> After the fix, the <mechanism description from RCA Root Cause> condition no longer triggers. Verified by: <specific exploration path / inputs>.

This satisfies `criteria/dogfood.md` DOGFOOD-02.

## CLI-only graceful degradation (UI-free bug fix)

When the Impact Scope contains **zero UI files** (CLI / API / backend-only fix):

1. Substitute visual exploration with:
   - CLI output capture (stdout / stderr)
   - API response inspection (JSON / headers)
   - Log file inspection
   - DB state queries
2. DOGFOOD-04 evidence requirement is satisfied by a **rationale paragraph** in `report.md`:

   > Impact Scope contains no UI files (<list affected paths>). Exploration is CLI-only; evidence is captured as CLI output / API response / log excerpts in this report.

3. Still produce the `screenshots/` and `videos/` directories (empty is acceptable) to keep the artifact structure consistent with DOGFOOD-01.

## Minimum report content

- Fix Impact Scope listing (copy from `fix_plan_doc`)
- Exploration coverage map (which Impact Scope items + Symmetry pairs were explored)
- Root Cause mechanism re-verification statement
- Evidence index (screenshots/videos listing, OR CLI-only rationale paragraph)
- Issue summary (new issues discovered, if any, with severity)

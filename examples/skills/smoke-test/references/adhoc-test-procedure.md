# adhoc-test Procedure

Generate and execute ad-hoc smoke test scenarios via browser.

## Procedure

1. Collect diff: `git diff <args.diff_base>...HEAD`
2. Read [scenario-generation.md](scenario-generation.md) to generate scenarios:
   - Start with 5 base perspectives.
   - If `args.design` is set, expand from design doc.
   - If `args.perspectives` is set, dispatch review agents for additional perspectives.
3. Execute each scenario via browser (reconnaissance-then-action pattern).
4. Take a screenshot after each scenario: `smoke-<scenario_name>.png`
5. On scenario failure, retry up to 2 times before marking FAIL.
6. Write report per [report-template.md](report-template.md).

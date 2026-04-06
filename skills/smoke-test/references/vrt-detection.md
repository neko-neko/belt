# VRT Detection

Detect Visual Regression Testing tooling and run checks.

## Detection Table

Check in order. Use the first match.

| Tool | Condition | Command |
|------|-----------|---------|
| Playwright snapshots | `playwright.config.*` exists AND uses `toMatchSnapshot` | `npx playwright test --grep snapshot` |
| reg-suit | `.reg/` directory or `regconfig.json` exists | `npx reg-suit run` |
| Storycap + reg-suit | `storycap` in devDependencies | `npx storycap && npx reg-suit run` |
| Loki | `loki` in devDependencies or `.lokirc` exists | `npx loki test` |
| Percy | `@percy/cli` in devDependencies | Skip (CI-only tool) |

If no VRT tooling detected → skip this phase entirely. No action needed.

## Diff Handling

When VRT detects visual differences:

1. Identify which diff images were produced (tool-specific output directory).
2. Present diff images to the user using the Read tool.
3. Ask user for decision:
   - **Approve**: Update baseline images and commit:
     ```
     git add <baseline-directory>
     git commit -m "test: update VRT baselines"
     ```
   - **Reject**: Record in report only. Do not update baselines.
4. Record the outcome in the VRT section of the smoke test report.

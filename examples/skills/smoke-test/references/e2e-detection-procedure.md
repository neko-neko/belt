# e2e-detection Procedure

Run E2E test suite with flaky detection (2-pass execution).

## Procedure

1. Read [e2e-flaky-detection.md](e2e-flaky-detection.md) to detect E2E suite.
2. If no E2E suite detected → skip this phase (no action needed).
3. Determine test scope: `args.full_e2e` → all tests, otherwise changed-files-only.
4. Execute tests twice (2-pass flaky detection).
5. Classify results: stable pass / implementation failure / flaky.
6. Flaky tests → PASS with report note (do not block).
7. Implementation failures → FAIL with fix suggestions.

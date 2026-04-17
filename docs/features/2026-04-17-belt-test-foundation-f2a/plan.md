# belt-test-foundation F2a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** belt-core 10 module の scenarios を M2 normalize で列挙、behavior test に X2 policy で doc-comment を付与、復帰 2 scenario + strip fix + audit-template patch を含めて F1 の 3 層構造を belt-core 全域に拡張する。

**Architecture:** feature-dev pipeline (W1) を使い、Phase A (9 module parallel subagent dispatch、controller が belt-core.yml merge)、Phase B (sequential: B2 strip fix / B3 uri_test.rs / B4-B5 復帰 / B6 audit-template patch)、Phase C (audit-report + README) の 3 phase で 15 commits に分割。scenarios_contract.rs の symmetric diff CI (HashSet-based、既に set-based) が各 commit の atomicity を自動 guard。

**Tech Stack:** Rust 1.94.1+ / Edition 2024 / Cargo workspace / serde-saphyr =0.0.23 / miette 7.6 / toml / filetime / feature-dev pipeline (belt plugin)

---

## Pre-flight Findings (plan 執筆時に確認済、実装では skip 可)

実装開始前に、以下は plan 執筆時の事前調査で確認済。execute 時に再確認は不要だが、前提条件として記録:

- **scenarios_contract.rs line 18-88**: `HashSet<String>` + `difference()` で symmetric diff 実装済 → Phase B1 (set-based 化) は skip。同一 scenario ID を複数 Rust test の doc-comment で参照しても orphan 検出対象外 (HashSet が自動 dedup)
- **scenarios_contract.rs line 127-156 `strip_string_literals`**: single-line + limited escape (`\\"` のみ) しか対応せず、raw string `r#"..."#` 非対応。multiline string 内 `/// scenario:` が strip されず false-positive を生む → Phase B2 で fix 必須
- **scenarios_contract.rs line 175-179**: walk scope = `crates/{belt,belt-agent,belt-core}/tests/` の `.rs` ファイルのみ。**`src/**` 以下の inline `#[cfg(test)] mod tests` は検出されない**
- **belt-core/src/uri.rs line 174-310**: inline unit tests 13 個が存在するが、上記の通り walk scope 外。F2a で uri_test.rs integration test 新規作成が必要 (inline tests との duplication 判定は F2b 送り、audit-report.md の Forward-to-F2b list に記録)
- **belt-core/src/config.rs line 35-38 `resolve_pipeline_path`**: `base_dir.join(&config.pipeline)`。Rust `Path::join` は絶対パスを与えると右側がそのまま返る (std ドキュメント guarantee) → 復帰 scenario `preserves_absolute_pipeline_path` は現状実装で pass、production code touch 不要
- **expander_with_test.rs (17 行)**: 0 test + historical explanation コメント (2026-04-16 に Invoker::Agents と共に削除された旨)。F2a では touch しない
- **belt.yml 既存 5 scenarios + belt-core.yml 既存 6 scenarios** (F1 時点)
- **13 behavior test file の tests count** (F2a baseline): engine 67 / view 41 / model 39 / lint 29 / gate 22 / error 6 / expander 5 / parser 4 / artifact_when_field 5 / uri (inline) 13 / (config 6, cli_test.rs 5 は F1 で pilot 済)

---

## Task 0: Worktree Setup + Baseline Verification

**Files:**
- (no file create/modify; environment setup only)

- [ ] **Step 1: Create F2a worktree via worktrunk**

Run:
```bash
wt switch --create feature/2026-04-17-belt-test-foundation-f2a
```

Expected: new worktree at `.claude/worktrees/f2a` or similar, branch switched.

- [ ] **Step 2: Verify baseline tests**

Run:
```bash
cargo test --workspace 2>&1 | tail -20
```

Expected output contains: `test result: ok. 397 passed; 0 failed` (or close, depending on F1 final count).

- [ ] **Step 3: Verify clippy baseline**

Run:
```bash
cargo clippy --workspace -- -D warnings 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Verify fmt baseline**

Run:
```bash
cargo fmt --all -- --check
```

Expected: no output (= clean), exit 0.

- [ ] **Step 5: Confirm pilot files unchanged since F1 (MV-34)**

Run:
```bash
git log --since="2026-04-17T00:00:00Z" --oneline -- \
  crates/belt/tests/cli_test.rs \
  crates/belt-core/tests/config_test.rs \
  crates/belt-core/tests/feature_dev_refresh.rs
```

Expected: empty output (pilot file re-audit trigger 非発動を確認)。

- [ ] **Step 6: Commit nothing (setup task)**

No commit created. Proceed to Task 1.

---

## Task 1: strip_string_literals Multiline Raw String Fix (Phase B2)

**Files:**
- Modify: `crates/belt-core/tests/scenarios_contract.rs:127-156` (replace `strip_string_literals`) + add new drift tests at end of file
- Test: `crates/belt-core/tests/scenarios_contract.rs` (self-test via added drift tests)

- [ ] **Step 1: Write failing drift test for multiline raw string (appended to file end)**

Add the following test functions at the end of `crates/belt-core/tests/scenarios_contract.rs`:

```rust
#[test]
fn drift_multiline_raw_string_is_stripped() {
    let src = r###"
let s = r#"
    /// scenario: belt-core-multiline-raw-false-positive
    some content
"#;
"###;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "multiline raw string containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_raw_string_with_hash_is_stripped() {
    let src = r####"
let s = r##"
    /// scenario: belt-core-raw-hash-false-positive
"##;
"####;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "raw string with hashes containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_doc_comment_outside_string_still_matches_after_fix() {
    let src = r###"
/// scenario: belt-core-positive-outside-string
fn test_fn() {
    let _s = r#"
        /// scenario: belt-core-inside-string-false-positive
    "#;
}
"###;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let matched: Vec<_> = stripped.lines().filter_map(match_scenario_line).collect();
    assert_eq!(
        matched.as_slice(),
        &["belt-core-positive-outside-string"],
        "fix must preserve doc-comment match outside raw strings (false-negative check)"
    );
}
```

- [ ] **Step 2: Run the new drift tests, verify they fail**

Run:
```bash
cargo test -p belt-core --test scenarios_contract -- drift_multiline_raw_string_is_stripped drift_raw_string_with_hash_is_stripped drift_doc_comment_outside_string_still_matches_after_fix 2>&1 | tail -15
```

Expected: 2 of 3 tests FAIL (multiline raw + raw with hash are currently false-positives; `drift_doc_comment_outside_string_still_matches_after_fix` may pass coincidentally since the outer `///` is on a separate line). Confirm the FAIL message reports line mismatches proving the current `strip_string_literals` fails on raw strings.

- [ ] **Step 3: Replace `strip_string_literals` with raw-string-aware implementation**

Replace the function body at `crates/belt-core/tests/scenarios_contract.rs:127-156` with:

```rust
/// Strip string literals (regular `"..."` and raw strings `r"..."` / `r#"..."#` /
/// `r##"..."##` etc) from Rust source. Replaces string contents with spaces,
/// preserving line numbers.
///
/// Supports:
/// - Regular strings with `\"` / `\\` escape handling (single-line only — strings
///   are terminated at `\n` as a safety net).
/// - Raw strings `r"..."`, `r#"..."#`, `r##"..."##`, etc, including multiline.
///   Closing tag matches the exact hash count recorded at the opening.
fn strip_string_literals(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];

        // Raw string: r"..." / r#"..."# / r##"..."## ...
        if b == b'r' {
            let mut hashes = 0;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                // Emit 'r' + hashes + '"' as spaces (preserve line breaks via newline detection below)
                for _ in 0..=(hashes + 1) {
                    out.push(' ');
                }
                let content_start = j + 1;
                let mut k = content_start;
                // Close tag: `"` followed by the same number of `#`
                let close_needed = hashes;
                let mut found_close_at = None;
                while k < bytes.len() {
                    if bytes[k] == b'"' {
                        let mut m = k + 1;
                        let mut count = 0;
                        while m < bytes.len() && bytes[m] == b'#' && count < close_needed {
                            count += 1;
                            m += 1;
                        }
                        if count == close_needed {
                            found_close_at = Some((k, m));
                            break;
                        }
                    }
                    k += 1;
                }
                let Some((close_quote, close_end)) = found_close_at else {
                    // Unterminated raw string: rest of file is inside string.
                    // Safety: emit spaces/newlines to end to avoid infinite loop.
                    for byte_in_tail in &bytes[content_start..] {
                        if *byte_in_tail == b'\n' {
                            out.push('\n');
                        } else {
                            out.push(' ');
                        }
                    }
                    return out;
                };
                for byte_in_content in &bytes[content_start..close_quote] {
                    if *byte_in_content == b'\n' {
                        out.push('\n');
                    } else {
                        out.push(' ');
                    }
                }
                // Emit closing `"` + hashes as spaces
                for _ in close_quote..close_end {
                    out.push(' ');
                }
                i = close_end;
                continue;
            }
        }

        // Regular string: "..."
        if b == b'"' {
            out.push(' ');
            let mut j = i + 1;
            let mut prev_escape = false;
            while j < bytes.len() {
                let c = bytes[j];
                if c == b'"' && !prev_escape {
                    out.push(' ');
                    j += 1;
                    break;
                }
                if c == b'\n' {
                    // Safety: regular strings should not span lines in test sources.
                    out.push('\n');
                    j += 1;
                    break;
                }
                out.push(' ');
                prev_escape = c == b'\\' && !prev_escape;
                j += 1;
            }
            i = j;
            continue;
        }

        // Default: copy byte as char (ASCII assumption for test sources)
        out.push(b as char);
        i += 1;
    }
    out
}
```

- [ ] **Step 4: Run all scenarios_contract tests, verify pass**

Run:
```bash
cargo test -p belt-core --test scenarios_contract 2>&1 | tail -20
```

Expected: All tests pass (10 existing + 3 new = 13 tests), `test result: ok. 13 passed`.

- [ ] **Step 5: Run full workspace test, verify no regression**

Run:
```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: `test result: ok. N passed` where N = 397 + 3 (new drift tests) = 400.

- [ ] **Step 6: Run clippy and fmt**

Run:
```bash
cargo clippy -p belt-core --tests -- -D warnings && cargo fmt --all -- --check
```

Expected: Both clean.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/scenarios_contract.rs
git -c commit.gpgsign=false commit -m "test(belt-core): fix strip_string_literals for multiline raw strings

Support raw strings (r\"...\" / r#\"...\"# / r##\"...\"##) in
strip_string_literals so multiline raw string literals containing
/// scenario: text are correctly stripped (no false-positive match).

Add 3 drift tests:
- drift_multiline_raw_string_is_stripped
- drift_raw_string_with_hash_is_stripped
- drift_doc_comment_outside_string_still_matches_after_fix (false-negative guard)

Part of F2a; enables binding verification against module test files
that include YAML fixtures inside raw strings."
```

---

## Task 2: Phase A — engine module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/engine_test.rs` (67 tests, 2292 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (append engine section)
- Modify: `crates/belt-core/tests/engine_test.rs` (add `/// scenario: <id>` doc-comments)

- [ ] **Step 1: Dispatch subagent with the following verbatim prompt (subagent-driven-development)**

Subagent: `general-purpose`, model: inherit parent, description: "Normalize engine module to M2 scenarios".

Prompt:
```
You are normalizing belt-core engine module tests into behavior scenarios for docs/testing/cli-behavior/belt-core.yml (F2a, M2 normalize, X2 doc-comment policy).

Context:
- F2a design: /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/docs/features/2026-04-17-belt-test-foundation-f2a/design.md
- Existing belt-core.yml with config module 6 scenarios: /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/docs/testing/cli-behavior/belt-core.yml
- Rule: M2 normalize = 1 scenario ↔ N test; group tests verifying the same behavior into one scenario.
- Rule: X2 policy = attach /// scenario: <id> doc-comment to every test that maps to a scenario. Tests that verify no scenario-worthy behavior (implementation-coupling / trivial-default / redundant) stay unannotated and go to the Forward-to-F2b list.
- Scenario ID format: belt-core-engine-<kebab-case-describing-behavior>
- Category: engine
- Severity: high | medium | low (high = load-bearing behavior, low = cosmetic)
- Technique: equivalence-partition | boundary-value | state-transition | error-guessing | decision-table
- Target scenario count band: 15-20 (engine has 67 tests; aggregation ratio ~4:1)

Target file: /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/crates/belt-core/tests/engine_test.rs

Your responsibilities:
1. Read the target file in full.
2. For each test function, classify:
   - maps to existing or new scenario → list as (test_fn_name, scenario_id)
   - behavior-less / redundant → forward to F2b with a short reason label from the 9-label set (redundant-with-<id> / trivial-default-assertion / tautology / state-transition-overlap-with-<id> / implementation-coupling / brittle-format-match / dead-fixture / unreachable-guard / obsolete-spec)
3. Author the scenarios YAML block (15-20 scenarios, strictly within band).
4. Author the doc-comment patch (unified diff or explicit line insertions).
5. DO NOT write to belt-core.yml directly. DO NOT commit. DO NOT modify any other file.

Output format (exact):

## Scenarios (to append to belt-core.yml under `scenarios:` list)
```yaml
  - id: belt-core-engine-init-creates-run-directory
    category: engine
    severity: high
    technique: equivalence-partition
    given: "..."
    when: "..."
    then: "..."
  # ... (15-20 scenarios)
```

## Doc-comment patches (for crates/belt-core/tests/engine_test.rs)
For each test fn that maps to a scenario, specify:
- test fn name
- line number where doc-comment should be inserted (immediately above #[test])
- the exact `/// scenario: <id>` line to insert (indented to match surrounding code)

Provide this as a list, not a patch file, because line numbers shift after edits. Example:
- `init_creates_run_directory` @ line 42: insert `    /// scenario: belt-core-engine-init-creates-run-directory` above `#[test]`
- `regate_resets_verify_verdict` @ line 128: insert `    /// scenario: belt-core-engine-regate-resets-downstream-verify-verdict` above `#[test]`

## Forward-to-F2b candidates
- `test_fn_name_1` — reason: redundant-with-<other-fn-name>
- `test_fn_name_2` — reason: implementation-coupling (asserts private state)
- ...

Constraints:
- Scenario count must be in [15, 20].
- Every annotated test fn name must exist in engine_test.rs.
- Every scenario must be mapped to at least 1 test fn.
- Forward list is informational only (not implemented in F2a).
- Do not invent scenarios without corresponding test.
```

- [ ] **Step 2: Receive subagent output, validate**

Verify subagent output contains:
- exactly 15-20 scenario blocks under "## Scenarios"
- every scenario has id/category/severity/technique/given/when/then
- every doc-comment entry references a real line in engine_test.rs
- no direct file write occurred (subagent output only)

If validation fails, re-dispatch with corrected constraints.

- [ ] **Step 3: Append scenarios to belt-core.yml**

Open `docs/testing/cli-behavior/belt-core.yml`. Immediately after the existing config scenarios (preserving kebab-case comment separator `# --- engine module ---`), insert the scenarios block from subagent output.

Also update the top-level `scope:` string to reflect engine now fully enumerated:
```yaml
scope: "belt-core pure library public API. config (6, F1) + engine (N, F2a)... F2a で 10 module 列挙済の方向で段階追加中"
```
(After Task 10 completes, scope string will be finalized.)

- [ ] **Step 4: Apply doc-comment patches to engine_test.rs**

For each line-insertion entry from subagent output, insert the `/// scenario: <id>` line above the corresponding `#[test]` attribute. Use Edit tool with the `#[test]` line as anchor to avoid line-number drift.

Example Edit:
```
old_string: (indent)#[test]
            fn init_creates_run_directory() {
new_string: (indent)/// scenario: belt-core-engine-init-creates-run-directory
            (indent)#[test]
            fn init_creates_run_directory() {
```

Process all entries one by one.

- [ ] **Step 5: Run scenarios_contract**

Run:
```bash
cargo test -p belt-core --test scenarios_contract 2>&1 | tail -10
```

Expected: All pass (symmetric diff shows yml engine IDs == Rust engine_test.rs doc-comment IDs).

If orphan-yml or orphan-rust is reported, diagnose and fix before committing:
- orphan-yml → yml has ID but Rust doesn't → check doc-comment typo or missing insertion
- orphan-rust → Rust has ID but yml doesn't → check yml append or subagent output mismatch

- [ ] **Step 6: Run full workspace test**

Run:
```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: 400 passed (no engine_test behavior changes, only doc-comment additions).

- [ ] **Step 7: Run clippy and fmt**

Run:
```bash
cargo clippy -p belt-core --tests -- -D warnings && cargo fmt --all -- --check
```

Expected: Both clean. doc-comments should not introduce clippy warnings (they're treated as comments on fn items).

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/engine_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add engine scenarios + doc-comments (67 tests → N scenarios)

F2a Phase A/engine. M2 normalize aggregates 67 tests into N behavioral
scenarios covering Engine::{init,next,verify,step,regate,status,
latest_run_id} APIs. X2 policy attaches /// scenario: <id> to every
test mapping to a scenario; behavior-less tests are forwarded to F2b
audit (recorded in audit-report.md).

Verified via scenarios_contract.rs symmetric diff (yml ↔ Rust doc-comment
set equality)."
```

Replace `N` with the actual scenario count in the commit message.

- [ ] **Step 9: Record Forward-to-F2b list in Task 15 scratch**

Append subagent's Forward-to-F2b list to a temporary file `.belt/runs/{run_id}/artifacts/forward-to-f2b-engine.md` for aggregation in Task 15. If run_id unavailable, save to `/tmp/forward-to-f2b-engine.md`.

---

## Task 3: Phase A — view module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/view_test.rs` (41 tests, 986 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (append view section)
- Modify: `crates/belt-core/tests/view_test.rs`

Identical procedure to Task 2 with the following substitutions:
- Module: `view`
- File: `crates/belt-core/tests/view_test.rs`
- Test count: 41
- Target scenario count band: **10-15**
- Scenario ID prefix: `belt-core-view-<kebab>`
- Commit message module name: `view`

- [ ] **Step 1: Dispatch subagent**

Use the same subagent prompt as Task 2 Step 1, substituting:
- "engine" → "view"
- "engine_test.rs" → "view_test.rs"
- "15-20" → "10-15"
- "67 tests" → "41 tests"
- "~4:1" → "~3:1"

- [ ] **Step 2: Receive and validate subagent output**

Expect 10-15 scenarios, all mapped to view_test.rs test fns.

- [ ] **Step 3: Append scenarios to belt-core.yml under `# --- view module ---` separator**

- [ ] **Step 4: Apply doc-comment patches to view_test.rs**

- [ ] **Step 5: Run scenarios_contract, fix any orphans**

- [ ] **Step 6: Run full workspace test (expect 400 passed, unchanged)**

- [ ] **Step 7: Run clippy and fmt**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/view_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add view scenarios + doc-comments (41 tests → N scenarios)

F2a Phase A/view. M2 normalize aggregates 41 tests into N scenarios
covering status enrichment, output dir scan, YAML drift handling,
and COMPLETED→null transitions. X2 doc-comment policy applied."
```

- [ ] **Step 9: Save forward list to forward-to-f2b-view.md**

---

## Task 4: Phase A — lint module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/lint_test.rs` (29 tests, 917 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/lint_test.rs`

Identical procedure to Task 2 with:
- Module: `lint`
- File: `lint_test.rs`
- Test count: 29
- Target band: **10-15**
- Scenario ID prefix: `belt-core-lint-<kebab>`

- [ ] **Step 1: Dispatch subagent (substitute lint/29/10-15/~2:1)**

- [ ] **Step 2: Validate subagent output**

- [ ] **Step 3: Append to belt-core.yml under `# --- lint module ---`**

- [ ] **Step 4: Apply doc-comment patches**

- [ ] **Step 5: Run scenarios_contract**

- [ ] **Step 6: Run workspace test**

- [ ] **Step 7: clippy + fmt**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/lint_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add lint scenarios + doc-comments (29 tests → N scenarios)

F2a Phase A/lint. M2 normalize covers 7 static checks (duplicate IDs,
regate, args, description, uses references, expansion attempt) and
pipeline shape validation."
```

- [ ] **Step 9: forward-to-f2b-lint.md**

---

## Task 5: Phase A — model module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/model_test.rs` (39 tests, 982 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/model_test.rs`

- Module: `model`
- Test count: 39
- Target band: **15-20**
- Scenario ID prefix: `belt-core-model-<kebab>`

- [ ] **Step 1: Dispatch subagent (substitute model/39/15-20/~2:1)**

Special note in prompt: model tests heavily exercise serde untagged enum deserialization (GateCheck), ArgType, Invoker, ArtifactRef. Scenarios should capture round-trip serialization behavior and variant discrimination rules.

- [ ] **Step 2-8: Standard flow**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/model_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add model scenarios + doc-comments (39 tests → N scenarios)

F2a Phase A/model. M2 normalize covers serde types (Pipeline, Phase,
GateCheck untagged enum, ArgDef, Invoker, Artifact, RunState) round-trip
and variant discrimination."
```

- [ ] **Step 9: forward-to-f2b-model.md**

---

## Task 6: Phase A — gate module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/gate_test.rs` (22 tests, 393 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/gate_test.rs`

- Module: `gate`
- Test count: 22
- Target band: **5-10**
- Scenario ID prefix: `belt-core-gate-<kebab>`

- [ ] **Step 1: Dispatch subagent (substitute gate/22/5-10/~3:1)**

Note in prompt: gate has 4 GateCheck kinds (cmd, file_exists, git_clean, has_output). Each kind should produce its own happy-path scenario. Error paths may consolidate into 1-2 scenarios per kind.

- [ ] **Step 2-8: Standard flow**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/gate_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add gate scenarios + doc-comments (22 tests → N scenarios)

F2a Phase A/gate. M2 normalize covers 4 GateCheck kinds (cmd,
file_exists, git_clean, has_output) with pass/fail semantics."
```

- [ ] **Step 9: forward-to-f2b-gate.md**

---

## Task 7: Phase A — error module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/error_test.rs` (6 tests, 81 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/error_test.rs`

- Module: `error`
- Test count: 6
- Target band: **3-5**
- Scenario ID prefix: `belt-core-error-<kebab>`

Baseline shown in Pre-flight: tests cover ConfigParse display/diagnostic_code, RegateRequired display/diagnostic_code, RegateFailed display/diagnostic_code (variant × attribute grid).

- [ ] **Step 1: Dispatch subagent (substitute error/6/3-5/~1.5:1)**

Note in prompt: error tests are variant × attribute (display vs diagnostic_code). M2 normalize should aggregate same-variant tests into 1 scenario each.

- [ ] **Step 2-8: Standard flow**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/error_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add error scenarios + doc-comments (6 tests → N scenarios)

F2a Phase A/error. M2 normalize covers BeltError variant display +
miette diagnostic_code for ConfigParse / RegateRequired / RegateFailed."
```

- [ ] **Step 9: forward-to-f2b-error.md**

---

## Task 8: Phase A — expander module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/expander_test.rs` (5 tests, 253 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/expander_test.rs`

- Module: `expander`
- Test count: 5
- Target band: **3-5**
- Scenario ID prefix: `belt-core-expander-<kebab>`

- [ ] **Step 1: Dispatch subagent (substitute expander/5/3-5/~1:1)**

Note in prompt: expander handles `uses:` resolution and `with:` arg substitution. Parent phase attribute (gate/regate/when) inheritance is a key behavior.

- [ ] **Step 2-8: Standard flow**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/expander_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add expander scenarios + doc-comments (5 tests → N scenarios)

F2a Phase A/expander. M2 normalize covers sub-pipeline expansion,
parent attribute inheritance, and with: arg substitution."
```

- [ ] **Step 9: forward-to-f2b-expander.md**

---

## Task 9: Phase A — parser module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/parser_test.rs` (4 tests, 215 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/parser_test.rs`

- Module: `parser`
- Test count: 4
- Target band: **3-4**
- Scenario ID prefix: `belt-core-parser-<kebab>`

- [ ] **Step 1: Dispatch subagent (substitute parser/4/3-4/~1:1)**

- [ ] **Step 2-8: Standard flow**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/parser_test.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add parser scenarios + doc-comments (4 tests → N scenarios)

F2a Phase A/parser. M2 normalize covers parse_pipeline /
parse_gate_definition / parse_sub_pipeline YAML deserialization."
```

- [ ] **Step 9: forward-to-f2b-parser.md**

---

## Task 10: Phase A — artifact_when module scenarios + doc-comments

**Files:**
- Read: `crates/belt-core/tests/artifact_when_field.rs` (5 tests, 225 lines)
- Modify: `docs/testing/cli-behavior/belt-core.yml`
- Modify: `crates/belt-core/tests/artifact_when_field.rs`

- Module: `artifact_when` (not `model`, per design.md decision)
- Test count: 5
- Target band: **3-5**
- Scenario ID prefix: `belt-core-artifact-when-<kebab>`

- [ ] **Step 1: Dispatch subagent (substitute artifact_when/5/3-5/~1:1)**

Note in prompt: Artifact.when is an expression evaluated against args (e.g., `when: "args.e2e"`). It filters produce phases. Scenarios should capture expression evaluation + filtering behavior.

- [ ] **Step 2: Receive subagent output**

- [ ] **Step 3: Append to belt-core.yml under `# --- artifact_when module ---` (new category, distinct from model)**

- [ ] **Step 4-7: Standard flow**

- [ ] **Step 8: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml crates/belt-core/tests/artifact_when_field.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add artifact_when scenarios + doc-comments (5 tests → N scenarios)

F2a Phase A/artifact_when. Separate category from model per design
decision; covers Artifact.when expression evaluation + produce filter."
```

- [ ] **Step 9: forward-to-f2b-artifact-when.md**

---

## Task 11: Phase B3 — uri_test.rs new integration file + scenarios

**Files:**
- Create: `crates/belt-core/tests/uri_test.rs` (new integration test file)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (append uri section)

Context: `belt-core/src/uri.rs` has 13 inline unit tests in `#[cfg(test)] mod tests`, but scenarios_contract.rs walk scope is `crates/*/tests/**/*.rs` only. F2a creates an integration test file with 3-5 black-box scenarios. Inline-test duplication resolution is F2b.

- [ ] **Step 1: Write the new test file**

Create `crates/belt-core/tests/uri_test.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

//! Integration tests for belt-core BeltUri parsing.
//! White-box unit tests for each error variant live in src/uri.rs #[cfg(test)] mod.
//! This file covers black-box behavior of the three selector variants (Run / Latest /
//! WorkspaceLatest) and the overall parse contract.

use belt_core::uri::{BeltUri, UriParseError};

/// scenario: belt-core-uri-latest-selector-parses
#[test]
fn latest_selector_parses_with_pipeline_and_path() {
    let u = BeltUri::parse("belt://latest/feature-dev/notes/phase-review.md")
        .expect("valid Latest URI must parse");
    match u {
        BeltUri::Latest { pipeline, path } => {
            assert_eq!(pipeline, "feature-dev");
            assert_eq!(path, "notes/phase-review.md");
        }
        other => panic!("expected BeltUri::Latest, got {other:?}"),
    }
}

/// scenario: belt-core-uri-workspace-latest-selector-parses
#[test]
fn workspace_latest_selector_parses_with_branch_pipeline_path() {
    let u = BeltUri::parse("belt://workspace/develop/latest/feature-dev/notes/phase-review.md")
        .expect("valid WorkspaceLatest URI must parse");
    match u {
        BeltUri::WorkspaceLatest { branch, pipeline, path } => {
            assert_eq!(branch, "develop");
            assert_eq!(pipeline, "feature-dev");
            assert_eq!(path, "notes/phase-review.md");
        }
        other => panic!("expected BeltUri::WorkspaceLatest, got {other:?}"),
    }
}

/// scenario: belt-core-uri-run-selector-parses
#[test]
fn run_selector_parses_with_run_id_and_path() {
    let u = BeltUri::parse("belt://run/01947abc-0000-7000-8000-000000000000/notes/phase-review.md")
        .expect("valid Run URI must parse");
    match u {
        BeltUri::Run { run_id, path } => {
            assert_eq!(run_id, "01947abc-0000-7000-8000-000000000000");
            assert_eq!(path, "notes/phase-review.md");
        }
        other => panic!("expected BeltUri::Run, got {other:?}"),
    }
}

/// scenario: belt-core-uri-missing-scheme-rejected
#[test]
fn non_belt_scheme_is_rejected() {
    let err = BeltUri::parse("https://example.com/foo")
        .expect_err("non-belt scheme must be rejected");
    assert!(
        matches!(err, UriParseError::MissingScheme(_)),
        "expected MissingScheme, got {err:?}"
    );
}

/// scenario: belt-core-uri-path-traversal-rejected
#[test]
fn path_traversal_segment_is_rejected() {
    let err = BeltUri::parse("belt://latest/feature-dev/../etc/passwd")
        .expect_err("path traversal must be rejected");
    assert!(
        matches!(err, UriParseError::PathTraversal { .. }),
        "expected PathTraversal, got {err:?}"
    );
}
```

- [ ] **Step 2: Append uri scenarios to belt-core.yml**

Add under `# --- uri module ---` separator:

```yaml
  - id: belt-core-uri-latest-selector-parses
    category: uri
    severity: high
    technique: equivalence-partition
    given: "a belt://latest/<pipeline>/<path> string"
    when: "BeltUri::parse() is called"
    then: "returns Ok(BeltUri::Latest { pipeline, path }) with field values from URI segments"

  - id: belt-core-uri-workspace-latest-selector-parses
    category: uri
    severity: high
    technique: equivalence-partition
    given: "a belt://workspace/<branch>/latest/<pipeline>/<path> string"
    when: "BeltUri::parse() is called"
    then: "returns Ok(BeltUri::WorkspaceLatest { branch, pipeline, path }) with field values from URI segments"

  - id: belt-core-uri-run-selector-parses
    category: uri
    severity: high
    technique: equivalence-partition
    given: "a belt://run/<run_id>/<path> string"
    when: "BeltUri::parse() is called"
    then: "returns Ok(BeltUri::Run { run_id, path }) with field values from URI segments"

  - id: belt-core-uri-missing-scheme-rejected
    category: uri
    severity: high
    technique: error-guessing
    given: "a URI string with non-belt:// scheme"
    when: "BeltUri::parse() is called"
    then: "returns Err(UriParseError::MissingScheme)"

  - id: belt-core-uri-path-traversal-rejected
    category: uri
    severity: high
    technique: error-guessing
    given: "a belt:// URI with .. segment in path"
    when: "BeltUri::parse() is called"
    then: "returns Err(UriParseError::PathTraversal)"
```

- [ ] **Step 3: Run scenarios_contract**

```bash
cargo test -p belt-core --test scenarios_contract 2>&1 | tail -10
```

Expected: All pass. uri scenarios (5) match uri_test.rs doc-comments (5).

- [ ] **Step 4: Run uri_test.rs directly**

```bash
cargo test -p belt-core --test uri_test 2>&1 | tail -10
```

Expected: 5 tests pass.

- [ ] **Step 5: Run workspace tests**

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: 405 passed (400 + 5 new).

- [ ] **Step 6: clippy + fmt**

```bash
cargo clippy -p belt-core --tests -- -D warnings && cargo fmt --all -- --check
```

Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/uri_test.rs docs/testing/cli-behavior/belt-core.yml
git -c commit.gpgsign=false commit -m "test(belt-core): add uri_test.rs integration file + 5 scenarios

F2a Phase B3. New integration test file covers 3 selector variants
(Latest/WorkspaceLatest/Run) happy paths + 2 error variants (missing
scheme, path traversal). Inline unit tests in src/uri.rs (13 tests)
remain; duplication review forwarded to F2b audit."
```

- [ ] **Step 8: Record inline-vs-integration duplication to forward-to-f2b-uri.md**

Content:
```
# uri module: inline tests vs integration tests — F2b duplication review

src/uri.rs #[cfg(test)] mod tests has 13 unit tests covering:
- parse_latest_happy_path, parse_workspace_latest_happy_path, parse_run_happy_path (overlap with integration Happy paths)
- parse_missing_scheme (overlap with integration)
- parse_unknown_selector, parse_empty_pipeline, parse_empty_run_id, parse_empty_path (unique to inline)
- parse_path_traversal_rejected (overlap with integration)
- parse_absolute_path_rejected (edge case)
- parse_workspace_missing_latest (malformed detail)
- to_string_roundtrip_all_variants (Display impl coverage)

F2a integration file added 5 scenarios (3 happy + 2 error).

F2b decision: either (a) delete inline tests once integration covers all (with scenarios for 7 additional edge cases), or (b) keep inline as white-box companion and label inline as implementation-coupling exemption.
```

---

## Task 12: Phase B4 — Restore `belt-lint-invalid-yaml-rejected` scenario + test

**Files:**
- Modify: `crates/belt/tests/cli_test.rs` (add 1 new test fn with doc-comment)
- Modify: `docs/testing/cli-behavior/belt.yml` (add 1 scenario)

- [ ] **Step 1: Add the new test to cli_test.rs (after existing 5 tests, before EOF)**

Open `crates/belt/tests/cli_test.rs` and append at the end of the file (keeping the existing 5 tests unchanged):

```rust
/// scenario: belt-lint-invalid-yaml-rejected
#[test]
fn lint_rejects_invalid_yaml_with_parse_error() {
    use assert_cmd::Command;
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut file = NamedTempFile::new().expect("create temp file");
    writeln!(file, "phases:\n  - id: a\n    bad_indent").expect("write invalid yaml");

    let output = Command::cargo_bin("belt")
        .expect("belt binary built")
        .args(["lint", file.path().to_str().expect("utf8 path")])
        .output()
        .expect("run belt lint");

    let code = output.status.code().expect("exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(code, 1, "invalid YAML must exit 1, got {code}: stderr={stderr}");
    assert!(
        stderr.to_lowercase().contains("parse")
            || stderr.to_lowercase().contains("yaml")
            || stderr.to_lowercase().contains("expected"),
        "stderr should indicate parse error; got: {stderr}"
    );
}
```

Before editing, read the existing file to identify:
- What `use` statements are already present (`assert_cmd::Command`, `tempfile::NamedTempFile`, `std::io::Write` — use the same helpers as the existing 5 tests).
- The indent convention (4 spaces).
- The location of the last `#[test]` block to insert after.

If the file already imports these modules at the top, remove the inner `use` statements from the new fn and use the top-level imports.

- [ ] **Step 2: Add the scenario to belt.yml**

Open `docs/testing/cli-behavior/belt.yml` and add after existing scenarios:

```yaml
  - id: belt-lint-invalid-yaml-rejected
    category: lint
    severity: high
    technique: error-guessing
    given: "a pipeline YAML file with invalid syntax (malformed indentation or structure)"
    when: "belt lint <file> is invoked"
    then: "belt exits with code 1 and stderr contains a parse error indication"
```

- [ ] **Step 3: Run the new test**

```bash
cargo test -p belt --test cli_test lint_rejects_invalid_yaml_with_parse_error 2>&1 | tail -10
```

Expected: test passes.

If the test fails with a different exit code or missing stderr signal, adjust the test's expected assertions — the behavior target is "exit 1 + stderr indicates parse failure" with tolerance for format variation (miette diagnostic / serde-saphyr message). Do not lock to exact phrases.

- [ ] **Step 4: Run scenarios_contract**

```bash
cargo test -p belt-core --test scenarios_contract 2>&1 | tail -10
```

Expected: pass. Symmetric diff shows belt.yml's new ID matches cli_test.rs's new doc-comment.

- [ ] **Step 5: Run full workspace**

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: 406 passed (405 + 1 new).

- [ ] **Step 6: clippy + fmt**

```bash
cargo clippy -p belt --tests -- -D warnings && cargo fmt --all -- --check
```

- [ ] **Step 7: Commit**

```bash
git add crates/belt/tests/cli_test.rs docs/testing/cli-behavior/belt.yml
git -c commit.gpgsign=false commit -m "test(belt): restore belt-lint-invalid-yaml-rejected scenario + test

F2a Phase B4. Add 1 test + 1 scenario that were scoped out of F1 due
to CLI-level invalid YAML coverage gap. Assertion is intentionally
loose (exit=1 + stderr contains parse-like token) to avoid brittleness
against miette/serde-saphyr format changes."
```

---

## Task 13: Phase B5 — Restore `belt-core-config-preserves-absolute-pipeline-path` scenario + test

**Files:**
- Modify: `crates/belt-core/tests/config_test.rs` (add 1 new test fn with doc-comment)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (add 1 scenario to config category)

**Pre-verified in Pre-flight Findings**: `resolve_pipeline_path` at `belt-core/src/config.rs:35-38` uses `base_dir.join(&config.pipeline)`. Rust `Path::join(absolute)` returns the absolute path as-is. Therefore the test assertion "absolute input is preserved" holds without production code changes.

- [ ] **Step 1: Add the new test to config_test.rs**

Open `crates/belt-core/tests/config_test.rs`, append after the existing 6 tests:

```rust
/// scenario: belt-core-config-preserves-absolute-pipeline-path
#[test]
fn preserves_absolute_pipeline_path() {
    use belt_core::config::{resolve_pipeline_path, BeltConfig};
    use std::path::{Path, PathBuf};

    let config = BeltConfig {
        pipeline: "/tmp/absolute/pipeline.yml".to_string(),
    };
    let config_path = Path::new("/some/config/dir/belt.toml");

    let resolved = resolve_pipeline_path(config_path, &config);

    assert_eq!(
        resolved,
        PathBuf::from("/tmp/absolute/pipeline.yml"),
        "absolute pipeline path in belt.toml must be returned unchanged (not joined with config_dir)"
    );
}
```

Before editing, check the existing imports at the top of config_test.rs. If `BeltConfig` / `resolve_pipeline_path` / `PathBuf` are already in scope, remove the inner `use` statements.

- [ ] **Step 2: Add the scenario to belt-core.yml**

Locate the config category section (existing 6 scenarios + potentially `belt-core-config-resolves-subdirectory-pipeline-path` which was already added in F1). Append:

```yaml
  - id: belt-core-config-preserves-absolute-pipeline-path
    category: config
    severity: medium
    technique: boundary-value
    given: "a belt.toml with an absolute pipeline_file path (starts with /)"
    when: "resolve_pipeline_path(config_path, config) is called"
    then: "returns the absolute path unchanged, not joined with config_path's parent directory"
```

- [ ] **Step 3: Run the new test**

```bash
cargo test -p belt-core --test config_test preserves_absolute_pipeline_path 2>&1 | tail -10
```

Expected: test passes immediately (behavior pre-verified).

If it fails, the pre-flight assumption was wrong; stop F2a and open a new issue for `resolve_pipeline_path` behavior discrepancy before continuing.

- [ ] **Step 4: Run scenarios_contract**

```bash
cargo test -p belt-core --test scenarios_contract 2>&1 | tail -10
```

Expected: pass.

- [ ] **Step 5: Run full workspace**

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: 407 passed (406 + 1 new).

- [ ] **Step 6: clippy + fmt**

```bash
cargo clippy -p belt-core --tests -- -D warnings && cargo fmt --all -- --check
```

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/config_test.rs docs/testing/cli-behavior/belt-core.yml
git -c commit.gpgsign=false commit -m "test(belt-core): restore belt-core-config-preserves-absolute-pipeline-path scenario + test

F2a Phase B5. Add 1 test + 1 scenario locking the behavior that
absolute pipeline_file paths in belt.toml are returned unchanged by
resolve_pipeline_path (Rust Path::join absolute-right semantics)."
```

---

## Task 14: Phase B6 — audit-template.md clarification patch

**Files:**
- Modify: `docs/testing/audit-template.md` (append clarification under "Pilot Audit の再実施 trigger" section)

- [ ] **Step 1: Read the current audit-template.md Pilot Re-audit section**

Open `docs/testing/audit-template.md` and locate the `## Pilot Audit の再実施 trigger (F2 着手時)` section (line ~60-73 based on F1 state).

- [ ] **Step 2: Append clarification paragraph**

After the existing section body (the `git log --since` example block), add:

```markdown

### Trigger 対象外 (F2a で明示)

「touch されている」判定は **既存 test fn の modify** を指す。以下は re-audit trigger の対象外:

- pilot file に**新規 test fn を追加**する変更 (既存 fn の assertion / setup / teardown 変更なし)
- pilot file に doc-comment (`/// scenario: <id>`) を付与する変更 (behavior 不変)
- pilot file の preamble (`#![allow(...)] reason = "..."`) 更新

理由: 新規追加 fn は F2a の「復帰 scenario + 対応 test」pattern であり、F1 pilot 判定済 fn の挙動を変えないため re-audit は不要。逆に、既存 fn の body 変更 (assertion 差し替え / fixture 変更 / 実装 side 変更による意味の shift) のみが audit 結果 stale 化の signal となる。
```

Use Edit tool:
```
old_string: (最後の ``` + ``` の後の空行 + 次 section 先頭)
new_string: (同 block + append 上記 markdown)
```

具体的には、Pilot Re-audit section の末尾（現状 line 73 付近）に上記を挿入。`audit_template_version: v1` frontmatter は変更しない (clarification なので SemVer bump 不要)。

- [ ] **Step 3: Verify scenarios_contract still passes (version check unchanged)**

```bash
cargo test -p belt-core --test scenarios_contract audit_template_version_v1_matches_expected 2>&1 | tail -5
```

Expected: pass (frontmatter `audit_template_version: v1` still present).

- [ ] **Step 4: Run full workspace**

```bash
cargo test --workspace 2>&1 | tail -5
```

Expected: 407 passed, unchanged from Task 13.

- [ ] **Step 5: Commit**

```bash
git add docs/testing/audit-template.md
git -c commit.gpgsign=false commit -m "docs(testing): clarify re-audit trigger excludes new fn addition

F2a Phase B6. Add 'Trigger 対象外' subsection to Pilot Re-audit trigger
section: new test fn addition, doc-comment attachment, and preamble
updates do not count as pilot 'touch'. Only existing fn body
modifications invalidate prior audit judgments.

audit_template_version remains v1 (clarification, not semantics change)."
```

---

## Task 15: Phase C — audit-report.md + README consistency check

**Files:**
- Create: `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md`
- Read (and optionally modify): `docs/testing/README.md`

- [ ] **Step 1: Aggregate forward-to-f2b-*.md files into master list**

Collect the 9 module forward lists saved in Tasks 2-10 Step 9 (`forward-to-f2b-{engine,view,lint,model,gate,error,expander,parser,artifact-when}.md`) + Task 11 Step 8 (`forward-to-f2b-uri.md`) into a single aggregated forward list.

Run (adjust path if runs directory differs):
```bash
cat /tmp/forward-to-f2b-*.md 2>/dev/null | wc -l
```

Or from `.belt/runs/{run_id}/artifacts/`, ls and cat all files.

- [ ] **Step 2: Author audit-report.md**

Create `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md`:

```markdown
---
audited_at: <ISO 8601 UTC timestamp at report authoring time, e.g. 2026-04-18T05:00:00Z>
audited_commit: <F2a HEAD sha at authoring time, from `git rev-parse HEAD`>
audit_template_version: v1
---

# belt-test-foundation F2a Audit Report

F2a audit 結果。F1 pilot で確立した methodology (`docs/testing/audit-template.md` v1) を belt-core 全 behavior module に application、M2 normalize + X2 doc-comment policy 下で judgment を記録。

## Methodology

- **Decision Tree Q1** で "scenario 登録済" 判定: F2a で scenarios.yml に追加した scenario に doc-comment 付与する test fn は kept
- **Q2-Q5 は F2b で適用**: F2a では "scenario 化できなかった test" を judgment せず Forward-to-F2b list に送る
- **Label 使用**: `kept` のみ。F2b の 8 label 適用は次 feature の範囲

## Judgment summary

### Phase A モジュール別 (9 module)

| module | test 数 | scenarios | annotated | forward-to-F2b | forward 内訳 |
|---|---|---|---|---|---|
| engine | 67 | <N>  | <M>     | <67-M>         | <主要な reason の distribution> |
| view | 41 | <N> | <M> | <41-M> | ... |
| lint | 29 | <N> | <M> | <29-M> | ... |
| model | 39 | <N> | <M> | <39-M> | ... |
| gate | 22 | <N> | <M> | <22-M> | ... |
| error | 6 | <N> | <M> | <6-M> | ... |
| expander | 5 | <N> | <M> | <5-M> | ... |
| parser | 4 | <N> | <M> | <4-M> | ... |
| artifact_when | 5 | <N> | <M> | <5-M> | ... |

実数は Phase A 完了時に埋める (subagent output の集計)。

### Phase B3 uri_test.rs (新規)

- test 追加数: 5
- scenario 数: 5
- 全 kept
- inline tests (src/uri.rs の 13 test) との duplication 判断は F2b (see Forward-to-F2b list)

### Phase B4-B5 復帰 (2 test + 2 scenario)

- `belt-lint-invalid-yaml-rejected` (cli_test.rs): kept
- `belt-core-config-preserves-absolute-pipeline-path` (config_test.rs): kept

## Forward-to-F2b list

F2b が Decision Tree Q2-Q5 + 8 reason label を適用する対象。module × test_fn × 疑い label の形式で列挙:

<ここに Tasks 2-10 の forward-to-f2b-*.md を集約した list を挿入>

### uri module duplication (inline vs integration)

src/uri.rs #[cfg(test)] mod tests の 13 test と tests/uri_test.rs の 5 test の重複 audit は F2b で実施。

## Summary

- Total F2a-audited behavior test: <合計>
- kept: <N>  (scenarios に map 済 + 復帰 2)
- forward-to-F2b: <M>
- deleted/merged/abstracted in F2a: 0 (Phase A は additive-only)

## Cross-reference

- Template: `docs/testing/audit-template.md` (v1)
- Scenarios: `docs/testing/cli-behavior/{belt,belt-core}.yml`
- Lock ledger: `docs/testing/lock-ledger.md` (unchanged in F2a)
- Design: `docs/features/2026-04-17-belt-test-foundation-f2a/design.md`
- Plan: `docs/features/2026-04-17-belt-test-foundation-f2a/plan.md`
- Test strategy: `docs/features/2026-04-17-belt-test-foundation-f2a/test-strategy.md` (Phase 2 output)
- F1 audit report (pilot 22 test): `docs/features/2026-04-17-belt-test-foundation/audit-report.md`
```

Fill in `<N>` / `<M>` values from subagent outputs collected in Tasks 2-10.

- [ ] **Step 3: Finalize belt-core.yml scope string**

Open `docs/testing/cli-behavior/belt-core.yml` and update the top `scope:` field to reflect F2a completion:

```yaml
scope: "belt-core pure library public API. F1: config (6). F2a: engine / view / lint / model / gate / error / expander / parser / uri / artifact_when の 10 module 列挙済 (77-107 scenarios)。残 shape-lock 4 file (bug_fix_refresh / review_skills_refresh / shared_criteria_parity / shared_filter_parity) の lock-ledger.md entry 追加は F2b scope"
```

- [ ] **Step 4: Check docs/testing/README.md for consistency**

Read `docs/testing/README.md`. Verify:
- "belt-core.yml は F1 では config module のみ、F2 で他 module 拡充予定" 的な記述があれば F2a 完了に合わせて update
- 構造宣言 (SSOT / binding / pilot) が F2a deliverable と矛盾しない

If no update needed, skip modifying README.

If update needed, edit the relevant paragraphs to reflect:
- belt-core.yml now covers 10 modules (77-107 scenarios)
- uri_test.rs is a new test file in crates/belt-core/tests/
- F2b scope is audit-driven (behavior-less test deletion, Duplication Candidates consolidation)

- [ ] **Step 5: Run scenarios_contract (final guard)**

```bash
cargo test -p belt-core --test scenarios_contract 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 6: Run full workspace test**

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: 407 passed.

- [ ] **Step 7: Run clippy + fmt**

```bash
cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check
```

Expected: both clean.

- [ ] **Step 8: Commit**

```bash
git add docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md docs/testing/cli-behavior/belt-core.yml
# Only include README if modified
git status
# If README was updated:
# git add docs/testing/README.md

git -c commit.gpgsign=false commit -m "docs(features): add F2a audit report + finalize belt-core.yml scope

F2a Phase C. Audit report captures kept judgments for all annotated
tests + forwards un-mapped tests to F2b with provisional reason labels.
belt-core.yml scope string updated to reflect 10-module enumeration.

No README changes (if unchanged) / README updated to reflect F2a
deliverable scope (if changed)."
```

---

## Task 16: Integrate to main (Phase 7 of feature-dev pipeline)

**Files:**
- (git operations only)

- [ ] **Step 1: Verify all 15 commits are on the F2a branch**

Run:
```bash
git log --oneline main..HEAD
```

Expected: ~15 commits listed (Task 1 + Tasks 2-10 (9 module) + Tasks 11-15 = 15 commits; Task 0 contributed 0 commits).

- [ ] **Step 2: Run full CI gate one last time**

```bash
cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check
```

Expected: all clean, 407 tests passing.

- [ ] **Step 3: Switch to main and merge**

Ask user for confirmation before executing:

```bash
git checkout main
git merge feature/2026-04-17-belt-test-foundation-f2a --no-ff
```

- [ ] **Step 4: Report merge SHA and test count to user**

Final status:
- Merge SHA: `<sha>`
- Total workspace tests: 407
- New scenarios added: 77-107 (F2a band, M2 normalize)
- Forward-to-F2b count: aggregated in audit-report.md
- F2b entry point: `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md` Forward-to-F2b list

---

## Self-Review Checklist (plan author — performed before handoff)

**Spec coverage** (design.md MV-01 〜 MV-45):

| MV range | Covered by |
|---|---|
| MV-01 〜 MV-09 (基本構造) | Tasks 2-10 (Phase A), 11, 12, 13, 14, 15 |
| MV-10 〜 MV-14 (SSOT ↔ Rust binding) | Task 1 (Phase B2) + Tasks 2-10 (symmetric diff per commit) |
| MV-15 〜 MV-24 (Module-level coverage) | Tasks 2-11 (each module, band constraint in subagent prompt) |
| MV-25 〜 MV-27 (復帰 behavior) | Tasks 12, 13 |
| MV-28 〜 MV-30 (strip fix) | Task 1 (3 new drift tests) |
| MV-31 〜 MV-35 (一貫性・regression) | Each task Step "Run workspace test" + Step "clippy + fmt" |
| MV-36 (Narrative notes) | feature-dev pipeline's phase-*.md authoring, implicit in W1 workflow |
| MV-37 〜 MV-38 (worktree) | Task 0 |
| MV-39 〜 MV-41 (doc-drift 予防) | Task 15 (README + scope + template patch cross-check) |
| MV-42 〜 MV-45 (F2a-specific risks) | Task 0 (baseline) + Task 13 (resolve_pipeline_path pre-verified in Pre-flight) + Task 11 (uri scope 3-5 strict band) + subagent prompt (module band verbatim) |

**Placeholder scan**: `<N>` / `<M>` placeholders exist in Task 15 audit-report template; these are data-dependent (subagent output counts) and must be filled at runtime.

**Type consistency**:
- `scenario ID` uses kebab-case `belt-core-<module>-<behavior>` across all tasks (consistent)
- `category` field matches module name (engine / view / lint / model / gate / error / expander / parser / uri / artifact_when) — note `artifact_when` (not `artifact-when`) per belt-core.yml YAML key convention
- `technique` enum values: equivalence-partition / boundary-value / state-transition / error-guessing / decision-table (consistent across tasks)
- Rust test fn names use snake_case (consistent)
- doc-comment format `/// scenario: <id>` (consistent)

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::doc_lazy_continuation,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::lint::{Severity, lint_pipeline};
use tempfile::TempDir;

mod common;
use common::helpers::write_yaml;

/// scenario: belt-core-lint-detects-duplicate-phase-ids
#[test]
fn lint_detects_duplicate_phase_ids() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: dup
version: 1
phases:
  - id: build
    description: "Build 1"
  - id: build
    description: "Build 2"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("duplicate")),
        "expected a diagnostic containing 'duplicate', got: {errors:?}",
    );
}

/// scenario: belt-core-lint-detects-invalid-regate-target
#[test]
fn lint_detects_invalid_regate_target() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-regate
version: 1
phases:
  - id: build
    description: "Build the project"
    regate:
      - nonexistent
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("nonexistent")),
        "expected a diagnostic mentioning 'nonexistent', got: {errors:?}",
    );
}

/// scenario: belt-core-lint-detects-undefined-arg-in-when
#[test]
fn lint_detects_undefined_arg_in_when() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-when
version: 1
args:
  smoke:
    type: bool
    default: false
phases:
  - id: build
    description: "Build the project"
    when: "args.nonexistent"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("nonexistent")),
        "expected a diagnostic mentioning 'nonexistent', got: {errors:?}",
    );
}

/// scenario: belt-core-lint-clean-pipeline-passes
#[test]
fn clean_pipeline_passes_lint() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: clean
version: 1
args:
  smoke:
    type: bool
    default: false
phases:
  - id: build
    description: "Build the project"
    gate:
      - cmd: "cargo build"
  - id: test
    description: "Run tests"
    when: "args.smoke"
    gate:
      - cmd: "cargo test"
    regate:
      - build
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no Error diagnostics, got: {errors:?}",
    );
}

/// scenario: belt-core-lint-invoke-pipeline-path-resolution
#[test]
fn lint_detects_missing_invoke_pipeline_file() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-invoke-pipeline
version: 1
phases:
  - id: sub
    description: "sub"
    invoke:
      pipeline: ./nonexistent.yml
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("nonexistent.yml")
            && (d.message.contains("invoke") || d.message.contains("pipeline"))),
        "expected diagnostic mentioning 'nonexistent.yml' and invoke/pipeline, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-invoke-pipeline-path-resolution
#[test]
fn lint_accepts_valid_invoke_pipeline_path() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Write a valid sub-pipeline file first.
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: run
    description: "sub run"
"#,
    );

    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r"
name: good-invoke-pipeline
version: 1
phases:
  - id: s
    invoke:
      pipeline: ./sub.yml
",
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors for valid invoke pipeline path, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-invoke-skill-validation
#[test]
fn lint_detects_invoke_skill_without_description() {
    // An `invoke: { skill: ... }` phase executes at the current phase rather
    // than delegating to a sub-pipeline, so it must still carry a description
    // for human-readable status output.
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r"
name: invoke-skill-no-desc
version: 1
phases:
  - id: review
    invoke:
      skill: /code-review
",
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("leaf phase") || d.message.contains("description")),
        "expected diagnostic mentioning 'leaf phase' or 'description', got: {errors:?}"
    );
}

/// scenario: belt-core-lint-invoke-skill-validation
#[test]
fn lint_detects_invoke_skill_without_leading_slash() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-invoke-skill
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: brainstorming
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("brainstorming") && d.message.contains("slash")),
        "expected diagnostic about leading slash, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-invoke-skill-validation
#[test]
fn lint_accepts_invoke_skill_with_leading_slash() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-invoke-skill
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: /brainstorming
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

/// scenario: belt-core-lint-invoke-skill-validation
#[test]
fn lint_detects_invoke_skill_empty() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: empty-invoke-skill
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: ""
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("empty")),
        "expected diagnostic about empty skill, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-detects-duplicate-produces-name-within-phase
#[test]
fn lint_detects_duplicate_produces_name_in_one_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: dup-produces
version: 1
phases:
  - id: p
    description: "p"
    produces:
      - name: doc
        path: "a.md"
      - name: doc
        path: "b.md"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("duplicate") && d.message.contains("doc")),
        "expected duplicate produces name error, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-consumes-resolution
#[test]
fn lint_detects_unresolved_consumes_named() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: unresolved-consumes
version: 1
phases:
  - id: first
    description: "first"
    produces:
      - name: design_doc
        path: "design.md"
  - id: second
    description: "second"
    consumes:
      - phantom_artifact
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("phantom_artifact") && d.message.contains("consumes")),
        "expected unresolved consumes error, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-consumes-resolution
#[test]
fn lint_detects_unresolved_consumes_qualified_unknown_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: unresolved-qualified
version: 1
phases:
  - id: first
    description: "first"
    produces:
      - name: doc
        path: "doc.md"
  - id: second
    description: "second"
    consumes:
      - name: doc
        from: nonexistent
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("nonexistent") && d.message.contains("consumes")),
        "expected unresolved qualified consumes error, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-consumes-resolution
#[test]
fn lint_accepts_consumes_resolved_to_earlier_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-consumes
version: 1
phases:
  - id: design
    description: "d"
    gate:
      - cmd: "true"
    produces:
      - name: design_doc
        path: "design.md"
  - id: plan
    description: "p"
    gate:
      - cmd: "true"
    consumes:
      - design_doc
  - id: review
    description: "r"
    gate:
      - cmd: "true"
    consumes:
      - name: design_doc
        from: design
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

/// scenario: belt-core-lint-consumes-resolution
#[test]
fn lint_detects_consumes_from_later_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-forward-consumes
version: 1
phases:
  - id: early
    description: "e"
    consumes:
      - late_doc
  - id: late
    description: "l"
    produces:
      - name: late_doc
        path: "late.md"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("late_doc")),
        "expected error about consuming later phase's output, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-validate-file-existence
#[test]
fn lint_detects_validate_file_missing() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-validate-file
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - file: ./nonexistent-criteria.md
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(
            |d| d.message.contains("nonexistent-criteria.md") && d.message.contains("validate")
        ),
        "expected validate file missing error, got: {errors:?}"
    );
}

/// scenario: belt-core-lint-validate-file-existence
#[test]
fn lint_accepts_validate_file_present() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Write a criteria file.
    write_yaml(&dir, "criteria.md", "# Criteria\n\n- C1: placeholder\n");

    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-validate-file
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - file: ./criteria.md
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

/// A completely empty phase (no invoke, gate, validate, or confirm) is an
/// authoring mistake — belt-core lint rejects it per spec DD-8.
///
/// scenario: belt-core-lint-empty-phase-rule
#[test]
fn lint_rejects_completely_empty_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: t
version: 1
phases:
  - id: empty
    description: "has nothing"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("'empty'")
            && d.message
                .contains("neither invoke, gate, validate, nor confirm")),
        "expected EmptyPhase diagnostic for 'empty', got {errors:?}"
    );
}

/// Phase with only `gate:` passes the empty-phase rule.
///
/// scenario: belt-core-lint-empty-phase-rule
#[test]
fn lint_accepts_phase_with_only_gate() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: t
version: 1
phases:
  - id: p
    description: p
    gate:
      - cmd: "true"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        !errors.iter().any(|d| d
            .message
            .contains("neither invoke, gate, validate, nor confirm")),
        "phase with only gate must pass empty-phase lint: {errors:?}"
    );
}

/// Phase with only `validate:` passes the empty-phase rule.
///
/// scenario: belt-core-lint-empty-phase-rule
#[test]
fn lint_accepts_phase_with_only_validate() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate:
      - "some criterion"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        !errors.iter().any(|d| d
            .message
            .contains("neither invoke, gate, validate, nor confirm")),
        "phase with only validate must pass empty-phase lint: {errors:?}"
    );
}

/// Phase with only `confirm: true` passes the empty-phase rule.
///
/// scenario: belt-core-lint-empty-phase-rule
#[test]
fn lint_accepts_phase_with_only_confirm() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r"
name: t
version: 1
phases:
  - id: p
    description: p
    confirm: true
",
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        !errors.iter().any(|d| d
            .message
            .contains("neither invoke, gate, validate, nor confirm")),
        "phase with only confirm must pass empty-phase lint: {errors:?}"
    );
}

/// scenario: belt-core-lint-validate-file-existence
#[test]
fn lint_validate_inline_strings_unaffected() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: inline-validate
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - "inline criterion 1"
      - "inline criterion 2"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

/// Lint rule: External URI grammar is validated.
///
/// Note: `BeltUri::parse` already rejects unknown selectors at YAML
/// deserialization time (see `BeltUri::Deserialize`), so `parse_pipeline`
/// may hard-fail before our dedicated lint pass runs. This test accepts
/// either outcome:
///   - `Ok(diags)` with a URI-grammar diagnostic, or
///   - `Ok(diags)` containing a parse-level error that mentions the URI.
///   - `Err(_)` (parse-level rejection propagated).
/// All three satisfy the intent: invalid URI grammar is surfaced.
///
/// scenario: belt-core-lint-rejects-invalid-belt-uri-grammar
#[test]
fn lint_rejects_invalid_uri_grammar() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-uri
version: 1
phases:
  - id: first
    description: "first"
    consumes:
      - name: bad
        uri: "belt://bogus/whatever/x.md"
"#,
    );

    let result = lint_pipeline(&path);
    match result {
        Ok(diags) => {
            assert!(
                diags.iter().any(|d| {
                    let m = d.message.to_lowercase();
                    m.contains("uri") || m.contains("selector") || m.contains("invalid")
                }),
                "lint should flag invalid URI grammar, got: {diags:?}"
            );
        }
        Err(err) => {
            let msg = format!("{err}");
            let low = msg.to_lowercase();
            assert!(
                low.contains("uri") || low.contains("selector") || msg.contains("belt://"),
                "parse error should mention URI grammar: {msg}"
            );
        }
    }
}

/// Lint rule: a `consumes: External` URI referencing `belt://latest/<pipeline>/...`
/// (or its workspace-qualified variant) should emit a *warning* when no sibling
/// `<pipeline>.yml` nor `<pipeline>/pipeline.yml` exists next to the current
/// pipeline file. This is authoring-time feedback — the producer may live in a
/// different repo, so the check is advisory, not fatal.
///
/// scenario: belt-core-lint-warns-on-belt-uri-unknown-sibling-pipeline
#[test]
fn lint_warns_on_belt_uri_with_unknown_sibling_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let consumer = tmp.path().join("consumer.yml");
    std::fs::write(
        &consumer,
        r#"name: consumer
version: 1
phases:
  - id: rca
    description: "rca"
    consumes:
      - name: prior
        uri: "belt://latest/no-such-producer/notes/x.md"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&consumer).unwrap();
    assert!(
        diags.iter().any(|d| d.message.contains("no-such-producer")),
        "expected a diagnostic mentioning 'no-such-producer', got: {diags:?}"
    );
}

/// scenario: belt-core-lint-warns-on-produces-without-gate
#[test]
fn lint_warns_on_produces_without_gate() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("p.yml");
    std::fs::write(
        &p,
        r#"name: p
version: 1
phases:
  - id: review
    description: "review"
    produces:
      - name: notes
        path: ".belt/runs/{run_id}/notes/phase-review.md"
    gate:
      - cmd: "echo ok"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&p).unwrap();
    assert!(
        diags
            .iter()
            .any(|d| d.severity == belt_core::lint::Severity::Warning
                && d.message.contains("not protected by gate")),
        "expected a Warning diagnostic mentioning 'not protected by gate', got: {diags:?}"
    );
}

/// After 2026-04-16, pipeline.yml authors must not use `invoke.agent:`.
/// Lint should produce a targeted diagnostic pointing to the migration.
///
/// scenario: belt-core-lint-rejects-removed-invoke-keys
#[test]
fn lint_rejects_invoke_agent_key() {
    let yaml = r"
name: p
version: 1
phases:
  - id: x
    invoke:
      agent: foo
";
    let result = belt_core::lint::lint_raw_pipeline_yaml(yaml);
    let err = result.expect_err("lint must reject invoke.agent");
    let message = format!("{err}");
    assert!(
        message.contains("invoke.agent")
            && (message.contains("no longer supported") || message.contains("removed")),
        "lint message must mention invoke.agent removal; got: {message}"
    );
}

/// scenario: belt-core-lint-rejects-removed-invoke-keys
#[test]
fn lint_rejects_invoke_agents_key() {
    let yaml = r"
name: p
version: 1
phases:
  - id: x
    invoke:
      agents:
        - foo
";
    let result = belt_core::lint::lint_raw_pipeline_yaml(yaml);
    let err = result.expect_err("lint must reject invoke.agents");
    let message = format!("{err}");
    assert!(
        message.contains("invoke.agents"),
        "lint message must mention invoke.agents removal; got: {message}"
    );
}

/// scenario: belt-core-lint-rejects-removed-invoke-keys
#[test]
fn lint_rejects_invoke_iterations_key() {
    let yaml = r"
name: p
version: 1
phases:
  - id: x
    invoke:
      skill: /x
      iterations: 3
";
    let result = belt_core::lint::lint_raw_pipeline_yaml(yaml);
    let err = result.expect_err("lint must reject invoke.iterations");
    let message = format!("{err}");
    assert!(
        message.contains("iterations"),
        "lint message must mention iterations removal; got: {message}"
    );
}

/// CLI-path regression: `lint_pipeline` (the entry point called by `belt lint`)
/// must surface the migration-hint diagnostic for invoke.agent, not the
/// generic serde untagged-enum error.
///
/// scenario: belt-core-lint-pipeline-surfaces-migration-hint
#[test]
fn lint_pipeline_surfaces_invoke_agent_migration_hint() {
    use std::io::Write;
    let yaml = r"
name: p
version: 1
phases:
  - id: x
    invoke:
      agent: foo
";
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    tmp.write_all(yaml.as_bytes()).expect("write");
    let result = belt_core::lint::lint_pipeline(tmp.path());
    // Adjust assertion to match the actual return shape: either an Err
    // whose Display contains "invoke.agent" + "no longer supported", OR
    // a diagnostic collection containing an Error-severity entry with
    // that message.
    let rendered = match result {
        Err(e) => format!("{e}"),
        Ok(diagnostics) => format!("{diagnostics:?}"),
    };
    assert!(
        rendered.contains("invoke.agent") && rendered.contains("no longer supported"),
        "lint_pipeline must surface the raw-YAML migration hint; got: {rendered}"
    );
}

/// scenario: belt-core-lint-rejects-belt-runs-literal-in-produces-path
#[test]
fn lint_rejects_belt_runs_literal_in_produces_path() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    produces:
      - name: notes
        path: ".belt/runs/{run_id}/notes/phase-p.md"
    gate:
      - file_exists: ".belt/runs/{run_id}/notes/phase-p.md"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags
            .iter()
            .any(|d| d.severity == belt_core::lint::Severity::Error
                && d.message.contains(".belt/runs/")),
        "lint must reject .belt/runs/ literal in produces.path"
    );
}

/// scenario: belt-core-lint-rejects-belt-runs-literal-in-gate-file-exists
#[test]
fn lint_rejects_belt_runs_literal_in_gate_file_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
    confirm: true
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(diags.iter().any(
        |d| d.severity == belt_core::lint::Severity::Error && d.message.contains(".belt/runs/")
    ));
}

/// scenario: belt-core-lint-rejects-run-id-template-in-produces-path
#[test]
fn lint_rejects_run_id_template_in_produces_path() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    produces:
      - name: notes
        path: "belt://current/notes/{run_id}.md"
    gate:
      - file_exists: "belt://current/notes/{run_id}.md"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags
            .iter()
            .any(|d| d.severity == belt_core::lint::Severity::Error
                && d.message.contains("{run_id}"))
    );
}

/// scenario: belt-core-lint-rejects-run-id-template-in-gate-cmd
#[test]
fn lint_rejects_run_id_template_in_gate_cmd() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    gate:
      - cmd: "test -f .belt/runs/{run_id}/x"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags
            .iter()
            .any(|d| d.severity == belt_core::lint::Severity::Error
                && (d.message.contains("{run_id}") || d.message.contains(".belt/runs/")))
    );
}

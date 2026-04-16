#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::match_wildcard_for_single_variants,
    clippy::default_trait_access,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::model::{
    ArgType, Artifact, ArtifactRef, GateCheck, GateDefinition, Invoker, Pipeline, RunState,
    RunStatus, SubPipeline, ValidationSource,
};
use belt_core::uri::BeltUri;

/// Parse a minimal pipeline YAML: name, version, one phase with a `cmd` gate.
#[test]
fn parse_minimal_pipeline() {
    let yaml = r#"
name: minimal
version: 1
phases:
  - id: build
    gate:
      - cmd: "cargo build"
"#;

    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.name, "minimal");
    assert_eq!(pipeline.version, 1);
    assert!(pipeline.description.is_none());
    assert!(pipeline.args.is_empty());
    assert_eq!(pipeline.phases.len(), 1);

    let phase = &pipeline.phases[0];
    assert_eq!(phase.id, "build");
    assert_eq!(phase.gate.len(), 1);
    match &phase.gate[0] {
        GateCheck::Cmd { cmd, .. } => assert_eq!(cmd, "cargo build"),
        other => panic!("expected GateCheck::Cmd, got {other:?}"),
    }
}

/// Parse a phase with ALL fields populated: when, confirm, `max_retries`, config,
/// produces, multiple gate variants, validate, regate.
#[test]
fn parse_phase_all_fields() {
    let yaml = r#"
name: full
version: 2
phases:
  - id: deploy
    description: "Deploy to production"
    when: "args.env == 'prod'"
    confirm: true
    max_retries: 3
    config:
      timeout: 300
      verbose: true
    produces:
      - name: release_bundle
        path: "dist/app.tar.gz"
      - name: checksum
        path: "dist/checksum.sha256"
    gate:
      - cmd: "make test"
      - file_exists: dist/app.tar.gz
      - git_clean: true
      - has_output: true
      - uses: gates/security-scan
        with:
          level: high
    validate:
      - "file_exists dist/app.tar.gz"
    regate:
      - "cmd make verify"
"#;

    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    let phase = &pipeline.phases[0];

    assert_eq!(phase.id, "deploy");
    assert_eq!(phase.description.as_deref(), Some("Deploy to production"));
    assert_eq!(phase.when.as_deref(), Some("args.env == 'prod'"));
    assert!(phase.confirm);
    assert_eq!(phase.max_retries, 3);

    // config
    assert_eq!(phase.config.len(), 2);

    // produces
    assert_eq!(phase.produces.len(), 2);
    assert_eq!(phase.produces[0].name, "release_bundle");
    assert_eq!(phase.produces[0].path, "dist/app.tar.gz");

    // gate variants
    assert_eq!(phase.gate.len(), 5);
    match &phase.gate[0] {
        GateCheck::Cmd { cmd, .. } => assert_eq!(cmd, "make test"),
        other => panic!("expected Cmd, got {other:?}"),
    }
    match &phase.gate[1] {
        GateCheck::FileExists { file_exists } => {
            assert_eq!(file_exists, "dist/app.tar.gz");
        }
        other => panic!("expected FileExists, got {other:?}"),
    }
    match &phase.gate[2] {
        GateCheck::GitClean { git_clean } => assert!(git_clean),
        other => panic!("expected GitClean, got {other:?}"),
    }
    match &phase.gate[3] {
        GateCheck::HasOutput { has_output } => assert!(has_output),
        other => panic!("expected HasOutput, got {other:?}"),
    }
    match &phase.gate[4] {
        GateCheck::Uses { uses, with } => {
            assert_eq!(uses, "gates/security-scan");
            assert_eq!(
                with.get("level").and_then(serde_json::Value::as_str),
                Some("high")
            );
        }
        other => panic!("expected Uses, got {other:?}"),
    }

    // validate / regate
    assert_eq!(phase.validate.len(), 1);
    assert_eq!(phase.regate.len(), 1);
}

/// Parse a standalone `GateDefinition` YAML.
#[test]
fn parse_gate_definition() {
    let yaml = r#"
name: security-scan
description: Run security scanning tools
inputs:
  level:
    type: string
    required: true
  timeout:
    type: number
    default: 60
checks:
  - cmd: "trivy fs --severity {{ level }} ."
  - file_exists: reports/security.json
"#;

    let gate: GateDefinition = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(gate.name, "security-scan");
    assert_eq!(
        gate.description.as_deref(),
        Some("Run security scanning tools")
    );
    assert_eq!(gate.inputs.len(), 2);

    let level = gate.inputs.get("level").expect("level input");
    assert!(matches!(level.input_type, ArgType::String));
    assert!(level.required);
    assert!(level.default.is_none());

    let timeout = gate.inputs.get("timeout").expect("timeout input");
    assert!(matches!(timeout.input_type, ArgType::Number));
    assert!(!timeout.required);
    assert_eq!(
        timeout.default.as_ref().and_then(serde_json::Value::as_u64),
        Some(60)
    );

    assert_eq!(gate.checks.len(), 2);
    match &gate.checks[0] {
        GateCheck::Cmd { cmd, .. } => {
            assert!(cmd.contains("trivy"));
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// Parse a `SubPipeline` YAML with inputs (including `list` type).
#[test]
fn parse_sub_pipeline() {
    let yaml = r#"
name: test-suite
description: Run test suite with coverage
version: 1
inputs:
  targets:
    type: list
    required: true
  coverage_threshold:
    type: number
    default: 80
phases:
  - id: unit-test
    gate:
      - cmd: "cargo test"
  - id: coverage-report
    gate:
      - file_exists: target/coverage/index.html
"#;

    let sub: SubPipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(sub.name, "test-suite");
    assert_eq!(
        sub.description.as_deref(),
        Some("Run test suite with coverage")
    );
    assert_eq!(sub.version, 1);
    assert_eq!(sub.inputs.len(), 2);

    let targets = sub.inputs.get("targets").expect("targets input");
    assert!(matches!(targets.input_type, ArgType::List));
    assert!(targets.required);

    let threshold = sub
        .inputs
        .get("coverage_threshold")
        .expect("threshold input");
    assert!(matches!(threshold.input_type, ArgType::Number));
    assert_eq!(
        threshold
            .default
            .as_ref()
            .and_then(serde_json::Value::as_u64),
        Some(80)
    );

    assert_eq!(sub.phases.len(), 2);
    assert_eq!(sub.phases[0].id, "unit-test");
    assert_eq!(sub.phases[1].id, "coverage-report");
}

/// Parse a pipeline with typed `args` (bool and number).
#[test]
fn parse_pipeline_with_args() {
    let yaml = r#"
name: configurable
version: 1
args:
  verbose:
    type: bool
    default: false
  parallelism:
    type: number
    default: 4
phases:
  - id: run
    gate:
      - cmd: "echo ok"
"#;

    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.name, "configurable");
    assert_eq!(pipeline.args.len(), 2);

    let verbose = pipeline.args.get("verbose").expect("verbose arg");
    assert!(matches!(verbose.arg_type, ArgType::Bool));
    assert_eq!(
        verbose
            .default
            .as_ref()
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );

    let par = pipeline.args.get("parallelism").expect("parallelism arg");
    assert!(matches!(par.arg_type, ArgType::Number));
    assert_eq!(
        par.default.as_ref().and_then(serde_json::Value::as_u64),
        Some(4)
    );
}

/// `cmd: "cargo test"` without timeout field deserializes with default 1800.
#[test]
fn cmd_default_timeout() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - cmd: "cargo test"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    match &pipeline.phases[0].gate[0] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "cargo test");
            assert_eq!(*timeout, 1800);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// `{ cmd: "make lint", timeout: 60 }` deserializes with explicit timeout.
#[test]
fn cmd_explicit_timeout() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - cmd: "make lint"
        timeout: 60
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    match &pipeline.phases[0].gate[0] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "make lint");
            assert_eq!(*timeout, 60);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// `{ cmd: "sleep 999", timeout: 0 }` deserializes with zero (no timeout).
#[test]
fn cmd_timeout_zero() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - cmd: "sleep 999"
        timeout: 0
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    match &pipeline.phases[0].gate[0] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "sleep 999");
            assert_eq!(*timeout, 0);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// Adding timeout to Cmd does not affect other `GateCheck` variants.
#[test]
fn cmd_timeout_does_not_affect_other_variants() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - file_exists: "*.md"
      - cmd: "test"
        timeout: 10
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    assert_eq!(pipeline.phases[0].gate.len(), 2);
    match &pipeline.phases[0].gate[0] {
        GateCheck::FileExists { file_exists } => assert_eq!(file_exists, "*.md"),
        other => panic!("expected FileExists, got {other:?}"),
    }
    match &pipeline.phases[0].gate[1] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "test");
            assert_eq!(*timeout, 10);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// `RunState` deserialisation without `regate_passed` defaults to empty `HashMap`
/// (backward compatibility with existing state.json files).
#[test]
fn run_state_regate_passed_defaults_to_empty() {
    let json = r#"{
        "run_id": "01961234-0000-7000-8000-000000000000",
        "pipeline": "test",
        "pipeline_file": "pipeline.yml",
        "version": 1,
        "args": {},
        "current_phase": "build",
        "completed_phases": [],
        "skipped_phases": [],
        "phase_attempts": {},
        "phase_verify_passed": {},
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    }"#;

    let state: RunState = serde_json::from_str(json).expect("should deserialize");
    assert!(
        state.regate_passed.is_empty(),
        "regate_passed should default to empty HashMap when absent"
    );
}

/// `RunState` round-trips `regate_passed` through serialization.
#[test]
fn run_state_regate_passed_round_trip() {
    use std::collections::HashMap;

    let mut regate_passed = HashMap::new();
    regate_passed.insert("design".to_string(), true);
    regate_passed.insert("review".to_string(), false);

    let state = RunState {
        run_id: "01961234-0000-7000-8000-000000000000".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: "build".to_string(),
        completed_phases: vec![],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed,
        phase_start_times: HashMap::new(),
        status: RunStatus::default(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&state).expect("should serialize");
    let deserialized: RunState = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(deserialized.regate_passed.len(), 2);
    assert_eq!(deserialized.regate_passed.get("design"), Some(&true));
    assert_eq!(deserialized.regate_passed.get("review"), Some(&false));
}

/// Backwards compat: `validate: ["string"]` must still parse to `Inline`.
#[test]
fn parse_validate_inline_backwards_compat() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - "inline criterion"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 1);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, "inline criterion"),
        other => panic!("expected Inline, got {other:?}"),
    }
}

/// New: `validate: [{ file: "./path" }]` parses to `File`.
#[test]
fn parse_validate_file_reference() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - file: "./criteria/p.md"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 1);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::File { file } => assert_eq!(file, "./criteria/p.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Mixed inline and file references in one validate list.
#[test]
fn parse_validate_mixed_inline_and_file() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - "inline one"
      - file: "./criteria/p.md"
      - "inline two"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 3);
    assert!(matches!(
        &pipeline.phases[0].validate[0],
        ValidationSource::Inline(s) if s == "inline one"
    ));
    assert!(matches!(
        &pipeline.phases[0].validate[1],
        ValidationSource::File { file } if file == "./criteria/p.md"
    ));
    assert!(matches!(
        &pipeline.phases[0].validate[2],
        ValidationSource::Inline(s) if s == "inline two"
    ));
}

/// Top-level scalar starting with `./` is treated as a file reference.
#[test]
fn parse_validate_scalar_shorthand_relative_file() {
    let yaml = r"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: ./criteria/p.md
";
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 1);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::File { file } => assert_eq!(file, "./criteria/p.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Top-level scalar starting with `/` is treated as a file reference.
#[test]
fn parse_validate_scalar_shorthand_absolute_file() {
    let yaml = r"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: /abs/path/criteria.md
";
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match &pipeline.phases[0].validate[0] {
        ValidationSource::File { file } => assert_eq!(file, "/abs/path/criteria.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Top-level scalar without path prefix is treated as inline criterion.
#[test]
fn parse_validate_scalar_shorthand_inline() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: "All checks pass"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, "All checks pass"),
        other => panic!("expected Inline, got {other:?}"),
    }
}

/// Top-level scalar with a dot-prefix that is NOT a relative path ("." alone, "..foo", etc)
/// must NOT be promoted to File — the prefix match is strict: `./` or `/`.
#[test]
fn parse_validate_scalar_shorthand_non_path_dot_prefix() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: ".hidden criterion text"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, ".hidden criterion text"),
        other => panic!("expected Inline, got {other:?}"),
    }
}

/// List form is unchanged: bare strings are Inline, even if they start with ./
#[test]
fn parse_validate_list_bare_string_stays_inline_even_with_dot_prefix() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate:
      - "./should-be-inline-because-in-list"
      - file: "./actual-file.md"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 2);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, "./should-be-inline-because-in-list"),
        other => panic!("expected Inline, got {other:?}"),
    }
    match &pipeline.phases[0].validate[1] {
        ValidationSource::File { file } => assert_eq!(file, "./actual-file.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Empty list is accepted (existing behavior, no change).
#[test]
fn parse_validate_empty_list_still_parses() {
    let yaml = r"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: []
";
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 0);
}

/// Parse a phase with one produces artifact (all fields populated).
#[test]
fn parse_phase_produces_single_artifact() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Create design"
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
        description: "Brainstormed design document"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].produces.len(), 1);
    let a: &Artifact = &pipeline.phases[0].produces[0];
    assert_eq!(a.name, "design_doc");
    assert_eq!(a.path, "docs/plans/*-design.md");
    assert_eq!(
        a.description.as_deref(),
        Some("Brainstormed design document")
    );
}

/// Parse a phase with produces artifact where description is omitted.
#[test]
fn parse_phase_produces_without_description() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Create design"
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].produces.len(), 1);
    assert!(pipeline.phases[0].produces[0].description.is_none());
}

/// Phase with no produces field defaults to empty vec.
#[test]
fn parse_phase_produces_default_empty() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Create design"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert!(pipeline.phases[0].produces.is_empty());
}

/// Parse consumes as a list of short (Named) references.
#[test]
fn parse_phase_consumes_named() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: plan
    description: "Plan"
    consumes:
      - design_doc
      - requirements
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].consumes.len(), 2);
    assert!(matches!(
        &pipeline.phases[0].consumes[0],
        ArtifactRef::Named(s) if s == "design_doc"
    ));
    assert!(matches!(
        &pipeline.phases[0].consumes[1],
        ArtifactRef::Named(s) if s == "requirements"
    ));
}

/// Parse consumes as a list of qualified references.
#[test]
fn parse_phase_consumes_qualified() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: plan
    description: "Plan"
    consumes:
      - name: design_doc
        from: design
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].consumes.len(), 1);
    match &pipeline.phases[0].consumes[0] {
        ArtifactRef::Qualified { name, from } => {
            assert_eq!(name, "design_doc");
            assert_eq!(from, "design");
        }
        other => panic!("expected Qualified, got {other:?}"),
    }
}

/// Parse consumes as a mixed list of short and qualified references.
#[test]
fn parse_phase_consumes_mixed() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: execute
    description: "Execute"
    consumes:
      - plan_doc
      - name: design_doc
        from: design
      - test_cases
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].consumes.len(), 3);
    assert!(matches!(
        &pipeline.phases[0].consumes[0],
        ArtifactRef::Named(s) if s == "plan_doc"
    ));
    assert!(matches!(
        &pipeline.phases[0].consumes[1],
        ArtifactRef::Qualified { name, from } if name == "design_doc" && from == "design"
    ));
    assert!(matches!(
        &pipeline.phases[0].consumes[2],
        ArtifactRef::Named(s) if s == "test_cases"
    ));
}

/// Phase with no consumes field defaults to empty vec.
#[test]
fn parse_phase_consumes_default_empty() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert!(pipeline.phases[0].consumes.is_empty());
}

/// Parse a phase with `invoke: { skill: "/foo" }`.
#[test]
fn parse_invoke_skill_variant() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Design"
    invoke:
      skill: /brainstorming
      args:
        swarm: true
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Skill { skill, args } => {
            assert_eq!(skill, "/brainstorming");
            assert_eq!(
                args.get("swarm").and_then(serde_json::Value::as_bool),
                Some(true)
            );
        }
        other => panic!("expected Skill, got {other:?}"),
    }
}

/// Parse a phase with `invoke: { pipeline: "./sub.yml" }`.
#[test]
fn parse_invoke_pipeline_variant() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: spec-review
    description: "Spec review sub-pipeline"
    invoke:
      pipeline: ../spec-review/pipeline.yml
      with:
        iterations: 2
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Pipeline { pipeline: p, with } => {
            assert_eq!(p, "../spec-review/pipeline.yml");
            assert_eq!(
                with.get("iterations").and_then(serde_json::Value::as_u64),
                Some(2)
            );
        }
        other => panic!("expected Pipeline, got {other:?}"),
    }
}

/// Phase without `invoke` field: `invoke` is None.
#[test]
fn parse_phase_invoke_default_none() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert!(pipeline.phases[0].invoke.is_none());
}

#[test]
fn artifact_ref_external_deserializes_from_yaml() {
    let yaml = r#"
- name: prior_review
  uri: "belt://latest/feature-dev/notes/phase-review.md"
"#;
    let refs: Vec<ArtifactRef> = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(refs.len(), 1);
    match &refs[0] {
        ArtifactRef::External { name, uri } => {
            assert_eq!(name, "prior_review");
            assert!(matches!(uri, BeltUri::Latest { .. }));
        }
        other => panic!("expected External, got {other:?}"),
    }
}

#[test]
fn artifact_ref_named_still_works() {
    let refs: Vec<ArtifactRef> = serde_saphyr::from_str("- notes\n").unwrap();
    matches!(refs[0], ArtifactRef::Named(_));
}

#[test]
fn artifact_ref_qualified_still_works() {
    let yaml = r"
- name: notes
  from: review
";
    let refs: Vec<ArtifactRef> = serde_saphyr::from_str(yaml).unwrap();
    matches!(refs[0], ArtifactRef::Qualified { .. });
}

#[test]
fn run_status_serializes_as_lowercase_string() {
    assert_eq!(
        serde_json::to_string(&RunStatus::InProgress).unwrap(),
        "\"in_progress\""
    );
    assert_eq!(
        serde_json::to_string(&RunStatus::Completed).unwrap(),
        "\"completed\""
    );
    assert_eq!(
        serde_json::to_string(&RunStatus::Failed).unwrap(),
        "\"failed\""
    );
}

#[test]
fn run_status_default_is_in_progress() {
    let default: RunStatus = Default::default();
    assert_eq!(default, RunStatus::InProgress);
}

#[test]
fn run_state_new_fields_roundtrip() {
    use std::collections::HashMap;

    let state = RunState {
        run_id: "01947abc".into(),
        pipeline: "feature-dev".into(),
        pipeline_file: "/tmp/feature-dev.yml".into(),
        version: 1,
        branch: Some("main".into()),
        resolved_consumes: {
            let mut m = HashMap::new();
            m.insert(
                "belt://latest/feature-dev/notes/phase-review.md".into(),
                "/abs/.belt/runs/01947/notes/phase-review.md".into(),
            );
            m
        },
        args: HashMap::new(),
        current_phase: "review".into(),
        completed_phases: vec![],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times: HashMap::new(),
        status: RunStatus::InProgress,
        created_at: "2026-04-14T00:00:00Z".into(),
        updated_at: "2026-04-14T00:00:00Z".into(),
    };
    let json = serde_json::to_string(&state).unwrap();
    let decoded: RunState = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.branch, Some("main".into()));
    assert_eq!(decoded.resolved_consumes.len(), 1);
    assert_eq!(decoded.status, RunStatus::InProgress);
}

#[test]
fn run_state_deserializes_legacy_without_new_fields() {
    let legacy = r#"{
        "run_id": "01947abc",
        "pipeline": "feature-dev",
        "pipeline_file": "/tmp/x.yml",
        "version": 1,
        "args": {},
        "current_phase": "review",
        "completed_phases": [],
        "skipped_phases": [],
        "created_at": "2026-04-14T00:00:00Z",
        "updated_at": "2026-04-14T00:00:00Z"
    }"#;
    let decoded: RunState = serde_json::from_str(legacy).unwrap();
    assert_eq!(decoded.branch, None);
    assert!(decoded.resolved_consumes.is_empty());
    assert_eq!(decoded.status, RunStatus::InProgress);
}

/// After the 2026-04-16 subagent-boundary refactor, `invoke.agent:` is
/// no longer a valid `Invoker` variant and must fail YAML deserialisation.
#[test]
fn invoker_agent_variant_is_rejected() {
    let yaml = r"
name: p
version: 1
phases:
  - id: x
    invoke:
      agent: some-agent
";
    let result: Result<Pipeline, _> = serde_saphyr::from_str(yaml);
    assert!(
        result.is_err(),
        "parsing invoke.agent must fail after Agent variant removal"
    );
}

/// After the 2026-04-16 refactor, `invoke.agents:` is no longer valid.
#[test]
fn invoker_agents_variant_is_rejected() {
    let yaml = r"
name: p
version: 1
phases:
  - id: x
    invoke:
      agents:
        - a
        - b
";
    let result: Result<Pipeline, _> = serde_saphyr::from_str(yaml);
    assert!(
        result.is_err(),
        "parsing invoke.agents must fail after Agents variant removal"
    );
}

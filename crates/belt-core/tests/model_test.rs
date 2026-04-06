use belt_core::model::{ArgType, GateCheck, GateDefinition, Pipeline, SubPipeline};

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
        GateCheck::Cmd { cmd } => assert_eq!(cmd, "cargo build"),
        other => panic!("expected GateCheck::Cmd, got {other:?}"),
    }
}

/// Parse a phase with ALL fields populated: when, confirm, max_retries, config,
/// artifacts, multiple gate variants, validate, regate.
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
    artifacts:
      - dist/app.tar.gz
      - dist/checksum.sha256
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

    // artifacts
    assert_eq!(phase.artifacts.len(), 2);
    assert_eq!(phase.artifacts[0], "dist/app.tar.gz");

    // gate variants
    assert_eq!(phase.gate.len(), 5);
    match &phase.gate[0] {
        GateCheck::Cmd { cmd } => assert_eq!(cmd, "make test"),
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
        GateCheck::Cmd { cmd } => {
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

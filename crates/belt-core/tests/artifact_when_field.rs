//! Integration tests for `Artifact.when` field support.
//!
//! Spec: `docs/specs/2026-04-15-debug-flow-refresh-design.md` ("Artifact.when Semantics").

use std::path::PathBuf;

use belt_core::expander::expand_pipeline;
use belt_core::parser::parse_pipeline;

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

fn write_fixture(name: &str, content: &str) -> PathBuf {
    let path = fixture_path(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn artifact_when_field_is_retained_on_parse() {
    let yaml = r#"
name: test-when-field
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    invoke:
      skill: /test-skill
    produces:
      - name: conditional_artifact
        path: "output/*.md"
        when: "args.e2e"
      - name: unconditional_artifact
        path: "other/*.md"
"#;
    let fixture = write_fixture("when_field.yml", yaml);
    let pipeline = parse_pipeline(&fixture).expect("parse must succeed");
    let produces = &pipeline.phases[0].produces;
    assert_eq!(produces.len(), 2, "both artifacts must be parsed");
    assert_eq!(
        produces[0].when,
        Some("args.e2e".to_string()),
        "Artifact.when must be retained on parse"
    );
    assert_eq!(
        produces[1].when, None,
        "unconditional Artifact.when must be None"
    );
}

#[test]
fn expander_retains_artifact_when_field_via_clone() {
    let yaml = r#"
name: test-when-expander
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    description: "Phase 1"
    invoke:
      skill: /test-skill
    produces:
      - name: conditional_artifact
        path: "output/*.md"
        when: "args.e2e"
"#;
    let fixture = write_fixture("when_expander.yml", yaml);
    let expanded = expand_pipeline(&fixture).expect("expansion must succeed");
    assert_eq!(
        expanded[0].produces[0].when,
        Some("args.e2e".to_string()),
        "expander must retain Artifact.when via Clone derive"
    );
}

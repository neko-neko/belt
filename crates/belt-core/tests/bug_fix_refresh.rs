//! Integration tests for the refreshed /bug-fix pipeline.
//!
//! Shape contract (spec docs/specs/2026-04-15-debug-flow-refresh-design.md):
//! - args = { e2e: bool, codex: bool } only (iterations / swarm / ui / smoke removed)
//! - 8 phases: rca → fix-plan → fix-plan-review → execute → code-review →
//!   monkey-test → dogfood → integrate
//! - All phases use skill: invoke (no pipeline:)
//! - Review phases (fix-plan-review, code-review) pass codex
//! - code-review has regate: [execute]; no other phase has regate
//! - Supplement injection for 5 phases (rca, fix-plan, monkey-test, dogfood, integrate)
//! - criteria skill-local (6 files) + shared (execute.md, code-review.md)
//! - `rca_scenarios.when` = "args.e2e" (type-level, not just YAML text)
//! - Dead letter references removed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::match_wildcard_for_single_variants,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::collections::HashMap;
use std::path::PathBuf;

use belt_core::{
    expander::expand_pipeline,
    model::{ArgType, Artifact, ArtifactRef, GateCheck, Invoker, Phase, Pipeline},
    parser::parse_pipeline,
};

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

fn bug_fix_dir() -> PathBuf {
    repo_root().join("plugins/belt/skills/bug-fix")
}

fn bug_fix_pipeline_path() -> PathBuf {
    bug_fix_dir().join("pipeline.yml")
}

fn bug_fix_pipeline() -> Pipeline {
    parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline.yml must parse")
}

const EXPECTED_PHASES: &[&str] = &[
    "rca",
    "fix-plan",
    "fix-plan-review",
    "execute",
    "code-review",
    "monkey-test",
    "dogfood",
    "integrate",
];

#[test]
fn args_are_e2e_and_codex_only() {
    let pipeline = bug_fix_pipeline();
    let mut keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["codex", "e2e"]);

    for (name, def) in &pipeline.args {
        assert!(
            matches!(def.arg_type, ArgType::Bool),
            "arg '{name}' must be bool"
        );
    }
}

#[test]
fn no_legacy_args() {
    let pipeline = bug_fix_pipeline();
    for legacy in ["iterations", "swarm", "ui", "smoke"] {
        assert!(
            !pipeline.args.contains_key(legacy),
            "legacy arg '{legacy}' must be removed"
        );
    }
}

#[test]
fn phase_count_and_order() {
    let pipeline = bug_fix_pipeline();
    let actual: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(actual, EXPECTED_PHASES);
}

#[test]
fn all_phases_use_skill_invoke() {
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
        let invoker = phase
            .invoke
            .as_ref()
            .unwrap_or_else(|| panic!("phase '{}' must have invoke", phase.id));
        match invoker {
            Invoker::Skill { skill, .. } => {
                assert!(
                    skill.starts_with('/'),
                    "phase '{}' skill must start with '/', got '{skill}'",
                    phase.id
                );
            }
            _ => panic!(
                "phase '{}' must use Invoker::Skill variant, got {invoker:?}",
                phase.id
            ),
        }
    }
}

#[test]
fn review_phases_pass_codex_only() {
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
        if !matches!(phase.id.as_str(), "fix-plan-review" | "code-review") {
            continue;
        }
        let Some(Invoker::Skill { args, .. }) = phase.invoke.as_ref() else {
            panic!("phase '{}' must use Invoker::Skill variant", phase.id);
        };
        let mut keys: Vec<&str> = args.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["codex"],
            "phase '{}' must pass only codex",
            phase.id
        );
        assert_eq!(
            args.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{}' codex must passthrough from args",
            phase.id
        );
    }
}

#[test]
fn only_code_review_has_regate() {
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
        if phase.id == "code-review" {
            assert_eq!(
                phase.regate,
                vec!["execute".to_string()],
                "code-review must have regate == [\"execute\"]"
            );
        } else {
            assert!(
                phase.regate.is_empty(),
                "phase '{}' must have empty regate, got {:?}",
                phase.id,
                phase.regate
            );
        }
    }
}

#[test]
fn rca_scenarios_when_is_typed() {
    let pipeline = bug_fix_pipeline();
    let rca = pipeline
        .phases
        .iter()
        .find(|p| p.id == "rca")
        .expect("rca phase must exist");
    let scenarios = rca
        .produces
        .iter()
        .find(|a| a.name == "rca_scenarios")
        .expect("rca_scenarios artifact must exist");
    assert_eq!(
        scenarios.when,
        Some("args.e2e".to_string()),
        "rca_scenarios.when must parse as a typed field (not silent-dropped)"
    );
}

#[test]
fn rca_scenarios_filtered_when_e2e_false() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("expansion must succeed");
    let mut args_false: HashMap<String, serde_json::Value> = HashMap::new();
    args_false.insert("e2e".to_string(), serde_json::Value::Bool(false));
    let active = belt_core::view::active_produces(&expanded[0], &args_false);
    let names: Vec<&str> = active.iter().map(|a| a.name.as_str()).collect();
    assert!(
        !names.contains(&"rca_scenarios"),
        "rca_scenarios must be omitted when args.e2e=false, got: {names:?}"
    );
    assert!(
        names.contains(&"rca_report"),
        "rca_report must always be present, got: {names:?}"
    );
}

#[test]
fn rca_scenarios_present_when_e2e_true() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("expansion must succeed");
    let mut args_true: HashMap<String, serde_json::Value> = HashMap::new();
    args_true.insert("e2e".to_string(), serde_json::Value::Bool(true));
    let active = belt_core::view::active_produces(&expanded[0], &args_true);
    let names: Vec<&str> = active.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"rca_scenarios"));
    assert!(names.contains(&"rca_report"));
}

#[test]
fn all_phases_have_max_retries_3_and_confirm_true() {
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
        assert_eq!(
            phase.max_retries, 3,
            "phase '{}' max_retries must be 3",
            phase.id
        );
        assert!(phase.confirm, "phase '{}' confirm must be true", phase.id);
    }
}

#[test]
fn supplement_files_exist() {
    let refs_dir = bug_fix_dir().join("references");
    for name in [
        "path-convention.md",
        "rca-supplement.md",
        "fix-plan-supplement.md",
        "monkey-test-supplement.md",
        "dogfood-supplement.md",
        "worktrunk-supplement.md",
    ] {
        assert!(
            refs_dir.join(name).exists(),
            "supplement file '{name}' must exist"
        );
    }
}

#[test]
fn dead_letter_references_removed() {
    let refs_dir = bug_fix_dir().join("references");
    for name in ["evidence-plan-protocol.md", "fix-dispatch-strategy.md"] {
        assert!(
            !refs_dir.join(name).exists(),
            "dead-letter reference '{name}' must be removed"
        );
    }
}

#[test]
fn criteria_files_exist() {
    let criteria_dir = bug_fix_dir().join("criteria");
    for name in [
        "rca.md",
        "fix-plan.md",
        "fix-plan-review.md",
        "monkey-test.md",
        "dogfood.md",
        "integrate.md",
    ] {
        assert!(
            criteria_dir.join(name).exists(),
            "criteria file '{name}' must exist"
        );
    }

    // Shared criteria: after plugin migration, pipeline.yml uses `./criteria/`
    // and the shared files are physically duplicated into each plugin.
    // Drift between feature-dev and bug-fix copies is checked in
    // `shared_criteria_parity.rs`.
    for name in ["execute.md", "code-review.md"] {
        assert!(
            criteria_dir.join(name).exists(),
            "duplicated shared criteria '{name}' must exist at {}",
            criteria_dir.display()
        );
    }
}

#[test]
fn skill_md_has_expected_sections() {
    let skill_md = bug_fix_dir().join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).expect("SKILL.md must exist");
    for section in [
        "## Supplement Loading",
        "## Phase-specific Runtime Notes",
        "## Red Flags",
        "## References",
        "argument-hint:",
    ] {
        assert!(
            content.contains(section),
            "SKILL.md must contain '{section}'"
        );
    }
}

#[test]
fn skill_md_declares_supplement_injection_per_phase() {
    let skill_md = bug_fix_dir().join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).expect("SKILL.md must exist");
    // rca, fix-plan, monkey-test, dogfood, and integrate phases must each
    // reference a specific supplement inside SKILL.md's Supplement Loading table.
    for supplement in [
        "rca-supplement.md",
        "fix-plan-supplement.md",
        "monkey-test-supplement.md",
        "dogfood-supplement.md",
        "worktrunk-supplement.md",
    ] {
        assert!(
            content.contains(supplement),
            "SKILL.md must reference supplement '{supplement}'"
        );
    }
}

// --- narrative artifact shape (context reset) ---

const BUG_FIX_NARRATIVE_PHASES: &[(&str, &str, &str)] = &[
    ("rca", "rca_notes", ".belt/runs/{run_id}/notes/phase-rca.md"),
    (
        "fix-plan",
        "fix_plan_notes",
        ".belt/runs/{run_id}/notes/phase-fix-plan.md",
    ),
    (
        "execute",
        "execute_notes",
        ".belt/runs/{run_id}/notes/phase-execute.md",
    ),
    (
        "code-review",
        "code_review_notes",
        ".belt/runs/{run_id}/notes/phase-code-review.md",
    ),
    (
        "monkey-test",
        "monkey_test_notes",
        ".belt/runs/{run_id}/notes/phase-monkey-test.md",
    ),
    (
        "dogfood",
        "dogfood_notes",
        ".belt/runs/{run_id}/notes/phase-dogfood.md",
    ),
];

fn find_phase<'a>(pipeline: &'a Pipeline, id: &str) -> &'a Phase {
    pipeline
        .phases
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("phase '{id}' must exist"))
}

fn find_produce<'a>(phase: &'a Phase, name: &str) -> &'a Artifact {
    phase
        .produces
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("phase '{}' must produce '{name}'", phase.id))
}

fn has_file_exists_gate(phase: &Phase, path: &str) -> bool {
    phase
        .gate
        .iter()
        .any(|g| matches!(g, GateCheck::FileExists { file_exists } if file_exists == path))
}

fn has_named_consume(phase: &Phase, name: &str) -> bool {
    phase
        .consumes
        .iter()
        .any(|r| matches!(r, ArtifactRef::Named(n) if n == name))
}

#[test]
fn bug_fix_narrative_phases_produce_notes() {
    let pipeline = bug_fix_pipeline();
    for (phase_id, artifact_name, path) in BUG_FIX_NARRATIVE_PHASES {
        let phase = find_phase(&pipeline, phase_id);
        let note = find_produce(phase, artifact_name);
        assert_eq!(note.path, *path, "phase '{phase_id}' note path mismatch");
    }
}

#[test]
fn bug_fix_narrative_phases_gate_notes() {
    let pipeline = bug_fix_pipeline();
    for (phase_id, _, path) in BUG_FIX_NARRATIVE_PHASES {
        let phase = find_phase(&pipeline, phase_id);
        assert!(
            has_file_exists_gate(phase, path),
            "phase '{phase_id}' must gate on file_exists: '{path}'"
        );
    }
}

#[test]
fn bug_fix_narrative_accumulating_consumes() {
    let pipeline = bug_fix_pipeline();

    let expected_consumes: &[(&str, &[&str])] = &[
        ("rca", &[]),
        ("fix-plan", &["rca_notes"]),
        ("execute", &["rca_notes", "fix_plan_notes"]),
        (
            "code-review",
            &["rca_notes", "fix_plan_notes", "execute_notes"],
        ),
        (
            "monkey-test",
            &[
                "rca_notes",
                "fix_plan_notes",
                "execute_notes",
                "code_review_notes",
            ],
        ),
        (
            "dogfood",
            &[
                "rca_notes",
                "fix_plan_notes",
                "execute_notes",
                "code_review_notes",
                "monkey_test_notes",
            ],
        ),
    ];

    for (phase_id, names) in expected_consumes {
        let phase = find_phase(&pipeline, phase_id);
        for name in *names {
            assert!(
                has_named_consume(phase, name),
                "phase '{phase_id}' must consume '{name}'"
            );
        }
    }
}

#[test]
fn bug_fix_non_narrative_phases_have_no_notes() {
    let pipeline = bug_fix_pipeline();
    for phase_id in ["fix-plan-review", "integrate"] {
        let phase = find_phase(&pipeline, phase_id);
        for artifact in &phase.produces {
            assert!(
                !artifact.path.starts_with(".belt/runs/"),
                "phase '{phase_id}' must not produce belt notes, got '{}'",
                artifact.path
            );
        }
    }
}

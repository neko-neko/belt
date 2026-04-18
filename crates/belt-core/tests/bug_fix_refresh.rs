//! Integration tests for the refreshed /bug-fix pipeline.
//!
//! Shape contract (spec docs/specs/2026-04-15-debug-flow-refresh-design.md):
//! - args = { e2e: bool, codex: bool } only (iterations / swarm / ui / smoke removed)
//! - 9 phases: rca → fix-plan → fix-plan-review → pre-execute-handover → execute →
//!   code-review → monkey-test → dogfood → integrate
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
    model::{ArgType, Invoker, Pipeline},
    parser::parse_pipeline,
};

mod common;
use common::helpers::repo_root;
use common::narrative::{
    assert_narrative_accumulating_consumes, assert_narrative_gate_paths,
    assert_narrative_produce_paths, assert_non_narrative_phases_have_no_notes,
};

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
    "pre-execute-handover",
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
    // Pure-checkpoint phases (phase.invoke.is_none()) are exempt: they carry
    // no implementation work, only a file_exists gate. E.g. `pre-execute-handover`
    // is a context-reset barrier between plan and execute. The contract under
    // test is: *if* a phase invokes anything, it must be a /-prefixed skill
    // (not a sub-pipeline, not a cmd). Pure checkpoints bypass this check by
    // design.
    let pipeline = bug_fix_pipeline();
    for phase in pipeline.phases.iter().filter(|p| p.invoke.is_some()) {
        let invoker = phase
            .invoke
            .as_ref()
            .expect("filter guarantees invoke.is_some()");
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
    // Pure-checkpoint phases (phase.invoke.is_none()) are exempt from the
    // max_retries invariant: `max_retries` makes sense only when there is
    // implementation work to retry. Pure checkpoints (e.g. `pre-execute-handover`)
    // have no invoke and no retry-able body; their `max_retries` stays at the
    // serde default (0). The `confirm: true` invariant still applies to all
    // phases because a checkpoint phase is exactly where we want a human
    // confirm beat.
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
        assert!(phase.confirm, "phase '{}' confirm must be true", phase.id);
        if phase.invoke.is_some() {
            assert_eq!(
                phase.max_retries, 3,
                "phase '{}' max_retries must be 3",
                phase.id
            );
        }
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

#[test]
fn bug_fix_narrative_phases_produce_notes() {
    let pipeline = bug_fix_pipeline();
    assert_narrative_produce_paths(&pipeline, BUG_FIX_NARRATIVE_PHASES);
}

#[test]
fn bug_fix_narrative_phases_gate_notes() {
    let pipeline = bug_fix_pipeline();
    assert_narrative_gate_paths(&pipeline, BUG_FIX_NARRATIVE_PHASES);
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

    assert_narrative_accumulating_consumes(&pipeline, expected_consumes);
}

#[test]
fn bug_fix_non_narrative_phases_have_no_notes() {
    let pipeline = bug_fix_pipeline();
    assert_non_narrative_phases_have_no_notes(&pipeline, &["fix-plan-review", "integrate"]);
}

//! Integration tests for the refreshed /bug-fix pipeline.
//!
//! Shape contract (spec docs/specs/2026-04-15-debug-flow-refresh-design.md):
//! - args = { e2e: bool, codex: bool } only (iterations / swarm / ui / smoke removed)
//! - 8 phases: rca → fix-plan → fix-plan-review → execute → code-review →
//!             monkey-test → dogfood → integrate
//! - All phases use skill: invoke (no pipeline:)
//! - Review phases (fix-plan-review, code-review) pass codex
//! - code-review has regate: [execute]; no other phase has regate
//! - Supplement injection for 5 phases (rca, fix-plan, monkey-test, dogfood, integrate)
//! - criteria skill-local (6 files) + shared (execute.md, code-review.md)
//! - rca_scenarios.when = "args.e2e" (type-level, not just YAML text)
//! - Dead letter references removed.

use std::collections::HashMap;
use std::path::PathBuf;

use belt_core::{
    expander::expand_pipeline,
    model::{ArgType, Invoker, Pipeline},
    parser::parse_pipeline,
};

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

fn bug_fix_dir() -> PathBuf {
    repo_root().join("plugins/bug-fix/skills/bug-fix")
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
    keys.sort();
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
        keys.sort();
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
        "## Phase-Specific Invocation Rules",
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
    // Phases 1 (rca), 2 (fix-plan), 6 (monkey-test), 7 (dogfood), 8 (integrate)
    // must each reference a specific supplement via INVOKE 1 in SKILL.md.
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

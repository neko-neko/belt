//! Integration tests locking the 2026-07-05 sonnet-lean reviewer
//! consolidation (/belt:code-review, /belt:spec-review).
//!
//! Shape contract (docs/specs/2026-07-05-sonnet-lean-pipeline-design.md):
//! - reviewer agents are consolidated 7 -> 3:
//!   code-review  -> code-reviewer + quality-reviewer
//!   spec-review  -> spec-reviewer
//! - the seven per-observation agent files are DELETED
//! - review skills still have no pipeline.yml / belt.toml
//! - parent SKILL.md still describes parallel Task dispatch with
//!   findings-*.json artifacts and passes output_path to agents
//! - 2026-07-07 four-stage rewrite: the pipeline agent bundle is locked —
//!   belt = {code-reviewer, quality-reviewer, spec-reviewer, qa-verifier},
//!   belt-agent = {explorer, implementer}; the five pre-rewrite belt-agent
//!   agents and audit-protocol.md are DELETED, and /belt:verify is replaced
//!   by /belt:qa

#![allow(
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

mod common;
use common::helpers::repo_root;

const REVIEW_SKILLS: &[(&str, &[&str])] = &[
    ("code-review", &["code-reviewer", "quality-reviewer"]),
    ("spec-review", &["spec-reviewer"]),
];

const CONSOLIDATED_AWAY: &[&str] = &[
    "security-reviewer",
    "test-reviewer",
    "ai-antipattern-reviewer",
    "cross-cutting-reviewer",
    "feasibility-reviewer",
    "ui-design-reviewer",
    "cross-cutting-spec-reviewer",
];

const BELT_AGENT_BUNDLE: &[&str] = &["explorer", "implementer"];

const BELT_BUNDLE_EXTRA: &[&str] = &["qa-verifier"];

const RETIRED_BELT_AGENT_AGENTS: &[&str] = &[
    "code-explorer",
    "code-architect",
    "impact-analyzer",
    "feature-implementer",
    "phase-auditor",
];

#[test]
fn review_skills_pipeline_yml_is_deleted() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins/belt/skills")
            .join(skill)
            .join("pipeline.yml");
        assert!(
            !path.exists(),
            "pipeline.yml must be deleted for {skill}: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_belt_toml_is_deleted() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins/belt/skills")
            .join(skill)
            .join("belt.toml");
        assert!(
            !path.exists(),
            "belt.toml must be deleted for {skill}: {}",
            path.display()
        );
    }
}

#[test]
fn consolidated_reviewer_agents_exist() {
    for (_skill, agents) in REVIEW_SKILLS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins/belt/agents")
                .join(format!("{agent}.md"));
            assert!(
                path.exists(),
                "consolidated agent file must exist: {}",
                path.display()
            );
        }
    }
}

#[test]
fn per_observation_agent_files_are_deleted() {
    for name in CONSOLIDATED_AWAY {
        let path = repo_root()
            .join("plugins/belt/agents")
            .join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "pre-consolidation agent file must be deleted: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_parent_skill_md_references_parallel_dispatch() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins/belt/skills")
            .join(skill)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{skill} SKILL.md must describe Task dispatch with findings-*.json: {}",
            path.display()
        );
    }
}

#[test]
fn consolidated_agents_use_output_path_arg_pattern() {
    use std::fs;
    for name in [
        "spec-reviewer.md",
        "code-reviewer.md",
        "quality-reviewer.md",
    ] {
        let path = repo_root().join("plugins/belt/agents").join(name);
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            content.contains("output_path"),
            "{name} must reference 'output_path' runtime arg"
        );
        assert!(
            !content.contains(".belt/runs/"),
            "{name} must not hardcode .belt/runs/ literals"
        );
    }
}

#[test]
fn pipeline_agent_bundle_exists() {
    for agent in BELT_AGENT_BUNDLE {
        let path = repo_root()
            .join("plugins/belt-agent/agents")
            .join(format!("{agent}.md"));
        assert!(
            path.exists(),
            "belt-agent bundle agent must exist: {}",
            path.display()
        );
    }
    for agent in BELT_BUNDLE_EXTRA {
        let path = repo_root()
            .join("plugins/belt/agents")
            .join(format!("{agent}.md"));
        assert!(
            path.exists(),
            "belt bundle agent must exist: {}",
            path.display()
        );
    }
}

#[test]
fn retired_belt_agent_agents_are_deleted() {
    for name in RETIRED_BELT_AGENT_AGENTS {
        let path = repo_root()
            .join("plugins/belt-agent/agents")
            .join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "retired belt-agent agent file must be deleted: {}",
            path.display()
        );
    }
    let audit_protocol = repo_root().join("plugins/belt-agent/references/audit-protocol.md");
    assert!(
        !audit_protocol.exists(),
        "audit-protocol.md must be deleted (dead wiring): {}",
        audit_protocol.display()
    );
}

#[test]
fn verify_skill_is_replaced_by_qa() {
    let verify_dir = repo_root().join("plugins/belt/skills/verify");
    assert!(
        !verify_dir.exists(),
        "verify skill directory must be deleted: {}",
        verify_dir.display()
    );
    let qa_skill = repo_root().join("plugins/belt/skills/qa/SKILL.md");
    assert!(
        qa_skill.exists(),
        "qa skill must exist as the replacement: {}",
        qa_skill.display()
    );
}

#[test]
fn qa_verifier_uses_evidence_dir_arg_pattern() {
    let path = repo_root().join("plugins/belt/agents/qa-verifier.md");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(
        content.contains("evidence_dir"),
        "qa-verifier.md must reference the 'evidence_dir' runtime arg"
    );
    assert!(
        !content.contains(".belt/runs/"),
        "qa-verifier.md must not hardcode .belt/runs/ literals"
    );
}

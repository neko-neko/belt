//! Integration tests locking the 2026-04-16 review-skills subagent-boundary
//! refactor (/code-review, /spec-review).
//!
//! Shape contract (post-2026-04-17 consolidation):
//! - plugins/belt/skills/<skill>/pipeline.yml is DELETED for review skills
//! - plugins/belt/skills/<skill>/belt.toml is DELETED for review skills
//! - plugins/belt/agents/<consolidated>.md is DELETED (code-reviewer / spec-reviewer)
//! - New per-observation agent files exist flat under plugins/belt/agents/
//! - Parent SKILL.md references parallel Task dispatch and cross-agent merge
//! - Legacy per-observation agent files (from the pre-2026-04-15 era) remain
//!   absent (locked by the untouched LEGACY list below).

#![allow(
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

mod common;
use common::helpers::repo_root;

/// (`skill_name`, expected agent file basenames after refactor)
///
/// After the 2026-04-17 consolidation, both review skills live under the
/// single `belt` plugin, and the reviewer agents are flat under
/// `plugins/belt/agents/`. `skill_name` identifies the skill directory.
const REVIEW_SKILLS: &[(&str, &[&str])] = &[
    (
        "code-review",
        &[
            "security-reviewer",
            "test-reviewer",
            "ai-antipattern-reviewer",
            "cross-cutting-reviewer",
        ],
    ),
    (
        "spec-review",
        &[
            "feasibility-reviewer",
            "ui-design-reviewer",
            "cross-cutting-spec-reviewer",
        ],
    ),
];

#[test]
fn review_skills_pipeline_yml_is_deleted() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("skills")
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
            .join("plugins")
            .join("belt")
            .join("skills")
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
fn review_skills_legacy_consolidated_agent_is_deleted() {
    const LEGACY_CONSOLIDATED: &[&str] = &["code-reviewer", "spec-reviewer"];
    for legacy in LEGACY_CONSOLIDATED {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("agents")
            .join(format!("{legacy}.md"));
        assert!(
            !path.exists(),
            "legacy consolidated agent must be deleted: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_new_observation_agents_exist() {
    for (_skill, agents) in REVIEW_SKILLS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins")
                .join("belt")
                .join("agents")
                .join(format!("{agent}.md"));
            assert!(
                path.exists(),
                "new observation agent file must exist: {}",
                path.display()
            );
        }
    }
}

#[test]
fn review_skills_parent_skill_md_references_parallel_dispatch() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("skills")
            .join(skill)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{skill} SKILL.md must describe parallel Task dispatch with findings-*.json: {}",
            path.display()
        );
    }
}

#[test]
fn legacy_per_observation_review_agent_files_are_removed() {
    // Untouched from 2026-04-15 refresh — locks the previous invariant that
    // the pre-refresh per-observation agent bundle is gone.
    const LEGACY: &[&str] = &[
        "code-review-quality",
        "code-review-security",
        "code-review-performance",
        "code-review-test",
        "code-review-ai-antipattern",
        "code-review-impact",
        "spec-review-requirements",
        "spec-review-design-judgment",
        "spec-review-feasibility",
        "spec-review-consistency",
        "spec-review-ui-design",
        "test-review-coverage",
        "test-review-quality",
        "test-review-design-alignment",
        "implementation-review-clarity",
        "implementation-review-feasibility",
        "implementation-review-consistency",
        "implementation-review-ui-spec",
    ];
    let agents_dir = repo_root().join(".claude/agents");
    for name in LEGACY {
        let path = agents_dir.join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "legacy agent file must remain deleted: {}",
            path.display()
        );
    }
}

#[test]
fn per_observation_agents_use_output_path_arg_pattern() {
    // 2026-04-18 belt://current URI migration: per-observation reviewer
    // agents must receive their target findings path as a runtime `output_path`
    // arg (injected by the parent skill), never hardcode `.belt/runs/`
    // literals. The parent resolves the URI; the agent writes to whatever
    // path is passed in.
    use std::fs;
    let agents = [
        "security-reviewer.md",
        "test-reviewer.md",
        "ai-antipattern-reviewer.md",
        "cross-cutting-reviewer.md",
        "feasibility-reviewer.md",
        "cross-cutting-spec-reviewer.md",
        "ui-design-reviewer.md",
    ];
    for name in agents {
        let path = repo_root().join("plugins/belt/agents").join(name);
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            content.contains("output_path"),
            "{name} must reference 'output_path' runtime arg in Output Format section"
        );
        assert!(
            !content.contains(".belt/runs/"),
            "{name} must not hardcode .belt/runs/ literals"
        );
    }
}

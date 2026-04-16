//! Integration tests locking the 2026-04-16 review-skills subagent-boundary
//! refactor (/code-review, /spec-review).
//!
//! Shape contract:
//! - plugins/<plugin>/skills/<plugin>/pipeline.yml is DELETED
//! - plugins/<plugin>/skills/<plugin>/belt.toml is DELETED
//! - plugins/<plugin>/agents/<single>.md is DELETED (code-reviewer / spec-reviewer)
//! - New per-observation agent files exist in plugins/<plugin>/agents/
//! - Parent SKILL.md references parallel Task dispatch and cross-agent merge
//! - Legacy per-observation agent files (from the pre-2026-04-15 era) remain
//!   absent (locked by the untouched LEGACY list below).

#![allow(
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

/// (plugin, expected agent file basenames after refactor)
const REVIEW_PLUGINS: &[(&str, &[&str])] = &[
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
fn review_plugins_pipeline_yml_is_deleted() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("pipeline.yml");
        assert!(
            !path.exists(),
            "pipeline.yml must be deleted for {plugin}: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_belt_toml_is_deleted() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("belt.toml");
        assert!(
            !path.exists(),
            "belt.toml must be deleted for {plugin}: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_legacy_consolidated_agent_is_deleted() {
    const LEGACY_CONSOLIDATED: &[(&str, &str)] = &[
        ("code-review", "code-reviewer"),
        ("spec-review", "spec-reviewer"),
    ];
    for (plugin, legacy) in LEGACY_CONSOLIDATED {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
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
fn review_plugins_new_observation_agents_exist() {
    for (plugin, agents) in REVIEW_PLUGINS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins")
                .join(plugin)
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
fn review_plugins_parent_skill_md_references_parallel_dispatch() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{plugin} SKILL.md must describe parallel Task dispatch with findings-*.json: {}",
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

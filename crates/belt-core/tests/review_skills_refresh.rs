//! Integration tests locking the refreshed review-skill pipelines
//! (/code-review, /spec-review, /implementation-review).
//!
//! Shape contract:
//! - args = { codex: bool } only (no iterations, swarm, ui)
//! - phases = [review, fix], review.invoke.agents = [<skill>-reviewer]
//! - consolidated agent files exist; legacy per-observation files are removed.

use std::path::PathBuf;

use belt_core::{error::BeltError, model::ArgType, parser::parse_pipeline};

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

fn pipeline_path(skill: &str) -> PathBuf {
    repo_root().join(format!("examples/skills/{skill}/pipeline.yml"))
}

const REVIEW_SKILLS: &[(&str, &str, &str)] = &[
    ("code-review", "code-reviewer", "code-review"),
    ("spec-review", "spec-reviewer", "spec-review"),
    (
        "implementation-review",
        "implementation-reviewer",
        "implementation-review",
    ),
];

#[test]
fn review_skills_args_are_codex_only() -> Result<(), BeltError> {
    for (skill, _agent, _label) in REVIEW_SKILLS {
        let pipeline = parse_pipeline(&pipeline_path(skill))?;
        let mut names: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec!["codex"],
            "{skill} args must be exactly {{codex}}"
        );
        let codex = pipeline
            .args
            .get("codex")
            .ok_or_else(|| BeltError::InvalidPipeline {
                message: format!("{skill} codex arg missing"),
            })?;
        assert!(
            matches!(codex.arg_type, ArgType::Bool),
            "{skill} codex arg must be bool"
        );
        assert_eq!(
            codex.default.as_ref().and_then(serde_json::Value::as_bool),
            Some(false),
            "{skill} codex default must be false"
        );
    }
    Ok(())
}

#[test]
fn review_skills_have_two_phases_review_then_fix() -> Result<(), BeltError> {
    for (skill, _agent, _label) in REVIEW_SKILLS {
        let pipeline = parse_pipeline(&pipeline_path(skill))?;
        let ids: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["review", "fix"],
            "{skill} phases must be [review, fix]"
        );
    }
    Ok(())
}

#[test]
fn review_skills_invoke_single_consolidated_agent() -> Result<(), BeltError> {
    // belt-core's typed Invoker model may not expose `agents` directly; assert
    // by parsing the raw YAML.
    for (skill, agent, _label) in REVIEW_SKILLS {
        let yaml = std::fs::read_to_string(pipeline_path(skill))?;
        let doc: serde_json::Value =
            serde_saphyr::from_str(&yaml).map_err(|e| BeltError::YamlParse {
                message: e.to_string(),
                src: Some(yaml.clone()),
            })?;
        let review_phase = doc
            .get("phases")
            .and_then(serde_json::Value::as_array)
            .and_then(|phases| {
                phases
                    .iter()
                    .find(|p| p.get("id").and_then(serde_json::Value::as_str) == Some("review"))
            })
            .ok_or_else(|| BeltError::InvalidPipeline {
                message: format!("{skill} review phase missing"),
            })?;
        let agents = review_phase
            .get("invoke")
            .and_then(|i| i.get("agents"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| BeltError::InvalidPipeline {
                message: format!("{skill} review.invoke.agents missing"),
            })?;
        let agent_list: Vec<&str> = agents
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        assert_eq!(
            agent_list,
            vec![*agent],
            "{skill} review.invoke.agents must be [{agent}]"
        );
    }
    Ok(())
}

#[test]
fn review_skills_consolidated_agent_files_exist() {
    let agents_dir = repo_root().join(".claude/agents");
    for (_skill, agent, _label) in REVIEW_SKILLS {
        let path = agents_dir.join(format!("{agent}.md"));
        assert!(
            path.exists(),
            "consolidated agent file must exist: {}",
            path.display()
        );
    }
}

#[test]
fn legacy_review_agent_files_are_removed() {
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
            "legacy agent file must be deleted: {}",
            path.display()
        );
    }
}

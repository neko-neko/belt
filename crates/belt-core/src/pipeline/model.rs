//! Pipeline model types deserialized from `pipeline.yml`.
//!
//! This module defines the Rust representation of `pipeline.yml` documents as
//! written by rule set authors. Every public field corresponds one-to-one with
//! a key in the spec (§Pipeline Model / §Flag System SSOT), and the types
//! intentionally err on the side of forward-compatibility: unknown or
//! extension-only fields are left either as `Option<String>` (rewind targets,
//! phase files) or as dynamic [`yaml::Value`] sequences (`uses`, lifecycle
//! hook lists) so later phases can introduce richer semantics without breaking
//! the Phase 1 parser.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::yaml;

/// Top-level document kind. Pipeline documents always declare `kind: pipeline`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineKind {
    /// The only legal value for the `kind` field of a `pipeline.yml` document.
    Pipeline,
}

/// A pipeline definition (root of `pipeline.yml`).
///
/// Deserialized via `serde` from `pipeline.yml`. All optional blocks default
/// to empty collections so minimal pipelines that declare only `kind`/`name`/
/// `version`/`phases` parse cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Document kind discriminator. Must be `pipeline`.
    pub kind: PipelineKind,
    /// Pipeline identifier (e.g. `feature-dev`).
    pub name: String,
    /// Schema version of this `pipeline.yml`.
    pub version: u32,
    /// Free-form description for humans; not consumed by the runtime.
    #[serde(default)]
    pub description: Option<String>,
    /// Ordered list of rule set paths to import before evaluation.
    #[serde(default)]
    pub imports: Vec<String>,
    /// CLI flag declarations keyed by the flag's long form (e.g. `--linear`).
    #[serde(default)]
    pub flags: BTreeMap<String, FlagDef>,
    /// Pipeline-level settings (arbitrary YAML values, runtime-specific).
    #[serde(default)]
    pub settings: BTreeMap<String, yaml::Value>,
    /// Declared artifacts produced and consumed by the phases.
    #[serde(default)]
    pub artifacts: BTreeMap<String, Artifact>,
    /// Ordered list of phases this pipeline executes.
    #[serde(default)]
    pub phases: Vec<Phase>,
    /// Pipeline-level `uses:` directives (rule set invocations). Stored as
    /// dynamic YAML values for forward-compatibility with future `uses` syntax.
    #[serde(default)]
    pub uses: Vec<yaml::Value>,
    /// Lifecycle triggers (e.g. regate rewinds driven by directive results).
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    /// Integration plugin declarations (Linear, Slack, etc.).
    #[serde(default)]
    pub integrations: Vec<Integration>,
    /// Skill/rule file to execute before the first phase starts.
    #[serde(default)]
    pub pre_pipeline_start: Option<PrePipelineStart>,
    /// Hook directives to run when the pipeline begins. Stored dynamically
    /// because the directive vocabulary is phase-specific.
    #[serde(default)]
    pub on_pipeline_start: Vec<yaml::Value>,
    /// Hook directives to run after the pipeline completes successfully.
    #[serde(default)]
    pub on_pipeline_complete: Vec<yaml::Value>,
}

/// Flag definition (from the `flags:` block).
///
/// Follows the spec §Flag System SSOT: a flag has a `type` (bool / string /
/// int), an optional `default`, may `enable` integrations/phases/params, and
/// may bind to a rule set parameter via `binds_to_param`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagDef {
    /// Flag type name (`bool`, `string`, `int`, ...). Defaults to `bool`
    /// when absent so that simple presence flags can omit the field.
    #[serde(default = "default_flag_type", rename = "type")]
    pub flag_type: String,
    /// Optional default value (typed dynamically to match the declared type).
    #[serde(default)]
    pub default: Option<yaml::Value>,
    /// What this flag enables when set (integrations / phases / params).
    #[serde(default)]
    pub enables: Option<FlagEnables>,
    /// Optional parameter binding: when the flag is set, bind its value to
    /// `rule_set.param` at evaluation time.
    #[serde(default)]
    pub binds_to_param: Option<BindsToParam>,
}

/// Nested `enables:` block inside a flag definition (spec §Flag System SSOT).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlagEnables {
    /// Integration names that become active when the flag is set.
    #[serde(default)]
    pub integrations: Vec<String>,
    /// Phase ids that become reachable when the flag is set.
    #[serde(default)]
    pub phases: Vec<String>,
    /// Rule-set-scoped parameters overrides. Keyed by rule set name, then by
    /// parameter name, with arbitrary YAML scalar values.
    #[serde(default)]
    pub params: BTreeMap<String, BTreeMap<String, yaml::Value>>,
}

fn default_flag_type() -> String {
    "bool".to_string()
}

/// Binds a pipeline flag to a rule set parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindsToParam {
    /// Target rule set name.
    pub rule_set: String,
    /// Parameter name inside the rule set.
    pub param: String,
}

/// Artifact declaration inside a pipeline's `artifacts:` mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Artifact kind (`file`, `directory`, `commit`, ...).
    #[serde(rename = "type")]
    pub artifact_type: String,
    /// Optional glob pattern identifying the artifact location.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Phase id that produces this artifact (at most one).
    #[serde(default)]
    pub produced_by: Option<String>,
    /// Phase ids that consume this artifact.
    #[serde(default)]
    pub consumed_by: Vec<String>,
}

/// A phase inside a pipeline's `phases:` sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    /// Stable phase identifier (e.g. `design`, `plan`, `implement`).
    pub id: String,
    /// Confirmation directive: `before`, `after`, or `none`.
    #[serde(default)]
    pub confirm: Option<String>,
    /// Optional path to a phase file (external rule set document).
    #[serde(default)]
    pub phase_file: Option<String>,
    /// Optional condition that disables this phase when evaluated `false`.
    #[serde(default)]
    pub skip_unless: Option<String>,
    /// Rule set invocations scoped to this phase.
    #[serde(default)]
    pub uses: Vec<yaml::Value>,
}

/// An integration plugin (Linear, Slack, etc.) enabled via `--<flag>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    /// Plugin name (matches an entry in `FlagEnables::integrations`).
    pub name: String,
    /// Flag long name (`--linear`) that activates this integration.
    #[serde(default)]
    pub enabled_by: Option<String>,
    /// Hook commands keyed by lifecycle event name.
    #[serde(default)]
    pub hooks: BTreeMap<String, HookCommand>,
    /// Optional skill file invoked when the integration boots.
    #[serde(default)]
    pub pre_pipeline_start: Option<PrePipelineStart>,
}

/// Hook command value — accepts either a single string or a list of strings.
///
/// Matches the existing `feature-dev` `pipeline.yml` style where a hook may be
/// either `on_phase_complete: sync_phase_summary` (single command) or
/// `on_phase_complete: [sync_phase_summary, sync_evidence]` (list).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookCommand {
    /// A single hook command string.
    Single(String),
    /// Multiple hook command strings executed in declaration order.
    Multiple(Vec<String>),
}

impl HookCommand {
    /// Returns the hook commands as a `Cow<[String]>`.
    ///
    /// For the `Multiple` variant this is a cheap borrow; for the `Single`
    /// variant a one-element `Vec` is allocated to present a uniform slice
    /// view.
    #[must_use]
    pub fn as_slice(&self) -> std::borrow::Cow<'_, [String]> {
        match self {
            HookCommand::Single(s) => std::borrow::Cow::Owned(vec![s.clone()]),
            HookCommand::Multiple(v) => std::borrow::Cow::Borrowed(v),
        }
    }
}

/// Pre-pipeline-start hook — points at a skill file to invoke before the first
/// phase executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePipelineStart {
    /// Relative path to the skill file.
    pub skill_file: String,
}

/// Trigger declaration inside `triggers:` — fires on a lifecycle event and
/// optionally rewinds the pipeline to a named phase (used by regate).
///
/// Fields are minimal for Phase 1: `event` identifies the trigger type,
/// `rewind_to` is the target phase name (validated by Task 14 lint rule), and
/// `rewind_strategy` picks the regate strategy file. `when` holds an optional
/// guard expression. Richer semantics (conditions, bodies) are added in later
/// phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Lifecycle event name that fires this trigger (e.g. `on_directive_fail`).
    pub event: String,
    /// Phase id to rewind to, if this trigger performs a regate.
    #[serde(default)]
    pub rewind_to: Option<String>,
    /// Strategy file for the regate (e.g. `rules/regate/restart.yml`).
    #[serde(default)]
    pub rewind_strategy: Option<String>,
    /// Optional guard expression; evaluated against the current pipeline state.
    #[serde(default)]
    pub when: Option<String>,
}

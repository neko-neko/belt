use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level pipeline definition loaded from a pipeline YAML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: u32,
    #[serde(default)]
    pub args: HashMap<String, ArgDef>,
    pub phases: Vec<Phase>,
}

/// Argument definition for pipeline-level `args`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgDef {
    #[serde(rename = "type")]
    pub arg_type: ArgType,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

/// Supported argument / input types for pipeline args and sub-pipeline inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    Bool,
    String,
    Number,
    List,
}

/// A single phase within a pipeline or sub-pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub with: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub invoke: Option<Invoker>,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub produces: Vec<Artifact>,
    #[serde(default)]
    pub consumes: Vec<ArtifactRef>,
    #[serde(default)]
    pub gate: Vec<GateCheck>,
    #[serde(default, deserialize_with = "validate_de::deserialize")]
    pub validate: Vec<ValidationSource>,
    #[serde(default)]
    pub regate: Vec<String>,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub max_retries: u32,
}

/// A single gate check. Deserialized as untagged enum so the YAML shape
/// determines the variant (e.g. `cmd: "..."` vs `file_exists: "..."`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GateCheck {
    Cmd {
        cmd: String,
        #[serde(default = "default_gate_timeout")]
        timeout: u64,
    },
    FileExists {
        file_exists: String,
    },
    GitClean {
        git_clean: bool,
    },
    HasOutput {
        has_output: bool,
    },
    Uses {
        uses: String,
        #[serde(default)]
        with: HashMap<String, serde_json::Value>,
    },
}

/// A single validation criterion. Either an inline string that the
/// orchestrator evaluates directly, or a reference to a markdown file whose
/// contents are the criteria. The file form replaces the audit-gate
/// sub-pipeline pattern used in BELT-20 MVP examples.
///
/// Ordering is significant for serde-saphyr untagged enum deserialization:
/// `Inline` (scalar string) is checked before `File` (mapping with a `file`
/// key), matching the `GateCheck` precedent where more specific struct
/// variants come after scalar variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValidationSource {
    Inline(String),
    File { file: String },
}

mod validate_de {
    use super::ValidationSource;
    use serde::de::{Deserializer, SeqAccess, Visitor};
    use std::fmt;

    /// Custom deserializer for `Phase.validate`.
    ///
    /// Accepts either:
    /// - A YAML scalar (string): if the scalar starts with `./` or `/`, it
    ///   is wrapped as `vec![ValidationSource::File { file }]`; otherwise as
    ///   `vec![ValidationSource::Inline(s)]`.
    /// - A YAML sequence: delegates to the stock `Vec<ValidationSource>`
    ///   deserializer which in turn uses the untagged enum discrimination.
    ///
    /// This is the "scalar shorthand" described in Plan B spec DD-2.
    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<ValidationSource>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = Vec<ValidationSource>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or a sequence of validation sources")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(classify_scalar(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(classify_scalar(value))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut out = Vec::new();
                while let Some(item) = seq.next_element::<ValidationSource>()? {
                    out.push(item);
                }
                Ok(out)
            }
        }

        deserializer.deserialize_any(V)
    }

    /// Scalar classification rule: `./` or `/` prefix → File; else Inline.
    /// The prefix match is strict: only these two prefixes qualify.
    fn classify_scalar(s: String) -> Vec<ValidationSource> {
        if s.starts_with("./") || s.starts_with('/') {
            vec![ValidationSource::File { file: s }]
        } else {
            vec![ValidationSource::Inline(s)]
        }
    }
}

/// A typed artifact produced by a phase. The `name` is a logical identifier
/// by which later phases reference the artifact via `consumes:`. The `path`
/// is the filesystem path the LLM is expected to produce (glob permitted for
/// runtime-determined filenames like `docs/plans/*-design.md`).
///
/// Glob resolution semantics are intentionally not specified here; they are
/// deferred to the Plan B examples migration implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// A reference to an artifact produced by an earlier phase. `Named` is the
/// short form — lint resolves it to the most recent earlier phase that
/// produced that name. `Qualified` disambiguates when multiple earlier phases
/// produce the same name.
///
/// Ordering: `Named` (scalar string) is checked before `Qualified` (struct
/// mapping) for serde-saphyr untagged enum disambiguation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArtifactRef {
    Named(String),
    Qualified { name: String, from: String },
}

/// Number of iterations for an `Invoker::Agents` invocation.
///
/// Accepts either a literal integer (`iterations: 3`) or a template
/// string referencing pipeline args (`iterations: "args.iterations"`).
/// Template resolution is performed by the skill orchestrator at
/// dispatch time, not at parse time. The default is `Literal(0)` so
/// that omitting the field is identical to legacy `iterations: 0`.
///
/// Variant ordering for serde-saphyr untagged enum disambiguation:
/// `Literal` (YAML integer) is listed before `Template` (YAML string) so
/// numeric literals are not coerced into strings. This mirrors the
/// `GateCheck` ordering lesson (see `docs/specs` Known Risks).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum IterationsSpec {
    Literal(u32),
    Template(String),
}

impl Default for IterationsSpec {
    fn default() -> Self {
        Self::Literal(0)
    }
}

/// Typed invocation target for a phase. Parallel to the existing `GateCheck`
/// untagged enum: belt-core models the invocation shape but the LLM
/// orchestrator is responsible for actually dispatching the skill, agent, or
/// sub-pipeline at runtime.
///
/// Variant ordering for serde-saphyr untagged enum disambiguation:
/// `Skill` (field: `skill`) → `Agent` (field: `agent`) → `Agents` (field:
/// `agents`) → `Pipeline` (field: `pipeline`). Each variant has a unique
/// required discriminating field, so ordering is defensive rather than
/// strictly necessary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Invoker {
    Skill {
        skill: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Agent {
        agent: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Agents {
        agents: Vec<String>,
        #[serde(default)]
        iterations: IterationsSpec,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Pipeline {
        pipeline: String,
        #[serde(default)]
        with: HashMap<String, serde_json::Value>,
    },
}

/// Reusable gate definition (standalone YAML file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDefinition {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub inputs: HashMap<String, InputDef>,
    pub checks: Vec<GateCheck>,
}

/// Input definition for gate definitions and sub-pipelines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDef {
    #[serde(rename = "type")]
    pub input_type: ArgType,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub required: bool,
}

/// A sub-pipeline that can be referenced via `invoke: { pipeline: ... }` from a phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubPipeline {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: u32,
    #[serde(default)]
    pub inputs: HashMap<String, InputDef>,
    pub phases: Vec<Phase>,
}

/// Persisted run state for a pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub run_id: String,
    pub pipeline: String,
    pub pipeline_file: String,
    pub version: u32,
    pub args: HashMap<String, serde_json::Value>,
    pub current_phase: String,
    pub completed_phases: Vec<String>,
    pub skipped_phases: Vec<String>,
    #[serde(default)]
    pub phase_attempts: HashMap<String, u32>,
    #[serde(default)]
    pub phase_verify_passed: HashMap<String, bool>,
    #[serde(default)]
    pub regate_passed: HashMap<String, bool>,
    /// Wall-clock time (UTC) at which each phase was first entered.
    /// Written once per phase on `init` and `step`; retries and regate do
    /// not update it. Used by the view layer to scope glob-based artifact
    /// resolution to files created during the phase's active window.
    #[serde(default)]
    pub phase_start_times: HashMap<String, DateTime<Utc>>,
    pub created_at: String,
    pub updated_at: String,
}

/// A phase after sub-pipeline resolution (via `invoke: { pipeline: ... }`)
/// and template expansion. All fields from the referenced sub-pipeline / gate
/// are merged in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedPhase {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub produces: Vec<Artifact>,
    #[serde(default)]
    pub consumes: Vec<ArtifactRef>,
    #[serde(default)]
    pub gate: Vec<GateCheck>,
    #[serde(default, deserialize_with = "validate_de::deserialize")]
    pub validate: Vec<ValidationSource>,
    #[serde(default)]
    pub regate: Vec<String>,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub invoke: Option<Invoker>,
    /// `output_dir` is computed at runtime, not from YAML.
    #[serde(skip)]
    pub output_dir: Option<String>,
}

/// Default gate command timeout in seconds (30 minutes).
fn default_gate_timeout() -> u64 {
    1800
}

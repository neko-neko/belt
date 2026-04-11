use crate::error::{BeltError, BeltResult};
use crate::model::{ExpandedPhase, Phase, SubPipeline};
use crate::parser::{parse_pipeline, parse_sub_pipeline};
use std::path::Path;

/// Parse a pipeline and expand all `uses:` references into flat, namespaced phases.
///
/// Each phase with a `uses:` key references a sub-pipeline YAML file (resolved
/// relative to `pipeline_path`'s directory). The sub-pipeline's phases are
/// flattened with namespaced IDs (`{parent_id}/{sub_phase_id}`).
///
/// Inheritance rules for the **last** sub-phase of each `uses:` expansion:
/// - `gate`, `regate`, `validate`: parent entries are **appended**
/// - `config`: merged with parent keys winning on conflict
///
/// **All** sub-phases inherit the parent's `when` if they lack their own.
///
/// A leaf phase (no `uses:`) **must** have a `description`; otherwise
/// `BeltError::InvalidPipeline` is returned.
pub fn expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>> {
    let pipeline = parse_pipeline(pipeline_path)?;
    let base_dir = pipeline_path.parent().unwrap_or_else(|| Path::new("."));

    let mut expanded = Vec::new();
    for phase in &pipeline.phases {
        if let Some(uses) = &phase.uses {
            let sub_path = base_dir.join(uses);
            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_phases = expand_sub_pipeline(&phase.id, phase, &sub);
            expanded.extend(sub_phases);
        } else {
            expanded.push(leaf_phase(phase)?);
        }
    }
    Ok(expanded)
}

fn expand_sub_pipeline(parent_id: &str, parent: &Phase, sub: &SubPipeline) -> Vec<ExpandedPhase> {
    let mut phases = Vec::new();
    for (i, sub_phase) in sub.phases.iter().enumerate() {
        let namespaced_id = format!("{parent_id}/{}", sub_phase.id);
        let is_last = i == sub.phases.len() - 1;

        // Merge config: parent config overrides sub-phase config (on last sub-phase only)
        let mut merged_config = sub_phase.config.clone();
        if is_last {
            for (k, v) in &parent.config {
                merged_config.insert(k.clone(), v.clone());
            }
        }

        // Last sub-phase inherits parent's gate, regate, validate
        let mut gate = sub_phase.gate.clone();
        let mut regate = sub_phase.regate.clone();
        let mut validate = sub_phase.validate.clone();

        if is_last {
            gate.extend(parent.gate.clone());
            regate.extend(parent.regate.clone());
            validate.extend(parent.validate.clone());
        }

        // when: sub-phase inherits parent's when if it doesn't have its own
        let when = sub_phase.when.clone().or_else(|| parent.when.clone());

        phases.push(ExpandedPhase {
            id: namespaced_id,
            description: sub_phase.description.clone().unwrap_or_default(),
            config: merged_config,
            artifacts: sub_phase.artifacts.clone(),
            produces: sub_phase.produces.clone(),
            gate,
            validate,
            regate,
            confirm: sub_phase.confirm,
            max_retries: sub_phase.max_retries,
            when,
            output_dir: None,
        });
    }
    phases
}

fn leaf_phase(phase: &Phase) -> BeltResult<ExpandedPhase> {
    let description = phase
        .description
        .clone()
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: format!("leaf phase '{}' must have a description", phase.id),
        })?;
    Ok(ExpandedPhase {
        id: phase.id.clone(),
        description,
        config: phase.config.clone(),
        artifacts: phase.artifacts.clone(),
        produces: phase.produces.clone(),
        gate: phase.gate.clone(),
        validate: phase.validate.clone(),
        regate: phase.regate.clone(),
        confirm: phase.confirm,
        max_retries: phase.max_retries,
        when: phase.when.clone(),
        output_dir: None,
    })
}

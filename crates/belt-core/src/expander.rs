use crate::error::{BeltError, BeltResult};
use crate::model::{ExpandedPhase, Invoker, Phase, SubPipeline};
use crate::parser::{parse_pipeline, parse_sub_pipeline};
use std::path::Path;

/// Parse a pipeline and expand all `invoke: { pipeline: ... }` references into
/// flat, namespaced phases.
///
/// Each phase whose `invoke` is an [`Invoker::Pipeline`] references a
/// sub-pipeline YAML file (resolved relative to `pipeline_path`'s directory).
/// The sub-pipeline's phases are flattened with namespaced IDs
/// (`{parent_id}/{sub_phase_id}`).
///
/// Inheritance rules for the **last** sub-phase of each expansion:
/// - `gate`, `regate`, `validate`: parent entries are **appended**
/// - `config`: merged with parent keys winning on conflict
///
/// **All** sub-phases inherit the parent's `when` if they lack their own.
///
/// A leaf phase (no `invoke: { pipeline: ... }`) **must** have a `description`;
/// otherwise `BeltError::InvalidPipeline` is returned.
pub fn expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>> {
    let pipeline = parse_pipeline(pipeline_path)?;
    let base_dir = pipeline_path.parent().unwrap_or_else(|| Path::new("."));

    let mut expanded = Vec::new();
    for phase in &pipeline.phases {
        if let Some(Invoker::Pipeline { pipeline, .. }) = &phase.invoke {
            let sub_path = base_dir.join(pipeline);
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
            produces: sub_phase.produces.clone(),
            consumes: sub_phase.consumes.clone(),
            gate,
            validate,
            regate,
            confirm: sub_phase.confirm,
            max_retries: sub_phase.max_retries,
            when,
            invoke: sub_phase.invoke.clone(),
            output_dir: None,
        });
    }
    phases
}

fn leaf_phase(phase: &Phase) -> BeltResult<ExpandedPhase> {
    // Sanity: if the phase has `invoke: { pipeline: ... }`, it should have
    // been handled by the sub-pipeline branch in `expand_pipeline`. Hitting
    // this case is a bug in the expander branching logic, not a user error.
    debug_assert!(
        !matches!(phase.invoke, Some(Invoker::Pipeline { .. })),
        "leaf_phase called with Invoker::Pipeline — expander branch logic is wrong"
    );

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
        produces: phase.produces.clone(),
        consumes: phase.consumes.clone(),
        gate: phase.gate.clone(),
        validate: phase.validate.clone(),
        regate: phase.regate.clone(),
        confirm: phase.confirm,
        max_retries: phase.max_retries,
        when: phase.when.clone(),
        invoke: phase.invoke.clone(),
        output_dir: None,
    })
}

/// If `template` has the exact shape `"args.<name>"` and `<name>` is a key in
/// `with_map`, returns `Some(with_map["<name>"].clone())`. Otherwise returns
/// `None`. The check is a full-string equality; substring or interpolated
/// forms (e.g. `"${args.port}"`) are intentionally excluded.
// Wired up by subsequent tasks in the expander with-merge plan; tests in the
// `#[cfg(test)] mod tests` block below exercise it in isolation for now.
#[cfg_attr(not(test), allow(dead_code))]
fn substitute_arg_in_value(
    template: &str,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    let name = template.strip_prefix("args.")?;
    if name.is_empty() || name.contains('.') {
        return None;
    }
    with_map.get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    fn mk_with(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn substitute_returns_none_when_template_is_not_args_dot_form() {
        let w = mk_with(&[("x", json!(5))]);
        assert!(substitute_arg_in_value("hello", &w).is_none());
        assert!(substitute_arg_in_value("args.", &w).is_none());
        assert!(substitute_arg_in_value("args.x.y", &w).is_none());
    }

    #[test]
    fn substitute_returns_none_when_name_not_in_with() {
        let w = mk_with(&[("x", json!(5))]);
        assert!(substitute_arg_in_value("args.y", &w).is_none());
    }

    #[test]
    fn substitute_rewrites_identity_template_to_template() {
        let w = mk_with(&[("count", json!("args.iterations"))]);
        assert_eq!(
            substitute_arg_in_value("args.count", &w),
            Some(json!("args.iterations"))
        );
    }

    #[test]
    fn substitute_rewrites_to_number_literal() {
        let w = mk_with(&[("count", json!(5))]);
        assert_eq!(substitute_arg_in_value("args.count", &w), Some(json!(5)));
    }

    #[test]
    fn substitute_rewrites_to_bool_literal() {
        let w = mk_with(&[("enabled", json!(true))]);
        assert_eq!(
            substitute_arg_in_value("args.enabled", &w),
            Some(json!(true))
        );
    }

    #[test]
    fn substitute_rewrites_to_non_template_string() {
        let w = mk_with(&[("name", json!("foo"))]);
        assert_eq!(substitute_arg_in_value("args.name", &w), Some(json!("foo")));
    }

    #[test]
    fn substitute_rewrites_to_null() {
        let w = mk_with(&[("x", Value::Null)]);
        assert_eq!(substitute_arg_in_value("args.x", &w), Some(Value::Null));
    }
}

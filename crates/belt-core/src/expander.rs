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
        if let Some(Invoker::Pipeline { pipeline, with }) = &phase.invoke {
            let sub_path = base_dir.join(pipeline);
            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_phases = expand_sub_pipeline(&phase.id, phase, &sub, with);
            expanded.extend(sub_phases);
        } else {
            expanded.push(leaf_phase(phase)?);
        }
    }
    Ok(expanded)
}

fn expand_sub_pipeline(
    parent_id: &str,
    parent: &Phase,
    sub: &SubPipeline,
    with: &std::collections::HashMap<String, serde_json::Value>,
) -> Vec<ExpandedPhase> {
    let mut phases = Vec::new();
    for (i, sub_phase) in sub.phases.iter().enumerate() {
        let namespaced_id = format!("{parent_id}/{}", sub_phase.id);
        let is_last = i == sub.phases.len() - 1;

        // Merge config: parent config overrides sub-phase config (on last sub-phase only).
        // Substitute against sub_phase's OWN config first — parent config values are
        // authored in the parent's arg scope and must not be rewritten against the
        // sub-pipeline's `with` map. Mirrors the I1 `when` scope rule fix (6493cf2).
        let mut merged_config = sub_phase.config.clone();
        if !with.is_empty() {
            substitute_in_value_map(&mut merged_config, with);
        }
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

        // Substitute against sub_phase's OWN when first. Parent's when is evaluated
        // in the parent run's arg scope and must not be rewritten against the
        // sub-pipeline's `with` map.
        let mut sub_when = sub_phase.when.clone();
        if !with.is_empty() {
            if let Some(w_str) = sub_when.as_deref() {
                if let Some(replacement) = substitute_arg_in_value(w_str, with) {
                    if let Some(rewritten) = value_to_when_string(&replacement) {
                        sub_when = Some(rewritten);
                    }
                }
            }
        }
        let when = sub_when.or_else(|| parent.when.clone());

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
            invoke: {
                let mut inv = sub_phase.invoke.clone();
                if !with.is_empty() {
                    if let Some(v) = inv.as_mut() {
                        substitute_in_invoker(v, with);
                    }
                }
                inv
            },
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

/// Render a substituted `with` value back into the `Phase.when` scalar string.
/// Returns `None` for values that are not safely representable as a `when`
/// token, leaving the original template intact at the caller.
fn value_to_when_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Walk a string-keyed JSON map and replace any `String("args.<name>")` value
/// whose `<name>` is in `with_map` with the corresponding `with_map` value.
fn substitute_in_value_map(
    map: &mut std::collections::HashMap<String, serde_json::Value>,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) {
    for v in map.values_mut() {
        if let serde_json::Value::String(s) = v {
            if let Some(replacement) = substitute_arg_in_value(s, with_map) {
                *v = replacement;
            }
        }
    }
}

/// Substitute an `IterationsSpec::Template("args.<name>")` using `with_map`.
/// Number values become `IterationsSpec::Literal(v as u32)`. Non-template
/// strings become `IterationsSpec::Template(s)`. Non-convertible values
/// (bool, null, array, object, non-integer number, out-of-range integer)
/// leave the original Template intact.
fn substitute_iterations_spec(
    spec: &mut crate::model::IterationsSpec,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) {
    let crate::model::IterationsSpec::Template(template) = spec else {
        return;
    };
    let Some(replacement) = substitute_arg_in_value(template, with_map) else {
        return;
    };
    *spec = match replacement {
        serde_json::Value::Number(n) => match n.as_u64().and_then(|v| u32::try_from(v).ok()) {
            Some(v) => crate::model::IterationsSpec::Literal(v),
            None => crate::model::IterationsSpec::Template(template.clone()),
        },
        serde_json::Value::String(s) => crate::model::IterationsSpec::Template(s),
        _ => crate::model::IterationsSpec::Template(template.clone()),
    };
}

/// Apply `with_map` substitution to every rewrite-eligible field of an
/// `Invoker`. Called once per sub-phase after clone.
fn substitute_in_invoker(
    invoker: &mut crate::model::Invoker,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) {
    use crate::model::Invoker;
    match invoker {
        Invoker::Skill { args, .. } | Invoker::Agent { args, .. } => {
            substitute_in_value_map(args, with_map);
        }
        Invoker::Agents {
            iterations, args, ..
        } => {
            substitute_iterations_spec(iterations, with_map);
            substitute_in_value_map(args, with_map);
        }
        Invoker::Pipeline {
            with: inner_with, ..
        } => {
            substitute_in_value_map(inner_with, with_map);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "panic! is the conventional assertion failure in test-only code"
)]
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

    #[test]
    fn expand_sub_pipeline_with_empty_with_is_byte_identical_to_legacy() {
        use crate::model::{Phase, SubPipeline};

        let parent = Phase {
            id: "p".into(),
            description: None,
            with: HashMap::new(),
            when: None,
            invoke: None,
            config: HashMap::new(),
            produces: Vec::new(),
            consumes: Vec::new(),
            gate: Vec::new(),
            validate: Vec::new(),
            regate: Vec::new(),
            confirm: false,
            max_retries: 0,
        };
        let sub = SubPipeline {
            name: "s".into(),
            description: None,
            version: 1,
            inputs: HashMap::new(),
            phases: vec![Phase {
                id: "leaf".into(),
                description: Some("d".into()),
                with: HashMap::new(),
                when: Some("args.x".into()),
                invoke: None,
                config: HashMap::new(),
                produces: Vec::new(),
                consumes: Vec::new(),
                gate: Vec::new(),
                validate: Vec::new(),
                regate: Vec::new(),
                confirm: false,
                max_retries: 0,
            }],
        };
        let empty: HashMap<String, serde_json::Value> = HashMap::new();
        let out = expand_sub_pipeline("p", &parent, &sub, &empty);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "p/leaf");
        // Empty with must not rewrite anything — when stays as-is.
        assert_eq!(out[0].when.as_deref(), Some("args.x"));
    }

    fn mk_leaf_phase(id: &str, when: Option<&str>) -> crate::model::Phase {
        crate::model::Phase {
            id: id.into(),
            description: Some("d".into()),
            with: HashMap::new(),
            when: when.map(str::to_string),
            invoke: None,
            config: HashMap::new(),
            produces: Vec::new(),
            consumes: Vec::new(),
            gate: Vec::new(),
            validate: Vec::new(),
            regate: Vec::new(),
            confirm: false,
            max_retries: 0,
        }
    }

    fn mk_parent_phase() -> crate::model::Phase {
        crate::model::Phase {
            id: "p".into(),
            description: None,
            with: HashMap::new(),
            when: None,
            invoke: None,
            config: HashMap::new(),
            produces: Vec::new(),
            consumes: Vec::new(),
            gate: Vec::new(),
            validate: Vec::new(),
            regate: Vec::new(),
            confirm: false,
            max_retries: 0,
        }
    }

    fn mk_sub(phases: Vec<crate::model::Phase>) -> crate::model::SubPipeline {
        crate::model::SubPipeline {
            name: "s".into(),
            description: None,
            version: 1,
            inputs: HashMap::new(),
            phases,
        }
    }

    #[test]
    fn when_rename_string_to_string() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.inner"))]);
        let w = mk_with(&[("inner", json!("args.outer"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("args.outer"));
    }

    #[test]
    fn when_rewrites_bool_true_to_string_literal_true() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.enabled"))]);
        let w = mk_with(&[("enabled", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("true"));
    }

    #[test]
    fn when_rewrites_bool_false_to_string_literal_false() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.enabled"))]);
        let w = mk_with(&[("enabled", json!(false))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("false"));
    }

    #[test]
    fn when_left_untouched_when_compound_expression() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.x && args.y"))]);
        let w = mk_with(&[("x", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        // Compound expression — not a full-string match.
        assert_eq!(out[0].when.as_deref(), Some("args.x && args.y"));
    }

    #[test]
    fn when_left_untouched_when_key_missing() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.missing"))]);
        let w = mk_with(&[("other", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("args.missing"));
    }

    #[test]
    fn when_inherited_from_parent_is_not_rewritten_by_sub_with() {
        let mut parent = mk_parent_phase();
        parent.when = Some("args.flag".into());
        let sub = mk_sub(vec![mk_leaf_phase("leaf", None)]); // sub.when is None
        // Sub-pipeline's with happens to have a key named "flag" — this must
        // NOT rewrite the parent's inherited when.
        let w = mk_with(&[("flag", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("args.flag"));
    }

    #[test]
    fn when_left_untouched_when_with_value_is_number() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.count"))]);
        let w = mk_with(&[("count", json!(5))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("args.count"));
    }

    #[test]
    fn when_left_untouched_when_with_value_is_null() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.x"))]);
        let w = mk_with(&[("x", serde_json::Value::Null)]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].when.as_deref(), Some("args.x"));
    }

    #[test]
    fn when_bool_rewrite_evaluates_correctly_in_eval_when() {
        use crate::engine::eval_when_for_test;
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_phase("leaf", Some("args.enabled"))]);
        let w = mk_with(&[("enabled", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        let empty_args: HashMap<String, serde_json::Value> = HashMap::new();
        // The rewritten when is "true"; with the engine's literal handling,
        // this must evaluate to true even without any runtime args.
        assert!(eval_when_for_test(out[0].when.as_ref(), &empty_args));
    }

    fn mk_leaf_with_config(
        id: &str,
        config: Vec<(&str, serde_json::Value)>,
    ) -> crate::model::Phase {
        let mut p = mk_leaf_phase(id, None);
        p.config = config
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        p
    }

    #[test]
    fn config_string_value_rewritten_to_literal() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config(
            "leaf",
            vec![("k", json!("args.n"))],
        )]);
        let w = mk_with(&[("n", json!(7))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!(7)));
    }

    #[test]
    fn config_string_value_rewritten_to_template() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config(
            "leaf",
            vec![("k", json!("args.n"))],
        )]);
        let w = mk_with(&[("n", json!("args.outer"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!("args.outer")));
    }

    #[test]
    fn config_non_string_values_untouched() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config(
            "leaf",
            vec![("k", json!([1, 2]))],
        )]);
        let w = mk_with(&[("k", json!("args.outer"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!([1, 2])));
    }

    #[test]
    fn config_inherited_from_parent_is_not_rewritten_by_sub_with() {
        let mut parent = mk_parent_phase();
        parent
            .config
            .insert("custom_key".into(), json!("args.outer_flag"));
        let sub = mk_sub(vec![mk_leaf_phase("leaf", None)]);
        // Sub's `with` happens to have a key named `outer_flag` — this must NOT
        // rewrite the parent's inherited config value for `custom_key`.
        let w = mk_with(&[("outer_flag", json!("args.renamed"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(
            out[0].config.get("custom_key"),
            Some(&json!("args.outer_flag"))
        );
    }

    #[test]
    fn config_bool_value_from_with_replaces_string_template() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config(
            "leaf",
            vec![("k", json!("args.flag"))],
        )]);
        let w = mk_with(&[("flag", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!(true)));
    }

    #[test]
    fn config_null_value_from_with_replaces_string_template() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config(
            "leaf",
            vec![("k", json!("args.x"))],
        )]);
        let w = mk_with(&[("x", serde_json::Value::Null)]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&serde_json::Value::Null));
    }

    #[test]
    fn config_multi_key_mix_of_matching_and_non_matching_templates() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config(
            "leaf",
            vec![
                ("k1", json!("args.present")),
                ("k2", json!("args.absent")),
                ("k3", json!("literal")),
                ("k4", json!(42)),
            ],
        )]);
        let w = mk_with(&[("present", json!("replaced"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k1"), Some(&json!("replaced")));
        assert_eq!(out[0].config.get("k2"), Some(&json!("args.absent")));
        assert_eq!(out[0].config.get("k3"), Some(&json!("literal")));
        assert_eq!(out[0].config.get("k4"), Some(&json!(42)));
    }

    fn mk_leaf_with_agents_iter(
        id: &str,
        iterations: crate::model::IterationsSpec,
    ) -> crate::model::Phase {
        let mut p = mk_leaf_phase(id, None);
        p.invoke = Some(crate::model::Invoker::Agents {
            agents: vec!["a".into()],
            iterations,
            args: HashMap::new(),
        });
        p
    }

    #[test]
    fn iterations_template_rewritten_to_literal_u32() {
        use crate::model::IterationsSpec;
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_agents_iter(
            "leaf",
            IterationsSpec::Template("args.count".into()),
        )]);
        let w = mk_with(&[("count", json!(5))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(crate::model::Invoker::Agents { iterations, .. }) => {
                assert_eq!(iterations, &IterationsSpec::Literal(5));
            }
            _ => panic!("expected Agents invoke"),
        }
    }

    #[test]
    fn iterations_template_rewritten_to_new_template() {
        use crate::model::IterationsSpec;
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_agents_iter(
            "leaf",
            IterationsSpec::Template("args.count".into()),
        )]);
        let w = mk_with(&[("count", json!("args.iterations"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(crate::model::Invoker::Agents { iterations, .. }) => {
                assert_eq!(
                    iterations,
                    &IterationsSpec::Template("args.iterations".into())
                );
            }
            _ => panic!("expected Agents invoke"),
        }
    }

    #[test]
    fn iterations_template_unchanged_when_with_value_not_convertible() {
        use crate::model::IterationsSpec;
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_agents_iter(
            "leaf",
            IterationsSpec::Template("args.count".into()),
        )]);
        // Bool is not a valid iteration count.
        let w = mk_with(&[("count", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(crate::model::Invoker::Agents { iterations, .. }) => {
                assert_eq!(iterations, &IterationsSpec::Template("args.count".into()));
            }
            _ => panic!("expected Agents invoke"),
        }
    }

    #[test]
    fn iterations_literal_untouched() {
        use crate::model::IterationsSpec;
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_agents_iter(
            "leaf",
            IterationsSpec::Literal(9),
        )]);
        let w = mk_with(&[("count", json!(5))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(crate::model::Invoker::Agents { iterations, .. }) => {
                assert_eq!(iterations, &IterationsSpec::Literal(9));
            }
            _ => panic!("expected Agents invoke"),
        }
    }

    #[test]
    fn invoker_skill_args_rewritten() {
        use crate::model::Invoker;
        let parent = mk_parent_phase();
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Skill {
            skill: "/s".into(),
            args: [("k".to_string(), json!("args.n"))].into_iter().collect(),
        });
        let sub = mk_sub(vec![leaf]);
        let w = mk_with(&[("n", json!(42))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(Invoker::Skill { args, .. }) => {
                assert_eq!(args.get("k"), Some(&json!(42)));
            }
            _ => panic!("expected Skill invoke"),
        }
    }

    #[test]
    fn invoker_agent_args_rewritten() {
        use crate::model::Invoker;
        let parent = mk_parent_phase();
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Agent {
            agent: "ag".into(),
            args: [("k".to_string(), json!("args.flag"))]
                .into_iter()
                .collect(),
        });
        let sub = mk_sub(vec![leaf]);
        let w = mk_with(&[("flag", json!(true))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(Invoker::Agent { args, .. }) => {
                assert_eq!(args.get("k"), Some(&json!(true)));
            }
            _ => panic!("expected Agent invoke"),
        }
    }

    #[test]
    fn invoker_agents_args_rewritten() {
        use crate::model::{Invoker, IterationsSpec};
        let parent = mk_parent_phase();
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Agents {
            agents: vec!["a".into()],
            iterations: IterationsSpec::default(),
            args: [("k".to_string(), json!("args.x"))].into_iter().collect(),
        });
        let sub = mk_sub(vec![leaf]);
        let w = mk_with(&[("x", json!("hello"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(Invoker::Agents { args, .. }) => {
                assert_eq!(args.get("k"), Some(&json!("hello")));
            }
            _ => panic!("expected Agents invoke"),
        }
    }

    #[test]
    fn invoker_pipeline_nested_with_rewritten() {
        use crate::model::Invoker;
        let parent = mk_parent_phase();
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Pipeline {
            pipeline: "nested.yml".into(),
            with: [("inner".to_string(), json!("args.outer"))]
                .into_iter()
                .collect(),
        });
        let sub = mk_sub(vec![leaf]);
        let w = mk_with(&[("outer", json!("args.iterations"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(Invoker::Pipeline {
                with: inner_with, ..
            }) => {
                assert_eq!(inner_with.get("inner"), Some(&json!("args.iterations")));
            }
            _ => panic!("expected Pipeline invoke"),
        }
    }
}

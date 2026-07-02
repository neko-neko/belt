use crate::error::{BeltError, BeltResult};
use crate::model::{ExpandedPhase, Invoker, Phase};
use crate::parser::{parse_pipeline, parse_sub_pipeline};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maximum sub-pipeline reference depth (root pipeline = depth 0).
const MAX_EXPANSION_DEPTH: usize = 4;

/// Parse a pipeline and expand all `invoke: { pipeline: ... }` references
/// into flat, namespaced phases — recursively.
///
/// Each phase whose `invoke` is an [`Invoker::Pipeline`] references a
/// sub-pipeline YAML file (resolved relative to the referencing file's
/// directory). Sub-pipeline phases are flattened with namespaced IDs
/// (`{parent_id}/{sub_phase_id}`, nested levels concatenate:
/// `{a}/{b}/{c}`). A sub-pipeline phase may itself reference another
/// sub-pipeline; cycles are rejected and nesting is capped at
/// [`MAX_EXPANSION_DEPTH`].
///
/// Inheritance rules at every level, applied to the expansion of each
/// referencing phase:
/// - `gate`, `regate`, `validate`: parent entries are **appended** to the
///   LAST expanded (innermost) leaf
/// - `config`: merged into the last leaf with parent keys winning
/// - `when`: propagates to **all** expanded leaves that lack their own
///
/// `regate` targets declared inside a sub-pipeline are renamed into that
/// sub-pipeline's expansion namespace (`execute` → `{parent_id}/execute`).
///
/// Every leaf phase (no `invoke: { pipeline: ... }`) **must** have a
/// `description`; otherwise `BeltError::InvalidPipeline` is returned.
pub fn expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>> {
    let pipeline = parse_pipeline(pipeline_path)?;
    let base_dir = pipeline_path.parent().unwrap_or_else(|| Path::new("."));
    let mut visited = vec![canonical_or_self(pipeline_path)];
    expand_phase_list(
        &pipeline.phases,
        base_dir,
        "",
        &HashMap::new(),
        &mut visited,
    )
}

/// Canonicalize for cycle detection; fall back to the raw path when the
/// file cannot be canonicalized (missing file errors surface in parsing).
fn canonical_or_self(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Expand one level of phases, recursing into `Invoker::Pipeline` refs.
///
/// - `ns`: namespace prefix — `""` at the root, `"{parent}/"` below.
/// - `with`: the substitution scope THIS level's phases were referenced
///   with (empty at the root). Applied to leaf `when`/`config`/`invoke`
///   and folded into child reference `with` maps before descending.
/// - `visited`: reference-path stack for cycle and depth detection.
fn expand_phase_list(
    phases: &[Phase],
    base_dir: &Path,
    ns: &str,
    with: &HashMap<String, serde_json::Value>,
    visited: &mut Vec<PathBuf>,
) -> BeltResult<Vec<ExpandedPhase>> {
    let local_ids: Vec<&str> = phases.iter().map(|p| p.id.as_str()).collect();
    let mut expanded = Vec::new();
    for phase in phases {
        if let Some(Invoker::Pipeline {
            pipeline: sub_ref,
            with: phase_with,
        }) = &phase.invoke
        {
            // Resolve the child's `with` in THIS level's scope first —
            // mirrors substitute_in_invoker (I1 parent-scope rule).
            let mut child_with = phase_with.clone();
            if !with.is_empty() {
                substitute_in_value_map(&mut child_with, with);
            }

            let sub_path = base_dir.join(sub_ref);
            let canonical = canonical_or_self(&sub_path);
            if visited.contains(&canonical) {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase '{ns}{}': cyclic sub-pipeline reference '{sub_ref}'",
                        phase.id
                    ),
                });
            }
            if visited.len() > MAX_EXPANSION_DEPTH {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase '{ns}{}': sub-pipeline nesting exceeds depth {MAX_EXPANSION_DEPTH}",
                        phase.id
                    ),
                });
            }

            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_base = sub_path
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let child_ns = format!("{ns}{}/", phase.id);

            visited.push(canonical);
            let mut sub_expanded =
                expand_phase_list(&sub.phases, &sub_base, &child_ns, &child_with, visited)?;
            visited.pop();

            if sub_expanded.is_empty() {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase '{ns}{}': sub-pipeline '{sub_ref}' has no phases",
                        phase.id
                    ),
                });
            }

            // Parent `when` propagates to every expanded leaf lacking one.
            // The parent's when is authored in the parent's arg scope, so
            // it is substituted against THIS level's `with`, not the child's.
            let parent_when = substituted_when(phase.when.as_deref(), with);
            for sub_phase in &mut sub_expanded {
                if sub_phase.when.is_none() {
                    sub_phase.when.clone_from(&parent_when);
                }
            }

            // gate/regate/validate append + config merge on the LAST leaf.
            if let Some(last) = sub_expanded.last_mut() {
                last.gate.extend(phase.gate.clone());
                last.regate.extend(phase.regate.clone());
                last.validate.extend(phase.validate.clone());
                let mut parent_config = phase.config.clone();
                if !with.is_empty() {
                    substitute_in_value_map(&mut parent_config, with);
                }
                for (k, v) in parent_config {
                    last.config.insert(k, v);
                }
            }

            expanded.extend(sub_expanded);
        } else {
            expanded.push(leaf_phase(phase, ns, with, &local_ids)?);
        }
    }
    Ok(expanded)
}

/// Substitute a `when` template against a `with` scope; returns the
/// (possibly rewritten) owned when.
fn substituted_when(
    when: Option<&str>,
    with: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    let w = when?;
    if !with.is_empty() {
        if let Some(replacement) = substitute_arg_in_value(w, with) {
            if let Some(rewritten) = value_to_when_string(&replacement) {
                return Some(rewritten);
            }
        }
    }
    Some(w.to_owned())
}

/// Materialize a leaf phase at namespace `ns`, applying this level's
/// `with` substitution to `when`, `config`, and the invoker, and renaming
/// sibling-scoped `regate` targets into the namespace.
fn leaf_phase(
    phase: &Phase,
    ns: &str,
    with: &HashMap<String, serde_json::Value>,
    local_ids: &[&str],
) -> BeltResult<ExpandedPhase> {
    let description = phase
        .description
        .clone()
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: format!("leaf phase '{ns}{}' must have a description", phase.id),
        })?;

    let mut config = phase.config.clone();
    if !with.is_empty() {
        substitute_in_value_map(&mut config, with);
    }

    let invoke = {
        let mut inv = phase.invoke.clone();
        if !with.is_empty() {
            if let Some(v) = inv.as_mut() {
                substitute_in_invoker(v, with);
            }
        }
        inv
    };

    // Regate targets naming a sibling phase in this file are renamed into
    // the expansion namespace; anything else is left verbatim.
    let regate = phase
        .regate
        .iter()
        .map(|t| {
            if local_ids.contains(&t.as_str()) {
                format!("{ns}{t}")
            } else {
                t.clone()
            }
        })
        .collect();

    Ok(ExpandedPhase {
        id: format!("{ns}{}", phase.id),
        description,
        config,
        produces: phase.produces.clone(),
        consumes: phase.consumes.clone(),
        gate: phase.gate.clone(),
        validate: phase.validate.clone(),
        regate,
        confirm: phase.confirm,
        max_retries: phase.max_retries,
        when: substituted_when(phase.when.as_deref(), with),
        invoke,
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

/// Apply `with_map` substitution to every rewrite-eligible field of an
/// `Invoker`. Called once per sub-phase after clone.
fn substitute_in_invoker(
    invoker: &mut crate::model::Invoker,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) {
    use crate::model::Invoker;
    match invoker {
        Invoker::Skill { args, .. } => {
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
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "unwrap/expect/panic are the conventional assertion style in test-only code"
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

    // The substitute-scope rules below were originally locked through the old
    // `expand_sub_pipeline(parent, sub, with)` entry point. After the recursive
    // rewrite, a sub-phase's OWN when/config/invoker substitution lives in
    // `leaf_phase(phase, ns, with, local_ids)` / `substituted_when`, so these
    // tests exercise those directly. Parent-inheritance scope rules (parent
    // values resolved in the PARENT scope, never the sub `with`) are locked via
    // `substituted_when` / `substitute_in_value_map` against a parent scope.

    #[test]
    fn when_rename_string_to_string() {
        let w = mk_with(&[("inner", json!("args.outer"))]);
        let out = leaf_phase(&mk_leaf_phase("leaf", Some("args.inner")), "p/", &w, &[])
            .expect("leaf phase");
        assert_eq!(out.when.as_deref(), Some("args.outer"));
    }

    #[test]
    fn when_rewrites_bool_true_to_string_literal_true() {
        let w = mk_with(&[("enabled", json!(true))]);
        let out = leaf_phase(&mk_leaf_phase("leaf", Some("args.enabled")), "p/", &w, &[])
            .expect("leaf phase");
        assert_eq!(out.when.as_deref(), Some("true"));
    }

    #[test]
    fn when_rewrites_bool_false_to_string_literal_false() {
        let w = mk_with(&[("enabled", json!(false))]);
        let out = leaf_phase(&mk_leaf_phase("leaf", Some("args.enabled")), "p/", &w, &[])
            .expect("leaf phase");
        assert_eq!(out.when.as_deref(), Some("false"));
    }

    #[test]
    fn when_left_untouched_when_compound_expression() {
        let w = mk_with(&[("x", json!(true))]);
        let out = leaf_phase(
            &mk_leaf_phase("leaf", Some("args.x && args.y")),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        // Compound expression — not a full-string match.
        assert_eq!(out.when.as_deref(), Some("args.x && args.y"));
    }

    #[test]
    fn when_left_untouched_when_key_missing() {
        let w = mk_with(&[("other", json!(true))]);
        let out = leaf_phase(&mk_leaf_phase("leaf", Some("args.missing")), "p/", &w, &[])
            .expect("leaf phase");
        assert_eq!(out.when.as_deref(), Some("args.missing"));
    }

    #[test]
    fn when_inherited_from_parent_is_not_rewritten_by_sub_with() {
        // In the recursive expander a leaf that declares no `when` inherits the
        // parent's when, which is resolved in the PARENT arg scope — never the
        // sub-pipeline `with`. Lock both halves of that guarantee.
        // 1. A None-when leaf materialized under the sub `with` stays None: it
        //    does not fabricate a when from a coincidentally-named sub key.
        let sub_with = mk_with(&[("flag", json!(true))]);
        let out =
            leaf_phase(&mk_leaf_phase("leaf", None), "p/", &sub_with, &[]).expect("leaf phase");
        assert_eq!(out.when, None);
        // 2. The parent's own when resolves in the parent scope (here empty),
        //    so a same-named key in the sub `with` cannot rewrite it.
        assert_eq!(
            substituted_when(Some("args.flag"), &HashMap::new()).as_deref(),
            Some("args.flag")
        );
    }

    #[test]
    fn when_left_untouched_when_with_value_is_number() {
        let w = mk_with(&[("count", json!(5))]);
        let out = leaf_phase(&mk_leaf_phase("leaf", Some("args.count")), "p/", &w, &[])
            .expect("leaf phase");
        assert_eq!(out.when.as_deref(), Some("args.count"));
    }

    #[test]
    fn when_left_untouched_when_with_value_is_null() {
        let w = mk_with(&[("x", serde_json::Value::Null)]);
        let out =
            leaf_phase(&mk_leaf_phase("leaf", Some("args.x")), "p/", &w, &[]).expect("leaf phase");
        assert_eq!(out.when.as_deref(), Some("args.x"));
    }

    #[test]
    fn when_bool_rewrite_evaluates_correctly_in_eval_when() {
        use crate::engine::eval_when_for_test;
        let w = mk_with(&[("enabled", json!(true))]);
        let out = leaf_phase(&mk_leaf_phase("leaf", Some("args.enabled")), "p/", &w, &[])
            .expect("leaf phase");
        let empty_args: HashMap<String, serde_json::Value> = HashMap::new();
        // The rewritten when is "true"; with the engine's literal handling,
        // this must evaluate to true even without any runtime args.
        assert!(eval_when_for_test(out.when.as_ref(), &empty_args));
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
        let w = mk_with(&[("n", json!(7))]);
        let out = leaf_phase(
            &mk_leaf_with_config("leaf", vec![("k", json!("args.n"))]),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        assert_eq!(out.config.get("k"), Some(&json!(7)));
    }

    #[test]
    fn config_string_value_rewritten_to_template() {
        let w = mk_with(&[("n", json!("args.outer"))]);
        let out = leaf_phase(
            &mk_leaf_with_config("leaf", vec![("k", json!("args.n"))]),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        assert_eq!(out.config.get("k"), Some(&json!("args.outer")));
    }

    #[test]
    fn config_non_string_values_untouched() {
        let w = mk_with(&[("k", json!("args.outer"))]);
        let out = leaf_phase(
            &mk_leaf_with_config("leaf", vec![("k", json!([1, 2]))]),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        assert_eq!(out.config.get("k"), Some(&json!([1, 2])));
    }

    #[test]
    fn config_inherited_from_parent_is_not_rewritten_by_sub_with() {
        // A leaf with no config of its own gains nothing from the sub `with`,
        // and the parent's config value is substituted in the PARENT scope — a
        // same-named key in the sub `with` must not rewrite it.
        let sub_with = mk_with(&[("outer_flag", json!("args.renamed"))]);
        let out =
            leaf_phase(&mk_leaf_phase("leaf", None), "p/", &sub_with, &[]).expect("leaf phase");
        assert!(out.config.is_empty());
        let mut parent_config = mk_with(&[("custom_key", json!("args.outer_flag"))]);
        substitute_in_value_map(&mut parent_config, &HashMap::new());
        assert_eq!(
            parent_config.get("custom_key"),
            Some(&json!("args.outer_flag"))
        );
    }

    #[test]
    fn config_bool_value_from_with_replaces_string_template() {
        let w = mk_with(&[("flag", json!(true))]);
        let out = leaf_phase(
            &mk_leaf_with_config("leaf", vec![("k", json!("args.flag"))]),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        assert_eq!(out.config.get("k"), Some(&json!(true)));
    }

    #[test]
    fn config_null_value_from_with_replaces_string_template() {
        let w = mk_with(&[("x", serde_json::Value::Null)]);
        let out = leaf_phase(
            &mk_leaf_with_config("leaf", vec![("k", json!("args.x"))]),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        assert_eq!(out.config.get("k"), Some(&serde_json::Value::Null));
    }

    #[test]
    fn config_multi_key_mix_of_matching_and_non_matching_templates() {
        let w = mk_with(&[("present", json!("replaced"))]);
        let out = leaf_phase(
            &mk_leaf_with_config(
                "leaf",
                vec![
                    ("k1", json!("args.present")),
                    ("k2", json!("args.absent")),
                    ("k3", json!("literal")),
                    ("k4", json!(42)),
                ],
            ),
            "p/",
            &w,
            &[],
        )
        .expect("leaf phase");
        assert_eq!(out.config.get("k1"), Some(&json!("replaced")));
        assert_eq!(out.config.get("k2"), Some(&json!("args.absent")));
        assert_eq!(out.config.get("k3"), Some(&json!("literal")));
        assert_eq!(out.config.get("k4"), Some(&json!(42)));
    }

    #[test]
    fn invoker_skill_args_rewritten() {
        use crate::model::Invoker;
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Skill {
            skill: "/s".into(),
            args: [("k".to_string(), json!("args.n"))].into_iter().collect(),
        });
        let w = mk_with(&[("n", json!(42))]);
        let out = leaf_phase(&leaf, "p/", &w, &[]).expect("leaf phase");
        match &out.invoke {
            Some(Invoker::Skill { args, .. }) => {
                assert_eq!(args.get("k"), Some(&json!(42)));
            }
            _ => panic!("expected Skill invoke"),
        }
    }

    #[test]
    fn invoker_pipeline_nested_with_rewritten() {
        use crate::model::Invoker;
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Pipeline {
            pipeline: "nested.yml".into(),
            with: [("inner".to_string(), json!("args.outer"))]
                .into_iter()
                .collect(),
        });
        let w = mk_with(&[("outer", json!("args.iterations"))]);
        let out = leaf_phase(&leaf, "p/", &w, &[]).expect("leaf phase");
        match &out.invoke {
            Some(Invoker::Pipeline {
                with: inner_with, ..
            }) => {
                assert_eq!(inner_with.get("inner"), Some(&json!("args.iterations")));
            }
            _ => panic!("expected Pipeline invoke"),
        }
    }
}

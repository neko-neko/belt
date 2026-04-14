# Expander `Invoker::Pipeline.with` merge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Thread `Invoker::Pipeline.with` through the expander so that sub-phase references to `args.<name>` are rewritten at expand time using the parent's `with:` map, making the `with:` rename contract actually honored at runtime.

**Architecture:** Pure extension of `crates/belt-core/src/expander.rs`. A new pure helper `substitute_arg_in_value` rewrites single string templates. The existing `expand_sub_pipeline` gains a `with: &HashMap<String, serde_json::Value>` parameter and walks each constructed `ExpandedPhase`, substituting five target fields. TDD — one target field per task, each adding failing unit tests before implementation.

**Tech Stack:** Rust 1.86.0 (MSRV), serde, serde_json, belt-core's existing model types (`Invoker`, `IterationsSpec`, `ExpandedPhase`).

**Spec:** `docs/specs/2026-04-14-expander-with-merge-design.md`

---

## File Structure

Modified files (no new files required):

- `crates/belt-core/src/expander.rs` — gains inline `#[cfg(test)] mod tests` block + `substitute_arg_in_value` helper + expanded signature on `expand_sub_pipeline` + field-level substitution calls.

Optional new files (Task 8):

- `crates/belt-core/tests/expander_with_test.rs` — integration-style end-to-end test of a synthetic parent+sub pipeline pair with a renamed argument. Uses tempdir fixtures.

No changes to: `model.rs`, `engine.rs`, `view.rs`, `lint.rs`, `parser.rs`, `gate.rs`, any binary crate, or any file under `examples/`.

---

## Task 1: Add `substitute_arg_in_value` helper with unit tests

**Files:**
- Modify: `crates/belt-core/src/expander.rs` — add pure helper + `#[cfg(test)] mod tests` block at end of file.

- [ ] **Step 1: Add failing unit tests at the end of `expander.rs`**

Append to `crates/belt-core/src/expander.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    fn mk_with(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect()
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
        assert_eq!(
            substitute_arg_in_value("args.count", &w),
            Some(json!(5))
        );
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
        assert_eq!(
            substitute_arg_in_value("args.name", &w),
            Some(json!("foo"))
        );
    }

    #[test]
    fn substitute_rewrites_to_null() {
        let w = mk_with(&[("x", Value::Null)]);
        assert_eq!(substitute_arg_in_value("args.x", &w), Some(Value::Null));
    }
}
```

- [ ] **Step 2: Run the failing tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: 7 test failures, all with `cannot find function 'substitute_arg_in_value'`.

- [ ] **Step 3: Implement the helper**

Insert directly above the `#[cfg(test)]` block:

```rust
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
```

- [ ] **Step 4: Run the tests again**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: 7 tests pass.

- [ ] **Step 5: Run clippy and fmt scoped to belt-core**

Run: `cargo clippy --package belt-core -- -D warnings`
Expected: no warnings.

Run: `cargo fmt --package belt-core`
Expected: no changes (or commit-ready formatting).

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "feat(belt-core): add substitute_arg_in_value helper for expander with-merge"
```

---

## Task 2: Thread `with` through `expand_sub_pipeline` (no behavior change)

**Files:**
- Modify: `crates/belt-core/src/expander.rs:28` — destructure `with` instead of `..`.
- Modify: `crates/belt-core/src/expander.rs:31` — pass `with` to `expand_sub_pipeline`.
- Modify: `crates/belt-core/src/expander.rs:40` — add `with` parameter to `expand_sub_pipeline`.

- [ ] **Step 1: Add a failing regression test**

Append to the `#[cfg(test)] mod tests` block in `expander.rs`:

```rust
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
```

- [ ] **Step 2: Run the test to verify compile error (signature mismatch)**

Run: `cargo test -p belt-core --lib expander::tests::expand_sub_pipeline_with_empty_with_is_byte_identical_to_legacy`
Expected: compile error — `expand_sub_pipeline` takes 3 args, not 4.

- [ ] **Step 3: Update `expand_pipeline` destructure and call site**

In `crates/belt-core/src/expander.rs`, replace:

```rust
        if let Some(Invoker::Pipeline { pipeline, .. }) = &phase.invoke {
            let sub_path = base_dir.join(pipeline);
            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_phases = expand_sub_pipeline(&phase.id, phase, &sub);
            expanded.extend(sub_phases);
```

with:

```rust
        if let Some(Invoker::Pipeline { pipeline, with }) = &phase.invoke {
            let sub_path = base_dir.join(pipeline);
            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_phases = expand_sub_pipeline(&phase.id, phase, &sub, with);
            expanded.extend(sub_phases);
```

- [ ] **Step 4: Update `expand_sub_pipeline` signature**

Replace the signature:

```rust
fn expand_sub_pipeline(parent_id: &str, parent: &Phase, sub: &SubPipeline) -> Vec<ExpandedPhase> {
```

with:

```rust
fn expand_sub_pipeline(
    parent_id: &str,
    parent: &Phase,
    sub: &SubPipeline,
    with: &std::collections::HashMap<String, serde_json::Value>,
) -> Vec<ExpandedPhase> {
    // Substitution is a no-op when `with` is empty — avoid walking fields.
    let _ = with;
```

The `let _ = with;` line silences the "unused parameter" warning for this task; it is removed in Task 3.

- [ ] **Step 5: Run the full belt-core test suite**

Run: `cargo test -p belt-core`
Expected: all existing + Task 1/Task 2 tests pass. No behavior change for any existing test.

- [ ] **Step 6: Run clippy and fmt scoped to belt-core**

Run: `cargo clippy --package belt-core -- -D warnings`
Run: `cargo fmt --package belt-core`
Expected: no warnings, no fmt changes beyond current work.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "refactor(belt-core): thread Invoker::Pipeline.with through expand_sub_pipeline (no-op)"
```

---

## Task 3: Apply substitution to `Phase.when` (Option<String>)

**Files:**
- Modify: `crates/belt-core/src/expander.rs` — remove the `let _ = with;` stub; add `when` rewrite.

- [ ] **Step 1: Add failing tests**

Append to the `#[cfg(test)] mod tests` block:

```rust
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
```

- [ ] **Step 2: Run the failing tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: the four new `when_*` tests (minus the "untouched" pair which pass by accident) fail on value mismatch.

- [ ] **Step 3: Replace the no-op stub with a per-phase rewrite**

In `expand_sub_pipeline`, remove `let _ = with;` and add the rewrite block at the end of the `for (i, sub_phase)` loop body, just before the `phases.push(ExpandedPhase { ... })` statement. Change the current:

```rust
        // when: sub-phase inherits parent's when if it doesn't have its own
        let when = sub_phase.when.clone().or_else(|| parent.when.clone());

        phases.push(ExpandedPhase {
```

to:

```rust
        // when: sub-phase inherits parent's when if it doesn't have its own
        let mut when = sub_phase.when.clone().or_else(|| parent.when.clone());
        if !with.is_empty() {
            if let Some(w_str) = when.as_deref() {
                if let Some(replacement) = substitute_arg_in_value(w_str, with) {
                    when = Some(value_to_when_string(&replacement).unwrap_or_else(|| w_str.to_string()));
                }
            }
        }

        phases.push(ExpandedPhase {
```

And add the small helper below `substitute_arg_in_value`:

```rust
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
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: all tests pass including the five `when_*` tests.

- [ ] **Step 5: Run clippy and fmt**

Run: `cargo clippy --package belt-core -- -D warnings`
Run: `cargo fmt --package belt-core`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "feat(belt-core): substitute Phase.when using Invoker::Pipeline.with"
```

---

## Task 4: Apply substitution to `Phase.config` values

**Files:**
- Modify: `crates/belt-core/src/expander.rs`.

- [ ] **Step 1: Add failing tests**

Append to the test module:

```rust
    fn mk_leaf_with_config(
        id: &str,
        config: Vec<(&str, serde_json::Value)>,
    ) -> crate::model::Phase {
        let mut p = mk_leaf_phase(id, None);
        p.config = config.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        p
    }

    #[test]
    fn config_string_value_rewritten_to_literal() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config("leaf", vec![("k", json!("args.n"))])]);
        let w = mk_with(&[("n", json!(7))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!(7)));
    }

    #[test]
    fn config_string_value_rewritten_to_template() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config("leaf", vec![("k", json!("args.n"))])]);
        let w = mk_with(&[("n", json!("args.outer"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!("args.outer")));
    }

    #[test]
    fn config_non_string_values_untouched() {
        let parent = mk_parent_phase();
        let sub = mk_sub(vec![mk_leaf_with_config("leaf", vec![("k", json!([1, 2]))])]);
        let w = mk_with(&[("k", json!("args.outer"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        assert_eq!(out[0].config.get("k"), Some(&json!([1, 2])));
    }
```

- [ ] **Step 2: Run tests to see failures**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: `config_string_value_rewritten_to_*` fail on value mismatch.

- [ ] **Step 3: Implement config rewrite**

Add a helper below `value_to_when_string`:

```rust
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
```

Then in `expand_sub_pipeline`, change the config merge block to call the helper after the merge is complete. Replace:

```rust
        // Merge config: parent config overrides sub-phase config (on last sub-phase only)
        let mut merged_config = sub_phase.config.clone();
        if is_last {
            for (k, v) in &parent.config {
                merged_config.insert(k.clone(), v.clone());
            }
        }
```

with:

```rust
        // Merge config: parent config overrides sub-phase config (on last sub-phase only)
        let mut merged_config = sub_phase.config.clone();
        if is_last {
            for (k, v) in &parent.config {
                merged_config.insert(k.clone(), v.clone());
            }
        }
        if !with.is_empty() {
            substitute_in_value_map(&mut merged_config, with);
        }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: all tests pass.

- [ ] **Step 5: clippy/fmt**

Run: `cargo clippy --package belt-core -- -D warnings`
Run: `cargo fmt --package belt-core`

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "feat(belt-core): substitute Phase.config values using Invoker::Pipeline.with"
```

---

## Task 5: Apply substitution to `Invoker::Agents.iterations` (IterationsSpec)

**Files:**
- Modify: `crates/belt-core/src/expander.rs`.

- [ ] **Step 1: Add failing tests**

Append to the test module:

```rust
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
                assert_eq!(iterations, &IterationsSpec::Template("args.iterations".into()));
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
        let sub = mk_sub(vec![mk_leaf_with_agents_iter("leaf", IterationsSpec::Literal(9))]);
        let w = mk_with(&[("count", json!(5))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(crate::model::Invoker::Agents { iterations, .. }) => {
                assert_eq!(iterations, &IterationsSpec::Literal(9));
            }
            _ => panic!("expected Agents invoke"),
        }
    }
```

- [ ] **Step 2: Run tests to see failures**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: `iterations_template_rewritten_to_*` fail on value mismatch.

- [ ] **Step 3: Implement IterationsSpec rewrite**

Add a helper:

```rust
/// Substitute an `IterationsSpec::Template("args.<name>")` using `with_map`.
/// Number values become `IterationsSpec::Literal(v as u32)`. Non-template
/// strings become `IterationsSpec::Template(s)`. Non-convertible values
/// (bool, null, array, object, non-integer number) leave the original
/// Template intact.
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
        serde_json::Value::Number(n) => match n.as_u64() {
            Some(v) if v <= u64::from(u32::MAX) => {
                crate::model::IterationsSpec::Literal(v as u32)
            }
            _ => crate::model::IterationsSpec::Template(template.clone()),
        },
        serde_json::Value::String(s) => crate::model::IterationsSpec::Template(s),
        _ => crate::model::IterationsSpec::Template(template.clone()),
    };
}
```

Add an `Invoker`-level dispatcher that rewrites only the `Agents.iterations` case for now:

```rust
/// Apply `with_map` substitution to every rewrite-eligible field of an
/// `Invoker`. Called once per sub-phase after clone.
fn substitute_in_invoker(
    invoker: &mut crate::model::Invoker,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) {
    if let crate::model::Invoker::Agents { iterations, .. } = invoker {
        substitute_iterations_spec(iterations, with_map);
    }
}
```

Then wire it into `expand_sub_pipeline`. In the block that builds the `ExpandedPhase`, replace:

```rust
            invoke: sub_phase.invoke.clone(),
```

with:

```rust
            invoke: {
                let mut inv = sub_phase.invoke.clone();
                if !with.is_empty() {
                    if let Some(v) = inv.as_mut() {
                        substitute_in_invoker(v, with);
                    }
                }
                inv
            },
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: all tests pass.

- [ ] **Step 5: clippy/fmt**

Run: `cargo clippy --package belt-core -- -D warnings`
Run: `cargo fmt --package belt-core`

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "feat(belt-core): substitute Invoker::Agents.iterations using with-map"
```

---

## Task 6: Apply substitution to `Invoker::{Skill, Agent, Agents}.args` values

**Files:**
- Modify: `crates/belt-core/src/expander.rs` — extend `substitute_in_invoker`.

- [ ] **Step 1: Add failing tests**

Append:

```rust
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
            args: [("k".to_string(), json!("args.flag"))].into_iter().collect(),
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
```

- [ ] **Step 2: Run tests (expect failures)**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: three new tests fail on value mismatch.

- [ ] **Step 3: Extend `substitute_in_invoker`**

Replace `substitute_in_invoker` with:

```rust
fn substitute_in_invoker(
    invoker: &mut crate::model::Invoker,
    with_map: &std::collections::HashMap<String, serde_json::Value>,
) {
    use crate::model::Invoker;
    match invoker {
        Invoker::Skill { args, .. } => substitute_in_value_map(args, with_map),
        Invoker::Agent { args, .. } => substitute_in_value_map(args, with_map),
        Invoker::Agents { iterations, args, .. } => {
            substitute_iterations_spec(iterations, with_map);
            substitute_in_value_map(args, with_map);
        }
        Invoker::Pipeline { .. } => {
            // Nested Pipeline.with handled in Task 7.
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: all tests pass.

- [ ] **Step 5: clippy/fmt**

Run: `cargo clippy --package belt-core -- -D warnings`
Run: `cargo fmt --package belt-core`

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "feat(belt-core): substitute Invoker::{Skill,Agent,Agents}.args using with-map"
```

---

## Task 7: Apply substitution to nested `Invoker::Pipeline.with` values

**Files:**
- Modify: `crates/belt-core/src/expander.rs` — extend `substitute_in_invoker` for the `Pipeline` variant.

- [ ] **Step 1: Add failing test**

Append:

```rust
    #[test]
    fn invoker_pipeline_nested_with_rewritten() {
        use crate::model::Invoker;
        let parent = mk_parent_phase();
        let mut leaf = mk_leaf_phase("leaf", None);
        leaf.invoke = Some(Invoker::Pipeline {
            pipeline: "nested.yml".into(),
            with: [("inner".to_string(), json!("args.outer"))].into_iter().collect(),
        });
        let sub = mk_sub(vec![leaf]);
        let w = mk_with(&[("outer", json!("args.iterations"))]);
        let out = expand_sub_pipeline("p", &parent, &sub, &w);
        match &out[0].invoke {
            Some(Invoker::Pipeline { with: inner_with, .. }) => {
                assert_eq!(inner_with.get("inner"), Some(&json!("args.iterations")));
            }
            _ => panic!("expected Pipeline invoke"),
        }
    }
```

- [ ] **Step 2: Run test (expect failure)**

Run: `cargo test -p belt-core --lib expander::tests::invoker_pipeline_nested_with_rewritten`
Expected: assertion failure.

- [ ] **Step 3: Extend `substitute_in_invoker` for `Pipeline`**

Replace the placeholder branch:

```rust
        Invoker::Pipeline { .. } => {
            // Nested Pipeline.with handled in Task 7.
        }
```

with:

```rust
        Invoker::Pipeline { with: inner_with, .. } => {
            substitute_in_value_map(inner_with, with_map);
        }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p belt-core --lib expander::tests`
Expected: all tests pass.

- [ ] **Step 5: clippy/fmt**

Run: `cargo clippy --package belt-core -- -D warnings`
Run: `cargo fmt --package belt-core`

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/expander.rs
git commit -m "feat(belt-core): substitute nested Invoker::Pipeline.with values"
```

---

## Task 8: End-to-end integration test with a synthetic rename pipeline pair

**Files:**
- Create: `crates/belt-core/tests/expander_with_test.rs`.

- [ ] **Step 1: Create the integration test file**

Create `crates/belt-core/tests/expander_with_test.rs`:

```rust
//! End-to-end test: parent pipeline with a renamed `with` entry produces
//! expanded sub-phases whose `args.X` references point at the parent's
//! argument names.

use belt_core::expander::expand_pipeline;
use belt_core::model::{Invoker, IterationsSpec};
use std::fs;
use tempfile::tempdir;

#[test]
fn parent_with_rename_rewrites_sub_phase_iterations_template() {
    let dir = tempdir().expect("tempdir");
    let parent_path = dir.path().join("parent.yml");
    let sub_path = dir.path().join("custom-review.yml");

    fs::write(
        &sub_path,
        r#"
name: custom-review
version: 1
args:
  count: { type: number, default: 1 }
phases:
  - id: vote
    description: "Cast votes"
    invoke:
      agents: [v1, v2]
      iterations: "args.count"
"#,
    )
    .expect("write sub");

    fs::write(
        &parent_path,
        r#"
name: parent
version: 1
args:
  iterations: { type: number, default: 3 }
phases:
  - id: review
    invoke:
      pipeline: ./custom-review.yml
      with:
        count: "args.iterations"
"#,
    )
    .expect("write parent");

    let expanded = expand_pipeline(&parent_path).expect("expand");
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].id, "review/vote");
    match &expanded[0].invoke {
        Some(Invoker::Agents { iterations, .. }) => match iterations {
            IterationsSpec::Template(s) => assert_eq!(s, "args.iterations"),
            other => panic!("expected Template, got {other:?}"),
        },
        other => panic!("expected Agents invoke, got {other:?}"),
    }
}
```

- [ ] **Step 2: Confirm `expander` and `model` are publicly re-exported**

Run: `cargo test -p belt-core --test expander_with_test`
Expected: either the test passes, or `expander` / `model` / `expand_pipeline` fail to resolve.

If the modules are not public, inspect `crates/belt-core/src/lib.rs` and either (a) make the modules public or re-exports available, or (b) move the test into `crates/belt-core/tests/parser_test.rs` if that file already exercises the same API surface — match the style of the existing `belt32_full_pipeline_with_all_new_types` test.

- [ ] **Step 3: Ensure `tempfile` is available as a dev-dependency**

Inspect `crates/belt-core/Cargo.toml`. If `tempfile` is not present under `[dev-dependencies]`, add it (match the version pinning convention of the workspace). If the workspace already pins it under `[workspace.dependencies]`, inherit with `tempfile = { workspace = true }`.

- [ ] **Step 4: Run the integration test**

Run: `cargo test -p belt-core --test expander_with_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/expander_with_test.rs crates/belt-core/Cargo.toml
git commit -m "test(belt-core): add integration test for expander with-rename"
```

---

## Task 9: Workspace verification + adversarial probe

**Files:** none modified.

- [ ] **Step 1: Run the full belt-core test suite**

Run: `cargo test -p belt-core`
Expected: all tests pass including Task 1–8 additions and the pre-existing 218 tests.

- [ ] **Step 2: Run belt-agent tests (integration with expander via Engine)**

Run: `cargo test -p belt-agent`
Expected: 40+ tests pass (unchanged — identity renames in `examples/` mean no behavior change).

- [ ] **Step 3: Run clippy workspace-scoped for touched crates**

Run: `cargo clippy --package belt-core --package belt-agent -- -D warnings`
Expected: no warnings.

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --package belt-core --package belt-agent -- --check`
Expected: no diff.

- [ ] **Step 5: Run `belt lint` on every example pipeline**

Run:
```bash
cargo run --quiet --bin belt -- lint examples/skills/feature-dev/pipeline.yml
cargo run --quiet --bin belt -- lint examples/skills/debug-flow/pipeline.yml
cargo run --quiet --bin belt -- lint examples/skills/spec-review/pipeline.yml
cargo run --quiet --bin belt -- lint examples/skills/implementation-review/pipeline.yml
cargo run --quiet --bin belt -- lint examples/skills/code-review/pipeline.yml
cargo run --quiet --bin belt -- lint examples/skills/test-review/pipeline.yml
cargo run --quiet --bin belt -- lint examples/skills/smoke-test/pipeline.yml
```

Expected: all 7 exit 0.

- [ ] **Step 6: Identity-rename smoke via belt-agent init**

In a scratch dir:

```bash
SCRATCH=$(mktemp -d)
cd "$SCRATCH"
cargo run --quiet --bin belt-agent --manifest-path "$OLDPWD/Cargo.toml" -- \
    init "$OLDPWD/examples/skills/feature-dev/pipeline.yml" \
    --arg iterations=3 --arg codex=false --arg ui=false --arg swarm=false \
    --arg e2e=false --arg smoke=false --arg doc=false
cargo run --quiet --bin belt-agent --manifest-path "$OLDPWD/Cargo.toml" -- \
    status | head -60
cd "$OLDPWD"
```

Expected: init prints a first phase; `status` JSON contains `spec-review/review` (or similar) with `invoke.agents.iterations` still showing `"args.iterations"` (identity rename preserved).

- [ ] **Step 7: Adversarial rename probe**

Create a temporary pipeline pair that exercises a non-identity rename:

```bash
PROBE=$(mktemp -d)
cat > "$PROBE/parent.yml" <<'EOF'
name: probe-parent
version: 1
args:
  iterations: { type: number, default: 3 }
phases:
  - id: review
    invoke:
      pipeline: ./custom-review.yml
      with:
        count: "args.iterations"
EOF
cat > "$PROBE/custom-review.yml" <<'EOF'
name: probe-sub
version: 1
args:
  count: { type: number, default: 1 }
phases:
  - id: vote
    description: "vote"
    invoke:
      agents: [v1]
      iterations: "args.count"
EOF
cargo run --quiet --bin belt -- lint "$PROBE/parent.yml"
```

Expected: `belt lint` succeeds. Then run:

```bash
cd "$PROBE"
cargo run --quiet --bin belt-agent --manifest-path "$OLDPWD/Cargo.toml" -- \
    init "$PROBE/parent.yml" --arg iterations=5
cargo run --quiet --bin belt-agent --manifest-path "$OLDPWD/Cargo.toml" -- \
    status
cd "$OLDPWD"
rm -rf "$PROBE"
```

Expected: `status` shows the `review/vote` sub-phase with `invoke.agents.iterations` rewritten away from `"args.count"` — either `"args.iterations"` or `5` (depending on how your CLI `--arg` parses the number; both are acceptable since the orchestrator resolution layer is unaffected).

- [ ] **Step 8: No new commit**

Verification is documentation-only; it does not produce new commits. Task 10 handles the final commit & memory update.

---

## Task 10: Memory update (Knowledge Capture) + optional push

**Files:**
- Modify (memory store): `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_belt32_invoker_artifact.md` — mark follow-up #7 as completed, append dated note with commit range.

- [ ] **Step 1: Update the existing BELT-32 memory file**

Edit `project_belt32_invoker_artifact.md`. In the "Outstanding follow-ups" list, change item 7 from "新発見 (2026-04-13): expander ..." to "✅ 完了 (2026-04-14、commits <range>)". Add a short summary of what was implemented (AST-level rewrite at expand time, 5 target fields, 9 commits).

- [ ] **Step 2: Run `git log --oneline` to get the commit range**

Run: `git log --oneline origin/main..HEAD`
Expected: the Task 1–8 commits (and Task 0 design spec if that hasn't been pushed).

- [ ] **Step 3: Update memory (Edit tool on the memory file, not a git commit)**

The memory file is outside the repo. No commit.

- [ ] **Step 4: Ask the user before pushing**

Do NOT push without explicit user approval. Summarize: commit range, brief changelog, and propose `git push origin main`.

---

## Self-Review Results

- **Spec coverage**
  - §"Substitution model" → Task 1 (helper), Task 3 (when), Task 4 (config), Task 5 (iterations), Task 6 (invoke args), Task 7 (nested pipeline with).
  - §"Typed field conversions" — IterationsSpec Number→Literal → Task 5 Step 3; Phase.when Bool→string → Task 3 Step 3.
  - §"Error & edge cases" — empty with fast-path → Task 2 gate; non-template string and conversion failure → Task 5 tests `iterations_template_unchanged_when_with_value_not_convertible` and Task 1 edge-case tests.
  - §"Tests" 7 cases — Task 1 + 3 + 4 + 5 + 6 + 7 cover identity, rename, literal-number, literal-bool, nested, no-op, conversion-failure.
  - §"Verification" — Task 9 steps 1–6 cover items 1–6; Task 9 step 7 covers item 7 (adversarial probe).
- **Placeholder scan** — no TBD/TODO; all tests and impls contain complete code.
- **Type consistency** — `substitute_arg_in_value` signature stable across tasks; `substitute_in_invoker` grows monotonically (Task 5 adds Agents-iterations, Task 6 adds args, Task 7 adds nested Pipeline). `substitute_in_value_map` introduced in Task 4 and reused in Task 6/7.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-04-14-expander-with-merge-plan.md`. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task (Tasks 1–8), review between tasks, then run Task 9 verification inline.
2. **Inline Execution** — execute all tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

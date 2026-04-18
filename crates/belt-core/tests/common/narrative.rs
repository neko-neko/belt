//! Shared narrative-artifact test helpers for pipeline integration tests.
//!
//! Used by `feature_dev_refresh.rs` and `bug_fix_refresh.rs`. Both pipelines
//! follow the same accumulating narrative contract defined in
//! `docs/specs/2026-04-15-narrative-artifact-design.md` (as amended by the
//! 2026-04-18 `belt://current` URI migration): each narrative phase produces
//! `belt://current/notes/phase-<id>.md`, gates on that same URI via
//! `file_exists`, and consumes all prior narrative notes as `ArtifactRef::Named`.
//!
//! `Artifact.path` is a `String` (not `Option<String>`), so the helpers below
//! compare paths directly rather than through `as_deref()`.

use belt_core::model::{Artifact, ArtifactRef, GateCheck, Phase, Pipeline};

/// Find a `Phase` by id, panicking on miss. Private to the module; the
/// `assert_*` helpers below are the public surface.
fn find_phase<'a>(pipeline: &'a Pipeline, id: &str) -> &'a Phase {
    pipeline
        .phases
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("phase '{id}' must exist"))
}

/// Find a produced `Artifact` by name within a `Phase`.
fn find_produce<'a>(phase: &'a Phase, name: &str) -> &'a Artifact {
    phase
        .produces
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("phase '{}' must produce '{name}'", phase.id))
}

/// True if `phase` has a `file_exists: <path>` gate.
fn has_file_exists_gate(phase: &Phase, path: &str) -> bool {
    phase
        .gate
        .iter()
        .any(|g| matches!(g, GateCheck::FileExists { file_exists } if file_exists == path))
}

/// True if `phase` consumes `name` as `ArtifactRef::Named`.
fn has_named_consume(phase: &Phase, name: &str) -> bool {
    phase
        .consumes
        .iter()
        .any(|r| matches!(r, ArtifactRef::Named(n) if n == name))
}

/// For each `(phase_id, produce_name, expected_path)` tuple, assert the produce
/// exists at the expected path.
pub(crate) fn assert_narrative_produce_paths(pipeline: &Pipeline, rows: &[(&str, &str, &str)]) {
    for (phase_id, produce_name, expected_path) in rows {
        let phase = find_phase(pipeline, phase_id);
        let artifact = find_produce(phase, produce_name);
        assert_eq!(
            artifact.path, *expected_path,
            "phase '{phase_id}' note '{produce_name}' path mismatch"
        );
    }
}

/// For each `(phase_id, produce_name, expected_path)` tuple, assert the phase
/// has a `file_exists` gate on that path. Narrative phases gate on the path
/// they produce, so `produce_name` appears in the assertion message to pin the
/// invariant to the shared constant's tuple shape.
pub(crate) fn assert_narrative_gate_paths(pipeline: &Pipeline, rows: &[(&str, &str, &str)]) {
    for (phase_id, produce_name, expected_path) in rows {
        let phase = find_phase(pipeline, phase_id);
        assert!(
            has_file_exists_gate(phase, expected_path),
            "phase '{phase_id}' must gate on '{produce_name}' at '{expected_path}'"
        );
    }
}

/// For each `(phase_id, expected_consumes)` pair, assert every expected name
/// is a named consume.
pub(crate) fn assert_narrative_accumulating_consumes(
    pipeline: &Pipeline,
    rows: &[(&str, &[&str])],
) {
    for (phase_id, expected_consumes) in rows {
        let phase = find_phase(pipeline, phase_id);
        for name in *expected_consumes {
            assert!(
                has_named_consume(phase, name),
                "phase '{phase_id}' must consume '{name}'"
            );
        }
    }
}

/// For each `phase_id`, assert it has no produce whose path references a
/// belt narrative note (either the legacy `.belt/runs/` literal form or the
/// current `belt://current/notes/` URI form).
pub(crate) fn assert_non_narrative_phases_have_no_notes(pipeline: &Pipeline, phase_ids: &[&str]) {
    for phase_id in phase_ids {
        let phase = find_phase(pipeline, phase_id);
        for artifact in &phase.produces {
            assert!(
                !artifact.path.starts_with(".belt/runs/"),
                "phase '{phase_id}' must not produce belt notes, got '{}'",
                artifact.path
            );
            assert!(
                !artifact.path.starts_with("belt://current/notes/"),
                "phase '{phase_id}' must not produce belt notes, got '{}'",
                artifact.path
            );
        }
    }
}

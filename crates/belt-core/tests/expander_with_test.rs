// Integration test: parent pipeline with a renamed `with` entry produces
// expanded sub-phases whose `args.X` references point at the parent's
// argument names. Pedantic `expect_used`/`panic` lints are allowed file-wide
// because clear panic-on-mismatch assertions are the plan-specified form.
//
// The original `parent_with_rename_rewrites_sub_phase_iterations_template`
// test was removed on 2026-04-16 together with `Invoker::Agents` /
// `IterationsSpec` (see docs/specs/2026-04-16-review-skills-subagent-
// boundary-design.md). The remaining expander with-merge coverage lives in
// `crates/belt-core/src/expander.rs`'s unit test module (Skill / Pipeline
// args rewriting).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::match_wildcard_for_single_variants,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

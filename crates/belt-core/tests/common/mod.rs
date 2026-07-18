#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    dead_code,
    reason = "test helpers use panic-on-mismatch per workspace convention; dead_code exempted centrally because each integration test binary uses a different subset of helpers"
)]

pub(crate) mod helpers;

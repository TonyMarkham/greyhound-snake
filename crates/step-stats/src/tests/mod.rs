// Tests may use unwrap/panic freely (AGENTS.md); the workspace lints
// deny `panic` for production code, so the test tree opts out here —
// the same opt-out every other test module declares.
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

mod app_config;

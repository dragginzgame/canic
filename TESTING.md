# Testing Layout Rules

This file documents the canonical test layout for `canic-core`.
Follow these rules during refactors to prevent test sprawl.

## Rules

- Unit tests live next to the code under `crates/canic-core/src/...` with `#[cfg(test)]`.
- Seam/workflow tests that need `crate::` internals live under `crates/canic-core/src/test/`.
- PocketIC/system tests live under `crates/canic-core/tests/*.rs` (top-level only).
- Avoid `#[path = "..."]` in tests; use top-level files in `tests/`.
- Test canister crates are not tests; keep them outside `tests/` (e.g. `crates/canic-core/test-canisters/`).
- Tests that need private internals must not be promoted to public API; use `cfg(test)` or feature-gated test exports.

# Canisters

This directory contains runnable canister crates that are not config-defined
Canic Apps. They may still be Cargo workspace members, ICP build targets, or
PocketIC fixtures, but `canic app list` must not discover them as Apps.

## Layout

- `audit/` – internal audit and performance probe canisters used by instruction,
  wasm-size, and capability-surface audits, including separate `minimal` and
  `minimal_metrics` shared runtime baselines.
- `sandbox/blank/` – manual local sandbox for temporary endpoint experiments.
  It uses `canic::start_local!()` with generated declared-only standalone
  config and is not part of `icp.yaml`, the demo topology, the reference
  release set, or automated test fixtures.
- `test/` – isolated PocketIC and integration-test fixture canisters that are
  not themselves a config-defined App.

## Local Workflow

- Build the sandbox manually:
  `CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host --example build_artifact -- sandbox_blank release . . canisters/sandbox/blank/canic.toml`
- Build audit probes through Cargo; `canister_minimal` is the no-metrics floor
  and `canister_minimal_metrics` enables the leaf metrics profile:
  `cargo check -p canister_minimal -p canister_minimal_metrics -p audit_leaf_probe -p audit_root_probe -p audit_scaling_probe`
- Build isolated test fixtures through Cargo, for example:
  `cargo check -p runtime_probe -p payload_limit_probe`

# Canisters

This directory contains runnable canister crates that are not config-defined
Canic fleets. They may still be Cargo workspace members, ICP build targets, or
PocketIC fixtures, but `canic fleet list` must not discover them as fleets.

## Layout

- `audit/` – internal audit and performance probe canisters used by instruction,
  wasm-size, and capability-surface audits, including the `minimal` shared
  runtime baseline.
- `sandbox/minimal/` – manual local sandbox for temporary endpoint experiments.
  It uses `canic::start_local!()` with generated standalone config and is not
  part of `icp.yaml`, the demo topology, the reference release set, or automated
  test fixtures.
- `test/` – isolated PocketIC and integration-test fixture canisters that are
  not themselves a config-defined fleet.

## Local Workflow

- Build the sandbox manually:
  `cargo run -q -p canic-host --example build_artifact -- sandbox_minimal`
- Build audit probes through Cargo, for example:
  `cargo check -p canister_minimal -p audit_leaf_probe -p audit_root_probe -p audit_scaling_probe`
- Build isolated test fixtures through Cargo, for example:
  `cargo check -p runtime_probe -p payload_limit_probe`

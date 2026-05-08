# Canisters

This directory contains runnable canister crates that are not config-defined
Canic fleets. They may still be Cargo workspace members, dfx build targets, or
PocketIC fixtures, but `canic fleet list` and implicit install config selection
must not discover them as fleets.

## Layout

- `audit/` – internal audit and performance probe canisters used by instruction
  and capability-surface audits.
- `sandbox/minimal/` – manual local sandbox for temporary endpoint experiments.
  It uses `canic::start_local!()` with generated standalone config and is not
  part of `dfx.json`, the demo topology, the reference release set, or automated
  test fixtures.

## Local Workflow

- Build the sandbox manually: `scripts/app/build.sh sandbox_minimal`
- Build audit probes through Cargo, for example:
  `cargo check -p audit_leaf_probe -p audit_root_probe -p audit_scaling_probe`

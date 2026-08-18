# Fleet Orchestration

A Fleet is one installed instance of an App on one network. Canic qualifies
its infrastructure and application artifacts, then coordinates installation
through one Fleet Coordinator and a Fleet Subnet Root with a root-local Wasm
Store on every occupied physical Subnet.

## What It Provides

- explicit network trust enrollment and canonical network identity
- operator-owned Fleet installation input separate from App configuration
- qualified Coordinator, root, Store, and application artifact sets
- journaled create, install, registration, and activation workflows
- Coordinator Registry plus independently validated root mirrors/directories
- root-owned canister lifecycle effects and bounded prepaid inventory

The host CLI orchestrates the workflow while the Coordinator and roots retain
the durable on-chain authority needed for exact retry and reconciliation.

## Boundary

Application canisters never receive filesystem, repository, identity-key, or
operator configuration authority. A root owns platform effects on its Subnet;
the Coordinator owns Fleet-wide planning and publication. Current release
transitions are reinstall-only. Scheduled 0.109 defines one exact stop-the-
world predecessor/successor exception without permitting mixed-version or
arbitrary historical adoption.

## Start Here

- [Installing Canic](../../../INSTALLING.md)
- [Fleet installation input](../../architecture/fleet-install-input.md)
- [Build artifact architecture](../../architecture/build-artifacts.md)
- [Host library guide](../../../crates/canic-host/README.md)
- [Current implementation status](../../status/current.md)

# canic-fleet-coordinator

Canonical Fleet Coordinator canister crate for Canic.

This package gives downstream workspaces a published artifact source for the
built-in `fleet_coordinator` role. Ordinary Canic applications do not declare
or customize this role: the host selects the exact package matching the
resolved `canic` release and falls back to a generated runtime-only wrapper
when the canonical package source is unavailable.

The package is a canister artifact source, not a reusable Rust dependency. It
builds only as a `cdylib`; shared types and behavior remain owned by `canic`,
`canic-core`, and `canic-control-plane`.

Unlike an App-owned Fleet Subnet Root, the Coordinator has no compiled App
configuration or build script. Its Cargo package metadata and exact runtime-only
host contract own the built-in identity; a package-local `canic.toml` would
incorrectly model it as an App-owned role. The complete runtime is the
maintained `fleet-coordinator-canister` feature bundle plus the standard Canic
lifecycle entrypoints.

## Canonical DID ownership

[`fleet_coordinator.did`](fleet_coordinator.did) is the checked-in canonical
interface for this crate. Ordinary local artifact builds copy it into
`.icp/local/canisters/fleet_coordinator/fleet_coordinator.did` and embed it as
local Wasm metadata without compiling a second debug Wasm.

After an intentional endpoint change, refresh and check the contract from the
Canic workspace with:

```bash
CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host --example build_artifact -- \
  fleet_coordinator debug . . apps/test/canic.toml --refresh-canonical-did
git diff --exit-code -- crates/canic-fleet-coordinator/fleet_coordinator.did
```

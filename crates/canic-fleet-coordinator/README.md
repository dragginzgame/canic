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

# canic-testing-internal

Workspace-only internal test support for Canic self-tests.

This crate is intentionally unpublished.

It owns the Canic-specific test seams that should not expand the public
`canic-testkit` API surface, including:
- root-topology setup and cached baselines
- attestation/delegation-specific PocketIC fixtures
- internal audit probes and root-only test helpers
- repo-only wiring between reference canisters and test harness code

Use this crate only for Canic's own workspace tests.
Downstream projects should prefer `canic-testkit`, which exposes the generic
publishable PocketIC/test helper surface without these repo-specific fixtures.

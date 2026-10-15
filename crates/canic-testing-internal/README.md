# canic-testing-internal

Workspace-only internal test support for Canic self-tests.

This crate is intentionally unpublished.

It owns the Canic-specific test seams that should not expand the reusable
`ic-testkit` API surface, including:
- root-topology setup and cached baselines
- attestation/delegation-specific PocketIC fixtures
- internal audit probes and root-only test helpers
- repo-only wiring between reference canisters and test harness code

In practice, this crate is where Canic keeps:
- the root baseline/artifact preparation flow used by the heavy PocketIC suites
- repo-only fixture policy that would be too opinionated for the reusable `ic-testkit` surface
- harness code that is allowed to know about reference roles, root release staging, and internal test-only endpoints

Use this crate only for Canic's own workspace tests.
Downstream projects should prefer `ic-testkit`, which exposes the generic
PocketIC/test helper surface without these repo-specific fixtures.

Ordinary native tests use normal libtest discovery in the workspace library
invocation. The library test binary compiles the stateful Fleet catalogue only
with `governed-pocketic-tests`; fixture-library consumers retain the default
`pocketic-fixtures` surface. Use the governed runner: source-bound activation
recovery precedes two isolated internal workers, with serial execution within
each worker. Exact single-case selection stays serial. See the
[testing guide](../../TESTING.md) for targeted commands and retained diagnostics.

## Continue From Here

- [Read the testing rules](../../TESTING.md)
- [Browse the test canisters](../../canisters/README.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)

## License

Source-available under the Dragginz Verifiable ICP Burn license (packaged as `LICENSE`).
Third-party Live Deployments require verifiable mainnet ICP burning: 100% of
ordinary protocol resource funding, or at least 20% of Cloud Engine infrastructure
value. Private and self-hosted deployments are included; zero-price arrangements
do not waive the requirement. Toko Studios Ltd has the express operational
exception in section 5. Earlier grants and third-party rights remain intact.

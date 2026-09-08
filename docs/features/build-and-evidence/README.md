# Builds, Provenance, And Evidence

Canic keeps artifact construction separate from deployment decisions. It can
build attached roles, record exactly what produced an artifact, compare saved
evidence envelopes, and apply passive policy without turning those reads into
Fleet mutation authority.

## What It Provides

- role-aware Wasm and Candid artifact construction
- deterministic artifact identities and checked configuration inputs
- optional build-provenance output for CI and release pipelines
- generic evidence envelopes and policy manifests
- passive policy gates over previously captured evidence

Evidence envelopes are designed for transport and comparison in CI. They
preserve the underlying report and its input fingerprints without claiming
that a deployment mutation happened.

The host generates thin Cargo packages for Root, Coordinator and Store under
`.canic/generated/` beside the selected configuration. All three bind the exact
Canic dependency and use unpublished build packages. Root selects configured
capabilities, Store compiles its configuration, and Coordinator remains
independent of App configuration. Cargo graphs are validated before artifact
finalization. Canonical Coordinator/Store Candid ships in `canic/candid`; Root
Candid follows its configured capabilities.

## Boundary

Build provenance is not runtime attestation. Evidence and policy commands do
not install canisters, change controllers, sign artifacts, import registries,
or adopt discovered resources. Fleet mutation remains solely in the reviewed
`canic fleet ensure` workflow.

## Start Here

- [Build artifact architecture](../../architecture/build-artifacts.md)
- [Evidence envelopes](../../architecture/evidence-envelopes.md)
- [CI policy gates](../../architecture/ci-policy-gates.md)
- [Managed-App qualification](managed-app-qualification.md)
- [Operator walkthrough](../../architecture/v1-operator-walkthrough.md)

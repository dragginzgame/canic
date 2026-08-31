# Canic Architecture

This directory contains current approved Canic system design notes.

For short capability overviews before entering these detailed designs, start
with the [feature guides](../features/README.md).

Use these documents as the maintained architecture baseline for implementation,
reviews, and developer handoff. Versioned WIP and release-line plans belong in
`docs/design/`; point-in-time audit evidence belongs in `docs/audits/`; exact
runtime/wire contracts belong in `docs/contracts/`.

Current architecture notes:

- [Authentication](authentication.md)
- [Build Artifacts](build-artifacts.md)
- [Build Provenance CI Policy](build-provenance-ci-policy.md)
- [CI Policy Gates](ci-policy-gates.md)
- [Evidence Envelopes](evidence-envelopes.md)
- [Fleet Ensure](../features/operations/fleet-ensure.md)
- [V1 Readiness Checklist](v1-readiness-checklist.md)
- [V1 Operator Walkthrough](v1-operator-walkthrough.md)

Current implementation design and handoff:

- [0.100 Multi-Subnet Fleet Coordinator and Registry Synchronization](../design/archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md)
- [0.100 Implementation Status](../design/archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/status.md)
- [Current Repository Status](../status/current.md)

Historical/superseded notes:

- [Authentication Subnet-State Addendum](authentication-subnet-state-addendum.md)

Operational guidance starts at
[Fleet ensure](../features/operations/fleet-ensure.md).

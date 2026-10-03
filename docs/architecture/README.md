# Canic Architecture

Architecture pages explain how Canic's major parts fit together and why their
security and ownership boundaries exist. They are intended for contributors,
reviewers, and integrators who need more detail than a feature guide.

<img src="../../assets/256x256/mechanic-help.png" align="left" width="110" alt="The Canic mechanic pointing readers toward the right architecture material" />

If you are learning or using Canic, start with the
[feature guides](../features/README.md). Return here when you need the design
behind a feature. For exact machine-facing rules, use the linked contracts
rather than treating an overview as an API specification.

Use these documents as the maintained architecture baseline for implementation,
reviews, and developer handoff. Versioned WIP and release-line plans belong in
`docs/design/`; point-in-time audit evidence belongs in `docs/audits/`; exact
runtime/wire contracts belong in `docs/contracts/`.

<br clear="left" />

## Choose The Right Level

| Question | Read |
| --- | --- |
| What can Canic do? | [Feature guides](../features/README.md) |
| How do I operate it? | [Operations and diagnostics](../features/operations/README.md) |
| Why is the system designed this way? | Architecture pages below |
| What exact machine-facing behavior is required? | [Contracts](../contracts/ARCHITECTURE.md) |
| What work is accepted or planned? | [Design roadmap](../design/README.md) |
| What evidence supports a finding? | [Audit index](../audits/README.md) |

## Maintained Architecture

- [Authentication](authentication.md)
- [Build Artifacts](build-artifacts.md)
- [Build Provenance CI Policy](build-provenance-ci-policy.md)
- [CI Policy Gates](ci-policy-gates.md)
- [Evidence Envelopes](evidence-envelopes.md)
- [Fleet Ensure](../features/operations/fleet-ensure.md)
- [V1 Readiness Checklist](v1-readiness-checklist.md)
- [V1 Operator Walkthrough](v1-operator-walkthrough.md)

## Current Design And Handoff

- [Design Roadmap](../design/README.md)
- [0.110 Fleet Runtime Contraction](../design/0.110-fleet-runtime-contraction/0.110-design.md)
- [0.110 Implementation Status](../design/0.110-fleet-runtime-contraction/status.md)
- [Current Repository Status](../status/current.md)

## Historical And Superseded Notes

- [0.100 Multi-Subnet Fleet Coordinator and Registry Synchronization](../design/archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md)
- [Authentication Subnet-State Addendum](authentication-subnet-state-addendum.md)

Operational guidance starts at
[Fleet ensure](../features/operations/fleet-ensure.md).

## Continue From Here

- [Choose a feature](../features/README.md)
- [Operate a Fleet](../features/operations/README.md)
- [Review exact contracts](../contracts/ARCHITECTURE.md)
- [Review the design roadmap](../design/README.md)
- [Find audit methods and evidence](../audits/README.md)
- [Browse all documentation](../README.md)

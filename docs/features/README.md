# Canic Features

Canic is a set of independent capabilities rather than one mandatory runtime
stack. This directory provides stable entry points for those capabilities.
Each guide explains what the feature owns, what it deliberately does not own,
and where its authoritative configuration, contracts, and runbooks live.

These guides are navigation documents. Exact schemas remain in `CONFIG.md`,
wire and authority rules remain under `docs/contracts/`, architecture remains
under `docs/architecture/`, and operator procedures remain under
`docs/operations/`.

## Feature Guides

| Capability | Use it for | Guide |
| --- | --- | --- |
| Canister runtime | Lifecycle, memory, typed calls, timers and metrics | [Runtime](runtime/README.md) |
| Authentication | Endpoint guards, delegated subjects, proof renewal and attestation | [Authentication](authentication/README.md) |
| Fleet orchestration | Reviewed desired-state effects, recovery and cycle conservation | [Fleet orchestration](fleet-orchestration/README.md) |
| Scaling and placement | Specs, Groups, services, children, pools and limits | [Scaling and placement](scaling-and-placement/README.md) |
| Builds and evidence | Artifacts, provenance, policy gates and managed-App qualification | [Builds and evidence](build-and-evidence/README.md) |
| Backup and restore | Host-side snapshots, verification and same-release recovery | [Backup and restore](backup-and-restore/README.md) |
| Blob storage | Optional product-data storage and Cashier-backed billing | [Blob storage](blob-storage/README.md) |
| Operations and diagnostics | CLI workflows, network trust, local replicas and Fleet ensure | [Operations and diagnostics](operations/README.md) |

For the exact delivery boundary of work in progress, see
[Current Status](../status/current.md).

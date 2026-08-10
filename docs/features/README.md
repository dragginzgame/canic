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

- [Canister runtime](runtime/README.md) — lifecycle, build integration, memory,
  calls, timers, and metrics.
- [Authentication](authentication/README.md) — endpoint guards, delegated
  tokens, proof renewal, and role attestation.
- [Fleet orchestration](fleet-orchestration/README.md) — Coordinator, roots,
  Stores, registries, installation, and lifecycle authority.
- [Scaling and placement](scaling-and-placement/README.md) — Component Specs,
  Groups, services, children, sharding, scaling, and limits.
- [Builds and evidence](build-and-evidence/README.md) — artifacts, provenance,
  evidence envelopes, policy gates, adoption, and catalogs.
- [Backup and restore](backup-and-restore/README.md) — snapshots, manifests,
  journals, verification, planning, and restore execution.
- [Blob storage](blob-storage/README.md) — optional product-data storage and
  billing integration.
- [Operations and diagnostics](operations/README.md) — CLI workflows, network
  trust, local replicas, inspection, medic, and recovery.

For the exact delivery boundary of work in progress, see
[Current Status](../status/current.md).

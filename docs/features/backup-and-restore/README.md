# Backup and restore

## Current availability

Live backup and restore CLI operations are unavailable in the current Canic tree.
The maintainer approved an immediate pre-1.0 hard cut on 2026-10-29: the private
`canic-backup` crate and the old CLI command groups are removed. There are no
compatibility aliases, forwarding package or copied engine in Host. This decision
supersedes the earlier plan to preserve the native restore runner during migration.

Existing backup data, retained journals and release evidence are not deleted or
rewritten by this source change. Historical operation records are evidence, not
permission to allocate new attempts or spending in the upstream engine.

## Ownership

[IC Backup](https://github.com/dragginzgame/ic-backup) owns generic plans, artifacts,
immutable manifests, original stage/attempt journals, bounded workflow advancement,
progress inspection and guarded source-reference release. Canic selects published
Backup and Backup Agent 0.50.2. No local dependency override is required.

Canic retains Fleet identities, exact release/controller authority, application
checkpoint hooks and the thin `canic_host::backup::checkpoint` integration.
The adapter delegates accounting to upstream owners and requires explicit fresh
admission and durable ingress/reply custody. It provides no default live provider.
Root checkpoint admission and participant/runtime fences remain in Canic.

## Verified integration and limits

The focused Host tests cover bounded stage advancement, cold continuation,
original lost-reply recovery and effect-free terminal replay. The installed
Root/two-participant checkpoint case passed on the recorded 0.49.1 inputs, including
fixture-authorized snapshot/load and typed refusal of pending redispatch. Evidence
is retained in `target/backup-0491-adoption/README.md`.
The earlier 0.50.0 selection also passes the corresponding installed Host workflow;
the [current handoff](../../status/current.md) records its exact graph and focused checks.

These proofs do not establish production Agent trust, Root-paid capture, complete
artifact transfer, application consistency or safe live restore/start. A stage
limit bounds new stages; it is not an elapsed-time or new spending budget.
Inspection cannot settle unknown effects or supply missing learned evidence.

## Remaining production integration

Future operator commands must consume the upstream owners directly and qualify:

- Current Fleet/network/release and exact controller/unit admission.
- Intent-before-effect accounting, signed ingress/reply custody and reconciliation
  of uncertain original effects without replacement budgets.
- Complete snapshot transfer and immutable source publication.
- Stopped restore, application post-load acceptance, accepted start and guarded
  terminal source-reference release.

[Canic #513](https://github.com/dragginzgame/canic/issues/513) tracks integration;
[#394](https://github.com/dragginzgame/canic/issues/394) and
[#550](https://github.com/dragginzgame/canic/issues/550) retain live/application
acceptance. Upstream composition was supplied through
[IC Backup #75](https://github.com/dragginzgame/ic-backup/issues/75).

See [independent package ownership](../../architecture/independent-package-adapters.md)
and [CLI documentation](../../../crates/canic-cli/README.md).

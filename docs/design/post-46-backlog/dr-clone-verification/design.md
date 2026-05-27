# Post-46 Backlog: DR, Clone, And Operational Verification

## Status

TBD.

This is a post-0.46 backlog topic. It captures deployment clone, trust-domain
migration, disaster recovery coordination, and post-deploy verification. It is
not a promised numbered follow-on release.

---

## Goal

A future line may give operators explicit plans for cloning, verification, and
role-scoped recovery while preserving trust-domain, authority, artifact, and
verification evidence.

Core operator question:

```text
Can I clone, verify, or recover part of a deployment without confusing
receipts with backups or crossing trust domains accidentally?
```

---

## Dependency On Completed Deployment Foundation

This topic reuses:

- 0.41 deployment plans, inventories, receipts, safety reports, and materialized
  artifact facts;
- 0.42 authority reconciliation;
- 0.43 executor boundaries;
- 0.44 artifact promotion and readiness reports;
- 0.45 external lifecycle proposals and receipts;
- 0.46 deployment groups and comparisons;
- post-46 artifact registry and retention where available;
- post-46 adoption profiles where a deployment is partial or brownfield;
- post-46 provenance, JSON, signing, and CI lock contracts where automation is
  involved.

0.46 may compare verification artifacts if present.

This topic would produce those artifacts and own the verification command and
profile if promoted into a real release line.

---

## Core Decision

This topic coordinates clone, migration, DR, and verification using deployment
truth objects.

It does not replace backup/restore journals or make receipts into backups.

Rollback and restore are explicit plan types. A failed deployment receipt can
prove partial state and provide rollback evidence, but it is not a rollback
guarantee by itself.

---

## Deployment Clone And Migration

Clone and migration are explicit plan types.

Tentative clone shape:

```text
DeploymentClonePlanV1 {
  source_deployment_id,
  target_deployment_id,
  target_root_policy,
  artifact_source_policy,
  authority_profile,
  migration_steps,
  verification_profile,
}
```

Trust-domain migration must be explicit:

```text
TrustDomainMigrationPlanV1 {
  old_root,
  new_root,
  migration_reason,
  authority_actions,
  artifact_actions,
  verifier_actions,
  required_external_actions,
}
```

A clone plan should never silently reuse source authority, source root,
controllers, or pool canister IDs. It may reuse artifact identity only through
0.44 promotion semantics and post-46 registry evidence where available.

---

## Operational Verification

Verification profiles define post-deploy checks:

```text
VerificationProfileV1 {
  readiness_checks,
  metadata_version_checks,
  cycle_floor_checks,
  protected_call_checks,
  metrics_snapshot_policy,
}
```

Verification results are evidence:

```text
VerificationResultV1 {
  deployment_id,
  profile_id,
  observed_at,
  passed,
  failed_checks,
  warnings,
  evidence,
}
```

Verification should check readiness, metadata skew, cycle floors, protected
call paths where applicable, and metrics snapshots according to the selected
profile.

---

## DR And Restore Integration

Deployment receipts are not backup journals, but recovery plans may reference
snapshots or backup artifacts.

Before a role upgrade, earlier lines may record rollback evidence such as:

```text
PreUpgradeRecoveryEvidenceV1 {
  role,
  previous_module_hash,
  previous_artifact_digest_or_locator,
  previous_canonical_embedded_config_sha256,
  snapshot_id_if_available,
  stable_state_compatibility_note,
}
```

This evidence can help build an explicit rollback or restore plan. It must not
be treated as proof that rollback is possible without live authority,
available artifacts or snapshots, and postcondition verification.

Snapshot reference:

```text
SnapshotReferenceV1 {
  snapshot_id,
  deployment_id,
  roles,
  source_kind,
  created_at,
}
```

Role-scoped restore plan:

```text
RoleRestorePlanV1 {
  deployment_id,
  role,
  snapshot_reference,
  target_canister,
  authority_requirements,
  verification_requirements,
}
```

Restore must remain authority-aware and postcondition-verified.

---

## Data Model

Core objects:

```text
DeploymentClonePlanV1
TrustDomainMigrationPlanV1
VerificationProfileV1
VerificationResultV1
SnapshotReferenceV1
RoleRestorePlanV1
```

All of them reference existing deployment truth objects rather than replacing
them.

---

## Commands

Tentative operator surface:

```text
canic deploy clone-plan <source> <target>
canic deploy verify <deployment>
canic deploy snapshot <deployment>
canic restore role <deployment> <role>
```

Command names are tentative. The stable part is the clone, migration,
verification, and restore plan model.

---

## Non-Goals

This future work should not:

- replace backup journals;
- run broad destructive cleanup;
- perform trust-domain migration without an explicit plan;
- restore without authority and postcondition verification;
- make deployment receipts a substitute for snapshots or backups;
- treat pre-upgrade evidence as a rollback guarantee;
- automatically roll back a partially applied multi-canister deployment;
- hide unresolved external lifecycle work during recovery.

---

## Implementation Slices

### Slice 1: Verification Profile Model

Define verification profiles and result evidence.

### Slice 2: Deploy Verify Command

Run profile-selected readiness, metadata, cycle, protected-call, and metrics
checks.

### Slice 3: Metrics Snapshot Evidence

Record metrics snapshots so 0.46 comparisons can consume them.

### Slice 4: Clone Plan

Create clone plans that preserve trust-domain and authority boundaries.

### Slice 5: Trust-Domain Migration Plan

Represent explicit root migration, verifier, authority, and external actions.

### Slice 6: Role-Scoped Restore Integration

Integrate snapshot references and role restore plans with deployment truth and
authority checks.

### Slice 7: Snapshot-Only DR Path

Document and implement a lighter snapshot-only DR path for deployments that do
not need full backup journal workflows.

---

## Exit Criterion

This topic is ready to promote into a real release line when:

```text
Operators can clone or verify a deployment and plan role-scoped recovery while
preserving trust-domain, authority, artifact, and verification evidence.
```

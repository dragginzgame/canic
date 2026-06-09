# Post-46 Deferred Deployment Ideas

## Status

Archived backlog notes. Remaining unfinished ideas from this file are optional
future ideas, not active release requirements.

These ideas were intentionally removed from the 0.46 release bar so the line
can stay focused on the deployment-target state hard cut and passive
two-target comparison. They are not approved numbered follow-on lines.

---

## Deployment Groups

A deployment group is a set of deployments that share an operational
relationship.

Examples:

```text
toko:
  prod
  staging
  v2
  destructive-rehearsal

tenant-a:
  prod
  staging
```

Tentative shape:

```text
DeploymentGroupV1 {
  name,
  fleet_template,
  deployments,
  promotion_lanes,
  comparison_policy,
}
```

A group does not imply a shared root. Different deployments may be different
trust domains.

Prerequisite: deployment-target local state must exist. Groups must not be
modeled as collections of fleet-named install records.

---

## Deployment Catalog

Deployments should eventually become addressable as operational objects, not
only as fleet template directory names.

Tentative shape:

```text
DeploymentCatalogEntryV1 {
  deployment_id,
  deployment_name,
  group,
  fleet_template,
  network,
  root_principal,
  trust_domain,
  last_inventory_id,
  last_receipt_id,
  labels,
  owner_hint,
}
```

The catalog must consume deployment-target state. It must not use
`.canic/<network>/fleets/*.json` as a live deployment catalog source.

---

## Promotion Readiness Across Deployments

Future comparison work may make promotion readiness visible across a group.

Example questions:

- Which staging artifacts are eligible for prod?
- Which prod roles differ from staging only by embedded config?
- Which roles changed module hash unexpectedly?
- Which user-owned canisters are pending consent?
- Which deployment has a different root trust anchor?
- Which verifier cascade is incomplete?

This should build on 0.44 promotion reports and 0.45 external lifecycle
reports. Only `DeploymentTruthInventory`-backed verification counts as
verified external completion. Supplied observations and consent evidence may
explain operator state, but they are not promotion safety proof.

---

## Lane Teardown

Teardown should be plan-driven, comparison-backed, and authority-aware.

Rules:

- start from a normalized deployment identity and comparison artifact;
- compare the target deployment identity before mutation;
- do not use production authority by default;
- show authority reconciliation requirements before teardown;
- emit receipts for every attempted cleanup action and verified postcondition;
- preserve enough evidence to explain what was removed and what was left.

Broad cleanup commands are a non-goal. A teardown plan targets a deployment
identity, not "whatever looks related" on the network.

---

## Verification Baselines

Future comparison work may compare verification artifacts if they already
exist:

```text
verification profile id
last verification result
metrics snapshot digest
readiness baseline
```

Full `canic deploy verify` remains separate future work. Verification profiles,
runtime checks, protected-call probes, cycle floor checks, and metrics snapshot
production should not be smuggled into the 0.46 comparison line.

---

## Test Deployment Lifecycle

Because the IC does not provide a public testnet, test deployments are real
deployments.

Future work may make test deployment operations safer:

- create named test deployment;
- compare to prod baseline;
- teardown with authority checks;
- rebuild from plan;
- preserve receipts for audit;
- prevent production authority from leaking into test deployment plans.

Teardown should be plan-driven and authority-aware. It should not be a broad
destructive cleanup command.

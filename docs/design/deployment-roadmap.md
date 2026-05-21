# Deployment Roadmap

## Status

Tentative cross-line roadmap for deployment work after the 0.40 protected
internal-call line.

This file is the compact contract across the 0.41 through 0.46 design docs. The
individual design docs own details.

---

## Line Shape

```text
0.41 tells truth and refuses unsafe installs.
0.42 reconciles authority.
0.43 abstracts execution.
0.44 promotes artifact identity, not authority.
0.45 coordinates lifecycle without pretending Canic has authority.
0.46 compares deployments using stable truth artifacts.
```

The order matters:

```text
observe truth
-> refuse unsafe states
-> reconcile authority
-> abstract execution
-> promote artifacts
-> coordinate external lifecycle
-> compare deployments
```

Canic should not become more flexible before it becomes more honest.

---

## Minimum Releasable Slices

These are minimum safe release bars for each line. They are not the full design
scope and they do not require every later command surface to exist before a line
can ship.

```text
0.41 minimum release:
  read-only inventory
  role artifact manifest
  safety report
  post-build materialization checks

0.42 minimum release:
  dry-run authority reconciliation
  exact external-action report

0.43 minimum release:
  current CLI backend behind executor boundary
  no new backend required

0.44 minimum release:
  digest-pinned artifact override plan
  promotion readiness report

0.45 minimum release:
  user/external control classification in plans
  external-upgrade proposal and verification shape

0.46 minimum release:
  plan/inventory/receipt comparison
  drift report over the 0.41 diff categories
```

---

## Cross-Line Invariants

### Receipts Are Not Truth

Receipts are evidence of attempted work and verified postconditions.

Live inventory wins over local state.

### Plans Do Not Execute Alone

Canic installs only after comparing:

```text
intended plan
vs
last receipt
vs
current live inventory
```

### Root Trust Anchor Defines Trust Domain

Same deployment means same root trust anchor.

Different root trust anchor means a new trust domain or an explicit migration.

### Config Digests Are Canonical

Safety decisions use canonical embedded config digests, not raw config file
bytes.

The first 0.41 implementation slice should lock canonical digest behavior with
tests for:

- reordered TOML;
- resolved includes;
- explicit defaults;
- normalized principals;
- changed root or trust-domain values.

### Promotion Never Copies Authority

Promotion may carry sealed wasm bytes or source/build identity.

It must not copy source root identity, controllers, authority profile, network,
pool canister IDs, or operation epoch into the target by default.

### Executor Backends Do Not Change Plan Meaning

`DeploymentPlanV1` means the same thing whether execution uses the current CLI
backend, PocketIC, or a future direct-agent backend.

### Unknown Control Classification Blocks Mutation

Unknown or unsafe canister control classification blocks mutation until
inventory or authority reconciliation proves a safer state.

### `wasm_store` Is A Role And A Possible Transport

`wasm_store` is modeled as a normal role artifact.

It may also be used as an artifact transport after bootstrap, but those concepts
must stay separate.

### `wasm_store` Availability Is Evidence

`wasm_store` catalog and staging state are artifact availability evidence.

Live inventory remains deployment truth.

---

## `wasm_store` Enablement Thread

0.41 through 0.46 should make `wasm_store` visible, typed, digest-checked,
receipted, transport-capable, diffable, and safe to resume against only with
live proof.

They should not turn `wasm_store` into the artifact registry, rollback
database, retention authority, source of deployment truth, or cross-deployment
promotion brain.

```text
0.41 observes wasm_store as a role artifact and artifact-transport endpoint.
0.43 routes wasm_store staging through DeploymentExecutor::stage_artifact.
0.44 permits receipt-backed and wasm_store-backed artifact locators in plans.
0.46 compares wasm_store artifact availability as evidence, not truth.
```

Post-0.46 work may promote `wasm_store` into a provenance-rich artifact
registry with pinning, rollback selection, cross-deployment cache reuse, and
deployment-aware retention.

### Automated Resume Requires Live Proof

A receipt can skip work only after live inventory proves the phase
postcondition still holds.

Automated resume is optional until the safety report model has been exercised.

---

## Design Docs

- [0.41 Deployment Truth Model](0.41-deployment-truth-model/0.41-design.md)
- [0.42 Authority Reconciliation](0.42-authority-reconciliation/0.42-design.md)
- [0.43 Backend-Agnostic Execution](0.43-backend-agnostic-execution/0.43-design.md)
- [0.44 Artifact Overrides And Promotion](0.44-artifact-promotion/0.44-design.md)
- [0.45 External And User-Owned Lifecycle](0.45-external-lifecycle/0.45-design.md)
- [0.46 Multi-Deployment Operations](0.46-multi-deployment-operations/0.46-design.md)

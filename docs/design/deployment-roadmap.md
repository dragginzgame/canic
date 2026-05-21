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

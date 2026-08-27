# V1 Operator Walkthrough

Canic distinguishes the local workspace, an App, and a Fleet. App commands own
source roles and artifacts. `canic fleet ensure` owns current live convergence.
No other command owns Fleet installation or recovery.

## Prepare The Workspace

Install the pinned tools described in `INSTALLING.md`, then inspect the current
command surface:

```bash
canic help
canic app --help
canic fleet ensure --help
```

Enroll exact network trust before using a connected environment:

```bash
sha256sum ./root-key.der
canic network enroll local \
  --root-key ./root-key.der \
  --fingerprint <64-lowercase-hex>
```

## Configure And Build An App

```bash
canic app create demo
canic scaffold canister demo app
canic app role attach demo app --component-spec demo.app
canic build demo app --profile release \
  --provenance artifacts/demo-app-provenance.json
```

App and build operations do not choose live canisters, spend cycles, or create
a Fleet operation.

## Declare Current Fleet State

Create `fleets/staging.toml` using the current `v1` schema documented in
[Fleet ensure](../features/operations/fleet-ensure.md). Every already-controlled
canister must have an exact Principal. A missing desired canister may omit its
Principal and will be created by the reviewed operation. Every artifact path is
resolved relative to the workspace and hashed before planning.

The desired document must identify:

- one exact Fleet, environment, treasury and Cycles Ledger;
- exact Ledger and management creation fees;
- material-balance, observation-burn, update-burn and stall bounds;
- each controlled canister's presence, subnet, controllers and cycle policy;
- its exact Wasm/init bytes when installation is desired;
- an exact treasury-bound drain contract before material retirement.

## Review A Plan

```bash
canic fleet ensure staging --desired fleets/staging.toml
```

Planning observes the configured live estate and writes current operator state
but executes no paid Fleet mutation. Review:

- `plan_sha256` and `operation_id`;
- every create, reuse, reinstall, replace, or delete disposition;
- observed and retained balances;
- scheduled treasury transfers;
- exact fees, maximum new funding and maximum operator debit;
- bounded observation/update burn;
- the post-operation cycle-conservation equation.

If a material canister has no exact drain authority, planning stops before any
effect and leaves it untouched.

## Apply Or Resume

```bash
canic fleet ensure staging \
  --desired fleets/staging.toml \
  --apply <plan_sha256>
```

Apply rechecks the desired bytes, artifacts, authority and bounded observation
drift before persisting the operation journal. Every effect receives an intent
record first. If a response is lost, rerun the same command; Canic observes the
live result or repeats the same idempotent Ledger/drain identity before moving
forward. It never opens a second operation over an incomplete journal.

When terminal, the report records measured conservation. Run the plan command
again immediately. A converged Fleet reports no mutation actions, and applying
that empty plan performs no creation, funding, transfer, install, controller,
start, stop, or delete effect.

## Hard-Cut Rule

Do not copy old install plans, deployment receipts, role journals, repair
receipts, recovery bundles, or installed-Fleet caches into the current state
directory. They are unsupported historical evidence and reject or remain
ignored. The operator must express any canister that still holds recoverable
cycles in the current desired document so the reviewed plan can reuse it or
drain it safely.

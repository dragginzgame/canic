# V1 Operator Walkthrough

This guide shows the compact pre-v1 Canic operator story as it exists now.
It is intentionally small: build one App role, save evidence, check that
evidence against policy, and inspect the canonical-network Fleet catalog.

The walkthrough is about command boundaries. It is not an import, promotion,
controller-mutation, teardown, signing, lock, or registry workflow.

For the command/file checklist version of this workflow, use:

```text
docs/architecture/v1-readiness-checklist.md
```

## Mental Model

Canic keeps three names separate:

- `app`: the source definition identified by `[app].name` in `canic.toml`;
- `role`: the package-backed canister role declared for that App;
- `deployment`: the deployment target recorded in local deployment-target
  state.

For example:

```text
app:        demo
role:       app
deployment: demo-staging
```

The App and role answer:

```text
What am I building?
```

The deployment target answers:

```text
What installed deployment am I checking?
```

Canic does not treat an App source identity as a deployment target. The names
may be similar in a project, but the command surfaces keep them separate.

## Setup Contract

Each package-backed canister crate declares its App and role in
`Cargo.toml`:

```toml
[package.metadata.canic]
app = "demo"
role = "app"
```

The App config declares package-backed roles:

```toml
[roles.app]
kind = "canister"
package = "canisters/app"
```

Declared roles can compile as source work. Attached roles are the roles that
can become build artifacts, deployment truth, install targets, and local
deployment plans.

The visible artifact build remains attached-role strict:

```text
canic build <app> <role>
```

## Build With Provenance

Build the selected attached role and save stable build provenance:

```text
canic build demo app --provenance artifacts/canic/app-build-provenance.json
```

This is an active artifact build. The extra provenance file is explicit. It
records source, Cargo, package identity, and produced artifact hashes in a
stable `canic.build_provenance.v1` payload wrapped by `EvidenceEnvelopeV1`.

It does not install the artifact, register it in `wasm_store`, change
controllers, attach topology, or update deployment truth.

## Plan Desired Deployment Shape

Before checking live deployment evidence, inspect the desired deployment shape
that Canic can derive from local config:

```text
canic deploy plan demo-staging
canic deploy plan demo-staging --json
canic deploy plan demo-staging --out artifacts/canic/deployment-plan.json
```

`canic deploy plan` emits a `DeploymentPlanReport` with `schema_version = 1`
and embeds the existing `DeploymentPlanV1` desired-state model. It separates
verified config facts, unresolved assumptions, blockers, warnings, future apply
preview labels, and next actions.

The command is diagnostic and planning-only. It does not install Wasm, create
canisters, change controllers, query live mainnet by default, write deployment
truth, create installed deployment records, sign evidence, or authorize apply.
`--out` writes JSON only and fails if the target file already exists or the
parent directory does not exist.

## Check Deployment Evidence

When a deployment target exists, run a passive deployment check and save a
stable evidence envelope:

```text
canic deploy check demo-staging \
  --evidence-envelope \
  --build-provenance artifacts/canic/app-build-provenance.json \
  > artifacts/canic/deployment-check-envelope.json
```

`canic deploy check` is a report surface. The envelope records command
provenance, target identity, supplied input fingerprints, warnings, blocked
actions, missing/stale evidence, evidence conflicts, and exit class.

It does not install Wasm, mutate controllers, create topology attachment,
register artifacts, or make stale evidence fresh.

## Gate Saved Evidence

Evaluate saved evidence against a strict project policy:

```text
canic evidence gate \
  --policy ci/canic-policy.toml \
  --manifest ci/canic-evidence.toml \
  --json \
  --output artifacts/canic/policy-gate-report.json
```

The policy gate reads existing evidence files. It does not run builds, query
live deployments, mutate evidence, edit config, update topology, change
controllers, register artifacts, or turn policy success into deployment truth.

A minimal project evidence manifest points at saved envelopes:

```toml
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "build_provenance"
path = "artifacts/canic/app-build-provenance.json"
required = true
payload_schema = "canic.build_provenance.v1"

[evidence.target]
app = "demo"
role = "app"

[[evidence]]
kind = "deployment_check"
path = "artifacts/canic/deployment-check-envelope.json"
required = true
payload_schema = "canic.deployment_check.v1"

[evidence.target]
deployment = "demo-staging"
```

## Inspect Known Fleets

List Fleets recorded in the selected canonical network catalog:

```text
canic deploy inspect catalog list
canic deploy inspect catalog list --json
```

Inspect one known Fleet:

```text
canic deploy inspect catalog inspect demo-staging
canic deploy inspect catalog inspect demo-staging --json
```

The selected environment profile resolves the catalog at:

```text
.canic/networks/<canonical-network-id>/fleets/catalog.json
```

It does not refresh live state, infer Fleets from App names, create
deployment truth, install Wasm, mutate topology, change controllers, register
artifacts, acquire locks, sign evidence, add groups, or scan saved evidence
files.

An empty catalog is valid when no Fleet has committed host authority on that
network. The command never falls back to removed environment-scoped deployment
state.

## Useful Local Smoke Checks

From a fleet directory, these checks should be safe because they do not query a
live deployment or mutate project state:

```text
canic deploy inspect catalog list
canic deploy inspect catalog list --json
canic deploy inspect catalog list --json --output /tmp/canic-catalog-smoke.json
```

Expected behavior in a fresh checkout without a Fleet catalog:

- the catalog has zero entries;
- the canonical network identity and selected environment are still reported;
- removed environment-scoped deployment state, if present, is ignored;
- `inspect <fleet>` fails clearly until that Fleet is known.

The maintained temporary-project smoke path is:

```text
scripts/ci/v1-readiness-smoke.sh
```

See:

```text
docs/operations/0.55-v1-local-smoke.md
```

The heavier local operator proof for build provenance plus deployment-check
envelope output is:

```text
scripts/ci/v1-operator-proof.sh
```

See:

```text
docs/operations/0.55-v1-operator-proof.md
```

That proof is still not a live install. It registers an explicit unverified
local deployment target in a temporary proof root and expects the
deployment-check envelope to be blocked while still proving the saved evidence
chain.

## What This Does Not Cover

This walkthrough deliberately avoids:

- deployment groups;
- promotion lanes;
- saved evidence catalogs;
- signing;
- locks;
- registry import;
- `wasm_store` retention or garbage collection;
- active adoption/import;
- controller mutation;
- topology mutation;
- install or upgrade authority;
- teardown;
- broad live verification.

Those are post-v1 concerns unless a later design adds a smaller concrete user
journey.

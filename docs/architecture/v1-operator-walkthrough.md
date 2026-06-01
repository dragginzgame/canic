# V1 Operator Walkthrough

This guide shows the compact pre-v1 Canic operator story as it exists now.
It is intentionally small: build one fleet role, save evidence, check that
evidence against policy, and inspect the local deployment catalog.

The walkthrough is about command boundaries. It is not an import, promotion,
controller-mutation, teardown, signing, lock, or registry workflow.

For the command/file checklist version of this workflow, use:

```text
docs/architecture/v1-readiness-checklist.md
```

## Mental Model

Canic keeps three names separate:

- `fleet`: the fleet template in `canic.toml`;
- `role`: the package-backed canister role declared for that fleet;
- `deployment`: the deployment target recorded in local deployment-target
  state.

For example:

```text
fleet:      demo
role:       app
deployment: demo-staging
```

The fleet and role answer:

```text
What am I building?
```

The deployment target answers:

```text
What installed deployment am I checking?
```

Canic does not treat a fleet template name as a deployment target. The names
may be similar in a project, but the command surfaces keep them separate.

## Setup Contract

Each package-backed canister crate declares its fleet and role in
`Cargo.toml`:

```toml
[package.metadata.canic]
fleet = "demo"
role = "app"
```

The fleet config declares package-backed roles:

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
canic build <fleet> <role>
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

## Check Deployment Evidence

When a deployment target exists, run a passive deployment check and save a
stable evidence envelope:

```text
canic deploy check demo-staging \
  --format envelope-json \
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
  --format json \
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
fleet = "demo"
role = "app"

[[evidence]]
kind = "deployment_check"
path = "artifacts/canic/deployment-check-envelope.json"
required = true
payload_schema = "canic.deployment_check.v1"

[evidence.target]
deployment = "demo-staging"
```

## Inspect Known Deployments

List deployment targets recorded in local deployment-target state:

```text
canic deploy catalog list
canic deploy catalog list --format json
```

Inspect one known deployment target:

```text
canic deploy catalog inspect demo-staging
canic deploy catalog inspect demo-staging --format json
```

The catalog reads only:

```text
.canic/<network>/deployments/<deployment>.json
```

It does not refresh live state, infer deployments from fleet names, create
deployment truth, install Wasm, mutate topology, change controllers, register
artifacts, acquire locks, sign evidence, add groups, or scan saved evidence
files.

An empty catalog is valid when no deployment-target state exists. In that case
the command reports warnings instead of inventing deployments from nearby fleet
or legacy install state.

## Useful Local Smoke Checks

From a fleet directory, these checks should be safe because they do not query a
live deployment or mutate project state:

```text
canic deploy catalog list
canic deploy catalog list --format json
canic deploy catalog list --format json --output /tmp/canic-catalog-smoke.json
```

Expected behavior in a fresh checkout without deployment-target state:

- the catalog has zero entries;
- warnings explain that no deployment-target state exists;
- legacy fleet-named state, if present, is ignored;
- `inspect <deployment>` fails clearly until that deployment target is known.

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

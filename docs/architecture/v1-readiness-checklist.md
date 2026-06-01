# V1 Readiness Checklist

This checklist is the maintained v1-candidate operator surface. It is a
readiness checklist, not a new workflow engine. Each command keeps one
boundary explicit so operators can see what Canic is doing and what it is not
doing.

Use the walkthrough for more explanation:

```text
docs/architecture/v1-operator-walkthrough.md
```

## Names To Keep Separate

- `fleet`: the fleet template in `canic.toml`;
- `role`: the package-backed canister role declared for that fleet;
- `deployment`: the deployment target recorded in local deployment-target
  state.

Do not treat a fleet name as a deployment target unless the project has
explicitly chosen the same string for both concepts.

## Required Project Files

A small managed fleet should have:

```text
fleets/<fleet>/canic.toml
icp.yaml
Cargo.toml
<canister-crate>/Cargo.toml
<canister-crate>/build.rs
<canister-crate>/src/lib.rs
```

Each canister package must declare both fields:

```toml
[package.metadata.canic]
fleet = "<fleet>"
role = "<role>"
```

Each package-backed role must be declared in `canic.toml`:

```toml
[roles.<role>]
kind = "canister"
package = "<path>"
```

Only attached roles can be built as deployment artifacts:

```toml
[subnets.<subnet>.canisters.<role>]
kind = "singleton"
```

## Command Checklist

Create the fleet config:

```text
canic fleet create <fleet>
```

Scaffold an ordinary package-backed role:

```text
canic scaffold canister <fleet> <role>
```

Attach the role when placement is known:

```text
canic fleet role attach <fleet> <role> --subnet <subnet>
```

Build an attached role and write stable build provenance:

```text
canic build <fleet> <role> --provenance <path>
```

Check a deployment target and save stable evidence:

```text
canic deploy check <deployment> --format envelope-json
```

Evaluate saved evidence against a project policy:

```text
canic evidence gate --policy <path> --manifest <path>
```

Inspect deployment-target local state:

```text
canic deploy catalog list
canic deploy catalog inspect <deployment>
```

## Expected Outputs

The v1 surface should produce or read these evidence artifacts:

- `EvidenceEnvelopeV1` for stable automation output;
- `canic.build_provenance.v1` for build provenance payloads;
- `canic.deployment_check.v1` for deployment-check payloads;
- `PolicyGateReportV1` or `ProjectEvidenceGateReportV1` for policy results;
- `DeploymentCatalogReportV1` for local deployment-target catalog output.

Raw command payloads may be command-specific. CI should prefer stable envelope
fields and payload schemas that are explicitly marked stable.

## Readiness Boundary

The checklist does not add authority. In particular, it does not:

- install or upgrade Wasm;
- mutate controllers;
- attach topology except through the explicit `fleet role attach` command;
- import brownfield deployments;
- register artifacts in `wasm_store`;
- sign evidence;
- acquire deployment locks;
- create deployment groups;
- perform teardown;
- make catalog entries live or fresh;
- turn policy success into deployment truth.

## Local Smoke Expectations

The maintained local smoke is:

```text
scripts/ci/v1-readiness-smoke.sh
```

It runs in a temporary project and proves the safe local subset of this
checklist: fleet creation, canister scaffold, declared-only inspection,
explicit role attachment, attached inspection, empty local deployment catalog,
and policy evaluation of one saved envelope.

Runbook:

```text
docs/operations/0.55-v1-local-smoke.md
```

The heavier local operator proof is:

```text
scripts/ci/v1-operator-proof.sh
```

Runbook:

```text
docs/operations/0.55-v1-operator-proof.md
```

It builds `demo.app`, writes stable build provenance, registers an explicit
local deployment target under a temporary proof root, and emits a
deployment-check envelope that fingerprints the build provenance. The
deployment check is expected to be blocked because the proof does not install
or live-verify the deployment.

In a fresh checkout without deployment-target state:

```text
canic deploy catalog list
canic deploy catalog list --format json
```

should succeed with zero catalog entries and warnings explaining that no
deployment-target state exists. This is expected. Catalog commands must not
invent deployments from fleet names or legacy install state.

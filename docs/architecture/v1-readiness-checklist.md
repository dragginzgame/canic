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

- `app`: the source definition identified by `[app].name` in `canic.toml`;
- `role`: the package-backed canister role declared for that App;
- `tree spec`: one permitted rooted canister topology declared by the App;
- `fleet`: one live installed App identified within a canonical network.

An App source identity is not a live Fleet identity.

## Required Workspace Files

A small managed App should have:

```text
apps/<app>/canic.toml
icp.yaml
Cargo.toml
<canister-crate>/Cargo.toml
<canister-crate>/build.rs
<canister-crate>/src/lib.rs
```

Each canister package must declare both fields:

```toml
[package.metadata.canic]
app = "<app>"
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
[component_specs.<component-spec>]
component_role = "<role>"
maximum_instances = 1

[component_specs.<component-spec>.children.<child-role>]
kind = "singleton"

[component_specs.<component-spec>.spawn_grants.<component-role>.<child-role>]
maximum_instances_per_parent = 1
```

## Command Checklist

Create the App config:

```text
canic app create <app>
```

Scaffold an ordinary package-backed role:

```text
canic scaffold canister <app> <role>
```

Attach the role when placement is known:

```text
canic app role attach <app> <role> --component-spec <component-spec>
```

The first role attached to a new Component Spec becomes its Component. A later
role attached to that Spec becomes a direct child; use `--kind` to select
`singleton`, `replica`, `shard`, or `instance`.

Build an attached role and write stable build provenance:

```text
canic build <app> <role> --provenance <path>
```

Inspect the desired deployment shape without mutation:

```text
canic deploy plan <fleet> --app <app>
canic deploy plan <fleet> --app <app> --json
canic deploy plan <fleet> --app <app> --out <path>
```

`canic deploy plan` emits a no-mutation `DeploymentPlanReport` with
`schema_version = 1`. It is not an evidence envelope and does not create
deployment truth. `--out` writes JSON only and does not create parent
directories.

Check a Fleet and save stable deployment evidence:

```text
canic deploy check <fleet> --evidence-envelope
```

Evaluate saved evidence against a project policy:

```text
canic evidence gate --policy <path> --manifest <path>
```

Inspect the Fleet catalog for the selected canonical network:

```text
canic deploy inspect catalog list
canic deploy inspect catalog inspect <fleet>
```

## Expected Outputs

The v1 surface should produce or read these evidence artifacts:

- `EvidenceEnvelopeV1` for stable automation output;
- `DeploymentPlanReport` for the 0.79 no-mutation deploy-plan output;
- `canic.build_provenance.v1` for build provenance payloads;
- `canic.deployment_check.v1` for deployment-check payloads;
- `PolicyGateReportV1` or `ProjectEvidenceGateReportV1` for policy results;
- `FleetCatalogReportV1` for canonical-network Fleet catalog output.

Raw command payloads may be command-specific. CI should prefer stable envelope
fields and payload schemas that are explicitly marked stable.

## Readiness Boundary

The checklist does not add authority. In particular, it does not:

- install or upgrade Wasm;
- mutate controllers;
- attach topology except through the explicit `app role attach` command;
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

It runs in a temporary workspace and proves the safe local subset of this
checklist: App creation, canister scaffold, declared-only inspection,
explicit role attachment, attached inspection, empty network-scoped Fleet catalog,
and policy evaluation of one saved envelope.

Runbook:

```text
docs/operations/0.55-v1-local-smoke.md
```

In a fresh checkout without a Fleet catalog:

```text
canic deploy inspect catalog list
canic deploy inspect catalog list --json
```

should succeed with zero catalog entries. This is expected. Catalog commands
must not invent Fleets from App names or read removed environment-scoped
deployment state.

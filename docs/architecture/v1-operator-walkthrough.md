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
- `fleet`: one live installed App identified within a canonical network.

For example:

```text
app:        demo
role:       app
fleet:      demo-staging
```

The App and role answer:

```text
What am I building?
```

The Fleet answers:

```text
What live installation am I checking?
```

Canic does not treat an App source identity as a Fleet identity. The names may
be similar in a project, but the command surfaces keep them separate.

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

Build the configured Fleet Subnet Root, every attached Component role, and the
canonical Fleet Coordinator and Wasm Store:

```text
canic build <app>
```

The command reports Component artifacts under Application Wasm and all three
platform canisters under Infrastructure Wasm. Select one deployable configured
role when a focused build or role-scoped provenance is needed:

```text
canic build <app> <role>
```

## Build With Provenance

Build the selected deployable role and save stable build provenance:

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
that Canic can derive from local config and the operator-owned Fleet input:

```text
canic deploy plan demo-staging --app demo --fleet-input deployments/demo-staging.toml
canic --environment ic deploy plan demo --app demo --fleet-input deployments/demo-ic.toml --refresh-catalog
canic deploy plan demo-staging --app demo --fleet-input deployments/demo-staging.toml --out artifacts/canic/deployment-plan.json
```

`canic deploy plan` emits a `DeploymentPlanReport` with `schema_version = 1`
and embeds the existing `DeploymentPlanV1` desired-state model. It separates
verified config facts, unresolved assumptions, blockers, warnings, future apply
preview labels, and next actions.

The command is diagnostic and planning-only. It does not install Wasm, create
canisters, change controllers, query live mainnet by default, write deployment
truth, create Fleet-catalog rows, sign evidence, or authorize apply. On an IC
target, `--refresh-catalog` may issue public NNS Registry query calls and update
only Canic's private `.canic/ic-query` cache when it is missing or invalid; it
does not perform an IC update call. Without that flag, a missing cache remains
a typed planning blocker with a direct `--refresh-catalog` remedy. The plan is
compiled from the validated snapshot authority, so a later install can
reproduce its digest without treating the cache path, collection time,
disposition or refresh request as decision input. The report still renders
that transient acquisition provenance separately.
`--out` writes JSON only and fails if the target file already exists or the
parent directory does not exist.

If an exact fresh-Fleet install session already contains effects, planning
switches to read-only recovery inspection. It validates the retained session,
original plan, release build and creation journals and emits an
`install_recovery` section with the original maximum debit, remaining debit,
fenced and uncertain creation outcomes, and the next replay phase. It does not
advance the session or acquire its install lock. Review that report before a
separately authorized resume; do not start a replacement fresh install. Once
paid effects exist, the report suppresses generic fresh-install proposal labels
and uses the retained next replay phase as its continuation guidance.

Before the first remote effect, Canic checkpoints the exact session, plans,
journals, receipts and finalized content-addressed artifacts under its stable
operator-state recovery bundle. Each governed phase and each repair
write-before-effect transition refreshes that bundle. Verification derives
the exact required sidecars and repair artifacts from every retained Root's
typed journal phase.
Verify a retained copy without local or remote mutation:

```text
canic deploy recovery verify <bundle-path>
```

If the original checkout was lost, import only missing exact files into a new
ICP root with `canic deploy recovery import <bundle-path> --into <icp-root>`.
Import rejects an existing conflicting file and performs no ICP call. Run the
ordinary read-only `canic deploy plan` after import; never reconstruct journal
fields manually or start a replacement Fleet.

## Check Deployment Evidence

When a Fleet exists, run a passive deployment check and save a stable evidence
envelope:

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
network.

## Useful Local Smoke Checks

From an App workspace, these checks should be safe because they do not query a
live Fleet or mutate project state:

```text
canic deploy inspect catalog list
canic deploy inspect catalog list --json
canic deploy inspect catalog list --json --output /tmp/canic-catalog-smoke.json
```

Expected behavior in a fresh checkout without a Fleet catalog:

- the catalog has zero entries;
- the canonical network identity and selected environment are still reported;
- `inspect <fleet>` fails clearly until that Fleet is known.

The maintained temporary-project smoke path is:

```text
scripts/ci/v1-readiness-smoke.sh
```

See:

```text
docs/operations/0.55-v1-local-smoke.md
```

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

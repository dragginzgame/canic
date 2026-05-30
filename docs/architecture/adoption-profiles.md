# Adoption Profiles

Canic adoption reporting is a passive onboarding tool. It helps an operator
compare existing Canic config, deployment evidence, package metadata, and
artifact evidence without making Canic own or mutate anything.

The command is:

```bash
canic fleet adoption report <fleet> --profile <profile>
```

The report is fleet-template scoped. It does not operate on deployment target
identity, install Wasm, update controllers, attach topology, import pools, or
edit manifests.

## Profiles

`brownfield`
: Existing deployment first. The report expects that some observed resources may
  be outside Canic management and should stay external until an operator makes
  explicit changes.

`partial`
: Mixed management. Some roles may already be Canic-managed while other
  observed canisters are candidates, external, or intentionally ignored.

`standalone`
: A small compile-only or single-canister shape. Declared-only roles remain
  valid report inputs without synthetic topology attachment.

`leaf-only`
: Authority-sensitive onboarding. Roles that imply controller or root-like
  authority stay visible, but the report avoids recommending declaration for
  those authority surfaces.

`hybrid-external-wasm`
: Existing external Wasm artifacts are reported as evidence. The report may show
  module hashes and external artifact paths, but artifact registry import stays
  outside adoption reporting.

`minimal`
: The smallest report shape. Use this when an operator only wants the current
  Canic configuration and missing-evidence warnings.

## Lifecycle Vocabulary

Adoption reporting uses the 0.49 role lifecycle directly:

- `declared-only`: a role exists under `[roles.<role>]` but is not attached to
  topology;
- `attached`: a role is declared and referenced by topology;
- `observed-only`: evidence mentions a canister that is not currently declared
  as a Canic role;
- `attached-unobserved`: config says a role is attached, but supplied evidence
  does not confirm a deployed canister;
- `evidence-conflict`: supplied evidence contradicts package metadata, role
  identity, authority, topology, or artifact expectations.

Declared-only roles are inspectable and may compile, but they cannot become
deploy artifacts, install targets, deployment-truth entries, local deployment
plans, inventories, or local artifact manifest roles until explicit topology
attachment exists.

## Report Boundary

Adoption report generation is read-only.

By default it writes only to stdout:

```bash
canic fleet adoption report demo --profile brownfield
```

An explicit output path writes only that report artifact:

```bash
canic fleet adoption report demo --profile partial --output adoption-report.txt
```

JSON output is available for inspection, but the 0.50 JSON shape is
experimental:

```bash
canic fleet adoption report demo --profile minimal --format json
```

Evidence can be supplied from existing JSON artifacts:

```bash
canic fleet adoption report demo --profile partial \
  --deployment-check check.json \
  --package-metadata package-metadata.json
```

or with a standalone inventory artifact:

```bash
canic fleet adoption report demo --profile partial \
  --inventory inventory.json \
  --artifact-manifest artifact-manifest.json \
  --package-metadata package-metadata.json
```

Those inputs are read-only:

- `--deployment-check` reads inventory evidence from a `DeploymentCheckV1` JSON
  artifact and, when present, reads plan role artifacts as artifact evidence;
- `--inventory` reads `DeploymentInventoryV1` JSON evidence;
- `--artifact-manifest` reads `RoleArtifactManifestV1` JSON evidence;
- `--package-metadata` reads a JSON array of `AdoptionPackageMetadataV1`
  entries.

Use either `--deployment-check` or `--inventory`, not both.

The report command must not:

- edit `canic.toml`;
- edit package manifests;
- attach topology;
- change controllers;
- install or upgrade canisters;
- import pools;
- write artifact manifests as if roles were deployable;
- rewrite inventories, deployment plans, or deployment-truth records.

## Recommendations

Recommendations are descriptive. They are not an execution plan.

Text output renders suggested commands as previews:

```text
Recommendations (report-only; not executed):
  - declare observed role candidate [warning; mutates-state; unsupported-by-adoption; blocked-in-0.50.0; requires-explicit-operator-action]
    suggested_action_preview: canic fleet role declare demo store --package canisters/store
    status: not executed by adoption report
    support: unsupported-by-adoption
    availability: blocked-in-0.50.0
```

Blocked actions are also report-only:

```text
Blocked adoption actions (not executed by report):
  - controller changes
  - topology attachment
  - pool import
  - artifact registry import
  - install
  - upgrade
```

To perform one of those actions, an operator must run the explicit Canic command
for that action outside adoption reporting. For example, declaring a role and
attaching topology are separate role-lifecycle commands:

```bash
canic fleet role declare demo store --package canisters/store
canic fleet role attach demo store --subnet application
```

## Evidence Rules

Name similarity alone is not enough for Canic to treat an observed canister as
managed. It may produce a candidate finding, but management requires explicit
configuration, topology, authority, package metadata, deployment-truth, or
artifact evidence.

Authority is a separate report dimension. A role can be configured as managed
while still being non-operable if supplied evidence shows controller drift or an
external controller requirement.

External Wasm evidence is informational. Module hashes, artifact paths, payload
hashes, and payload sizes can appear in role findings, but importing those
artifacts into `wasm_store` is outside the adoption report surface.

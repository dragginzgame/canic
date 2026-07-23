# Adoption Profiles

Canic adoption reporting is a passive onboarding tool. It helps an operator
compare existing Canic config, deployment evidence, package metadata, and
artifact evidence without making Canic own or mutate anything.

The command is:

```bash
canic app adoption report <app> --profile <profile>
```

The report is App-scoped. It does not operate on live Fleet or deployment target
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

Adoption reporting uses the maintained role lifecycle directly:

- `declared-only`: a role exists under `[roles.<role>]` with an explicit
  `package = "<path>"`, but is not attached to topology;
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
canic app adoption report demo --profile brownfield
```

An explicit output path writes only that report artifact:

```bash
canic app adoption report demo --profile partial --output adoption-report.txt
```

JSON output is available for inspection, but the adoption-report schema is
experimental:

```bash
canic app adoption report demo --profile minimal --json
```

Evidence can be supplied from existing JSON artifacts:

```bash
canic app adoption report demo --profile partial \
  --deployment-check check.json \
  --cargo-metadata cargo-metadata.json
```

or with a standalone inventory artifact:

```bash
canic app adoption report demo --profile partial \
  --inventory inventory.json \
  --artifact-manifest artifact-manifest.json \
  --package-metadata package-metadata.json
```

`--cargo-metadata` expects a pre-existing Cargo metadata artifact, for example:

```bash
cargo metadata --format-version 1 --no-deps > cargo-metadata.json
```

Those inputs are read-only:

- `--deployment-check` reads inventory evidence from a `DeploymentCheckV1` JSON
  artifact and, when present, reads plan role artifacts as artifact evidence;
- `--inventory` reads `DeploymentInventoryV1` JSON evidence;
- `--artifact-manifest` reads `RoleArtifactManifestV1` JSON evidence;
- `--package-metadata` reads a JSON array of `AdoptionPackageMetadataV1`
  entries;
- `--cargo-metadata` reads `[package.metadata.canic]` App/role evidence from
  a saved `cargo metadata --format-version 1` JSON artifact and normalizes
  Cargo package paths against the selected App config.

Use either `--deployment-check` or `--inventory`, not both.
Use either `--package-metadata` or `--cargo-metadata`, not both.
Unresolved observations and unresolved artifact entries already present in
supplied evidence are carried into `missing_or_stale_evidence`.

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
  - declare observed role candidate [warning; mutates-state; unsupported-by-adoption; requires-explicit-operator-action]
    suggested_action_preview: canic app role declare demo store --package canisters/store
    status: not executed by adoption report
    support: unsupported-by-adoption
```

`suggested_action_support` is the sole authority for whether the passive
adoption surface supports a recommendation. The report does not project a
release-specific availability field. Recommendation objects reject unknown
JSON fields rather than silently accepting stale or misspelled decisions.

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
canic app role declare demo store --package canisters/store
canic app role attach demo store --subnet application
```

Declaration recommendations are authority-gated. If an observed-only role is
already Canic-authorized, the report may recommend a future
`canic app role declare ...` command as a blocked, non-executed preview. If
the observed candidate is user-controlled, externally controlled, or unknown,
the report recommends authority review first and does not preview role
declaration.

## Evidence Rules

Observed canister text output includes the match confidence and supplied
evidence details when present:

```text
Observed canisters:
  - aaaaa-aa: role=store, confidence=candidate, classifications=observed-only
    deployment_target_evidence: inventory-1
```

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

If supplied artifact manifest evidence and inventory artifact evidence disagree
about whether the same role is Canic-built or externally supplied, the role is
reported as an `evidence-conflict`. Adoption reporting does not choose one
artifact source or try to reconcile it.

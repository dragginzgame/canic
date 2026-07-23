# CI Policy Gates

This document describes Canic's passive CI policy gate surface.

## Purpose

Canic evidence commands produce stable envelopes and provenance records. A CI
job still needs a project-specific answer to:

```text
Does this saved evidence satisfy the policy for this project?
```

Policy gates answer that question without running builds, querying live
deployments, installing Wasm, mutating topology, changing controllers,
registering artifacts, or turning policy success into deployment truth.

Current commands:

```text
canic evidence gate --policy <path> --envelope <path>
canic evidence gate --policy <path> --manifest <path>
```

The first form evaluates one `EvidenceEnvelopeV1`. The second evaluates a
project evidence manifest that points at existing envelope files.

## Policy Files

Policy files are strict TOML. Unknown keys fail instead of being ignored.

Minimal policy:

```toml
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
```

Typical conservative policy for stable build provenance:

```toml
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"
allowed_payload_schemas = ["canic.build_provenance.v1"]
allowed_payload_stability = ["stable"]

[exit_class]
allowed = ["success"]

[summary]
fail_on_evidence_conflicts = true
fail_on_blocked_actions = true
allow_missing_or_stale_evidence = false

[build_provenance]
require_clean_source = true
require_cargo_lock = true
require_wasm_gzip = true
require_sha256 = true
require_package_identity_matches_target = true
```

Build-provenance rules apply only to stable `canic.build_provenance.v1`
payloads. They do not run `canic build`; they inspect an existing saved
envelope.

## Project Evidence Manifests

A project evidence manifest groups existing evidence files and records what
each file is expected to represent.

Example:

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

Manifest paths resolve from the manifest's project root. Entries are evidence
expectations, not deployment plans.

Manifest behavior:

- missing required evidence fails with `missing_required_evidence`;
- missing optional evidence reports `success_with_warnings`;
- payload schema mismatches fail with `blocked_by_policy`;
- target identity mismatches fail with `blocked_by_policy`;
- every listed file is evaluated by the same policy gate logic as
  `--envelope`.

## Minimal CI Flow

One practical flow is:

```text
mkdir -p artifacts/canic

canic build demo app \
  --provenance artifacts/canic/app-build-provenance.json

canic deploy check demo-staging \
  --evidence-envelope \
  --build-provenance artifacts/canic/app-build-provenance.json \
  > artifacts/canic/deployment-check-envelope.json

canic evidence gate \
  --policy ci/canic-policy.toml \
  --manifest ci/canic-evidence.toml \
  --json \
  --output artifacts/canic/policy-gate-report.json
```

The first command is the normal artifact build. The deployment check and policy
gate steps are passive evidence/reporting steps.

## Output

Raw policy output is:

```text
PolicyGateReportV1
ProjectEvidenceGateReportV1
```

Envelope output is also available:

```text
canic evidence gate --policy ci/canic-policy.toml \
  --manifest ci/canic-evidence.toml \
  --evidence-envelope \
  --output artifacts/canic/policy-gate-envelope.json
```

For single-envelope gates, `--evidence-envelope` wraps
`PolicyGateReportV1`. For manifest gates, it wraps
`ProjectEvidenceGateReportV1`.

## Exit Classes

Policy gates use `ExitClassV1`:

```text
success
success_with_warnings
blocked_by_policy
evidence_conflict
missing_required_evidence
invalid_input
execution_failed
internal_error
```

Recommended CI behavior:

- pass on `success`;
- treat `success_with_warnings` according to project policy;
- fail on `blocked_by_policy`;
- fail on `evidence_conflict`;
- fail on `missing_required_evidence`;
- fail on `invalid_input`, `execution_failed`, and `internal_error`.

## What Policy Gates Do Not Prove

Policy success means the saved evidence matched the policy at evaluation time.
It does not prove:

- the network state is still fresh;
- the artifact has been installed;
- the artifact is registered in `wasm_store`;
- controller authority has changed;
- topology has been attached or modified;
- adoption candidates have been imported;
- deployment truth has been updated.

Policy gates are deliberately passive. Active deployment, installation,
registry, signing, lock, and controller workflows remain separate work.

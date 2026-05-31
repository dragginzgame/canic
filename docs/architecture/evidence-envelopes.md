# Evidence Envelopes

This document is the current architecture note for Canic's stable evidence
envelope output.

## Purpose

Canic emits many command-specific evidence payloads: deployment checks,
adoption reports, inventories, artifact manifests, promotion reports, and
authority reports. These payloads are useful, but they are not all stable public
automation contracts.

Evidence envelopes provide a stable outer JSON contract for CI, GitOps, and
audit systems:

```text
stable envelope, command-specific payload
```

The envelope records how evidence was produced without freezing every nested
payload DTO.

## Current Commands

The current envelope emitters are:

```text
canic fleet adoption report <fleet> --profile <profile> --format envelope-json
canic deploy check <deployment> --format envelope-json
```

Existing raw JSON output remains available:

```text
canic fleet adoption report <fleet> --profile <profile> --format json
canic deploy check <deployment>
canic deploy check <deployment> --format json
```

Raw adoption report JSON remains experimental. Raw deployment-check JSON is
still the command-specific `DeploymentCheckV1` payload. Automation that wants a
stable outer contract should use `--format envelope-json`.

## Envelope Contract

`EvidenceEnvelopeV1` includes:

- envelope schema identity;
- Canic version;
- normalized command provenance;
- target identity such as deployment, fleet, profile, network, or role;
- generation timestamp;
- source config fingerprint when available;
- supplied input fingerprints;
- nested payload schema identity;
- nested payload SHA-256;
- nested payload JSON;
- structured warnings, blocked actions, missing/stale evidence, and evidence
  conflicts;
- `ExitClassV1`.

The stable contract is the envelope. A nested payload is stable only when its
payload schema explicitly says so.

## Payload Stability

Current payload schema stability:

```text
canic.evidence_envelope.v1 = stable
canic.adoption_report.v1   = experimental
canic.deployment_check.v1  = internal
```

Do not write CI policy against nested payload fields unless that payload schema
is marked stable. Prefer envelope fields such as `exit_class`, `target`,
`payload_schema`, and structured summary message codes.

## Exit Classes

`ExitClassV1` is the automation-facing result classification:

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

`success_with_warnings` is process-success by default. CI policy may choose to
fail on warnings, but the command does not treat every warning as a shell
failure.

## Safety Boundary

Envelope output does not make passive commands active.

Envelope generation must not:

- install or upgrade Wasm;
- attach topology;
- import adoption candidates;
- change controllers;
- register artifacts;
- delete or garbage-collect artifacts;
- teardown resources;
- perform extra live discovery beyond the underlying command;
- make stale evidence fresh;
- turn the envelope itself into deployment truth.

The envelope describes command provenance and report summary. It is not a
substitute for live checks, deployment truth validation, or operator review.

## Input Fingerprints

File input fingerprints use SHA-256 over file bytes where possible. Paths should
be project/config-relative or otherwise safe for CI artifacts. The
`path_display` field records how the path was handled:

```text
relative          path is relative to the selected project/config root
absolute_redacted path was outside that root and intentionally omitted
omitted           path was unavailable or intentionally not included
```

Timestamps are explanatory metadata only; they are not provenance by
themselves.

When an evidence path is redacted from normalized command provenance,
`argv_redactions` records which argument was affected.

## CI/GitOps Guidance

Recommended automation behavior:

- fail on `evidence_conflict`;
- fail on `blocked_by_policy`;
- warn or fail by project policy on `success_with_warnings`;
- archive the full envelope as the CI artifact;
- compare payload hashes or envelope summary fields rather than raw internal
  DTO layouts;
- treat `missing_or_stale_evidence` as an operator-review signal unless the
  command classifies it as `missing_required_evidence`.

## Deferred Work

Not part of the current envelope surface:

- signing;
- external key management;
- attestations;
- artifact registry import;
- active adoption/import;
- controller mutation;
- topology mutation;
- teardown;
- broad live deployment verification;
- GitHub Actions-specific integration;
- stable schemas for every nested payload DTO.

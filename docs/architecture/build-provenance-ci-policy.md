# Build Provenance CI Policy

This document describes how CI/GitOps jobs should consume Canic build
provenance. It is policy guidance, not a new command contract.

## Purpose

`canic build <fleet> <role> --provenance <path>` writes an
`EvidenceEnvelopeV1` whose payload schema is stable:

```text
canic.build_provenance.v1
```

The payload explains which source state, Cargo inputs, package metadata, build
profile, toolchain, and produced artifacts led to the build output. CI should
use that evidence to decide whether an artifact is acceptable for a project
policy. Canic does not force one global policy for dirty worktrees, warning
handling, or artifact promotion.

## Minimal Pipeline

A conservative build-evidence pipeline writes provenance at build time and then
passes that saved evidence into passive report envelopes:

```text
canic build demo app --provenance artifacts/canic/build-provenance.json

canic fleet adoption report demo --profile minimal --evidence-envelope \
  --build-provenance artifacts/canic/build-provenance.json \
  --output artifacts/canic/adoption-envelope.json

canic deploy check demo-staging --evidence-envelope \
  --build-provenance artifacts/canic/build-provenance.json \
  > artifacts/canic/deployment-check-envelope.json
```

Only the build command creates build artifacts. The adoption and deployment
check commands only fingerprint the saved provenance file as supplied evidence.

## Stable Fields

CI policy should branch on the stable envelope plus the stable
`BuildProvenanceV1` payload:

- `envelope_schema.id == "canic.evidence_envelope.v1"`;
- `payload_schema.id == "canic.build_provenance.v1"`;
- `payload_schema.stability == "stable"`;
- `target.kind == "artifact"`;
- `target.fleet`, `target.role`, `target.profile`, and `target.network`;
- `exit_class`;
- `summary.warnings[].code`;
- `payload.source`;
- `payload.cargo`;
- `payload.artifacts`.

The envelope target records the selected ICP environment name. Its inputs also
contain one `build_environment` entry whose note records both that environment
and the resolved Canic build network, for example
`environment=staging;build_network=ic`. CI should retain this distinction:
environment names select deployment state, while the build network controls
the runtime class baked into Wasm.

Do not treat timestamps as provenance. `generated_at` explains when Canic wrote
the envelope; it does not prove that source, Cargo inputs, or produced bytes are
fresh.

## Dirty Source State

`payload.source` records Git state when available.

Recommended CI policy:

- accept `dirty == false` with `dirty_policy == "clean"`;
- fail or require manual approval when `dirty == true`;
- fail or require manual approval when `dirty` is absent or
  `dirty_policy == "unknown"`;
- archive `dirty_summary_digest` for review when present.

The dirty digest is `sha256(git status --porcelain=v1 -z)`. It is a review
signal, not a source snapshot. It does not make a dirty build reproducible, and
it intentionally does not upload diffs or file contents into the provenance
payload.

## Cargo Input Drift

`payload.cargo` records Cargo and package inputs used for the build.

Recommended CI policy:

- require `cargo_lock_sha256` to be present for workspace builds that should be
  lockfile-reproducible;
- compare `cargo_lock_sha256` against the expected branch, release, or baseline
  value when enforcing lock drift;
- require `package_manifest_sha256` to be present;
- compare `package_manifest` and `package_name` against the expected package;
- record `rustc_version`, `cargo_version`, `target`, and `profile` in CI
  artifacts;
- treat absent toolchain fields as review signals, not as proof of failure
  unless project policy says they are required.

Canic records evidence; it does not choose whether a project may build with
uncommitted lockfile changes, a different Rust toolchain, or non-default build
features.

Canic can evaluate the common source/Cargo/artifact checks with a passive
policy gate:

```toml
[build_provenance]
require_clean_source = true
require_cargo_lock = true
require_wasm_gzip = true
require_sha256 = true
require_package_identity_matches_target = true
```

See [CI Policy Gates](ci-policy-gates.md) for the full policy-file and
manifest workflow.

## Package Metadata Identity

Package identity comes from `[package.metadata.canic]`.

Recommended CI policy:

- require `payload.cargo.package_metadata_fleet == envelope.target.fleet`;
- require `payload.cargo.package_metadata_role == envelope.target.role`;
- require both values to match the intended release target;
- reject any artifact whose package metadata does not match the selected
  `fleet.role`.

Canic must not infer this identity from crate names. If package metadata is
missing or mismatched, the Canic-managed build should fail before provenance is
written.

## Artifact Hash Matching

`payload.artifacts` records produced artifacts with byte size and SHA-256 hash.

Recommended CI policy:

- require a `wasm_gzip` artifact for deployable Canic release artifacts;
- compare the `wasm_gzip.sha256` value against deployment-check artifact
  evidence when such evidence is available;
- compare raw `wasm.sha256` separately from `wasm_gzip.sha256`;
- treat gzip and raw Wasm hashes as different byte identities;
- require `hash_algorithm == "sha256"`;
- archive byte sizes so unexpected artifact growth is visible.

Build provenance does not import, register, pin, or garbage-collect artifacts.
It only records the bytes produced by the build path.

## Report Linkage

Adoption and deployment-check envelopes may fingerprint saved build provenance:

```text
--build-provenance <path>
```

In those report envelopes, the provenance file appears in `inputs` as:

```text
kind = "build_provenance"
schema.id = "canic.build_provenance.v1"
schema.stability = "stable"
```

This links reports back to saved build evidence without making provenance a
deployment-truth source. A deployment check remains responsible for its own
local plan, inventory, diff, and safety report.

## Non-Goals

This policy guidance does not add:

- signing or attestations;
- CI locks or project manifest semantics;
- artifact registry import;
- `wasm_store` retention or garbage collection;
- controller mutation;
- topology mutation;
- deployment install authority;
- active adoption/import.

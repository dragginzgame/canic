# Current Status

Last updated: 2026-07-01

## Purpose

This is the compact handoff for new agent sessions. Read this first, then
inspect only the files needed for the current task. Detailed historical status
before this compaction is archived at
`docs/status/archive/2026-06-30-precompact.md`.

## Current Line

- The active line is post-`0.76.9` bridge-free delegated auth cleanup.
  Delegated-token `RootProof` is chain-key-only:
  `RootProof::IcChainKeyBatchSignatureV1`. The old bridge-backed
  canister-signature delegated root-proof renewal path is historical
  documentation only, not public runtime/API/CLI code or active auth stable
  state.

- Closed 0.76 gates include chain-key batch DTO/canonical/verifier support,
  management-canister ECDSA signing wrappers, persisted chain-key root
  delegation batch state, delegated auth registry/proof epoch state, root timer
  prepare/sign/install orchestration, issuer lazy repair, high-s signature
  normalization, hard-cut config validation for
  `root_proof_mode = "chain_key_batch"`, local test-fleet chain-key trust
  anchors, removal of bridge-backed delegated root-proof endpoints/DTO/API/CLI
  surfaces, and live PocketIC coverage for no-external-liveness, lazy repair,
  multi-issuer batching, concurrent repair collapse, and legacy bridge surface
  absence.

- The `0.76.7` release completed the first pre-1.0 auth cleanup pass:
  recurring audit templates were refreshed, chain-key auth operator wording was
  clarified, a role-attestation data-certificate error was narrowed, and the
  chain-key batch renewal implementation was split into private `batch_id`,
  `install`, `merkle`, and `selection` helper modules.

- The `0.76.8` release completed the structure/docs continuation:
  active config docs now describe the required chain-key batch trust-anchor
  fields, auth DTO/API surfaces are split by concern while preserving
  `dto::auth::*` and `AuthApi::*` call paths, chain-key batch signing and tests
  have separate private modules, and focused Makefile validation targets cover
  auth, chain-key auth, CLI, and fast runtime checks.

- The `0.76.9` cleanup slice clarifies host/operator diagnostics and active
  auth docs after the hard cut: root-auth signer subnet wording, removed auth
  command-tail tests, role-attestation DTO docs, chain-key batch source-map
  docs, and deployment-target state diagnostics. It does not change
  delegated-auth runtime behavior.

- The `0.76.10` cleanup slice centralizes host deployment-truth report
  diagnostic codes and diff categories across artifacts, identity/config,
  controllers, canisters, pools, installed-module hashes, verifier readiness,
  observation assumptions, and receipt-aware resume checks without changing
  serialized output or delegated-auth runtime behavior.

- The `0.76.11` release completed the follow-up host deployment-truth
  diagnostic constant pass after `0.76.10`, covering authority overlap/unsafe
  blocker codes, executor/preflight blocker codes, comparison-input blockers,
  root-verification blockers, and receipt artifact-gate reuse of the
  report-owned artifact-missing code without changing serialized output.

- The `0.76.12` release completed the deployment-truth report producer
  module-boundary cleanup after `0.76.11`: leaf-local diagnostic constants are
  private, while producer-owned constants that tests or sibling report
  consumers intentionally import remain at their existing deployment-truth
  boundary.

- Current local cleanup is tightening deployment-truth executor-local metadata
  visibility and centralizing executor authority blocker subject derivation
  after `0.76.12` without changing behavior. A broader
  `unreachable_pub` scan found private-module host helpers where rustc and the
  repo's clippy policy prefer different visibility shapes; those are left
  unchanged rather than adding lint suppressions.

## Open Work

- No 0.76 gate remains open for timer renewal, lazy repair, multi-issuer
  batching, concurrent signing-volume proofing, verifier negatives,
  retry/failure state-machine coverage, platform-spec review, or non-Rust
  verifier fixture decisions.

- Before release preparation, run the focused gates for touched surfaces and
  broaden to the release matrix as needed. Do not assign a new patch version or
  change Cargo package versions unless the maintainer explicitly asks for
  release preparation.

## Useful Validation

Recent 0.76 validation already passed before this cleanup:

```text
cargo check --locked -p canic-core -p canic
cargo test --locked -p canic-core chain_key --lib
cargo test --locked -p canic-core chain_key_batch --lib
cargo test --locked -p canic-core workflow::runtime::auth --lib
cargo test --locked -p canic --test protocol_surface
cargo check --locked -p canic-tests --test root_suite
```

Focused aliases added for ordinary local iteration:

```text
make test-auth
make test-auth-chain-key
make test-cli
make test-runtime-fast
```

When PocketIC is available and the change touches live 0.76 auth behavior,
run:

```text
POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic \
  cargo test --locked -p canic-tests --test root_suite auth_076 -- --nocapture --test-threads=1
```

## Standing Constraints

- Preserve dirty worktree state and keep edits scoped.
- Do not change Cargo versions, workspace dependency versions, release script
  defaults, install URLs, or matching lockfile package versions unless the
  maintainer explicitly requests a version bump or release-preparation change.
- Follow `docs/governance/ci-deployment.md` for command, git, versioning, and
  release boundaries.
- Follow `docs/governance/changelog.md`; ordinary development goes under root
  `CHANGELOG.md` `Unreleased` when release notes are warranted.
- Follow `docs/governance/code-hygiene/README.md`; use directory modules with
  `mod.rs`, keep DTOs passive, and keep dependency direction
  `endpoints -> workflow -> policy -> ops -> model/storage`.

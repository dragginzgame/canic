# Current Status

Last updated: 2026-06-21

## Purpose

This file is the compact handoff for new agent sessions. Read it first, then
inspect only the files needed for the current task.

## Current Line

- `0.70.5` is in progress as a narrow blob-storage billing reserve-policy
  hardening slice after the pushed `0.70.4` release. Current work makes
  `_immutableObjectStorageFundFromProjectCycles` all-or-nothing against the
  configured project-cycle reserve: reserve-blocked requests return a skipped
  report with zero attached cycles instead of partially topping up Cashier. It
  also rejects unsafe billing configs with zero upload-balance thresholds or
  gateway principal limits that cannot fit the target runtime before stable
  config is replaced. Current work also consumes the local `ic-memory 0.7.1`
  bump and exposes a controller-only `canic_memory_ledger.memories` inventory
  of Canic stable memories with `memory_manager_id`, `stable_key`, `state`, and
  live backing `size` fields. Raw allocation records also retain optional
  `memory_size.wasm_pages` and `memory_size.bytes` diagnostics.
  Focused validation passing so far:
  ```text
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture
  cargo test --locked -p canic-core memory --lib -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage-billing -- -D warnings
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo clippy --locked -p canic --lib --features blob-storage-billing -- -D warnings
  cargo clippy --locked -p canic --test protocol_surface -- -D warnings
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```

- `0.70.4` is pushed as a narrow blob-storage billing hardening slice after the
  pushed `0.70.3` release. It maps malformed Cashier response decoding
  failures to the stable `InternalRpcMalformed` public error code instead of
  collapsing them into generic `Internal`. Backend billing
  status now distinguishes malformed Cashier balance payloads with explicit
  `BalanceMalformed` / `CashierBalanceMalformed` variants instead of reporting
  them as transient balance unavailability. PocketIC also proves malformed
  Cashier `account_top_up_v1` success payloads are rejected from
  `_immutableObjectStorageFundFromProjectCycles` with `InternalRpcMalformed`.
  The gateway-sync failure case now pins `InternalRpcMalformed` while still
  proving invalid Cashier gateway lists do not replace the previous local
  gateway set.
  Focused validation passing for this slice:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture
  cargo test --locked -p canic --features blob-storage-billing --test protocol_surface -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p canic-core --lib --features blob-storage-billing -- -D warnings
  cargo clippy --locked -p blob_storage_cashier_mock -- -D warnings
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  ```

- `0.70.3` is pushed as a narrow blob-storage billing test-hardening slice
  after the pushed `0.70.2` release. It adds controller-only one-shot
  failure hooks to the mock Cashier canister and uses PocketIC to prove
  `get_blob_storage_status` reports Cashier balance failures as
  `BalanceUnavailable` with the expected blocker/warning, and that the
  generated `_immutableObjectStorageUpdateGatewayPrincipals` endpoint rejects
  invalid Cashier gateway lists without replacing the previous local gateway
  set. It also proves the generated
  `_immutableObjectStorageFundFromProjectCycles` endpoint maps known Cashier
  `account_top_up_v1` failures to the stable public error codes
  introduced in `0.70.2`, and that mock Cashier failure-control endpoints
  remain controller-only. It also removes the stale no-op mock Cashier delay
  endpoint that was not part of the production protocol or active test flow.
  PocketIC now pins that forced mock top-up failures do not update the mock's
  successful top-up record.

- `0.70.2` is pushed as a narrow blob-storage billing hardening slice after the
  pushed `0.70.1` release. It rejects zero-cycle
  `_immutableObjectStorageFundFromProjectCycles` requests with `InvalidInput`
  before acquiring the transient funding guard, avoiding a misleading
  `reserve would be violated` skipped report when the caller requested no
  cycles. The funding guard still covers the project-cycle balance observation
  and Cashier top-up decision for nonzero requests. Known Cashier top-up
  failure variants now map to stable public Canic error codes instead of
  collapsing to generic `Internal`.

- `0.70.1` is pushed as a narrow blob-storage billing hardening slice after the
  pushed `0.70.0` backend MVP. It adds a transient in-memory single-flight guard
  around project-cycle funding so overlapping
  `_immutableObjectStorageFundFromProjectCycles` calls fail with a typed
  conflict while one Cashier top-up is already in progress. The guard is not
  stable state and releases automatically on drop, so upgrades start unlocked.
  The final cleanup adds read-only status decision coverage and keeps the
  billing-only stable-storage record impl gated so plain `blob-storage` still
  compiles without `blob-storage-billing`.

- `0.70.0` blob-storage billing is pushed as a Toko-compatible backend MVP. The
  maintainer has approved current local Toko `boss` commit
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56` as the protocol source for
  `account_balance_get_v1`, `account_top_up_v1`, and
  `storage_gateway_principal_list_v1`; production Cashier still has no default
  in tests or runtime config. The implementation now has the off-by-default
  `blob-storage-billing` feature, Cashier DTO/Candid snapshots, typed Cashier
  wrappers, stable billing config, configured gateway-principal sync, explicit
  project-cycle funding, read-only `get_blob_storage_status`, a billing endpoint
  macro with separate sync/funding/status guards, a standalone mock Cashier
  canister, and PocketIC coverage through the probe canister. Important protocol
  detail: `account_top_up_v1` takes an optional request record because Toko calls
  it with `Some(request)`. Deferred hardening remains status-triggered gateway
  sync, broader failure matrices, concurrent-funding stress coverage, and
  separate payment-account linking.
  Focused validation passing for this slice:
  ```text
  cargo fmt --all
  bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
  bash scripts/ci/check-blob-storage-inventory-gate.sh
  cargo check --locked -p canic-core --features blob-storage-billing
  cargo check --locked -p canic --features blob-storage-billing
  cargo check --locked -p blob_storage_probe
  cargo check --locked -p blob_storage_cashier_mock
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture
  cargo test --locked -p canic --features blob-storage-billing --test protocol_surface -- --nocapture
  cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture
  cargo test --locked -p canic --test workspace_manifest -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage-billing -- -D warnings
  cargo clippy --locked -p canic --lib --features blob-storage-billing -- -D warnings
  cargo clippy --locked -p blob_storage_probe -- -D warnings
  cargo clippy --locked -p blob_storage_cashier_mock -- -D warnings
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  bash scripts/ci/check-release-validation-matrix.sh
  git diff --check
  ```

- `0.69.5` is pushed as a focused blob-storage malformed-input
  regression cleanup. Current work closes the 0.69 design checklist gap for
  `BlobRootHash` malformed text coverage by explicitly testing missing
  prefixes, whitespace bytes, and control bytes, and expands gateway byte input
  conversion coverage to reject oversized as well as undersized hashes. It
  also pins deletion-confirmation inventory edge cases for unknown,
  live-but-not-pending, pending, and already-confirmed roots, and extends API
  facade coverage for missing `require_live`, repeated pending-delete marking,
  non-gateway batch-confirm no-op behavior, malformed-input no-mutation
  behavior, repeated create-certificate canonical idempotency, and explicit
  re-registration after gateway confirmation. Stable storage coverage now pins
  that uppercase/lowercase equivalent roots share one normalized live and
  pending-deletion key. This does not change runtime behavior, endpoint shape,
  Cashier/billing state, or public admin surface. The root and detailed 0.69
  changelogs are finalized for this slice.
  Focused validation passing for this slice:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic-core model::blob_storage::hash --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic-core ops::blob_storage::conversion --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic-core ops::blob_storage::lifecycle --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic-core api::blob_storage --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic-core storage::stable::blob_storage --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  git diff --check
  ```
  Post-push cleanup audit passing:
  ```text
  bash scripts/ci/check-blob-storage-inventory-gate.sh
  bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
  bash scripts/ci/run-layering-guards.sh
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  bash scripts/ci/check-recovery-runbooks.sh
  bash scripts/ci/check-diagnostic-consistency-audit.sh
  bash scripts/ci/check-release-package-install-validation.sh
  bash scripts/ci/check-rc-readiness-audit.sh
  cargo check --locked -p canic-core --features blob-storage
  cargo check --locked -p canic --features blob-storage
  cargo check --locked -p blob_storage_probe
  cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --features blob-storage --test blob_storage_endpoint_macro -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p canic --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p blob_storage_probe -- -D warnings
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  cargo test --locked -p canic --test workspace_manifest -- --nocapture
  cargo test --locked -p canic --test release_index_guard -- --nocapture
  cargo test --locked -p canic --test install_script_surface -- --nocapture
  make fmt-check
  make clippy
  make test-unit
  git diff --check
  ```

- `0.69.4` is pushed as a blob-storage developer-readiness cleanup slice.
  Current work replaces tuple-shaped local count plumbing with a named passive
  `BlobStorageLocalCounters` DTO and `BlobStorageApi::local_counters()` helper.
  The `blob_storage_probe` count query now returns the named DTO so downstream
  wrapper examples and local tests have stable, readable field names without
  adding gateway, Cashier, billing, or public admin endpoints. The integration
  runbook now includes a guarded host-status wrapper example for
  `local_counters()`. The slice also pins `create_certificate` compatibility:
  the returned DTO echoes the request hash for the gateway contract while
  internal live state stores the canonical normalized hash. The root and
  detailed 0.69 changelogs are finalized for this slice.
  Focused validation passing for this slice:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic --test protocol_surface blob_storage_gateway_dtos_roundtrip_through_candid -- --nocapture
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage -- --nocapture
  cargo check --locked -p blob_storage_probe
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p canic --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p blob_storage_probe -- -D warnings
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --features blob-storage --test blob_storage_endpoint_macro -- --nocapture
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  bash scripts/ci/check-blob-storage-inventory-gate.sh
  bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
  rg -n 'account_top_up_v1|storage_gateway_principal_list_v1|get_blob_storage_status|_immutableObjectStorageUpdateGatewayPrincipals|_immutableObjectStorageFundFromProjectCycles|Cashier|BlobProjectCyclesTopUpReport|BlobStorageBilling|GatewayPrincipalSync' crates canisters fleets -g '*.rs' -g '*.did' -g '*.toml'
  git diff --check
  ```
  The billing-leak scan returns no matches; `rg` exits 1 for that expected
  result.

- `0.69.3` is pushed as a blob-storage developer-readiness cleanup slice.
  Current work adds the downstream integration runbook, aligns the 0.69 design
  and handoff with the completed non-billing M0-M5 MVP, and extends the
  `blob_storage_probe` test canister with a controller-only gateway-principal
  removal helper and local count query. `BlobStorageApi` now exposes local
  stored-blob, pending-deletion, and gateway-principal count helpers for
  host-owned guarded status endpoints. PocketIC coverage now proves that
  removing a gateway principal revokes pending-deletion listing and makes
  deletion confirmation a no-op until the principal is re-added, while the
  counters stay consistent across pending deletion, revocation, re-add,
  confirmation, and upgrade. This remains test/developer surface only; 0.69
  still does not add a production Cashier sync endpoint or billing surface.
  The root and detailed 0.69 changelogs are finalized for this slice.
  Focused validation for this slice:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  cargo check --locked -p canic --features blob-storage
  cargo clippy --locked -p canic --lib --features blob-storage -- -D warnings
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --features blob-storage --test blob_storage_endpoint_macro -- --nocapture
  cargo check --locked -p blob_storage_probe
  cargo clippy --locked -p blob_storage_probe -- -D warnings
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  bash scripts/ci/check-blob-storage-inventory-gate.sh
  bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
  git diff --check
  ```

- `0.69.2` is pushed as the non-billing blob-storage backend lifecycle
  slice. Current Toko `boss` is
  clean at commit `9ca150b396a2bde42f2b8977a04a7ca2c6172b56` and is accepted
  as the 0.69 project-side protocol source. The gateway inventory records all
  six `_immutableObjectStorage*` method sections as complete Toko evidence,
  and the 0.69 implementation gate now accepts the completed inventory.
  Current Toko maps remote assets directly through
  `PostRemoteAsset.blob_root_hash` and `RemoteAsset.blob_root_hash`, requires
  production assets to reference registered live blobs, marks that same root
  hash pending deletion on remote-asset deletion, and intentionally permits
  placeholder hashes only in local builds. M1-M5 are complete for the current
  non-billing MVP: the off-by-default `blob-storage` feature, four non-billing
  gateway protocol constants/Candid fixture coverage, passive
  `CreateCertificateResult` DTO, Toko-compatible canonical `BlobRootHash`
  validation/conversion, and `BlobStorageApi` helpers for `sha256:<64-hex>`
  strings and 32-byte gateway inputs are in place. Stable live-blob,
  pending-deletion, and gateway-principal maps exist behind the feature, with
  lifecycle/API helpers for register-live, check/require-live,
  mark-pending-delete, gateway-confirm-delete, pending-deletion listing, and
  gateway-principal membership. `canic_emit_blob_storage_endpoints!(guard =
  ...)` exports exactly the four 0.69 non-billing `_immutableObjectStorage*`
  methods with exact method names, create-certificate guard wiring,
  gateway-principal filtering for scrubber endpoints, and no deferred
  billing/sync method emission. A
  standalone `blob_storage_probe` canister now proves the non-billing lifecycle
  under PocketIC: create-certificate registers a live root, liveness queries use
  32-byte gateway inputs, pending deletion is gateway-filtered, non-gateway
  confirmation is ignored, gateway confirmation clears pending state, and live,
  pending-deletion, and gateway-principal stable state survives a post-upgrade
  round trip. The same test covers liveness batch duplicate and input-order
  behavior plus multi-row pending-deletion repeat-return shape. Protocol
  surface tests now pin that only create-certificate uses the host guard, while
  scrubber methods remain gateway-principal scoped. 0.70 billing remains
  separately gated:
  the Cashier inventory records Toko call-site DTO/wrapper expectations only,
  and actual Cashier implementation or generated/deployed Cashier `.did`
  remains missing. The source-inspection handoff is
  `docs/operations/blob-storage-source-handoff.md`. The downstream integration
  runbook is `docs/operations/blob-storage-integration.md`.
  Focused validation for this pushed slice:
  ```text
  cargo fmt --all -- --check
  cargo check --locked -p canic-core --features blob-storage
  cargo check --locked -p canic --features blob-storage
  cargo check --locked -p blob_storage_probe
  cargo test --locked -p canic-core blob_storage --lib --features blob-storage -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --features blob-storage --test blob_storage_endpoint_macro -- --nocapture
  cargo clippy --locked -p canic-core --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p canic --lib --features blob-storage -- -D warnings
  cargo clippy --locked -p canic --features blob-storage --test blob_storage_endpoint_macro -- -D warnings
  cargo clippy --locked -p blob_storage_probe -- -D warnings
  cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
  cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture
  POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  bash scripts/ci/check-blob-storage-inventory-gate.sh
  bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
  rg -n 'account_top_up_v1|storage_gateway_principal_list_v1|get_blob_storage_status|_immutableObjectStorageUpdateGatewayPrincipals|_immutableObjectStorageFundFromProjectCycles|Cashier|BlobProjectCyclesTopUpReport|BlobStorageBilling|GatewayPrincipalSync' crates canisters fleets -g '*.rs' -g '*.did' -g '*.toml'
  git diff --check
  ```

- `0.69.1` is pushed as the ICP CLI 1.0 compatibility patch. Local
  `icp --version` reports `icp 1.0.0`. The official release notes call out the
  default gateway-domain change to `icp.net`, password-protected identity
  session caching, and removal of `--set-controller`. The active Canic codebase
  has no `--set-controller` or `icp0.io` dependency, `tool-versions.env`
  already pins `CANIC_ICP_CLI_VERSION=1.0.0`, and the host/CLI compatibility
  gate has been updated from the old 0.3.x line to `>=1.0.0, <2.0.0`.
  Operator ergonomics now document `icp settings session-length` /
  `icp identity reauth`, and `canic info medic` reports those commands as a
  non-failing hint; no release-critical flow depends on session caching.

- `0.68.26` is prepared as the root proof provisioning audit closeout and
  blob-storage handoff point. The 0.68 MVP remains:
  root issuer policy upsert -> root batch prepare -> direct root query get ->
  root batch install -> issuer-local active proof status -> issuer-local
  delegated-token prepare/get. The latest pass refreshed the oldest recurring
  audit definitions and recorded passing audience-target-binding,
  capability-scope-enforcement, and change-friction reports. Root proof
  provisioning is ready to hand focus back to the deferred blob-storage line.

- `0.66.8` is pushed as the current post-0.65 feedback baseline. Pause broad
  cleanup until real Canic usage reports concrete regressions or patch-worthy
  defects. This release renamed the
  shared root-key/root-canister trust-anchor config from delegated-token-only
  wording to `AuthProofVerifierConfig`, splits non-root startup predicates so
  role-attestation caches and token issuers require root proof verification
  support while endpoint delegated-token verifiers additionally require issuer
  proof verification support, renames the private root delegation proof client,
  renames the internal delegated root certificate builder away from issue
  terminology, renames internal auth operation inputs away from one-shot
  signing terminology, removes unused RPC workflow error variants that still
  used `shard_pid` auth wording, and cleans active docs/runbooks/audit
  references away from stale shard/mint wording.
  Recorded validation for `0.66.8`:
  ```text
  cargo fmt --all -- --check
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
  cargo test --locked -p canic-core root_delegation_proof_client_endpoint_table_is_prepare_only --lib -- --nocapture
  cargo test --locked -p canic-core auth_proof_verifier_config --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core workflow::rpc --lib -- --nocapture
  cargo check --locked -p canic-tests
  bash scripts/ci/check-diagnostic-consistency-audit.sh
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.30` is committed and the 0.65 line is now a zero-management-ECDSA
  normal-auth hard cut. Delegated-token root proofs, delegated-token issuer
  proofs, and `SignedRoleAttestation` proofs use IC canister signatures with
  update/query prepare-get flows. Delegated tokens are self-contained,
  issuer-signed, reusable until TTL/audience/grant/subject checks fail, and
  may carry opaque signed `ext: Option<Vec<u8>>` claims bytes. Issuers use
  controller-installed `ActiveDelegationProof` state for token preparation;
  protected endpoint verification is local and synchronous with no root,
  issuer, or management-canister call on the hot path. Normal auth no longer
  exposes the shard ECDSA token leg, threshold-ECDSA auth features, one-shot
  `mint_token`, standalone delegated-grant capability proofs, protected
  internal-invocation proofs, or `SubnetState` delegated root-key trust. The
  checked-in wasm-store Candid sidecar, active auth docs, 0.65 design status,
  and root/detailed changelogs are aligned through `.31`. Remaining closeout is
  broad release validation and maintainer-owned release preparation.
  Planning note: the former 0.66 endpoint perf-observability design is now
  `docs/design/0.67-perf-observability/0.67-design.md`. The 0.66 line is
  reserved for auditing, testing, and fixing the 0.65 auth epoch, with no new
  auth protocol features. The `0.66.0` changelog is prepared in
  `CHANGELOG.md` and `docs/changelog/0.66.md` for dev handoff/testing.
  Current focused validation:
  ```text
  cargo fmt --all -- --check
  cargo check --locked -p canic-core -p canic
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo tree -i time --locked
  cargo check --locked -p canic-tests
  cargo clean -p ic-agent
  cargo check --locked -p canic-host
  cargo test --locked -p canic-core ops::auth --lib -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  rg -n 'shard_sig|sign_with_ecdsa|EcdsaOps::sign_bytes|auth-threshold-ecdsa-sign|auth-threshold-ecdsa-public-key|auth-shard-secp256k1-verify|ThresholdEcdsaSign|IcThresholdEcdsaSecp256k1|EcdsaP256Sha256|shard_public_key_sec1|key_name_hash|derivation_path_hash|RootPublicKeyRecord|RootTrustAnchor|DelegationProof.*root_sig|sign_prepared_delegation_proof|DelegatedTokenMintRequest|mint_token|request_internal_invocation_proof|InternalInvocationProof|SignedDelegatedGrant|DelegatedGrantProof' crates canisters fleets scripts Cargo.toml Makefile -g '*.rs' -g '*.toml' -g '*.did' -g '*.sh' -g 'Cargo.toml' -g 'Makefile'
  rg -n 'delegated_root_public_key|SubnetRootPublicKeyInput|RootPublicKeyRecord' crates canisters fleets scripts Cargo.toml Makefile -g '*.rs' -g '*.toml' -g '*.did' -g '*.sh' -g 'Cargo.toml' -g 'Makefile'
  git diff --check
  ```
- `0.65.1` is pushed as the threshold-ECDSA public-key feature split. The
  patch decouples delegated-token root proof startup from root threshold-ECDSA
  signing. `auth-threshold-ecdsa-public-key`
  now gates management-canister ECDSA public-key fetches, while
  `auth-threshold-ecdsa-sign` gates signing and includes the public-key feature.
  Root runtime startup requires `auth-root-canister-sig-create` plus
  `auth-threshold-ecdsa-public-key` for configured delegated-token proof
  issuance, and shard token issuers require `auth-threshold-ecdsa-sign`.
  Legacy delegated-grant root public-key publication now skips cleanly when
  public-key fetch support is not compiled. Current validation:
  ```text
  cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
  cargo test --locked -p canic-core --features auth-root-canister-sig-create,auth-threshold-ecdsa-public-key workflow::runtime::auth --lib -- --nocapture
  cargo check --locked -p canic-core --features auth-root-canister-sig-create
  cargo check --locked -p canic-core --features auth-root-canister-sig-create,auth-threshold-ecdsa-public-key
  cargo check --locked -p canic-core --features auth-delegated-token-verify,auth-threshold-ecdsa-sign
  cargo check --locked -p canic --features auth-root-canister-sig-create,auth-threshold-ecdsa-public-key,control-plane
  cargo check --locked -p canic --features auth-threshold-ecdsa-sign
  cargo check --locked -p delegation_root_stub
  cargo check --locked -p delegation_signer_stub
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.2` is pushed as the post-hard-cut auth surface cleanup pass. Normal
  client helpers for one-shot root ECDSA role-attestation and
  internal-invocation proof issuance failed locally with the hard-cut `0.65`
  error instead of routing to root only to be rejected. The root rejection
  endpoints and workflow tests remained as explicit compatibility-failure
  coverage, while `RootAuthMaterialClient` listed only the still-needed
  structural bootstrap calls for attestation key-set refresh and delegation
  proof preparation. The stale outbound protected-internal proof cache and
  public client surface (`CanicCall`, `CanicInternalClient`, and
  `canic_internal_client!`) were removed, leaving protected endpoint
  descriptors only as retained verifier/rejection metadata. Active docs direct
  normal parent/shard calls to delegated-token endpoints until a replacement
  protected-internal proof protocol exists. The root and detailed `0.65.2`
  changelogs are finalized. Validation:
  ```text
  cargo test --locked -p canic-core request_role_attestation_fails_locally_after_hard_cut --lib -- --nocapture
  cargo test --locked -p canic-core request_internal_invocation_proof_fails_locally_after_hard_cut --lib -- --nocapture
  cargo test --locked -p canic-core root_auth_material_client_endpoint_table_is_structural_bootstrap_only --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core api::ic::canic --lib -- --nocapture
  cargo test --locked -p canic-core --test protected_internal_call_guard -- --nocapture
  cargo test --locked -p canic --test protected_endpoint_macro -- --nocapture
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo check --locked -p project_hub_stub -p project_instance_stub -p project-protocol-stub
  cargo clippy --locked -p canic-core --lib -- -D warnings
  git diff --check
  ```
- `0.65.3` is pushed as the hard-cut cleanup that deleted the dead
  normal-auth `AuthApi::request_role_attestation` and
  `AuthApi::request_internal_invocation_proof` wrappers instead of retaining
  public methods that only return the hard-cut error. The root
  `canic_request_role_attestation` and
  `canic_request_internal_invocation_proof` rejection endpoints remain as the
  explicit compatibility-failure surface. The stale delegation root test
  endpoint `root_issue_self_attestation` is also removed; the explicit
  `*_test` attestation helpers used by PIC role-attestation tests remain. The
  sharding root test stub also no longer exposes a direct fake-signing
  `canic_request_role_attestation` endpoint; its capability RPC path continues
  to reject role-attestation issuance. Outbound root-capability RPC now rejects
  non-structural proof paths locally instead of fetching or caching a fresh
  root response attestation from the removed one-shot issuer.
  The root and detailed `0.65.3` changelogs are finalized. Validation:
  ```text
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core ops::rpc --lib -- --nocapture
  cargo check --locked -p delegation_root_stub
  cargo check --locked -p sharding_root_stub
  cargo test --locked -p canic-core --test protected_internal_call_guard -- --nocapture
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo check --locked -p canic-core -p canic
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.4` is pushed as the zero-management-ECDSA normal-auth closeout scope
  and design cleanup. It records the remaining issuer-proof, role-attestation,
  delegated-grant, feature, Candid, and verifier-purity blockers; rewrites
  `docs/design/0.65-canister-signatures/0.65-design.md` as the focused active
  Canic 0.65 protocol; preserves `DelegatedRoleGrant` token grants and
  issuer-signed opaque `ext: Option<Vec<u8>>` claims data; pins the exact
  canister-signature verifier message construction, `"sig"` certified-data
  shape, positive cache value, root-key/network startup checks, and TTL
  guidance; and rejects inbound root-capability
  `CapabilityProof::RoleAttestation` envelopes before attestation signature
  verification or replay dispatch. Validation:
  ```text
  cargo test --locked -p canic-core workflow::rpc::capability --lib -- --nocapture
  cargo test --locked -p canic-core --features auth-shard-secp256k1-verify verify_root_delegated_grant_signature --lib -- --nocapture
  cargo test --locked -p canic-core --test protected_internal_call_guard -- --nocapture
  cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint -- --nocapture
  cargo fmt --all -- --check
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.5` is committed as the standalone delegated-grant capability proof hard
  cut. `CapabilityProof::DelegatedGrant` remains a legacy wire shape classified
  for rejection metrics, but now fails before payload decode, capability-hash
  checks, secp256k1 verification, replay, or capability execution. The
  dedicated delegated-grant capability verifier module, delegated-grant blob
  encode/decode helpers, and `TryFrom<DelegatedGrantProof>` success conversion
  are removed. Token grants remain untouched as `DelegatedRoleGrant` values
  inside delegation certs and delegated-token claims. Validation:
  ```text
  cargo test --locked -p canic-core workflow::rpc::capability --lib -- --nocapture
  cargo test --locked -p canic-core --test protected_internal_call_guard -- --nocapture
  cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint -- --nocapture
  cargo fmt --all -- --check
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.6` is committed as a bounded heap-only positive delegated-token
  verifier cache. Current tokens are still shard-signature based, so the cache
  key binds proof hash, claims hash, current `shard_sig` hash, and caller; the
  final issuer-proof cut must replace that signature component with
  `issuer_proof_hash`. Cache values contain only `valid_until_ns` and
  `verified_at_ns`, expire no later than token/cert expiry, and are bounded to
  1024 entries with oldest-verified eviction. On cache hits, the verifier still
  reruns canonical token checks plus local time, audience, grant, subject, and
  endpoint scope authorization before returning. Current validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core access::auth::token --lib -- --nocapture
  cargo fmt --all -- --check
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.7` is committed as opaque signed delegated-token `ext` bytes on
  the current token leg. `DelegatedTokenIssueRequest` and
  `DelegatedTokenClaims` now carry `ext: Option<Vec<u8>>`; canonical claims
  encoding includes a distinct ext presence marker plus ext bytes; the current
  shard-token signature covers ext through the canonical claims hash; and
  preparation plus verification reject ext payloads above 4096 bytes. This is
  still the current shard-signature token shape; the issuer-proof hard cut must
  preserve the same signed claims field when
  `IssuerProof::IcCanisterSignatureV1` replaces `shard_sig`. Current
  validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core access::auth::token --lib -- --nocapture
  cargo check --locked -p canic-testing-internal
  ```
- `0.65.8` is committed as the issuer-proof hard cut foundation without flipping
  endpoint behavior yet. It adds `IssuerProof`,
  `IssuerProofAlgorithm`, and `IssuerProofBinding` DTOs; canonical
  `IssuerProof` bytes/hash; `issuer_proof_binding_hash` over issuer principal,
  proof algorithm, binding, and optional signer generation; issuer
  canister-signature seed/domain/verifier-message helpers for
  `DelegatedTokenClaims`; and a future positive-cache key helper that binds
  proof hash, claims hash, `issuer_proof_hash`, and caller. Current runtime
  verification still uses the shard-signature wrapper until `DelegatedToken`
  carries `issuer_proof`. Current validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core ops::auth::issuer_canister_sig --lib -- --nocapture
  cargo fmt --all -- --check
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.9` is committed as the explicit delegated-token audience hardening. It
  replaces the legacy global delegated-token audience
  with explicit `Canister`, `CanicSubnet`, and `Project` audiences. Token
  verification now receives local canister and optional local Canic subnet
  context, canonical/replay/PIC audience hashing binds the concrete audience
  value, and token-issue replay payload hashes also bind signed `ext` bytes.
  Validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core access::auth::token --lib -- --nocapture
  cargo check --locked -p canic-core -p canic -p canic-testing-internal
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.10` is committed as the persisted active-delegation-proof
  foundation for issuer-local token issuance. `ActiveDelegationProof` is now a
  passive DTO, stable auth state has explicit active-proof records with
  backward-compatible default decode, `AuthStateOps` exposes set/get/clear
  accessors, and active proof lookup fails closed when the installed proof is
  not yet valid or expired. The install endpoint and issuer prepare/get
  canister-signature flow remain pending. Current validation:
  ```text
  cargo test --locked -p canic-core ops::storage::auth --lib -- --nocapture
  cargo test --locked -p canic-core storage::stable::auth --lib -- --nocapture
  cargo check --locked -p canic-core -p canic -p canic-testing-internal
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.11` is committed as active-delegation-proof install validation
  for issuer-local root-certified authority. It adds passive install
  request/response DTOs and a pure validation helper that requires the
  delegation cert to target the current issuer canister, rejects not-yet-valid
  or expired certs, computes the canonical cert hash, verifies the root
  canister-signature proof through configured root trust anchors, and persists
  the resulting `ActiveDelegationProof` with install metadata and a refresh
  target inside the cert lifetime. The public install endpoint and issuer
  prepare/get token canister-signature flow remain pending. Current validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated::active_proof --lib -- --nocapture
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo check --locked -p canic-core -p canic -p canic-testing-internal
  cargo fmt --all -- --check
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.12` is committed as the controller-gated non-root
  `canic_install_active_delegation_proof` endpoint. `AuthApi` now has a public
  install wrapper over the `.11` validation/store path, the non-root auth
  provisioning endpoint bundle emits the installer, protocol constants include
  `CANIC_INSTALL_ACTIVE_DELEGATION_PROOF`, the replay manifest classifies the
  endpoint as intentionally non-idempotent controller maintenance, and the
  canonical `wasm_store.did` was refreshed to include the install DTOs and
  method. The issuer prepare/get token canister-signature flow remains pending.
  Current validation:
  ```text
  CANIC_CONFIG_PATH=fleets/test/canic.toml CANIC_REFRESH_WASM_STORE_DID=1 cargo run -q -p canic-host --example build_artifact -- wasm_store
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic --test endpoint_macro -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo check --locked -p canic-core -p canic -p canic-testing-internal
  cargo fmt --all -- --check
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.13` is committed as the issuer canister-signature primitive for
  the next token-issuer hard cut. `canic-core` and facade `canic` now expose
  `auth-issuer-canister-sig-create` and
  `auth-issuer-canister-sig-verify`; the issuer auth module now mirrors the
  root SignatureMap pattern for claim-hash prepare/get, refreshes certified
  data to the exact `labeled_hash(b"sig", SIGNATURES.root_hash())` shape,
  stores caller-bound pending retrieval metadata with the one-minute retrieval
  window, returns `IssuerProof::IcCanisterSignatureV1`, and verifies issuer
  canister id, seed, `domain_len || issuer_domain || claims_hash`, and raw IC
  root key. Issuer-proof prepare metrics and an
  `IssuerCanisterSignaturePrepare` replay cost class are present for the
  upcoming public endpoint. This does not yet replace `DelegatedToken.shard_sig`
  with `issuer_proof` or remove shard ECDSA normal-auth features. Current
  validation:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic-core ops::auth::issuer_canister_sig --lib -- --nocapture
  cargo test --locked -p canic-core ops::runtime::metrics::delegated_auth --lib -- --nocapture
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo check --locked -p canic-core --features auth-issuer-canister-sig-create
  cargo check --locked -p canic-core --features auth-issuer-canister-sig-verify
  cargo check --locked -p canic --features auth-issuer-canister-sig-create
  cargo check --locked -p canic --features auth-issuer-canister-sig-verify
  cargo check --locked -p canic-core -p canic -p canic-testing-internal
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo clippy --locked -p canic-core --lib --features auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify -- -D warnings
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.16` is pushed and moves `SignedRoleAttestation` from root ECDSA
  signatures to root canister-signature proofs. The root auth bundle now emits
  `canic_prepare_role_attestation` update plus `canic_get_role_attestation`
  query; prepare validates caller/subject/role/subnet/TTL, replay-protects the
  request under `auth.prepare_role_attestation.v1`, caches caller-bound pending
  payload metadata, and commits the root `"sig"` certified-data tree. Get
  returns the prepared payload with `RootProof::IcCanisterSignatureV1`.
  Runtime role-attestation verification now verifies the embedded root proof
  against the configured root canister id plus raw IC root key and performs
  local subject/audience/subnet/time/epoch checks with no root refresh or
  management-canister call on the protected path. The old direct one-shot
  `canic_request_role_attestation` endpoint,
  `AuthApi::request_role_attestation_root` wrapper, and role-attestation
  capability-RPC issuance variant are removed. Legacy standalone capability
  proof DTOs and disabled proof-mode branches are removed; root capability
  envelopes now accept only structural proofs. The ECDSA attestation key refresh
  timer is now inert for role attestations, and the PIC role-attestation helper
  uses the real prepare/get flow. Current validation:
  ```text
  cargo fmt --all
  cargo fmt --all -- --check
  cargo check --locked -p canic-core -p canic
  cargo check --locked -p delegation_root_stub -p delegation_signer_stub -p canister_root -p canister_user_shard -p canic-tests
  cargo check --locked -p delegation_root_stub -p delegation_signer_stub -p sharding_root_stub
  cargo test --locked -p canic-core ops::auth::root_canister_sig --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
  cargo test --locked -p canic-core workflow::rpc --lib -- --nocapture
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo test --locked -p canic-core --test protected_internal_call_guard -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo check --locked -p canic-core --features auth-root-canister-sig-create,auth-root-canister-sig-verify
  cargo check --locked -p canic --features auth-root-canister-sig-create,auth-root-canister-sig-verify
  cargo check --locked -p canic-tests
  git diff --check
  ```
  Attempted targeted PocketIC validation:
  ```text
  TMPDIR="$(pwd)/.tmp/test-runtime" ICP_ENVIRONMENT=local cargo test --locked -p canic-tests --test pic_role_attestation verification -- --test-threads=1 --nocapture
  ```
  That run rebuilt local wasm artifacts and reached serialized PocketIC startup,
  then hit the known `Failed to bind PocketIC server to address 127.0.0.1:0`
  infrastructure panic. The idle test process group was interrupted with
  `kill -INT`.
  Follow-up from design review: the active 0.65 design/status docs require
  explicit forwarded-user-token rejection coverage; implementation remains
  pending.
- `0.65.17` is pushed and deletes the remaining legacy protected-internal
  call envelope protocol and verifier-root-key leftovers from active code. The
  slice removes `canic_protected_endpoint!`, `ProtectedInternalEndpoint`,
  generated internal endpoint descriptors, `CanicInternalCall*` DTOs, internal
  invocation proof verification, app-role caller predicates, protected internal
  tests, the `canic_attestation_key_set` endpoint, verifier-local attestation
  key cache/storage, delegated-root-public-key subnet state, shard secp256k1
  verification features, and the threshold-ECDSA public-key auth feature. Test
  canisters now model service calls through delegated-token endpoints, while
  active role attestations remain on root canister-signature prepare/get. The
  auth trust-chain CI guard now checks that the retired key-refresh verifier
  flow stays absent instead of reading the deleted `verify_flow.rs` module.
  Current validation:
  ```text
  TMPDIR="$(pwd)/.tmp/test-runtime" ICP_ENVIRONMENT=local bash scripts/ci/run-workspace-tests.sh fast
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo test --locked -p canic-core ops::runtime::metrics::auth --lib -- --nocapture
  cargo test --locked -p canic-core workflow::metrics::query --lib -- --nocapture
  cargo test --locked -p canic-core ops::storage::state --lib -- --nocapture
  cargo test --locked -p canic-core storage::stable::state --lib -- --nocapture
  cargo test --locked -p canic-macros --lib -- --nocapture
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo clippy --locked -p canic-macros --lib -- -D warnings
  cargo check --locked -p canic-core --features auth-root-canister-sig-create,auth-root-canister-sig-verify,auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify
  cargo check --locked -p canic --features auth-root-canister-sig-create,auth-root-canister-sig-verify,auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify
  cargo check --locked -p project_instance_stub -p project-protocol-stub -p sharding_root_stub
  cargo check --locked -p canic-tests
  cargo fmt --all
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.18` is pushed and adds the explicit
  `AUTH_TIME_SKEW_ALLOWANCE_NS = 60_000_000_000` verifier allowance for
  not-from-the-future checks. Delegated-token verification now accepts
  delegation cert `not_before_ns` and token `claims.issued_at_ns` values up to
  60 seconds ahead of verifier time, role-attestation verification accepts
  `issued_at_ns` up to 60 seconds ahead, and expiry remains strict with no
  grace after signed `expires_at_ns`. Current validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated::verify --lib -- --nocapture
  cargo test --locked -p canic-core ops::auth::verify::attestation --lib -- --nocapture
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.19` is pushed and removes caller-provided delegated-token nonce
  input. `DelegatedTokenPrepareRequest`, `PrepareDelegatedTokenIssuerProofInput`, and the
  internal token-preparation input no longer accept nonce bytes; issuer token
  preparation derives `DelegatedTokenClaims.nonce` from `"canic-token-nonce-v1"`,
  `prepared_by`, prepare `operation_id`, `subject`, `issuer_pid`, and selected
  `cert_hash`. Delegated-token prepare replay payload hashing no longer binds a
  request nonce field, while replay identity still binds the operation id
  through shared replay receipts and binds subject, audience, grants, TTL, and
  `ext` through the payload hash. The auth trust-chain CI guard now rejects
  `raw_rand`, management-canister calls, `.await`, and direct call sites in the
  token preparation modules. Current validation:
  ```text
  cargo test --locked -p canic-core ops::auth::delegated::prepare --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo check --locked -p canic-testing-internal
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo check --locked -p canic-tests
  cargo check --locked -p canic-core -p canic
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.20` is pushed and pins the auth certified-data ownership boundary in
  CI. The auth trust-chain guard requires the only active Rust
  `certified_data_set` callers to be the root and issuer canister-signature
  helpers, and requires both owners to commit
  `labeled_hash(LABEL_SIG, signature_root_hash)` so the canister-signature
  `"sig"` tree shape cannot drift. Validation:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.21` is pushed and deletes the
  isolated threshold-ECDSA signing adapter, removes the
  `auth-threshold-ecdsa-sign` feature from `canic-core` and the `canic`
  facade, drops stale threshold-ECDSA replay external-effect and platform-call
  metric variants, removes dead `ecdsa_key_name` config knobs plus checked-in
  test config values, removes the unused direct workspace `k256` dependency,
  removes absence-only legacy surface tests, deletes the auth trust-chain and
  forbidden-crypto grep scripts plus the delegated-auth hard-cut guard test,
  removes stale replay actor issuer shard metadata, and removes the short-lived
  certified-data owner grep guard in favor of the design-status audit note.
  It also removes removed-design macro/parser/config/CLI tests and the special
  `caller::has_app_role` parser rejection branch, leaving unknown predicates to
  fail through the normal parser path. The legacy Canic memory-ledger probe is
  removed too; non-native ledger payloads now use the ordinary corrupt-ledger
  error.
  Active auth docs now describe issuer canister-signature tokens instead of the
  old shard ECDSA token leg, and the 0.65 design doc treats removed proof
  families as absent surfaces instead of decode-and-reject compatibility
  branches. Operations docs and the recurring auth-abstraction audit no longer
  instruct maintainers to run deleted source-shape grep guards.
  Active-code scan now finds no
  `EcdsaOps::sign_bytes`, `auth-threshold-ecdsa-sign`,
  `ThresholdEcdsaSign`, `EcdsaPurpose`, `ecdsa_key_name`, direct `k256`,
  `secp256k1`, or ECDSA platform metric surface in active auth/source/test
  feature wiring. Current validation:
  ```text
  cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
  cargo test --locked -p canic-core ops::replay --lib -- --nocapture
  cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
  cargo test --locked -p canic-core config::schema --lib -- --nocapture
  cargo test --locked -p canic-core config::schema::subnet --lib -- --nocapture
  cargo test --locked -p canic-core memory::ledger --lib -- --nocapture
  cargo test --locked -p canic-macros --lib -- --nocapture
  cargo test --locked -p canic-cli --lib -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo check --locked -p canic-host
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  bash scripts/ci/check-recovery-runbooks.sh
  bash scripts/ci/check-diagnostic-consistency-audit.sh
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.22` is pushed and adds optional local Candid sidecar
  support to shared ICP CLI canister call/query helpers. Host and CLI paths now
  pass `--candid .icp/<network>/canisters/<role>/<role>.did` when Canic can
  resolve an existing generated sidecar from the project root and registry role,
  covering root registry reads, `info list` live probes, `info medic` root
  readiness, metrics queries, cycles reports, backup/snapshot registry
  preflights, and canister-mode `cycles convert` refill calls. Calls without a
  sidecar keep the previous ICP CLI behavior, and management-canister fabricate
  calls remain unchanged. Follow-up CLI audit centralizes role/registry-entry
  sidecar lookup in `support::candid` and resolves cycles report ICP
  root/Candid context once per canister worker instead of repeating it for every
  endpoint probe. Current validation:
  ```text
  cargo test --locked -p canic-host icp --lib -- --nocapture
  cargo check --locked -p canic-host
  cargo check --locked -p canic-cli
  cargo test --locked -p canic-cli --lib -- --nocapture
  cargo clippy --locked -p canic-host -p canic-cli --lib --tests -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.23` is pushed and removes
  unused internal-invocation proof scope error variants from active auth errors
  and renames non-root delegated-token startup checks/logging around issuer
  canister-signature support instead of stale signer-key material wording.
  The active canister auth config key is renamed from
  `delegated_token_signer` to `delegated_token_issuer` across schema,
  checked-in configs, bootstrap rendering, and release-set details.
  Active config documentation no longer lists delegated-token or
  role-attestation ECDSA key settings in the current 0.65 auth config surface.
  `Cargo.lock` now pins the transitive `time` crate line to `0.3.41` so the
  IC/PocketIC build path does not pick up the incompatible `0.3.48` update
  through `tracing-subscriber 0.3.23`.
  It also renames local root proof preparation variables away from generic root
  signature wording in delegation-proof and role-attestation preparation, and
  renames delegated-token root verification failure reporting from
  root-signature invalid to root-proof invalid. Active source/test scans now
  find no removed internal-invocation proof names or removed ECDSA token-leg
  names. Current validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
  cargo test --locked -p canic-core ops::auth --lib -- --nocapture
  cargo test --locked -p canic-core config::schema::subnet --lib -- --nocapture
  cargo test --locked -p canic-host release_set --lib -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo check --locked -p canic-host
  cargo check --locked -p canic-tests
  cargo tree -i time --locked
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.24` is committed as the delegation-issuer test-surface rename. It
  renames the active test canister package from
  `delegation_signer_stub` to `delegation_issuer_stub`; renames the root-managed
  test role from `signer` to `issuer`; updates the embedded root-stub build
  inputs, wasm include name, and checked-in test configs; renames PIC
  role-attestation fixture metadata/helpers from signer-oriented names to
  issuer-oriented names; renames the delegation issuer stub endpoints from
  `signer_*` to `issuer_*`; renames copied project-hub verifier helper
  endpoints to `verifier_*`; and corrects root-stub build-script rerun markers
  for embedded test canister packages to point at `canisters/test`. Current
  validation:
  ```text
  cargo fmt --all
  cargo check --locked -p delegation_issuer_stub -p delegation_root_stub -p project_hub_stub
  cargo check --locked -p canic-testing-internal -p canic-tests
  cargo tree -i time --locked
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.25` is committed as the issuer signer generation removal. It removes
  the unsupported issuer signer generation hook from active
  delegation certificates, stable auth records, canonical cert bytes, issuer
  proof binding hashes, wasm-store Candid, fixtures, and active auth docs.
  Issuer proof binding now covers only the active authority context: issuer
  canister id, issuer proof algorithm, and issuer proof binding. Current
  validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core ops::storage::auth --lib -- --nocapture
  cargo test --locked -p canic-core access::auth::token --lib -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.26` is committed as the active auth proof terminology cleanup. It
  removes unused token-signature auth error variants, reports
  root/issuer canister-signature failures as proof failures, renames
  role-attestation verifier errors to attestation proof wording, and updates the
  positive delegated-token cache/comments/docs away from stale
  signer/signature terminology, including the stale design reservation for
  issuer signer generation rotation. Current validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-core ops::auth --lib -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.27` is committed as the internal auth proof naming cleanup. It
  renames cache-hit delegated-token verification helpers, full embedded-proof
  verification helpers, issuer token proof preparation locals, and
  role-attestation hash domain constants away from stale signing/signature
  names. It also updates root startup errors and delegated-cert comments from
  signing wording to proof issuance/proof creation wording and removes the stale
  `DelegatedTokenSignerPrewarmPlan` section marker. Current validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-core ops::auth --lib -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.65.28` is committed as the status/design closeout after `0.65.27`. It
  reconciles the 0.65 design status document with the current hard-cut
  state. The status file no longer describes 0.65 as design-correction pending,
  treats verifier purity and zero-ECDSA source shape as manual closeout audits
  rather than permanent source-shape guards, records forwarded
  user-token rejection as covered by delegated-token subject/caller binding,
  marks Candid/endpoint/docs reconciliation complete, and leaves only final
  release validation plus maintainer-owned release-note preparation as
  closeout work.
- `0.65.29` is committed as the generated-surface closeout after `0.65.28`. It
  refreshes the checked-in wasm-store Candid sidecar against the current
  zero-ECDSA auth DTOs and endpoints. The sidecar no longer exposes
  `SubnetAuthStateInput.delegated_root_public_key` or
  `SubnetRootPublicKeyInput`, and it now includes delegated-token
  prepare/get DTOs and methods plus the structural-only capability proof
  shape. The slice also restores the `Cargo.lock` `time` dependency line to
  `0.3.41`, because the newer `0.3.48` line fails to compile with
  `ic-agent 0.47.3`.
- `0.65.30` is committed as the changelog catch-up after `0.65.29`. It adds
  root and detailed 0.65 changelog entries for `.28` and `.29`, then
  normalizes current release-note wording away from the old source-shape guard
  phrase. Follow-up focused validation passed for core/facade compilation, auth
  unit tests, protocol-surface tests, formatting, changelog governance, and
  active-source scans for removed ECDSA/shard-token/delegated-root-key/proof
  surfaces.
  Current validation:
  ```text
  cargo fmt --all -- --check
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic-core ops::auth --lib -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  rg -n 'shard_sig|sign_with_ecdsa|EcdsaOps::sign_bytes|auth-threshold-ecdsa-sign|auth-threshold-ecdsa-public-key|auth-shard-secp256k1-verify|ThresholdEcdsaSign|IcThresholdEcdsaSecp256k1|EcdsaP256Sha256|shard_public_key_sec1|key_name_hash|derivation_path_hash|RootPublicKeyRecord|RootTrustAnchor|DelegationProof.*root_sig|sign_prepared_delegation_proof|DelegatedTokenMintRequest|mint_token|request_internal_invocation_proof|InternalInvocationProof|SignedDelegatedGrant|DelegatedGrantProof' crates canisters fleets scripts Cargo.toml Makefile -g '*.rs' -g '*.toml' -g '*.did' -g '*.sh' -g 'Cargo.toml' -g 'Makefile'
  rg -n 'delegated_root_public_key|SubnetRootPublicKeyInput|RootPublicKeyRecord' crates canisters fleets scripts Cargo.toml Makefile -g '*.rs' -g '*.toml' -g '*.did' -g '*.sh' -g 'Cargo.toml' -g 'Makefile'
  git diff --check
  ```
- Local `0.65.31` handoff/status refresh after committed `0.65.30` updates the
  top `docs/status/current.md` current-line summary from the early
  root-proof-only hard-cut state to the actual zero-management-ECDSA
  normal-auth state: canister-signature root proofs, issuer proofs, and role
  attestations; self-contained reusable delegated tokens with signed opaque
  `ext`; issuer `ActiveDelegationProof`; local protected verification; removed
  shard/threshold-ECDSA/one-shot/internal-proof/delegated-root-key surfaces; and
  release validation as the remaining closeout. It also narrows the 0.65 design
  status remaining work to final validation plus maintainer-owned release
  preparation, restores the `Cargo.lock` `time` dependency line to `0.3.41`
  after broader test compilation exposed drift back to `0.3.48`, and validates
  `cargo check --locked -p canic-tests`.
- `0.65.15` is committed and removes the active shard ECDSA key/signature
  authority fields from delegated-token `DelegationCert`. Certs now bind
  issuer canister-signature authority instead of shard ECDSA key material.
  `DelegatedTokenClaims` and verifier output use `issuer_pid`, root proof
  preparation no longer fetches a
  threshold-ECDSA public key, the `auth-delegated-token-verify` feature now
  pulls root plus issuer canister-signature verification, test issuers use
  `auth-issuer-canister-sig-create`, runtime startup checks require issuer
  canister-signature creation for delegated-token issuers, and the checked-in
  wasm-store Candid surface reflects the issuer-bound cert shape. Remaining
  ECDSA scan matches are isolated to the standalone ECDSA ops feature
  definitions, historical replay effect tests/records, and legacy
  attestation-key helpers used by keyed internal-invocation proofs. Current
  validation:
  ```text
  cargo check --locked -p canic-core -p canic
  cargo check --locked -p canic-core --features auth-root-canister-sig-create,auth-issuer-canister-sig-create,auth-delegated-token-verify
  cargo check --locked -p delegation_root_stub -p delegation_signer_stub -p canister_root -p canister_user_shard
  cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
  cargo test --locked -p canic-core access::auth::token --lib -- --nocapture
  ```
- `0.65.14` is committed and flips delegated tokens from shard ECDSA signatures
  to issuer canister-signature proofs. `DelegatedToken` now carries
  `issuer_proof`, runtime verification validates the issuer proof over the
  canonical claims hash, and the positive verifier cache key binds
  `issuer_proof_hash`. The non-root auth bundle exposes
  `canic_prepare_delegated_token` update plus `canic_get_delegated_token`
  query; prepare uses the installed `ActiveDelegationProof`, enforces
  `subject == msg.caller()`, is replay-protected under
  `auth.prepare_delegated_token.v1`, and stores caller-bound pending token
  metadata for query retrieval. `AuthApi::issue_token` and
  `DelegatedTokenIssueRequest` are removed, test fleet one-shot issue wrappers
  are removed, and the PIC helper now installs active proof material before
  prepare/get. The old shard-token replay entries plus stale shard/threshold
  ECDSA signing cost classes are removed from normal auth. Current validation:
  ```text
  cargo fmt --all -- --check
  cargo check --locked -p canic-core -p canic
  cargo check --locked -p canic-testing-internal
  cargo check --locked -p canister_user_shard
  cargo check --locked -p delegation_signer_stub
  cargo test --locked -p canic-core ops::auth --lib -- --nocapture
  cargo test --locked -p canic-core access::auth::token --lib -- --nocapture
  cargo test --locked -p canic-core api::auth --lib -- --nocapture
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo test --locked -p canic-core ops::cost_guard --lib -- --nocapture
  cargo test --locked -p canic --test endpoint_macro -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo check --locked -p canic-core --features auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify
  cargo check --locked -p canic --features auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify
  cargo check --locked -p canic-core --features auth-threshold-ecdsa-sign
  cargo clippy --locked -p canic-core --lib -- -D warnings
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
  Attempted targeted PocketIC validation:
  ```text
  TMPDIR="$(pwd)/.tmp/test-runtime" ICP_ENVIRONMENT=local cargo test --locked -p canic-tests --test root_suite delegated_token_verification_uses_self_contained_root_proof -- --test-threads=1 --nocapture
  ```
  That run rebuilt local wasm artifacts successfully, then hit the known
  `Failed to bind PocketIC server to address 127.0.0.1:0` infrastructure panic
  and was interrupted after only a defunct `pocket-ic` child plus idle
  `root_suite` parent remained.
- Local `0.64.3` closeout candidate after pushed `0.64.2` finishes the 0.64
  topology line with no required deferred implementation work. The 0.64 design
  note is marked implemented/closed, old open questions are recorded as closed
  0.64 decisions, and optional future ideas are labeled as future feature scope
  rather than 0.64 debt. Root index builder regression coverage now locks down
  stale direct-root singleton residue: AppIndex and SubnetIndex continue to
  derive entries from configured service-filtered role sets, so a leftover
  direct-root child role that is no longer index-eligible is excluded without
  producing a duplicate-root-service failure. The root and detailed `0.64.3`
  changelogs are drafted.
  Validation:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic-core config::schema --lib -- --nocapture
  cargo test --locked -p canic-core ops::topology::index --lib -- --nocapture
  cargo test --locked -p canic-core registry_policy_seam --lib -- --nocapture
  cargo test --locked -p canic-core index_addressing_seam --lib -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.64.2` is pushed as the index import role-bound hardening. AppIndex and
  SubnetIndex imports now reject roles outside the configured explicit
  AppIndex/service-derived SubnetIndex sets, including partial snapshots.
  Root builders still derive index entries from direct root service records,
  and builder-generated partial imports use an explicit trusted internal path.
  Validation:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic-core index_addressing_seam --lib -- --nocapture
  cargo test --locked -p canic-core index --lib -- --nocapture
  cargo test --locked -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo check --locked -p canic-core -p canic
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.64.1` is pushed as the service manager runtime-policy hardening. Runtime
  registration policy now requires service parents for directory, scaling, and
  sharding manager pools; singleton child creation remains parent-scoped and
  valid. Public Candid error-code names for the existing manager-parent policy
  errors remain compatibility labels, while internal policy variants and
  messages use service terminology. Detailed notes live in
  `docs/changelog/0.64.md`.
- `0.64.0` is pushed as the service/singleton topology split. The config model
  now has explicit `kind = "service"` for root-scoped, root-created canisters;
  root bootstrap, SubnetIndex, and current AppIndex validation are
  service-driven; singleton remains parent-scoped child cardinality for
  downstream app shapes such as `project_instance -> project_ledger`. Active
  checked-in fleet configs, scaffolds, install/status/build fixtures, and the
  packaged downstream CLI probe use `kind = "service"` for root-created roles.
  Detailed notes live in `docs/changelog/0.64.md`; the design note is
  `docs/design/0.64-service-singleton-topology/0.64-design.md`.
  Validation:
  ```text
  cargo test --locked -p canic-core config::schema --lib -- --nocapture
  cargo test --locked -p canic-core ops::topology::index --lib -- --nocapture
  cargo test --locked -p canic-core registry_policy_seam --lib -- --nocapture
  cargo test --locked -p canic-cli scaffold --lib -- --nocapture
  cargo test --locked -p canic-cli status --lib -- --nocapture
  cargo test --locked -p canic-cli build --lib -- --nocapture
  cargo test --locked -p canic build_support::config --lib -- --nocapture
  cargo test --locked -p canic-host adoption --lib -- --nocapture
  cargo test --locked -p canic-host icp_config --lib -- --nocapture
  cargo test --locked -p canic-host build_provenance --lib -- --nocapture
  cargo test --locked -p canic-host deployment_truth --lib -- --nocapture
  cargo test --locked -p canic-host install_root --lib -- --nocapture
  cargo test --locked -p canic-host release_set --lib -- --nocapture
  cargo check --locked -p canic-core -p canic-host -p canic-cli -p canic
  scripts/ci/verify-packaged-downstream-cli.sh
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.63.3` changelog is finalized for the batch where the root topology test
  helper treats `canic_memory_ledger` as absent from the default root bundle,
  matching the `diagnostics.memory_ledger = true` opt-in contract from
  `0.63.1`. Local maintainer CI debugging now has an optional GitHub CLI helper
  under `scripts/dev/`, and the GitHub Actions workflow uses Node
  24-compatible first-party action majors (`checkout`/`setup-node` `v6`,
  `cache` `v5`) after recent CI annotations warned about Node 20 action runtime
  deprecation.
  Validation:
  ```text
  POCKET_IC_BIN=/tmp/pocket-ic-server-14.0.0/pocket-ic cargo test -p canic-tests --test root_suite root_cases::hierarchy::root_reference_topology_is_consistent -- --test-threads=1 --nocapture
  actionlint .github/workflows/ci.yml
  bash -n scripts/dev/gh-ci.sh scripts/dev/install_dev.sh scripts/ci/*.sh
  scripts/dev/gh-ci.sh --list --limit 3
  scripts/dev/gh-ci.sh --failed
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- The historical post-46 backlog is no longer an active `0.64` planning line.
  Historical source material stays under `docs/design/archive/post-46-backlog/`;
  optional feature-shaped follow-ups now live under
  `docs/design/ideas/post-46-backlog/`. The deleted 0.64 draft had no concrete
  release need beyond backlog pressure. Already-covered backlog items remain
  credited to 0.50 passive adoption, 0.51 evidence envelopes, 0.52 build
  provenance, 0.53 policy gates/project evidence manifests, and 0.54 passive
  deployment catalog. Future work should require a named feature need, owner,
  and release scope before it leaves `ideas/`.
  Validation:
  ```text
  stale-reference scan for the deleted 0.64 path and active-0.64 wording
  directory absence check for the deleted 0.64 backlog design
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.63.2` adds joined-topology coverage metrics to
  `canic nns topology summary`. The summary now reports whether cached nodes
  resolve to known node-provider, node-operator, and data-center rows, and
  whether cached node operators resolve to known node-provider and data-center
  rows. Text output includes a `join_coverage` table; JSON summary output uses
  topology summary report schema version `2` with explicit known/unknown
  counters. Local setup and CI now read external dev-tool pins from root
  `tool-versions.env`; the ICP CLI installer installs the pinned version,
  verifies that resolved `icp --version` matches it, honors `CARGO_HOME`, and
  emits a non-fatal warning when GitHub's latest `dfinity/icp-cli` release tag
  differs from the pin. NNS component cache schemas, runtime canisters, Candid,
  Cargo package versions, dependencies, and lockfiles are unchanged.
  Validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-host nns_topology --lib -- --nocapture
  cargo test --locked -p canic-host nns --lib -- --nocapture
  cargo test --locked -p canic-cli --lib nns -- --nocapture
  cargo fmt --all -- --check
  cargo run --locked -p canic-cli -- nns topology summary
  cargo run --locked -p canic-cli -- nns topology summary --format json
  cargo check --locked -p canic-cli
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  bash -n scripts/dev/install_dev.sh scripts/ci/install-actionlint.sh scripts/ci/install-icp-cli.sh scripts/ci/require_icp.sh tool-versions.env
  make -n install-dev update-dev
  bash scripts/ci/install-icp-cli.sh
  ACTIONLINT_INSTALL_DIR=/tmp/canic-actionlint-test bash scripts/ci/install-actionlint.sh
  actionlint .github/workflows/ci.yml
  bash -c 'source scripts/ci/require_icp.sh; require_icp_tools'
  git diff --check
  ```
- `0.63.1` adds `canic nns topology refresh` as the one-shot refresh path for
  every cached mainnet NNS component consumed by `canic nns topology summary`
  and makes the controller-only `canic_memory_ledger` recovery diagnostic
  opt-in per canister role through `diagnostics.memory_ledger = true`. The NNS
  command refreshes subnet, node, node-provider, node-operator, and data-center
  reports with shared `--source-endpoint`, `--dry-run`, `--lock-stale-after`,
  and `--format json` support, returning an aggregate per-component refresh
  report with cache path, registry version, fetched timestamp, item count,
  write status, and replacement status. The shared runtime and wasm-store
  endpoint bundles now emit `canic_memory_ledger` only when the build config
  enables `canic_memory_ledger_enabled`, and the checked-in default
  `crates/canic-wasm-store/wasm_store.did` no longer exposes the memory-ledger
  DTOs or method. This is a CLI/host operator workflow expansion plus an
  intentional default Candid/runtime surface reduction; no Cargo package
  versions, release scripts, dependencies, or lockfiles change.
  Validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-host nns --lib -- --nocapture
  cargo test --locked -p canic-cli --lib nns -- --nocapture
  cargo test --locked -p canic-core diagnostics_memory_ledger --lib -- --nocapture
  cargo test --locked -p canic --test protocol_surface -- --nocapture
  cargo fmt --all -- --check
  cargo check --locked -p canic --lib
  cargo check --locked -p canic-cli
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo run --locked -p canic-cli -- nns topology refresh --dry-run
  bash scripts/ci/wasm-audit-report.sh
  git diff --check
  ```
- `0.63.0` starts the post-0.62 NNS topology expansion without changing Cargo
  package versions, release scripts, Candid, runtime canisters, package
  artifacts, or lockfiles. The CLI now exposes `canic nns topology summary`,
  which composes existing cached NNS subnet, node, node-provider,
  node-operator, and data-center reports into aggregate counts, subnet-kind
  distribution, and per-source registry-version provenance. `canic nns node
  list` also gains filter flags for subnet/subnet-prefix, subnet kind,
  data-center/data-center-prefix, node-provider/node-provider-prefix, and
  node-operator/node-operator-prefix while preserving the existing node cache
  schema. The root README and CLI README advertise the new operator-facing
  surface, and `CHANGELOG.md` plus `docs/changelog/0.63.md` record the
  versioned `0.63.0` changelog entry.
  Validation:
  ```text
  cargo fmt --all
  cargo test --locked -p canic-host nns --lib -- --nocapture
  cargo test --locked -p canic-cli --lib nns -- --nocapture
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo fmt --all -- --check
  cargo check --locked -p canic-cli
  cargo run --locked -p canic-cli -- nns topology summary
  cargo run --locked -p canic-cli -- nns node list --data-center zh2 --kind application
  git diff --check
  ```
- `0.62.6` adds the non-versioned RC readiness audit at
  `docs/operations/rc-readiness-audit.md` plus CI guard
  `scripts/ci/check-rc-readiness-audit.sh`. The audit records
  `READY TO CLOSE 0.62 IMPLEMENTATION WORK`, marks the 0.62 design slice
  record historical, and separates remaining package/install, broad workspace,
  local ICP/canister, tag, and final release gates into RC/full validation
  rather than additional implementation slicing. This is docs/CI-only work: no
  runtime behavior, Candid, CLI output, JSON/output format, dependency,
  lockfile, fixture, snapshot, generated output, package artifact, release
  version, tag, or publish operation changes are introduced. Validation:
  ```text
  actionlint
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  bash scripts/ci/check-recovery-runbooks.sh
  bash scripts/ci/check-diagnostic-consistency-audit.sh
  bash scripts/ci/check-release-package-install-validation.sh
  bash scripts/ci/check-rc-readiness-audit.sh
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
  cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
  git diff --check
  ```
- `0.62.5` adds the non-versioned release package/install validation checklist
  at `docs/operations/release-package-install-validation.md` plus CI guard
  `scripts/ci/check-release-package-install-validation.sh`. The checklist
  classifies existing package, installed CLI, packaged downstream CLI,
  packaged downstream `wasm_store`, release build, local fleet install, and
  local canister validation gates, and records artifact verification and
  human-owned release-flow boundaries. This is docs/CI-only work: no runtime
  behavior, Candid, CLI output, JSON/output format, dependency, lockfile,
  fixture, snapshot, generated output, package artifact, or release package
  changes are introduced. Validation:
  ```text
  actionlint
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  bash scripts/ci/check-recovery-runbooks.sh
  bash scripts/ci/check-diagnostic-consistency-audit.sh
  bash scripts/ci/check-release-package-install-validation.sh
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.62.4` adds the non-versioned diagnostic consistency audit at
  `docs/operations/diagnostic-consistency-audit.md` plus CI guard
  `scripts/ci/check-diagnostic-consistency-audit.sh`. The audit classifies
  existing public errors, internal runtime logs, metrics, tests, and docs for
  replay-sensitive failure classes including duplicate replay, missing or
  invalid operation IDs, expiration, caller/issuer mismatch, delegation-proof
  replay, delegated-token replay, pending operations, recovery-required state,
  cost-boundary refusal, permit-boundary refusal, and durable-publication
  ambiguity. This is docs/CI-only work: no runtime behavior, Candid, CLI
  output, JSON/output format, dependency, lockfile, fixture, snapshot,
  generated output, package artifact, or release package changes are
  introduced. Validation:
  ```text
  actionlint
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  bash scripts/ci/check-recovery-runbooks.sh
  bash scripts/ci/check-diagnostic-consistency-audit.sh
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.62.3` adds the non-versioned recovery/retry runbooks at
  `docs/operations/recovery-retry-runbooks.md` plus CI guard
  `scripts/ci/check-recovery-runbooks.sh`. The runbooks document safe operator
  recovery decisions for replay-sensitive failures and uncertain operations,
  including same-input retries, committed replay, in-progress operations,
  payload/caller mismatches, expired authorization, delegation caller/issuer
  mismatch, project-local pending ICP refill, recovery-required refill,
  cost-boundary refusal, durable-publication ambiguity, upgrade interruption,
  and unexpected receipt state. This is docs/CI-only work: no runtime behavior,
  Candid, CLI output, JSON/output format, dependency, lockfile, fixture,
  snapshot, generated output, package artifact, or release package changes are
  introduced. Validation:
  ```text
  actionlint
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  bash scripts/ci/check-recovery-runbooks.sh
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.62.2` adds the non-versioned upgrade/state compatibility audit at
  `docs/operations/upgrade-state-compatibility-audit.md` plus CI guard
  `scripts/ci/check-upgrade-state-audit.sh`. The audit classifies
  replay-sensitive state areas including replay receipts, operation IDs,
  pending operation logs, delegated-auth hard-cut state, caller/issuer binding,
  delegated-token prepare replay, ICP refill replay, cost-guard accounting,
  upgrade request replay, lifecycle post-upgrade ordering, durable-publication
  state, and stable-memory ABI ownership. No release blocker was found in this
  audit. This is docs/CI-only work: no runtime behavior, Candid, CLI output,
  JSON/output format, dependency, lockfile, fixture, snapshot, generated output,
  package artifact, or release package changes are introduced. Validation:
  ```text
  actionlint
  bash scripts/ci/check-release-validation-matrix.sh
  bash scripts/ci/check-upgrade-state-audit.sh
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture
  cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
  git diff --check
  ```
- `0.62.1` adds the non-versioned release-validation matrix at
  `docs/operations/release-validation-matrix.md`. The matrix separates slice
  close-out, implementation close-out, RC promotion, and final release/tag
  validation; it also classifies required local/CI gates, focused
  replay/auth/cost gates, governance checks, package/install probes, broad
  workspace checks, and environment-specific local ICP/canister checks. The
  matrix is linked from `docs/governance/ci-deployment.md`, the operations docs
  index, and CI guard `scripts/ci/check-release-validation-matrix.sh` so it is
  active release-validation infrastructure, not an archive design note. This is
  docs/CI-only matrix work: no runtime behavior, Candid, CLI output,
  JSON/output format, dependency, lockfile, fixture, snapshot, generated output,
  package artifact, or release package changes are introduced. Validation:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  bash scripts/ci/check-release-validation-matrix.sh
  git diff --check
  ```
- `0.62.0` starts the bounded post-0.61 release-durability line with a
  docs-only charter/reconciliation slice. The new design is
  `docs/design/0.62-release-durability/0.62-design.md`, and the line is scoped
  to release validation, upgrade confidence, operator recovery, validation
  governance, targeted tests, and minimal diagnostics. This slice also
  reconciles stale tracked 0.62 changelog content that described the old
  "Broad NNS inspection foundation" identity; that NNS registry-version work is
  already part of the 0.61 history and remains recorded under `0.61.3`. No
  runtime behavior, Candid, CLI output, JSON/output format, dependency,
  lockfile, fixture, snapshot, generated output, package artifact, or release
  package changes are introduced. Validation:
  ```text
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.40` fixed control-plane compile failures caused by the
  permit-required lifecycle create boundary and cleaned up 0.61
  release-readiness wording. Bootstrap auto-create, bootstrap wasm-store
  creation, and runtime wasm-store publication creation now reserve, complete,
  or recover a management-deployment `CostGuardPermit` before calling
  `CanisterLifecycleEvent::Create`. The replay-protection design now labels
  the branch-slice plan as a historical implementation record and directs
  current readiness decisions to the acceptance criteria plus executable
  replay-policy, hard-cut, and cost-guard gates. No CLI commands, flags, output
  columns, JSON report shapes, dependencies, or lockfiles changed. Validation:
  ```text
  cargo check --locked -p canic-control-plane --all-targets
  cargo test --locked -p canic-control-plane --all-targets -- --nocapture
  cargo clippy --locked -p canic-control-plane --all-targets --all-features -- -D warnings
  cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
  cargo test --locked -p canic-core replay_policy --lib -- --nocapture
  cargo clippy --locked -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test --locked -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.39` added an aggregate release-candidate replay manifest gate.
  `release_candidate_manifests_have_no_release_blockers` now fails if the
  endpoint manifest, root capability command manifest, or pool admin command
  manifest contains any `ReleaseBlocker` entry, and reports blockers with their
  manifest scope. The 0.61 design release-candidate section now points at that
  executable gate. This is manifest/test/docs-only; no runtime paths, CLI
  commands, flags, output columns, or JSON report shapes changed. Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.38` added a durable-publication replay-policy regression guard. The
  test derives the expected durable-publish endpoint set from protected
  wasm-store update methods plus root template publication admin methods, then
  proves each entry is implemented, monotonic, `DurablePublish`, and carries
  durable-publish quota/reserve metadata. It also fails if unrelated endpoints
  drift into the durable-publish cost class. This is manifest-only; no runtime
  paths, CLI commands, flags, output columns, or JSON report shapes changed.
  Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.37` put actual canister upgrade installs behind a management-deployment
  `CostGuardPermit`. `CanisterLifecycleEvent::Upgrade` now carries explicit
  cost context, lifecycle upgrade reserves deployment quota/cycles only after
  the module-hash plan says an upgrade is needed, and the install workflow no
  longer exposes an unpermitted lifecycle install helper. The reserve boundary
  logs command kind, quota subject, payer, and target canister without logging
  module bytes or payloads. Already-current upgrades still skip before quota or
  cycle reservation. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::canister_lifecycle --lib -- --nocapture
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core --test cost_guard_boundary_guard -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.36` threaded the root provision deployment `CostGuardPermit` through
  lifecycle creation. `CanisterLifecycleEvent::Create` now carries the reserved
  permit, provisioning allocation uses permit-required wrappers for pool
  top-up and fresh canister creation, and initial canister install uses
  permit-required management install wrappers. The cost-guard boundary guard now
  also rejects unpermitted provisioning workflow management calls. No CLI
  commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core --test cost_guard_boundary_guard -- --nocapture
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core workflow::ic::provision --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.35` tightened the threshold-ECDSA signing cost-guard boundary.
  `EcdsaOps::sign_bytes` now requires a `CostGuardPermit` in both
  `auth-crypto` and non-`auth-crypto` builds, and prepared auth signing wrappers
  pass their existing permits through to the lower signing adapter. A new
  `canic-core` source guard pins private `CostGuardPermit` construction,
  prepared-auth-only ECDSA signing calls, and permit-required ICP refill
  value-transfer adapters. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core --test cost_guard_boundary_guard -- --nocapture
  cargo test -p canic-core ops::auth --lib -- --nocapture
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.34` tightened the ICP refill value-transfer cost-guard boundary.
  `IcpRefillOps::icrc1_transfer` and `IcpRefillOps::notify_top_up` now require
  a `CostGuardPermit`, and the refill workflow requires the reserved
  value-transfer permit before marking or executing ledger transfer and CMC
  notify external effects. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::ic::icp_refill --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.33` added shared pending replay receipt quotas at
  `reserve_or_replay_receipt`. Fresh shared receipts now reject with
  `ResourceExhausted` when the actor already has 64 pending receipts or the
  command kind already has 512 pending receipts. Pending quota counts
  non-expired `Reserved`, `ExternalEffectInFlight`, and `RecoveryRequired`
  receipts; expired, committed, and terminal-failed receipts do not count.
  Existing committed replay receipts still return their cached response before
  current pending quota checks. No CLI commands changed in this patch.
  Validation:
  ```text
  cargo test -p canic-core ops::replay::receipt --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core workflow::ic::icp_refill --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.32` added write-before-send pending operation logging for generated
  manual ICP refill operation IDs. Live `canic cycles convert` canister mode
  now writes generated IDs before dispatch to:
  ```text
  .canic/operations/pending.json
  ```
  Matching `pending_send` records are reused for the same generated-ID command
  after a CLI crash or uncertain send; successful CLI return marks the entry
  `completed`, and failures leave it pending. Non-JSON output reports
  `operation_id_source=pending_log` when a local pending record supplies the
  operation ID. Provided `--operation-id <hex64>` values bypass the pending log.
  No CLI commands, flags, or JSON report shapes changed in this patch.
  Validation:
  ```text
  cargo test -p canic-cli cycles::convert --lib -- --nocapture
  cargo clippy -p canic-cli --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.31` made CLI-generated ICP-refill operation IDs visible in non-JSON
  `canic cycles convert` canister-mode output. The CLI now records whether the
  `operation_id` was supplied or generated; JSON output remains unchanged, but
  non-JSON dry-runs and live calls print the generated client ID before the
  endpoint call:
  ```text
  operation_id=<hex64>
  operation_id_source=generated
  ```
  Supplying `--operation-id <hex64>` keeps the generated-ID notice suppressed.
  The same slice removes the global `used_underscore_binding` Clippy allow
  while keeping `missing_panics_doc` allowed, then cleans macro-visible
  delegated-token/internal-call arguments in the test canister stubs. No CLI
  commands, flags, or JSON report shapes changed in this patch.
  Validation:
  ```text
  cargo test -p canic-cli cycles::convert --lib -- --nocapture
  cargo clippy -p delegation_signer_stub -p project_hub_stub -p project_instance_stub -p runtime_probe --all-targets --all-features -- -D warnings
  cargo clippy -p canic-cli --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.30` normalized the hard-cut missing-operation-ID boundary from
  `docs/design/0.61-replay-protection/0.61-design.md`. Public errors now expose
  `ErrorCode::OperationIdRequired` with message
  `operation_id is required for this command`. Delegation-proof replay
  metadata, delegated-token issue/mint replay metadata, and pool `CreateEmpty`
  replay metadata return that code when replay metadata is absent. Root
  capability `MissingReplayMetadata` now maps to the same public code, covering
  `RequestCycles` replay preflight. Zero or oversized replay TTL values remain
  `InvalidInput`, and replay conflicts remain `Conflict`. No CLI commands
  changed in this patch. Validation:
  ```text
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core workflow::rpc --lib -- --nocapture
  ```
- `0.61.29` added stable replay receipt upgrade-shape coverage from
  `docs/design/0.61-replay-protection/0.61-design.md`. Stable replay record
  tests now prove committed receipts preserve status, response schema, response
  bytes, and external-effect data through CBOR round-trip; pending `Reserved`
  and `RecoveryRequired(ExternalEffectStatusUnknown)` receipts preserve status
  and effect metadata; and unsupported replay receipt schema versions return a
  controlled decode error instead of being accepted. No CLI commands changed in
  this patch. Validation:
  ```text
  cargo test -p canic-core storage::stable::replay --lib -- --nocapture
  ```
- `0.61.28` added executable delegated-auth hard-cut source guard coverage from
  `docs/design/0.61-replay-protection/0.61-design.md`. The new
  `canic-core` integration test scans live runtime source under
  `crates/canic-core/src` and fails if removed verifier-local token-use replay
  APIs, records, capacity constants, stable `auth/token_uses.rs`, or
  `delegated_token_uses` storage fields reappear. Historical docs and audit
  reports are intentionally outside the scan. No CLI commands changed in this
  patch. Validation:
  ```text
  ```
- `0.61.27` added delegated-token mint replay decision coverage from
  `docs/design/0.61-replay-protection/0.61-design.md`. Auth API tests now prove
  committed `auth.mint_token.v1` receipts return cached `DelegatedToken`
  responses, same-operation actor and payload mismatches reject, in-progress
  duplicate mint receipts block with no external effect recorded, and the
  token-signing cost guard can reject a fresh signing operation before any ECDSA
  signing adapter is reachable. `AuthApi` now builds token-signing
  `CostGuardRequest` values through one helper shared by live code and tests.
  No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core api::auth --lib -- --nocapture
  ```
- `0.61.26` closed the delegated-token mint wrapper manifest gap from
  `docs/design/0.61-replay-protection/0.61-design.md`.
  `signer_issue_token` and `user_shard_issue_token` are now explicit
  `ENDPOINT_REPLAY_POLICY_MANIFEST` entries with implemented
  `ReplayProtected(auth.mint_token.v1)` policy, `CostClass::ThresholdEcdsaSign`,
  signing quota, and signing cycle-reserve metadata. A new replay-policy
  regression test scans `canisters/` and `fleets/` Rust sources for
  `#[canic_update]` functions that call `AuthApi::mint_token` and fails if a
  wrapper is missing from the manifest. This is manifest/test coverage for the
  replay/cost-guarded mint path landed in `0.61.25`. No CLI commands changed in
  this patch. Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  ```
- `0.61.25` started delegated-token mint replay hardening. Public
  `DelegatedTokenIssueRequest` and `DelegatedTokenMintRequest` handling now
  requires caller-provided replay metadata, reserves shared receipts with
  command kinds `auth.issue_token.v1` and `auth.mint_token.v1`, hashes the
  authoritative proof/token payload without metadata, and returns committed
  `DelegatedToken` responses for duplicate replays. Shard token signing now
  crosses a prepared-token boundary: fresh execution reserves a
  `CostGuardPermit`, marks `ThresholdEcdsaSign(DelegatedToken)` before ECDSA,
  and signs through `sign_prepared_delegated_token`. The mint path reserves one
  outer token receipt, requests the root proof with the same operation
  metadata, and commits the final shard token response under the mint command.
  The live mint/issue path now emits `Topic::Auth` logs for replay reservation,
  committed replay return, blocked replay decisions, signing cost guard
  reservation, ECDSA effect marking, recovery-required signing failure, and
  final response commit without logging token, proof, signature, or receipt
  response bytes. Active auth contract, recurring audit docs, and the 0.61
  design now describe delegated-token verification as TTL-bounded bearer-token
  verification, with replay-sensitive mutations assigned to domain operation
  receipts. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo test -p canic-core ops::auth --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo test -p canic-testing-internal --lib -- --nocapture
  cargo check -p canister_user_shard -p delegation_signer_stub -p delegation_root_stub -p sharding_root_stub
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.24` graduated root capability `ProvisionCanister` from command-level
  blocker to implemented replay-protected management-deployment behavior.
  `ProvisionCanister` execution now resolves the requested parent, checks that
  the parent is registered, reads the configured initial-cycle target, reserves
  a `CostGuardPermit` with `CostClass::ManagementDeployment`, and marks
  `ExternalEffectDescriptor::ManagementCreateCanister` before lifecycle
  create/install work can allocate from the pool, create a canister, top up a
  pool allocation, change controllers, install code, write registry state, or
  propagate topology. The guard uses command kind `root.provision.v1`, the
  requesting caller as quota subject, the root canister as payer, a 60-second
  quota window, max 10 operations per window, the configured initial-cycle
  amount as the cycle reservation, and a 1 TC minimum remaining cycle balance.
  Post-boundary provisioning failures recover the in-flight cycle reservation
  and preserve the replay receipt as
  `RecoveryRequired(ExternalEffectStatusUnknown)`. Successful provisioning
  completes the deployment guard and returns the new canister principal through
  the existing root replay commit flow. `ProvisionCanister` and
  `canic_response_capability_v1` are now marked implemented in the replay
  manifests; there are no remaining endpoint release blockers. No CLI commands
  changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  ```
- `0.61.23` graduated root capability `RequestCycles` from command-level
  blocker to implemented replay-protected value-transfer behavior.
  `RequestCycles` execution now reserves a `CostGuardPermit` after
  authorization and before the `deposit_cycles` management await. The guard
  uses command kind `root.request_cycles.v1`, the requesting child as quota
  subject, the root canister as payer, a 60-second quota window, max 60
  operations per window, the approved transfer amount as the cycle reservation,
  and a 1_000_000_000 minimum remaining cycle balance. The transfer path now
  marks `ExternalEffectDescriptor::ManagementCall { method: "deposit_cycles" }`
  before the await; infrastructure errors recover the guard and preserve the
  replay receipt as `RecoveryRequired(ExternalEffectStatusUnknown)`.
  Successful transfers record the funding ledger, complete the guard, and
  commit the cached cycles response through the existing replay flow.
  `ProvisionCanister` is now the only remaining root capability command
  blocker, so `canic_response_capability_v1` remains blocked only through that
  command. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  ```
- `0.61.22` split the remaining root capability RPC blocker into command-level
  replay policy. `canic_response_capability_v1` is now represented as
  `CommandDispatch(root.capability_rpc.v1,
  root.capability.command_manifest.v1)` and remains an endpoint release
  blocker while the command manifest has blockers. The new
  `ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST` covers every
  `RootCapabilityCommand` variant. `UpgradeCanister`, `RecycleCanister`,
  `IssueRoleAttestation`, and `IssueInternalInvocationProof` are implemented
  replay-protected commands. `ProvisionCanister` remains blocked until root
  provisioning records a management-deployment cost barrier and
  external-effect/recovery boundary before create/install work; `RequestCycles`
  remains blocked until root cycles funding records a value-transfer cost
  barrier and external-effect/recovery boundary before cycles transfer. No CLI
  commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  ```
- `0.61.21` graduated ICP refill from release blocker to implemented
  replay-protected value-transfer behavior. Fresh manual refill execution now
  reserves a `CostGuardPermit` with `CostClass::ValueTransfer` before the
  first ledger transfer or CMC notify external-effect boundary for a replay
  attempt. The guard uses command kind `icp.refill.v1`, the replay actor as
  quota subject, the current canister as payer, a 60-second quota window, max
  60 operations per window, and a 1_000_000_000 cycle reservation/minimum.
  Terminal committed refill responses complete the guard; resumable,
  recovery-required, and response-commit-failed outcomes recover it. The
  endpoint replay manifest now records `canic_icp_refill` as implemented with
  value-transfer quota/reserve policy, leaving only
  `canic_response_capability_v1` as an endpoint release blocker. No CLI
  commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::ic::icp_refill --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  ```
- `0.61.20` continued the ICP refill shared replay-core migration from
  `docs/design/0.61-replay-protection/0.61-design.md`. Fresh manual refill
  execution now carries the shared `ReplayReceiptToken` through record
  advancement, ledger transfer, and CMC notify. Ledger transfer marks
  `ExternalEffectDescriptor::IcpTransfer` before `icrc1_transfer`; CMC
  `notify_top_up` marks `ExternalEffectDescriptor::ManagementCall` before the
  notify await. Transport or infrastructure failures after either marked
  external-effect boundary preserve the receipt as
  `RecoveryRequired(ExternalEffectStatusUnknown)`. Known retryable
  application-level ledger/CMC outcomes still leave the refill business record
  resumable and discard the temporary uncommitted receipt. Canic runtime logs
  under `Topic::Cycles` now cover refill replay reservation, committed replay
  returns, replay conflicts, effect marking, terminal commits, resumable
  receipt aborts, and recovery-required outcomes. `canic_icp_refill` remains a
  release blocker until value-transfer quota and reserve enforcement is wired
  into the refill path. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::ic::icp_refill --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  ```
- `0.61.19` continued the ICP refill shared replay-core migration from
  `docs/design/0.61-replay-protection/0.61-design.md`. Fresh manual ICP refill
  execution now reserves a shared replay receipt before creating or advancing
  the refill business record. Terminal refill responses are committed into the
  shared receipt and duplicate committed replays return the cached
  `IcpRefillResponse`; actor mismatch, payload mismatch, in-progress, expired,
  recovery-required, and terminal-failed receipt decisions map to public
  conflict errors. Resumable refill records still abort the temporary shared
  receipt so existing transfer/notify retry behavior is preserved until
  external-effect marking lands. `canic_icp_refill` remains a release blocker
  until ledger transfer and CMC notify effects are marked in flight before the
  external calls and uncertain outcomes become recovery-required receipts. The
  design now also requires Canic runtime logs for refill replay decisions,
  replay conflicts, external-effect marking, terminal commits, resumable
  receipt aborts, retryable outcomes, and recovery-required outcomes. No CLI
  commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::ic::icp_refill --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  ```
- `0.61.18` started the ICP refill shared replay-core migration from
  `docs/design/0.61-replay-protection/0.61-design.md`. ICP refill now has
  shared replay identity helpers for command kind `icp.refill.v1`, conversion
  from `IcpRefillRequest.operation_id` into `OperationId`, direct-caller replay
  actor derivation, and canonical payload hashing through
  `ReplayPayloadHasher`. The manual-refill path now constructs a shared
  `ReplayReceiptReserveInput` from those fields and uses its operation ID bytes
  for the existing refill record lookup/create path. Tests prove the refill
  payload hash excludes `operation_id` while binding the actor, source
  canister, source subaccount, target canister, amount, and refill mode.
  `canic_icp_refill` remains a release blocker until live refill execution
  reserves, marks, and commits shared replay receipts around transfer/notify
  progress. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::ic::icp_refill --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.17` completed the canister-upgrade manifest graduation from
  `docs/design/0.61-replay-protection/0.61-design.md`.
  `ENDPOINT_REPLAY_POLICY_MANIFEST` now records `canic_canister_upgrade` as
  implemented with `ResponseIdempotent(management.canister_upgrade.v1)`, cost
  class `ManagementDeployment`, and deployment quota/reserve policy. The proof
  is the existing upgrade planner: repeated upgrade requests become no-ops once
  the installed module hash matches the approved target hash, while missing or
  different hashes still request an upgrade. `UpgradeCanisterRpc` now has
  focused request-shape coverage proving replay metadata is carried into the
  root request DTO and non-upgrade response variants are rejected. The remaining
  endpoint release blockers are now `canic_icp_refill` and
  `canic_response_capability_v1`. No CLI commands changed in this patch.
  Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo test -p canic-core domain::policy::upgrade --lib -- --nocapture
  cargo test -p canic-core ops::rpc::request --lib -- --nocapture
  ```
- `0.61.16` completed the pool-admin endpoint manifest graduation from
  `docs/design/0.61-replay-protection/0.61-design.md`. The endpoint-level
  `canic_pool_admin` entry is no longer a release blocker. The replay policy
  model now has `CommandDispatch`, and `canic_pool_admin` is recorded as
  `CommandDispatch(pool.admin.v1, pool.admin.command_manifest.v1)` with
  deployment quota/reserve policy. A regression test pins the endpoint-level
  classification, and another test fails if any `PoolAdminCommand` manifest
  entry regresses to `ReleaseBlocker`. The manifest now pins the remaining
  endpoint release blockers to `canic_canister_upgrade`, `canic_icp_refill`,
  and `canic_response_capability_v1`. This is manifest-only; pool runtime
  behavior and CLI output did not change. Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  ```
- `0.61.15` completed the pool `Recycle` replay-proof slice from
  `docs/design/0.61-replay-protection/0.61-design.md`.
  `POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST` now records `Recycle` as
  implemented with `ResponseIdempotent(pool.recycle.ensure_v1)` and deployment
  quota/reserve policy. Recycle now removes the canister from the subnet
  registry and records a metadata-preserving pending-reset pool entry before
  crossing the management reset boundary; duplicate retries stop at an
  existing pending-reset or ready pool entry instead of repeating the reset
  path. Successful recycle preserves the original registry role, parent, and
  module hash in the ready pool entry. Failed immediate reset leaves the
  pending-reset pool entry in place and schedules pool reset recovery. No CLI
  commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  ```
- `0.61.14` completed the pool `ImportImmediate` replay-proof slice from
  `docs/design/0.61-replay-protection/0.61-design.md`.
  `POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST` now records `ImportImmediate` as
  implemented with `ResponseIdempotent(pool.import_immediate.ensure_v1)` and
  deployment quota/reserve policy. The pool workflow now has focused coverage
  proving immediate import detects both ready and pending-reset pool entries
  before the reset path; duplicate retries keep a single pool entry and
  preserve `PendingReset` once the first request has marked the canister for
  reset. Pool `Recycle` remains the only explicit pool admin variant release
  blocker because it can still cross management reset before removing the
  subnet-registry entry. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo fmt --all -- --check
  ```
- `0.61.13` completed the attestation key-set manifest correction and ICP CLI
  0.3 cleanup batch. From
  `docs/design/0.61-replay-protection/0.61-design.md`,
  `canic_attestation_key_set` is now classified as implemented
  `SnapshotConvergent(auth.attestation_key_set.v1)` with cost class `None` and
  no quota or cycle-reserve policy. The endpoint can refresh cached root
  attestation public-key material, but it uses the ECDSA public-key query path
  rather than threshold signing and does not issue proof material. A manifest
  regression test pins that it stays out of the signing quota/reserve bucket.
  `make install-dev`, `make update-dev`, and CI now install `icp` from the
  official `icp-cli` `0.3.0` GitHub release installer under Cargo's bin
  directory instead of installing npm `@icp-sdk/icp-cli` into `$HOME/.local`.
  Local setup and CI share `scripts/ci/install-icp-cli.sh`; the dev installer
  still installs `ic-wasm` through npm and removes the legacy user-local npm
  `icp` wrapper when it points at `@icp-sdk/icp-cli`. Local
  `icp --version` reports `icp 0.3.0`, and help inspection confirmed the
  Canic-used command families still exist for canister
  calls/status/top-up/snapshots, local network lifecycle,
  project/environment reads, cycles, and token wrappers. Canic host command
  contexts that already carry an explicit ICP project root now pass
  `--project-root-override <path>` to ICP CLI 0.3 while preserving the existing
  subprocess `current_dir`. Local replica command construction now honors an
  `IcpCli` environment with `icp network <action> -e <environment>`, while the
  no-target default remains `icp network <action> local`. No Canic CLI
  commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  bash -n scripts/dev/install_dev.sh scripts/ci/require_icp.sh scripts/ci/install-icp-cli.sh
  bash scripts/ci/install-icp-cli.sh 0.3.0
  make -n install-dev update-dev
  actionlint .github/workflows/ci.yml
  icp --version
  icp canister call --help
  icp canister status --help
  icp canister top-up --help
  icp canister snapshot create --help
  icp canister snapshot download --help
  icp canister snapshot upload --help
  icp canister snapshot restore --help
  icp network start --help
  icp network status --help
  icp --project-root-override /home/adam/projects/canic environment list
  icp --project-root-override /home/adam/projects/canic project show
  ic-wasm --version
  which -a icp
  cargo test -p canic-host icp -- --nocapture
  cargo test -p canic-cli --lib icp -- --nocapture
  cargo clippy -p canic-host --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  cargo test -p canic --test changelog_governance -- --nocapture
  git diff --check
  ```
- `0.61.12` completed the canister-status manifest correction slice from
  `docs/design/0.61-replay-protection/0.61-design.md`.
  `canic_canister_status` is now classified as an implemented update-shaped
  read-only endpoint with `QueryOrReadOnly`, cost class `None`, and no quota or
  cycle-reserve policy. The endpoint reads management-canister status but does
  not mutate Canic state and does not perform deployment, signing,
  value-transfer, or publication effects. A manifest regression test pins that
  classification. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.11` completed the pool `ImportQueued` convergence-proof slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. The pool admin command
  manifest now marks `ImportQueued` as implemented with
  `SnapshotConvergent(pool.import_queued.ensure_v1)` and cost class `None`.
  A focused workflow test proves duplicate PIDs in the same queued-import
  request and a repeated request leave exactly one pending-reset pool entry per
  canister: the first call records one add and one skip, while the repeated
  call records all skips. The production queued-import path still performs
  authorization, admissibility checks, metrics, scheduling, and IC timestamps;
  the test exercises the internal authorized state transition with those
  native-test IC calls disabled. Pool `Recycle` and `ImportImmediate` remain
  release blockers because they can still cross management reset effects. No
  CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.10` completed the root auth-material replay recovery slice from
  `docs/design/0.61-replay-protection/0.61-design.md`.
  Role-attestation and internal-invocation proof issuance now split signing
  into prepare/sign phases, mark a `ThresholdEcdsaSign` external-effect
  descriptor on the shared root replay receipt immediately before guarded
  ECDSA signing, recover cycle reservations on signing failure, and preserve
  recovery-required receipts for uncertain post-signing or post-commit
  failures. Generic root replay abort now removes only receipts that are still
  `Reserved`, so receipts already marked in-flight or recovery-required survive
  the execution-error path. The replay policy manifest now marks
  `canic_request_role_attestation` and
  `canic_request_internal_invocation_proof` as implemented. No CLI commands
  changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo test -p canic-core ops::auth --lib -- --nocapture
  cargo test -p canic-core ops::replay --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.9` completed the root auth-material signing cost-guard slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. Root role-attestation
  and internal-invocation proof signing now require `CostGuardPermit`, and
  fresh root auth-material signing reserves `ThresholdEcdsaSign` signing quota
  plus an in-flight cycle budget before threshold ECDSA. Signing failures
  recover the cycle reservation; successful signatures complete the quota and
  reservation. The role-attestation and internal-invocation proof manifest
  entries remain release blockers because generic root capability execution
  still aborts fresh replay on execution error; a later slice must mark or
  recover post-ECDSA failures before those endpoints are fully implemented. No
  CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::rpc::request::handler::execute --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo test -p canic-core ops::auth --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.8` completed the next slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. Shared root replay
  receipts now reject cross-variant request-id reuse: before normal same-command
  replay evaluation, root replay checks for receipts with the same replay actor
  and operation id under any other root capability command kind. Live
  cross-command matches return a duplicate replay conflict; expired-only
  cross-command matches preserve the expired replay decision. This prevents an
  operation id committed for one root capability variant from being treated as
  fresh for another variant. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core ops::replay::guard --lib -- --nocapture
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core storage::stable::replay --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.7` completed the pool admin variant replay-inventory slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. The replay policy
  inventory now includes command-level coverage for every `PoolAdminCommand`
  variant: `CreateEmpty`, `Recycle`, `ImportImmediate`, and `ImportQueued`.
  `CreateEmpty` is recorded as implemented with `pool.create_empty.v1`; the
  non-CreateEmpty variants now have explicit response-idempotent ensure-style
  classifications but remain release blockers until replay receipts or stronger
  idempotence guards are implemented. `ImportImmediate` also now returns
  success before admissibility probing or management reset when the target
  canister is already present in the pool. No CLI commands changed in this
  patch. Validation:
  ```text
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.6` completed the pool create replay/cost-guard slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. Pool
  `CreateEmpty` now carries replay metadata, reserves a
  `pool.create_empty.v1` shared replay receipt, reserves deployment quota and
  an in-flight cycle budget before management `create_canister`, marks the
  management create effect in flight, calls the guarded management adapter with
  a `CostGuardPermit`, commits the created pool principal for duplicate replay,
  and marks uncertain post-management failures as recovery-required instead of
  re-executing the external effect on retry. `canic_pool_admin` remains a replay
  manifest release blocker until the non-CreateEmpty variants are classified
  and guarded. No CLI commands changed in this patch. Validation:
  ```text
  cargo test -p canic-core workflow::pool --lib -- --nocapture
  cargo test -p canic-core ops::cost_guard --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo test -p canic-core ops::replay --lib -- --nocapture
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo test -p canic-core --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.5` completed the first shared cost-guard slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. The branch now has the
  first shared cost-guard foundation in `ops::cost_guard`, backed by durable
  intent-store reservations. Root delegation-proof signing reserves a
  per-command/per-caller signing quota slot and an in-flight cycle reservation
  after fresh replay preflight and before threshold ECDSA. The prepared proof
  signer now requires an unforgeable `CostGuardPermit`, so the root signing
  adapter cannot be called through that path without crossing the guard.
  Committed replay responses still return without current quota or reserve
  checks. The replay policy manifest now marks `canic_request_delegation` as
  implemented; other costed endpoints remain release blockers. No CLI commands
  changed in this patch. Validation:
  ```text
  cargo test -p canic-core ops::cost_guard --lib -- --nocapture
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo test -p canic-core replay_policy --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  ```
- `0.61.4` completed the root delegation replay receipt slice from
  `docs/design/0.61-replay-protection/0.61-design.md`. Root
  delegation-proof issuance now uses shared replay receipts: shard-side
  requests attach root replay metadata, root rejects missing/invalid replay
  metadata, the endpoint reserves `auth.prepare_delegation_proof.v1` receipts
  before threshold ECDSA signing, marks the signing effect in flight, commits
  Candid-encoded proof bytes for duplicate replay, and reports conflict or
  recovery states for non-fresh receipts. The auth signing ops are split into
  prepare/sign phases so the API owns the replay/effect boundary. Shared
  receipt terminal transitions also preserve an existing external-effect
  descriptor when moving to committed, failed, or recovery-required states.
  The same patch also adds cached NNS data-center inspection:
  ```text
  canic nns data-center refresh
  canic nns data-center list
  canic nns data-center list --verbose
  canic nns data-center info <data-center-prefix>
  canic nns data-center list --format json
  ```
  Data-center metadata is derived from the shared mainnet registry relation
  inventory now used by nodes, node operators, node providers, and data
  centers: subnet membership, node records, node-operator records, and
  `data_center_record_<id>` values are fetched once and projected into the
  report. Rows include data-center id, region, owner, optional GPS,
  node-operator count, distinct node-provider count, and assigned-node count.
  The cache is `.canic/data-center/ic/data-centers.json`; refresh uses
  `.canic/data-center/ic/refresh.lock` and atomic cache replacement.
  Validation:
  ```text
  cargo test -p canic-core api::auth --lib -- --nocapture
  cargo test -p canic-core ops::replay --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic-ic-registry data_center -- --nocapture
  cargo test -p canic-host nns_data_center --lib -- --nocapture
  cargo test -p canic-cli --lib nns -- --nocapture
  cargo test -p canic-cli --lib command_family_help_returns_ok -- --nocapture
  ```
- `0.61.3` completed Slice B from
  `docs/design/0.61-replay-protection/0.61-design.md` Slice B. Root RPC replay
  now uses the shared replay receipt store instead of the legacy root replay
  map. Root capability replay prepares shared receipt tokens, checks receipt
  capacity, explicitly reserves fresh receipts, commits response bytes to
  shared receipts, returns committed receipt responses for duplicate requests,
  aborts reserved receipts on policy/execution failure, and purges expired
  receipts through shared receipt storage. The active legacy `RootReplayOps`
  and root replay slot-key module are removed; the old `RootReplayRecord`
  binary encoding remains test-only as historical stable-shape coverage.
  Explicit root command kinds are now:
  `root.provision.v1`, `root.upgrade.v1`, `root.recycle_canister.v1`,
  `root.request_cycles.v1`, `root.issue_role_attestation.v1`, and
  `root.issue_internal_invocation_proof.v1`. Delegation-proof issuance and pool
  `CreateEmpty` replay protection remain later 0.61 slices.
  The same in-progress 0.61.3 batch also adds the next broad NNS inspection
  surfaces:
  ```text
  canic nns registry version
  canic nns node refresh
  canic nns node list
  canic nns node list --verbose
  canic nns node info <node-prefix>
  canic nns node list --format json
  canic nns node-operator refresh
  canic nns node-operator list
  canic nns node-operator list --verbose
  canic nns node-operator info <node-operator-prefix>
  canic nns node-operator list --format json
  ```
  Node-operator metadata is derived from mainnet registry subnet membership,
  node records, and node-operator records, then cached at
  `.canic/node-operator/ic/operators.json`; refresh uses
  `.canic/node-operator/ic/refresh.lock` and atomic cache replacement. Registry
  Node metadata is derived from the same registry traversal and includes node,
  operator, provider, subnet, subnet kind, and data-center fields, cached at
  `.canic/node/ic/nodes.json`; refresh uses `.canic/node/ic/refresh.lock` and
  atomic cache replacement. Registry version is a live read against the
  canonical NNS registry canister.
  Validation:
  ```text
  cargo test -p canic-core ops::replay --lib -- --nocapture
  cargo test -p canic-core workflow::rpc::request::handler --lib -- --nocapture
  cargo test -p canic-core --lib -- --nocapture
  cargo clippy -p canic-core --all-targets --all-features -- -D warnings
  cargo test -p canic --test changelog_governance -- --nocapture
  cargo test -p canic-ic-registry node -- --nocapture
  cargo test -p canic-ic-registry node_operator -- --nocapture
  cargo test -p canic-host nns_node --lib -- --nocapture
  cargo test -p canic-host nns_node_operator --lib -- --nocapture
  cargo test -p canic-cli --lib nns -- --nocapture
  cargo clippy -p canic-ic-registry -p canic-host -p canic-cli --lib -- -D warnings
  cargo fmt --all -- --check
  git diff --check
  ```
- `0.61.0` completed Slice A by adding `canic-core::replay_policy`, a manifest
  that classifies Canic-emitted update endpoints by replay policy,
  implementation status, cost class, quota policy, and cycle-reserve policy.
  Manifest tests compare the static inventory against the facade macro files
  that emit Canic-owned update endpoints.
- The branch also hard-cuts verifier-local delegated-token update consumption:
  `access/auth/token.rs` no longer calls a consumed-use path, auth ops/storage
  no longer expose consumed-token APIs, `storage/stable/auth/token_uses.rs` is
  removed, and `AuthStateRecord` no longer contains the consumed-token field.
  Delegated tokens are TTL-bounded bearer credentials again; replay-sensitive
  commands must use domain replay receipts in later 0.61 slices. A focused
  upgrade-shape test proves old serialized auth state with historical consumed
  markers decodes into the new state shape while dropping that removed field.
  The auth trust-chain shell guard now matches this invariant: it preserves the
  endpoint order `verify -> subject binding -> scope check` and no longer
  requires the removed verifier-local consume step.
- `canic_app` set-style commands are now response-idempotent for the 0.61
  replay-safety line. The root endpoint returns `AppCommandResponse`; repeated
  `SetStatus` and `SetCyclesFundingEnabled` requests return success with
  `changed = false` instead of already-in-state errors, while actual changes
  still cascade root state.
- Local root delegation-proof issuance now rejects
  `msg_caller() != request.shard_pid` at the API boundary before delegated-token
  config lookup or threshold ECDSA signing. This fixes the authorization
  ordering finding, but delegation-proof response replay/caching is still a
  later 0.61 blocker.
- `0.60.10` adds the first non-subnet NNS inspection view:
  ```text
  canic nns node-provider list
  canic nns node-provider list --verbose
  canic nns node-provider info <node-provider-prefix>
  canic nns node-provider list --format json
  ```
  The command queries the mainnet NNS governance canister
  `rrkah-fqaaa-aaaaa-aaaaq-cai` with the Candid `list_node_providers` query,
  keeps the live call inside `canic-ic-registry`, shapes report/text output in
  `canic-host`, and exposes the surface through `canic-cli`. Non-verbose text
  mirrors the subnet list style with five-character provider principals plus
  assigned-node counts. Verbose text and JSON keep full principals, registry
  version, and reward-account detail; JSON keeps a nullable `name` field that
  the native source does not populate. Node counts are assigned mainnet subnet
  nodes derived from registry node/operator records. `info` resolves exact
  provider principals or unique provider-principal prefixes. The command is
  mainnet-only in 0.60 and rejects non-`ic` networks like the existing NNS
  subnet commands.
- `0.60.6` moves the public NNS subnet inspection surface from
  `canic subnet catalog ...` to `canic nns subnet ...`, records packaged
  downstream CLI proof for the current 0.60 subnet catalog line, and simplifies
  catalog stale-cache help. The verifier packages the publishable crate chain,
  repoints an isolated downstream CLI consumer at the packaged archives, builds
  offline, and runs basic CLI probes outside the workspace source tree. This
  proves `canic-subnet-catalog`, `canic-ic-registry`, `canic-host`, and
  `canic-cli` remain package-ready after the 0.60.5 registry chunking, compact
  list, and help cleanup changes. `canic nns subnet list/info` now use the
  7-day freshness default internally, keep stale status visible in output, and
  direct operators to the existing force-refresh path. `canic nns subnet info`
  also accepts unique cached subnet-principal prefixes for subnet lookups, so
  compact inputs can resolve when they identify exactly one subnet principal:
  ```text
  canic nns subnet list
  canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
  canic nns subnet info <subnet-prefix>
  canic nns subnet refresh
  ```
  No cache paths, catalog JSON fields, estimate fields, or registry transport
  behavior change in this patch. The catalog resolver API intentionally gains
  subnet-prefix resolution and typed prefix errors. Validation:
  ```text
  cargo test -p canic --test workspace_manifest publishable_members_do_not_depend_on_unpublished_workspace_members -- --nocapture
  cargo test -p canic-subnet-catalog -- --nocapture
  cargo test -p canic-host subnet_catalog -- --nocapture
  cargo test -p canic-cli --lib nns -- --nocapture
  cargo test -p canic-cli --lib -- --nocapture
  cargo build -p canic-cli
  target/debug/canic nns help
  target/debug/canic nns subnet help
  target/debug/canic nns subnet list help
  target/debug/canic nns subnet info help
  target/debug/canic nns subnet refresh help
  target/debug/canic nns subnet info tdb
  target/debug/canic nns subnet info ryjl3-tyaaa-aaaaa-aaaba-cai
  bash scripts/ci/verify-packaged-downstream-cli.sh
  cargo test -p canic --test changelog_governance -- --nocapture
  make fmt-check
  git diff --check
  ```
- `0.60.5` covers the registry/catalog/help cleanup batch. The shared
  `canic-ic-registry` adapter reconstructs high-capacity NNS registry values
  inside the adapter boundary: `get_value` responses with
  `large_value_chunk_keys` trigger Candid `get_chunk` requests, each returned
  chunk is SHA-256 validated before concatenation, and
  missing/rejected/mismatched chunks produce typed registry fetch errors before
  any catalog write. The current `canic nns subnet list` surface is compact by
  default for standard terminal widths, uses the first five characters of each subnet
  principal, and omits wider metadata columns; `--verbose` restores the
  full-principal/full-metadata text output. JSON output remains unchanged and
  full-fidelity. `canic help`, `canic nns help`, and
  `canic nns subnet help` describe the refresh-capable NNS subnet surface.
- `0.60.4` records the operator proof for the 0.60 catalog-derived estimate
  source. The proof refreshed the mainnet catalog at registry version `59015`,
  listed the cached catalog, resolved `mf7xa-laaaa-aaaar-qaaaa-cai` to
  fiduciary application subnet
  `pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae`, and
  generated:
  ```text
  docs/audits/reports/2026-06/2026-06-04/instruction-footprint.md
  docs/audits/reports/2026-06/2026-06-04/artifacts/instruction-footprint/
  ```
  The report records `rate_source = nns-registry-cache`,
  `formula_version = base_13_node_linear_v1`,
  `cycles_per_billion_instructions = 2615384616`, and `catalog_stale = false`.
  Current equivalent proof commands:
  ```text
  target/debug/canic nns subnet refresh --format json
  target/debug/canic nns subnet list --format json
  target/debug/canic nns subnet info mf7xa-laaaa-aaaar-qaaaa-cai --format json
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal mf7xa-laaaa-aaaar-qaaaa-cai
  ```
- `0.60.3` wires the refreshed mainnet subnet catalog into instruction-audit
  execution-cycle estimates as an optional cached source. The report path
  still performs no live NNS lookup; operators refresh first with
  `canic nns subnet refresh` and then opt in with:
  ```text
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal <canister-principal>
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal <canister-principal> --allow-stale-subnet-catalog
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal <canister-principal> --subnet-catalog-stale-after <duration>
  ```
  Explicit `--cycles-per-billion-instructions` still wins over every source,
  explicit `--estimate-node-count` still wins over the catalog, and catalog
  estimates are omitted when the cache is missing, stale by default,
  unresolved, missing a positive node count, or resolved to a non-application
  subnet. Catalog-derived rates accept arbitrary positive application subnet
  node counts with `ceil(1_000_000_000 * node_count / 13)` and record optional
  registry/subnet/catalog/routing provenance only when the catalog supplies the
  estimate source.
- `0.60.2` adds live mainnet NNS subnet catalog refresh behind the shared
  `canic-ic-registry` adapter. `canic-host` owns the refresh lock and atomic
  cache replacement for `.canic/subnet-catalog/ic/catalog.json`; `canic-cli`
  now exposes the current surface:
  ```text
  canic nns subnet refresh
  canic --network ic nns subnet refresh
  canic nns subnet refresh --dry-run --output <path>
  ```
  The command remains mainnet-only in 0.60, rejects non-`ic` networks, writes
  through `<canic-cache-root>/subnet-catalog/ic/refresh.lock` and
  `catalog.json.tmp.<pid>`, and leaves any existing catalog intact on refresh
  failure. Protobuf transport and registry value decoding stay inside
  `canic-ic-registry`; host/CLI surfaces remain protobuf-free. Instruction
  audit estimate integration lands in `0.60.3`.
- `0.60.1` was the intermediate cached NNS subnet inspection rename from
  `canic subnet network ...` to `canic subnet catalog ...`; `0.60.6` supersedes
  that public route with `canic nns subnet ...`. The current CLI exposes:
  ```text
  canic nns subnet list
  canic --network ic nns subnet list
  canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
  ```
  The historical 0.60.1 route defaulted to mainnet `ic`, rejected non-`ic`
  networks, required an existing local catalog file, and did not include
  instruction-audit estimate integration yet.
- `0.60.0` starts the NNS subnet inspection line with cached mainnet IC subnet
  catalog support. The new pure `canic-subnet-catalog` crate owns schema
  validation, future-schema rejection, and canonical principal-byte routing
  resolution. `canic-host` owns the cache path
  `.canic/subnet-catalog/ic/catalog.json`, stale-cache reporting, and list/info
  report preparation.
- `0.59.7` keeps instruction-footprint report output unchanged while
  centralizing dynamic report status labels and the missing-baseline sentinel
  in report support code. Focused coverage now pins baseline selection and
  missing-baseline delta rendering to the same `N/A` sentinel.
- `0.59.6` keeps estimate behavior unchanged while splitting boolean estimate
  flag parsing from positive integer source parsing. Direct
  `CANIC_INSTRUCTION_AUDIT_ESTIMATE_EXECUTION_CYCLES` misuse now reports a
  boolean flag error instead of a positive-integer error.
- `0.59.5` keeps estimate artifact behavior unchanged while pinning the
  instruction-footprint markdown estimate section title, instructions-only
  label, and table header as report constants. Report-rendering tests now prove
  the estimate section is omitted when no rows have estimates and uses the
  required instructions-only label when estimates are present.
- `0.59.4` keeps the offline estimate artifact shape unchanged while pinning
  the remaining JSON contract labels (`kind`, `charge_model`,
  `subnet_source`, `source_meaning`, and `rate_source`) as named
  report-support constants. Existing tests still assert literal values so label
  changes remain deliberate.
- `0.59.3` keeps the offline estimate object behavior unchanged while making
  the fixed `execution_cycle_estimate.omitted_costs` list a single static
  contract reused by every report row. The serialized JSON shape is unchanged
  and now has focused coverage for the pinned omitted-cost list.
- `0.59.2` restores CI `RUSTUP_TOOLCHAIN` propagation through `$GITHUB_ENV`
  so nested Cargo wasm builds use the toolchain that has
  `wasm32-unknown-unknown` installed. It also removes the noisy ICP-refill
  endpoint macro `compile_fail` doctest from the release-gate doc-test lane.
  The missing-guard `compile_error!` branch remains covered by an ordinary
  unit test, so the macro still requires host-supplied
  `guard = <access expression>` without printing an expected red compiler
  diagnostic during `make patch`. The 0.59 design doc now records this as
  release-gate output hygiene, not a change to the estimate model.
- `0.59.1` tightens the 0.59 report-input contract and fixes workflow linting.
  Direct environment-driven instruction-audit estimates now reject
  node-count/rate inputs when estimate mode is disabled, matching the shell
  wrapper's `--estimate-execution-cycles` requirement. The CI workflow no
  longer uses invalid job-level `${{ env.* }}` expressions for Rust toolchain
  env, and `make install-dev` / `make update-dev` now pass the pinned
  `ACTIONLINT_VERSION` and install directory into the shared dev setup.
- `0.59.0` starts the instruction-accounting and offline estimate line:
  ```text
  docs/design/0.59-instruction-accounting-cost-estimates/0.59-design.md
  docs/changelog/0.59.md
  ```
  The instruction-footprint audit now records explicit
  `performance_counter(1)` / `counter_id = 1` metadata, keeps measured row
  fields instruction-named, and preserves `sample_origin` as the message-kind
  scope (`update`, `query`, or `composite_query`). Optional offline execution
  cycle estimates are host/test-side report decoration only: callers must pass
  `--estimate-execution-cycles` with either `--estimate-node-count 13|34` or
  `--cycles-per-billion-instructions <cycles>`, update rows receive the
  `execution_cycle_estimate` sibling object, query/composite-query rows are not
  presented as charged query costs, and 0.59 adds no NNS/catalog/network lookup
  or new cycles CLI namespace.
- `0.58.16` finalized the post-`0.58.15` cleanup. It moves the remaining
  ICP-refill recovery eligibility predicates for notify execution and stale
  transfer-window detection into storage ops, leaving workflow to provide
  policy timing and orchestrate transfer/notify steps. Retry request validation
  and stored-record-to-request conversion now also live with the rest of the
  refill record boundary helpers.
- `0.58.15` finalized the post-`0.58.14` cleanup. It moves ICP-refill recovery
  record status predicates and in-flight/resumable lookup filters into storage
  ops, so workflow no longer scans the stable refill record set directly for
  policy preflight or hub self-refill recovery. Manual refill policy preflight
  now also shares one input builder across the rate-gated and non-rate-gated
  paths.
- `0.58.14` finalized the post-`0.58.13` cleanup. It centralizes ICP-refill
  completed-cycle `Nat` saturation in storage ops, reuses one direct-child
  refill parent check in workflow, and shares cycles-timer in-flight guard
  helpers between child top-up and hub ICP self-refill. Refill metrics,
  grant-ledger reuse, and top-up scheduling now share the same deterministic
  helper shapes without changing refill records, endpoints, CLI, metrics
  labels, or funding semantics.
- `0.58.13` recorded successful registered direct-child ICP refills into the
  existing cycles-funding grant ledger after CMC `notify_top_up` completes,
  making budget/cooldown accounting observe completed direct-child refill
  grants without adding a refill-specific grant store, changing refill records,
  or changing endpoint/CLI shape.
- `0.58.12` wired existing cycles-funding hooks into pure ICP-refill policy
  evaluation. Manual and hub self-refill requests deny with
  `CyclesFundingDisabled` while funding is disabled, and registered direct-child
  refill targets consume the existing child funding cooldown ledger through
  `FundingCooldownActive`. This closes the design gap that refill must consume
  existing funding policy hooks without adding a new refill-specific policy
  island or changing refill records, endpoints, metrics, or CLI shape.
- `0.58.11` finalized the post-`0.58.10` ICP-refill validation follow-up. It
  adds focused workflow regression coverage for the manual `notify_top_up`
  retry cap: the fifth CMC `Processing` response or retryable notify failure
  becomes terminal `Failed` state with `NotifyMaxAttempts`. It also expands
  focused recovery coverage for CMC notify terminal variants and ledger
  transfer mappings, including refunded, transaction-too-old,
  invalid-transaction, bad-fee, duplicate, and stale transfer outcomes.
  Finally, it adds `icp-refill` facade doctests to the fast workspace lane so
  the endpoint macro's missing-guard `compile_fail` contract is exercised
  during normal validation.
- `0.58.9` paused the ICP-refill work long enough to action downstream Canic
  adoption feedback from the `canic-test` build. That follow-up adds:
  ```text
  docs/getting-started/local-academic-fleet.md
  .cursor/skills/canic-academic/SKILL.md
  ```
  and promotes `canic info list` / `canic info medic` plus target hygiene,
  `CANIC_ROOT`-style canister ID naming, sourced shell helper rules, sharded
  internal-call shape, metrics stale/deployed checks, and install-versus-upgrade
  guidance into the README/install surfaces. The CLI help/diagnostic side now
  also adds `canic info env <deployment>` for sourceable `CANIC_<ROLE>`
  canister ID exports, nudges `canic install` users toward the project upgrade
  flow for already-installed canisters, adds a blocked-install hint that points
  at `info list` / `medic` and the project upgrade flow, and makes missing or
  empty `canic_metrics` output point at deployed Wasm / metrics profile checks.
  Protected internal-call validation now includes accepted caller roles and the
  explicit generated-client call shape, which addresses hub-to-shard role
  mismatch traps without changing the transport. The access contract now has a
  protected internal-call recipe section for generated clients, lower-level
  `CanicInternalClient`, and raw `icp` public-endpoint calls.
- Previous minor: `0.58.x` ICP-to-cycles refill primitive. `0.58.0` starts
  the line with:
  ```text
  docs/design/0.58-convert-icp/0.58-design.md
  docs/changelog/0.58.md
  ```
  The line is scoped to a Canic-managed canister-side ICP refill primitive:
  source canister transfers ICP to the CMC top-up account, records the ledger
  block, calls direct `notify_top_up`, persists compact recovery state, and
  integrates with the existing `cfg.topup` / `request_cycles` funding chain.
  It deliberately does not add a parallel `canic icp convert` namespace,
  overload `canic cycles topup --icp`, make identity-funded conversion the
  primary shape, add a dedicated CLI retry verb, create a second funding
  policy island, or add broad new query/metric families. The planned build
  order is DTOs/records first, then storage ops, IC infra, `ops::ic`, pure
  policy gates, workflow orchestration, opt-in endpoint/macros, funding-chain
  integration, local fabrication, and only then any thin CLI trigger.
  The 0.58.0 design now explicitly limits `cfg.topup.icp_refill` to the MVP
  controls, splits `IcpRefillRecord` recovery state from `CycleTopupEvent`
  observability, defines the composable endpoint guard shape, requires local
  fabrication dry-runs to say they bypass the canister refill endpoint, and
  pins hub self-refill to `CycleTrackerWorkflow`. Timer-driven self-refill may
  defer to 0.58.1 if it cannot stay inside the existing funding interval.
  Initial implementation has started with passive refill DTOs, the MVP
  `topup.icp_refill` config policy, validation for nonzero refill limits/rate
  gate, the authoritative stable `IcpRefillRecord` map plus storage ops and
  deterministic transition helpers, the low-level ICP ledger / CMC helper
  layer, pure refill policy gates, and the manual canister-side refill
  workflow skeleton:
  ```text
  crates/canic-core/src/dto/icp_refill.rs
  crates/canic-core/src/storage/stable/icp_refill.rs
  crates/canic-core/src/ops/storage/icp_refill.rs
  crates/canic-core/src/infra/ic/icp_refill.rs
  crates/canic-core/src/ops/ic/icp_refill.rs
  crates/canic-core/src/domain/policy/icp_refill.rs
  crates/canic-core/src/workflow/ic/icp_refill/mod.rs
  crates/canic-core/src/workflow/ic/icp_refill/tests.rs
  ```
  This now has the reusable manual workflow path that prepares an
  `IcpRefillRecord`, executes `icrc1_transfer`, retries from the persisted
  transfer identity for an existing `operation_id`, blocks stale pre-ledger
  retry after the ICRC-1 24-hour deduplication window, updates persisted fee
  on `BadFee`, validates ICP ledger decimals, retries/directly calls
  `notify_top_up`, caps manual notify attempts at five, estimates dry-run
  cycles from the current ICP/XDR rate, and maps ledger/CMC recovery states
  through storage ops. The facade now has an opt-in `icp-refill` feature plus
  `canic_emit_icp_refill_endpoints!(guard = ...)`, which emits a guarded
  `canic_icp_refill` update method that immediately delegates dry-run/live
  requests to the workflow and keeps retry on the same endpoint. The existing
  `CycleTrackerWorkflow` timer now has the pinned root hub self-refill hook:
  when `topup.icp_refill` is enabled and the sampled hub balance is below
  `min_hub_cycles_before_refill`, it schedules or resumes the ICP refill
  workflow on the same timer path before any child grant fan-out. The cycles
  CLI now has the retained thin trigger:
  ```text
  canic cycles convert <deployment> <role-or-canister> --source <role-or-canister> --icp-e8s <amount>
  canic cycles convert <deployment> <role-or-canister> --fabricate --cycles <amount>
  ```
  Canister mode resolves the source and target from the installed deployment
  registry and calls the guarded `canic_icp_refill` endpoint with the requested
  Candid payload. Fabrication mode is rejected outside `local` and calls local
  `provisional_top_up_canister`; its dry-run text/JSON carries the required
  `mode=fabricate (does not call canister refill endpoint)` label. Post-0.58.3
  cleanup has moved the command-specific convert parser, execution path,
  Candid rendering, and tests into:
  ```text
  crates/canic-cli/src/cycles/convert/mod.rs
  ```
  The shared cycles wallet wrapper now owns only generic `icp cycles` command
  routing plus deployment-target resolver helpers used by convert/top-up.
  `0.58.4` fixed live CI runner disk exhaustion, not an attestation regression:
  `pic_role_attestation` failed while rebuilding the root test stub because the
  bootstrap `wasm_store` nested target hit `No space left on device`. The
  release removes the duplicate workflow-level canister artifact prebuild and
  has `scripts/ci/run-workspace-tests.sh` clear generated PocketIC wasm target
  caches before each heavy PocketIC suite. `0.58.5` cleaned up the ICP refill
  core by centralizing repeated infra error mapping in `ops::ic::icp_refill`
  and repeated status/error mutation helpers in `ops::storage::icp_refill`;
  the shared workflow transfer stale-window branch is now also a single helper
  for requested transfers and bad-fee retries. `0.58.6` splits the ICP refill
  workflow into a directory module so the production workflow lives in
  `mod.rs` and the workflow unit tests live in sibling `tests.rs`; removes
  stale dead-code suppressions and unused ICP account wrappers; adds a
  storage-level `records()` helper for callers that do not need stable-map
  keys; avoids duplicate manual policy evaluation when no ICP/XDR rate gate is
  configured; replaces intentional non-macro lint `allow(...)` attributes with
  `expect(...)`; and deliberately leaves the `finish!` macro's generated
  dead-code allow in place to avoid downstream false positives. `0.58.6` also
  adds `#[canic_query(composite)]` support, forwards the marker to the CDK
  query attribute, rejects composite markers on updates, and makes endpoint
  perf rows include an explicit call-kind label (`query`, `composite_query`,
  or `update`) when those rows are durable. Ordinary query calls still do not
  commit perf counters; use same-call `QueryPerfSample<T>` probes for
  query-side instruction measurements. `0.58.7` makes endpoint attribute
  parsing drier: `name`, `internal`, and `composite` now share
  literal/boolean marker helpers, short access path decoding is centralized,
  and parser tests cover the shared rejection paths. `0.58.8` splits endpoint
  macro parse, validate, and expand into directory modules with sibling
  `tests.rs` modules, and moves access-plan synthesis into
  `expand/access.rs`, so production macro files stay focused without changing
  macro behavior.
- Previous minor: `0.57.x` audit rotation and feedback window. This is a
  maintenance line, not a new feature line. The purpose is to rotate the
  recurring audits while real users try the compact v1 surface, then use that
  feedback to decide what actually needs work. `0.57.0` starts with:
  ```text
  docs/audits/recurring/system/publish-surface.md
  docs/changelog/0.57.md
  ```
  The publish-surface audit definition now reflects the current eight
  published crates, the post-0.56 installed/packaged proof story, the declared
  Rust `1.91.0` MSRV package contract, and the special `canic-wasm-store`
  bootstrap/runtime posture. It adds no commands, DTOs, deployment groups,
  signing, locks, registry import, teardown, controller mutation, or active
  adoption/import.
- The completed upstream ICP network launcher watch has been removed from CI
  after it flagged a newer launcher candidate for manual testing. Historical
  0.38 notes still document why the watch existed.
- Ran the 2026-06-02 DRY consolidation audit:
  ```text
  docs/audits/reports/2026-06/2026-06-02/dry-consolidation.md
  ```
  Verdict: PASS, risk 4/10. No blocker or high-severity duplication issue was
  found. Current watchpoints remain the large `deploy` CLI owner,
  command-specific evidence envelope wrappers, narrow backup/snapshot registry
  transport duplication, backup/restore test fixtures, and the large Wasm audit
  shell subsystem.
- Continued the DRY cleanup follow-up by moving deploy output-format enums,
  parser helpers, passive catalog command handling, and passive comparison
  command handling plus deployment-root, registration, current-install, and
  authority dry-run, resume-report, and passive deployment-truth command
  handling into:
  ```text
  crates/canic-cli/src/deploy/output_format.rs
  crates/canic-cli/src/deploy/catalog.rs
  crates/canic-cli/src/deploy/compare.rs
  crates/canic-cli/src/deploy/authority.rs
  crates/canic-cli/src/deploy/install.rs
  crates/canic-cli/src/deploy/register.rs
  crates/canic-cli/src/deploy/resume_report.rs
  crates/canic-cli/src/deploy/root.rs
  crates/canic-cli/src/deploy/truth.rs
  ```
  The new modules own shared JSON/text output-format parser glue and the
  local-state-only `deploy catalog` command family plus the artifact-only
  `deploy compare` command family and the deployment-root inspect/verify
  namespace, authority dry-run check/evidence/report/receipt namespace,
  passive resume-safety report command, passive deployment-truth field
  rendering commands, explicit `deploy register` state registration, and the
  current install runner entrypoint. This is behavior-neutral CLI
  command-family cleanup; it does not change deploy command semantics.
- Post-`0.57.12` DRY cleanup has moved passive `deploy external` command
  parsing, help, output-format selection, dispatch, report builders, local
  external artifact ID helpers, and `deploy check` evidence-envelope handling
  into:
  ```text
  crates/canic-cli/src/deploy/external.rs
  crates/canic-cli/src/deploy/check.rs
  ```
  This keeps behavior unchanged, keeps the existing direct builder/envelope
  tests pointed at module-local helpers, and reduces the main deploy owner to
  roughly 4.5k lines.
- The deploy test body has been moved out of `deploy/mod.rs` into:
  ```text
  crates/canic-cli/src/deploy/tests/mod.rs
  crates/canic-cli/src/deploy/tests/fixtures.rs
  crates/canic-cli/src/deploy/tests/authority.rs
  crates/canic-cli/src/deploy/tests/deploy_check.rs
  crates/canic-cli/src/deploy/tests/external.rs
  crates/canic-cli/src/deploy/tests/promote.rs
  crates/canic-cli/src/deploy/tests/root.rs
  ```
  This is a mechanical layout cleanup so the production deploy owner stays
  readable, with authority dry-run, `deploy check` parsing/status/envelope,
  passive external lifecycle, passive promotion, and deployment-root tests
  already separated from the shared fixture module.
- Previous minor: `0.56.x` v1 packaged downstream proofs is closed. The
  design is:
  ```text
  docs/design/0.56-v1-packaged-downstream-proofs/0.56-design.md
  ```
  `0.56.0` proposes a release-confidence line, not a new product feature line.
  It should prove that the installed CLI and packaged Canic crates can support
  the compact v1 story from clean downstream projects without repository-only
  shortcuts or stale command shapes. It deliberately keeps deployment groups,
  signing, locks, registry import, teardown, controller mutation, active
  adoption/import, broad live verification, one-command deployment pipelines,
  and new stable public DTO families out of scope. The packaged proof boundary
  is strict: after package archives are created, proof paths must not pass via
  repository path dependencies, `target/debug/canic`, unpublished local crates,
  hard-coded local paths, or repository `.canic` / `.icp` state.
  `0.56.0` also starts the retained release-probe hard cut:
  ```text
  docs/operations/0.56-v1-release-probes.md
  scripts/ci/verify-installed-canic-cli.sh
  ```
  The installed CLI probe now installs `canic` into a temporary root and runs
  the maintained v1 readiness smoke through the installed binary. The retained
  probe inventory documents the release question, installed-CLI use,
  packaged-crate use, temp-root behavior, and network assumptions for each
  retained packaged/installed probe. The packaged downstream CLI fixture now
  uses current fleet-scoped role declarations instead of topology-only legacy
  config.
  `0.56.1` has hardened the installed CLI smoke:
  ```text
  docs/operations/0.56-installed-cli-smoke.md
  scripts/ci/verify-installed-canic-cli.sh
  ```
  The proof now asserts it is using the temporary installed binary rather than
  `target/debug/canic`, isolates `HOME`, `CARGO_HOME`, `CARGO_TARGET_DIR`, and
  `TMPDIR`, and runs the maintained v1 readiness smoke through that binary.
  `0.56.2` has hardened the packaged downstream CLI proof:
  ```text
  docs/operations/0.56-packaged-downstream-cli.md
  scripts/ci/verify-packaged-downstream-cli.sh
  ```
  The proof now rejects repository crate paths and `target/debug/canic` in the
  packaged tool root, isolates proof execution paths where practical, and runs
  current v1 read-only commands against a downstream project. It also packages
  and patches `canic-control-plane` explicitly so local pre-publication
  versions do not pass by resolving that dependency from crates.io.
  `0.56.3` has hardened the special packaged downstream `wasm_store` proof:
  ```text
  docs/operations/0.56-packaged-wasm-store.md
  scripts/ci/verify-packaged-downstream-wasm-store.sh
  ```
  The proof now packages and patches same-version Canic sibling crates
  explicitly, rejects repository crate paths and `target/debug/canic`, isolates
  proof execution paths where practical, and verifies that the generated
  bootstrap wrapper points at packaged Canic sources. This remains an internal
  bootstrap/runtime proof, not ordinary downstream dependency guidance.
  `0.56.4` has closed the line with:
  ```text
  docs/audits/release-lines/0.56-closeout.md
  ```
  Verdict: PASS. The audit verifies the installed CLI proof, packaged
  downstream CLI proof, packaged `wasm_store` bootstrap proof, declared Rust
  `1.91.0` MSRV lane, retained probe inventory, and absence of new product
  surface or mutation authority.
- Previous minor: `0.55.x` v1 stabilization and readiness is closed. The design
  is:
  ```text
  docs/design/0.55-v1-stabilization-readiness/0.55-design.md
  ```
  `0.55.0` has started the line as a stabilization design only. It does not add
  commands, DTOs, mutation authority, or new deployment-management concepts.
  The line should prove the compact v1 operator story and close docs/help/test
  gaps before Canic adds deployment groups, signing, locks, registry import,
  teardown, controller mutation, active adoption/import, or broad live
  verification.
  `0.55.1` has added the maintained readiness checklist and aligned the current
  docs/help surface:
  ```text
  docs/architecture/v1-readiness-checklist.md
  canic evidence gate --policy <path> --envelope <path>
  canic evidence gate --policy <path> --manifest <path>
  ```
  `0.55.2` has added a maintained local smoke proof:
  ```text
  scripts/ci/v1-readiness-smoke.sh
  docs/operations/0.55-v1-local-smoke.md
  ```
  The smoke uses a temporary project and covers the safe local setup/catalog/
  evidence-gate subset without running artifact builds, installs, live
  deployment checks, controller mutation, registry import, teardown, or active
  adoption/import.
  `0.55.3` has closed the line with a v1 candidate audit:
  ```text
  docs/audits/release-lines/0.55-closeout.md
  ```
  Verdict: PASS. No blocker/high findings were found.
  `0.55.4` has resolved the release-readiness follow-up with:
  ```text
  scripts/ci/v1-operator-proof.sh
  docs/operations/0.55-v1-operator-proof.md
  ```
  The proof builds `demo.app` with stable build provenance, registers an
  explicit local deployment target under a temporary proof root, and emits a
  deployment-check envelope that fingerprints the build provenance. The check
  is expected to be blocked because the proof does not install the fleet,
  verify a live root, or build every fleet artifact.
  `0.55.5` has added the final post-0.55.4 closeout audit:
  ```text
  docs/audits/release-lines/0.55-final-closeout.md
  ```
  Verdict: PASS. The final audit supersedes the 0.55.3 candidate audit and
  verifies the maintained v1 command surface, both proof scripts, proof
  artifacts, docs/help alignment, passive/active boundaries, and 0.54
  passive-catalog transition.
- Previous minor: `0.54.x` passive deployment catalog is closed. The design is:
  ```text
  docs/design/0.54-passive-deployment-catalog/0.54-design.md
  ```
  The closeout audit is:
  ```text
  docs/audits/release-lines/0.54-closeout.md
  ```
- `0.54.0` has added the v1-sized catalog commands:
  ```text
  canic deploy catalog list
  canic deploy catalog inspect <deployment>
  ```
  The commands read only `.canic/<network>/deployments/<deployment>.json`,
  default to text output, support raw JSON with `--format json`, and write the
  selected format with `--output <path>`. They do not query live deployments,
  create deployment truth, infer deployments from fleet-template names, mutate
  topology/controllers/state, install Wasm, register artifacts, or add
  deployment groups.
- `0.54.1` has added the compact pre-v1 operator walkthrough:
  ```text
  docs/architecture/v1-operator-walkthrough.md
  ```
  The guide connects `canic build <fleet> <role> --provenance <path>`,
  `canic deploy check <deployment> --format envelope-json`,
  `canic evidence gate --policy <path> --manifest <path>`, and the passive
  deployment catalog while keeping the v1 boundary small.
- `0.54.2` has closed the line with:
  ```text
  docs/audits/release-lines/0.54-closeout.md
  ```
  The audit verifies local-state-only catalog behavior, text/JSON output,
  explicit output files, missing/legacy/malformed-state handling, the passive
  boundary, and the absence of groups, locks, signing, registry import,
  teardown, controller mutation, topology mutation, install authority, and
  active adoption/import.
  This slice also resolves the 0.49 closeout design-doc follow-up by removing
  stale role-only metadata wording and unshipped scaffold/attach/build examples
  from the implemented 0.49 design.
- Previous minor: `0.53.x` CI policy gates and project evidence manifests is
  closed. The implemented design is:
  ```text
  docs/design/0.53-ci-policy-gates-project-manifests/0.53-design.md
  ```
  The closeout audit is:
  ```text
  docs/audits/release-lines/0.53-closeout.md
  ```
  The line consumes 0.51 `EvidenceEnvelopeV1` and 0.52
  `canic.build_provenance.v1` evidence to evaluate passive CI policy gates,
  implemented as:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  canic evidence gate --policy <path> --manifest <path>
  ```
  It did not add deployment locks, signing, provider wrappers, artifact
  registry import, controller mutation, topology mutation, active
  adoption/import, or deployment/install authority. The policy implementation
  stays narrow around strict policy files, existing `EvidenceEnvelopeV1`
  evidence, stable envelope fields, stable `canic.build_provenance.v1` payload
  rules, project evidence manifests over existing envelope files, and stable
  policy gate report results.
- Previous minor: `0.52.x` source, build, and artifact provenance is closed.
  The implemented design is
  `docs/design/0.52-source-build-artifact-provenance/0.52-design.md`; the
  closeout audit is `docs/audits/release-lines/0.52-closeout.md`.
- Completed release-work area: 0.52 builds on 0.51's stable evidence envelopes
  by adding source, Cargo, build, and artifact provenance for:
  ```text
  canic build <fleet> <role> --provenance <path>
  ```
  The command emits an `EvidenceEnvelopeV1` containing stable
  `canic.build_provenance.v1` payload. 0.52 intentionally keeps signing, CI
  locks, provider wrappers, controller mutation, topology mutation, artifact
  registry import, adoption mutation, and deployment/install authority out of
  scope.
- 0.51 CI/GitOps evidence envelopes are closed. The implemented design is at
  `docs/design/0.51-ci-gitops-provenance-evidence-envelopes/0.51-design.md`;
  the closeout audit is `docs/audits/release-lines/0.51-closeout.md`.
- 0.50 adoption profiles and safe onboarding are closed with documented
  caveats. Treat the 0.50 line as the immediate passive-report foundation:
  brownfield, partial, standalone, leaf-only, hybrid external-Wasm, and minimal
  onboarding reports classify configured and observed roles with non-executed
  recommendations.
- 0.49 role lifecycle and topology attachment is the immediate foundation:
  fleet-scoped roles can be declared before topology attachment, but only
  attached roles can become deploy artifacts, install targets, or deployment
  truth.
- The 0.41-0.47 deployment-truth sequence is closed with documented caveats.
  Treat those lines as background constraints, not current implementation
  targets: 0.41 passive truth, 0.42 dry-run authority, 0.43 execution
  boundary, 0.44 artifact promotion, 0.45 passive external lifecycle, 0.46
  deployment-target identity, and 0.47 verified registered-root recovery.
- 0.47 closed the main 0.46 caveat: a registered root starts as
  `not_verified` and can become `verified` only through explicit
  deployment-truth root evidence plus the guarded root-verification receipt
  path. It did not add broad live deployment verification, live inventory
  crawling, group/catalog UX, teardown/test-deployment flows, or root
  rotation.
- 0.48 setup work closed the redundant authored setup surfaces. In
  particular, package metadata is the canister role source of truth, canister
  crates are runtime artifacts rather than reusable Rust dependencies, and
  production `ICP_ENVIRONMENT=ic` builds avoid debug Candid sidecars/metadata
  bloat.
- 0.49 must preserve deployment-truth strictness, but it is not a new
  deployment-truth verification line.

## Recent Work

- Added the 0.55.0 v1 stabilization design:
  ```text
  docs/design/0.55-v1-stabilization-readiness/0.55-design.md
  ```
  The design frames 0.55 as a proof/readiness line for the existing compact
  v1 surface:
  ```text
  canic fleet create <fleet>
  canic scaffold canister <fleet> <role>
  canic fleet role attach <fleet> <role> --subnet <subnet>
  canic build <fleet> <role> --provenance <path>
  canic deploy check <deployment> --format envelope-json
  canic evidence gate --policy <path> --manifest <path>
  canic deploy catalog list
  canic deploy catalog inspect <deployment>
  ```
  It intentionally avoids new public DTO families, deployment groups, signing,
  locks, registry import, teardown, controller mutation, topology mutation
  beyond existing role lifecycle commands, install authority, and active
  adoption/import.
- Added the 0.55.1 v1 readiness checklist:
  ```text
  docs/architecture/v1-readiness-checklist.md
  ```
  The checklist names the compact command set, required files, expected
  evidence outputs, and passive boundaries. It is linked from the root README,
  installation guide, architecture index, and v1 operator walkthrough.
  `canic evidence gate --help` now includes examples for both single-envelope
  and project-manifest evaluation.
- Added the 0.55.2 local smoke proof:
  ```text
  scripts/ci/v1-readiness-smoke.sh
  docs/operations/0.55-v1-local-smoke.md
  ```
  The script runs in a temporary workspace and proves fleet creation, role
  scaffold, declared-only inspection, explicit role attachment, attached
  inspection, empty local deployment catalog output, and passive evidence-gate
  evaluation. It documents that real artifact build/provenance, install, and
  deployment-check evidence remain heavier manual/local-operator paths rather
  than this fast smoke.
- Added the 0.55.3 v1 candidate closeout audit:
  ```text
  docs/audits/release-lines/0.55-closeout.md
  ```
  The audit verifies the compact v1 command surface, CLI help, docs alignment,
  local smoke proof, test coverage, and passive/active boundaries. Verdict:
  PASS.
- Added the 0.55.4 v1 operator proof:
  ```text
  scripts/ci/v1-operator-proof.sh
  docs/operations/0.55-v1-operator-proof.md
  ```
  It covers real build provenance and deployment-check envelope output against
  an explicit registered local deployment target without installing, live
  verifying, changing controllers, importing artifacts, or mutating repository
  `.canic`/`.icp` state.
- Implemented the 0.54.0 passive deployment catalog:
  ```text
  canic deploy catalog list
  canic deploy catalog inspect <deployment>
  ```
  The catalog is intentionally narrow before v1: it reads local
  deployment-target state only, emits text or `DeploymentCatalogReportV1` JSON,
  writes only explicit `--output` artifacts, and keeps deployment groups,
  promotion lanes, saved-evidence catalogs, locks, signing, registry import,
  provider wrappers, teardown, and active adoption/import deferred.
- Added the 0.54.1 v1 operator walkthrough:
  ```text
  docs/architecture/v1-operator-walkthrough.md
  ```
  It documents the compact build -> evidence -> policy -> catalog flow, records
  the local catalog smoke expectations for a fresh checkout, and leaves
  deployment groups, saved-evidence catalogs, locks, signing, registry import,
  teardown, controller mutation, topology mutation, and active adoption/import
  out of v1.
- Added the 0.54.2 closeout audit:
  ```text
  docs/audits/release-lines/0.54-closeout.md
  ```
  Verdict: PASS. No release-blocking findings.
- Cleaned up the implemented 0.49 design doc so it now matches shipped CLI
  surfaces: `canic fleet create <name>`, `canic scaffold canister <fleet>
  <role>`, and `canic fleet role attach <fleet> <role> --subnet <subnet>
  [--kind <kind>]`. Removed stale references to role-only package metadata,
  scaffold attachment flags, build dev flags, detach/normalize commands, and
  pool/max-shard attach flags.
- Drafted and then cut the tentative 0.54 design to the v1-sized operator
  story:
  ```text
  docs/design/0.54-passive-deployment-catalog/0.54-design.md
  ```
  The design intentionally defers deployment groups, promotion lanes,
  saved-evidence catalogs, locks, signing, registry import, provider wrappers,
  teardown, and active adoption/import until after the v1 surface is simpler
  and closed.
- 0.53.6 has closed the CI policy gate line with:
  ```text
  docs/audits/release-lines/0.53-closeout.md
  ```
  The audit verdict is PASS. It verifies the passive single-envelope gate,
  build-provenance policy rules, project evidence manifests, duplicate
  manifest-path hardening, CLI help, docs, tests, and unchanged passive
  boundary.
- 0.53.5 has hardened project evidence manifests. Duplicate evidence paths are
  now invalid before policy gate evaluation, so one saved envelope cannot be
  evaluated more than once under a single manifest. The passive boundary is
  unchanged.
- 0.53.4 has added maintained policy-gate architecture guidance:
  ```text
  docs/architecture/ci-policy-gates.md
  ```
  The guide documents policy files, project evidence manifests, single-envelope
  and manifest gate command shapes, minimal CI usage, output formats, exit
  classes, and the passive safety boundary. Evidence-envelope and build
  provenance architecture docs now link to it.
- 0.53.3 has added project evidence manifests to the passive policy gate:
  ```text
  canic evidence gate --policy <path> --manifest <path>
  ```
  `ProjectEvidenceManifestV1` groups existing evidence envelope files with
  required/optional status, expected payload schema, and expected target
  identity. Manifest gates emit `ProjectEvidenceGateReportV1`; required
  missing evidence fails with `missing_required_evidence`, optional missing
  evidence reports `success_with_warnings`, and payload/target mismatches fail
  with `blocked_by_policy`. The command remains passive: it does not run
  builds, generate evidence, discover live deployments, mutate manifest/
  evidence/config/topology/controllers, register artifacts, or turn policy
  success into deployment truth.
- 0.53.2 has added optional build-provenance field rules to the passive
  single-envelope policy gate:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  ```
  The new `[build_provenance]` policy table can require clean source evidence,
  `Cargo.lock` evidence, gzip Wasm output, SHA-256 artifact hashes, and package
  metadata `fleet.role` matching the evaluated envelope target. The gate still
  consumes one existing `EvidenceEnvelopeV1`; it does not run builds, generate
  provenance, query deployments, mutate policy/evidence/config/topology/
  controllers, register artifacts, or turn policy success into deployment
  truth.
- 0.53.1 has added the passive single-envelope CI policy gate:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  ```
  The command reads one strict `CiPolicyV1` TOML file and one existing
  `EvidenceEnvelopeV1`, evaluates stable envelope fields, payload schema
  identity/stability, evaluated exit class, structured summary state, and
  required input fingerprints, then emits stable `PolicyGateReportV1` output.
  Raw `--format json` emits the report; `--format envelope-json` wraps it in a
  new `EvidenceEnvelopeV1` with `target.kind = "policy_gate"` and fingerprints
  for both the policy file and evaluated envelope. The gate is passive and does
  not run builds, query live deployments, mutate evidence/config/topology/
  controllers, register artifacts, or turn policy success into deployment
  truth.
- 0.53.0 has hard-cut stale CLI surfaces before policy-gate work:
  ```text
  canic fleet config <fleet>
  canic backup manifest validate --manifest <file>
  ```
  The old top-level `canic config` and `canic manifest` command families are
  removed. Global `--network` forwarding now reaches every `canic deploy ...`
  leaf that consumes deployment network state.
- Drafted the tentative 0.53 CI policy gate and project evidence manifest
  design:
  ```text
  docs/design/0.53-ci-policy-gates-project-manifests/0.53-design.md
  ```
  The design now tightens the first implementation slice to a single-envelope
  policy gate:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  ```
  The first policy implementation slice evaluated envelope schema, payload
  schema identity/stability, evaluated exit class, and structured summary
  evidence state, then emitted a stable `PolicyGateReportV1` that distinguishes
  evaluated evidence from the gate result. Later slices added build-provenance
  field rules and project evidence manifests.
- 0.52.4 has closed the source/build/artifact provenance line with:
  ```text
  docs/audits/release-lines/0.52-closeout.md
  ```
  The audit verdict is PASS. It verifies stable `canic.build_provenance.v1`
  payload modeling, explicit build provenance output, saved build-provenance
  evidence inputs, CI/GitOps policy docs, and unchanged deployment, install,
  topology, and controller mutation boundaries.
- 0.52.3 has added CI/GitOps policy guidance for build provenance:
  ```text
  docs/architecture/build-provenance-ci-policy.md
  ```
  The guide explains recommended checks for dirty source state, `Cargo.lock`
  drift, package metadata `fleet.role`, raw Wasm vs gzip Wasm artifact hashes,
  and saved build-provenance linkage from passive adoption/deployment-check
  envelopes. It does not add signing, CI locks, provider wrappers, registry
  import, controller mutation, topology mutation, install authority, or active
  adoption/import.
- 0.52.2 has added saved build-provenance evidence inputs for passive
  envelope reports:
  ```text
  canic fleet adoption report <fleet> --profile <profile> --format envelope-json --build-provenance <path>
  canic deploy check <deployment> --format envelope-json --build-provenance <path>
  ```
  These options fingerprint the supplied `canic.build_provenance.v1` envelope
  as stable input evidence only. They require `--format envelope-json` and do
  not re-run builds, import artifacts, validate deployment truth from
  provenance, attach topology, mutate controllers, or turn provenance into
  authority. The slice also adapts Canic memory-ledger diagnostics to the
  locked `ic-memory 0.7.0` API.
- 0.52.1 has added explicit build provenance output:
  ```text
  canic build <fleet> <role> --provenance <path>
  ```
  The file is an `EvidenceEnvelopeV1` with stable
  `canic.build_provenance.v1` payload. `BuildProvenanceV1` records source
  state, dirty-source status, Cargo lock/package manifest evidence, package
  metadata `fleet.role`, toolchain/profile/target data, and produced Wasm/gzip
  Wasm SHA-256 hashes after successful artifact generation. Ordinary
  `canic build <fleet> <role>` still writes no provenance file, and provenance
  output does not mutate deployment truth, controllers, topology, `wasm_store`,
  artifact registries, adoption state, install state, or canister lifecycle.
- Drafted the proposed 0.52 source, build, and artifact provenance design. The
  design keeps `EvidenceEnvelopeV1` as the stable automation wrapper from 0.51
  and proposes stable `SourceProvenanceV1`, `CargoProvenanceV1`,
  `ArtifactProvenanceV1`, and `BuildProvenanceV1` records. The first emitter is
  designed as:
  ```text
  canic build <fleet> <role> --provenance <path>
  ```
  It records build provenance only after successful artifact generation, with
  signing, CI locks, project manifests, provider wrappers, registry import,
  controller mutation, topology mutation, adoption mutation, and
  deployment/install authority explicitly deferred.
- 0.51.6 has cleaned up the historical post-46 CI/GitOps provenance backlog.
  The backlog is now marked partially superseded by 0.51, uses the implemented
  `EvidenceEnvelopeV1` and `ExitClassV1` names. Later lines completed
  source/build/artifact provenance and project evidence manifests; remaining
  future scope is optional idea material under `docs/design/ideas/`.
- 0.51.5 has closed the evidence-envelope line with
  `docs/audits/release-lines/0.51-closeout.md`. The audit verdict is PASS: the stable
  envelope model, passive adoption-report and deployment-check emitters, shared
  input fingerprinting, exit-class precedence, envelope comparison, docs, and
  targeted validation are aligned. The historical post-46 backlog wording has
  since been reconciled and unfinished feature ideas moved out of active release
  planning.
- 0.51.4 has added concrete CI/GitOps guidance for stable evidence envelopes.
  `docs/architecture/evidence-envelopes.md` now shows passive artifact layouts,
  a minimal adoption/deployment-check/compare pipeline, raw JSON vs envelope
  JSON examples, recommended CI policy fields, and explicit limits on what
  envelope artifacts and envelope comparison prove.
- 0.51.3 has added `canic evidence compare --left <path> --right <path>` for
  CI-friendly comparison of stable `EvidenceEnvelopeV1` fields. The command is
  read-only, compares envelope schema/provenance/target/input/payload
  hash/summary/exit-class data, and intentionally ignores `generated_at`,
  `canic_version`, and the nested command-specific payload body.
- 0.51.2 has centralized evidence-envelope exit-class precedence in
  `canic-host::evidence_envelope`. Adoption-report and deployment-check
  envelope emitters now classify from the same structured summary, and
  deployment-check conflicts report `evidence_conflict` ahead of generic
  `blocked_by_policy`. The evidence-envelope architecture docs now spell out
  CI policy guidance for warnings, blockers, missing required evidence, and
  conflicts.
- 0.51.1 has hardened evidence-envelope fingerprints. Adoption-report and
  deployment-check envelopes now share the `canic-host::evidence_envelope`
  file fingerprint and payload-hash helpers, `InputFingerprintV1` records
  `path_display`, paths under the selected root render relative, and absolute
  evidence paths outside that root are omitted rather than copied into CI
  artifacts.
- 0.51.1 also hard-cuts fleet role declarations to require
  `package = "<path>"` on every `[roles.<role>]` entry. Standalone generated
  configs use `package = "."`, workspace governance rejects package paths that
  do not contain a real `Cargo.toml`, and test/special configs now use concrete
  package paths instead of placeholder role-name directories. The old
  package-less `minimal` topology fixture role has been removed from alternate
  test fleet configs; the remaining `canisters/audit/minimal` crate is only the
  standalone audit baseline. Adoption reports now call observed-only package
  state `undeclared-role` rather than preserving a non-package role concept.
- 0.51.0 has started the stable evidence-envelope line. `canic-host` now
  defines `EvidenceEnvelopeV1`, `ExitClassV1`, target/provenance/schema/input
  fingerprint DTOs, structured summary messages, and SHA-256 helpers. The
  adoption report CLI accepts `--format envelope-json`, preserves raw
  `--format json`, and emits an envelope with fleet/profile target identity,
  source config/input fingerprints, payload schema identity, payload hash,
  structured warnings/blockers/evidence gaps/conflicts, and an envelope exit
  class without adding mutation or live discovery. Deployment check also
  accepts `--format envelope-json`, preserving raw `DeploymentCheckV1` for
  existing JSON output while wrapping deployment/fleet target identity,
  provenance, config fingerprint metadata, payload identity, safety summary,
  and exit class. Release validation fixtures were updated to the hard-cut
  role lifecycle, and internal Wasm artifact builders now invoke Cargo with
  `--locked`.
- 0.50.15 has closed the adoption line by updating the 0.50 design doc from
  tentative planning language to implemented release-line language, keeping
  JSON output experimental throughout `0.50.x`, and adding regression coverage
  for symmetric artifact evidence conflicts, the authority recommendation
  matrix, and explicit artifact-manifest precedence over deployment-check plan
  artifacts.
- 0.50.14 has made adoption reports mark conflicting artifact evidence as
  `evidence-conflict` when supplied artifact manifest and inventory artifact
  evidence disagree about whether the same role is Canic-built or externally
  supplied.
- 0.50.13 has made adoption reports preserve unresolved inventory observations
  and unresolved artifact entries from supplied evidence in
  `missing_or_stale_evidence`, without retrying observation or mutating
  deployment-truth state.
- 0.50.12 has expanded text adoption reports so observed canister rows include
  match confidence and supplied evidence details such as controllers, Wasm
  evidence, deployment-target evidence, and warnings.
- 0.50.11 has gated observed-only role declaration recommendations on
  authority evidence. Canic-authorized candidates may still receive a blocked
  `canic fleet role declare ...` preview, while user-controlled, external, or
  unknown candidates receive an authority-review recommendation first.
- 0.50.10 has added `--cargo-metadata <path>` adoption evidence. The option
  reads `[package.metadata.canic]` `fleet` and `role` from a saved
  `cargo metadata --format-version 1` JSON artifact, rejects ambiguous use with
  `--package-metadata`, normalizes package paths against the selected fleet
  config, and does not run Cargo.
- 0.50.9 has extended `--deployment-check <path>` adoption evidence so saved
  `DeploymentCheckV1.plan.role_artifacts` also supply artifact evidence when an
  explicit `--artifact-manifest <path>` is not provided.
- 0.50.8 has added `--deployment-check <path>` to adoption reports. It reads
  inventory evidence from a saved `DeploymentCheckV1` JSON artifact, rejects
  ambiguous use with `--inventory`, and still performs no live discovery or
  mutation.
- 0.50.7 has added explicit read-only evidence inputs for adoption reports:
  `--inventory <path>`, `--artifact-manifest <path>`, and
  `--package-metadata <path>`. These feed existing JSON evidence into
  `canic fleet adoption report <fleet> --profile <profile>` without live
  discovery or mutation.
- 0.50.6 has added active adoption profile architecture docs. The new
  `docs/architecture/adoption-profiles.md` page documents the read-only report
  boundary, profile vocabulary, lifecycle classifications, recommendation
  previews, blocked actions, and evidence rules.
- 0.50.5 has polished adoption text rendering. Recommendations now render as
  report-only/non-executed output, suggested commands use
  `suggested_action_preview` with explicit status/support/availability lines,
  and blocked actions are described as actions not executed by the report.
- 0.50.4 has clarified hybrid external-Wasm adoption reporting. Role findings
  now carry supplied module-hash and external artifact evidence, hybrid reports
  warn that artifact registry import is outside adoption reporting, and
  `artifact registry import` is listed as a blocked adoption action.
- 0.50.3 has added standalone and leaf-only adoption report coverage.
  Standalone profile fixtures keep compile-only roles declared-only without
  synthesized topology. Leaf-only reports now leave authority-sensitive
  observed roles visible but suppress role-declaration recommendations for
  those authority surfaces.
- 0.50.2 has expanded passive adoption report coverage with brownfield and
  partial fixtures, plus focused externally controlled, observed-only, and
  declared-only fixture cases. These tests assert that report recommendations
  remain passive and that name-free observations do not invent role-declaration
  actions.
- 0.50.1 has wired the passive report model into a read-only CLI surface:
  `canic fleet adoption report <fleet> --profile <profile>`. The command
  selects the matching fleet config, renders text by default, can emit
  experimental JSON with `--format json`, and writes only an explicitly
  requested report artifact with `--output <path>`.
- 0.50.0 has started with a host-side passive adoption report model.
  `canic-host::adoption` now defines adoption profiles, role/resource
  classifications, report findings, non-executed recommendations, and
  `adoption_report_from_config_source(...)`. The report builder consumes
  supplied config, optional deployment inventory, optional artifact manifest,
  and optional package metadata evidence without reading or writing project
  files.
- 0.49 implementation landed the role-lifecycle foundation:
  `canic.toml` accepts explicit `[roles.<role>]` declarations under a required
  `[fleet] name`, canister package metadata now carries both `fleet` and
  `role`, and `canic::build!` validates declared `fleet.role` identity while
  emitting attached-vs-declared role state.
- `canic fleet role list <fleet>` and
  `canic fleet role inspect <fleet> <role>` now expose the read-only role
  lifecycle state: declared package metadata, attached topology labels,
  compile eligibility, deploy-artifact eligibility, and next action.
- `canic fleet role declare <fleet> <role> --package <path>` now adds a
  config-only ordinary role declaration for existing package-backed canisters.
  It does not attach topology, rejects `root` and duplicate declarations, and
  validates the updated config before writing.
- `canic fleet role attach <fleet> <role> --subnet <subnet>` now moves a
  declared ordinary role into direct topology, defaulting to `kind =
  "singleton"` unless `--kind` selects `shard`, `replica`, or `instance`.
- `canic scaffold canister <fleet> <role>` now creates a declared-only
  ordinary canister crate under an existing fleet config. It writes package
  metadata with the selected `fleet` and `role`, adds the matching role
  declaration and workspace member, and intentionally leaves topology
  attachment to `canic fleet role attach`.
- `canic build <fleet> <role>` is now the only visible artifact build shape.
  It selects the matching fleet config, passes that config into Cargo builds,
  and rejects declared-only roles before building artifacts.
- Deployment-truth role selection now uses deployable-role terminology and
  excludes declared-only roles from install targets, local artifact manifests,
  inventories, and local deployment plans while leaving them visible through
  role lifecycle inspection.
- `canic fleet role rename <fleet> <old-role> <new-role>` now renames ordinary
  declared roles in the selected fleet config, updates exact topology
  role-bearing references, and updates matching package metadata when the
  declared package manifest is editable.
- Workspace governance now checks committed `[package.metadata.canic]`
  fleet-role metadata against declared fleet roles and package paths. Generated
  standalone configs are declared-only and no longer synthesize root topology or
  attach the requested standalone role.
- 0.48 made package metadata the role source for `canic::build!` and
  `canic::start!()`, and 0.49 made that identity fleet-scoped through
  `[package.metadata.canic] fleet` plus `role`. Package-name inference and old
  build/root macro variants were removed.
- Root and non-root managed canisters now both use `canic::start!()`.
  `role = "root"` selects the root lifecycle and endpoint bundle; all other
  roles select the normal managed canister lifecycle.
- The active setup docs were refreshed around the single normal startup
  surface, derived singleton topology, Candid artifact behavior, and the
  canister artifact boundary.
- Demo fleets now include `user_hub` and `user_shard` sharding walkthrough
  canisters with inspection-oriented endpoints, without adding them to the
  main test flow.
- The published workspace MSRV is Rust `1.91.0`, while the internal toolchain
  remains Rust `1.96.0`; shipped runtime duration constants may use the
  standard minute/hour duration helpers available at the advertised floor.
- 0.47 started by making deployment-truth inventory carry explicit
  `observed_root` evidence. `DeploymentRootObservationV1` records deployment
  target, network, fleet template, root principal, observed canister ID,
  observation source, controller facts, module hash, status, and role
  assignment source. Local inventory identity now records the deployment
  target name rather than the fleet template name.
- 0.47 now has a passive root-verification report shape.
  `DeploymentRootVerificationRequestV1` consumes an existing
  `DeploymentCheckV1`, and `DeploymentRootVerificationReportV1` can mark
  source-check evidence as `EvidenceSatisfied` without persisting verified
  root state. The exact `unverified_deployment_root` blocker is allowed only
  as the sole hard blocker; unrelated safety blockers keep verification
  blocked.
- `canic deploy root inspect --request <file>` now reads a
  root-verification request JSON file and emits a passive report as JSON by
  default or text with `--format text`. The command does not install code,
  register state, query live inventory, or write `root_verification =
  verified`.
- `canic deploy root verify <deployment> --from-check <file>` now performs the
  explicit 0.47 state transition. It reads registered deployment-target state,
  validates a `DeploymentCheckV1` with explicit root evidence, writes
  `root_verification = verified` only after a state-digest compare-and-swap
  check, and emits a `DeploymentRootVerificationReceiptV1`. The command does
  not install code or mutate IC/controller state. Re-verifying an already
  verified same root emits a `NoStateChange` receipt without rewriting local
  state, and verified root replacement remains blocked. Receipt validation now
  requires local-state digest changes to match the claimed transition: promotion
  must change state, while `NoStateChange` must not. The receipt artifact now
  has JSON round-trip and schema-shape coverage.
- Root-verification report and receipt validators now reject malformed SHA-256
  digest fields up front, so archived root-verification artifacts cannot carry
  non-digest source-check, plan, inventory, report, receipt, or local-state
  digest strings while still passing validation. Report validation also rejects
  forged check rows whose displayed expected/observed fields no longer match
  the check-row evidence.
- Root verification now rejects stale or tampered source `DeploymentCheckV1`
  artifacts before they can satisfy root evidence. The source check's embedded
  schema must be supported, its diff must still match its plan and inventory,
  and its safety report must still match that diff.
- Root-verification report validation now rejects duplicate or unexpected
  identity/evidence check rows, keeping archived root-verification evidence
  schema-stable instead of accepting arbitrary check-row additions.
- `canic deploy root` help now describes the namespace as inspection or
  explicit verification rather than a passive-only report surface, and the
  0.47 design status has been updated to show that the root verify command,
  receipt artifact, and state transition have landed.
- Root-verification receipt text now distinguishes local-state mutation from
  canister execution by rendering `canister_execution: none` and
  `local_state_write: recorded`.
- Root-verification reports now carry `observed_root_observation_source`
  explicitly and validate the `root_observation_source` check row against that
  archived field. Report text renders the source so operators can tell
  deployment-truth `IcpCanisterStatus` evidence from local-state echo.
- Root-verification receipts now preserve the source report evidence status
  and source root observation source, and receipt validation requires
  `EvidenceSatisfied` plus `IcpCanisterStatus` before accepting the receipt as
  self-consistent.
- Root-verification reports now preserve `observed_root_canister_id` as an
  archived evidence field and validate the matching evidence row against that
  field directly, so root-canister evidence is not inferred from the adjacent
  root-principal display field.
- Root-verification receipts now preserve `source_observed_root_canister_id`
  and require it to match the verified root principal, keeping standalone
  receipt evidence bound to the exact root canister ID accepted by the source
  report.
- Root-verification receipts now also preserve the source report's passive
  state transition and validate it against the receipt transition, preserving
  whether the accepted report predicted promotion or same-root re-verification.
- Root-verification receipts now preserve the source report's current
  root-verification state and validate it against the receipt's previous
  local-state trust state, so a standalone receipt cannot pair a successful
  write with a source report built from a different root-verification state.
- Root-verification receipts now also preserve the source report source enum
  in JSON, text, and digest input, keeping standalone receipts explicit that
  accepted root evidence came from a deployment-truth check artifact.
- Root-verification receipts now preserve the source report `requested_at`
  timestamp in JSON, text, and digest input, tying standalone receipts to when
  the accepted passive root evidence report was generated.
- Root-verification receipt validation now rejects unsupported source report
  timestamp labels while accepting the RFC3339-style labels used by request
  artifacts and the `unix:<seconds>` labels emitted by the explicit verify
  path.
- For verify-path receipts, `unix:<seconds>` source report timestamps must
  match `verified_at_unix_secs`, preserving the single local write timestamp
  used to build the accepted report and receipt.
- Source-guard coverage now verifies explicit root verification validates
  deployment-truth evidence before local-state mutation, writes verified state
  only through the compare-and-swap helper, and creates the receipt after the
  guarded write.
- Local install state moved from fleet-template storage to deployment-target
  storage. New state records `deployment_name`, `fleet_template`, and
  `root_verification`; state writes no longer delete other deployments sharing
  a root, and legacy fleet-state files now produce a clear fail-closed recovery
  error instead of being projected into deployment truth.
- The deployment-target install-state API no longer uses fleet-owned reader
  names, and persisted state no longer carries a duplicate `fleet` field beside
  `fleet_template`. The shared host lookup boundary is now
  `canic-host::installed_deployment`, and deployment-target state that still
  contains the stale duplicate field fails closed instead of being accepted as
  current state.
- Deployment-target state now records `created_at_unix_secs` and
  `updated_at_unix_secs`; stale state containing the pre-cut
  `installed_at_unix_secs` field fails closed instead of being accepted as
  current state.
- Local deployment plan and inventory collection now resolve root identity from
  deployment-target state using `deployment_name`, not the configured fleet
  template name. `canic deploy install <deployment> --plan <file>` validation
  now requires the supplied plan deployment identity to match the explicit
  install target exactly rather than accepting a fleet-template fallback.
- `canic deploy register <deployment> --fleet-template <fleet> --root
  <principal> --allow-unverified` now writes minimal deployment-target state
  for explicit operator recovery. The `--allow-unverified` acknowledgement is
  required because registered roots are marked `not_verified`; plan generation
  does not use them as trusted root authority until verification evidence is
  recorded.
- Unverified registered roots are install safety blockers, not ordinary plan
  warnings. The deployment-truth gate now refuses current-install mutation when
  local deployment state records a root that has not been explicitly verified.
- Installed-deployment CLI diagnostics for backup, cycles, metrics, list,
  status, and medic paths now describe missing or lost live state as
  deployment-target state and point operators at explicit `deploy register`
  recovery instead of stale fleet-owned placeholders.
- Legacy fleet-state recovery guidance now requires operators to provide the
  owning fleet template explicitly; it no longer suggests that deployment
  target and fleet-template names are interchangeable.
- Source-guard coverage now keeps `canic deploy check` and the host
  deployment-truth check/preflight path read-only so checks cannot silently
  rewrite `root_verification`.
- 0.46 has started with passive `DeploymentComparisonReportV1` comparison over
  two existing `DeploymentCheckV1` artifacts. It binds check/plan/inventory
  digests for both sides, compares normalized identity/artifact/module/config/
  authority/pool/verifier/external-lifecycle evidence categories, validates
  archived digest drift, and renders host-owned passive text with no execution.
- `canic deploy compare --left <file> --right <file>` now reads two
  `DeploymentCheckV1` JSON artifacts and prints a passive comparison report as
  JSON by default or host-owned text with `--format text`; it does not query
  live state, install code, apply authority, or mutate deployments.
- Deployment comparison now preserves each input check's safety status. A pair
  of matching blocked or warning `DeploymentCheckV1` artifacts no longer
  renders as safe solely because there is no cross-target drift.
- Archived comparison targets now require explicit deployment names and
  networks, so comparison evidence cannot erase the deployment-target identity
  that 0.46 is hardening.
- Comparison now re-checks each input `DeploymentCheckV1` diff/report against
  its embedded plan and inventory. Stale or tampered input checks are rendered
  as hard comparison failures, not as reusable readiness evidence.
- `canic deploy compare` help now calls out that archived input checks are
  revalidated before comparison status is rendered.
- Release commits now run a dedicated release-index guard before tagging. The
  guard refuses staged non-release files and release files that also have
  unstaged edits, preventing accidental mixed code/version release commits.
- The release-index guard now has focused regression coverage for empty
  release indexes, staged deletions, staged non-release files, partially staged
  release files, and clean release-only indexes.
- Backup, cycles, metrics, and list missing-state diagnostics now all include
  the required `canic deploy register ... --allow-unverified` acknowledgement,
  keeping explicit recovery guidance aligned with the deployment-target hard
  cut.
- `canic info list`, `canic info cycles`, `canic info metrics`, and
  `canic backup create` now present live positional inputs as installed
  deployment targets, not fleets. Live list/metrics/cycles text output renders
  `Deployment:`, config-only output keeps `Fleet template:`, and metrics/cycle
  reports serialize `deployment` instead of `fleet`.
- Backup create/status/inspect now keep the CLI boundary deployment-target
  shaped: create options use `deployment`, default output directories use
  `backups/deployment-...`, create/inspect tables render `DEPLOYMENT`, dry-run
  status and inspect JSON serialize `deployment`, and the lower-level backup
  plan `fleet` field is mapped only at the CLI boundary.
- Snapshot download now follows the same deployment-target backup layout
  boundary: it parses an installed deployment target, defaults output to
  `backups/deployment-...`, uses deployment-root/membership wording in errors,
  and restore help examples now point at deployment-prefixed backup layouts.
  Its explicit canister membership validation helpers now use deployment
  terminology instead of stale fleet-selection naming.
- Restore apply dry-run and journal artifacts now use deployment-level
  verification vocabulary at the restore-plan boundary: plan JSON uses
  `deployment_verification_checks`, verification summaries use
  `deployment_checks`, operation counts use `deployment_verifications`, and
  journal operation kinds serialize as `verify-deployment`. Command previews
  also describe deployment-root verification instead of fleet-root
  verification.
- `canic-backup` now hard-cuts the backup manifest boundary to deployment
  vocabulary: public Rust types are `DeploymentBackupManifest`,
  `DeploymentSection`, and `DeploymentMember`, manifest JSON uses
  `deployment` plus `deployment_checks`, and crate metadata/docs plus test-only
  helpers use deployment backup wording.
- Backup artifact persistence now writes `deployment-backup-manifest.json`
  instead of the stale fleet-named manifest file, full non-root backup plans
  serialize scope as `non-root-deployment`, and manifest validation errors use
  deployment member/role wording.
- Canic no longer declares `pocket-ic` directly in the workspace or test fleet
  manifests; PocketIC now enters the dependency graph only through
  `ic-testkit`, keeping version ownership centralized in the testkit package.
- `canic-host` package metadata now describes host ownership around
  deployment and fleet-template workflows instead of presenting the crate as a
  fleet-owned live-state library.
- 0.45 has started with passive `LifecycleAuthorityReportV1` /
  `LifecycleAuthorityV1` projection from `DeploymentCheckV1`. The projection
  consumes existing `CanisterControlClassV1` values, reports direct,
  external/proposal, observe-only, verify-external-completion, and blocked
  lifecycle modes, and records required verification facts without mutating
  deployment state or adding a consent/execution path.
- 0.45 also has the first passive `ExternalUpgradeProposalReportV1` /
  `ExternalUpgradeProposalV1` artifacts, derived from `ExternalLifecyclePlanV1`
  rather than ad hoc role-only inputs. They bind lifecycle authority rows to
  current module/config observations, target artifact/config facts, consent
  requirements, lifecycle/proposal digests, and allowed authorization modes.
  Directly controlled rows do not produce external proposals, and
  unknown-unsafe rows remain blocked.
- `ExternalUpgradeReceiptV1` now records pending, refused, delegated, and
  externally executed outcomes with structural verification against observed
  module/config facts. Receipts now also validate against the exact proposal
  they claim to satisfy, including proposal identity, before-state facts,
  target verification result, and verification notes. These receipts remain
  evidence; live inventory remains truth.
- The passive 0.45 artifacts now have digest/text parity: lifecycle authority
  reports, lifecycle plans, proposal reports, and external completion receipts
  validate archived drift and render host-owned passive text that explicitly
  reports no execution.
- `canic deploy external plan <deployment>` and
  `canic deploy external proposals <deployment>` now expose the first passive 0.45
  CLI surface. `canic deploy external pending <deployment>` adds a passive pending
  external lifecycle report over the same local deployment truth. They default
  to JSON, support `--format text`, and do not request consent, execute
  external upgrades, install code, or mutate deployment state.
- `ExternalLifecyclePendingReportV1` now summarizes pending external lifecycle
  work from `ExternalLifecyclePlanV1` and `ExternalUpgradeProposalReportV1`,
  carrying direct/pending/blocked counts, pending proposal links, blocked
  subjects, residual exposure, digest validation, and passive text rendering
  without adding any external consent or execution path.
- `CriticalExternalFixReportV1` now gives the critical-fix path a passive
  residual exposure artifact over lifecycle/pending evidence. It records
  affected roles/canisters, directly patchable roles, externally blocked roles,
  required external actions, protected-call implications, residual exposure,
  and operator next steps without claiming deployment completion or mutating
  external canisters. `canic deploy external critical-fix <deployment>` exposes that
  report as JSON by default or host-owned text with `--format text`.
- `ExternalUpgradeVerificationReportV1` now packages a validated
  proposal/receipt pair into a digest-pinned passive verification artifact. It
  records the verification result, notes, live-inventory requirement, and
  source proposal/receipt digests while preserving the invariant that reported
  completion is not deployment truth. `canic deploy external verify --request
  <file>` exposes the report from an `ExternalUpgradeVerificationReportRequest`
  JSON file as JSON by default or passive text with `--format text`.
- `ExternalUpgradeVerificationCheckV1` now bridges verification-policy
  postconditions to either supplied observation facts or an existing
  deployment-truth check artifact. It reports each required postcondition as
  satisfied or mismatched, records the observation source and observed control
  class, and keeps supplied evidence from becoming live verified completion.
- `ExternalUpgradeCompletionReportV1` now prevents downstream consumers from
  conflating consent evidence, reported external action, and verified
  completion. `canic deploy external inspect completion --request <file>`
  exposes that passive rollup from archived proposal/evidence/check inputs.
- The 0.45 test guard now verifies external lifecycle code continues to use
  `CanisterControlClassV1` instead of adding a parallel external/user
  classifier.
- `ExternalUpgradeConsentEvidenceV1` now separates reported consent/action
  evidence from verification evidence. It links a proposal/receipt pair,
  records consent state, reporter, consent requirements, and allowed
  authorization modes, and remains passive structural evidence rather than
  live completion proof. `canic deploy external inspect consent --request
  <file>` exposes it as an advanced passive artifact without promoting it to a
  top-level lifecycle workflow command.
- `ExternalLifecycleCheckV1` now summarizes lifecycle plan, proposal, and
  pending evidence into one passive status artifact with direct, pending,
  blocked, and residual-exposure counts plus operator next actions.
  `canic deploy external check <deployment>` exposes that check as JSON by default
  or host-owned text with `--format text`, without consent delivery, external
  execution, live lookup, install, or mutation.
- `ExternalLifecycleHandoffV1` now packages pending external proposals into
  passive operator coordination instructions. It carries proposal/check/pending
  digests, consent channel/subject facts, target verification facts, blocked
  subjects, and residual exposure while preserving the boundary that handoff is
  not consent delivery or execution. `canic deploy external handoff <deployment>`
  exposes the packet as JSON by default or host-owned text with
  `--format text`.
- 0.44 has started with passive role artifact source DTOs and validation for
  digest-pinned override inputs. Receipt-backed artifact sources are limited to
  deployment/staging receipt evidence and do not accept authority dry-run
  artifacts as artifact sources.
- `canic deploy install <deployment> --plan <file>` accepts a raw
  `DeploymentPlanV1` or an `ArtifactPromotionPlanV1` envelope and routes the
  supplied plan through the current install deployment-truth/preflight gate
  plus activation operation runner. Blocked promotion plan envelopes are
  rejected before mutation, and the path now has focused coverage for raw and
  ready promotion plan input, blocked promotion envelopes, supplied-plan
  network/target mismatch rejection, missing root wasm artifact validation
  before mutation, and CLI source-guard mediation through `install_root`.
- Ready `ArtifactPromotionPlanV1` envelope installs now write an artifact
  promotion execution receipt wrapper after successful current install. The
  wrapper links promotion plan/provenance evidence to the nested deployment
  receipt while keeping raw `DeploymentPlanV1` installs on the ordinary
  deployment-receipt path.
- 0.44 closeout confirmed the release bar: digest-pinned artifact override
  plans, readiness reports, role-scoped sealed-wasm vs source/build artifact
  levels, target canonical embedded config checks, and promoted-plan execution
  only through the deployment-truth/preflight-mediated current-install runner
  path.
- 0.44 also has the first passive promotion readiness model. It reports
  role-scoped promotion source identity, target wasm/config identity,
  byte/config identity comparisons, blocking findings, and target-store
  restage warnings without executing promoted plans. Readiness artifacts now
  have validation for schema, identity fields, status/blocker consistency,
  duplicate roles, digest shape, restage state, and finding severities.
  Readiness artifacts now also carry deterministic readiness digests over their
  target plan link, status, role rows, blockers, and warnings.
- Promotion readiness also has host-owned passive text rendering, keeping
  operator formatting out of future CLI code and clearly labeling the surface
  as non-executing readiness output.
- Promotion DTO JSON shape tests now pin the initial source/input/readiness
  field names. Source/build readiness explicitly permits target config digest
  changes, while sealed-wasm readiness still blocks embedded-config mismatch.
- `check_promotion_readiness(...)` is now the host-owned passive entry point
  for building and validating readiness from a target plan plus role promotion
  inputs.
- `promoted_deployment_plan_from_inputs(...)` now produces a pure promoted
  target `DeploymentPlanV1` from validated readiness. It applies sealed-wasm
  artifact identity for selected roles while preserving the target plan's
  authority profile and trust domain. Source/build promotion leaves target
  materialization output in the target plan.
- `promoted_deployment_plan_transform_from_inputs(...)` now returns the
  promoted plan together with `PromotionPlanTransformV1` role summaries that
  record before/after artifact identity, embedded-config changes, and whether
  source/build promotion preserved target materialization output.
- Promotion plan transforms also have host-owned passive text rendering, so a
  future CLI surface can present role-level artifact/config changes without
  owning promotion formatting logic itself.
- `validate_promotion_plan_transform(...)` now validates archived transform
  artifacts for schema, promoted-plan linkage, duplicate roles, role presence,
  role summary consistency, and transform flag consistency.
- `PromotionPlanTransformEvidenceV1` now wraps validated promotion transforms
  with passive evidence ID and generated-at provenance. Evidence validation
  rechecks the nested transform and does not claim execution, staging, or live
  deployment state. Transform evidence artifacts now also carry deterministic
  evidence digests over their metadata and nested transform.
- Promotion transform evidence now has host-owned passive text rendering that
  explicitly reports `execution: none`.
- `PromotionArtifactIdentityReportV1` now records role source locator kind
  separately from artifact identity kind, so later promotion planning can
  compare/dedupe by identity semantics instead of source-kind labels alone.
- Promotion artifact identity reports also group roles by deterministic
  artifact identity key, showing when distinct source locators resolve to the
  same sealed or source/build identity.
- Promotion artifact identity reports also carry validated summary counters for
  shared identity groups, digest-pinned roles, source/build roles, and deferred
  identities, making dedupe semantics explicit report data. They now also carry
  deterministic report digests over their summary, role rows, identity groups,
  and blockers, so archived identity reports reject stale grouping drift.
- Promotion artifact identity reports also have host-owned passive text
  rendering for future CLI/report consumers.
- Source/build promotion now has passive `BuildRecipeIdentityV1`,
  `BuildMaterializationInputV1`, and `BuildMaterializationResultV1` DTOs with
  validation, separating reusable build recipe identity from target-specific
  config input and concrete output digests.
- `BuildMaterializationEvidenceV1` now links those source/build pieces with a
  computed target materialization-input digest, consistency flags, validation,
  and passive text output that explicitly reports no execution occurred.
  Materialization evidence now also carries a deterministic evidence digest
  over the recipe, materialization input, materialization result, computed
  input digest, and consistency flags; materialization identity reports and
  source-build transform links preserve that digest beside the evidence ID.
- Role promotion policy checks now model the 0.44 policy distinction between
  roles that must reuse sealed bytes and roles that may rebuild only when
  byte-identical output is later proven.
- Role promotion policy checks now carry deterministic check digests over
  their status, role decisions, and blockers, so archived policy reports reject
  stale decision drift directly.
- `canic deploy promote inspect policy --request <file>` now exposes passive
  role promotion policy checks as JSON by default or host-owned text with
  `--format text`.
- `canic deploy promote inspect readiness --request <file>` and
  `canic deploy promote inspect artifact-identity --request <file>` now expose the
  existing passive promotion readiness and artifact identity reports through the
  same JSON-default/text-optional CLI surface, without staging, installing,
  querying `wasm_store`, or mutating deployment state.
- Promotion readiness can now include those policy blockers directly, so
  readiness consumers can see sealed-byte and byte-identity policy failures
  without treating the standalone policy check as execution authority.
- Source/build promotion transforms can now carry validated materialization
  evidence links, giving the passive transform summary the recipe/input/result
  evidence ID, materialization evidence digest, target materialization-input
  digest, and output digests it would rely on before any execution path is
  introduced.
- Passive promotion transforms now carry deterministic promotion-plan lineage
  digests, giving later execution receipts a stable promoted-plan identity to
  cite without treating source authority as target authority.
- `canic deploy promote inspect transform --request <file>` and
  `canic deploy promote inspect transform-evidence --request <file>` now expose
  passive promoted-plan transforms and transform-evidence wrappers as JSON by
  default or host-owned text with `--format text`, without adding a promotion
  execution path.
- `canic deploy promote inspect target-lineage --request <file>` now exposes passive
  target execution lineage reports as JSON by default or host-owned text with
  `--format text`, keeping target-preflight linkage inspectable without
  attempting execution.
- Receipt-backed promotion artifact sources now require source receipt lineage
  digests, keeping artifact provenance tied to a specific archived receipt
  lineage instead of a locator alone.
- Passive wasm-store artifact identity reports can now be derived from staging
  receipts, preserving role locators, transport, chunk publication counts, and
  verified postcondition facts without querying `wasm_store`. They now also
  carry deterministic report digests over staged role locators, transport,
  chunk facts, verified postconditions, status, and blockers.
- Passive wasm-store catalog verification reports can now compare those staged
  wasm-store identities against supplied catalog observations, reporting
  missing catalog entries, artifact mismatches, and chunk-count mismatches
  without querying `wasm_store` or executing promotion. Each role observation
  carries a deterministic digest so archived catalog evidence cannot drift
  silently. Catalog verification reports now also carry deterministic
  verification digests over the identity-report link, role observations,
  status, and blockers.
- `canic deploy promote inspect wasm-store-identity --request <file>` and
  `canic deploy promote inspect catalog-verification --request <file>` now expose
  passive staged wasm-store identity and supplied-catalog verification reports
  through the same JSON-default/text-optional CLI surface, without live catalog
  lookup.
- Passive source/build materialization identity reports now aggregate validated
  materialization evidence by role and group roles by materialized output
  identity. They now also carry deterministic report digests over their role
  evidence, output groups, status, and blockers, so archived source/build
  materialization reports reject stale grouping drift.
- `canic deploy promote inspect materialization-identity --request <file>` now exposes
  passive source/build materialization identity reports as JSON by default or
  host-owned text with `--format text`.
- Passive artifact promotion provenance reports now link promotion plans to
  readiness, artifact identity, transform, target execution lineage,
  wasm-store identity, wasm-store catalog verification, and materialization
  identity report IDs without claiming execution. Provenance cites wasm-store
  catalog verification reports by both ID and digest. Catalog verification must
  reference the same wasm-store identity report or it becomes a passive
  provenance blocker. Role-level provenance also preserves the catalog
  observation digest and blocks locator drift between wasm-store identity and
  catalog verification artifacts. Role-level provenance also preserves the
  materialization evidence digest for source/build roles. Promotion execution
  receipt wrappers carry those same role-level catalog and materialization
  digests forward.
- Passive artifact promotion plan envelopes now carry deterministic plan
  digests over their linkage, readiness, artifact identity, transform, optional
  target execution lineage, and blocker set. Promotion provenance reports cite
  wasm-store identity reports, wasm-store catalog verification reports,
  materialization reports, and the promotion plan by both ID and digest, carry
  their own deterministic provenance report digests, and reject stale linkage,
  role, blocker, or execution-boundary drift. Promotion execution receipt
  wrappers cite both the promotion plan and provenance report by ID and digest,
  and carry their own deterministic execution receipt digest over nested
  receipt and role evidence.
- `canic deploy promote plan --request <file>`,
  `canic deploy promote check --request <file>`, and
  `canic deploy promote diff --request <file>` now form the small public
  promotion report surface for plan, readiness, and transform diff output.
  `canic deploy promote inspect provenance --request <file>` keeps passive
  provenance under the advanced inspection namespace. These commands do not
  treat promotion artifacts as execution authority.
- Passive artifact promotion execution receipts now wrap existing deployment
  receipts with promotion provenance linkage, promoted-plan lineage, and
  role-level execution evidence without adding a separate promotion executor.
  They require ready provenance, so blocked passive provenance cannot be
  presented as execution evidence, and the nested deployment receipt role
  evidence must match the promotion provenance role set.
- `canic deploy promote inspect execution-receipt --request <file>` now exposes the
  passive artifact promotion execution receipt wrapper as JSON by default or
  host-owned text with `--format text`, without adding a separate promotion
  executor.
- `0.43.8` is closed. The closeout report is
  `docs/audits/reports/2026-05/2026-05-25/0.43-closeout.md`.
- `0.43.8` adds a private current-install
  phase-operation runner, so activation phases now execute through a common
  phase/action/evidence boundary instead of manually wiring each operation
  into `run_phase`.
- `0.43.8` also adds source-guard coverage proving
  current-install activation phases use the operation runner and run only
  after deployment-truth and execution preflight gates are recorded.
- `0.43.7` routes current-install root bootstrap resume
  and readiness waiting through narrow operation values that own phase
  evidence and execution calls. This keeps current behavior intact while
  reducing the remaining ad hoc activation closure wiring before the executor
  boundary is fully separated.
- `0.43.7` also routes configured artifact builds through
  a narrow operation value that owns build-target evidence, role names, and the
  existing build call without changing build behavior.
- `0.43.7` also routes root canister resolution through a
  narrow operation value that owns root-target evidence and the existing root
  lookup/create call without changing canister creation behavior.
- `0.43.7` also routes release-set manifest emission
  through a narrow operation value that owns manifest-path evidence and the
  existing manifest writer call without changing manifest output.
- `0.43.7` also aligns current-install execution preflight
  phase evidence with the actual deployment-truth receipt phases emitted by the
  installer, replacing the older coarse phase list with receipt-level phase
  names.
- `0.43.6` adds a narrow testkit preflight
  context and validation coverage proving the harness path consumes the same
  `DeploymentPlanV1`, safety report, authority plan, and phase list as the
  current CLI executor. This satisfies the first harness-level plan-shape gate
  without making `canic-host` own test harness execution.
- `0.43.6` also routes current-install root wasm installation,
  root funding, and `stage_release_set` through narrow operation values that
  own their phase evidence and execution calls. This keeps current behavior
  intact while moving those phases out of ad hoc installer closure wiring and
  closer to the executor operation boundary.
- `0.43.5` hardens shared deployment receipt status classification. Generic
  deployment receipt construction now derives
  `FailedBeforeMutation`, `FailedAfterMutation`, and `PartiallyApplied` from
  command results plus role-phase evidence, giving later executor extraction a
  single receipt-status boundary instead of ad hoc current-install decisions.
  Receipt-aware resume checks now also reject receipts whose claimed execution
  status contradicts their command result and role-phase evidence.
- `0.43.4` starts the artifact-staging receipt model.
  `StagingReceiptV1` and `ArtifactTransportV1` now capture role artifact
  identity, transport, wasm-store locator, prepared chunk hashes, published
  chunk counts, and verified postconditions. Current install uses that typed
  shape to enrich `stage_release_set` phase evidence from the release-set
  manifest without changing installer mutation behavior.
- `0.43.3` removes the standalone `canic-cdk` workspace
  crate. The curated `canic::cdk` facade now comes from `canic-core::cdk`,
  `canic-core` owns the small CDK helper surface it still needs, and
  `canic-backup` now keeps its hash helpers locally instead of depending on a
  broad CDK support crate. This continues the 0.43 cleanup of facade/support
  boundaries while preserving the public `canic::cdk` import path.
- `0.43.2` hardens passive execution-preflight evidence.
  `DeploymentExecutionPreflightV1` now has validation helpers for standalone
  artifacts and source-check-bound artifacts, rejecting schema drift, blank
  provenance IDs, status/blocker mismatches, capability-list inconsistencies,
  and mixed check/preflight identity before later executor surfaces consume
  the readiness result. Current-install preflight paths run that validation
  before returning read-only readiness or writing the `execution_preflight`
  receipt. Host tests now pin the `DeploymentExecutionPreflightV1` JSON field
  and enum shape so passive executor-readiness artifacts do not drift
  accidentally before a CLI surface is promoted.
- `0.43.1` adds
  `deployment_execution_preflight_from_check(...)`, letting callers feed a
  `DeploymentCheckV1` directly into passive execution readiness without
  rebuilding authority reconciliation by hand. It also adds host-owned text
  rendering for `DeploymentExecutionPreflightV1` so the readiness artifact has
  a human-oriented summary before any CLI surface is promoted. Current install
  now persists an `execution_preflight` deployment-truth receipt after the
  materialized safety gate and stops before later install phases when that
  preflight is blocked. `check_install_execution_preflight(...)` exposes the
  same current-install execution readiness path as a read-only host API for
  future CLI or executor integration.
- `0.43.0` added a concrete current-CLI executor
  wrapper, routes current-install execution context through that executor
  object, and gates the existing current install phases on the backend
  capabilities they need before current install begins mutating deployment
  state.
- It also adds a passive `DeploymentExecutionPreflightV1` gate over
  `DeploymentPlanV1`, `SafetyReportV1`, `AuthorityReconciliationPlanV1`, and
  executor capabilities. This gives 0.43 a plan-shaped executor-readiness
  artifact without running backend operations.
- `0.43.0` starts the backend-agnostic execution line. Deployment receipts can
  now carry optional execution context metadata, and Canic has a minimal
  `DeploymentExecutor` trait plus current-CLI backend capability helpers.
  Current-install deployment truth receipts now attach current-CLI execution
  context metadata when they are written. Current install behavior is otherwise
  unchanged; this slice creates the vocabulary later extraction will use.
- `0.42.14` hardens the authority closeout boundary without adding controller
  mutation. Authority CLI help now documents that successful command exit means
  a local dry-run artifact was produced, not that controller state changed or
  that the whole deployment is safe. The 0.42 design/status docs now clarify
  that authority `Safe` is authority-scoped, and that dry-run receipts/evidence
  are structural self-consistency artifacts rather than tamper-evident proof.
- The 0.42.14 handoff constraints now propagate into the 0.43 through 0.46
  design docs: 0.43 owns plan-driven execution rather than standalone
  authority-apply UX, 0.44 excludes authority dry-run artifacts as promotion
  artifact sources, 0.45 projects existing control classifications into
  lifecycle authority instead of reclassifying them, and 0.46 treats authority
  dry-run artifacts as reporting evidence only.
- Added source-scan tests to keep authority CLI and deployment-truth authority
  paths free of controller mutation primitives, plus JSON shape tests that pin
  the `Authority*V1` artifact field names and enum strings used by automation.
- Added explicit `Authority*V1` schema-governance rules so future authority
  changes do not silently rename fields, reinterpret existing fields, or blur
  dry-run receipts with any later controller-mutating receipt surface.
- Added a receipt-only host helper for `canic deploy authority receipt`, so the
  CLI no longer builds a full authority evidence bundle just to extract the
  receipt. The receipt output still uses the same report/check provenance
  validation path and remains read-only.
- Authority dry-run receipt construction now rejects `finished_at` timestamps
  earlier than `started_at` directly, preserving the timestamp-order invariant
  even when callers build a standalone receipt without a full evidence bundle.
- The generic authority receipt-from-check helper now takes an explicit report
  ID. Only the local-ID convenience wrapper chooses Canic's standard local
  report and receipt IDs.
- Authority receipt and evidence text output now explicitly reports
  `controller_mutation: none_attempted` for dry-run receipts.
- Authority plan, report, evidence, and receipt text output now stamps
  `mode: dry_run`, keeping the read-only authority boundary visible in every
  human-oriented authority surface.
- Top-level `canic deploy authority` help no longer describes the authority
  leaves as JSON-only now that each leaf supports JSON by default and text via
  `--format text`.
- `0.42.12` is live. It covered receipt-only authority output hardening and
  explicit dry-run labels across authority text/help surfaces.
- Removed the unused SNS-specific CDK surface, including the baked-in SNS
  canister catalog; SNS deployment identities should be discovered from
  live/mainnet sources instead of maintained as static framework data.
- Removed the broad CDK NNS system-canister table. The NNS registry and
  exchange-rate canister principals now live beside the Canic core infra
  adapters that call them.
- `0.42.11` covered authority receipt hardening, the `ic-testkit` helper split,
  MSRV declaration update, and stale CDK helper/static-canister cleanup.
- Removed the obsolete `canic-cdk::structures::BTreeMap` wrapper. Stable-storage
  code now imports the upstream `ic_memory` B-tree map directly as
  `StableBtreeMap`, and map clearing uses upstream `clear_new()`.
- The published MSRV is Rust `1.91.0`, separate from the internal Rust
  `1.96.0` toolchain. The repo may use stabilized `std::assert_matches!`
  diagnostics in internal tests without forcing downstream source consumers
  onto the internal compiler.
- Moved the reusable PocketIC helper surface out of the Canic workspace into
  the sibling `ic-testkit` repository. Canic now consumes it through the
  workspace `ic-testkit` dependency, while Canic-specific root/auth
  fixtures remain in `canic-testing-internal`.
- `0.42.10` is live. Continued authority receipt hardening after it:
  standalone dry-run receipt construction now rejects unsupported source
  schema versions, missing source report check provenance, blank receipt
  identity inputs, and missing completion timestamps before emitting receipt
  evidence.
- `0.42.10` tightened authority-reporting after `0.42.9`:
  authority apply-readiness blockers now distinguish unsafe canister authority
  from other hard authority findings. Unsafe canister hard-failure evidence is
  still preserved in the report and receipt, but report counts and next-action
  guidance no longer double-count it as a separate hard authority-profile
  finding. Blocked authority reports also keep external-action and
  missing-observation next actions alongside unsafe/hard blocker guidance
  instead of hiding that follow-up work until the blockers are resolved, and
  blocked report summaries now include those warning-level counts when they
  coexist with blocking authority findings. Reports with blockers also keep
  next-action guidance for automatic dry-run candidates, so reviewable
  controller changes stay visible even when they cannot be applied yet.
  Evidence validation now has explicit regression coverage for mutated
  unsafe-blocker readiness, keeping archived evidence tied to the report model
  that produced it.
- `0.42.9` moved authority evidence ownership into `canic-host`: dry-run
  evidence validation now rejects blank required identity fields and full
  evidence bundles whose nested report or receipt omits source check
  provenance. Completed receipts also reject `finished_at` timestamps earlier
  than `started_at`. Authority dry-run evidence bundle construction now lives
  in `canic-host` deployment-truth code, authority report construction from a
  full deployment check has a host-owned helper, and local authority
  report/receipt/evidence IDs are generated by the host layer. CLI authority
  tests now cover parsing, format rejection, and host-helper delegation, while
  detailed authority DTO and text-rendering behavior stays in `canic-host`.
  The four read-only authority CLI leaves now share one parse/load/render
  helper and explicitly test JSON as the default authority output format. This
  keeps the CLI as a consumer of validated host evidence and keeps
  archived/read-only evidence self-contained without adding controller mutation.
- `0.42.8` is live: dry-run
  evidence validation now rejects schema-version drift and receipts whose
  operation status or command result no longer represents a completed
  successful dry run. It also recomputes report summaries from the
  reconciliation plan and rejects mutated report counts, readiness,
  breakdowns, observation gaps, or next actions. Completed dry-run receipts
  must now include `finished_at`, and evidence `generated_at` must match that
  completion time. This remains a passive consistency guard over
  archived/read-only evidence and does not add controller mutation.
- Added read-only human-oriented authority output:
  `canic deploy authority check|evidence|report|receipt --format text` renders
  existing authority DTOs as deterministic operator summaries while JSON
  remains the default automation format. The text plan includes per-canister
  dry-run decisions, and the evidence/receipt text surfaces preserve
  hard-failure, observation-gap, and external-action details. The renderers
  live in `canic-host` deployment-truth code so the CLI is a consumer rather
  than the owner of the presentation model. Text output also preserves
  evidence generation time and controller add/remove deltas for automatic and
  external authority actions, plus verified controller observations with
  observed and desired controller sets.
- `0.42.6` is live: authority reports and dry-run receipts now carry source
  check IDs, inventory IDs, and authority profile hashes, matching
  evidence-bundle provenance so standalone outputs remain self-describing.
  Receipt construction rejects mismatched report/plan/check provenance and
  altered report content instead of producing mixed evidence, and complete
  evidence bundles are validated before CLI output to preserve dry-run
  semantics and controller-observation evidence.
- `0.42.5` makes authority evidence more self-describing. Authority actions,
  automatic-action candidates, external-action records, and dry-run receipt
  observations now carry typed controller deltas; authority dry-run receipts
  include the source authority report ID; authority reports include the
  inventory ID and authority profile hash; and bootstrap `wasm_store` artifact
  builds no longer fail on runners without the optional `ic-wasm` binary.
- `0.42.4` tightens dry-run authority readiness. External-action records now
  contain only actual external authority actions, standalone receipts preserve
  unresolved observation gaps, reports include typed apply-readiness blockers,
  and the 0.42 design/status docs now frame apply, pool mutation, remote
  lock/epoch checks, and post-apply verification as promoted-or-later work.
- `0.42.3` tightens break-glass authority reporting. Authority reconciliation
  now blocks staging/emergency principal overlap with normal expected
  controllers as `authority_profile_overlap` hard failures, reports count hard
  findings, receipts preserve them, and blocked reports emit specific next
  actions for unsafe canister findings versus hard authority findings.
- `0.42.2` adds passive authority dry-run receipts and read-only
  `canic deploy authority receipt|evidence <deployment>` JSON output. Receipts
  preserve verified controller observations and unresolved external actions
  while explicitly recording that no controller mutations were attempted.
- `0.42.1` adds the read-only authority report/evidence surface. It includes
  `AuthorityReportV1`, `AuthorityReportCountsV1`, self-contained
  external-action records, pool authority cases, explicit
  `AuthorityAutomaticActionV1` records, typed observation gaps, action-count
  breakdowns, control-class breakdowns, and next-action guidance without
  applying controller changes.
- Started `0.42.0` authority reconciliation with a passive
  `AuthorityReconciliationPlanV1` model, dry-run planner over the existing
  `DeploymentCheckV1`, and read-only
  `canic deploy authority check <deployment>` JSON output. The first planner
  classifies already-correct controller sets, deployment-controlled controller
  deltas that could be applied automatically later, external-action cases for
  non-exclusive control classes, and unsafe unknown canisters, without mutating
  IC state.
- `0.41.18` was a cleanup-only deployment truth report refactor. Duplicate
  evidence grouping and diff/finding construction now share local helpers, and
  verifier readiness no longer uses a panic-shaped `expect("checked above")`
  path. No operator-facing behavior change was intended.
- Deployment diffs now detect duplicate planned verifier role-epoch
  expectations: conflicting minimum epochs hard-fail, while exact duplicate
  planned epoch requirements warn and compare only once.
- Receipt-aware deployment diffs now detect duplicate phase receipts:
  conflicting postcondition evidence hard-fails resume, while exact duplicate
  phase receipts warn without changing the resumable phase set.
- Receipt-aware deployment diffs now detect duplicate role-phase receipts:
  conflicting role-scoped phase evidence hard-fails resume, while exact
  duplicate role-phase receipts warn without changing the resumable phase set.
- Deployment diffs now detect duplicate observed artifact evidence by role:
  conflicting artifact observations hard-fail, while exact duplicate artifact
  observations warn instead of being collapsed by role-indexed lookup.
- Deployment diffs now detect duplicate verifier role-epoch observations:
  conflicting epoch evidence hard-fails, while exact duplicate epoch evidence
  warns instead of being collapsed by role-indexed lookup.
- Deployment diffs now detect duplicate planned artifact entries by role:
  conflicting planned artifact evidence hard-fails, while exact duplicate
  planned entries warn and compare only once.
- Deployment diffs now detect duplicate planned canister declarations:
  conflicting role-to-ID assignments hard-fail, while exact duplicate planned
  canister entries warn and compare only once.
- Deployment diffs now detect duplicate planned pool declarations:
  conflicting pool identity-to-ID assignments hard-fail, while exact duplicate
  planned pool entries warn and compare only once.
- Observed pool canister control classes now reuse enriched child live-status
  evidence, so pool safety reports can reflect live controller drift rather
  than only registry parentage.
- Controller drift checks now treat `subnet_registry+icp_canister_status`
  observations as live status evidence, so enriched child observations with
  missing expected controllers fail as controller drift instead of registry-only
  uncertainty.
- Deployment diffs now hard-fail when a concrete expected canister ID is
  observed with a different role assignment, making ID/role topology drift
  explicit.
- Deployment diffs now detect duplicate observed canister IDs: conflicting role
  assignments hard-fail, while exact duplicate observations warn as suspicious
  inventory evidence.
- Deployment diffs now apply the same duplicate-ID guard to pool canisters:
  conflicting pool identities for one canister ID hard-fail, while exact
  duplicate pool observations warn.
- Deployment diffs now hard-fail when a canister appears in both non-pool and
  pool observations with conflicting role identities, making cross-surface
  topology contradictions explicit.
- Deployment diffs now hard-fail when an expected non-pool role has no
  concrete planned canister ID and multiple observed canisters claim that role,
  avoiding first-match ambiguity in passive inventory reports.
- Installed module-hash comparison now targets the concrete planned canister ID
  when available, and hard-fails ambiguous role-only module-hash evidence
  instead of letting duplicate role observations decide the hash check.
- Local deployment truth now treats the implicit bootstrap `wasm_store` role as
  part of the passive role set. Plans expect it, local artifact manifests and
  inventories observe its `.wasm.gz` artifact when present, and missing
  bootstrap store artifacts remain typed gaps rather than installer mutation.
- Installed child canister inventory now enriches subnet-registry role
  observations with read-only live status/controllers/module hashes when those
  status reads succeed. Failed child status reads remain typed observation gaps
  and do not erase the registry-derived role fact.
- Deployment diffs now warn on extra observed non-pool canister roles so
  unexpected registry/live topology is visible in reports without blocking
  current installer continuation.
- Duplicate observed canisters for an otherwise planned non-pool role are
  reported through the same extra-canister warning class rather than being
  hidden by the expected role name.
- Local deployment truth plans and inventories now populate
  `deployment_manifest_digest` from the observed root release-set manifest file
  when it exists. Missing manifests remain typed assumptions or observation
  gaps instead of installer authority.
- Local deployment truth plans and inventories now populate canonical runtime
  config digests from the parsed `ConfigModel`, keeping raw config SHA-256 as
  separate local consistency evidence.
- Local deployment truth identities now include stable set digests for planned
  authority, expected/observed topology, artifact sets, and pool identities
  where those passive facts are available.
- Local deployment inventories now map live subnet-registry role entries into
  observed canister facts. Registry-derived observations satisfy role
  existence and module-hash evidence without pretending controller authority
  was observed.
- Current install now persists additional deployment receipts for release-set
  manifest emission, successful root canister resolution, local artifact build,
  the IC-mutating root install/funding/staging/bootstrap phases, and observed
  `wait_ready` evidence, plus the final local install-state write. The build
  receipt now carries role-scoped artifact outcomes for configured build targets
  when those roles are present in the deployment truth plan.
- Current-install deployment truth gates now treat every
  `SafetyReportV1.hard_failures` entry as a blocker instead of maintaining a
  hand-picked blocker-code allowlist. Warnings remain report-only.
- Current-install deployment truth gates now persist the lightweight
  `DeploymentReceiptV1` artifact-gate receipt as machine-readable JSON under
  `.canic/<network>/deployment-receipts/<deployment>/` before any installer
  mutation.
- `canic deploy resume-report <deployment>` can now discover the latest persisted
  local deployment receipt automatically; `--receipt <file>` remains available
  for explicit comparisons.
- Added passive pool-canister comparison to deployment truth diffs. Planned
  pool identities now produce `pool_diff` entries, missing concrete pool
  canisters or mismatched pool IDs block, unsafe observed pool control classes
  block, and undeclared observed pool canisters warn without changing installer
  execution.
- Tightened passive verifier-readiness diffs so required role epochs are
  compared against observed epochs: stale observed epochs block and missing
  required role-epoch observations warn.
- Local deployment plans now populate `expected_pool` from configured
  scaling, sharding, and directory pool identities, so pool expectations appear
  in passive deployment truth reports instead of staying empty.
- Local deployment inventory can now map installed deployment registry entries into
  `observed_pool` for configured pool roles. Ambiguous role-to-pool mappings
  are reported as observation gaps rather than guessed.
- Added receipt-aware deployment truth comparison for resume reporting. It
  evaluates plan, inventory, and prior receipt identity together, reports
  blockers for mismatched plans, roots, failed commands, or unverified
  postconditions, and only marks phases resumable after live truth and receipt
  postconditions agree.
- Current-install deployment truth gates now construct and print a lightweight
  `DeploymentReceiptV1` with explicit `Complete` or `FailedBeforeMutation`
  operation status for the artifact materialization gate.
- Added read-only `canic deploy resume-report <deployment> --receipt <file>` to
  print passive `ResumeSafetyV1` JSON from the current deployment truth check
  and a prior `DeploymentReceiptV1`, without resuming or mutating state.
- Extended local deployment truth plans with installed root identity from
  `.canic` state, so the plan records the current root trust anchor and
  concrete expected root canister when available. The current-install safety
  gate now blocks when that expected root is missing from observed inventory.
- Fresh local deployment truth plans now record missing install state as an
  explicit non-blocking plan assumption, and deployment truth reports surface
  plan assumptions as warning findings.
- Current-install gate output now prefixes findings with stable source labels
  (`plan`, `inventory`, or `diff`) and subjects, making plan assumptions
  distinguishable from live observation gaps.
- Current-install artifact receipts now include role-scoped materialization
  evidence. Each configured role records whether its artifact was verified or
  failed, while the deployment truth check remains the gate authority.
- Wired configured deployment controllers into the local deployment truth plan
  so controller drift checks compare live root status against `canic.toml`
  authority intent.
- Promoted the current-install deployment truth gate beyond missing artifacts:
  materialized artifact digest drift and observable controller-authority drift
  now block before manifest emission, install, or staging.
- Blocked current-install deployment truth gates now print their summary,
  receipt postcondition, and machine-readable blocker codes before returning
  the install error.
- Deployment truth gate errors and warning output now include finding codes so
  failed current installs remain scriptable without parsing prose.
- Added controller authority comparison to the deployment truth diff. Live
  root controllers must include the expected authority profile controllers;
  authority-profile overlaps block as unsafe; undeclared live controllers warn;
  declared staging and emergency controllers are treated as intentional
  authority rather than unexplained drift.
- Corrected the 0.41 config identity model after the design update: raw local
  config SHA-256 values are now raw evidence only, while
  `deployment_manifest_digest` remains reserved for the canonical deployment
  manifest identity. Raw config drift still blocks as a local consistency
  finding.
- Started live inventory expansion for installed roots: when local install
  state identifies a root canister, deployment truth now attempts a read-only
  ICP status observation and records live controllers, module hash, and status
  when available. Failed live reads become typed observation gaps.
- Added installed module-hash comparison to the normalized diff so planned
  role module identity can be checked against live root status observations.
- Aligned `DeploymentReceiptV1` with the revised partial-execution design by
  adding operation status and role-scoped phase receipt fields. Current
  installer receipts still populate this lightly; richer per-role outcomes
  remain future execution work.
- Added lightweight deployment truth receipt helpers for the current-install
  artifact materialization gate. The install path now constructs a
  `materialize_artifacts` phase receipt from live check evidence, but the gate
  still makes decisions from the deployment truth check, not from receipt trust.
- Clarified the deployment roadmap/design contract that execution is partial,
  not atomic: receipts must preserve per-role/per-phase outcomes, while
  recovery starts with re-inventory and resume analysis rather than implicit
  rollback.
- Clarified the promotion roadmap/design contract that sealed wasm promotion
  and source/build promotion are separate role-scoped modes. Source/build
  recipe identity is distinct from target-specific materialization input and
  target materialization result because embedded config can intentionally
  change output bytes.
- Added `canic deploy diff <deployment>` and `canic deploy report <deployment>` so the
  normalized deployment diff and safety report are directly inspectable without
  parsing the full deployment check JSON.
- Added local deployment config SHA-256 evidence to the deployment truth plan
  and inventory, and made the diff fail closed when the observed deployment
  manifest digest disagrees with the plan.
- Made `canic deploy check <deployment>` usable as a read-only automation gate: it
  still prints the full `DeploymentCheckV1` JSON, but now exits non-zero when
  the derived `SafetyReportV1` is blocked.
- Tightened local artifact consistency checks: if the plan and inventory both
  observe a `.wasm.gz` file digest for the same role, a mismatch becomes a
  blocking deployment truth finding.
- Added a read-only current-install deployment truth preflight helper. It
  adapts `InstallRootOptions` into the existing local deployment truth check
  pipeline without calling installer mutation steps.
- Added `canic deploy plan|inventory|check <deployment>` as the first read-only
  operator-facing deployment truth commands. They print local deployment truth
  JSON and do not replace `canic install`.
- Added the first current-install deployment truth safety gate. After the build
  phase, the installer now refuses to continue when the deployment truth check
  proves configured role artifacts are missing.
- Added changelog governance coverage so `## Unreleased` remains root-only and
  detailed minor changelog files stay versioned.
- Added per-design-line `status.md` logs to the 0.41-0.46 design directories
  and historical post-46 backlog topics.
  These files are now the durable place to record what actually landed, what
  drifted from the design, and what remains open for each minor.
- Clarified the deployment roadmap ladder without changing the hard cut:
  0.41 is truth/report groundwork and current-install safety checks, 0.42 is
  report-first dry-run authority reconciliation, and 0.43 owns full
  plan-driven deploy-install execution unless explicitly promoted earlier.
- Added a read-only local deployment plan builder that produces
  `DeploymentPlanV1` from resolved fleet config and the local role artifact
  manifest. It records unresolved assumptions instead of querying IC state or
  changing installer mutation behavior.
- Added a read-only local deployment check wrapper that ties together plan
  construction, inventory collection, diffing, and safety-report rendering.
  This is the first usable shape for a future current-install safety gate, but
  it still does not mutate deployment state.
- Added local `.wasm.gz` file SHA-256 observations to deployment truth
  inventory and role-artifact manifests. These are recorded as explicit
  `ObservedFileDigest` evidence and remain separate from release-set payload
  hashes so observation does not turn release-set metadata into live truth.
- Split `canic-host::deployment_truth` into focused module files before adding
  more behavior: `mod.rs` owns public exports and the schema version,
  `model.rs` owns passive V1 DTOs, `observe.rs` owns local inventory and
  artifact observation, `report.rs` owns diff/report classification, and
  `tests.rs` owns the focused host-side coverage.
- Added a read-only local role artifact manifest builder for
  `RoleArtifactManifestV1`. It maps configured roles and materialized
  `.wasm.gz` files into deployment truth artifact records, reusing
  release-set payload hashes when available and recording missing artifact
  facts as observation gaps.
- Added the first passive deployment truth evaluator. It compares
  `DeploymentPlanV1` and `DeploymentInventoryV1` into `DeploymentDiffV1`, then
  renders `SafetyReportV1` findings for missing artifacts, unsafe control
  classes, identity mismatches, config drift, verifier-readiness gaps, and
  inventory observation gaps without changing installer behavior.
- Added the first read-only local deployment inventory collector. It maps
  configured fleet roles, local install-state root identity, and materialized
  `.wasm.gz` artifacts into `DeploymentInventoryV1`, while missing config or
  artifacts become explicit observation gaps rather than installer errors.
- Added passive host-side deployment truth V1 model scaffolding under
  `canic-host::deployment_truth`. The new types cover plans, inventories,
  receipts, diffs, safety reports, role artifacts, canister control classes,
  verifier readiness, and phase postconditions, with JSON round-trip tests but
  no installer behavior changes.
- Started `0.41.0` as a design-prep slice for the deployment truth model. This
  line follows the 0.40 attested-call hard cut and focuses on making intended
  deployment state, observed inventory, phase receipts, diffs, and safety
  reports explicit before deployment mutation.
- Reframed tentative `0.41` as a deployment truth model at
  `docs/design/0.41-deployment-truth-model/0.41-design.md`. The 0.41 line now
  centers `DeploymentPlanV1`, `DeploymentInventoryV1`,
  `DeploymentReceiptV1`, and `DeploymentDiffV1` / `SafetyReportV1`, with
  receipts treated as evidence rather than truth. The roadmap now continues
  through 0.42 authority reconciliation, 0.43 backend-agnostic execution,
  0.44 artifact promotion, 0.45 external/user-owned lifecycle, and 0.46
  multi-deployment operations.
- Started `0.40.0` by adding the passive Candid DTOs for the protected
  internal-call wire ABI:
  `CanicInternalCallEnvelopeV1`, `CanicInternalCallHeaderV1`,
  `InternalInvocationProofRequest`, `InternalInvocationProofPayloadV1`, and
  `SignedInternalInvocationProofV1`. The first slice also adds the
  `CANIC_INTERNAL_INVOCATION_PROOF_V1` signing domain and hash helper so
  method-scoped invocation proofs cannot share the generic role-attestation
  signing domain.
- Continued `0.40.0` by adding root issuance for method-scoped internal
  invocation proofs. Root now accepts `InternalInvocationProofRequest` through
  the root capability workflow and direct auth endpoint, authorizes the subject
  role from either AppIndex or subnet registry ownership, verifies that the
  audience is known, rejects empty method bindings, signs the proof with the
  internal invocation proof domain, and chooses the signed epoch from root
  config rather than caller input.
- Continued `0.40.0` by adding verifier-side internal invocation proof checks
  and the first generated protected update wrapper path. `caller::has_role(...)`
  and `caller::has_any_role([...])` are now parsed and validated as attested-role
  predicates, update-only in V1, and protected wrappers decode
  `CanicInternalCallEnvelopeV1` inside Canic, verify the proof against
  caller/audience/method/role/subnet/TTL/epoch bindings, then decode original
  Candid args only after authorization succeeds. Mixed non-attested access
  predicates are rejected for this protected wrapper path so no existing
  `requires(...)` condition is silently dropped.
- Continued `0.40.0` by adding the low-level `CanicCall` primitive through
  `canic::api::ic` and the prelude. `CanicCall` keeps raw `Call` unchanged,
  encodes original endpoint args, requests a root-signed method-scoped proof for
  the caller role, builds the internal-call envelope, and dispatches it to the
  protected endpoint. The first cut is correctness-only: no outgoing proof cache
  and no retry-on-stale-material path yet.
- Started `0.40.1` by adding a heap-only outgoing internal-invocation proof
  cache for `CanicCall`. The cache reuses only exact root/key/subject/role/
  subnet/audience/method/TTL call-edge proofs, evicts near-expiry entries, and
  rejects cached proofs below the local role epoch floor; callee verification
  remains the authority.
- Continued `0.40.1` by adding coarse protected internal-call auth error codes
  and a narrow `CanicCall` repair path: if the callee returns stale role-epoch
  material or unknown verifier-key material, the caller invalidates its cached
  proof, obtains fresh root-signed material, and retries the protected call
  once. Expired proofs, malformed envelopes, authorization failures, and domain
  handler errors are not retried.
- Started `0.40.2` by migrating the local wasm-store update surface onto the
  protected internal-call protocol. Wasm-store update endpoints now require
  `caller::has_role("root")`, while root control-plane calls to those update
  methods use `CanicCall`.
  Catalog/status queries remain structural root-query exceptions until a
  protected-query design exists. The same slice aligned direct root auth RPC
  decoding for role attestations and internal invocation proofs so callers
  decode the signed proof payload returned by the direct endpoint instead of
  the local root capability response envelope. Reconcile coverage now asserts
  that old raw update tuples fail against protected wasm-store updates.
- Continued `0.40.2` by consuming `ic-memory` for generic multi-crate
  static range and memory declaration registration. Canic now declares its core
  and control-plane ranges through `ic-memory`, delegates declaration/opening
  macros to the generic runtime, removes the stale Canic-local declaration
  registry, and keeps only the Canic-owned eager TLS touch queue for framework
  storage wrappers.
- Continued `0.40.2` by separating reusable PocketIC helpers from Canic runtime
  crates. The reusable helper boundary now lives in sibling `ic-testkit`, while
  Canic-specific role/init/readiness fixtures stay in unpublished
  `canic-testing-internal`.
- Started `0.40.3` by adding protected-internal-call guardrails. The protected
  wasm-store update method list now lives in `canic-core::protocol`, the
  control-plane caller path consumes that canonical classifier, and a source
  guard test rejects first-party raw `Call`/`CallOps` usage for those protected
  method names.
- Extended the same guardrail slice so the wasm-store macro declarations and
  checked-in `wasm_store.did` are tested against the protected-update and
  structural-query manifests, preventing the protected ABI list from drifting
  away from exported endpoint shape.
- Tightened those manifest checks so they are exact-set comparisons in both
  directions: listed methods must appear with the expected ABI, and newly
  envelope-protected or structural-query wasm-store methods cannot appear
  without a manifest update.
- Added the first internal endpoint classification manifest for 0.40. The guard
  parses Canic's built-in macro-emitted internal endpoints and fails if any are
  missing an explicit protected/bootstrap/query-exception/capability/discovery/
  operator classification.
- Added a focused macro expansion regression for protected internal endpoints
  with `name = "..."` exports. The generated wrapper must compare the envelope
  target method and verify the invocation proof against the exported wire name.
- Started `0.40.4` by adding a typed `WasmStoreInternalClient` for the root
  control-plane publication path. Template source resolution, prepare/chunk/
  stage calls, and store-local GC calls now go through one client that selects
  `CanicCall` for protected updates and keeps catalog/status as structural raw
  query exceptions.
- Extended `0.40.4` by giving the wasm-store client an explicit endpoint table
  tested against the protected/query manifests, re-exporting those manifests
  through `canic::protocol`, and adding a private `RootAuthMaterialClient` so
  delegation, role-attestation, internal-invocation-proof, and key-set refresh
  requests use one structural bootstrap client boundary. Both clients now keep
  explicit endpoint tables with focused manifest/classification tests.
- Started `0.40.5` by removing the transitional AppIndex-only
  `caller::has_app_role(...)` path from the macro DSL and runtime access
  evaluator. Protected sibling Canic RPC now has one supported role surface:
  root-signed `caller::has_role(...)` / `caller::has_any_role(...)` envelopes.
- Started `0.40.6` by adding the first generated-client metadata surface for
  protected internal endpoints. The endpoint macro now emits a hidden
  `ProtectedInternalEndpoint` descriptor for every root-signed role-protected
  internal endpoint, and `CanicInternalClient` can call those descriptors through
  `CanicCall` without duplicating method names or accepted-role metadata.
- Extended `0.40.6` by adding protocol-owned protected descriptors for the
  built-in wasm-store update methods and routing `WasmStoreInternalClient`
  through `CanicInternalClient`, leaving only structural catalog/status queries
  on the raw call path.
- Tightened the same `.6` client surface with
  `ProtectedInternalEndpoint::required_single_role()`, so generated clients for
  single-role protected endpoints can derive the caller role from endpoint
  metadata and reserve explicit role selection for multi-role endpoints.
- Started `0.40.7` by turning protected endpoint descriptor accessors into a
  stable generated symbol shape, `canic_internal_endpoint_<endpoint>()`, and
  adding the first `canic_internal_client!` facade macro for typed protected
  update clients backed by those descriptors and `CanicInternalClient`.
- Extended `0.40.7` so `canic_internal_client!` supports explicit
  `role = ...` method clauses for multi-role protected endpoints while keeping
  single-role descriptors as the ergonomic default.
- Extended the `.7` client surface with `CanicInternalCallOptions` and generated
  client `with_*` transport controls for wait mode, attached cycles, and proof
  TTL, so typed clients do not need to drop down to raw `CanicCall` for those
  settings.
- Added integration coverage for the actual downstream flow: a protected
  `#[canic_update(... caller::has_role(...))]` endpoint emits
  `canic_internal_endpoint_<endpoint>()`, and `canic_internal_client!` consumes
  that generated descriptor directly.
- Started `0.40.8` by adding `canic_protected_endpoint!` so shared protocol
  modules can publish `ProtectedInternalEndpoint` descriptors for
  cross-canister generated clients without depending on the target canister
  implementation crate.
- Tightened the `.8` descriptor boundary so protected endpoint descriptors
  reject missing method names, empty accepted-role sets, empty caller roles, and
  duplicate caller roles, while shared protocol descriptor macros reject
  `roles = []` at compile time.
- Started `0.40.9` by adding a real project hub/instance fixture for generated
  protected clients: a test-only shared protocol crate owns the instance
  descriptor, the instance exposes a `caller::has_role("project_hub")`
  protected endpoint, and the hub calls it through `canic_internal_client!`.
- Extended the `.9` fixture into PocketIC coverage: the project hub provisions
  a project instance, calls its protected endpoint through the generated client,
  and a raw direct call to the protected target is rejected.
- Fixed two runtime bugs found by that coverage: the built-in wasm-store
  protected client now decodes the endpoint payload type instead of a
  double-nested `Result`, and auth-material root request metadata is
  domain-separated from provisioning/cycles request metadata so independent
  per-canister counters cannot collide in the same second.
- Started `0.40.10` by making role-attestation issuance use the root's current
  role epoch instead of copying the caller-supplied request epoch, matching the
  internal invocation proof model. The same slice removes the ignored epoch
  field from replay and capability proof payload identity, adds a canonical
  root-capability request payload helper, and domain-separates the remaining
  root request/capability metadata nonce streams. Outbound root-response
  attestation caching now treats the local role epoch as a minimum floor so
  newer root-signed epochs remain reusable while stale cached proofs are still
  rejected.
- Started `0.40.11` by extending the protected internal-call raw-call source
  guard beyond the wasm-store manifest. The guard now also discovers protected
  method names from shared `canic_protected_endpoint!` descriptors and
  protected `#[canic_update(... caller::has_role ...)]` declarations, while
  ignoring macro definitions and doc-comment examples.
- Started `0.40.12` by moving protected internal endpoint envelope decoding
  inside the Canic wrapper. Protected wrappers now read raw ingress bytes,
  decode `CanicInternalCallEnvelopeV1`, verify the proof, and only then decode
  the original endpoint arguments, so malformed raw calls return typed
  `InternalRpcMalformed` errors instead of failing in CDK argument decoding.
  The checked-in wasm-store DID and guard tests now reflect that protected
  updates expose a no-argument raw-ingress wrapper in Candid while `CanicCall`
  sends the envelope bytes directly.
- Followed up after `0.40.12` by aligning the 0.40 design notes and this
  handoff with the raw-ingress protected wrapper model. Historical implementation
  entries should now be read as current raw-ingress behavior rather than typed
  envelope Candid arguments.
- Continued that follow-up by making `CanicCall` encode the internal-call
  envelope explicitly and dispatch those bytes through `with_raw_args(...)`,
  matching the no-argument protected wrapper ABI at the public call boundary.
  A source guard now rejects a regression back to typed envelope-argument
  dispatch. The same low-level call boundary now rejects empty target methods
  and zero effective proof TTLs locally before requesting root proof material.
  Protected endpoint descriptors and handwritten `CanicCall` role selection
  treat whitespace-only method/role metadata as invalid.
- Final closeout pass is aligning the 0.40 design doc with the implemented
  raw-ingress wrapper, descriptor/generated-client, root issuance, heap-only
  cache, and endpoint-classification state.
- Started the next 0.40.13-sized hardening slice by strengthening the protected
  raw-call source guard. It now scans raw call expressions instead of only
  single lines, catches multi-line protected method literals/constants, and
  keeps external calls plus structural query exceptions allowed. The same guard
  now bracket-matches endpoint attributes so nested `caller::has_any_role([...])`
  role arrays do not hide protected methods from discovery. Raw-call pattern
  matching now avoids treating allowed `CanicCall::...` usage as forbidden raw
  `Call::...` usage.
- Started the next 0.40 hardening slice by making verifier-side auth material
  time windows explicit. Role attestations and internal invocation proofs now
  reject malformed windows where `expires_at <= issued_at`, reject future
  `issued_at` values, and map not-yet-valid internal invocation proofs to the
  non-retryable `AuthProofExpired` public class. The outgoing `CanicCall` proof
  cache also refuses malformed or future proof windows before retaining proof
  material. Root-issued role attestations and internal invocation proofs now
  share the same TTL/window construction path, keeping zero TTL, over-limit TTL,
  and expiry-overflow rejection consistent across both auth-material families.
  Internal invocation proof payload construction also rejects blank
  `audience_method` values, matching the authorization preflight guard.
- Started `0.39.1` by adding an AppIndex-backed
  `caller::has_app_role(role)` internal access predicate, giving app hubs and
  shards a first-class way to trust canonical sibling app canisters without
  relying on root-only subnet-registry checks.
- Started `0.39.2` by hardening the local `ic-memory` extraction crate while
  keeping `canic-memory` self-contained and publishable until `ic-memory` has
  an explicit publish order.
- Tightened the `ic-memory` capability boundary so sealed declaration snapshots
  and validated allocation sets cannot be fabricated by public struct literal,
  and runtime fingerprints now flow into staged generation diagnostics.
- Added a generic `ic-memory` diagnostic-export builder while deferring any
  `canic-memory` compile-time dependency on `ic-memory` until the standalone
  crate is ready to be published first.
- Started `0.39.4` as the packaging correction after `0.39.3` was published out
  of sequence: `ic-memory` is path-only local extraction scaffolding, and
  `canic-memory` is self-contained for crates.io publishing until `ic-memory`
  has an explicit publish order.
- Started `0.39.5` as the next local extraction slice for generic allocation
  lifecycle mechanics inside `ic-memory`.
- Added the first generic `ic-memory` physical commit model: dual protected
  generation slots with marker/checksum validation, highest-valid recovery,
  corrupt-newer-slot tolerance, and a native `LedgerCommitStore` boundary for
  allocation-ledger recovery and commits.
- Added generic `ic-memory` lifecycle mechanics for generation-scoped
  reservations, explicit retirements, `reserved -> active` activation, and an
  `AllocationBootstrap` pipeline that recovers, validates, stages, commits, and
  publishes validated allocations without owning framework endpoint policy.
- Started `0.39.6` with explicit genesis recovery boundaries:
  `ic-memory` can initialize from a supplied genesis ledger only when the
  protected commit store is physically empty, exposes commit-slot recovery
  diagnostics, validates `ledger_schema_version`/`physical_format_id`
  compatibility and allocation-history integrity before recovery or commit, and
  still fails closed on corrupt, incompatible, malformed, or partially written
  stores.
- Extended the same `0.39.6` slice so explicit reservation and retirement
  operations go through generic bootstrap helpers and the protected commit
  protocol instead of requiring adapters to hand-roll recover/stage/commit
  sequencing.
- Started `0.39.7` by adding Canic-owned policy adapter coverage in the
  unpublished `canic-tests` crate. The tests prove Canic's
  `MemoryManagerId(u8)` rules against `ic-memory` traits without adding a
  runtime/build dependency from publishable crates to the unpublished local
  extraction crate.
- Started `0.39.8` by moving generic `MemoryManager` slot-shape validation
  into `ic-memory`: known substrate marker, descriptor version,
  `MemoryManagerId`, usable IDs `0-254`, and ID `255` as the invalid sentinel.
  Canic namespace and range ownership still live in the Canic policy adapter.
- Extended `0.39.8` so `canic-memory` now directly depends on local
  `ic-memory` for stable-key grammar and schema-metadata validation. Packaging
  `canic-memory` as an independent crate is intentionally not the active
  constraint while this extraction converges.
- Continued `0.39.8` by making the Canic namespace/range policy an explicit
  adapter module in `canic-memory`, moving the temporary ID `0` self-record key
  to `ic_memory.ledger.v1`, reserving `0-9` for `ic-memory`, narrowing
  `canic-core` to `11-79`, and moving control-plane stores to `80-85`.
- Moved the CBOR serializer and `impl_storable_*` macros from `canic-memory`
  to `canic-cdk`; `canic-memory` now only re-exports them as compatibility
  glue while the memory crate is being retired.
- Started `0.39.9` by removing direct `canic-memory` dependencies from the
  top-level `canic` facade and `canic-control-plane`. `canic-core` is now the
  remaining Canic runtime boundary that directly owns `canic-memory` bootstrap
  glue while the extraction continues toward deleting the compatibility crate.
- Started `0.39.10` by moving the Canic managed-memory macro surface into
  `canic-core`: explicit-key memory declarations, range reservations, and
  eager-init helpers now expand through the core adapter, while the legacy
  implicit `ic_memory!` macro is not part of the core surface. The duplicated
  macro module has also been removed from `canic-memory`, leaving that crate as
  temporary backend glue.
- Started `0.39.11` by removing the `canic-memory` crate from the workspace.
  Its remaining backend modules now live under `canic-core::memory`, and
  `canic-core` depends directly on `ic-memory` for allocation-governance
  primitives.
- Started `0.39.12` by routing Canic runtime memory declarations through
  `ic-memory::DeclarationSnapshot`, adding a production Canic
  `AllocationPolicy` adapter, projecting the existing Canic physical ABI ledger
  into `ic-memory::AllocationLedger`, and running generic allocation-history
  validation during bootstrap without changing the persisted ledger format.
  The validated allocation set is now published from bootstrap, and Canic memory
  opening uses `ic-memory::AllocationSession` over the current MemoryManager
  substrate.
- Started `0.39.13` by moving reusable dual-slot protected recovery selection
  into `ic-memory`, making Canic's physical ledger recovery call the generic
  selector, and making Canic generation commits choose the inactive slot from
  validated recovery state instead of the unprotected `committed_slot` header
  field.
- Started `0.39.14` by adding `ic-memory::DualProtectedCommitStore` and making
  both `ic-memory::DualCommitStore` and Canic's physical ABI ledger record use
  the same trait-provided authoritative-slot recovery and inactive-slot
  selection mechanics.
- Extended `0.39.14` so protected commit recovery diagnostics are generated
  from the same generic `ic-memory` store trait and surfaced through Canic's
  ledger snapshot response.
- Started `0.39.15` by pointing Canic's workspace dependency at the standalone
  crates.io `ic-memory 0.0.1` package and removing the stale in-tree
  `ic-memory` workspace member/source copy.
- Removed the remaining current `canic-memory` references from README and the
  packaged-downstream publish verification scripts; historical changelog/audit
  references still describe older releases.
- Added a workspace manifest guard so explicitly publishable crates cannot add
  runtime or build dependencies on workspace crates marked `publish = false`.
- Wired the same manifest-boundary guard into `scripts/ci/publish-workspace.sh`
  before any publish attempt.
- Started `0.39.16` by moving the current `ic-memory` governance-slot range
  and ledger self-record metadata behind the standalone `ic-memory` API; Canic
  consumes that authority descriptor instead of defining the range itself.
- Canic now targets published `ic-memory 0.4.0` and consumes its generic
  `MemoryManagerRangeAuthority`, native stable-cell ledger record, CBOR ledger
  codec, and stable-structures re-export. Downstream application IDs are no
  longer modeled as a named Canic authority range; they are accepted when
  `ic-memory` validates the slot shape and the ID does not collide with a
  reserved range. The temporary local crates.io patch to the sibling checkout
  has been removed; `Cargo.lock` resolves the crate from crates.io with a
  registry checksum.
- Continued `0.39.16` by thinning `canic-core::memory`: macro-backed memory
  opens now validate by explicit stable key through `ic-memory::AllocationSession`,
  the old implicit-key declaration/registration helpers are gone, and
  `memory::api` is reduced to the ledger diagnostic facade.
- Removed the old per-crate range-reservation runtime path from
  `canic-core::memory`; Canic now keeps range concepts only as policy/ledger
  authority diagnostics, not as a registration prerequisite.
- Replaced Canic-local range DTOs in the memory diagnostic internals with
  `ic-memory` authority records and added the authority `mode` to the
  controller ledger diagnostic response.
- Collapsed the remaining live Canic memory registry duplication. Macro-backed
  stable-memory slots now register immutable `ic-memory::AllocationDeclaration`
  values before bootstrap, ad hoc pre-bootstrap registration remains a small
  pending queue, runtime bootstrap validates and commits a sealed
  `DeclarationSnapshot` through the native `ic-memory` ledger, and diagnostics
  are derived from native `ic-memory` state rather than a second authoritative
  registry map.
- Tightened the physical ledger writer hard cut: Canic now records entries only
  when they are present in an `ic-memory::ValidatedAllocations` set, and the
  old Canic-local key/ID historical conflict scanner has been removed from the
  writer path.
- Hard-cut Canic allocation persistence to the native `ic-memory` durable
  ledger: `crates/canic-core/src/memory/ledger.rs` is now a small stable-cell
  adapter over `ic_memory::LedgerCommitStore`, old Canic physical ledger
  records/projection/writer/checksum ownership are gone, and old Canic physical
  ledger bytes fail closed with an explicit hard-cut error.
- Removed Canic's direct `ic-stable-structures` workspace dependency; memory
  and `canic-cdk::structures` now use `ic_memory::stable_structures` so Canic
  does not drift from the storage substrate version selected by `ic-memory`.
- Drafted the proposed 0.40 attested Canic-call hard cut at
  `docs/design/0.40-attested-canic-calls/0.40-design.md`, replacing
  AppIndex-only sibling authorization with root-signed caller-role envelopes
  for Canic-to-Canic internal endpoints.
- Moved the backup/restore design track forward to
  `docs/design/0.35-backup-restore/0.35-design.md` and marked the old 0.34
  draft as superseded.
- Added the 0.35.2 controller-policy follow-up: root init and post-upgrade now
  retain the installing or upgrading root controller in the runtime controller
  set used for newly allocated managed children.
- Added the 0.35.3 changelog entry covering local replica port visibility,
  `canic replica start --port <port>`, configured-port local queries, ownership
  diagnostics, `canic fleet sync`, automatic `icp.yaml` sync after
  `canic fleet create <name>`, explicit `topup = {}` default top-up config
  blocks, and the default top-up amount change from `4T` to `5T`.
- Started the 0.35.4 endpoint cleanup by removing stale root wasm-store
  bootstrap upload endpoints, controller-gating root state/app-registry/log
  diagnostics, simplifying `canic_canister_status` to controller-only access,
  and updating wasm-store reconcile coverage to current managed release roles.
- Collapsed the root wasm-store endpoint surface by removing the duplicate
  publish-to-current shortcut plus split publication/retired status endpoints;
  current publication uses `canic_wasm_store_admin` and controller reads use
  `canic_wasm_store_overview`.
- Ran the 2026-05-13 recurring `instruction-footprint` performance audit as
  the first `0.35` baseline. It reports risk `3 / 10`; root delegation is the
  highest sampled endpoint at `800834` average local instructions, and the
  first-run baseline deltas are intentionally `N/A`.
- Reran the 2026-05-13 recurring `audience-target-binding` invariant audit. It
  reports risk `3 / 10` and confirms role-attestation, delegated-token,
  delegated-grant, and capability-proof audience/target binding still fails
  closed.
- Reran the 2026-05-14 oldest latest-run recurring audit,
  `token-trust-chain`, at
  `docs/audits/reports/2026-05/2026-05-14/token-trust-chain.md`. It reports
  risk `4 / 10`, finds no trust-chain correctness break, and leaves only
  structural watchpoints around `dto::auth` fan-in plus runtime verifier/guard
  edit pressure.
- Reran the next oldest latest-run recurring audit,
  `auth-abstraction-equivalence`, at
  `docs/audits/reports/2026-05/2026-05-14/auth-abstraction-equivalence.md`.
  It reports risk `3 / 10`, finds no abstraction bypass, and the recurring
  definition now uses current `crates/canic-macros` paths, targeted auth scans,
  and the auth trust-chain guard as required evidence.
- Promoted the repeated ad hoc `dry-consolidation` audit into the recurring
  system suite and reran it at
  `docs/audits/reports/2026-05/2026-05-14/dry-consolidation.md`. It reports
  risk `4 / 10`, down from May 12 after installed-fleet resolution, registry
  parsing, response parsing primitives, and major CLI command modules gained
  clearer owners.
- Applied a small dry-consolidation follow-up: `snapshot download` now uses the
  host installed-fleet resolver/cache for installed deployments, and `medic` reads
  installed-fleet state through the host installed-fleet boundary.
- Added the proposed 0.36 backup/restore v1 design at
  `docs/design/0.36-backup-restore/0.36-design.md`. The 0.36 release focus is
  proving and hardening the existing backup/restore execution code into a full
  operator-working backup and in-place restore flow with durable journals,
  receipts, resume/retry behavior, and status/verify gates.
- Started the first pushable 0.36.0 proof slice by adding backup runner tests
  for max-step resume without replaying completed/preflight work and failed
  snapshot retry from the recorded failed operation.
- Kept backup resume proof at the runner/test layer instead of exposing a public
  manual pause flag for `canic backup create`; 0.36 should start with the
  smallest operator surface that works.
- Added backup status coverage for execution layouts so durable
  plan/journal/manifest state reports `running`, `failed`, and `complete`
  without introducing new operator flags.
- Tightened `canic backup status --require-complete` to require the complete
  execution layout, including the finalized manifest, instead of accepting a
  completed execution journal by itself.
- Tightened `canic backup verify` for execution-backed backups so manifest and
  artifact verification also requires the persisted backup plan and execution
  journal to match and be complete.
- Changed backup create persistence to preserve an existing output layout and
  its progressed execution journal, so the CLI wrapper now honors the same
  resume boundary that the backup runner already supported.
- Changed `canic backup list` to surface execution-backed manifest state
  (`running`, `complete`, `failed`, or invalid plan/journal) instead of
  reporting all manifest-bearing layouts as `ok`.
- Started `0.36.1` by tightening `canic backup create --out <dir>` resume
  safety: existing layouts are preserved only when the stored plan matches the
  requested fleet, network, root, scope, target set, and operation graph.
- Extended backup create resume compatibility to authority and quiescence
  policy fields so dry-run layouts are not reused as executable backup layouts.
- Added a `LAYOUT` column to `canic backup create` output so fresh and resumed
  output layouts are visible to operators.
- Tightened `canic backup list` so manifest-plus-plan layouts with no execution
  journal report `invalid-plan-journal`, not `dry-run`.
- Tightened `canic backup create --out <dir>` so manifest-backed layouts with a
  missing execution journal are treated as incomplete instead of having a new
  journal synthesized.
- Tightened backup status, inspect, and verify so manifest-backed layouts with
  missing execution journals use the same incomplete-layout error instead of
  falling through to raw file-read failures.
- Tightened backup execution integrity so terminal mutating operations require
  matching operation receipts; preflight-completed validation operations remain
  receiptless as intended.
- Started `0.36.3` restore-runner hardening by making upload-snapshot commands
  fail if successful output does not include the uploaded snapshot id required
  by later load-snapshot operations.
- Added explicit `canic restore run --retry-failed` recovery so failed restore
  operations can be moved back to ready after inspection without hand-editing
  the apply journal.
- Tightened legacy restore upload-id parsing so only uploaded-snapshot-labelled
  text can satisfy a successful upload command without structured JSON.
- Tightened restore-runner journal loading so completed or failed operations
  must have matching command receipts before any runner mode proceeds.
- Started `0.36.4` by rejecting duplicate restore operation receipt attempts
  and adding an active-line changelog width check for root and detailed notes.
- Started `0.36.5` by requiring backup execution operation receipts to carry
  `updated_at` so terminal outcomes stay auditable in persisted journals.
- Tightened backup execution receipt recording so invalid receipts roll back
  the attempted operation transition instead of leaving partial in-memory
  state.
- Adjusted the changelog check so root `CHANGELOG.md` patch bullets stay on
  one line while detailed changelog notes keep the 88-column prose wrap.
- Started `0.36.6` by making backup execution integrity compare terminal
  mutating operation state with the latest matching receipt, so stale retry
  history cannot hide a hand-edited journal state mismatch.
- Folded persisted backup execution `restart_required` validation into the
  `0.36.6` slice so edited journals cannot hide a required restart window.
- Tightened `0.36.6` further by requiring backup execution transition
  timestamps before mutation and rejecting persisted pending or terminal
  operation states without `state_updated_at`.
- Added `0.36.6` persistence integrity coverage that rejects terminal backup
  operation timestamp drift from the latest durable operation receipt.
- Started `0.36.7` by requiring restore apply-journal command receipts to keep
  their update timestamp, command preview, exit status, and bounded
  stdout/stderr audit payloads.
- Folded stale local-replica status handling into `0.36.7`: ICP CLI local
  status is now treated as stale unless the configured gateway port is
  actually reachable, so `canic replica start` no longer reports a dead
  configured port as already running.
- Started `0.36.8` by tightening restore-runner journal loading so terminal
  restore operations must be backed by the latest matching command receipt
  attempt with the same state timestamp.
- Folded a `canic list --subtree` role-anchor fix into `0.36.8`: unique role
  names now resolve to their canister principal, while repeated roles require a
  concrete principal.
- Extended the same role-or-principal subtree selector to
  `canic cycles --subtree`, filtering the registry before cycle history,
  balance, and top-up queries run.
- Started `0.36.9` by adding the `canic info` read-only command group with
  `info list` and `info cycles` leaves, then removed the old top-level
  `canic list` and `canic cycles` aliases.
- Started `0.36.10` by proving the local `test` fleet `app` subtree
  backup/restore operator path end to end. The run exposed and fixed restore
  runner ICP command generation: network flags now sit on the concrete leaf
  command, and fresh snapshot uploads no longer pass `--resume`.
- Extended `0.36.10` cycle reporting so `canic info cycles` shows explicit
  burn and top-up rates alongside net cycle movement in a compact default
  table, keeps wider diagnostics behind `--verbose`, and includes JSON fields
  for the derived burn and top-up per-hour values.
- Fixed full non-root deployment backup manifest finalization so root-omitted
  sibling branches are emitted as separate consistency units. The deployed
  local `test` fleet now completes `canic backup create test` with six
  non-root targets, and the resulting layout passes status and verification.
- Normalized `canic backup list` timestamps for unfinished execution layouts:
  failed/running rows use unix markers from execution journals when available,
  legacy run-id stamps are converted to unix markers before display, and local
  stale backup artifact directories were removed so only the verified complete
  `test` backup remains.
- Started `0.36.11` by proving the full six-canister `test` fleet restore path
  from backup row `1`: verify backup, plan with readiness gates, apply journal,
  dry-run, one-step execute/resume, full execute, require-complete, and final
  `canic info list test` readiness.
- Added `canic backup prune` for explicit operator cleanup of backup
  directories. The first selectors are `--failed` and `--keep <count>`, with
  `--dry-run` previews and backup-list ordering.
- Started `0.36.12` by removing the `/tmp` restore choreography: restore
  plan/apply/run now accept backup-list row references, `restore prepare`
  writes default plan and apply-journal files inside the backup layout, and
  `restore status` exposes completion/attention gates for prepared restores.
- Started `0.36.13` by polishing the restore row-reference operator path:
  command help and docs now lead with `restore prepare/status/run <backup-ref>`,
  and missing prepared plan or apply-journal defaults fail with explicit
  `canic restore prepare <backup-ref>` guidance instead of raw file IO errors.
- Started `0.36.14` by making row-reference restore run/status verify that the
  prepared apply journal's `backup_root` points back at the selected backup
  directory, so copied or stale journals cannot silently read restore artifacts
  from a different backup layout.
- Started `0.36.15` by adding `restore status/run --require-ready`, giving
  operators and CI a pre-mutation guard that writes the normal JSON summary and
  then fails if the prepared apply journal is blocked or not ready.
- Closed the active 0.36 implementation track after the `0.36.15` readiness
  guard. Further backup/restore work should be bug fixes or changes proven by
  real operator use, not additional v1 scope expansion.
- Started `0.37.0` by rerunning the refreshed `bootstrap-lifecycle-symmetry`
  audit at
  `docs/audits/reports/2026-05/2026-05-16/bootstrap-lifecycle-symmetry.md` and
  fixed the non-root post-upgrade continuation path so config/auth continuation
  failures return typed errors through the lifecycle adapter instead of
  panicking inside workflow runtime.
- Refreshed and reran the next oldest recurring audit,
  `canonical-auth-boundary`, at
  `docs/audits/reports/2026-05/2026-05-16/canonical-auth-boundary.md`. It found
  no boundary bypass and now explicitly checks current macro/core auth paths,
  required scopes, update replay consumption, and private token-material helper
  limits.
- Exported `DelegatedToken` from `canic::prelude` so normal authenticated
  endpoint modules do not need a separate DTO import.
- Added a config-schema regression proving obsolete per-canister delegated-auth
  verifier tables are rejected instead of accepted through compatibility shims.
- Updated the internal audit scaling probe to use `scale_replica` and
  `policy.initial_workers = 0` so the dry-run planning probe no longer tries
  to allocate startup workers in a standalone PocketIC scenario.
- Refreshed and reran the layer boundary audit at
  `docs/audits/reports/2026-05/2026-05-16/layer-boundary.md`. It found and
  fixed two core layering drifts: workflow no longer imports module-source
  runtime types from the API layer, and cycles authorization no longer depends
  on storage `CanisterRecord` shapes. The CI layering guard now catches both
  regression classes.
- Added and ran the workflow purity audit at
  `docs/audits/reports/2026-05/2026-05-16/workflow-purity.md`. It moved
  cycles-funding policy into `domain/policy`, moved the mutable funding ledger
  into ops, moved HTTP and management DTO conversion helpers into ops, and
  added a layering guard against workflow-defined policy types.
- Added and ran the ops purity audit at
  `docs/audits/reports/2026-05/2026-05-16/ops-purity.md`. It renamed delegated
  auth certificate validation from an ops-owned policy surface to explicit
  certificate rules and documented RPC, auth, metrics, and atomic storage ops
  as accepted hotspots with watchpoints.
- Added and ran the access purity audit at
  `docs/audits/reports/2026-05/2026-05-16/access-purity.md`. It moved stable
  app-mode facts and whitelist config reads behind ops helpers, added an
  access storage/stable-type layering guard, and documented delegated-token
  boundary decode plus delegated-session cleanup as watchpoints.
- Added and ran the security-boundary ordering audit at
  `docs/audits/reports/2026-05/2026-05-16/security-boundary-ordering.md`. It
  found no critical ordering violation and added guards for authenticated
  endpoint macro access-before-dispatch ordering plus cached root response
  attestation payload subject binding.
- Started `0.37.2` by restoring stable-memory ABI tracking in `canic-memory`:
  ID `0` now stores a persisted layout ledger, and historical range or ID drift
  is rejected even if the old declaration is removed from the current binary.
- Started the `0.38.0` hard-cut by making ID `0` the canonical ledger
  self-record, treating IDs `1-99` as Canic framework expansion budget, and
  widening `canic-core` to `11-99`. The later `0.39` hard cut removed the
  named downstream application authority range from Canic policy.
- Added explicit stable-memory ABI keys for Canic-managed memory declarations
  so package, module, type, or label renames do not silently allocate new
  stable memories or strand old ones.
- Started the 0.38 stable-memory ABI design at
  `docs/design/0.38-stable-memory-abi/0.38-design.md` so this work can move as
  an urgent minor instead of remaining a patch-level cleanup note.
- Added current declaration-snapshot validation so duplicate memory IDs,
  duplicate stable keys, and exact duplicate declarations fail before user
  ledger records are committed during bootstrap.
- Added historical-ledger preflight for pending bootstrap claims so failed
  bootstrap validation cannot persist earlier user claims from the same
  snapshot before a later historical conflict is discovered.
- Reworked the persisted layout ledger into a generation-framed store with two
  committed slots, generation checksums, header metadata, and highest-valid
  generation selection.
- Ledger mutation, validation, and diagnostic snapshots now fail closed if no
  committed generation validates.
- Tightened namespace enforcement so non-Canic crates cannot claim `canic.*`
  stable keys even if they choose IDs inside the framework range.
- Split public `MemoryApi` declaration from opening: startup code can declare
  explicit-key slots before bootstrap, and post-bootstrap calls only open
  already-validated slots instead of creating new ledger claims.
- Split `ic_memory_key!` macro declaration from opening as well: constructors
  queue declaration descriptors before registry validation, and eager stable
  stores open virtual memory only after the runtime registry is validated.
- Made the macro open guard target-independent and added host-test bootstrap
  hooks for core and control-plane tests so unit tests validate before opening
  stable-store handles.
- Added `MemoryApi::ledger_snapshot()` as a first diagnostic read path over
  persisted ABI ledger history that does not depend on current registry
  reconstruction.
- Started the post-`0.38.0` ABI diagnostics follow-up by adding optional
  `schema_version` and `schema_fingerprint` metadata to managed memory
  declarations, registry DTOs, and ledger declaration history. Metadata remains
  informational and is not part of allocation identity.
- Added canonical allocation authority records to the old ABI ledger for the
  previous Canic framework/application boundary, exposed through
  `MemoryApi::ledger_snapshot()` diagnostics. The current native `ic-memory`
  path now reports only reserved infrastructure ranges owned by Canic policy.
- Tightened ABI ledger physical-header validation so invalid magic, format,
  schema version, header length, or committed slot metadata fails closed during
  bootstrap instead of being repaired.
- Added raw stable-memory preflight before declaration-snapshot mutation:
  brand-new memory may initialize the genesis ledger, while foreign or corrupt
  raw memory and existing `MemoryManager` state without a valid ID `0` Canic
  ABI ledger fail closed.
- Tightened the wasm `MemoryApi::ledger_snapshot()` diagnostic path so it
  decodes only the ID `0` ABI ledger from raw stable memory and does not depend
  on normal runtime registry reconstruction.
- Started `0.38.2` by adding a controller-only `canic_memory_ledger`
  diagnostic query for opt-in memory observability builds. It bypasses normal
  Canic endpoint dispatch and exposes committed ID `0` ledger header fields,
  the authoritative committed generation, authorities, ranges, and memory
  records through a dedicated DTO.
- Started `0.38.3` by moving `canic_memory_ledger` into the default Canic
  runtime endpoint bundles, including the canonical `wasm_store` surface, while
  keeping the heavier live `canic_memory_registry` diagnostic opt-in.
- Started `0.38.4` by extending the source-level stable-memory ABI guard across
  the Canic-managed runtime surface, including the canonical `wasm_store`, and
  clarifying `canic-memory` documentation around declaration, bootstrap, and
  post-validation opening phases.
- Started `0.38.5` by aligning current stable-memory ABI documentation around
  the final Canic-managed memory contract and clarifying that IDs `1-4` are
  range-protected metadata expansion budget, not canonical per-ID reserved
  records.
- Folded a `canic info cycles` freshness fix into `0.38.5`: when live cycle
  balance data is available, cycle summaries now derive deltas and rates
  through the live balance timestamp so post-sample auto-top-up events are
  visible before the next hourly tracker sample.
- Started `0.38.6` by adding persisted ABI ledger `layout_epoch` validation
  and exposing the compiled epoch through `MemoryApi::ledger_snapshot()`, core
  memory DTOs, `canic_memory_ledger`, and the canonical `wasm_store` DID.
- Started `0.38.7` by hard-cut reallocating `canic.core.app_state.v1` from ID
  `62` to ID `18`, colocating app runtime state with core env and subnet state
  before the 0.38 stable-memory ABI layout is treated as frozen.
- Reworked the PR #8 topology direction for `0.38.7`: local ICP network
  settings such as `ii` and `nns` remain in `icp.yaml`; the later `0.38.8`
  cleanup made Canic's ICP project config checks read-only.
- Started `0.38.8` by stopping Canic from deriving or rewriting `icp.yaml` from
  `canic.toml`, making `canic status` check ICP project config read-only,
  pinning the checked-in local ICP network launcher to
  `v13.0.0-2026-05-07-04-27`, and adding an upstream watch workflow that fails
  when a newer launcher tag appears, prompting a test for the delegation
  certificate fix from upstream `dfinity/ic` commit `17524c56`.
- Started `0.38.9` after `0.38.8` was published by removing the misleading
  `canic fleet sync` command and replacing it with `canic fleet check <name>`.
- Folded hidden-support cleanup into `0.38.9`: renamed the hidden `canic-core`
  `__control_plane_core` bridge to `control_plane_support`, moved neutral
  formatting to hidden `shared_support::format`, and removed the broad
  `core_support` caller aliases from `canic-control-plane`.
- Started `0.39.0` by adding the root `ic-memory` crate as the future
  standalone repository boundary. The first slice includes generic stable-key
  parsing, allocation-slot descriptors, schema metadata, declaration
  collection/sealing, policy and substrate traits, validated allocation
  sessions, generation/ledger data shapes, and diagnostic export shapes without
  depending on Canic or `canic-cdk`.
- Extended the `0.39.0` generic crate with allocation-history validation and
  pure logical generation staging. Current declarations are now checked against
  policy, stable-key history, slot history, and retired allocation tombstones,
  while omitted historical records remain owned and active.
- Added a source-level guard test that rejects implicit registration, direct
  raw stable-memory APIs, independent `MemoryManager` access, and
  `RestrictedMemory` carve-outs in Canic-managed runtime crates.
- Split root install guidance into `INSTALLING.md` and refreshed README
  examples around the current `canic info list` command group.
- Renamed the test fleet scaling worker role from `scale` to `scale_replica`,
  changed role cycle config from `topup_policy` to `topup`, and enabled explicit
  default `topup = {}` policy blocks for the main test app, hub, shard, and
  scaling roles.
- Slimmed the ICP build hook path: `icp.yaml` now invokes
  `cargo run -q -p canic-host --example build_artifact -- <role>` directly,
  the Rust builder owns `ICP_WASM_OUTPUT_PATH` copying, and the old
  `scripts/app/build.sh` wrapper has been removed.
- Tightened local replica ownership checks so `canic replica start --background`
  and `canic status` use project-scoped ICP network status instead of broad
  local ping, while `canic replica stop` distinguishes "this project is already
  stopped" from "port 8000 is owned by a different ICP network/project".
- Added configured local gateway port output to `canic status` and
  `canic replica status`, plus `canic replica start --port <port>` to update
  this project's `icp.yaml` `gateway.port` before starting.
- Hard-cut the managed child controller policy for 0.35.1: newly allocated
  non-root canisters now receive configured controllers, root, and their direct
  parent as controllers; pool reuse updates the controller set before install.
- Tightened `canic install <fleet>` build output by hiding unset requested
  profile noise, using operator labels for build context, omitting duplicate
  ICP root context, adding `WASM_GZ` sizes to the build table, and making
  local root top-up output show the checkpoint phase, exact amount, and target.
- Added explicit restore-run stop/start phases so apply journals now schedule
  snapshot upload, target stop, snapshot load, target start, and verification
  operations instead of depending on manual canister state changes.
- Completed the 0.33 ICP CLI hard cut: `icp.yaml`, `.icp`, ICP CLI install/list/
  medic/snapshot/restore flows, native replica controls, and project status.
- Removed default fleet/network state and the old public `canic network`
  command; fleet-scoped commands take positional fleet names.
- Made the standard pre-1.0 `canic` facade capabilities default so fleet
  canisters no longer choose Canic feature flags manually.
- Trimmed the public metrics surface into role-inferred profiles and tiered
  selectors while keeping metrics enabled by default before 1.0.
- Added `canic endpoints` with Candid method/argument output and changed
  generated Candid finalization to require a trailing `canic::finish!()`.
- Made `canic endpoints` fleet-scoped and moved `--icp <path>` and
  `--network <name>` to top-level-only CLI options; command-local placement is
  hard-rejected instead of kept as a hidden compatibility path.
- Removed low-value list/config selectors: `canic list --root` is gone,
  `canic list --from` is now `canic list --subtree`, and `canic config --from`
  is gone.
- Removed `canic endpoints --did`; endpoint lookup now uses fleet metadata and
  known local role `.did` artifacts only, and registered principals infer their
  fallback role from the fleet registry instead of taking `--role`.
- Removed `KIND` from the live `canic list` table, added `CYCLES` in `0.33.6`,
  and added `CANIC` in `0.33.7`; version and cycle balances now use parallel
  `icp canister call canic_metadata` and `canic_cycle_balance` reads.
- Replaced the separate generated `canic_canister_version` and
  `canic_standards` endpoints with a single `canic_metadata` endpoint that
  includes package metadata, Canic version, and IC canister version.
- Local root installs keep a `100.00 TC` root ready target, including
  pre-bootstrap and post-ready top-up checkpoints for reused local root
  canisters.
- Grouped `snapshot`, `backup`, `manifest`, and `restore` under a dedicated
  backup/restore section in the top-level `canic help` output.
- Fixed local `canic snapshot download <deployment>` target discovery to use
  decoded local replica registry queries instead of parsing the ICP CLI
  transport JSON wrapper.
- Fixed real snapshot-download id extraction to use
  `icp canister snapshot create --quiet` and hex-only parsing, preventing table
  units such as `MiB` from being treated as snapshot ids.
- Removed `--resume` from fresh snapshot downloads and documented the 0.34
  backup/restore redesign around root-stays-up subtree backup phases.
- Centralized byte-size and TC cycle formatting through shared format helpers
  so list and config output use the same labels.
- Removed public install overrides: `canic install` is now just
  `canic install <fleet>` with fleet config, root target, and readiness timeout
  owned by Canic.
- Added hard fleet identity checks: duplicate discovered `[fleet].name` values
  fail config discovery, and install requires the config identity to match the
  requested fleet directory.
- Moved the `minimal` shared-runtime baseline under `canisters/audit` and made
  `canic status` compare local deployments against bootstrap-required roles.
- Refreshed the module-structure audit and reduced the current structural risk
  readout to `3/10`.
- Split current 0.33 hotspots in `canic-core` IC management/provisioning,
  `canic-control-plane` publication, and `canic-backup` restore
  runner/apply-journal internals into normal directory modules.
- Ran the oldest outstanding recurring audit, `change-friction`, against the
  current 0.33 line. It reports medium friction risk at `5/10`: the broad
  DFX-to-ICP CLI hard cut raised patch radius, but no cross-layer leakage was
  confirmed. The rerun after reloading ICP CLI used `icp 0.2.6`, clean snapshot
  `09f5d238`, and included the committed `0.33.7` metadata/list slice.
- Started remediating the change-friction follow-up by splitting `canic list`
  live registry projection into `crates/canic-cli/src/list/live.rs`, reducing
  the command root from the audited `902` lines to `506` lines.
- Deduplicated `canic list` table width/separator/alignment rendering through
  `crates/canic-cli/src/list/table.rs` for both config and registry tables.
- Deduplicated the live-list threaded query collector used by local readiness,
  `canic_metadata` version reads, and `canic_cycle_balance` reads.
- Centralized list config-loader host-config error mapping so adding config
  table columns does not repeat install-state conversion boilerplate.
- Split list endpoint response parsing into `crates/canic-cli/src/list/parse.rs`
  so metadata and cycle-balance response-shape tests live beside the parsers
  rather than the live transport code.
- Promoted table rendering to `canic-host::table` and routed list, status,
  fleet-list, backup-list, medic, and install config-choice tables through one
  host/operator header/underline/spacing/alignment helper.
- Split deployed-registry tree traversal into `crates/canic-cli/src/list/tree.rs`
  so `list/render.rs` no longer owns hierarchy selection and presentation at
  the same time.
- Split host root readiness polling and diagnostics into
  `crates/canic-host/src/install_root/readiness.rs`, reducing
  `install_root/mod.rs` from `901` to `586` lines while preserving the install
  orchestration flow.
- Started the 0.34 backup/restore rework by adding `canic-backup::plan` with
  typed backup plans, targets, operations, authority/read preflights,
  quiescence policy, and operation receipts. This is a model-only slice; live
  snapshot execution is unchanged.
- Split backup plan validation from execution readiness: plans can represent
  `Proven`, `Declared`, or `Unknown` control/read authority for dry-run output,
  while mutating backup execution requires proven authority for every selected
  target.
- Added target-scoped control and snapshot-read authority preflight receipts so
  future execution can upgrade a plan only after proof covers every selected
  target.
- Added typed authority preflight request DTOs derived from `BackupPlan`, giving
  root coordination and host-side authority adapters a stable input contract.
- Added typed topology and quiescence preflight request/receipt DTOs plus
  execution-gate validation for topology drift, target-set changes, policy
  mismatches, and rejected quiescence.
- Added a full execution preflight receipt bundle so future backup execution can
  apply authority receipts and validate topology/quiescence gates through one
  typed boundary.
- Added `preflight_id`, `validated_at`, and `expires_at` to preflight receipts
  and the execution preflight bundle so stale or cross-preflight evidence cannot
  authorize later mutation.
- Added `canic-backup::execution` with a model-only backup execution journal
  built from `BackupPlan` phases, including preflight acceptance, ordered
  operation transitions, durable operation receipts, retryable failures, resume
  summaries, and `restart_required` tracking after stops.
- Added typed preflight receipt-bundle acceptance to the execution journal so
  mutation cannot be unblocked by a bundle from a different plan.
- Added `BackupLayout` read/write support for
  `backup-execution-journal.json`, keeping phase execution progress separate
  from the existing artifact download journal.
- Added `BackupLayout` read/write support for `backup-plan.json` so future
  backup runners can resume against the exact validated plan instead of
  reconstructing the operation graph.
- Added execution-layout integrity verification that rejects a persisted
  execution journal when its plan/run ids or operation graph no longer match
  the stored `backup-plan.json`.
- Added the first `canic backup create <fleet> --dry-run` CLI path, including
  optional `--subtree <role-or-principal>` planning, installed-fleet registry
  discovery, persisted `backup-plan.json`, persisted
  `backup-execution-journal.json`, and a compact dry-run summary table while
  keeping real mutation disabled.
- Made `canic backup list` include plan-only dry-run directories as
  `STATUS=dry-run`, using the persisted plan id as `BACKUP_ID` and planned
  target count as `MEMBERS`.
- Made `canic backup status --dir <dry-run-dir>` understand dry-run
  `backup-plan.json` plus `backup-execution-journal.json` layouts and report
  execution-journal progress while `--require-complete` still rejects them as
  non-backups.
- Added `canic backup inspect --dir <dry-run-dir>` with table and JSON output
  for plan metadata, selected targets, authority evidence, operation order, and
  execution-journal state.
- Added a `#` column to `canic backup list` so operators can refer to visible
  backup rows by a short ordinal as well as by `BACKUP_ID`.
- Made `canic backup inspect`, `canic backup status`, and
  `canic backup verify` accept either the `canic backup list` row number or
  `BACKUP_ID` as a positional backup reference, with `--dir <dir>` kept for
  explicit paths and ambiguous backup ids rejected fail-closed.
- Made `canic backup verify` reject dry-run plan layouts with the typed
  `DryRunNotComplete` error instead of falling through to a missing-manifest
  filesystem error.
- Added registry-backed backup plan construction for explicit subtrees and
  non-root fleet scopes, including top-down stop/snapshot phases, bottom-up
  start phases, and post-restart download/verify/finalize phases.
- Added backup selector resolution for explicit principals and unambiguous
  roles, rejecting missing or ambiguous role selectors before planning.
- Reran the oldest latest-run lightweight recurring audit, `publish-surface`,
  at `docs/audits/reports/2026-05/2026-05-11/publish-surface.md`. It reports
  package-surface risk `3/10`: all 11 publishable crates package and verify.
- Completed the publish-surface follow-up by aligning `crates/canic/README.md`
  with the default facade features and refreshing the recurring audit's
  canonical published crate map.
- Ran the full-codebase DRY consolidation audit for 2026-05-12. It reports
  medium consolidation risk at `5/10`, with installed-fleet resolution and
  large CLI command modules as the highest-value follow-ups.
- Added `canic-host::installed_fleet` with `InstalledFleetResolution`,
  `InstalledFleetSource`, `InstalledFleetRegistry`, and
  `ResolvedFleetTopology`, then routed `canic list`, `canic cycles`,
  `canic metrics`, and `canic endpoints` through the shared installed-fleet
  resolver.
- Split `canic endpoints` into command orchestration, endpoint model, Candid
  parsing, transport, and rendering modules while keeping behavior unchanged.
- Split `canic cycles` into command orchestration, options, response parsing,
  transport/report collection, rendering, and model modules while keeping
  behavior unchanged.
- Split `canic metrics` into command orchestration, options, response parsing,
  transport/report collection, rendering, and model modules while keeping
  behavior unchanged.
- Split top-level CLI command catalog/help rendering and global option
  forwarding out of `canic-cli::lib`, leaving the root focused on command
  dispatch and error mapping.
- Moved shared ICP response parsing primitives from `canic-cli` to
  `canic-host::response_parse`, and switched CLI list/cycles/metrics parsers to
  import the host-owned helpers directly.
- Moved the live subnet registry DTO/parser from `canic-backup::discovery` to
  `canic-host::registry`.
- Promoted the shared installed-fleet resolver to `canic-host::installed_fleet`;
  CLI list/cycles/metrics/endpoints now consume host-owned install-state
  lookup, local replica preference, ICP CLI fallback, registry parsing, and
  topology projection.
- Split the old `canic-cli::args` module into the `canic-cli::cli` directory
  with `clap`, `defaults`, `help`, and `globals` modules, removing the broad
  argument-helper drawer while preserving command behavior.
- Moved `path_stamp` and `registry_tree` under `canic-cli::support` to keep the
  `canic-cli` crate root focused on command families and explicit support
  modules.
- Split `canic-cli::backup` command-family help and report rendering into
  `backup::command` and `backup::render`; `backup::mod` is down to about
  `1050` lines.

## Current Memory Boundary

- Canic no longer maintains a live local allocation registry. Macro/static
  declarations and the small ad hoc pending queue are declaration inputs only.
- Runtime bootstrap collects declarations, validates and commits them through
  the native `ic-memory` durable ledger with Canic policy, publishes
  `ValidatedAllocations`, and only then opens stable-memory handles.
- `ic-memory` owns generic allocation validation: stable-key grammar, schema
  metadata bounds, `MemoryManager` ID shape and ID `255` rejection, duplicate
  declaration keys/slots, historical stable-key movement rejection, physical
  slot reuse rejection, and retired/tombstone rejection when represented in the
  native ledger.
- Canic still owns `canic.*` namespace policy, framework reserved IDs,
  rejection of application claims against reserved ranges, lifecycle ordering,
  eager TLS touches, and diagnostic DTO shaping. `ic-memory` owns
  `ic_memory.*` authority checks, declaring-crate/range composition, and
  validated handle opening.
- Canic no longer preserves the old Canic physical allocation ledger format.
  There is no projection bridge or dual-read compatibility path in the current
  hard cut; old allocation-ledger bytes require a separate migration or
  destructive reset tool before a future compatible boot.
- The opt-in live `canic_memory_registry` endpoint and DTOs have been removed.
  `canic_memory_ledger` is the single supported memory diagnostic surface.

## Validation Recently Run

- `cargo fmt --all --check`
- `cargo test -p canic-core memory --lib`
- `cargo test -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo test -p canic-core memory::policy -- --nocapture`
- `cargo check --workspace`
- `cargo test -p canic --test protocol_surface`
- `git diff --check`
- `cargo fmt --all`
- wasm CI shell helpers syntax check
- `cargo check -p canic-host --examples`
- `cargo check -p canic-host --examples -p canic-tests`
- `cargo test -p canic-host canister_build -- --nocapture`
- `cargo clippy -p canic-host --examples -- -D warnings`
- `cargo check -p canic-core -p canic-host -p canic-testing-internal -p canister_scale`
- `cargo test -p canic build_support -- --nocapture`
- `cargo test -p canic-core config::schema -- --nocapture`
- `cargo test -p canic-core config::schema::subnet -- --nocapture`
- `cargo test -p canic-host release_set -- --nocapture`
- `cargo test -p canic-host install_root::tests::config_selection -- --nocapture`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo clippy -p canic -p canic-core -p canic-host -p canic-testing-internal --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-core workflow::ic::provision::allocation -- --nocapture`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo check -p canic-host`
- `cargo test -p canic-host install_root -- --nocapture`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-backup restore -- --nocapture`
- `cargo test -p canic-cli restore -- --nocapture`
- `cargo check -p canic-backup -p canic-cli`
- `cargo clippy -p canic-backup -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo test -p canic-cli snapshot -- --nocapture`
- `cargo test -p canic-cli replica -- --nocapture`
- `cargo test -p canic-cli status -- --nocapture`
- `cargo test -p canic-host icp -- --nocapture`
- `cargo test -p canic-host icp_config -- --nocapture`
- `cargo test -p canic-host replica_query -- --nocapture`
- `cargo clippy -p canic-cli -p canic-host --all-targets -- -D warnings`
- `cargo run -p canic-cli -- status`
- `cargo run -p canic-cli -- replica status`
- `cargo test -p canic-host snapshot_id -- --nocapture`
- `cargo test -p canic-host snapshot -- --nocapture`
- `cargo test -p canic-backup discovery -- --nocapture`
- `cargo test -p canic-backup snapshot -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-host`
- `cargo test -p canic-host cycle -- --nocapture`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `cargo build -p canic-cli --bin canic`
- `time target/debug/canic list test`
- `target/debug/canic list test`
- `target/debug/canic install demo`
- `target/debug/canic list demo`
- `target/debug/canic snapshot download demo --dry-run`
- `cargo run -q -p canic-cli --bin canic -- endpoints test app`
- `cargo run -q -p canic-cli --bin canic -- endpoints test app --json`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo test -p canic --test canic_metadata -- --nocapture`
- `cargo check -p canic`
- `cargo clippy -p canic --all-targets -- -D warnings`
- `cargo check -p canic-wasm-store`
- `cargo test -p canic-core --lib -- --nocapture`
- `cargo test -p canic-core --lib workflow::ic -- --nocapture`
- `cargo test -p canic-core --lib ops::ic -- --nocapture`
- `cargo check -p canic-control-plane`
- `cargo clippy -p canic-control-plane --all-targets -- -D warnings`
- `cargo test -p canic-control-plane --lib -- --nocapture`
- `cargo check -p canic-backup`
- `cargo clippy -p canic-backup --all-targets -- -D warnings`
- `cargo test -p canic-backup --lib -- --nocapture`
- `cargo test -p canic-backup plan -- --nocapture`
- `cargo test -p canic-backup execution -- --nocapture`
- `cargo test -p canic-backup persistence -- --nocapture`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo run -q -p canic-cli --bin canic -- backup create demo --dry-run --out /tmp/canic-backup-plan-demo`
- `cargo run -q -p canic-cli --bin canic -- backup create demo --subtree app --dry-run --out /tmp/canic-backup-plan-demo-app`
- `cargo run -q -p canic-cli --bin canic -- backup list`
- `cargo package -p canic -p canic-backup -p canic-cli -p canic-control-plane -p canic-core -p canic-host -p canic-macros -p canic-wasm-store --locked --allow-dirty`
- `cargo metadata --no-deps --format-version 1`
- `cargo run -q -p canic-cli --bin canic -- backup status --dir backups/deployment-demo-20260510-222116`
- `cargo test -p canic-cli endpoints -- --nocapture`
- `cargo test -p canic-cli cycles::tests -- --nocapture`
- `cargo test -p canic-cli metrics::tests -- --nocapture`
- `cargo test -p canic-cli usage_lists_command_families -- --nocapture`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli version_flags_return_ok -- --nocapture`
- `cargo test -p canic-cli global_ -- --nocapture`
- `cargo test -p canic-host install_root -- --nocapture`
- `cargo test -p canic-cli list::parse -- --nocapture`
- `cargo clippy -p canic-host -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-cli installed_fleet -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-host -p canic-backup -p canic-cli`
- `cargo test -p canic-host registry -- --nocapture`
- `cargo test -p canic-host installed_fleet -- --nocapture`
- `cargo test -p canic-backup --lib -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-host -p canic-backup -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-cli`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo run -q -p canic-cli --bin canic -- backup inspect --dir backups/deployment-demo-20260510-222116`
- `cargo run -q -p canic-cli --bin canic -- backup inspect --dir backups/deployment-demo-20260510-222116 --json`
- `cargo run -q -p canic-cli --bin canic -- backup list`
- `cargo run -q -p canic-cli --bin canic -- backup inspect 1`
- `cargo run -q -p canic-cli --bin canic -- backup status 1`
- `cargo run -q -p canic-cli --bin canic -- backup verify 1`
- `cargo run -q -p canic-cli --bin canic -- backup inspect plan-demo-20260510-222116 --json`
- `cargo run -q -p canic-cli --bin canic -- backup status plan-demo-20260510-222116`
- `git show --stat --name-only --format=fuller 8a5814fd`
- `git show --stat --name-only --format=fuller cf24f77e`
- `git show --stat --name-only --format=fuller 53476764`
- `git show --stat --name-only --format=fuller 6ea85fdb`
- `git show --stat --name-only --format=fuller 5b474986`
- `icp --version`
- `git show --stat --name-only --format=fuller 09f5d238`
- `cargo test -p canic-cli list:: -- --nocapture`
- `cargo check -p canic-cli`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-host install_root::tests -- --nocapture`
- `cargo check -p canic-host`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `bash scripts/ci/instruction-audit-report.sh`
- `cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture`
- `cargo test -p canic-core --lib verify_delegated_token_rejects_audience_subset_drift -- --nocapture`
- `cargo test -p canic-core --lib verify_delegated_token_rejects_missing_local_role_for_role_audience -- --nocapture`
- `cargo test -p canic-core --lib prepare_delegated_token_rejects_audience_expansion -- --nocapture`
- `cargo test -p canic-core config::schema::subnet::tests::canister_config_rejects_legacy_delegated_auth_table -- --nocapture`
- `cargo test -p canic-core config::schema -- --nocapture`
- `cargo check -p canic-control-plane -p canic -p canic-tests --tests`
- `cargo test -p canic-control-plane publication -- --nocapture`
- `cargo test -p canic-tests --test root_wasm_store_reconcile -- --test-threads=1 --nocapture`
- `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture`
- `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture`
- `cargo fmt --all --check`
- `cargo check -p canic-tests --tests`
- `git diff --check`

## Known Worktree Notes

- The worktree is intentionally dirty during active slice work.
- Do not revert unrelated edits.
- Agents must not stage, commit, push, bump versions, or run release targets.

## Cost-Control Rules

- Prefer scoped searches over broad repo searches.
- Avoid searching `docs/changelog/**`, `docs/audits/reports/**`, and generated
  outputs unless the task is specifically about those files.
- Write detailed findings to files; summarize only the high-signal result in
  chat.
- Keep final responses concise and include validation commands actually run.

## Good Next Tasks

1. Start 0.44 with promotion planning and readiness only: artifact-source
   model, digest-pinned override inputs, receipt-backed artifact references,
   and target embedded-config validation.
2. Keep authority dry-run artifacts out of promotion artifact sources.
   `AuthorityReceiptV1` and `AuthorityDryRunEvidenceV1` remain structural
   authority-reporting evidence only.
3. Do not add promoted-plan execution shortcuts. If promoted plans execute
   through current install, they must use the 0.43.8 private operation runner
   and deployment-truth/preflight gate path.
4. Preserve 0.44's core rule: promote artifact identity, not source authority
   or stale embedded topology.

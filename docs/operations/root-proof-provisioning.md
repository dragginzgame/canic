# Root Proof Provisioning Runbook

This runbook is the compact developer handoff for delegated-auth root proof
provisioning. It documents the active 0.76 chain-key path, not the historical
bridge-backed canister-signature flow.

## Source Map

| Concern | Source |
| --- | --- |
| Root endpoint macro surface | `crates/canic/src/macros/endpoints/root.rs` |
| Issuer endpoint macro surface | `crates/canic/src/macros/endpoints/nonroot.rs` |
| Public auth API adapters | `crates/canic-core/src/api/auth/mod.rs` |
| Chain-key batch proof state and lookup | `crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs` |
| Root issuer renewal templates, attempts, and status | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/` |
| Root renewal timer orchestration | `crates/canic-core/src/workflow/runtime/auth/renewal.rs` |
| Root chain-key management-canister signing | `crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs` |
| Root batch install broadcast workflow | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` |
| Issuer active proof verification | `crates/canic-core/src/ops/auth/delegated/active_proof.rs` |
| Issuer-local canister-signature proof support | `crates/canic-core/src/ops/auth/issuer_canister_sig.rs` |
| Operator renewal CLI | `crates/canic-cli/src/auth/` |
| PocketIC hard-cut coverage | `crates/canic-tests/tests/root_cases/auth_076.rs` |
| Active architecture contract | `docs/architecture/authentication.md` |
| Wire/protocol contract | `docs/contracts/AUTH_DELEGATED_SIGNATURES.md` |

## Supported Flow

Root-owned renewal is the active delegated-auth liveness path for issuers with
enabled renewal templates:

```text
controller             -> root canic_upsert_root_issuer_renewal_template update
root timer             -> root prepares due issuer renewal attempts
root                   -> management canister sign_with_ecdsa
root                   -> issuer canic_install_active_delegation_proof update
operator/medic         -> root canic_root_issuer_renewal_status query
operator/medic         -> issuer canic_active_delegation_proof_status query
caller/session         -> issuer canic_prepare_delegated_token update
caller/session         -> issuer canic_get_delegated_token query
```

Issuer delegated-token preparation also has a root lazy-repair path. When an
issuer in `chain_key_batch` mode has no usable active proof, it may request the
internal root update:

```text
issuer                 -> root canic_get_or_create_chain_key_delegation_proof update
root                   -> management canister sign_with_ecdsa when no reusable batch exists
root                   -> issuer canic_install_active_delegation_proof update
```

Lazy repair must reuse a valid existing chain-key batch when possible and must
honor signing retry-after state after management-canister failures. It must not
fall back to per-login signing or to the old bridge-backed proof shape.

## Operator Surface

The retained operator command is status-only:

```bash
canic auth renewal status <deployment> --issuer <principal>
canic auth renewal status <deployment> --issuer <principal> --json
canic medic deployment <deployment> --auth-renewal <principal>
```

The status command reports root-owned template/state/attempt data and, when the
issuer status endpoint is locally available, compares root's last installed
proof hash and expiry against the issuer's active proof. A drift warning is
observational; it does not mutate root or issuer state.

## Hard Invariants

- Delegated-token root proofs use `RootProof::IcChainKeyBatchSignatureV1` in
  hard-cut mode.
- Root chain-key proof batches are signed by root through the management
  canister ECDSA API.
- Subnet, root canister id, issuer canister id, key id, derivation path,
  audience, grants, and certificate window are explicit verifier inputs.
- Root timer renewal and issuer lazy repair share the same batch state and must
  deduplicate management-canister signing work.
- Issuers verify root proof material before storing active proof state.
- Product frontends must not orchestrate root proof renewal.
- Operator status and medic output must not trigger signing or install work.
- Legacy bridge-backed root proof provisioning endpoints are not part of the
  active protocol surface.
- Role-attestation and issuer-local canister-signature proof support remain
  separate from delegated-token root proof liveness.

## State Bounds

Root renewal and chain-key proof state are bounded:

- max 64 issuers per root signing batch; additional due issuers wait for a
  later batch after the current batch is signed and installed
- max 128 non-expired, non-installed chain-key proof batches; a full pending
  batch queue rejects new batch creation with resource exhaustion
- max one active renewal attempt per issuer
- certificate TTL bounded by the enabled renewal template and root policy
- signing retry-after recorded after retryable management-canister failures

Prepared signing state is terminally resolved by callback success, callback
failure, stale-callback discard, expiry, or install result recording. Stale
issuer install failures must not overwrite a later successful install for the
same issuer.

## Status and Repair

Use status before escalating a delegated-auth liveness issue:

```bash
canic auth renewal status <deployment> --issuer <principal> --json
```

Important status outcomes:

| Status | Meaning |
| --- | --- |
| `configured` | Root has a renewal template and no active drift is observed. |
| `active_attempt` | Root has in-progress renewal work for the issuer. |
| `disabled` | The renewal template is disabled. |
| `missing` | Root has no renewal template for the issuer. |
| `drift_detected` | Issuer-observed active proof differs from root state. |
| `unavailable` | CLI could not observe issuer-local status from local metadata or transport. |

For `active_attempt`, allow the root timer or lazy repair path to finish before
forcing additional work. For `drift_detected`, first re-check status to rule out
local metadata or transport staleness. If drift persists, repair by letting root
chain-key renewal or issuer lazy repair install a fresh active proof.

## Expected Failures

| Failure | Meaning |
| --- | --- |
| `RootChainKeyUnavailable` | Root cannot obtain or use the configured chain-key public key. |
| `RootChainKeySigningFailed` | Management-canister signing rejected or failed. |
| `RetryAfterActive` | A previous signing failure recorded retry-after state. |
| `TemplateDisabled` | The issuer renewal template was disabled while work was active. |
| `TemplateChanged` | Active renewal work no longer matches the enabled template. |
| `PolicyRejected` | Root issuer policy or request validation rejected renewal work. |
| `QuotaExceeded` | Bounded renewal or chain-key batch state is full. |
| `CallFailed` | Root could not reach the issuer install endpoint. |
| `RejectedBySigner` | Issuer rejected the proof after local verification. |
| `DriftDetected` | Issuer-observed active proof differs from root-managed renewal state. |

## Validation

Fast local checks for this surface:

```bash
cargo fmt --all -- --check
cargo check --locked -p canic-core -p canic -p canic-macros
cargo check --locked -p canic-cli
cargo test --locked -p canic --test protocol_surface
cargo test --locked -p canic-core chain_key --lib
git diff --check
```

PocketIC checks for end-to-end behavior:

```bash
POCKET_IC_BIN=/path/to/pocket-ic \
  cargo test --locked -p canic-tests --test root_suite auth_076 -- \
  --test-threads=1 --nocapture
```

PocketIC needs local port binding. In restricted sandboxes, run those tests with
the normal local-test allowance.

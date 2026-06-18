# Root Proof Provisioning Runbook

This runbook is the compact developer handoff for root proof provisioning. It
is operational guidance, not a new product surface.

## Source Map

| Concern | Source |
| --- | --- |
| Root endpoint macro surface | `crates/canic/src/macros/endpoints/root.rs` |
| Issuer endpoint macro surface | `crates/canic/src/macros/endpoints/nonroot.rs` |
| Public auth API adapters | `crates/canic-core/src/api/auth/mod.rs` |
| Root batch prepare/get and pending metadata | `crates/canic-core/src/ops/auth/delegation.rs` |
| Root batch install broadcast workflow | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` |
| Issuer active proof verification | `crates/canic-core/src/ops/auth/delegated/active_proof.rs` |
| Root canister-signature proof assembly | `crates/canic-core/src/ops/auth/root_canister_sig.rs` |
| PocketIC root provisioning coverage | `crates/canic-tests/tests/root_cases/sharding.rs` |
| Active architecture contract | `docs/architecture/authentication.md` |
| Wire/protocol contract | `docs/contracts/AUTH_DELEGATED_SIGNATURES.md` |

## Supported Flow

Provisioning is an explicit controller/operator action in the current MVP:

```text
controller/provisioner -> root canic_upsert_root_issuer_policy update
controller/provisioner -> root canic_prepare_delegation_proof_batch update
controller/provisioner -> root canic_get_delegation_proof_batch direct query
controller/provisioner -> root canic_install_delegation_proof_batch update
root                   -> issuer canic_install_active_delegation_proof update
caller/session         -> issuer canic_prepare_delegated_token update
caller/session         -> issuer canic_get_delegated_token query
```

Product frontends must not run the provisioning sequence during normal auth.
After install, normal delegated-token issuance remains issuer-local until the
active proof expires.

## Hard Invariants

- Root proof retrieval is a direct root query.
- Issuer composite-query wrappers must not retrieve root proof material.
- Root install updates must not assemble canister-signature proofs.
- Root verifies submitted install proofs against pending metadata before
  broadcasting them.
- Issuers verify the root proof again before storing active proof state.
- Root issuer policy registration is controller-only in the MVP.
- Retrieval authorization is controller-only in the MVP.
- Provisioner ACLs, root timers, issuer self-refresh, and retrieval tickets are
  deferred.

## Pending State Bounds

The MVP keeps root provisioning state bounded:

- max 64 issuers per prepare batch
- max 128 pending batches
- max 16 pending root delegation proofs per issuer
- max one-minute retrieval window
- cert TTL bounded by root issuer policy and `auth.delegated_tokens.max_ttl_secs`

Root prunes pending batch metadata opportunistically during prepare and install.
Uninstalled entries are removed after retrieval expiry. Installed entries stay
available for idempotent reinstall until certificate expiry. Signature-map leaf
pruning is intentionally outside the MVP.

## Status and Refresh

Provisioners should query issuer active-proof status before running the root
sequence. The status response exposes non-secret operational state:

- `Missing`
- `Valid`
- `RefreshNeeded`
- `Expired`
- root canister id
- issuer principal
- certificate hash
- expiry timestamp
- refresh-after timestamp

If status is `Missing`, `Expired`, or current time is past `refresh_after_ns`,
run prepare -> direct root query -> install. If status is `Expired`, issuer-local
delegated-token prepare should fail until refresh succeeds.

## Expected Failures

| Failure | Meaning |
| --- | --- |
| `RootDataCertificateUnavailable` | Retrieval did not run in a direct root query context. |
| `Invalid` during policy upsert | Submitted root issuer policy is malformed. |
| `Forbidden` during prepare | Issuer is unregistered, disabled, or outside root issuer policy. |
| `ResourceExhausted` during prepare | Batch or pending metadata quota is exhausted. |
| `ProofMismatch` during install | Submitted proof does not match pending root metadata. |
| `ExpiredOrSuperseded` during install | Retrieval or certificate window expired before install. |
| `CallFailed` during install | Root could not reach the issuer install endpoint. |
| `RejectedBySigner` during install | Issuer rejected the proof after local verification. |
| `AlreadyInstalled` during install | Repeated install is idempotent for that proof. |

## Validation

Fast local checks for the provisioning slice:

```bash
cargo fmt --all -- --check
cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture
cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture
cargo test --locked -p canic --test protocol_surface root_delegation_proof_batch -- --nocapture
cargo clippy --locked -p canic-core --lib -- -D warnings
git diff --check
```

PocketIC checks for end-to-end behavior:

```bash
TMPDIR="$PWD/.tmp/test-runtime" ICP_ENVIRONMENT=local \
  cargo test --locked -p canic-tests --test root_suite root_batch -- \
  --test-threads=1 --nocapture

TMPDIR="$PWD/.tmp/test-runtime" ICP_ENVIRONMENT=local \
  cargo test --locked -p canic-tests --test root_suite \
  root_unavailable_after_batch_install_does_not_break_signer_local_issuance -- \
  --test-threads=1 --nocapture
```

PocketIC needs local port binding. In restricted sandboxes, run those tests
with the normal elevated local-test allowance.

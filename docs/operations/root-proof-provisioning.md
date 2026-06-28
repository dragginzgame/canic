# Root Proof Provisioning Runbook

This runbook is the compact developer handoff for root proof provisioning. It
is operational guidance, not a new product surface.

## Source Map

| Concern | Source |
| --- | --- |
| Root endpoint macro surface | `crates/canic/src/macros/endpoints/root.rs` |
| Issuer endpoint macro surface | `crates/canic/src/macros/endpoints/nonroot.rs` |
| Public auth API adapters | `crates/canic-core/src/api/auth/mod.rs` |
| Root batch prepare/get and pending metadata | `crates/canic-core/src/ops/auth/delegation/mod.rs` and child modules |
| Root-managed renewal templates, attempts, and state | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal.rs` |
| Root batch install broadcast workflow | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` |
| Issuer active proof verification | `crates/canic-core/src/ops/auth/delegated/active_proof.rs` |
| Root canister-signature proof assembly | `crates/canic-core/src/ops/auth/root_canister_sig.rs` |
| Operator renewal CLI | `crates/canic-cli/src/auth.rs` |
| PocketIC root provisioning coverage | `crates/canic-tests/tests/root_cases/sharding.rs` |
| Active architecture contract | `docs/architecture/authentication.md` |
| Wire/protocol contract | `docs/contracts/AUTH_DELEGATED_SIGNATURES.md` |

## Supported Flow

Manual provisioning remains an explicit controller/operator action:

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

Root-managed renewal is the unattended long-term path for issuers with enabled
renewal templates:

```text
root timer             -> root prepares due scheduled renewal attempts
bridge/provisioner     -> root canic_delegation_renewal_work query
bridge/provisioner     -> root canic_get_delegation_renewal_proof_batch direct query
bridge/provisioner     -> root canic_install_delegation_proof_batch update
root                   -> issuer canic_install_active_delegation_proof update
operator/medic         -> root canic_root_issuer_renewal_status query
operator/medic         -> issuer canic_active_delegation_proof_status query
```

The bridge can be the CLI:

```bash
canic auth renewal run-once <deployment>
canic auth renewal status <deployment> --issuer <principal>
canic auth renewal provisioner list <deployment>
canic auth renewal provisioner enable <deployment> <principal>
canic auth renewal provisioner disable <deployment> <principal>
```

`run-once` completes already scheduled root work. It does not create renewal
intent, widen issuer policy, alter audiences or grants, or manufacture proof
material outside root's scheduled batch records.

## Hard Invariants

- Root proof retrieval is a direct root query.
- Issuer composite-query wrappers must not retrieve root proof material.
- Root install updates must not assemble canister-signature proofs.
- Root verifies submitted install proofs against pending metadata before
  broadcasting them.
- Issuers verify the root proof again before storing active proof state.
- Root issuer policy registration is controller-only in the MVP.
- Generic manual proof retrieval remains controller-only.
- Renewal provisioners may retrieve and install only root-scheduled renewal
  batches.
- Renewal provisioners cannot create renewal templates, issuer policies, or
  arbitrary proof references.
- Root-managed renewal status is authoritative for scheduling; issuer status is
  observational and used by CLI/medic for drift reporting.
- Product frontends must not run renewal.

## Pending State Bounds

Root provisioning and renewal state are bounded:

- max 64 issuers per prepare batch
- max 128 pending batches
- max 16 pending root delegation proofs per issuer
- max one-minute retrieval window
- cert TTL bounded by root issuer policy and `auth.delegated_tokens.max_ttl_secs`

Root prunes pending batch metadata opportunistically during prepare and install.
Uninstalled entries are removed after retrieval expiry. Installed entries stay
available for idempotent reinstall until certificate expiry. Signature-map leaf
pruning is intentionally outside the MVP.

Root-managed renewal also prunes expired scheduled renewal batch transport
records before preparing fresh work. Per-issuer renewal attempts remain stored
so expiry, failure, and repair status stay visible.

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

For root-managed renewal, operators should prefer the renewal status command:

```bash
canic auth renewal status <deployment> --issuer <principal>
```

The CLI reports root-owned template/state/attempt data and, when issuer status
is available, compares root's last installed proof hash/expiry against the
issuer's active proof. A drift warning is observational; it does not mutate root
state. Repair requires an explicit controller action, manual install, or bridge
run.

## Bridge Outage and Manual Repair

During a bridge outage, root keeps preparing only bounded scheduled work. If a
scheduled batch is not retrieved or installed before its window closes, root
records `RetrievalExpired` or `InstallDeadlineExpired`, clears the active
attempt when appropriate, and retries after backoff. Expired scheduled batch
transport records are pruned before fresh work is prepared, so a dead bridge
cannot permanently consume renewal batch quota.

Recovery sequence:

```bash
canic auth renewal status <deployment> --issuer <principal>
canic auth renewal run-once <deployment>
canic auth renewal status <deployment> --issuer <principal>
```

If status still reports drift after a successful bridge run, use the manual
controller prepare/get/install flow to repair the issuer active proof, then
check status again. Successful manual installs that exactly match the enabled
renewal template re-anchor root-managed renewal state.

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
| `RetrievalExpired` in renewal state | Bridge did not retrieve scheduled renewal proof material before the retrieval window closed. |
| `InstallDeadlineExpired` in renewal state | Bridge submitted scheduled renewal proof material after the install deadline. |
| `TemplateChanged` in renewal state | A scheduled attempt no longer matches the enabled renewal template. |
| `TemplateDisabled` in renewal state | The enabled template was disabled while work was active. |
| `QuotaExceeded` in renewal state | Prepare-stage renewal work hit bounded pending metadata limits. |
| `PolicyRejected` in renewal state | Prepare-stage renewal work was rejected by root issuer policy or request validation. |
| `DriftDetected` in CLI/medic output | Issuer-observed active proof differs from root-managed renewal state. |

## Validation

Fast local checks for the provisioning slice:

```bash
cargo fmt --all -- --check
cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture
cargo test --locked -p canic-core root_issuer_renewal --lib -- --nocapture
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
  root_unavailable_after_batch_install_does_not_break_issuer_local_issuance -- \
  --test-threads=1 --nocapture
```

PocketIC needs local port binding. In restricted sandboxes, run those tests
with the normal elevated local-test allowance.

# Canic Auth Survivorship Cleanup Audit

Baseline note: this report was captured before the follow-up hard-cut cleanup
that split role-attestation root proofs from delegated-token `RootProof` and
removed delegated `RootProof::IcCanisterSignatureV1` from DTO/stable records.
It remains a baseline evidence report; current code should be checked directly
for post-cleanup state.

Date: 2026-06-30

Scope: closeout audit of delegated-auth survivorship after 0.76 hard-cut auth
work. This report is evidence-only. It does not implement cleanup.

## 1. Executive Summary

Does exactly one active delegated-auth flow survive?

**No, redundant/dead auth systems remain.**

Runtime delegated-token verification appears hard-cut to exactly one active root
proof verifier family: `RootProof::IcChainKeyBatchSignatureV1`. I found no
active runtime/config path where `RootProof::IcCanisterSignatureV1` can verify a
delegated-token root proof in 0.76 steady state. I also found no active bridge,
CLI run-once, provisioner, or direct-root-query renewal surface in the endpoint
macros or current CLI implementation.

The repo is not survivorship-clean, though. Legacy proof DTO/stable variants,
historical bridge/provisioner stable records, and role-attestation root
canister-signature code still share names and types with delegated-auth root
proofs. Active docs are corrected, but older design/audit docs outside archive
paths still describe bridge liveness as current-at-that-time behavior.

Top findings:

| Severity | Finding | Evidence | Required action |
| --- | --- | --- | --- |
| P0 | None found. | Delegated-token verifier requires `RootProofMode::ChainKeyBatch` before chain-key verification (`crates/canic-core/src/ops/auth/token.rs:388`), and chain-key verifier rejects non-chain-key root proofs (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`). | No P0 action from this audit. |
| P1 | Legacy root-proof public/stable variants remain. | Public DTO still has `RootProof::IcCanisterSignatureV1` and `RootProofMode::IcCanisterSignature` (`crates/canic-core/src/dto/auth.rs:36`, `crates/canic-core/src/dto/auth.rs:46`); stable record still has `RootProofRecord::IcCanisterSignatureV1` (`crates/canic-core/src/storage/stable/auth/records.rs:260`). | Split or retire legacy delegated-root proof shapes under a compatibility plan. |
| P1 | Root canister-signature code still returns/verifies `RootProof::IcCanisterSignatureV1` for role attestation. | Role attestation calls root canister-signature prepare/get/verify (`crates/canic-core/src/ops/auth/attestation.rs:64`, `crates/canic-core/src/ops/auth/attestation.rs:97`, `crates/canic-core/src/ops/auth/attestation.rs:123`); helper returns `RootProof::IcCanisterSignatureV1` (`crates/canic-core/src/ops/auth/root_canister_sig.rs:291`). | Separate non-delegated role-attestation proof DTOs from delegated-token `RootProof`. |
| P1 | Historical bridge/provisioner stable records remain decode-capable. | Records are explicitly decode-only (`crates/canic-core/src/storage/stable/auth/records.rs:403`, `crates/canic-core/src/storage/stable/auth/records.rs:417`); tests still round-trip nonempty historical records (`crates/canic-core/src/storage/stable/auth/records.rs:540`). | Keep until a stable-state removal/migration plan exists; mark as cleanup debt. |
| P1 | Some non-archive design/audit docs still preserve old bridge model. | `docs/design/0.76-auth/audit-findings.md:166` describes bridge/provisioner direct-query flow; `docs/design/audits/root-delegation-proof-renewal-audit.md:194` says direct root query was required under the old primitive. | Add superseded banners or move to archive to avoid active-behavior ambiguity. |
| P2 | CLI tests still contain removed `run-once`/`provisioner` command tails as forwarding fixtures. | `crates/canic-cli/src/tests.rs:531`, `crates/canic-cli/src/tests.rs:541`, `crates/canic-cli/src/tests.rs:849`, `crates/canic-cli/src/tests.rs:859`. | Rename/label tests as removed-tail preservation, or remove when global forwarding compatibility is no longer needed. |

Answers to the closeout questions:

- Legacy root canister-signature proofs accepted for delegated tokens: **No**.
  The chain-key verifier rejects the legacy root-proof variant
  (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`), and tests pin
  that rejection (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:702`,
  `crates/canic-core/src/ops/auth/token.rs:1226`).
- Bridge/provisioner/CLI renewal reachable: **No active runtime surface found**.
  Endpoint macros expose chain-key lazy repair only
  (`crates/canic/src/macros/endpoints/root.rs:97`), and protocol tests assert
  old bridge methods/predicates are absent (`crates/canic/tests/protocol_surface.rs:779`).
- Root signing on login hot path: **No under a fresh active proof**. A
  missing/stale-proof prepare may trigger lazy repair, but it signs an epoch
  batch, not login/session/token data, and tests assert repeated login under a
  fresh proof performs zero additional root signing
  (`crates/canic-tests/tests/root_cases/auth_076.rs:99`,
  `crates/canic-tests/tests/root_cases/auth_076.rs:140`).
- Old DTOs/stable records/config flags remain: **Yes**. They are rejected by
  active delegated-token verification, but they remain cleanup debt.

## 2. Auth Flow Inventory

| Flow | Status | Entry points | Proof type | Runtime reachable? | Evidence | Required action |
| --- | --- | --- | --- | --- | --- | --- |
| Chain-key batch root renewal | SURVIVOR | Root timer/update; root issuer template admin; root batch install workflow | `RootProof::IcChainKeyBatchSignatureV1` | Yes | Timer sweep prepares, signs, installs (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:78`); management signing through chain-key signer (`crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:193`). | Keep. |
| Chain-key lazy repair | SURVIVOR | Internal root update `canic_get_or_create_chain_key_delegation_proof` | `RootProof::IcChainKeyBatchSignatureV1` | Yes | Endpoint is internal and subnet-registered (`crates/canic/src/macros/endpoints/root.rs:97`); root uses caller as issuer (`crates/canic-core/src/api/auth/mod.rs:169`); workflow requires chain-key mode (`crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:39`). | Keep. |
| Issuer-local delegated-token issuance | SURVIVOR | `canic_prepare_delegated_token`; `canic_get_delegated_token` | Issuer `IssuerProof::IcCanisterSignatureV1` plus stored root proof | Yes | Public issuer endpoints (`crates/canic/src/macros/endpoints/nonroot.rs:31`, `crates/canic/src/macros/endpoints/nonroot.rs:38`); prepare uses active proof and issuer canister signature (`crates/canic-core/src/ops/auth/token.rs:93`, `crates/canic-core/src/ops/auth/token.rs:118`). | Keep. |
| Delegated-token verifier | SURVIVOR | Endpoint auth guard; `AuthOps::verify_token` | Chain-key root proof, issuer canister signature | Yes | Guard calls `AuthOps::verify_token` before subject/scope binding (`crates/canic-core/src/access/auth/token.rs:60`); root proof verifier is chain-key-only (`crates/canic-core/src/ops/auth/token.rs:388`). | Keep. |
| Root canister-signature delegated root proof | DEAD_CODE / DUPLICATE_CONCEPT | Public/stable DTO shape only for delegated tokens | `RootProof::IcCanisterSignatureV1` | Not accepted by delegated-token verifier | DTO/stable variants remain (`crates/canic-core/src/dto/auth.rs:36`, `crates/canic-core/src/storage/stable/auth/records.rs:260`); verifier rejects non-chain-key (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`). | Remove/split after compatibility plan. |
| Root role-attestation canister signature | SURVIVOR outside delegated-token flow / DUPLICATE_CONCEPT | `canic_prepare_role_attestation`, `canic_get_role_attestation` | `RootProof::IcCanisterSignatureV1` | Yes when role attestation configured/features enabled | Root endpoints exist (`crates/canic/src/macros/endpoints/root.rs:104`, `crates/canic/src/macros/endpoints/root.rs:111`); attestation ops use root canister signatures (`crates/canic-core/src/ops/auth/attestation.rs:64`). | Keep functionally, but split DTO naming from delegated-token root proof. |
| Historical bridge/provisioner renewal | LEGACY_ARCHIVE | Removed endpoints/CLI; decode-only stable fields | Old canister-signature root proof batches | No active surface found | Endpoint absence tests (`crates/canic/tests/protocol_surface.rs:779`); PocketIC absence tests (`crates/canic-tests/tests/root_cases/auth_076.rs:86`); CLI status-only command (`crates/canic-cli/src/auth/mod.rs:250`). | Archive docs; remove stable fields only with migration plan. |
| Manual controller old proof provisioning | LEGACY_ARCHIVE / DEAD_CODE | Old `prepare/get/install_delegation_proof_batch` names | Old canister-signature root proof | No active surface found | Old names asserted absent (`crates/canic/tests/protocol_surface.rs:779`). | Keep only historical docs until safe removal. |
| Raw IC root-key verification | SURVIVOR for issuer/role canister signatures; not delegated root trust anchor | Config `ic_root_public_key_raw_hex` | IC canister signatures | Yes, but not root chain-key authority | Raw key is paired with network for issuer proof verification (`crates/canic-core/src/config/validation/auth.rs:60`); chain-key policy has its own public key (`crates/canic-core/src/dto/auth.rs:92`). | Keep; document distinction. |
| Composite query / direct root proof retrieval | LEGACY_ARCHIVE / not found active | Old root proof retrieval names | Old canister-signature proof | Not found in active macros | Active docs forbid query/direct-root liveness (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:311`); old names absent in macros tests (`crates/canic/tests/protocol_surface.rs:779`). | Archive stale docs. |
| Controller-only guards | SURVIVOR, not delegated-token auth | `caller::is_controller`, root admin endpoints | N/A | Yes | Root admin/auth endpoints are controller/internal-gated (`crates/canic/src/macros/endpoints/root.rs:74`). | Keep separate from delegated-auth flow. |
| Test/local bypasses | Not found as active auth bypass | Tests/mocks only | Mock signers, test keys | Test only | Test key rejected on mainnet by config (`crates/canic-core/src/config/validation/auth.rs:174`) and verifier (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:278`). | Keep tests; no runtime bypass found. |

## 3. Single Survivor Definition

The repo currently treats the active delegated-token root proof flow as:

```text
RootProof::IcChainKeyBatchSignatureV1
threshold ECDSA secp256k1 key_1
canonical batch header and delegation cert
Merkle witness from issuer leaf to batch root
RootKeyPolicyV1 verifier trust anchor
issuer active proof install
issuer-local canister-signature token proof
local verifier checks
no bridge/direct-query liveness
```

Evidence:

- Active contract trust chain names `RootProof::IcChainKeyBatchSignatureV1`
  (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:8`).
- Active issuance flow is root timer/update to `sign_with_ecdsa` to issuer
  install (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:263`).
- Runtime renewal enforces `root_proof_mode="chain_key_batch"`
  (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:138`).
- Lazy repair also enforces chain-key mode
  (`crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:143`).
- Active docs forbid bridge/direct-query liveness and per-login root signing
  (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:457`).

Deviations:

- Public/stable `RootProof` still contains `IcCanisterSignatureV1`
  (`crates/canic-core/src/dto/auth.rs:36`,
  `crates/canic-core/src/storage/stable/auth/records.rs:260`).
- `RootProofMode::IcCanisterSignature` still exists as a DTO variant
  (`crates/canic-core/src/dto/auth.rs:46`), although config rejects it.
- Role attestation still uses `RootProof::IcCanisterSignatureV1`, so `RootProof`
  is not delegated-token-only (`crates/canic-core/src/ops/auth/attestation.rs:97`).
- Named designs such as `DelegationCertV2`, `ChainKeyIssuerLeafV1`,
  `ActiveRootProofKey`, `LegacyBridge`, `DualCode`, `ChainKeyPreferred`, and
  `ChainKeyOnly` were not found in active source. The implementation is a hard
  cut, not a per-issuer cutover-state model.

## 4. RootProof Variant Audit

| Variant | Used where | Accepted by delegated-token verifier? | Stored in stable records? | Migration role? | Action |
| --- | --- | --- | --- | --- | --- |
| `RootProof::IcChainKeyBatchSignatureV1` | Delegated-token root proof; active proof install; token verifier | Yes | Yes, `RootProofRecord::IcChainKeyBatchSignatureV1` | Survivor | Keep. |
| `RootProof::IcCanisterSignatureV1` | Legacy delegated-token DTO/stable shape; active role-attestation root proof | No for delegated tokens; yes for non-delegated role attestation | Yes, `RootProofRecord::IcCanisterSignatureV1` | Not a delegated-token migration path found | Split non-delegated role attestation from delegated-token `RootProof`; retire delegated legacy variant when safe. |

Required hard-cut answer:

**Can `RootProof::IcCanisterSignatureV1` still be accepted in 0.76 steady
state?**

- Delegated-token root proof: **No**. `AuthOps::verify_delegation_root_proof`
  requires `RootProofMode::ChainKeyBatch` (`crates/canic-core/src/ops/auth/token.rs:388`);
  `verify_chain_key_batch_root_proof` returns `LegacyRootProofRejected` unless
  the proof is `IcChainKeyBatchSignatureV1`
  (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`).
- Role attestation: **Yes, intentionally outside delegated-token root proof**.
  Role-attestation verify calls `verify_root_canister_signature_proof`
  (`crates/canic-core/src/ops/auth/attestation.rs:123`), which accepts only
  `RootProof::IcCanisterSignatureV1`
  (`crates/canic-core/src/ops/auth/root_canister_sig.rs:328`).

No wildcard match arm that accepts old delegated-token root proofs was found.
Config modes cannot re-enable the old delegated-token proof family: validation
rejects any root proof mode except `chain_key_batch`
(`crates/canic-core/src/config/validation/auth.rs:99`).

## 5. DTO And Stable Record Survivorship

| Type/Record | Current use | Survivor/Migration/Dead/Duplicate | Evidence | Required action |
| --- | --- | --- | --- | --- |
| `DelegationCert` | Token-facing root cert | SURVIVOR | DTO fields at `crates/canic-core/src/dto/auth.rs:251`; verifier hashes and checks it (`crates/canic-core/src/ops/auth/delegated/verify.rs:160`). | Keep. |
| `DelegationCertV2` | Not found | Not found | Exact active-source search returned no hits. | None. |
| `DelegationProof` | Token embeds cert + root proof | SURVIVOR | `crates/canic-core/src/dto/auth.rs:269`. | Keep. |
| `DelegatedTokenClaims` | Token claims | SURVIVOR | `crates/canic-core/src/dto/auth.rs:338`. | Keep. |
| `DelegatedToken` | Public token DTO | SURVIVOR | `crates/canic-core/src/dto/auth.rs:356`. | Keep. |
| `RootProof` | Delegated-token root proof and role-attestation proof | DUPLICATE_CONCEPT | Both variants in one enum (`crates/canic-core/src/dto/auth.rs:36`); role attestation embeds the legacy variant (`crates/canic-core/src/ops/auth/attestation.rs:171`). | Split role-attestation proof type or remove legacy delegated-root variant when stable/API allows. |
| `IcCanisterSignatureProofV1` | Issuer token proof and role-attestation root proof; legacy delegated-root proof material | DUPLICATE_CONCEPT | DTO at `crates/canic-core/src/dto/auth.rs:64`; issuer proof uses it (`crates/canic-core/src/ops/auth/issuer_canister_sig.rs:286`); root role attestation uses it (`crates/canic-core/src/ops/auth/root_canister_sig.rs:291`). | Keep issuer proof; isolate root role attestation use from delegated `RootProof`. |
| `IcChainKeyBatchSignatureProofV1` | Active delegated-token root proof | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:146`; verifier pattern matches it (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`). | Keep. |
| `ChainKeyBatchHeaderV1` | Signed batch header | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:158`; signed hash used as management message (`crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:219`). | Keep. |
| `ChainKeyIssuerLeafV1` | Not found under this name | Not found | Equivalent appears as `ChainKeyDelegationCertV1` (`crates/canic-core/src/dto/auth.rs:179`). | Update docs if still using old name. |
| `ChainKeyDelegationCertV1` | Per-issuer Merkle leaf payload | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:179`; verifier enforces cert/leaf equality (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:337`). | Keep. |
| `ChainKeyRootSignatureV1` | Chain-key ECDSA signature material | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:200`. | Keep. |
| `ChainKeyBatchWitnessV1` | Merkle witness | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:213`; witness directions encoded by enum (`crates/canic-core/src/dto/auth.rs:222`). | Keep. |
| `ActiveDelegationProof` | Issuer-local active proof | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:279`; install validates root proof first (`crates/canic-core/src/ops/auth/delegation/active.rs:29`). | Keep. |
| `ActiveRootProofKey` | Not found | Not found | Exact active-source search returned no hits. | None. |
| `RootKeyPolicyV1` | Verifier trust anchor | SURVIVOR | DTO at `crates/canic-core/src/dto/auth.rs:92`; config constructs it with `proof_mode: ChainKeyBatch` (`crates/canic-core/src/ops/auth/token.rs:539`). | Keep. |
| `IssuerProofAlgorithm` | Issuer proof algorithm binding | SURVIVOR | Only `IcCanisterSignatureV1` (`crates/canic-core/src/dto/auth.rs:232`). | Keep if issuer proof remains canister-signature based. |
| `IssuerProofBinding` | Issuer proof seed binding | SURVIVOR | `IcCanisterSignatureV1 { seed_hash }` (`crates/canic-core/src/dto/auth.rs:241`). | Keep. |
| `RootProofRecord::IcCanisterSignatureV1` | Stable decode shape | DEAD_CODE / MIGRATION_ONLY risk | Record variant remains (`crates/canic-core/src/storage/stable/auth/records.rs:260`). | Remove only with stable migration/versioning plan. |
| `RootDelegationRenewalBatchRecord` | Historical bridge batch stable state | LEGACY_ARCHIVE | Comments say decode-only, active uses chain-key record (`crates/canic-core/src/storage/stable/auth/records.rs:403`). | Keep until stable cleanup; do not use active runtime. |
| `RootProvisionerRecord` | Historical provisioner ACL stable state | LEGACY_ARCHIVE | Comments say active 0.76 has no provisioner ACL (`crates/canic-core/src/storage/stable/auth/records.rs:417`). | Keep until stable cleanup; do not use active runtime. |

## 6. Verifier Branch Audit

How many root proof verifier branches can return `Ok` in active 0.76 delegated
token config?

**Exactly one: `IcChainKeyBatchSignatureV1`.**

Evidence:

- Runtime verifier config parser returns `RootProofMode::ChainKeyBatch` only
  for `root_proof_mode = "chain_key_batch"` and errors otherwise
  (`crates/canic-core/src/ops/auth/token.rs:458`).
- Delegated-token root proof verification rejects any verifier config not in
  `ChainKeyBatch` mode (`crates/canic-core/src/ops/auth/token.rs:388`).
- Chain-key proof verification destructures `RootProof::IcChainKeyBatchSignatureV1`;
  every other variant returns `LegacyRootProofRejected`
  (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`).
- Missing chain-key root policy is an error, not a permissive fallback
  (`crates/canic-core/src/ops/auth/token.rs:391`).
- Proof-supplied chain-key public key is not trusted alone: verifier requires it
  to equal the configured policy public key
  (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:420`).
- The raw IC root key is used for issuer canister-signature verification
  (`crates/canic-core/src/ops/auth/token.rs:355`), not as the delegated-token
  root chain-key trust anchor.

No delegated-token verifier fallback to canister-signature root proof was found.

## 7. Entry Point And Endpoint Audit

| Endpoint | Query/Update | Caller policy | Flow | Survivor status | Required action |
| --- | --- | --- | --- | --- | --- |
| `canic_upsert_root_issuer_policy` | Update | Controller | Root issuer policy admin | SURVIVOR | Keep. |
| `canic_upsert_root_issuer_renewal_template` | Update | Controller | Root timer renewal template | SURVIVOR | Keep. |
| `canic_root_issuer_renewal_status` | Query | Controller | Status only | SURVIVOR | Keep. |
| `canic_get_or_create_chain_key_delegation_proof` | Update | Internal, registered subnet caller | Chain-key lazy repair | SURVIVOR | Keep. |
| `canic_prepare_delegated_token` | Update | Public issuer endpoint; request policy checks subject/scopes | Issuer token prepare | SURVIVOR | Keep. |
| `canic_get_delegated_token` | Query | Public issuer endpoint; caller-bound pending token | Issuer token retrieval | SURVIVOR | Keep. |
| `canic_install_active_delegation_proof` | Update | Controller on issuer | Issuer active proof install | SURVIVOR | Keep; it verifies root proof before storage. |
| `canic_active_delegation_proof_status` | Query | Public | Status | SURVIVOR | Keep. |
| `canic_prepare_role_attestation` | Update | Internal registered subnet | Role attestation | SURVIVOR outside delegated-token root flow | Keep separate. |
| `canic_get_role_attestation` | Query | Internal registered subnet | Role attestation | SURVIVOR outside delegated-token root flow | Keep separate. |
| `canic_delegation_renewal_work` | Query | Removed | Bridge renewal | LEGACY_ARCHIVE | Keep absent. |
| `canic_get_delegation_renewal_proof_batch` | Query | Removed | Bridge/direct query proof retrieval | LEGACY_ARCHIVE | Keep absent. |
| `canic_install_delegation_proof_batch` | Update | Removed | Bridge proof install | LEGACY_ARCHIVE | Keep absent. |
| `canic_upsert_delegation_renewal_provisioner` | Update | Removed | Provisioner ACL | LEGACY_ARCHIVE | Keep absent. |

Evidence:

- Current root auth macro exposes the chain-key lazy-repair update and role
  attestation endpoints, not old bridge get/install endpoints
  (`crates/canic/src/macros/endpoints/root.rs:70`).
- Current non-root auth macro exposes token prepare/get/install/status
  (`crates/canic/src/macros/endpoints/nonroot.rs:27`).
- Active proof install loads verifier config and verifies the root proof before
  storing issuer-local active state (`crates/canic-core/src/ops/auth/delegation/active.rs:29`).
- Protocol tests assert old bridge endpoint names and the provisioner caller
  predicate are absent (`crates/canic/tests/protocol_surface.rs:779`).

Required conclusion:

- Must remain for 0.76 delegated auth: root issuer policy/template/status,
  root chain-key lazy repair, issuer active proof install/status, issuer token
  prepare/get, verifier endpoint guards.
- Must remain disabled/removed: bridge work/get/install endpoints, provisioner
  ACL endpoints/predicates, direct root proof query endpoints.

## 8. Bridge / CLI / Provisioner Cleanup Audit

If every bridge/CLI/provisioner path is disabled, does active delegated auth
still renew?

**Yes, by source and test evidence.** Timer renewal and lazy repair are
canister-owned and chain-key mode gated.

Evidence:

- Timer renewal starts only on root, enabled templates, delegated-token auth, and
  `chain_key_batch` mode (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:43`).
- Timer sweep prepares due templates, signs next batch, and installs proofs
  (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:93`,
  `crates/canic-core/src/workflow/runtime/auth/renewal.rs:107`,
  `crates/canic-core/src/workflow/runtime/auth/renewal.rs:115`).
- CLI auth renewal has status only: `renewal_command()` contains only
  `status_command()` (`crates/canic-cli/src/auth/mod.rs:250`), and `run_command`
  only dispatches `RenewalStatus` (`crates/canic-cli/src/auth/mod.rs:280`).
- Medic hints tell operators to use status and chain-key renewal/lazy repair,
  not bridge run-once (`crates/canic-cli/src/auth/mod.rs:896`).
- CI proof script asserts help omits removed `run-once` and provisioner commands
  (`scripts/ci/auth-renewal-cli-proof-lib.sh:83`,
  `scripts/ci/auth-renewal-cli-proof-lib.sh:95`).
- PocketIC tests assert old bridge/provisioner methods are not callable
  (`crates/canic-tests/tests/root_cases/auth_076.rs:86`).

Classification:

| Path | Classification | Evidence |
| --- | --- | --- |
| `canic auth renewal status` | SURVIVOR status-only | `crates/canic-cli/src/auth/mod.rs:250` |
| `canic auth renewal run-once` | LEGACY_ARCHIVE / removed | CI omission checks (`scripts/ci/auth-renewal-cli-proof-lib.sh:83`) |
| Renewal provisioner ACL | LEGACY_ARCHIVE stable decode only | `crates/canic-core/src/storage/stable/auth/records.rs:417` |
| Old bridge root endpoints | LEGACY_ARCHIVE / removed | `crates/canic/tests/protocol_surface.rs:779` |
| Historical design docs | LEGACY_ARCHIVE or stale | `docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:739` |

## 9. Config Flag Audit

| Config | Meaning | Default | Can enable old flow? | Survivor status | Required action |
| --- | --- | --- | --- | --- | --- |
| `auth.delegated_tokens.enabled` | Enables delegated token auth | `false` | No | SURVIVOR | Keep. |
| `auth.delegated_tokens.root_proof_mode` | Root proof primitive selector | `"chain_key_batch"` | No, validation rejects other values | DUPLICATE_CONCEPT / P1 cleanup | Make implicit once compatibility allows. |
| `auth.delegated_tokens.chain_key_root_proof.*` | Root chain-key verifier/signing policy | Empty until configured | No | SURVIVOR | Keep. |
| `auth.delegated_tokens.ic_root_public_key_raw_hex` | Raw IC root key for canister-signature issuer/role proof verification | `None` | No delegated-root fallback found | SURVIVOR | Keep distinction documented. |
| `auth.delegated_tokens.network` | Mainnet/local/test policy | `"mainnet"` | No | SURVIVOR | Keep. |
| `auth.delegated_tokens.chain_key_root_proof.allow_test_key` | Off-mainnet test key allowance | `false` | No on mainnet | SURVIVOR | Keep guard. |
| `delegated_token_issuer` canister flag | Allows issuer prepare/get | Per-canister config | No | SURVIVOR | Keep. |
| `delegated_token_verifier` canister flag | Allows endpoint delegated-token verifier | Per-canister config | No | SURVIVOR | Keep. |
| Renewal provisioner config/ACL | Old bridge provisioner | Not active | No active config found | LEGACY_ARCHIVE stable decode | Remove with stable plan. |

Evidence:

- Schema docs define only `root_proof_mode = "chain_key_batch"` as 0.76
  contract (`crates/canic-core/src/config/schema/mod.rs:383`).
- Validation rejects any root proof mode except `chain_key_batch`
  (`crates/canic-core/src/config/validation/auth.rs:99`).
- Config tests pin rejection of `"canister_signature"`
  (`crates/canic-core/src/config/schema/tests.rs:444`).
- Mainnet rejects `test_key_1` (`crates/canic-core/src/config/validation/auth.rs:174`);
  off-mainnet `test_key_1` requires `allow_test_key = true`
  (`crates/canic-core/src/config/validation/auth.rs:180`).

## 10. Login Hot Path Audit

Can login/session/token issuance call root signing or bridge renewal?

- Fresh active proof path: **No root signing and no bridge**.
- Missing/stale active proof path: **May call root lazy repair**, which can
  sign a chain-key batch if no reusable batch exists. This is not per-login,
  per-session, per-token, per-user, nonce, or request-payload signing. It signs
  the canonical batch header for a proof epoch.

Evidence:

- Public prepare validates caller/subject/scopes before delegated token prepare
  (`crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs:63`).
- Issuer token prepare uses active proof and issuer canister-signature prepare,
  not root threshold signing (`crates/canic-core/src/ops/auth/token.rs:93`,
  `crates/canic-core/src/ops/auth/token.rs:118`).
- Lazy repair runs only after stale/missing material errors
  (`crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs:239`) and calls
  root `CANIC_GET_OR_CREATE_CHAIN_KEY_DELEGATION_PROOF`
  (`crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs:258`).
- Root chain-key signing call sites are in `sign_next_chain_key_root_delegation_batch`
  and `get_or_create_chain_key_delegation_proof_for_issuer`
  (`crates/canic-core/src/ops/auth/delegation/mod.rs:131`,
  `crates/canic-core/src/ops/auth/delegation/mod.rs:165`).
- Management signing wrapper is only the chain-key signer path
  (`crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:105`).
- Integration tests assert first missing-proof lazy repair produces one signing
  operation and repeated login under fresh proof produces zero additional root
  signing (`crates/canic-tests/tests/root_cases/auth_076.rs:118`,
  `crates/canic-tests/tests/root_cases/auth_076.rs:140`).

Signing call-site classification:

| Call site | Classification | Evidence |
| --- | --- | --- |
| `MgmtInfra::sign_with_ecdsa` | Management wrapper | `crates/canic-core/src/infra/ic/mgmt/signing.rs:33` |
| `MgmtOps::sign_with_ecdsa` | Ops wrapper | `crates/canic-core/src/ops/ic/mgmt/signing.rs:25` |
| `ManagementCanisterChainKeySigner::sign_with_ecdsa` | Timer renewal / lazy repair | `crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:105` |
| `sign_chain_key_batch_header` | Timer renewal / lazy repair | `crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:193` |
| Mock `ChainKeySigner` implementations | Test | `crates/canic-core/src/ops/auth/delegation/chain_key_batch.rs:1307` |
| `prepare_issuer_canister_signature` | Login/token issuer proof, not root threshold signing | `crates/canic-core/src/ops/auth/token.rs:118` |

No bridge call site was found in the login/token path.

## 11. Lazy Repair Survivorship Audit

Lazy repair belongs to the one surviving flow.

| Requirement | Status | Evidence |
| --- | --- | --- |
| Update-based only | Present | Root endpoint is `canic_update(internal, requires(caller::is_registered_to_subnet()))` (`crates/canic/src/macros/endpoints/root.rs:97`). |
| Caller equals requested issuer | Present by shape | API passes `IcOps::msg_caller()` as the issuer id; there is no request-supplied issuer parameter (`crates/canic-core/src/api/auth/mod.rs:169`). |
| Registered/enabled issuer | Present | Endpoint caller predicate requires registered subnet; root batch prepare uses current issuer policy and registry in `get_or_create_chain_key_delegation_proof_for_issuer` (`crates/canic-core/src/ops/auth/delegation/mod.rs:184`). |
| Cache-first and singleflight | Present in tests | Cached batch and in-flight repair tests passed in `cargo test -p canic-core auth --lib`; test names include `chain_key_lazy_repair_get_or_create_signs_once_then_reuses_cached_proof` and `chain_key_lazy_repair_reuses_in_flight_batch_without_extra_signing`. |
| Pending/retry-after | Present in tests/source | Core test name `chain_key_lazy_repair_respects_retry_after_before_resigning`; renewal outcome has `RetrievalExpired`/retry states (`crates/canic-core/src/dto/auth.rs:503`). |
| No old-proof fallback | Present | Lazy repair requires `chain_key_batch` (`crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:143`). |
| No login-specific signing | Present | Signer hashes batch header only (`crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:219`). |

## 12. Canister-Signature Code Survivorship

Canister signatures still survive in two places with different risk profiles:

| Code | Classification | Reason | Required action |
| --- | --- | --- | --- |
| Issuer-local canister signature token proof | SURVIVOR | Issuer proof enum has only `IcCanisterSignatureV1` (`crates/canic-core/src/dto/auth.rs:55`); token get returns issuer proof (`crates/canic-core/src/ops/auth/issuer_canister_sig.rs:286`); verifier checks expected issuer id and seed (`crates/canic-core/src/ops/auth/issuer_canister_sig.rs:313`). | Keep while issuer proof mechanism is canister-signature based. |
| Root canister signature role attestation | SURVIVOR outside delegated-token flow / DUPLICATE_CONCEPT | Root payload kind is only `RoleAttestation` (`crates/canic-core/src/ops/auth/root_canister_sig.rs:36`); attestation uses root canister-signature proof (`crates/canic-core/src/ops/auth/attestation.rs:64`). | Split from delegated-token `RootProof` naming. |
| Root canister-signature delegated proof | DEAD_CODE as active delegated auth / stale DTO | Legacy root proof variant remains, but delegated verifier rejects it. | Remove/split after compatibility plan. |
| `certified_data` / `data_certificate` proof assembly | SURVIVOR only for canister-signature issuer/role proofs | Issuer proof sets certified data (`crates/canic-core/src/ops/auth/issuer_canister_sig.rs:230`); root role-attestation proof does the same (`crates/canic-core/src/ops/auth/root_canister_sig.rs:236`). | Keep for those non-chain-key proof surfaces; do not use for delegated-token root liveness. |

## 13. Docs Survivorship Audit

| Doc | Classification | Evidence | Required action |
| --- | --- | --- | --- |
| `docs/contracts/AUTH_DELEGATED_SIGNATURES.md` | Active 0.76 behavior | Defines chain-key trust chain (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:8`), rejects legacy root proof (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:258`), forbids bridge/per-login root signing (`docs/contracts/AUTH_DELEGATED_SIGNATURES.md:457`). | Keep. |
| `docs/architecture/authentication.md` | Active 0.76 behavior | Says old bridge surfaces are not active and liveness is timer/lazy repair (`docs/architecture/authentication.md:358`); says normal login has no management signing (`docs/architecture/authentication.md:409`). | Keep. |
| `docs/operations/root-proof-provisioning.md` | Active 0.76 runbook | Says active chain-key path, not historical bridge flow (`docs/operations/root-proof-provisioning.md:3`); operator surface is status-only (`docs/operations/root-proof-provisioning.md:56`). | Keep. |
| `docs/design/0.76-auth/0.76-design.md` | Active design | Explicitly rejects bridge/CLI/direct query liveness (`docs/design/0.76-auth/0.76-design.md:200`). | Keep. |
| `docs/design/0.76-auth/audit-findings.md` | Stale/superseded design audit | Describes bridge/provisioner direct-query flow as required under old primitive (`docs/design/0.76-auth/audit-findings.md:166`). | Add superseded banner or move to archive. |
| `docs/design/audits/root-delegation-proof-renewal-audit.md` | Stale/superseded audit | Calls external direct root query and bridge required under old primitive (`docs/design/audits/root-delegation-proof-renewal-audit.md:194`). | Add superseded banner or archive. |
| `docs/design/archive/0.65-canister-signatures/0.65-design.md` | LEGACY_ARCHIVE | Archive path; describes old provisioner/direct query (`docs/design/archive/0.65-canister-signatures/0.65-design.md:484`). | Keep archived. |
| `docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md` | Historical design, not archive path | Documents old `run-once` and provisioner commands (`docs/design/0.74-root-managed-delegation-proof-renewal/0.74-design.md:739`). | Consider archive banner/path. |

No active runbook was found telling operators to run bridge/CLI/direct root query
for 0.76 auth liveness.

## 14. Test Survivorship Audit

| Test group | Status | Evidence |
| --- | --- | --- |
| No-external-liveness | Present | PocketIC test `auth_076_chain_key_batch_renews_without_external_liveness` (`crates/canic-tests/tests/root_cases/auth_076.rs:65`). Not rerun in this audit. |
| Legacy proof rejection | Present | Unit tests reject legacy canister-signature root proof (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:702`, `crates/canic-core/src/ops/auth/token.rs:1226`). |
| Bridge disabled | Present | Protocol test asserts old endpoint names absent (`crates/canic/tests/protocol_surface.rs:779`); PocketIC test asserts callable methods absent (`crates/canic-tests/tests/root_cases/auth_076.rs:86`). |
| Single verifier branch | Present by source + tests | Chain-key verifier pattern match (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`); legacy proof rejection tests above. |
| No per-login signing | Present | PocketIC lazy repair login test (`crates/canic-tests/tests/root_cases/auth_076.rs:99`). Not rerun in this audit. |
| Signing call-site classification | Partial | Source search found management signing only in chain-key signer/wrappers/tests; no automated static test pins all call sites. |
| Cutover hard end condition | Present by absence | Exact active-source search found no `LegacyBridge`, `DualCode`, `ChainKeyPreferred`, or `ChainKeyOnly`. Hard cut has no cutover state. |
| DTO/stable record migration rejection | Partial | Stable records preserve decode/round-trip for historical fields (`crates/canic-core/src/storage/stable/auth/records.rs:540`); delegated verifier rejects legacy root proofs. No full upgrade fixture was run here. |
| Docs/config default hard cut | Present | Config validation test rejects legacy root proof mode (`crates/canic-core/src/config/schema/tests.rs:444`). |
| Canonical encoding fixtures | Present | `cargo test -p canic-core auth --lib` ran tests including `chain_key_canonical_hashes_match_golden_fixtures`. |
| Merkle fixtures | Present | `cargo test -p canic-core auth --lib` ran tests including `chain_key_merkle_witness_root_matches_golden_fixture`. |
| Signature fixtures | Present | `cargo test -p canic-core auth --lib` ran high-s, malformed length, and signer verification tests. |
| BuildNetwork/test-key tests | Present | Config/verifier tests reject mainnet test key and require off-mainnet allow flag. |

Dangerous old-flow test behavior:

- No test asserting delegated-token legacy root proof acceptance was found.
- Some tests intentionally construct `RootProof::IcCanisterSignatureV1` for
  role-attestation or active-proof helper stubs
  (`crates/canic-core/src/ops/auth/attestation.rs:171`,
  `crates/canic-core/src/ops/auth/delegated/active_proof.rs:133`). These are
  not delegated-token root-proof acceptance tests, but they reinforce the
  duplicate-concept risk.
- CLI tests still mention removed `run-once`/`provisioner` tails as global
  forwarding fixtures (`crates/canic-cli/src/tests.rs:531`). These should be
  renamed or removed when no longer needed.

## 15. Dead Code And Redundancy Candidates

| Candidate | Classification | Why safe/unsafe to remove | Dependencies | Suggested slice |
| --- | --- | --- | --- | --- |
| `RootProofMode::IcCanisterSignature` | DEAD_CODE / DUPLICATE_CONCEPT | Active config rejects it; public DTO/API compatibility risk. | Candid/API compatibility, docs, tests. | Remove or deprecate root proof mode selector after public API review. |
| `RootProof::IcCanisterSignatureV1` as delegated-token root proof | DEAD_CODE / DUPLICATE_CONCEPT | Delegated verifier rejects it; role attestation still uses same enum. | Role-attestation DTO split, stable records. | First split role-attestation proof type, then remove delegated legacy variant. |
| `RootProofRecord::IcCanisterSignatureV1` | Risky stable migration cleanup | Stable persisted enum variant remains. | Stable decode/upgrade compatibility. | Add explicit migration/version plan, then remove or quarantine. |
| `RootDelegationRenewalBatchRecord` | LEGACY_ARCHIVE stable cleanup | Decode-only bridge state; removal can break old snapshots. | Stable import/export compatibility. | Keep now; later stable-schema migration. |
| `RootProvisionerRecord` | LEGACY_ARCHIVE stable cleanup | Decode-only provisioner ACL state; removal can break old snapshots. | Stable import/export compatibility. | Keep now; later stable-schema migration. |
| `root_canister_sig` module for role attestation | DUPLICATE_CONCEPT, not dead | Active role-attestation functionality may depend on it. | Role-attestation feature flags and APIs. | Rename/split to `role_attestation_root_canister_sig`; do not delete blindly. |
| `docs/design/0.76-auth/audit-findings.md` | Stale docs | Superseded audit describes old bridge liveness. | Design history. | Add superseded banner or move to archive. |
| `docs/design/audits/root-delegation-proof-renewal-audit.md` | Stale docs | Superseded audit describes old bridge/direct query liveness. | Design history. | Add superseded banner or move to archive. |
| CLI forwarding tests with removed command tails | P2 test cleanup | They may still protect global forwarding behavior; not active CLI. | CLI test intent. | Rename tests/fixtures as removed-tail compatibility or drop when safe. |
| Bridge endpoint code | Already removed from active macros | No active endpoint code found to delete. | N/A | No production deletion slice from this audit. |

## 16. "Worse Than Dead Code" Findings

| Check | Result | Evidence | Severity |
| --- | --- | --- | --- |
| Two delegated-token root verifier branches both returning `Ok` | Not found | Only chain-key variant destructures successfully (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`). | None |
| Old proof selected when new policy missing | Not found | Missing chain-key policy errors (`crates/canic-core/src/ops/auth/token.rs:391`). | None |
| Permissive default enables old flow | Not found | Default root proof mode is chain-key batch (`crates/canic-core/src/config/schema/mod.rs:474`); validation rejects other values (`crates/canic-core/src/config/validation/auth.rs:99`). | None |
| Test key accepted on mainnet | Not found | Config rejects mainnet `test_key_1` (`crates/canic-core/src/config/validation/auth.rs:174`); verifier rejects non-production key on `BuildNetwork::Ic` (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:281`). | None |
| Raw IC root key fallback in chain-key mode | Not found | Chain-key verifier requires configured `RootKeyPolicyV1` public key equality (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:420`). | None |
| Bridge install bypasses chain-key verifier | Not found active | Old install endpoint absent in protocol test (`crates/canic/tests/protocol_surface.rs:779`); active install verifies root proof (`crates/canic-core/src/ops/auth/delegation/active.rs:29`). | None |
| Controller/admin endpoint installs arbitrary active proof | Not found | Controller install routes through verifier before storage (`crates/canic-core/src/api/auth/mod.rs:120`, `crates/canic-core/src/ops/auth/delegation/active.rs:29`). | None |
| Stable migration rehydrates legacy proof as accepted | Partial risk, not found active | Stable variant exists (`crates/canic-core/src/storage/stable/auth/records.rs:260`), but active install/token verification rejects legacy root proof. No full upgrade fixture run here. | P1 cleanup risk |
| Wildcard match treats unknown proof as accepted | Not found | Chain-key verifier explicit `let ... else` rejection (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:171`). | None |
| Hidden external process requirement | Not found active | Timer/lazy repair code owns signing; active runbook says product frontends must not orchestrate renewal (`docs/operations/root-proof-provisioning.md:71`). | None |

## 17. Commands Run

| Command | Result | Notes |
| --- | --- | --- |
| `sed -n '1,230p' docs/status/current.md` | Read | Session handoff per `AGENTS.md`. |
| `rg -n "RootProof|IcCanisterSignatureV1|IcChainKeyBatchSignatureV1|sign_with_ecdsa|..." ...` | Read | Broad auth survivorship search. |
| `rg -n "DelegationCertV2|ChainKeyIssuerLeaf|ActiveRootProofKey|LegacyBridge|DualCode|ChainKeyPreferred|ChainKeyOnly|chain_key_single_issuer|vetKD|vetkd|Schnorr|sign_with_schnorr|schnorr_public_key" crates/canic-core/src crates/canic/src crates/canic-cli/src crates/canic-tests` | Exit 1, no matches | No active source hits for those removed/deferred concepts. |
| `rg -n "canic_get_delegation_renewal_proof_batch|canic_delegation_renewal_work|canic_install_delegation_proof_batch|run-once|provisioner|..." crates/canic/src crates/canic-core/src crates/canic-cli/src scripts/ci crates/canic-tests docs` | Read | Found active absence tests, decode-only stable records, archived/stale docs, and CLI removed-tail tests; no active bridge endpoint implementation. |
| `rg -n "sign_with_ecdsa|ecdsa_public_key|ChainKeySigner|sign_chain_key_batch_header" crates/canic-core/src crates/canic-tests` | Read | Found management wrappers, chain-key signer, chain-key batch workflow, and tests/mocks. |
| `nl -ba ...` source reads | Read | Read DTOs, stable records, verifier code, endpoint macros, config validation, CLI auth, workflows, active-proof install, docs/tests. |
| `nl -ba crates/canic-core/src/workflow/runtime/auth/prepare.rs ...` | Failed path read | Correct current file is `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs`, which was then read. |
| `cargo fmt --check` | Pass | No formatting changes made. |
| `cargo test -p canic-core auth --lib` | Pass | 255 passed; 0 failed; 468 filtered out. This covered core auth verifier, canonical, chain-key, lazy repair, stable decode, and config tests. |
| `cargo test -p canic protocol_surface` | Pass | Ran only 2 tests matching that filter; followed with the exact root-delegation test. |
| `cargo test -p canic root_delegation_proof_batch_surface_is_pinned` | Pass | 1 passed in `tests/protocol_surface.rs`; validates root delegation endpoint surface and old bridge absence assertions. |

Commands not run:

| Command | Reason |
| --- | --- |
| `make test` | Explicit prior maintainer instruction not to run it here; also timeout-prone. |
| `cargo test -p canic-tests root` | PocketIC/root integration suite is heavier and environment-dependent; source evidence and targeted unit/protocol tests were sufficient for this audit turn. |
| Full `cargo test --workspace` | Too broad for a read-only closeout audit and likely expensive. |

## 18. Final Verdict

Final verdict:

```text
Exactly one surviving auth flow: no
```

More precise verdict:

```text
No, redundant/dead auth systems remain.
```

Runtime delegated-token auth is hard-cut enough for the core product invariant:
the active delegated-token root verifier accepts only
`RootProof::IcChainKeyBatchSignatureV1`; bridge/provisioner/CLI renewal surfaces
are not active; repeated login under a fresh active proof performs no root
threshold signing.

P0 blockers:

- None found.

P1 blockers:

1. Public/stable legacy delegated-root proof shapes remain:
   `RootProof::IcCanisterSignatureV1`, `RootProofMode::IcCanisterSignature`,
   and `RootProofRecord::IcCanisterSignatureV1`.
2. Role-attestation root canister-signature proof still uses the same
   `RootProof` enum, creating a duplicate concept and future misuse risk even
   though delegated-token verification rejects it.
3. Historical bridge/provisioner stable records remain decode-capable and
   round-trip in tests; this is intentional compatibility debt, not active
   runtime reachability.
4. Superseded design/audit docs outside archive paths still describe bridge and
   direct root query liveness as required under the old primitive.

Recommended cleanup slices:

1. Split role-attestation root canister-signature material out of delegated
   `RootProof`, or introduce an explicit `RoleAttestationRootProof` DTO so
   `RootProof` can become chain-key-only for delegated tokens.
2. Remove or deprecate `RootProofMode::IcCanisterSignature` and the
   `"canister_signature"` config spelling after checking Candid/API compatibility.
3. Add superseded banners or archive moves for old bridge-era design/audit docs;
   rename CLI removed-tail tests so they cannot be read as active CLI coverage.
4. Plan a stable-state cleanup for decode-only bridge/provisioner records and
   `RootProofRecord::IcCanisterSignatureV1`; do not delete them without an
   explicit stable migration strategy.

Recommended next Codex prompt:

```text
Do the narrow hard-cut cleanup slice: split role-attestation root canister-signature
proofs away from delegated-token RootProof without changing stable delegated-auth
records yet, then update tests/docs so delegated RootProof is chain-key-only in
active code.
```

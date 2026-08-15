# Canic 0.102 Authentication String Frontier

Date: 2026-08-15

## Status

This B1 ledger expands and semantically reconciles the authentication producer
graph that is hidden by
`AuthValidationError::Auth(String)` and by adjacent direct string conversions
at `v0.101.53`. It assigns no numeric codes. Its purpose is to prevent one
generic auth code from concealing different owners, retry rules and recovery
actions.

The frontier is wider than the `Auth(String)` variant itself. It includes:

- typed auth errors converted through `Auth(err.to_string())`;
- typed auth errors converted directly to broad `InternalError` constructors;
- already-typed causes unnecessarily converted to text and wrapped again; and
- `ChainKeySignerError` text persisted in the durable batch record before a
  generic runtime error is returned.

## Structural Expansion

The recursive pass found ten additional Canic-owned typed error owners behind
the string stops in
[transitive-error-inventory.md](transitive-error-inventory.md).

| Typed owner | Declared variants | Non-test variants | Current terminal treatment |
| --- | ---: | ---: | --- |
| `AudienceError` | 7 | 7 | Transparent child of preparation, verification and certificate-rule errors; ultimately formatted |
| `CanonicalAuthError` | 9 | 9 | Transparent child of several auth owners; ultimately formatted |
| `PrepareDelegatedTokenError` | 11 | 10 | Two selected broad mappings; remaining production variants formatted through `Auth(String)` |
| `ChainKeySignerError` | 5 | 5 | Formatted into durable batch failure text and a generic `Ops` error |
| `RetainedDelegatedTokenLookupError` | 2 | 2 | Formatted through `Auth(String)` |
| `PrepareDelegationCertError` | 5 | 5 | Every variant formatted to broad invalid input |
| `CertRuleError` | 8 | 8 | Transparent child of preparation and verification errors; ultimately formatted |
| `VerifyDelegatedTokenError` | 23 | 23 | Five selected paths; other variants formatted to broad invalid input |
| `InstallActiveDelegationProofError` | 5 | 5 | Time failures selected; typed root-proof cause preserved; remaining variants formatted |
| `ChainKeyRootProofError` | 22 | 22 | A few stale/time groups selected by variant; all other variants formatted |
| **Total** | **97** | **96** | |

`PrepareDelegatedTokenError::IssuerProofFailed` is test-only, which explains
the one-variant difference. The table deliberately says *non-test*, not
*producer-reachable*: `CertRuleError::RootPidMismatch`,
`VerifyDelegatedTokenError::CertAudienceRejected` and
`VerifyDelegatedTokenError::MissingLocalRole` are compiled into production but
cannot be reached through the current private runtime call graph. Adding this
structural expansion to the first recursive pass gives an expanded structural
perimeter of **64 Canic-owned typed owners and 610 counted variants**. The
original 514 figure is declaration-based; the additional 96 excludes the one
known test-only variant. The combined number is therefore deliberately
conservative rather than a production-reachability claim. It is not a proposed
code count:
transparent wrappers, nested causes and shared semantics must not receive
duplicate codes. It is still a lower bound because direct prose constructors
and dependency-owned errors are not enum variants in that total.

## Typed Conversion Paths

### Delegated-token preparation

`map_prepare_delegated_token_error` currently treats:

- `CertExpired` as broad proof expiry;
- `TokenOutlivesCert` as broad stale auth material; and
- every other production variant, including nested `AudienceError` and
  `CanonicalAuthError`, as `Auth(err.to_string())`.

The two selected variants need exact preparation-specific leaves because one
requires certificate renewal and the other requires a shorter token TTL or
renewed certificate. The remaining eight direct preparation reasons and their
nested typed causes need exhaustive mappings. `Audience` and `Canonical` are
cause edges, not second allocation points.

### Delegated-token verification

`map_verify_delegated_token_error` currently preserves typed root- and
issuer-proof causes, distinguishes certificate pending/expired, token expiry
and issuer-proof absence, then formats every other reason as broad invalid
input. The 23-variant owner contains:

- 18 direct semantic reasons after excluding the two typed proof-cause carriers
  and the three transparent `Audience`, `Canonical` and `CertRules` edges; and
- nested audience, canonical and certificate-rule causes that must retain
  their own exact identity.

The metric mapper already matches every variant independently. It is useful
evidence of the current semantic split, but metric labels are not diagnostic
allocation authority.

### Certificate preparation and active-proof installation

All `PrepareDelegationCertError` variants currently become broad invalid input.
Only `CertTtlZero` and `CertExpiresAtOverflow` are direct leaves; `Audience`,
`Canonical` and `CertRules` forward nested causes.

`InstallActiveDelegationProofError` correctly preserves its typed root-proof
cause and distinguishes certificate pending and expiry. `IssuerMismatch` needs
its own binding leaf, while `Canonical` forwards its nested canonical cause.

### Chain-key root-proof verification

`map_chain_key_root_proof_error` currently selects these broad groups:

- root-key-policy expiry, policy mismatch and proof/key/Registry epoch floors
  become stale auth material;
- other expiry becomes proof expiry;
- not-yet-valid becomes proof pending; and
- everything else becomes invalid input.

The target-sensitive expiry split is real and must survive: expired protected
verifier policy requires configuration or authority repair, while an expired
proof requires renewal. `Canonical` is a cause edge. The remaining direct
variants describe distinct binding, policy, window, Merkle and signature
failures and must not be recovered from their formatted text.

`require_current_epoch_floors` separately formats `ProofEpochTooOld` and
`RegistryEpochTooOld` into stale-material errors. It must use the same exact
leaves as the main root-proof mapper rather than create another identity.

### Chain-key signing

`ChainKeySignerError` has four direct reasons plus one typed `InternalError`
cause carrier:

- header/policy mismatch;
- test-key rejection;
- derived public-key mismatch;
- signature verification failure; and
- management-call failure carrying an existing typed cause.

The current signing workflow marks every one of them `FailedRetryable`, stores
`err.to_string()` in `batch.failure`, schedules a retry and returns one generic
`Ops` error. That is not a valid one-code grouping. Management transport may be
retryable; protected policy, key identity and malformed-signature failures do
not have the same unchanged-retry contract. B4/B5 must split the typed
retry/terminal decision before replacing the durable string. The
`Management(InternalError)` carrier must preserve its existing diagnostic
instead of receiving a wrapper code.

## Reconciled Typed Leaf Additions

The structural variants above reduce to **64 new exact candidates** after
transparent cause edges, same-semantics reuse and current-path sediment are
removed. These are additions to the 48 exact candidates already recorded in
[auth-policy-leaves.md](auth-policy-leaves.md), not replacements for them.

### Audience and canonical input

| New exact candidate | Direct typed producer | Projection | Action |
| --- | --- | --- | --- |
| `AUTH_AUDIENCE_GRANTS_EMPTY` | `AudienceError::GrantsEmpty` | self | Supply at least one role grant |
| `AUTH_AUDIENCE_GRANT_COUNT_EXCEEDED` | `AudienceError::TooManyGrants` | self | Reduce grants to the bounded maximum |
| `AUTH_AUDIENCE_GRANT_SCOPES_EMPTY` | `AudienceError::EmptyGrantScopes` | self | Supply at least one scope for every grant |
| `AUTH_AUDIENCE_GRANT_SCOPE_COUNT_EXCEEDED` | `AudienceError::TooManyGrantScopes` | self | Reduce scopes to the per-grant maximum |
| `AUTH_CANONICAL_ROLE_EMPTY` | `CanonicalAuthError::EmptyRole` | self | Supply a nonempty canonical role |
| `AUTH_CANONICAL_ROLE_INVALID` | `CanonicalAuthError::InvalidRole` | self | Use only admitted role-label bytes |
| `AUTH_CANONICAL_SCOPE_EMPTY` | `CanonicalAuthError::EmptyScope` | self | Supply a nonempty canonical scope |
| `AUTH_CANONICAL_SCOPE_INVALID` | `CanonicalAuthError::InvalidScope`, `AudienceError::GrantScopeRejected` | self | Use only admitted scope-label bytes |
| `AUTH_CANONICAL_SCOPES_NONCANONICAL` | `CanonicalAuthError::NonCanonicalScopes` | self | Sort and deduplicate scopes |
| `AUTH_CANONICAL_ROLE_GRANTS_NONCANONICAL` | `CanonicalAuthError::NonCanonicalRoles`, `AudienceError::NonCanonicalGrants` | self | Sort and deduplicate role grants |
| `AUTH_CANONICAL_AUDIENCES_NONCANONICAL` | `CanonicalAuthError::NonCanonicalAudiences` | self | Sort and deduplicate audiences |
| `AUTH_CANONICAL_ISSUER_POLICIES_NONCANONICAL` | `CanonicalAuthError::NonCanonicalIssuerPolicies` | self | Sort and deduplicate issuer policies |
| `AUTH_CANONICAL_TOKEN_EXTENSION_TOO_LARGE` | `CanonicalAuthError::TokenExtTooLarge` | self | Reduce the bounded token extension |

`GrantScopeRejected` is the same invalid-label decision as
`CanonicalAuthError::InvalidScope`, and `NonCanonicalGrants` is the same
ordering decision as `NonCanonicalRoles`. Their current wrapper names do not
justify second codes.

### Token, certificate and active-proof semantics

| New exact candidate | Direct typed producer | Projection | Action |
| --- | --- | --- | --- |
| `AUTH_CERT_NOT_YET_VALID` | preparation, verification and active-proof `CertNotYetValid` | self | Wait for the certificate window or correct time evidence |
| `AUTH_TOKEN_TTL_ZERO` | `PrepareDelegatedTokenError::TokenTtlZero` | self | Request a positive token TTL |
| `AUTH_TOKEN_EXPIRY_OVERFLOW` | `PrepareDelegatedTokenError::TokenExpiresAtOverflow` | self | Correct issue time or TTL |
| `AUTH_TOKEN_OUTLIVES_CERT` | token preparation and verification `TokenOutlivesCert` | self | Shorten or renew the token/certificate |
| `AUTH_TOKEN_AUDIENCE_NOT_SUBSET` | token preparation and verification `AudienceNotSubset` | self | Restrict the token audience to its certificate |
| `AUTH_TOKEN_GRANTS_NOT_SUBSET` | token preparation and verification `GrantsNotSubset` | self | Restrict token grants to certificate grants |
| `AUTH_CERT_EXPIRY_OVERFLOW` | `PrepareDelegationCertError::CertExpiresAtOverflow` | `AUTH_PROOF_INVALID` | Repair protected issuance time/TTL arithmetic |
| `AUTH_CERT_WINDOW_INVALID` | `CertRuleError::InvalidCertWindow` | `AUTH_PROOF_INVALID` | Reject and reacquire the malformed certificate |
| `AUTH_CERT_TTL_EXCEEDED` | `CertRuleError::CertTtlExceeded` | self | Use a certificate within verifier policy |
| `AUTH_CERT_MAX_TOKEN_TTL_ZERO` | `CertRuleError::TokenTtlZero` | `AUTH_PROOF_INVALID` | Reissue the certificate with a positive token ceiling |
| `AUTH_CERT_MAX_TOKEN_TTL_EXCEEDED` | `CertRuleError::TokenTtlExceeded` | self | Reissue within verifier token policy |
| `AUTH_CERT_MAX_TOKEN_TTL_OUTLIVES_CERT` | `CertRuleError::TokenTtlOutlivesCert` | `AUTH_PROOF_INVALID` | Reissue with a ceiling no longer than the certificate |
| `AUTH_ISSUER_PROOF_BINDING_MISMATCH` | `CertRuleError::IssuerProofBindingHashMismatch` | `AUTH_PROOF_INVALID` | Reacquire an exactly bound certificate |
| `AUTH_CERT_HASH_MISMATCH` | `VerifyDelegatedTokenError::CertHashMismatch` | `AUTH_PROOF_INVALID` | Reacquire an exactly bound token and certificate |
| `AUTH_TOKEN_ISSUER_CERT_MISMATCH` | `VerifyDelegatedTokenError::IssuerPidMismatch` | `AUTH_PROOF_INVALID` | Use claims bound to the certificate issuer |
| `AUTH_TOKEN_WINDOW_INVALID` | `VerifyDelegatedTokenError::TokenInvalidWindow` | self | Supply a positive, ordered token window |
| `AUTH_TOKEN_ISSUED_BEFORE_CERT` | `VerifyDelegatedTokenError::TokenIssuedBeforeCert` | self | Issue the token within the certificate window |
| `AUTH_TOKEN_AUDIENCE_REJECTED` | `VerifyDelegatedTokenError::TokenAudienceRejected` | self | Use a token addressed to the local Fleet |
| `AUTH_TOKEN_GRANT_REJECTED` | `VerifyDelegatedTokenError::TokenGrantRejected` | self | Grant the exact local role |
| `AUTH_SCOPE_REJECTED` | `VerifyDelegatedTokenError::ScopeRejected` | self | Request only scopes carried by the local grant |

The following existing exact identities are deliberately reused:

- certificate expiry reuses `AUTH_CERT_EXPIRED`;
- token expiry, pending time and TTL ceiling reuse
  `AUTH_TOKEN_EXPIRED`, `AUTH_TOKEN_NOT_YET_VALID` and
  `AUTH_TOKEN_TTL_EXCEEDED`;
- zero certificate TTL reuses `AUTH_ROOT_ISSUER_CERT_TTL_ZERO` because the only
  production journey is the same protected root-issuer request;
- `CertRuleError::RootPidMismatch` reuses `AUTH_ROOT_AUTHORITY_INVALID` if its
  redundant current check is retained; and
- active-proof issuer mismatch reuses `AUTH_ISSUER_PRINCIPAL_MISMATCH` because
  both maintained paths bind the installed proof issuer to the local Canister.

`VerifyDelegatedTokenError::CertAudienceRejected` is unreachable because the
current audience-subset rule is equality: once token audience equals certificate
audience and the token audience is locally accepted, the certificate audience
must be accepted. `MissingLocalRole` is also unreachable because the sole
runtime adapter always supplies its protected local role. B4 must delete those
branches or prove a new producer before either can receive a number.

### Chain-key signing, lookup and proof verification

| New exact candidate | Direct typed producer | Projection | Action |
| --- | --- | --- | --- |
| `AUTH_CHAIN_KEY_SIGNER_HEADER_POLICY_MISMATCH` | `ChainKeySignerError::HeaderPolicyMismatch` | `AUTH_CHAIN_KEY_SIGNING_FAILED` | Stop retrying and repair protected batch/policy identity |
| `AUTH_CHAIN_KEY_TEST_KEY_FORBIDDEN_ON_IC` | `token::verifier_config::configured_chain_key_root_verifier`, `delegated::chain_key::verify_key_id_network` and `delegated::chain_key_signing::validate_signing_key_network`; IC-network branch | self | Configure the production key |
| `AUTH_CHAIN_KEY_TEST_KEY_OPT_IN_REQUIRED` | `token::verifier_config::configured_chain_key_root_verifier`, `delegated::chain_key::verify_key_id_network` and `delegated::chain_key_signing::validate_signing_key_network`; local-network opt-in branch | self | Explicitly allow the local test key or select another key |
| `AUTH_CHAIN_KEY_SIGNER_PUBLIC_KEY_MISMATCH` | `ChainKeySignerError::PublicKeyMismatch` | `AUTH_CHAIN_KEY_SIGNING_FAILED` | Stop retrying and reconcile the configured key |
| `AUTH_CHAIN_KEY_SIGNER_SIGNATURE_INVALID` | invalid signature returned by `ChainKeySignerError` | `AUTH_CHAIN_KEY_SIGNING_FAILED` | Stop retrying and inspect signer/key authority |
| `AUTH_CHAIN_KEY_CRYPTO_UNAVAILABLE` | feature-disabled `delegated::chain_key::{verify_chain_key_ecdsa_signature_enabled, verify_chain_key_ecdsa_public_key_shape_enabled}` and `delegated::chain_key_signing::normalize_chain_key_ecdsa_signature_enabled` | self | Deploy the role with the required crypto feature |
| `AUTH_TOKEN_RETRIEVAL_EXPIRED` | `RetainedDelegatedTokenLookupError::Expired` | self | Prepare a new token operation |
| `AUTH_TOKEN_RETRIEVAL_MISSING` | `RetainedDelegatedTokenLookupError::Missing` | self | Prepare the exact token before retrieval |
| `AUTH_CHAIN_KEY_PROOF_SCHEMA_MISMATCH` | `ChainKeyRootProofError::SchemaVersionMismatch` | `AUTH_PROOF_INVALID` | Reacquire a current proof |
| `AUTH_CHAIN_KEY_ROOT_MISMATCH` | `RootCanisterMismatch` | `AUTH_PROOF_INVALID` | Reacquire proof from the configured root |
| `AUTH_CHAIN_KEY_ISSUER_MISMATCH` | `IssuerCanisterMismatch` | `AUTH_PROOF_INVALID` | Reacquire proof for the exact issuer |
| `AUTH_CHAIN_KEY_HEADER_CERT_MISMATCH` | `HeaderDelegationCertMismatch` | `AUTH_PROOF_INVALID` | Reacquire an exactly bound proof |
| `AUTH_CHAIN_KEY_HEADER_SIGNATURE_MISMATCH` | `HeaderSignatureMismatch` | `AUTH_PROOF_INVALID` | Reacquire an exactly bound signature |
| `AUTH_CHAIN_KEY_CERT_MISMATCH` | `DelegationCertMismatch` | `AUTH_PROOF_INVALID` | Reacquire an exactly bound certificate |
| `AUTH_CHAIN_KEY_POLICY_MISMATCH` | `PolicyMismatch` | `AUTH_CHAIN_KEY_MATERIAL_STALE` | Refresh proof or reconcile protected verifier policy |
| `AUTH_CHAIN_KEY_PROOF_EPOCH_STALE` | `ProofEpochTooOld` | `AUTH_CHAIN_KEY_MATERIAL_STALE` | Renew at the accepted proof epoch |
| `AUTH_CHAIN_KEY_VERSION_STALE` | `KeyVersionTooOld` | `AUTH_CHAIN_KEY_MATERIAL_STALE` | Renew with the accepted key version |
| `AUTH_CHAIN_KEY_REGISTRY_EPOCH_STALE` | `RegistryEpochTooOld` | `AUTH_CHAIN_KEY_MATERIAL_STALE` | Renew against current Registry authority |
| `AUTH_CHAIN_KEY_POLICY_WINDOW_INVALID` | root-policy `InvalidWindow` and config validation | `AUTH_CHAIN_KEY_MATERIAL_STALE` | Repair protected policy timing |
| `AUTH_CHAIN_KEY_PROOF_WINDOW_INVALID` | batch/certificate `InvalidWindow` | `AUTH_PROOF_INVALID` | Reacquire a well-formed proof |
| `AUTH_CHAIN_KEY_POLICY_NOT_YET_VALID` | root-policy `NotYetValid` | self | Wait for the policy window or repair config |
| `AUTH_CHAIN_KEY_PROOF_NOT_YET_VALID` | batch/certificate `NotYetValid` | self | Wait for the proof window |
| `AUTH_CHAIN_KEY_POLICY_EXPIRED` | root-policy `Expired` | `AUTH_CHAIN_KEY_MATERIAL_STALE` | Replace expired verifier policy |
| `AUTH_CHAIN_KEY_PROOF_EXPIRED` | batch/certificate `Expired` | self | Renew the proof |
| `AUTH_CHAIN_KEY_PROOF_TTL_EXCEEDED` | `RootProofTtlExceeded` | `AUTH_PROOF_INVALID` | Renew within revocation policy |
| `AUTH_CHAIN_KEY_CERT_OUTSIDE_BATCH_WINDOW` | `DelegationCertOutsideBatchWindow` | `AUTH_PROOF_INVALID` | Reacquire a correctly bounded proof |
| `AUTH_CHAIN_KEY_MERKLE_WITNESS_INVALID` | `InvalidMerkleWitness` | `AUTH_PROOF_INVALID` | Reacquire valid witness evidence |
| `AUTH_CHAIN_KEY_SIGNATURE_LENGTH_INVALID` | `InvalidSignatureLength` | `AUTH_PROOF_INVALID` | Reacquire a correctly encoded signature |
| `AUTH_CHAIN_KEY_SIGNATURE_COMPONENT_ZERO` | `ZeroSignatureComponent` | `AUTH_PROOF_INVALID` | Reacquire a valid signature |
| `AUTH_CHAIN_KEY_SIGNATURE_HIGH_S` | `HighSSignature` | `AUTH_PROOF_INVALID` | Reacquire a canonical signature |
| `AUTH_CHAIN_KEY_SIGNATURE_INVALID` | cryptographic `SignatureInvalid` | `AUTH_PROOF_INVALID` | Reacquire valid signed proof material |

The dynamic `target` on the current window variants must become a bounded typed
target before mapping. Policy and proof windows remain separate because their
authority and remediation differ. Test-key rejection also splits by network:
"forbidden on IC" and "local opt-in missing" cannot retain one enum variant if
they are to have exact retry/action identity.

The `String` payloads inside signer/proof signature errors must not survive as
a hidden classification boundary. B4 must distinguish a missing compiled
crypto capability from invalid returned/submitted signature material and then
discard library prose.

## Direct Prose Constructors

There are 43 textual `AuthValidationError::Auth(...)` construction sites in
eight production auth files. A construction site is not necessarily one
semantic leaf because generic field helpers serve several fields, and one site
formats a nested typed owner. The current families are:

| Family | Current source owners | Hard-cut direction |
| --- | --- | --- |
| Attestation preparation/retrieval | `ops/auth/attestation.rs`, `ops/auth/root_canister_sig.rs` | Type TTL overflow, missing preparation and expired retrieval separately |
| Delegated-token ownership/retrieval | `ops/auth/token/mod.rs`, `ops/auth/token/retention/mod.rs` | Reuse subject/caller policy identity; type retained-token missing and expired separately |
| Verifier enablement | `ops/auth/token/verification.rs` | Exact canister-role verifier-disabled leaf |
| Root and IC trust-anchor configuration | `ops/auth/token/verifier_config.rs` | Finite typed configuration reasons; no principal, hex decoder or key bytes in diagnostics |
| Chain-key signing configuration | `ops/auth/delegated/chain_key_signing.rs` | Share typed configuration reasons with verifier construction where semantics and remediation are identical |
| Root delegated-auth configuration | `ops/auth/delegation/mod.rs` | Type missing root-proof configuration and invalid revocation latency |
| Typed preparation/lookup forwarding | `ops/auth/token/error.rs`, `ops/auth/token/mod.rs` | Exhaustive mappings from the source enums; no formatted forwarding |
| Duplicate wrapper | `ops/auth/attestation.rs` | Propagate `auth_proof_verifier_config()`'s existing `InternalError` unchanged |

The trust-anchor configuration prose reduces to a finite reason model. Field
names must be a bounded internal enum where code still needs to choose a
field; they must not remain dynamic strings. The provisional reason groups are:

- required field missing or empty;
- malformed principal;
- malformed hexadecimal value;
- fixed-length value has the wrong length;
- invalid secp256k1 public-key shape;
- derivation-path hash mismatch;
- invalid validity window;
- zero revocation-latency bound;
- unsafe test-key selection for the build network;
- test-key opt-in missing;
- required IC root key missing or empty;
- IC root key has the wrong length; and
- IC-versus-local root-key mismatch.

Missing and empty values may share one diagnostic only when they have the same
configuration owner and remediation. Verifier and signer helpers may share a
leaf only where their source authority, meaning and operator action are
identical; different wording alone does not justify another code.

### Reconciled direct-prose additions

The 43 construction sites plus adjacent direct `InternalError` prose reduce to
**20 new exact candidates**. Repeated helper sites and the typed forwarding
sites receive no additional identity.

| New exact candidate | Current producer family | Projection | Action |
| --- | --- | --- | --- |
| `AUTH_ATTESTATION_EXPIRY_OVERFLOW` | `AuthOps::prepare_role_attestation`; `issued_at_ns.checked_add(ttl_ns)` overflow branch | self | Correct issue time or TTL |
| `AUTH_ATTESTATION_RETRIEVAL_MISSING` | `AuthOps::get_role_attestation`; missing `PENDING_ROLE_ATTESTATIONS` entry | self | Prepare the attestation again |
| `AUTH_ROOT_PROOF_RETRIEVAL_MISSING` | `root_canister_sig::get_root_canister_signature_proof`; missing `PENDING_ROOT_PROOFS` entry | self | Prepare the exact root proof again |
| `AUTH_ROOT_PROOF_RETRIEVAL_EXPIRED` | `root_canister_sig::get_root_canister_signature_proof`; `retrieval_expires_at_ns` fence | self | Start a new proof preparation |
| `AUTH_TOKEN_VERIFIER_DISABLED` | `token::verification::require_current_canister_delegated_token_verifier`; disabled-role branch | self | Enable the verifier for the role before retry |
| `AUTH_CHAIN_KEY_POLICY_UNAVAILABLE` | `AuthOps::plan_due_chain_key_root_delegation_batch`, `AuthOps::signed_chain_key_delegation_proof_for_issuer` and `delegation::current_chain_key_registry_identity`; missing `chain_key_root` policy | self | Install the required verifier policy |
| `AUTH_TOKEN_RETENTION_ACTOR_CAPACITY` | `token::retention::prune_and_admit`; `max_active_per_actor` branch | self | Wait for pruning or reduce outstanding preparations |
| `AUTH_TOKEN_RETENTION_GLOBAL_CAPACITY` | `token::retention::prune_and_admit`; `max_active_per_command_kind` branch | self | Wait for pruning before retry |
| `AUTH_ACTIVE_DELEGATION_PROOF_MISSING` | `AuthOps::prepare_delegated_token_issuer_proof`; missing result from `AuthOps::active_delegation_proof` | self | Provision an active proof |
| `AUTH_ROOT_CANISTER_PRINCIPAL_INVALID` | `token::verifier_config::configured_root_canister_id`; empty and `Principal::from_text` branches | self | Correct the configured root Canister principal |
| `AUTH_CHAIN_KEY_CONFIG_REQUIRED` | `token::verifier_config::{required_chain_key_field, required_chain_key_u64}`, `delegated::chain_key_signing::{required_chain_key_field, required_chain_key_derivation_path, required_chain_key_u64}` and `delegation::required_chain_key_max_revocation_latency_ns` | self | Complete the bounded chain-key configuration |
| `AUTH_CHAIN_KEY_CONFIG_HEX_INVALID` | `token::verifier_config::{configured_chain_key_root_verifier, required_fixed_32_chain_key_hex}` and `delegated::chain_key_signing::{chain_key_signing_policy_from_config, required_chain_key_derivation_path, required_fixed_32_chain_key_hex}` | self | Correct hexadecimal configuration |
| `AUTH_CHAIN_KEY_CONFIG_FIXED_LENGTH_INVALID` | `token::verifier_config::required_fixed_32_chain_key_hex` and `delegated::chain_key_signing::required_fixed_32_chain_key_hex`; failed 32-byte conversion | self | Supply the required 32-byte value |
| `AUTH_CHAIN_KEY_PUBLIC_KEY_INVALID` | `token::verifier_config::configured_chain_key_root_verifier` and `delegated::chain_key_signing::chain_key_signing_policy_from_config`; `verify_chain_key_ecdsa_public_key_shape` rejection | self | Supply a valid public key |
| `AUTH_CHAIN_KEY_DERIVATION_PATH_HASH_MISMATCH` | `delegated::chain_key_signing::chain_key_signing_policy_from_config`; derived/configured hash inequality | self | Recompute and freeze the exact path hash |
| `AUTH_CHAIN_KEY_REVOCATION_LATENCY_ZERO` | `token::verifier_config::configured_chain_key_root_verifier` and `delegation::required_chain_key_max_revocation_latency_ns`; zero-value branches | self | Configure a positive bound |
| `AUTH_IC_ROOT_KEY_REQUIRED` | `token::verifier_config::configured_ic_root_public_key_raw`; absent or empty configured value | self | Configure the network's root key |
| `AUTH_IC_ROOT_KEY_HEX_INVALID` | `token::verifier_config::configured_ic_root_public_key_raw`; `decode_hex` rejection | self | Correct the root-key encoding |
| `AUTH_IC_ROOT_KEY_LENGTH_INVALID` | `token::verifier_config::configured_ic_root_public_key_raw`; `IC_ROOT_PUBLIC_KEY_RAW_LENGTH` mismatch | self | Supply the required raw key length |
| `AUTH_IC_ROOT_KEY_NETWORK_MISMATCH` | `token::verifier_config::validate_build_network_root_key_pair`; IC/local key mismatch branches | self | Use the root key for the exact build network |

The active-proof fallback needs a small hard cut rather than three status
codes. An expired local proof reuses `AUTH_CERT_EXPIRED`; a missing proof uses
`AUTH_ACTIVE_DELEGATION_PROOF_MISSING`; and a proof before `not_before_ns`
reuses `AUTH_CERT_NOT_YET_VALID`. The current status helper labels that last
case `Valid`, while the `RefreshNeeded` arm cannot be reached after a successful
active-proof lookup. B4 must make the time reason explicit and delete the
unreachable fallback instead of allocating `stale`, `valid` or `refresh`
diagnostics.

The direct subject/caller check reuses `AUTH_SUBJECT_CALLER_MISMATCH`. The
attestation verifier-config wrapper, root-authority text conversion and typed
retained-token forwarding preserve their existing exact causes.

## Same-Semantics Reuse And Wrapper Removal

The following paths must not allocate new wrapper identities:

- delegated-token prepare subject/caller mismatch reuses
  `AUTH_SUBJECT_CALLER_MISMATCH`;
- the attestation verifier-config rewrap propagates the original exact
  configuration diagnostic;
- `RootProofInvalid` and `IssuerProofInvalid` preserve their typed causes;
- `ChainKeySignerError::Management` preserves its typed management cause;
- `Audience`, `Canonical` and `CertRules` wrapper variants preserve their
  nested causes; and
- duplicate epoch-floor checks reuse the root-proof epoch leaves.

`ProofInvalid(String)` and `AttestationProofInvalid(String)` remain separate
internal leaves with the common safe public projection `AUTH_PROOF_INVALID`.
Their dependency error text is discarded. No proof bytes, key material,
principal, scope, epoch, timestamp or decoder text may enter the compact
public error.

## Signing Retry And Durable-State Contract

The current `FailedRetryable` transition is wrong for four protected failures.
The replacement decision is typed before persistence:

| Signer cause | Disposition | Durable diagnostic |
| --- | --- | --- |
| Header/policy mismatch | terminal | `AUTH_CHAIN_KEY_SIGNER_HEADER_POLICY_MISMATCH` |
| IC test key or missing local opt-in | terminal until configuration changes | exact test-key code |
| Derived public-key mismatch | terminal | `AUTH_CHAIN_KEY_SIGNER_PUBLIC_KEY_MISMATCH` |
| Returned signature invalid | terminal | `AUTH_CHAIN_KEY_SIGNER_SIGNATURE_INVALID` |
| Required crypto feature absent | terminal until redeployment | `AUTH_CHAIN_KEY_CRYPTO_UNAVAILABLE` |
| `Management(InternalError)` | inherit the typed management cause's disposition | preserve the nested code |

`Management` is not automatically retryable: transport/unavailability may use
bounded retry, while typed invalid-input, authorization or protected-identity
rejections remain terminal. B5 must replace the formatted `batch.failure` with
a numeric diagnostic and an explicit typed disposition/status (including a
terminal state). A failed exact batch remains inspectable; unchanged timers do
not resubmit terminal failures. The status/view DTO exposes only the numeric
safe projection while the protected batch remains the numeric observability
owner for the exact signer code.

The two new safe projections from this pass are:

- `AUTH_CHAIN_KEY_SIGNING_FAILED`, for protected signer output/policy failures;
  and
- `AUTH_CHAIN_KEY_MATERIAL_STALE`, for protected verifier policy and accepted
  epoch/key floors that require renewed authority material.

Both are deliberately less specific than their stored internal codes. All
cryptographic proof-shape and binding failures reuse the already-recorded
`AUTH_PROOF_INVALID` projection.

## Delegated-Session Public Boundary Correction

Source-to-ledger reconciliation found a gap in the preceding family
arithmetic. `AuthApi::set_delegated_session_subject` is a maintained public SDK
helper whose static decision branches were present at the pinned source
baseline, but Slice 7 of the dynamic-context ledger classified only their
interpolated values. Those branches were not added to the provisional exact-
leaf count. Dynamic-value closure is not producer closure.

The complete helper adds nineteen exact meanings. Eight subject-rejection
meanings are shared between the wallet-caller and requested-subject checks:

| New exact candidate | Typed producer | Class / origin | Projection | Disposition and action |
| --- | --- | --- | --- | --- |
| `AUTH_DELEGATED_SESSION_SUBJECT_ANONYMOUS` | `DelegatedSessionSubjectRejection::Anonymous` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; supply a non-anonymous user principal |
| `AUTH_DELEGATED_SESSION_SUBJECT_MANAGEMENT_CANISTER` | `DelegatedSessionSubjectRejection::ManagementCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; never use the management Canister as a user subject |
| `AUTH_DELEGATED_SESSION_SUBJECT_LOCAL_CANISTER` | `DelegatedSessionSubjectRejection::LocalCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; supply a user principal rather than the receiving Canister |
| `AUTH_DELEGATED_SESSION_SUBJECT_ROOT_CANISTER` | `DelegatedSessionSubjectRejection::RootCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; do not establish a user session for the configured root |
| `AUTH_DELEGATED_SESSION_SUBJECT_PARENT_CANISTER` | `DelegatedSessionSubjectRejection::ParentCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; do not establish a user session for the registered parent |
| `AUTH_DELEGATED_SESSION_SUBJECT_SUBNET_CANISTER` | `DelegatedSessionSubjectRejection::SubnetCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; do not establish a user session for a protected same-Subnet Canister |
| `AUTH_DELEGATED_SESSION_SUBJECT_FLEET_SUBNET_ROOT_CANISTER` | `DelegatedSessionSubjectRejection::FleetSubnetRootCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; do not establish a user session for the Fleet Subnet Root |
| `AUTH_DELEGATED_SESSION_SUBJECT_DIRECT_CHILD_CANISTER` | `DelegatedSessionSubjectRejection::DirectChildCanister` | `Forbidden` / delegated-session admission | self | `DoNotRetry`; do not establish a user session for a registered direct child |

The remaining eleven meanings bind the helper's time, subject, replay and
capacity decisions:

| New exact candidate | Current decision | Class / origin | Projection | Disposition and action |
| --- | --- | --- | --- | --- |
| `AUTH_DELEGATED_SESSION_ISSUED_TIME_OVERFLOW` | current IC seconds cannot be represented as nanoseconds | `Invariant` / delegated-session time | self | `DoNotRetry`; preserve state and inspect the observed time contract |
| `AUTH_DELEGATED_SESSION_SUBJECT_MISMATCH` | verified bootstrap-token subject differs from the requested subject | `Forbidden` / delegated-session binding | self | `DoNotRetry`; acquire a token for the exact requested subject |
| `AUTH_DELEGATED_SESSION_CONFIGURED_TTL_ZERO` | protected maximum session TTL is zero | `Invariant` / delegated-session configuration | `RUNTIME_CONFIGURATION_INVALID` | `RetryAfterStateChange`; correct configuration and reinstall |
| `AUTH_DELEGATED_SESSION_REQUESTED_TTL_ZERO` | caller requests a zero-second session | `InvalidInput` / delegated-session request | self | `DoNotRetry`; request a positive bounded TTL |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_TOKEN_INVALID` | canonical bootstrap-token fingerprint encoding fails | `Invariant` / delegated-session bootstrap | self | `DoNotRetry`; reject the token without exposing codec prose |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_REPLAY_REUSED` | an exactly bound token no longer owns an active matching session | `Forbidden` / delegated-session replay | self | `DoNotRetry`; acquire a fresh bootstrap token |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_REPLAY_CONFLICT` | a token fingerprint is bound to another wallet or subject | `Forbidden` / delegated-session replay | self | `DoNotRetry`; acquire a fresh token without exposing the retained binding |
| `AUTH_DELEGATED_SESSION_CAPACITY_EXCEEDED` | global active-session capacity is full | `ResourceExhausted` / delegated-session capacity | self | `RetryAfterStateChange`; inspect guarded capacity status and retry after expiry/pruning |
| `AUTH_DELEGATED_SESSION_SUBJECT_CAPACITY_EXCEEDED` | the requested subject has reached its session ceiling | `ResourceExhausted` / delegated-session capacity | self | `RetryAfterStateChange`; inspect request-scoped guarded capacity status |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY_EXCEEDED` | global live bootstrap-binding capacity is full | `ResourceExhausted` / delegated-session replay capacity | self | `RetryAfterStateChange`; inspect guarded capacity status and retry after expiry/pruning |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_BINDING_SUBJECT_CAPACITY_EXCEEDED` | the requested subject has reached its bootstrap-binding ceiling | `ResourceExhausted` / delegated-session replay capacity | self | `RetryAfterStateChange`; inspect request-scoped guarded capacity status |

The disabled-service branch reuses `AUTH_DELEGATED_TOKENS_DISABLED`; maximum-
TTL conversion reuses `AUTH_DELEGATED_TOKEN_MAX_TTL_OVERFLOW`; a bootstrap token
that expires at the seconds boundary reuses `AUTH_TOKEN_EXPIRED`; configuration
and token verification preserve their typed causes. The `Upserted` formatter
arm is unreachable after the preceding match and receives no code.

The four capacity identities require the guarded bounded status owners already
specified by `DPC-073` through `DPC-078`. The replay-conflict identity retains
only aggregate metrics and the protected binding; neither principal crosses the
public boundary. No generic session detail string is introduced.

### Producer And Observation Ownership

The nineteen rows have these exhaustive producer owners:

| Exact identities | Producer owner |
| --- | --- |
| eight `AUTH_DELEGATED_SESSION_SUBJECT_*` identities | `access::auth::validate_delegated_session_subject`; `AuthApi::set_delegated_session_subject` preserves the exact `DelegatedSessionSubjectRejection` variant for both the wallet and requested-subject checks |
| `AUTH_DELEGATED_SESSION_ISSUED_TIME_OVERFLOW` and `AUTH_DELEGATED_SESSION_SUBJECT_MISMATCH` | `AuthApi::set_delegated_session_subject` |
| `AUTH_DELEGATED_SESSION_CONFIGURED_TTL_ZERO` and `AUTH_DELEGATED_SESSION_REQUESTED_TTL_ZERO` | `AuthApi::clamp_delegated_session_expires_at`, preserving `AuthOps::clamp_delegated_session_expires_at`'s exact typed decision |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_TOKEN_INVALID` | `AuthApi::delegated_session_bootstrap_token_fingerprint` |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_REPLAY_REUSED` | `AuthApi::enforce_bootstrap_replay_policy` |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_REPLAY_CONFLICT` | `AuthApi::enforce_bootstrap_replay_policy` |
| `AUTH_DELEGATED_SESSION_CAPACITY_EXCEEDED` | `AuthStateOps::upsert_delegated_session_with_bootstrap_binding`, preserving the exact `DelegatedSessionUpsertResult` capacity variant; B4 deletes the prose-only `delegated_session_capacity_message` adapter |
| `AUTH_DELEGATED_SESSION_SUBJECT_CAPACITY_EXCEEDED` | `AuthStateOps::upsert_delegated_session_with_bootstrap_binding`, preserving the exact `DelegatedSessionUpsertResult` capacity variant; B4 deletes the prose-only `delegated_session_capacity_message` adapter |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY_EXCEEDED` | `AuthStateOps::upsert_delegated_session_with_bootstrap_binding`, preserving the exact `DelegatedSessionUpsertResult` capacity variant; B4 deletes the prose-only `delegated_session_capacity_message` adapter |
| `AUTH_DELEGATED_SESSION_BOOTSTRAP_BINDING_SUBJECT_CAPACITY_EXCEEDED` | `AuthStateOps::upsert_delegated_session_with_bootstrap_binding`, preserving the exact `DelegatedSessionUpsertResult` capacity variant; B4 deletes the prose-only `delegated_session_capacity_message` adapter |

Each table's current-decision text is its concise host summary. Every self-
projected identity is safe because it exposes only a finite admission, replay,
time or capacity decision and never a principal, token fingerprint, configured
capacity or dependency cause. It is publicly observable through the returned
code. `AUTH_DELEGATED_SESSION_CONFIGURED_TTL_ZERO` is the sole masked row: the
exact numeric code must be recorded in guarded authentication status before
returning `RUNTIME_CONFIGURATION_INVALID`. Its protected configuration remains
the retrieval owner. The replay-conflict metric and stable binding remain
aggregate/private observation only; neither is required to reconstruct the
public exact code.

These rows now contain producer, summary, class, origin, disposition, action,
projection, observation and exposure evidence. Numeric allocation and the
permanent catalogue owner remain deliberately unset until the complete B1 set
is reviewed together.

## Allocation Consequences

This pass does not add one code per structural variant. It reconciles the
structural and delegated-session perimeter to **103 new exact candidates and
two new safe projections**:

- 64 new candidates from the typed owner graph; and
- 20 new candidates from direct prose and adjacent broad constructors; and
- 19 new candidates from the maintained delegated-session public helper.

It establishes these decisions for the final B1 allocation:

1. wrappers and typed cause carriers preserve the nested code;
2. identical configuration decisions share finite typed reasons;
3. preparation, verification, installation and retrieval time failures remain
   separate when their retry action differs;
4. chain-key proof bindings and signature failures remain exact internally and
   use the safe proof projection where exposure requires it; and
5. durable signing state records typed retry disposition plus numeric
   diagnostic identity, not a formatted error.

Together with [auth-policy-leaves.md](auth-policy-leaves.md), authentication and
policy now account for **151 provisional exact candidates and six distinct safe
projections**. The structural 96-variant expansion remains evidence for the
reconciliation, not permission to allocate placeholders. The numbers remain
provisional until whole-ledger collision and observability checks complete.

## Dynamic Formatter Closure

Slices 40-46 of
[dynamic-public-context.md](dynamic-public-context.md) classify every dynamic
value on the maintained authentication formatter graph: attestation/root and
issuer Canister-signature proofs, delegated-token preparation/retrieval/
verification, chain-key verification/configuration/signing/batch state, base
scope/time errors, nested audience/canonical/certificate fields and both final
signature string buckets. Mixed-origin nested fields follow their enclosing
typed source; a caller-input route never authorizes protected issuer-policy or
Registry disclosure.

The producer trace also finds no workspace constructor for
`AuthExpiryError::{CertExpired, TokenExpired, TokenNotYetValid,
TokenTtlExceeded}`. B4 removes those four variants. Their semantic candidates
remain live because the maintained delegated-token preparation, verification
and active-proof types produce the same exact decisions. This is type sediment,
not an allocation reduction or a compatibility case.

Authentication dynamic-context closure is therefore complete at 105 rows
(`DPC-359` through `DPC-463`). The remaining B1 transitive formatter frontier
starts with Component Registry messages; it does not justify reopening generic
auth detail text.

## Required Tests

- exhaustive mapping for every retained variant of all ten typed owners,
  excluding the test-only preparation variant and deleting or explicitly
  guarding the two unreachable verifier variants;
- guards proving transparent wrappers and typed cause carriers allocate no
  second code;
- a guard proving certificate-audience rejection cannot follow the equality
  subset rule, or a new producer before it is numbered;
- a guard proving the runtime adapter always supplies the local role, or a new
  producer before `MissingLocalRole` is numbered;
- exact subject/caller semantic reuse;
- verifier/signing configuration parity tests for shared reason groups;
- separate IC-forbidden and local-opt-in test-key decisions;
- separate policy-versus-proof window mapping, including not-yet-valid and
  expired cases;
- proof that malformed principals, keys, signatures and decoder text do not
  cross the public boundary;
- separate retrieval-missing and retrieval-expired tests;
- typed chain-key signing retry-disposition tests, including terminal
  protected failures and preserved management-cause disposition; and
- current-schema encoding/restoration tests for the changed durable batch
  failure owner.

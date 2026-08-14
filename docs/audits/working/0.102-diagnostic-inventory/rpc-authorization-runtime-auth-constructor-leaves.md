# Canic 0.102 RPC Authorization And Runtime Auth Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all twelve production `InternalError`
constructor references in root-capability RPC authorization and top-level
runtime authentication contracts. It assigns no number and changes no runtime
behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/rpc/request/handler/authorize.rs` | 6 |
| `workflow/runtime/auth/mod.rs` | 6 |
| **Total** | **12** |

## Root-Capability RPC Authorization

The six sites add five exact authority meanings and reuse the Component-caller
identity qualified by the execution pass:

| Exact candidate or disposition | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `RPC_PROVISION_PARENT_MODE_INVALID` | 1 | `InvalidInput` / structural request | self | Request only `ThisCanister` parentage for structural provisioning |
| `RPC_PROVISION_PARENT_AUTHORITY_MISMATCH` | 1 | `Forbidden` / immediate-parent authority | self | Invoke from the exact registered immediate parent |
| reuse `RPC_COMPONENT_CALLER_REQUIRED` | 1 | `Forbidden` / Component-child authority | self | A Fleet Subnet Root cannot present itself as the Component parent of a child recycle |
| `RPC_CALLER_AUTHORITY_MISMATCH` | 1 | `Forbidden` / protected Registry caller | self | Derive authority for the exact transport caller; never substitute a presented binding |
| `RPC_ROOT_CALLER_AUTHORITY_REQUIRED` | 1 | `Forbidden` / root self-call authority | self | Root self-calls must use the protected Fleet Subnet Root caller authority |
| `RPC_COMPONENT_CALLER_AUTHORITY_REQUIRED` | 1 | `Forbidden` / registered Component authority | self | A Component caller must use its Component authority and cannot borrow root authority |

The three caller-authority decisions remain distinct. Principal mismatch,
root-self authority absence and a Component presenting root authority reject
different substitutions and must remain independently testable. None may be
collapsed into controller status, same-Subnet placement or a caller-presented
binding.

`EnvOps::require_root`, `RpcWorkflowError::NotChildOfCaller` and
`RpcWorkflowError::ChildNotFound` are transparent typed causes outside these
six direct sites. They preserve their already-qualified access and RPC
meanings; the authorization wrapper receives no code.

## Runtime Cryptographic Contracts

The six startup sites add four exact feature-contract meanings and reuse one
existing chain-key capability identity at two sites:

| Exact candidate or disposition | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `AUTH_ROOT_CANISTER_SIGNATURE_CREATION_UNAVAILABLE` | 1 | `Invariant` / root build contract | self | Rebuild the root with role-attestation signature creation support |
| reuse `AUTH_CHAIN_KEY_CRYPTO_UNAVAILABLE` | 2 | `Invariant` / root signer or verifier build contract | self | Rebuild the affected role with the admitted chain-key capability |
| `AUTH_ISSUER_CANISTER_SIGNATURE_CREATION_UNAVAILABLE` | 1 | `Invariant` / issuer build contract | self | Rebuild the issuer role with canister-signature creation support |
| `AUTH_ROOT_CANISTER_SIGNATURE_VERIFICATION_UNAVAILABLE` | 1 | `Invariant` / root-proof verifier build contract | self | Rebuild the verifier role with root canister-signature verification support |
| `AUTH_ISSUER_CANISTER_SIGNATURE_VERIFICATION_UNAVAILABLE` | 1 | `Invariant` / delegated-token verifier build contract | self | Rebuild the verifier role with issuer canister-signature verification support |

Creation and verification do not share an identity, and root-proof versus
issuer-proof verification remain separate. They require different Cargo
features, roles and remediation. Missing chain-key signing and verification
reuse `AUTH_CHAIN_KEY_CRYPTO_UNAVAILABLE` because the existing exact meaning
already owns missing compiled chain-key capability and its redeployment action.

The later `auth_proof_verifier_config()` call is transparent typed
configuration validation. It must preserve the exact trust-anchor diagnostic
rather than being reclassified as a missing build feature.

## Dynamic Public Context

The four role-bearing messages in the non-root contract are classified as
`DPC-195` through `DPC-198` in
[dynamic-public-context.md](dynamic-public-context.md). Each role is fixed by
the checked-in Component topology and the exact build target, so the value is
caller-derivable and is discarded from the compact diagnostic. RPC
authorization messages are static; caller, Subnet, capability and time values
appear only in structured logs and do not become public diagnostic fields.

## Reconciliation

All twelve direct sites now have one disposition. They add nine exact meanings,
reuse two existing identities across three sites and add no projection. The
effective constructor frontier moves from 2,217 to 2,229 classified sites and
from 282 to 270 open sites. The qualified semantic set reaches 2,470 exact
candidates plus 31 safe projections: 2,501 current symbolic identities.

## Required Tests

- exact structural-parent mode and immediate-parent authority rejection;
- wrong caller principal, root-self authority and Component-as-root authority
  rejection as three independent branches;
- proof that caller-presented binding, controller status and same-Subnet
  placement never replace Registry-derived caller authority;
- one exhaustive runtime build-contract mapping across all six sites;
- separate root/issuer creation and verification feature diagnostics;
- shared chain-key capability identity for signing and verification absence;
  and
- transparent trust-anchor configuration failure after feature checks pass.

## Next Slice

Continue with runtime-auth renewal and preparation admission, keeping timer,
durable batch and replay ownership separate.

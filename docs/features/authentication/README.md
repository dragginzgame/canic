# Authentication

<p align="center">
  <img src="../../../assets/1400x600/canic-authentication.jpg" alt="The Canic mechanic opening a secure door with an access key" width="700" />
</p>

Authentication answers two questions: **who is making this request, and are
they allowed to make it?** Canic checks those answers before application logic
runs.

The IC also needs to distinguish the canister that made a call from the user or
service it may represent. Canic keeps those identities separate so an
application login cannot accidentally become permission to manage canisters or
spend their cycles.

## What It Provides

- guards that protect public canister methods by caller, infrastructure role,
  or represented user
- reusable signed tokens that a receiving canister can verify locally
- root-managed chain-key delegation proof renewal
- issuer canister-signature proofs and bounded replay protection
- optional root-signed role attestation
- session identities that do not replace the calling canister's infrastructure
  permissions

Cargo features and `canic.toml` settings are both explicit. Issuer and verifier
roles must opt into the runtime capabilities they use.

## Boundary

Delegated subject identity is for application endpoints. Framework-owned
creation, placement, upgrade, recycling, and cycles operations continue to use
the raw transport caller and protected topology authority. Canic does not turn
an application token into controller or Fleet authority.

## Start Here

- [Authentication architecture](../../architecture/authentication.md)
- [Receiver-local caller authority proposal](../../architecture/authentication.md#receiver-local-caller-authority--design-proposal-2026-10-03)
- [Delegated-signature contract](../../contracts/AUTH_DELEGATED_SIGNATURES.md)
- [Access architecture](../../contracts/ACCESS_ARCHITECTURE.md)
- [Authentication configuration](../../../CONFIG.md#authdelegated_tokens)
- [Root proof provisioning](../../operations/root-proof-provisioning.md)

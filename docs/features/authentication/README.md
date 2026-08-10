# Authentication

Canic separates infrastructure caller authority from authenticated application
subjects. Endpoint guards authenticate before workflow code runs, and every
subnet, parent, subject, audience, and raw-caller binding remains explicit.

## What It Provides

- endpoint guards for callers, topology roles, and delegated subjects
- reusable delegated tokens verified locally by endpoint canisters
- root-managed chain-key delegation proof renewal
- issuer canister-signature proofs and bounded replay protection
- optional root-signed role attestation
- delegated session subject binding without replacing infrastructure authority

Cargo features and `canic.toml` settings are both explicit. Issuer and verifier
roles must opt into the runtime capabilities they use.

## Boundary

Delegated subject identity is for application endpoints. Framework-owned
creation, placement, upgrade, recycling, and cycles operations continue to use
the raw transport caller and protected topology authority. Canic does not turn
an application token into controller or Fleet authority.

## Start Here

- [Authentication architecture](../../architecture/authentication.md)
- [Delegated-signature contract](../../contracts/AUTH_DELEGATED_SIGNATURES.md)
- [Access architecture](../../contracts/ACCESS_ARCHITECTURE.md)
- [Authentication configuration](../../../CONFIG.md#authdelegated_tokens)
- [Root proof provisioning](../../operations/root-proof-provisioning.md)

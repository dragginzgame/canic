# Retired profile qualification fixture

The maintainer retired the profile application as a hands-on demo in favour of
[a persistent counter in the ongoing demo](demo.md). The profile protocol,
worker, discovery entry, manager and native test client now live under
`qualification/canic-profile-*`, with private versions independent of Canic releases.
There is no `make profile` product command.

These fixtures remain because the installed Auth/Blob composition, ownership,
capacity, failed-upgrade rollback, discovery and management tests still use them.
Moving them does not establish replacement counter coverage. Retire a fixture
only when its relevant failure cases have another qualified owner.

## Regression commands

From the repository root, after explicit simulator preparation:

```sh
make profile-test
make profile-clippy
make profile-doc
```

These are qualification commands, not application setup. The selected Storage
0.76 migration still blocks the persistent cases. Test scripts select the
qualification manifest explicitly when building profile artifacts and the private
client; they continue using the normal product CLI for deployment effects.

## Current mainnet demo

This heading retains the historical link target; the profile fleet is retired.
Its four original identities and tree are preserved in
[`qualification/fixtures/profile-mainnet.toml`](../../qualification/fixtures/profile-mainnet.toml).
The journal resolves to the same repository-root `.canic` directory. For explicit
read-only inspection from the repository root:

```sh
canic fleet canister list --config qualification/fixtures/profile-mainnet.toml
canic fleet show --config qualification/fixtures/profile-mainnet.toml --manager rk6n2-wyaaa-aaaad-qmoua-cai
```

No canister has been stopped, deleted, reassigned or reinstalled by retirement.
Existing journals, approval digests and controller authority remain unchanged.
The [qualification history](../qualification/history.md#profile-mainnet-deployment-and-preservation)
retains the actual deployment and preservation evidence.

## Store-backed profile fleet

The existing `profile_store` case in `make root-test` retains the Root/Store/profile
consumer proof. Its script now builds the profile components from the qualification
workspace. The current product workflow is the [Root journey](root.md); the
profile fixture does not define the new demo's application or recovery contract.

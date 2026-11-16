# Optional Auth receiver witness

Canic mounts three receiver endpoints behind `canic`'s optional `auth` feature:

| Export | Mode | Passive contract |
| --- | --- | --- |
| `establish_portable_session` | Update | `EstablishSession` (`0x415554480012`) |
| `portable_session_read` | Query | `ReadSession` (`0x415554480013`) |
| `logout_portable_session` | Update | `Logout` (`0x415554480014`) |

The application supplies synchronous handlers through `mount_auth_endpoints!`.
They receive the actual ingress caller and IC time. Input credentials are
untrusted material; they cannot select enrollment, issuer keys or application
permission. Current application operations must authorize through the real Auth
receiver with protected scopes, including a fresh check after an await.
A returned session view is a receipt, not continuing authority.

Auth owns proof validation, sessions, replay, logout, throttling and enrollment
admission. Canic owns export composition and guards, Platform owns generic IC
entrypoints, and Wire owns codecs. Each direction preserves all four supplier
allowances. A bad size/header is refused before copying the body; malformed
decoding never invokes the handler. Encoding failure traps, rolling back the
whole synchronous message; ordinary Auth refusals commit applicable charges.
Mounting does not initialize an engine, enroll authorities or activate readiness.

## Direct Root enrollment

The additional `auth-enrollment` feature uses the supplier engine to validate
and install a complete, authenticated Root snapshot. Root's optional `auth`
feature mounts `configure_auth`, `revoke_auth` and `auth_enrollment`. Its existing
controller guard admits configuration and revocation; the snapshot is bound to
the actual configured receiver. No controller gains browser identity or user
permission. This first runtime contract permits one explicit immutable receiver,
audience, issuer, Ed25519 key, role and scope grant with a finite approval deadline.
An exact configuration retry cannot replace the grant or reenable a revoked key.

Receiver lifecycle explicitly begins refresh, acquiring IC randomness under an
already reserved 30-second receipt deadline. One consuming ticket makes one
direct Root call. Pending work stays busy after a failed call or callback trap;
only expiry permits a new attempt, whose sequence fences late replies. Complete
entry validation and canonical commitments precede supplier metadata publication
and host entry replacement in the same callback. Snapshot freshness is at most
60 seconds and cannot exceed the grant deadline. A refresh never extends an
existing session. Remote clock differences can cause conservative refusal.

The installed witness now uses this actual Root path, replacing its earlier
fixed-enrollment shortcut. Root or receiver upgrades lose volatile Auth safety
state and remain closed. Root management custody still reopens independently;
its ordinary maintenance receipt does not claim Auth preservation. Default fleet
recipes do not enable Root's `auth` feature; managed Auth preservation is pending.

## Approve a fleet grant

For an already installed Auth-enabled Root and receiver, add an optional `auth`
table under `apps.APP.fleets.FLEET` in the existing config. It has these fields:

| Fields | Required meaning |
| --- | --- |
| `root`, `receiver` | Distinct app slot names with explicit fleet principal bindings and app-owned Wasm paths matching installed code |
| `issuer` | Application-approved issuer principal |
| `key` | Ed25519 verification public key, 64 lowercase hex digits; never a private key |
| `network_id`, `application_id` | Explicit canonical identities, each 64 lowercase hex digits; not display names or Host's network-binding digest |
| `role`, `scopes` | Application role and a nonempty, sorted, unique list of application scopes |
| `not_before_ns`, `expires_at_ns` | Original finite authority window, in Unix nanoseconds |

Root must have the selected operator as its sole controller. The receiver must
have Root as its sole controller. Roles, tree relationships and membership cannot
substitute for either check or confer application permission. Planning retains
the complete grant and exact code/network bindings without a network mutation:

```sh
canic fleet auth plan --app demo --fleet local --controller ROOT_CONTROLLER
canic fleet status CONFIGURATION_OPERATION --app demo --fleet local
# After reviewing the retained plan, submit its exact grant:
canic fleet apply CONFIGURATION_OPERATION --app demo --fleet local --identity OPERATOR_IDENTITY
```

Application lifecycle still owns the explicit receiver refresh. A successful
configuration reply establishes neither enrollment nor login readiness.
The existing journal owns all signatures and replies; changed config cannot
amend an approval. Replanning identical input returns the original operation.

After a successful configuration reply is retained, terminal revocation uses
that original grant, even if the source config has changed:

```sh
canic fleet auth plan-revocation CONFIGURATION_OPERATION --app demo --fleet local
canic fleet apply REVOCATION_OPERATION --app demo --fleet local --identity OPERATOR_IDENTITY
# Recover an interrupted call using its original signed attempt:
canic fleet reconcile CONFIGURATION_OPERATION --app demo --fleet local --identity OPERATOR_IDENTITY
```

An expired signed attempt without its reply stays blocked. Missing evidence is
not permission to re-sign or to revoke an unknown configuration through a new
path. Revocation cannot select a different grant or reopen a retired key.
Already issued snapshots remain usable only within their original freshness.

Known Auth approvals block Root/receiver preservation upgrades in the same
journal, including after revocation. Fresh enrollment refuses existing upgrade
approvals. Keep the journal: this is a local workflow fence, not discovery of
out-of-band Auth configuration. Direct upgrades still lose volatile Auth state
and stay closed. Stable Auth needs its supplier's persistence contract; Canic
does not add a temporary store or reconstruct lost replay evidence.

## Run locally

After `make canic` and `make qualification-setup`:

```sh
make auth-test
make auth-clippy auth-doc
```

The test builds through the same public artifact owner used by applications:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --output target/auth/receiver.wasm
canic artifact build --package canic-root --features auth --output target/auth/root.wasm
canic artifact inspect target/auth/receiver.wasm
```

This uses PocketIC and synthetic cycles. It opens no browser and deploys nothing
to mainnet. The qualification-only canister is a separate application artifact,
not another shipped adapter package or default Canic dependency.

## Inputs and limits

Both lockfiles select immutable Auth Git revision
`b78991e9833320692921bf766e51ad361c9c8c4d`: engine `ic-auth` 0.46.1 and passive
`ic-auth-wire` / `ic-auth-protocol-types` 0.42.0. The basic `auth` feature selects
only passive contracts; `auth-enrollment` selects `receiver-memory` and
`ed25519-verification`. No sibling edits, source copies or supplier publication
are required. The exact tested artifact is recorded in
[qualification history](../qualification/history.md).

The witness has a controller-approved Root grant and a public test signer,
two session slots and bounded volatile replay/verification state. It
proves caller mismatch and unauthorized configuration refusal, malformed frames,
ordinary cryptographic refusal charges, exact retries, current authority reads,
logout/replay, and actual reply-encoder failure rolling back Auth and application
state. It also covers a real enrollment callback trap, retained busy admission,
freshness-only refresh preserving the original session, in-flight revocation,
and upgrades of both owners staying closed. Native contract cases cover forged
bindings, stale/expired/conflicting replies and terminal revocation. It provides
no durable session preservation or complete Fleet login qualification.

The production engine remains supplier-owned and standalone. Remaining scopes:

- [#9 — Fleet-authorized Auth enrollment](https://github.com/dragginzgame/canic/issues/9).
- [#10 — Auth issuance, renewal and reconciliation](https://github.com/dragginzgame/canic/issues/10).
- [#11 — Installed profile/browser Auth journey](https://github.com/dragginzgame/canic/issues/11).

Certified proof mounting, browser lifecycle, durable Auth state and recovery are
not established by this receiver witness. There is no Auth CLI or parallel
deployment workflow yet.

# Optional Auth composition witness

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

## Issuer endpoints

The same optional `auth` feature exposes passive Canic-owned transport contracts
and `mount_auth_issuer_endpoints!` for application handlers:

| Export | Caller contract | Behavior |
| --- | --- | --- |
| `issue_portable_credential` | Independently authenticated account | Reserve one exact intent before entropy/signing |
| `renew_portable_credential` | Browser authenticated against the original login | Preserve grants, login deadline and exact renewal sequence |
| `reconcile_portable_credential` | Original account or browser signer role | Lookup only; return original completion or unresolved evidence |

All three are replicated updates. Initial requests cannot supply an account,
issuer key, policy or deadline. Renewal's account is a selector verified through
Auth's retained browser binding. Input frames are capped at 8 KiB; replies at
128 KiB, with separate allocation/node/depth bounds. Canonical claims and proof
bytes remain supplier-owned and are returned unchanged. The application supplies
async issue/renew handlers and a synchronous reconciliation handler. Mounting
installs no engine, key, storage, enrollment or timer.

The witness uses the real `MemoryIssuer` with two login slots, four operations,
16 KiB retained result reservations, a 300-second login, 60-second proof window,
30-second completion window and 60-second result retention after the later
proof/completion deadline. Its application account differs from its installer.
It rechecks permission after real IC entropy; local test-key signing and result
publication share the callback transaction. Reply failure traps that transaction
and retains the earlier reservation as unresolved. Exact duplicate delivery and
reconciliation never start another signing attempt. Explicit supplier pruning
closes old epochs instead of making consumed request identities reusable.

Run the selected installed cases with:

```sh
bash scripts/dev/test-auth.sh auth_issuer_tests
```

These tests independently verify signatures and count committed local signing
results. One credential is admitted through the real Root-enrolled receiver.
This is a local test-key witness, not a production key service or the complete
profile/browser journey. There is no external signer effect in this slice, and
no claim of external signing recovery or certified proof publication. A trapped
local signature is rolled back with its result and cannot escape the callback.

The pinned supplier's `MemoryIssuer::revoke` accepts an authenticated account;
its contract explicitly leaves browser-authenticated issuer logout to a later
binding API. Canic needs that API to verify the original browser/login and revoke
that retained account login atomically. It must retain result evidence and fence
late completion. Canic does not add a parallel login index or reinterpret the
browser caller as an account to work around it. Receiver logout remains available.

## Run locally

After `make canic` and `make qualification-setup`:

```sh
make auth-test
make auth-clippy auth-doc
```

For the installed enrollment concurrency and expiry cases alone:

```sh
bash scripts/dev/test-auth.sh auth_enrollment_tests
```

The concurrent case independently counts committed reservations, direct Root
dispatches and installed snapshots: an overlapping refresh cannot renew the
original deadline or create another call. The expiry case separates Root and
receiver onto two subnets so each real await can be interrupted. Expiry while
obtaining entropy leaves admission closed; expiry while fetching a snapshot
preserves the previous supplier metadata and application entry. Only a later
explicit refresh reserves a new window. These counters belong to the fixture,
not the production protocol. A Testkit 0.107.3 barrier now delays genuine Root
responses before publication for two independent receivers. Releasing the revoked
receiver first cannot install or replace the other's active grant, challenge or
deadline. The installer alone selects that qualification-only barrier; it supplies
no snapshot or authenticated marker. There are no simulated-time jumps while held.
Same-receiver expired/replaced response markers retain native coverage only: this
does not claim an installed arbitrary network-reordering proof or complete #9.

The authority-loss case stops or upgrades a still-active Root after a real
session is admitted. Both a transport rejection and Root's closed response leave
the previous snapshot and session deadlines intact. Failed refreshes retain their
original busy window; a later explicit retry cannot extend the old authority.
Queries admit the original session immediately before freshness expires and refuse
at and after that boundary. Root loss is not an instantaneous broadcast to
receivers: their already accepted authority remains bounded by its original lease.

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
`bfe2e548a997f10a876f01b4c72c4e5fec52c8d6`: engine `ic-auth` 0.46.1 and passive
`ic-auth-wire` / `ic-auth-protocol-types` 0.42.0. The basic `auth` feature selects
only passive contracts; `auth-enrollment` selects `receiver-memory` and
`ed25519-verification`. Only the qualification application's issuer additionally
selects `issuer-memory` and Platform's local signing API. No sibling edits, source copies or supplier publication
are required. The exact tested artifact is recorded in
[qualification history](../qualification/history.md).

Application signing and the native test verifier now select registry Platform
0.34.9's `ed25519` capability as `ic-platform-crypto`. Enrollment and issuer entropy
use Protocol's `raw_rand` with explicit six-byte request and 42-byte reply budgets.
Product selects Protocol 0.2.7; qualification retains Testkit 0.107.3's exact
0.2.6 selection. The witness and its real Root use Runtime 0.23.14, and committed
Auth source uses the matching Types 0.21.2 boundary. The later browser-logout
checkpoint `6d5c9704152cea05d6a095c8765fd74368c7d538` is now committed; its
adoption belongs to Canic #16/#17 and is deferred to that integration increment.
Host transport migration remains separate.

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
not established by this witness. The CLI supports grant configuration and
revocation through the existing fleet journal. Issuer canister mounts exist;
issuer CLI commands, browser login and browser-bound issuer logout remain pending.

# Optional Auth composition witness

Stateful fleet receivers keep their sessions and replay protection in stable
leases. After an upgrade, the receiver reopens its checkpoint and must refresh
its enrollment from Root before serving sessions. See [Stateful Auth storage](#stateful-auth-storage).

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

## Hubs without a database

Enable `receiver-role` for the stable receiver lifecycle. Add `role-attestation`
for the retained service directory and verifier used by the Toko hubs.

```rust
canic::mount_receiver_lifecycle! {
    hub,
    config_key = "app.hub.config.v1",
    authority = "app",
    auth_read = "read",
}
```

Omit `auth_read` on a discovery-only hub to leave the portable session endpoints
unmounted. A private issuer callback uses
`#[canic::update(guard = canic::guard::require_issuer)]`. Its authority
comes from the actual caller and protected receiver installation. A browser session
is unnecessary. The stable receiver recovers that binding on upgrade.

Existing hubs must preserve their original authority, config key and other
allocations. For example, `reserved_keys = ["app.hub.issuer.v1"],` retains an
obsolete issuer allocation without reading it as a second source of authority.
Optional `init`, `pre_upgrade` and `post_upgrade` hooks run after receiver
initialization, sealing and reopening respectively. Failed reopen traps before
the application hook runs.

## Direct Root enrollment

The additional `auth-enrollment` feature uses the supplier engine to validate
and install a complete, authenticated Root snapshot. Root mounts `grant_auth_access`, `revoke_auth_access`, `fetch_auth_enrollment` and
`refresh_auth_receiver`. Its existing
controller guard admits configuration and revocation; the snapshot is bound to
the actual configured receiver. No controller gains browser identity or user
permission. Root retains up to 32 receiver principals, each with an immutable audience,
issuer, Ed25519 key, role, scopes and finite approval deadline. Each principal
has its own snapshot revisions, challenge history and terminal revocation. A
revoked grant still occupies its slot.
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
fixed-enrollment shortcut. After a grant, upgrading Root closes its grant authority. Root management custody still reopens independently;
its ordinary maintenance receipt does not claim Auth preservation. Granting receiver authority requires explicit operator approval.

## Approve a fleet grant

For an installed Root and Auth receiver, add an optional `auth`
table under `apps.APP.fleets.FLEET` in the existing config. It has these fields:

| Fields | Required meaning |
| --- | --- |
| `root`, `receiver` | Distinct bound Root and receiver slots with reviewed Wasms matching installed code |
| `issuer` | Application-approved issuer principal |
| `key` | Ed25519 verification public key, 64 lowercase hex digits; never a private key |
| `network_id`, `application_id` | Explicit canonical identities, each 64 lowercase hex digits; not display names or Host's network-binding digest |
| `role`, `scopes` | Application role and a nonempty, sorted, unique list of application scopes |
| `not_before_ns`, `expires_at_ns` | Original finite authority window, in Unix nanoseconds |

Root must have the selected operator as its sole controller. The receiver's sole
controller defaults to Root. For an independently installed hub such as Toko's,
set `receiver_controller = "operator"` in the fleet's Auth table. This binds the
same operator principal already named in the plan. Both installed code hashes
and both exact controller sets must match before dispatch. For fleet-supplied
Root with no app Wasm path, `root_wasm` selects the reviewed file relative to
`canic.toml`. A declared app Root Wasm cannot be overridden. Roles, tree relationships and membership cannot
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

A receiver’s Auth enrollment leaves application upgrades available. A receiver
can resume an unexpired session after reopening and refreshing its Root grant.
An upgrade approval grants no new Auth authority. Existing approvals and
revocations keep their original code and controller bindings.

The journal still blocks a Root upgrade when it holds grant authority, including
terminal revocations. A previously approved Root upgrade also blocks a new grant
configuration. Qualifying preservation of those records will lift that restriction.

## Issuer endpoints

The same optional `auth` feature exposes passive Canic-owned transport contracts
and `mount_auth_issuer_endpoints!` for application handlers:

| Export | Caller contract | Behavior |
| --- | --- | --- |
| `issue_portable_credential` | Independently authenticated account | Reserve one exact intent before entropy/signing |
| `renew_portable_credential` | Browser authenticated against the original login | Preserve grants, login deadline and exact renewal sequence |
| `reconcile_portable_credential` | Original account or browser signer role | Lookup only; return original completion or unresolved evidence |
| `revoke_portable_login` | Browser authenticated against the original login | Revoke through Auth's retained binding, retaining original results/deadline |

All four are replicated updates. Initial requests cannot supply an account,
issuer key, policy or deadline. Renewal's account is a selector verified through
Auth's retained browser binding. Input frames are capped at 8 KiB; replies at
128 KiB, with separate allocation/node/depth bounds. Canonical claims and proof
bytes remain supplier-owned and are returned unchanged. The application supplies
async issue/renew handlers and synchronous reconciliation/logout handlers. Mounting
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
An installed test rotates the fixture's signing key and authority generation
while a browser renewal awaits entropy. The callback and a duplicate renewal
are refused. Lookup of the renewal stays unresolved, while lookup of the
completed issue returns the original credential and deadline. A fresh device
login verifies under the replacement key. Rotation is an installation-owned
fixture control using public test seeds.

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

`MemoryIssuer::revoke_browser` verifies the captured browser against the original
account/login selectors and atomically revokes that login. Exact repeats return
the original deadline. Initial/renewal work completing later stays fenced; other
devices and retained completed results remain available. Missing or pruned records
refuse logout. The interruption fixture also refuses logout after an upgrade
that skips its issuer checkpoint. The installed `auth_logout_tests` cover these boundaries,
malformed input and post-mutation encoding failure rolling back revocation.
There is no Canic login index or reinterpretation of a browser as an account.
Receiver logout remains its separate, unchanged unit-input operation.

The [optional browser adapter](../../crates/canic/browser/README.md) now implements
Auth's issuer transport and adapts this login-specific logout to Auth cleanup.
It checks the original intent against the selected Ed25519 device's public key
before issuance, and derives the expected browser signer from that key.
It preserves original signer roles and clock conversion across lookup. Its generated codecs come
from the mounted Rust DTOs. The installed profile slice below advances
[Canic #11](https://github.com/dragginzgame/canic/issues/11).

## Installed profile/browser composition

The witness's optional `profile` feature reuses `canic-profile-worker::state`
without the worker's standalone entrypoints. There is one profile persistence
engine, with the same immutable owner/user binding, revision and capacity rules.
Auth supplies an authenticated subject under the receiver's one protected issuer
namespace. A browser is a transport identity; being an operator or controller
does not give access to a user's profile. The default profile worker remains
unchanged and does not acquire an Auth dependency.

Prepare the simulator/npm inputs, then select an already installed Chromium:

```sh
make qualification-setup auth-browser-prepare
CANIC_AUTH_BROWSER_EXECUTABLE=/absolute/path/to/chromium make auth-profile-test
```

This downloads no browser and runs headless, with no desktop windows. The browser
uses the real Platform SDK and Auth coordinator/journal against the simulator's
HTTP gateway with an explicit root key. A separate issuer and two existing
Roots each admit one independent receiver. The single-Root test below exercises
the hub and shard topology separately. Qualification keys are public fixed seeds.

Eleven browser cases cover lost issuance, renewal and admission replies across renderer
reopening, independent device/user logins, profile isolation and immutable IDs,
reply-encoding failure rolling back actual profile writes, renewal and original
result lookup, logout/cleanup reopening, retained credential replay refusal,
wrong-scope admission with actual Unconfirmed cleanup, another renderer's logout
fencing a suspended SDK signer, and logout before an actual issuer result reaches
the coordinator. A blocked browser HTTP request preserves the cleanup attempt and
deadline across renderer loss; resumed cleanup confirms only authenticated replies.
An attempt timing out at the original cleanup deadline is explicitly resumed from
that same journal, never assigned a fresh deadline. The late issuer result cannot
restore readiness, and its unknown issuance remains visible even after confirmed
cleanup. Replacement uses a fresh key and journal; Auth refuses overwriting even
a finished journal. The old journal and revoked issuer login remain retained.
The fixture delays terminal key disposal until replacement admission, then checks
the exact old binding before deletion. Replayed old revocations leave the new
session and immutable profile owner intact; disposing an active login refuses
before invoking host custody. These are public fixed-seed custody hooks, not a
production wallet or key-deletion guarantee. The issuer fixture admits five retained
logins and three slots per account to include the revoked predecessor; revocation
does not reclaim its slot. The native harness independently observes six committed
signatures, five logins and exactly two successful profile writes.
The browser interruption fixture deliberately skips the Auth checkpoint on upgrade.
Both profiles reopen, but the receiver refuses the previously active login and
returns `Closed` on refresh.

The independent device requests only `read`: its authenticated profile read succeeds
while a write is refused without changing the stored profile. The final browser case
revokes one actual Root grant using a controller-signed HTTP call, then refreshes
that receiver through its existing direct enrollment pull. Cached browser readiness
cannot authorize reads or writes after that refresh. An exact configuration retry
cannot clear revocation, and the independently enrolled receiver remains usable.
Native observations require zero committed writes at the revoked receiver.
This checks authority at application execution; it does not claim that Root revocation
instantly invalidates a receiver's still-fresh earlier snapshot.

The local clock mapping subtracts one second from credential/session deadlines.
It is an explicit fixture assumption, not production cross-clock synchronization.
Auth's verification bucket starts empty; the harness allows twenty seconds of
simulated policy time before real-time browser progression. It does not bypass
budgets or extend accepted credentials/snapshots. Node output is bounded to 64 KiB
per stream and 90 seconds; the simulator has a separate 180-second lifetime.

This proves local composition, not a shipped Auth-enabled profile deployment.
The late-result case holds a real authenticated result after server completion;
it does not pause the issuer's remote entropy callback. Installed native tests
cover that callback fence and authority loss separately. Canic #11 now has installed
evidence for its selected Ed25519 browser flow. The Canic #9 prerequisite now has
an installed same-receiver supersession proof using Testkit 0.109.0's bounded
`advance_time`, supplied by [Tooling #11](https://github.com/dragginzgame/ic-tooling/issues/11).
The original production receipt window is unchanged. A stale active completion
cannot clear a newer pending refresh or overwrite its committed revoked snapshot;
both release orders preserve supplier metadata, deadlines and counters. Raw clock
changes and automatic progress remain forbidden while replies are held.
Production key custody,
certified issuer proofs remain unqualified. The stable Ed25519 path is described below.

## Internet Identity account handoff for Toko

Internet Identity (II) delegates a short-lived session key for the application's
origin. On an authenticated update the issuer sees the account principal as the
IC caller. That principal alone does not identify the provider: an unrelated
key can also make authenticated calls. Keep the II-delegated account signer
available for initial issuance and original issue lookup. Give Canic the
separate portable browser public key in the issue request. The issuer derives
the credential subject from the authenticated account and checks Toko's allowed
grants. Browser renewal uses the issuer's retained account/login binding.
[ICP sign-in guide](https://docs.internetcomputer.org/guides/authentication/internet-identity/)

### Where Toko verifies the provider

The Rust reference implementation is DFINITY's `identity-attributes`. Its
`sign_in_finish` reads the signer and bundle attached to the authenticated IC
message. It checks the signer against `trusted_attribute_signers`, verifies the
bundle, then calls the application's callback with `msg_caller()`. The local
deployment must set that signer to its installed II principal and set
`frontend_origins` to the exact Toko frontend origin. The verifier checks the
origin and age and consumes a backend-issued nonce. The request body supplies
none of these trust settings. [Verifier source](https://github.com/dfinity/identity-attributes/blob/24da86edfd05bee13874e3f5a5da92f5b7978ee2/rust/src/lib.rs)
and [bundle checks](https://github.com/dfinity/identity-attributes/blob/24da86edfd05bee13874e3f5a5da92f5b7978ee2/rust/src/internal/verify.rs).

Toko's local fleet installs a separate verifier. Its `on_verified` callback
records an expiring admission for the authenticated principal. Only the configured
issuer may query that admission. The issuer captures the ingress caller, asks the
verifier about that principal, checks the application's allowed grants, and checks
the deadline again after obtaining entropy. Its disposable signing key binds the
account to the distinct browser public key. Toko owns these admission decisions.

### Installed Toko witness

From a prepared Toko checkout run `make test-ii-signup`. Its preparation and pinned
inputs are in `docs/development.md` under “Local Internet Identity signup”. The
verifier uses DFINITY's Git revision `24da86edfd05bee13874e3f5a5da92f5b7978ee2`.
The browser driver, verifier and test issuer are in `qualification/ii/browser.mjs`,
`qualification/toko-ii-verifier` and `qualification/toko-ii-test-issuer`.

The headless Chromium witness signs in to an installed II with a passkey, presents
the signed attributes, rejects a consumed nonce, and refuses credential issuance
to an unrelated self-authenticating key. The issuer binds the accepted account
to the browser key, and both hub and assigned shard enforce the resulting session.
The browser recovers an interrupted logout, then a fresh sign-in with a different
browser key returns to the same account, route and edited profile. The Canic
integration run also exercised a plain finish call from the unrelated key without
an II bundle. This covers the local handoff in [#21](https://github.com/dragginzgame/canic/issues/21).
It uses a disposable issuer and local root of trust.

### Opt-in receiver

Select `auth-enrollment` for each participating Wasm. It includes the `endpoints`
runtime for named application methods. Declare the shared fleet policy once. This
example uses a disposable issuer; bind the fixture's actual issuer before planning.

```toml
[apps.toko.fleets.local.receiver_policy]
root = "root"
roles = ["user_hub", "user_shard"]
issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai"
role = "user"
network_id = "1111111111111111111111111111111111111111111111111111111111111111"
application_id = "2222222222222222222222222222222222222222222222222222222222222222"
[apps.toko.fleets.local.receiver_policy.limits]
default_session_secs = 60
max_session_secs = 1800
max_proof_ttl_ns = 60000000000
max_admitted_logins = 10_000
```

`roles` selects app canister roles. The singular `role` selects the credential
role they accept. Receivers inherit the IC Auth starting profile. Set
`limits.max_sessions` for physical sessions and `limits.max_admitted_logins`
for retained (issuer, account, device slot) rows. The supplier defaults are 20,000 sessions and 65,536 device slots. Expiry and logout retain the device-generation fence. A full
login table refuses a new slot even when the receiver has spare session capacity.
One issuer with 10,000 accounts using one device slot each needs 10,000 rows.
Eight used slots per account would need 80,000. The installed test qualified
10,000 single-slot accounts with at most 32 active sessions. See the
[0.51 checkpoint measurement](../qualification/history.md#ic-auth-051-stable-receivers--2026-11-26)
before choosing a larger limit.

A stable upgrade reopens the policy saved at installation. Editing fleet TOML only changes
future installs. Enlarging an existing stable receiver requires a migration
that preserves every generation fence and revocation. IC Auth currently
requires exact checkpoint configuration equality. Keep the original receiver
and policy until such a migration is qualified.

Use the lower-level handler mounts for custom subject admission.

Each receiver still needs an individual Root grant. Existing explicit declarations
work. A selected receiver's grant can omit root, issuer, role, network_id and
application_id; explicit values must agree with the policy. Each grant keeps
its own receiver identity, verification key, scopes, controller and time window.

Named Store-backed creations and shard-pool approvals bind the encoded receiver
policy and the original application arguments to the approved install. An
independent hub installer can obtain those bytes from
`FleetConfig::receiver_install_args` with the resolved Root principal. The
whole-fleet bootstrap cannot yet bind an unresolved future Root into an app's
install envelope. Bootstrap infrastructure first for that arrangement.

Put the callable mount under `src/endpoints/` and the lifecycle mount in
`src/lifecycle.rs`. The app supplies a storage-opening function. Fresh install passes
`Some(&Install)`; upgrade with empty arguments passes `None`. A failed reopen traps in that hook.

```rust
// src/endpoints/mod.rs
canic::mount_auth_session!(read = "read");

// src/lifecycle.rs
canic::mount_auth_lifecycle!(storage = open);
```

The bootstrap receives the approved install envelope on fresh installation. An app that needs only its
own bytes reads `input.application`. A hub can validate the app nonce
alongside the approved Root, issuer and audience before the mount initializes
the receiver. With a topology-enabled shard pool, decode the inner
`canic::topology::Install`, call `check_root(input.root)` and use its application
bytes as the nonce. Attach the parent using the existing bounded principal cell.
On upgrade the hook accepts empty arguments and reopens the app and Auth stores.

Legacy revision `377fbce` checked the observed browser against the actual IC
caller before applying local scope policy in
`crates/canic-auth/src/api/application_authorization`. The rewrite retains that
check at entry and on resumption. IC Auth owns the current session decision.

Scoped handlers take only their Wire arguments. The endpoint attribute supplies a
handler-local `auth` binding for the admitted browser, account and declared scope.
Synchronous handlers run inside IC Auth's authorization callback. Each async
invocation owns its binding and checks it again before a subsequent privileged
effect. Provisioning checks it across placement, delivery and allocation lookup.

```rust
#[canic::query(scope = "read", contract = signup::GetMe)]
fn get_me() -> Option<MyProfile> {
    profiles::find(auth.issuer(), auth.subject())
}
```

A scoped query or update returning `Result<T, E>` can select a synchronous
`scope_guard` with signature `fn(&Authorization) -> Result<(), E>`. Canic
passes the handler’s admitted authorization to it before entering the body.
A missing session or scope still rejects the call. An application refusal
returns the guard’s error through the ordinary Wire response.

```rust
fn project_owner(auth: &canic::auth::session::Authorization) -> Result<(), ProjectError> {
    ops::require_project_owner(auth.subject()).map(|_| ())
}

#[canic::query(scope = "read", scope_guard = project_owner, reply_limits = PROJECT_CREATE)]
fn get_beneficiary(id: Ulid) -> Result<BeneficiaryView, ProjectError> {
    ops::beneficiary::get(id).map(Into::into)
}
```

The same guard can protect a `scope = "write"` update. A separate `guard = …`
still runs before decoding. After an await, explicitly recheck the session and
any application ownership that can change before issuing the next effect.

For Toko, change `get_my_route(session: SessionView)` to `get_my_route()`
and use `Identity::from_authenticated_subject(auth.issuer(), auth.subject())?`.
Change `register(session: SessionView, name: String)` to `register(name: String)`
and call `provisioning::place::<Users>(&auth, canic::shard_pool!("user_shard"), retained_shard)`. Account validation,
row updates and post-await application ownership checks still belong to Toko.
For other async workflows, pass the local binding by reference and call
`auth.reauthorize()?` before another privileged effect, mapping the refusal
into the application's error type.

An internal coordinator can obtain the same binding with the fallible
`canic::auth::session::authorize("write")`. It takes the browser principal
and time from the current IC execution and returns `SessionReadRefusal` on
denial. Keep the binding across calls and recheck before each protected effect.
For example, Toko's project coordinator can use:

```rust
let auth = canic::auth::session::authorize("write").map_err(|_| CreationError::Forbidden)?;
let owner = Identity::from_authenticated_subject(auth.issuer(), auth.subject())
    .map_err(|_| CreationError::Forbidden)?;
host.associate(user, project).await?;
auth.reauthorize().map_err(|_| CreationError::Forbidden)?;
key.mark_ready(project)?;
```

For native contract generation, link each inspected actor into the CI executable
(for example, `use toko_user_hub as _;`) and enable `canic/inspection` in that
executable. Serialize `canic::inspection::methods("toko_user_hub", &schemas)?`.
This replaces parsing Rust signatures and mount macro bodies. The map uses the
effective IC export name and includes the mode, Wire ID, request and reply.
Each direction carries its Rust root and the Wire registry’s reachable definitions.
JSON represents each 64-bit ID as a hexadecimal string. The normal Wasm build omits the collector.

Start with `canic::inspection::Schemas::default()`. For a Serde-only root,
register its exact type and checked layout, for example
`schemas.describe_as::<Option<ProjectFilter>, Option<ProjectFilterLayout>>()`.
Use a named Wire layout for recursive types and keep Toko’s application byte
fixtures to demonstrate that the projected layout matches its Serde codec.
An unavailable projection identifies the export and failing direction.

Existing clients still send unit for the read and a string
for registration. The portable session establishment and read replies still use
`SessionView`.

Controller refresh pulls the individual receiver's grant. The standard DB hub
and shard reopen both application data and the Auth checkpoint. The installed
User Directory qualification checks that private reads stay closed until fresh
enrollment and recover the original account's row after refresh.

## Toko signup and private shard reads

Toko's local signup witness uses the hub as its Auth receiver. A browser with
an expired hub session can renew at the installed issuer, look up that exact
renewal and present it to the hub. The hub recovers the original Toko user ID
and shard for the verified issuer and subject. A route identifies the destination;
the shard must make its own admission decision for any private read.

From the Toko checkout, with the documented Testkit server prepared:

```sh
make test-signup
```

### One Root for the hub and managed shards

Declare one `[[apps.toko.fleets.local.auth]]` table per **bound receiver slot**.
Each table contains all the grant fields listed above. Use the same `root` slot
and reviewed Root Wasm for both. For example, the hub table selects
`receiver = "user_hub"` and `receiver_controller = "operator"`. The shard
table selects its concrete slot, such as `receiver = "user_shard"`, and
`receiver_controller = "root"` (the default). Each shard instance needs its
own principal and approval. Declaring a component or allocating a placement key
does not enroll future instances. Existing single-table declarations still work.

With the local fleet bound, retain the two plans separately:

```sh
canic fleet auth plan --app toko --fleet local --receiver user_hub --controller "$OPERATOR"
canic fleet auth plan --app toko --fleet local --receiver user_shard --controller "$OPERATOR"
canic fleet apply "$HUB_PLAN" --app toko --fleet local --identity "$IDENTITY"
canic fleet apply "$SHARD_PLAN" --app toko --fleet local --identity "$IDENTITY"
```

To retire the shard, use its original configuration operation. Apply the returned
revocation plan and explicitly refresh the shard to observe the change:

```sh
canic fleet auth plan-revocation "$SHARD_PLAN" --app toko --fleet local
canic fleet apply "$SHARD_REVOCATION_PLAN" --app toko --fleet local --identity "$IDENTITY"
```

Mount `canic::mount_auth_refresh_endpoint!(crate::auth::pull_enrollment);` on
each receiver with `auth-enrollment` enabled. The async handler returns
`Result<(), enrollment::Error>` and uses the application's existing receiver
and its `begin → entropy → prepare → fetch → finish` sequence. The mount checks
the current IC caller against the canister's controllers before invoking the
handler. The installer has no residual refresh permission after control transfers.

| Update signed by the operator | Destination | Wire request |
| --- | --- | --- |
| `refresh_auth` | Operator-controlled hub | `enrollment::REFRESH` with unit |
| `refresh_auth_receiver` | Shard's Root | `enrollment::TRIGGER_REFRESH` with the shard principal |

Both receivers mount `canic::mount_auth_refresh_endpoint!`. A direct caller sends
`enrollment::encode_refresh_request()?` to `enrollment::REFRESH_METHOD` and
decodes the returned bytes with `enrollment::decode_refresh_response`. A valid
request from an unrelated caller returns the typed `Error::Binding`. A malformed
frame traps during admission. The receiver contract uses the same method ID and
limits for either controller topology.

Root accepts the second update only from a controller and only for a registered
receiver. It sends a single cycle-free `refresh_auth` update to that receiver.
The shard sees Root as its controller and calls back to its configured Root for
its own snapshot. The hub never has to proxy this call. Decode each result as
`Result<(), enrollment::Error>` with its response method ID. A transport
timeout leaves the refresh outcome unknown. Readiness and grant receipts do
not authorize automatic retries.

For example, a Rust operator client can encode the shard trigger using the
existing Wire codec. Submit these bytes in a signed update to Root:

```rust
use canic::auth::enrollment;
let bytes = ic_wire::encode(
    &shard_principal,
    ic_wire::Encoding::Frame {
        method: enrollment::TRIGGER_REFRESH,
        direction: ic_wire::Direction::Request,
    },
    enrollment::LIMITS,
)?;
```

The historical single-grant Root lacked this trigger. Use a fresh installation
for the current Auth contract. Root records first grant use durably and keeps
detailed grant history in memory. Once it has issued a grant, losing that
history closes enrollment, including attempts to replay an earlier approval.

For a direct profile read, Toko's existing `user_shard::get_me` already looks
up the verified issuer and subject. Keep its input free of account selectors.
In the installed journey, check each boundary separately: a new shard starts
closed, an admitted user reads their own record on their assigned shard, the
other shard returns no record for that user, and an unenrolled caller cannot
read either. Exercise revocation both before and after the receiver refreshes;
the last accepted snapshot has a bounded freshness window. On upgrade, the
application reopens its database and the receiver reopens its own Auth checkpoint.
Serving sessions also requires an authenticated enrollment refresh.

Legacy review at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` covered
`crates/canic-core/src/domain/policy/pure/auth/root_provisioning/{mod.rs,tests/mod.rs}`
and its history at `e8605c44ffaf77ceb7ddaf85b374f42b7a48703d`. That policy
checked the protected fleet audience, allowed role scopes and issuer deadlines.
Those obligations still apply. The rewrite additionally binds each snapshot
to the actual receiver principal and an individual refresh challenge.

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
A separate same-receiver case uses the bounded Testkit clock operation described
above and releases the expired continuation before or after the newer revoked
snapshot. This is controlled callback ordering, not arbitrary network scheduling.

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
canic artifact build --package canic-root --output target/auth/root.wasm
canic artifact inspect target/auth/receiver.wasm
```

This uses PocketIC and synthetic cycles. It opens no browser and deploys nothing
to mainnet. The qualification-only canister is a separate application artifact,
not another shipped adapter package or default Canic dependency.

A Root that never issued a grant reopens its empty grant history after upgrade.
After its first grant, an upgrade closes enrollment, worker reset, released-child
retirement and standby imports because the detailed grant history was volatile.

## Inputs and limits

Fresh stateful Auth installations use stable leases exclusively. Existing 0.50 owners require their original executables. A receiver upgrade seals the checkpoint and the next executable reopens it before an authenticated Root refresh.

Both lockfiles select registry engine `ic-auth` 0.51.1, `ic-auth-wire` 0.50.0 and
`ic-auth-protocol-types` 0.44.2. The private browser client is packed from the
independently qualified supplier revision `8c7abf8243cd5fc9c6bf72bd2dd248087b805e01`. Session
admission and read replies carry the verified optional `ext` field (up to
4096 bytes). In a scoped endpoint, `auth.user_id()` returns these verified bytes
as `Option<&[u8]>`. Pre-registration sessions may have no binding. Toko can
interpret the bytes as a ULID and still route by `auth.subject()`. The basic `auth` feature selects
only passive contracts; `auth-enrollment` selects `receiver-stable` and
`ed25519-verification`. Only the qualification application's issuer additionally
selects `issuer-stable` and Platform's local signing API. No sibling edits, source copies or supplier publication
are required. The exact tested artifact is recorded in
[qualification history](../qualification/history.md).

Application signing and the native test verifier now select registry Platform
0.35.1's `ed25519` capability as `ic-platform-crypto`. Enrollment and the qualification issuer both call `canic::entropy::fresh(300)`.
The helper uses Platform’s `raw_rand` codec with a six-byte request budget and
a 42-byte reply budget.
Application canisters can enable Canic's `raw-rand` feature and call
`canic::entropy::fresh(10).await` for 32 bytes with a ten-second bounded wait.
A failure returns to the caller after that attempt. The application owns ID
generation and rechecks authorization after the await. Enrollment separately
rechecks its challenge and deadline. [Canic #34](https://github.com/dragginzgame/canic/issues/34)

Product and qualification select Protocol 0.3.5 with Testkit 0.109.0.
The witness and its real Root use Runtime 0.23.17, and committed
Auth source uses the matching Types 0.21.2 boundary. Browser logout and transport
shipped in 0.219.0; [Canic #16](https://github.com/dragginzgame/canic/issues/16) and
[Canic #17](https://github.com/dragginzgame/canic/issues/17) are complete.
Production custody and the full application journey remain separate.

The witness has a controller-approved Root grant and a public test signer,
two session slots and bounded replay/verification state. It
proves caller mismatch and unauthorized configuration refusal, malformed frames,
ordinary cryptographic refusal charges, exact retries, current authority reads,
logout/replay, and actual reply-encoder failure rolling back Auth and application
state. It also covers a real enrollment callback trap, retained busy admission,
freshness-only refresh preserving the original session, in-flight revocation,
and a deliberately skipped checkpoint leaving the interruption witness closed. Native contract cases cover forged
bindings, stale/expired/conflicting replies and terminal revocation.

Canic's installed Ed25519 qualification covers the issuer lifecycle in
[#10](https://github.com/dragginzgame/canic/issues/10) and the profile browser
journey in [#11](https://github.com/dragginzgame/canic/issues/11). Two accounts
can recover independently after choosing the same issuance operation ID. The
receiver also returns the original receipt when a renderer loses the admission
reply and reopens its journal. See the [runnable qualification](../../qualification/README.md)
for enrollment, recovery, device replacement, logout and upgrade coverage.

Production key custody and certified proofs require separate qualification.

## Stateful Auth storage

Auth is optional. A fleet selecting `receiver_policy` always gets a stable
receiver. Remove the former `storage` field from the table. Install fresh
roles built for IC Auth 0.51. Older installation frames and checkpoints are
refused. Existing 0.50 installations require their original executables.

The receiver seals the supplier checkpoint and its own configuration and
challenge sequence. Reopen requires the successor canister version, matching
configuration and a fresh authenticated Root pull. Refresh a hub through its
controller or a Root-controlled shard through `refresh_auth_receiver`.
Applications using `canic/auth-enrollment` mount
`mount_auth_lifecycle!(storage = open)`. Their bootstrap supplies
`session::stable::{ENGINE_KEY, HOST_KEY}` alongside application leases.
The `storage` macro argument is the bootstrap function. A change to retained
policy, including the login ceiling, is refused on reopen.

Issuers select `canic/auth-issuer-stable`, pass a dedicated lease to
`auth::issuer::StableIssuer`, and seal their signing policy with the engine.
Root retains its own grant and revocation obligations. The installed witness
uses local Ed25519 signing and checks unresolved operations across restart.

Managed Root upgrades with live or revoked grants remain a roadmap item. The
release gate is an installed Root transition that preserves all authority entries,
original request bindings, snapshot revisions and terminal revocations.

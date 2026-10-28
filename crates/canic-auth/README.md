# Canic Auth adapter scaffold

**Unimplemented and unpublished.** `canic_auth::mount!()` deliberately fails
compilation. Depending on this crate does not install endpoints, allocate memory,
register timers or enable application authorization. No mounting configuration,
method roster, Wire IDs or DTOs are invented by this scaffold.

This is the reserved Canic adapter boundary for portable issuer/receiver
endpoints. IC Auth owns credential verification, canonical contracts and
session/login/replay semantics. Canic owns protected Fleet enrollment, actual
caller binding, endpoint admission and atomic durable composition. Existing Root
proof and role-attestation endpoints stay with their current framework owner.

## Integration still required

The canonical acceptance owner is [Canic #491](https://github.com/dragginzgame/canic/issues/491).
The [authentication architecture](../../docs/architecture/authentication.md) owns
the current flow and activation restrictions. This table maps those obligations
to the future adapter; it is not a separate implementation tracker.

| Boundary | Required work before mounting is available |
| --- | --- |
| Root enrollment | Connect protected issuer/key/grant registration, receiver-bound delivery, monotonic revisions/generations, revocation and key retirement. Credential claims cannot supply trusted authority. |
| Issuer | Bind authenticated account, browser key, grants and original login deadline. Compose issuance and lookup-only reconciliation with retained original custody and explicit key-custody policy. |
| Receiver endpoints | Consume upstream bounded Wire contracts for admission, local session access and logout; bind the actual browser caller and protected enrollment. Use shared Canic endpoint macros. |
| Durable storage | Connect the shared engine to atomic session/replay/login commits, retained clocks/floors, budgets and protected metadata. Preserve original deadlines and logout tombstones. |
| Lifecycle and memory | Declare explicit checked memory requirements and synchronous restoration through the application's sole lifecycle owner. Restore closed until fresh authority is established. |
| Browser delivery | Supply and qualify the browser adapter and generated client against the same contracts; retain unknown logout outcomes and durable login invalidation. |
| Acceptance | Prove Root/issuer/receiver installation, valid and denied admission, changed authority, failed commits, lost replies, logout, bounded maintenance and same-release recovery. Production clock/custody requirements remain governed by the architecture. |

The common adapter entry point is `mount!`, contributing endpoints only. Lifecycle
and memory composition remain explicit. Implement production integration and its
owning tests before replacing the compile-time refusal or enabling publication.
Do not add placeholder successful handlers, permissive guards, copied Auth DTOs,
a second session engine or an automatic issuer registration path.

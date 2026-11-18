# Optional Canic Auth browser adapter

This private module belongs to the existing Canic package. It implements IC Auth's
`IssuerTransport` and `LogoutTransport` using the actual Canic issuer contracts.
It adds no Rust package and is not installed by normal `make canic` or imported
by applications that do not select it. Publication remains disabled.

Prepare the committed supplier and npm dependencies explicitly, then validate:

```sh
make auth-browser-prepare
make auth-browser-test
```

Preparation reads `../ic-auth` only through `git archive` at
`a05b5ddd4099a3f64b95deb173a0c16746d98ac5` (Auth 0.47.1). Dirty supplier source,
including qualification work, is excluded. `prepare-auth-browser.sh PATH` accepts
another checkout containing that commit. The private npm package is not assumed
published: its deterministic tarball lives in `target/auth-browser/`, with its
integrity bound by this module's lockfile. Platform npm 0.2.0 owns SDK access.

`httpIssuerAdapter(binding, { host, rootKey })` creates authenticated update
transports through Auth, with explicit gateway trust, the temporary identity and
AbortSignal supplied by its coordinator. `issuerAdapter` accepts an explicitly
owned authenticated raw transport for other hosts and focused tests. Neither
creates an identity, stores keys, opens a journal or retries effects. Set
`binding.browserKey` to `browserIdentity.getPublicKey()` from the selected
Ed25519 device. The adapter checks the original intent's raw key against this
public key and derives the browser principal using the SDK. A restored intent
for another device fails at construction, before transport. On reopening, use
the original intent, issuer, account and clock offset with the retained device.

Pass `adapter.transport` to Auth's `SessionCoordinator` issuer configuration and
`adapter.revoke` to `cleanupOnce`/`cleanupUntilFinished`. Auth owns readiness,
guarded application calls, cross-tab claims, local invalidation, cleanup attempts
and key disposal. Application permissions still belong to the application.

The original account signs issuance and sequence-zero lookup. The retained browser
signs renewal, renewal lookup and logout. Each call binds the original operation,
login and grants. Missing/unresolved evidence throws without issue/renew fallback.
Canonical credential claims and proof bytes remain unchanged. Renewal and lookup
never calculate a new lifetime from response arrival.

`deadlineOffsetNs` is an explicit conservative journal-minus-issuer clock mapping
for credential admission. Establish it outside this adapter and persist the same
value for the original login; no default, clock synchronization or offset inference
is supplied. The receiver-session mapping remains a separate required Auth
coordinator input. Logout acknowledgement timestamps stay in the destination's
clock domain; Auth's separately chosen fixed cleanup deadline is unchanged.
Issuer logout converts Auth's unit request into the original Canic login selectors.
Receiver logout uses the unchanged supplier unit request and response.

`auth-codecs` generates `src/generated` from the Rust types and Wire 0.24.3 runtime.
Regenerate intentionally with the command in `make auth-browser-test`, using
`crates/canic/browser/src/generated` as destination. The check regenerates into
`target/` and compares every output. Complete issuer frames retain 8 KiB input /
128 KiB output limits, typed bounds, method/direction, and browser node/depth
ceilings. Rust endpoints retain their separate allocation/node/depth accounting;
Wire explicitly does not translate Rust allocation budgets into a JavaScript heap
limit. HTTP/body transport limits remain with the supplier transport, not this codec.

Seven focused cases cover a mismatched device key, native Rust frame parity, original selectors and signer
roles, unchanged credential bytes/deadlines, unavailable/malformed/oversized or
wrong-direction replies, lost reply without fallback, cleanup mapping, cancellation
and a real Auth HTTP signature with one submission. The HTTP loss case uses an
intercepted fetch; it does not prove certificate success or installed Chromium
orchestration. `make auth-test` qualifies the issuer/receiver Wasm boundary.
The [installed profile journey](../../../docs/demos/auth.md#installed-profilebrowser-composition)
uses this same module, actual Chromium/IndexedDB, two receivers and the shared
profile engine. After explicit preparation, run
`CANIC_AUTH_BROWSER_EXECUTABLE=/absolute/path/to/chromium make auth-profile-test`.
Playwright Core drives that executable headlessly and downloads no browser.
[Canic #11](https://github.com/dragginzgame/canic/issues/11) retains the remaining
acceptance cases. No production wallet, certified issuer proof or durable Auth
state claim is made here.

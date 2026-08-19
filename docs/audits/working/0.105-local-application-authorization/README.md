# 0.105 B1 Local Application Authorization Evidence

Date: 2026-08-19

## Captured Predecessor

- Branch: `main`.
- Released predecessor: annotated tag `v0.104.1`, peeled commit
  `464c186d9d82112d1ea4c7bdb1f47bcd5e5224a5`.
- B1 source basis: that exact product source plus the uncommitted B1-only test,
  fixture and evidence changes listed by `git status --short`.
- Release boundary: reinstall only. No pre-0.105 token, session, Candid or
  stable-state shape is accepted by 0.105.
- Runtime mutation: none. B1 adds only evidence and test-only qualification.

The complete producer and consumer map is in
[consumer-inventory.tsv](consumer-inventory.tsv). The exact predecessor costs
are in [resource-baseline.md](resource-baseline.md).

## B1 Decision

**Accepted 2026-08-19.** The maintainer selected both smallest hard cuts:

1. preparation derives both signed `presenter` and signed `subject` from the
   authenticated caller and removes caller-nominated subject selection; and
2. replay admission is target-local, retains every live consumed proof and
   returns typed capacity denial at the per-subject or global bound.

No compatibility lane, first-use bearer binding, controller bypass, live
tombstone eviction or Fleet-wide quota coordinator is authorized.

## Seven Evidence Gates

| Gate | Disposition | Evidence |
| --- | --- | --- |
| Signed-presenter propagation | **Requires the specified 0.105 hard cut** | Every producer and consumer is inventoried below. The accepted request has no subject field; preparation signs its authenticated caller as both presenter and subject, and presentation requires both to equal the current caller. |
| Canonical scope hard cut and issuance | **Requires the specified 0.105 hard cut** | The exact grammar, owner, configuration spelling and issuer-policy convergence are frozen below. The predecessor accepts a wider unbounded label grammar and publicly self-issues only `session` and `verify`. |
| Proof/session lifetime and capacity | **Requires the specified 0.105 hard cut** | The 60-second proof and 1,800-second session ceilings are compatible. The accepted target-local policy denies fresh replay growth at 256 live proofs per subject or 4,096 globally without evicting live tombstones. |
| One declaration owner | **Confirmed** | `canic-core::model::auth::application_authorization` owns scope, verified authority, active-session and replay value types. Stable records are encoding-only projections; facade and access only re-export or borrow model values. |
| Pure policy boundary | **Confirmed** | Access reads caller/time once. Ops reads protected authority, canonical records and derived indexes. Policy consumes values and performs no IC call, storage access, serialization, logging or mutation. |
| Authority-generation transitions | **Confirmed** | The complete table below assigns every current validity input to future-only, immediate-denial or generation-advancing behavior. Existing root registry/proof epochs are not reused because they have different authority meanings. |
| Browser-neutral native acquisition | **Confirmed** | A real `ic-agent 0.49.2` `Secp256k1Identity` loaded from test PEM prepares through `canic_command`, retrieves through `canic_status`, and presents the exact token to a separate verifier through PocketIC's HTTP gateway. Canic never receives the private key. |

## Presenter Propagation And Blocking Authority Gap

The selected mechanical propagation is complete:

1. `DelegatedTokenPrepareRequest` carries neither `subject` nor `presenter`.
2. `RuntimeAuthWorkflow::prepare_delegated_token` reads the authenticated
   ingress caller once and supplies it as `prepared_by`.
3. preparation derives `claims.presenter = claims.subject = prepared_by`;
   replay payload, nonce and retained preparation identity bind that caller.
4. canonical claims encoding places `presenter` before `subject`; the claims
   hash and issuer canister signature therefore bind both.
5. retrieval remains keyed by `(claims_hash, prepared_by)` and requires the
   same ingress caller.
6. verification returns presenter and subject in the single
   `VerifiedApplicationAuthority`; positive-cache identity includes the signed
   claims hash and actual caller, and a cache hit still checks
   `presenter == caller`.
7. the proof-bearing guard and session establishment both require signed
   presenter equality before using the subject.
8. active-session and replay records bind caller, subject, proof fingerprint
   and local authority generation.
9. pre-0.105 claims, direct `subject == caller` guard semantics and the
   subject-only session are deleted together.

This resolves the authority gap without inventing delegation. Cryptographic
co-signing, first presentation, replay state and controller status are not
treated as substitutes for caller authentication.

## Frozen Scope And Configuration Contract

The predecessor scope grammar is any non-empty ASCII string made from
`[a-z0-9_:-]`. It has no byte ceiling, permits leading/trailing separators and
empty colon segments, and admits up to 32 scopes in each of 16 role grants.
Public preparation self-issues only the broad `session` and `verify` constants;
root issuer policy and renewal templates hold the wider configured grants.

0.105 hard-cuts that to the design's 1-64 byte, non-empty colon-segment grammar,
with 16 retained scopes and 1,024 aggregate bytes per session. The one model
parser and `canic::application_scope!` use identical rules. Exact duplicates
reject and canonical signed material remains sorted and unique.

The exact protected role configuration is:

```toml
[component_specs.<spec>.auth.local_application_authorization]
allowed_scopes = ["my_app:sql_read"]
default_session_ttl_secs = 900
maximum_session_ttl_secs = 1800
```

Child roles use the same table below
`component_specs.<spec>.children.<role>.auth`. Presence enables the capability;
there is no second boolean. The same role must set
`delegated_token_verifier = true`. Infrastructure roles reject the table.
Host validation sorts and validates `allowed_scopes` once and rejects empty,
duplicate, malformed or over-limit input. Root issuer policy remains the grant
authority: an application scope must be both declared for the target role and
allowed by the issuer policy/template. The public preparation path admits
that intersection and no free-form or parallel application-scope API.

## Frozen Type, Facade And Public Surface Owners

- Model declaration module:
  `crates/canic-core/src/model/auth/application_authorization/mod.rs`.
- Stable encoding projection: the existing auth cell at memory ID 34 and key
  `canic.core.auth.state.v1`; 0.105 replaces the old record members in place.
- Ops conversion/state owner: `ops::auth` and `ops::storage::auth`.
- Pure decisions: `domain::policy::pure::auth::application_authorization`.
- Public synchronous facade:
  `canic::access::auth::authorize_local_application`; its request, decision,
  denial, subject and borrowed scope types are re-exported from the same
  module and are not redeclared by the facade.
- Public role variants retain the design names
  `ApplicationSession::{Establish, Clear}` and
  `CanisterStatusRequest::ApplicationSession`; no new method identity is
  introduced.

The old test-canister-specific session methods are consumers, not protocol
owners. They are removed when the old `AuthApi::{set,clear}_delegated_session`
and subject lookup are hard-cut; they do not become aliases for the role
variants.

## Frozen Authority-Generation Table

0.105 introduces one local durable `application_authority_generation`. It is
not the root issuer-registry epoch and not the root proof epoch. Activation
derives one protected authority snapshot containing verifier enablement, Fleet,
role, accepted issuers, allowed application scopes, TTL policy and the current
generation before ingress is admitted.

| Locally activated change | Disposition |
| --- | --- |
| Verifier disabled | Immediate denial; no old session can authorize while disabled. Re-enable advances generation before ingress. |
| Fleet binding changes | Advance generation. |
| Role binding changes | Advance generation. |
| Accepted issuer removed or replaced | Advance generation. Adding an issuer affects future establishment only. |
| Allowed application scope removed | Advance generation, even for sessions retaining unrelated scopes. Adding a scope affects future establishment only. |
| Default session TTL changes | Future establishment only. |
| Maximum session TTL reduced | Advance generation. Increasing it affects future establishment only. |
| Subject becomes inadmissible | Immediate denial. Any protected topology/policy transition that can make it admissible again advances generation first, preventing revival. |
| Capability disabled | Immediate denial. Re-enable advances generation before ingress. |
| Authority snapshot unavailable or invalid | Immediate denial. |

No other predecessor input changes whether a retained scoped session is valid.
Per-resource entitlement remains application-owned and is deliberately absent
from this generation.

## Capacity Conclusion

The proof/session clocks are separable: establishment can reject a token whose
complete signed lifetime exceeds 60 seconds while a successfully committed
session uses a protected positive TTL no greater than 1,800 seconds. Current
active-session limits already equal the proposed 2,048 global and 128 per
subject ceilings.

Issuer-local retained preparation limits remain 64 per caller and 512 globally
for a 60-second retrieval window, but they are not misrepresented as a
Fleet-wide guarantee. Each target admits at most 256 live consumed proofs per
subject and 4,096 globally. Fresh growth beyond either bound returns a typed
capacity denial and preserves existing authority. Live tombstone eviction is
forbidden.

## Native-Agent Result

Focused command:

```text
CANIC_POCKET_IC_SERVER_URL=http://127.0.0.1:38721/ cargo test --locked -p canic-tests --test native_agent_delegation -- --test-threads=1 --nocapture
```

Result: `1 passed; 0 failed` in 23.5 seconds after the Fleet fixture became
ready. The proof journey used one PEM-backed native principal for preparation,
retrieval and presentation, one issuer and one distinct verifier on the same
live PocketIC runtime. The fixture found and corrected a stale test-only
PocketIC `key_1` public-key pin in both delegation stubs. It also starts the
HTTP gateway before proof provisioning so all proof timestamps share live
PocketIC time.

This proves browser-neutral acquisition through the current verifier. It does
not claim the unimplemented 0.105 session-establishment variant.

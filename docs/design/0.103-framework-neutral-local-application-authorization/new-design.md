Canic 0.103 Design: Framework-Neutral Local Application Authorization

Date: 2026-08-13

Status

Status: B1 evidence work approved; B2 implementation held. No runtime implementation is authorized.

Release boundary: reinstall only. A 0.103 installation starts from fresh current-schema state and does not decode, import, migrate or adopt pre-0.103 delegated-session state.

Sequence: this line follows the 0.102 compact diagnostic-code design and precedes the renumbered 0.104 transport groundwork.

Protocol posture: pre-1.0 hard cut. Removed records, APIs, methods and documentation disappear; there is no alias, fallback, legacy decoder, dual write or dual authorization path.

Dependency posture: this is a Canic-only design. Canic does not depend on IcyDB or any other application framework. No external framework release, repository, type, lifecycle macro or test suite gates 0.103. A consuming application may independently depend on Canic and another framework and may adapt their public contracts in application-owned code.

Authority posture: local application authorization is separate from Canister controllership. Neither controllers nor Fleet infrastructure receive an implicit application-access bypass.

Decision

Canic will replace its subject-only delegated session with one bounded, caller-bound, scope-bearing local application session derived from a fully verified delegated token.

The session lets an unchanged application endpoint make a synchronous local decision:

observed transport caller + required application scope
    -> Allow(authenticated application subject) | Deny(reason)

The access adapter reads msg_caller and current IC time exactly once, loads one bounded protected snapshot and delegates to pure policy. The policy performs no IC API read, await, inter-Canister call, state mutation, cleanup, logging, payload parsing or application-data access. This is suitable for an application-owned guard in front of an endpoint whose existing ABI cannot carry a Canic token on every request.

This is not a reader registry or a second grant system. The delegated-token verifier remains the sole authority for issuer trust, Fleet audience, target role, authenticated subject, granted scopes and proof expiry. The retained session is a short-lived, strictly narrower materialization of that verified authority.

Problem

Canic 0.101.53 already has two relevant capabilities:

a proof-bearing Canic endpoint can verify a delegated token locally against the current Fleet, role, subject, scopes and time; and

a bounded delegated session can map a transport caller to an authenticated subject.

They do not currently compose:

the subject-only session does not retain verified issuer, Fleet, role, scope, authority-generation or strict-expiry evidence; and

the maintained scoped guard obtains its delegated token from Candid argument zero on every call.

The proof-bearing lane is correct for Canic-owned endpoints. It cannot authorize an unchanged endpoint whose ABI is owned by an application or another library. Adding a Canic token argument would change that endpoint's contract and couple its owner to Canic.

Using controllership instead is wrong in both directions:

it gives a reader install, upgrade, stop and delete authority; and

a managed Component may be controlled only by Fleet infrastructure, leaving no human application caller able to use a controller-gated read surface.

0.103 closes the reusable Canic gap without adding framework-specific policy.

Goals

0.103 must:

separate application authorization from controller and infrastructure authority;

reuse the existing delegated-token verifier and existing Fleet/role/scope policy;

replace the weaker subject-only session rather than add a parallel session meaning;

bind retained authority to the exact transport caller, authenticated subject, issuer, Fleet, role, scope subset, authority generation and strict expiry;

expose one synchronous, local, bounded and side-effect-free authorization decision;

preserve an unchanged application endpoint ABI;

fail closed when configuration, session state or current protected authority is absent, expired or inconsistent;

make establishment, replacement, clear, expiry and policy staleness finite and inspectable;

allow separate application scopes for surfaces with different sensitivity;

make active local authority auditable through Canic's protected operator-inspection path;

preserve same-release upgrade reconstruction and bounded cleanup; and

prove instruction, stable-memory and raw-Wasm costs before promotion.

Non-Goals

0.103 does not:

add an IcyDB, SQL, schema, database, reader or framework-specific type, field, endpoint, metric or diagnostic;

modify or gate any downstream repository;

add a principal whitelist, reader registry, entitlement database or second grant store;

make any application endpoint public or anonymous;

make controllers implicit application readers;

pass a token, scope or Canic authorization blob through a foreign endpoint ABI;

define row-, field-, tenant-, project-, object- or statement-level policy;

authorize application mutation merely because a scope check succeeds;

call a parent, Fleet Subnet Root, Fleet Coordinator, issuer, management Canister or remote policy service during authorization;

add a generic policy language, dynamic predicate registry, callback registry or remote-decision cache;

promise instantaneous Fleet-wide revocation without locally activated evidence;

preserve pre-0.103 session state or methods;

solve lifecycle-macro composition with an unrelated runtime; or

replace the broader declarative authentication-profile work assigned to 0.106.

Current Canic Evidence And Required B1 Reconciliation

The provisional baseline is Canic 0.101.53. B1 must reproduce it against the exact implementation source before any runtime mutation.

Concern

Current owner and behavior

0.103 gap

Token verification

AuthOps::verify_token verifies proof, Fleet audience, role, local scopes and expiry without a network call

Verified authority exists only for the immediate proof-bearing request

Scoped endpoint guard

auth::authenticated(scope) reads a token from ingress argument zero

An unchanged application endpoint has no Canic proof argument

Resolved identity

ResolvedAuthenticatedIdentity separates the transport caller from the authenticated subject

It does not retain the complete authority needed by a later request

Delegated session

One bounded caller-to-subject record retains issue/expiry data and a bootstrap fingerprint

It is not scope-bearing authorization

Lookup

Active lookup may scan the bounded session vector and prune expired records

An application guard needs a read-only bounded exact-caller lookup

Bounds

Current global and per-subject limits exist

Scope count, scope bytes, record bytes, restore cost and maximum lookup cost are not frozen

Lifecycle

Current Canic lifecycle restores delegated identity state

Current scoped-session reconstruction and index ownership are not defined

Inspection

A caller can inspect its delegated subject

Operators cannot distinguish configuration, current authority policy and active local sessions

B1 must identify the exact current owners and remove any stale or duplicate path uncovered by the inventory. It may tighten the ceilings in this design. Raising a ceiling or changing a security invariant requires a design amendment and maintainer approval.

Mandatory B1 Decisions

B1 is not merely a measurement pass. It must produce explicit evidence and a maintainer decision for each of these blockers before B2:

Caller/subject binding: trace the current wallet transport caller, token subject, delegated subject, certificate/proof and bootstrap fingerprint from issuance through verification and session commit. Prove the presenter-binding rule described below or stop promotion.

Scope hard cut and issuance: inventory every scope producer and consumer, including configuration, preparation, certificates, tokens, macros, verifier projection, tests and public facades. Prove how a role explicitly admits application-defined scopes.

Proof lifetime and replay capacity: prove that application-session establishment accepts only the short proof lifetime below and that maximum proof issuance/renewal cannot exhaust live tombstone capacity.

Type ownership: identify one domain declaration owner for canonical scopes, verified authority, active sessions and replay records; eliminate facade/access duplicates.

Context/policy purity: prove that caller/time/protected-state acquisition is confined to the access/ops boundary and that policy is a pure value-to-value decision.

The B1 report records each as Confirmed, Hard-cut change required, or Promotion blocked. An unresolved item cannot be deferred into mutating B2 work.

Canonical Ownership

Owner

Responsibility

Delegated-token authority

proof verification, issuer trust, caller/subject binding, Fleet audience, target role, granted scopes and token/certificate time bounds

Canic domain/model

canonical scope and verified-authority value types, bounded session/replay records, exact invariants and stable encoding

Canic ops

conversion from verified runtime material, exact record/index lookup, protected snapshot acquisition and state operations

Canic policy

pure narrowing, caller/subject binding, expiry, replay, capacity, stale-authority and scope decisions

Canic access facade

one caller/time acquisition, request assembly, synchronous policy delegation and public re-exports

Consuming application

scope naming within the admitted grammar, endpoint-to-scope mapping, framework adapter and resource-level policy

Endpoint/framework owner

endpoint generation, guard call order, public denial mapping, business safety and response bounds

Client

proof acquisition, explicit session establishment, renewal with fresh proof, clear and caller continuity

Canic host/operator surfaces

configuration validation, policy inspection, active-session audit, capacity, expiry and diagnostics

Canic authenticates and scope-authorizes an application subject. It does not decide what a database statement means, what resource that subject owns or what public error another framework returns.

Caller, Subject And Presenter Binding

The transport caller and authenticated application subject are deliberately distinct identities:

transport_caller is the principal sending the establishment request and every later guarded request;

authenticated_subject is the delegated application subject and equals the verified token claim sub; and

raw controller, topology, parent, root, Fleet and infrastructure checks continue to use transport_caller, never authenticated_subject.

0.103 does not define transport_caller == authenticated_subject. A wallet or other admitted presenter may act for a different application subject only when the one current proof protocol cryptographically or certificationally binds that presenter, subject and proof together.

An unbound bearer proof is not accepted merely because it is presented first. First successful presentation atomically consumes the already-verified proof identity and records the exact (proof_fingerprint, transport_caller, authenticated_subject) binding; it does not manufacture missing delegation authority. Concurrent presentation of the same proof by another caller loses the atomic replay decision and fails.

If B1 finds that the current bootstrap path intentionally implements “first presenter wins” bearer semantics rather than verified presenter delegation, promotion stops. B2 may proceed only after the design explicitly hard-cuts to a presenter-bound proof using the one current token/proof format. It must not add a parallel bearer or relay lane.

Explicit Enablement And Surface Discovery

Local application authorization is opt-in for each role.

The role must explicitly enable both:

the existing delegated_token_verifier = true capability; and

the new local-application-authorization capability.

The exact configuration location and spelling are frozen in B1 using the repository's current configuration conventions. The semantic rules are fixed:

enabling local application authorization without delegated-token verification is a host/configuration error;

omission means disabled, never allow;

disabled roles do not export the three session Candid methods;

enabled roles export exactly the current methods defined below;

the Rust authorization facade returns Disabled if called in a role whose protected runtime capability is unavailable;

configuration cannot name a framework, method, table or application resource; and

runtime environment variables or mutable unprotected state cannot enable the capability.

Method presence in the current Candid distinguishes “surface not installed” from a denial returned by an installed surface. Within an installed surface, compact diagnostic codes distinguish malformed proof, invalid authority, resource exhaustion and other establishment failures. The Rust-only local decision distinguishes its closed denial reasons without exporting Canic detail through another framework automatically.

Public Canic Contract

Candid Session Operations

An enabled role exports exactly these operations:

type ApplicationScope = text;

type ApplicationSessionRequest = record {
  delegated_token : DelegatedToken;
  requested_scopes : vec ApplicationScope;
  requested_ttl_secs : opt nat64;
};

type ApplicationSessionView = record {
  authenticated_subject : principal;
  issuer : principal;
  scopes : vec ApplicationScope;
  established_at_ns : nat64;
  expires_at_ns : nat64;
  authority_generation : nat64;
};

type InactiveApplicationSession = variant {
  Missing;
  Expired : record { expired_at_ns : nat64 };
  StaleFleet;
  StaleRole;
  StaleGeneration : record {
    session_generation : nat64;
    current_generation : nat64;
  };
  InadmissibleSubject;
};

type ApplicationSessionStatus = variant {
  Active : ApplicationSessionView;
  Inactive : InactiveApplicationSession;
};

canic_establish_application_session :
  (ApplicationSessionRequest) ->
  (variant { Ok : ApplicationSessionView; Err : Error });

canic_clear_application_session :
  () -> (variant { Ok : null; Err : Error });

canic_application_session_status :
  () -> (variant { Ok : ApplicationSessionStatus; Err : Error }) query;

DelegatedToken and Error are the one current Canic types. Under 0.102, Error is the compact numeric record. The generated Rust DTO names follow these Candid names unless B1 proves an existing canonical naming rule that requires a mechanical adjustment before B2. No downstream design owns these names or fields.

The request deliberately contains no caller, subject, issuer, Fleet, role, generation or expiry timestamp supplied as authority. The IC supplies the transport caller. The verifier and protected local state derive every authoritative value.

requested_scopes and requested_ttl_secs are narrowing proposals only:

requested_scopes must be non-empty and a subset of the token's verified local scopes;

unsorted requested scopes are accepted and sorted once at the boundary;

exact duplicate requested scopes are rejected rather than silently deduplicated;

no empty list means “all scopes”;

an ungranted requested scope rejects the complete establishment request;

requested_ttl_secs must be nonzero when present and cannot widen any verified or configured time bound; and

invalid requests leave any existing session unchanged.

Rust-Only Authorization Facade

The public facade exposes one synchronous local decision equivalent to:

pub struct LocalApplicationAuthorizationRequest<'a> {
    pub observed_transport_caller: Principal,
    pub required_scope: ApplicationScopeRef<'a>,
}

pub enum LocalApplicationAuthorizationDecision {
    Allow(AuthorizedApplicationSubject),
    Deny(LocalApplicationAuthorizationDenial),
}

pub struct AuthorizedApplicationSubject {
    pub subject: Principal,
    pub expires_at_ns: u64,
}

pub enum LocalApplicationAuthorizationDenial {
    Disabled,
    CallerMismatch,
    Anonymous,
    MissingSession,
    Expired,
    StaleAuthority,
    InadmissibleSubject,
    MissingScope,
    AuthorityUnavailable,
}

B1 freezes the exact module path and derives. The semantics and denial partition above are normative.

The facade/access boundary:

reads the actual IC transport caller and current time exactly once;

compares it with observed_transport_caller before session lookup;

loads one bounded protected-authority snapshot and one canonical session record through ops;

passes caller, time, required scope, protected snapshot and record to pure policy;

accepts one already-validated borrowed ApplicationScopeRef, never an unchecked &str;

returns only the authenticated subject and strict session expiry on success;

contains no Candid or Serde obligation unless a current Canic facade convention independently requires one;

has no success variant that means controller, infrastructure caller or anonymous; and

does not expose issuer, Fleet internals, proof fingerprints or diagnostic prose to the guarded endpoint.

The explicit caller comparison prevents an adapter from authorizing one principal while the surrounding framework serves another. Policy never calls msg_caller, time, stable storage or another IC API itself.

Canonical Scope Contract

0.103 does not describe its proposed grammar as the predecessor contract. The 0.101.53 source currently admits a different character set, has no equivalent 64-byte rule and permits 32 scopes in one role grant. The following is an intentional 0.103 hard cut, subject to B1 inventory and maintainer acceptance before B2.

One domain/model module declares the owned ApplicationScope, its borrowed validated form and every bound. The tuple/string field is private. ops, policy, access, DTO and facade code import or re-export those types; none redeclares them.

The 0.103 canonical scope grammar is:

1 to 64 ASCII bytes;

colon-separated non-empty namespace segments;

each segment starts with [a-z0-9] and thereafter contains only [a-z0-9_-];

no ., whitespace, uppercase, Unicode, empty segment or trailing separator;

exact byte equality with no case folding or Unicode normalization;

at most 32 scopes in a verified role grant, preserving the current grant ceiling unless B1 approves a lower hard cut;

at most 16 requested and retained scopes in one local application session;

at most 1,024 aggregate scope bytes in one session; and

sorted canonical storage with binary-search membership.

Ordering has no authority meaning. Boundary sequences may arrive unsorted and are sorted once. Exact duplicates are rejected before signing or persistence; they are not silently deduplicated. Signed/certified canonical material must already commit to the sorted unique representation required by its current owner.

Application code constructs a static validated scope through one facade re-export of a domain-owned compile-time helper, conceptually:

pub const SQL_READ: ApplicationScopeRef<'static> =
    canic::application_scope!("my_app:sql_read");

The helper validates the same grammar at compile time and creates no runtime registry. Dynamic host/configuration input uses the domain type's one checked parser. There is no public unchecked constructor and no first-call lazy initialization in a query guard.

Application scope issuance reuses the current delegated grant owner:

role configuration explicitly declares which canonical application scopes that role may receive;

public proof/token preparation may request only scopes declared for the target role and admitted by current issuer policy;

certificate/token construction sorts and commits the exact granted scope set;

local verification reconstructs that same set; and

session establishment may retain only the caller-requested subset.

This is an extension of the existing role-grant authority, not a second allowlist. A free-form client string does not create an issuable scope. B1 must inventory and hard-cut the current login-only preparation path so application-specific scopes can be admitted explicitly without aliases or a parallel preparation API.

Application code owns scope meaning. Names are application-namespaced and capability-specific. Separate surfaces with materially different sensitivity use separate scopes. Broad historical scopes such as read survive only if B1 proves they are intentional current authority; otherwise they are removed by the hard cut rather than aliased.

One Verified Authority Projection

Proof-bearing and session-bearing lanes remain two evidence transports, not two grant authorities.

The domain/model owner declares one internal VerifiedApplicationAuthority value. Ops converts the current verifier's successful runtime material into it; access and policy never reconstruct it independently. It contains only the verified values needed by common policy:

transport caller;

authenticated subject;

issuer;

current Fleet binding;

current role binding;

canonical granted local scopes;

authority generation;

verified issue/not-before/expiry bounds; and

the existing proof identity needed by replay policy.

A Canic-owned proof-bearing endpoint consumes that projection immediately. Session establishment narrows it to the caller's requested scope subset and time bound before persistence. The session-bearing lane can never fabricate, union, extend or delegate authority. Facade code may re-export public domain types but does not own their declaration.

0.103 removes duplicate caller/subject binding, scope matching, expiry and denial classification exposed by this convergence. The old subject-only session does not remain as a fallback identity lane.

Canonical Session And Replay State

The authoritative active record is conceptually:

pub struct LocalApplicationSessionRecord {
    transport_caller: Principal,
    authenticated_subject: Principal,
    issuer: Principal,
    fleet: FleetKey,
    role: CanisterRole,
    scopes: Vec<ApplicationScope>,
    authority_generation: u64,
    established_at_ns: u64,
    expires_at_ns: u64,
    proof_fingerprint: [u8; 32],
}

The exact private encoding is frozen after B1. A field may be replaced by a smaller canonical binding only if all authorization and audit invariants remain mechanically provable. The record does not retain:

encoded proof, certificate or signature bytes;

arbitrary proof extensions;

certificate hashes that are not consulted by current policy;

framework, endpoint, method or resource identity;

application rows, object identifiers or entitlement state;

diagnostic prose; or

authority for another Fleet or role.

Replay state is authoritative only for proof-consumption semantics. It is not a grant. A bounded replay record contains the proof fingerprint, bound caller/subject, authority generation and strict removal time needed to prevent reuse or resurrection. Every successfully consumed proof remains recorded through its own verified proof expiry, even after its session expires, is replaced, is cleared or its authority generation becomes stale.

Application-session establishment accepts only a proof whose complete verified lifetime is at most 60 seconds. “Complete verified lifetime” means expires_at - issued_at under the one current proof contract, not merely the proof's remaining lifetime when presented. If that lifetime cannot be proven from signed/certified material, establishment denies. The ordinary delegated-token ceiling may remain longer for proof-bearing endpoints, but such a long-lived proof is ineligible for this tokenless-session bridge. Tombstones therefore remain live for no more than 60 seconds from proof issue time, making their capacity a bounded burst limit rather than a 24-hour renewal accumulator.

Heap lookup indexes and subject-count indexes are derived state. They are rebuilt from canonical records, never serialized as independent authority and never consulted without the corresponding canonical record.

Establishment, Replacement, Replay And Clear

Session establishment follows this exact order:

read the transport caller from IC ingress;

reject anonymous;

require the protected local-application-authorization and delegated-verifier capabilities;

validate and bound the requested scope set and TTL before expensive work;

verify the complete token through the one current verifier and require its complete verified lifetime to be at most 60 seconds;

require token.claims.sub to equal the authenticated application subject and require the current proof protocol's verified presenter/delegation binding to admit this transport caller;

require every requested scope to be in the verified local scope set;

derive the current Fleet, role and authority generation from protected local state;

calculate strict effective expiry;

evaluate replay, replacement and capacity policy without mutating state;

atomically commit the active session and required replay record; and

return the canonical current view.

Effective expiry is exclusive and equals the earliest of:

verified token expiry;

verified certificate expiry when the maintained proof format has one;

established_at + requested_ttl when requested;

established_at + configured_default_ttl when omitted; and

established_at + MAX_LOCAL_APPLICATION_SESSION_TTL.

Arithmetic is checked. An already-expired or zero-duration result is rejected. The configured default may be lower than the hard ceiling but never higher.

The hard maximum accepted application-session proof lifetime and the hard maximum local application-session lifetime are both 60 seconds. They are Canic security bounds, independent of any consumer. B1 may lower either value but cannot raise one without a design amendment.

Replay and replacement semantics are:

exact replay of the same proof by the same caller with the same requested scopes and TTL is idempotent only while the exact active session exists; it returns that session without extending expiry;

the same proof with different scopes, TTL, caller or subject is rejected and cannot replace authority;

presentation of the same proof by a different caller is a replay conflict even when no active session remains;

the same proof can never extend a session or recreate a cleared session;

a different valid proof for the same caller atomically replaces the old session with exactly the newly requested verified subset; scopes are replaced, never unioned;

replacement retains the old proof's replay tombstone until that proof expires, so a caller cannot roll back to earlier or broader authority;

invalid proof, invalid narrowing request or capacity rejection leaves the old session unchanged;

replacement that causes no net global growth is evaluated against the post-replacement per-subject counts; and

interrupted exact retry observes either the old complete state or the new complete state, never a partial record.

canic_clear_application_session is caller-scoped and idempotent. It removes only the calling principal's active session; a missing active session returns success. It retains every consumed proof fingerprint as a bounded tombstone until that proof's strict expiry, so clear cannot be reversed with the same proof. A fresh valid proof may establish a new session.

Authority-generation change invalidates active sessions but does not erase, re-key or make reusable a live proof fingerprint. A fresh proof under the current generation is required.

canic_application_session_status is read-only. It returns Active only when the caller's record currently passes caller, time, Fleet, role, generation and subject-admissibility checks. A physically retained invalid record maps exactly as follows:

missing or already cleaned -> Missing;

strict time failure -> Expired;

Fleet mismatch -> StaleFleet;

role mismatch -> StaleRole;

generation mismatch -> StaleGeneration;

subject no longer admissible -> InadmissibleSubject; and

protected-authority read failure -> compact Canic error.

It never prunes or repairs state.

Cleanup removes expired active records and replay tombstones in bounded work. Authorization itself never performs cleanup.

Local Authorization Algorithm

Authorization has a value-acquisition boundary followed by pure policy.

The access adapter:

reads actual msg_caller and current IC time exactly once;

receives the application/framework-observed caller and validated required scope;

loads one bounded protected-authority snapshot through ops; and

loads at most one canonical session record through the exact-caller index.

It then calls pure policy with those values. Policy applies this fixed denial precedence:

actual caller is anonymous -> Anonymous;

actual caller differs from observed caller -> CallerMismatch;

local application authorization is disabled -> Disabled;

protected authority is unavailable -> AuthorityUnavailable;

no exact canonical record exists -> MissingSession;

current time is not strictly before expiry -> Expired;

Fleet, role or authority generation differs -> StaleAuthority;

authenticated subject is no longer admissible -> InadmissibleSubject;

required scope is absent -> MissingScope; and

otherwise -> Allow.

The access adapter may read msg_caller, time and bounded Canic-owned state. Neither access nor policy performs an inter-Canister call or await. Policy performs no IC API read. No authorization step mutates, prunes, logs, schedules work, reads application state, parses endpoint payload or observes another framework's readiness.

The authorization hot path is O(log G + log S) or better, where G is the bounded active-session count and S is the bounded sorted scope count in the selected session. It does not scale with application rows, schemas, indexes, receipts, logs or endpoint argument length.

Controllers, Infrastructure And Application Subjects

Controller and topology authority remain separate from application identity:

a controller does not pass local application authorization unless it independently establishes a valid scoped session;

a Fleet Coordinator, Fleet Subnet Root, Component parent or managed child relationship does not imply an application session;

topology and infrastructure guards continue to consume the raw transport caller, never the authenticated application subject returned here; and

application resource ownership continues to consume application-owned state after Canic authentication.

0.103 does not infer “human” or “Canister” from principal text. It reuses the current typed subject-admissibility and infrastructure-authority rules. B1 must remove any string or principal-shape heuristic. If current Canic cannot prove the distinction for a given path, that path fails closed rather than guessing.

Expiry, Policy Change And Revocation

The revocation contract is local and bounded:

caller clear is immediate on that Canister and the consumed proof cannot resurrect the session;

strict expiry is no later than 60 seconds after establishment and never later than any proof or configured bound;

a locally activated authority-generation change invalidates every session from an earlier generation immediately on that Canister;

verifier disablement or unavailable protected authority denies immediately;

issuer or grant-policy removal prevents new establishment as soon as the new policy is locally active; and

without a locally activated generation change, a previously established session remains usable only until its already-fixed strict expiry.

Authority generation is one durable monotonic Canic-owned value or an existing equivalent current owner. Every locally activated change to verifier trust, accepted issuer policy, Fleet binding, role binding or other session-validity input must change that binding before ingress is admitted. B1 must identify the existing owner; B2 must consolidate rather than introduce a second generation counter.

Documentation must not call this instant Fleet-wide revocation. The operator surface reports the exact local generation and maximum remaining staleness. Per-resource entitlement requiring immediate revocation stays in application-owned state and is checked after Canic returns Allow.

Stable State, Lifecycle And Recovery

0.103 has one current persisted format. Pre-0.103 bytes are unsupported because the release boundary is reinstall only.

For the current format:

active sessions and replay tombstones have one canonical encoding;

derived indexes are not independent persisted authority;

restore validates bounds, uniqueness, canonical scopes, caller keys, generation and time arithmetic;

corrupt or over-limit authority state fails closed and cannot be partially admitted;

the exact-caller and subject-count indexes are rebuilt synchronously before ingress or deferred application work;

expired records may remain physically present after restore but are unusable immediately and are removed only by bounded cleanup;

the cleanup timer/operation is registered through the existing Canic timer owner;

same-release upgrade preserves current-format active sessions and tombstones;

backup and restore use the same canonical snapshot model; and

interrupted establishment is recovered as an atomic old-or-new outcome.

canic::start! remains the Canic lifecycle owner. 0.103 restores only Canic-owned authority. Composition with another runtime's lifecycle exports is not an authorization concern and is neither required nor claimed by this line.

Release-set activation must not expose a mixture of pre-0.103 and 0.103 Canic session semantics within one maintained Fleet release. Existing release-set identity and activation authority enforce the reinstall boundary; no versioned session protocol or dual decoder is added.

Bounds And Resource Contract

These are promotion ceilings, not capacity targets:

Quantity

Maximum

Active sessions per Canister

2,048

Active sessions per authenticated subject

128

Retained replay fingerprints per Canister

4,096

Retained replay fingerprints per subject

256

Canonical scopes per session

16

One canonical scope

64 bytes

Aggregate scope bytes per session

1,024 bytes

One encoded active-session record

2,048 bytes

Canonical stable active-session and replay state at maximum admission

8 MiB

Reconstructed heap indexes at maximum admission

4 MiB

Expired entries removed by one cleanup invocation

128

Accepted application-session proof lifetime

60 seconds

Local application-session lifetime

60 seconds

Local authorization at maximum admitted state

1,000,000 IC instructions

Raw non-gzipped Wasm growth over the accepted predecessor

256 KiB

B1 records the current values and may propose lower limits. B2 freezes the final constants. Admission validates encoded and aggregate sizes before commit. Fresh growth over capacity returns a typed resource-exhausted diagnostic and leaves existing state unchanged. Replacing an existing caller's session does not require a spare global slot but must satisfy every post-replacement bound.

No live replay tombstone is evicted to admit renewal. Because an eligible proof lives for at most 60 seconds, B1 must prove that the maximum admitted proof-issuance burst in any rolling 60-second window is no greater than 256 per subject and 4,096 per Canister, or lower those upstream issuance limits. Normal renewal cannot accumulate tombstones across a 24-hour token lifetime because such a token is ineligible for session establishment.

Measurements distinguish:

Symbol

Measurement

E

establishment instructions, including cold and warm proof verification

A

allowed local authorization at 1, median and maximum admitted G and S

D

denial instructions for each closed denial reason

B

stable bytes per active session and replay record

H

reconstructed heap-index bytes at maximum admitted state

R

synchronous restore/index reconstruction at maximum state

C

one bounded cleanup invocation at its maximum removal count

M

raw non-gzipped Wasm delta for representative configured roles

Raw canonical release Wasm is authoritative. Builder-produced gzip is secondary context.

Diagnostics, Metrics And Auditability

Public Canic endpoint failures use the 0.102 compact diagnostic registry. Establishment must allocate distinct current codes for at least:

capability disabled or unavailable;

malformed or non-canonical requested scope;

requested scope not granted;

invalid, expired, wrong-audience, wrong-role or caller-mismatched proof, subject to the existing security projection policy;

replay conflict;

invalid TTL; and

capacity exhaustion.

Sensitive verifier detail retains its internal numeric diagnostic and maps to a safe public code. No arbitrary verifier, token or application prose crosses the public boundary.

The Rust-only denial enum is not automatically serialized by a consuming endpoint. An application adapter normally collapses every denial to its own surface-specific denial. Canic never dictates that foreign diagnostic.

Bounded metrics cover:

establishment started, completed and rejected;

create, replacement and idempotent replay;

clear, strict-expiry observation and cleanup;

replay conflict and capacity rejection; and

authority-generation invalidation.

The pure authorization function does not mutate metrics. A Canic-owned endpoint wrapper or consuming application may record aggregate allow/deny counts outside the decision. Metrics and logs never label token bytes, proof fingerprints, SQL/payload content, transport callers, authenticated subjects, scopes or resource identifiers.

Audit answers three different questions without conflating them:

What can be established? Protected operator inspection reports enabled state, accepted issuer policy, current Fleet, role, authority generation, scope grammar, limits and maximum lifetime.

What is active now? A bounded paginated operator view, protected by Canic's existing Fleet/operator inspection authority rather than raw controllership alone, reports each active caller, authenticated subject, issuer, canonical scopes, establishment time, strict expiry and generation. It omits proof fingerprints and proof bytes.

What resources may the subject use? Canic does not answer this; the consuming application owns it.

The caller self-status method reports only that caller's current session view. Anonymous receives no session information. Operator inspection distinguishes declared configuration, protected runtime binding and observed current sessions, and marks expired-but-not-yet-cleaned records as inactive.

Framework-Neutral Adapter Contract

A consuming application may adapt Canic's decision to any synchronous two-way guard:

fn may_use_surface(context: ApplicationGuardContext) -> ApplicationGuardDecision {
    match canic::access::application::authorize(
        LocalApplicationAuthorizationRequest {
            observed_transport_caller: context.caller,
            required_scope: MY_APPLICATION_SCOPE,
        },
    ) {
        LocalApplicationAuthorizationDecision::Allow(_) => {
            ApplicationGuardDecision::Allow
        }
        LocalApplicationAuthorizationDecision::Deny(_) => {
            ApplicationGuardDecision::Deny
        }
    }
}

The names outside canic::access::application are placeholders owned by the consuming application. Canic production code, configuration, state, Candid, metrics and diagnostics contain no foreign-framework identity.

The adapter must be synchronous, bounded and fail closed. It must not convert a denial into controller fallback, perform a remote lookup or expose Canic's internal denial detail through the guarded surface. The consuming application owns any later subject-to-resource decision.

Non-Normative Motivating Integration

An independently developed database framework may choose to expose an application-supplied guard for generated SQL and schema reads. An application that independently uses that framework and Canic could:

obtain a Canic delegated proof;

establish a short-lived session on the target Canister with explicit SQL-read and/or schema-read scope subsets;

call the framework's unchanged query or schema method as the same transport principal; and

map Canic Allow/Deny to the framework's own guard decision.

This example is explanatory only:

Canic does not import, name or release-gate on that framework;

that framework does not need Canic to implement or test an application guard;

Canic qualification uses a generic local consumer fixture, not the framework repository;

the framework can qualify its guard with a trivial application predicate;

combined end-to-end evidence belongs to the consuming application; and

neither framework's release is a prerequisite for the other's promotion.

Layer Ownership

The maintained dependency direction remains:

endpoints -> workflow -> policy
                     +-> ops -> model

Layer

0.103 responsibility

dto

passive establishment, self-status and operator-inspection boundary data

model/domain

sole declarations of canonical scope, borrowed scope, verified-authority, active-session and replay value types; bounds and snapshot invariants

ops

conversion from verified runtime material, exact lookup, protected-config reads and storage operations

policy

pure scope narrowing, expiry, replay, binding, capacity and replacement decisions

workflow

verify-establish, clear, cleanup, inspection and exact-retry orchestration

access

one msg_caller/time acquisition, bounded context assembly and synchronous delegation to pure policy

endpoints/macros

capability-gated export, marshalling and immediate workflow/access delegation

facade

framework-neutral re-exports of maintained domain types and public request/decision surfaces; no duplicate declarations

host/CLI

configuration validation, declared/observed inspection and rich diagnostic catalogue

Policy reads no stable state, calls no IC API, records no metric and depends on no DTO. Ops does not declare domain values. Access does not reconstruct verifier output. Workflow does not hand-construct or mutate canonical records outside model/ops owners. Restore is synchronous Canic lifecycle work, not a user callback.

Security Invariants

Application authorization never implies controller, install, stop, delete, upgrade, topology or infrastructure authority.

Controllers and infrastructure callers receive no implicit application-access bypass.

Omitted, disabled, missing, expired, stale, corrupt or unavailable authority fails closed.

Session establishment uses the one current delegated-token verifier.

The token subject is the authenticated application subject; a different transport caller is admitted only through verified presenter/delegation binding, never first-use bearer inference.

Requested scopes and TTL may only narrow verified authority.

An empty requested scope set never means all scopes.

The actual IC caller must equal the adapter-observed caller and the session's exact bound caller.

An establishment proof and its resulting session each have a maximum complete lifetime of 60 seconds; the session never outlives its proof, certificate or configured/requested bound.

Exact replay never extends expiry, changes scopes or recreates a cleared session.

Replacement is atomic and replaces scopes; it never unions them.

Access reads caller, time and bounded Canic state once; pure policy performs no IC API read. Neither performs an inter-Canister call, await, mutation, prune, log, timer, payload parse or application-state read.

Fleet, role or authority-generation mismatch invalidates the session.

Topology and infrastructure policy always consumes raw transport identity, never the returned application subject.

Authentication and scope authorization do not imply application resource ownership.

Proof/signature bytes, arbitrary extensions and application entitlement data are not persisted.

Every record, index, scope collection, page and cleanup operation is bounded.

Derived indexes cannot become independent authority.

No public or operator surface exposes proof fingerprints.

No controller fallback, principal whitelist, compatibility decoder, legacy alias, shim or dual session format exists.

Canic production owners contain no foreign-framework-specific contract.

Alternatives Rejected

Add A Canic Token To Every Application Endpoint

Rejected. It changes the endpoint ABI, couples its owner and clients to Canic and repeats proof transfer and verification on every request.

Keep Controller Authorization

Rejected. Controller authority is excessive for a reader and may be held only by Fleet infrastructure.

Add A Principal Reader Set

Rejected. It becomes a second grant lifecycle, loses authenticated-subject semantics and duplicates expiry, revocation and audit policy.

Let Another Framework Store Canic Grants

Rejected. It makes that framework interpret Canic tokens and creates competing authority.

Call A Parent Or Issuer During Every Decision

Rejected. It adds latency, availability coupling and cache/revocation ambiguity to the endpoint hot path.

Store Every Verified Token Scope Automatically

Rejected. A valid broad token must not silently become a broad tokenless session. Establishment requires an explicit non-empty requested subset.

Refresh With The Same Proof

Rejected. Reusing a proof to move expiry would defeat the advertised maximum staleness bound. Renewal requires fresh proof.

Retain Subject-Only And Scoped Sessions

Rejected. Two current session meanings duplicate identity resolution and permit accidental authorization through the weaker record.

Add Resource-Level Policy To Canic

Rejected. Canic owns identity and coarse application scopes; the application owns its resources and entitlements.

Make A Particular Framework A Qualification Dependency

Rejected. A generic consumer fixture proves the Canic contract. Cross-framework evidence is useful integration evidence but cannot authorize or block a Canic release.

Complexity Gate

The justified state-space delta is:

one current bounded session meaning replacing the weaker subject-only meaning;

one bounded replay/tombstone owner preserving existing proof-consumption guarantees;

one exact-caller derived lookup;

one synchronous closed decision; and

one protected bounded audit view.

Promotion fails if implementation adds a second verifier, token format, grant store, session representation, authority generation, framework adapter, policy language, dynamic registry, application entitlement table, remote cache, unbounded state or compatibility path.

Every batch reports files/line delta, semantic owners, states added and removed, duplicate paths removed, stable bytes, instructions, restore cost and raw Wasm, plus whether the maintained structure became simpler, stayed neutral or became more complex.

Release-Batch Plan

Patch numbers remain maintainer-owned.

Batch

Bounded outcome

Required evidence

Status

B1

Exact 0.101.53 auth/session/config/lifecycle/diagnostic baseline and frozen 0.103 public contract

mandatory caller/subject, scope issuance, proof-lifetime/capacity, ownership and purity decisions; producer/consumer inventory, Candid, stable/heap encoding, operator authority, E/A/D/B/H/R/C/M baseline and duplicate-flow report; no runtime mutation

Approved to begin; B2 blocked

B2

One canonical scope and verified-authority policy

scope hard cut, non-empty subset narrowing, closed denial enum, pure binding/expiry/replay/replacement policy and proof/session convergence

Blocked on B1

B3

Canonical scoped session and replay state

hard-cut records, exact-caller and subject-count derived indexes, atomic replacement, tombstoned clear, bounded cleanup, snapshot and synchronous restore

Blocked on B2

B4

Explicit enablement and current Candid operations

capability validation, establish/clear/self-status DTOs and methods, TTL clamp, compact diagnostics and generated Candid

Blocked on B3

B5

Framework-neutral synchronous facade

public request/decision types, native proof/session policy convergence, generic unchanged-ABI consumer fixture and duplicate-code removal

Blocked on B4

B6

Operator audit, security and resource gates

protected paginated inspection, authority-generation invalidation, metrics, no-secret evidence and accepted bounds

Blocked on B5

B7

Canic-only qualification and hard-cut closeout

coordinator-controlled generic child, scoped human use, controller denial, expiry/clear, same-release upgrade, residue audit, docs and release evidence

Blocked on B6

B1 may begin under this evidence-only authority. Mutating B2-B7 work remains blocked until B1 reconciles the exact predecessor, resolves every mandatory B1 decision and the maintainer approves the resulting contract and measurements.

Validation Matrix

Boundary

Required evidence

Enablement

omitted capability exports no methods; verifier-only role exports no session methods; invalid capability combination fails host validation; enabled role exports exactly the three methods

Candid

exact current request/view/error shapes; no legacy delegated-session methods or fields; method presence matches declared role capability

Scope

predecessor inventory; 0.103 grammar edges including rejected .; 32-grant/16-session split; unsorted normalization; duplicate rejection; compile-time construction; explicit role declaration and issuance; empty set, subset success, one missing scope rejects all and no “all” shorthand

Proof

token subject equals authenticated subject; distinct presenter succeeds only with verified delegation binding; unbound/first-use bearer denied; cross-caller race has one atomic winner; forged, expired, not-yet-valid, wrong issuer/Fleet/role and unavailable verifier fail through current security projection

Proof lifetime

complete lifetime at 60 seconds accepted; over 60 seconds rejected even when remaining life is short; missing trusted issue time rejected; rolling per-subject/Canister issuance cannot exhaust live tombstones at admitted rates

Establishment

effective-expiry minimum, checked arithmetic, idempotent exact replay, no expiry extension, replay conflict, atomic replacement, no scope union and capacity old-state preservation

Clear

caller-only removal, no cross-caller clear, proof tombstone retained across clear/replacement/generation change, same proof cannot resurrect, fresh current-generation proof may establish

Authorization

exact precedence: anonymous, caller mismatch, disabled, unavailable, missing, expired, stale Fleet/role/generation, inadmissible subject, missing scope, allow

Status

exact Missing, Expired, StaleFleet, StaleRole, StaleGeneration and InadmissibleSubject mapping; unavailable authority is a compact error

Purity

no state write, pruning, metrics, logs, timers, application reads or remote calls on every allow/deny path

Lifecycle

one canonical current encoding, same-release upgrade, synchronous bounded index reconstruction, expired record denial before cleanup, corrupt/over-limit state fail-closed and atomic interrupted retry

Revocation

clear immediate; verifier disablement immediate; local generation change immediate; fresh establishment denied after local policy removal; prior session denied by strict expiry within 60 seconds

Audit

caller self-status isolation; protected paginated active-session inspection; configuration/policy/session distinction; expired entries marked inactive; no proof fingerprint exposure

Infrastructure separation

coordinator-controlled child admits a scoped authenticated application subject while controller/topology authority remains unchanged; controller has no implicit read

Resource

maximum G/S lookup, per-subject counts, proof burst, canonical stable bytes, reconstructed heap-index bytes, maximum restore, cleanup, instruction ceilings and raw-Wasm delta

Dependency

Canic manifests and production source contain no foreign-framework dependency or framework-specific identifier; generic fixture supplies its own local adapter

Residue

subject-only records/methods, old formatters, fallback guards, aliases, dual readers and stale docs are absent

Acceptance Criteria

0.103 is complete when:

the subject-only delegated session record and its APIs are removed without aliases or fallback;

each role must explicitly enable local application authorization in addition to delegated-token verification;

enabled roles export exactly the three current session methods and disabled roles export none;

establishment stores only a non-empty explicit subset of fully verified local scopes;

the token subject is the authenticated application subject and any different transport caller is admitted only by verified presenter/delegation binding;

the active session is bound to exact caller, subject, issuer, Fleet, role, authority generation and strict expiry;

proofs accepted for session establishment have a complete verified lifetime no greater than 60 seconds;

exact replay cannot extend expiry, alter scope or resurrect a cleared session;

replacement is atomic and never unions authority;

one synchronous facade authorizes an unchanged-ABI generic consumer endpoint;

every missing, anonymous, mismatched, expired, stale, unavailable or insufficient-scope path fails closed in the frozen precedence;

controllers and Fleet infrastructure receive no implicit application authority;

access reads caller/time/state once and pure policy performs no IC API read; neither performs an inter-Canister call, await, mutation, cleanup, log, metric, timer, payload parse or application-state read;

proof-bearing and session-bearing paths share one domain-owned verified-authority projection and policy;

active records and replay state have one current canonical encoding and derived indexes cannot become authority;

same-release upgrade reconstructs current state synchronously within the accepted stable/heap bounds;

local clear/generation change and the 60-second proof/session bounds are measured and reported honestly;

caller self-status and protected paginated operator inspection are complete, bounded and omit proof material;

public failures use 0.102 numeric diagnostics and host-owned prose;

source, state, Candid, metrics and diagnostics contain no framework-specific production contract;

Canic qualification and release do not require an IcyDB or other external-framework artifact;

the 0.106 profile line reuses this scope/session authority rather than adding another; and

raw non-gzipped Wasm, canonical stable bytes, derived heap bytes, instructions, Candid and complexity deltas are reviewed and accepted.

Relationship To Other Canic Lines

0.102 owns numeric diagnostic identity and host rendering for new public failures.

0.104 transport groundwork consumes no local application-session authority.

0.105 Coordinator Workers remain infrastructure identities and gain no application authority from this line.

0.106 declarative authentication profiles must compile into the same canonical scopes and delegated-token authority and must not add a second session store.

0.107 standalone blob extraction does not inherit Canic sessions implicitly; an application integration must establish its own authority boundary.

0.108 funding, 0.109 archives, 0.110 language-neutral guests, 0.111 observability and 0.112 Canister estates gain no user authority merely by observing an Allow decision.

Promotion Gate

Mutating implementation may begin only after:

B1 freezes the exact maintained source identity, configuration owner, session/replay encoding, verifier projection, scope representation, lifecycle owner, operator authority and performance baseline;

B1 confirms that the 60-second, cardinality, byte, instruction and raw-Wasm ceilings are implementable or proposes only lower values;

the maintainer approves the hard-cut replacement over changing every application ABI or adding a reader registry;

exact public module/type/method names and compact diagnostic allocations are accepted;

the authority-generation owner and every change that advances it are explicit;

the 0.106 design is reconciled to reuse this single authority; and

the maintainer explicitly authorizes B2-B7.

Until then, this document authorizes Canic B1 inventory and measurement only. It authorizes no runtime, Candid, stable-state, package-version, changelog-version or downstream-repository mutation.
# Idea: Declarative Authentication Profiles

Date: 2026-08-03

## Status

- Classification: deferred, unnumbered idea. Its former `0.108` working number
  is retired; the scheduled local application-authorization line is now 0.105.
- Former review status: proposed for maintainer review.
- Release boundary: reinstall only. A future authentication-profile release
  consumes no prior-release token, certificate, issuer-policy, session or
  verifier state.
- Implementation approval: none.
- Sequence: this idea follows the
  [0.100 Fleet Coordinator design](../../0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md),
  the
  [0.101 Component provisioning design](../../0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md),
  the scheduled
  [0.105 framework-neutral local application authorization design](../../0.105-framework-neutral-local-application-authorization/0.105-design.md),
  the
  [direct transport idea](../cross-subnet-data-transport-groundwork/design.md)
  and the proposed
  [Coordinator Worker idea](../coordinator-workers/design.md).
- Current-runtime posture: Canic already verifies one Fleet-audience token
  containing several role grants. It does not yet compile one declarative
  profile into verifier selection, issuer policy, renewal and client login
  behavior.
- Provider posture: authentication providers and wallet products are outside
  the runtime protocol. Canic accepts an authenticated caller Principal and
  must not add provider-specific type, configuration, method or state names.
- Protocol posture: pre-1.0 hard cut. Accepted contracts replace the current
  caller-supplied grant preparation and manual verifier-policy assembly in
  place. No alias, compatibility decoder, legacy fallback or parallel login
  path is retained.

This design is deliberately complete enough to guide review but does not
freeze the exact configuration spelling, component-subtree binding or
application-authorized admission hook. Those decisions are completion gates
below.

## Summary

This idea proposes one declarative **authentication profile** as the application
developer's sole authority surface for ordinary user authentication.

An application declares once:

- who may obtain the profile;
- which Canister roles accept it;
- which endpoint scopes each role receives;
- the maximum token lifetime; and
- whether the profile is Fleet-wide or bound to one protected runtime
  subtree.

Canic then compiles and installs the cryptographic machinery:

- issuer and verifier role requirements;
- canonical profile and grant digests;
- root issuer policy and renewal templates;
- one multi-role delegated token;
- local verifier projections; and
- client-readable profile metadata.

The expected Toko shape is:

~~~text
direct authenticated caller
  -> request profile "project_user" once
  -> one token with project_instance and project_ledger grants
  -> direct project query
  -> direct ledger query
~~~

The ledger does not require a second token, a second chain-key signature, a
ledger-local registration update or a relay through its parent. It verifies
the same self-contained token locally against its protected Fleet, profile,
role and scope.

Every actual ingress request still requires the normal IC transport
authentication for its caller. That signature is not a Canic role-admission
operation and cannot be removed by a profile.

## Problem

The maintained delegated-token runtime has the required cryptographic
properties but exposes too many independent setup decisions to application
developers.

Today a complete deployment may need to coordinate:

1. global delegated-token verifier configuration;
2. per-role `delegated_token_issuer` and `delegated_token_verifier` flags;
3. controller-owned root issuer policy installation;
4. root renewal-template installation;
5. caller-supplied token audience and grant lists;
6. endpoint `auth::authenticated(...)` scopes;
7. client prepare/get sequencing; and
8. token placement as the first Candid argument.

Those surfaces can drift independently. A role may expose an authenticated
endpoint but not be configured as a verifier. A caller may need to know Canic
role names and scope vocabulary merely to log in. A root policy may omit one
ordinary child role. A custom application read scope cannot use the open
prepare path even when configuration intends every authenticated user to have
that scope.

The result is secure failure, but poor developer ergonomics and unnecessary
operator work.

The parent-child topology does not itself solve the problem. The IC does not
propagate a user's caller identity through a parent relay, and a child does
not inherit an ingress signature merely because its parent created it. The
shared authority must be explicit, signed once and locally verifiable.

## Decision

The authentication-profile design will make profiles the canonical source of login-time
role and scope grants.

The client requests a bounded profile identity and context. It does not
choose its subject, audience, roles or scopes. The issuer derives the subject
from the authenticated caller and expands the profile from protected
configuration.

The token retains explicit expanded audience and role grants so each target
can verify it without an issuer, root, parent or management-Canister call.
The profile identity and canonical digest additionally prove which installed
policy produced those grants.

Manual booleans and controller policy calls become generated implementation
details where the compiler and install plan have enough authority to derive
them. Application code continues to declare endpoint scopes because the
endpoint owns the operation being protected.

## Authentication Is Not Resource Authorization

An authentication profile answers:

~~~text
May this authenticated subject invoke this scoped endpoint on this role?
~~~

It does not answer:

~~~text
Does this subject own project X, ledger account Y or administrative resource Z?
~~~

The receiving application workflow remains responsible for resource
authorization after Canic authenticates the subject. In particular:

- a `project_ledger` role grant does not grant every holding in every ledger;
- a `ledger.read` scope permits entry to the read surface but does not choose
  the account returned;
- caller-supplied owner or project Principals remain untrusted unless the
  endpoint is intentionally public; and
- administrative entitlements remain application policy, not an implication
  of successful login.

This separation allows one ordinary Fleet-wide user profile to work across
many project and ledger Canisters without making the token an ownership
database.

## Provider-Neutral Identity Model

Canic recognizes only protocol identities:

| Term | Meaning |
| --- | --- |
| transport caller | Principal authenticated by the IC for this request |
| authenticated subject | application subject selected by the maintained Canic identity boundary |
| delegated token | signed Canic audience, profile, grant, scope and lifetime authority |
| local application session | 0.105 bounded local mapping from transport caller to verified subject, issuer, role scopes and expiry |
| role attestation | separate Canister-to-Canister service identity proof |

The current `RootComponentMembershipApi` is also outside authentication-profile
authority. It is an online, same-root application authorization lookup over the
active Component Registry: the receiver supplies only its observed transport
caller and the application selects the admitted managed roles. It neither
issues a user identity nor replaces portable role attestation or cross-root
Fleet-service peer authority.

No maintained runtime or configuration type may name a login provider, wallet
product, frontend framework or transport library.

An ordinary direct caller uses its transport Principal as the token subject.
If a valid 0.105 local application session exists, its materialized authority
remains bound to the exact transport caller and verified subject under the
existing caller-lane separation. A proof-bearing request remains bound to the
same resolved subject directly. Topology predicates continue to consume the
raw transport caller.

Different login transports may produce different Principals for the same
human. Canic accepts either Principal independently. Merging their application
accounts is an explicit account-linking protocol and is outside ordinary
profile issuance.

## Proposed Configuration

The illustrative configuration is global because one profile may span roles
from more than one Component Spec. Exact TOML spelling remains subject to
schema review.

~~~toml
[auth.authentication_profiles.project_user]
admission = "authenticated_caller"
reach = "fleet"
max_ttl_secs = 3600

[auth.authentication_profiles.project_user.grants.project_instance]
scopes = ["project.read", "session"]

[auth.authentication_profiles.project_user.grants.project_ledger]
scopes = ["ledger.read", "session"]
~~~

The profile is the authority. Application clients do not repeat the grant
list.

An administrative profile may instead require application authorization:

~~~toml
[auth.authentication_profiles.project_administrator]
admission = "application"
reach = "fleet"
max_ttl_secs = 900

[auth.authentication_profiles.project_administrator.grants.project_instance]
scopes = ["project.admin"]

[auth.authentication_profiles.project_administrator.grants.project_ledger]
scopes = ["ledger.admin"]
~~~

The compiler rejects:

- empty or malformed profile identities;
- empty profiles;
- roles absent from the complete Component Topology;
- duplicate or noncanonical grants and scopes;
- zero or excessive token lifetimes;
- unknown admission or reach modes;
- application scopes without a matching endpoint declaration;
- authenticated endpoints unreachable from every profile; and
- issuer or verifier feature requirements unavailable in the selected build.

## Profile Contracts

The conceptual configuration contracts are:

~~~rust
pub struct AuthenticationProfileSpec {
    pub admission: AuthenticationProfileAdmission,
    pub reach: AuthenticationProfileReach,
    pub max_ttl_secs: u64,
    pub grants: Vec<AuthenticationProfileGrantSpec>,
}

pub enum AuthenticationProfileAdmission {
    AuthenticatedCaller,
    Application,
}

pub enum AuthenticationProfileReach {
    Fleet,
    ComponentSubtree,
}

pub struct AuthenticationProfileGrantSpec {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}
~~~

These names describe design ownership, not frozen public DTOs.

Every compiled profile receives:

- one bounded `AuthProfileId`;
- one canonical `AuthProfileDigest`;
- one normalized audience rule;
- one canonical sorted role-grant set;
- one maximum token TTL; and
- one admission policy.

The digest covers every authority-bearing field. Presentation labels and
documentation do not affect it.

## Admission

### Authenticated Caller

`AuthenticatedCaller` is the ordinary login profile.

The issuer:

1. rejects the anonymous Principal;
2. derives the subject from the current authenticated caller;
3. resolves the requested profile from protected configuration;
4. clamps the requested lifetime to the profile and global ceilings; and
5. emits exactly the profile's configured grants.

This replaces the current rule that callers may submit arbitrary roles while
only `session` and `verify` scopes are self-grantable. A custom scope such as
`ledger.read` is safe for ordinary callers only when the installed profile,
not the caller payload, says every admitted caller receives it.

### Application

`Application` admission is for entitlements such as project administrator,
moderator or billing operator.

The final implementation design must define one typed local authorization
boundary. It must not accept a caller-selected predicate path, raw function
name, arbitrary serialized policy or parent assertion. The decision must bind:

- authenticated subject;
- exact profile;
- exact reach context;
- current application entitlement revision; and
- maximum permitted lifetime.

The issuer expands grants only after that decision succeeds. The authorization
decision is not embedded as unverified extension bytes.

No `Controller` admission mode is proposed for user tokens. Controller and
infrastructure administration already have distinct raw-caller and role-
attestation boundaries and must not be collapsed into login profiles.

## Reach and Runtime Scope

### Fleet Reach

`Fleet` reach uses the maintained `FleetKey` acceptance boundary. It is the
initial Toko requirement and permits one user token to authenticate directly
to project, ledger and other declared application roles across the Fleet.

Role and endpoint scope checks remain exact. Resource access remains local
application policy.

### Component-Subtree Reach

`ComponentSubtree` is the stricter optional mode for applications that require
the token itself to be limited to one protected runtime subtree.

It must not be implemented by trusting a caller-supplied parent Principal or
by treating the top-level `ComponentInstanceId` as sufficient for every
nested application partition. Toko's Project Instance is a descendant of its
Project Hub Component and its Ledger is a further descendant; several Project
Instances may therefore share one top-level Component identity.

Before this mode is implementation-approved, the authentication-profile design must freeze:

1. the canonical subtree-anchor identity;
2. how that identity is allocated and never ambiguously reused;
3. how descendants inherit it through protected init and cascade state;
4. how an issuer proves the caller may request that exact subtree;
5. how each verifier obtains the same binding without a network call; and
6. how removal, retry and backup preserve or terminate the binding.

Fleet reach does not wait for this decision. Component-subtree reach must not
be simulated with a weak payload field while its authority is unresolved.

## Compiled Authentication Manifest

Profile configuration compiles into one canonical
`AuthenticationProfileManifest` under the exact App, Fleet and Component
Topology authority.

The manifest contains only bounded protocol data:

- Fleet identity;
- profile identities and digests;
- admission and reach modes;
- normalized role grants and scopes;
- maximum lifetimes;
- issuer-role requirements;
- verifier-role projections; and
- required cryptographic build capabilities.

It is included in the semantic configuration digest and immutable install
plan before network effects. Root issuer policy, renewal templates and local
verifier projections must all derive from this same manifest. They cannot be
independently edited representations of the profile.

The compiler derives:

- every role appearing in a profile grant is a delegated-token verifier;
- every selected issuer role is a delegated-token issuer;
- the union of grants and TTL ceilings admitted for each issuer;
- each role's exact accepted profile digests and scopes; and
- the root's bounded renewal authority.

The existing manual `delegated_token_issuer` and
`delegated_token_verifier` booleans are hard-cut once equivalent derivation is
complete. Keeping both manual and generated sources would create ambiguous
authority.

## Issuer Selection

Profiles must not force a client to guess an arbitrary Canister Principal.
The final configuration contract must select issuer roles or protected Fleet
services and define how a concrete active issuer is resolved.

An issuer instance is eligible only when:

- its role is admitted by the compiled authentication manifest;
- it is an exact active Component Registry member;
- its local protected configuration matches the manifest;
- the root has installed current proof material for that exact issuer; and
- its published discovery binding is current.

Dynamic issuer instances receive their policy through the same Component
creation and activation authority as their other protected configuration.
They do not require an operator to make an ad hoc controller call after each
creation.

The final design must decide whether issuer discovery uses a 0.101 Fleet
service, an application-owned authenticated router or one exact role-specific
Directory. It must not introduce a generic public list of token-minting
Canisters.

## Token Preparation

The proposed public request surface is conceptually:

~~~rust
pub struct DelegatedTokenPrepareRequest {
    pub metadata: Option<AuthRequestMetadata>,
    pub profile: AuthProfileId,
    pub reach: AuthenticationProfileReachRequest,
    pub ttl_ns: u64,
    pub ext: Option<Vec<u8>>,
}
~~~

The request does not contain:

- `subject`;
- raw audience authority;
- role grants; or
- scopes.

The issuer derives those fields and prepares one claims hash. The existing
issuer canister-signature mechanism still uses an update-then-query shape:

~~~text
caller -> issuer prepare update
caller -> issuer get query
~~~

The authentication-profile design does not claim to remove that initial token-signature operation. Its
guarantee is that adding project, ledger and other profile roles does not add
another prepare, issuer signature, root signature or verifier update.

One token carries the complete bounded profile grant set. Root chain-key
proof renewal remains batched and outside the login hot path.

## Token Claims

The token continues to carry the expanded authority needed for local
verification. Conceptually, claims add profile identity:

~~~rust
pub struct DelegatedTokenClaims {
    pub presenter: Principal,
    pub subject: Principal,
    pub issuer_pid: Principal,
    pub profile: AuthProfileId,
    pub profile_digest: AuthProfileDigest,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    // current time, nonce, proof and bounded extension fields
}
~~~

The exact current fields remain unless this design explicitly replaces them.
There is no `V2`, schema discriminator or compatibility decoder.

The issuer proof signs the complete canonical claims hash once. The root proof
continues to bind the issuer's maximum authority. The profile digest prevents
the same profile name from silently changing meaning at a verifier.

## Verification

An authenticated endpoint verifies locally:

1. token and certificate canonical shape;
2. root and issuer proofs;
3. time, TTL and accepted epoch floors;
4. exact Fleet and optional subtree reach;
5. installed profile identity and digest;
6. token grants as a subset of issuer/profile authority;
7. current Canister role in the profile grant set;
8. endpoint-required scope in that role grant; and
9. token subject against the resolved authenticated subject.

No verification step calls:

- the parent;
- the root;
- the issuer;
- the Fleet Coordinator;
- a login provider;
- a wallet; or
- the management Canister.

Plain query, composite-query where otherwise valid, and update guards use the
same local proof and profile path.

## Endpoint Contracts

Endpoints remain explicit about their required scope:

~~~rust
#[canic::canic_query(requires(auth::authenticated("ledger.read")))]
fn holdings(token: DelegatedToken) -> HoldingsView {
    // Application workflow derives or authorizes the resource for the
    // authenticated subject. It does not trust a payload owner assertion.
}
~~~

The proof-bearing Candid contract continues carrying the token as argument zero
so the proof is visible and independently auditable. Client helpers may inject
that argument but must not silently substitute a session for an endpoint that
declared proof-bearing authorization.

The independent 0.105 session-bearing lane exists for explicitly declared
application or framework endpoints whose maintained ABI cannot carry Canic
proof material. It consumes the same profile scopes and verified-authority
policy. The authentication-profile design must not create another tokenless session record, decision
facade or grant authority.

The role-contract catalog records every authenticated endpoint and scope.
Compilation cross-checks the catalog against profile grants:

- a configured scope must protect at least one current endpoint for that
  role;
- an authenticated endpoint must be reachable from at least one current
  profile unless explicitly marked non-login/internal; and
- one profile cannot grant a scope to a role whose compiled endpoint surface
  lacks it.

## Installation and Renewal

The normal fresh install owns profile materialization.

Before application Canister activation, installation:

1. validates the canonical authentication manifest;
2. stores its digest in the immutable Fleet install authority;
3. projects each root's issuer and verifier requirements;
4. installs root issuer policies from the manifest;
5. installs root renewal templates from the same manifest;
6. supplies each issuer and verifier its protected local projection;
7. obtains and distributes required active proof material; and
8. verifies live profile status before allowing authenticated traffic.

The existing controller-only root issuer-policy endpoints may remain as
internal implementation operations only if installation and same-release
reconciliation own every invocation. They are not an application setup step.
If no runtime operator mutation remains necessary, the public controller
surface is removed.

Renewal may rotate proof material and advance accepted epochs. It cannot
change profile grants, admission, reach or lifetime independently of the
protected authentication manifest.

## Client Experience

Canic should provide a provider-neutral token client around an authenticated
IC calling transport.

The client needs only:

- the transport's current Principal;
- issuer discovery for the requested profile;
- profile identity and optional reach context; and
- target Canister actors.

It owns:

- prepare/get sequencing;
- bounded token caching;
- expiry-aware refresh;
- one-token reuse across all profile roles;
- argument-zero token injection in generated convenience wrappers; and
- clearing cached material when the transport Principal changes.

The client does not own:

- provider-specific authentication UI;
- wallet approval policy;
- application account linking;
- privilege expansion;
- hidden token persistence beyond configured lifetime; or
- resource authorization.

The core protocol has no provider enum. Provider-specific adapters, if useful,
belong in client packages and implement one generic authenticated-call
boundary.

## Inspection and Diagnostics

The proposed passive command is:

~~~text
canic auth inspect --component-spec <spec> --profile <profile>
~~~

Exact CLI placement is not frozen. Its stable structured projection should
show:

- profile identity and digest;
- admission and reach;
- maximum TTL;
- issuer requirement;
- every target role and scope;
- matching authenticated endpoints;
- missing endpoint coverage;
- unused grants or scopes;
- required runtime/build features; and
- whether install provisioning is complete.

Live inspection must distinguish configuration intent from observed issuer,
root-proof and verifier readiness. It never prints bearer tokens, token
fingerprints, private client state or sensitive extension bytes.

Medic should fail when:

- a required verifier lacks the profile projection;
- an issuer lacks current root policy or proof material;
- a live role's profile digest differs from install authority;
- an endpoint scope is absent from every admitted profile;
- a profile references a role absent from protected topology; or
- a token would require unsupported cryptographic features.

## Layer Ownership

The maintained dependency direction remains:

~~~text
endpoints -> workflow -> policy
                     +-> ops -> model
~~~

The workflow branches are independent. Policy never calls ops.

Ownership is:

| Layer | Responsibility |
| --- | --- |
| config/model | canonical profile, manifest and protected runtime records |
| policy | pure profile validation, admission and grant/reach decisions |
| ops | canonical conversion, profile lookup, storage and one-step crypto operations |
| workflow | compile/install/renew/prepare orchestration |
| access | argument-zero decode and local profile/token enforcement |
| endpoints/macros | authenticate and delegate immediately |
| host/CLI | compilation, installation, inspection and rich diagnostics |
| client SDK | transport-neutral token acquisition, cache and wrapper ergonomics |

DTOs remain passive. Workflow does not construct persisted records directly.
Application admission hooks cannot mutate Canic auth state during their pure
decision boundary.

## Security Invariants

The authentication-profile design must preserve:

1. The caller never chooses its token subject.
2. The caller never submits raw roles or scopes for expansion.
3. Every grant originates from one protected profile manifest.
4. Every token grant is a subset of root-approved issuer authority.
5. Every verifier matches exact Fleet, profile digest, local role and scope.
6. Authentication never implies resource ownership or administration.
7. Raw topology predicates never consume a delegated application subject.
8. Direct verification performs no network or threshold-signature call.
9. Adding a role to a profile does not add a per-role signature operation.
10. Removal of a role or scope leaves no compatibility acceptance path.
11. Profile and proof state is bounded by explicit counts, bytes and TTLs.
12. Provider-specific assumptions cannot enter the runtime protocol.
13. Unknown or stale profile identities and digests fail closed.
14. Component-subtree reach cannot ship before exact inherited authority is
    frozen and tested.
15. Application-authorized admission cannot trust a caller-selected policy or
    unverified payload assertion.

## Bounds

The implementation must freeze finite limits for:

- profiles per App/Fleet;
- grants per profile;
- scopes per role grant;
- profile identity bytes;
- scope identity bytes;
- issuer roles and active issuer instances;
- token TTL;
- manifest encoded bytes; and
- client-retained active tokens.

The current delegated-token bounds of 16 role grants and 32 scopes per role
are the starting point. The authentication-profile design must either retain them with measured evidence
or replace them explicitly. It must not make profile configuration an
unbounded stable or Wasm-resident catalog.

## Hard Cuts

Once implemented, the authentication-profile design removes:

- caller-supplied token subjects;
- caller-supplied raw audience and grant lists on the public login surface;
- manual verifier flags derivable from profile grants;
- manual issuer flags derivable from profile issuer selection;
- application/operator responsibility for root issuer-policy assembly;
- application/operator responsibility for renewal-template assembly;
- documentation that teaches one token mint per target role;
- any provider-specific authentication terminology in Canic contracts; and
- any fallback to the old public prepare shape.

No anti-resurrection test preserves an old request form. Current profile
behavior receives positive coverage instead.

## Non-Goals

The authentication-profile design does not:

- implement a login provider or wallet;
- merge Principals belonging to one human;
- make wallet approval optional;
- make raw query responses certified;
- turn an authentication scope into project or asset ownership;
- replace service-to-service role attestations;
- preserve tokens or sessions from earlier releases;
- introduce cross-release migration or mixed-version operation;
- infer administrative profiles from controller status;
- permit caller-defined policy code;
- make every Component descendant an authenticated verifier implicitly; or
- modify a downstream application repository.

## Relationship To Canic 0.105

0.105 owns proof-to-session materialization, exact transport-caller binding,
bounded local scope lookup and the synchronous framework-neutral decision.
The authentication-profile idea owns declarative profile compilation, issuance,
verifier projections and client acquisition.

The authentication-profile design must:

- use the 0.105 canonical application-scope identity;
- let profile-issued tokens establish the same current 0.105 session record;
- bind the session authority generation to the installed profile projection;
- report its maximum token/session revocation latency honestly; and
- retain no parallel subject-only session or profile-specific tokenless guard.

A proof-bearing endpoint and a session-bearing foreign endpoint remain two
explicit evidence modes over one profile/grant authority. Neither mode is a
compatibility fallback for the other.

## Toko Example

Toko can declare one ordinary profile containing its maintained user-facing
roles:

~~~text
profile: project_user
admission: authenticated caller
reach: Fleet
grants:
  project_instance -> project.read, session
  project_ledger   -> ledger.read, session
~~~

One caller obtains one token from its selected active issuer. The token subject
is whichever Principal the caller's current authenticated transport presents.

The caller may then:

1. query the Project Instance directly with that token;
2. discover the exact Ledger through protected application topology;
3. query the Ledger directly with the same token; and
4. receive holdings only after the Ledger authorizes the authenticated subject
   against its own records.

The Project Instance is not a relay. The Ledger does not add the user to local
state merely to accept the profile. A different login transport producing a
different Principal obtains the same profile shape for that different subject.
If Toko wants both Principals to share one account, it adds a separate explicit
dual-control account-linking protocol.

## Implementation Slices

### Slice 1: Canonical Profile Contracts

- Freeze profile identity, admission, reach and digest types.
- Add bounded configuration and canonical manifest compilation.
- Validate roles and scopes against Component Topology and endpoint catalogs.
- Derive issuer/verifier feature requirements.
- Add hard-cut DTO and canonical encoding tests.

### Slice 2: Install-Owned Provisioning

- Bind the authentication manifest into install authority.
- Derive root issuer policies and renewal templates.
- Provision dynamic issuer and verifier instances through Component lifecycle.
- Remove manual application setup and independently editable flags.
- Add interrupted-install and same-release reconciliation evidence.

### Slice 3: Profile Token Issuance and Verification

- Replace caller-supplied subject/audience/grants with profile requests.
- Bind token claims to profile identity and digest.
- Enforce installed local profile projection in the verifier.
- Preserve one update-then-query issuer signature per token, not per role.
- Prove one token works directly across project and ledger roles.

### Slice 4: Client and Inspection

- Add provider-neutral token acquisition and refresh helpers.
- Add token-aware generated convenience wrappers.
- Add passive profile inspection and structured diagnostics.
- Extend Medic with manifest, issuer and verifier readiness.
- Document public reads versus authenticated and certified reads.

### Slice 5: Optional Component-Subtree Reach

- Freeze exact subtree-anchor allocation and inheritance.
- Bind issuer admission and verifier state to that anchor.
- Prove nested Project Instance and Ledger descendants share exactly the
  intended subtree and no sibling subtree.
- Exercise removal, backup, restore and retry.
- Do not expose the mode until every authority decision is complete.

## Required Qualification

Implementation completion requires targeted proof that:

- a direct caller can obtain one ordinary profile token and use it against at
  least two roles;
- adding a Ledger role creates no additional login-time signature or policy
  update;
- a caller cannot alter subject, grants, scopes, profile digest or reach;
- custom self-service scopes come only from installed profile policy;
- application-admitted profiles cannot be self-granted;
- verifier and issuer requirements are derived consistently from one manifest;
- installation and renewal cannot publish divergent policy;
- every supported direct query and update path enforces the same local proof;
- raw caller predicates remain unaffected by application identity resolution;
- stale profile digests and removed grants fail closed;
- bounds reject oversized profiles, grants, scopes and manifests;
- no provider-specific name appears in runtime schema, DTOs, state or methods;
  and
- the Toko project/ledger journey works with two distinct authenticated caller
  Principals without treating them as the same account.

## Open Decisions

Before implementation approval, maintainers must decide:

1. the exact TOML placement and bounded profile identifier;
2. exact issuer selection and discovery authority;
3. the typed application-admission hook;
4. whether Fleet reach alone ships first or Component-subtree reach is part of
   the initial hard cut;
5. the final profile-manifest owner and install-plan projection;
6. whether profile removal invalidates immediately through a digest/epoch or
   only through bounded TTL and proof floors;
7. exact client package and generated-wrapper ownership;
8. exact CLI inspection placement and JSON contract; and
9. retained or revised profile/grant/scope bounds.

## Completion Gate

The authentication-profile idea is ready for implementation only when:

- one profile is the sole source of ordinary login grants;
- issuer selection and application admission have typed, bounded authority;
- install-owned policy provisioning has no manual parallel path;
- Toko's project and ledger use one token without a parent relay;
- authentication remains distinct from project and holdings authorization;
- provider neutrality is explicit in every maintained contract;
- the hard-cut request and configuration removals are enumerated; and
- component-subtree reach is either fully frozen or explicitly absent from the
  first implementation surface; and
- profile-issued session authority reuses the complete 0.105 current surface
  without a second local grant or session store.

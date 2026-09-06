# Idea: Declarative Authentication Profiles

Reviewed: 2026-09-06

## Status

- Deferred and unnumbered; no implementation or release is approved.
- Retained need: compile repeated issuer, verifier and client-login policy
  from one bounded application declaration.
- Owners: config/role qualification, existing authentication runtime and
  public application authorization facade.
- Repository scope: Canic only; application policy remains application-owned.

## Current Boundary

Canic already supports Fleet-audience delegated tokens with multiple role
grants and the released framework-neutral local application-session boundary.
See [authentication architecture](../../../architecture/authentication.md) and
[delegated-signature contracts](../../../contracts/AUTH_DELEGATED_SIGNATURES.md).

Declarative profile compilation is not implemented. A future profile must
reuse current token verification, session materialization, scope identity and
exact transport-caller binding. It must not introduce another session store,
grant authority or tokenless guard.

Fleet ingress admission remains separate from application authentication and
resource authorization. A profile cannot grant Fleet infrastructure authority,
resource ownership or permission merely because a caller is Fleet-admitted.

## Retained Direction

An application could declare one profile containing:

- admission policy: authenticated caller or explicit application authorization;
- exact roles and endpoint scopes;
- finite token lifetime and revocation bounds; and
- Fleet-wide reach, with subtree-bound reach deferred unless needed.

One compiled manifest would own normalized grants, issuer requirements,
verifier projections and their semantic configuration digest. Fleet Ensure
would materialize that exact authority before activation through the existing
Root/runtime workflows. Renewal may rotate proof material but cannot
independently broaden grants, reach or lifetime.

Clients request a profile identity rather than assembling their own grant
list. Issuer discovery must resolve an exact active protected binding, never
an arbitrary Principal or an unauthenticated list of token issuers.

Authentication providers and wallet products remain outside the protocol.
The runtime consumes authenticated transport callers and explicit proof,
subject and issuer authority.

## Important Constraints

- Bind tokens to exact subject, caller, issuer, audience, role, scope and
  expiry. Application workflows still authorize each resource/action.
- Proof-bearing endpoints retain explicit proof material. Session-bearing
  foreign/framework endpoints use the maintained session lane; client helpers
  must not silently switch the endpoint's evidence mode.
- Cross-check profile grants against the compiled role/endpoint catalog,
  including empty grants, unknown roles/scopes and unavailable capabilities.
- Define profile-to-session authority generation, revocation latency and
  stale-projection denial before adding configuration syntax.
- Derive capability requirements from the profile only when complete
  equivalence is proved. Hard-cut superseded manual settings at that boundary,
  rather than retaining two writable policy sources.
- Use the existing command/status families and bounded capability variants.
  A profile does not justify a new endpoint for every operation.
- Keep admission policy pure; endpoint authentication and workflow effects
  remain in their maintained layers.

## Evidence Before Scheduling

1. Demonstrate a real repeated-configuration or login problem in one
   application using the current authentication/session surfaces.
2. Freeze the manifest, exact issuer selection, application-admission decision
   seam, scope catalog and finite limits.
3. Prove one token authorizes the intended roles while wrong caller, subject,
   audience, role, scope, expiry and configuration digest fail closed.
4. Prove existing session materialization and revocation obey the same grants,
   without introducing parallel session policy.
5. Exercise fresh provisioning, renewal, interrupted distribution and
   same-release recovery through existing authority owners.
6. Measure runtime, generated-surface and validation cost before promotion.

## Disposition

Retain the declarative-policy idea, with Fleet-wide reach as the initial
candidate. The former configuration sketches and detailed release plan are
retired until a current consumer and complete batch justify them.

Coordinator Workers and transport extensions are not prerequisites for
profile compilation. Current authentication and session contracts are the
baseline. No cross-release token, session or issuer-state compatibility is
proposed.

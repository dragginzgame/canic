# Canic 0.102 Access Diagnostic Leaves

Date: 2026-08-12

## Status

This provisional B1 ledger expands the security-critical
`AccessError::Denied(String)` bucket at `v0.101.53`. It allocates no numbers.
The source has 28 production denial construction sites after excluding inline
tests; generic dependency and decode helpers serve several sites, so that is
not a code count.

Access predicates run before endpoint handlers. Their public result may mask a
protected internal cause, but the cause must remain numerically observable and
must never be reconstructed from denial prose.

## Existing Semantic Reuse

The access boundary must reuse these already-proposed auth leaves rather than
allocate access-specific duplicates:

- `DelegatedAuthCertExpired` reuses `AUTH_CERT_EXPIRED`;
- `DelegatedAuthTokenExpired` reuses `AUTH_TOKEN_EXPIRED`;
- delegated-token subject/caller mismatch reuses
  `AUTH_SUBJECT_CALLER_MISMATCH`;
- delegated-token disabled reuses `AUTH_DELEGATED_TOKENS_DISABLED`;
- attestation subject/caller mismatch reuses
  `AUTH_ATTESTATION_SUBJECT_MISMATCH`; and
- token or attestation verification preserves the exact internal auth code and
  its already-approved public projection, including `AUTH_PROOF_INVALID`.

`access_error_from_verification` currently selects two expiry cases from broad
public `ErrorCode` values and formats every other `InternalError`. B4 replaces
that bridge with typed diagnostic preservation. Access code must not parse host
class, origin or prose, and an access wrapper must not overwrite a more exact
auth identity.

## Environment And Fleet Gates

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `ACCESS_FLEET_SUBNET_ROOT_REQUIRED` | `access::env::is_fleet_subnet_root` | self | Call the endpoint only on its Fleet Subnet Root |
| `ACCESS_BUILD_NETWORK_MISMATCH` | configured build network differs | self | Use an endpoint built for the required network |
| `ACCESS_BUILD_NETWORK_UNAVAILABLE` | build network is absent | self | Rebuild with the required `ICP_ENVIRONMENT` identity |
| `ACCESS_FLEET_DISABLED` | disabled query or update | self | Enable the Fleet before retrying |
| `ACCESS_FLEET_READONLY` | update while read-only | self | Use a query or restore write mode before retrying |

Disabled query and disabled update share one code: owner, meaning and action
are identical. Read-only remains separate because queries still work and the
operator action differs.

## Caller And Topology Predicates

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `ACCESS_CONTROLLER_REQUIRED` | `is_controller` | self | Use an admitted controller caller |
| `ACCESS_WHITELIST_REQUIRED` | initialized whitelist does not contain caller | self | Add or use a whitelisted caller |
| `ACCESS_DIRECT_CHILD_REQUIRED` | `is_child` | self | Call from an exact direct child |
| `ACCESS_PARENT_REQUIRED` | `is_parent` | self | Call from the configured immediate parent |
| `ACCESS_ROOT_REQUIRED` | `is_root` | self | Call from the exact configured root |
| `ACCESS_SELF_REQUIRED` | `is_same_canister` | self | Use the self-call-only route |

Caller principals never enter the compact diagnostic. Configuration, parent or
root lookup failure is not the same as a negative predicate: it preserves its
exact internal configuration/environment code and projects publicly through
`ACCESS_DEPENDENCY_UNAVAILABLE`.

## Delegated Token And Attestation Input

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `ACCESS_REQUIRED_SCOPE_MISSING` | verified token lacks endpoint scope | self | Obtain a token granting the required scope |
| `ACCESS_DELEGATED_TOKEN_MALFORMED` | bounded first-argument decode fails | self | Submit a canonical bounded delegated token as argument one |
| `ACCESS_ROLE_ATTESTATION_MALFORMED` | bounded first-argument decode fails | self | Submit a canonical bounded role attestation as argument one |

Decoder errors, type names and ingress bytes are discarded. Quota and maximum
type-length checks remain in place and execute before cryptographic work.

Delegated-token TTL overflow is protected configuration failure, not access
denial chosen by the caller. It preserves an exact configuration diagnostic and
projects through `RUNTIME_CONFIGURATION_INVALID`. Token/attestation verifier
unavailability likewise preserves the typed dependency cause and uses
`ACCESS_DEPENDENCY_UNAVAILABLE` only as its safe public projection.

## Deployment, Expression And Root Gates

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `ACCESS_SERVICE_GUARD_INVALID` | endpoint's static Fleet-service guard cannot parse | `ACCESS_CONFIGURATION_INVALID` | Correct the endpoint declaration and reinstall |
| `ACCESS_SERVICE_AUTHORITY_REQUIRED` | active Component lacks exact service Authority purpose | self | Route through the admitted active Authority member |
| `ACCESS_EXPRESSION_RULE_REQUIRED` | an access expression contains no rules | `ACCESS_CONFIGURATION_INVALID` | Declare at least one predicate and reinstall |
| `ACCESS_NEGATED_PREDICATE_MATCHED` | a negated predicate succeeds | self | Satisfy the configured inverse policy |
| `ACCESS_ACTIVE_COMPONENT_REQUIRED` | caller is not an active Registry member | self | Call from an exact active Component member |
| `ACCESS_ROOT_OR_ACTIVE_COMPONENT_REQUIRED` | caller is neither active root nor active member | self | Call from one of the two admitted identities |

The two control-plane predicates currently discard the exact Registry or root
runtime cause. A simple membership miss may return the exact public access leaf;
Registry corruption, unavailable activation state or another protected failure
must retain its internal diagnostic and project through
`ACCESS_DEPENDENCY_UNAVAILABLE` instead of masquerading as an ordinary foreign
caller.

## Access Error Shape

The compact cut needs a finite access error shape with:

- exact access-denial variants;
- cause-preserving variants for typed auth/configuration/runtime failures; and
- explicit internal/public diagnostic pairs where a dependency cause is
  masked.

`Denied(String)` and `AccessErrorKind` are then sediment. They are deleted
together; a second three-class access taxonomy would only recreate the broad
wire enum inside the runtime.

The current access log may retain predicate name and static expression context
as operational metadata. It must log the numeric diagnostic and must not use
the rendered error string as identity. Caller principals remain subject to the
existing bounded logging policy and never join the diagnostic code or host
catalogue entry.

## Current Count

After the six explicit reuses above and typed cause preservation, this family
adds **20 exact candidate leaves** and two new safe projections:

- `ACCESS_DEPENDENCY_UNAVAILABLE`; and
- `ACCESS_CONFIGURATION_INVALID`.

`AUTH_PROOF_INVALID` and `RUNTIME_CONFIGURATION_INVALID` are reused projections,
not additional leaves.

## Required Tests

- exhaustive access mapping with no `Denied(String)` fallback;
- exact controller, whitelist, child, parent, root and self rejection codes;
- Fleet disabled-versus-read-only behavior;
- bounded malformed token and attestation rejection before cryptographic work;
- exact expiry and subject-mismatch semantic reuse;
- preservation of internal verifier, configuration and Registry causes through
  safe public projection;
- active-member negative-path tests distinguishing absence from unavailable or
  corrupt Registry state; and
- residue guards deleting `AccessErrorKind`, string-based verification mapping
  and formatted denial construction.

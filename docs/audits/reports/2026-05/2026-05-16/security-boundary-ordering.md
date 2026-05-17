# Security Boundary Ordering Audit - 2026-05-16

## Run Context

- Definition:
  `docs/audits/recurring/system/security-boundary-ordering.md`
- Related baselines:
  - `docs/audits/reports/2026-05/2026-05-16/canonical-auth-boundary.md`
  - `docs/audits/reports/2026-05/2026-05-16/access-purity.md`
  - `docs/audits/reports/2026-05/2026-05-16/ops-purity.md`
- Snapshot: `79cda86e`
- Branch: `main`
- Worktree: dirty
- Method: V1.0, ordering and trust-boundary scan

## Executive Summary

Risk: **3 / 10**.

No critical ordering violation was found.

Endpoint delegated-token updates follow:

```text
decode -> verify token material -> verify root/shard trust -> bind subject to caller -> check scope -> consume update replay marker -> dispatch handler
```

Root RPC capabilities follow two intentional replay modes:

- default root path authorizes before replay reservation;
- replay-first envelope path reserves before authorization, but aborts the
  reservation on authorization or execution failure and commits only after
  successful authorized execution.

The audit added two small regression guards:

- authenticated endpoint macro expansion must evaluate access before dispatch;
- cached root response attestations must still bind the attestation payload
  subject before reuse.

## Verification Ordering Map

| Boundary | Ordering | Verdict |
| --- | --- | --- |
| `access/auth/token.rs` | decode token, verify via `AuthOps`, bind subject/caller, enforce scope, consume update token | Pass |
| `ops/auth/token.rs` | config enabled, shard key binding, root trust anchor, local role, pure verifier, metrics | Pass |
| `ops/auth/delegated/verify.rs` | cert rules/time/hash, signature presence, root key resolution, root signature, claims/audience/scope, shard signature | Pass |
| endpoint macro wrapper | construct call/context, evaluate access, return denial, dispatch implementation | Pass |
| root RPC default | map request, authorize, replay preflight, execute, commit replay | Pass |
| root RPC replay-first | replay reserve, authorize, abort on denial, execute, abort on execution failure, commit on success | Pass |
| RPC attestation cache | cache lookup checks root/audience/subject/role/epoch/payload/expiry, then envelope hash is still rebuilt | Pass |

## Trust-Boundary Table

| Trust Boundary | Source Of Truth | Cache/Metric Status | Notes |
| --- | --- | --- | --- |
| Delegated token material | `ops/auth/delegated/verify.rs` | No cache | Verifier returns only after trust chain, audience, and scopes pass. |
| Endpoint subject | Access context plus verified token subject | No cache | `enforce_subject_binding` runs before update-token consumption. |
| Update replay token | Auth stable replay marker | State mutation | Consumed only after full endpoint authorization and before handler dispatch. |
| Root replay | Root replay store | State mutation | Reservations are pending markers; commits store responses only after success. |
| Role attestation | Root registry and signing key | Cache on caller side only | Cached attestation is reused only if payload fields still bind to request context. |
| Metrics | Runtime metrics stores | Not a trust source | Metrics record outcomes but are not read back as authorization inputs. |

## Replay Sequencing Analysis

### Endpoint Update Tokens

The endpoint delegated token path consumes update tokens after token material
verification, subject binding, and required-scope enforcement. The consumption
happens before handler dispatch, which is intentional: it prevents a duplicated
update from reaching user side effects.

Existing guard:

- `delegated_auth_guard_preserves_verify_bind_scope_consume_order`

### Root RPC Replay

Default root request handling authorizes before replay validation. This avoids
reserving replay state for clearly unauthorized requests.

Replay-first handling is used for capability-envelope execution and cycles
paths. In that path, fresh replay reservations are aborted on policy denial or
execution failure. Cached duplicate responses are decoded before acceptance;
decode failures become replay errors rather than partial success.

Existing guards:

- replay-first validates replay before policy when selected;
- replay-first aborts reserved replay state on policy denial;
- compact replay decode rejects trailing bytes and truncated payloads.

## Endpoint Macro Sequencing Analysis

Generated endpoint wrappers build the `EndpointCall`, resolve the caller or
authenticated identity, evaluate default or explicit access, return on denial,
and only then call dispatch.

Added guard:

- `authenticated_endpoint_expansion_evaluates_access_before_dispatch`

The macro also rejects access-gated infallible endpoints, which prevents guard
denials from becoming traps.

## RPC Capability Handling Review

Capability envelopes are built after request mapping and, for signed root
capabilities, after resolving a role attestation. The capability hash includes
the target canister, capability service, capability version, and canonical
request without metadata.

The attestation cache is accepted as a cache, not as authority:

- cache key fields must match root, audience, subject, role, and epoch;
- cached payload fields must also match subject, role, audience, and epoch;
- cached payload must not be expired;
- the capability hash is rebuilt per request even with a cached attestation.

Added guard:

- `cached_root_response_attestation_rejects_payload_subject_drift`

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `access/auth/token.rs` | Medium | Keep verify/bind/scope/consume order mechanical and tested. |
| `ops/auth/token.rs` | Medium | Metrics must remain bounded outcome recording, never verifier input. |
| `ops/rpc/mod.rs` | Medium | Attestation cache and capability envelope hashing are security-sensitive. |
| `workflow/rpc/request/handler/replay.rs` | Medium | Replay-first paths must keep abort-on-denial and commit-after-success. |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | Medium | Cycles funding combines replay, policy, execution, ledger, and metrics. |
| `canic-macros/src/endpoint/expand.rs` | Medium | Wrapper code must keep access evaluation before dispatch. |

## Recommended Guard Additions

Completed in this slice:

- macro expansion test for authenticated access-before-dispatch;
- attestation cache payload-subject binding test.

Recommended future guard if this surface changes:

- add a source-order test for root RPC replay-first sequence:
  reserve, authorize, abort on denial, execute, abort on execution failure,
  commit on success.

## Final Verdict

Pass with watchpoints.

Ordering invariants hold. The main risk is future drift in the central hot
paths, not a current bypass.

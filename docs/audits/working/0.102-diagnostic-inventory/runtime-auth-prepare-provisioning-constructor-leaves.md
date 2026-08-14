# Canic 0.102 Runtime Auth Prepare And Provisioning Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all seventeen production
`InternalError` constructor references in the authentication prepare
orchestrator and issuer-proof provisioning adapter. It assigns no number and
changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/auth/prepare/mod.rs` | 3 |
| `workflow/runtime/auth/provisioning/mod.rs` | 14 |
| **Total** | **17** |

## Prepare Orchestration

The three direct sites add two exact retained-response capacity meanings and
retain one transparent remote diagnostic:

| Exact candidate or disposition | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `REPLAY_RETAINED_ACTOR_CAPACITY` | 1 | `ResourceExhausted` / per-actor retained responses | self | Release or wait for retained responses before preparing another token |
| `REPLAY_RETAINED_COMMAND_CAPACITY` | 1 | `ResourceExhausted` / per-command retained responses | self | Release or wait for command-wide retention capacity |
| transparent remote public diagnostic | 1 | root-issued delegation-proof response | source diagnostic | Preserve the exact registered root diagnostic without an orchestration wrapper |

Retained-response capacity is not pending-operation capacity. The former
bounds replayable terminal responses; the latter bounds in-flight receipts and
is already represented by `REPLAY_PENDING_ACTOR_CAPACITY` and
`REPLAY_PENDING_COMMAND_CAPACITY`. They must remain independently observable
and configurable.

Reservation, lazy repair, replay-owner revalidation, response staging and
response commit otherwise preserve their typed errors. If abort or recovery-
marker cleanup also fails, the shared replay helper must retain the primary
registered diagnostic and attach the secondary typed failure only to its
operation-correlated observation. That helper's dynamic values remain in the
shared replay-helper frontier rather than being counted as new prepare
constructors here.

## Issuer-Proof Availability And Completion

Four provisioning sites add one exact meaning and reuse the qualified proof-
availability meaning:

| Exact candidate or disposition | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| reuse `AUTH_PROOF_UNAVAILABLE` | 3 | `Unavailable` / signed issuer proof | self | Retry after signing or proof preparation advances |
| `AUTH_ISSUER_PROOF_INSTALLATION_INCOMPLETE` | 1 | `Unavailable` / issuer install batch | self | Inspect the exact batch and issuer failure before retry |

Missing batch identity, signing still in flight and absent signed issuer proof
share one exact meaning because each reports the same not-yet-available
artifact, retry condition and batch authority. A completed loop with neither a
success nor retained first failure is a different state contradiction and
therefore receives its own identity.

## Issuer Install Cause Propagation

Eight constructors currently replace typed causes with issuer-specific prose.
They add no wrapper identity:

| Exact candidate or disposition | Sites | Current boundary | Required hard cut |
| --- | ---: | --- | --- |
| reuse `IC_CALL_REQUEST_ENCODING_FAILED` | 2 | interactive install and renewal batch | Preserve the exact typed request-encoding cause |
| transparent typed IC transport cause | 2 | interactive install and renewal batch | Preserve the exact transport/effect diagnostic and its retry disposition |
| reuse `IC_CALL_RESPONSE_DECODING_FAILED` | 2 | interactive install and renewal batch | Preserve the exact typed response-decoding cause |
| transparent issuer public diagnostic | 2 | interactive install and renewal batch | Return or record the issuer's registered diagnostic unchanged |

Interactive provisioning and timer renewal may project the same nested cause
differently only when their exposure boundary requires it; they must not mint
separate encoding, transport or response identities. The install-failure
record remains a bounded lifecycle disposition, but it does not replace the
exact diagnostic in the operation-specific or guarded runtime observation.

## Protected Configuration

The final two sites reuse existing configuration meanings:

| Exact candidate or disposition | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| reuse `AUTH_DELEGATED_TOKEN_MAX_TTL_OVERFLOW` | 1 | `InvalidInput` / protected TTL configuration | `RUNTIME_CONFIGURATION_INVALID` | Configure a maximum TTL representable in nanoseconds |
| reuse `AUTH_CHAIN_KEY_CONFIG_REQUIRED` | 1 | `Invariant` / protected verifier configuration | `RUNTIME_CONFIGURATION_INVALID` | Configure the required minimum accepted proof epoch |

These are the same fields, authority and repair actions already qualified in
the renewal path. Lazy repair does not create a second configuration owner.

## Dynamic Public Context

Fourteen values are classified as `DPC-236` through `DPC-249` in
[dynamic-public-context.md](dynamic-public-context.md). Seven are caller-
derivable quota, command and issuer values. Seven are typed recovery, request,
transport or response causes that must propagate without `to_string()`.

The shared `abort_reserved_receipt_after_failure` formatter is outside these
seventeen direct sites. Its static context and typed secondary cleanup failure
remain one explicit shared replay-helper input; they are not silently treated
as closed by this ledger.

## Reconciliation

All seventeen direct sites now have one disposition. They add three exact
meanings, reuse five existing exact identities and retain nine transparent
typed edges. The effective constructor frontier moves from 2,276 to 2,293
classified sites and from 223 to 206 open sites. The qualified semantic set
reaches 2,504 exact candidates plus 31 safe projections: 2,535 current symbolic
identities.

## Required Tests

- retained-response versus pending-receipt capacity remains distinct;
- all three proof-not-yet-available branches reuse one exact identity;
- zero-success/no-failure installation fails closed;
- interactive and renewal mappings preserve exact request, transport,
  response and issuer diagnostics without prose wrappers;
- issuer identity and nested cause prose disappear from compact diagnostics;
- protected TTL and proof-epoch configuration reuse exact existing meanings;
- cleanup/recovery-marker failure preserves the primary code and records the
  typed secondary failure against the same replay operation; and
- lazy repair revalidates the exact replay owner after the await before the
  second preparation attempt.

## Next Slice

Continue with the root-issuer policy facade and root delegation-batch sweep,
preserving typed policy and batch causes without string dispatch.

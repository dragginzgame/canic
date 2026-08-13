# Canic 0.102 IC Infrastructure Diagnostic Leaves

Date: 2026-08-12

## Status

This provisional B1 ledger covers the complete IC infrastructure frontier at
`v0.101.53`. It follows the seven Canic-owned error types already counted in
[transitive-error-inventory.md](transitive-error-inventory.md) and expands the
four dependency-owned boundaries that currently reach `IcInfraError`.

It allocates no numbers. The labels below are semantic candidates for the final
allocation review. Aggregate wrappers and dependency error values never become
Canic protocol identities by formatting.

## Aggregate Boundary

`IcInfraError` has five transparent Canic-owned wrapper variants plus raw call,
Candid-encode and Candid-decode variants. The wrapper variants receive no
codes. `OpsError::IcInfra` and `RequestOpsError::IcInfra` are further transparent
ownership edges and likewise receive no codes.

The present terminal conversions format the whole chain into one broad infra,
ops or RPC error. B4 must instead map at the `infra::ic` adapter, while the
source is still typed.

## Canic-Owned Leaves

### Embedded release-build identity

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `RELEASE_BUILD_IDENTITY_MISSING` | `EmbeddedReleaseBuildError::Missing` | `Invariant` / embedded build identity | `RUNTIME_RELEASE_BUILD_INVALID` | Rebuild through the qualified builder; no unchanged runtime retry |
| `RELEASE_BUILD_IDENTITY_LENGTH_INVALID` | `EmbeddedReleaseBuildError::Invalid(ReleaseBuildIdParseError::Length)` | `Invariant` / embedded build identity | `RUNTIME_RELEASE_BUILD_INVALID` | Correct build metadata; no unchanged runtime retry |
| `RELEASE_BUILD_IDENTITY_ENCODING_INVALID` | `EmbeddedReleaseBuildError::Invalid(ReleaseBuildIdParseError::CanonicalHex)` | `Invariant` / embedded build identity | `RUNTIME_RELEASE_BUILD_INVALID` | Correct canonical lowercase hexadecimal metadata; no unchanged runtime retry |

`EmbeddedReleaseBuildError::Invalid` is a cause edge. The two parse reasons are
the leaves.

### Cycles Ledger and ICP refill adapters

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `CYCLES_LEDGER_BALANCE_OUT_OF_RANGE` | `CyclesLedgerInfraError::CyclesOverflow` | `Invariant` / Cycles Ledger response | `IC_PLATFORM_RESPONSE_INVALID` | Reject the response and inspect adapter/ledger contract; no blind retry |
| `CYCLES_LEDGER_BLOCK_INDEX_OUT_OF_RANGE` | `CyclesLedgerInfraError::BlockIndexOverflow` | `Invariant` / Cycles Ledger response | `IC_PLATFORM_RESPONSE_INVALID` | Reject the response and inspect adapter/ledger contract; no blind retry |
| `ICP_REFILL_BLOCK_INDEX_OUT_OF_RANGE` | `IcpRefillInfraError::LedgerBlockIndexOverflow` | `Invariant` / ICP Ledger response | `IC_PLATFORM_RESPONSE_INVALID` | Reject the response and inspect refill contract; no blind retry |
| `ICP_REFILL_MAINNET_OVERRIDE_REJECTED` | `IcpRefillInfraError::MainnetSystemCanisterOverrideRejected` | `Forbidden` / refill network policy | self | Remove unsafe mainnet overrides or use the explicit test-only authority |
| `ICP_REFILL_TARGET_PRINCIPAL_TOO_LONG` | `IcpRefillInfraError::PrincipalTooLongForCmcSubaccount` | `InvalidInput` / CMC subaccount derivation | self | Supply a principal representable by the CMC contract |

The two block-index overflows remain separate because their protocol owners and
operator remediation differ even though both are range failures.

### Management and NNS adapters

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `MANAGEMENT_CANISTER_CYCLES_OUT_OF_RANGE` | `MgmtInfraError::CanisterCyclesOverflow` | `Invariant` / management response | `IC_PLATFORM_RESPONSE_INVALID` | Reject impossible local representation; inspect platform/adapter contract |
| `MANAGEMENT_SIGN_COST_ALGORITHM_INVALID` | `SignCostError::InvalidCurveOrAlgorithm` | `Invariant` / management cost API | `IC_PLATFORM_EFFECT_FAILED` | Correct the protected signing algorithm; no unchanged retry |
| `MANAGEMENT_SIGN_COST_KEY_INVALID` | `SignCostError::InvalidKeyName` | `InvalidInput` / management cost API | `IC_PLATFORM_EFFECT_FAILED` | Correct protected key selection; no unchanged retry |
| `MANAGEMENT_SIGN_COST_UNKNOWN` | `SignCostError::UnrecognizedError` | `Invariant` / management cost API version boundary | `IC_PLATFORM_EFFECT_FAILED` | Fail closed and review IC/CDK version support |
| `NNS_REGISTRY_REQUEST_REJECTED` | `NnsRegistryInfraError::Rejected` | `Unavailable` / NNS Registry query result | `IC_PLATFORM_EFFECT_FAILED` | Inspect bounded operator context; retry only when Registry state or availability changes |

`MgmtInfraError::SignCost` is a cause edge. The pinned `ic-cdk 0.20.2`
`SignCostError` has three variants, including a forward-compatible unknown
numeric system result. Canic needs an owned three-reason adapter so a dependency
upgrade forces review rather than silently formatting a new result.

The NNS rejection reason is provider text. It may remain in a bounded approved
operator log, but it is not part of the code identity and never crosses the
compact public boundary.

## Raw Call Boundary

The pinned `ic-cdk 0.20.2` `CallFailed` surface has three variants. Its
`CallRejected` value exposes six known `RejectCode` values plus an unrecognized
raw code. The exact owned adapter leaves are:

| Candidate label | Dependency fact | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `IC_CALL_LIQUID_CYCLES_INSUFFICIENT` | `CallFailed::InsufficientLiquidCycleBalance` | `ResourceExhausted` / local call admission | self | Top up liquid cycles before retrying |
| `IC_CALL_PERFORM_FAILED` | `CallFailed::CallPerformFailed` | `Unavailable` / local call admission | `IC_PLATFORM_EFFECT_FAILED` | Effect was not performed; retry only through the owning idempotent workflow |
| `IC_CALL_REJECTED_SYSTEM_FATAL` | `RejectCode::SysFatal` | `Unavailable` / IC call rejection | `IC_PLATFORM_EFFECT_FAILED` | Stop blind retry and inspect platform state |
| `IC_CALL_REJECTED_SYSTEM_TRANSIENT` | `RejectCode::SysTransient` | `Unavailable` / IC call rejection | self | Bounded retry through the owning same-release journal |
| `IC_CALL_REJECTED_DESTINATION_INVALID` | `RejectCode::DestinationInvalid` | `NotFound` / IC destination | self | Re-observe exact target; destructive absence decisions retain the typed rejection |
| `IC_CALL_REJECTED_BY_CANISTER` | `RejectCode::CanisterReject` | `Conflict` / remote Canister | `IC_PLATFORM_EFFECT_FAILED` | Inspect the typed remote protocol; no text-based classification |
| `IC_CALL_REJECTED_CANISTER_ERROR` | `RejectCode::CanisterError` | `Unavailable` / remote Canister execution | `IC_PLATFORM_EFFECT_FAILED` | Inspect target health; retry only when the owning operation is safe |
| `IC_CALL_REJECTED_SYSTEM_UNKNOWN` | `RejectCode::SysUnknown` | `Unavailable` / IC call rejection | `IC_PLATFORM_EFFECT_FAILED` | Treat outcome according to the owning effect journal; no blind retry |
| `IC_CALL_REJECT_CODE_UNKNOWN` | `CallRejected::reject_code()` fails | `Invariant` / CDK version boundary | `IC_PLATFORM_EFFECT_FAILED` | Fail closed and review IC/CDK support |

The raw rejection message never selects a code. `raw_reject_code()` is used only
to preserve the unknown-code distinction after the typed getter rejects it.

`IcInfraError::is_canister_not_found()` is a current machine consumer. It must
continue to decide from the typed destination-invalid rejection before public
projection. A compact diagnostic code is not a substitute for exact typed
absence evidence, and transport failure, system rejection or unreachable state
must never be treated as absence.

## Candid Boundary

| Candidate label | Dependency fact | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `IC_CALL_REQUEST_ENCODING_FAILED` | `candid::Error` while constructing call arguments | `Invariant` / IC call request adapter | `IC_PLATFORM_PROTOCOL_INVALID` | Fix the owned DTO/adapter; no unchanged runtime retry |
| `IC_CALL_RESPONSE_DECODING_FAILED` | `CandidDecodeFailed` | `Invariant` / IC call response adapter | `IC_PLATFORM_PROTOCOL_INVALID` | Qualify the exact remote interface and response type; no blind retry |

The Candid formatter's type name and decoder message are not code inputs. If
the dynamic details are needed during development, they belong in a bounded
internal observation, not the public error or the host catalog key.

## Current Count And Projections

The frontier provisionally yields **24 exact candidate leaves**:

- 13 from Canic-owned release-build, Ledger, refill, management and NNS
  semantics;
- nine from call admission and rejection; and
- two from Candid request/response adaptation.

It also introduces four safe public projections:

- `RUNTIME_RELEASE_BUILD_INVALID`;
- `IC_PLATFORM_RESPONSE_INVALID`;
- `IC_PLATFORM_PROTOCOL_INVALID`; and
- `IC_PLATFORM_EFFECT_FAILED`.

An exact leaf may be public as-is only where it reveals no protected target and
gives the caller a necessary distinct action. Every masked exact leaf needs a
numeric observability owner in the calling workflow. That context review may
make a projection narrower, but it must not collapse an internal typed fact or
permit message parsing.

## Required Tests

- exhaustive mappings for the seven Canic-owned types, with transparent
  wrappers allocating no codes;
- pinned exhaustive adapters for all three `CallFailed`, three `SignCostError`
  and six known reject-code variants plus the dependency-unknown cases;
- request-encoding versus response-decoding separation;
- raw reject and decoder text non-propagation;
- typed destination-invalid absence tests alongside every non-absence reject;
- safe public projection and numeric internal-observation tests; and
- an adapter review guard that fails when the pinned `ic-cdk` error surface
  changes.

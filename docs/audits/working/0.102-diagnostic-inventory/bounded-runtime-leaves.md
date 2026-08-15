# Canic 0.102 Bounded Runtime Diagnostic Leaves

Date: 2026-08-12

## Status

This provisional B1 ledger covers the small standalone terminal owners left
after configuration, Component policy, authentication and IC infrastructure.
It allocates no numbers. All source values, principals, limits and available
pool lists disappear from the compact error; the candidate label is the entire
diagnostic identity.

## Topology Snapshot Validation

All 14 `TopologySnapshotValidationError` variants have live producers and
remain distinct:

| Candidate label | Typed producer | Caller action |
| --- | --- | --- |
| `TOPOLOGY_PARENT_CHAIN_EMPTY` | `EmptyParentChain` | Send a non-empty receiver-first branch |
| `TOPOLOGY_RECEIVER_MISMATCH` | `ReceiverMismatch` | Bind the first node to the exact receiver |
| `TOPOLOGY_RECEIVER_ROLE_MISMATCH` | `ReceiverRoleMismatch` | Bind the receiver's admitted role |
| `TOPOLOGY_IMMEDIATE_PARENT_MISMATCH` | `ImmediateParentMismatch` | Bind the exact registered immediate parent |
| `TOPOLOGY_PATH_NODE_DUPLICATED` | `DuplicatePathNode` | Remove the repeated path node |
| `TOPOLOGY_PARENT_LINK_BROKEN` | `BrokenParentLink` | Restore exact immediate-parent continuity |
| `TOPOLOGY_CHILDREN_ROW_DUPLICATED` | `DuplicateChildrenRow` | Send one row per branch parent |
| `TOPOLOGY_CHILDREN_ROW_MISSING` | `MissingChildrenRow` | Include every branch parent's row |
| `TOPOLOGY_CHILDREN_ROW_UNEXPECTED` | `UnexpectedChildrenRow` | Remove rows outside the branch |
| `TOPOLOGY_CHILD_DUPLICATED` | `DuplicateChild` | Deduplicate the parent's child list |
| `TOPOLOGY_CHILD_PARENT_CONFLICT` | `ConflictingChildParent` | Bind the child to one immediate parent |
| `TOPOLOGY_PARENT_LISTS_SELF` | `SelfChild` | Remove the self-child edge |
| `TOPOLOGY_NEXT_HOP_MISSING` | `NextHopMissing` | Include the path successor in the direct-child row |
| `TOPOLOGY_NEXT_HOP_ROLE_MISMATCH` | `NextHopRoleMismatch` | Bind the next hop to its admitted role |

These leaves are safe as-is for the authenticated topology RPC. They expose no
principal or role value, and the exact rejection is necessary to repair the
snapshot. The typed input remains the authority; the receiver must never infer
parentage from a diagnostic code.

## Runtime Log Storage

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `RUNTIME_LOG_COUNT_INCONSISTENT` | `StorageError::LogCountInvariant` | `Invariant` / runtime log store | `RUNTIME_LOG_STATE_INVALID` | Stop mutation and inspect durable log state |
| `RUNTIME_LOG_SEQUENCE_CONFLICT` | `StorageError::LogSequenceConflict` | `Conflict` / runtime log store | `RUNTIME_LOG_STATE_INVALID` | Stop mutation; do not overwrite the existing sequence |
| `RUNTIME_LOG_SEQUENCE_EXHAUSTED` | `StorageError::LogSequenceExhausted` | `ResourceExhausted` / runtime log store | `RUNTIME_LOG_STATE_INVALID` | Stop appending; operator intervention is required |
| `RUNTIME_LOG_TIMESTAMP_REGRESSED` | `StorageError::LogTimestampRegressed` | `Invariant` / runtime log store | `RUNTIME_LOG_STATE_INVALID` | Reject the regressing record and inspect time/source state |

The exact internal code must be written to an approved numeric runtime
observation because the public projection deliberately masks durable log
details.

## ICP Refill Workflow And Policy

`IcpRefillWorkflowError::PolicyDenied` is a transparent cause edge. The seven
`IcpRefillPolicyViolation` values are the leaves.

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `ICP_REFILL_DRY_RUN_EXECUTION_REJECTED` | `DryRunRequest` | `InvalidInput` / refill command | self | Use the dry-run endpoint or clear `dry_run` |
| `ICP_REFILL_VALUE_OUT_OF_RANGE` | `NatU64Overflow` | `Invariant` / refill response conversion | `ICP_REFILL_RESPONSE_INVALID` | Reject the upstream value; no blind retry |
| `ICP_REFILL_LEDGER_DECIMALS_UNEXPECTED` | `UnexpectedLedgerDecimals` | `Invariant` / ICP Ledger contract | `ICP_REFILL_RESPONSE_INVALID` | Stop refill and verify the selected Ledger |
| `ICP_REFILL_NOT_CONFIGURED` | `PolicyDenied(NotConfigured)` | `Unavailable` / refill policy | self | Configure refill before retrying |
| `ICP_REFILL_CYCLES_FUNDING_DISABLED` | `PolicyDenied(CyclesFundingDisabled)` | `Unavailable` / funding policy | self | Enable cycles funding before retrying |
| `ICP_REFILL_AMOUNT_ZERO` | `PolicyDenied(AmountZero)` | `InvalidInput` / refill request | self | Supply a positive ICP amount |
| `ICP_REFILL_AMOUNT_EXCEEDS_LIMIT` | `PolicyDenied(MaxRefillPerCall)` | `InvalidInput` / refill policy | self | Reduce the amount to the configured ceiling |
| `ICP_REFILL_RATE_UNAVAILABLE` | `PolicyDenied(RateUnavailable)` | `Unavailable` / rate gate | self | Refresh trusted rate evidence before retrying |
| `ICP_REFILL_RATE_GATE_DENIED` | `PolicyDenied(RateGateDenied)` | `Unavailable` / rate gate | self | Retry only after the observed rate satisfies policy |
| `ICP_REFILL_ALREADY_IN_PROGRESS` | `PolicyDenied(ConcurrentRefill)` | `Conflict` / refill concurrency | self | Resume or await the existing operation |

One adjacent direct prose constructor is also a current producer:
`require_build_network()` becomes
`ICP_REFILL_BUILD_NETWORK_UNAVAILABLE`. It is an invariant owned by build
configuration, projects through `RUNTIME_CONFIGURATION_UNAVAILABLE`, and cannot
succeed unchanged at runtime.

The policy mapper already distinguishes invalid input, conflict and
unavailable by typed variant before formatting. B4 must retain that exhaustive
match and remove the intermediate `message` construction. The durable refill
record already has a separate typed `IcpRefillErrorCode`; its `error_message`
belongs to the B5 field-by-field audit and must not be conflated with these
request-policy diagnostics.

## Placement Index Workflow

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `PLACEMENT_INDEX_DISABLED` | `IndexDisabled` | `Unavailable` / placement configuration | self | Configure an Index before requesting placement |
| `PLACEMENT_INDEX_POOL_UNKNOWN` | `UnknownPool` | `InvalidInput` / placement configuration | self | Select a configured pool |
| `PLACEMENT_INDEX_INSTANCE_NOT_DIRECT_CHILD` | `InstanceNotDirectChild` | `Forbidden` / parent authority | self | Use an exact registered direct child |
| `PLACEMENT_INDEX_INSTANCE_ROLE_MISMATCH` | `InstanceRoleMismatch` | `Forbidden` / role authority | self | Use a child with the admitted Index role |

The current `UnknownPool` prose embeds the requested name and a joined list of
available pools. Those values remain available from typed configuration/status
surfaces and do not enter the compact diagnostic.

## Current Blob-Storage Boundary And The 0.109 Hard Cut

The 0.109 standalone-blob design removes this entire owner from Canic, but a
future hard cut is not evidence that a current 0.102 producer is absent. The
maintained release order is 0.102 before 0.109, so every current producer below
must receive a 0.102 identity and its number must retire without reuse when the
later design removes the subsystem. There is no temporary generic blob code.

### Root Hash And Lifecycle: 7

| Candidate label | Typed producer | Class / public projection | Action and retry |
| --- | --- | --- | --- |
| `BLOB_ROOT_HASH_EMPTY` | `BlobRootHashError::Empty` | `InvalidInput` / self | Supply a nonempty canonical hash |
| `BLOB_ROOT_HASH_PREFIX_INVALID` | `BlobRootHashError::InvalidPrefix` | `InvalidInput` / self | Supply the exact `sha256:` prefix |
| `BLOB_ROOT_HASH_LENGTH_INVALID` | `BlobRootHashError::InvalidLength` | `InvalidInput` / self | Supply the required canonical text length |
| `BLOB_ROOT_HASH_HEX_INVALID` | `BlobRootHashError::InvalidHexCharacter` | `InvalidInput` / self | Supply lowercase or uppercase hexadecimal bytes only |
| `BLOB_ROOT_HASH_BYTE_LENGTH_INVALID` | `BlobStorageConversionError::InvalidRootHashByteLength` | `InvalidInput` / self | Supply exactly one 32-byte root hash |
| `BLOB_NOT_LIVE` | `BlobStorageLifecycleError::BlobNotLive` | `NotFound` / self | Register the blob before using the lifecycle operation |
| `BLOB_PENDING_DELETION` | `BlobStorageLifecycleError::BlobPendingDeletion` | `Conflict` / self | Complete or cancel the existing deletion journey before retrying |

`BlobStorageConversionError::InvalidRootHash` is a transparent cause edge and
owns no additional identity.

### Billing Configuration And Admission: 11

| Candidate label | Typed producer | Class / public projection | Action and retry |
| --- | --- | --- | --- |
| `BLOB_BILLING_CONFIG_MISSING` | `BlobStorageBillingWorkflowError::BillingConfigMissing` | `Unavailable` / self | Configure billing before retrying the dependent operation |
| `BLOB_BILLING_CASHIER_CANISTER_REQUIRED` | `BlobStorageBillingPolicyError::CashierCanisterRequired` | `InvalidInput` / self | Supply one concrete Cashier Canister principal |
| `BLOB_BILLING_GATEWAY_PRINCIPAL_LIMIT_ZERO` | `BlobStorageBillingPolicyError::GatewayPrincipalLimitZero` | `InvalidInput` / self | Configure a positive gateway-principal limit |
| `BLOB_BILLING_PROJECT_CYCLES_RESERVE_ZERO` | `BlobStorageBillingPolicyError::ProjectCyclesReserveZero` | `InvalidInput` / self | Configure a positive project cycles reserve |
| `BLOB_BILLING_MIN_UPLOAD_BALANCE_ZERO` | `BlobStorageBillingPolicyError::MinUploadBalanceZero` | `InvalidInput` / self | Configure a positive minimum upload balance |
| `BLOB_BILLING_TARGET_UPLOAD_BALANCE_ZERO` | `BlobStorageBillingPolicyError::TargetUploadBalanceZero` | `InvalidInput` / self | Configure a positive target upload balance |
| `BLOB_BILLING_MIN_UPLOAD_BALANCE_EXCEEDS_TARGET` | `BlobStorageBillingPolicyError::MinUploadBalanceExceedsTarget` | `InvalidInput` / self | Raise the target or lower the minimum balance |
| `BLOB_BILLING_REQUESTED_CYCLES_ZERO` | `BlobStorageBillingPolicyError::RequestedCyclesZero` | `InvalidInput` / self | Request a positive funding amount |
| `BLOB_BILLING_NAT_EXCEEDS_U128` | `BlobStorageConversionError::BillingNatExceedsU128` | `InvalidInput` / self | Reduce the rejected request/configuration field to `u128` range |
| `BLOB_BILLING_GATEWAY_PRINCIPAL_LIMIT_EXCEEDS_USIZE` | `BlobStorageConversionError::GatewayPrincipalLimitExceedsUsize` | `InvalidInput` / self | Reduce the configured limit to the target's collection bound |
| `BLOB_BILLING_FUNDING_IN_PROGRESS` | `BlobStorageFundingInProgress` | `Conflict` / self | Await or reconcile the in-flight funding operation, then retry |

`BlobStorageBillingWorkflowError::{BillingPolicy, BoundaryConversion,
FundingInProgress}` are transparent typed edges. The configuration field name
embedded by the Nat conversion is caller-derived and does not create separate
leaves.

### Cashier Response And Top-Up: 9

| Candidate label | Typed producer | Class / public projection | Action and retry |
| --- | --- | --- | --- |
| `BLOB_CASHIER_CYCLE_BALANCE_INVALID` | `CashierDecodeError::InvalidCycleBalance` | `Invariant` / `BLOB_CASHIER_RESPONSE_INVALID` | Stop and inspect the exact guarded Cashier observation |
| `BLOB_CASHIER_GATEWAY_LIST_EMPTY` | `CashierDecodeError::EmptyGatewayPrincipalList` | `Invariant` / `BLOB_CASHIER_RESPONSE_INVALID` | Do not replace gateway authority from the empty response |
| `BLOB_CASHIER_GATEWAY_PRINCIPAL_INVALID` | `CashierDecodeError::InvalidGatewayPrincipal` | `Invariant` / `BLOB_CASHIER_RESPONSE_INVALID` | Reject the response without exposing the foreign principal |
| `BLOB_CASHIER_GATEWAY_LIMIT_EXCEEDED` | `CashierDecodeError::TooManyGatewayPrincipals` | `Invariant` / `BLOB_CASHIER_RESPONSE_INVALID` | Reject the over-limit response before authority mutation |
| `BLOB_CASHIER_BALANCE_INTERNAL` | `BlobStorageBillingWorkflowError::CashierBalanceInternal` | `Unavailable` / self | Retry only under the caller's bounded provider policy; discard foreign prose |
| `BLOB_CASHIER_TOP_UP_NOT_AUTHORIZED` | `BlobStorageCashierAccountTopUpError::NotAuthorized` | `Forbidden` / self | Inspect Cashier account authority without exposing its asserted principal |
| `BLOB_CASHIER_TOP_UP_ACCOUNT_BALANCE_OVERFLOW` | `BlobStorageCashierAccountTopUpError::AccountBalanceOverflow` | `ResourceExhausted` / self | Repair the Cashier account balance before retrying |
| `BLOB_CASHIER_TOP_UP_INTERNAL` | `BlobStorageCashierAccountTopUpError::InternalError` | `Unavailable` / self | Retry only under bounded provider policy; discard foreign prose |
| `BLOB_CASHIER_TOP_UP_WITHOUT_CYCLES` | `BlobStorageCashierAccountTopUpError::TopUpWithoutCycles` | `Invariant` / `BLOB_CASHIER_RESPONSE_INVALID` | Reconcile the admitted call/effect; never blame caller input for the provider contradiction |

The four decode leaves and the top-up-without-cycles contradiction record their
exact numeric identities in the guarded runtime observation before projecting
to `BLOB_CASHIER_RESPONSE_INVALID`. Foreign Cashier values, principals and
messages never cross the public compact boundary. The other four response
variants are safe exact identities only after their dynamic payload is removed.
`BlobStorageBillingWorkflowError::{CashierDecode, CashierTopUp, Internal}` are
transparent edges; the last preserves the already-qualified IC call diagnostic
and its approved projection.

## Current Count

The current-source frontier contains **60 exact semantic candidates**:

- 14 topology snapshot leaves;
- four runtime-log leaves;
- ten typed refill leaves plus one direct build-network leaf;
- four Placement Index leaves; and
- 27 current blob-storage leaves whose numbers retire when 0.109 removes their
  producers.

It introduces four safe projections:

- `RUNTIME_LOG_STATE_INVALID`;
- `ICP_REFILL_RESPONSE_INVALID`;
- `RUNTIME_CONFIGURATION_UNAVAILABLE`; and
- `BLOB_CASHIER_RESPONSE_INVALID` until its 0.109 retirement.

## Required Tests

- exhaustive typed mapping for every retained variant;
- exact topology rejection tests with no dynamic identity in the public error;
- masked runtime-log numeric observability;
- refill policy mapping without intermediate formatting;
- exact missing build-network mapping;
- Placement Index errors without requested/available pool prose;
- exhaustive blob hash, lifecycle, billing-policy, conversion, Cashier and
  funding mapping in 0.102, including transparent wrapper tests;
- exact guarded Cashier observations before safe projection and
  retirement-without-reuse evidence in 0.109;
- no durable refill recovery decision based on `error_message`.

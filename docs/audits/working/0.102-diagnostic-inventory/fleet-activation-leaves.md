# Canic 0.102 Fleet Activation Diagnostic Leaves

Date: 2026-08-13

## Status

This provisional B1 ledger covers fresh activation admission, the protected
activation record and its status/transition mapper at immutable baseline
`v0.101.53`. It allocates no numbers. The current free-form `InvalidRecord` and
`InvalidTransition` reasons are grouped only where owner, action and retry
policy are identical.

## Fresh Admission

The eight direct `PrepareFleetActivationError` decisions remain exact internal
candidates:

```text
FLEET_ACTIVATION_RELEASE_BUILD_MISMATCH
FLEET_ACTIVATION_AUTHORITY_EPOCH_INVALID
FLEET_ACTIVATION_APP_MISMATCH
FLEET_ACTIVATION_ROOT_PRINCIPAL_MISMATCH
FLEET_ACTIVATION_WASM_STORE_AUTHORITY_MISMATCH
FLEET_ACTIVATION_WASM_STORE_PRINCIPAL_INVALID
FLEET_ACTIVATION_WASM_STORE_PRINCIPAL_MISMATCH
FLEET_ACTIVATION_WASM_STORE_MODULE_HASH_ZERO
```

`PrepareFleetActivationError::Topology` is a transparent edge, but this path
has a different authority and recovery action from native configuration
compilation. `prepare_root_install` calls `validate_root_binding`; its eleven
reachable path-qualified candidates are:

```text
FLEET_ACTIVATION_TOPOLOGY_ANONYMOUS_BINDING_PRINCIPAL
FLEET_ACTIVATION_TOPOLOGY_ROOT_PRINCIPAL_CONFLICT
FLEET_ACTIVATION_TOPOLOGY_ROOT_LIMIT_NONPOSITIVE
FLEET_ACTIVATION_TOPOLOGY_CANISTER_POOL_RANGE_INVALID
FLEET_ACTIVATION_TOPOLOGY_ADMISSIONS_EMPTY
FLEET_ACTIVATION_TOPOLOGY_ADMISSION_ORDER_NONCANONICAL
FLEET_ACTIVATION_TOPOLOGY_ADMISSION_ZERO
FLEET_ACTIVATION_TOPOLOGY_ADMISSION_SPEC_UNKNOWN
FLEET_ACTIVATION_TOPOLOGY_ADMISSION_SPEC_HASH_MISMATCH
FLEET_ACTIVATION_TOPOLOGY_ADMISSION_EXCEEDS_FLEET_MAXIMUM
FLEET_ACTIVATION_TOPOLOGY_DIGEST_MISMATCH
```

`ComponentTopologyError::CanonicalBytesExceeded` is not reachable on this
path: the topology was already compiled within the canonical bound and the
root projection is a subset. Fleet-wide duplicate-root and admission-sum
checks belong to the Fleet Registry/planning owners, not one root's init.

All 19 fresh-admission leaves project publicly to
`FLEET_ACTIVATION_ADMISSION_INVALID`. Supplied principals, App identity,
hashes, epochs, build IDs and limits remain out of the public error. The action
is to reject the fresh init and reinstall from one corrected immutable plan;
unchanged bytes are never retried as a new activation.

## Protected Record And Transitions

| Candidate label | Current source | Class/origin | Action and retry |
| --- | --- | --- | --- |
| `FLEET_ACTIVATION_RECORD_ENCODE_FAILED` | `Encode(String)` | `Invariant` / stable activation record | Stop mutation; fix serialization/runtime code |
| `FLEET_ACTIVATION_RECORD_CAPACITY_EXCEEDED` | `RecordTooLarge` | `ResourceExhausted` / stable record bound | Reduce bounded activation evidence before retry |
| `FLEET_ACTIVATION_ALREADY_INITIALIZED` | `AlreadyInitialized` | `Conflict` / one-time initialization | Replay only the exact initialized operation |
| `FLEET_ACTIVATION_NOT_INITIALIZED` | `NotInitialized` | `Unavailable` / activation state | Complete fresh initialization first |
| `FLEET_ACTIVATION_STATE_INVALID` | every durable `InvalidRecord` reason | `Invariant` / protected activation state | Fail closed; inspect state and reinstall/restore correctly |
| `FLEET_ACTIVATION_NOT_ACTIVE` | `NotActive` | `Unavailable` / activation phase | Complete the required activation journey |
| `FLEET_ACTIVATION_IDENTITY_MISMATCH` | `IdentityMismatch` | `Conflict` / protected operation identity | Use the exact operation and credential identity |
| `FLEET_ACTIVATION_EVIDENCE_MISMATCH` | `EvidenceMismatch` | `Conflict` / immutable evidence | Replay exact evidence; never overwrite the retained value |
| `FLEET_ACTIVATION_TRANSITION_INVALID` | ordinary `InvalidTransition` phase/sequence reasons | `Conflict` / activation state machine | Inspect status and issue the exact next transition |
| `FLEET_ACTIVATION_TIMESTAMP_INVALID` | zero activation timestamp | `Invariant` / runtime clock input | Stop activation; correct the runtime time source |
| `FLEET_ACTIVATION_EVIDENCE_HASH_INVALID` | stringified activation-evidence hashing cause | `Invariant` / canonical evidence | Preserve the future typed canonicalization cause and stop mutation |

`InvalidRecord` currently contains more than thirty prose sites across the
storage facade and mapper: runtime-role disagreement, missing Store/root
authority, illegal root-only evidence on non-roots, incomplete Component
runtime evidence, Directory hash disagreement, credential-manifest
inconsistency and protected deployment mismatch. They deliberately share
`FLEET_ACTIVATION_STATE_INVALID`: all are contradictions in the same protected
record, all fail closed before returning a partial status, and all require the
same operator action. The prose reason is not recovery authority.

Most `InvalidTransition` sites likewise mean that the requested edge is not
admitted from the current durable phase, including Directory preparation,
active-only synchronization, credential generation, root/non-root cascade
mode and snapshot preparation. They share one transition identity. The zero
timestamp and stringified hashing error have different origin/action and must
be split into typed leaves before the string bucket is deleted.

`FLEET_ACTIVATION_RECORD_ENCODE_FAILED`, timestamp failure and evidence-hash
failure project publicly to `FLEET_ACTIVATION_STATE_INVALID`. The other direct
labels are safe public identities as written: they disclose no protected
value. Exact record-invalid codes must also be emitted to the approved bounded
numeric runtime observation so the masked public result remains diagnosable.

## Current Count

This pass contributes **30 exact semantic candidates**:

- eight direct fresh-admission leaves;
- eleven root-topology admission leaves; and
- eleven protected record/transition leaves.

It introduces one additional safe projection,
`FLEET_ACTIVATION_ADMISSION_INVALID`. `FLEET_ACTIVATION_STATE_INVALID` is itself
one exact record-state candidate and is reused as the public projection for
three masked internal failures; it is not counted twice.

## Required Tests

- exhaustive fresh-admission mappings and masking of every dynamic value;
- proof that only the eleven root-binding topology leaves are reachable;
- one exact record-invalid code for every current mapper/storage prose site;
- typed separation of phase conflict, zero timestamp and evidence hashing;
- no status projection from contradictory protected state;
- exact-retry tests for identity/evidence matches and conflicting retry
  rejection; and
- no transition decision based on formatted reason text.

# 0.105 B1 Released Resource Baseline

The predecessor is `v0.104.1`, peeled commit
`464c186d9d82112d1ea4c7bdb1f47bcd5e5224a5`. All byte counts use the current
CBOR serializer and maximum-length 29-byte principals. The retained focused
unit test `released_session_and_replay_cbor_footprint_is_exact` reproduces the
stable values.

| Symbol | Released predecessor result | Method and conclusion |
| --- | --- | --- |
| E | N/A for local application establishment | 0.104 has no role-owned local application-session establishment variant. Its test-only `set_delegated_session_subject` consumer is a different subject-only contract and is not a comparable baseline. |
| A | N/A for scoped local authorization | 0.104 has no synchronous `caller + scope` local application decision. Session identity resolution scans and may mutate the complete session vector, while proof-bearing authorization verifies Candid argument zero. |
| D | N/A for the closed local denial partition | The 0.105 denial enum and precedence do not exist. Current failures are split across `AccessError`, compact endpoint errors and raw-caller fallback metrics. |
| B | 176 bytes per active-session record; 163 bytes per replay-binding record | Empty `AuthStateRecord` is 257 bytes. Adding one representative record increases the full encoding by the exact direct-record size. At 2,048 sessions plus 4,096 bindings, with other auth members empty, the full encoded cell is 1,032,069 bytes. |
| H | 0 bytes of reconstructed lookup indexes | No exact-caller or subject-count index exists. The stable vectors themselves are cloned/read in heap but are canonical authority, not derived indexes. |
| R | 0 instructions attributable to index reconstruction; N/A as a comparable restore measurement | Lifecycle restores no application-session index. B3 must introduce and measure synchronous reconstruction. |
| C | Up to 6,144 removals and a full scan of 6,144 records in one call | Cleanup retains over both complete bounded vectors: up to 2,048 sessions and 4,096 bindings. There is no 128-record work cap. |
| M | 19,124,317 raw bytes across the four released 0.104 product roles | Component 3,560,554; Root 8,406,365; Coordinator 3,818,092; Wasm Store 3,339,306. B1 changes no shipped product role, so its product delta is zero. B2's first product mutation must rebuild this exact role set. |

The corresponding released gzip total is 4,959,729 bytes. It is retained for
artifact identity context but is not substituted for the design's raw-Wasm
`M` bound.

The immutable artifacts were built from `v0.104.0`. The published 0.104.1
closeout correction changed tests, test fixtures, development dependencies,
guards and documentation but no shipped product-role source or runtime
dependency, so that exact table remains the released 0.104.1 product
predecessor footprint.

## Stable-State Facts

- Cell key: `canic.core.auth.state.v1`.
- Physical memory ID: 34.
- Active-session ceiling: 2,048 per Canister and 128 per delegated subject.
- Bootstrap replay-binding ceiling: 4,096 per Canister and 256 per delegated
  subject.
- Current active record: wallet caller, delegated subject, second-based issue
  and expiry, optional token fingerprint.
- Current replay binding: wallet caller, delegated subject, fingerprint,
  second-based bind and expiry.
- Missing from both: issuer, Fleet, role, scopes, local authority generation
  and exact establishment request hash.
- Read path: linear scan and stale-record pruning, including stable mutation on
  lookup.
- Cleanup: full-vector `retain`, not bounded incremental work.
- Restore: no derived index or reconstruction step.

The 0.105 hard cut replaces these two old meanings. It does not preserve their
bytes, add a second cell, migrate them or retain raw-caller fallback.

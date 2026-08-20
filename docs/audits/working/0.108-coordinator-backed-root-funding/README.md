# Canic 0.108 B1 Attached-Cycles Recovery Evidence

Date: 2026-08-20
State: bounded PocketIC proof passes; B1 awaits maintainer acceptance

## Authority And Scope

This record supports only B1 of the accepted
[0.108 design](../../../design/0.108-coordinator-backed-root-funding/0.108-design.md).
It adds one unpublished test-only Canister and one serial PocketIC integration
target. It changes no production runtime, stable state, Candid, configuration,
timer owner, CLI, package version or remote resource.

The source base is published `v0.105.0`, commit
`b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811`, plus the active uncommitted
0.106/0.108 worktree. The observed toolchain is Rust/Cargo 1.97.1 and PocketIC
15.0.0. The exact test Wasm build fingerprint is
`de76bc03af3da9050e644acf0cb300de66d1771e5e1e324e4417cd77faac88b0`;
the 528,765-byte artifact has SHA-256
`fc69f153ffc80369c32279210eba54c0975aba6622e431718cdb2738f65ee86f`.

## Selected Platform Primitives

The Coordinator probe uses
`ic_cdk::call::Call::bounded_wait(...).with_arg(...).with_cycles(...).await`.
The exact root reads `msg_cycles_available()` and calls
`msg_cycles_accept(exact_amount)` only after caller, operation, amount and
target bindings pass. Fresh acceptance and receipt persistence occur
synchronously without an `await`. Exact replay accepts zero and returns the
prior receipt, leaving the attached principal unaccepted for automatic return
to the Coordinator.

These are test-only proof choices for the workspace's pinned `ic-cdk 0.20.2`;
B1 does not add their surrounding state machine to production.

## Interruption And Authority Matrix

| Boundary | PocketIC action | Retained authority | Result |
| --- | --- | --- | --- |
| Intent | Stop and restart the Coordinator after a separate prepare message | Exact root, operation `[0x17; 32]`, 1T grant, call cost and reservation remain in the probe heap | Prepared intent remains byte-equivalent |
| Caller | A second Coordinator attaches the same request to the root | Root is initialized with the exact first Coordinator | Foreign caller is denied, accepts zero and creates no receipt |
| Call | Stop the root before the Coordinator dispatches | Coordinator retains the prepared exact intent | Call fails without a root receipt; restart admits the same request |
| Receipt | Root accepts and commits, then the Coordinator response callback traps | Root retains the exact receipt; Coordinator remains prepared | No new operation is minted after response loss |
| Replay | Coordinator dispatches the same root/method/arguments/amount | Root receipt binds operation, Coordinator, root and amount | Root accepts zero, returns the prior receipt and Coordinator commits once |

The journey uses no production `cfg(test)` branch. The complete fixture package
is test-only, unpublished, dependency-leaf guarded and absent from shipped role
configuration.

## Dated Headroom Observation

The exact proof grant is `1,000,000,000,000` cycles.

| Quantity | PocketIC 15.0.0 observation |
| --- | ---: |
| Exact `cost_call` for the bounded root method and encoded request | 42,102,499,000 |
| Grant plus exact call reservation | 1,042,102,499,000 |
| Coordinator execution beyond the accepted fresh grant | 12,326,635 |
| Coordinator execution during zero-accept replay | 12,460,866 |
| Root execution deducted during fresh acceptance | 5,216,228 |
| Root execution deducted during replay | 5,227,404 |

The replay Coordinator spent less than the attached 1T principal and the root
balance did not increase, proving automatic return of the unaccepted replay
principal. These are one deterministic PocketIC observation, not IC-mainnet
costs or universal production thresholds.

For the next protected-policy review, B1 proposes the checked admission shape:

~~~text
exact grant
    + cost_call(exact method and encoded request)
    + 100,000,000 Coordinator execution allowance
~~~

and a separate `100,000,000` root execution allowance. Each allowance is the
smallest 100M-rounded value above the corresponding maximum observation. They
remain candidate 0.108 inputs until the maintainer accepts B1; later exact
public payload bounds must recompute `cost_call` rather than copying the 1T
fixture value.

## Focused Validation

- `cargo check -p root_funding_probe --target wasm32-unknown-unknown`: pass.
- `cargo clippy --locked -p root_funding_probe --target wasm32-unknown-unknown -- -D warnings`: pass.
- `cargo clippy --locked -p canic-tests --test pic_root_funding_recovery -- -D warnings`: pass.
- `cargo test --locked -p canic-host qualification_harness_packages_are_test_only_leaves --lib`: pass, 1 test.
- `bash scripts/ci/check-workspace-test-inventory.sh`: pass, 39 targets with 9 serial PocketIC targets.
- Governed pinned-server run of `cargo test --locked -p canic-tests --test pic_root_funding_recovery -- --nocapture --test-threads=1`: pass, 1 test.

The first direct behavioral invocation was rejected before PocketIC startup
because it intentionally omitted `CANIC_POCKET_IC_SERVER_URL`. A second
sandboxed invocation was denied loopback access. Neither reached product
behavior. The final governed pinned-server invocation above is the behavioral
result.

## B1 Disposition

The bounded proof is complete and ready for maintainer acceptance. Acceptance
would freeze the selected CDK primitives, the interruption/replay result and
the two candidate rounded execution allowances as inputs to B2. It would not
authorize 0.106 B2, any remote effect or 0.108 production/stable-state mutation
beyond the already accepted B2 sequence.

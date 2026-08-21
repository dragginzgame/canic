# Canic 0.108 Recovery, Admission And Protected Policy Evidence

Date: 2026-08-21
State: M0 accepted 2026-08-21; M1 protected-policy hard cut complete

## Authority And Scope

This record supports M0 and M1 of the accepted
[0.108 design](../../../design/0.108-coordinator-backed-root-funding/0.108-design.md).
M0 adds one unpublished test-only Canister and one serial PocketIC integration
target. M1 adds protected policy to the existing Fleet-input, plan, init,
root-authority and Registry contracts, but no runtime grant state machine,
timer, treasury ledger or public endpoint.

The source base is `main` at
`c9361036eb10c593c2db4b3c302a489ac0a50c49` plus the retained uncommitted
0.107 closeout-evidence correction and active M0 changes. The observed
toolchain is Rust/Cargo 1.97.1 and PocketIC 15.0.0. The exact test Wasm build
fingerprint after the final M1 payload-bound recomputation is
`f93c470cc9e016449c8dfb446703ee8401ef21fa8658ab17ad64b3a5514d1cc9`;
the 625,326-byte artifact has SHA-256
`692bc4facc00a9b886c08009864319ad2b59807887c44dcd4a0cc041d54479e9`.

## Selected Platform Primitives

The Coordinator probe uses
`ic_cdk::call::Call::bounded_wait(...).with_arg(...).with_cycles(...).await`.
The exact root reads `msg_cycles_available()` and calls
`msg_cycles_accept(exact_amount)` only after caller, operation, amount and
target bindings pass. Fresh acceptance and receipt persistence occur
synchronously without an `await`. Exact replay accepts zero and returns the
prior receipt, leaving the attached principal unaccepted for automatic return
to the Coordinator.

The Root also records `canister_cycle_balance()` immediately before and after
fresh acceptance. PocketIC observes an exact increase of the 1T attached
principal only after `msg_cycles_accept`, proving that the selected balance
API excludes incoming attached cycles before acceptance.

These are test-only proof choices for the workspace's pinned `ic-cdk 0.20.2`;
M0 does not add their surrounding state machine to production.

## Interruption And Authority Matrix

| Boundary | PocketIC action | Retained authority | Result |
| --- | --- | --- | --- |
| Intent | Stop and restart the Coordinator after a separate prepare message | Exact root, operation `[0x17; 32]`, 1T grant, call cost and reservation remain in the probe heap | Prepared intent remains byte-equivalent |
| Caller | A second Coordinator attaches the same request to the root | Root is initialized with the exact first Coordinator | Foreign caller is denied, accepts zero and creates no receipt |
| Call | Stop the root before the Coordinator dispatches | Coordinator retains the prepared exact intent | Call fails without a root receipt; restart admits the same request |
| Root transaction | Root accepts 1T, writes its receipt and deliberately traps before reply | The complete Root replicated message is atomic | Root retains neither the 1T balance increase nor receipt; Coordinator receives the attached principal back apart from call execution |
| Receipt | Root accepts and commits, then the Coordinator response callback traps | Root retains the exact receipt; Coordinator remains prepared | No new operation is minted after response loss |
| Replay | Coordinator dispatches the same root/method/arguments/amount | Root receipt binds operation, Coordinator, root and amount | Root accepts zero, returns the prior receipt and Coordinator commits once |

The journey uses no production `cfg(test)` branch. The complete fixture package
is test-only, unpublished, dependency-leaf guarded and absent from shipped role
configuration.

## Dated Headroom Observation

The exact proof grant is `1,000,000,000,000` cycles. The reported operating
observations below exclude the additional deliberate Root-trap call, whose
separate assertion proves its attached principal is refunded.

| Quantity | PocketIC 15.0.0 observation |
| --- | ---: |
| Final `canic_command` cost for the 16 KiB encoded-command bound | 42,118,809,000 |
| Grant plus final bounded call reservation | 1,042,118,809,000 |
| Coordinator execution beyond the accepted fresh grant | 12,341,408 |
| Coordinator execution during zero-accept replay | 12,461,694 |
| Root execution deducted during fresh acceptance | 5,217,306 |
| Root execution deducted during replay | 5,227,446 |

The replay Coordinator spent less than the attached 1T principal and the root
balance did not increase, proving automatic return of the unaccepted replay
principal. These are one deterministic PocketIC observation, not IC-mainnet
costs or universal production thresholds.

For the next protected-policy review, M0 retains the checked admission shape:

~~~text
exact grant
    + cost_call(exact method and encoded request)
    + 100,000,000 Coordinator execution allowance
~~~

and a separate `100,000,000` Root request/retry execution allowance. Each
allowance is the smallest 100M-rounded value above the corresponding maximum
attached-cycles observation. M1 recomputed `cost_call` for the existing public
`canic_command` method and a conservative 16 KiB encoded funding-command
bound. M2 must assert the final command DTO's maximum encoding fits that bound.
This does not replace the distinct automatic ICP-refill execution/recovery
floor recorded below.

## Root Admission Floors

The expanded probe executes the actual nested Root-to-Coordinator-to-same-Root
shape. The final bounded Root outbound `canic_command` request reserves
`42,118,809,000` cycles. Fresh Root request execution beyond the returned 1T
grant is `10,859,260` cycles; terminal retry execution is `5,659,729` cycles.
The smallest 100M-rounded value above the maximum call reservation plus
observed execution remains `42,200,000,000` cycles, which M1 freezes as the
minimum `request_threshold`.

The emergency probe follows fee, decimals and ICP/XDR-rate acquisition, then
deliberately loses the ledger-transfer response, recovers the exact duplicate
block, deliberately loses the CMC-notify response, replays the exact notify
and commits the terminal result. Its maximum exact call reservation is
`42,102,599,000` cycles and the complete failure/recovery journey consumes
`58,420,824` cycles. The corresponding 100M-rounded automatic-refill
execution/recovery floor is `42,200,000,000` cycles.

These values are dated fixture measurements, not universal IC-mainnet costs.
M1's final method/bound recomputation keeps both validation constants at
`42,200,000,000`; they may not be lowered without an equal or stronger
PocketIC result.

## M1 Protected Policy Hard Cut

Fleet-input schema 1 now requires `coordinator.root_funding` whenever roots
exist and requires `root_funding` for every root. Optional per-root
`icp_refill` and nested `automatic` policy are protected Fleet input only.
Unknown fields and any attempt to place the policy in `canic.toml` fail
closed. One shared model validator owns positive fields, reserve/request/
emergency floors, target ordering, per-root and Fleet grant feasibility, ICP
caps, reserved system Principals and explicit IC-override acknowledgement.

The host propagates exact typed policy through the canonical plan and digest,
Coordinator init/install journal, root install journal and activation
authority. Coordinator genesis and root activation independently validate
their protected copies before persistence. Registry publication, canonical
Registry hashing and mirror comparison include the complete root authority;
policy drift therefore blocks instead of being repaired from a request.

Canonical policy hashes use domain-separated, explicit field encodings. The
accepted fixtures are:

- Coordinator policy:
  `26a8f270734f672e87b68c6e6ca8b98df1001d7dd7cea985c979a6bdf4963618`;
- full root funding policy:
  `83eabbea6076f289519e734faf021f134ce7e4e028b16ddcff5af64f1dc6f40c`;
  and
- complete Registry manifest with the root policy:
  `88d950e6a112c8bea333bf69b8e40afb675024895654723a8def99945385fa95`.

Changing any decision-bearing Coordinator/root policy field changes its
policy hash and the complete plan/Registry identity. The generic
`CanisterConfig.icp_refill` model, renderer and application-config authority
are deleted without an alias. The existing manual refill workflow now reads
only validated root activation authority.

The canonical Coordinator Candid was regenerated deterministically. Its only
M1 expansion is the intended optional Coordinator init policy plus root
binding/Registry policy types. Runtime-funding variants occur only as nested
authority data; no new public method or command/status selector exists yet.

## Bounded Receipt Decision

The design now rejects a generalized receipt collection. Each root retains
one current operation and one last exact terminal result. A checked monotonic
sequence is part of the operation identity: exact current/last replay returns
the retained result, stale or skipped sequences reject, and exact N+1 arrival
acknowledges N before replacement. At most one Coordinator call attempt for a
root may be outstanding. Under those invariants there is no delayed N call to
service after N+1 is admitted and no need for a replay horizon, purge timer or
separate cross-Canister acknowledgement.

The retained pure counterexample test passes fresh sequence 1, exact current
retry, conflicting operation-ID reuse, exact terminal replay, skipped sequence
rejection, exact successor replacement and stale predecessor rejection.

## Offline Break-Glass Authority

M0 fixes the offline authority chain to the canonical-network Fleet catalog
row plus the digest-bound Fleet install plan and canonical per-placement Root
install journals. The existing catalog test proves local resolution preserves
the exact Coordinator and refuses to reinterpret the Coordinator-anchored
Fleet as the removed single-root topology. The new journal test reloads the
canonical host file and derives the exact Coordinator and Root Principals from
its protected authority without invoking ICP, Coordinator or Root transport.
M6 must compose those validated sources; live calls may corroborate but can
never be required for target selection.

## Focused Validation

Newly executed on 2026-08-21 for M0:

- `cargo check --locked -p root_funding_probe --target wasm32-unknown-unknown`: pass.
- `cargo clippy --locked -p root_funding_probe --target wasm32-unknown-unknown -- -D warnings`: pass.
- `cargo clippy --locked -p canic-tests --test pic_root_funding_recovery -- -D warnings`: pass.
- Governed pinned-server run of `cargo test --locked -p canic-tests --test pic_root_funding_recovery -- --nocapture --test-threads=1`: pass, 4 tests covering both atomicity cases, pre-acceptance balance semantics, current/last sequence bounds, nested Root request/retry costs and the complete emergency failure/recovery floor.
- `cargo test --locked -p canic-host durable_root_journal_resolves_break_glass_authority_without_target_calls --lib`: pass, 1 test.
- `cargo test --locked -p canic-host coordinator_catalog_rejects_the_removed_single_root_topology_resolver --lib`: pass, 1 test.
- `cargo test --locked -p canic-host qualification_harness_packages_are_test_only_leaves --lib`: pass, 1 test.
- `cargo clippy --locked -p canic-host --lib -- -D warnings`: pass.
- `bash scripts/ci/check-workspace-test-inventory.sh`: pass, 39 targets with 9 serial PocketIC targets.
- `make current-document-semantics-gate`: pass.
- `cargo test --locked -p canic --test changelog_governance`: pass, 1 test.
- `git diff --check`: pass.

Newly executed on 2026-08-21 for the final M1 bound and protected policy:

- Governed pinned-server run of `cargo test --locked -p canic-tests --test pic_root_funding_recovery -- --nocapture --test-threads=1`: pass, 4 tests; exact 16 KiB `canic_command` reservation is `42,118,809,000` cycles and both frozen floors remain `42,200,000,000` cycles.
- `cargo test --locked -p canic-host fleet_install_input --lib`: pass, 23 tests covering exact policy propagation and strict invalid-policy rejection.
- Focused core policy/hash, activation, Registry and plan-digest tests: pass.
- Focused Coordinator genesis and Fleet-budget admission tests: pass.
- `cargo test --locked -p canic-cli --no-run`: pass.
- Canonical Coordinator Candid regeneration: pass; the checked-in surface contains only the intended protected data expansion and no funding endpoint.

Final combined-main reconciliation on 2026-08-21:

- Affected core, control-plane, host, CLI, testing and protocol packages pass
  locked compilation and warning-denied Clippy; the root-funding probe also
  passes warning-denied Wasm Clippy.
- Focused funding policy, memory ownership, role-contract, Coordinator genesis,
  Fleet-input/plan, activation-journal, provisioning-identity, finalized-Candid
  and generated-protocol tests pass.
- Formatting, workspace test-inventory, current-document semantics, changelog
  governance and whitespace checks pass. The prior governed PocketIC evidence
  is retained; the downstream fresh Toko installation remains the explicit
  end-to-end confirmation.

The first 2026-08-21 pinned server start was denied a sandbox loopback bind and
reached no product behavior. The approved local-only server and targeted test
above are the behavioral result.

## M0 Disposition

The maintainer accepted complete M0 on 2026-08-21. It freezes the
selected CDK primitives, both separate transaction boundaries, nested request
and emergency execution-floor methods, fixed reservation-time windows,
monotonic current/last-result retention, the single Draining funding fence and
offline break-glass authority as inputs to production design.

M1 is complete: strict protected policy, validation, canonical hashing,
plan/init/root/Registry propagation and generic refill removal are present,
with no grant state machine or new public endpoint. Ordinary continuation may
proceed to M2. Neither M0 nor M1 authorizes a 0.106 B2 effect, remote mutation,
versioning or publication.

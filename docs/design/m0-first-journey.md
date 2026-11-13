# M0 first journey decisions and evidence

Status: M0 in progress, 2026-11-13. The [architecture](canic-system-design.md)
is accepted; local creation and direct-cycle funding commit-boundary experiments
pass. The M1 CLI now exercises retained signed host attempts, deployment and
same-canister upgrade locally. Mainnet creation, installation and greeting have
passed. A local infrastructure-owner maintenance window now preserves both effect
journals across a changed executable and failed upgrades. A bounded artifact
fixture also installs a retained worker module without the operator. Mainnet
upgrade, production owner persistence and adoption measurements remain unqualified.
This record contains scoped decisions and evidence, not a second implementation
backlog. The first product is the [M1 Hello World loop](../../README.md), with the
mainnet upgrade prepared for the operator's local identity unlock.

## First capability and consumer invariant

M1 exercises build, reviewed installation, invocation and status of one greeting
canister, followed by a changed executable with explicit preservation semantics.
It needs no database, browser frontend, Fleet admission or autonomous scaling.

The proposed M2 capability is managed installation and readiness of a shared
stateful worker, followed in M3 by a qualified maintenance-window upgrade.
The application retains user registration, routing, database schema and resource
authorization. This is a narrow adoption proposal, not a full Toko shard port.

The first multi-canister demo also requires a runnable browser fleet view,
delivered with that increment and no later than M2. The team must be able to
inspect deployed canisters, their relationships, observed state and interrupted
operations. The [visualization acceptance contract](canic-system-design.md#demo-fleet-visualization-from-the-first-multi-canister-increment)
owns the scope; M3 extends the same view to autonomous workers. This is planned
work and is not part of the currently implemented single-canister CLI.

The design also admits an optional `ic-relay` module alongside direct platform
calls. Its [managed pilot and comparison](canic-system-design.md#direct-calls-and-optional-ic-relay)
use two demo canisters and one bounded application command once that journey is
available. Select a compatible supplier revision and measure batching gains,
latency, storage and recovery against the direct path before adoption. Relay's
existing supplier experiments do not qualify this rewrite. This choice neither
adds a Relay dependency to M1 nor expands the M0 closure conditions.

The candidate invariant is concrete: an application-supplied user ID remains
unchanged; its wallet and delegation principal mappings resolve to that same
user; private-record reads by another user remain rejected. The upgrade witness
also preserves a profile value written before the transition. Canic must not
acquire ownership of these records or reinterpret their routing keys.

Read-only inspection of Toko at commit
`44d4e2c6d41e3575b2503e79ffa369129ff3ecf2` identifies these source inputs:

- `fleets/toko/user/shard/src/lib.rs`: explicit IcyDB lifecycle participation,
  registration endpoints, session resolution and private-record ownership check.
- `fleets/toko/user/shard/src/ops/user.rs`: supplied-ID registration, principal
  mappings, profile persistence and registration retry tests.
- `fleets/toko/user/shard/src/tests/ownership_unit_tests.rs`: owner and foreign-user
  cases for the private-record check.
- `fleets/toko/user/shard/Cargo.toml`: Canic/IcyDB runtime and build dependencies.

The inspected source files match that commit. The working manifest additionally
sets `publish = false`, and the working lockfile is modified. These observations
establish an invariant candidate, not a build baseline or a passing test result.
Pin the agreed consumer manifest, lockfile, toolchain and selected dependency
graph before measuring adoption. Toko source changes remain outside this scope.

## Topology candidate and retained responsibilities

Use R → E → application as the first experiment candidate. R is the operator
principal; E is one subnet-local management owner. Compare direct-host bootstrap
for its authority and cost, not as a substitute for operator-absent execution.
Select the product arrangement after the custody experiment establishes its limits.

E retains intent, reservations, exact effect identities and committed outcomes.
R retains E's maintenance operation and the inventory/exposure needed to recover
it outside E. Application code cannot clear E's unresolved obligations.
For autonomous installation, first evaluate a bounded set of approved artifact
bytes retained by E before granting growth authority; unplugging the host must
not remove those bytes. No Coordinator or separate Store enters this experiment.
Artifact size limits and storage costs must be measured before selecting that path.

## First effect experiment

The [runnable fixture](../../qualification/README.md) exercises actual management
creation and direct-cycle funding with explicit trap injection. It introduces no
product operation engine or configuration language.

| Boundary | Evidence the experiment must establish |
| --- | --- |
| Host submission | Exact signed-attempt bytes/identity survive process interruption before submission; restart does not invent another attempt |
| Executor dispatch | Intent, reservation and outgoing-call enqueueing commit together; a trap before return produces no external call from that execution |
| Returned result | Creation principal or funding completion/refund evidence survives a committed callback; writing then trapping in that callback is a separate negative case |
| Subsequent work | A later message can fail without erasing the committed outcome; reconciliation uses that outcome without repeating the paid effect |
| Maintenance | R can stop/upgrade/recover E while keeping its operation and exposure outside the affected canister; governance is investigated only if selected |

The [system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/)
defines the dispatch commit boundary; the
[execution properties](https://docs.internetcomputer.org/references/message-execution-properties/)
define rollback within a trapping callback. These are specification evidence,
not results for a selected executor implementation. Rust `await` syntax alone
does not establish a new message boundary.

Classify each injected window as proven no effect, proven effect with retained
evidence, reconstructible uncertainty, or blocked uncertainty within a stated
bound. Harness inventory may establish what happened in a test, but cannot be
used as a production recovery mechanism unavailable to R or E. Never infer the
created principal from inventory similarity or payment identity from balances.

### Creation results on 2026-11-13

`make m0-test` passed on Linux x86-64 with Rust 1.100.0, PocketIC 16.1.0,
Testkit 0.102.0, Platform 0.32.1, Runtime 0.22.1 and Wire 0.22.0. The lockfile
records the complete selected graph. The maximum-evidence codec check and all
five installed scenarios passed; native/Wasm Clippy, Rustdoc, shell and tooling
agreement checks also passed.

| Injected window | New canisters | Committed executor evidence | Outcome |
| --- | --- | --- | --- |
| After enqueue, before dispatch commit | 0 | Empty slot; no reservation | Proven no creation effect; execution still costs cycles |
| Before result write | 1 | Issued intent, no principal, 2T cycles exposure | Blocked uncertainty; another creation is refused |
| After result write, in the same callback | 1 | Issued intent, no principal, 2T cycles exposure | Blocked uncertainty; writing the record did not commit it |
| Separate continuation after result commit | 1 | Principal/refund retained; continuation write rolled back | Original outcome reconstructed by reading committed evidence; replay creates nothing |
| No fault | 1 | Principal/refund retained; child controlled by executor | Proven effect with retained evidence; replay creates nothing |

The implementation consequence is to keep irreplaceable-result capture small and
commit it before fallible subsequent work. This does not eliminate traps before
that commit. A single uncertain attempt blocks this fixture's creation capacity;
product acceptance of that blocked exposure and its disposition remain decisions.
No claim of automatic recovery for every creation window is justified.

### Direct-cycle funding results on 2026-11-13

The same `make m0-test` run passed all ten funding interruption cases: five
windows against each of a live empty recipient and a deleted recipient on the
same subnet. Each fresh executor attaches 1T synthetic cycles. The
[management API](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/)
defines `deposit_cycles`; the selected path uses an unbounded call and captures
the matching callback refund before further work. No ledger is involved.

| Injected window | Live recipient | Deleted recipient | Committed executor evidence |
| --- | --- | --- | --- |
| After enqueue, before dispatch commit | No credit | No deposit dispatched | Empty slot, zero reservation; proven no deposit effect |
| Before result write | Credited 1T | Attachment refunded | Issued intent, no receipt or refund observation, full 1T exposure blocked |
| After result write, in the same callback | Credited 1T | Attachment refunded | Same blocked uncertainty; uncommitted writes were discarded |
| Separate continuation after result commit | Credited 1T | Attachment refunded | Completion/zero refund, or rejection code 3/full 1T refund retained; zero unresolved exposure |
| No fault | Credited 1T | Attachment refunded | Same retained outcomes; identical replay returns evidence without another deposit |

For dispatch traps, repetition still produces no credit. For every occupied
slot, changed amount and changed destination are refused. Admission tests also
reject zero/over-limit amounts, oversized principal bytes and non-controller
callers without consuming the slot, then successfully admit one valid deposit.
Both maximum-evidence Wire codec checks pass under the original finite limits.
Native/Wasm Clippy, Rustdoc, formatting and shell checks pass.

Recipient balances and executor balance decreases are independent test oracles,
not inputs to recovery. In the deleted-recipient callback-trap cases, observed
executor decreases were 13,337,427 and 13,337,441 cycles respectively, rather than
the 1T attachment. This establishes a physical refund in this fixture but supplies
no committed operation receipt to the executor. It therefore keeps 1T reserved
and refuses replay. A reject code alone would not release exposure: the probe
retains any unrefunded portion even when a rejection record commits.

The bounded result is deliberately conservative: no callback-loss window was
reconstructed from balances. A retained receipt survives later work; absent that
receipt, a single uncertain attempt blocks the funding slot. Product acceptance
and disposition of this bound remain open. Local dispatch-resource failures,
cross-subnet deposits and upgrade persistence have not been qualified here.

### Qualified inputs

Creation was rerun successfully after sharing its dispatch injection with the
funding probe. SHA-256 digests below identify that combined passing run; paths
are relative to the repository:

```text
6a0f87ca7a9b37a369e6a8f8fd020da8de0268c0266767b4022121a57638f709  qualification/Cargo.lock
5d2801ce434ba03e75d53acf8944eaa9678f8850d3b92984f73d389c777253d8  qualification/m0-probe/src/lib.rs
add2c1182af05ae42107016b8d799e755e07dafee88dce6d6b48a3dcf9da81da  qualification/m0-probe/src/canister/creation.rs
b943d5b8079997ce47bbd8ccde6c17f474c35df0a4f2fe386c874941c08f4cc2  qualification/m0-probe/src/canister/funding.rs
e92fcc8110ade8fbf27ce0389108c019c7a87423662ce9c37045709afd942d8a  qualification/m0-probe/src/canister/mod.rs
6cdbc9aeb059bc5d1bdb1f8fa0d20c757bc2d676e415c1a6735873b9e08765ae  qualification/m0-probe/src/tests.rs
097984a2af24b618afda29c1b47055874360255266ba2c966a72b93af0c77cb9  qualification/m0-tests/tests/creation.rs
b742937cc45f496e7a48a0013430dce579e10b7afeebd19ab4f63868c5dfc48b  qualification/m0-tests/tests/funding.rs
18f497e7f4d307e816dd4e2c074f29e3640f699c5537f27bccfba27bfad9fcde  qualification/m0-tests/tests/support/mod.rs
bc1737fc22075d576fe26887b098052b7412bcbe72d5958a15498022e44bbdde  qualification/target/wasm32-unknown-unknown/release/canic_m0_probe.wasm
b44e1eccd66e02328146b3e209e3403e3a3428a25d25555aca2f67500e5ae9db  target/m0/server/16.1.0-x86_64-linux/pocket-ic
```

### Owner-maintenance results on 2026-11-13

`make m0-test` passed again with the maintenance hooks, on Linux x86-64 with
Rust 1.100.0, PocketIC 16.1.0, Testkit 0.102.0, Platform 0.32.2, Runtime 0.22.2
and Wire 0.22.1 from the current qualification lockfile. This includes the original
five creation cases, ten funding cases, funding admission and both bounded codec
checks. `make m0-maintenance-test` selects the new rehearsal alone.
Native/Wasm Clippy, Rustdoc, formatting and shell checks also passed. Native
macOS execution remains unqualified.

Six maintenance cases cover empty journals after dispatch traps, retained
creation/deposit successes, lost callback evidence, retained evidence after later
failure, a retained funding rejection/refund and a rejection whose callback
evidence was lost. Each case establishes R → E and, when its creation result is
retained, E → child. R is a distinct simulated operator principal; an unrelated
principal cannot upgrade E.

The harness retains original evidence outside E, stops E, and attempts a changed
executable whose post-upgrade hook restores the journals, overwrites stable bytes
and traps. The old module, stable bytes and both journals survive unchanged. R
can restart E, then stop and successfully upgrade it to the changed executable.
Repeating the failing upgrade with populated stable memory also preserves the
running candidate and its evidence. These observations agree with the atomic
upgrade contract in the [system API specification](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#canister-upgrades).

Replaying the original commands after maintenance creates no extra canister or
deposit. Lost outcomes still reserve the full 2T creation and 1T deposit exposure,
even when the test oracle observed a refund. Retained outcomes remain readable;
the upgrade does not turn uncertainty into another allowance.

The test owns one raw 64 KiB stable page containing two independently bounded
Wire frames. It reuses the existing journal schemas and limits. This is an
isolated persistence experiment, not a production Storage integration. Host
observations are retained in harness memory, so the result does not qualify a
durable R journal or host crash recovery. It also excludes callbacks still in
flight, schema migration, recovery from an unconditionally trapping pre-upgrade
hook, governance and mainnet executor maintenance. Those limits prevent this
result from closing the entire infrastructure-recovery requirement.

The qualification lockfile SHA-256 is
`9d196c90c9f4ed0dbc5b95b2cf8d218da7c330d074f6d90c94424ce58676285a`.
The two separately retained executables are `target/m0/original.wasm`
(`bd1624f19e1f569f7267b983d5d22fd1b300617ebdce7ac8c80e24fecb180094`)
and `target/m0/maintenance-candidate.wasm`
(`8e91ccfef31e28fd0074effc2ef88f243a1986c3491550a4a5c524897e6c558c`).
The maintenance source SHA-256 is
`e70078f9cc972fad74b8c93f2bfc1c4f001fc4493a969bc45877a5c571eed12d`;
its installed test is
`1ed73914f578c3722625c0507841a8ce40044eea1d29e7ba5371a00f91bcae90`.

### Artifact-custody results on 2026-11-13

`make m0-artifact-test` passed two native admission/codec checks and four installed
scenarios on Linux x86-64, Rust 1.100.0 and PocketIC 16.1.0. The selected suppliers
are Platform 0.32.2, Runtime 0.22.2, Wire 0.22.1, Testkit 0.102.0 and SHA-256
implementation `sha2` 0.11.1. Native/Wasm Clippy, Rustdoc and shell checks passed.

The fixture retains one approved raw Wasm, exact hash and target before a future
dispatch time. A failed upgrade preserves the approval; a successful upgrade
restores it into a changed executable. Advancing only the simulator clock and
rounds then installs the actual Hello World worker, which returns its greeting.
No operator update or host artifact delivery occurs during that interval.

Platform's released direct `InstallCodeArgs` export is native-only. The qualified
on-chain alternative uses `upload_chunk` into E's own management chunk store,
then `install_chunked_code` with the approved hash and target, following the
[management API](https://docs.internetcomputer.org/references/management-canister/#install_chunked_code).
There is one chunk and no separate Store canister. Each request commitment is
retained before dispatch; an unknown result does not create another attempt.

| Case | Retained evidence and disposition |
| --- | --- |
| Successful autonomous installation | One upload and one install; callback retained; module hash and sole executor controller verified |
| Upload callback writes then traps | One upload, zero installs; original upload commitment remains blocked; physical chunk presence is only a test oracle |
| Install callback writes then traps | One install, no committed success reply; after executor maintenance, module/controller observation reconstructs the installed binding without reinstalling |
| Target already has code | Installation rejected; original code/version retained; approval cannot be replaced to retry |

All cases also reject unauthorized staging, oversized upload frames, wrong hashes
and changed approvals. Replaying the original approval never repeats an upload
or install. Worker versions independently prove no reinstall; observing the
installed binding does not manufacture a missing callback receipt. These claims
assume the selected sole executor and its operator do not independently replace
the worker; application readiness remains a separate future obligation.

The rebuilt worker is 43,632 uncompressed bytes; its upload frame is 43,699 bytes.
At the observed post-install point E uses 1,441,792 bytes of Wasm heap, 65,536
bytes of stable memory and 1,048,576 bytes of IC-managed chunk storage. These are
total fixture measurements, not incremental attribution or a production budget.
The 65,536-byte artifact ceiling fits a 65,619-byte upload and a 65,758-byte
maximum-evidence checkpoint, requiring two stable pages. Chunk storage is retained
after this experiment; cleanup and retention economics have not been qualified.
In the successful 61-second simulated interval, E's balance fell 374,600,091
cycles and the worker's 136,804,576 cycles. These include fixture execution,
storage time and observations; they are not installation-only costs or mainnet
prices. The heartbeat is test scheduling, not the product's idle-cost policy.

This supports executor-local custody for the bounded first autonomous experiment.
The host-only path remains simpler for M1 but requires the operator to supply
installation. A separate Store would add a canister without a demonstrated need
for this one-artifact workload; its costs and benefits have not been measured.
Product selection still needs governed storage, demand-driven scheduling and the
stateful consumer's actual artifact budget. This fixture pre-creates the worker
and does not yet compose creation, installation and readiness into the M2 path.

SHA-256 inputs: qualification lockfile
`a14cd0d337142a9dc550e4fa3c2e5f9c14de6eabca99164523de3071e52b4bf9`;
executor A `3ebb48eecbc840651390fcdb727a88f37e0726127753193dc43ee7ea994986bd`;
executor B `b4fc51bb87a334050c61c97d23b8b1657db7630437eea66aec83f6fa63d59d60`;
worker `0ac16f95cca67500fc8f6615012365cd81fcc09bde253035cde783476e89635f`.
The executables are retained separately under `target/m0/artifact/`. The earlier
digests identify their own qualification runs, not this expanded lockfile.

## First CLI journey on 2026-11-13

The direct-host arrangement is selected for the stateless M1 greeting: one
operator controls one application canister. Host retains the approval, artifact,
signed attempts and returned results in `.canic/`; ICP CLI retains key custody.
This does not select the enduring owner needed for M2/M3 autonomous work.

`make m1-test` passed on Linux x86-64 with Rust 1.100.0, PocketIC 16.1.0 and
repository-prepared ICP CLI 1.7.0. Eight host checks cover plan/artifact admission,
writer exclusion, immutable retained bytes, external creation DTOs, signed
request/status binding and refusal to replace expired uncertain attempts.
The installed test invokes the actual CLI process to deploy executable A, call
`Hello from Canic, team!`, recover its receipt without a signer, and upgrade the
same principal to executable B returning `Hello again from Canic, team!`.

The harness also removes the saved creation reply, and later the upgrade reply,
to model result loss after network completion. Apply recovers using the original
signed requests, without a signer, unchanged signed bytes or an extra canister.
These cases qualify resubmission within the original validity window. They do
not emulate host power failure; synchronized file/parent publication belongs to
Host FS. Expired unresolved attempts remain blocked. Deleting the only real
deployment journal is never a recovery instruction.

Native/Wasm Clippy, library Rustdoc, formatting, shell and tooling agreement
checks passed. The demo's Wasm dependency graph has no Host, Agent, CLI or simulator
dependency. Qualification invokes the CLI process so PocketIC's older pinned
`thiserror` does not force a downgrade of the product graph.

An anonymous mainnet `icrc1_fee` query returned 100,000,000 cycles, matching the
[provider contract](https://github.com/dfinity/cycles-ledger/blob/main/README.md).
The operator subsequently completed ledger/CMC creation and mainnet installation
through this CLI. Native macOS execution remains unqualified. This path assumes
the ledger fee remains unchanged
between preflight and submission, and one coordinated operator uses the retained
state directory. Cross-host fencing and concurrent external upgrades are not
qualified. The stateless demo's upgrade proves principal/controller preservation,
not the M3 application-data or infrastructure-journal promise.

The preceding M0 input digests identify their original combined probe run. The
qualification lockfile now also includes the M1 process harness; that addition
does not replace the older evidence's source identity.

### Mainnet observation on 2026-11-13

The operator applied operation
`50dea196b16ff449b86f0000fde720971745b50f8b1c349bdc3e0dc7cdb1f786`,
creating `z4fqh-uqaaa-aaaan-q6yrq-cai` under the sole controller
`o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe`.
The retained creation and installation replies and terminal receipt are present.
An independent completed-apply invocation with no available signer revalidated
the certified controller and module hash
`cdb19d3a2a115eea571730fe9598af5b0319e9aa1b3545f9c595cb3ac19dd42b`.
A fresh, explicitly uncertified greeting query returned
`Hello from Canic, team!` at approximately 17:04 UTC.

The next executable's hash is
`187f6d99668f93609c8adc72a800fb6c0ca1f8ff2d85147f22497137f0f7832c`.
Its prepared upgrade binds the same canister and exact previous module, with
zero creation funding. Mainnet execution of that upgrade remains pending; the
local same-canister rehearsal already passed. Authority-bearing signed files
remain in `.canic/`, outside Git; these observations are not a backup of them.

## Bounds and preservation witness

Start with one executor, one application and at most one unresolved paid attempt;
an unresolved creation pauses new creation instead of replenishing its allowance.
Use synthetic local cycles first. Record attached-cycle ceilings, protected
recovery reserve and observed fees separately. The operator restored
`canic-mainnet`; the first proposed deployment budgets 1T cycles from its default
cycles-ledger account. Approval is an explicit apply of the reviewed digest and
requires local identity unlock. Local results do not authorize a live debit.

Measure the Hello World setup and operator steps first. Before M2, measure the
selected stateful consumer's current lifecycle glue, endpoint/memory/timer owners,
dependencies, total Wasm and code-section sizes, initial stable memory and idle
work. Record numeric acceptance budgets before implementing that adoption slice;
no footprint or cost advantage has yet been measured.

For M3, executable B changes observable application behavior while preserving
the user ID, principal mappings, profile and private-read rule from executable A.
Across the transition, an interrupted management operation retains its original
identity, evidence and outstanding exposure at its surviving owner; B cannot
turn it into a new allowance. Separate this from the experiment upgrading E:
application preservation does not prove infrastructure-owner recovery.

M0 closes on the bounded effect results, selected topology/custody/artifact path,
agreed consumer scope and baseline, numeric budgets and this concrete preservation
scenario. A documented blocked window is acceptable only with a bounded exposure
and an explicit disposition; it is not automatic recovery. The mainnet CLI
upgrade remains ready for local identity unlock. The next M0 decisions concern
the consumer baseline and numeric budgets, and the product custody/scheduling
path beyond these bounded maintenance and artifact experiments. These
results alone do not close M0.

# Retained Fleet feedback corrections

Date: 2026-09-07. Scope: maintainer-selected CANIC-143, CANIC-144 and the
CANIC-125 follow-up, extending the open 0.110.10 draft after CR1.

## Contract and ownership

- CANIC-143: the existing native funding action records exact Root and pool
  lifecycle authority. Ready assets require empty modules; reviewed Failed and
  PendingReset imports can retain modules until their separately journalled
  reset. The adapter verifies pool membership, lifecycle and sole-Root control.
  Replanning compares that authority and funding sufficiency before effects.
  The existing withdrawal intent and receipt retain lost-response ownership.
- CANIC-144: Root Start/reinstall prerequisite policy receives the existing
  current Fleet state. Symbolic configuration resolves through its retained
  Principal; conflicting configured identities, wrong Fleet binding and live
  identity/controller/Subnet drift reject. Existing release comparison still
  selects explicit reviewed reinstall.
- CANIC-125: Component provisioning waits include recursively required initial
  children from exact compiled Component Spec identities. The automatic floor
  remains capped at 64 observations and the configured floor remains explicit.
  Provisioning burn reservation covers that selected bound. Future descendant
  capacity does not multiply waits or terminal inventory work. The existing
  pacing and same-operation retry owners remain unchanged.

The funding action JSON now uses `pool_funding.root` and
`pool_funding.lifecycle`, with null for ordinary non-pool funding. This is a
schema-1 hard cut, with no old-record decoder. No runtime Candid change, new
command, application retry owner, custody mechanism or cross-release migration
is introduced. CANIC-141 remains deferred.

## Focused evidence

| Boundary | Result |
| --- | --- |
| Fleet host tests | 150 passed; two pre-existing ignored tests not selected |
| Changed host and CLI all-target/all-feature Clippy | Passed |
| Final internal-fixture all-target/all-feature Clippy | Passed |
| Installed underfunded imports, controller rejection, lost withdrawal/reset replies | Passed in 310.66s, plus 50.26s native compilation; conservation and both effect-free replays pass |
| Generated symbolic-seed restart through reinstall and replay | Passed in 393.03s; controller drift, lost install reply, working Fleet, conservation and both effect-free replays |
| Formatting and current document semantics | Passed; 92 changed Rust files and no document advisories |
| Final changelog regression and diff checks | Passed |

Logs use `/tmp/canic-feedback-` names. Host regressions cover lifecycle
inspection, current funding serialization, retained symbolic Root identity,
recursive initial-child demand, spec-hash rejection, the wait cap, continuation
authority and paced command issuance without duplication.

The existing two-import recovery proof now installs inert real Wasm before
reset, preserving both a Failed asset and an installed PendingReset asset. It
also introduces controller drift on the second target before funding to prove
rejection before any debit, then loses the first withdrawal reply before
continuing the second import. The existing generated-reinstall proof now starts
with generated fresh authority and keeps its real terminal journal and symbolic
seed for regeneration. A repeated source-link setup failure was corrected in
that fixture before the passing rerun; no production source changed after the
passing funding proof. No duplicate long deployment case is added.

## Readiness and downstream boundary

RF1 is complete. The combined canonical Root and feedback batch is ready for
release approval, with no remaining implementation or focused-test blocker.
The open 0.110.10 changelog covers both. Package versions remain 0.110.9.
The maintainer-selected release gate and publication remain. No broad gate,
Git mutation, publication, live deployment or sibling edit is included.

After adoption, Toko Miner must qualify ordinary generated retained-import
convergence and preserved local restart against its own exact environment and
identities. Mainnet timing remains environmental: exhausting the bounded wait
still leaves the exact operation resumable. No downstream live success is
claimed by this Canic qualification.

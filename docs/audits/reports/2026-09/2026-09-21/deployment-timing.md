# Deployment timing and preparation qualification

Scope: the maintainer explicitly includes deployment timing and performance in
`.35`; the metric-history layout remains parked. This report covers Canic-owned
instrumentation and controlled local qualification. It does not close Toko's
live staging acceptance or establish a new mainnet deployment time.

## Implemented behavior

Normal Fleet Ensure review/apply and generation retain one private, bounded
JSONL receipt under `.canic/diagnostics/fleet/`. The existing progress owner
prints its path before measured work. JSON output emits a structured path event.
The receipt carries UTC Unix timestamps, monotonic elapsed time, exact available
input/operation/plan bindings, existing progress DTOs, stage boundaries and typed
request purposes/targets/methods. It records no Candid arguments, responses,
identity passwords or exported keys. The existing human display stays compact.

Stages distinguish preparation, submission, independent submission,
reconciliation and pacing. A start without its matching end remains incomplete.
Stage and request parent IDs distinguish inclusive nested work. Request overlap
counts outer request lifetimes; nested identity work does not invent extra
workers. Identity lookup wall time includes process startup and local IPC.
Install, controller changes, starts, stops and deletion join request attribution.

A repeated poll, local animation or increased waiting time does not establish
remote advancement. Progress receipts separately identify increases in applied
receipts or observed provisioning counts within the same operation and plan.
Workflow completion retains scope and terminal status; it is not deployment
proof. Errors retain available progress bindings. Successor review guidance keeps
the exact environment, executable, desired input and explicit signing identity.
Originating retry target/stage/category are displayed when the existing protected
DTO provides them; otherwise the missing origin is explicit. Canic does not
invent the runtime owner's next retry deadline.

Each receipt is capped at 8 MiB, with a 64 KiB per-event limit and reserved space
for its final outcome and omitted-event count. Unix files are created with 0600
permissions and never replace an existing path. Complete callback records are
written directly; successful finalization syncs the file. A killed process can
leave a partial final line. Missing final outcome, omitted events, or diagnostic
I/O failure mean incomplete evidence, never operation failure or success.
Continuation writes another file and preserves the interrupted one. Receipts are
informational; journals and exact reviewed authority still govern execution.

## Measured preparation improvement

Install preparation previously obtained status separately for balance and
version. It now shares one fresh response within that single pre-intent step.
The scope expires on success and failure, before intent publication or effect
submission. Each retry and subsequent preparation starts fresh. Typed management
version completion, controller admission, post-effect observation, receipts and
conservation remain intact. Other actions retain their existing preparation.

Eight alternating warm local-IPC pairs use the production adapter against the
same fake status producer and assert identical balance/version results. Requests
fall from two to one. Median step time falls from 4.463 ms to 2.229 ms. These are
small local subprocess measurements with other repository builds active on the
host; they do not establish a Fleet or mainnet speedup. The timing assertion is
on calls and returned evidence, not an absolute latency threshold.

The existing nineteen-Workload/five-Ready retained-estate PocketIC case passes
on both the control and candidate. It retains exact authority rejection, lost
withdrawal/install replies, successor convergence,
application-state checks, conservation and effect-free replay. All 24 selected
Wasm, compressed-Wasm and Candid hashes across its two release sets are identical.
The [control patch](deployment-control.patch) disables only the one-action
preparation-sharing hook; both paths retain the same diagnostic instrumentation.

| Matched measured work | Control | Candidate |
| --- | ---: | ---: |
| Initial working Fleet | 90.633 s | 92.850 s |
| Root reinstall review/authority rejection | 5.464 s | 5.054 s |
| Lost install response and recovery | 11.927 s | 11.733 s |
| Successor reviews and convergence | 167.431 s | 166.535 s |
| Retained application state and replay | 34.139 s | 32.056 s |
| Install/all-effect preparation, 59 boundaries | 16.577 s / 80 calls | 16.001 s / 77 calls |
| Independent submission, six batches | 5.924 s / 24 calls | 5.839 s / 24 calls |

The last two rows are stage attribution inside the preceding journeys, not
additional wall time. The three omitted preparation reads are replaced by three
cache hits. Both runs observe at most four concurrent outer request lifetimes.
Planning makes the same 407 inclusive logical calls in both runs; its wall time
varies from 34.312 to 32.389 seconds. The candidate performs eleven additional
pending reconciliations/backoffs, so the three-read preparation reduction is not
a claim that every invocation makes fewer total calls. No pacing, admission,
independent-effect bound or reconciliation rule changes.

This single pair establishes the specific read reduction and recovery safety,
not a material overall deployment speedup. The five comparable deployment rows
total approximately 309.594 versus 308.228 seconds, within the observed local
variation. Other repository builds were active on the host. The fixture retains
its original 25 ms observation pacing; this is not production-network latency.

Whole journeys take 655.055 and 383.895 seconds. Initial artifact preparation
alone takes 288.652 versus 29.699 seconds; replacement artifact resolution takes
39.890 versus 28.865 seconds. Both complete fixture-cache lookups miss because
host source is an input, while Cargo compilation is much warmer for the
candidate. The large whole-journey difference must not be presented as a new
cache or deployment optimization. The control/candidate runner durations are
826 and 466 seconds, respectively (targeted suite durations: 825 and 464 seconds).

[Structured measurements](deployment-timing.json) retain source/lock identities,
phase records, request-purpose/target attribution, observation counts, selected
artifact hashes and raw-log hashes. Full local logs and parsed request records
remain under `.tmp/toko35-performance/`. Measurements describe this uncommitted
working-tree checkpoint; they are not a published release-validation receipt.

## Build-reuse matrix

The opt-in real-Cargo fixture separates input lookup, a forced compiler probe
and synthetic release sealing. The forced warm compiler probe intentionally
runs even on complete-reuse hits: it measures Cargo separately and is not the
production hit path. Generated declaration/link/finalization timings for Toko's
eight real artifacts cannot be inferred from this small fixture.

- Unchanged source: complete reuse, identical input identity and Wasm.
- Relocated frozen source: complete miss, then a hit in that new location;
  output roots and absolute source authority change. The observed Wasm remains
  identical. Each location has its own mutable Cargo output directory.
- Qualification-only documentation edit within the selected package: complete
  miss, identical Wasm, retained Cargo compilation reuse. The full package tree
  is intentional authority because a build script can read non-Rust files.
- Runtime-source and dependency-source edits: complete misses and changed Wasm.

No environment exclusion, package-tree weakening or shared mutable target is
introduced to manufacture a hit. This matrix explains invalidation categories;
it is not a reconstruction of Toko's 532-second application build.

## Evidence and limits

Focused native checks pass: 387 Fleet regressions (seven opt-in cases ignored),
62 ICP-selected cases, 10 catalog cases, 51 Fleet CLI cases and the real-Cargo
matrix. These selections overlap. Three-package all-target/all-feature Clippy
passes with warnings denied. The CLI cases include an actual child-process kill,
separate continuation evidence, bounded output and exact nonterminal outcome
binding. Existing host cases retain interrupted independent batches, sibling
receipts, changed authority, conservation and effect-free replay.

Logs: `.tmp/toko35-{fleet-native,icp-native,catalog-native,cli-timing-tests,
build-matrix,timing-clippy,preparation-tests}.log`. Final presentation-only CLI
checks are retained separately as `.tmp/toko35-cli-final{,-clippy}.log`.

Remaining boundaries are explicit:

- Request elapsed time includes local startup/IPC and remote confirmation.
  Pure subprocess CPU, internal IC hops and upstream HTTP/certification retry
  costs are not individually observable through these adapters.
- Registry endpoint timings include collection and certification. Final
  acquisition completion additionally includes upstream agreement, validation
  and publication. Upstream `ic-query` retains validated history prefixes in
  memory only; Canic does not serialize private upstream state or weaken
  assurance/freshness to add cross-process resume. Existing cache-hit,
  disagreement, cancellation and exact-retry proofs pass.
- A normal Toko staging run and interrupted live continuation, using the same
  payload class and topology, remain downstream acceptance. Sibling repositories
  remain read-only. No production deployment, broad suite, version transaction,
  commit, push or publication is performed by this qualification.
- CANIC-172's fuller early native-funding readiness remains separate work;
  existing readiness still describes the operator-Ledger/estimate boundary.
  CANIC-148's real-producer gate remains unresolved with its design parked.

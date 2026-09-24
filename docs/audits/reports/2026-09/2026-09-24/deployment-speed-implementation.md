# Bounded deployment observation improvements

Date: 2026-09-24. Development against published 0.110.40,
`1ee0f742f79af572f134ed10aa893f0b5aea3492`. Open batch: 0.110.41.
Owner: `canic-host`. This implements the first two candidates from the
[investigation](deployment-speed-opportunities.md); the larger compilation
change is a [separate proposal](../../../../design/ideas/release-binding-finalization/design.md).

## Store reconciliation

Independent upload batches now open separate Store observation scopes before
and after submission. Each successful catalog response is shared only for the
exact Principal, Candid path/hash, template and version. Every action still
validates its Candid binding and tests its own content hash. Failed queries are
not cached; scopes expire on success/error, the next pass starts fresh, and
submission explicitly clears retained observations before authority admission.

This does not reuse balances or change four-way admission, chunk-zero
preparation, result drainage, per-effect journal writes, stall bounds or retries.
Pool imports do not use the Store catalog cache. Existing observation timing
events count reused catalog reads without adding fields.

The adapter fixture demonstrates three predicates from one catalog request;
outside the scope the same three predicates require three requests. Separate
passes observe new content. Tests cover a missing chunk, wrong content hash,
completion without a receipt, query failure, and failure after a successful
cached read. Existing exact-authority/Candid tests and workflow partial-failure,
restart and effect-free replay tests also pass. A full admitted four-chunk pass
can therefore replace four matching catalog reads with one; this is a bounded
request reduction, not a claim about every catalog read in a deployment.

## Authenticated queries

Each `IcpCli` context lazily creates one HTTP connection pool and one Tokio IO
worker shared by its clones. Concurrent callers do not hold the setup mutex
while issuing reads. The last owner shuts down idle IO without blocking an
async caller's runtime. Setup failures leave the context retryable.

Each logical query still builds a new Agent after resolving the current network
and root key, exporting the selected signer, and checking its Principal. The
pool contains no signer, Agent, query result or subnet-certificate cache. The
existing response/decoding bounds, signature verification, transient-error
classification, three-attempt bound and 10/30-second deadlines remain intact.
Management, mint, debit and frontend transports keep their existing lifetimes.

The HTTP fixture requires two fresh Agents on cloned contexts to send their
requests over one accepted TCP socket. A concurrent barrier checks that sharing
the runtime does not serialize four callers. The authority fixture changes
signer and root key, verifies both fresh Agent bindings, and rejects a changed
reported Principal before any remote request. Existing 502 retry and nonretry
authentication/integrity/decoding cases remain covered. Fault-server fixtures
disable signature verification only in their explicit test Agents; production
builders retain verification.

`reqwest` becomes a normal host dependency so default-feature queries can share
their client. Its version and the lockfile are unchanged. No CLI or receipt
schema changes; HTTP client construction has its own typed query setup error.

## Evidence and limits

Local evidence directory: `.tmp/deployment-speed-20260924/`. `commands.txt`
records the final targeted commands; source and successful-log SHA-256 inventories
bind the evidence. The focused selection covers 21 native tests. Strict host
Clippy includes the `local-fleet` feature and test targets. Formatting, layering,
document semantics, whitespace and a query-source secret scan accompany it.

The initial sandboxed run passed 19 cases and could not bind the two HTTP fixture
sockets. The same selection passed with localhost access enabled. No real IC
request, downstream source edit, application build, deployment, broad validation,
version bump or Git publication was performed. Prior issue #28 fixture sources
and their manifest/lockfile hashes remain unchanged.

The proof is reduced requests and setup, not a measured wall-time improvement.
There is no arbitrary latency qualification gate. A matched live convergence
and terminal-replay receipt remains useful for quantifying the effect, including
fresh identity work that deliberately remains. The old .38 deployment receipt
and .39 build matrix cannot establish a .41 speedup. Issue #28's downstream
launcher/live-run acceptance and the accepted B3/B4 work remain separate.

## Batch and cadence

Group Store sharing, query pooling, recovery/authority qualification and their
diagnostics/docs into one host deployment batch. Both changelog surfaces use one
open .41 entry; package versions remain .40 until the maintainer's release flow.
Although 0.110 exceeds the soft twelve-release guideline, this bounded operator
performance follow-up belongs with the maintained deployment path and existing
feedback rather than opening a new minor. Do not allocate a release per slice.
The compilation proposal is unscheduled and does not waive the human-owned
minor closeout gate.

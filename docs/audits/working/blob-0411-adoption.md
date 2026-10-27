# Blob ownership and acceptance audit

## Current mounted coverage summary — Blob 0.47.0

This supersedes the historical inventory below. The canonical method owner is
`ic_blob_storage_contracts::mount_service_endpoints!`, expanded by the Canic
adapter; export parity is not exhaustive behavioral coverage.

| Mounted surface | Current consumer evidence | Remaining boundary |
| --- | --- | --- |
| Upload, reference, tenant, certificate and read descriptors | `pic/blob_service/journey/` and managed installation cases, both shapes | Actual provider bytes/browser deployment |
| Local status, admission, manifest, content, capacity and receipts | `journey/inspection/`, original identities and restored observations | Selected assertions, not exhaustive branch coverage |
| Tenant/service upload history | `journey/inspection/pagination/`, result/scan boundaries, scope and complete union | Concurrent live provider activity is not simulated |
| Gateway sync/cancel/revoke | `operator/provider/{gateways,races}/`, success, bounds, exact cancellation, discarded reply, held stale callbacks and fencing, both shapes | Default-branch delivery |
| Account balance and payment relationship | `operator/provider/{account,relationship}/`, exact bindings, typed provider/decode refusals | Actual provider/payer linkage |
| Funding history/outcome/assessment | `operator/funding/`, empty/refunded records and cursors; `paid/outcome/` covers exact acknowledged, credit-required and unknown-transfer outcomes, refunds/grants/fences, caller/intent refusal and read-only replay through recovery | Positive independently confirmed credit and receipt replay remain #494; real-provider evidence remains Blob #34 |
| External GC callbacks | `operator/provider/gc/`, bounds, caller/revocation, atomic refusal, replay and restore, both shapes | Physical deletion and billing cessation |

Current requalification uses Blob/Contracts 0.47, Tooling/Testkit 0.68.1,
Platform 0.6.15, Wire 0.7.5, Auth 0.16, Memory/Storage 0.51 and Timers 0.31.
The two-composition generated-client case and all five embedded profiles pass (536s/255s),
including finalized Wasm, exact retained schemas/bindings, permissions, held-reply
races and paid-outcome recovery. Generated tenant requests exercise actual Blob
endpoints, caller/service-binding denial, malformed frames and unchanged mutation
replay. Current installed logs are in `/tmp/canic-blob047-installed/`; strict
adapter/consumer Clippy and dependency admission are in `/tmp/canic-blob047/`.
Earlier internal Clippy evidence retains its original graph. Each current catalog method has a corresponding invocation in
the maintained fixture modules named above. This is method reachability plus the
selected assertions, not exhaustive branch coverage. Positive independent-credit
confirmation and immutable receipt replay remain #494. Earlier dated sections
retain their historical graphs and gaps; this summary supersedes them.

No current mounted method is claimed uninvoked. Distinct local test results do
not prove every branch, real-provider behavior or default-branch delivery.

The shared frontend delivery path also passes installed SDK query/update
acceptance on the earlier Blob 0.45 graph (451s; `/tmp/canic-blob-browser-current/`), with all
three lock hashes unchanged. Its application probe does not establish Blob
browser upload/download or independent provider credit.

## Historical evidence — superseded graphs and gaps

The following reports retain their original dates and dependency selections.
Statements of missing coverage or unpublished dependencies below describe those
runs only; use the current summary above and the current handoff for open work.

## Blob held-funding concurrency qualified — 2026-10-22

The embedded managed proof holds the first provider callback, invokes concurrent
funding ingress twice and verifies that neither the original dispatch nor a new
reservation proceeds. The exact held request and mounted funding summary remain
unchanged. Releasing malformed bytes refunds the whole first offer; exhausted
provider slots reject the second offer with a whole refund. Their distinct typed
outcomes remain observable with zero accepted cycles before and after restoration.
No production funding policy, provider evidence, scheduler or journal was added.

Strict embedded-consumer and internal Clippy pass. The installed managed case
passes in `/tmp/canic-blob-concurrency/embedded-retry.log`. The first build was
correctly rejected after Cargo.lock changed from Wire 0.6.0 to 0.6.1 during the
build; that incoming selection was preserved. No mixed-input result is admitted.
This is PocketIC/framework evidence, not native macOS or real-provider acceptance.

Published Blob remains 0.42.0 and Testkit 0.64.1. Tooling #235's bounded
cycle-acceptance fixture is implemented upstream but not published. Accepted
payments/partial refunds, accepted malformed/error/lost callbacks, positive credit
and budget renewal remain unqualified under #494. Real-provider payer linkage is
application/deployment evidence. #494 stays open for these gaps and default-branch
delivery; the complete batch is not ready to push or publish. Changelogs are updated.
No commit, push, release, deployment or sibling edit.


## Host 0.64.1 closes the local held-callback gap — 2026-10-22

Published Testkit 0.64.1 supplies the native held-reply fixture. Both managed
compositions pass cancellation/revocation-before-callback proofs in
`operator/provider/races/`: observe the held original request, commit the local
invalidation, verify the same reply remains held, release it, then require
Conflict and exact unchanged terminal state. A new authorized refresh still
succeeds. No live/automatic progress or simulated clock advancement is used
while replies are held; provider upgrades occur only outside held calls.

Strict Clippy, actual Rust 1.91 native compilation, three dependency admissions
and both managed cases pass; normal adapter Wasm excludes Host/Testkit/wat/wast.
Logs: `/tmp/canic-host0641/`. #622 remains open for default-branch delivery;
#494/#235 own the distinct accepted-payment gap. No native macOS, real-provider
or complete release readiness claim. Earlier missing-fixture notes are historical.


## #622 populated funding inspection — 2026-10-22

The embedded installed fixture's two real rejected/full-refund operations now
qualify descending funding history, exclusive cursor boundaries, exact retained
outcomes, changed amount/target conflicts, caller refusal, unknown-operation
absence and repeated observation. Queries preserve stable memory. The same
checks run before restoration, while fenced and after continuity recovery.
This proves populated refunded records, not accepted/credited payment outcomes.

Strict internal Clippy, the targeted embedded managed PocketIC case and focused
format/diff checks pass; `/tmp/canic-622-populated/{clippy,embedded}.log`.
No production, dependency or deployed fixture source changed in this slice.
The dedicated fixture still qualifies empty funding views; populated records
are exercised in the application that opts into the funding API.

Published Testkit remains 0.64.0. #234's held-reply support is implemented locally
upstream but unpublished, so cancellation/revocation-before-stale-callback tests
remain open. #235 and #494 own accepted-payment qualification. #622 remains open
for delayed callbacks and default-branch delivery. No full-release readiness,
live-provider/native macOS qualification, commit, push or deployment is claimed.

## Blob GC bounds and remaining race/funding gaps — 2026-10-22

Both managed compositions now qualify the mounted external GC liveness endpoint
at the shared 10,000-root bound, reject over-count and over-byte requests, and
refuse gateway listing/acknowledgment after explicit revocation while retaining
public conservative liveness. Strict internal Clippy and both targeted managed
PocketIC cases pass: `/tmp/canic-blob-races/{clippy-gc,dedicated-gc,embedded-gc}.log`.
No production API or dependency changes in this slice.

The attempted delayed gateway callback test did not establish cancellation-first
ordering: immediate provider replies completed during Canic's asynchronous
continuity admission and cancellation correctly returned `Conflict`. Both failed
experiments are retained in `/tmp/canic-blob-races/{dedicated,embedded}.log`; the
timing-dependent test was removed. This is not evidence of a Blob defect.
[IC Tooling #234](https://github.com/dragginzgame/ic-tooling/issues/234) requests a
bounded held-response fixture so [#622](https://github.com/dragginzgame/canic/issues/622)
can prove cancellation/revocation before the original callback.

[#494](https://github.com/dragginzgame/canic/issues/494) remains pending a decision
between an opt-in application-supplied trusted-evidence host API and inspection-only
until a concrete evidence source exists. Shared funding dispatch/journal APIs
are available; provider qualification, payer linkage, complete account activity
and other liabilities cannot be inferred from local journal/balance alone.
No paid path, fabricated evidence or automatic schedule was introduced.

No 100% integration or complete 0.112 readiness claim; delayed races, guarded
funding and real-provider acceptance remain unqualified. Native macOS and live
provider execution were not performed. No commit, push, release or deployment.


## Superseding adoption: published 0.42.0

The maintained selection is now published Blob/contracts 0.42.0 in all three
Canic graphs. Both managed compositions pass again, including the three newly
mounted external Candid GC callbacks. Their bounded decoders and policy remain
upstream-owned; Canic supplies raw endpoint admission, context and Candid exports.
Tests cover liveness, gateway authority, malformed roots, atomic batch refusal,
release/listing, acknowledgment replay and restoration while retaining physical
bytes and billing liabilities. The 0.41 evidence below remains historical.

The integration exposed a Canic macro parser defect with invisible groups around
forwarded literal export names. The corrected parser and focused regressions pass.
Scoped strict Clippy, both independent consumer Clippy checks, three locked
admissions, Rust 1.91 adapter Wasm and both consumer native checks pass. Managed
consumer Wasm/runtime qualification uses the configured compiler. Direct consumer
MSRV Wasm checks refused the unsupported Cargo build path; they are not passes.
Evidence and initial failures: `/tmp/canic-blob042/`.

Remaining: delayed gateway callback races (#622), guarded paid-dispatch and
renewal/uncertainty composition (#494), default-branch delivery and real-provider
evidence. The 0.42 acknowledgment profile removes the old universal independent
credit-proof prerequisite; it does not make provider acknowledgment credit proof.
No full-integration percentage or complete 0.112 release readiness is established.

## Follow-through: operator cross-tenant pagination

Both managed compositions compare the complete service-wide history against the
union of two separately authorized tenant scans. Full entries match, without
duplicate identities or repeated cursors; multiple service pages remain bounded.
Tenant callers are denied service history, and a service cursor reused under a
tenant scope returns CursorScope. Cursor bindings and unchanged stable memory
are asserted. Strict Clippy and both managed cases pass; evidence:
`/tmp/canic-blob-service-pages/`.

The cross-tenant pagination gap below is now covered. Delayed gateway callbacks,
paid-funding/provider acceptance and default-branch delivery remain distinct gaps;
no exhaustive coverage or production-readiness claim is made.

## Follow-through: populated tenant history

Both managed compositions populate 65 additional reservations through public
admission. Tenant history returns all original IDs in order across result-limited
pages; a nonmatching filter crosses the scan limit with an empty continuation.
Returned cursors advance and reject changed tenant/filter reuse. Queries preserve
stable memory. Fixture capacities increase while reservation accounting remains
exact; production page limits are unchanged. Strict Clippy and both cases pass:
`/tmp/canic-blob-pagination/`.

This qualifies tenant upload-history pagination, not service-wide cross-tenant
scans, populated paid-funding history or delayed callback races. #622 stays open
and local working-tree evidence is not default-branch delivery.

## Follow-through: payment-relationship observations

Both managed compositions now call the mounted PaymentRelationship branch with
controlled replies. Exact reported service/payer bindings, operator and request
scope checks, signed limits, a 128-bit unsigned counter, absent relationships and
all four structured provider errors pass. Malformed and oversized replies refuse.
Responses retain the exact request and local status/funding remains unchanged.
Strict Clippy passes; logs: `/tmp/canic-blob-relationship/`.

This supersedes the missing relationship-branch coverage below, without claiming
exhaustive branch coverage or live-provider billing qualification. Delayed callback
races and populated pagination remain #622 work. No production/dependency change.

## Follow-through: provider reply bounds

The mounted account endpoint rejects a 4097-byte reply with ReplyTooLarge and
unchanged local status. Gateway sync rejects a 65,537-byte reply, retains the
original pending sequence, refuses redispatch, and permits only exact operator
cancellation without changing membership. Strict Clippy and both managed shapes
pass; logs: `/tmp/canic-blob-bounds/`. No production changes or live-provider
claims. Payment-relationship responses, callback races and populated pagination
remain #622 acceptance gaps.

## Follow-through: controlled gateway/account transport

All current catalog methods now have direct invocations in the managed fixtures.
Published Testkit `reply-fixtures` supplies a local Cashier response canister,
without another provider engine. Gateway refresh/revocation, exact cancellation
of a malformed-reply attempt, original-sequence inspection after a discarded
reply, account balance decoding/refusal and restored fences pass in both shapes.
Strict Clippy and all three dependency admissions pass; the deployed normal Wasm
graph excludes Testkit and its encoder. Logs: `/tmp/canic-blob622-provider/`.

This removes the original missing-method-call gap, not every acceptance gap.
Delayed callbacks after cancellation/revocation, populated pagination, the
payment-relationship observation branch and additional provider rejection/bound
cases still need qualification. Populated paid funding and actual provider
deletion/billing retain their existing issue owners. #622 remains open; no
exhaustive line/branch coverage or default-branch delivery is claimed.

## Follow-through: operator inspection

Local status and funding history/outcome/assessment now have mounted tests in
both compositions, including scope/actor refusals, explicit absence, invalid
cursor/intent fields, reserve/provider blockers and unchanged local observations
across restoration. Strict Clippy and both managed cases pass; evidence is in
`/tmp/canic-blob622-operator/`. The earlier inventory is historical: four methods
still lack direct invocation (gateway sync/cancel/revoke and account inspection).
Populated funding records, paid reconciliation and exhaustive pagination are not
established by the empty-journal tests; #622 remains open.

## Follow-through: first #622 inspection slice

Subsequent local tests now invoke six previously absent methods: upload admission,
manifest, content lookup, reference capacity, upload history and reference receipt.
Both dedicated/embedded managed cases and strict governed internal Clippy pass;
logs are in `/tmp/canic-blob622/`. They cover original identities/declarations,
absence, foreign observers, changed requests/cursors, stable-state preservation
and historical inspection after restoration. Receipt recovery also checks the
original retain result after later release. This is not exhaustive branch coverage.

The original fourteen-method inventory below records the audit baseline. Eight
still lack direct calls: local status, funding history/outcome/assessment, account
inspection and gateway sync/cancel/revoke. #622 remains open for those paths and
the complete acceptance matrix; pending tests are not default-branch delivery.

## Verdict

The storage-engine extraction is implemented in the inspected Canic tree.
Full mounted-service coverage and end-to-end provider acceptance are **not complete**.
Do not describe this as 100% tested. No line/branch coverage measurement was run;
passing the existing journeys is evidence for their assertions, not every method.

Reviewed Canic base `ae6d053d586630cf76c8a033a32ace70b7cd0e1f` plus preserved
pending work; published Blob/contracts 0.41.1 and Testkit 0.64.0. The sibling
release is `d4dbfa1`; sibling sources were read-only. No downstream application
was assessed. Existing pending work was neither committed nor reset.

## Ownership and removal

| Responsibility | Canonical implementation | Audit result |
| --- | --- | --- |
| Installation, storage, tenant policy, registration, references, accounting and recovery | `ic-blob-storage/src/ops/service/` and `workflow/service/` | Canic delegates through `ServiceInstallation` and `ServiceWorkflow<Host>`; no second engine found. |
| DTOs, method catalog and Wire codecs | `ic-blob-storage-contracts/src/endpoints/` and `wire/` | Adapter mounts the shared catalog; no copied service schema or codec. |
| Canic caller/context and bounded endpoint IO | `integrations/blob-service/src/endpoints/` and `transport/` | Legitimate framework composition, including the externally required Candid certificate endpoint. |
| Memory grants and lifecycle | `integrations/blob-service/src/ops/memory/` and `lifecycle/` | One Canic memory runtime; shared Blob installation opens granted handles synchronously. |
| Public metrics | `integrations/blob-service/src/ops/metrics/` | Reads shared usage, observes platform version through the shared owner and maps gauges into Canic's sampler/cache. No second accounting/timer owner. |

Production Blob dependencies remain in the explicit integration package, not the
framework Core/Host/CLI manifests. Searches of current Rust under `crates/` and
`integrations/` found no retired completion-verifier/attestation flow, old
`BlobStorageApi`, direct project-cycle funding endpoint or provider GC callback
replacement. Historical design/audit documents are evidence, not active engines.
Canic's generic fixture importer still exists; its historical Blob-labelled
issue #123 is not evidence of a duplicate application Blob service.

## Historical 0.41.1 coverage matrix

| Surface | Evidence | Remaining limitation |
| --- | --- | --- |
| Dedicated and embedded production builds/installations | Both exact managed cases rerun successfully on 0.41.1; finalized Wasm and endpoint parity checked | Linux PocketIC, not installed-IC/provider or native macOS qualification |
| Installation bounds, tenant/uploader binding, invalid input rollback | `pic/blob_service/tests.rs` and installation decoder tests | Assertions cover selected boundaries, not measured exhaustive branch coverage |
| Two uploaders, preparation, certificate, original registration, lost reply, reference replay/release, download descriptor and expiration/revocation | `pic/blob_service/journey/` through actual mounted endpoints | No real provider bytes or production browser authentication |
| Same-release restore/fence/recovery and embedded app state | Both managed cases | Does not promise cross-release migration or activation from an older snapshot |
| Metrics values, units, identity, freshness, visibility/cache and restoration | `pic/blob_service/metrics/` in both cases | No new live-provider accounting evidence |
| Remaining mounted methods | Export catalog parity only; direct invocations absent in Canic tests | [Canic #622](https://github.com/dragginzgame/canic/issues/622) |
| Upstream algorithms | Inspected upstream admission, certificate, reference and recovery tests | Upstream suites were not rerun; their tests do not prove the Canic wrapper |
| Native build/dependency boundaries | Preceding 0.41.1 strict adapter/consumer Clippy and all three locked admissions passed | Current patch MSRV/native macOS not rerun |

The canonical catalog contains 27 owned Wire methods and the external Candid
certificate method. At that original inspection, fourteen methods had no direct invocation in the
maintained Canic Rust tests inspected:

- Gateway: `blob_sync_gateways`, `blob_cancel_gateway_sync`, `blob_revoke_gateway`.
- Account/funding: `blob_inspect_account`, `blob_funding_history`,
  `blob_funding_outcome`, `blob_funding_preparation_assessment`.
- Inspection: `blob_local_status`, `blob_lookup_content`, `blob_reference_capacity`,
  `blob_reference_receipt`, `blob_upload_admission`, `blob_upload_history`,
  `blob_upload_manifest`.

This is an observed host-acceptance gap, not an assertion those upstream methods
are broken. Name inventory is not a coverage percentage. #622 requires direct
success/refusal, original-operation, interruption and restore evidence as relevant,
using shared contracts and controlled provider fixtures without live paid effects.

## Open ownership and release gates

- [Canic #444](https://github.com/dragginzgame/canic/issues/444) coordinates delivery;
  [#620](https://github.com/dragginzgame/canic/issues/620) and
  [#621](https://github.com/dragginzgame/canic/issues/621) have local journey/metrics
  evidence, not default-branch delivery. #620's retired verifier acceptance was
  corrected to the current trusted-registration contract during this audit.
- [Blob #32](https://github.com/dragginzgame/ic-blob-storage/issues/32) owns provider
  GC callbacks, missing-method behavior and physical deletion evidence;
  [#6](https://github.com/dragginzgame/ic-blob-storage/issues/6) retains lifetime
  capacity, reference/deletion and separate billing acceptance. No placeholder
  callback or unconditional liveness response is an acceptable workaround.
- [Blob #34](https://github.com/dragginzgame/ic-blob-storage/issues/34) owns exact
  payer/credit acquisition. [Canic #494](https://github.com/dragginzgame/canic/issues/494)
  cannot qualify guarded paid dispatch before that contract exists. This is
  separate from a pre-funded deployment and is not supplied by passive inspection.
- [Blob #31](https://github.com/dragginzgame/ic-blob-storage/issues/31),
  [#5](https://github.com/dragginzgame/ic-blob-storage/issues/5) and
  [#4](https://github.com/dragginzgame/ic-blob-storage/issues/4) retain actual
  application/provider uploads, browser serving/integrity and publisher asset-map
  acceptance. Applications own their transactions; Canic should not absorb them.

No new upstream algorithm defect was established. Existing upstream issues own
the verified gaps; #32 received current adoption evidence instead of a duplicate
issue. No issue was closed merely because local tests passed. Complete 0.112
release readiness cannot be inferred from this Blob-only audit.

## Retained execution evidence

`/tmp/canic-blob-audit/{dedicated,embedded}.log` records the exact commands:
`make test-pocketic-case CASE=pic::blob_service::tests::managed_composition_dedicated`
and the corresponding `managed_composition_embedded` case. No broad suite ran.
Prior patch checks are in `/tmp/canic-blob0411/`; these temporary logs are local
diagnostics, not immutable published validation receipts.

Managed release IDs:

- Dedicated: `7353c7afd216016e3b8f7d45ea60c0b296e4e2fb5c9b12a5f23eb88568f7b50b`.
- Embedded: `86d1ae4d5d23e820154b18f9e26a99bf6d78410e9441efc15cdae8add8ac9ef1`.

Inspected root lock SHA-256:
`3dfe793665f6ed372927b89567e3b39cdae05024b83a7f1f25eacb26dcb2953f`.
No source change, deployment, payment, sibling edit or package publication ran.

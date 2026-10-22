# Blob 0.41.1 ownership and acceptance audit — 2026-10-22

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

## Coverage matrix

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
certificate method. Fourteen methods have no direct invocation anywhere in the
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

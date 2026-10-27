# 0.112 consolidated push-readiness plan

Audit date: 2026-10-26. Status: owned cut locally qualified; external DTO supply blocks full requested cleanup.
This is a development checklist, not release authority or a validation receipt.

## Current release scope — 2026-10-26

The maintainer narrowed the remaining release outcome to **Blob storage and Wire**.
Follow the [selected delivery boundary](../../../design/0.112-standalone-blob-service-extraction/0.112-design.md#accepted-release-scope--blob-and-wire--2026-10-26).
Production portable Auth integration and live Backup completion are deferred and
must not block this push. Keep those unavailable capabilities closed. Retain the
adopted dependencies and existing Root proof/role-attestation behavior.

The retained-code opportunities below remain recorded, but unstarted unrelated
refactors are not prerequisites for this release. Preserve existing pending work
and qualify its actual effects on the selected Blob/Wire delivery. Do not remove
endpoint authorization, custody refusals or recovery checks to narrow the scope.

## CT-06 supply preparation — 2026-10-27

The current registry still ends at Platform 0.6.19 (not yanked; checksum recorded
in the structured evidence). Default-branch management exports are unchanged;
no open supplying PR or matching open issue was found. Existing Platform #38
concerns compiler retirement, not these missing contracts.

A [reviewable upstream patch](platform-external-contracts.patch) is prepared from
Platform c75fdff26751f771492e5c44f154b218610362f6 in an isolated archive. It adds
21 explicit management/lifecycle/ECDSA/history exports, TransferArg/Memo and ten
ICRC-21 DTOs from existing pinned dependencies. The explicit icrc21 namespace
avoids colliding with the existing generic ledger Value. No new dependency,
implementation copy, compatibility alias or cryptographic algorithm is added.
Migration/inventory guidance and a proposed 0.6.20 changelog draft accompany it;
Cargo/package versions remain 0.6.19. This is an unpublished proposal, not a
supplying release or Canic adoption.

Locked offline native and Wasm library compilation, strict targeted library
Clippy and Rust 1.91 compilation pass. The patch applies to the pristine base;
the isolated lock is unchanged. Runtime tests and Platform's full Clippy gate
were not run; no installed provider or release qualification is implied. Exact
commands/results are under ct06SupplyPreparation in the structured evidence.
The sibling checkout and Canic's production code, manifests and locks are unchanged.

Adoption needs more than renaming imports. Upstream CanisterSettings contains
additional optional fields, and CanisterInstallMode includes the complete variant
set. Their Candid type tables need not match Canic's narrower mirrors byte for
byte. Qualify complete request costs and reply limits, preserve unspecified
settings and permitted actions, and retain exact original bytes across uncertain
retries. The sender-version uninstall payload maps specifically to UninstallCodeArgs.
Removing local derives does not by itself remove the required external codec
from compiled Wasm or demonstrate a size improvement.

CMC notify/rate, Cycles Ledger creation, NNS registry and asset DTOs remain outside
this export patch and need exact canonical suppliers. CT-06 therefore remains
open. Submission of the isolated PR awaits the maintainer's decision because the
original task explicitly withheld commits and pushes. Merge, publication and
Canic dependency adoption remain separate; no remote message or write occurred.

## Final consumer qualification — 2026-10-26

This continuation qualifies the preserved owned-Candid cut at dirty `main`
`cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5`. Package versions remain 0.111.0;
0.112.0 is the open changelog target. It is development evidence, not a release
receipt. No dependency selection, production behavior, Git index or release ref changed.
Final timer checks exposed fixture defects corrected as described below. The root and both independent consumer lock hashes
match the continuation's captured baseline.

The dedicated and embedded Blob generated-client case passes with Node 24.21.0
and TypeScript 6.0.3. It exercises retained generated query/update bindings,
authorization, changed bindings, malformed/trailing frames and identical update
replay against installed services. This is Node-driven client/codec qualification
using PocketIC transport, not a browser UI or live provider proof. The first
attempt omitted the required tool environment and stopped at `prepared Node`;
that setup failure is retained separately from the passing rerun.

`make test-packaged-downstream-cli` passes from eight extracted Canic packages.
The installed binary builds all four App/Fleet artifacts, reuses the complete
build, initializes the managed Fleet, reconciles failed-before-install and lost
install/controller responses, verifies installed identities and workload, and
replays terminal completion offline with zero new effects. Its cold build took
622.95 seconds and complete reuse 34.97 seconds with a **debug** CLI; these are
qualification timings, not production performance claims. The installed PocketIC
case passed in 740.17 seconds (741 seconds for the runner stage). The tool and downstream locks, package archive hashes,
CLI hash and sealed release manifests are captured independently under
`/tmp/canic-closeout-final-20261026/`. Relevant upstream selections match the root
(Wire 0.7.18, Platform 0.6.19, Host/Build 0.68.9); this does not relabel the Blob
consumers' Wire 0.7.17 / Blob 0.49.1 graphs.

The follow-up source review confirms native profile derivation and final-Wasm
admission at `crates/canic-core/src/role_contract/profile.rs:31` and
`crates/canic-host/src/canister_build/artifact/mod.rs:246`, with the infrastructure path
at `crates/canic-host/src/canister_build/infrastructure/mod.rs:116`. Provenance separately retains
external Candid artifacts while requiring native schema/TypeScript outputs in
`crates/canic-host/src/build_provenance/artifacts.rs:18`. Canonical Coordinator and Store Candid services
now expose only external ICRC-10. Their payload-contract metadata still serves
Canic's transport admission, independent of Wire structural decoding. Historical
ablation patches remain revision-bound evidence; no current Make/CI runner was
found referencing those patches. The remaining Ciborium matches in the inspected
active source/tool directories are inside those historical patches.

### Exact upstream supply gap

Platform 0.6.19 and read-only source `c75fdff26751f771492e5c44f154b218610362f6`
were inspected. The remaining management and ICRC types exist in its pinned
underlying dependencies but are absent from the public Platform export surface.
The concrete export requests and Canic callers are recorded in the structured
evidence, under `upstreamExportGaps`:

- Management settings/update/upload/install-chunk/uninstall and ECDSA argument/
  result families, including their mode, log-visibility and environment types.
- Complete replicated-history origin/change/record identities for exact authority
  matching and fixture construction. `ChangeDetails` and `CanisterInfoResult`
  alone do not expose the missing enum names.
- ICRC transfer arguments/memo and ICRC-21 request/response/error DTO families.
- CMC, Cycles Ledger, NNS and asset protocol families additionally require an
  exact published supplier; unrelated management/ledger records are not substitutes.

Canic's minimum Platform requirement is already 0.6.19. No direct dependency on
its private underlying crates, local sibling override, serialization workaround,
compatibility alias or publication was added. Removing these mirrors remains a
named upstream supply/publication/adoption batch with current-contract acceptance.

### Reconciled A–E outcome

This table supersedes the dated open/pending statements in the execution ledger
below. Prior results remain attached to their recorded revisions and graphs.

| Checklist | Current outcome and evidence | Remaining boundary |
| --- | --- | --- |
| A — native identity | Source-complete for the accepted owned cut. Native schema derives compiled profiles; final Wasm, sealed manifests and retained schemas are re-admitted. Configured App/Fleet and both Blob compositions build; exact binding, cache reuse and malformed/wrong-method refusal pass. | External Candid artifacts have separate owners; they are not Wire hashes or fallback codecs. |
| B — endpoint/caller coverage | Owned runtime/control-plane/Blob callers, managed initialization, authority hashes, fixture and generated consumers are native. Earlier native/macro/ingress, checkpoint and held-reply cases pass; current client/operator installed checks pass. | CT-06 external DTO supply remains blocked. Production Auth/live Backup are explicitly deferred with gates retained. |
| C — generated/package delivery | Canonical artifacts and embedded peer were regenerated through owners. Both Blob generated clients and frontend admission pass; the eight-package extracted CLI qualifies all four artifacts, cache reuse and installed recovery. | Package version is still 0.111.0. No release validation receipt or registry publication is inferred. |
| D — integrated qualification | Owning native/lint/build/installed checks pass, including final frontend admission, operator recovery/export, all 11 timer-authority cases and autonomous Root deletion readiness. Earlier Root bootstrap covers subtree recycle/replacement. | Targeted local evidence only: no broad suite, production provider, paid operation, native macOS or TEE proof. |
| E — readiness review | Current source/locks, canonical owner paths, package boundaries, draft notes and keep/block lists reviewed. Reports distinguish the root, independent Blob and extracted package graphs. Final timer fixture admission and rejection injection were corrected and requalified; production behavior is unchanged. | Full requested cleanup remains blocked by CT-06. No tracker updates, issue closure, Git publication or release transaction performed. |

The final frontend case qualifies authenticated generated-client calls and local
trust/admission, including stale or mismatched evidence refusal. The operator case
qualifies the actual public CLI and ICP boundary, Component recovery and terminal
binding/export. Timer qualification exercises cancellation/recurrence, restoration,
registration and observer authority. Root retirement exercises exact draining
receipt replay, autonomous progress, asset/cycle accounting and external deletion
readiness on an exclusively owned PocketIC instance. These checks qualify existing
retained-code changes as well as native transport composition; no production
installation was retired.

**Extraction verdict:** the implemented owned Wire/Blob extraction is locally
qualified on the recorded candidate and distinct consumer graphs. CT-07's final
consumer obligation is complete. The complete requested 0.112 cleanup remains
**not push-ready** until CT-06 is resolved or the maintainer explicitly accepts
that external DTO ownership gap. Moving derives into Platform would remove local
mirrors; it does not establish that required external Candid code disappears from
Wasm. The measured owned-cut Root reduction remains 204,456 code bytes (2.32%).
The existing root/detailed 0.112 draft notes cover the implemented behavior.
These routine fixture corrections need no separate release note or version.

### Final timer fixture corrections

The first timer target passed nine cases and failed two. The finite-intent caller
failed during Wire encoding: a TTL-free request fits 1 KiB cumulative allocation,
but a request with a TTL needs more. One-dimension diagnostics show 4 KiB admits
TTL zero and the full u64 range; raising only nodes, depth or bytes did not help.
Only `BEGIN_INTENT` in `canisters/test/runtime_probe/src/timer_wire/mod.rs` and
its expected caller contract in `crates/canic-testing-internal/src/pic/runtime_probe/mod.rs`
now use 4 KiB allocation. The 256-byte, 32-node and depth-eight limits stay fixed.
A native round-trip regression covers absent/zero/maximum TTL and resource seeds.
The native inventory explicitly classifies that regression.

The optional-family rejection fixture had injected an oversized checkpoint label.
The current bounded checkpoint recorder correctly accounts for it as overflow,
so that input no longer rejects publication. Both the installed runtime probe and
the owning native publication test now inject a 128-byte endpoint label instead.
Its complete public metric name exceeds the publication limit and exercises the
maintained rejection path. Assertions still require the typed invalid-request
error, retained prior performance snapshot, continued selected-family sampling and
cycle tracking. Production recording, sampling, limits and recovery are unchanged.

Passing package/browser/frontend/operator results predate only these fixture/test
corrections; their production inputs and locks remain unchanged. The final timer
rerun qualifies the changed fixture, and Root deletion readiness follows it.
The final timer run passes all 11 cases in 93.19 seconds (180 seconds including
the native compile stage). Initial failures and the allocation diagnostic remain
in the structured evidence.

Strict fixture Clippy initially found six method-ID literals missing separators.
Only formatting changed; all numeric identities are preserved and the same strict
command passes on rerun. The first lint log remains separate from that result.

## Owned Candid completion — 2026-10-26

This section supersedes earlier lifecycle/hash exceptions and intermediate size
measurements in this working plan. The maintainer explicitly selected deleting
owned `CandidType`, including managed initialization and authority hash inputs.
Required external IC/provider protocols still use their specified encoding. The
current candidate remains the dirty Canic `main` checkout at
`cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5`, package version 0.111.0 with an open
0.112.0 draft; this is not a published or release-qualified 0.112 build.

The inspected root lock selects Wire 0.7.18, Blob/Contracts 0.49.2, Query 0.74.3,
Memory/Storage 0.51.5 and Host/Build/Testkit 0.68.9. Both independent Blob
consumers select Wire 0.7.17, Blob/Contracts 0.49.1 and Memory/Storage 0.51.4;
the dedicated native artifact dependency is 0.68.8. All select Platform 0.6.19,
Candid 0.10.39 and serde_cbor 0.11.2. Root Query/Blob selection changes predated
this owned-Candid slice. Its root lock is byte-identical to the captured starting
lock. Platform features were added explicitly where its public DTOs are now used;
no retained dependency version, sibling source or release authority was changed.
The embedded consumer lock additionally gains the published ICRC ledger-type
closure selected by Core's explicit Platform feature. The captured pre-cut lock
was recovered with its exact SHA-256; all retained versions/checksums are unchanged.
The structured evidence lists the added packages and dependency edges. Its final
lock SHA-256 is `941f7dee1b73719a908155931a018deae3bf7dfc0c2e8e2a56cc9e33ac2183db`,
which is also the parent lock recorded by the qualified generated Root.

Structured command outcomes and lock/artifact identities: [owned cut evidence](owned-candid-cut.json).
Exact locks, before snapshots, logs and artifact hashes:
`/tmp/canic-candid-hard-cut-20261026/` (`selections.json`, `before/`,
`root-size.jsonl`). Earlier graph evidence remains separate.

**Verdict:** the owned derive/initializer/hash removal is implemented and its
final targeted consumer qualification passes as recorded above. The complete
requested 0.112 cleanup remains blocked by CT-06 external DTO supply/adoption.

### Ownership and findings

| ID / priority / confidence | Classification and exact scope | Evidence, invariant and acceptance |
| --- | --- | --- |
| CT-01 / P0 / high | REMOVE, implemented: owned `CandidType` in Contracts DTOs/config/IDs, Core configuration/attestation inputs and Control Plane records. `crates/canic-contracts/src/protocol/authority/mod.rs:9` owns the shared current authority allowance; its `encode`/`decode` delegate to IC Wire native payload APIs available in both selected Wire versions. | Directory synchronization, runtime/component receipts, role attestations, provisioning and registry/retirement hashes made derives reachable despite RPC migration. Their callers now use native bytes; domain separation, exact authority projections, bindings and original-operation bytes remain. Host retained fields use `registry_wire_hex`/`status_wire_hex`. No old-hash reader or compatibility lane. Native authority, Core auth/config and Control Plane provisioning/registry cases qualify the changed hashes; installed recovery is required before push. |
| CT-02 / P0 / high | REPLACE, implemented: managed initialization in `protocol/initialization/mod.rs:8`, `canic/src/endpoint/mod.rs:234`, lifecycle macro expansion, Core module installation, Host `ops/canic_init`, internal installers and both independent Blob consumers. | Lifecycle entrypoints previously kept owned type graphs reachable. Native tuple decoding now precedes restoration/hooks with byte/allocation/node/depth admission. Management `install_code` still Candid-encodes its external envelope containing opaque native init bytes. Retain exact install bytes across uncertain retries. Nested Blob service arguments remain the selected upstream Blob install contract. Generated DIDs no longer declare an owned init class. Both installed compositions, malformed/trailing/oversized init, generated managed artifacts and same-release recovery are acceptance checks. |
| CT-03 / P1 / high | REPLACE, implemented: directory cursor bytes in `canic-control-plane/src/workflow/component_registry/mod.rs:5336`; fixture metadata/grants in `canic-control-plane/src/storage/stable/fixture_store/mod.rs` use canonical storage. | Cursors delegate to native Wire, preserving the 2,048-byte cap and exact head/parent/role/status bindings. A valid 597-byte cursor exhausted the initial allocation allowance; raising only allocation to 262,144 passed while raising only node/depth budgets did not. Final native cursor regression passes. This is a bounded correctness allowance, not a claimed performance improvement. Fixture chunks remain opaque and actual encoded size remains accounted. |
| CT-04 / P1 / high | REPLACE, implemented: available Platform management/status/snapshot and ICRC account/transfer-error mirrors in Core and Host. | Imports use the selected published Platform 0.6.19 identities: `CanisterIdRecord`, `CanisterInfoResult`, `CanisterStatusResult` where its complete shape is usable, `CanisterStatusType`, `MemoryMetrics`, `QueryStats`, `Snapshot`, `DeleteCanisterSnapshotArgs`, `CodeDeploymentMode`, `Account`, `TransferError`. Canic retains routing, exact controller/status projections, bounded replies and certified-error policy. Focused management, version-observation and ledger consumers compile/test; full upstream status fixture replaces the partial copied schema. No aliases or duplicate error display implementation. |
| CT-05 / P1 / high | REMOVE, implemented: `ConsentMessageResponse` from `canic-contracts/src/dto/icrc21/mod.rs`; dispatcher/workflow/generated endpoint now use standard `Result<ConsentInfo, Icrc21Error>`. | This enum only duplicated `Result` and its conversion. Standard ICRC-21 Candid response shape is preserved. Explicit endpoint rejection mode keeps Canic startup/access errors separate from provider errors. The ICRC-21-enabled `runtime_probe` compile and actual registered-handler facade test cover macro expansion and dispatch. |
| CT-06 / P1 / high | BLOCKED: remaining external DTO mirrors in Core `infra/ic/{mgmt/types.rs,icp_refill.rs,cycles_ledger.rs,nns/registry.rs}`, Contracts `dto/icrc21`, Control Plane `ops/canister_pool/capacity_import/history`, Host management/history/ledger/CMC/asset adapters. | Platform 0.6.19's `management_canister_types/mod.rs:10` and `icrc_ledger_types/mod.rs:9` expose a finite subset. Missing exports include settings/install/chunk/ECDSA contracts, log/environment nested types, complete change/origin/record constructors, ICRC transfer arguments/memo and ICRC-21 DTOs. CMC, Cycles Ledger, NNS registry and asset contracts also need a canonical published supplier. Importing a similarly named but different ICP/ICRC protocol would be wrong. Keep current authentication, cycles and recovery behavior until an owning upstream change is published/adopted and its consumers qualified. This blocks the requested all-external-types-from-Platform end state; it does not make external Candid transport obsolete. |
| CT-07 / P0 / high | KEEP, qualified: generated, installed, recovery and packaged-consumer acceptance protects Canic composition. | Final Blob generated clients, frontend admission, extracted-package CLI, operator recovery, timer authority and Root deletion-readiness pass on the preserved source. Earlier ingress/checkpoint/held-reply/release-binding evidence qualifies unchanged production paths; final timer fixture corrections are recorded separately. Keep each result attached to its root, independent Blob or extracted-package graph. No broad gate or live-provider proof is implied. |

Platform source was also inspected read-only at
`/home/adam/projects/ic-platform`, `main`
`c75fdff26751f771492e5c44f154b218610362f6`. Its handoff records published 0.6.19;
the later source commit changes changelog presentation, not these exports.
Existing sibling guidance/lock dirt was preserved. No unselected main-only API
is used as evidence of availability. The missing exports require an upstream
capability/publication/adoption batch, not a local compatibility shim.

Retained adapters have concrete responsibilities: native initialization owns the
Canic envelope budget and exact artifact tuple; authority encoding owns the
current authority allowance while hash callers own domain separation; lifecycle
macros restore before scheduling user work; Blob composition mounts endpoints,
binds caller/auth/service identity, composes memory and installs exact bytes.
The Wire `CanisterHistoryResponse.history_candid` field deliberately carries
opaque IC management evidence; it is not an owned Candid hash or fallback.
Generic Candid call adapters marshal required external protocols with Canic
transport/authentication/retry policy. Host configured foreign initializers and
cycle-drain contracts remain external. They are not Wire fallback paths.

### Verification and size

The ordinary fast dedicated Root comparison uses the same governed profile,
selected Wire 0.7.17 and existing admission limits. Before: code 8,802,273 bytes,
full artifact 9,334,807 bytes, SHA-256
`88a49030c440cef629b21f17c29b2eb9f0e59ec1c6d9ed56c17aa43aab3ad646`.
After the owned derive/init/hash cut: code 8,597,689 bytes, full artifact
9,109,417 bytes, SHA-256
`56e7ccc5f735964d8eb398839afc736bfb5a009d953525ba73526b77dc4ec852`.
That is 204,584 fewer code bytes (2.32%); no 40% reduction is established.
This artifact precedes the final cursor allowance and consent-wrapper edits;
its hash is evidence for that exact source snapshot, not the final worktree.
The final full-App rebuild (`bc353a6cdc28d33dec5b2c58c0c3daed7ac26f25bbe739dc28a49f949a04ebe7`)
produced Root code **8,597,817** bytes, full artifact **9,109,546** bytes, SHA-256
`2d93f41d95edbd5422ea5bd0690a59f49ea4445cdd02da2106307b95fb338c35`.
Against the immediate pre-cut Root this is **204,456 fewer code bytes (2.32%)**.
Its governed artifact admission passed; this does not alone establish Root
installation/recovery. Exact record: `root-installed-cut-size.jsonl`.
The older 8,876,601-byte Root was already Wire-migrated. Separate Postcard
experiment measurements above neither qualify nor replace this comparison.

Native evidence is recorded per command, not as a workspace qualification:
Contracts all-feature library cases; Core config/auth; Control Plane provisioning,
registry and cursor; internal fixture payloads; facade endpoint/IDs/protocol;
Host initialization, management and version-observation; macro lifecycle;
changed test/application native compilations. The changed operation-ID vector
was independently reconstructed from native payload bytes, domain and length,
then its owning regression passed. Early stale-vector, cursor-budget and import
failures were corrected; logs preserve both failures and passing reruns.
The first dedicated installed case also exposed a harness version assertion:
it compared the consumer's locked Blob 0.49.1 with the root harness's 0.49.2.
`pic/blob_service/artifacts::blob_contract_release` now reads the consumer lock
and requires one exact contract identity; the installed reply is compared to
that value. No dependency selection or runtime version check was relaxed.
The rerun passes, as does the embedded case covering malformed/provider-error/
callback-trap/liquidity/budget outcomes and verified complete-build reuse.
All 11 `pic_ingress_payload_limits` cases pass.

Checkpoint installed qualification initially rejected a complete fixture authority
at host encoding. One-dimension diagnostics recorded a transition payload of 827
bytes and a two-binding restore payload of 1,652 bytes. Both encode/decode need
more than the scalar fixture allowance: 2 MiB cumulative allocation admits both;
restore encoding requires more than 2,048 traversal nodes and passes at 4,096.
Depth 16 already admits both. The private checkpoint fixture and its expected
caller contract now use those measured allocation/node allowances; its existing
byte/depth caps and production compact-digest checkpoint protocol are unchanged.
The checkpoint view receives the same allocation allowance. Temporary diagnostics
were removed. The exact freeze/restore/delayed-message case then passes.
Evidence: `checkpoint-diagnostic-r2.log`, `installed-checkpoint-r3.log`.

Root bootstrap initially exposed a private operation-ID fixture allowance:
Wire 0.7.18 charges four traversal nodes per value, so a complete 32-byte ID plus
its enclosing tuple exceeded 128 nodes. Only the three operation-bearing App
fixture requests now use 256 nodes, preserving their 16 KiB byte cap, 64 KiB
allocation allowance and depth 16. Caller declarations match; the native
`operation_fixture_budgets_admit_complete_current_requests` regression covers
zero/full IDs and maximum companion integers. Its module is explicitly classified
in the existing native inventory. The installed Root bootstrap case then passes.
Held Store grant/revocation reply recovery exposed an obsolete CDK "trap" path
in the feature-gated fixture macro. All six calls now delegate to the existing
`IcOps::trap` platform adapter; the installed recovery rerun passes. Ordinary
production macro expansion is unchanged by this fixture correction.

Governed regeneration passes for Coordinator and Store canonical DIDs, the
embedded allocation peer, dedicated Blob and embedded Backend Wasm. Canonical
Coordinator DID shrank from 10,968 to 2,173 bytes and Store from 3,487 to 1,937;
these are interface-text sizes, not Wasm savings. Single-role builds are not
installed composition proofs. The final governed release-binding PocketIC case
also passes, covering fast/release identity, malformed and wrong-method refusal,
initializer mismatch and reinstall. Exact commands and outcomes are retained in
the structured evidence; broad workspace suites were not run. Final strict
fixture/library/application Clippy passes. The first App lint found missing
separators in method-ID literals; formatting was corrected without changing
values and its rerun passes. Changed-file formatting, diff whitespace and scoped
document-link checks also pass.

### Ordered remaining batches

1. Resolve CT-06 in the owning upstream repository: expose the exact management,
   history and ICRC contracts; identify canonical CMC/Cycles Ledger/NNS/asset
   suppliers. Publish and explicitly adopt them before deleting Canic mirrors and
   duplicate conversions. This is a separate dependency-adoption batch; no sibling
   edits, upgrades or publication were authorized by this continuation. Preserve
   externally specified Candid bytes, explicit authority and bounded replies.
2. Qualify the changed external consumers after adoption: owning management,
   history, ledger/refill, consent and asset adapter checks, affected feature/Wasm
   graphs, then installed tests for any changed lifecycle or paid-effect path.
   Re-run only the obligations invalidated by that change. Do not add shims.
3. After the scope gap is resolved or explicitly accepted, the maintainer selects
   the governed release/version transaction. Local A–E reconciliation and CT-07
   acceptance are complete; the current draft is not a release receipt. Optional
   storage/performance experiments and ten unrelated refactors remain separate.

### Must resolve before the 0.112 push

- Resolve CT-06's missing published Platform/external contracts and qualify their
  adoption, or explicitly accept that named external DTO ownership gap. The
  current checkout is not zero-CandidType.
- Keep the accepted scope and exact candidate/consumer graphs attached to the
  human-owned release preparation; package versions remain 0.111.0 and no 0.112
  release-validation receipt has been created by this development work.

## Storage codec experiment — 2026-10-26

The maintainer requested comparing current CBOR storage with Postcard before
deciding on another storage hard cut. The isolated experiment is complete;
production storage remains CBOR. Exact measurements, artifact hashes, source
provenance, limitations and remaining adoption work are recorded in
[storage-codec-experiment.json](storage-codec-experiment.json). The scratch
snapshot, probe source, artifacts and logs remain in
`/tmp/canic-storage-experiment-20261026/`.

The preferred candidate reuses IC Wire's bounded **payload** codec without RPC
framing, while retaining separate storage types. Both the probe and dedicated
Root comparison select Wire 0.7.17. Instruction measurements use an uninstrumented
PocketIC Wasm; requested heap peaks come from a separate allocator-instrumented
Wasm. These are codec measurements, not end-to-end latency or cycle-cost claims.

| Measurement | Current CBOR | Bounded Postcard |
| --- | ---: | ---: |
| Managed Root code-body bytes | 8,797,826 | 7,420,852 |
| In-flight funding record bytes | 995 | 375 |
| Funding decode instructions | 258,667 | 96,498 |
| Funding decode peak requested heap bytes | 6,400 | 64 |
| Funding encode instructions | 103,795 | 125,170 |
| Auth replay decode instructions | 41,220 | 12,502 |
| Provisioning retry decode instructions | 81,589 | 21,392 |

Both Root builds pass existing Wasm admission guards. The captured snapshot
initially failed compilation because `MetricsStatusRequest` still derived Candid
while `MetricsKind` no longer did. Both compared variants contain the same
scratch-only restoration of that trait. Subsequent live owned-Candid work uses
its own correction and has continued independently; these results qualify the
recorded snapshot, not that moving worktree.

The probes pass round trips, every truncated prefix, trailing-data rejection,
typed byte/allocation/node/depth refusals, and refusal of a short input declaring
one million zero-width elements. PocketIC also proves that an interrupted stable
write rolls back and the original funding record remains readable on repeated
calls. This does not qualify the complete Root paid-effect recovery workflow.

The result supports a deliberate structured-storage cut, not an immediate global
serializer replacement. Funding encoding is about 21% slower; maximum `u128`
values grow from 17 to 19 bytes and cost more instructions. The Root prototype
uses generous experiment-only limits, not qualified record-owned allowances.
Several checkpoint, caller-authority and completion-receipt hashes call the
generic storage encoder. The prototype separates those calls and preserves their
CBOR bytes for a controlled comparison; any intentional owned hash-contract cut
belongs to its owning protocol work, not an incidental storage change.
Production adoption still requires record-specific limits, explicit hash/storage
ownership, current schema/footprint/test updates, and installed
interruption/replay/checkpoint qualification. No production code, dependency
lock, changelog version or release surface was changed by this experiment; the
complete release batch remains not push-ready.

## Extraction closeout implementation — 2026-10-26

**Superseded intermediate storage qualification:** the dedicated Root build passed admission with
9,434,885 code-body bytes and 3,148,027 bytes of headroom. Direct generic CBOR
parsing exceeded the code limit; the discarded erased-serde experiment exceeded
the function limit. Neither guard was weakened. The maintained decoder parses
once into borrowed storage values, preserving duplicate entries for record
validation; encoding still streams directly. The temporary value tree adds
allocation, so this is a measured Wasm-size correction, not a runtime performance
claim. No erased-serde dependency was retained. All 319 Contracts cases pass.
Affected storage/lint, regeneration and installed/package acceptance are being
rerun; earlier 801-case evidence does not qualify this correction. Logs:
`contracts-tree-final.log`, `root-tree-build.log` in the closeout directory.


This section supersedes earlier source-only and remaining-work statements below
for the extraction findings it names. Installed and packaged qualification is
still in progress; it is not a release receipt. The maintainer authorized the
hard-cut remediation after the audit, explicitly removed the unqualified credit
wrapper, and separately selected replacing Ciborium with serde_cbor storage.

The audited Canic baseline was dirty `main` at
`cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5` (1,085 status entries), with package
version 0.111.0 and an open 0.112.0 changelog. It is an uncommitted 0.112 candidate,
not a tagged 0.112 build. Read-only upstream baselines were clean Wire 0.7.18 at
`c748379325266d7fa614fefee9d745eb4a88acca` and Blob 0.49.1 at
`122a26030d565304d1715a2606ca68719688217c`. Later sibling changes are not
qualified by this evidence. Baseline file hashes and diffs are retained in
`/tmp/canic-extraction-audit-20261026/`; remediation snapshots, logs and exact
final dependency selections are in `/tmp/canic-extraction-closeout-20261026/`.

The root selects Wire 0.7.18, Memory/Storage 0.51.5, Host/Build/Testkit 0.68.9,
Query 0.74.2 and Backup 0.44.4. Both independent Blob consumers select Wire
0.7.17 and Memory/Storage 0.51.4; the dedicated native artifact dependency is
0.68.8. Applicable graphs share Blob/Contracts 0.49.1, Platform 0.6.19, Auth
0.17.1, Metrics 0.16.3, Timers 0.31.5 and serde_cbor 0.11.2 with standard features.
No retained registry package identity/checksum was upgraded by this closeout.
The three locked metadata captures are separate evidence, not one unified graph.
The 2026-10-22 accepted Wire cut supersedes the earlier owned-Candid exception;
external IC/provider protocols retain Candid. The accepted 2026-10-26 completion
boundary above moves owned lifecycle arguments and authority hashes to native Wire.

| Finding | Classification, priority and confidence | Implementation and acceptance |
| --- | --- | --- |
| EX-01 — stale owned DIDs and finalization gap | REMOVE / remediation; P0, high | Regenerate `crates/canic/candid/{fleet_coordinator,wasm_store}.did` through Host. `artifact_io::finalize_wasm_artifact` at `crates/canic-host/src/artifact_io/mod.rs:113` now checks exact endpoint/codec parity on embedded and normal paths as well as sidecar-only paths. The actual release-binding template emits its Wire declaration. Finalization refusal tests preserve prior artifact bytes. Both final managed builds pass; installed binding and packaged builders remain in qualification. |
| EX-02 — evidence did not qualify the selected graphs | BLOCKED until owning checks pass; P0, high | Root and independent consumers differ as recorded above. Qualify final native-schema/Wasm outputs, both Blob compositions and browser clients, checkpoint/Root same-release recovery and packaged Fleet builders. Earlier passing runs stay bound to their recorded inputs. Native checks and canonical generators pass; final installed/package results remain pending. |
| EX-03 — duplicate installed Blob journal access | REPLACE; P1, high | `integrations/blob-service/src/funding/mod.rs:43` delegates prepare/dispatch/renew through Blob 0.49.1's public `Workflow::ACCESS`; delete `InstalledJournal` and its copied fence/journal mechanics. Retained wrappers bind actual IC actor/service and Canic-composed service limits. Installed uncertainty, trap, renewal and replay cases qualify that boundary. |
| EX-04 — active documentation and abandoned fixture artifact | REMOVE / correct; P1, high | Delete the unconsumed DID written by `scripts/ci/auth-renewal-cli-proof-lib.sh`. Update Contracts/facade/macro READMEs, starter endpoints, payload inspection, metrics, funding and release-census guides to actual native declarations and limits. `canic_coordinator_registry` remains a live Wire query, not dead code. Frozen reports, ablation locks and revision-bound patches remain historical evidence. Changed documentation links and shell syntax pass. |
| EX-05 — unqualified independent-credit wrapper | REMOVE; P0 for exposed capability, high | Remove `canic_blob_service::funding::confirm_credit` and its wrapper-only fixture calls. Current provider fixtures do not supply independently authenticated positive-credit evidence. Upstream journals, uncertain amounts, renewal refusal, no-resend protection and same-release restoration remain. Reintroducing a Canic capability requires a real evidence source and installed acceptance; no shim or paid retry is added. |
| EX-06 — allocation solely to measure a Store frame | REPLACE; P2, high | `append_chunk_actions` in `crates/canic-host/src/fleet_ensure/ops/current_protocol/mod.rs:2334` uses Wire's `encoded_frame_len`, available in the selected 0.7.17/0.7.18 API. Actual submission still encodes and retains its request. Existing chunk-boundary/oversized-label tests pass. Avoiding the discarded buffer is the mechanism; no time, instruction or Wasm improvement is claimed without measurement. |
| EX-07 — obsolete delegation Candid benchmark | REMOVE; P2, high | Delete `crates/canic-core/benches/serialize.rs`, its bench declaration and sole Criterion dependency; remove the stale test-only exclusion. The maintained proof transport is Wire, cryptographic bytes belong to Auth and stable records use their own storage format. No retained package identities change; unused benchmark/CBOR dependency closures disappear. |

Additional accepted storage work: `canic-contracts/src/serialization/mod.rs:29`
delegates CBOR grammar to serde_cbor and privately selects exactly 16 big-endian
bytes for nested u128 values. This is Canic's record representation, not a copied
Wire codec. Numeric Wire values, external IC CBOR, canonical hash arrays and
separately fixed-width storage retain their representations. Invalid widths and
trailing data reject; no predecessor reader is introduced. Native roundtrips cover
maximum cycle values, bounded records, nested containers and same-release records.
The exact known unmaintained advisory/checksum is maintainer-approved. Enabling
serde_cbor's optional tag feature broke signed IC envelopes through feature
unification; standard features restore the stack's expected decoding and all
171 selected Host cases pass.

Qualification also found valid operation IDs/cursors exceeding Canic's declared
Wire node/allocation budgets. The owning `protocol/funding_read` and
`protocol/release_evidence` declarations now admit their valid typed requests
without changing frame-byte caps. The native declaration/schema builds carry
those exact budgets. Private installed-fixture helper gates now match the
`fleet_registry` caller module, preserving both ordinary native and governed
PocketIC compilation. Historical Q6 byte-count snapshots in active tests became
current-record roundtrip/bound checks; frozen Q6 evidence is unchanged.

Keep endpoint authentication and context, method identities and semantic budgets,
transport/retry ownership, exact schema-to-Wasm admission, lifecycle/memory and
metrics composition, installation bindings, and dedicated/embedded tests. Canic's
Wasm Store remains its artifact publication/registry service; its name does not
make it the extracted Blob storage engine. Keep external provider/management/
Ledger/CMC contracts, initializer bytes, canonical authority hashes, controlled
cycles and existing Blob liabilities. Source cleanup does not retire installations.
The shared-tooling snapshot passed integrity verification; no exporter-owned
snapshot file was locally modified by this closeout.

Ordered acceptance batches: (1) pure dead surface/docs/benchmark removal;
(2) published-API delegation and finalizer admission; (3) separately accepted
CBOR representation and exact risk exception; (4) regenerate canonical and
embedded artifacts; (5) exact installed and packaged qualification; (6) reconcile
all results and the open 0.112 notes. Batches 1–4 are implemented. Targeted native
evidence totals 801 passing cases (316 Contracts, 312 storage, 171 Host, one
payload metadata and one real macro/schema case); affected strict library/test
lint passes. Batches 5–6 remain active. `commands.log` records generator and
qualification invocations. No broad gate, staging, commit, push, publication,
version bump, deployment or live provider operation has been performed.

## Accepted retained-code simplification — 2026-10-26

The maintainer requested saving and implementing all eighteen opportunities from
the two complexity reviews. Blob, Wire and Backup are excluded and remain owned
by the concurrent extraction session. These are implementation slices in the
current open batch, not separate releases or a new minor. Recheck current source and
shared-target ownership before each slice; preserve concurrent work.

| Review rank | Outcome and canonical owner | Included evidence and focused validation | Status |
| --- | --- | --- | --- |
| 1 | Host Fleet apply: explicit admission, replay, execution and completion phases with a validated execution context | Owning native scope/recovery tests; exact retained authority, funding and effect-free terminal replay | Pending |
| 2 | Control Plane allocation: share top-level/child paid-effect progression; retain distinct parent, install and commitment authority | Owning transition tests and selected allocation PocketIC journey; uncertain creation/install reconciliation | Pending |
| 3 | Control Plane Directory: share observe/validate/act/reconcile/independently-observe orchestration | Exact binding/hash refusals, lost replies and owning installed convergence case | Pending |
| 4 | Control Plane registry storage: typed snapshot and compare-and-commit helpers | Conflicting metadata/rows, atomic coordinated publication and existing mutation tests | Pending |
| 5 | Host Fleet effect evidence: explicit action/phase evidence instead of unrelated optional fields | Owning durable-shape, paid retry limits, malformed evidence and same-operation recovery tests | Pending |
| 6 | Control Plane provisioning: share reservation/claim/install/registry cursor mechanics | Stage prerequisites, distinct hash domains, cursor boundaries and one-step replay tests | Shared cursor commit implemented; 16 owning native tests pass; installed qualification pending |
| 7 | Host Fleet plan comparison: explicit borrowed authority projection instead of cloning and clearing a plan | Bounded observation movement, preserved funding checks and authority-drift refusals | Implemented; 80 earlier workflow cases pass; both authority regressions and strict library lint pass on the refreshed lock |
| 8 | Core ICP refill: one pure transition decision for outcome, uncertainty and budget reservation | Bad-fee/notify retry, uncertain transfer, refund and original spending-authority cases | Implemented; all 86 focused tests and strict library lint pass on the final domain-owned implementation |
| 9 | Host complete build: own product orchestration and structured result; CLI parses/renders | Cold/reuse, retained inputs, exact release finalization and CLI result tests | Pending |
| 10 | Control Plane removal: common exact-target stop/reobserve and recycle sequence for Components and subtree leaves | Controller/module/status refusals, existing pool claim/recovery and selected removal PocketIC case | Implemented; nine native workflow tests pass; installed qualification pending shared target |
| 11 | CLI auth: retain proof/batch states and unavailable reasons as typed values until rendering | Every status decision, issuer drift, medic guidance and unchanged JSON/text labels | Implemented; 12 auth tests pass on the refreshed lock; strict library lint passes |
| 12 | Core public metrics: producers supply counter/gauge and saturation semantics instead of name-prefix inference | Current counter deltas, timer registration windows, gauges, saturation and public projection tests | Pending |
| 13 | Core configuration: one provisioning-grant graph checker shared by validation and topology compilation | Unknown/self/cyclic targets, canonical output and existing boundary-specific diagnostics | Implemented across all three boundaries; configuration tests pass in the 189-case Core selection; strict library lint passes |
| 14 | CLI reports: bounded ordered parallel collection shared by metrics, cycles and live listing | Concurrency bound, stable order, per-target failures and panic handling | Implemented; 19 owning collector/transport tests pass, including concurrency/order and panic handling; strict library lint passes |
| 15 | Host/CLI command context: resolve workspace, Fleet and ICP target context once | Exact environment/root binding, target disambiguation and owning report/command cases | Pending |
| 16 | Core timer custody: share retained once-registration setup, identity lookup and cancellation | Retain rollback, duplicate custody, scheduling and cancellation at existing domain owners | Implemented for six domain owners; 189 selected Core tests pass, including custody refusal and rollback; strict library lint passes; installed qualification pending |
| 17 | Core Component Groups: validated compilation context reuses graph validation and group flattening | Occurrence-preserving paths, labels/purposes, cycles, bounds and canonical bytes | Pending |
| 18 | Core configuration: common internal borrowed runtime projections for Component/child declarations | Exact role config values, existing TOML declarations and generated runtime projection | Pending |

Start with 10, then 6 and 7, before the larger 3/2 consolidations. Each change
includes obsolete-path deletion, direct diagnostics and current changelog notes.
Keep intent-before-effect boundaries, independent observations, bounded paid
effects, exact claims, synchronous commits and terminal replay. A smaller source
tree alone does not establish faster builds, smaller Wasm or safe recovery.
No broad validation, Git publication, version change or deployment is implied.

Opportunity 10 uses one private `WorkloadRemovalTarget` for stop, reobservation,
exact controller/module validation and stopped-workload recycling. Existing
pre-effect intent persistence, separate Component/leaf claim derivation,
already-pooled reconciliation and terminal receipt commits remain at their owners.
The obsolete paired execution/observation helpers are removed. Evidence and
the causal source snapshot: `/tmp/canic-retained-complexity-20261026/`.
Focused Root-feature workflow tests pass (nine cases); the default-feature test
build encounters an unrelated stale `WasmStoreCanisterApi::operation_status`
test call in the concurrent extraction surface. The selected installed case is
`Prepared Root initial-Shard bootstrap`; its first invocation was interrupted
while waiting for the concurrent Blob browser run's shared build lock. No
installed result or complete-batch push-readiness is claimed.

Opportunity 6 gives the four stages one `commit_member_advance` transaction:
load and validate the retained operation, require exact advance disposition,
validate stage-specific member evidence, replace exactly one typed cursor and
compare-and-commit. The four canonical cursor record/hash constructors remain
explicit because their distinct field names and domains are issued authority.
This removes 91 production lines without changing stored records or hash bytes.
All 16 owning Root-feature provisioning tests pass, including ordering,
multi-placement boundaries, restart, one-step replay and corrupt evidence.

Opportunity 7 replaces all six clone-and-clear comparisons with borrowed named
authority projections owned by Host ops; workflow delegates comparison.
Exhaustive destructuring requires new record fields to
be classified; ordered list membership and exact protocol actions still compare
in full. Only the previously excluded canister funding estimates, estate
observations and informational recovery estimates may vary. Balance movement
and funding sufficiency stay checked independently by the existing callers.
The projections intentionally use more declaration lines to expose the authority
boundary; this is not a source-size or performance benchmark claim. Two direct
regressions cover estimate movement versus changed targets, protocol funding,
creation authority and pool membership. The workflow selection passes 79 cases
in the sandbox; its sole localhost fixture failure passes via the exact compiled
regression with localhost access (80 passed overall; two ignored). No ignored
paid-provider test was activated. Logs and causal snapshots remain under
`/tmp/canic-retained-complexity-20261026/`.

Opportunities 11/13/14/16 are implemented together in the retained-code cleanup.
Auth state and issuer refusal decisions retain enums until report projection;
labels remain unchanged. A shared borrowed graph preserves source diagnostics,
canonical topology bytes and incoming grants from unadmitted requesters. Six
core timer owners share inactive retained registration, identity lookup and
cancellation; domain activity/schedules and existing rollback stay at their owners.
CLI collectors now use eight scoped workers per batch with ordered results and
per-target panic projection. Core selection: 189 pass; auth: 12 pass; owning
report transport/collector cases: 19 pass. The wider CLI selection has one
unrelated funding-registry fixture failure (`Authority(MissingEnvironment)`),
with 96 passing cases; do not describe that command as passing. A stale
management-ops test now asserts native reserve evidence directly instead of
serializing an internal outcome. Evidence and before snapshots:
`/tmp/canic-cleanup-continue-20261026/`. Both exact plan-authority regressions
and strict Core/Host/CLI library lint pass on the refreshed lock. Installed
timer/removal qualification remains separate.

Opportunity 8 centralizes outcome status, typed failure, transfer uncertainty and
budget reservation in one pure domain decision beside the existing resumability
invariant. Ops apply its result and outcome-specific evidence in the existing
single-record transaction. No record schema, transfer identity, fee-reconciliation
ordering, provider calls or issued spending authority changed. All 86 focused
refill tests and strict library lint pass on the final domain-owned implementation.
Eight of the eighteen opportunities are implemented; the other ten remain in the
accepted table above as deferred simplification work under the narrowed release
scope. Complete Blob/Wire closeout before resuming unrelated refactors.

## Outcome and boundary

Deliver the accepted Blob/Wire hard cut as one coherent batch: owned endpoint
bytes, callers, native schemas, compiled profile identity, retained artifacts,
frontend bindings and operator recovery agree. Remove superseded owned Candid
transport and delivery machinery. Preserve independently specified external IC /
provider Candid and lifecycle safety. The later accepted owned-Candid completion
boundary deliberately changes owned initialization/hash contracts; storage follows
its separately accepted hard cut.

Freeze the currently selected dependency graph while completing this batch.
The maintainer explicitly requested a refresh on 2026-10-26: Auth 0.16.4,
Blob/Contracts 0.49.0, Tooling 0.68.7, Wire 0.7.16, Platform 0.6.18,
Memory/Storage 0.51.3, Timers 0.31.4 and Query 0.73.5 are selected in all
applicable workspaces. Backup 0.44.3 and Metrics 0.16.2 were retained at that refresh. Shared Tooling
now selects 0.26.20. A concurrent root lock refresh selects newer Metrics, Wire,
Memory/Storage and Timers patches; qualify the actual lock, not these older
minimum declarations. Locked metadata, dependency boundaries, runtime-owner admission
and targeted Host/Backup/Blob library Clippy pass. Blob adapter Wasm compilation
and both runtime-reporting budget tests also pass. Installed qualification must
be renewed on these inputs. Evidence: `/tmp/canic-upstream-refresh-20261026/`.
Upgrade only to resolve a demonstrated blocking defect; record which evidence
must be repeated when inputs change. Preserve all unrelated dirty work.

Known generated-source gaps are corrected and focused qualification has resumed.
Continue managed artifact production in C and compiler/acceptance fixes in D. Do not run workspace-wide
validation or release commands as an inferred part of this plan. Publication,
version mutation and deployment remain separate actions.

## Latest published adoption and remaining sediment — 2026-10-26

Subsequent Auth adoption selects verified published Auth/protocol types 0.17.1
across the root and both Blob consumer locks. Strict Core/Contracts lint, 350
`auth`-filtered native cases, affected fixture regressions and the production Core
receiver Wasm feature check pass. No runtime adapter or custody/activation change
was needed; two logout fixtures now select the upstream ledger error explicitly.
Evidence: `/tmp/canic-auth-adopt-20261026/`. Blob/browser installed evidence below
predates this Auth update; it is not new installed Auth or TEE qualification.

The later maintainer-requested refresh selects Blob/Contracts 0.49.1,
Tooling/Build/Host/Testkit 0.68.8, Query 0.74.1, Wire 0.7.17, Platform 0.6.19,
Memory/Storage 0.51.4, Metrics 0.16.3 and Timers 0.31.5 in all applicable
workspaces at that refresh; Auth subsequently advances as recorded above and
Backup remains 0.44.3. Registry archive
checksums were verified. Three-graph dependency admission and strict Backup/Host/CLI/
governed internal-test Clippy pass. Ten Query catalog, 251 Backup adapter, four
CLI funding and one Store operation identity regression pass. Logs and before snapshots: `/tmp/canic-push-completion-20261026/`.

The execution ledger below retains earlier source milestones; its outstanding
operator, generated-browser and packaged delivery claims are superseded by the
installed qualification entries at the end of this document. Earlier browser/package passes used Blob 0.49.0. Both dedicated and embedded
installed compositions now pass on 0.49.1, including all five embedded funding
outcomes and verified artifact cache reuse. A first attempt refused a real
mid-build root lockfile change; the stable retry passes with Query 0.74.1 and
Backup 0.44.3. Both complete native-schema/Wasm artifact builds pass.

Host caller review distinguishes management/controller/ledger/CMC/asset Candid,
initializer bytes and retained authority encodings from owned Wire transport.
The configurable drain adapter has no Canic canister implementation and requires
an exact independently supplied endpoint contract plus two-sided cycle evidence;
its Candid transport is not an owned endpoint fallback.

The unused generic Candid-text Fleet transition runner is removed. Native
`FleetProtocol` actions now own planning, observation, execution and same-operation
recovery. The hidden step list, artifact map, duplicate readiness projection,
custom journal projection and CLI progress label are deleted. Lifecycle argument
encoding lives in `ops/init_arguments/` with unchanged exact hashes, bindings and
bounds. Current native recovery fixtures replace the old transition fixtures;
release fixtures now carry valid Wasm/schema pairs and current receipt shapes.
All 609 active Fleet tests, 29 CLI Fleet report/command tests and strict
Host/CLI library/test Clippy pass on the
current lock (Wire 0.7.18, Host/Testkit 0.68.9, Memory/Storage 0.51.5,
Query 0.74.2 and Backup 0.44.4). These selections were already present when this
cleanup compiled; this pass did not update manifests or locks. The exact governed
minimal-Wasm creation/install/lost-reply/replay PocketIC proof passes. This fixture
no longer pretends to implement managed Root protocols; native protocol barriers
and publication recovery remain in the dedicated Fleet cases.
Evidence: `/tmp/canic-wire-transition-cut-20261026/`.

## Execution ledger — 2026-10-26

| Package | Current evidence / remaining work |
| --- | --- |
| A | Native precompile schema projection implemented; configured/batch/infrastructure and audit Root profile producers consume it; terminal registry admission recomputes native profiles; final schema/provenance checks reject input drift. Provenance and schema cache paths retain distinct Wire identities and separately required external/lifecycle Candid evidence; no blanket sidecar deletion is justified. Remaining exact artifact qualification is C/D. Production library compilation now passes; see current qualification below. |
| B | Removed local Fleet DID staging and observatory fixture copying; preparation verifies release schemas before allocation. Root membership and its actual installed callers now use native Wire; all five demo sharding and both sandbox endpoints migrated. Two unconsumed Root fixture endpoints and the exhausted Candid checker were removed. Scaling, Fleet admission and managed guard/initialization add 13 migrated declarations with installed callers; the funding instruction audit now checks the actual typed transferred-cycle response. Checkpoint and Hub/Shard recovery/funding add 16 migrated declarations and their installed callers; two unused generation setters removed. Exact checkpoint authority and durable encoding remain intact. Sharding/index/startup/caller-authority add 11 migrated declarations and installed callers; settled-update helper uses native budgets and retains no-resubmit behavior. Runtime memory/lifecycle/performance and Root proof provisioning add seven native declarations with installed callers; raw observation queries retain measurement isolation and register native schemas. Issuer fixture: six endpoints migrated with callers and five unconsumed methods removed; decoded-attestation predicate and macro compile fixture follow native DTOs. Audit family: seven declarations use Wire with funding callers; Root balance retains raw guarded closed-fence access. Cohort source regression covers native slot/value preservation; historical ablation patches are revision-bound evidence. Raw Hub fixture release and its two installed callers now use Wire; its import record encoding is unchanged. Receipt intent begin/load/settle now use native Wire with installed callers and a standalone schema producer; generic Candid codecs removed without changing durable intent or lifecycle behavior. Managed sharding Root controls and host/self-call consumers now use native Wire; full allocation shape and all eight native schema registrations are present, with unchanged settlement polling. Source regression distinguishes settlement/acknowledgement slots. Cycles Ledger fixture: seven owned controls and 40 callers migrated, with lossless bounded magnitude observations; external protocols remain Candid. Control schema producer and unit/wide-amount regression source added. Root funding probe: 15 owned endpoints, 12 host calls and inter-canister grant calls migrated; provider Candid retained. Role-attestation ingress reread removed; all guard expansion consumes decoded proofs. Raw candidate roster is classified in source, not runtime-qualified. Operator component CLI fixture now runs after the production Ensure journey using finalized release authority; synthetic terminal plan/journal and loose schema helpers removed. Lost-reply injection targets the current Root provision method. Installed acceptance remains unrun. Production macro scan classifies 186 native declarations and two required external standards; remaining inventory is Host callers, generated templates and retained artifact records. |
| C | Root stub now has an actual native producer; generators share precompile/final admission projection. Dedicated and embedded single-role outputs regenerated; complete dedicated and embedded four-artifact builds pass; embedded allocation peer refreshed and verified. Both installed Blob compositions pass; generated browser consumers remain. Existing 0.112 notes updated; final consolidation pending. |
| D | Contracts/runtime/adapter/Host/CLI native library Clippy passes; 68 focused tests pass, Contracts MSRV and adapter Wasm checks pass. Host schema regressions and managed embedded artifact build pass; both installed Blob compositions pass after refreshing/verifying the stale peer; Root deletion-readiness now passes after correcting fixed-request and maximum-manifest resource budgets; operator and generated browser cases remain. |
| E | Not ready; no publication or release actions. |

Task snapshots: `/tmp/canic-profile-wire/before/` and `/tmp/canic-membership-cut/before/` and `/tmp/canic-fixture-cut/before/` and `/tmp/canic-checkpoint-cut/before/` and `/tmp/canic-placement-cut/before/`. Native profile derivation no
longer consumes Candid bytes at searched call sites. The native projection is not
final-Wasm admission: builders compare its exact hash after runtime finalization.
Do not conflate these two checks or remove final admission.

The lockfile inspected during implementation already selects Tooling/Testkit
0.68.6 and root Memory/Storage 0.51.3 (independent Blob consumers retain 0.51.2), newer than the initial handoff. Offline resolution added existing Wire/Serde dependencies to migrated fixture packages;
package versions, sources and checksums are unchanged. Use the actual selected
graph for qualification.

## Audit findings and confidence

| Finding | Evidence | Consequence |
| --- | --- | --- |
| Compiled profiles are native Wire-derived in source | `crates/canic-core/src/role_contract/profile.rs`: `derive_protocol_profile_hashes` hashes canonical Wire schema bytes in the Wire domain; configured and infrastructure builders supply that schema | Source migration implemented; final artifact qualification remains, package A |
| Normal builders already produce Wire schemas | `crates/canic-host/src/canister_build/artifact/mod.rs`: `qualify_wire_output` on single, configured and full build paths | Keep this integration; complete its sequencing rather than rebuild delivery |
| Retained schema admission and CLI inspection are connected in source | `release_set/declaration`, `protocol_binding/release`, frontend resolver, `info endpoints` | Connected source; final generated/frontend/package qualification passes |
| Focused qualification has resumed | Runtime/contracts/macros/Host/CLI/adapter library checks and dedicated managed build pass | Test-feature, embedded, generated and installed qualification remain |
| Owned role-union selectors are cut in source | Current handoff and macro/runtime changes | Does not prove every direct method, lifecycle argument or generated client migrated |
| Installed component-removal acceptance remains outstanding | Existing registered Root baseline cases cover initial-shard subtree recycle/replacement, held Store grant/revocation replies, and draining Root deletion readiness | Run the existing owning cases; the earlier narrow-search absence hypothesis was incorrect |
| Browser Platform supply is available | Published `@tokostudios/ic-platform@0.1.0`; previous 404 used the wrong npm scope. Manifest/lock/imports adopted with SDK Core 6.1.1 | Installed generated-client qualification remains; no unpublished alias |
| Live Backup is unfinished | Owning design status; active CLI refuses live creation; Root-paid orchestration/restore convergence pending | Do not claim live Backup from Wire completion |

This audit identifies the coordinated work below. It is not a claim that every
runtime path has already been inspected or that no compiler defects remain.
Package B closes that explicit inventory gap before source completion is declared.

## Source inventory and dependency order

The [endpoint candidate inventory](endpoint-candidates.tsv) is a dated source
search, not an executable allowlist or a qualification score. Its 71 entries are
now classified: 38 native Wire declarations, 17 external-provider fixture
endpoints and 16 external-Candid adapter qualification endpoints. The named
source families and their discovered callers have migrated; this does not replace
an exhaustive macro/generated-code/caller review or installed acceptance.

The funding fixture's owned methods now use native frames, while its provider
models keep their Candid encoding. The role-attestation guard verifies the decoded
native proof; its independent ingress Candid reader has been removed. Production
typed macros retain Candid for external ICRC-10/ICRC-21. Managed lifecycle
initialization now uses native Wire; record storage has its separate owner.

**Next dependency audit:** A's remaining declaration extraction and sidecar
finalization must be classified against external and lifecycle consumers. The
owned raw-endpoint migrations no longer justify retaining an owned Candid
protocol pipeline. Trace build records, caches, provenance and release assembly
before deleting exhausted fields; retain genuinely required external evidence.

The 2026-10-26 follow-up traced `canister_build/artifact`, `infrastructure`,
`compiled`, `artifact_io` and `build_provenance/artifacts`. Runtime output already
carries distinct Candid and Wire hashes; external declaration artifacts still
flow through finalization and provenance. Required external initialization and
provider codecs remain consumers. This is not evidence that every Candid field
is necessary, but it rules out wholesale deletion or relabelling of those hashes.
Removed the exhausted variant-selector payload metadata from `canic::finish!`,
its core declaration record and fixture readers; retain actual ingress limits.

The operator CLI fixture's retained-release gap was corrected in source on
2026-10-26. Previously it supplied only loose schema files. The deleted `operator_cli_protocol` manufactured a fixture-specific release identity. `component_operation/ops/transport` resolves
Root authority through `protocol_binding/release::resolve_release_registry_wire_schema`,
which requires persisted infrastructure/application manifests, finalized schema
hashes and exact installed module/profile identity. The CLI case now reuses the existing production Ensure journey and
`baseline/tests/release_artifacts` sealing owner, with capacity reviewed before
installation for one initial and one operator-provisioned Component. Synthetic
plan/journal helpers and the loose-schema producer are deleted. The retry injector
now selects `component_control::provision::METHOD` and asserts that the reply-loss
marker was consumed. Terminal replay, export and schema-tamper assertions remain.
Source checks pass; compilation and installed acceptance remain deferred.

## Ordered execution checklist

The final A–E reconciliation above owns current status. Dated audit notes below
retain intermediate findings; their then-pending checks are superseded by the
recorded final results, not additional unfinished release requirements.

### A. Native protocol identity pipeline — locally qualified

Owners: `canic-core/src/role_contract`, `canic-host/src/canister_build`,
`build_provenance`, `release_set`, and their metadata consumers.

- Trace every Candid hash/profile field from generation through runtime metadata,
  manifests, provenance, cache admission, installed registry, frontend and CLI.
  Classify each as owned protocol identity, required external interface identity,
  or separately owned storage/authority encoding.
- Derive the owned compiled profile from the canonical native schema contract.
  Establish a non-circular build order: native declaration/schema production,
  profile derivation, runtime compilation with that profile, final-Wasm/schema
  admission. Do not include a final-Wasm hash in its own embedded identity.
  Check the published Wire producer API supports this ordering before changing it;
  report an exact upstream limitation if it does not.
- Apply the same order to configured App, Root, Coordinator, Store, batch builds,
  audit Root and Blob dedicated/embedded consumers. Bind the same recipe/features.
- Update manifests, persisted current-release records, provenance, retained schema
  resolution and generated bindings together. Distinguish raw installed module
  hash, compressed upload hash and Wire schema hash.
- Delete exhausted owned Candid profile build passes, caches, fields and adapters
  after all consumers migrate. Retain only classified external protocol needs.

Exit: no owned compiled profile depends on DID bytes; all normal build variants
have one coherent identity path; mismatched schema/module/profile/features refuse;
cache reuse and retained retry authority preserve those exact identities.

### B. Endpoint and consumer coverage — owned cut qualified; CT-06 blocked

Owners: runtime/control-plane macros and workflows, Host/CLI, Blob adapter,
`canic-testing-internal`, `canic-tests`, generated frontend templates.

- Inventory actual declarations and callers by family: managed App, Root,
  Coordinator, Store, Blob dedicated/embedded, Auth, Backup/checkpoint, audit and
  test canisters. Record method identity, request/response owner, budgets,
  authorization, caller, schema producer and acceptance case.
- Derive the inventory from declarations/call sites; do not create a second
  production method roster. For remaining Candid classify the exact independently
  defined external contract. Owned lifecycle values use Wire; persisted records
  follow their separate storage contract.
- Finish remaining owned direct calls, empty requests, response/error decoding,
  generated clients and fixture calls. Preserve traps, callbacks, caller/account
  bindings, paid-effect ordering, limits and original-operation replay.
- Check macro defaults and feature-dependent exports for accidental Candid lanes,
  variant-layout drift, dev-only feature unification and receiver-only issuer
  dependencies. Remove obsolete adapters/derives only after tracing consumers.
- Review the current dirty batch for disconnected helpers, duplicate transport,
  old selectors and misleading docs. Fix direct migration fallout; park unrelated
  refactors with an explicit reason instead of expanding this batch indefinitely.

Exit: every owned endpoint family has an actual migrated consumer and declared
qualification case; remaining Candid has a concrete contract owner and reason.

Follow-up caller audit (2026-10-26): funding release census now compares the
actual method-bound Wire frames across replay, then projects decoded evidence for
payment assertions. Top-up history stays in its native DTO through comparison;
no Candid serialization is needed to prove unchanged history. The remaining
management-status conversion fixtures bridge PocketIC's `ic-management-canister-types`
0.8.0 to the Platform-exposed 0.11.0 types; keep them until their exact upstream
identities align. Provider fixture replies retain Candid; managed lifecycle
initialization uses Wire and fixture import records use canonical storage. The macro's Candid
adapter still serves external ICRC and bounded-adapter qualification; do not remove
its decoder merely because owned endpoints select Wire. This review is source
classification, not installed qualification or completion of the full inventory.

Generated source audit also found the release-binding Wasm probe constructing
DIDL bytes manually. Its template and two host reads now use native Wire with
one shared method/limits definition; its temporary workspace copies the current
Wire dependency declaration rather than pinning an independent version. The
finalization fixture retains an empty external service declaration, with no owned
Candid query. Installed malformed/wrong-method rejection assertions join the
existing exact binding, mismatched initializer and reinstall checks. Execution
remains deferred; this probe is not a production schema producer.

Production macro inventory (2026-10-26): a balanced-attribute scan of
`crates/canic/src/macros` finds 186 typed/raw Wire declarations and two typed
Candid declarations, `icrc10_supported_standards` and
`icrc21_canister_call_consent_message` in `endpoints/standards.rs`. The latter
implement external standards. The Blob lexical emitter selects Wire for owned
methods; its Candid certificate arm is supplied by Blob Contracts 0.48.0's
explicit external `_immutableObjectStorageCreateCertificate` descriptor. Provider
GC remains external Candid. Default/bounded Candid endpoints in
`payload_limit_probe` qualify that adapter, not an alternate owned protocol.
These are dated descriptive counts, not CI membership requirements or runtime
qualification. Remaining caller inventory is Host transport and generated source;
remaining artifact audit is exact retained external/lifecycle evidence.

Host caller audit (2026-10-26): removed `decode_json_result_response` and the
unconsumed Agent Candid query adapter after tracing their only remaining test
callers. External Ledger/management transports retain plain Candid decoding.
Release pool/funding fixtures now construct native snapshot/status replies instead
of removed selector enums. Chunk planning was still enforcing a Candid size;
it now encodes the exact native `PublishRequestRef` with `template_bytes` method
identity and request limits used by submission, retaining the chunk envelope cap.
The existing two-chunk and oversized-label regression follows that same native
boundary. Source checks pass; compilation and installed acceptance remain deferred.
The later accepted completion boundary above also migrates retained registry
snapshots, receipt/observation identities and managed initialization bytes.

Artifact follow-up (2026-10-26): `build_provenance/artifacts` requires native
schema and TypeScript artifacts, while separately recording an existing Candid
artifact. `ProtocolProfileProvenanceV1` carries Wire schema/profile identities.
`wire_schema::acquire_schema` re-admits cached schemas against final Wasm and
compares retained bindings with regenerated output; release artifact sealing also
re-admits the schema. These inspected paths are connected rather than an abandoned
second owned protocol pipeline. Keep required external sidecars and their hashes;
qualification still needs to execute on regenerated outputs.

The funding recovery probe's retired Coordinator method string also affected
its call-cost model. It now uses the current Contracts funding method constants
for Root acceptance and Coordinator requests. Its 16 KiB conservative payload
bound and recovery flow are unchanged; only a Contracts dependency edge was added,
with no selected package version/checksum change. Source checks only.

### C. Generated and packaged delivery — locally qualified

Owners: Host generators/build recipes, three Cargo workspaces, frontend package,
fixture regeneration, active docs and existing 0.112 changelog draft.

- Update generators before regenerating their outputs. Align main and both Blob
  consumer manifests/locks with the frozen published graph; no sibling overrides.
- Once A/B source is complete, regenerate native schemas, frontend bindings and
  required embedded artifacts through their owning commands. Refresh the embedded
  allocation fixture once against the final inputs, then verify it.
- Retire obsolete owned DID delivery only where A/B prove no consumer remains.
  Keep necessary external Candid fixture assets and storage codecs.
- Inspect package inclusion, artifact recipes, cache invalidation, release hashes,
  frontend exports and independent-workspace feature selections.
- Consolidate active examples and 0.112 release notes around the complete outcome,
  including reinstall-only behavior, changed commands and unavailable features.
  Do not allocate a release per implementation slice.

Exit: shipped/generated surfaces come from maintained generators and agree with
the selected source graph; no stale embedded fixture or unpublished path dependency.

### D. Integrated result — targeted qualification complete

Check shared target ownership before each compile sequence. Capture exact source /
lock inputs and commands. Use owning package/test filters, never a broad suite
inferred from continuation. Choose actual registered test names after B's inventory.

| Layer | Required evidence |
| --- | --- |
| Source/gates | Scoped formatting, dependency-role and deployment exclusions, layering, endpoint admission, crypto/runtime identity and changed release-contract guards |
| Native | Focused checks and strict Clippy for changed runtime/contracts/macros, Host/CLI, adapter and relevant test-feature combinations |
| Build/release | Exact schema/profile/module binding, changed/missing identity refusal, cache hit/miss, final artifact admission and embedded fixture verification |
| Wasm | Relevant production artifacts and both independent Blob consumer graphs; no host parser or dev-feature dependency leakage |
| Blob | Real dedicated/embedded endpoints: installation, storage operations, account/provider/funding integration, bounded paid effects, lost replies and same-operation recovery |
| Fleet/operator | Ensure/bootstrap, provision, uploads, inventory/readiness/metrics/endpoints; component/subtree/root removal and cycle conservation; interruption and effect-free terminal replay |
| Retained auth/checkpoint boundaries | Preserve authorization and refusal/recovery behavior for endpoints retained by the Wire cut; production portable-session success and live capture are deferred |
| Browser | Generated query/update clients, budgets and errors, authenticated calls to installed fixtures, schema mismatch rejection and dependency graph |

Repair compiler, Clippy and fixture fallout across the affected batch before
reporting completion. Add missing installed coverage where B identifies a real
hole; do not replace installed evidence with parser/unit tests. Re-run checks
invalidated by fixes, not the entire matrix after every editorial change.

Exit: the required focused evidence passes on the completed inputs, or each
failure has a precise owner and reproducible blocker. PocketIC is local installed
qualification; it is not production IC, native macOS, provider or TEE evidence.

### E. Final readiness review — local review complete; CT-06 blocked

- Reconcile A–D against the actual working tree and original Wire acceptance
  contract, not only the latest edited files. Ensure no required row is omitted.
- Review the complete open changelog, package surfaces, lockfiles and generated
  artifacts. Summarize changes and failed/unrun checks truthfully.
- When explicitly authorized to post, update #613 and relevant Blob trackers
  with actual evidence; retain Auth work as deferred rather than resolved. No
  tracker messages were sent by this local continuation. Close only
  when resolution is present on the default branch and the issue's acceptance
  contract is satisfied. Local source work does not satisfy that criterion.
- Report separately: source batch complete; focused qualification complete;
  release metadata ready; external capability gates; publication not executed.
- The maintainer chooses the governed release command and its validation boundary.
  Do not manufacture release receipts or claim an unrun full suite passed.

## External and scope gates: not hidden implementation leftovers

| Item | Current treatment | What resolves it |
| --- | --- | --- |
| #637 browser Platform | Published `@tokostudios/ic-platform` 0.1.0 adopted; authenticated fixture calls work | Complete authenticated frontend/observatory journey passes on refreshed graph; fixture now restores tampered schema bytes |
| Live Backup | Explicitly deferred; retain live-create refusal | Future separately selected capture/restore integration; not a Blob/Wire release blocker |
| Production portable Auth | Explicitly deferred; application admission and unqualified clock/custody profiles remain closed | Future production integration and qualified deployment evidence; not a Blob/Wire release blocker |
| Qualified TEE custody | Never imply software tests qualify hardware or silently downgrade | Required qualified deployment evidence when hardware becomes available |
| Live Blob provider acceptance | Local framework integration can be qualified independently | Explicit deployment/provider authority and bounded spending plus actual live evidence; no invented account or budget |

None of these permits silently weakening a contract. Conversely, unavailable
optional deployment capabilities must not be confused with an unbuildable framework.
The final report must state which capability scope is ready; no global “100%”
claim while live acceptance or explicitly required production paths are absent.

## Execution cadence and stop rules

A continuation resumes this checklist across files and subsystems. It does not
stop after one endpoint, one deletion, one test or one successful command.
Give short progress updates while working; finish the whole currently actionable
package and continue to its dependent work. Preserve a compact ledger here instead
of appending a historical narrative to the current handoff.

Stop only at completion, a concrete upstream/authority/product decision that
prevents further independent work, or an execution/session limit. If interrupted,
record exact remaining checklist entries and evidence so the next continuation
resumes rather than re-audits completed work. No new dependency churn, speculative
cleanup or unrelated issue hunt while these required entries remain.

## References

- [Accepted Wire contract](../../../design/0.112-standalone-blob-service-extraction/wire-hard-cut.md)
- [Owning integration status](../../../design/0.112-standalone-blob-service-extraction/status.md)
- [Current handoff](../../../status/current.md)
- [#613](https://github.com/dragginzgame/canic/issues/613)
- [#637](https://github.com/dragginzgame/canic/issues/637)
- [Platform browser supply](https://github.com/dragginzgame/ic-platform/issues/57)

### Packaged CLI consumer source correction — 2026-10-26

The extracted-package CLI consumer now declares the native schema producer and
uses one copied protocol module for its generated application and installed Wire
caller. Remove its direct Candid/CDK declarations; retain Platform Candid for
lifecycle/external adapters. Shell syntax and scoped formatting pass; compilation,
schema generation and installed execution remain deferred. Evidence:
`/tmp/canic-packaged-wire-20261026/`.

The separate packaged-Wasm-store script now generates a native-principal Wire
probe and native schema producer, admits the schema against compiled Wasm in both
local/IC environments, and checks exact method identity and byte limits. Fleet
artifact checks require Wire JSON/TypeScript alongside retained Wasm/gzip/DID.
Remove obsolete delegated-token config and direct Candid/CDK fixture dependencies.
Generated TOML, generated Rust formatting, shell syntax and scoped ShellCheck pass;
the schema assertion rejects missing/duplicate methods and altered ID/kind/limits.
The CLI fixture's generated TOML and copied protocol also pass source checks.
Evidence: `/tmp/canic-packaged-store-wire-20261026/`. No compilation or runtime
execution ran. These two known generated-consumer source gaps are corrected;
artifact regeneration and installed qualification remain outstanding. The complete
release batch remains unqualified and not push-ready.

### Focused qualification update — 2026-10-26

The dedicated managed Blob role now delivers final Wasm, a 55-method native schema
and TypeScript through the production builder. Log:
`/tmp/canic-wire-dedicated-build-20261026-r3.log`. Earlier attempts refused changed
build inputs; the unchanged retry is a successful build, not proof the transient
cause is fixed. No mutation guard was weakened. Focused compiler fixes and check
logs use `/tmp/canic-wire-qualification-20261026/` and
`/tmp/canic-wire-*-20261026*.log`. Current evidence is recorded in the handoff;
historical source-only statements above describe their original slices.

The complete four-artifact dedicated build passes after correcting infrastructure
precompile role selection and macro-forwarded Result recognition. The installed
journey reached allocation but found stale embedded Candid fixture bytes; the
owning refresh and verify now pass. Continue the installed retry, not another
source-only audit. Current commands, logs and remaining boundaries are in
`docs/status/current.md`. No full-suite or deployment result is claimed.

### Installed Blob/Wire continuation — 2026-10-26

The current frozen graph passes the complete frontend/observatory journey, both
Blob browser compositions and fast/release binding at installed endpoints. The
frontend fixture restores the schema it deliberately corrupts. Release-binding
fixture generation inserts the Wire catalog table before use and selects the
prepared optimizer explicitly. Testkit 0.68.7 and refreshed allocation peer are
qualified. Evidence: `/tmp/canic-blob-wire-long-20261026/`.

The first embedded browser build refused native-producer input drift; unchanged
retry passed, without source/manifest/lock changes during that retry. The cause is
unresolved. Generated packaged delivery and operator acceptance remain open;
these focused passes do not close all of C–E or establish live-provider evidence.

Generated Root delivery from extracted packages now passes (`packaged-root-r1.log`),
including native schema and bindings, exact generated package dependency admission
and Wasm finalization. Its resolved upstream graph matches the selected versions.
This is one packaged Root proof, not the whole packaged CLI/Fleet matrix.

The operator Component CLI journey passes on the current graph (`operator-r2.log`):
real ICP identity/calls, review refusal, lost-reply recovery, terminal export,
transport-free replay and tampered-schema refusal. Its wrapper supplies the actual
PocketIC URL/root key to network discovery. Packaged CLI qualification now enters
the existing private scratch owner before starting its installed case.

Extracted-package CLI acceptance passes (`packaged-cli-r4.log`): generated build,
cache reuse with identical release identity, installed four-artifact deployment,
injected pre-install/lost-install/lost-controller recovery and effect-free terminal
replay. Cold build took 704.8 seconds; installed case 814 seconds. Both packaged
App generators now explicitly declare empty Canic runtime features. Scratch-runner
cache auto-selection retains its executable. Prior failed attempts are evidence,
not passes. Three Host migration lints are corrected; five Auth CLI draft lints
remain owned by the concurrent simplification work (`clippy-r1.log`).

Final strict-lint qualification is incomplete: the Host retry encountered the
concurrent `config::provisioning_grants` extraction while its consumers were being
edited (`host-clippy-r2.log`). Preserve that work and rerun the affected lint only
after inputs settle. Do not invalidate earlier package/installed proofs merely
for editorial changes, but new runtime changes need their owner's qualification.
No further shared-target build was started after detecting the moving inputs.

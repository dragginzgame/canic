# Toko Wasm size: Canic and application audit

Date: 2026-09-28. Outcome: repository-wide source and resolved-feature audit
complete; new optimized Toko byte measurements remain unperformed.

Canic can help, but there is no unused default-feature bundle that will make
`project_instance` small. The best bounded Canic opportunity is separating blob
client contracts from the implementation. The larger opportunities are narrowing
role-owned persistence, making optional observability genuinely removable, and
the already planned standalone blob service. Toko also has application-owned
removal and splitting opportunities. None has a newly measured byte saving here.

## Follow-up: existing ic-blob-storage implementation

The maintainer's follow-up prompted direct read-only inspection of
`../ic-blob-storage`, at HEAD `44b0d9e8c4ffcab558e41e87bbbe52851f2c35a4`, package
`0.2.15`, with unrelated Unreleased work. Its current status supersedes the older
Canic tracker as evidence of external implementation progress: this is already
a substantial independent library, not merely a bootstrapped repository.

It contains durable service components, shared workflows, explicit Rust clients
for admission/manifests/status/references/downloads, and a browser integration
composed with Caffeine. Its library has no Canic dependency and explicitly avoids
automatic storage registration, memory bootstrap and lifecycle exports.

This changes **C1/C4 ownership and sequencing**: prefer completing the narrow
protocol/client boundary in ic-blob-storage and consuming that from Toko, rather
than introducing a temporary parallel blob API in Canic. Its current single Rust
library exposes DTOs, model, ops, policy and workflow together, with unconditional
ic-memory, ic-cdk, JSON and hashing dependencies and no client-only feature set.
That is dependency coupling, not proof that all service code survives LTO. Measure
the actual client closure; a separate protocol/client package or equivalent
compile-time separation should keep service storage/provider workflows out of
the consumer where they are unnecessary. Its own dependency guide already plans
protocol/client/adapter ownership separation.

The substantial instance saving still requires the blob service to run separately
and Toko to remove its embedded implementation. Linking the replacement library's
service workflows into project_instance does not achieve that architectural cut.
Reuse its existing clients instead of creating another client implementation.
Canic should retain generic deployment/lifecycle responsibilities.

Its `docs/status/current.md` and `docs/roadmap.md` still mark M1–M3 in progress
and M4 managed parity/acceptance not implemented. Production adapters, provider
qualification, operational recovery and actual consumer integration remain open;
local browser/PocketIC evidence is not completed service replacement evidence.
Thus C2/C3/C5/C6 and Toko's non-blob findings remain, while the blob work should
converge on this existing implementation. No current Toko byte saving is established
by the existence or version of that crate. No sibling files were modified.

## Evidence and limits

- Toko: clean checkout `6519b72d2a420564dabaf700fc55f7b8603d9fd3`, workspace
  `0.3.1`, Canic `=0.110.6`, IcyDB git tag `v0.255.3`, Rust `1.97.1`.
- Canic: HEAD `06d73694d4c15a5d2c80469030c8944281334fbe`, package version
  `0.110.46`, with substantial pre-existing uncommitted `.47` work. Current
  source findings are working-tree findings; historical experiments keep their
  original source identities.
- Inventoried all 17 role manifests, 349 Rust files under the backend/fleets
  source trees, checked-in Candid, shared libraries and generators, all three App
  configurations, build wrappers, and relevant frontend/operator consumers.
  Source counts include tests and comments and are not executable-byte estimates.
  Reviewed retention paths and candidate boundaries; this is not a line-by-line
  correctness/security certification of every application function.
- Resolved each role independently with locked, offline Cargo trees for
  `wasm32-unknown-unknown`, normal dependency edges. Also resolved the instance
  with defaults disabled. Proc-macro subgraphs still appear in these trees:
  dependency presence alone does not establish runtime retention.
- The downstream inventory, source hashes and dependency-tree snapshots were
  removed from Canic on 2026-09-29. The findings here describe the recorded
  checkout above; current application evidence belongs in the downstream repo.
  An initial tree attempt could not materialize IcyDB in the read-only shared
  cache; an isolated temporary Cargo home resolved all trees successfully.
- No current Toko Wasm artifacts were available. No Toko source/config/lockfile
  edits, compilation, deployment or live observations ran. No Canic runtime
  edits or broad validation ran. Only this audit and its evidence were added.

The old Toko observation of 10,275,629 code bytes and 268 methods is historical
pressure evidence, not this checkout's baseline. See the
[B1 input ledger](../../../working/0.110-fleet-runtime-contraction/b1-input-evidence.md).
The present checked-in instance DID enumerates 238 methods; checked-in files
can be stale, and this count is not a freshly generated interface or Wasm export
count. Do not infer a byte improvement from the different method counts.

Track executable code-section bytes, defined functions, raw bytes, gzip bytes,
data/custom sections and table entries separately. Smaller gzip or Candid
metadata does not itself improve code-section/function headroom. Current
[ICP resource documentation](https://docs.internetcomputer.org/references/resource-limits/)
lists a 12 MiB code ceiling, while the
[execution-error reference](https://docs.internetcomputer.org/references/execution-errors/)
still states 10 MiB and 50,000 functions. Canic's current supported build guard
uses 10 MiB and 50,000 defined functions. This audit does not change that contract
or establish the limits of a selected replica.

## Resolved role coverage

All roles use `default-features = false` on Canic. In the isolated normal
dependency graphs, only the application Root package selects the control plane.
The instance's test-only `control-plane` declaration does not enter its normal
graph. Shared `toko` no longer depends on Canic; role-local adapters already
avoid that source of global feature unification.

| Role | Canic feature purpose | IcyDB SQL selected | Primary audit disposition |
| --- | --- | --- | --- |
| project_instance | Delegated verification, Root signature verification, blob storage/billing | No | Main size target; persistence, blob service, diagnostics, application surface |
| project_hub | Delegated verification, Root signature verification, blob billing | Yes | Blob client-only split; SQL selection |
| project_generator | Delegated/ECDSA/Root signature verification | Yes | Generator already extracted; SQL and generated adapters |
| project_ledger | ECDSA and Root signature verification | Yes | SQL, generated rows and settlement codecs |
| project_registry | ECDSA and Root signature verification | Yes | SQL and optional observability |
| project_user | Base runtime | Yes | Minimal application wrapper; SQL and role necessity review |
| user_hub | ECDSA/Root signature verification, sharding | Yes | Preserve sharding; SQL selection |
| user_shard | Delegated verification, issuer signing, Root signature verification | Yes | Issuer is real; preserve signing, review SQL/diagnostics |
| discovery_hub | Sharding | No | Preserve pool ownership; optional observability |
| discovery_shard | ECDSA and Root signature verification | Yes | SQL selection; keep attestation checks |
| market | ECDSA and Root signature verification | Yes | SQL; real payment/recovery behavior |
| notification | ECDSA and Root signature verification | Yes | SQL; small application surface |
| oracle_pokemon | Base runtime | Yes | SQL; external JSON/HTTP work belongs here |
| oracle_registry | Base runtime | No | Minimal wrapper; role necessity review |
| http_hub | Base runtime | No | Minimal wrapper; role necessity review |
| http_worker | Base runtime | No | Minimal wrapper; role necessity review |
| root | Control plane, root signing and verification | No | Separate artifact; shrinking it does not shrink the instance |

IcyDB metrics is selected across the graphs through the workspace dependency,
including through shared `toko`/`design`. That does not prove every binary retains
the entire metrics implementation. Eleven roles select SQL; the size-critical
instance does not. The Root row describes its checked-in package, not proof that
the current canonical Canic builder deploys that application Root source.

## Ranked Canic work

### C1 — Separate blob protocol/client types from storage and billing ownership

**Best small, concrete feature-boundary correction.** Toko's
`fleets/toko/project/hub/src/ops/blob_storage.rs` only imports blob DTOs and uses
`Call` to contact project instances. It does not invoke a local `BlobStorageApi`.
Nevertheless its manifest enables `blob-storage-billing`, which enables
`blob-storage`. Both appear in the resolved graph.

The coupling exists in Canic now and in the pinned `.6`: billing DTOs and domain
re-exports in `crates/canic-core/src/dto/blob_storage.rs` require the implementation
feature. `crates/canic-core/Cargo.toml` makes billing imply storage. A client
cannot currently select those contracts alone.

Expose passive request/response types independently of the implementation,
either without a feature gate or through a small contracts/client package in
the planned extraction. Keep stable storage, Cashier orchestration and endpoint
emitters in the implementation owner. Do not create a second protocol or DTO copy.
The Hub can then drop both implementation features while retaining its behavior.

Expected direct beneficiary: **project_hub**, not project_instance, which really
owns live/deletion state, certificate creation, billing configuration and funding.
Measure the Hub after optimization: the enabled feature graph proves coupling,
not a retained byte total. Current Canic already gates blob storage structures,
so an unused implementation could partly disappear under LTO.

Acceptance: a client-only consumer compiles with exact DTO/Candid equality;
optimized feature/symbol evidence confirms the resulting closure; real Hub-to-
instance status/sync/funding behavior remains valid. Fold this boundary into
the standalone blob contract if that avoids disposable intermediate APIs.

### C2 — Narrow the persisted Fleet record used by application roles

**Largest existing attribution signal for Canic-owned runtime work.** Current
`crates/canic-core/src/storage/stable/fleet_activation/mod.rs` stores one
`FleetActivationRecord` containing application state alongside optional Root and
Store authority, cascade manifests and credential manifests. An `Option::None`
value does not remove its decoder's other type branches.

Split owner-specific authoritative records and their codecs at the compile-time
role boundary, keeping only the activation/binding/directory state a workload
actually needs. Preserve validated identities, immutable release bindings,
interruption recovery and same-release restore. Selection must remove the
unwanted typed record and decoder reachability, not merely skip initialization
or move files into another crate.

B2 already implemented role-selected memory initialization and split auth state
into local-application, issuer and Root owners. Do not propose those completed
changes as new work or use the old monolithic auth experiment as a present-day
savings estimate. Further record/codec work was explicitly stopped at B3; this
audit recommends reconsideration on evidence, not automatic continuation.

Acceptance: compare a real delegated-verifier/child-provisioning/blob consumer
before and after; prove fresh activation, invalid-authority rejection, same-release
restart and lost-response recovery. Keep the maintained pre-1.0 v1 contract and
reinstall-only release boundary; add no migration/compatibility lane.

### C3 — Make optional observability removable through the role contract

**Medium-sized, reusable candidate with visible product tradeoffs.** The managed
status emitter in `crates/canic/src/macros/endpoints/role.rs` retains public
Metrics/History and controller Logs/MemoryAllocations/Metrics/CycleHistory
providers. Current metric-family dispatch already uses `canic_metrics_*` cfgs;
extend that machinery rather than replacing it with a second selection system.

`RuntimeWorkflow::start_all` also reaches log retention and public sampling.
Public sampling currently checks an empty enabled set at runtime. An empty
publication configuration disables activity; it is not evidence that the sampler,
projection and history codecs disappeared from the binary.

Provide explicit build-selected optional diagnostics/publication surfaces,
with a small mandatory health/readiness/recovery surface. Remove unselected
providers, response variants, timer roots and storage owners together. Consider
separate opt-ins for public history and deep controller diagnostics so a product
can keep current health without compiling every report.

Toko actively uses metrics, cycle history/balance, metadata and logs in
`frontend/src/queries/metrics/useMetrics.ts`, and IcyDB metrics through the user
shard proxy. These are **not unused features**. Choosing a smaller surface needs
corresponding product/UI decisions and regenerated declarations. A status endpoint
that returns an error while still mentioning every DTO may save less than removing
the type/provider root. Keep financial accounting and authority diagnostics needed
for safe operations even when optional presentation is excluded.

### C4 — Use the planned standalone blob service for substantial instance relief

The instance currently owns Canic blob lifecycle and billing, with seven explicit
storage/billing handlers in `fleets/toko/project/instance/src/lib.rs`, plus
`ops/immutable_storage.rs`, remote-asset checks and billing integration. Moving
this complete implementation to a service can remove runtime/storage/codecs from
the instance while keeping a small client.

Moving the same source into a Rust crate linked into the instance does not offer
that reduction. A separate canister/service must own the state and effects.
Account for the client's Candid/auth/retry cost and increased cross-canister
latency; aggregate fleet code may increase even while the instance shrinks.

The [0.111 extraction plan](../../../../design/0.112-standalone-blob-service-extraction/status.md)
already selects this direction. Its service/provider contract, external owner,
publication and final 0.110 human closeout gates remain authoritative. This audit
does not start that minor or authorize external repository changes. Preserve
tenant authorization, upload/read/deletion behavior, bounded payment effects,
lost-response reconciliation and deletion/billing cessation. Do not discard the
only records of outstanding external obligations.

### C5 — Support a size-critical local build through the governed finalizer

Toko already uses `opt-level = "z"`, fat LTO, one codegen unit, aborting panics and
stripping in both Release and Fast. Its wrapper adds Binaryen `-Oz --converge`
for the instance; the aggressive nightly build-std recipe is optional, selected
by `TOKO_CANIC_SIZE_SHIMS`, not the default of `bin/build_project_instance.sh`.

Current Canic's `artifact_io::optimize_release_wasm_artifact` already runs Binaryen
`-Oz` for Release but skips Fast. A supported explicit size-optimization selection
for a Fast/local artifact would eliminate Toko's filename-sensitive `ic-wasm`
shim. Keep it in finalization before hashing/publication and bind the exact tools,
flags and transformation sequence into provenance/cache identity. Compare one
pass with convergence before adopting the extra work. This is a build-contract
improvement, not an unmeasured claim of additional reduction over Toko's recipe.

Do not apply O3 after the size pass. Do not silently strip panic diagnostics or
switch toolchains globally. Do not post-edit finalized release Wasm without
regenerating its attestation and hashes. The `.47` size admission work improves
early failure reporting; it does not make an oversized module smaller.

### C6 — Inspect repeated endpoint wrappers only after the larger roots

The instance has 226 source Canic query/update attributes, many distinct DTOs,
delegated-token guards and IcyDB request wrappers. Canic can keep shared dispatch,
logging, payload checking and error mapping out of unnecessarily generic helpers.
Compare representative repeated signatures and same DTOs after optimization;
LTO/Binaryen may already fold them.

Do not replace typed endpoints with one enormous command enum just to reduce the
export count: all branch codecs may remain. Candid wire encoding still requires
type-specific work even when declaration generation is moved to build time.
The existing `finish!` uses a dedicated declaration pass, and B1 found no executable
benefit from removing endpoint Candid type construction. Preserve predecode bounds
for canister-origin calls; the earlier raw-adapter experiment was only 967 code
bytes in its fixture. Neither is the first optimization target.

## Existing measurements that inform this ranking

These are independently optimized **canonical App** ablations against immutable
`v0.110.5`, not Toko predictions or safe production patches. Rows overlap.

| Experiment | Code bytes removed | Defined functions removed | Interpretation |
| --- | ---: | ---: | --- |
| B1 row 3, activation persistence family | 288,120 | 187 | Record/codec/mapper family worth narrowing; behavior removed by the experiment |
| B1 row 4, authorization persistence integration | 147,812 | 79 | Historical signal; later auth ownership work already changed this area |
| B1 row 5, shared bounded CBOR callers | 696,139 | 406 | Strong serialization signal; includes overlapping persistence families |
| B1 row 12, metrics providers | 28,256 | 53 | Read-side projection cost alone, not all observability |
| B1 row 16, status projection | 95,157 | 181 | Wider status family; overlaps metrics and other providers |
| B1 row 8, Candid declaration construction | 0 | 0 | Metadata reduction was not executable-code reduction |

Sources: [row 3](../2026-09-04/artifacts/wasm-ablation-b1-03/artifact-metrics.tsv),
[row 4](../2026-09-04/artifacts/wasm-ablation-b1-04/artifact-metrics.tsv),
[row 5](../2026-09-05/artifacts/wasm-ablation-b1-05/artifact-metrics.tsv),
[row 12](../2026-09-23/b1-row12-measurement/artifact-metrics.tsv),
[row 16](../2026-09-23/b1-row16-measurement/artifact-metrics.tsv), and
[row 8](../2026-09-22/b1-row8-measurement/artifact-metrics.tsv).

The later [B2 closeout](../2026-09-23/b2-storage-closeout.md) measured an App
reduction of 25,609 code bytes but small increases in several other roles.
The [final residual inventory](../../../release-lines/supporting/0.110-fleet-runtime-contraction/residual-inventory.json)
still finds storage/serializer and observability families in optimized artifacts;
its named-body totals overlap and are not removable-byte budgets. There is no
basis to promise that merely selecting `.46` saves the instance a particular
number of KiB or even produces a net reduction.

## Toko-owned opportunities and exclusions

| Candidate | Evidence and action | Effect on project_instance |
| --- | --- | --- |
| Correct ineffective local blob switches | Instance and Hub dependency features are unconditional; no-default instance tree differs only at its root line. Wire local switches to dependency features and gate all consumers if a blob-free product is desired. | No saving from today's switch; removing remote media/billing is a product change |
| Remove local-only seed code from production | `seed_billing_test_data` is exported; its large `ops/billing/seed.rs` implementation uses a runtime environment guard, not compile exclusion. Gate the endpoint, DTO and implementation together. | Concrete direct removal candidate; size unmeasured |
| Make SQL opt-in for production | Eleven roles default to SQL and declare `icydb_sql_query(introspection = true)`. Gate declarations and select intended admin builds explicitly. | None in its isolated graph: SQL is already absent |
| Select IcyDB diagnostics per role | Workspace selects `metrics`; instance emits controller metrics/reset and a role-authenticated admin proxy. Preserve the UI or deliberately change it when removing these. | Possible direct saving; owned by Toko/IcyDB, not Canic |
| Reduce typed database/DTO duplication | `design_macros` emits per-entity insert/update/delete adapters; shared `database.rs` has many entity-generic wrappers; the instance adds more typed adapters and many response shapes. Keep large common processing non-generic and compare generated cohorts. | Potentially material but requires generated/optimized attribution |
| Further application split | Generator and ledger are already siblings. Remaining instance code includes asset management, authoring, collections/tokens, storefront/vendor claims, billing analytics and NNS/SNS stake checks. Evaluate moving authoring/assets or analytics as coherent owners. | Largest product-level option; additional RPC/client/recovery costs |
| Product feature removal | Neuron-gated claims, local HTTP/media, detailed analytics and authoring are candidates only if the product can omit them. Gate whole schema/handler/workflow roots, not just UI buttons. | Direct but deliberate loss of functionality |
| Trim shared dependencies | `toko` lists `ic-ledger-types`, with no `ic_ledger_types` source reference found; it brings CDK 0.19 alongside 0.20.2 into every role graph. Confirm removal by compile and artifact comparison. | Likely build/dependency cleanup; old unused CDK may already be eliminated |
| Retire minimal unused roles | HTTP Hub/Worker and Oracle Registry are tiny wrappers; Project User is primarily a DB/SQL wrapper. Verify actual topology/product need before dropping them. | Reduces fleet artifacts/canister overhead, not automatically instance code |

IcyDB's pinned generator already selects the canister's schema fragment and its
owned stores/entities (`ActorBuilder::get_entities` filters by store owner).
Do not assume every shared `design` entity is generated into every canister.
Moving source to separate crates without removing registration/call roots does
not establish a saving. Existing historical per-entity estimates cannot be
multiplied by the current entity count.

Both SHA-2 0.10 and 0.11 appear in the instance's normal graph. Canic/upstream
certification uses 0.10; Toko and Canic's k256 0.14 path use 0.11. Changing only
Toko's SHA-2 version will not eliminate the duplicate cryptographic stack.
Likewise Candid `printer` is enabled through several consumers; Canic already
disables Candid defaults at its workspace boundary. Nat/Int are real wire types.
Audit the whole target graph before claiming that disabling one default removes
the printer or bignum implementation.

Canic's delegated verifier really needs chain-key ECDSA for the Root proof and
IC canister-signature verification for issuer proofs. Role attestations also
use IC signature verification. Root and issuer verification flags share that
dependency: deleting one label does not necessarily remove the crypto. Current
`.46` additionally requires ECDSA for attestation-cache configuration validation.
Do not remove proof checks, required cache validation, paid-effect journals,
payload bounds, memory restoration or same-release recovery to win bytes.

Host-only TOML handling, proc-macro dependencies, PocketIC dev dependencies and
the Root control plane should not be counted as instance Wasm bytes. There is
one resolved ic-memory version (0.12.3) and one ic-timers version (0.7.0) in the
pinned graphs. Current Canic/IcyDB integration uses newer coordinated versions;
an adoption experiment must preserve one memory/timer owner.

## Recommended next batch and measurement boundary

First establish a fresh, hash-bound instance baseline for this exact Toko source
and a separately qualified matched Canic/IcyDB candidate. Moving from `.6` to
current Canic is an API/config/declaration adoption, not a guaranteed one-line
version bump. Toko still consumes `canic_status`; current Canic has distinct
public/authorized observability surfaces. Regenerate and adapt callers deliberately.
Any later deployment follows the governed clean-reinstall path.

Then measure independent candidates in this order:

1. Blob client-contract separation on a Canic-owned Hub-like consumer, plus
   downstream Hub feature reduction when separately authorized. Pair it with
   Toko's production seed-code exclusion as a separate application measurement.
2. Optional diagnostics/publication selection on an endpoint-heavy verifier
   fixture. Record which UI/operator features are removed versus retained.
3. A role-specific activation-record prototype, retaining the full same-release
   safety contract. Resume deferred codec work only as an accepted new batch.
4. The planned complete blob-service extraction after its existing design gates;
   measure the instance client and service separately.
5. Application-owned authoring/assets/analytics separation if the remaining
   instance headroom is insufficient.

For each candidate, hold compiler, target, profile, feature closure, generated
configuration, release binding and optimizer sequence constant. Retain two clean
optimized repetitions, exact source/tool hashes, code/function/raw/gzip/data/table
vectors, exports and Candid. Use symbol and call-root analysis for explanation,
then measure the marginal optimized result; never sum overlapping family totals.
Compare combined changes again rather than adding their individual deltas.
Use Canic's existing 512 KiB code reserve as a proposed acceptance target and
record defined-function reserve independently against the selected contract.

Run focused positive/negative tests and the relevant same-release interruption
proof for the owner changed. No full-suite pre-run is implied by this proposal.
The audit itself is ready for review; it does not change the existing `.47`
implementation/release readiness or satisfy the human minor-closeout gate.

## Selected implementation follow-through

The maintainer subsequently selected three bounded Canic changes. The
[scope amendment](../../../../design/0.110-fleet-runtime-contraction/2026-09-28-toko-size-follow-through.md)
records that authority; the current handoff owns completion readiness.

- Activation persistence now selects a concrete ordinary, Root or Store codec
  from its lifecycle entrypoint. One bounded durable cell remains authoritative;
  a transient view is populated only after a successful durable write or exact
  decode. The encoding carries a schema and role tag and rejects unrelated
  fields, omitted required fields, trailing bytes and oversized records. This
  is a reinstall-only schema cut. Same-release restart remains supported.
- Optional diagnostics, history, log reads and metric reads are selected per
  declared role through TOML, generated capabilities and protocol-profile
  identity. All default to enabled. Narrow relay response types avoid retaining
  omitted provider serializers; unsupported shared relay requests return the
  typed `REQUEST_INVALID` code. Financial observation and recovery remain.
  With both history and metrics disabled, the generated lifecycle does not
  register public sampling. Internal accounting/recording and bounded log
  retention remain; explicitly called application APIs can retain providers.
- Passive blob billing DTOs and value enums no longer require either local
  blob feature. Embedded state/workflows remain feature-gated. This prepares
  client-only consumers without introducing a second service protocol or
  assuming the standalone library is integration-ready.

Toko and `ic-blob-storage` remain unmodified. Downstream adoption still needs
the current Canic API/configuration changes and regenerated Candid bindings.
Toko must explicitly opt out of unwanted observation groups and remove blob
features from client-only dependencies to realize those cuts. Its active
metrics/log consumers must be considered before opting out.

The optimized minimal-fixture comparison isolates optional observability:
both builds use the new role codecs. It does not measure a codec-only delta or
Toko's resulting Wasm. Full canonical-role absence and the historical B3/B4
matrix are not claimed by this bounded follow-through.

### Targeted qualification

All 212 selected native checks pass: `canic-core` activation (46), role contract
(30), configuration schema (67) and bounded-cell (2); `canic` protocol surface
(47) and managed endpoint ownership (10), with no default features; and
`canic-host` metadata (2) and release configuration (8). The protocol checks
include billing DTOs without blob features, the generated lean relay reply
decoding at Root, and typed rejection of disabled reads.

The `icydb_lifecycle_composition` PocketIC integration target passes all 12
cases (201.66s test body, 352s runner including native compilation). It covers
ordinary-role reconstruction, application lifecycle/timer custody, fixture
recovery and retained Store uploads across repeated same-release upgrades.
The exact internal case
`pic::fleet_registry::baseline::tests::root_restart_reconciles_held_store_grant_and_revocation_replies`
also passes (391.32s body including nested artifact builds, 566s runner).
That case retains grant/revocation authority and held-reply reconciliation
across Root restart.

Warning-denied Clippy passes for `canic-core`, `canic` and `canic-host`, selecting
their libraries/tests and all features (38.68s final run). Scoped Rust formatting
passes. Canonical Store and Coordinator declarations were regenerated with the
Host refresh flow and then built through the ordinary no-refresh path. Their
exact extracted bytes are retained, including the extractor's terminal blank
line; the remaining diff passes whitespace checks.

Lean Root and standalone-local Fast builds also pass. Inspection of their
generated Candid confirms the selected local request variants while retaining
health, readiness and financial reads. Root retains its remote observation
contract. The local fixture initially failed admission because its temporary
configuration omitted a Component specification; adding that fixture attachment
resolved the rejection without changing production code. These are compile and
declaration checks, not additional installed-canister measurements.

Native logs: `/tmp/canic-wasm-final-{activation,role,config,bounds,protocol,metadata,release-config,clippy}.log`.
PocketIC logs: `/tmp/canic-wasm-{lifecycle,root-recovery}.log`.
No broad workspace/release gate, version transaction, Git publication or live
deployment ran. Native changes after the PocketIC runs were import cleanup only.

### Final optimized observation

The final `canisters/audit/minimal` pair uses Rust 1.98.1, `ic-wasm` 0.11.1,
Binaryen 132 and the canonical Host Release pipeline. Both runs use the same
configuration path, fixture, feature closure and unbound release-template state;
only the four optional observation switches change. Delegated authentication is
disabled in this fixture. Each setting was built twice in alternating order,
with identical Wasm, gzip and Candid bytes on repetition. Compiler caches were
retained; these are repeated artifacts, not cold-build timing measurements.

| Measurement | Default observations | All four disabled | Reduction |
| --- | ---: | ---: | ---: |
| Code section bytes | 2,623,363 | 2,405,383 | 217,980 (8.31%) |
| Total Wasm bytes | 2,885,159 | 2,635,446 | 249,713 (8.66%) |
| Gzip bytes | 1,034,911 | 942,632 | 92,279 (8.92%) |
| Defined functions | 4,785 | 4,340 | 445 |
| Data section bytes | 214,942 | 196,022 | 18,920 |

Both artifacts retain 38 imports, 11 exports and one table. The
[structured evidence](toko-wasm-follow-through.json) records exact configurations,
artifact/Candid hashes, tools and limits. Its
[source inventory](toko-wasm-follow-through-source.sha256.gz) binds 1,506 relevant
source/package files in the uncommitted candidate, including earlier `.47`
changes. The inventory was checked again against the final source.
Local artifact copies and section inventories live in
`.tmp/toko-wasm-size/final-{full,lean,full-repeat,lean-repeat}/`.

The initial exploratory pair differed by a few bytes before final cleanup; this
table supersedes it. No instruction, cycle or activation-codec-only saving is
claimed, and these percentages must not be applied to Toko without measuring
its actual selected feature graph.

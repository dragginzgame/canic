# Public Status and Protected Observability

Date: 2026-09-07
Scope: the read-surface addition to the open 0.110.10 Canic batch.

## Contract

`canic_public_status` exposes basic health/discovery and explicitly enabled
cached aggregates. `canic_observability` applies one controller authorization
rule to all its variants. The existing authenticated Root relay remains the
operator path to Root-controlled workloads. Publication and player admission
never grant observer authority.

Authentication, admission, durable-operation, Coordinator Registry and Store
catalog reads have separate owning methods. Optional authentication methods
are omitted when their capability is absent. This directly replaces the mixed
workload and Coordinator status contracts; there are no compatibility aliases
or new authority stores. Mutation and recovery owners remain unchanged.

The immutable `public_metrics` selection defaults to empty. Its families are
application aggregates, cycle balances, operation/funding counters, instruction
performance and shard occupancy. Public queries read bounded cached snapshots
with sampling time, availability, staleness and truncation. Existing cycle
observations can refresh built-in families; trusted applications may sample
from an existing update/timer owner and publish additional aggregates. No new
Canic timer or remote query fan-out is introduced.

The implementation propagates through generated role surfaces, protocol/replay
inventories, configured runtime projection, canonical infrastructure Candid,
host and CLI callers, fixtures, operator documentation and the draft changelog.
Prepared-state allowlists include the Store catalog query required during Root
bootstrap and retain distinct role boundaries. The native authentication journey
exposed that missing allowlist entry. The focused policy tests and final runtime
rerun pass, including fresh Root/Store bootstrap and Component activation.
`cycles convert` reads its exact receipt through Root operation status.
Store Candid refresh now reuses its already extracted declaration instead of
trying to extract one from final runtime Wasm. The endpoint renderer regression
checks column contents and alignment independently of fixture-name width.

## Targeted evidence

| Check | Result |
| --- | --- |
| Root, Coordinator and Store fast Wasm builds | PASS; both canonical infrastructure DIDs refreshed |
| Protocol surface | 42 passed, including structural request/response equality against canonical Candid |
| Managed endpoint gates | 7 passed |
| Public metric cache and actual shard assignment projection | 5 passed |
| Read replay manifests and Prepared endpoint policy | 2 and 5 passed |
| Candid refresh | 4 passed |
| Fleet host regressions | 150 passed; ignored deployment case excluded |
| Observer routing | 1 passed |
| CLI inspection, authentication and endpoint rendering | 18, 14 and 2 passed |
| CLI cycle conversion | 20 passed |
| Affected-package all-target/all-feature warning-denied Clippy | PASS, including final policy/fixture corrections and final CLI-only feature configuration |
| `timer_authority` PocketIC target | 6 passed, 12.81s tests / 17s cached runner |
| Coordinator Registry caller/replay PocketIC case | 1 passed, 101.12s including artifact build / 188s runner |
| Native authentication/session PocketIC target | 7 passed, 304.45s including artifact rebuilds / 336s runner |

The runtime publication proof checks outsider public health, default-disabled
metrics, exact family selection, trusted publication, unchanged protected-query
denial, and stale reads retaining the original application snapshot. The shard
projection test uses real local assignment records, checks aggregate count and
capacity, and proves assignment changes are invisible until the next sample.
All fixtures remain application-neutral.

The final CLI-only Clippy run also checks core without sharding. Its empty
shard projection is now a `const fn`; the final warning-denied check passes.
Formatting and diff checks pass. Runtime evidence predates only that
behavior-preserving qualifier and the separately checked CLI receipt route.

Logs are `/tmp/canic-read-*.log`, `/tmp/canic-public-metrics-tests.log` and
`/tmp/canic-public-observability-pocketic.log`. This is targeted source
qualification, not an immutable published validation receipt. No broad workspace
gate, version transaction, Git publication, live deployment or sibling edit was
performed.

## Adoption and limits

Toko Miner must pin the released Canic/CLI, rebuild sealed artifacts, regenerate
bindings and route each read to its owning method. Its public demo selects the
intended `public_metrics` families and connects sampling to an existing trusted
update/timer. The UI should render sample time and stale/unavailable states.
Protected observer reads must remain independent of player admission.

Shard occupancy counts assigned keys; interpreting those as users requires the
application's one-key-per-user contract. Cycle balances, funding transfers,
ICP amounts and instruction counts retain distinct meanings and units. Exact
application cycle costs can be supplied as explicit application aggregates;
Canic does not infer consumption from transfers or instructions. The independent
Coordinator exposes public health/discovery and keeps financial/Registry
observability protected rather than inheriting an App publication setting.

Live application integration, display semantics and sampling cadence remain
for Toko Miner to validate after adoption. The
[maintained read contract](../../../../features/runtime/public-observability.md)
contains configuration and method ownership. The preceding
[canonical Root](canonical-fleet-root.md) and
[retained Fleet](retained-fleet-feedback.md) handoffs retain their earlier
convergence, recovery and conservation evidence. CANIC-141 remains deferred.

The complete accepted canonical Root, retained Fleet and read-surface batch is
ready for release approval. Implementation, targeted qualification, interface
propagation, cleanup and the open 0.110.10 changelog are complete; package
versions remain 0.110.9. No in-scope implementation blocker remains. The
maintainer-selected release gate and publication remain, while downstream
adoption and live qualification are distinct from Canic source readiness.

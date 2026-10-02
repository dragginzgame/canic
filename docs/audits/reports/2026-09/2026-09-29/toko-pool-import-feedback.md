# Toko Miner pool-import feedback triage — 2026-09-29

## Result

CANIC-188 is a confirmed P1 live-deployment blocker. CANIC-187 is confirmed P2
publication friction. Both source corrections are now implemented and qualified in the open 0.110.49
batch. The exact issued-import repair is separately qualified below. The live
incident remains open; no downstream state was changed.

Toko's current dirty feedback and deployment record report frozen v0.3.17
qualification and infrastructure completion under Canic/CLI 0.110.48. Actual
unpaid-review cancellation and the corrected infrastructure funding path have
now succeeded. Full Fleet convergence, terminal replay, frontend publication and
browser acceptance remain incomplete. Staging is partially reset; its older
frontend must not be described as a usable completed deployment.

## CANIC-188: reservation budget exhaustion

Read-only inspection of the retained protected Root status confirms:

| Observation | Exact value |
| --- | --- |
| Sources | 24: six Ready, one Stopped, 17 AwaitingHandoff |
| Import phase / receipt | Reserved / absent |
| Cumulative reservation | 3,999,937,878,090 cycles |
| Root debit ceiling | 4,000,000,000,000 cycles |
| Remaining reservation | 62,121,910 cycles |
| Paid calls | 94 of 400 |
| Initial Root native balance | 400,795,989,905,357 cycles |
| Last retained Root balance | 400,751,487,330,209 cycles |
| Difference between retained balances | 44,502,575,148 cycles |

The apply log reports `Root rejected the capacity import request: E81`.
These are retained downstream observations, not fresh live queries.

The inspected `.48` source established the diagnosis (subsequently corrected):

- `crates/canic-host/src/fleet_ensure/ops/clean_reinstall/mod.rs::import_request`
  derives the paid-call bound as `16 * sources + 16` but fixes the Root debit
  ceiling at 4T cycles regardless of source count.
- `crates/canic-control-plane/src/ops/canister_pool/capacity_import/mod.rs::reserve_call`
  increments `max(reserved_debit_cycles, observed_debit)` by each conservative
  call bound. Completed unused reservations are not released there.
- The workflow reserves both management observations and mainnet-only subnet
  lookups while advancing sources. A local-only successful journey is insufficient
  evidence for the mainnet reservation envelope.
- `reserve` permits an exact existing reservation but rejects changed authority
  while the old reservation is unreleased. Merely increasing a new Host review's
  budget cannot change this already-issued Root reservation.

The required correction was to separate conservative outstanding/uncertain reservations from
settled observed debits; derive and validate a sufficient whole-operation bound
before any source is stopped; expose the exhausted budget and phase in typed
operator diagnostics. Qualify the 24-source path with mainnet-equivalent costs,
bounded retries and interruption cuts.

The issued operation required the separately accepted, evidence-bound recovery
design linked below.
It must preserve custody, exact operation identity, unfinished-effect authority,
conservation and terminal effect-free replay. Do not assume a larger future
planner constant repairs it. Preserve journals and the qualified build; neither
extra funding nor unchanged apply addresses this exhaustion. Do not hand-edit
receipts or reinstall Root to discard the owner. Recovery must respect the
reinstall-only release contract and cannot silently introduce compatibility or
cross-release state migration.

## CANIC-187: formatting-only seed publication

`crates/canic-host/src/fleet_ensure/generate/infrastructure_bootstrap/mod.rs::seed_projection`
previously parsed the supplied TOML, validated physical authority, then unconditionally
serialized it with `toml::to_string_pretty`. An already-resolved physical seed
therefore loses its original bytes despite unchanged values, invalidating Toko's
frozen-source check after otherwise successful infrastructure execution.

Implemented correction: retain original bytes when projection changes no TOML
values, while keeping real identity updates under the existing durable publication
owner. Test both no-op byte preservation and real ID publication/replay. Explain
that policy/seed paths may be output destinations. Toko's dedicated mutable
operator-input orchestration remains downstream-owned.

## Evidence and boundaries

Inspected Toko files:

- `docs/upstream/canic.md`, CANIC-187/188 and September 29 current feedback;
  SHA-256 `d21b522f59f086efd32168b8bd6f58b849a8fff66a96d64daf9520efd1256820`.
- `docs/status/evidence/2026-09-29-staging-0.3.17.md` and current status.
- `.canic/staging-runs/20260929T144210-vkn1e57j/import-failure/` apply log,
  protected status and Root logs. Status SHA-256:
  `7e8bd471a40253e1016b9961712b21bdcf0d1414d9ff2929a978e7ae48fa7a13`.

At initial diagnosis, the implicated implementation matched local HEAD
`8d37c74c9a4457b9e2bd47ee883f98fd2889d63b`; the prior cleanup was separate.
The follow-through changes are uncommitted and retain that unrelated work.

## Implemented corrections and qualification

Whole-operation budgets now derive from a Root call-cost quote and complete call
count, with admission before handoff. Successful callbacks settle only their own
unused allowance; uncertain calls retain their reserves. Typed operator errors
report phase and debit/call bounds. No-op estate seed projections preserve bytes;
real ID updates continue through the durable publication owner.

Targeted native tests, warning-denied Clippy, the real Root import PocketIC journey
and the public CLI completed-estate recovery/replay journey pass. The
[repair decision](../../../../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md)
records the maintainer's narrow exception and exact live preconditions. The
[incident tooling](../../../../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-retired-tooling.md) reproduces a
frozen `.48` repair and qualifies real installation, discarded-reply reconciliation,
replay rejection and restoration of the original Root within its original debit
ceiling. Native tests cover exact evidence rejection and the remaining 242-call
path. The seeded PocketIC record has a synthetic private Stopped observation;
this qualification does not claim to replay the entire historical deployment.

The exact reproduced candidate and original Root are retained in Canic's
`.canic/incident-repairs/canic188/ed3084b6908a04b28effa21e00ec425aaf382d1423849fcbb4f3b12414d61f48/`
with hashes, logs and structured qualification. The local batch is ready for
review. No live recovery, payment, sibling edit, Git publication or external
message occurred. Live completion and immediate effect-free replay remain required
before declaring CANIC-188 closed.

## September 30 urgent publication qualification

The maintainer prioritized the corrective `.49` publication ahead of the remaining
review backlog. An explicit current-runtime PocketIC case now exercises nine
installed Workloads and fifteen empty spares through the public completed-estate
CLI. It passes insufficient-budget rejection without source mutation, lost-install
response recovery, all 24 source imports, exact Root conservation, offline import
replay with unchanged journal bytes, nine-Workload/fifteen-Ready convergence,
terminal replay and a later ordinary Ensure operation.

The final case passed in 322.32 seconds; its governed invocation took 341 seconds.
Import consumed 220 of 784 reviewed calls. Root's observed debit was
51,096,447,856 cycles against a generated ceiling of 33,008,458,000,000 cycles.
The complete log is `target/review-validation/canic188-24-source-final.log`.
Earlier attempts retained beside it exposed two new fixture assertions: an
invalid negative-capacity setup and a Fleet-only receipt field used for import.
They did not require production-code corrections. The incident-sized case is
opt-in; the ordinary release catalogue keeps its smaller recovery fixture.

Final native qualification passes 29 Control Plane, two Core and 78 Host
import/budget tests, plus the exact Host generation journey containing seed-byte
preservation and changed-ID publication assertions. Logs are
`target/review-validation/canic188-native-final.log` and `canic187-seed-final.log`.
Changed Testing-package all-target/all-feature warning-denied Clippy passes in
`canic188-clippy-final.log`. No complete workspace gate or version mutation ran.

This uses the maintained local-network runtime contract and current artifacts;
mainnet call-count/cost arithmetic has separate native coverage. It does not
replay Toko's historical private state or replace the frozen repair qualification.
All 17 retained repair-bundle checksum checks pass. The new local `.48` import
reported with an issued uninstall is a different operation and is not authorized
by the staging-specific repair. Publication readiness and live incident closure
remain separate decisions.

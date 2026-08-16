# Canic 0.102 Implementation Status

Date: 2026-08-17

## Status

- State: Register, Map and the atomic Cut are complete in the untagged
  `0.102.2` draft. The maintained model is: canisters send `u16`, runtime
  reasons are typed and the host owns prose.
- The same `.2` draft stages one flat `code + name` release baseline and
  removes the archived B1 census and initial row counts from permanent test
  authority. The baseline may move freely until `.2` ships.
- Release: the public wire is now exactly `Error { code: u16 }`.
  `InternalError` owns an exact registered code and its explicit public
  projection; it stores no message, class, origin or optional public DTO.
- Superseded work: the 991-row candidate encoded cause together with handling,
  exposure and context. It was never released, so its rows may be replaced
  directly and are neither current nor retired identities.
- Repository scope: Canic diagnostics only; unrelated work neither supplies
  evidence for nor blocks this line.
- Release boundary: 0.102 is reinstall-only. Every Canic-owned canister in an
  activated Fleet must use one admitted release set with matching callers.

The detailed source inventory remains in the working
[0.102 diagnostic audit](../../audits/working/0.102-diagnostic-inventory/index.md).
It is temporary conversion evidence, not a recurring producer-registration
workflow.

## Implementation Phases

| Phase | Outcome | Status |
| --- | --- | --- |
| Register | Review the 167 mechanical buckets by semantic cause; create `reasons.toml`; generate runtime constants and the host catalogue only | Complete: 161 registered and ten local families |
| Map | Map existing typed failures at qualifying boundaries; keep local failures local; select explicit safe public projections | Complete |
| Cut | Make `InternalError` code-first and atomically replace `ErrorCode + message` with `Error { code: u16 }` across Rust, Candid, host, CLI and tests | Complete |
| Clean and measure | Remove dead prose and temporary audit tooling/authority, archive useful conversion evidence, touch only directly affected durable fields and record representative Wasm deltas | Complete: targeted four-role evidence recorded |

Historical B1/B2 labels may remain in audit and changelog evidence. They do not
add phases or architecture to this plan.

## Accepted Evidence

- B1 found 2,895 provisional labels, 3,898 exact producer-qualified entries
  and 31 public projections. These are coverage observations, not code counts.
- The rejected tuple generated 991 rows and 503 singletons. A fresh comparison
  showed 166 canonical conditions, 167 origin-qualified mechanical buckets,
  348 rows after adding broad class, 572 after handling, 664 after projection
  and 991 for the full tuple.
- All 656 dynamic values formerly embedded in public messages are inventoried,
  so operation IDs, deadlines and other meaningful values can be checked before
  prose is removed.
- The retained `CANIC-WASM-001/v3` baseline covers representative Component,
  Fleet Subnet Root, Fleet Coordinator and Wasm Store artifacts from immutable
  tag `v0.101.53`.
- The superseded assets and their tests prove useful raw/registered type,
  lookup and generation scaffolding. They do not approve the old taxonomy.
- Finite typed Access and Store cleanup may be retained, but provisional
  numeric mappings must be regenerated from the reviewed reason register.

## Current Register Review

The code-free review began with 167 mechanical origin-qualified buckets and
initial hints of 157 possible-global and ten possible-local groups. Four mixed
global buckets required semantic splits. The accepted review therefore exactly
partitions all 2,895 provisional identities into 161 qualifying registered
cause families and ten local typed families across 171 reviewed rows. No
exact-owner boundary decision remains open. A
superseded granular pass produced 1,355 rows by splitting subsystem, field,
phase and public target; it remains archived migration evidence and has no
allocation authority. The final number of reasons is not a KPI, but handling,
projection and producer context do not manufacture additional identities.

## Current Decisions

- The only public wire is `Error { code: u16 }`; delete `ErrorCode` and
  `message` completely.
- `DiagnosticCode` preserves any observed `u16` losslessly.
  `RegisteredDiagnosticCode` is the Canic-owned producer identity, with Rust
  privacy as the primary construction guard.
- Codes identify semantic causes, not producer sites or handling. Existing
  typed callers continue making retry, reconciliation and policy decisions from
  their own typed state. 0.102 adds no generic handling framework.
- `InternalError` is code-first. Public projection is explicit at typed mapping
  or construction boundaries and exhaustively tested; there is no central
  projection table and identity is never recovered by matching text.
- Global codes are for public, retrievable operator, durable-evidence or
  machine-decision boundaries. A sensitive exact failure is registered only if
  that identity independently qualifies; otherwise it remains local typed
  state and may map directly to a safe public registered reason.
- `reasons.toml` contains code, name, origin, summary, optional guidance and
  `retired`; the focused guard rejects every other field. It generates only
  runtime constants and the host catalogue.
- Unreleased allocations may change freely. Once released, `code + name` is the
  immutable semantic identity; summary and guidance may change, origin may
  change after review and retirement may move only from false to true.
- No JSON registry is generated until a concrete maintained non-Rust consumer
  requires one.
- A masked registered reason uses an existing suitable exact owner. Without
  one, the exact failure stays local and maps directly to the safe public
  reason; 0.102 does not create a status, receipt, correlation or lifecycle
  subsystem solely to make it qualify.
- Correctness-, recovery- and caller-required dynamic data receives the
  smallest endpoint-specific typed owner when necessary. Nonessential operator
  context may be deliberately dropped and recorded rather than forcing new
  infrastructure.
- Durable work is limited to fields directly disturbed by removal of the
  diagnostic string representation. Independently meaningful operational and
  recovery text remains unchanged.
- B1's exhaustive tooling and allocation authority are removed at closeout;
  useful inventory and review evidence remain archived conversion history.
- General logs, protocol strings and application data remain owned and intact.

## Focused Validation

Historical B1 arithmetic remains archived evidence. Permanent targeted checks
establish unique nonzero ledger rows, released `code + name` preservation,
generated-code drift, raw/registered separation, explicit projection, lossless
remote forwarding, host lookup, the exact one-field Candid wire, and the
affected typed mappings in core and the control plane. Core,
control-plane, host, CLI, Fleet Coordinator and Wasm Store feature builds
compile. The complete broad validation recorded for the earlier release
checkpoint remains historical evidence; it is not a reason to rerun broad
suites during focused 0.102 development.

For the simplified contract, targeted checks must cover only:

- unique nonzero reason rows and no reuse relative to the latest released
  ledger;
- generated runtime-constant and host-catalogue drift;
- raw decoding/forwarding and registered producer construction boundaries;
- typed reason mappings and explicit public projection;
- the exact one-field Candid wire and host known/retired/unknown rendering;
- current encoding/lifecycle behavior for stable records actually changed; and
- representative release-Wasm absence plus closeout measurement.

The targeted [closeout Wasm evidence](../../audits/working/0.102-diagnostic-inventory/inventory.md#targeted-0102-closeout)
builds a representative Component, Fleet Subnet Root, Fleet Coordinator and
Wasm Store through the canonical release builder. All four data sections are
smaller than the retained baseline and bounded scans find no host catalogue or
B1 register material. Root is smaller overall; the other three roles grow
across the non-isolated release-line comparison, so 0.102 makes no causal
diagnostic-savings claim.

## Next Action

The focused `.2` identity guard and migration-authority cleanup pass their
targeted tests and warning-denied Clippy targets. Do not generate JSON, add
handling metadata, restore B1 test authority or create new observability
infrastructure. The maintainer owns full validation when publishing.

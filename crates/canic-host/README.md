# canic-host

`canic-host` contains the behavior that runs on an operator's computer: building
canister Wasm, communicating with IC networks, comparing a deployed Fleet with
its approved plan, checking evidence, and storing local operator state. None of
this crate runs inside an application canister.

Normal operators use the installed `canic` binary. Direct Rust consumers may
use the build and `fleet_ensure` modules when embedding the same current
contract.

Generic host mechanics use the published IC Host Tooling 0.8.8 packages:
`ic-host-artifacts` owns Wasm inspection, gzip encoding/decoding and bounded
stream reads, copying and hashing; `ic-host-fs` owns regular/private reads,
path observations, durable publication and descriptor locks,
`ic-host-process` owns executable resolution, child cleanup and communication, and
`ic-host-tools` owns Candid normalization and ICP JSON/hex envelope decoding.
Direct consumers import reads from `ic_host_fs::read` and publication/locks from
`ic_host_fs::durable`. Canic retains schemas, authority, byte budgets, tool pins,
transaction ordering and interruption recovery.

Release representation checks compare gzip output directly with raw Wasm without
allocating another complete decoded payload. Store publication uses shared chunk
and whole-upload identities; Canic retains chunk sizes, request bounds and order.
Generated lock-seed JSON streams through typed durable publication within its
4 KiB budget, retaining exact pretty bytes and publication/cleanup causes.
Build-input, frontend-file and local executable fingerprints share no-follow
descriptor hashing. Canic retains trusted ancestors, exact frontend file lengths,
aggregate identity framing and the executable's 512 MiB limit.

Foreground replica startup uses shared communication with direct-child ownership,
preserving the command's terminal process group and forwarding stderr live.
It retains the existing absence of a runtime deadline or diagnostic byte quota;
terminal-write failure kills and reaps the direct child with typed cleanup evidence.
Descendant lifetime and paid-effect lock custody retain their existing owners.

Compiled Candid extraction retains at most 16 MiB of stdout and 64 KiB of
diagnostics, with a 120-second capture deadline. Failed or over-budget extraction
returns the shared typed process error and bounded evidence; it publishes no
partial declaration or cache record. Normalization can add one final newline.

Direct response-error consumers match
`IcpJsonResponseError::Envelope(ic_host_tools::response::ResponseError)` for
JSON/hex/envelope failures. Canic retains typed Candid and canister rejection
errors. This is a public hard cut; no old parser or compatibility variants remain.

## Build

```bash
canic build <app> <role> --profile release
```

Every managed package declares exact App/role metadata. Artifact builds are
non-incremental for deterministic Wasm. Cargo owns compiler-wrapper selection:
configure `RUSTC_WRAPPER` or Cargo's wrapper settings when compiler caching is
wanted. Canic runs Cargo once with that configuration and retains compiler
failures. Complete artifact reuse remains independent of compiler caching.

## Fleet Ensure

Production Fleet convergence uses the maintained Ensure owner:

```bash
canic fleet ensure <fleet> --desired <path>
canic fleet ensure <fleet> --desired <path> --apply <plan_sha256>
```

`canic fleet generate --fresh` can first create or replay a durable,
effect-free logical seed for an empty estate. It does not install anything or
own a second mutation path. The generated document enters the same reviewed
plan/apply journal above; create results are retained before dependent
controller and treasury references are resolved.

The host modules follow the strict boundary:

```text
CLI -> workflow -> policy
                +-> ops -> model
```

- `model` owns the current `v1` plan, journal and conservation records.
- `policy` validates desired/live inputs and compiles the immutable plan.
- `ops` owns artifact hashing, current state files and one platform effect.
- `workflow` persists intent, reconciles replay and publishes terminal state.

Supplied infrastructure and capacity use explicit reviewed bootstrap/import
operations before ordinary Ensure. Clean reinstall reviews the selected current
build, complete physical inventory and current controllers, including unfinished
predecessors. It preserves original evidence and reconciles genuinely uncertain
paid or controller effects before publishing new reset authority. See the
[operator guide](../../docs/features/operations/fleet-ensure.md) for these
procedures and unreadable-record handling; historical contracts are not decoded
or migrated.

## Cycle Safety

The plan records exact controlled balances, canister dispositions, scheduled
transfers, fees, funding and bounded burn. Every mutating platform call has a
durable intent first. Ledger and configured drain effects use exact replay
identities. Apply refuses a changed plan, unsafe live drift or a debit/burn
above the reviewed maximum.

A controller cannot pull cycles from an arbitrary canister. Material
physical replacement/deletion therefore requires an exact treasury-bound idempotent
drain endpoint. Without it, policy returns `NoSafeDrain` and leaves the
canister untouched. Stop and delete remain separate effects with fresh status
and residual-balance checks. ID-preserving clean reinstall instead keeps the
selected IDs and their native cycles while clearing application and framework
state under separately reviewed reset authority.

The complete desired document and operator procedure are documented in
[Fleet ensure](../../docs/features/operations/fleet-ensure.md).

## ICP Identity

When ICP CLI uses password storage, pass its supported identity password file
through the individual operator environment. Canic forwards the path to ICP
CLI and does not read or render the password contents. Keep credentials outside
the repository.

Operator Component provisioning is available through `component_operation`:
the host retains exact review and submission authority while Root owns its
lifecycle. See the [Component operation guide](../../docs/features/operations/component-operations.md).

Frontend browser artifacts and external native-cycle preflight are owned by
`frontend`. See the [frontend handoff guide](../../docs/features/operations/frontend-handoff.md)
and [independent SDK consumer](examples/frontend-consumer/README.md).

The [Fleet observatory](../../docs/features/operations/fleet-observatory.md) supplies
independent role observations and bounded public reports from the host.

## Persistent local Fleet

The optional `local-fleet` feature exposes `LocalFleetSession` for a persistent
PocketIC-backed Fleet with separate application subnets, exact release-bound
preparation, ordinary Fleet Ensure convergence and a fixed browser gateway.
The packaged `local_fleet` example uses only public library APIs. It accepts
bounded JSON-line commands for seed/generation, convergence, current role/subnet
discovery, frontend export, status, restart, time advance and shutdown; reset
binds one exact former session. See the
[local Fleet guide](../../docs/features/operations/local-development-fleet.md)
for configuration, executable fingerprinting, resource limits and simulation
fidelity. Production canisters do not gain a testing dependency.

## Continue From Here

- [Plan and operate a Fleet](../../docs/operations/README.md)
- [Review build evidence](../../docs/features/build-and-evidence/README.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)

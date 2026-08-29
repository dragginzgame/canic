# canic-host

`canic-host` owns operator-machine artifact builds, current desired-state Fleet
reconciliation, network/ICP transport, evidence policy and supporting local
state. It is not a canister runtime.

Normal operators use the installed `canic` binary. Direct Rust consumers may
use the build and `fleet_ensure` modules when embedding the same current
contract.

## Build

```bash
canic build <app> <role> --profile release
```

Every managed package declares exact App/role metadata. Artifact builds are
non-incremental for deterministic Wasm. An explicit `RUSTC_WRAPPER` wins;
otherwise the host discovers `sccache` on `PATH`.

## Fleet Ensure

The only maintained Fleet mutation owner is:

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

Historical install plans, release-pair loaders, role journals, repair receipts,
recovery bundles, adoption paths and installed-Fleet caches are not read.

## Cycle Safety

The plan records exact controlled balances, canister dispositions, scheduled
transfers, fees, funding and bounded burn. Every mutating platform call has a
durable intent first. Ledger and configured drain effects use exact replay
identities. Apply refuses a changed plan, unsafe live drift or a debit/burn
above the reviewed maximum.

A controller cannot pull cycles from an arbitrary canister. Material
replacement/deletion therefore requires an exact treasury-bound idempotent
drain endpoint. Without it, policy returns `NoSafeDrain` and leaves the
canister untouched. Stop and delete remain separate effects with fresh status
and residual-balance checks.

The complete desired document and operator procedure are documented in
[Fleet ensure](../../docs/features/operations/fleet-ensure.md).

## ICP Identity

When ICP CLI uses password storage, pass its supported identity password file
through the individual operator environment. Canic forwards the path to ICP
CLI and does not read or render the password contents. Keep credentials outside
the repository.

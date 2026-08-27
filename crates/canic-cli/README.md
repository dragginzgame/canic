# canic-cli

`canic-cli` publishes the `canic` operator binary. The maintained command
families are:

```text
admission
app
auth
backup
blob-storage
build
cycles
diagnostic
evidence
fleet
info
inspect
medic
network
replica
restore
scaffold
state
status
token
```

Commands removed by the pre-1.0 hard cut have no aliases or fallback parser.
In particular, fresh install, deployment-plan, adoption, retained recovery,
retained Root repair, and recovery-bundle modes are not accepted. Current
Authority-bearing Fleet commands read only a terminal `fleet ensure` inventory
and its exact Registry protocol bindings; they do not consult a former install
cache. This includes `info subnets`, which requires a complete agreeing live
Coordinator Registry and Root-summary snapshot. `cycles funding` is protected
current status only. Its former
install-plan-owned policy-rotation flags are not retained.

## Install

From a checkout:

```bash
cargo install --locked --path crates/canic-cli
canic help
```

From crates.io after publication:

```bash
cargo install --locked canic-cli --version <version>
```

Downstream workspaces should use the same `canic-cli` version as their `canic`
crate graph. The supported ICP CLI range is documented in the root
`INSTALLING.md`.

## App And Build

Create an App, declare or attach exact roles, and build deterministic Wasm:

```bash
canic app create <app>
canic app role attach <app> <role> --component-spec <component-spec>
canic build <app> <role> --provenance artifacts/<role>-provenance.json
```

Standalone builds default to the fast profile; select `--profile release` for
production artifacts. Canic keeps Wasm compilation non-incremental and uses an
explicit `RUSTC_WRAPPER`; when no wrapper is supplied, it discovers `sccache`
on `PATH`.

## Fleet Ensure

`canic fleet ensure` is the sole Fleet installation and convergence workflow.
The first invocation observes current state and retains a reviewed plan without
executing Fleet mutations:

```bash
canic fleet ensure staging --desired fleets/staging.toml
```

Review `plan_sha256`, all canister dispositions, the maximum operator debit,
fees, funding and burn, and the cycle-conservation equation. Then apply exactly
that plan:

```bash
canic fleet ensure staging \
  --desired fleets/staging.toml \
  --apply <plan_sha256>
```

Use `--json` for the complete stable report. The current desired-state schema
and retirement drain contract are documented in
[Fleet ensure](../../docs/features/operations/fleet-ensure.md).

The reconciler writes only current-generation state under:

```text
.canic/fleet-ensure/<environment>/<fleet>/
```

It does not scan or import former plan, deployment, recovery, or bundle paths.
An interrupted apply resumes the retained operation and reconciles the exact
effect before retry. A terminal immediate rerun produces no mutation actions.

## Cycle Safety

The reviewed plan separates:

- observed cycles in the controlled estate;
- cycles retained in reused canisters;
- cycles scheduled for treasury transfer;
- Cycles Ledger and management creation fees;
- bounded observation and update burn;
- requested new funding and maximum operator debit;
- create, reuse, reinstall, replace, and delete dispositions.

Apply stops if the selected account cannot cover the reviewed maximum, if the
plan or live authority drifts, or if actual debit/burn would exceed its bound.
A material canister cannot be replaced or deleted without an exact configured
treasury-bound drain endpoint. If the IC cannot recover those cycles safely,
Canic leaves the canister untouched and returns a typed blocker.
The update response alone is insufficient: stop and deletion remain fenced
until fresh observations prove both the bounded source debit and exact
treasury credit.

## Network, Replica, Evidence And State

Enroll exact network trust before connected operation:

```bash
sha256sum ./root-key.der
canic network enroll local \
  --root-key ./root-key.der \
  --fingerprint <64-lowercase-hex>
```

The `replica` group owns local launcher lifecycle. `evidence` validates and
gates stable evidence documents. `state` audits declared Canic metadata. Use
leaf `--help` output for the current grammar.

## Diagnostics

Look up one compact diagnostic by canonical code or decimal value:

```bash
canic diagnostic E123
canic diagnostic 123
```

For argument-boundary debugging, `CANIC_TRACE_ARGV=1` prints every raw argument
before parsing. It may disclose secrets and should not be retained in shared
logs.

# Controller-only endpoint performance

From the repository root, start a fresh managed fleet.

```sh
make demo DEMO_APP=metrics
```

At the prompt, exercise two ordinary updates and ask Root for their measurements.

```text
ping hello
checksum hello
ping again
metrics
exit
```

The report has a row for each observed update. In this run, `ping` has two
completed calls and `checksum` has one. Latest, average (rounded down) and
maximum are IC instructions spent in the handler. Repeating `metrics` leaves
both rows unchanged. An upgrade clears the measurements.

## Use it in an application

Select `features = ["canic/endpoint-metrics"]` in the **application canister's build
recipe** in `canic.toml`. Remove that entry to build the same actor without
instrumentation or a reader. All fleets using this artifact share the selection.

Canic supplies the controller query and recorder. Write ordinary handlers:

```rust
// double
// Double the value, saturating at the largest u32.
// limits : default
#[canic::update]
fn double(value: u32) -> u32 {
    value.saturating_mul(2)
}
```

`canic app build` verifies the endpoint instrumentation marker and shared query export in the built
Wasm before replacing the previous output. Its modules column shows
`Endpoint Metrics` on enabled builds.

After the first successful call, `double` appears in the shared `metrics`
query when requested with `canic::metrics::Request::Endpoints(None)`.
The same query accepts `Timers`, `Blob`, `Db`, `Jobs` and `Entropy` requests.
DB reports query and commit totals. Jobs reports scheduled callback outcomes.
Entropy reports supplier-managed fresh request outcomes. Each counter includes
its overflow flag. A canister upgrade starts a new recording period. A missing
recorder supplies no readings. One guard applies to every source. To override
the controller default, call `canic::metrics::set_guard(check)` during install
and reopen. Supply a synchronous `fn() -> Result<(), &'static str>` that
checks the actual caller.

The recorder and endpoint need no initialization, stable memory, timer, or
per-handler labels. The child checks its actual IC controller. For a managed
child, enable `child-metrics` and the `canic-root-client` codec in Root's build
recipe, as in [canic.toml](canic.toml). The operator can call
`canic fleet call --canister root --identity IDENTITY child-metrics 0 endpoints` for
ordinary managed child slot zero. Pass the returned cursor after `endpoints` for another page, or select a supplier by name (`blob`, `db`, `entropy`, `jobs` or `timers`).

Run `make metrics-test` with a prepared simulator to qualify the installed actor.

## Timers and Jobs

Select `canic/timers` for timers or `canic/jobs` for jobs. Add `canic/metrics`
to record supplier counters. In your install and reopen handlers, start the
runtime before binding timers or restoring jobs:

```rust
canic::lifecycle::initialize_timers().expect("initialize timer runtime");
```

`canic::mount_lifecycle!` already calls `initialize_diagnostics()` before
each handler. With handwritten IC lifecycle endpoints, call
`canic::lifecycle::initialize_diagnostics()` once at the start of install and
reopen. That attaches the selected recorders and leaves timer startup explicit.

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

Canic automatically samples completed synchronous `#[canic::update]` handlers
and mounts the controller-guarded `metrics` query. Write ordinary handlers:

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
Wasm before replacing the previous output. The endpoint count includes the standard metrics query. The modules column lists optional mounts. Available readings depend on runtime initialization.

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
per-handler labels. The child checks its actual IC controller. Every Root exposes `observe_child_metrics` for ready managed children. Its controller can call
`canic fleet call --canister root --identity IDENTITY observe-child-metrics 0 endpoints` for
ordinary managed child slot zero. Pass the returned cursor after `endpoints` for another page, or select a supplier by name (`blob`, `db`, `entropy`, `jobs` or `timers`).

Run `make metrics-test` with a prepared simulator to qualify the installed actor.

## Timers and Jobs

Select `canic/timers` for timers or `canic/jobs` for jobs. Both contribute supplier counters. In your install and reopen handlers, start the
runtime before binding timers or restoring jobs:

```rust
canic::lifecycle::initialize_timers().expect("initialize timer runtime");
```

`canic::mount_lifecycle!` already calls `initialize_diagnostics()` before
each handler. With handwritten IC lifecycle endpoints, call
`canic::lifecycle::initialize_diagnostics()` once at the start of install and
reopen. That attaches the selected recorders and leaves timer startup explicit.

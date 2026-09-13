# Persistent local development Fleet

The optional `canic-host/local-fleet` feature supplies the reusable public
harness for CANIC-017. It owns a foreground PocketIC process, persistent local
state and one fixed browser gateway. Canic's normal artifact, initialization,
Fleet Ensure and frontend handoff owners supply the deployment contracts.

## Start a consumer

Enable `local-fleet` on `canic-host`, or run the packaged
[`local_fleet` example](../../../crates/canic-host/examples/local_fleet.rs) from
its package directory:

```bash
cargo run --locked --features local-fleet --example local_fleet -- \
  run /absolute/workspace local-fleet.json /absolute/path/to/icp
```

The workspace must exist. Use the pinned PocketIC 16.0.0 binary and the supported
ICP CLI, with the intended operator identity already configured. No internal
Canic test crate, downstream topology installer, custom Registry or Ledger stub
is needed. A representative `local-fleet.json` is:

```json
{
  "schema_version": 1,
  "name": "development",
  "server_binary": "/absolute/path/to/pocket-ic",
  "server_binary_sha256": "<exact-binary-sha256>",
  "gateway_port": 4943,
  "application_subnets": 2,
  "maximum_canisters": 12,
  "canister_memory_bytes": 268435456,
  "allocation_debit_cycles": 500000000000000,
  "request_timeout_secs": 60,
  "server_lifetime_secs": 86400
}
```

The first JSON response reports the session identity, environment name,
application subnet IDs, gateway and observed local DER root key. Keep stdin
open while developing. Commands are JSON lines, each at most 64 KiB. Paths are
resolved from the selected workspace. `status` is read-only; EOF requests a
clean checkpoint and shutdown. A wrong-session shutdown command rejects while
leaving the owner running.

## Generate and converge a Fleet

First build the App's complete **local** release through `canic build`; retain
its finalized release-build ID and sealed artifacts in the workspace. Use the
maintained [Fleet policy format](fleet-ensure.md), assigning each Root to a
reported application subnet. Every Root requires an explicit Component Spec
admission; the Fleet admission sum and declared placement capacities must agree.
The qualification uses one Coordinator, two Roots, two Stores, two application
placements and two Ready pool assets.

Add the returned environment name to the workspace's existing `icp.yaml` as an
environment using the local network. The harness enrolls its observed local
trust and binds ICP's direct target to that exact environment. For example,
merge this entry into the existing environment list:

```yaml
environments:
  - name: <environment-from-status>
    network: local
    canisters: []
```

Send `seed` with the current local creation-fee input, then generate and converge:

```json
{"command":"seed","source":"fleet-policy.toml","seed":"fleet-seed.toml","creation_fee_cycles":1300000000000}
{"command":"generate","app_config":"apps/example/canic.toml","fleet":"development","release":"<finalized-local-release-id>","seed":"fleet-seed.toml","source":"fleet-policy.toml","out":"local-desired.toml"}
{"command":"converge","desired":"local-desired.toml"}
{"command":"discover","fleet":"development"}
```

The fee above is the controlled qualification's input, not a mainnet quote.
The configured allocation debit includes actual simulated Ledger/creation
charges; status reports the resulting native balance. There is no arbitrary
minimum allocation debit beyond positivity: the Ledger and normal Fleet funding
policy decide whether the chosen funding is sufficient.

Preparation accepts only fresh local authority in this same workspace and
session environment. It allocates the complete declared estate, resolves
controllers and initializes new Roots with Canic's maintained initializer.
Each Root installation has a retained attempt before submission; an installed
module and its complete authority reconcile a lost response before another
attempt can be consumed. Ordinary Fleet Ensure then owns Store bootstrap,
publication, registration, provisioning and activation. A changed source or
release cannot retarget that preparation. An interrupted command resumes from
the same desired document; do not regenerate or retimestamp an uncertain effect.

## Discovery and browser use

`status` describes local allocations. Its `allocation_role` is the originally
requested role: a pool asset can subsequently become an application canister.
`discover` joins the maintained terminal Fleet inventory to observed local
placement and returns current role, Principal, parent, subnet and module hash.
It rejects a nonterminal Fleet, a foreign role identity or an invalid Fleet name.

Preparation stages exact sealed Candid into the selected environment's normal
binding layout. Use the existing [frontend handoff](frontend-handoff.md), through
`{"command":"frontend","fleet":"development","input":"frontend.json","out":"browser"}`.
The frontend input must select this session's environment and gateway, its
reported role IDs and the configured admission derivation origin. PocketIC
installs Internet Identity alongside its Registry, CMC and Cycles Ledger;
its local provider is `http://rdmx6-jaaaa-aaaaa-aaadq-cai.localhost:<gateway-port>`.
The handoff includes the enrolled local root key. Never use it as mainnet trust.

Frontend bundles are immutable. An exact retry can use the same output directory;
a new review needs a new directory when its manifest changes. Existing browser
bindings remain usable after a same-release restart when their identities and
trust remain intact. The SDK qualification uses an admitted Ed25519 identity;
it does not establish the complete Internet Identity registration/login UI.

## Persistence, recovery and reset

`LocalFleetSession` exclusively locks `.canic/local-fleets/<name>`. It records
an unpredictable session identity before startup, verifies the executable's
exact SHA-256 and retains configuration, subnet placement, trust and Canic
version. State traversal rejects symlinks and special files. Every reset
receives a distinct `instance-<session>` simulator path and environment name.
A late orphan write can reach only its discarded generation, never the new
instance or its Ensure journal.

Named allocations retain the Cycles Ledger request and creation timestamp before
submission. The bundled Ledger's explicit simulated anonymous balance funds
those requests. Lost responses retry that same request and reconcile its returned
canister ID. Beyond the Ledger's deduplication window an unresolved request
fails; Canic does not silently submit a new timestamp or create twice.

`restart`, `advance` and `shutdown` require the exact session from status:

```json
{"command":"advance","session":"<session-from-status>","seconds":120}
{"command":"restart","session":"<session-from-status>"}
{"command":"shutdown","session":"<session-from-status>"}
```

Time advancement pauses automatic progress, advances at most one day and restores
the same gateway port. Restart/shutdown stop the gateway, request a simulator
checkpoint and confirm terminal server acknowledgement before marking state
saved. A timed-out save terminates only the process owned by that handle. A
missing or unacknowledged checkpoint rejects reopening; explicit reset is
required. Hard process termination or expiry is not a successful checkpoint.

After the owner exits, discard one exact local session with:

```bash
cargo run --locked --features local-fleet --example local_fleet -- \
  reset /absolute/workspace development <session-from-status>
```

Reset retains intent before removing that session's simulator directory and
preparation records. Interrupted reset resumes; terminal replay does nothing.
An old reset cannot discard a replacement. Stored PIDs never authorize cleanup,
and a busy owner rejects reset. An orphaned server may retain its gateway until
its configured lifetime expires; the harness does not signal an unowned PID.
Local reset discards simulated data and balances and is not a live-estate
recovery procedure. Historical Ensure records and the shared sealed-artifact
cache remain with their normal owners; reset does not sweep them or other local
sessions.

## Resource and fidelity limits

Configuration bounds application subnets to 2–8, named allocations to 8–64,
per-canister memory reservation to 64 MiB–2 GiB in 64 KiB increments, request
waits to 1–60 seconds and server lifetime to 60 seconds–7 days. These are
explicit developer-harness limits, not mainnet limits. The built-in NNS/II
subnets and system services are additional to the named application allocation
budget. The tree inspection is bounded to 100,000 entries and depth 64; private
records, command input and binding files have separate finite byte bounds.
These checks are not an OS-enforced process RSS or disk quota. Release artifacts
reuse the existing exact build cache; no second artifact cache or silent eviction
policy is introduced.

The Fleet qualification uses real separate simulator canisters/subnets and
Root-to-Coordinator registry synchronization across the subnet boundary. It does
not model mainnet node/provider behavior, consensus, network latency, boundary
nodes, availability or production charging. CANIC-141's scalar-fee rejection
remains unchanged. Local preallocation avoids asking ordinary Ensure to make
paid creates across different subnets; it does not authorize that mainnet plan.

## Qualification

The two-Root public host/real ICP acceptance case passes in 190.34 seconds
(238 seconds for its targeted runner). It converges all nine canisters, checks
two application subnet placements, exports verified browser bindings, makes an
authenticated SDK call, restarts, converges again and repeats the browser call
using the original immutable bundle. All 4,126 recorded source inputs stayed
unchanged during that run. Log: `/tmp/canic-op4-public-fleet-final.log`.

The owned simulator was observed at 481,916 KiB RSS during convergence. This is
one sample, not a peak or an enforced bound; the runner's separate shared server
was not the Fleet simulator. The final focused lifecycle run passes four
cases in 22.32 seconds, covering real Ledger duplicate reconciliation, exact
session rejection, persistence/time/trust, reset interruption, terminal
replay and isolation from a late write into a discarded generation. Log:
`/tmp/canic-op4-lifecycle-generation-final.log`.

The actual Cargo package archive contains 280 Rust files identical to workspace
source. The extracted consumer compiles with the exact local Canic dependencies
and unchanged locked external dependencies. Its public JSON interface passes
real allocation/duplicate requests, time advance, restart, clean shutdown/reopen,
terminal reset and old-reset rejection in 21.12s. Evidence:
`/tmp/canic-op4-package-evidence.json`, `/tmp/canic-op4-package-build.log`,
`/tmp/canic-op4-consumer-result.json`. This packaged startup/lifecycle check and
the full workspace-source Fleet journey are separate qualifications. The direct
ICP environment regression, 20 generator cases, scoped host/fixture/example
Clippy and layering checks also pass.

This is generic Canic qualification. Toko Miner still needs to adopt the
published package and verify its own representative workload; no sibling
repository or live Fleet was changed.

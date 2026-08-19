# 0.104 B1 Propagation Contract

Date: 2026-08-18

## Native Downstream Adoption

B6 publishes `docs/features/runtime/native-timers.md` and updates the runtime
feature index. The guide does not introduce a replacement Canic scheduler. It
must show:

1. an exact direct `ic-timers = "=0.6.1"` dependency so Canic and application
   code resolve one provider package;
2. application-owned `TimerIdentity` labels that do not claim the `canic`
   owner;
3. direct `OnceRegistration`, `AfterCompletionRegistration` and provider
   callback result/directive use;
4. native cancellation, unregister and deliberate detachment semantics;
5. volatile registration custody separated from durable domain demand;
6. synchronous reconstruction during init/post-upgrade after Canic has
   initialized the shared provider;
7. no stable provider handle, snapshot, generation, deadline or recurrence
   state; and
8. shared inventory/status evidence that keeps Canic and application rows
   separately owned.

`canisters/test/runtime_probe` is the maintained direct-provider fixture. Its
existing `companion-framework/inventory/visible` native row remains. Its
public `timer!`, `timer_interval!`, `canic::api::timer::cancel` and hidden
`TimerApi::set` coverage becomes native registration/capacity/cancel coverage.
The standalone Saltz burner remains separately classified evidence; the guide
does not depend on that experimental application.

## Synchronous Lifecycle Participant

The only canonical grammar is:

```rust
canic::start!(
    lifecycle_participant(
        init = crate::lifecycle::after_canic_init,
        post_upgrade = crate::lifecycle::after_canic_post_upgrade,
    ),
);
```

Both paths coerce at compile time to safe `fn() -> ()`. Exactly one paired
declaration is admitted. There is no partial declaration, closure, async
function, unsafe function, argument, result, registry, trait object, persisted
callback identity or runtime selection. Compile-fail coverage rejects every
malformed form and duplicate declaration.

The exact order for both lifecycle phases is:

```text
enter the one Canic-owned lifecycle export
  -> initialize the shared ic-timers runtime once
  -> restore Canic synchronous state and exact native claims
  -> invoke the matching application participant exactly once
  -> schedule only state-admitted Canic bootstrap and deferred user work
  -> return
```

Managed init invokes the participant while the Component remains Prepared.
Managed post-upgrade invokes it before the Active/Prepared scheduling branch,
including when the Component remains inactive. Root follows the equivalent
root restore order. A participant trap aborts the enclosing lifecycle message;
no later bootstrap/deferred timer commits, and corrected-cause retry reruns
from the prior committed boundary.

The 22 executable `canic::start!` call sites are eligible. The five executable
`start_local!` call sites may use the same grammar as a development surface.
The single canonical `start_wasm_store!` and `start_fleet_coordinator!` call
sites reject it. The seam adds no Candid method, stable record, authority,
readiness state or extra lifecycle export.

## Exact Canic/IcyDB Fixture

B7 adds one Canic-owned executable fixture at
`canisters/test/canic_icydb_lifecycle_probe`. It consumes the published crate
`icydb = "=0.230.2"`; no sibling path or mutable Git revision is used. The
verified crates.io archive SHA-256 is
`6a43703b839835ed1a532f280b5ec370aae4e690860da8aa0e8335347924ff24`.
That release depends on `ic-timers 0.6.1`, and Canic's exact workspace pin must
force one resolved `ic-timers 0.6.1` package identity.

The fixture uses IcyDB's published participant mode:

```rust
icydb::start!(participant);

fn after_canic_init() {
    crate::__icydb_lifecycle_participant::init();
}

fn after_canic_post_upgrade() {
    crate::__icydb_lifecycle_participant::post_upgrade();
}

canic::start!(
    lifecycle_participant(
        init = crate::after_canic_init,
        post_upgrade = crate::after_canic_post_upgrade,
    ),
);
```

Canic owns the one lifecycle export pair; IcyDB exports none in participant
mode. Artifact and PocketIC evidence must prove unchanged Candid, exactly one
`canister_init`/`canister_post_upgrade` pair, separate Canic and IcyDB provider
inventory rows, progress for both owners, reconstruction after same-release
upgrade while Prepared/inactive, participant-trap rollback and successful
corrected-cause retry. Shipped Canic crates import no IcyDB type.

## B2-B8 Propagation Map

| Batch | Required mutation | Direct fallout | Focused evidence |
| --- | --- | --- | --- |
| B2 | Delete public macros/facade/transient claims and use native callback results | facade/prelude, runtime probe, core/control-plane callback imports, public docs | facade absence/compile-fail guard; core/facade tests; targeted warning-denied Clippy |
| B3 | Reshape memory ID 60 to minimal async-job fences and cycle-only replay generation | storage, ops, state-contract manifest, control-plane support, recovery fixtures | bounded encoding, claim/takeover/exact retry/stale completion property tests |
| B4 | Move auth/cycles/placement/intent/log registrations and schedule reconstruction to exact owners | lifecycle profile pruning, metrics/status mapping, owner tests | positive/negative profile builds and owner-specific interruption/retry/stop journeys |
| B5 | Move pool custody, watchdog, snapshot suspend/resume and every private deferral to native claims | control-plane lifecycle/workflows, start macros, Store/Coordinator paths, intent-authority fixture | pool/lifecycle/snapshot tests, runtime probe and exact role inventory |
| B6 | Publish native guide and paired lifecycle participant grammar | scaffolds, macro parser/expansion, runtime docs and fixtures | compile pass/fail matrix, one export pair, unchanged Candid, exact ordering/rollback PocketIC proof |
| B7 | Add exact published-IcyDB composition fixture | test-only workspace/package graph and focused artifact acquisition | one provider package, both inventory owners, progress, inactive restore, trap/retry proof |
| B8 | Replace lexical count guard with semantic ownership guard and close docs/changelog | source roots include apps/crates/executable canister fixtures; active docs contain no facade vocabulary | semantic classification, dependency graph, residue scan, targeted timer/lifecycle suites and measured closeout |

The semantic guard may exclude ordinary unit/integration test modules from its
production-authority scan, but it must not exclude executable sources merely
because they live under `canisters/test`. Every classified file must resolve to
one of: fixed Canic consumer, private lifecycle consumer, native registration
custody, domain async-job recovery, DTO/metrics projection, independent
application custody, or prohibited scheduling authority.

B8 completes this contract with 45 classified files across `apps/`,
`canisters/` and `crates/`. The prohibited class is empty, executable canister
fixtures remain in scope, and class-specific checks prevent recovery,
projection, lifecycle and independent-application files from acquiring a
different scheduling responsibility silently.

## Measurement Contract

After the first source-mutating batch, rebuild the four exact fast-profile
artifacts recorded in `README.md` and report signed raw/gzip deltas. At B8,
repeat the same build and additionally report:

- provider declaration and scheduled-registration counts by representative
  role;
- absence of automatic top-up callbacks in Root, Coordinator, Store and
  Runtime-only managed profiles;
- scheduler/work instruction samples, latest, maximum and total values for the
  directly comparable runtime-probe jobs;
- maximum Wasm-memory growth pages for those callbacks; and
- the same test/toolchain/config identity for every before/after pair.

Wall-clock Cargo or PocketIC suite duration is harness evidence, not canister
runtime performance. It may be reported separately but cannot support a timer
performance claim.

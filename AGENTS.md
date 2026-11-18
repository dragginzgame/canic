# Canic local instructions

This repository adopts the reviewed engineering baseline in
[TOKO-TOOLING.md](TOKO-TOOLING.md). Read it before changing this repo. Shared files
are managed by `.toko-tooling.snapshot`; update them through the authenticated
installer, never by editing installed bytes.

## Product and architecture

- Before starting any new subsystem or major mechanism, hold a grilling session
  with the maintainer. Challenge the user outcome, assumptions, legacy evidence,
  simpler alternatives, configuration versus runtime ownership, per-instance
  policy, authority, migration and failure recovery. Resolve scope and material
  design decisions together, then record the agreed outcome in the existing
  subsystem notes before implementation. Accepted roadmap scope alone does not
  replace this session.
- Derive Canic from user outcomes and platform constraints. Legacy Canic represents
  four years of engineering evidence, not mandatory feature parity. The current
  design is also revisable. Before substantive subsystem work, inspect the relevant
  `canic-legacy` implementation, regression tests and Git history read-only. Identify
  the failure it solved and changed assumptions; reuse sound solutions and applicable
  regressions. Record the revision, paths and decision in the existing subsystem
  notes. Reuse prior research when it covers the same question and revision.
- Read the affected sections of the accepted
  [system design](docs/design/canic-system-design.md) before changing their contracts.
  Its [scope register](docs/design/canic-system-design.md#17-complete-capability-disposition-register)
  governs implementation admission. Current qualification scope belongs in
  [qualification/README.md](qualification/README.md); dated evidence belongs in
  [qualification history](docs/qualification/history.md). Neither release numbers
  nor renamed test packages establish milestone completion.
- Preserve App → Fleets → Canisters. App definitions own shared roles, parent
  relationships and build inputs; fleets bind networks and deployed identities.
  Roles and parentage grant no controller, application or spawn authority. Keep
  canister trees homogeneous: external operator/controller principals are separate
  metadata, never indented ancestors. Do not infer application parentage from IC
  controllers or routing links.
- Keep one canonical workflow per task. No aliases, convenience shortcuts, duplicate
  configuration mechanisms or generic workflow engines without a demonstrated need.
  Prefer extending existing owners over new packages or parallel paths.
- The managed starter uses a Root, separate versioned Wasm Store and application
  children. Root owns effect authority and retained evidence; Store owns durable
  artifact bytes, immutable releases and pins. Do not grow Root into an artifact
  repository or implicitly migrate retained inline fleets. Cascading discovery,
  subtree operations and singleton/scaling/shard/keyed allocation patterns are
  admitted requirements, with general implementations still pending. Keep allocation,
  placement, parentage and controller authority separate.
- Exclude ICP conversion from Canic; operators fund cycles manually. Application
  frontend integration and Toko deployment integration remain deferred. Toko's
  existing workspace shell is admitted for fleetless definition/build qualification;
  the selected persistence/recovery consumer is a small counter in the fresh
  [demo fleet](docs/demos/demo.md), declared under `apps/demo`. Capture and restore
  are operator-triggered; restore stays fenced until explicit acceptance/reopening.
  Counter increments are public; the application owns its explicit readiness and
  fence report. Observing readiness cannot activate or reopen application writes.
  The counter's local persistence/upgrade witness uses Storage named cells. Managed
  creation/upgrade through separate Root/Store now has a local installed CLI witness;
  named counter commands use an optional app-owned native codec. Backup qualifies local restoration of a stopped counter.
  Application acceptance and reopening remain pending. `fleet call` signs as the selected user, never proxies through
  Root and never treats controller membership as application permission. Calls
  retain exact request bytes in the existing journal; resumed signed calls are
  observation-only, and uncertainty blocks replacement calls to that target.
  The profile app is retired from the product; its packages under `qualification/`
  remain regression fixtures for ownership, composition and retained effect paths.
  Apps may have zero fleets;
  initialize apps independently and declare networks only through fleet setup.

## Ownership and retained-state contracts

Read the owning contract before changing an operation; preserve its exact authority,
bytes, recovery boundaries and qualified predecessor paths:

| Area | Canonical contract and runnable scope |
| --- | --- |
| Configuration, selection and CLI | [Configuration/operator experience](docs/design/canic-system-design.md#5-configuration-and-operator-experience), [current commands](docs/design/canic-system-design.md#current-command-contract) |
| Creation, funding, interruption and recovery | [Durable operations](docs/design/canic-system-design.md#7-durable-operations-and-uncertainty), [controller graph](docs/design/canic-system-design.md#controller-and-recovery-graph) |
| Whole-fleet and Root maintenance | [Infrastructure roles](docs/design/canic-system-design.md#infrastructure-roles), [Root demo](docs/demos/root.md) |
| Publication, custody and Store maintenance | [Versioned Store](docs/design/canic-system-design.md#per-fleet-versioned-wasm-store), [Store demo](docs/demos/wasm-store.md) |
| Persistence and migration | [Persistent releases](docs/design/canic-system-design.md#13-persistent-releases-checkpoints-and-recovery) |
| Actor composition and suppliers | [Endpoint ownership](docs/design/canic-system-design.md#endpoint-ownership-and-actor-composition), [Blob witness](docs/demos/blob.md) |

- One explicit `PlanAction` owns authority. Preserve historical record codecs,
  approval digests, exact signed attempts and original deadlines. Unknown outcomes
  retain exposure; expiry, funding or missing receipts cannot authorize replacement.
- Whole-fleet operations share effect owners and the journal lock. Permanent child
  identities, Store infrastructure and application slots remain distinct. Cancel
  only before effect preparation and retain revocation; no-change compares retained
  completion rather than fresh network convergence.
- Maintenance preserves complete child/Store custody. Old owners need qualified
  exact-code transitions; never fall back to endpoints omitting retained evidence.
  Reconciliation observes admitted effects without renewing budgets or authority;
  Store uses its restricted recovery contract, not generic advance.
- Reopen state and preserve leases/formats; never initialize after load failure or
  reset consumed authority. Unsupported old executables must refuse newer records.
- Reports, watchers and listings project existing owners. They grant no completion,
  readiness or permission and never sign, reconcile or publish receipts.
- Canic composes endpoints; Platform owns generic IC entrypoint macros, Wire owns
  codecs and Storage owns checked allocation. Passive protocols remain Host-safe.
  Mounting endpoints never implicitly boots storage, timers or services.

## Packages and qualification

Fresh counter and Root installations use Storage named leases. Existing retained
images require an explicitly qualified transition. Read the
[current qualification](qualification/README.md#storage-078-adoption) before
persistent-actor work.

Keep the root virtual Cargo workspace, products under `crates/`, fleet components
under `apps/`, and the separate qualification workspace under `qualification/`.
Use Rust 1.100.0 and inherited workspace lints. Runtime and Host dependency graphs
remain separate; do not pull optional supplier runtimes into default applications.

| Owner | Responsibility | Affected checks |
| --- | --- | --- |
| `canic` | Small greeting contract and optional local actor adapters | `make test`, `make clippy`, `make doc` |
| `canic-host`, `canic-cli`, `apps/demo/canic-hello` | Native planning, custody and hands-on greeting journey | `make journey-test`, `make journey-clippy`, `make journey-doc` |
| `apps/demo` | Persistent public counter, explicit owner-only fencing and readiness | `make counter-test`, `make counter-clippy`, `make counter-doc` |
| `canic-management`, `canic-management-protocol`, `canic-root` | Shared bounded effect engine, passive contracts and explicit Root mounts | `make root-test`, `make root-clippy`, `make root-doc`; `make growth-test` for shared effect boundaries |
| `qualification/canic-profile-*` | Retained ownership, composition, discovery and autonomous-management fixtures; not a product demo | `make profile-test`, `make profile-clippy`, `make profile-doc` |
| `ic-wasm-store` supplier; Canic's Root/Host integration | Supplier owns artifacts, pins and admission; Canic owns orchestration | `make store-test`, `make store-clippy`; `make root-doc` covers management contracts |
| `qualification/canic-asset-catalog` | Installed local Blob/catalog composition | `make blob-test`, `make blob-clippy`, `make blob-doc` |
| `canic::auth`, `qualification/canic-auth-witness` | Optional issuer/receiver mounts and installed volatile Auth composition | `make auth-test`, `make auth-clippy`, `make auth-doc` |

The commands above select scope families; choose their affected cases for ordinary
changes. `make endpoint-test` owns installed mount/authorization/rollback proofs;
`make store-readiness-test` owns Store handover/readiness; `make publication-test`
owns the installed CLI Store workflow. The narrow Store cases are selected through
`bash scripts/dev/test-store.sh CASE` (dispatch, custody, admission, quiescence,
maintenance). `bash scripts/dev/test-root.sh profile_store` selects the Store-backed
profile journey. These fixtures use synthetic cycles. Never select `fault-injection`
for live deployment; `maintenance-candidate` is a qualification marker.

Store implementation, passive protocol and supplier tests live in
`dragginzgame/ic-wasm-store`. Both workspaces select the crates.io protocol through
their lockfiles. Host declares the registry runtime as a `cfg(any())` artifact
input: the builder consumes its source through Cargo metadata without linking it
into Host. The public builder requires matching runtime/protocol versions and
uses the runtime archive's packaged lockfile, checking its protocol selection too.
No sibling checkout, copied runtime or Git fallback is required.

The harness owns explicit `make qualification-setup`; setup and tests share
`IC_TESTKIT_SERVER_DIRECTORY` (default `target/qualification/server/prepared`).
Testkit owns client types, held replies, assets and bounded server/instance
lifetimes. Verify assets offline; only explicit setup downloads. Successful close
records a deletion attempt, not independent remote deletion proof. Retire probes
only when their failure cases have a qualified product owner. Exact supplier
versions and current witnesses belong in manifests, locks and qualification notes.

### Supplier composition boundaries

Read the [Blob witness](docs/demos/blob.md) and [Auth contract](docs/demos/auth.md)
before changing their mounts, retained state or qualification. Those documents,
manifests/locks and qualification records own exact versions, source captures,
current journeys and remaining gates; do not substitute sibling working trees.

- Blob owns Service, identity, accounting and maintenance admission; the app owns
  users, catalog and publication authority. Pass captured callers unchanged and
  authorize users before invoking trusted application authority. Compose through
  one Storage bootstrap, preserving participant contributions and explicit create/open.
  Participant additions/removals require migration, not a second bootstrap.
- Preserve catalog/Blob rollback, exact maintenance attempts, drain/stop/start and
  activation custody. Issued effects are observed, never replaced. Readiness does
  not activate admission; activation requires the matching explicit authorization.
  Old custody-incomplete endpoints refuse newer records. Host-driven preservation
  remains closed until its complete contract is qualified. A local fleet witness
  is not a provider journey.
- Auth Rust and browser selections have independent, exact qualification identities.
  Optional mounts stay passive; apps select engines, account authentication,
  protected authorities and browser binding. Host imports passive types only.
- Capture caller/time at ingress; preserve directional budgets and synchronous
  reply rollback. Mounts grant no enrollment, storage or timer authority. Lost
  volatile replay/revocation state stays closed; fresh enrollment is not recovery.
  Retained Auth authority blocks preservation upgrades until qualified.
- Issuers own intents/renewal/results/pruning; reconciliation is lookup-only.
  Local signers do not qualify production keys or external signing recovery.
  Logout passes the captured caller; account fields are selectors. Browser code
  uses generated Wire codecs and Auth coordination without its own journal.
  Browser tests require explicitly prepared tools and remain headless; controller
  identity cannot replace application ownership. Keep default deployments free
  of optional Auth until selected.

## CLI and developer experience

- `canic.toml` is the only app/fleet source format. Preserve comments when adding
  declarations; journals, signed attempts and `--json` output retain their codecs.
- Group actions under `app`, `artifact` and `fleet`; alphabetize help. Only `--json`
  is global. Reject context options that the action does not consume. Infer only a
  sole app, then a sole fleet within it; never infer app identity from a child name.
  Keep discovery bounded and share its catalog between listings, selection and edits.
- `make canic` builds and installs on PATH and verifies resolution. User-facing
  commands are `canic ...`, never target-directory paths, PATH edits or aliases.
  Use the prepared default signer; `--icp-bin` is for explicit overrides/fixtures.
- Render tables with `ic-host-text::plain_table` / `bordered_table`; Canic owns columns and wording, the
  supplier owns escaping, widths and alignment. Do not add a formatter alias.
- Do not automatically open browsers, terminals or desktop windows. Serve requested
  previews over localhost and give a clickable `http://localhost:PORT/` URL, never
  a filesystem HTML link or VS Code preview. Keep requested viewers running and
  explain how to stop them. `canic fleet view REPORT`, `fleet show --serve` and
  `fleet show --watch` use localhost; add `--open` only on explicit request and use
  the normal external browser.
- Each user-facing increment includes a few short fenced `sh` examples near the
  current root changelog entry, with prerequisites and effects. Keep launches opt-in.

## Delivery and checks

- Keep `publish = false`; no first-party public licence is selected. Rewrite versions
  start at 0.200.0. Do not publish to crates.io until maturity and explicit maintainer
  authorization. GitHub pushes do not authorize publication. There are no local
  dependency exceptions or release adapters.
- For a Tooling update, change both exact workspace pins and locks, preview with
  `make toko-tooling-preview`, then run `make toko-tooling-update`. Verify the
  installed snapshot and tools with `make toko-tooling-check tools-check` and run
  the affected consumer checks. If setup fails, fix the reported phase and rerun
  the shared installer against the same locked archive.
- Ordinary changes use focused tests, Clippy and Rustdoc for affected packages/cases.
  Use `make fmt-check` and `make toko-tooling-check` for formatting and snapshot
  agreement. `make lint` aggregates both workspaces' lint/docs/format checks; `make ci`
  is the full local repository gate, reserved for authorized release/full-check
  work. This target does not require waiting for hosted CI completion.
  Tool preparation is explicit: `make install-tools`, then `make tools-check`.
  The formatting hook is installed with `make install-hooks`.
- After pushes apply the shared [post-push review](TOKO-TOOLING.md#post-push-review)
  to the App → Fleets → Canisters model and retained authority/recovery paths.
  Keep deferred findings in existing owning issues or design/qualification notes;
  do not create a parallel debt tracker.

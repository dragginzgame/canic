# Canic local instructions

This repository adopts the reviewed engineering baseline in
[TOKO-TOOLING.md](TOKO-TOOLING.md). Read it before changing this repo. Shared files
are managed by `.toko-tooling.snapshot`; update them through the authenticated
installer, never by editing installed bytes.

## Product and architecture

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
  frontend integration and Toko application integration are deferred. The selected
  consumer remains the Canic profile demo.

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

- One explicit `PlanAction` carries each operation's authority. Keep historical
  field names and directory layouts in record codecs, preserving approval digests,
  exact signed attempts and original deadlines. Unknown outcomes retain exposure;
  expiry, funding or a missing receipt cannot authorize a replacement effect.
- Whole-fleet operations reuse existing effect owners and the journal lock. Children
  have permanent identities in one ordered collection. Current one/two-child and
  bounded-upgrade limits are qualification boundaries, not a reason to duplicate
  child/sibling workflows. Store is infrastructure and consumes no application slot.
  Cancellation is allowed only before effect preparation and retains revocation.
  No-change compares retained completion, not fresh network convergence.
- Root maintenance must preserve every child's complete custody and any retained
  Store operation. Old owners require an explicitly qualified exact-code transition;
  never probe endpoints or fall back to observations that omit custody. Reconciliation
  observes only admitted effects; it does not renew budgets or dispatch authority.
  Store maintenance uses its restricted recovery contract, not generic advance.
- Reopen persistent state; never initialize after a load failure, reset consumed
  authority or discard uncertain history. Storage owns checked allocations. Preserve
  existing leases and record formats when reorganizing modules. Unsupported older
  executables must refuse newer state rather than silently dropping evidence.
- Public diagnostic reports do not establish completion, readiness or permission.
  Terminal, JSON and browser views project the same owners. Watchers never sign,
  reconcile, publish receipts or mutate fleets. Local artifact listings project
  retained publication records, not a second catalog or live Store discovery.
- Canic composes actor endpoints; Platform owns generic IC entrypoint macros; Wire
  owns codecs; Storage owns memory allocation. Protocol packages stay passive and
  Host-safe. Endpoint mounting never implicitly boots storage, timers or services.

## Packages and qualification

Keep the root virtual Cargo workspace, products under `crates/`, fleet components
under `apps/`, and the separate qualification workspace under `qualification/`.
Use Rust 1.100.0 and inherited workspace lints. Runtime and Host dependency graphs
remain separate; do not pull optional supplier runtimes into default applications.

| Owner | Responsibility | Affected checks |
| --- | --- | --- |
| `canic` | Small greeting contract and optional local actor adapters | `make test`, `make clippy`, `make doc` |
| `canic-host`, `canic-cli`, `apps/hello/canic-hello` | Native planning, custody and hands-on greeting journey | `make journey-test`, `make journey-clippy`, `make journey-doc` |
| `canic-management`, `canic-management-protocol`, `canic-root` | Shared bounded effect engine, passive contracts and explicit Root mounts | `make root-test`, `make root-clippy`, `make root-doc`; `make growth-test` for shared effect boundaries |
| `apps/profile/` | Profile protocol/client, discovery entry, persistent worker and autonomous manager | `make profile-test`, `make profile-clippy`, `make profile-doc` |
| `canic-wasm-store`, `canic-wasm-store-protocol` | Durable artifacts, pins and maintenance admission | `make store-test`, `make store-clippy`, `make store-doc` |
| `qualification/canic-asset-catalog` | Installed local Blob/catalog composition | `make blob-test`, `make blob-clippy`, `make blob-doc` |

The commands above select scope families; choose their affected cases for ordinary
changes. `make endpoint-test` owns installed mount/authorization/rollback proofs;
`make store-readiness-test` owns Store handover/readiness; `make publication-test`
owns the installed CLI Store workflow. The narrow Store cases are selected through
`bash scripts/dev/test-store.sh CASE` (dispatch, custody, admission, quiescence,
maintenance). `bash scripts/dev/test-root.sh profile_store` selects the Store-backed
profile journey. These fixtures use synthetic cycles. Never select `fault-injection`
for live deployment; `maintenance-candidate` is a qualification marker.

The qualification harness owns explicit `make qualification-setup`; simulator setup
and tests share `IC_TESTKIT_SERVER_DIRECTORY`, defaulting to
`target/qualification/server`. Tests verify prepared assets. Retire probes only when
their failure cases have a qualified product owner; renaming is not replacement.

### Blob boundary

- The qualified supplier is pinned Blob 0.100.6, Git revision
  `decde204acb6d7c8362a0003e904beef72ff89b6`. Keep its sibling checkout read-only;
  do not substitute pending supplier work. Initial preparation needs authenticated
  Git access. `scripts/dev/blob-inputs.pl` records locked source identities and
  hashes around installed qualification; changed inputs require requalification.
- Blob owns Service, identity, accounting and maintenance admission. Canic mounts
  endpoints; the application owns users, catalog and publication permission. Pass
  captured ingress callers unchanged; check real user permission before invoking
  trusted local application authority. Compose through one Storage bootstrap.
- Reopen with the real Service; preserve catalog/Blob rollback and exact maintenance
  attempts. Do not activate during mounting or bypass closed admission. Current
  fleet effects do not own Blob's drain/activation sequence: do not advertise a
  fleet-managed provider journey. Blob remains optional and independently usable.

## CLI and developer experience

- Group actions under `app`, `artifact` and `fleet`; alphabetize help. Only `--json`
  is global. Reject context options that the action does not consume. Infer only a
  sole app, then a sole fleet within it; never infer app identity from a child name.
  Keep discovery bounded and share its catalog between listings, selection and edits.
- `make canic` builds and installs on PATH and verifies resolution. User-facing
  commands are `canic ...`, never target-directory paths, PATH edits or aliases.
  Use the prepared default signer; `--icp-bin` is for explicit overrides/fixtures.
- Render tables with `ic-host-text::table`; Canic owns columns and wording, the
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
- Ordinary changes use focused tests, Clippy and Rustdoc for affected packages/cases.
  Use `make fmt-check` and `make toko-tooling-check` for formatting and snapshot
  agreement. `make lint` aggregates both workspaces' lint/docs/format checks; `make ci`
  is the full repository gate, reserved for authorized release/full-check work.
  Tool preparation is explicit: `make install-tools`, then `make tools-check`.
  The formatting hook is installed with `make install-hooks`.
- After pushes apply the shared [post-push review](TOKO-TOOLING.md#post-push-review)
  to the App → Fleets → Canisters model and retained authority/recovery paths.
  Keep deferred findings in existing owning issues or design/qualification notes;
  do not create a parallel debt tracker.

# Qualification history and design decisions

This is a dated evidence log, including the original M0 decisions. Statements
and commands below describe their recorded revisions; they are not current setup
instructions or a second roadmap. Use the [qualification guide](../../qualification/README.md)
for current test ownership and commands, the [Root guide](../demos/root.md) for
the current hands-on workflow, and the [architecture](../design/canic-system-design.md)
for scope and milestone gates. Old local output paths remain evidence references.

## Canic source-push validation: 2026-11-16

Push preparation found pre-existing lockfile selections for Host FS/Artifacts
0.80.2 beyond the earlier tooling-only adoption. These were preserved and
qualified: `make ci-native` passed (221 tests, one installed-only test ignored),
and the actual CLI `fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`
installed witness passed using those locks. It covers repeat Store replacement,
lost-effect recovery, Root maintenance and the following child upgrade. The
supplier's foundation release at `828c2692523a7a906188c111c9b58f30d0f52468`
adds retained-file publication; Canic does not select that new API here.
Artifacts' implementation is unchanged in the coordinated patch release.
Snapshot/pin/format checks, actionlint and diff checks passed. Unchanged Wasm
and Store-limitation evidence above/below was reused; no full local aggregate,
mainnet operation, version bump or crates.io publication was selected.

## Published Toko Tooling 0.44.5 adoption: 2026-11-16

Both Canic workspace catalogs and lockfiles now select published 0.44.5, archive
SHA-256 `aef8ac5d84866cf4ceb5b9d91ce9d458708069fc0c585345f07054186d267199`.
The authenticated exporter preview and installation retained all entrypoints;
only the pin checker and its jq module changed among the installed payloads.
The baseline and prepared executable catalogs are unchanged. No other locked
dependency changed, and no sibling checkout was modified by this adoption.

`make toko-tooling-check check-pins tools-check fmt-check`, actionlint and
`git diff --check` passed. This clears the installed-checker blocker recorded
below without policy exceptions or supplier regression-suite duplication.
[Canic #3](https://github.com/dragginzgame/canic/issues/3) remains open for hosted
evidence; changes are uncommitted/unpushed. Store integration retains its existing
published selection and is independent of this tooling adoption.

## Hosted CI and explicit Store preservation boundary: 2026-11-16

Local work over `a8b6f86128c186fcfa295b99432f66918c38e1b1` addresses Canic
[#3](https://github.com/dragginzgame/canic/issues/3) and
[#4](https://github.com/dragginzgame/canic/issues/4). Backup is deferred.
The workflow selects Linux and macOS arm64/Intel, native/Wasm product checks,
formatting, snapshot/pins and one history-wide secret scan. No signer/simulator
setup or deployment is part of routine CI. Shared helpers were added through
the authenticated 0.44.4 exporter. Legacy CI at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` was reviewed: retain explicit host and
Wasm coverage without its old infrastructure/provisioning matrix.

Linux `make ci-native` passed (221 tests, one existing installed-only test ignored),
as did `make ci-wasm`, actionlint, formatting and snapshot admission. Gitleaks
passed after reviewing seven exact historical false positives for the same public
PocketIC DER verification key. `.gitleaksignore` lists only those fingerprints;
retained fixture bytes and digests are unchanged. Native macOS and hosted checks
have not run. The installed pin checker rejects mandatory exact tooling pins and
private qualification versions without release-group metadata. The authorized
source-owner repair passes declaration/qualification fixtures, ShellCheck, version
checks and a read-only check against Canic. It is not yet adopted: 0.44.4 remains
selected and `make check-pins` still fails. No policy exception was added.
The pinned Blob repository was also confirmed private, with no Actions secrets
configured in Canic. The workflow now requires `CANIC_BLOB_READ_TOKEN` solely
during locked fetch; hosted authentication remains unverified until configured.

The maintainer subsequently authorized credential setup. A dedicated read-only
Blob deploy key (GitHub ID `170052507`) was created and its private half stored
as Canic's `CANIC_BLOB_DEPLOY_KEY` Actions secret. The workflow was changed from
the proposed personal token to SSH over 443, with the host key verified against
GitHub's authenticated metadata, process-local URL rewriting and an ephemeral
key file removed on exit. A real `git ls-remote` through that exact transport
succeeded, and GitHub reported `read_only: true`; local private-key material was
deleted. Actionlint and diff checks passed. The workflow remains unpushed and
hosted execution is still pending; the user is handling supplier releases.

`bash scripts/dev/test-store.sh preservation` passed both installed limitation
regressions with registry runtime/protocol 0.2.2; Store SHA-256 is
`7b522a37d7f865dcf64e5ce1377a7dee944411b6fe48342261fa1e0ce5b9e825`.
One reproduces identical complete pages before/after restoring an empty upload
while an incomplete alias retains the reservation. The other hands a mixed
active/tombstone catalog to real Root: it retains Closed/Mismatch before stop,
including duplicate continuation and activation. Focused harness Clippy passed.
These are limitation proofs, not successful removal qualification. Existing
publication/installation/maintenance evidence below does not establish that case.

Reused the legacy Store research below: immutable identities and authoritative
stable custody remain; mutable-version replacement does not. The maintainer
authorized [Store #2](https://github.com/dragginzgame/ic-wasm-store/issues/2)'s
versioned correction at its supplier. Canic's registry inputs and mutation
surfaces remain unchanged pending release/adoption and consumer qualification.
No push, supplier publication or mainnet effects occurred in this batch.

## One identity-keyed Host custody projection: 2026-11-16

Pending 0.217.0 shares complete Root decoding between Store-backed child recovery
and Root maintenance. The three existing response contracts map once into child
identity → journal/source pairs, preserving empty slots, uncertain effects and
all supplied Store maintenance history. The approved operation still selects the
exact method; wrong contracts never trigger endpoint probing or fallback.
Existing plan, request, pin and response encodings and the two-child ceiling stay
unchanged. This is a Host projection, not the future runtime capacity contract.

This reuses the pinned legacy research under **Named upgrades preserve each child's
allocation identity** and **Cascading and workflow scope review**, revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: preserve target identity and complete
custody without copying old workflow engines. The old positional wire layout
remains a compatibility obligation at the decoder, not an indexing convention for
both consumers. Their duplicate decoding branches are removed. No new package,
endpoint, lifecycle owner or allocation policy is added.

Thirteen focused Host tests pass, including exact contract/direction refusal,
truncated replies and preservation of empty/uncertain identity-source pairs.
The installed `fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`
journey passes in 87.69s with real CLI/signing, repeat Store maintenance, recovery,
Root maintenance and subsequent child upgrade. Host Clippy/Rustdoc, formatting
and tooling agreement pass; `make canic` installs the qualified CLI on PATH.
Local logs are `target/qualification/root-custody-projection-*`. No runtime
capacity increase, mainnet effect, release or push is part of this increment.

## Published host foundation adoption: 2026-11-16

Pending 0.217.0 adopts registry FS/Artifacts 0.80.1 and Process 0.80.2;
Text 0.76.0 remains the current published selection. The supplier API and Canic
qualification adapters at IC Tooling commit
`ca0bef0cd29ab272c3e3419d28292ca83ecff875` informed the migration. No source
overrides, compatibility aliases or legacy filesystem/process wrappers remain.
Canic owns path, permission, record and diagnostic-disclosure policy; the supplier
owns held-directory publication, durability, bounded process execution and reaping.
The CLI delegates report publication to Host. Profile retains its small independent
approval writer instead of importing the provisioning Host dependency graph.

Linux native qualification passed 131 Host tests (one existing ignored case),
69 CLI tests, the profile approval-record regression and the footprint parser
regression. `make journey-test growth-test` passed installed deployment/upgrade,
receipt replay, autonomous creation/maintenance and interrupted child upgrades.
The new qualification-only wasmparser 0.263.0 measurement preserves section,
export and module budgets and validates complete modules; no parser enters the
shipped host graph. Process/fs boundary cases cover limits, deadline, failed exit,
lock interoperability, immutable records, aliases and retained staging custody.
No browser or mainnet deployment was launched. macOS consumer checks were not run.

Testkit 0.106.0 still selects the older 0.79 host stack transitively inside the
independent qualification workspace. Its dependency migration remains supplier-owned;
the product graph selects only the new FS/Process stack. Local logs are under
`target/qualification/ic-host-adoption-*`; these are regenerable check output.

## Identified persistent collections preserve admitted work: 2026-11-16

Pending 0.217.0 replaces the positional journal/artifact writers with bounded
CUJ8/CUA4 collections carrying explicit numeric child identities. Historical
readers remain at the persistence boundary; requests, approvals, Store pins and
complete-custody responses keep their existing encodings. Both original Storage
keys, backing configuration, journal ceiling and combined inline artifact budget
remain unchanged. The current admission ceiling is still two children: larger
capacity needs its complete-custody and Host transition before admission changes.

This continues the allocation identity work and its pinned legacy research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, recorded below. Stable target identity
is retained; legacy container shapes are not product requirements. The codec now
uses Wire's bounded lists/byte fields and size admission, rather than another
custom multi-child framing scheme. Artifact payloads encode directly; obsolete
per-child writers were removed, with their readers retained for migration.
Duplicate, missing, reordered and unknown IDs
refuse before entering runtime state. Both stores validate before the upgrade
commits their migration; load failure never initializes a replacement.

The actual codec lives once in management's private storage module. Native
development dependencies allow direct negative tests without introducing runtime
dependencies into Host or a new package. The Root runner includes these checks.
The child protocol module now uses ordinary directory layout for its tests.

Focused evidence:

- Five actual-codec tests and all eleven passive-protocol tests pass, including
  full inline artifact capacity and complete child/Store history within 16 KiB.
  A full mixed inline/Store predecessor migrates through real governed Cells
  without changing any Storage diagnostic. Encoding reserves Cell framing and
  rejects excess data before a growth attempt. The initially considered opaque
  per-child envelopes needed extra framing space; direct payload encoding removes
  that overhead and fits the original allocation without changing its capacity.
- Three installed Root cases pass in 71.46s: migration of reverse-order pending
  children, historical independent child upgrades, and the persistent profile
  growth/Root/Store journey. The frozen positional predecessor is
  `a00ce6fcac8687c9c350e317c53d2d653665c1c531639af9d07961ade96cc90d`.
- The installed downgrade case confirms that the old executable traps without
  changing stable state or the installed module. Compatible
  replacements reopen the new format without another payload rewrite.
- The CLI Store-backed fleet fixture passes in 102.45s, including Host-approved
  migration with both child upgrade histories, interrupted installation reply,
  exact signed-attempt reuse and identical complete custody before/after upgrade.
  Payload bytes change once, while the existing backing size remains unchanged.
- Native/Wasm Clippy, affected fixture Clippy, native/Wasm Rustdoc, formatting
  and tooling agreement pass. Supplier selections and lockfiles are unchanged.

Logs and runtime fingerprints are under `target/qualification/identified-storage-*`.
Older executables intentionally refuse the new formats; they are not a supported
downgrade path. No capacity increase, new endpoint, mainnet effect, release gate,
push or publication was performed. The pending release remains 0.217.0 because
this migration and the earlier Rust API changes alter compatibility.

## Numeric child identity preserves existing authority: 2026-11-16

Pending 0.217.0 replaces the public `children::Slot` enum with the checked numeric
`ChildId`. All Host, CLI, management and qualification callers use the new type;
there is no old-type alias. Its private field and constructor admit only current
identities 0 and 1. Existing allocations remain `FIRST` and `SECOND`, scoped to
their management owner. Host construction uses the checked numeric constructor;
journals, timers and active guards retain these identities as keys.

Historical First/Second names and discriminants live in a private serialization
codec. They remain necessary for retained JSON, Wire requests, approval digests
and Store pin identities. The code does not reinterpret approval bytes, expose
numeric JSON, infer a new allocation from an unknown ID, or increase capacity.
No additional storage schema, runtime endpoint, dependency or allocation engine
was introduced. The explicit Rust API break is recorded in the root changelog.

This reuses the pinned allocation/target/cascade research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded under **Cascading and workflow
scope review** and **Named upgrades preserve each child's allocation identity**.
The relevant invariant remains permanent target identity independent of traversal
or admission order, with old authority retained rather than translated into a new
operation. The current numeric bound remains a qualification boundary; expanded
capacity still needs its explicit contract and record/protocol transition.

Focused evidence:

- 114 Host tests pass with one existing ignored installed entrypoint. Retained
  approval digests and JSON spellings remain exact; invalid numeric/unknown JSON
  is refused.
- All 11 protocol tests pass. The frozen old enum and numeric identity produce
  identical payloads, complete request frames and Store pin digests. Unsupported
  constructors and unknown Wire discriminants refuse. An initial negative fixture
  used a differently named variant at discriminant zero, which correctly decoded
  as the original identity; the fixed probe uses an actually unknown discriminant.
- Five installed Root cases pass in 69.99s: positional predecessor preservation,
  independent sibling recovery, bounded upgrade histories, original two-child CLI
  operations/views, and persistent profile growth/Root/Store maintenance. Root and
  profile management executables were rebuilt through the public artifact builder.
- Named growth and four repeated upgrades pass on inline and Store-backed fleets
  in 95.55s using the prepared pre-change publication executables. Their hashes
  remain unchanged across execution.
- Native Host/CLI/protocol and Wasm management/Root/profile Clippy, the affected
  Root/Store/publication fixture targets, Host/protocol/management Rustdoc,
  formatting and tooling agreement pass.

The broader fixture compile attempt exposed `tests/fleet_growth.rs` being auto-
discovered as an unintended standalone Cargo test target despite belonging to
`publication`. Moved it to `tests/fleet_growth/mod.rs`, preserving ordinary module
discovery, the test name and coverage. No path override or duplicate harness remains.
Logs and actual runtime/publication fingerprints are
`target/qualification/numeric-child-*`. `make canic` installed the updated CLI on
PATH; package versions remain 0.216.5 pending delivery. No push, release gate,
crates.io publication or mainnet effects ran.

## Child identity owns continuation state: 2026-11-16

The next allocation increment removes positional heap state from the shared
management engine. Retained timers use a map keyed by child identity, active
observations use a set of those identities, and qualification faults use the same
keys. Child upgrade admission checks its own active identity; Store maintenance
requires the active set to be empty. Future cleanup removes only its own identity.
Existing timer names, explicit recovery tokens, effect journals and deadlines
are unchanged. No second scheduler, allocation abstraction or dependency was added.

This continues the journal identity work recorded for 0.216.5 and reuses the
legacy target/allocation/cascade research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded under **Cascading and workflow
scope review** and **Named upgrades preserve each child's allocation identity**.
The lesson is that admission or traversal order must not determine an operation's
owner; the existing child identity supplies that owner. Historical positional
storage and public custody representations remain unchanged at their codecs.

Rebuilt normal, maintenance and fault executables for both Root and the profile
manager through `canic artifact build`. Four selected installed cases pass in
71.31s: positional predecessor preservation with reverse admission order; lost
creation callback with independent sibling progress and upgrade recovery; both
children's bounded upgrade histories; and the persistent profile growth/owner
maintenance journey, including same-instance recovery after callback cleanup.
The predecessor test compares complete stable bytes in both executable directions.

```sh
CANIC_CLI=$(command -v canic) cargo test --locked \
  --manifest-path qualification/Cargo.toml -p canic-qualification --test root -- \
  children::identity children::lost_creation children::two_children profile_store --nocapture
```

Wasm Clippy passes for management, Root and profile manager with all features;
management Wasm Rustdoc and `make fmt-check` pass. Logs and actual locked/artifact
fingerprints are under `target/qualification/identity-scheduling-*`. No claim of
reduced Wasm size or execution cost is made by replacing these collections.

This is a preservation-qualified step toward bounded allocation identities, not
an increase in capacity. The public First/Second contract and two-child bound
remain. Additional children still require an explicit bounded identity/capacity
contract and qualified record/protocol transition; do not add Third/Fourth enum
variants. Pending 0.217.0 remains unpushed; no mainnet effects or release gate ran.

## Persistent profile growth after infrastructure maintenance: 2026-11-16

Pending 0.217.0 now qualifies the second-child path with the selected persistent
consumer, extending `profile_store` rather than adding another bootstrap fixture.
After the original worker and Root/Store transitions, the CLI declares `archive`,
publishes its artifact and separately approves creation in Second. Its lexical
position before `worker` does not change the original allocation. Both users
create distinct profiles with the same IDs in the new worker; their original
records stay untouched. Each worker owns its namespace and permissions.

The added worker then changes from the actual profile-v1 executable to profile-v2.
The original worker stays on v1 with its latest records. Another Root replacement
retains both creation records, the original worker's two upgrades and the added
worker's upgrade. Root, Store and both workers preserve every stable byte across
that replacement. Both users can read their own complete profiles; cross-user and
Root reads remain Forbidden. Fleet inspection reports all four canisters without
inventory issues. The existing interrupted-install/reconciliation and incompatible
application readiness cases still run in the same witness.

Reused the legacy research and ownership decision under **Store-backed profile
composition**, at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, and the existing
exact-target/component-operation research under **First Root-owned child upgrade**.
The application still owns user data and readiness; Canic owns bounded effects
and custody, Store owns artifacts, and Storage owns application allocations.
No runtime code, dependency, protocol or stable format changed. The fixture's
existing completion helper now handles completed publications too, removing its
duplicated publication-apply calls.

Focused validation using the installed CLI and prepared, unchanged runtime inputs:

```sh
CANIC_CLI=$(command -v canic) cargo test --locked \
  --manifest-path qualification/Cargo.toml -p canic-qualification \
  --test root profile_store -- --nocapture
```

The installed case passes in 70.54s; Root-fixture Clippy and `make fmt-check` pass.
Logs and actual artifact/lock fingerprints are `target/qualification/profile-growth-*`.
An initial run exposed the fixture's old assumption that timer counters remain
nonzero after the new final Root replacement. The corrected check proves activity
before replacement and a fresh diagnostic epoch afterward; durable custody and
application preservation checks remain unchanged. This follows the existing
installed metrics reset contract, not a product workaround.

The profile guide and root changelog include the second-worker commands. This is
explicit growth to two workers, not automatic sharding, user movement, new entry
routing, reusable capacity or mainnet qualification. No push or release gate ran.

## Reused artifacts retain fresh installation authority: 2026-11-16

The pending 0.217.0 growth witness now performs four named child upgrades,
B → C → B → C, for both inline and Store-backed fleets. The repeated B → C
transition receives a distinct approval against the current completed predecessor;
the earlier signed attempts remain byte-identical. Named and whole-fleet planning
select the same pending operation. Store-backed successors reuse their original
completed publication operations, and the CLI publication inventory is unchanged
through installation planning. Each upgrade retains the lost-admission-reply
exercise. Later Root maintenance preserves all four records and every stable byte.

This extends the exact-authority and retained-uncertainty question covered by
the pinned legacy component-operation/lifecycle research under **First Root-owned
child upgrade**, revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`.
No production logic, new workflow, dependency or persistence format was needed.
The existing witness was extended rather than duplicating its fleet bootstrap.
The Root demo's obsolete prohibition on adding a second child was removed; it now
distinguishes a separate addition approval from rewriting the original envelope.

Focused execution using the installed CLI and prepared artifacts:

```sh
CANIC_CLI=$(command -v canic) CANIC_PUBLICATION_ARTIFACTS="$PWD/target/publication" \
  cargo test --locked --manifest-path qualification/Cargo.toml \
  -p canic-qualification --test publication \
  existing_fleet_adds_child_preserving_identity_and_maintenance -- --nocapture
```

The installed case passes (98.11s); publication-fixture Clippy and `make fmt-check`
pass. Logs are `target/qualification/reused-upgrade-*.log`. The greeting variants
differ only by valid custom sections: this is artifact/authority reuse evidence,
not an application data rollback or schema downgrade guarantee. Shorter synthetic
dispatch advances keep the longer fixture within the unchanged ingress window.
Prior product checks remain applicable; no product code or locked input changed.
No push, release gate or mainnet effects were performed.

## Named successors use the retained serial chain: 2026-11-16

Pending 0.217.0 removes the first-upgrade restriction for named children with a
retained whole-fleet creation approval. Shared history validates the current
predecessor and identifies only proven completed earlier upgrades; those records
remain immutable while a separate successor is retained. A matching latest
approval still replays with its original schedule and digest, pending or complete.
Standalone named deployments without a whole-fleet history keep their existing
first-upgrade limit.

The original creation-module check moved from inventory projection into locked
authority admission: a successor needs the latest completed module, while an
independent first upgrade still needs its original module. Both planners now use
one check for the eight-upgrade bound, combined inline artifact custody and
strictly ordered dispatch. Exact retry comparison also has one owner, replacing
the repeated field list. No new index, protocol, runtime format, dependency or
workflow engine was added. This reuses the pinned component-operation and
lifecycle research recorded under **First Root-owned child upgrade**, legacy
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`; retain explicit predecessors and
uncertainty rather than choosing a replacement from configuration.

All 113 Host tests pass with one existing ignore, including shared bound checks
and unchanged standalone admission. The installed growth witness passes two
consecutive named upgrades on both inline and Store-backed fleets (78.39s).
Missing predecessor receipts, wrong targets/code and stale predecessor hashes
refuse. Lost stage replies retain exact signed attempts; named replay and
whole-fleet replan select the same pending operation. Later Root maintenance
preserves both creation records and both upgrades of the added child.
The original two-child creation, upgrade, maintenance and view regression also
passes (29.41s).
Host/CLI and fixture Clippy, Host Rustdoc, formatting and tooling agreement pass.
Logs are `target/qualification/named-serial-*.log`.

The first longer fixture exceeded the real signer's ingress window because its
repeated synthetic time advances put the simulator too far ahead. Smaller
scheduled advances keep this fixture inside the unchanged production window;
no signing or admission rule was relaxed. `make canic` installs the development
CLI on PATH. Package versions remain 0.216.5; no release gate, push or mainnet
effects were performed.

## Named upgrades share complete fleet history: 2026-11-16

Pending 0.217.0 removes the original-fleet-only named-upgrade check. Named first
upgrades and whole-fleet planning now use one read-only reconstruction of original
children, later creation approvals, child upgrades and Root/Store maintenance.
The existing history checks moved out of next-operation selection; no persistent
index, runtime endpoint or second recovery engine was introduced. Explicit target,
predecessor code, Root code, network, operator, slot and Store remain checked.
Completed Store maintenance is bound before comparing a retained named approval,
so filesystem order cannot change the resumed digest.

Reuse the **First Root-owned child upgrade** and **Named upgrades preserve each
child's allocation identity** research at legacy
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, including component-operation
implementation/tests and installation/lifecycle history. Retain exact approval,
target and uncertainty rather than restoring a separate legacy workflow owner.

The installed growth witness now plans the added child's first upgrade by name
after Root maintenance, and after Store maintenance on the stored path. Wrong
targets, stale Root code and missing creation completion refuse. Named replay and
whole-fleet replan return the same approval; later installation and complete Root
maintenance preserve both children. All four publication/Store-maintenance cases
pass (101.96s); the original two-child named-upgrade/maintenance/view journey also
passes (26.96s), alongside 113 Host tests with one existing ignore. Host/CLI and
fixture Clippy, Host Rustdoc, formatting and tooling agreement pass. Logs are
`target/qualification/named-growth-*.log`.

The old limitation recorded below is resolved for first upgrades. Serial named
upgrade planning remains outside this increment; whole-fleet planning owns those
successors. Named fleet-owned upgrades now require completed initial creation
and no unrelated unfinished operation, matching the ordered custody contract.
This stricter admission is explicit in the pending breaking-change notes.
Existing records, signatures and runtime formats are unchanged. The installed
development CLI remains version 0.216.5 until release; no push or mainnet effects
were performed.

## Replanning checks every reservation before reuse: 2026-11-16

Pending 0.217.0 fixes directory-order-dependent named planning. Previously the
first matching deployment or publication returned immediately, bypassing later
conflicts, malformed records and the existing inventory bound. The same locked
scan now retains a candidate locally and returns it only after checking the
remaining reservation scope. Multiple matching approvals refuse rather than
choosing arbitrary custody. Valid replays retain their original digest, deadline
and artifact bytes; no persisted record, index or recovery mechanism was added.

Reuse the **First Root-owned child upgrade** research at legacy revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, especially
`crates/canic-host/src/component_operation/ops/mod.rs` and
`crates/canic-host/src/component_operation/tests.rs`: immutable review identity,
drift refusal and uncertainty retention remain the requirements. The rewrite's
filesystem journal needs an order-independent complete reservation check, not a
ported operation catalogue.

All 113 Host tests pass (one existing installed-only ignore). New regressions
exercise both competing requests regardless of directory order, distinct
approvals with identical apparent inputs, legacy/explicit First aliases,
publication authority conflicts, corruption after a match and 129 retained
operations against the 128-operation planning bound. Refusal preserves original
records. Valid unrelated records still permit exact reuse after expiry.
The installed inline/Store-backed growth, prior-maintenance, lost-admission-reply,
child-upgrade and later Root-maintenance journey passes again (62.15s). Host
Clippy/Rustdoc, formatting and tooling agreement pass. Evidence is under
`target/qualification/reservation-replay-*.log`; `make canic` installs the updated
development CLI on PATH.

The separate named-upgrade limitation is now explicit in the qualification guide:
added children use whole-fleet planning. Extending named selection must reuse
ordered creation/maintenance custody rather than add another reservation bypass.
No supplier changes, full release gate, push or mainnet effects were performed.

## Existing fleets add a separately approved child: 2026-11-16

Pending 0.217.0 extends whole-fleet plan/apply from a completed one-child fleet
to its second child. Original fleet records and debit authority stay immutable;
the addition uses the existing `CreateChild` plan, signer, Root admission and
Store publication owners. Configuration sorting cannot reassign retained slots.
The same ordered history includes additions in later child upgrades and complete
Root maintenance. Shared creation-evidence loading and publication selection
serve both old and new paths; there is no new registry, runtime endpoint, storage
format, dependency or allocation framework. The public `FleetPlanning::AddChild`
result requires a new exhaustive-match arm, so the pending note selects a minor
release. Package versions remain 0.216.5 until delivery is requested.

Reuse the legacy research recorded under **Two-child fleet qualification** and
**Cascading and workflow scope review**, revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, especially
`crates/canic-core/src/workflow/cascade/topology.rs` and
`crates/canic-core/src/ops/cascade/tests/mod.rs`, with the recorded
`c04cd56c11d287cf5baebb472baac61706434dc5` history. Preserve exact target binding,
wrong-parent refusal and independently retained outcomes. This bounded addition
does not need legacy directory transport or a general pool owner.

Focused evidence:

- Eleven Host fleet-planning tests and all 68 CLI tests pass.
- The new installed publication case passes for inline and Store-backed growth
  with an isolated authenticated signer (61.57s). It covers prior Root/Store
  maintenance, a lexically earlier new name, separate publication/creation,
  mixed-change/removal refusal, unchanged original records, lost admission reply,
  identical signed-attempt reuse, subsequent child upgrade and Root replacement
  preserving all stable bytes. Inventory and actual canister counts exclude
  duplicate creation. Old Root approvals still refuse changed code.
- The three existing publication/Store-maintenance journeys pass (102.76s).
  Host/CLI and fixture Clippy, Host/CLI Rustdoc, formatting and tooling agreement
  pass. Logs are `target/qualification/fleet-growth-*.log`; `make canic` installs
  the development executable on PATH.

Initial fixture corrections aligned JSON assertions with the retained codec,
kept inline artifacts within their existing bound, expected old-code refusal
after maintenance, and reused the authenticated signer required by Store
maintenance. No product authority check was weakened.

The runtime still supports only two children. Growth uses Root's current cycles
and existing reserve checks, not renewed host funding. Whole-fleet planning owns
this new journey; general allocation, pool reuse, removal and inline-to-Store
migration remain unqualified. No full release gate, push or mainnet effects were
performed.

## 0.216.5 release qualification: 2026-11-16

The sequential Linux `make -j1 ci` gate passed with all 292 captured inputs
unchanged. It includes 111 Host tests (one existing ignore), 68 CLI tests,
15 installed Root cases (two existing predecessor ignores), 26 Store cases,
three publication journeys and five Blob cases, plus profile, growth, endpoint,
effect/custody, native/Wasm lint, documentation, formatting and tooling checks.
The log is `target/qualification/release-0216-5-ci.log`; source and delivery
evidence is under `target/evidence/0.216.5/release/`.

Fresh registry review confirms the current non-yanked first-party selections.
Topology remains at the current qualified Git revision. Blob retains Service
0.100.6; upstream 0.100.10 provider/browser work and its transitive Wire refresh
remain deferred to the next requested Blob maintenance batch. No supplier changes
are included. Product versions advance to 0.216.5 after qualification; private
qualification packages remain 0.0.0. This is GitHub delivery only. Runtime formats,
existing approvals and the two-child bound are unchanged.

## Runtime child state is addressed by identity: 2026-11-16

Pending 0.216.5 replaces runtime journal/artifact array indexing with ordinary
bounded maps keyed by the existing child identity. Creation, upgrades, Store
delivery, funding reservations and maintenance use those same owners. The storage
codec alone maps historical positions to identities and writes the exact existing
CUJ7/CUA3 bytes; wire custody remains in its qualified order. No schema, lease,
endpoint, allocation policy or capacity bound changes. Volatile timer/observation
arrays retain the protocol's current bound.

Reuse the pinned legacy target-binding, cascading and retained-operation research
recorded below at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`. Preserve identity
across continuations without importing a pool catalogue, directory owner or generic
collection framework. Two entries do not establish scalable allocation; this
separates the effect engine from the historical record positions first.

The retained positional executable and its provenance are documented in the
qualification fixtures. Its new installed witness preserves every stable byte
across pending admissions in reverse slot order, autonomous creation and completed
state read by both executables. Existing predecessor/full-history, callback-loss
and shared-cycle-reservation cases pass. The installed two-child Store publication
journey starts from that same predecessor and preserves all Root stable bytes
through Host-driven maintenance, including both children and Store pins/history.
The first new test incorrectly compared live status diagnostics with stored
custody; it now compares complete retained custody and stable bytes separately.
Runtime/fixture Clippy, runtime Rustdoc, formatting and tooling agreement pass.
Evidence is under `target/qualification/identified-children-*`. The patch remains
local and unpushed; package versions and suppliers are unchanged.

## Explicit child admission bound: 2026-11-16

Pending 0.216.5 reports the declared direct-child count and qualified 1–2 bound
before artifact reads or journal opening. Store remains separate. Reuse the
permanent-target/cascade research recorded in the next entry; no allocation,
capacity policy, record migration or replacement authority is introduced.

Eleven focused Host fleet tests pass, including zero/three children with and
without Store and absent artifacts. The installed multi-child CLI journey passes:
adding a lexically earlier third declaration while both children are scheduled
preserves the fleet approval, both signed admissions, operation count and Root
stable bytes. Restoring configuration returns the original plan; subsequent
creation, lost-reply recovery, child upgrades and Root maintenance still pass.
Host and fixture Clippy, Host Rustdoc, formatting and tooling agreement pass.
Logs are under `target/qualification/child-bound-*`; `make canic` installs the
working CLI. Package versions remain 0.216.4, with 0.216.5 pending and unpushed.

## Shared child-slot ownership: 2026-11-16

Pending 0.216.5 removes Host's duplicate ordered slot list. The passive child
protocol now supplies the identities used by fleet approval validation, artifact
binding, maintenance source checks and runtime validation/continuation. Volatile
timer, observation and fault arrays use that same list's length. Fixed historical
record codecs remain explicit; this is not a storage migration or capacity increase.
No new allocator, package, configuration switch or lifecycle owner is introduced.

Reuse the legacy cascading review recorded under the two-child increment below,
revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, especially the topology and
cascade regression paths and their inspected history. Permanent target binding
and sibling failure isolation remain required; legacy directory machinery is
unnecessary for this ownership correction. Existing tests already cover changed
configuration names/order, retained approval digests and wrong-slot refusal, so
this cleanup adds no duplicate fixtures.

Fourteen focused Host planning/maintenance tests pass (one existing installed-only
entry remains ignored by the native harness). Two installed child cases pass with
freshly built Root, maintenance and fault executables: actual predecessor history
migration, both children's full upgrade histories, independent continuation and
lost-callback recovery. Unchanged application artifacts and the prepared simulator
are reused from release qualification. Native/Wasm Clippy, Rustdoc, formatting
and tooling agreement pass. Logs are under `target/qualification/slot-owner-*`.

The compatible pending version is 0.216.5; package versions remain at released
0.216.4 until delivery is requested. No push, publication or mainnet effect occurs.

## 0.216.4 release qualification: 2026-11-16

The final sequential Linux `make -j1 ci` gate passed with all 290 captured inputs
unchanged. It includes 110 Host tests (one existing ignore), 68 CLI tests,
14 installed Root cases (two existing predecessor ignores), 26 Store cases,
three publication journeys and five Blob cases, plus the effect, artifact-custody,
profile, growth and endpoint checks and native/Wasm linting/documentation.
The first gate found an older installed fixture expecting the pre-slot-label
heading; the assertion was corrected and the full gate rerun. Both logs remain
under `target/qualification/release-0216-4-ci*`; input and release evidence are
under `target/evidence/0.216.4/release/`.

Fresh registry review confirms current non-yanked first-party registry selections.
Topology remains at its current Git 0.106.1 revision. Blob retains the qualified
Service Git 0.100.6 input; current upstream 0.100.10 provider/browser work and its
transitive Wire refresh remain deferred to the next requested Blob maintenance
batch. This release changes no supplier selections.

After the gate, twelve product package versions and three qualification references
advance to 0.216.4; private qualification packages remain 0.0.0. GitHub delivery is
the selected effect. No crates.io publication or mainnet deployment is included.
The two-child runtime bound remains explicit; the release improves observation,
report identity and retained-slot upgrade planning without a new allocation engine.

## Named upgrades preserve each child's allocation identity: 2026-11-16

Pending 0.216.4 removes first-child assumptions from the existing named upgrade
planner. It carries the original slot alongside the controller from retained
creation evidence; fleet reservation admission resolves that named child rather
than always reading First. The single-child declaration restriction now applies
to creation only. Exact target, network, previous module, Root code, completion
and original signed-attempt constraints remain. No slot option, new allocator,
record format, supplier dependency or deployed runtime change is introduced.

This reuses the pinned legacy research under “First Root-owned child upgrade” and
“Managed-child CLI approval” below: revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, component operation/workflow, transport
and lifecycle regressions, and their recorded path history. Preserve immutable
target identity and uncertainty; the historical single-child declaration guard
does not apply to an upgrade with exact retained allocation custody.

Eleven focused Host tests pass, including the named-upgrade custody fixture for
legacy None, explicit First and Second. It rejects missing completion, changed
targets/modules/funding and changed pending authority, preserves creation records,
and still refuses ambiguous new allocation before opening a journal. The installed
multi-child CLI journey passes with a named second-child upgrade, lost-reply
recovery, unchanged later planning and complete Root maintenance. Its first run
exposed the overbroad creation guard; the corrected focused rerun passes.
Host and Root-witness Clippy, Host Rustdoc, formatting and tooling agreement pass;
logs are under `target/qualification/named-child-upgrade-*`.

Section 8 now explicitly distinguishes today's two-slot bound from future
operator-approved capacity and spare pools. General allocation requires stable
IDs, a bounded collection and qualified preservation, not more fixed enum variants.
This increment does not implement that collection or increase admitted capacity.

## Child report identity and naming: 2026-11-16

Pending 0.216.4 gives terminal and HTML child reports one shared slot label:
the stable physical slot plus an unambiguous local name, falling back to the
reported principal. Legacy None and explicit First refer to the same slot.
Conflicting retained names cannot select whichever record appears first, and
HTML now says Child rather than incorrectly calling each canister an application.
The existing role/name projection moved from terminal-only ownership to the
shared fleet module; no duplicate resolver, tree edges or identity registry was
added. This reuses the legacy application-tree/name-ambiguity research recorded
below at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, including its implementation,
regressions and path history. Declared parentage remains independent of controllers.

All 19 CLI fleet tests pass, including escaped fallback identities, repeated and
conflicting names, record-order independence, network/owner separation, unknown
reports and unchanged JSON. Both affected installed CLI journeys pass with actual
terminal/HTML assertions, including discovery without a local journal. The first
run exposed stale expected labels and an attempted overwrite of a saved report;
the fixtures now retain a distinct report per observation and the focused rerun
passes. CLI and Root-witness Clippy, formatting and tooling agreement pass.
Logs are under `target/qualification/child-labels-*`. `make canic` installs the
pending CLI; package versions remain 0.216.3 until release. No browser was opened.

## Child discovery without a local deployment journal: 2026-11-16

Pending 0.216.4 observes both supported child slots of each selected management
owner. Previously the local operation list selected slots, so a child admitted
through another operator's journal could be absent. The first-slot legacy query
and explicit First slot now share retained creation/upgrade constraints without
changing any persisted records. Empty unplanned second slots stay hidden;
unavailable observations produce an inventory issue rather than an empty claim.
The existing owner/global time budgets and single metrics observation remain.

Research reuses the application-tree implementation, regression and history review
below at legacy `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`. It also inspected
`crates/canic-host/src/registry_discovery/mod.rs` and
`crates/canic-host/src/canister_protocol/registry_read/mod.rs`, with discovery path
history `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f` and
`aed32be2afbaf66d7e2867292da52f75ce028e41`. Retain the separation between
observation and custody/authority. Today's two fixed slots need neither legacy's
Coordinator registry nor its release-schema/partition discovery machinery.

Thirteen focused Host inventory tests pass, including legacy/explicit first-slot
approval mismatch rejection and unchanged records. The installed
`discovers_both_children_without_local_deployment_records` witness passes with the
real CLI, Root and two children admitted in reverse order. It verifies observed
controllers, unchanged Root stable bytes, no local journal, one metrics report
per owner and unknown status for an older Root's missing second-slot endpoint.
The existing `multi_child_cli_creation_upgrades_maintenance_and_views` also passes.
Host/CLI and Root-witness Clippy, formatting and tooling agreement pass. Logs are
under `target/qualification/child-discovery-*`.

This remains sampled public inventory, not authenticated completion, atomic
membership or recursive cascading discovery. No canister executable, state format,
supplier dependency or mainnet deployment changes.

## Fleet browser metrics projection: 2026-11-15

Pending 0.216.4 completes the existing observation surface: a collapsed timer
section in saved/live HTML uses the same counter rows as terminal details.
It preserves decimal totals, overflow quality, initialization windows and unknown
reports without adding requests, rates, persistent history or another collector.
Legacy research reused the metrics work below and inspected
`crates/canic-host/src/observatory/view/metrics/mod.rs` at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, with path history `ad8852d`:
retain exact text, source reset identity and unavailable state; the legacy cached
family/registration hierarchy is unnecessary for this fixed aggregate report.

All 18 focused CLI `fleet::` tests and CLI Clippy pass; logs are under
`target/qualification/metrics-view-*`. The projection checks cover maximum values,
unknown versus zero, escaped identity/error text and unchanged input snapshots.
No browser window, live deployment or installed-runtime requalification is part
of this host-only change. The recording and endpoint contracts remain 0.216.3's.

## 0.216.3 release qualification: 2026-11-15

The sequential Linux `make -j1 ci` gate passed with all 288 captured input hashes
unchanged. It includes 109 Host tests (one existing ignore), 66 CLI tests,
13 installed Root cases (two existing predecessor ignores), 26 Store cases,
three publication journeys, five Blob cases and the effect, artifact-custody,
profile, growth and endpoint checks, plus native/Wasm linting and documentation.
Evidence is under `target/evidence/0.216.3/release/`; the gate log is
`target/qualification/release-0216-3-ci.log`.

Fresh registry review confirms the selected direct registry suppliers, including
Timers 0.45.10, are current and non-yanked. Topology retains the current committed
Git 0.106.1 input; it has no registry entry. Blob remains the qualified Service
Git 0.100.6 input: registry 0.82.1 does not provide the selected application
boundary. Published Git 0.100.10 provider/browser work is deferred to the next
requested Blob maintenance batch, along with its older transitive Wire selection.
Canic itself selects Wire 0.24.1. This release changes no further suppliers.

After qualification, twelve product versions and three qualification references
advance to 0.216.3; private qualification packages remain 0.0.0. GitHub delivery
is the selected release effect; crates.io publication and mainnet deployment
remain outside it. The release includes timer adoption, bounded public metrics
and resolved build-output preflight, preserving retained effect/state contracts.

## IC Timers retained slots and explicit recovery: 2026-11-15

Pending 0.216.3 adopts published `ic-timers` 0.45.10 from 0.44.3. The crates.io
index and downloaded archive agree on checksum
`e9bc6f83f3b47cdfc531fb78ab9fab02781dd5e2709bee1de67829f1395cb2ac`.
Reviewed the published `api.rs`, `transport.rs` and manifest against the
[supplier changelog](https://github.com/dragginzgame/ic-scheduling/blob/main/releases/timers/CHANGELOG.md).
The new API requires explicit declaration lifetimes and exact abandonment
recovery tokens. Legacy research reuses **Profile worker callback recovery**
below: task-owned cleanup releases heap exclusion, while durable effect intent
and the original authority survive. It does not justify replaying uncertain calls.

The existing management engine binds each fixed child slot once with
`Lifetime::Retained`. Ordinary scheduling only changes demand. Authenticated
creation/upgrade reconciliation first inspects durable domain evidence, then
recovers a current abandoned timer to inactive and derives eligible demand from
the journal in the same local commit. Initialization still restores the journal
strictly and reconstructs volatile timer declarations; no timer snapshot, new
recovery endpoint or persistent schema is added. Canic owns effect eligibility;
Timers owns wakeups, callback isolation and recovery tokens.

Only the product lockfile changes. The separate qualification graph has no
Timers dependency and remains byte-identical; its installed executables are built
from the product graph. Locked metadata resolves both graphs. Default and
all-feature Wasm trees for Root/profile manager select one Timers 0.45.10 and one
Platform Runtime 0.22.5; native CLI has no Timers. This initial migration left the
supplier's optional metrics feature disabled; the subsequent instrumentation
increment below enables it for management owners. No sibling checkout or other
supplier version is changed.

Focused installed qualification passed, sequentially:

- `bash scripts/dev/test-growth.sh`: nine protocol tests and two installed growth
  cases; the existing retained-predecessor case remains explicitly ignored.
  Upload/install callback traps remain blocked through further timer opportunities
  before same-owner reconciliation. Creation and child-upgrade evidence, private
  state and original approvals survive.
- `bash scripts/dev/test-root.sh profile_store children::`: five installed cases
  cover independent children, sibling reservation/isolation, the real CLI fleet
  flow and Store-backed private profiles through callback recovery and Store/owner
  maintenance. The generated consumer also passes Wasm Clippy.
- `bash scripts/dev/test-store.sh dispatch`: both installed dispatch cases pass,
  with the focused native admission/upgrade checks. Past earliest-dispatch times
  remain eligible within the original expiry; future work waits and expired,
  unstarted work does not create a child.

Product Wasm all-feature Clippy/Rustdoc and qualification growth/Root/Store Clippy
pass. Logs, metadata and dependency trees are under
`target/qualification/ic-timers-*`; executable receipts/copies are under
`target/growth`, `target/root` and `target/store`. The default Root SHA-256 is
`e32f29bec9ee84e06e84efea2b790bcd99d35f6a86f3e33d9e8d5f730185c07e`;
the default profile manager is
`7670963982be7a02804cfe4022fb06dcdd7ab1141eda74bfb94f93b9952a200f`.
These are local synthetic-cycle observations, not a mainnet upgrade or a full
release gate. Lost creation principals remain unrecoverable; timer recovery does
not expand the domain's evidence or retry contract. Existing deployed executables
retain their previous timer implementation until explicitly upgraded.

## Management timer metrics: 2026-11-15

The maintainer confirmed metrics remain an admitted operational requirement:
reusable suppliers gate instrumentation; management owners enable it normally.
Pending 0.216.3 therefore enables `ic-timers/metrics` in the shared management
engine, selecting its exact `ic-metrics` 0.30.8 dependency. `canic::metrics` owns
passive transport types and an optional endpoint mount, not a recorder. Root and
the profile manager explicitly mount that public query. The adapter feature only
selects Platform entrypoint support; default applications and native Host/CLI have
no Timers/Metrics recording dependency. No second management feature or runtime
configuration switch is introduced.

Legacy research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` inspected
`crates/canic-core/src/ops/runtime/metric_providers/{mod.rs,tests.rs}`,
`workflow/metrics/query/mod.rs`, and the public-metrics protocol tests in
`canic-contracts`. Provider history includes `4426e24`, `4176ee9` and `d704f4d`.
Retain fixed names, source overflow quality, reset windows and maximum-frame
qualification. Do not restore the provider registry, storage policies, paging,
sampling timers or metric history: six aggregate timer counters need none of them.
The supplier still owns accumulation; Canic projects its totals without arithmetic.

The 32-byte empty request and 1,024-byte response ceilings include Wire framing.
The native allocation allowance separately covers Wire's conservative traversal
of nested fixed records. JSON counter totals are decimal strings, preserving
u128 precision independently of browser number handling. The reset window captures
canister version and IC time at lifecycle initialization; later controller changes
do not reset it. Normal returns carrying a domain failure are classified as
expected failures. Accepted/completed counts do not count successful child effects.
Read-only Host collection runs once per selected owner after operation and metadata
observations, within the existing total budget and a two-second per-owner ceiling.
Older/missing endpoints remain unknown; terminal presentation is details-only.

Focused validation: the maximal passive Wire/JSON round trip and ten terminal
cases pass. Installed `profile_store` passes through the real CLI, including
abandonment, expected failure, private state and owner maintenance. The new
installed metrics case refuses malformed/oversized/wrong-method/wrong-direction
requests, permits an unrelated public reader, preserves stable bytes through
repeated queries, and resets only diagnostics on upgrade. Its first assertion
incorrectly expected the maintenance executable marker to stay unchanged; after
explicitly accounting for that intended marker, the focused rerun passes with
operation evidence and stable bytes unchanged. No product workaround was needed.

Evidence is under `target/qualification/timer-metrics-*`, using product-built
artifacts under `target/root`. The default Root SHA-256 is
`976493b54b3e8b0f7e84d13110881a31303d50238a1d12675b6a47045b972401`;
the profile manager is
`f0b376314b6504e6c100f9e0fe5a88e80c81fa0fe3e98524923ad601276be792`.
The qualification lockfile remains unchanged. Native and Wasm graph checks show
instrumentation only in selected management builds. These are local synthetic-cycle
observations; no mainnet installation, persistent metrics, history, rates or
application instrumentation is qualified. Two existing Blob endpoint lint findings
were cleaned up without changing its range guard or decoder behavior.
Native/Wasm Clippy, Rustdoc, formatting and tooling agreement passed. `make canic`
installed the pending CLI on the existing PATH; it still reports 0.216.2 until
release preparation. This increment remains uncommitted and undeployed.

## 0.216.2 post-push review and build destinations: 2026-11-15

Verified `main` and annotated `v0.216.2` at
`521b054d425ad152a84c5cecb8fc80d53b459ac6`. Reviewed the delivered recipe selector,
error rendering, Store evidence readers and their status/maintenance callers.
Those changes retain the existing build and custody owners. The actionable
finding was lexical-only duplicate output admission: different configured paths
could designate the same destination, including an unselected canister's output.

Pending 0.216.3 resolves destination identities with the already-selected
`ic-host-fs` path owner before compilation. Original paths still reach the artifact
writer, preserving its independent checks and output spelling. This is a local
preflight snapshot, not namespace confinement against concurrent filesystem edits.
No private resolver, new dependency or second build workflow is introduced.
Legacy research reuses **Starter workflow simplification** below; filesystem
resolution is a supplier responsibility, not a legacy Canic mechanism to restore.

Both native preflight cases passed, covering relative/absolute and normalized
paths, existing/dangling symlink aliases and unchanged links/absent locks/outputs.
Host/CLI Clippy, Host Rustdoc, formatting and tooling agreement passed. An actual
generated app built through the installed `canic app build --canister app`,
producing only its 44,681-byte Wasm; Root/Store outputs stayed absent. Evidence is
under `target/qualification/app-build-destinations-*`, with the local fixture in
`target/qualification/build-destinations.u07IWD`. The pending fix is installed and
uncommitted; release 0.216.2 remains the last pushed version.

## 0.216.2 release qualification: 2026-11-15

The sequential Linux `make -j1 ci` gate passed with all 281 captured input hashes
unchanged. It includes 108 Host tests (one existing explicit ignore), 65 CLI tests,
twelve installed Root cases, 26 Store cases, all three CLI publication journeys,
five Blob cases and the effect, artifact-custody, profile, growth and endpoint
checks, plus native/Wasm Clippy, Rustdoc, formatting and tooling agreement.
The gate log is `target/qualification/release-0216-2-ci.log`; inputs, metadata,
supplier review and delivery intent are under `target/evidence/0.216.2/release/`.

Fresh supplier review leaves every selection unchanged. Timers 0.45.10 is now
available; its module/command refactoring over 0.45.9 remains part of the deferred
runtime-supplier maintenance batch. Blob retains the qualified 0.100.6 Git input;
registry 0.82.1 lacks the admitted Service boundary. Newer Blob Git work remains
deferred. Store/protocol remain the non-yanked registry 0.2.2 pair.
After the gate, only the twelve product versions and three qualification references
advance to 0.216.2; private qualification packages stay 0.0.0. No supplier update,
crates.io publication or mainnet deployment is part of this release.

## Selected application recipe build: 2026-11-15

Pending 0.216.2 adds `app build --canister NAME` to the existing recipe owner.
Omitting it retains all-recipe builds. Selection names an app canister, never a
role or inferred Cargo package. All recipes are admitted before compilation;
unknown selection and invalid recipes refuse before creating locks or outputs.
Only selected recipes reach the existing locked artifact builder. JSON keeps the
same canister-list shape, with one entry for a selected build.

This reuses **Starter workflow simplification** research below at legacy
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, including `canister_build` tests and
history `95bdb4ed61aaead6bddfc4a68f6e509063106f64`: package/workspace ownership
comes from declarations. No new build owner, alias, configuration or dependency
was introduced. The installed Store-upgrade witness removes its manual
manifest/package/output argument assembly and uses the app recipe directly.

The native preflight case and CLI context/help checks passed. The installed
`fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`
witness passed in 83.07s, including exact preservation of the unselected Root and
Store artifact bytes and Cargo.lock, then publication and upgrade of the edited
application at its original principal. Affected Host/CLI and publication Clippy,
Host Rustdoc, formatting and tooling checks passed. Evidence is under
`target/qualification/app-build-selection-*`; `make canic` installed the pending
CLI and its help exposes the selector. No mainnet change or release was made.

## Retained child delivery in operation status: 2026-11-15

Pending 0.216.2 projects the existing validated `Delivery` from Host's creation
and upgrade evidence readers instead of discarding it. Store-backed status adds
`delivery` (nullable); inline status keeps its original JSON shape. Terminal
output shows pin state, the original cache deadline in UTC and `prepare cache`
in the effect table. It does not observe current cache contents or admit repair.

This reuses the legacy research in **Versioned Wasm Store decision** below at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: durable artifact custody and management
cache are separate, and readiness failures need useful diagnostics. The rewrite
keeps the existing bounded effect authority; legacy cache reconstruction does
not justify a retry allowance in this presentation change.

Focused checks passed: the Host source-binding and JSON-shape cases, sixteen CLI
status cases, the terminal-error safety case, affected native and publication
Clippy, Host Rustdoc, formatting and tooling agreement. The installed
`bootstrap_store_then_publish_resume_and_retain_versions` case passed in 30.15s,
covering absent admission-only evidence, retained creation/upgrade pins and
deadlines, exact manifests and unchanged earlier delivery. Logs are
`target/qualification/delivery-*.log`; the fixture is
`target/publication/run-895829`. `make canic` installed the pending CLI. Human and
JSON status also succeeded after the simulator stopped, using only its retained
journal (`delivery-status-human.txt` and `delivery-status.json` beside the logs).
No runtime, wire contract, persistent format, supplier or dependency changed.

The follow-up reproduced a retained delivery disappearing after a partial Root
recovery reply (`delivery-history-before.log`). Both creation and upgrade readers
now preserve that earlier evidence and reject backwards pin transitions or any
change/removal of an observed cache deadline. Full observations that omit required
delivery still refuse; partial replies cannot supply evidence never observed.
This reuses the immutable-intent and late-callback research recorded below at the
same legacy revision, without importing a retry workflow or adding a persisted
record. The shared readers also serve continuation and fleet maintenance planning.

Seven managed-operation and ten fleet-planning tests passed, including forward
pin progression, partial replies, historical replay, shorter/longer/removed
deadlines and explicit delivery loss for both child actions. The installed
`fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`
witness passed in 81.60s. Affected Host/CLI Clippy, Host Rustdoc, formatting and
tooling checks passed; `make canic` installed the pending update. Evidence is in
`target/qualification/delivery-history-*.log`. The command contract also corrects
its stale two-artifact description to match the qualified Root, optional Store
and one/two-child planner.

## 0.216.1 release qualification: 2026-11-15

The complete sequential Linux `make -j1 ci` gate passed with all 280 captured
input hashes unchanged. Coverage includes 108 Host tests, 63 CLI tests, twelve
installed Root cases, 26 Store cases, all three CLI publication journeys and five
Blob cases, plus effect/custody, profile/growth, endpoint composition, native/Wasm
Clippy, Rustdoc, formatting and tooling agreement. Existing explicitly ignored
historical fixtures remain exclusions. The gate log is
`target/qualification/release-0216-1-ci.log`; captured inputs and metadata are under
`target/evidence/0.216.1/release/`.

Fresh supplier review confirms Store/protocol 0.2.2 and the other selected direct
registry suppliers are current, except Timers 0.44.3 versus 0.45.9. The optional
Timer refresh is deferred to the next requested runtime-supplier maintenance
batch with dispatch/recovery qualification. Blob retains the qualified Git
0.100.6 Service; its registry remains 0.82.1 and optional Git 0.100.9 at
`61d4ce02e7afd421bb48d9b8e5a198333e6055a0` remains deferred to the next requested
supplier-maintenance batch. Topology stays at the verified current Git revision
`0a3eca919f05d777798406087049cc51ab7ae2a8`; no registry package is available.

After the gate, product metadata advances to 0.216.1, with only the twelve root
and three qualification-local product lock entries changing. Qualification
packages remain 0.0.0. Delivery selects `main` and annotated tag `v0.216.1` at
`https://github.com/dragginzgame/canic.git`; no registry publication or mainnet
deployment is authorized by this release. The retained 0.1.0 Store executable
and the separate 0.2.2 archive inputs remain recorded below.

## Pre-registry Store upgrade witness: 2026-11-15

The existing installed replacement proof now runs from both the current Store
and the retained 0.1.0 executable. The predecessor is 501,823 bytes, SHA-256
`e8a74f2155d91da7964525984c0d4e83d53d037f11190a71ce0c664b5933a3ad`, matching
the earlier extraction evidence. It is retained with provenance in the existing
qualification fixtures, without a Git build fallback or copied implementation.

Both lockfiles separately advanced to registry 0.2.2; the maintainer confirmed
there is no active session and asked to preserve them. This witness therefore
qualifies 0.1.0 → 0.2.2, not an inferred transition from the prior 0.2.1 run.
The runtime/protocol archive hashes are
`5ac8eeefd0b49bc36a1e946bc984dcabae974cb6510150296d90ae205d5e1cd0` and
`85f5b667c70c507beac46ccd0bd86d2330d27faf63deaa99d37754234016d253`;
their recorded source is `ed89b13cd63d6f4bdf1205f90a83798f3c022d6c`.
The normal and maintenance-candidate Wasm hashes used here are
`7b522a37d7f865dcf64e5ce1377a7dee944411b6fe48342261fa1e0ce5b9e825` and
`a51740c5e501f46fc2c26b6fea1783df3790884b78a42c41b8282c052a1b5f0e`.

Seven installed maintenance cases pass, including shared releases/pins, unfinished
upload progress, equality of every Store stable byte across replacement, lost
callbacks, Root replacement, expired recovery and explicit/stale activation.
The new case first exposed a fixture assumption that predecessor code always
matched today's build; the approval now binds the installed code observed by its
actual controller before handover and asserts the supplied predecessor digest.
One direct rerun omitted the runner's required artifact variable and failed before
simulation; the corrected invocation passed all seven cases in 11.14s. Both failed
runs remain in `target/qualification/store-registry-predecessor*.log`.

The selected protocol budget test, Host source-selection test, nine CLI Store
cases, native/qualification Clippy, formatting and tooling agreement pass.
Additional logs use `target/qualification/store-predecessor-*`. Production
maintenance logic is unchanged; both predecessors reuse one test body and setup
owner. This adds local preservation evidence, not mainnet deployment authority
or tombstone-maintenance support. Pending work remains 0.216.1 and unpushed.

## Registry Wasm Store adoption: 2026-11-15

The requested registry adoption selects Store and protocol 0.2.1, published from
`05755252817fdaff23455ab6550d98695fe3a169`. Archive SHA-256 values are
`141a4945cdec9fb63dd651996c79c7a796c8adcb4e69f0fc93385c933920e3f5`
(runtime) and `d6dbd06b94cd7d9a18af9f733bde5cc1e2c06e9575889f2eb041f21dc2b1e4b0`
(protocol). Both consumer locks retain the same protocol; unrelated dependency
selections are unchanged. All 25 archived files match the unpacked build sources.
The sibling checkout was inspected read-only; its pending changes were not inputs.

The existing artifact builder now selects a matching crates.io runtime/protocol
pair through locked metadata, builds with the archive's packaged lockfile and
checks that lock's protocol too. Host's false-target artifact dependency supplies
source without linking the Store runtime; the native normal/build graph confirms
only the passive protocol is present. Compiler output stays under Canic's target
directory and the consumer workspace selects the Rust toolchain. Git layout
assumptions and their selection path were removed, with no fallback or new crate.
This reuses the extraction ownership/legacy research recorded below; source
distribution changes, while Store keeps custody and Canic keeps orchestration.

Focused source-selection refusal tests, native Host/CLI and Wasm Root/management
Clippy, Host/CLI Rustdoc, formatting and tooling agreement pass. The installed
fleet witness passes in 82.78s: bootstrap, two Store replacements, interrupted
upload recovery, Root maintenance and a later application upgrade. The final
public-builder check produces the same 576,047-byte Store used by that witness,
SHA-256 `d239918a1510089b4ec3319a12b6406e020eb447b08e476a5a5c0bf094b951cc`.
Logs and locked metadata use `target/qualification/store-registry-*`.

Supplier 0.2 adds removal/restoration; Canic's typed publication commands do not
forward them. Tombstone identity and ordering need a preservation contract before
qualifying that flow, as recorded in the Store demo. This is Linux/local evidence,
not mainnet replacement or tombstone maintenance qualification. `make canic`
installs the development CLI; product version remains 0.216.0, with compatible
pending notes under 0.216.1. No commit, push or live deployment was performed.

## 0.216.0 release qualification: 2026-11-15

The maintainer authorized GitHub delivery of the repeat Store-maintenance increment.
The complete sequential Linux `make -j1 ci` gate passed with all 279 captured
input hashes unchanged. This includes 108 Host and 63 CLI tests, 12 installed Root
cases, 25 Store cases, all three CLI publication/upgrade journeys and five Blob
cases, plus effect/custody, profile/growth, endpoint, native/Wasm lint, Rustdoc,
formatting and tooling checks. Existing explicit historical fixture exclusions
remain exclusions. The gate log is `target/qualification/release-0216-0-ci.log`;
input and supplier evidence is under `target/evidence/0.216.0/release/`.

Fresh registry review confirms the selected direct first-party registry versions
remain current. Store and Topology remain at their qualified Git revisions, which
still match their default branches and have no registry entry. Blob retains
0.100.6 at `decde204acb6d7c8362a0003e904beef72ff89b6`; its registry is still
0.82.1. The optional Git update to 0.100.9 at
`61d4ce02e7afd421bb48d9b8e5a198333e6055a0` on branch `0.100` is deferred to the
next supplier-maintenance batch. Its callback/client and Wire adoption work is
outside this release's qualified local Service composition. No supplier changed.

After validation, the coordinated product version and local entries in both
lockfiles advance to 0.216.0. Qualification members remain 0.0.0 and all packages
remain unpublished. Delivery selects `main` and annotated tag `v0.216.0` at
`dragginzgame/canic`; it includes no crates.io publication or mainnet deployment.
Native macOS qualification is not claimed.

## Repeat Store maintenance under the existing owner: 2026-11-15

The pending 0.216.0 increment extends the existing Store maintenance journal and
fleet planner. Root retains up to eight immutable attempts, archives only Ready
predecessors and admits the next sequence against the exact Store/code boundary.
Every callback mutation matches both approval and phase; historical replay returns
its own evidence without touching a successor. Full history is never evicted.
The original 16 KiB Storage lease remains unchanged. The worst-field codec fixture
round-trips two full child journals plus eight Store attempts in 12,880 payload
bytes. CUJ7 explicitly migrates recognized predecessors, including CUJ6; older
executables refuse the new tag. No Wasm payload enters Root's journal.

New Host approvals bind the sequence and select an exact-attempt read. Historical
plan bytes, digests, deadlines and signed read methods remain unchanged. Root
maintenance after a second attempt binds every predecessor and selects the new
complete-history read; the older read refuses to omit history. Child delivery
selects that same contract, and publication reuse follows the validated Store
code chain. No CLI command, crate, dependency, allocator, workflow engine or
supplier change was added. The supplier remains at
`541d911410dae0f38eefc371693024bf2068c55b` and owns artifact bytes and admission.

Reused the recorded legacy immutable-intent, late-callback and serial-upgrade
research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, especially the
[bounded serial child upgrade decision](#bounded-serial-child-upgrades) and
[first Store workflow](#fleet-commands-carry-store-replacement-through-later-upgrades-2026-11-15).
The applicable lesson is preserving original authority through continuations,
not importing the legacy workflow catalogue. The supplier admission implementation
already supports successive exact attempts; Canic required the missing orchestration
and custody boundary, not another Store mechanism.

Focused Linux qualification passed:

- 108 Host tests and nine protocol tests; four affected maintenance tests were
  rerun after adding full-history omission/substitution cases. The installed Host
  entrypoint ignored in native tests ran through the Store harness.
- All 25 installed Store cases. New witnesses migrate both a completed operation
  and an issued installation from the retained 0.215 Root, complete eight Store
  upgrades, recover the second upload, preserve all prior records and stable Store
  bytes, refuse stale/reused/mismatched authority and a ninth attempt, and reject
  older Root code without losing custody. A broad regression exposed changed
  Occupied/Time precedence; the original occupied-authority precedence was restored.
- The installed CLI/signer journey (80.38s) completes two Store replacements with
  interrupted second-upload recovery, preserves both through Root maintenance,
  then publishes, upgrades and calls the application at the same identity. A third
  plan selects sequence three. Local fixtures use isolated credentials and synthetic
  cycles, never the operator wallet.
- Both manager growth/upgrade cases passed; the optional external historical
  manager fixture remains explicitly ignored. Native and Wasm Clippy, affected
  Rustdoc, formatting and tooling agreement passed. `make canic` installed the
  development CLI on PATH; browser launches remain opt-in.

Logs are under `target/qualification/store-repeat-*.log`; raw evidence stays under
`target`. The retained CUJ6 binary and provenance are in the existing qualification
fixtures. Limits remain eight attempts, 1 MiB replacement Wasm, bounded observations,
no reset/cancellation/eviction or repair of uncertain attempts. New plans require
the current Root endpoints through separately approved Root maintenance; no probing
or fallback is introduced. No macOS, mainnet or full release-gate claim is made.
Package versions remain 0.215.0 pending release; these changes are local and unpushed.

## 0.215.0 release qualification: 2026-11-15

The authorized Canic release includes the standalone Store adoption and extraction
cleanup. The complete sequential Linux `make -j1 ci` gate passed, including 107
Host and 63 CLI tests, effect/custody, profile/growth, 12 Root cases, endpoint
composition, 23 Store integration cases, three CLI publication journeys and five
installed Blob cases, plus native/Wasm lint, Rustdoc, formatting and tooling
agreement. Existing explicit installed/predecessor exclusions remain exclusions;
this does not claim native macOS or mainnet qualification. All 290 captured tracked
path states, including intended deletions, remained unchanged during validation.
The gate log is `target/qualification/release-0215-0-ci.log`; source and supplier
evidence is under `target/evidence/0.215.0/release/`.

Fresh registry review found the direct first-party registry selections current.
Blob retains its qualified Git Service revision; its registry release lacks the
selected contract. Topology and the new Store protocol retain their qualified
Git revisions and have no available registry entry. Blob's internally owned older
Wire selection remains intentional. No supplier versions changed during release.

After validation, the coordinated product version and local-package entries in
both lockfiles advance to 0.215.0. Qualification members retain 0.0.0, the Store
supplier retains 0.1.0 at `541d911410dae0f38eefc371693024bf2068c55b`, and all
first-party workspace packages remain unpublished. This delivery does not install
replacement application or infrastructure executables on mainnet.

## Standalone Wasm Store extraction: 2026-11-15

The extraction residue review checked maintained source, both manifests/locks,
build runners, generated starter wiring and current documentation. No Store state
engine, allocation keys or duplicate supplier DTO definitions remain in Canic.
It removed the now-unused `CANIC_STORE_CANDIDATE` runner variable, merged the
adjacent test-helper impl blocks and retired `store-doc` plus the duplicate native
management Clippy invocation. `root-doc`/`root-clippy` retain that coverage. Current
docs now identify supplier codec ownership and no longer call Root-driven Store
maintenance or fleet CLI integration pending. The focused admission selection
passed both installed cases; harness Clippy, formatting/tooling agreement, shell
syntax and Make's explicit dry-run preview passed. Logs are the `residue-*` files
under `target/evidence/store-extraction/`. Historical evidence and operator journals
were preserved; this review did not alter runtime or protocol behavior.

The maintainer authorized a sibling repository at `/home/adam/projects/ic-wasm-store`
and GitHub `dragginzgame/ic-wasm-store`. Supplier commit
`541d911410dae0f38eefc371693024bf2068c55b` contains the implementation/protocol
extracted from Canic `c39aee1e34dae82be6da89bf6242c3a0e22ade84`, with product
version 0.1.0 and private qualification version 0.0.0. Every package remains
unpublished. The remote main ref was verified; post-push review found no duplicate
Canic owner or runtime dependency in the supplier.

Runtime/protocol Rust source differs only in the renamed crate import and formatter
import ordering. Endpoint names/numbers, Wire records, allocation names, quotas
and exact maintenance attempts remain unchanged. Existing Store build recipes
select `ic-wasm-store`; no old-name alias is retained. Both Canic locks select the
same supplier Git revision, containing only the passive protocol in their library
graphs. The public artifact builder resolves the runtime from that source and its
committed lockfile, with compiler output under Canic's target directory.

Supplier Linux qualification passed: ten native state tests, one protocol test,
native/Wasm Clippy, both workspaces' Rustdoc, formatting/tooling admission and the
moved installed upload/reopen/version-reuse witness. That witness also passed
using the retained pre-extraction executable (SHA-256
`5bf18e1ee35c8c84ebaba44d713cd87bc8759dfe6bb033f707307748ca21df25`) as the
predecessor, preserving an unfinished 8 MiB upload across replacement. Supplier
evidence is under its `target/qualification/extraction-*`.

Canic validation passed: 107 Host tests (one installed-only case ignored by the
native run), 63 CLI tests, nine management protocol tests, native consumer Clippy,
Root native/Wasm and installed-harness Clippy, Host/CLI Rustdoc, formatting and
tooling agreement. All 23 remaining installed Store integration cases and all
three CLI publication journeys passed, including generated starter builds, Store
replacement, Root maintenance and a subsequent child upgrade. The installed Host
custody case is exercised by the Store runner. Evidence is under
`target/evidence/store-extraction/`.

An initial concurrent invocation of the two build runners copied a maintenance
Root into the fault fixture's path; hashes identified the shared Cargo-output
collision. That run is retained as failed evidence; the Store runner passed after
the other runner finished compilation. Installed build suites must run sequentially.
No runtime fallback or extra workflow was added to conceal it. This extraction
does not qualify native macOS, mainnet replacement, repeatable Store maintenance
or automatic conversion of existing inline fleets. Legacy research reuses the
[recorded Store decision](#versioned-wasm-store-decision-2026-11-15), including its
frozen revision, regression paths and immutable-version decision.

## 0.214.0 release qualification: 2026-11-15

The maintainer authorized GitHub delivery of the retained artifact listing and
Host/status simplification. The complete Linux `make -j1 ci` gate passed:
106 Host and 63 CLI tests, installed effect/custody, profile/growth, Root/endpoint,
24 Store cases, three publication journeys and five Blob cases, plus native/Wasm
lint, Rustdoc, formatting and tooling agreement. Existing explicitly ignored
installed/predecessor cases remain excluded; this run does not claim macOS or
mainnet qualification. All 288 captured input hashes remained unchanged.

Fresh public registry review confirms the locked direct first-party registry
selections are current. Blob 0.100.6 and topology 0.106.1 keep their qualified Git
revisions: Blob's registry release lacks the selected Service contract, and
topology has no registry entry. No supplier selection or sibling checkout changed.
Review evidence and input hashes are under `target/evidence/0.214.0/release/`;
the gate log is `target/qualification/release-0214-0-ci.log`.

After validation, prepare the coordinated product version and both lockfiles for
0.214.0, date the changelog and record this result. Qualification packages retain
0.0.0 and every package remains unpublished. The native Rust status return type
changes; CLI JSON and retained approval/journal formats do not. No crates.io
publication or application deployment is authorized by this delivery.

## Host/status simplification: 2026-11-15

Pending 0.214.0 removes the internal JSON transport between Host status, fleet
inventory and CLI. One typed projection carries the existing operation evidence;
serialization remains an output boundary. Inventory reuses its admitted plan and
artifact under the existing lock and consumes the already-read child evidence.
Store execution and CLI guidance share Host's continuation classification. No
new journal, cache, effect authority or persistence format is introduced.

Legacy research used `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`:
`crates/canic-host/src/fleet_ensure/dto/progress/mod.rs`,
`crates/canic-cli/src/fleet/progress/render/mod.rs`, and the regression cases in
`crates/canic-host/src/fleet_ensure/ops/current_protocol/component_progress/tests.rs`.
The DTO path history includes `8c87ae4c8e09cdbb6f2eb3562ecb0a821e871bcf` and
`aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`. Reuse its owner-derived typed progress
principle and distinction between unknown progress and completion; do not import
its Coordinator phases, placement cursors or rendering framework into this
bounded workflow. Current regression coverage retains unknown outcomes, separate
receipts, exact child selection, failures and expired recovery guidance.

Source inspection started at `6c05a8d7c351f80d7ed93128c41bc3d7e7aa206c` with the
pending artifact-list work preserved. Focused validation on Rust 1.100.0:

- Host native tests: 105 passed, one existing installed test ignored; the added
  JSON-shape regression subsequently passed separately. Host/CLI Clippy, Root's
  Wasm all-feature Clippy and Host Rustdoc passed.
- CLI tests: 60 initially passed; three loopback-viewer cases were denied socket
  access by the sandbox and then passed with socket access. The final focused
  status selection passed all 15 tests after the typed representation was final.
- The installed `fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`
  fixture passed in 57.44 seconds, including original-effect recovery, Store/Root
  maintenance and subsequent child work. Synthetic cycles only.
- Captured six existing operation status outputs before replacing the installed
  CLI, then compared the same journal through `canic` after `make canic`. All six
  JSON outputs matched byte for byte, including nested fleet steps, publication,
  Store/Root maintenance and child upgrades. The comparison used no replica or
  signer. Evidence is in `target/qualification/simplification-before/`,
  `simplification-after/` and `simplification-*.log`.
- Formatting and tooling agreement passed. The module-layout scan found no
  remaining `name.rs`/`name/` pairs in the maintained Rust source roots. AGENTS
  links/anchors resolve after replacing repeated implementation history with
  canonical design/demo links and retaining durable constraints.

The native `status` return type is a pending Rust API change; CLI JSON and retained
approval/journal formats are unchanged. Historical readers and independent
canister authority checks remain. No mainnet action, publication or push occurred.

## Retained fleet artifact listing: 2026-11-15

Pending 0.214.0 adds `fleet artifact list` under the deployed-fleet context.
It exposes the publication history already retained by independent plans and
materialized whole-fleet bootstrap. Store still owns live releases/bytes; Host
owns the existing approvals and signed completion evidence. The listing creates
no catalog, runtime endpoint, selector alias, latest-version rule or effect owner.
No supplier or package version is changed during development.

Read legacy `api/template/mod.rs`, `api/template/status_tests.rs` and
`storage/stable/template/manifest/tests/mod.rs` in `crates/canic-control-plane`
at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, plus the API's path history
(`aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`,
`e8605c44ffaf77ceb7ddaf85b374f42b7a48703d`,
`aed32be2afbaf66d7e2867292da52f75ce028e41`). Preserve exact operation/release
identity, deterministic listing and the separation between catalog metadata,
publication progress and effect authority. Do not import the former Root overview,
template catalog or activation/GC machinery. This consumer asks for local retained
evidence; a live catalog would require its separate controller-mediated read path.

The reader reuses the bounded journal enumerator, existing per-operation lock,
plan/Wasm validation and signed publication receipt admission. Fleet code resolves
its own nested paths and approved Root/Store outputs. Rows retain both the outer
status address and exact nested publication digest. Selection compares app, fleet
and the complete network binding. Invalid records produce partial results instead
of disappearing; absent completion remains unconfirmed, not proven no effect.
The shared IC Host table renderer owns spacing/escaping. Only JSON exposes full
artifact hashes and operation metadata; signed request bodies are never exported.

Focused native checks cover missing journals without creation, immutable version
ordering, app/fleet/network isolation, retained-but-unconfirmed attempts, forged
completion without a signed observation, corrupt Wasm and partial results without
signed-body disclosure. CLI cases cover supported scope, refused effect arguments,
terminal escaping, empty evidence and JSON partial results. Host/CLI and publication
qualification Clippy, Host Rustdoc, formatting and tooling agreement pass.

The installed Store-maintenance/publication/starter-upgrade witness passed in
57.21s. It checks no publication row before bootstrap materializes its future
identities, the nested completion's outer status address and exact artifact,
and independent publication moving from unconfirmed to retained completion after
infrastructure maintenance. After the replica stopped, the installed `canic`
listed both historical versions from the same local journal. Logs and the JSON
example are under `target/qualification/artifact-list-*`. No runtime changes,
mainnet effects, browser launches or push occurred. Package metadata remains
0.213.0 pending release of the 0.214.0 increment.

## 0.213.0 release qualification: 2026-11-15

The maintainer authorized GitHub delivery. Fourteen product packages select
0.213.0; qualification packages retain 0.0.0. Registry freshness review confirms
the selected first-party registry suppliers are current, including Wire 0.24.1
and Tooling 0.44.4. Blob 0.100.6 and topology 0.106.1 retain their explicitly
qualified Git revisions; topology has no registry index entry and Blob's registry
release does not contain the selected Service contract. Neither source is replaced
by an unreviewed sibling checkout. All packages remain unpublished.

Pre-push review found two test helpers at the native harness's `tests/` root:
`profile_store.rs` and `store_upgrade_tests.rs`. Cargo discovered them as independent
test crates although they import their enclosing suite. Move them to matching
module directories without changing test bodies or suite names. Normal workspace
test discovery can then compile them through their actual owners. The first
release gate was deliberately stopped before this edit; its partial output is
retained separately. Normal qualification-workspace discovery now compiles all
test targets. The complete Linux `make -j1 ci` gate passed on the corrected layout,
with all 283 captured input hashes unchanged. This includes 104 Host and 61 CLI
tests, 24 installed Store cases, three publication journeys, five Blob cases,
the Store-backed profile witness, effect/custody, growth, Root and endpoint checks,
and native/Wasm lint/docs/format/tooling agreement. Three optional retained-binary
cases remain explicitly ignored by the aggregate; the prior focused Wire-adoption
run separately exercised the retained Root transition described below.

Gate log: `target/qualification/release-0213-0-ci.log`; frozen inputs and supplier
review: `target/evidence/0.213.0/release/`. This result paragraph is the only
post-gate source edit and changes no executable or release metadata. macOS and
mainnet were not exercised; no crates.io publication is authorized.

## Toko Tooling 0.44.4 and Wire 0.24.1 adoption: 2026-11-15

The requested refresh selects registry Toko Tooling 0.44.4 and IC Wire 0.24.1
in both maintained workspaces. The sparse registry index established stable,
non-yanked availability; downloaded archives matched the advertised SHA-256:
`b52bb793b386309b8c4a9e88f32711bb5d1fd11a47bf337c11c63ca2aa22fc97`
for Tooling and
`6811e5270a9c430cf0b1cf86ba4c8f6a3c15b597cd8f2f99fa2af7a4f82e1297`
for Wire. Wire's published source identifies revision
`57c17dcfb8f844a306eb81dbdc1aa5b77c51e4bc`. The authenticated tooling installer
previewed and updated the existing 47-file snapshot; only the baseline and receipt
changed. Prepared tool versions and retained executable entrypoints are unchanged.

Reviewed Wire's actual published API before changing consumers. All Canic host,
runtime, application, generated-source and qualification codec calls now use
`encode`/`decode` with explicit `Encoding::Payload` or method/direction frames.
No old-API alias, codec facade, schema rewrite, endpoint owner or implicit
lifecycle was added. Limits and DTOs remain owned by their existing contracts.
Wire 0.24.1 appeared during the fresh-starter check; its source diff against the
initially selected 0.24.0 contains only comment/documentation changes. Earlier
0.24.0 behavior evidence below therefore remains relevant; final locks select
0.24.1, and fresh starters resolve it too. Other suppliers remain out of this
requested refresh scope.

The pinned independent Blob library remains at
`decde204acb6d7c8362a0003e904beef72ff89b6` with its own Wire 0.22.4 persistence
dependency. Canic's optional adapter projects only the supplier-selected numeric
RPC limits into its own Wire type. The default Wasm graph still excludes Blob,
Storage and Runtime; the composed catalog graph has one shared Storage/Runtime
identity and two independently owned codecs. No sibling checkout was changed.

The new tooling package-directory rule moves the greeting to
`apps/hello/canic-hello`, the five profile packages to their `canic-profile-*`
directories, and the four qualification packages to their exact package names.
Both Cargo metadata inventories confirm the names. Native references and build
fixtures follow the move; historical evidence paths stay historical. Generated
source uses `starter-app/` without changing the application slot named `app`.
Existing projects, deployment identities, approvals and journals are not rewritten.

Validation and input evidence are retained locally under
`target/qualification/supplier-adoption/`. Native product tests passed (186 cases,
one explicitly installed-only case ignored), and runtime Wasm compilation passed.
Affected Clippy/Rustdoc checks passed; the Blob-specific checks were rerun after
correcting a catalog load argument during migration. The five installed Blob
cases passed, including authorization, shared references, transaction rollback,
malformed frames and exact drain/reopen/activation. The Store-backed profile
witness passed across worker/Store/owner maintenance and callback recovery.
The generated starter builds and passes its own formatting/Wasm Clippy checks.
Formatting, prepared tools and authenticated snapshot agreement pass.

With the final Wire 0.24.1 selection, all five installed Blob cases passed again
with matching before/after source and lock receipts. Installed management endpoint
admission and reply rollback passed. The generated starter's full Store-maintenance/publication/
child-upgrade witness passed, including editing/rebuilding the source application
and preserving its principal. A retained pre-migration Root built with Wire 0.22.4
upgraded to the new Root through the existing predecessor witness, preserving its
child, original approval and exact replay. This is local synthetic-cycle evidence,
not a mainnet rollout or a claim that every historical binary is compatible.
The installed development `canic` is on the normal PATH; package metadata remains
0.212.0 pending the 0.213.0 release. No publication or push was performed.

## Worker-upgrade continuation guidance: 2026-11-15

The installed callback-recovery work exposed misleading native CLI guidance:
UpgradePending always printed another apply command, even for a retained uncertain
installation needing explicit reconciliation. Repeating that command can consume
the bounded observation slots without requesting recovery. Share the existing
status projection's upgrade decision with progress output: issued upload/install
and supported failures select reconcile, ordinary progress/Ready-without-receipt
select apply, and Empty/Expired/ReserveBlocked select status for review. Retain
the original operation, scope and signer via the existing shell-safe renderer.
No new command, alias, Host authority, JSON field or management behavior is added.
Store and whole-fleet creation continuations retain their existing owners.

Reused the recorded legacy diagnostics/custody research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: presentation must preserve uncertainty,
scope and original effect authority. Status and progress now consume one upgrade
guidance function instead of contradictory decisions. The touched progress tests
move to their adjacent test module. Focused cases cover all upgrade phases,
failures, shell quoting, retained scope/signers, and unchanged outcome evidence.

All 15 status and three progress tests pass, together with CLI Clippy/Rustdoc,
formatting and diff checks. `make canic` installs the development CLI through the
normal PATH. A read-only `canic fleet status` against the earlier callback-failure
fixture reports InstallIssued, directs original-operation recovery and retains
the absence of a receipt. Evidence: `target/qualification/upgrade-guidance-*.log`.
No simulator rerun was needed for this presentation-only change. Package metadata
still reports 0.212.0; the pending increment remains 0.213.0, local and unpushed.

## Profile worker callback recovery: 2026-11-15

Continued pending 0.213.0 at the remaining installed composition boundary: a worker
upgrade can commit remotely while its callback traps before Root retains success.
Reused the existing management engine's fault point 8, the earlier installed
growth upgrade cases, and the legacy custody/readiness research below. No new
fault endpoint, retry policy or recovery engine was needed.

This exposed a product defect: the timer set its per-slot heap exclusion before
awaiting an effect, but a trapping callback never reached the manual clear. The
existing growth fixtures replaced the owner before reconciliation, hiding the
stuck flag. Replace manual acquire/clear pairs in scheduling and both reconciliation
paths with one task-owned standard-library DropGuard. Runtime 0.22.5's call cleanup
cancels and drops the suspended task; dropping the guard clears only heap exclusion,
never durable intent, reservations or attempt counts. No timeout, forced unlock,
new lease record or supplier change is involved.

Additional read-only legacy inspection at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` covered
`crates/canic-core/src/workflow/topology/guard.rs` and its path history
(`ed9af4e1b35cad3a0c5622521dd64abe02d5fb4b`,
`538c87924e8f719a471ece8ffa7558785922a328`,
`173fb90f7fc41c3c2afa1f0425780e28d2ad8c0b`), plus
`crates/canic-core/src/ops/storage/async_job_recovery/tests.rs` and the
0.104 B3 domain async-job recovery report. Preserve scoped destruction and the
tests' separation between operation identity and execution exclusion. Do not port
legacy lease takeover or watchdog machinery: this path has an executor-owned
cleanup boundary and cannot reissue uncertain management effects.

The profile witness bootstraps normally, then selects the existing fault-enabled
manager through an ordinary maintenance approval. After the worker upgrade,
the new executable serves the unchanged private records while Root remains at
InstallIssued. Replanning returns the same operation; another apply observes
uncertainty without a completion receipt. Explicit `fleet reconcile` reconstructs
the approved outcome. The test checks retained approval, exact signed admission,
effect fingerprints and counts, and precisely one child version increment. It
then continues through Store maintenance, replacement with the normal manager,
private writes and a subsequent worker upgrade. All fault selection is local,
with synthetic cycles and the fixture's isolated identity.

The first run caught a qualification reader using profile limits for management
status. It now uses the management protocol's declared limits for both encoding
and decoding; product limits and admission checks are unchanged. This proves a
recoverable committed upgrade, not arbitrary rejected or unknown installations.

The installed profile CLI witness passes in 61.49s. The affected inline growth
suite passes both current tests in 14.73s (one optional predecessor fixture remains
ignored), including creation upload/install callback recovery before owner
maintenance. The two-child lost-creation/sibling-upgrade witness passes in 7.26s.
Runtime Wasm Clippy/Rustdoc with all features, focused Root/growth qualification
Clippy, generated-app lint, formatting, shell syntax and diff checks pass.
Evidence: `target/qualification/profile-store-recovery-*.log`. No mainnet effects
or delivery occurred. The fix requires the updated management executable; recovery
of a guard already stranded in an older deployed Root is not established here.

## Profile operation after infrastructure maintenance: 2026-11-15

Continued the same pending 0.213.0 composition, extending its installed witness
rather than adding a second deployment engine or readiness mechanism. The
previous proof preserved existing records but did not exercise writes and another
worker transition after both infrastructure owners changed. Reused the legacy
custody/readiness research recorded below and the current Store-upgrade witness's
post-maintenance publication flow. Profile's existing revision/idempotency rules
remain application-owned.

Both users update their private records after Store and owner maintenance. Exact
retries return the retained record; conflicting stale writes and controller writes
are refused, with complete worker stable bytes unchanged. A new publication and
upgrade approval then selects the compatible earlier worker executable. The plan
must bind the current Root module, maintained Store module and activation attempt,
and the currently installed worker module. The worker keeps its principal and
latest full application state; this is a code change, not application rollback.
Private reads, fleet observations and an unchanged final plan remain checked.

Also moved the pending Cargo diagnostic regressions into their adjacent test
module, preserving their cases and the single Cargo diagnostic owner. No new
product endpoint, storage schema, feature flag, supplier or permission path.
Only this selected compatible profile pair is covered; arbitrary old executables
and broader allocation or mainnet recovery are not inferred from it.

The extended installed witness passed in 57.59s with
`CARGO_NET_OFFLINE=true bash scripts/dev/test-root.sh profile_store`, using the
prepared simulator and isolated signer. Localhost permission was required for
PocketIC; the first sandbox attempt could not bind its port. The three relocated
Cargo diagnostic tests, focused Root qualification Clippy, generated-app lint,
formatting and diff checks pass. Evidence:
`target/qualification/profile-store-continuation-{installed,clippy,format}.log`
and `target/qualification/build-diagnostics-relocation-tests.log`. Work remains
local and unpushed, with package versions still at 0.212.0; no mainnet effects.

## Store-backed profile composition: 2026-11-15

The next coherent increment is pending 0.213.0, including the earlier build
diagnostic repair. Package versions remain at released 0.212.0 until delivery.
The profile manager explicitly mounts existing Store publication, custody,
creation/upgrade, maintenance and public observation endpoints. Its readiness
decoder, lifecycle, memory allocation and timer ownership are unchanged. The
generic greeting Root is unchanged. The selected new consumer has one manager
in the Root role, one Store and one profile worker; existing inline/mainnet
deployments are not migrated.

Reviewed the actual profile worker's strict Storage opening and authenticated
user operations, the manager's profile readiness decoder, the management engine
and Host's Store-bound admission/maintenance checks. Reused the recorded legacy
Root-bootstrap research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: installing
code is not readiness, and dependent work waits for retained custody. This use
case needs the existing application-owned decoder, not a generic readiness
registry, greeting facade, copied provisioning engine or new package. No runtime
Wire format, stable-memory schema or supplier version changes.

The installed profile Store witness passed in 49.54s. The real CLI creates the
fleet, publishes and upgrades the worker, replaces Store, then replaces the
management owner. Two users retain their full profiles and cannot read each
other's data; the worker's controller also receives Forbidden. Worker stable
bytes remain unchanged across both executable transitions. Store replacement
compares complete preservation evidence before/after installation; its explicit
activation legitimately advances the maintenance fence. Store backing is then
unchanged across owner maintenance. Fleet views report three canisters and Ready
creation, upgrade and Store observations without inventory issues.

A separate incompatible greeting is installed under the profile owner. It stays
Verified with a readiness failure, original approval and exactly one creation,
upload and installation attempt. Reapplying retains the same evidence without
producing a completion receipt. This is accepted blocked qualification exposure,
not permission to reinstall or bypass readiness.

Store maintenance's anonymous-operator rejection remains intact. Both this
witness and the existing Store-upgrade witness now share isolated test-identity
preparation; credentials remain in fixture directories. The existing Store
interruption/recovery witness passed in 57.73s with that helper. The affected
inline profile-preservation test also passed; manager Wasm Clippy, qualification
Clippy/Rustdoc, generated-app lint, formatting, shell syntax and diff checks pass.
The documented init/add/build flow builds all three packages with no identity
bindings or journal creation. Evidence: `target/qualification/profile-store-*.log`
and `target/qualification/profile-store-config-1sqhtg75/`.

Only the selected one-worker composition is qualified here. Broader allocations,
arbitrary readiness protocols, Blob fleet lifecycle and mainnet Store maintenance
remain open. No push, mainnet effect or browser launch occurred.

## 0.212.0 post-push review and 0.212.1 build diagnostics: 2026-11-15

GitHub main and peeled `v0.212.0` match
`724744bab2a5bf6972f3021431e6da0e66b55b26`; the annotated tag object is
`04e445d86dc3487f0c29b372c6413c674e828dd3`. Local main and origin/main match.
Compared the pushed tree to the gate inputs: only the documented release-result
entry differs. Reviewed the actual generator/status changes, build and planning
callers, installed fixtures and current guidance. No new deployment owner,
authority bypass, compatibility alias or migration claim was introduced. The
greeting-specific readiness contract remains an explicit qualification boundary.

One actionable usability issue surfaced in the generated-source workflow:
`artifact::build` and app-build lock preparation retained Cargo output through
Host Process but formatted only the supplier's deliberately terse exit error.
Pending 0.212.1 adds one Cargo-specific diagnostic projection shared by these
callers. It selects the first compiler error or Cargo stderr, limits the preview
to 4,096 characters and marks truncation. It preserves the original ToolError and
its bounded evidence. Process execution/cleanup stays supplier-owned; signer and
provider error handling is unchanged. The native Host package now directly uses
the already selected Host Text escaper; no supplier version or Wasm graph changes.

Reused the recorded starter/build ownership and legacy config-discovery diagnostic
research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: keep one builder, escape
untrusted presentation and preserve original evidence. Three focused tests pass,
covering compiler-vs-stderr selection, control escaping, Unicode/truncation and
an actual missing Cargo package that preserves an existing build output.
Host/CLI Clippy and formatting pass. `make canic` installed the development CLI.
Its generated-project smoke reports the actual E0425 source location, preserves
the previous Wasm on failure, then builds all three canisters after correction
with the same lockfile. Missing local dependency during lock preparation also
reports the cause and preserves the existing Wasm. Evidence:
`target/qualification/build-diagnostics-{tests,clippy,format,install}.log` and
`target/qualification/build-diagnostics-qi2u7yl8/`.

The next increment is local and unpushed; package versions remain at released
0.212.0. No mainnet effect or browser launch occurred.

## 0.212.0 release qualification: 2026-11-15

The maintainer authorized GitHub delivery and subsequent development. All fourteen
product packages select 0.212.0; qualification packages retain 0.0.0. Both locks
change only Canic package versions. Suppliers, publication restrictions and
existing deployment journals are unchanged.

The full Linux `make -j1 ci` gate passed with all 280 captured inputs unchanged.
It includes 101 Host tests, 60 CLI tests, 24 installed Store cases, three CLI
publication journeys and five Blob cases, plus effect/custody, profile, growth,
Root and endpoint qualification and native/Wasm lint/docs/format/tooling checks.
Three optional historical-binary cases remain explicitly skipped. The publication
journeys took 106.87s; generated bootstrap measured 8,108 ms and the rebuilt app
upgrade 5,177 ms after publication. These are local samples, not mainnet claims.
macOS and mainnet were not exercised; no crates.io publication is authorized.

Gate log: `target/qualification/release-0212-0-ci.log`; input evidence:
`target/evidence/0.212.0/release/`. This release-result entry is the only change
after input comparison and does not alter executable or release metadata inputs.

## Generated application rebuild loop: 2026-11-15

Continued pending 0.212.0 by replacing the Store-upgrade witness's prebuilt Hello
World replacement with a source edit and build of the actual generated app. It
checks the original greeting, a warm all-recipe build with unchanged artifacts
and Cargo.lock, publication of the edited app, and the changed greeting at the
original principal after upgrade. The later app-only artifact build deliberately
preserves the feature-selected infrastructure used for maintenance faults. The
fixture reads its package from the declared recipe instead of inferring it from
the logical app name; the two-child walkthrough now uses the correct `starter-app`
package as well. Removed the unused replacement-artifact build from the runner.

Reused the scaffold/Root ownership research cited in the preceding work and
reviewed the actual Root/profile-manager readiness implementations. The supplied
Root decodes a greeting probe; the profile manager decodes profile readiness.
Neither is an arbitrary-application readiness contract. Recorded that boundary
in the design and generated guidance: the next Store-backed stateful consumer
must qualify explicit application readiness and preserve its selection for
retained work. A greeting-compatible facade or install success cannot replace
that proof. This increment adds no endpoint, runtime policy, dependency or
alternative management workflow, and does not claim a persistent replica loop.

The focused installed witness passed in 58.41s with synthetic cycles, including
the retained Store/Root interruption cases. Bootstrap measured 8,193 ms and the
generated app upgrade 5,233 ms after publication; these are local samples, not
mainnet estimates. Qualification Clippy, formatting, shell syntax and diff checks
passed. `make canic` installed the updated generated guidance. Evidence is in
`target/qualification/starter-rebuild-{installed,clippy,format,install}.log`.
No push, mainnet effects, browser launch or supplier changes occurred.

## Generated Root/Store/app starter: 2026-11-15

Pending 0.212.0 now generates the already selected topology: supplied Root and
Wasm Store, plus the generated greeting app. Source initialization remains local
and refuses occupied destinations; config-only initialization is unchanged.
Existing projects, bindings and journals are not migrated. The default reviewed
creation debit is 4T (3T Root including 1T app funding, plus 1T Store). New app bytes
use the existing publication approval followed by a separate upgrade approval.
The pending version is a minor increment because generation and budget defaults
change; package manifests remain at released 0.211.1 until delivery.

Reused the recorded legacy scaffold, Root bootstrap and versioned Store research
at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: refuse destination replacement,
retain effect results before dependent work, and gate admission on artifact
readiness. The existing convergence owner already implements those boundaries;
this change selects it through declarations rather than adding an installer,
workflow engine, feature switch or copied Root/Store implementation. Store stays
outside the generated application's runtime dependency graph. Supplying-package
validation now shares one loop instead of repeating manifest checks.

Focused validation passed:

- Five Host starter tests and 15 CLI configuration tests, including no-effect
  generation, shared definitions and refusal before publication.
- The installed publication runner now initializes and builds the actual default
  starter. Normal Root/Store/app creation completed in 8,148 ms locally; the later
  child upgrade took 4,973 ms with publication already retained. The same witness
  passes Store interruption/recovery and subsequent Root/child maintenance checks.
  Fault code is selected only through reviewed maintenance after normal bootstrap.
- Seven affected inline CLI/predecessor cases pass with explicit inline fixture
  declarations, including two children, reply loss, untracked-work refusal and
  private profile preservation. Test filters reach the Root harness; unrelated
  runtime cases were not repeated.
- Host/CLI and affected qualification Clippy, Host Rustdoc, generated app lint,
  Rust/manifest formatting and diff checks pass. `make canic` installed the local
  development binary; plain `canic app init`, app/fleet canister listings display
  three unbound canisters and create no journal.

Logs are `target/qualification/starter-store-{native,cli,installed,inline,clippy,doc,format,install}.log`.
The application remains stateless; existing-fleet Store adoption, general trees,
mainnet Store maintenance and unlimited history are not qualified. No supplier,
dependency, permission, scheduler or authority contract changed. No push,
crates.io publication, mainnet effect or browser launch occurred in this increment.

## 0.211.1 post-push review and next increment: 2026-11-15

Verified GitHub main and peeled `v0.211.1` at
`05ab75a2fb5cf8276236802cea0a621089357f95`, with annotated tag object
`7c7d1af437544ad426e56ada440b600983b39920`. Local main and origin/main match.
Compared the pushed tree against the release input hashes: only this history
file differs, by the recorded release-result entry added after validation.
Reviewed the actual pushed plan/admission and runtime changes, their native and
installed witnesses, Store guidance and direct rendering callers. No competing
effect owner, renewed deadline, second recovery workflow or unnecessary dependency
was found in that scope. Older-Root timing support and mainnet qualification remain
explicit limits, not inferred compatibility.

One presentation issue remained: scheduled dispatch reused the generic deadline
renderer and displayed `expired`, even though it is the original admission cutoff
and not proof of operation failure. The local display cleanup, now included in
pending 0.212.0, uses the existing
UTC formatter to label that timestamp explicitly; effect expiry and ready-to-dispatch
wording stay distinct. This adds no policy or runtime mechanism. Reused the existing
retained-evidence/legacy progress research recorded above; the invariant remains
that timing and request counts do not claim success or failure.

Seventeen focused status/progress tests, CLI Clippy, formatting and diff checks
passed. `make canic` installed the development display. The real retained scheduled
fleet from `target/publication/store-upgrade-1655996/journal` changes exactly one
human-output line; before/after JSON is byte-identical. Evidence:
`target/qualification/0211-2-*`. This next increment is local and unpushed; package
versions remain at released 0.211.1 until the next release. No mainnet operation
or browser launch occurred, and crates.io publication remains disabled.

## 0.211.1 release qualification: 2026-11-15

The maintainer authorized GitHub delivery and subsequent development. All fourteen
product packages select 0.211.1; qualification packages retain 0.0.0. Both locks
retain the confirmed rand 0.10.4 update and otherwise preserve supplier selections.
Packages remain unpublished; this release authorizes neither crates.io publication
nor mainnet effects. Scope includes ready-to-dispatch planning, bounded runtime
admission, compact Store progress and installed publication measurements.

The full Linux `make -j1 ci` gate passed with all 280 captured input hashes unchanged.
It includes 101 Host tests, 60 CLI tests, 24 installed Store cases, three CLI
publication journeys and five installed Blob cases, plus effect/custody, profile,
growth, Root and endpoint qualification. Native/Wasm Clippy, Rustdoc, formatting
and tooling agreement passed. Three optional historical-binary cases remain
explicitly ignored; the Host's separately ignored installed entrypoint is exercised
by Store's complete-custody harness. macOS and mainnet were not exercised.
The publication journeys took 106.08s together and Blob 7.42s. In this run the
default bootstrap measured 8,199 ms and the later child upgrade 4,771 ms; those are
local samples with the limits recorded above, not mainnet performance claims.

Gate log: `target/qualification/release-0211-1-ci.log`; before/after source evidence:
`target/evidence/0.211.1/release/`. This release-result entry was added after the
matching input comparison; executable inputs and release metadata remain unchanged.

## Ready-to-dispatch Host and CLI journey: 2026-11-15

Connected the qualified runtime contract to fleet and named child planning.
New plans use one captured timestamp for plan creation and earliest dispatch,
with expiry ten minutes later. Host's single admission-cutoff function uses that
expiry only when the two timestamps are equal; explicit future timestamps keep
their earlier pre-dispatch cutoff. All historically valid plans had strictly
future dispatch, so their original digests, cutoffs and signed attempts remain
unchanged. Replanning still returns the retained approval, including after expiry.
Bootstrap and artifact publication consume the same finite window; no phase gets
an independent renewal. Existing Root code may refuse late admission, so it needs
an explicit future schedule or separately reviewed maintenance before this default.

Reused the legacy/bootstrap analysis and runtime proof in the following entry.
The change adds no wire field, serialized timing mode, endpoint, configuration
switch, scheduler or retry owner. Plan/expiry validation admits equality but still
refuses dispatch before plan creation. The existing CLI help and status renderer
now describe readiness instead of showing a fictitious future wait; generated
starter instructions and current demo/design guidance use the same workflow.

Twenty focused Host tests passed, including historical digest fixtures, repeated
planning, named defaults, explicit cutoffs, and expired unsubmitted immediate and
scheduled fleets refusing to sign creation. Seventeen CLI status/progress cases,
Host/CLI and publication-harness Clippy, Host Rustdoc, formatting and diff checks
passed. The installed publication Store-upgrade journey passed in 49.38 seconds:
default fleet plan/apply created Root, Store and app in 7,873 ms, then after Store
recovery and Root maintenance a default child plan/apply upgraded the existing app
in 4,870 ms. The latter timing excludes its earlier artifact publication. Neither
uses `--execute-at` or simulated clock jumps. These are individual local samples,
not mainnet performance measurements or comparisons with the earlier publication-only
baseline. Existing lost-reply, original-authority and complete-custody checks remain.

`make canic` installed the local development CLI. Its actual retained fleet status
reports `Child dispatch: when approved prerequisites are ready` alongside the
original effect deadline. The installed help also describes the new default.
Logs and captures: `target/qualification/ready-planning-*`; installed journal:
`target/publication/store-upgrade-2111591/journal`. Both retained lockfile hashes
stayed unchanged, preserving the maintainer-confirmed rand update. This increment
remains local under 0.211.1, with package versions still at released 0.211.0.
No push, mainnet operation or browser launch was performed.

## Earliest-dispatch runtime admission: 2026-11-15

The latency investigation found two meanings attached to `execute_at_ns`: the
earliest timer dispatch and the last acceptable admission time. The protocol now
uses it only as the earliest dispatch bound. Creation and upgrade admission accept
that timestamp at or before current time while the original expiry is still future.
Creation keeps its ten-minute interval bound. Upgrade explicitly bounds the
original interval to one hour, preserving the restriction previously implied by
future-only admission plus the one-hour expiry horizon. No timestamp, deadline,
request commitment or retry allowance is rewritten.

Reused the pinned legacy Root-bootstrap and retained-work research under
[the Root extraction](#one-child-root-implementation)
and the [Store decision](#versioned-wasm-store-decision-2026-11-15): revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, Root bootstrap
`crates/canic-control-plane/src/workflow/bootstrap/root/mod.rs`, publication chunk
custody and bootstrap diagnostic history `ed9f133`. Their invariant is retained
authority/artifact custody before deferred work and truthful readiness. They do
not require a fixed ten-minute delay. The current Root already has a durable
approval and one timer owner; changing admission needs neither another scheduler
nor a new endpoint, wire shape, stable record or supplier.

`bash scripts/dev/test-store.sh dispatch` passed two installed witnesses in 8.05s.
One uses the real Root and Store to create and upgrade after the earliest timestamp,
without an operator continuation. Exact approvals, attempts, references and installed
modules survive Root maintenance; expired admission leaves state unchanged and exact
replays after expiry retain completed evidence. The other proves future work still
waits and a timer first run after expiry performs no creation. Native boundary
checks cover equality, expiry, oversized intervals and integer extremes. Protocol
and installed-harness Clippy, runtime Wasm Clippy, protocol Rustdoc, formatting,
shell syntax and diff checks passed. Logs: `target/qualification/ready-dispatch-*`.

This qualifies the runtime step, not an immediate-dispatch Host journey. The Host
still defaults to ten minutes and refuses fresh admission after that time. The next
step must bind readiness to the immutable plan and verify the public CLI path before
changing that default; older Roots remain unsupported for elapsed-time admission.
No mainnet effects or release were performed. The maintainer confirmed preserving
the pre-existing `rand 0.10.4` lockfile update; its input hashes stayed unchanged
during these checks. Earlier local 0.211.1 status and measurement work is retained.

## Local publication measurement baseline: 2026-11-15

Extended the existing installed publication witness rather than adding a benchmark
runner, CLI mode or runtime metrics owner. This follows design section 12 and the
[versioned Store research](#versioned-wasm-store-decision-2026-11-15): legacy
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` publication/chunk custody and bootstrap
diagnostics establish reuse and retained-evidence requirements, but supply no
latency benchmark. No scheduling or authority contract changed.

The first measurement assertion incorrectly equated file size with uploaded
payload. The real installed run exposed existing intra-Wasm chunk reuse. The
corrected witness independently hashes input chunks, compares unique counts/bytes
with decoded retained Put requests, and requires zero requests for warm reuse.
The padded 2,044,909-byte artifact retained three unique Put payloads totaling
734,189 bytes; its second version retained zero. A passing local sample measured:

| Command segment | Host wall time |
| --- | ---: |
| Cold plan | 54 ms |
| Prepare interrupted before first chunk signing | 1,131 ms |
| Observation-only reconcile | 747 ms |
| Resume upload and publish | 2,292 ms |
| Warm version plan | 59 ms |
| Warm version publication | 1,343 ms |

These are command durations including signer/transport/observation overhead, not
isolated Store execution or full deployment latency. Builds, replica setup, Store
bootstrap, child scheduling/installations and cycles are excluded. Payload totals
describe retained completed requests, not network retransmissions. No speedup,
mainnet performance or timing threshold is claimed; original ten-minute default
dispatch timestamps remain unchanged.

The selected installed journey passed in 29.57 seconds, including its existing
interruption, version retention, child creation/upgrade and cache-reconstruction
checks. Harness Clippy, formatting and diff checks passed. Evidence is in
`target/publication/run-1972926/publication-measurements.json`, with artifact hashes
and original operation IDs; logs are `target/qualification/publication-measurements-*`.
No dependencies, product endpoints, recovery owners or runtime state were added.
Changes remain local in the pending 0.211.1 entry.

## 0.211.0 post-push review and 0.211.1 status cleanup: 2026-11-15

Verified GitHub main and peeled `v0.211.0` at
`7092a627f49b55f82f0c9bc518dd750891d98256`, with annotated tag object
`cb7c212ec9c7d2fa85116b3e186875b0d6efcad6`. The local tracking reference was
refreshed. Reviewed that delivered commit and its direct Root/Store, signed
authority, fleet-planning, status and subnet-comparison callers. The existing
runtime remains the sole effect owner; complete custody selection is explicit,
and subnet diagnostics do not acquire journal or deployment authority. One Store
attempt, the infrastructure size bound and mainnet qualification remain deliberate
limits rather than hidden compatibility fallbacks.

One presentation issue was actionable: Store status printed every retained scan
and continuation row, including placeholders with no submitted attempt. The
completed installed witness produced 74 lines. The new 0.211.1 draft groups these
states through the existing IC Host table renderer; its same retained operation
now produces 31 lines. Rejections, unknown outcomes, the original deadline and
historical receipt stay visible. Counts do not establish completion. Full JSON
is byte-identical before/after and retains individual attempt paths. No Host
journal, protocol, effect, endpoint, command or dependency changes were needed.

Reused the pinned legacy progress-rendering research under
[readable retained-operation status](#readable-retained-operation-status), including
revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` and history `afc2521`:
unknown readiness and applied counts cannot claim success. The focused regressions
exercise 1024 retained replies alongside unresolved, rejected and unsigned requests,
bounded terminal output, absent observations and unchanged source evidence.
All 11 CLI status tests, CLI Clippy, formatting and diff checks passed. `make canic`
installed the development display; the actual local status command confirmed the
line reduction and unchanged JSON without network observations or a browser.
Logs: `target/qualification/store-status-*.log` and matching text/JSON captures.
The patch remains local and unpushed; package versions stay at released 0.211.0
until the next release. The completed release gate remains evidence for the
unchanged runtime; it was not repeated for this presentation-only patch.

The next focused continuation shares one CLI Store renderer between local status
and apply/reconcile outcomes. Issued effects and post-install verification suggest
restricted reconcile even after dispatch expiry. Fresh-effect phases suggest apply
only before the original deadline and without a retained failure; otherwise they
suggest local status review. This is presentation, not a second admission policy:
Root and Host still enforce the original operation. Catalog scan counters distinguish
incomplete, complete and absent metadata evidence without claiming payload integrity.
The same legacy retained-evidence research applies; no runtime or journal changes
were needed. Seventeen focused status/progress tests, CLI Clippy, formatting and
diff checks passed, covering every phase, exact expiry, missing host time, retained
failures and shell-safe scope/signer follow-ups. `make canic` installed the local
change. The retained installed witness now renders 32 lines, including both catalog
scans; its JSON remains byte-identical. Logs and captures:
`target/qualification/store-guidance-*`. No deployment, network observation or
browser launch was needed. This continuation remains local and unpushed.

## 0.211.0 release inputs: 2026-11-15

The maintainer authorized the GitHub release and subsequent development. Scope
includes restricted Store recovery, complete Root custody, the installed fleet
Store upgrade workflow and the completed subnet-comparison work. Product packages
and both lockfiles select 0.211.0; qualification packages retain 0.0.0. Packages
remain unpublished, with no crates.io or mainnet deployment selected.

Fresh registry review confirms the locked first-party selections remain current:
Host fs/process/artifacts 0.79.0, Host text 0.76.0, Platform 0.33.7, Runtime 0.22.5,
Types 0.20.4, Storage 0.74.1, Timers 0.44.3, Wire 0.22.4, Testkit 0.106.0 and
Tooling 0.44.3. Query main remains the selected topology revision
`0a3eca919f05d777798406087049cc51ab7ae2a8`. Blob's default branch remains
`86fc663d14b5fde78db1267157ce9b1b9dbe2888`; its optional update from the qualified
`decde204acb6d7c8362a0003e904beef72ff89b6` is deferred to the next supplier-adoption
batch. Its registry release remains 0.82.1; no sibling checkout was changed.

The complete Linux `make -j1 ci` gate passed with all 278 captured input hashes
unchanged. This includes 100 Host tests, 54 CLI tests, 22 installed Store cases,
three CLI publication/upgrade journeys and five Blob cases, plus the effect,
artifact, profile, growth, Root and endpoint suites. Native/Wasm lint, Rustdoc,
formatting and tooling agreement passed. The native Host entrypoint ignored by
ordinary unit tests ran through its installed Store harness. Optional historical
manager/Root fixtures remain explicitly ignored. macOS and mainnet maintenance
were not exercised. Logs: `target/qualification/release-0211-*.log`; captured
inputs: `target/evidence/0.211.0/release/`.

## Fleet commands carry Store replacement through later upgrades: 2026-11-15

Continued the pending 0.211.0 increment with `PlanAction::UpgradeStore` and the
existing fleet plan/apply/status/reconcile path. A changed Store artifact selects
one approval after completed child/publication work; simultaneous artifact changes,
unfinished work and a second Store attempt are refused. The approval binds exact
Root/Store code, original operator/deadline and replacement bytes. Root remains
the management-effect owner. Host retains signed admission and continuations;
reconcile uses only the restricted recovery contract. Each apply permits 32
continuations with a 4096-observation lifetime bound. Exhaustion retains evidence.

Completion points to the exact admitted signed Ready observation rather than
introducing another receipt or runtime journal. Ordered fleet history advances
the Store binding only through that evidence. Root maintenance automatically
preserves the complete Store operation. `StoredArtifact.maintenance` binds the
completed attempt for later child plans and explicitly selects complete Root
custody; absence preserves old serialized approvals, digests and read methods.
The installed test first exposed the old child-only reader refusing the retained
operation. That refusal remains; the new source-bound selection fixes the caller
instead of weakening the preservation contract or probing/falling back on errors.

Reused the legacy checkpoint/late-callback research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded below: preserve original
operation authority and treat callback evidence separately from remote effects.
This is Host composition over the rewrite's qualified engine, not a port of the
legacy workflow machinery. No supplier, crate, dependency, allocator, timer or
new CLI command was added. The signed metadata reader now serves typed publication
and Store observation callers under its existing authority-retention mechanism.

Focused validation passed:

- 40 Host cases across plan, fleet convergence, managed source custody, signed
  authority, publication, maintenance and Store upgrade; the separately executed
  installed Host entrypoint remains ignored in ordinary native runs. After adding
  explicit full-custody rejection cases, the affected 20 cases passed again.
- Nine CLI status cases, including visible Store phase without a false completion.
- The new installed CLI/signer witness (82.39s) bootstraps Root/Store/app, preserves
  pending approvals, refuses reconcile before admission, stops before installation,
  loses upload callback 15, recovers by observation, pauses before activation and
  compares Store stable bytes. Missing/substituted completion custody blocks status
  and planning. It then replaces Root, publishes and upgrades the child through
  complete custody, and refuses a successor Store upgrade.
- The existing named bootstrap/publication/child journey (29.42s) still passes,
  retaining the original Store read contract and signed recovery behavior.
- Host/CLI Clippy, publication-fixture Clippy, Host Rustdoc, formatting and tooling
  agreement passed. Logs: `target/qualification/store-workflow-*.log`.

The fixture uses generated isolated credentials and synthetic cycles; no operator
wallet, mainnet deployment or browser is involved. The runtime is unchanged from
the preceding complete-custody increment, so its passing Store interruption and
growth qualification remains applicable. The 1 MiB infrastructure bound, one
Store attempt, no reset/cancellation/successor maintenance and no mainnet
maintenance claim remain explicit. `make canic` installed the development CLI on
PATH and command help was checked. Package versions remain 0.210.0; these changes
are local and unpushed under the pending 0.211.0 entry.

## Root replacement carries the Store operation: 2026-11-15

Continued after 0.210.0 with complete controller-only Root custody (Wire method
57): both child slots, all artifact sources and the optional Store operation are
read synchronously from the existing stores. No remote call, write or lifecycle
action occurs. Older child-only readers still refuse a retained Store operation,
including Ready. Reused the same legacy checkpoint/late-callback research recorded
below at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`; preserving the surviving
intent remains essential even when replacing its owner.

Host's `StoreMaintenance.operation` selects the complete read on both sides of
the existing direct installation. Exact evidence, Store identity/code and live
controller/placement checks remain required. Absent operation keeps the old
serialized plan, digest and signed read method. The public Rust struct addition
requires literal callers to adapt, so the pending entry is now **0.211.0**;
the preceding recovery work is included. Package versions remain 0.210.0.

Focused validation passed: four Host maintenance tests, combined maximum-custody
Wire budget, and the installed Host replacement witness (11.16s). The latter
creates a real Store-backed child, closes/stops Store, refuses changed operation
evidence before installation, interrupts after Root installation and resumes the
same signed request ID. It verifies the new Root module, exact pre/post custody,
unchanged Root/Store stable bytes and recovery by the original Store operator.
Malformed and unauthorized custody reads preserve state. Four Store maintenance
cases (8.12s) also compare complete custody at the existing interruption phases,
including Ready. Shared growth checks passed (two cases, 14.33s; the optional
historical-manager input remains explicitly ignored). Native/Wasm Root and Host
Clippy, installed Store fixture Clippy, Root/Host Rustdoc, formatting, tooling
agreement and diff checks passed. Logs: `target/qualification/root-custody-*.log`.

Host and the simulator retain separate dependency graphs: the installed harness
invokes a Cargo-built Host test executable, as existing CLI witnesses invoke the
CLI. Testkit's PocketIC requires thiserror 2.0.18 while the selected Host/Query
graph requires 2.0.21; no dependency pin was changed to combine them. The native
entrypoint is explicitly selected by `test-store.sh root-maintenance` and the
aggregate Store runner, rather than accidentally skipped with ordinary unit tests.
No new product crate, journal, allocator, timer or workflow engine was added.

The installed witness supplies an explicit library approval. Whole-fleet planning
still needs authenticated Store-upgrade receipts before selecting that operation;
it cannot promote the public diagnostic report into authority. Store-upgrade
plan/apply/reconcile, successor maintenance and mainnet use remain unqualified.
This work is local and unpushed, with no deployment or publication.

The separately coordinated subnet-comparison work is retained in the same pending
entry. Its owner reports two Host and four CLI tests, production Clippy/Rustdoc,
and one installed read-only mainnet smoke. Exact scope and source evidence are in
[the comparison note](../design/fleet-subnet-comparison.md); those results are
independent of this Store qualification and add no deployment authority.

## Restricted Store recovery for 0.210.1: 2026-11-15

Added the explicit Root update `recover_store_maintenance` (Wire method 56).
It selects the exact retained approval and original operator, captures the phase
before awaiting and reuses the existing observation branches and journal. It can
retain outcomes and complete post-install metadata verification; it cannot issue
new maintenance effects. Reserved, Closed, Stopped, Uploaded, Installed, Verified
and Ready remain unchanged even before dispatch expiry. Existing methods, stored
formats and authority deadlines remain unchanged. No new crate, timer, storage
allocation, workflow engine or copy of supplier policy was introduced.

Reused the legacy checkpoint sequence/late-callback research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded under “Store admission survives
maintenance”: exact operation selection and retained outcomes remain the relevant
invariants. This increment changes recovery access, not the Store lifecycle.

Focused Linux validation passed: the scoped protocol budget test; four installed
maintenance cases (7.90s); four installed closure/stop and migration cases (7.18s);
and `make growth-test` (two installed cases, 13.87s; the optional historical-manager
input remains explicitly ignored). Recovery fixtures cover concurrent calls,
Root replacement, expired authority, unchanged Store bytes, malformed frames,
wrong callers, altered attempts and an old attempt unable to reopen a newer gate.
Native/Wasm Root Clippy, installed Store fixture Clippy, Root Rustdoc, formatting,
tooling agreement and diff checks passed. Logs are under
`target/qualification/store-recovery-*.log`.

This is local installed runtime qualification, not a Host/CLI Store upgrade or
mainnet claim. Host plan/apply/reconcile and carrying the Store operation through
Host-driven Root maintenance remain the next integration work. The new increment
is uncommitted and unpushed; package versions and the installed CLI remain 0.210.0.

## 0.210.0 post-push review: 2026-11-15

Verified remote main and peeled `v0.210.0` at
`5b8d5e91ac601da13ec9d50e8d7a4f4f5efa16bb`, with annotated tag object
`4b4366b3677be65be69312fcdb6553307940f7bb`. Reviewed the delivered changes and
their runtime, Host, CLI, configuration and qualification callers before starting
new work. Store remains the sole durable artifact owner; Root reuses its bounded
journal, and Host views project diagnostics without effect authority. Placements
uses Query's bounded collector and keeps diagnostic routing separate from
deployment authority. Historical journal codecs/fixtures and refusal by old
child-only custody methods protect real preservation obligations and remain
deliberate compatibility, not removable leftovers.

No additional corrective defect or duplicate policy owner was found in the pushed
scope. The documented Host upgrade gap remains: generic advance can dispatch new
effects, so observation-only reconciliation needs an explicit restricted contract;
later Host-driven Root maintenance must also carry the retained Store operation.
The next compatible increment starts at 0.210.1 with the recovery boundary. This
does not claim repository-wide absence of debt or a completed CLI upgrade journey.

## 0.210.0 release inputs: 2026-11-15

The maintainer authorized delivery and confirmed the concurrent placement work
finished and included. Product packages select 0.210.0; qualification package
identities remain 0.0.0. Scope includes the bounded Store replacement runtime,
public fleet diagnostics and configured placement collection. GitHub-only delivery
remains selected; all packages retain `publish = false`.

Fresh registry review found the locked first-party selections current: Host
fs/process/artifacts 0.79.0, Host text 0.76.0, Platform 0.33.7, Runtime 0.22.5,
Types 0.20.4, Storage 0.74.1, Timers 0.44.3, Wire 0.22.4, Testkit 0.106.0 and
Toko Tooling 0.44.3. Placement selects unpublished `ic-topology` 0.106.1 at
`0a3eca919f05d777798406087049cc51ab7ae2a8`, verified as Query's current main
revision; its registry index is absent. Blob remains at qualified Git revision
`decde204acb6d7c8362a0003e904beef72ff89b6` (0.100.6), independently of its older
registry release 0.82.1. Reviewed its default branch again at
`86fc663d14b5fde78db1267157ce9b1b9dbe2888`; optional adoption is deferred to the
next dependency-maintenance batch so this release preserves the installed Blob
witness's selected input. Neither supplier checkout is changed.

Release review found anonymous-controller admission could retain Store maintenance
that Root's existing reopen validation would reject. The initial aggregate run
was interrupted before editing (`release-0210-ci.log`). Admission now refuses
that operator before retaining anything. The installed CLI witness checks the
refusal and unchanged stable bytes, then hands Root to an explicit operator for
maintenance while continuing to observe anonymously through the CLI.

The next aggregate run passed, but its post-run input comparison found four late
placement-summary changes from the concurrent Query session: CLI presentation,
its tests, its test-only topology dependency and the corresponding lockfile edge.
The owning session confirmed it had stopped Canic edits/builds. Reviewed those
changes and included the summary of read/configured/unresolved/unbound counts and
distinct observed subnets; no extra queries or JSON changes. That mixed-input run
(`release-0210-ci-final.log`) is not the final release gate. The combined candidate
is captured and validated again below.

The final combined Linux `make -j1 ci` gate passed with all 267 captured input
hashes unchanged before/after. This includes 95 Host tests, 49 CLI tests, all 21
installed Store cases, both CLI publication journeys, five installed Blob cases,
and the effect/custody, profile, growth, Root and endpoint qualifications. Native
and Wasm Clippy, documentation, formatting and tooling agreement passed. Optional
historical manager/Root inputs remain explicitly ignored; macOS and mainnet were
not exercised by this release gate. Evidence is retained under
`target/evidence/0.210.0/release/`; final output is
`target/qualification/release-0210-ci-combined.log`. `make canic` installed and
verified 0.210.0 on the existing PATH without opening a browser or changing PATH.

## Fleet views expose Store maintenance: 2026-11-15

Continued pending 0.210.0 with read-only Host/CLI visibility before admitting a
Host-driven replacement workflow. Reused the legacy registry/tree/list and
topology research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded under
“Application tree and role correction”: declared relationships do not grant
authority, unknown observations remain unknown, and exact network bindings must
stay separate. No legacy registry or discovery mechanism was imported.

Root explicitly mounts `store_maintenance_status`, a public bounded query over
the existing retained metadata. The private replicated custody endpoint remains
controller-only. Host selects the query solely from retained Store bootstrap
authority and a known creation result, deduplicated by network/Root/Store, with
at most sixteen reports inside the existing thirty-second snapshot budget.
Configuration role edits cannot select queries. A foreign Store report or missing
method remains unknown; public reports cannot add canisters, replace identities,
resume effects or create completion receipts.

Terminal and localhost views share phase wording. The ordinary view stays compact;
details expose the original deadline and both catalog scan counts. Verification
waiting for explicit activation differs from completed historical maintenance.
Neither claims current Store admission or readiness. JSON carries the same
bounded report. There is no new persistent state, artifact copy, signature owner,
workflow engine, timer, crate or dependency for this change.

Focused validation passes: 32 Host fleet cases, 20 CLI fleet/render/watch cases,
the Store bootstrap projection and scoped public/private Wire-budget probes.
Four installed Store maintenance cases pass (7.81s), comparing anonymous public
reports with controller-only observations through stopped/uncertain/verified/
completed states and Root replacement. Malformed queries preserve state; public
access does not open the private endpoint. The actual whole-fleet CLI journey
passes (107.60s), retaining four canister identities and observing no attempt
before maintenance, then Stopped while Store is unavailable. Repeated views leave
both Root and Store stable bytes unchanged. Native/Wasm/qualification Clippy,
Rustdoc, formatting and tooling agreement pass. Logs use
`target/qualification/store-fleet-*.log`.

Concurrent placement work was identified and confirmed by the maintainer; its
files and dependency changes were preserved. This increment does not qualify
that work or change its ownership. Store upgrade execution through Host's existing
plan/apply/status/reconcile flow, retained Store-operation custody during subsequent
Host-driven Root maintenance, repeated attempts and mainnet remain pending.
No package version bump, installation to the operator's PATH, commit, push, browser
launch or live deployment was performed for this increment.

## Root installs and activates its Store: 2026-11-15

Continued pending 0.210.0 from the bounded preservation scan. Reused the legacy
manifest/custody and checkpoint interruption research recorded below; legacy did
not establish the rewrite's upgrade guarantee. Inspected the existing child
upgrade owner and Platform's actual Wasm management exports. `InstallCodeArgs`
is native-only in Platform 0.33.7, so the runtime uses its existing
`UploadChunkArgs`/`InstallChunkedCodeArgs` contract instead of adding a parallel
management type or supplier change. The
[management specification](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/#ic-method-install_chunked_code)
defines installation from the controlled same-subnet management cache.

Explicit installation supplies at most 1 MiB of approved raw Wasm. Root uploads
once into Store's management cache and retains the upload commitment, then issues
one exact chunked-install request with its original sender version. Host owns the
durable bytes; Root never stores them. Installation uncertainty is resolved only
by approved module/sole-controller/stopped observation. The continuation starts
Store with admission closed and repeats the same metadata scan. Only a complete
matching result admits explicit activation of the original attempt. No implicit
activation, deadline renewal or upload/install/start/activation replay exists.
Completion releases mutation fences but preserves the operation; the old child-only
custody surface still refuses it, including Ready, so Host cannot omit that record.

The existing unreleased CUJ6 shape now includes installation and before/after
preservation evidence. CUJ5 and earlier released shapes retain explicit migration;
no compatibility lane was added for an intermediate, unshipped CUJ6 shape. The
maximum maintenance receipt is 910 encoded bytes in the envelope probe. Its
scoped budget preserves the 4 KiB frame bound while allowing 256 KiB traversal
allocation, 2,048 nodes and depth 16; child observation budgets are unchanged.
The first budget probe exposed the original allocation limit and was corrected
before installed qualification. An initial installed assertion also used an
invalid publication deadline; its rejection was correct and the fixture now
uses publication's own shorter window. Failed attempts remain recorded in
`store-install-budget-probe.log` and `store-install-installed.log`.

Four installed maintenance cases pass (7.71s), covering callback faults 15–18,
pre-install dispatch rollback at fault 19, Root replacement with uncertain effect
evidence, wrong bytes/callers/malformed requests, expired dispatch, rejected
modules and a replacement lacking the required admission endpoint. The successful
path compares every Store stable byte before/after upgrade, including all payloads,
shared-version pins and pending upload state, then resumes publication and child
admission after explicit activation. A separate expiry case recovers installed
code but cannot issue a new start. Runtime verification itself compares metadata,
not all payload hashes. No general corruption-repair guarantee is claimed.
The focused completion rerun (7.79s) also refuses child-only custody after Ready
and proves that replaying the completed activation cannot reopen a newer Store
gate. The maximum 1 MiB installation frame round-trips through its scoped budget
and remains refused by the unchanged smaller worker-staging budget.

The four existing quiescence/historical-migration cases pass (7.24s), as do the
Wire-budget test, native/Wasm/fixture Clippy, Rustdoc and formatting. Required
`make growth-test` passes both installed cases (14.44s); its optional historical
manager input remains ignored. Normal manager footprint is 1,070,794 raw bytes,
763,202 code bytes and 18 stable pages. Shared journal decoding adds measured
binary cost even when Store-maintenance mounts are absent; no additional storage
allocation, dependency, crate, timer, catalog mirror or generic workflow engine
was added. Logs use `target/qualification/store-install-*.log`.

Remaining limits: one retained maintenance attempt, no reset/cancellation or
successor history, no automatic repair after rejection/expiry, no larger-than-one-
chunk infrastructure delivery, and no Host/CLI/mainnet qualification. The current
Host Store code bindings cannot advance until that operation is integrated into
existing plan/apply/status custody. Package versions and installed CLI remain
0.209.0. This increment is local, with no commit, push or live deployment.

## Closed Store catalog preservation: 2026-11-15

Started pending 0.210.0 after reviewing the 0.209.0 push. Reused the legacy
immutable manifest/chunk and snapshot/reopen research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, plus the checkpoint sequence/late-callback
research recorded below. The changed requirement is an external, bounded
preservation commitment under the rewrite's exact Store closure. Legacy's mutable
catalog replacement and checkpoint orchestrator remain excluded.

Store now returns one ordered release-custody entry under its exact closed
attempt and captured controller. Each page includes invariant quota/counters;
the entry includes aliases, incomplete-upload progress and every shared pin.
Root advances one page per call, retaining only a digest, cursor and count in its
existing journal. Concurrent responses can commit the same cursor only once.
Strict key progression, unchanged counters, the complete release count and an
explicit end page precede stop. There is no payload scan, catalog copy, new
dependency/allocation/timer, installation path or CLI command.

A native migration probe rejected the initial assumption that Serde's default
field could decode the prior Wire record (`Malformed`). Removed that assumption
and added an explicit CUJ5 decoder before writing CUJ6. The retained normal Root
from the 0.209.0 gate is now an installed fixture, with provenance and checksum in
`qualification/tests/fixtures/README.md`. An initial fixture setup attempted an
unsupported downgrade from CUJ6 and correctly trapped; corrected it to install
the predecessor fresh. Real Closed and Stopped CUJ5 attempts now preserve every
authority/stop field, with absent preservation evidence remaining absent. Closed
attempts may scan; already-stopped attempts cannot gain invented evidence.

Focused validation passes: ten Store native cases, both protocol suites and
maximum metadata/status envelopes; four installed quiescence cases (7.19s),
including Root replacement mid-scan and concurrent continuations; and the required
shared-engine growth regression (two passing installed cases, 14.67s; optional
older-manager input ignored). Focused native/Wasm/fixture Clippy, Rustdoc and
formatting pass. Logs use `target/qualification/store-preservation-*.log`.
The growth witness measures the normal manager at 1,045,965 raw bytes, 746,707 code
bytes and 18 stable pages; the shared journal codec has a measurable binary cost
even though that manager does not mount Store-maintenance endpoints.

The full upgrade remains blocked: complete payload preservation, installation,
restart and matching activation still need qualification. Stores lacking the new
page contract cannot complete this scan; no fallback grants replacement authority.
Package versions and installed CLI remain 0.209.0; 0.210.0 is a local pending
increment, with no additional push or mainnet effect.

## 0.209.0 post-push review: 2026-11-15

Verified remote main and peeled `v0.209.0` at
`e3e7be86301c377db8b064d2ef914f5cde936265` (tag object
`af4ff0561ff687bf1fbbbaaa98781ae3613060fb`). `make canic` installed 0.209.0 on
the normal PATH. Reviewed the delivered Store gate, cache fences, Root journal,
explicit endpoint callers, old-custody refusal and qualification boundaries.
No duplicate authority owner, catalog mirror or unqualified live CLI path was
introduced. The intentionally incomplete Stopped endpoint remains an experimental
runtime boundary, not a completed upgrade feature.

The next preservation gap is complete catalog coverage before Store becomes
unavailable: 0.209 reads individual release custody but retains no ordered catalog
commitment. Pending 0.210 addresses that through one bounded entry per
continuation under the exact closed attempt, not another provisioning workflow.

## 0.209.0 release qualification: 2026-11-15

The complete Linux `make -j1 ci` gate passed with unchanged release inputs,
including native/Wasm lint and documentation, effect/custody fixtures, 89 Host
and 44 CLI tests, profile/growth/Root journeys, all 16 installed Store cases,
both CLI publication cases and all five installed Blob composition cases.
Optional historical-manager/Root inputs remain explicitly ignored; macOS and
mainnet were not exercised. Input hashes are retained under
`target/evidence/0.209.0/release/`; the complete output is
`target/qualification/release-209-ci.log`. All 14 product packages select 0.209.0;
qualification packages stay 0.0.0 and crates.io publication remains disabled.

Fresh registry review found the locked first-party selections current:
Host fs/process/artifacts 0.79.0, Host text 0.76.0, Platform 0.33.7,
Platform Runtime 0.22.5, Platform Types 0.20.4, Storage 0.74.1, Timers 0.44.3,
Wire 0.22.4, Testkit 0.106.0 and Toko Tooling 0.44.3. Blob remains at reviewed
Git revision `decde204acb6d7c8362a0003e904beef72ff89b6` (0.100.6); the registry's
latest stable is 0.82.1. Its development branch now points to
`86fc663d14b5fde78db1267157ce9b1b9dbe2888`. Review/adoption of that optional newer
Git input is deferred to the next dependency-maintenance batch; it is not part
of this qualified release. No external dependency selection changed.

## Root-owned Store quiescence: 2026-11-15

Continued pending 0.209.0 with the surviving Root journal and closure/stop proof.
Reused the legacy checkpoint sequence, retained-intent and late-callback research
recorded below. No checkpoint orchestrator or second workflow engine was imported.
Root explicitly mounts stage/advance/observe for one immutable Store approval,
capturing the real operator, old/replacement code commitments, Store attempt and
original dispatch deadline. CUJ5 adds its optional record beside existing child
journals under the same two storage allocations; recognized predecessors load
without inventing Store authority. Downgrade after writing CUJ5 is unsupported.

Staging requires quiescent child work and fences child admission/reconciliation,
publication mutations and raw Store gate changes. Existing complete child-custody
reads refuse the pending record, so Host Root maintenance cannot omit it. Root
verifies old Store code and sole control before closing and stopping. Dispatch
retains intent before enqueueing, with separate callback faults 13/14. Recovery
observes the matching gate or stopped status, including after expiry; it never
issues another stop or renews dispatch authority. Stopped status is the drain
boundary; no exact canister-version increment is assumed.

Focused Linux validation: the metadata-budget test passes; three installed
quiescence cases pass (6.88s), covering actual sole-Root authority, both callback
faults, Root replacement, post-deadline recovery, unfinished-child and wrong-control
refusal. The retained `root-one-child-history.wasm` predecessor migrates without
gaining Store authority. The existing Store-backed creation/upgrade/custody case
passes (9.75s). Required shared-engine `make growth-test` reports two passing
installed cases (14.24s), covering autonomous creation and persistent child
upgrades; its optional older-manager fixture remains explicitly ignored.
Focused protocol/runtime/fixture Clippy, Wasm Rustdoc, formatting, shell syntax
and diff whitespace checks pass. Logs are
`target/qualification/store-quiescence-{native,installed,child-regression,growth,lint,doc,format}.log`.

The new runtime path deliberately ends at Stopped. It cannot install, restart,
activate, cancel or reset; one attempt remains retained. It is not exposed through
CLI and must not be used live. Installation, full Store preservation and matching
activation must extend this same journal before a complete upgrade is admitted.
Host retains replacement bytes; Root retains only their commitment. No new crate,
dependency, storage allocation, timer, catalog mirror or automatic activation was
added. No commit, push, version change, mainnet effect or full repository gate was
performed; package versions remain 0.208.0.

## Store admission survives maintenance: 2026-11-15

Continued the pending 0.209.0 increment with Store-local mutation admission.
Additional read-only legacy research at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` covered
`workflow/application_checkpoint/mod.rs`, `ops/application_checkpoint/mod.rs`
and `ops/application_checkpoint/tests.rs` under `crates/canic-control-plane/src`.
The test `original_unit_storage_replay_and_release_fence_delayed_prepares`
demonstrates exact operation/sequence selection, persistence, idempotent replay,
mutation closure and rejection of delayed acknowledgements. The workflow also
rechecks its authority after await. Reviewed the file history at `d704f4d` and
`aed32be`. Adapted these invariants; did not import participant discovery,
installation registries, checkpoint orchestration or the older Store retirement
workflow (`workflow/wasm_store/mod.rs`), which solves a different lifecycle.

Store persists the latest sequence, operation, captured controller and closed
phase in CWS2 settings under its original allocation. Only the next sequence may
close an open gate; only the exact attempt/owner can activate it. Replaying Close
after activation observes that completed phase rather than closing again.
Unknown/corrupt records fail; recognized CWS1 settings migrate without inventing
a prior attempt. Quota changes preserve the gate. This is a one-way payload codec
transition, not support for downgrading the executable.

All publication/pin/quota mutations check the gate; reads remain available.
Cache work captures an epoch before dispatch and rechecks it after awaits and
before returning readiness. A close/reopen cycle fences old continuations even
when the current phase is open. Already-issued cache uploads can finish; closure
is not a drain certificate. Root's fixed `store_admission` mount reuses current
controller/deadline admission and the bounded metadata transport. Store commits
survive a Root reply trap and are recovered through exact observation/replay.

Focused Linux validation: nine Store native tests and the metadata budget test
pass. Two installed admission cases pass (7.13s): closed admission survives Store
and Root replacement, pins/incomplete uploads persist, wrong-owner and stale
activation fail, and an actual in-flight cache repair is fenced while a fresh
repair after activation succeeds. The existing cold/warm/cache-loss installation
and child upgrade witness also passes (6.60s), using hash-verified unchanged
greeting artifacts. Focused native/Wasm and fixture Clippy, Wasm Rustdoc, formatting
and shell syntax pass. A redundant clone in a test was removed during linting.
Logs are `target/qualification/store-admission-{native,installed,cache-regression,lint,doc}.log`.

The installed fixture deliberately retains direct operator control to upgrade
Store; it does not impersonate Root or qualify the Root-only installation path.
CWS1 migration is covered by its actual record codec, not a historical installed
binary. No general workflow engine, dependency, crate, memory allocation, timer,
CLI alias or automatic activation was added. Host-driven Store upgrades remain
blocked pending the external Root effect journal, quiescence and recovery proof.
No push, release, mainnet mutation or full repository gate was performed.

## Bounded Store preservation observation: 2026-11-15

Started the pending 0.209.0 increment after the 0.208.0 post-push review. Reused
the legacy manifest/chunk storage, snapshot/reopen regressions and history at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded in the
[Store research](#versioned-wasm-store-decision-2026-11-15). Stable bytes, exact
release identity and retained obligations remain authoritative. Legacy's mutable
catalog replacement is still excluded, and its maintenance mechanics do not
establish the rewrite's preservation guarantee.

Added a replicated Store observation of one release's progress and all pins on
its shared artifact. Compact owner/operation/release records avoid repeating a
full manifest per pin; the maximum 32 pins and 32-chunk manifest fit the existing
metadata budget. Store reads its existing stable records without changing their
codecs or leases. Root explicitly mounts a typed controller/deadline read using
the same bounded transport as readiness. No caller can supply a claimed pin owner,
and observation never creates a pin or retains a second catalog in Root.

The focused installed witness passes (7.02s): two real controllers pin shared
bytes under different release labels, both pins and an incomplete upload survive
a Store executable change, and actual Root calls read them after sole-control
handover. Malformed, unauthorized, expired and missing-release requests, reply
fault 12 and Root replacement preserve both owners' stable bytes. Store's upgrade
is performed by its test controller before handover, not by a claimed Host flow.
The two existing readiness regressions also pass (6.98s) after sharing transport.
All seven Store native cases and the maximum Wire budget test pass, as do focused
native/Wasm Clippy, fixture Clippy, Wasm Rustdoc, formatting and shell syntax.
Logs are `target/qualification/store-custody-{native,installed,readiness,lint,doc}.log`.

Added one focused runner selection; the readiness selection continues to run only
its own cases. No crate, dependency, persisted schema, CLI command, lifecycle hook,
timer, installation path or catalog mirror was added. These separate reads are
not an atomic catalog snapshot or a payload integrity scan. Store maintenance
still needs qualified quiescence, an external effect journal and same-operation
recovery before Host can admit it. No new release or mainnet mutation was made;
package versions and the installed CLI remain 0.208.0.

## 0.208.0 post-push review: 2026-11-15

Verified remote main and the peeled `v0.208.0` tag at
`cc37504fdbb2219cdc325a6e80ddfb1a97eb1ac4`. Reviewed the delivered fleet release
composition, convergence chain, publication reuse, Root custody checks, record
codecs and CLI callers against the installed witness and design. No actionable
release discrepancy was found in that scope. Publication and installation keep
separate approvals, completed historical publication cannot renew mutation
authority, and old inline encodings remain covered. No new runtime owner or
dependency was added. Two private Host modules group existing responsibilities;
they do not introduce parallel workflows.

Store's own upgrade remains explicitly blocked in convergence. Its current
observations expose release progress or a known caller/operation pin, but cannot
enumerate every retention obligation for a release's shared artifact. The next
increment starts with this missing bounded preservation observation, reusing
Store's existing records and Root's typed controller/deadline admission. It must
not claim an atomic fleet snapshot or a completed Store upgrade workflow.

## 0.208.0 release qualification: 2026-11-15

The complete Linux `make -j1 ci` gate passed in 271 seconds with unchanged source
inputs. This covers native/Wasm lint and documentation, effect and artifact custody,
89 Host and 44 CLI tests, installed application/Root/Store journeys, both publication
journeys including four serial child upgrades across Root maintenance, and all five
Blob composition cases. Historical-binary opt-in cases and macOS/mainnet were not
run. Raw gate and input evidence is under `target/evidence/0.208.0/release/`.

Fresh sparse-registry review of direct first-party dependencies in both workspaces
found the selected stable versions current. Blob remains at its qualified immutable
Git revision. Release preparation changes all 14 product versions to 0.208.0 and
their local lockfile entries; the four qualification package versions stay 0.0.0.
External selections are unchanged and every package retains `publish = false`.
Version-sensitive Host/CLI checks and `make canic` verify the prepared release.

## Host Root maintenance preserves Store custody: 2026-11-15

Extended the existing whole-fleet maintenance owner to carry complete Store
delivery evidence alongside child journals. Reused the legacy source/test/history
research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded in the
[Store decision](#versioned-wasm-store-decision-2026-11-15) and
[runtime integration](#root-uses-retained-store-references-2026-11-15). The relevant
requirements remain immutable source identity, preserved pins and history, and a
reconstructible management cache. The rewrite's installed full-custody contract,
not an assumed legacy upgrade guarantee, supplies the maintenance mechanism.

Maintenance plans bind the Store identity/code and both slots' exact creation and
historical upgrade deliveries, including pin state, operation identity and cache
deadlines. Existing evidence readers now return their already-validated delivery
alongside status; there is no second journal scan or status summary parser. Root
installation still belongs to the direct effect owner. Preflight and postflight
select only `observe_stored_children`, compare the complete record including empty
slots, and verify child and Store code/controllers plus Store subnet placement.
The optional boxed custody field preserves inline plan encodings while avoiding
growth of every PlanAction value. Unknown legacy custody contracts are not probed.

Completed release publication remains valid artifact evidence after a reviewed
Root code change. Fleet convergence reuses the existing inventory and signed
completion receipt; it does not grant new publication authority or renew an
unfinished old-code operation. The installed witness returns to an earlier release
after maintenance and refuses that selection when the original completion proof
is missing. Subsequent installation still binds current Root code and exact Store
source. Root maintenance recovery continues through the original `fleet apply`;
the child reconciliation command is not extended into a second maintenance path.

Focused Linux evidence:

- 89 Host and 44 CLI tests pass; the ten convergence cases were rerun after
  completing historical-release reuse. Native maintenance cases reject changed
  pins, source identities, cache deadlines, unexpected slots, incomplete retention
  and legacy inline observation methods. Logs are
  `target/qualification/store-maintenance-{native,convergence}.log`.
- Both installed publication journeys pass on the maintenance implementation
  (90.44s). After the final reuse change, the whole-fleet witness passes again
  (100.22s): two applications above 2 MB, four child upgrades across both slots,
  Root replacement between upgrades, changed Store authority refused before
  installation, interrupted postflight, lost install reply, unchanged signed
  attempt, identical complete before/after custody, repeat completion and an old
  release reused under the new Root code. Logs are
  `target/qualification/store-maintenance-{installed,reuse}.log`; final fleet
  custody is under `target/publication/fleet-324467/`.
- Focused Host/CLI and publication-fixture Clippy, Host/CLI Rustdoc, formatting
  and tooling agreement pass. `make canic` installs the development CLI on PATH.
  No full release gate, macOS or mainnet result is claimed.

Removed stale inline-only/maintenance-pending guidance and consolidated duplicated
whole-fleet instructions. Kept source validation and live Store verification with
their existing owners; added no crate, dependency, runtime endpoint, storage
allocation, timer, signing path or workflow engine. Current guarantees cover
completed child work and an exclusive operator maintenance window, not active or
uncertain effects. Host-driven Store upgrades, inline migration, pin reclamation
and application-specific data migration remain separate qualification work.
The pending 0.208.0 notes now include the optional Store custody field on
`PlanAction::MaintainRoot`; inline callers supply `None`. Package versions remain
0.207.0. No push or mainnet mutation was performed.

## Whole-fleet Store artifacts and serial upgrades: 2026-11-15

Continued after the maintainer confirmed 0.207.0 delivery. Fixed the review's
Slot-index cast and misleading Store-bootstrap diagnostic. Reused the source,
regression and history research at legacy revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` in the
[Store decision](#versioned-wasm-store-decision-2026-11-15) and
[Root integration](#root-uses-retained-store-references-2026-11-15): immutable
release identity, stable bytes as authority, reusable chunks and explicit
bootstrap readiness remain the relevant requirements. No legacy mutable catalog
selection, old-version pruning or new workflow engine is needed.

Fresh whole fleets with a declared Store now bind each child's manifest into
the aggregate approval. Release identity is `(application canister name, complete
content hash)`. Root/Store principals resolve only from their approved creation
results and typed completion evidence. Nested publication reuses the existing
Plan, signer, observation and retention owners. Both publications must complete
before any child admission. Publication progress is visible in fleet status;
reconciliation can observe it but cannot publish or admit a child.

For each changed child, convergence returns an independently reviewable publication
plan first. Once its signed completion evidence is retained, replan creates the
existing managed upgrade plan with that exact Store/manifest. The ordered child,
code and slot chain keeps its eight-upgrade bound and source owner. Root holds
references and pins, not application bytes. Existing inline fleets, including
those with a bootstrapped Store, retain their original delivery mode and digests.
Host Root maintenance explicitly refuses Store-backed children until its full
custody-preservation flow is qualified; no inline-only maintenance record may
silently replace Store evidence.

Focused Linux evidence:

- All 88 Host and 44 CLI tests pass, including maximum-size planning with two
  8 MiB manifests, exact future Store binding, original one/two-child digests and
  retained Store-plus-inline replanning. Native output is at
  `target/qualification/store-fleet-native.log`.
- The installed whole-fleet publication fixture passes in 80.40 seconds with
  two application Wasms above 2 MB. It interrupts Store handover and second-release
  completion, loses publication and child-admission replies, preserves original
  signatures, and proves neither child is admitted before both publications.
  The second release reuses the first object's bytes without chunk upload.
  Three upgrades cover app, worker, then app again, clearing the management cache
  before each. Publication and installation are independently approved; replay,
  unchanged convergence, four-canister inventory, missing readiness evidence and
  the Root-maintenance refusal are checked. Retained fixture data is under
  `target/publication/fleet-244972/`.
- The existing named publication/deployment/first-upgrade fixture also passes
  against the unchanged product inputs. The initial extended fleet run exposed
  accumulated simulator clock skew beyond the real signer's five-minute window;
  the fixture now uses simulator-relative short schedules like the Root suite.
  Production deadlines were not relaxed. A native maximum-manifest case identified
  Wire's per-value node charge; only the bounded metadata node allowance increased.
- Focused Host/CLI and publication-fixture Clippy, Host/CLI Rustdoc, `make fmt-check`
  and `make toko-tooling-check` pass. Rustdoc output is at
  `target/qualification/store-fleet-doc.log`. No full release gate, macOS or
  mainnet run is claimed.

Pending notes select 0.208.0 because Host adds a public fleet-child field and a
planning result variant, and fresh Store fleets now use publication before
installation. Package versions remain 0.207.0 until authorized release preparation.
This batch adds one private fleet-publication composition module, no crate,
dependency, alias, configuration switch, mutable latest resolver, runtime engine
or catalog mirror. Inline migration, Host Root/Store maintenance and pin reclamation
remain explicit limitations. No push or mainnet mutation was performed.

## 0.207.0 post-push review: 2026-11-15

Reviewed delivered commit `e908fe97e0b470c395656b56930cb6d58a8d5f3e` after
verifying remote main and the peeled `v0.207.0` tag. The review covered release
selection, managed staging/observation/reconciliation, retained approval matching,
typed Store receipts and direct CLI callers against the design and installed
witness. Source custody stays with the existing owners; there is no new runtime
engine, payload repository in Root, mutable latest resolution or signing path.
Old inline digests remain qualified. Whole-fleet release selection and custody
migration are documented limitations, not implicit supported flows.

No release-blocking discrepancy was found in this scope. Two minor cleanup items
remain: `managed/stored.rs::upgraded` casts Slot to an index instead of using the
existing `Slot::index()` owner, and the Store-bootstrap refusal in CLI `run_fleet`
mentions scheduling even when the rejected input is `--release`. Use the existing
index method and make that diagnostic identify the unsupported application-release
selection. Neither changes effect authority; no new framework or broad audit is
needed. This post-push note remains local and is not part of the release tag.

## 0.207.0 release qualification: 2026-11-15

The complete Linux `make -j1 ci` gate passed on the release inputs in 242 seconds.
It includes native/Wasm lint and documentation, effects and maintenance, artifact
custody, 87 Host tests, 44 CLI tests, installed deployment/profile/growth,
11 Root cases, endpoint admission, 10 Store cases, two publication/deployment
journeys and five Blob cases. Three historical-binary migration cases remain
explicitly opt-in and were not run. No macOS or live mainnet result is claimed.

All 14 product packages select 0.207.0; the four qualification packages stay at
0.0.0. All retain `publish = false`. Fresh registry review covered every direct
first-party registry supplier in both workspaces and found the locked selections
current, including the incoming Wire 0.22.4 update. Blob remains at the qualified
immutable revision `decde204acb6d7c8362a0003e904beef72ff89b6`.
Release logs, metadata, supplier review and source fingerprints are retained
locally under `target/evidence/0.207.0/release/`. `make canic` installs 0.207.0 on
the normal PATH. Delivery is GitHub-only, with no registry publication or mainnet
mutation.

## Named Store release deployment: 2026-11-15

Continued from the 0.206.0 review with the two scoped cleanups: convergence now
uses the effect owner's typed receipt validation, and readiness/status share the
Store observation budget. Reused the Store source/test/history research at legacy
revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` recorded in
[the Store decision](#versioned-wasm-store-decision-2026-11-15) and
[Root integration](#root-uses-retained-store-references-2026-11-15). The same
question applies: retain exact versioned content while treating management chunks
as a reconstructible cache. No legacy catalog mirror, pruning policy, workflow
engine or mutable latest alias was imported.

Named `fleet plan --release PUBLICATION_OPERATION` now resolves completed signed
publication custody in the same fleet. The existing child owner carries the exact
manifest and Store code, with omitted source fields preserving old inline digests.
Host independently checks Root/Store authority, code and subnet placement before
unfinished delivery. Existing Store endpoints own staging, pins, creation,
installation, readiness and upgrade; Host adds no runtime state or effect owner.
Complete replicated custody reads check the selected slot, release and pin identity.
Recovery and signing use the existing operation journal, deadlines and budgets.
Changing an admitted child between inline and Store custody is refused.

The installed CLI/signer witness uses a real Wasm above the inline transport limit,
refuses changed Store control before signing, loses the creation admission reply,
recovers through Store custody, installs, and upgrades the same child from a new
version after clearing the derived cache. It calls the upgraded greeting endpoint
and checks replay does not create additional canisters. The first run exposed
simulator/host clock skew beyond ingress validity; explicit shorter fixture
schedules preserve the production ten-minute default and five-minute ingress
checks. A later assertion was corrected to the application's actual greeting.

Focused evidence: 87 Host native cases, 44 CLI cases (three loopback cases rerun
with the required sandbox permission), Host/CLI Clippy and Rustdoc, and publication
harness Clippy. Both installed publication journeys pass (28.55s), including
whole-fleet admission and missing-Store-proof convergence refusal. Formatting
and Tooling snapshot/dependency agreement pass. Logs:
`target/qualification/store-deploy-*.log`. Both incoming lockfiles already selected
Wire 0.22.4; this work preserves that pending update and qualifies against it.
The supplier checkouts and pins are unchanged.

This is named first-child creation and its first upgrade. Whole-fleet release
selection, inline migration and host maintenance carrying full Store custody are
still separate qualification work; no generic scheduler, new crate, publication
shortcut, mainnet action or performance claim was added. The pending changelog is
0.207.0 because Host's public managed-plan argument and owner construction change;
package versions and delivery remain untouched.

## 0.206.0 post-push review: 2026-11-15

Reviewed delivered commit `df9321fc17c99c93e07d631173200a99c2a890bc`
after verifying remote main and the peeled `v0.206.0` tag. Focused on Store
bootstrap, its fleet/convergence and status callers, typed signing, endpoint
authority and the accompanying design/CLI claims. Store stays optional and
outside application slots. Existing direct effect custody is reused; no Root
payload/catalog mirror, alternate signer or generic forwarding endpoint was
introduced. Retained no-Store digests and installed interruption witnesses
support the compatibility/recovery claims. No release-blocking discrepancy was
identified in this scoped review.

Two small cleanup items remain for the next implementation pass: status repeats
the Store observation limit (`32`) instead of obtaining it from its owner, and
fleet convergence reads Store completion through the status JSON projection.
Give the former one Store-owned constant and the latter a typed retained receipt
query; keep the existing evidence validation and independent publication budget.
These are maintenance risks, not observed qualification failures. Store release
selection for app deployment and Store adoption/maintenance remain explicitly
unqualified product work, rather than hidden compatibility paths. This review
note is retained locally after the push; it is not part of the release tag.

## 0.206.0 release qualification: 2026-11-15

The Linux release scope passes across the aggregate gate and its resumed stages:
lint (including optional Blob), native libraries, effects, artifact custody,
Host/CLI, installed deployment, profile, growth, Root, endpoints, Store,
publication and Blob. This includes 85 Host tests, 43 CLI tests, 11 Root cases,
10 Store cases, two publication journeys and five Blob cases. Three historical
binary migrations remain explicitly opt-in; they were not run. The gate caught
two stale test expectations: an unconditional refusal of Store in whole-fleet
plans and the old pending-admission label. Corrected those tests, reran their
affected journeys and Clippy, and retained successful evidence for unchanged
inputs. No production changes were needed to repair the release gate.

All 14 product packages select 0.206.0; four qualification packages remain 0.0.0.
Every package retains `publish = false`. A fresh registry review found the
selected first-party registry suppliers current. Blob remains pinned to the
qualified 0.100.6 source revision
`decde204acb6d7c8362a0003e904beef72ff89b6`; no sibling checkout was changed.
Release logs, dependency metadata and source fingerprints are retained locally
under `target/evidence/0.206.0/release/`. Delivery is GitHub-only; this evidence
does not claim macOS or live mainnet qualification, or registry publication.

## Whole-fleet Store bootstrap: 2026-11-15

The existing fleet envelope now admits one declared Store alongside Root and one
or two application children. Reuse the legacy bootstrap-ordering research below
and the qualified named Store action: infrastructure readiness precedes dependent
application admission, while original output references and effect custody stay
with their existing owners. No legacy scheduler, catalog mirror or implicit
service activation was imported.

Store binds Root's approved creation output, its own exact executable and a
separate 1T operator debit. Whole-fleet `--cycles` is the total, divided into that
Store debit and Root funding (which already covers child grants). Defaults are
4T for one app plus Store and 5T for two. Store's nested journal uses the existing
direct creation/install/handover/readiness path. Both application slots remain
available. A lost handover reply is recoverable before child admission;
reconcile never admits those children, and apply retains their original schedule.
Cancellation, inventory, status and convergence all include Store. Existing
one/two-child approval fixtures retain their exact digests and output layouts
when Store is absent.

Nine focused fleet tests and eight CLI status tests pass. The installed
`make publication-test` runner passes both the named publication journey and a
four-canister fleet witness (18.67s together): pause after Store handover, prove
neither child was admitted, recover the removed local handover reply, then admit
and complete both children. Root's stable state remains unchanged until application
admission. Replay creates nothing extra, four-canister inventory has no issues,
no-change uses retained completion, and missing Store readiness proof refuses
convergence. The first native run caught a changed no-Store reconciliation error;
restoring the original admission ordering resolved it. Host/CLI and installed
harness Clippy, Rustdoc and the retained-digest regressions pass. Logs:
`target/qualification/fleet-store-*.log`.

The simplification is composition over the existing owners, with one explicit
infrastructure field rather than another application slot or lifecycle engine.
Store remains optional and the generated starter remains minimal. Store artifact
replacement and addition/removal on retained fleets are deliberately refused;
adoption and maintenance need their own qualification. Application deployment
still uses inline payloads; selecting a published Store release is the next
integration gap. No mainnet action or push accompanies this work.

## Named Store bootstrap through the CLI: 2026-11-15

Continued the existing 0.206.0 work using the legacy bootstrap ordering and
failure research recorded immediately below. The enduring requirements are exact
creation output custody, explicit controller handover and initialized-state
evidence before completion. Reuse the current direct creation/install signer and
journal, rather than copying legacy's bootstrap scheduler or fitting Store into
Root's greeting-child slots. No supplier or legacy checkout was changed.

A named `wasm-store` plan with `--root-module` now selects a distinct semantic
action under an already-bound Root. It binds Store bytes, Root identity/code,
operator, budget and same-Root-subnet placement. Before creation, retain Root's
certified subnet; select it in the cycles-ledger request or route local creation
through Root. Recheck placement before installation/handover and on completed
replay. A changed subnet blocks instead of rebinding an existing attempt.
Root controller/code checks precede effects. Operator-controlled installation
uses the original direct owner; sole-controller handover retains its exact
signed management call. After handover preparation, recovery validates original
creation/install custody without dispatching either effect again. A lost handover
reply can be reconstructed from certified sole Root control, exact code and a
retained signed replicated readiness response. Reconcile grants read authority
only; expired uncertain mutations are never re-signed.

Retained deployment digests stay unchanged: the record codec adds an omitted
optional Store binding, while semantic authority stays in `PlanAction`. Store
does not consume an application slot. Publication and Store readiness share
the existing narrowly typed fresh-read signing mechanism; no arbitrary call
forwarding, second signer, runtime catalog, payload mirror or new crate was added.

Focused evidence: two native bootstrap admission/reopen cases, the retained
approval digest case, nine signed-authority cases and four direct provider/plan
cases pass. The installed CLI witness (17.75s) creates Store on Root's subnet,
refuses changed Root control before creation, interrupts readiness after handover,
removes the local handover reply, reconciles and replays without new canisters
or changed effect signatures, refuses missing readiness proof, then publishes
multiple versions and resumes an interrupted upload. Root stable bytes remain
unchanged. Host/CLI and harness Clippy and Rustdoc pass. Logs are under
`target/qualification/store-bootstrap-*.log`.
The focused installed direct-deployment regression also passes (9.62s), including
receipt recovery, replay and upgrade of the same canister through the shared
installation path. Formatting and Tooling snapshot/dependency agreement pass.
`make canic` installs the updated 0.206.0 CLI on PATH.

The installed evidence uses synthetic local cycles. Mainnet validation here
covers the typed cycles-ledger request with an explicit subnet; no live mainnet
Store was created. Whole-fleet Store composition, release-selected deployment,
Store maintenance through Root and general placement remain pending. Unknown
creation/install results and exhausted/expired unresolved observation attempts
retain their bounded blocked outcomes. No push accompanies this increment.

## Store readiness after Root handover: 2026-11-15

Continued 0.206.0 with one prerequisite to retained Store bootstrap. Re-read
legacy's [Root bootstrap](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/bootstrap/root/mod.rs)
at the pinned research revision, alongside the previously recorded Store tests
and bootstrap diagnostic history below. Preserve its ordering of authority,
required-component readiness and activation. Do not import its catalog mirroring,
pruning or implicit reconciliation into Root. The current bounded Store already
owns the source of truth; Root's application slots have greeting semantics and
cannot stand in for infrastructure readiness.

Store now exposes replicated `observe_info`; Root explicitly mounts
`observe_store` with controller/target/deadline admission shared with publication.
The new method has its own Wire identity, leaving existing publication encoding
unchanged. It issues one bounded typed call with no cycles, timers, durable Root
state or app slot. Its report is not a code/controller certificate, deployment
receipt or capacity reservation. Automatic Host creation/handover must retain its
own operation and bind certified module/controller evidence before completion;
existing Store bootstrap refusals remain in place.

Focused validation passed: one native readiness-frame case and two installed
readiness cases (6.92s), then the existing publication-frame case and both
installed publication regressions (7.63s). Actual Root calls fail before handover
and succeed afterward, including repeated reads, reply fault 12 and Root
maintenance. Both empty application slots and both owners' stable bytes remain
unchanged. Unauthorized, malformed, expired, invalid-target and unavailable-target
requests cannot establish readiness. Publication still resumes after maintenance
and recovers retained Store commits after lost Root replies. Root/Store native
and Wasm Clippy, installed-harness Clippy and Rustdoc passed. Logs:
`target/qualification/store-readiness-{tests,publication-regression,lint,doc}.log`.

The existing Store runner now selects this focused scope without building unused
application artifacts; the aggregate still includes every Store case. No parallel
bootstrap engine, catalog, payload owner or new package was added. No mainnet
operation or push accompanied this increment.

## Post-push review: 0.205.1

Reviewed delivered commit `0086d49e13ef7e8c8e74a9c2f829707e3869e338` and its
direct Host/CLI, Root/Store, Blob and validation callers after the atomic main/tag
push. Remote `main` and peeled `v0.205.1` both resolve to that commit. The
normal pre-commit hook passed with the immutable Blob input; no override remains
necessary. Publication stays out of deployment reservations and canister inventory;
Root retains no artifact catalog or payload, and receipts bind signed observations.

Two actionable follow-ups begin 0.206.0:

- `crates/canic-host/src/artifact.rs` reused direct ingress size admission for
  build/inspection, requiring external builds for otherwise accepted Store inputs.
  Share Store’s bounded artifact capacity across build, inspection and publication;
  preserve direct/inline deployment admission at their existing owners. The
  earlier Store/legacy research covers this same custody/transport distinction.
- `Makefile` omitted the optional Blob feature from `lint` and its installed
  witness from `ci`. Release 0.205.1 explicitly ran these existing checks; include
  them in the normal aggregates once each, without a second validation runner.

Generated Store bootstrap, deployment release selection and Blob management
lifecycle integration remain explicit unqualified paths, not hidden fallbacks.
No broader redesign or additional push follows from this review.

## 0.206.0 first increment: artifact capacity

The public builder and inspector now use the same 8 MiB capacity owner as Store
publication. Direct ingress planning still uses 1.5 MB and inline children keep
their runtime custody bounds. No new transport, planner, adapter or configuration
mode was added. Aggregate lint/CI now schedule the existing Blob checks.

Eight focused Host tests passed, covering artifact capacity/refusal, publication
admission and original direct-plan digests. The installed CLI witness inspected
and published a valid Wasm over 2 MB, resumed interrupted upload, retained signed
evidence and reused the same bytes under another version (14.94s). Host/CLI and
the publication harness passed Clippy. An admitted Make preview confirms one
Blob feature-lint invocation and one Blob installed-test invocation in CI; the
full repository gate was not rerun for this development increment. Logs:
`target/qualification/206-{tests,lint,publication}.log` and
`target/qualification/206-ci-roster.txt`. This work remains local after the 0.205.1 release.

## 0.205.1 release qualification: 2026-11-15

Fourteen product packages move to 0.205.1; the four qualification packages remain
0.0.0. This GitHub release adds Root-mediated Store publication, its reviewed
fleet CLI workflow and receipt custody, plus Tooling 0.44.3. Publication to
crates.io stays disabled. No mainnet state or saved deployment approval changes.

The local Blob library's runtime implementation is now present unchanged in
its pushed 0.100.6 commit `decde204acb6d7c8362a0003e904beef72ff89b6`. Both
Canic graphs select that exact Git source, with authenticated access required for
initial preparation. The external path exception is retired. The input recorder
uses locked Cargo metadata instead of a mutable sibling path and still compares
actual source files and consumer locks before/after the installed witness. The
supplier checkout was inspected read-only; no supplier changes were made.

The full Linux gate passed: `make lint blob-clippy blob-doc`, then the native
and installed test leaves `test effect-test artifact-custody-test journey-test
profile-test growth-test root-test endpoint-test store-test publication-test
blob-test`. This includes 79 Host tests, 43 CLI tests, eleven installed Root
cases, eight Store cases, the actual publication CLI witness and five Blob cases
against the immutable supplier. Three historical-binary scenarios remain
explicit opt-ins and were skipped. The selected root `ic-metrics 0.30.8` input
is covered by this release's affected native/Wasm builds and installed paths.

Registry search and cached index admission confirm the selected first-party
registry suppliers match current stable, non-yanked releases. The old crates.io
Blob product is intentionally not substituted for the committed rewrite. No
unrelated dependency refresh was performed. Formatting, snapshot agreement,
dependency inventory and documentation links pass. Source/lock hashes and full
logs are under `target/evidence/0.205.1/release/`; Blob's before/after receipts
match under `target/blob/`. No macOS, live provider or new mainnet qualification
is claimed. The installed CLI reports 0.205.1.

## Toko Tooling 0.44.3 adoption: 2026-11-15

Selected published 0.44.3 in both workspace catalogs and locks. The authenticated
preview/update retained all 47 managed files, changing only the baseline and
two LOC reporter files. The archive checksum is
`8a9bc7b6827044baac8134cada7ee6bc31f86c70beec9608d1d39d911eba15e1`.
The shared baseline now owns post-push review; AGENTS keeps only Canic's domain
model and recovery concerns instead of repeating the shared policy.

Locked metadata passed for both workspaces, as did tool admission, snapshot
agreement, formatting and diff checks. The installed PATH launcher produced
identical default and `-d 0` output; `-d 1` expanded actual Canic crate rows.
No producer regression matrix or product simulator suite was rerun for this
tooling-only change. Evidence: `target/qualification/tooling-0.44.3/`.

The root lock's `ic-metrics` selection separately changed from 0.30.7 to 0.30.8
after the narrow Tooling update was inspected. That unrelated change was
preserved; this adoption does not claim runtime qualification for it.
No supplier source, application implementation, release version or delivery was
changed by the Tooling adoption.

## Fleet CLI artifact publication: 2026-11-15

The existing Host plan/apply/status/reconcile owners now publish one explicit
release into an already-bound Root-owned Store. V3 approval binds exact artifact
bytes, label, network, operator and both canister/code identities. The journal
lock, signer admission, retained attempts and immutable file owner are reused.
V1/v2 deployment digest fixtures remain unchanged. Publication does not reserve
an application slot or appear as an installed canister in fleet inventory.

The [preceding publication research](#publication-through-root-after-store-handover-2026-11-15)
at legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` covers this same
question: preserve exact release identity, bounded transfer and observable
progress, without the legacy Root catalog/cache workflow. The current qualified
Host signer already owns durable request custody, so no legacy transport or
second journal/executor was added. Store remains the only durable catalog/byte
owner; Root performs one typed call at a time.

Every mutation retains its original dispatch deadline and signature. Fresh
recovery authority can only observe that exact release, including after effect
approval expiry. Exact observation can establish a lost mutation's outcome,
but cannot authorize remaining expired work. A completion summary alone is
insufficient: its retained observation index, signed request, response and exact
manifest must agree. Thirty-two observations bound this initial recovery history;
exhaustion blocks without discarding evidence.

Focused validation passed: seventeen native cases across publication admission,
retained operation encoding, signer custody and CLI argument conflicts; the
installed `make publication-test` witness passed in 12.79s. The witness uses the
actual CLI, pinned ICP signer, Root and Store with synthetic cycles. It interrupts
before the first chunk signature, reconciles without mutation, resumes without
replacing Prepare's signature, publishes two versions with warm byte reuse and
proves unchanged Root stable memory. It also refuses wrong certified code and
missing signed receipt proof, checks human status, and keeps publications out of
canister inventory. Host/CLI and the new harness passed focused Clippy; Host
Rustdoc passed. Logs are `target/qualification/publication-{host-tests,host-clippy,host-doc,installed}.log`.

Review removed misleading deployment-receipt text from publication status and
kept inventory sampling under one lock. It also found that the old greeting-child
bootstrap could admit a declared Store as an application. Whole-fleet and named
child planning now refuse that unsupported role before journal creation; the
focused regression proves both refusals. A separate Store bootstrap/readiness
contract and deployment release selection are the next coherent integration,
not another workflow engine. Current publication requires existing bound owners;
Store maintenance through Root and existing-fleet migration remain pending.
Artifact publication admits 8 MiB; the current public builder/inspector still
admits 1.5 MB, so larger inputs need an external build. No mainnet deployment,
measured deployment speedup, browser launch, version bump or push is claimed.

## Publication through Root after Store handover: 2026-11-15

The next Host integration prerequisite is complete: the external operator can
publish after Root becomes the Store's sole controller. One explicitly mounted
`store_publication` endpoint admits a fixed prepare/put/publish/observe command,
canonical Store target and five-minute dispatch window. It performs one bounded
call, with no retry, attached cycles, catalog mirror or durable payload at Root.
Store remains the immutable release/capacity owner; publication creates no child
approval, pin or installation. CLI integration still needs retained host authority,
selected-release policy and reviewed Store bootstrap in existing fleet plan/apply.

Revisited legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` publication
`release/catalog.rs`, `release/chunks.rs` and template admission tests, using the
links in the [original Store research](#versioned-wasm-store-decision-2026-11-15).
The same bootstrap diagnostic history `ed9f133` remains relevant. Retain exact
release identity, bounded transfers and observable progress. Omit the legacy
Root catalog mirror, approved-version replacement/pruning, cache side effects
and broad publication workflow: Store already owns these immutable outcomes.
The installed regressions preserve conflict/hash/size admission at that owner.

`make store-test` passed twelve native tests and eight installed cases (16.98s).
New cases resume an incomplete multi-chunk upload across changed Root executables,
retain earlier versions and prove Root stable bytes do not change. They refuse
direct operator access to the Root-owned Store, unauthorized Root calls, malformed
frames, invalid targets/windows and expired dispatch. Fault point 12 makes the
real Root reply encoder fail after prepare, put and publish have committed at
Store. Observation and exact replay recover each result without extra custody;
no cross-canister rollback or automatic retry is claimed.

`make root-clippy root-doc` and the qualification Store test's focused Clippy
passed. Current logs: `target/qualification/store-publication-{tests,lints,fixture-lints}.log`.
The public artifact builder admitted the new Root within the existing Host size
limit. No supplier, persistent record schema, old endpoint byte contract or
fleet CLI default changed; no mainnet, browser or push was performed.

## Tooling, Platform and Wire adoption: 2026-11-15

Selected published Toko Tooling 0.44.0 in both workspaces and installed its
checksum-verified snapshot through preview/update. All 47 retained managed files
were admitted; only the baseline text changed. The retired Python verifier has
no Canic caller. Existing package lint inheritance and Rust 1.100.0 stay intact.
Prepared tools, snapshot agreement, formatting and dependency-list wiring pass.

Registry inspection confirms Platform 0.33.7, Runtime/macros/guard 0.22.5,
Types 0.20.4 and Wire 0.22.3. Both incoming lockfiles already selected these;
this adoption changes only Tooling's locked package. Compatible manifest minima
remain supported rather than raising them for unused additive APIs. No supplier
checkout, application authority, request bytes or protocol layout was edited.
No broad dependency refresh, version bump, publication or deployment occurred.

Full locked/offline metadata admits both independent workspaces. The selected
native Host and installed catalog graphs each retain one canonical Types/Wire
identity; the catalog shares one Runtime/macros/guard and Storage identity.
Default Canic remains codec-only. Focused journey/Blob Clippy and Rustdoc pass,
as do seven Host signed-attempt admission/recovery tests and all five installed
Blob cases. This refreshes the earlier Wire 0.22.2 witness for 0.22.3; current
hashes are in [the Blob guide](../demos/blob.md). Logs and graph observations are
under `target/qualification/adoption/`. Full CI and mainnet were not run.

Before the requested GitHub push, moved Store's directory-owning canister module
to `canister/mod.rs` to finish the ordinary Rust module-layout migration. Focused
Store Clippy/Rustdoc, eleven native tests, six installed Store/Root tests (17.04s)
and formatting/snapshot admission passed on the adopted graph. Logs are
`target/qualification/adoption/store-push-{checks,tests}.log`. This is a source
push of the pending increment, not a tagged release; the local Blob input remains
explicitly development-only.

## Optional embedded Blob composition: 2026-11-15

Completed the optional adapter and real installed catalog witness described in
[Blob composition](../demos/blob.md), which owns current scope, source identities
and focused evidence. No supplier checkout changes, dependency refresh, provider
calls, mainnet deployment or push occurred. The local uncommitted Blob library
is explicitly selected and fingerprinted, not represented as published input.

Legacy research inspected the fixed revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`:
[endpoint composition](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-blob-service/src/endpoints/mod.rs),
[lifecycle](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-blob-service/src/lifecycle/mod.rs),
[embedded application storage](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/qualification/blob-service/embedded-consumer/src/app/mod.rs),
[installed refusal regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/qualification/canic-tests/src/pic/blob_service/journey/refusals/mod.rs)
and [adapter release history](https://github.com/dragginzgame/canic-legacy/commit/abfb30a51dff117375e7afdac04d5b25b58d39b0).
Keep explicit lifecycle, host-granted memory, distinct application storage,
authenticated caller boundaries and trapping failures. Replace manual raw dispatch
and broad endpoint rosters with three Platform/Wire mounts around the rewrite's
one real Service. Omit legacy service crates, funding, provider callbacks,
metrics samplers and recovery machinery: none serves this local consumer.

Canic gains one optional module rather than another adapter crate or universal
Core. Application state stores publication decisions and original authorizations,
not a second reference/accounting ledger or admission flag. The installed tests
expose the present Host/Root lifecycle incompatibility instead of weakening Blob's
exact drain/activation contract. The independent supplier has no Canic dependency.

Alongside the integration, removed the Store test's `#[path]` override and
same-name file/directory layout. Its child module now follows ordinary Rust
discovery under `store_root_tests/mod.rs`; the installed Store test binary still
compiles. Prior Store runtime evidence is retained; this relocation changes no
runtime behavior and did not justify replaying its simulator suite.

## Root uses retained Store references: 2026-11-15

The pending 0.205.1 engine now accepts explicit Store-backed creation and serial
upgrades. This reuses the existing child journals, timer owner, cycle reservations,
management effects, readiness verification and upgrade history. The artifact lease
selects either existing inline payloads or bounded Store metadata; Store-backed
children never retain application Wasm. A new per-child `CUS1` payload uses the
same two original allocation identities and capacities. Inline codecs and old
approval frames are preserved. No storage directory rewrite or new workflow
engine was introduced.

The source/tests/history research in the preceding Store entries remains
applicable: stable Store bytes are authoritative and the management cache is
derived. The first actual Root consumer replaces the pending per-chunk cache
endpoint with one bounded `prepare_cache` call. Store observes once and uploads
only missing distinct chunks, checking current caller authority and the original
deadline before every upload. Root's existing upload intent records that exact
request; installation remains a separate one-attempt effect.

Each Store operation binds its slot, action, unchanged effect approval, Store
principal and exact versioned manifest into a domain-separated digest. Root
retains one manifest plus that digest, pin progress and cache deadline. Pin intent
and outgoing call share a commit boundary; a completed pin callback commits
before later creation/upgrade work. Lost pin replies recover through replicated
observation. Lost cache replies recover by observing Store's actual chunk cache.
Neither recovery reissues creation or installation. Full Store custody includes
all historical upgrade sources; old complete-custody/reconciliation endpoints
refuse Store-backed work. A child cannot change artifact backends through an
upgrade request. Mixed inline and Store-backed children preserve both codecs.

Passing evidence:

- `make store-test`: eleven native protocol/custody tests and six installed
  cases. The installed suite took 16.40 seconds with synthetic cycles. Actual
  Root installs a valid executable larger than its former inline limit with its
  own management cache empty, then performs serial upgrades from Store. Pin and
  creation/upgrade-cache callback traps recover after Root maintenance; exact
  historical source bindings and an inline sibling survive maintenance.
- The installed refusal cases keep unpublished releases from creating a child,
  retain blocked missing-cache exposure without replay, and prevent late pin
  recovery from authorizing creation past the original deadline.
- `cargo test --locked --manifest-path qualification/Cargo.toml -p
  canic-qualification --test root
  children::two_children_preserve_history_and_upgrade_independently -- --exact
  --nocapture`: passed against the new Root, preserving actual predecessor
  history and independent inline-child upgrades (10.21 seconds). Its selected
  Root/app artifacts were copied from the same `target/store/` builds.
- Focused native and Wasm Clippy for management, Root and Store, qualification
  Store Clippy, profile-manager Wasm assembly, Root/Store Rustdoc,
  `make fmt-check toko-tooling-check` and diff checks passed. Supplier versions
  and the installed tooling snapshot are unchanged.

Logs: `target/qualification/store-root-tests.log`,
`store-inline-compatibility.log`, `store-native-lints.log`,
`store-root-clippy.log`, `store-root-docs-format.log`,
`store-root-format.log` and `store-profile-assembly.log` in the same directory.
The docs/format combined log records the initial manifest-order failure; the
separate format log records its correction and the passing tooling check.
Root hash: `b70af4f10a3604736707f8aa32acd1b4a461e00501403c9ee9d9d0e78f5e3005`;
maintenance candidate:
`b185314489f76de171f7a484571c0034f6b83499e4f9ad1621577d504447bb49`.
Store hash: `c14c7755a20d7f4db01228d1f26f001cbc223dd63d8775a1d519718721cfb6ec`.
These are local qualification binaries, not a deployed release.

The first Root recovery path deliberately remains blocked when an uncertain cache
preparation has missing chunks. A separately approved bounded repair flow is
still needed; no fresh deadline or automatic replay is inferred. Fleet Host/CLI
bootstrap, uploads through Root, Store maintenance through its real controller,
selected-version policy, warm-deployment latency measurement and existing-fleet
conversion remain next gates. No fleet CLI default, mainnet state or pushed
version changed in this increment. Current scope is in the
[Store guide](../demos/wasm-store.md).

## Store pins and installation-cache delivery: 2026-11-15

The next Store increment retains exact caller/operation/artifact pins and repairs
one management-cache chunk per bounded request. Same-content aliases cannot
relabel an existing pin. Replays preserve the obligation; the 32-pin artifact
limit refuses new work without eviction. No pin release or deletion is exposed.
The new `CWA2` artifact record reads `CWA1` with no pins under the existing four
allocation identities; native fixtures cover that transition and maximum pinned
metadata. Adding pins did not rewrite the Storage directory or create another
backing owner. Stable metadata has its own bounded decode policy, separate from
endpoint frame limits.

Revisited legacy
[cache restoration](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/runtime/template/publication/release/chunks.rs)
and [Store retirement](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/wasm_store/gc/mod.rs).
Carry forward stable-byte authority, missing-chunk repair and explicit Root-owned
retirement intent. No old GC timer or automatic retirement is imported. The
source/tests/history inspection recorded below remains applicable to the same
revision; the new pin ownership is a contract derived from the current design.

Installed qualification now proves cold cache population, warm reuse, refusal
without an exact pin, unchanged version commitments, expired dispatch and
unauthorized cache access. It clears the management cache explicitly, upgrades
Store, observes the same pin, reconstructs the cache, and uses real IC
`install_chunked_code` to install and upgrade a greeting. Both Store and target
have the same explicit controller on the fixture's one subnet. The test does not
impersonate a running Root. Discarding a successful cache reply then replaying
does not upload again; this is not a callback-trap qualification claim.

`make store-test store-clippy store-doc fmt-check toko-tooling-check` passed:
six native custody cases, one maximum-Wire-frame case and two installed cases.
The installed pair took 7.75 seconds with synthetic cycles. Logs are in
`target/qualification/store-delivery-checks.log`; current builder outputs remain
under `target/store/` (replacing the earlier local binaries recorded below).
Original Store hash: `741e0fc948e65b19c0f76d02aef040bde79b1d7da7f19e752c5631e3e4f8c77b`;
candidate: `75e4d997d0997215332f940412b7864900b2f5cba17931fc879323dc52c7b841`.

Root integration remains the next gate. Its current approval/custody records
describe local payloads and one management upload, without Store bindings or pin
receipts. A Store reference must be retained and included in complete maintenance
and reconciliation evidence, including historical upgrades, before replacing that
path. Preserve existing approval bytes and signed attempts; old observation
contracts cannot silently claim complete custody for a new reference-backed
operation. Qualify pin acquisition before dependent effects, cache recovery,
original deadlines and Root maintenance through that same operation owner. No
Root persistence change, fleet CLI default change, mainnet call or push was made.

## Standalone versioned Store qualification: 2026-11-15

The pending 0.205.1 increment adds the passive Store protocol and a separately
compiled publication canister. It reuses IC Storage, Wire, Platform entrypoints
and the public artifact builder. Supplier selections are unchanged. Root's
existing payload records and fleet deployment behavior are unchanged.

Adapted the Store regressions identified below: hash/identity conflicts,
byte/release capacity, visible headroom, reopened stable collections and
metadata-only catalog access. The new catalog deliberately retains earlier
versions instead of adopting legacy approved-manifest replacement. Native tests
cover incomplete-upload read refusal, exact replay, full digest mismatch, quota
changes, pagination and content reuse. A maximum-frame test caught insufficient
Wire node allowance; four-entry pages now round-trip with maximum labels,
32 chunk hashes and missing-position lists under the explicit metadata policy.

The installed PocketIC fixture qualifies the current 8 MiB artifact ceiling:
32 distinct 256 KiB chunks, interrupted after the first chunk, then a stopped
Store upgraded to a different executable. The second executable retains exact
progress, accepts only missing chunks, publishes verified bytes and returns them
through replicated reads. A second version reuses the same artifact without
increasing chunk count or payload reservation. Anonymous mutation, oversized
input, conflicting version bytes and quota shrink below custody are refused.
No deletion endpoint exists; unfinished uploads keep their reservation.

Evidence: `make store-test store-clippy store-doc fmt-check toko-tooling-check`
passed; the installed case took 7.65 seconds locally. This is a synthetic-cycle
fixture duration, not mainnet deployment latency or a speedup comparison. The
initial sandbox attempt could not bind loopback; the focused rerun had local
loopback permission. Logs are in `target/qualification/store-checks.log`; builder
records and exact binaries are under `target/store/`:

- Original: `dd29d6f8f592652af86f5bdb001bfeac29efc96ffc3944ffb82e984c07927ada`.
- Maintenance candidate: `02c225d5ee19351b67023d00e8325da53e20a59a37987dfe27eb73ed1a371806`.

The documented `canic artifact build --package canic-wasm-store --output
target/wasm-store.wasm` and `canic artifact inspect target/wasm-store.wasm` were
also exercised; the original executable is 332,758 bytes. No browser, mainnet
deployment, push or publication was performed.

Next is the existing management owner consuming pinned Store references and
rebuilding its installation cache, then fleet CLI integration. Selected-release
policy, retention/deletion, autonomous installation, failed Store migration,
Root-held payload migration and cross-subnet delivery remain unqualified. The
standalone fixture does not replace the retained artifact-effect probe or qualify
the full section-12 contract. See [current Store scope](../demos/wasm-store.md).

## Versioned Wasm Store decision: 2026-11-15

The maintainer selects a separate dynamic, versioned Store per managed fleet,
superseding the initial decision to retain demo artifacts inside Root/manager.
The current aggregate artifact budget and historical storage layout demonstrate
why continued artifact growth should not enlarge Root's authority journal.
[Design section 12](../design/canic-system-design.md#per-fleet-versioned-wasm-store)
owns the new contract and qualification gates. This is a design decision, not a
shipped Store, runtime migration or measured performance improvement.

Legacy research used `dragginzgame/canic-legacy` at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`:

- [Manifest storage](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/storage/stable/template/manifest/mod.rs)
  and [catalog publication](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/runtime/template/publication/release/catalog.rs)
  retain release keys, versions, hashes, sizes and Store bindings. Preserve exact
  identity while separating release history from the current deployment selection.
- [Chunk cache restoration](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/runtime/template/publication/release/chunks.rs)
  explicitly treats stable Store bytes as authoritative and repairs missing
  management-cache chunks. Carry forward that ownership and reuse criterion.
- [Storage regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/storage/stable/template/chunked/tests/mod.rs)
  cover reopened chunk references, exact snapshots, slot reuse and metadata-only
  counts. [Admission regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/ops/storage/template/tests/mod.rs)
  cover hash mismatch, byte/template/version limits and visible headroom. Adapt
  these cases when implementing Store; they have not been run against new code.
  The same suite expects replacement of an approved manifest to remove its older
  version. Do not carry that behavior into the new retained release catalog.
- Reviewed the publication-chunk file's history listing at commits `aa02dc0`,
  `afc2521` and `e9748fa`, and the
  [bootstrap diagnostic correction](https://github.com/dragginzgame/canic-legacy/commit/ed9f133149ac0dd220877b981ff8b09eb9dc6643),
  which added Root logs when waiting for readiness fails. Keep phase/blocker
  diagnostics in the public workflow; this history supplies no latency benchmark.

Current source inspection finds default dispatch at planning time plus ten
minutes in `crates/canic-host/src/fleet_plan/mod.rs` and
`crates/canic-host/src/managed/mod.rs`. The new design separates that scheduled
wait from build, upload, installation and observation time. It requires warm
artifact reuse and a qualified ready-to-dispatch contract, without changing
retained timestamps or claiming the Store alone will make deployment faster.
The [IC management specification](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/#ic-method-install_chunked_code)
requires a same-subnet chunk source and appropriate caller control; the initial
Root/Store/app graph must qualify this before cross-subnet delivery is added.

Validation for this change is documentation consistency, source/link inspection
and `git diff --check`. No executable or deployed state changed for this decision.

## Post-push review: 0.205.0

Review `f6b5c37ead9f443f49d1d7cac0b0897f56d221e7`, delivered to `main` and
annotated tag `v0.205.0`, against the operation model, fleet membership,
retained-record codecs, status consumers and current setup documentation.
Historical optional fields remain confined to record codecs; their golden
digest fixtures justify retaining them. The two-child runtime boundary remains
explicit and does not claim general tree or subtree execution.

Three concrete leftovers are corrected in the pending 0.205.1 patch:

- Maintenance status had a second list of observation steps, missing the
  supported predecessor's two actual preflight records. Status now projects the
  same selected observations as execution. Retained replies remain distinct
  from completion receipts, including after an installation reply is lost.
- Detailed child status blocks lacked their names. Each block now identifies
  its retained child, using the existing terminal escaping owner.
- README copied outdated Tooling/Testkit versions. Manifests and lockfiles remain
  the version owners; the documentation no longer duplicates those numbers.

Reuse the legacy progress projection and regressions recorded under
[readable retained-operation status](#readable-retained-operation-status), at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801` and history
`afc2521ed02744c1e1a376d59dae5e2b80d9dec5`: preserve exact-request correlation and
unknown outcomes without adding a second progress engine. The predecessor
fixture and two-child CLI journey now check the corrected local projection.
No approvals, signed attempts, runtime effects or dependencies change in this patch.

Focused validation passes three Host maintenance tests, eight CLI status tests,
Host/CLI and Root-fixture Clippy, and formatting. The installed predecessor
journey verifies empty/completed custody and unknown installation outcomes;
the installed two-child journey verifies both named detail sections alongside
creation, upgrades and maintenance. Logs remain under
`target/qualification/status-*.log`. These follow-up changes remain local and
uncommitted after the 0.205.0 push; `make canic` installs them for hands-on use.

## 0.205.0 release qualification

On 2026-11-15, the complete Linux release gate passes: `make lint`, then
`make -j1 test effect-test artifact-custody-test journey-test profile-test
growth-test root-test endpoint-test`. This includes 74 Host and 42 CLI unit
tests and eleven installed Root tests. Three historical executable scenarios
retain their explicit opt-in status; supported predecessor maintenance and the
one-child history migration are exercised by the normal Root suite.

Both lockfiles' already-selected IC Platform 0.33.6 is covered by the native and
Wasm checks and rebuilt installed fixtures. Twelve product packages remain at
0.205.0; three qualification packages remain at 0.0.0. All are non-publishable.
The release keeps saved approvals, signed attempts and current mainnet canisters
unchanged. Logs and selected source/dependency hashes are local under
`target/evidence/0.205.0/release/`; no macOS or new mainnet qualification is claimed.

## Host operation simplification (0.205.0 work, 2026-11-14)

Replace independently optional operation authority with `PlanAction` variants.
Use one ordered fleet child collection with explicit runtime slot identity, and
one ordered custody collection for Root maintenance. Planning, execution,
reconciliation, inventory and CLI presentation consume these models. Historical
`child`/`sibling` fields and directory names belong only to retained-record codecs;
old approval digests and signed attempts are not rewritten. Fleet apply/status
JSON exposes named collections; the nested immutable approval keeps its encoding.

This adapts the legacy research in [Cascading and workflow scope review](#cascading-and-workflow-scope-review)
and [Application tree and role correction](#application-tree-and-role-correction): exact member
identity, immediate-parent validation, stable ordering and per-target evidence.
The pinned cascade history's lesson remains applicable: one child's evidence
must not substitute for another's. The current runtime is deliberately bounded
to two permanent children; this change introduces neither arbitrary tree
execution nor a general workflow engine.

Retained unsigned v1/v2 fleet and operation fixtures prove unchanged digests.
Negative cases reject partial/conflicting authority, changed creation grants,
missing first custody, reordered/duplicate slots and duplicate child names.
Existing wrong-parent, wrong-target, expired-authority, signed-attempt retention
and uncertain-result tests remain in the normal suites. Installed CLI tests
exercise the new named result collection through the existing deployment,
upgrade, recovery and maintenance journeys.

Validation: 74 Host and 42 CLI unit tests pass. Ten selected installed Root tests
and the direct CLI journey pass, including two-child creation, independent
upgrades, both predecessor maintenance cases, private profile preservation and
lost-reply recovery. Host/CLI and changed qualification tests pass Clippy; Host
Rustdoc passes. Installed logs are `target/qualification/simplification-root.log`
and `simplification-direct.log`. `make canic` installs the working 0.205.0 CLI
on PATH. No runtime/protocol change, new dependency, mainnet mutation or push is
part of this simplification.

## Qualification ownership cleanup (0.205.0 work)

Consolidate the M0/M1 native harnesses into `qualification/tests/`
(`canic-qualification`). Shared simulator setup no longer belongs to M0; every
fixture and the explicit setup example use the same prepared directory and override.
Rename the retained Wasm fixtures to `effect-probe` and `artifact-probe`, and the
Make workflows to `qualification-setup`, `effect-test`, `effect-maintenance-test`,
`artifact-custody-test` and `journey-test`, with corresponding lint/doc targets.
No compatibility aliases, additional crate or simulator installer are introduced.
The workspace shrinks from four packages to three without deleting scenarios.

The coverage comparison retains isolated creation commit boundaries, the funding
success/refund/uncertainty matrix, combined-journal maintenance rollback, and
install-only occupied-target refusal. Product Root/growth tests overlap some
cases but do not yet replace all those obligations. Their existing regression
sources, binaries, authority and storage contracts are unchanged; this is test
ownership cleanup, not a new runtime design. The legacy research recorded below
remains applicable; no new legacy mechanism is imported.

Validation on 2026-11-14: explicit shared setup, `make effect-test` and
`make artifact-custody-test` pass (four probe unit tests and five installed tests,
including their interruption matrices). The moved two-child CLI journey and both
retained-predecessor maintenance tests pass against the same prepared server.
An unprepared directory selected through `IC_TESTKIT_SERVER_DIRECTORY` fails
before creating the directory or starting the simulator; tests do not run setup.
Native harness/probe and Wasm probe Clippy, Rustdoc, formatting, snapshot/dependency
agreement, shell syntax and local documentation links pass. Both predecessor
binary hashes are unchanged. Registry package selections and checksums match the
previous release; only local package ownership changes. Logs remain under
`target/qualification/ownership-*.log`. No full repository gate, push or mainnet
mutation is part of this cleanup.

## Recorded status before ownership cleanup

Status: working toward M2, 2026-11-14, with narrow M3 paths qualified. This
record retains the initial M0 decisions alongside subsequent journey evidence;
it does not declare the full milestone gates complete. The [architecture](../design/canic-system-design.md)
is accepted; local creation and direct-cycle funding commit-boundary experiments
pass. The M1 CLI now exercises retained signed host attempts, deployment and
same-canister upgrade locally. Mainnet creation, installation, greeting and the
profile entry/worker deployment and persistent worker upgrade have passed.
A local infrastructure-owner maintenance window now preserves both effect
journals across a changed executable and failed upgrades. A bounded artifact
fixture also installs a retained worker module without the operator. The profile
demo now qualifies a one-slot manager that creates and installs its worker
after admission without the host, including normal mainnet growth and successful
mainnet manager maintenance. Production
management-owner persistence and comparative adoption measurements remain unqualified.
This record contains scoped decisions and evidence, not a second implementation
backlog. The first product is the [M1 Hello World loop](../../README.md), followed
by the [persistent mainnet profile pair](../demos/profile.md#current-mainnet-demo).

## Two-child fleet qualification (0.205.0 work)

Extend the enduring effect owner and its existing CLI approval flow. The runtime has
stable First/Second slots, each retaining its own creation, current upgrade and
up to eight upgrade approvals. Original endpoints remain bound to First so old
signed attempts retain their meaning. New requests carry the selected slot in
the authenticated Wire payload; callbacks never consult an ambient active child.
Each child owns its timer and observation guard. Lost callback evidence blocks
that child while its sibling can continue.

Legacy research reuses the cascading review below at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, particularly
`workflow/cascade/topology.rs` and `ops/cascade/tests/mod.rs` under
`crates/canic-core/src/`. The prior review of
`c04cd56c11d287cf5baebb472baac61706434dc5` establishes why first-error cascade
handling was replaced with per-target outcomes. Carry forward explicit target
binding, wrong-parent/receiver refusal and sibling failure isolation. Do not port
the directory cascade or introduce routing ownership merely to create two children.

Both slots stay inside the original two checked IC Storage leases. CUJ4 reads
v1/v2/v3 journals into First and leaves Second Empty; CUA3 retains four creation/
current-upgrade buffers. The installed capacity experiment found that the original
three eight-page storage buckets cannot hold four independent 512 KiB artifacts.
Keep the directory intact: share a 1,048,512-byte artifact budget (64 bytes reserved
for cell/payload framing), checked before admission or replacement. An older
payload above this new aggregate bound refuses migration atomically. No directory
rewrite, backing bypass or claimed universal migration is used.

Cycle admission reserves every scheduled, unissued creation attachment plus the
shared 0.5T liquid reserve. Issued attachments are already deducted; uncertain
creation never reacquires authority. Full-owner custody includes both slots even
when empty. The old single-child custody endpoint traps once Second is occupied,
so a one-child maintenance client cannot silently discard admitted work.

The fixture in `qualification/m1-tests/tests/children/` covers actual v3 migration,
operator-absent sibling creation, exact replay, wrong-slot upgrade refusal,
independent upgrades, callback loss, Root maintenance, shared cycle reservations,
and artifact capacity. The retained release binary is documented with its hash
in `qualification/m1-tests/fixtures/README.md`.

Validation on 2026-11-14: `make root-test` passes ten installed tests (two older
historical fixtures remain opt-in), `make growth-test` passes its three protocol
tests and two installed scenarios (one historical fixture remains opt-in), and
`make endpoint-test` passes. The final three child tests pass after adding
second-slot interrupted-upgrade reconciliation: the creation-uncertain sibling
stays blocked, the recovered sibling reaches Ready with one install attempt.
Both histories fill to eight entries at the exact shared artifact budget and
survive maintenance. Root Clippy/Rustdoc, formatting and tooling agreement pass.
`make canic` installs 0.205.0 on PATH; no mainnet state changes or publication occur.
That runtime qualification build measured 906,591 bytes, within the existing Host artifact bound.
Logs remain local under `target/root/children-*.log`.

The installed CLI now binds a new fleet's two names to permanent First/Second
slots before signing. V2 fleet approvals retain per-child attempts under the same
journal lock; optional fields remain absent in v1 records. A frozen public,
unsigned v1 approval in `crates/canic-host/src/fleet_plan/tests/fixtures/v1.json`
retains the mainnet creation digest `a92356c7cfbeae5a2b380666093d7eae551f09f417e4264a0319729a76e9a1ab`.
Configuration cannot rename or reassign that retained mapping. Root maintenance
binds both complete journals and verifies both children's live code/controllers.
The same bounded fleet collector projects both children to terminal and HTML;
it never publishes completion receipts from public observations.

`qualification/m1-tests/tests/fleet_children/` exercises the shipped CLI from
generation and `app build` through creation, replay, lost second-child admission
reply recovery, explicit reconciliation, independent upgrades and Root maintenance.
It checks both views, retained-name calls, rename refusal, unchanged creation
receipts, and untracked Second work blocking maintenance before install signing.
This installed journey passes; ten earlier installed Root scenarios also passed
in the selected runs (two historical fixtures remain opt-in). Focused host and
CLI regression checks cover old approval digests, scoped fresh-read authority and
build recipe argument relationships. The 71 host tests pass; all 42 CLI tests
are covered across focused runs, including the three localhost tests rerun with
socket permission. Final affected Clippy/Rustdoc, formatting and tooling agreement
pass. The Root used by the CLI fixture is 908,532 bytes. Logs remain under
`target/root/multi-*.log`.

Focused sediment/drift review: remove one-child assumptions from the existing
convergence path and collector, centralize signed method selection, remove the
unused old journal writer, and correct stale one-child-only documentation. Keep
first-slot wrappers only to preserve existing signed contracts. Expose the
existing manifest/package recipe on `app canister add` so `app build` still works
after adding a child. No command aliases, new storage owner, general workflow
engine or supplier upgrades are introduced.

The remaining limits are deliberate: two fixed direct children, eight upgrades
each, one artifact change per approval, serial host admission, and no expansion
of an already-retained one-child fleet. Admitted runtime work proceeds independently;
this does not promise host-side best-effort cascading after an admission error.
Mainnet qualification for two children, arbitrary tree depth and subtree cascades
remain outstanding. The current mainnet starter is unchanged.

## 0.204.1 release qualification

On 2026-11-14, the complete release gate passes: all `make lint` stages, followed
by `make test m0-test m0-artifact-test m1-test profile-test growth-test root-test
endpoint-test`. This includes 68 host tests, 40 CLI tests and seven installed Root
tests, with the supported predecessor fixture always enabled. Three unrelated
historical-binary fixtures retain their explicit opt-in status. Release logs and
input hashes are local under `target/evidence/0.204.1/release/`.

The already-selected product lockfile update to IC Timers 0.44.3 is qualified by
this gate. Its missing Wasm dependency was fetched with `--locked`, then the
remaining lint stages resumed before tests. Both manifests and locks stayed
unchanged throughout validation. Registry review confirms selected first-party
suppliers are current stable, non-yanked versions. Product metadata has 12 members
at 0.204.1; four qualification members remain at 0.0.0, all with publication disabled.

The mainnet evidence below belongs to its retained artifacts, built before the
Timers refresh. This release does not replace those approved bytes or redeploy
the demo. `starter/` and its active journals remain local, outside Git; the
reusable generator and the exact predecessor regression fixture are delivered.

## Mainnet predecessor transition and subsequent app upgrade

On 2026-11-14, create the generated `starter/mainnet` fleet using the exact
qualified predecessor, then maintain Root and upgrade its app through the CLI:

- Root: `epxmv-tiaaa-aaaah-awe2a-cai`, solely controlled by the Canic operator
  `o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe`.
- Application: `eiwkb-6qaaa-aaaah-awe2q-cai`, solely controlled by that Root.
- Creation: `a92356c7cfbeae5a2b380666093d7eae551f09f417e4264a0319729a76e9a1ab`.
- Root maintenance: `c18ae2216d767a9f75678152e4faf837eaf6e2d5c3be75c47e3f8d41e724b1f9`.
- Child upgrade: `0a68a8de48d4ca642147087a893813359ca4b7eda6847cf554acfaaa91c757cc`.

Maintenance changes Root from the pinned predecessor hash below to
`3a6bad4277923e88604781f5700f70830a5f14ffbf31d9b9ae9c6784d81384e1`.
Its two authenticated preflight replies and full-custody postflight agree on
creation authority, child, dispatch commitments and empty upgrade history.
Both principals and controllers survive. The subsequent app upgrade changes
`2e4c774d9fdabe34922a4b353c71c54c7edbe7a21e94e6df102273f740eef071` to
`5d65b94c2be98a2c9a6eac541cc0a11f3d1bc9944368bdf3479b3b9321bb9262`, with
one upload/install, child version 2 → 3 and authenticated Ready completion.
The greeting changes from `Hello from Canic, team!` to
`Hello from upgraded Canic, team!`; these greeting queries are uncertified.
The final read-only fleet snapshot has two canisters and no inventory issues;
planning reports unchanged against retained completion.

The maintainer explicitly authorized a one-off conversion through ICP CLI:
1.78 ICP plus the 0.0001 ICP transfer fee credited 3,009,524,000,000 cycles.
The fleet's 3T debit funds Root and its 1T child attachment. Final public wallet
observations are 3.32915125 ICP and 415,079,115,186 cycles. No conversion command
was added to Canic. A preliminary unsubmitted ten-minute creation preview was
cancelled before preparing the actual two-minute approval; its revocation stays
retained. No existing profile or hello-world deployment was changed.

The generated project is `starter/`, with operation custody in its configured
`.canic/apps/starter/fleets/mainnet/` and auxiliary observations/builds/funding
receipts in `.canic/demo-evidence/`. Keep these records for recovery. The mainnet
run proves the normal transition; interruption recovery and completed-old-upgrade
preservation were qualified locally, without injecting failures into the live demo.

## Qualified single-upgrade Root transition

The 0.204.1 host admits one exact pre-0.204 Root module,
`e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296`, for maintenance
after completed child work. Its controller-only replicated creation observation
(ID 21) and single-upgrade observation (unit ID 25) together describe the entire
old journal. Compare both records, including an empty slot, before preparing the
install. Normalize only the already excluded live page count and executable
marker. Preserve each original reply and signed attempt separately; do not
invent a synthesized authenticated response. The replacement uses the current
full-custody postflight, and only its completed receipt advances future authority.

The exact predecessor hash already belongs to the immutable maintenance plan;
it selects these read contracts before dispatch. Unknown hashes retain the
current full-custody requirement. There is no endpoint probing or fallback on
rejection, no unbound live read for child recovery, no pending-attempt re-signing
and no old-plan rewriting. The operator still owns an exclusive maintenance
window. Active/uncertain work, other old modules and Root replacement remain
unqualified. The known predecessor cannot represent multiple historical upgrades.

Research reuses and rereads the pinned legacy component-operation owner and
lifecycle-boundary regressions at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`,
plus the `aed32be2afbaf66d7e2867292da52f75ce028e41` history recorded below.
Preserve exact intent, refusal on changed custody and failed-upgrade preservation;
legacy's catalogue and retries do not establish this transition. Inspect the
single-slot engine and private endpoint mounts at Canic `v0.203.1` (`a2ecaa3`).
The retained binary is a behavioral witness, not a reproducible-build claim.
Keep it under `qualification/m1-tests/fixtures/` so the normal Root suite always
exercises this supported predecessor rather than silently skipping a cache input.

Focused qualification passes four maintenance/authority unit tests, the remaining
signer tests, Host/CLI and Root-fixture Clippy, and the installed Root suite:
seven passed, two older unrelated fixtures remain opt-in. Installed cases cover
empty and completed single slots, retained ID-25 historical evidence, untracked
Scheduled refusal before install preparation, lost installation-reply recovery
without another installation, unchanged creation records/controllers/child, and
a subsequent child upgrade bound to the maintained Root. Existing greeting,
profile, two-fleet and current-Root maintenance journeys also pass. Logs are
under `target/evidence/0.204.1/predecessor/`. Runtime code is unchanged.

## Post-push review: 0.204.0

Review pushed commit `b37a274` against App → Fleets → Canisters, CLI planning and
status callers, custody checks, signed installation and bounded runtime history.
Whole-fleet convergence reuses the existing plan publisher and effect owners;
maintenance retains both custody observations before accepting completion.
No new aliases, deployed roles or supplier dependencies were introduced. Old
journal readers preserve required v1/v2 state and are exercised by migration
qualification; they are not redundant deployment modes.

One concrete leftover is `crates/canic-host/src/plan.rs::retain_locked`, whose
only caller is now in that module. Narrow its visibility in 0.204.1 rather than
keeping an unnecessary internal entrypoint. No further actionable duplication
was found in this focused review. The documented old-Root bootstrap,
active-work maintenance and multiple-child limits remain separate future work.

## 0.204.0 release qualification

On 2026-11-14, adopt published Toko Tooling 0.43.2 through its authenticated
installer in both workspaces. All 47 selected snapshot files retain their bytes;
the exact pins, locks and snapshot provenance advance together. Registry-index
review confirms the selected first-party Host, Platform, Wire, Storage, Timers,
Metrics, Testkit and Tooling versions are current stable, non-yanked releases.
The third-party transitive Certification 3.2.0 and qualification management types
0.8.0 remain supplier-owned selections; review their newer major/minor lines at
the next dependency-maintenance boundary, without overriding supplier constraints
in this tooling adoption.

The full release gate passes: `make lint`, then `make test m0-test
m0-artifact-test m1-test profile-test growth-test root-test endpoint-test`.
This includes 65 host tests, 40 CLI tests, installed creation/funding/recovery,
serial child upgrades, Root maintenance and endpoint authority/rollback checks.
The separately enabled retained-v2-manager fixture also passes scheduled and
completed journal migration into serial upgrade history. Two older Root binary
fixtures remain opt-in and were not rerun in this release. Logs, selected-input
hashes and registry evidence are local under `target/evidence/0.204.0/`.
After version preparation, both Cargo graphs verify all 12 product members at
0.204.0, all four qualification members at 0.0.0 and publication disabled.
Starter source-generation tests, recursive CLI help/version checks, formatting
and tooling agreement pass; `make canic` installs 0.204.0 on the existing PATH.

This release qualifies local simulator paths. No mainnet deployment or funding
occurs, and crates.io publication remains disabled. Older Roots without the
custody endpoint, maintenance during active work and multiple managed children
remain outside the qualified maintenance path.

## Root maintenance through whole-fleet planning

`fleet plan` now treats a Root-only artifact change as a separate maintenance
approval after completed creation and child upgrades. It derives the Root identity,
previous code and exact completed journal under the existing journal lock. The
optional maintenance field binds that custody into the existing Plan digest;
plans without it retain their original bytes and digests. No new signer,
installation engine, reset command or automatic rollback is introduced.

The shared engine exposes controller-only replicated `observe_custody` (ID 30).
Its bounded response contains creation and every admitted upgrade, oldest first,
without artifact bytes. The live page count and maintenance-executable marker are
excluded from durable custody. Current/creation artifact preservation remains
checked by the existing lifecycle validation. This adds no storage allocation or
stored schema. The full eight-upgrade response passes the bounded codec fixture.

Apply checks Root/child code and sole-controller bindings, retains an authenticated
pre-install journal read, reuses the direct signed `install_code` attempt, and
requires matching post-install custody before publishing completion. Missing or
mismatched preflight/postflight evidence cannot unlock the next plan. Completion
advances only future child plans' Root hash; old approvals and signatures remain
unchanged. Root and child changes use separate approvals. Unfinished/failed/uncertain
work cannot be overtaken. The operator must keep the window exclusive; this is
not a distributed fence against another concurrent controller session.

The installed Root must already support ID 30. An older owner fails the preflight
before installation; there is no fallback or automatic rebinding. Bootstrap from
older deployed owners, maintenance of active or blocked work and replacement of
the Root principal remain distinct qualification requirements. Existing mainnet
deployments are unchanged.

Research reuses the pinned legacy evidence below and rereads component-operation
`ops/mod.rs` and `qualification/canic-tests/tests/lifecycle_boundary.rs` at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`. Exact retained submission and fail-closed
cross-release checks remain useful; legacy's repeated prepared-state and rejected
upgrade tests do not prove journal-preserving maintenance. Keep their refusal and
identity invariants, omitting legacy release catalogues and pool orchestration.
The earlier `aed32be2afbaf66d7e2867292da52f75ce028e41` history investigation remains
the provenance for that boundary, not evidence of a finished legacy upgrade flow.

Validation passed on the installed PATH CLI: three whole-fleet cases (29.74 s)
cover greeting/profile preservation, Root maintenance followed by two more child
upgrades, lost install-reply resumption without another installation, second-fleet
isolation, missing postflight evidence and refusal of untracked child work before
signing installation. Installed endpoint admission passed (9.07 s). The full
custody codec and read-authority rejection tests passed, alongside affected host/CLI
tests, Clippy, Rustdoc, formatting and tooling agreement. A retained fleet envelope
from before the optional Plan field still loads under its original digest.
Logs are under `target/repeated-upgrade/maintenance-*` and
`target/repeated-upgrade/root-maintenance-test.log`. The CLI is installed locally;
no mainnet mutation or release accompanies this increment.

## Bounded serial child upgrades

Whole-fleet planning now admits the next child upgrade after verified completion,
retaining at most eight immutable approvals. The runtime archives each completed
status, including request commitments and version observations, before admitting
its successor. Creation custody never changes. Current and creation artifact bytes
stay in the original artifact allocation; prior bytes remain in each host plan,
with hashes and outcomes retained on-chain. No record eviction, slot reset or
automatic replacement of failed, expired or uncertain work is admitted. The
individually created-child path retains its existing first-upgrade planning bound.

The v3 journal reads original creation and v2 single-upgrade payloads inside the
same two allocations. Eight upgrade records plus creation fit the existing 8 KiB
journal; the worst-field codec fixture uses 2,793 payload bytes. Codec allocation
and node budgets cover nine records, while the stable byte limit remains fixed.
The limit of eight is an explicit experimental bound, not an unlimited maintenance
claim or permission to clear history when full.

Dispatch order, exact child/Root bindings and old/new code form one serial chain.
A successor's fresh observed canister version must not regress from its predecessor;
it need not equal it. The installed repeat test exposed that ordinary application
updates and replicated readiness calls advance canister versions, as specified by
the [IC interface specification](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#canister-version).
Each operation retains its existing strict pre-install version and exact increment
check. This still qualifies a quiet installation window; intervening application
activity or platform version changes can conservatively block it. General online
rollout/drain remains a separate qualification requirement, not an inferred guarantee.

`observe_upgrade` now takes the exact approval using Wire ID 29 and can return
historical evidence without dispatch. Historical reconciliation similarly returns
the retained outcome without touching current work. Public ID 25 remains the
latest-status query; fleet views select only the tip of a validated local chain.
The host reads previously retained ID-25 replies but never substitutes an unbound
live read or re-signs a mismatched old attempt. Older owners require separately
qualified maintenance and explicit binding handling; existing plans and deployed
mainnet owners are not rewritten. Root replacement through the CLI remains open.

Reuse the legacy component-operation, installation and lifecycle research recorded
below, especially immutable intent, exact replay and failed-upgrade preservation.
Legacy did not prove this serial maintenance contract. Add explicit replay after a
code-hash cycle, predecessor completion, full history, pending successor refusal
and v2 migration cases to the current fixtures rather than copying a general
workflow catalogue. The same journal lock protects whole-fleet chain validation
and publication through the existing Plan writer; there is no second signer or
runtime effect owner.

Installed validation passed: three whole-fleet CLI upgrades for both greeting and
profile workloads with second-fleet isolation (four enabled Root tests, 29.86 s);
eight runtime upgrades, historical replay/reconciliation, full-history refusal,
profile preservation and existing callback/rejected-hook cases (15.12 s). Migration
passed from the retained v2 manager SHA-256
`7d689001ba89738e5ffb5d3f77fc95e240a45b4bd28eeb13b4f5d952ce278749`
with its first upgrade both Scheduled and Ready (21.96 s). Reproduce that opt-in
fixture with `CANIC_PREVIOUS_UPGRADE_MANAGER` and the ignored
`previous_single_upgrade_manager_migrates_to_retained_history` growth test.
Two older v1 Root fixtures remain opt-in and were not repeated. Logs are under
`target/repeated-upgrade/`. The pre-existing lockfile refresh reached Platform
0.33.5 and Runtime/macros/guard 0.22.5 during this work; the final installed serial,
migration and Root runs use those selections. No supplier refresh was initiated
by this increment. No mainnet mutation or release accompanies it.

The installed PATH command also passed both whole-fleet journeys after moving
profile-state and ownership assertions after all three upgrades (29.74 s).
Approval-bound endpoint checks, focused host/CLI/protocol tests, affected Clippy
and Rustdoc checks, formatting and tooling agreement passed.

## Allocation-pattern scope clarification

The maintainer confirmed retaining singleton, scaling worker, shard and keyed
instance outcomes. [Design section 8](../design/canic-system-design.md#8-allocation-placement-and-scale)
now states their scopes, separates physical placement from assignment and tree
authority, and makes upgrades/recovery part of each selected consumer's acceptance.
The capability register names all four explicitly. This is a design clarification;
no new configuration values, allocation engine or runtime qualification is claimed.

Research at legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` reused
`CONFIG.md` lifecycle classes and scaling/index/sharding pool definitions, then
inspected `crates/canic-core/src/workflow/placement/index/create.rs`, its
`tests.rs`, and `workflow/placement/sharding/release/tests/mod.rs`.
Path history and commit
[`ccca2a1b3a426691891efc972886020ad72b74a6`](https://github.com/dragginzgame/canic-legacy/commit/ccca2a1b3a426691891efc972886020ad72b74a6)
show why allocation identity must survive claim finalization and receipt cleanup:
late completion must neither bind nor recycle a reused principal. The inspected
regressions also cover wrong-parent/role binding, existing and pending resolution
without creation, exact stale-claim recovery, and release only for a configured
shard pool. Carry these cases into the corresponding implementations; they were
reviewed here, not executed against a new placement subsystem.

Keep the protections and shared allocation ownership. Do not require legacy
Component/Group catalogues, a mandatory Coordinator or four separate engines.
Singleton scope is per concrete parent, keyed scope is explicit, and the scaling
name does not promise application data replication. General policies follow real
consumer needs; the fixed child slot remains the qualified whole-fleet allocation
path. Serial upgrade evidence is recorded above; multiple-child allocation is not
yet qualified.

## First whole-fleet child upgrade

Extend the existing `fleet plan` path for a fleet created through its Root-plus-app
approval. Under the journal lock, resolve the child and original code from
completed creation, requiring the retained receipt and matching authenticated
Ready evidence. Retain a normal managed upgrade Plan with zero creation debit;
do not add an upgrade envelope, runtime journal or second effect owner. The
original fleet bytes and nested creation evidence remain unchanged. The supplied
Root artifact, network, operator and declared parent/child pair must still match.

Pending upgrade authority takes precedence over historical creation completion.
Identical replanning preserves its schedule even after expiry; different artifacts
cannot reset its slot. After authenticated upgrade completion, identical artifacts
produce a local no-change result. This compares retained completion, not current
network state; apply still checks exact live module/controller bindings. Root
replacement, repeated upgrades and adoption of unrelated individual deployments
remain outside this increment. The first repeatable maintenance contract still
needs bounded history and admission rules; clearing the existing journal is not
an implementation option.

Reuse the legacy research recorded under the child-upgrade investigation below:
revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, component-operation workflow,
operations/tests, installation workflow and lifecycle-boundary regressions;
history `aed32be2afbaf66d7e2867292da52f75ce028e41`. The applicable protections are
immutable review identity, exact approved bytes, retained uncertainty, drift
refusal and application-state preservation. Legacy's install workflow is not
evidence of a qualified persistent upgrade product. The current small topology
needs no catalogue, generic workflow engine or automatic subtree expansion.

The whole-fleet qualification uses the existing CLI and generated app for two
fleets, then upgrades one child and recovers a lost admission reply. A second run
selects the profile manager/worker to verify private data, ownership and canister
identity across the same CLI path. The profile wrapper adds only the existing
authenticated `observe` mount needed by Host; its application readiness and
lifecycle remain explicit. Native cases reject receipts without Ready evidence,
stale creation artifacts while an upgrade is pending, substituted children and
renewed schedules. No mainnet mutation or publication accompanies this work.

Validation: the existing 63 host tests and 40 CLI tests passed; the five focused
fleet-plan tests include the new custody regression. Both whole-fleet simulator
journeys passed (26.79 seconds), alongside the existing Root/individual-child
journeys in the preceding Root run. The endpoint contract fixture passed with
the manager observation mount. Focused Host/CLI, qualification and manager-Wasm
Clippy, Host/manager Rustdoc, formatting and tooling agreement passed. Logs use
`target/fleet-upgrade-*.log`; the initial Root run retains its corrected fixture
assertion failures, and `whole-test.log` records the successful rerun. Historical
binary migration tests remain opt-in and were not repeated. No engine state,
storage schema, supplier selection or retained plan encoding changed.

## 0.203.1 supplier adoption and delivery

The final batch adopts published Host Text 0.76.0 and qualification Testkit
0.106.0. Registry source provenance identifies Text commit
`16bdd537d6652fb7e1987e3cdac5d5d0ce70a8d1` and Testkit commit
`d12b0174cd04c556f065c39bc12dd366628cc6df` in IC Tooling. Text's removed
quantity/timestamp APIs had two fleet-plan callers: Canic now formats exact
cycle budgets and uses Time 0.3.55 (already in the host graph) for UTC calendar
conversion. Tables, widths and escaping stay with Host Text. Testkit's removed
diagnostic APIs have no callers here; its 0.79 Host dependencies remove the
qualification graph's duplicate 0.78 family. No compatibility wrappers or new
runtime supplier enter the canister graphs.

Registry freshness review on 2026-11-14 confirmed the selected Host family
0.79.0, Text 0.76.0, Testkit 0.106.0, Platform 0.33.4, Runtime 0.22.4,
Storage 0.74.1, Timers 0.44.2, Wire 0.22.1 and Toko Tooling 0.43.1 against
current publications. Types 0.20.4 is also current. Unrelated supplier selections
stay unchanged.

Release validation completed the configured `make ci` obligations: `make lint`
first, then the library, M0, artifact, M1, profile, growth, Root and endpoint
checks. All enabled tests passed on Linux with synthetic cycles, including 63
host tests and 40 CLI tests. Two historical Root migration tests remain explicitly
ignored without their retained older Wasm inputs; this release does not renew
that historical-artifact evidence. The locked Root/manager normal-build Wasm
graphs exclude Host, Testkit, Time and tooling dependencies. Validation logs are
retained at `target/release-0.203.1-{lint,tests}.log`. Version 0.203.1 is prepared
for GitHub delivery only; crates.io publication remains disabled.

## Explicit CLI reconciliation

The next M2 increment exposes `fleet reconcile OPERATION` for retained Root
creation and child-upgrade approvals, including resolved whole-fleet children.
It cannot deploy a missing Root, materialize an unadmitted child or reset slots.
The host checks the exact Root module and sole operator controller, then signs
only a request carrying the original approval. Root checks that approval before
observation, volatile fencing or scheduling. The request may continue originally
approved work; the engine retains its original dispatch deadlines and ceilings.

Legacy research reused revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`,
`crates/canic-host/src/component_operation/{workflow/mod.rs,ops/mod.rs,tests.rs}`,
and the previously reviewed operation history
`4176ee97405185bf4780b5ada0f9206ead750313` /
`0509ec870891515923f79d3932ece127b351cd15`. Preserve its immutable operation,
lock, retained intent and lost-response cases. Reuse the rewrite's existing
signed-attempt owner and evidence validation; omit legacy pool/capacity workflow
machinery. A failed read remains unknown, never permission for a replacement.

Recovery has a retained ten-minute submission ceiling, independent of effect
authority. Its exact signed request precedes submission and is reused on resume;
expiry without a terminal reply remains blocked. It shares the existing 32
ordered observation slots: each slot may retain a recovery reply before its
normal observation. No observation can overtake unresolved recovery, and recovery
cannot be inserted ahead of an already signed observation. A committed recovery
result survives a host interruption even before the next observation/receipt.
Read-only status shows its retained request state; show/watch never invoke it.

This changes the recovery wire contract: IDs 27/28 carry exact approvals, replacing
unit IDs 22/26. Stage/status bytes and stable journals remain unchanged. Old owners
reject new requests before acting; there is no legacy fallback or automatic owner
upgrade/rebinding. Existing mainnet owners have not been upgraded. Maintenance of
a trapped owner, lost creation principals, expired effects and depleted reserved
slots remain outside what this command can repair.

Focused validation passed: native host/CLI checks (including expired request
custody, retained recovery evidence, scope and cancelled/unadmitted fleets),
installed endpoint rejection/rollback, autonomous creation and interrupted child
upgrades, and the two real CLI journeys covering managed children/upgrades and
whole-fleet composition. The upgrade rejection fixture now permits already
scheduled timers to advance while checking that refused approvals cannot replace
the retained one. CLI fixtures distinguish retained recovery evidence from a
missing staging reply and confirm public watching adds no observations.

Native/Wasm and fixture Clippy, affected Rustdoc, formatting and tooling-snapshot
checks passed. Three localhost-only CLI tests needed execution outside the socket
sandbox and passed there. `make canic` installed the updated CLI on PATH. No
mainnet effects, automatic windows, publication or push were performed.

## IC Host 0.79 adoption

The requested 2026-11-14 refresh selects published FS/Process/Artifacts 0.79.0;
Text 0.75.0 is independently current. Review used IC Tooling revision
`ccea377e144f3c5212dc274ef8812995e3f64d24` and its
[Foundation ledger](https://github.com/dragginzgame/ic-tooling/blob/ccea377e144f3c5212dc274ef8812995e3f64d24/releases/foundation/CHANGELOG.md).
This release retires HTTP and the standalone Wasm inspector, neither of which
Canic selects. The published retained packages' `src/` trees match 0.78.0.
The product graph selects only 0.79; direct qualification Wasm inspection selects
Artifacts 0.79, while Testkit 0.105.0 still requires its internal 0.78 family.
No artifact types cross that boundary. Preserve the other supplier selections.

A parallel host test initially encountered journal-lock contention; its isolated
rerun passed. The shared timestamp-based test-directory helper could reuse a
path, so it now uses IC Host's atomic unique-directory allocator. This changes
test isolation only; production locking and custody policy remain unchanged.

Validation passed: 60 host tests and 39 CLI tests, profile-client compilation,
focused native Clippy, qualification growth-test compilation with the two
Artifacts versions, formatting and tooling-snapshot agreement. The dependency
delta is limited to the requested Host family. Earlier installed simulator results
are not renewed by this compile-only qualification check. `make canic` rebuilt
and installed the CLI on PATH; version and local app listing passed.

## Endpoint composition ownership decision

On 2026-11-14 the maintainer selected [Canic-owned actor endpoint composition](../design/canic-system-design.md#endpoint-ownership-and-actor-composition),
Platform-owned generic IC entrypoint macros and endpoint-independent Wire codecs.
This supersedes the proposed adoption/extension of Wire Endpoint for Canic #2.
The first local implementation now mounts Root and profile-manager management
methods with `canic_management::mount_endpoints!`. Each canister selects its
exports; one fixed management adapter binds the handlers, request limits, access
decisions and passive protocol IDs to Platform's existing hooks. It invokes Wire
directly and replaces the 14 handwritten codec functions. Application lifecycle
and readiness remain outside mounting. No new crate or dependency was added.

`make endpoint-test` builds the actual Root, normal/fault/maintenance managers and
profile worker. The installed fixture covers unauthorized and oversized frames,
malformed/wrong-ID/wrong-direction frames, exact envelope admission boundaries,
asymmetric stage budgets, public reads and replicated observation sharing the
declared IDs. Fault-only points 9/10 force the real response encoder to fail:
synchronous staging rolls back journal/artifact writes and its creation timer;
reconciliation after a real readiness call retains the previously committed
child principal, approval and spent attempts. Maintenance clears volatile
callback fencing and resumes observation/replay without another child.

The sandbox initially blocked PocketIC's localhost bind; rerunning the focused
installed fixture with local networking passed. The existing `make growth-test`
protocol and installed creation/upgrade recovery cases also passed, including
uncertain effects and infrastructure maintenance. Wasm Clippy, focused test
Clippy and management Rustdoc passed. This is local synthetic-cycle evidence;
there was no mainnet mutation. Supplier retirement and general actor declarations
or catalogue generation remain upcoming work; this adapter mounts a fixed
management surface rather than introducing a general registry.

Read-only legacy research used revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: the endpoint
[pipeline](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-macros/src/endpoint/mod.rs),
[expansion](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-macros/src/endpoint/expand/mod.rs),
[typed tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-macros/src/endpoint/typed_tests.rs)
and [raw Wire tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-macros/src/endpoint/raw_wire_tests.rs).
Path history includes `8c87ae4c`, `ccae09b4` and `aa02dc0d`; this review does
not establish which historical release qualified each execution path.
The code couples explicit contracts and access/startup decisions to generated
adapters; its tests reject missing contracts and conflicting payload declarations.
Retain those invariants and relevant regression cases. Replace its hidden Core
plumbing and raw execution generation with current Platform hooks, and preserve
the current rewrite's actual authorization/admission ordering rather than blindly
copying legacy order. The initial documentation decision ran no tests; the local
implementation evidence above qualifies the selected current paths.

## Post-push review: 0.203.0

Reviewed published commit a45a2e3f3606e03ba628524850a31ed99ab3728a against its
plan/apply/status/cancel and inventory callers, app discovery/build ownership,
and starter documentation before opening 0.203.1. No new duplicate workflow,
authority owner or stale alias was identified in that scope. Build inputs remain
app-owned; network/journal selection remains fleet-owned. Cancellation retains
revocation and cannot reset an issued effect. The two-canister limit and lack of
whole-fleet upgrades remain explicit. This is a scoped design review, not a claim
of full production qualification. Opening the next version initially changed only
metadata; the later endpoint ownership decision is recorded above.

## 0.203.0 release qualification

The complete local Linux gate, `make ci`, passed before version preparation:
formatting, snapshot/dependency agreement, native/Wasm lint and docs, host/CLI
unit tests, creation/funding boundaries, maintenance, artifact custody, direct
deployment, profile persistence, autonomous growth and Root/fleet journeys.
Two historical migration fixtures remain explicitly ignored by default because
they require separately supplied old executables; this run does not renew those
historical migration claims. No mainnet mutation is part of this release.

Registry freshness review on 2026-11-14 found every direct first-party supplier
in the product and required qualification graphs current: Host FS/Process/Artifacts
0.78.0, Text 0.75.0, Platform 0.33.3, Runtime 0.22.4, Storage 0.74.1, Timers 0.44.2,
Wire 0.22.1, Testkit 0.105.0 and Tooling 0.43.1. Preparation changes only local
Canic versions in both locks; no dependency refresh or crates.io publication.
The installed 0.203.0 CLI reports the final version, and its generated-source
build plus two-fleet simulator journey passed again (15.83s).

## Requested tooling and Testkit refresh

Adopt Toko Tooling 0.43.1 through the authenticated installer and Testkit 0.105.0
in qualification. Review Testkit's published source revision
`9378ecb023aaff406a492d3f9bf18dd58bdb8221`: its baseline-pool removal affects no
Canic caller, which already owns fresh instances and explicit close. Its Host
0.78 dependencies replace the private 0.77 copies and remove `command-fds`/`nix`.
No other supplier refresh is required. A concurrent Platform Types 0.20.4 lock
selection was preserved; final qualification uses that combined graph.

The tooling preview retained all executable payloads and tool catalogs unchanged;
the updated baseline adds the changelog quick-start requirement, now reflected
in the pending entry. Installation, prepared-tool admission, snapshot/dependency
agreement and formatting passed. All M0/M1 Testkit callers passed Clippy, and
the installed whole-fleet recovery/isolation journey passed in 13.98 seconds
with Testkit 0.105.0. These checks cover local Linux simulator use; no mainnet
deployment or release is part of this refresh.

## Starter workflow simplification

The starter now has one app build command and one bounded discovery catalog.
Generated app definitions own workspace/package recipes and replaceable Wasm
outputs. The artifact builder remains the sole compilation owner, uses the
selected workspace working directory, builds one library with --locked, and
retains no deployment authority. App build prepares an absent lockfile once;
existing locks survive repeated builds. Source generation defaults to the CLI's
matching checkout, with an explicit source override still available.

Research reuses legacy configuration and workflow evidence below at
377fbceb3994c9d1f3033979ef4e4ce9e6b85801, plus canister_build/{mod.rs,tests.rs}.
Build history 95bdb4ed61aaead6bddfc4a68f6e509063106f64 and its
wire_schema/build/{mod.rs,tests/mod.rs} change distinguish package/workspace
ownership from invocation context. Retain explicit package ownership, locked
compilation and supplied Root code. Omit the old cache/release-set/schema
machinery: this consumer needs two ordinary Wasm outputs, not that framework.

Listings and selectors share current/direct-child config discovery, matching
declared identities and refusing duplicates. Discovery is bounded (1024 entries,
64 configs), skips generated/hidden directories and child symlinks, and never
searches ancestors. Creation still targets the explicit/default config rather
than redirecting into a discovered project. Explicit config selection wins.

Cancellation is local revocation under the existing journal lock, permitted only
while Root/child directories contain the original approval inputs. Any effect
preparation, even an incomplete attempt directory, blocks cancellation. Keep the
original approval and revocation; old apply always refuses. Active inventory and
reservation checks omit revoked approvals but validate their retained evidence.
No timer, signature or remote management slot is reset. This preserves legacy's
unknown-outcome rule while giving genuinely unsubmitted expired plans an exit.

Planning may read a qualified ICP identity's public principal; approval still
binds the exact principal. Human output exposes UTC deadlines, budgets and
scope-preserving continuations. The default signer selection has one CLI owner.
The main tutorial now uses whole-fleet deployment; the duplicated individual
bootstrap recipe and stale pending-deployment claims are retired. Docs distinguish
replaceable builds from config/journal custody and external identity storage.

Evidence: 59 Host unit tests, 12 CLI context tests, two continuation/authority
argument tests, Host/CLI Clippy and Host Rustdoc passed. The installed whole-fleet
simulator test passed in 18.67s, including generated-source defaults, app builds,
lock preservation, identity-based planning, two fleets, lost-reply recovery and
replay without another signing attempt. The installed CLI smoke check also
selected a nested app whose directory differed from its declared name, cancelled
and replaced a pristine approval, and refused applying its revoked digest.
Anonymous mainnet planning was correctly refused. No mainnet deployment or release is part
of this cleanup.

## First whole-fleet approval

The first slice composes exactly one new Root and its new app. The fleet envelope
binds their names, both artifacts, network/operator, Root debit and child attachment,
and fixed dispatch/deadlines. A typed `CreatedCanister` reference names the Root
creation digest. Only its retained result and matching completion receipt can
materialize the child's immutable plan. The fleet digest uses bounded Wire;
existing single-canister plan encodings and signed requests are unchanged.

Research at legacy `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` reuses
`component_operation/{workflow,ops}/mod.rs`, `component_operation/tests.rs` and
`component_topology/mod.rs`. The workflow history at
`4176ee97405185bf4780b5ada0f9206ead750313` and parent
`0509ec870891515923f79d3932ece127b351cd15` retains the same-operation resume
contract while changing timer ownership. Preserve immutable review authority,
intent before submission, correlated outcomes, failed reads as unknown and
parent-before-child resolution. Do not import pools, Coordinator/Store roles or
a generic transport/workflow framework for this two-step consumer.

One journal lock covers publication and apply. Atomic directory publication
retains both artifacts and the complete fleet approval before effects. Root and
child use the existing effect/signing handlers; child materialization resumes
from Root's retained output without changing the review. Replanning never renews
authority. A new Root cannot start after child admission closes. Already-issued
Root work may settle under its original final deadline; if child admission is
then too late, the fleet remains incomplete with the funded Root retained.
Uncertainty never grants a second creation or a replacement schedule.

Status and call select the composed operation; fleet inventory projects its
deployment steps with the shared status/apply address. Views remain read-only,
and reported Ready does not create a completion receipt. App definitions and
fleet bindings remain unchanged by deployment. The generated README now uses
the fleet workflow. Old individual operation guides remain for those deployments.
This is new-fleet creation and replay, not arbitrary desired-state convergence:
larger trees, adoption, fleet upgrades and cascading discovery remain outside
this qualified slice.

Local qualification passed: 55 Host and 37 CLI unit tests, Host/CLI Clippy,
Host Rustdoc and both workspaces' formatting. The installed whole-fleet CLI
test deployed two isolated fleets, recovered a lost admission reply, retained
completion and replayed without another signer invocation or creation (14.16s).
The existing individual creation/upgrade CLI regression also passed (17.43s).
Final inventory review preserved partial results when a fleet journal is busy;
the focused fleet-approval and partial-inventory regressions passed after that
correction. `make canic` builds the current CLI. These are local simulator
results, not mainnet qualification; this increment remains unreleased.

## 0.202.0 release scope and supplier review

This release contains the completed one-child upgrade, live progress, retained
failure diagnostics and CLI cleanup. Whole-fleet plan/apply, cascading discovery
and CLI owner recovery remain the next M2 work; no partial fleet executor is
added to meet a version deadline. The explicit finish line is in the
[delivery plan](../design/canic-system-design.md#16-delivery-plan-with-deletion-gates).

Adopt published IC Host FS/Process/Artifacts 0.78.0 after reviewing the
[Foundation release](https://github.com/dragginzgame/ic-tooling/blob/dcd7159e5cbb5df24d209c8ff4f8306f51a1b2cc/releases/foundation/CHANGELOG.md).
Canic uses none of its removed APIs. Product FS/Process and direct qualification
artifact inspection select 0.78; Testkit 0.104.1 retains private 0.77 dependencies.
Those native types are not exchanged at Canic's artifact boundary. Text 0.75.0
is current independently. The product graph drops `command-fds` and `nix`.

The 2026-11-14 registry freshness review also found the locked Platform 0.33.2,
Runtime 0.22.3, Storage 0.74.1, Timers 0.44.2, Wire 0.22.1 and Testkit 0.104.1
current. Toko Tooling 0.43.1 is available; retain the verified 0.43.0 snapshot
and defer that optional refresh to the next tooling-maintenance batch. The
requested adoption is IC Host; no unresolved compatibility blocker is asserted.
Existing Platform and Testkit lockfile updates are preserved and qualified with
this release rather than attributed to the Host update.

The full Linux `make ci` gate passed on this selected graph: native/Wasm Clippy,
Rustdoc, formatting, tooling agreement, 52 Host tests, 37 CLI tests and all
installed creation/funding/maintenance, artifact, deployment, profile, growth
and Root journeys. The installed two-fleet CLI scenario passed in 17.97 seconds.
Both explicitly selected retained-binary migration cases also passed (6.86
seconds), using the previous Root and manager artifact hashes recorded below.
Raw logs remain in `target/evidence/0.202.0/`. No macOS or mainnet qualification
is implied by this Linux release gate. Crates.io publication remains disabled.

## Retained Root failure diagnostics

The CLI status projection now decodes the existing management Wire types to
show dispatch counts, retained request-hash presence, child/refund custody,
upgrade versions and failure reasons. Its direct dependency is the narrow
management protocol, already present transitively; no runtime graph or external
supplier is added. Host JSON, journal bytes, effect authority and recovery
mechanics are unchanged. Unknown evidence remains distinct from zero or success.

Reuse the pinned legacy progress renderer and tests described under
[readable retained-operation status](#readable-retained-operation-status), at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, including the inspected
`afc2521ed02744c1e1a376d59dae5e2b80d9dec5` history. Keep the unknown-readiness
and exact-request-correlation invariants; no live progress engine is needed
inside the local status renderer.

Guidance separates observation, original-operation completion and owner recovery
review. An issued effect is not retry permission. Expiry does not prove absence;
reserve funding does not reset a consumed slot. Missing creation output remains
unresolved, and Ready without a receipt still requires current metadata checks
through apply. Owner maintenance/reconciliation has no qualified CLI command yet.
This increment improves diagnosis; it does not close that execution gap.

Validation passed: six focused CLI status tests, CLI and Root-fixture Clippy,
formatting, Toko Tooling snapshot/dependency agreement and the installed two-fleet
CLI scenario (17.93 seconds). The scenario checks human status before admission
evidence, during scheduled work and after completion, with unchanged retained
JSON before and after each read. The CLI was rebuilt with `make canic`; runtime
code and stable payloads are unchanged by this diagnostic increment.

## Read-only child-upgrade progress

The next increment extends the existing fleet collector and terminal/HTML/watch
projections. A retained upgrade plan selects its Root by exact network and
principal. Only those owners receive `upgrade_status` queries; older or unrelated
fleets without an upgrade plan receive none. A report must be empty before
admission or match the full approved manager, child, old/new code and schedule.
Missing, mismatched, ambiguous or timed-out reports remain unknown. Creation and
upgrade share the existing five-second owner budget and thirty-second snapshot
deadline; neither failure erases the other's available evidence.

Reuse the pinned legacy component-operation/regression and progress-rendering
research recorded below at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, including
the inspected `4176ee97405185bf4780b5ada0f9206ead750313` and
`aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f` path history. Preserve request/approval
correlation, separate reported progress from completion custody, and keep reads
out of the effect owner. No generic progress engine or second watcher is needed.

Upgrade reports never add identities, change slot bindings or publish receipts.
When an upgrade is the only retained operation, its child and previous code also
constrain any creation report before that report can add a child to inventory.
Native regressions cover all approval fields, exact-network selection, absent/
exhausted/ambiguous observation, escaped errors and unknown upgrade presentation
alongside available creation. The installed two-fleet CLI fixture observes Empty,
Scheduled and Ready, proving that no admission, replicated observation or receipt
is written by fleet viewing. Authenticated apply still settles completion.
Validation passed: 52 Host and 34 CLI tests, focused native/Root-fixture Clippy,
Host Rustdoc, formatting and the installed two-fleet CLI scenario (17.85 seconds).
The CLI was rebuilt with `make canic`. Runtime code and stable payloads are
unchanged by this observation increment, so its creation/upgrade fault fixtures
were not rerun.

The batch is named 0.202.0 because the earlier Host API renames and one-way
runtime payload migration change consumer contracts. Release preparation follows
the qualification above; the observation increment itself did not deploy any
canisters. Existing dependency lockfile edits were preserved, with the later
requested IC Host adoption qualified in the release gate.

## First Root-owned child upgrade

The next narrow journey admits one upgrade of the already-created child. It
uses the existing CLI plan/apply/status and signed-attempt owner. The plan binds
the creation receipt, child principal, old/new module hashes, Root code and
operator, schedule and deadline. A separate permanent upgrade slot retains its
own approval, two dispatch commitments and old/new canister versions. Creation
authority, output identity and artifact bytes remain immutable. No slot reset,
general workflow engine, automatic install retry or repeated-upgrade history is
introduced. Application data preservation belongs to its lifecycle.

Legacy research at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` inspected:

- [component operations](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/ops/mod.rs)
  and [regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/tests.rs):
  immutable review identity, intent before submission, drift refusal and retained uncertainty.
- [installation workflow](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/runtime/install/mod.rs),
  `crates/canic-core/src/ops/ic/mgmt/lifecycle.rs` and
  `crates/canic-core/src/infra/ic/mgmt/lifecycle.rs`: exact approved bytes, target,
  chunk store and permit boundary. That pinned install path selects Install;
  it is not evidence of a working persistent child-upgrade flow.
- [lifecycle boundary tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/qualification/canic-tests/tests/lifecycle_boundary.rs):
  failed post-upgrade rollback, retained preparation and foreign-release refusal.
  Carry forward the preservation invariant, without copying test retries or
  byte-patched executable fixtures into the new effect owner.
- Path history `aed32be2afbaf66d7e2867292da52f75ce028e41` moves installation
  initialization from generic Candid values to explicitly encoded protocol bytes.
  Keep exact encoding at the effect boundary; the narrow profile/greeting upgrade
  needs no legacy release catalogue, permit engine or generic lifecycle participant.

Dispatch first observes old code, sole Root control and child version. Upgrade
intent and outgoing `install_chunked_code` enqueue share one local commit boundary.
Successful callback evidence commits separately; later observation requires new
code and exactly one version increment before readiness. If the callback is lost,
controller reconciliation can reconstruct that outcome without another install.
A rejected application post-upgrade leaves the old application usable while the
upgrade slot remains blocked with one issued attempt. Host recovery uses only
the matching replicated `observe_upgrade`; fresh read authority cannot dispatch
effects or extend the original deadline.

The first migration experiment rejected extra storage allocations: IC Storage
0.74.1 intersects reopen limits with the directory capacity fixed at creation
(`runtime/directory_model/persistence/mod.rs`). Increasing `StorageConfig` cannot
expand that directory. The final layout retains the original two checked keys,
three-extent capacity and 64-page bound. Versioned payloads pack both journals
and both bounded artifacts inside them. Lifecycle strictly decodes/validates the
old schema before committing the v2 payload; existing Wire messages are unchanged.
There is no backing-directory rewrite, second owner or silent reinstall.
Older binaries cannot read the new payload; downgrade is not qualified.

Installed qualification covers normal profile upgrade, upload callback loss,
pre-install dispatch trap, install callback loss and rejected application upgrade.
Every path retains private profile ownership/data, original creation custody and
at most one installation attempt. Owner maintenance preserves the upgrade journal.
The CLI fixture replaces one greeting without affecting the second fleet, recovers
a lost stage reply, retains both receipts and replays completion without signing.
Native evidence checks reject missing version proof, changed request commitments,
foreign child bindings and regressing observations.

Migration also passes from retained Root SHA-256
`691b6e1cfd14498be542ca98e8f4bf379c2912ebaada548c8c5b53f293ea5a08`
after completed creation, and profile manager
`4fd7e504d88e4f0139a834f3b27f8aa027edb9a6a861284669411a37f0b0d1c4`
while scheduled. Reproduce with the two ignored `previous_` Root fixtures and
`CANIC_PREVIOUS_ROOT` / `CANIC_PREVIOUS_MANAGER` naming those retained Wasm files.
Both passed in 7.03 seconds. The normal growth plus upgrade boundary fixtures
passed in 13.49 seconds; the manager uses 18 stable pages after creation.
Final checks passed: 48 Host, 33 CLI and two management-protocol tests; both
normal Root fixtures (15.92 seconds); focused native/Wasm and qualification
Clippy, native/Wasm Rustdoc and repository formatting. `make canic` rebuilt the
local executable. The two retained-binary migration checks ran explicitly,
despite remaining opt-in/ignored in the ordinary Root target.

At this stage the live Root report covered creation only; the read-only progress
increment above adds separately matched upgrade reports. Recovery from engine callback traps
is qualified through controller Wire calls and owner maintenance, with no CLI
maintenance command yet. One successful upgrade is not an arbitrary migration,
subtree operation, repeated-upgrade product or production persistence claim.
No mainnet mutation or release accompanies this local increment.

## 0.201.0 release review

The release review covers app-owned definitions versus fleet bindings, canister-only
trees, shared management ownership, and the separation of public Root reports from
authenticated completion evidence. The growth fixture's obsolete report-heading
assertion now checks the explicit uncertified Manager label.
The installed Root CLI fixture pauses automatic simulator progress around manual
time advancement, preventing its clock from racing the live gateway.

The follow-up cleanup renames Host's phase export to `ManagementPhase` and updates
all callers without a compatibility alias. App listings now feed app-owned names
and parent links directly to the tree renderer; fleet views project those same
links from their resolved snapshot. The layout helper no longer requires fleet
selection, network bindings or controller data. Generic routes use source and
destination labels, without assuming Profile behaviour or private-data ownership.
Clap declarations and their argument conversion live in `args.rs`, separately
from command execution; command names, selectors and help ordering are unchanged.

This reuses the pinned legacy configuration/topology and CLI research below at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`: preserve immediate parents, separate
app definitions from fleet bindings, and reject ambiguous command context. The
cleanup changes presentation and source ownership, not effect or persistence
contracts. Existing tree, partial-inventory, route-admission and CLI-context
regressions remain the relevant checks.
Validation passed: all 33 CLI tests, Host/CLI Clippy, Host Rustdoc and repository
formatting checks. No runtime engine changes required another simulator run.

## CLI bootstrap and two-fleet qualification

The installed starter fixture now creates Root through the same public plan/apply
path used by operators. Previously it installed Root directly through PocketIC
and configured its ID before testing the child. Only the local subnet routing
canister remains fixture setup. The child now resolves its Root from the completed
named receipt, and successful deployment leaves config identity bindings empty.
The separate configured-Root authority-drift cases remain covered.

Reuse the pinned Root-bootstrap and component-operation research below at
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, plus the app/fleet configuration
research: retain creation custody before using a parent output, reuse approved
operations, and keep fleet membership distinct from shared app definitions.
No new deployment owner, config format, command or lifecycle mechanism is added.

The same generated app then deploys into a second fleet on the same local network.
The fixture proves distinct Roots and children, separate default journals and
observations, unchanged first-fleet evidence, and refusal of cross-fleet apply
before signer invocation. Replanning and replay retain the original operations;
the subnet contains exactly the routing canister and the four fleet canisters.
Lost staging reply and completion receipt recovery remain in this installed path.

Validation passed: focused Root-fixture Clippy and the installed CLI scenario
`managed_child_cli_retains_admission_and_recovers_lost_reply` (14.20 seconds).
The first sandbox attempt could not bind localhost; the permitted simulator run
passed. This uses the unchanged generated-app and Root build artifacts, synthetic
cycles and the rebuilt CLI. Mainnet deployment, persistent child upgrades and
whole-fleet convergence are not qualified by this check.

## Read-only Root progress

The next 2026-11-14 increment closes the observation gap between Root admission
and retained completion. The existing fleet collector now selects each planned
Root by exact network binding and principal, alongside explicitly selected
profile managers. Terminal output, saved reports and localhost watch reuse that
collector; no new command, journal or polling owner is introduced. Automatic
selection stays network-scoped even when two replicas reuse the same principal;
explicit principal-only selection still refuses that ambiguity. The combined
selection remains capped at sixteen owners and the existing snapshot deadline.

Reuse the same pinned legacy component-operation and progress-rendering research
recorded in the managed-child and readable-status sections below, including the
request-correlation and unknown-readiness regressions and their inspected path
history. Preserve their separation of reported progress and terminal evidence.
A public report must match the retained approval before its child enters the
observation inventory. Mismatches stay unknown; an empty pre-admission Root is
allowed. Neither reported readiness nor independently observed child metadata
establishes retained child custody, changes slot bindings or writes receipts.
Trees still contain canisters only; external controllers remain metadata.

Native tests cover exact network selection, explicit/automatic deduplication,
selection budgets and refusal of foreign approvals and self-child reports.
The installed CLI fixture observes Scheduled then Ready without applying,
checks both terminal and HTML Root labels, and proves the child's retained
identity and receipt remain absent until authenticated completion. Watching
creates no signed observation attempt and spends no new management allowance.
The simulator uses synthetic cycles and opens no browser. This is bounded
single-level Root observation, not general cascading subtree discovery.

## Managed-child CLI approval

The next 2026-11-14 increment connects the generated app to the supplied Root
through the existing fleet plan/apply/status workflow. Explicit `--root-module`
selects Root-owned creation; parent declarations alone never authorize it.
Resolve the immediate parent from a configured identity or a completed named
receipt, reject ambiguous/conflicting identities and refuse multiple children
for this one-slot owner. Each apply checks certified Root code and sole operator
control. The child receipt and fleet projection name Root as its controller.

Research at legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` inspected
[component operation workflow](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/workflow/mod.rs),
[operation regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/tests.rs)
and [transport selection tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/ops/transport/tests/mod.rs).
The path history includes `4176ee97405185bf4780b5ada0f9206ead750313`
(native timer ownership) and `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`
(the aligned supplier hard cut). Carry forward intent before submission,
immutable review identity, incomplete successful progress, authority-drift
refusal and monotonic output custody. The narrow Root needs no legacy release
registry, capacity pool or generalized component-operation journal.

The existing typed plan adds an optional management binding; absent bindings
serialize exactly as before. Golden checks preserve retained direct v1/v2
plan digests; no new format version, migration or parallel reader is introduced.
The single retained plan owns Root identity/code, child bytes, signer, exact 1T
attachment and schedule. New schedules default to ten minutes after planning,
with an effect deadline ten minutes later. Reopening a named slot returns its
original authority, including a past schedule; it cannot renew an allowance.
A declared child refuses fresh direct-host creation. Existing direct operations
remain recoverable by their original digest even if the declaration changes.

Staging uses the existing exact signed-attempt owner. A successful pending result
is separate from completion. Root adds a controller-guarded replicated
`observe` endpoint that only reads committed status. Its retained reply can
recover lost staging evidence without re-signing admission. Only that exact
read method, recipient and payload may receive fresh signing authority after
the effect deadline; it cannot reconcile or dispatch effects. Public queries do
not establish created-principal custody. Observation attempts are bounded at 32;
an uncertain expired observation stays blocked. Child identity, attempts,
request commitments and outcome evidence cannot regress. Root readiness plus
certified child module/controllers are required before retaining a receipt.

Focused host/CLI tests cover planning, exact replay, changed authority, output
substitution, signer scope and preserved direct-plan encodings. Installed CLI
qualification creates the generated config, binds a fixture Root, plans and
admits its app, loses the staging reply, reconstructs admission through a
replicated read, observes one ready child, projects its fleet membership and
recovers a missing completion receipt without invoking the signer. Existing
Root maintenance qualification remains passing. No mainnet mutation or browser
launch occurred. Whole-fleet convergence, child upgrades, arbitrary subtree
operations and general infrastructure maintenance approvals remain separate.

## Generated starter implementation

The next 2026-11-14 increment extends the existing `app init` action with
`--source DIRECTORY --canic-source CHECKOUT`. It generates a new standalone
workspace, its config, an editable greeting application, and a declaration for
the supplied Root. The default initialization path still edits declarations for
existing code. Source generation cannot be combined with `--config`; it owns the
new project's config rather than editing a second existing document.

Reuse the pinned legacy scaffold implementation, tests and canonical-Root/source-
alias history inspected in the Root section below. Carry forward refusal of
occupied targets and validation before writes. Replace the legacy multi-file
workspace mutation/rollback with one complete directory publication through
Host FS: the new project's parent must exist, staging is private, publication
cannot replace an existing name, and failures retain source/error/path evidence.
The generator does not edit the containing Cargo workspace or current fleet
config. Tests cover malformed names/source, existing and symlink destinations,
source aliases, shared config semantics and competing initializers.

The selected Canic checkout must match the CLI version and expose the supplied
Root and greeting contract packages. Resolve its physical path before writing
dependency and build references. The generated workspace inherits its selected
compiler, supplier declarations and lint policy; it keeps publication disabled.
The user prepares and retains the generated lockfile explicitly. Generation
does not fetch dependencies, invoke Cargo, create journals or authorize effects.
No crates.io Canic package or copied Root runtime is inserted into the app.

`artifact build --output WASM_FILE` atomically publishes a replaceable build
input after selecting and admitting the Cargo artifact. The generated config's
`artifacts/root.wasm` and `artifacts/app.wasm` paths resolve beside that config.
Existing planning continues to retain immutable bytes under its original owner;
build outputs are not deployment receipts or new runtime authority.

The focused host/CLI tests and installed Root fixture passed. The latter now uses
the actual CLI generator and builder, prepares the generated lockfile offline,
and installs the generated greeting under Root on PocketIC with synthetic cycles.
It retains the controller, admission, maintenance and replay checks from the
previous Root increment. Source generation and managed-child CLI approval are
implemented; whole-fleet convergence remains pending. No mainnet mutation or
browser launch occurred. See the [starter commands](../demos/root.md).

## One-child Root implementation

On 2026-11-14, the next local implementation supplies `canic-root` for one
greeting application. The canister tree is `Root → application`; Root's external
operator is controller metadata, not a canister node. This is a bounded Root
implementation, not generated source or whole-fleet convergence.

Legacy research used revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`:

- [Canonical Root materialization](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/canonical_root/mod.rs)
  and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/canonical_root/tests.rs)
  supply the ownership lesson: application generation must consume Canic's
  supplied Root rather than fork its management implementation into each app.
- [Root bootstrap](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/bootstrap/root/mod.rs)
  establishes synchronous custody before deferred work and withholds readiness
  while required releases/activation evidence are absent. Retain that ordering;
  the selected one-child workload needs neither its Store nor its Coordinator
  and capacity-import machinery.
- [Scaffold implementation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/scaffold/mod.rs)
  and [regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/scaffold/tests.rs)
  refuse existing targets and validate planned writes before mutation. Preserve
  these cases when generation is implemented; they are not proved by this runtime
  increment.
- [History f2f88e1](https://github.com/dragginzgame/canic-legacy/commit/f2f88e1c89c38629aaff0a299f9e32c373dea1b0)
  fixes canonical Root identity through retained source aliases. Future source
  materialization must bind to the resolved config identity; this increment
  builds explicit packages and adds no source-discovery mechanism.

Extract the already installed profile manager's one-slot engine into
`canic-management`, with Wire/evidence types in `canic-management-protocol`.
The profile manager and Root each mount lifecycle and endpoints explicitly and
supply their own bounded readiness decoder. The engine owns effect issuance,
retained artifact bytes, journal transitions and reconciliation. Preserve RPC
numbers, field/enum order, stable-memory keys and timer identity. Move every
consumer to the single protocol owner without a compatibility re-export. The
native host graph no longer depends on the profile application's protocol.

`make root-test` passed with CLI-built Root and Hello World Wasm, synthetic cycles
and an external identity controlling Root. Qualification proves unauthorized
admission refusal, an upgrade while the original approval is scheduled, one
created child with Root as sole controller, observed module binding, a greeting,
exact replay and changed-approval refusal. `make growth-test` passed all five
interruption windows, expiry/reserve refusal, artifact custody and maintenance
against the extracted engine. Lost creation evidence remains blocked, not retried.

An additional installed upgrade from the retained pre-extraction profile manager
Wasm (`4fd7e504d88e4f0139a834f3b27f8aa027edb9a6a861284669411a37f0b0d1c4`)
preserved its admitted authority and artifact, resumed creation and reached
profile readiness through the new engine. Reproduce that evidence with an exact
retained previous artifact using `CANIC_PREVIOUS_MANAGER` and the explicitly
selected ignored test `previous_profile_manager_preserves_admitted_work` in
`qualification/m1-tests/tests/root.rs`; the default Root test needs no historical
binary. The shared protocol frame/budget regression moved with its owner.

Root/management and affected native/Wasm callers passed Clippy; Root/management
Rustdoc passed. No mainnet canister, signer, controller or balance was changed.
Source generation and managed-child CLI approval shipped in 0.201.0. Persistent
application upgrades through Root and whole-fleet CLI convergence remain upcoming. See the
[hands-on Root guide](../demos/root.md).

## Reusable app definition implementation

The app-definition portion of 0.201.0 implements shared app definitions, independently
of the Root and source generator. `apps.APP.canisters` owns roles, parent relationships
and optional Wasm input paths; each fleet owns only network, journal selection
and existing-ID bindings. A new fleet projects the app's complete declared layout
without copying another fleet's IDs. The existing plan/apply and observation
owners consume a resolved snapshot, and retained deployment-plan formats and
digests remain unchanged.

`app canister add/list` edits and inspects the shared model. `fleet canister
bind/list` binds existing identities for observation and lists the resolved fleet.
The retired fleet-local add command has no alias. The config is unversioned and
rejects the previous layout; [README migration instructions](../../README.md#initialize-an-app-and-fleet)
describe the explicit external edits. This checkout retains all original
principals, network selections and `.canic` journal paths. Old source/layout
examples later in this evidence record describe their historical increments.

Research reuses the pinned declaration mutation/config regression and history
review below, plus legacy
[topology finalization](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/mod.rs)
and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/tests.rs).
Retain shared logical definitions versus explicit deployment facts, deterministic
projection, bounded validation and no fabricated principals. Do not import the
general Spec catalogue, warm pools or Coordinator dependency to describe the
current two demos. Static parent declarations still grant no spawning authority.

Focused regressions cover two fleets sharing one definition with independent
bindings/plans/journals, malformed/conflicting bindings, no fleet-level topology
or artifact overrides, duplicate names, complete config limits, unchanged files
on refusal, concurrent initialization, locks/symlinks, config-relative Wasm paths,
source edits refusing new authority for occupied slots, and retained v1/v2 plan
recovery. The installed M1 journey uses the new declaration command while retaining
the same create/install/call/upgrade effect path. Generated source, generalized
management ownership, whole-fleet convergence and runtime cascades are still
separate unimplemented outcomes; this increment does not qualify them.

Validation: 22 focused host config tests, nine CLI configuration/context tests,
Host/CLI Clippy and Host Rustdoc passed. The rebuilt CLI passed the installed
PocketIC create/call/replay/receipt-recovery/same-canister-upgrade journey. Its
first launch was blocked by sandbox localhost binding; the rerun with localhost
access passed. Formatting and tooling agreement passed. Actual checkout app/fleet
listings preserve the profile tree and IDs. No mainnet mutation or push occurred.

## Cascading and workflow scope review

The maintainer's 2026-11-14 answers admit a reusable app definition, a generated
Rust starter with one Root-controlled application canister, persistent local
development, whole-fleet convergence, composable scaling/sharding, cascading
discovery and project-subtree operations. ICP conversion is excluded; operators
supply cycles manually. Application frontend work is deferred. The simple
handover default retains workspace evidence and separately provisions signer
authority; journal-free disaster recovery is not promised. These are accepted
design requirements, not shipped CLI capabilities. The canonical scope is in
[architecture section 1](../design/canic-system-design.md#accepted-developer-and-operator-journeys).

Evaluation used legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`.
The recommendation is to adapt the behavior through narrow owners, not move the
old Core, Coordinator/Registry and Store dependencies wholesale:

| Legacy evidence | Decision for the rewrite |
| --- | --- |
| [Topology cascade](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/cascade/topology.rs), [validation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/ops/cascade/mod.rs) and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/ops/cascade/tests/mod.rs) | Preserve exact receiver/role/parent checks, branch slicing, explicit leaf rows and refusal of missing/duplicate/foreign children. Adapt allocation-generation binding; do not add a Store just to retain its topology endpoint. |
| [Component runtime discovery](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/component_runtime/mod.rs) and [regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/component_runtime/tests/mod.rs) | Preserve local validated lookup, all exact role matches in member-path order, exact replay, monotonic directory progression and conflict refusal. Discovery is not permission or a live health check. General group/service catalogues are not prerequisites. |
| [Directory synchronization](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/component_directory_synchronization/mod.rs) | Preserve intent before dispatch and exact target coverage before marking synchronization complete. Replace required Coordinator/Fleet Registry ownership with the selected management owner's committed allocation projection for the initial single-subnet journey. |
| [State cascade](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/cascade/state.rs) and [bounded report regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/ops/cascade_report/tests.rs) | Preserve local application versus descendant completion, per-target uncertainty, reconciliation failures, complete counts with bounded detail and no mutation on malformed aggregation. Use durable operation generations and bounded resumable scheduling rather than copying one recursive awaited call chain. |
| [Installed partial-failure/retry fixture](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/qualification/canic-tests/src/pic/fleet_registry/baseline/tests/state_cascade/mod.rs) | Bring forward wrong-caller refusal, immediate-parent enforcement, stopped descendant with successful sibling outcomes, unchanged-state retry completing propagation, and recovery endpoints during logical pause. Add selected-subtree isolation and overlapping-transition tests. |

History matters here: [commit c04cd56](https://github.com/dragginzgame/canic-legacy/commit/c04cd56c11d287cf5baebb472baac61706434dc5)
replaced unit/first-error cascade results with bounded per-target reports,
role-correct transport and a recheck before later sends. This addressed loss of
partial outcome information; it was not cosmetic abstraction. The pinned native
Wire implementation retains those lessons. Its
[endpoint admission tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/domain/policy/pure/fleet_activation/native_cascade_admission_tests.rs)
also show that the topology endpoint is Store-specific while managed components
receive runtime configuration. Therefore "move cascading" must include both
directory configuration and state propagation, not just the topology module.

Changed assumptions and unproven paths:

- Legacy's pre-send state-value comparison is useful but does not establish a
  durable generation fence across pause/resume/pause or out-of-order delivery.
  Bind commands and replies to generations and preserve pending work across upgrades.
- The inspected installed fixture starts at a Root; it does not establish
  arbitrary project-subtree selection, overlapping ancestor/descendant changes,
  child creation during a pause, or atomic subtree upgrades. Qualify those directly.
- The transport uses an unbounded response wait and sequential fanout. A stopped
  child returning failure is not proof of progress past an indefinitely pending
  call. Select and qualify bounded scheduling/in-flight exposure separately from
  each effect's transport wait contract.
- A configuration tree is currently descriptive. Do not reuse its `parent` field
  as endpoint authorization or management authority without an installed binding.

No legacy tests were run or copied for this documentation-only evaluation. Their
source supplies regression cases for implementation, not new qualification.
Search of current `crates/` and `apps/` found no ICP conversion implementation to
delete. Removed its prospective effect contract and scope entries; preserved
historical external conversion receipts, public wallet observations and required
cycle-provider accounting. No mainnet authority or configuration changed.

## Domain and configuration review

On 2026-11-14, the maintainer identified an implementation that put apps inside a
fleet, contradicting the existing definition of a fleet as one application
instance. The implementation and declaration were corrected to **App → Fleets →
Canisters**. This review compared domain/configuration ownership, host planning,
operation selection and recovery, inventory projections, and the narrow manager
custody graph with the architecture. It is not a full security audit of every
supplier or a production qualification claim.

| Finding | Resolution and witness |
| --- | --- |
| Hierarchy and deployment boundaries were implemented inconsistently | App/fleet selection, independent default journals and shared-journal filtering; tests cover two fleets of one app and a second app |
| Upgrade planning could retain an unrelated target before apply refused it | Refuse outside-fleet targets before writing the plan; installed CLI still upgrades an admitted target and preserves recovery |
| One physical identity could be declared in two fleets on one exact network binding | Reject conflicting membership across the declaration; equal principals on different trust bindings remain valid |
| Duplicate JSON map keys silently replaced declarations | Reject duplicate app, fleet and canister keys before selection |
| Speculative command names and stale implementation claims obscured the actual product | Keep the shipped vocabulary canonical and distinguish the one-slot manager from a general executor/Root; remove the obsolete claim that no implementation exists |

The architectural distinctions remain: local names do not grant controller
authority; config edits cannot rewrite approved requests; a shared operator or
executable does not merge fleet budgets; a report does not establish readiness or
authorize a retry. The current declarations are local selection, not an on-chain
membership registry. Rename/transfer, general activation bindings, distributed
fencing and production restore require their own admitted contracts.

The existing dispatch/callback distinction and wait-policy exposure remain
consistent with the reviewed [IC system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/)
and [execution properties](https://docs.internetcomputer.org/references/message-execution-properties/).
This source review does not extend the previously qualified provider windows.

Validation: 24 host tests, 15 CLI tests, the installed PocketIC deployment/upgrade/
recovery journey, M1 Clippy/Rustdoc, formatting and tooling agreement passed.
Regression cases live in [host config tests](../../crates/canic-host/src/config/tests.rs),
[CLI config tests](../../crates/canic-cli/src/config_tests.rs) and the
[installed journey](../../qualification/canic-qualification/tests/journey.rs). Mainnet
canister code and operation evidence were not changed by these fixes. Comparative
adoption measurements and broader production lifecycle remain unqualified; this
review does not close M0 or admit the optional capability catalogue.

### Legacy evidence applied to configuration and terminal output

The named-directory lookup reuses this pinned configuration/history research and
rechecks legacy `config_discovery/{mod.rs,tests.rs}` at the same revision below.
Retain explicit selection, bounded discovery and validation of declared app
identity. Listings and selection now share the current/direct-child config
catalog; the earlier named-directory fallback is retired. Explicit config paths
always win. Duplicate identities require explicit selection. The recursive legacy
workspace scanner and ancestor search are unnecessary here. Malformed discovered config blocks selection;
an app name cannot contain path traversal, and a directory name never substitutes
for validated config contents. All CLI contexts share the Host resolver; relative
artifacts and journals continue to belong to the selected config.
Validation passed: 23 Host config tests, 12 CLI context tests, Host/CLI Clippy,
Host Rustdoc and formatting. The installed CLI also resolved a generated starter
beside an unrelated current config for listings, fleet initialization, app edits,
planning and status, while preserving the current config and selecting the
starter's journal. Explicit `--config` still refused the wrong app. No apply or
network mutation was part of that lookup check.

The 0.200.1 configuration increment was compared with committed legacy revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`. This is a scoped source and history
review, separate from the earlier working-tree inventory in the architecture.

- [Declaration mutation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/release_set/config/mutation/declare.rs)
  checked app identity, rejected duplicate names and validated the resulting
  configuration before publishing it. Current app/fleet/slot edits retain those
  protections. The legacy package bindings, reserved roles and topology compiler
  are not requirements of the current local slot declaration.
- [Configuration implementation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/release_set/config/mod.rs)
  and its [regression tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/release_set/config/tests.rs)
  keep projections on one validated snapshot, retain decoding evidence and refuse
  malformed mutations. Canic's owned `Config` and `SelectedFleet` provide the
  snapshot boundary; the existing shared durable writer owns atomic publication.
  A single-file edit does not need legacy's multi-file package-rename rollback.
- [Discovery diagnostics tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/config_discovery/tests.rs)
  escape hostile paths while preserving original evidence. This exposed raw
  config paths in Canic's three declaration success messages and incomplete
  Unicode escaping in its private fleet renderer. Both now use
  `ic_host_text::escape::terminal_text`; paths, configuration and JSON retain
  their original values. Explicit config selection needs no recursive discovery
  or directory traversal subsystem.
- [CLI configuration tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/config/tests/mod.rs)
  distinguish a fleet name from its bound app. Current tests already exercise
  `hello/demo`, multiple fleets and multiple apps. Local declarations remain
  distinct from live membership and authority observations.

History comparison included the config tests before `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`
(parent `3700c6e1bf78cb04a60dcfaf0f789d4dd992c9e0`) and the CLI config module before
`e8605c44ffaf77ceb7ddaf85b374f42b7a48703d`
(parent `e6a061999dfae207bc20917ffefa2082e7ffe2e7`). The snapshot, escaping and
app/fleet regression cases predate those reorganizations; moving ownership did
not create the underlying guarantees.

Adapted witnesses in the current normal test paths:

| Legacy invariant | Current regression |
| --- | --- |
| One loaded snapshot survives a later config edit | `loaded_config_and_selection_retain_one_validated_snapshot` |
| Nested malformed input refuses mutation and retains its decoding source | `nested_unknown_fields_refuse_edits_without_changing_config` |
| Display escaping preserves original paths and machine output | `declaration_messages_escape_paths_while_json_retains_original_paths` |
| External diagnostics cannot inject terminal direction or line controls | `terminal_escapes_external_unicode_controls_without_changing_evidence` |

Validation: 19 host configuration tests and all 19 CLI tests passed, along with
M1 Clippy/Rustdoc, formatting and tooling agreement. The CLI was rebuilt with
`make canic`. Viewer tests required localhost access; no browser was launched.
The installed deployment simulator was already qualified for the preceding slot
declaration change and was not rerun for this presentation-only production edit.

This review qualifies local declarations and fleet text presentation, not all
legacy subsystems, every CLI error path or production management authority.

### Existing canister declarations from the CLI

The same 0.200.1 declaration path now accepts `fleet canister add --id PRINCIPAL`.
This closes the documented manual JSON step for observing an existing canister.
It uses the existing config schema, principal parser, membership validation and
atomic writer. It does not read journals or contact a provider. Existing names
remain immutable through `add`; creation in a declared-ID slot remains refused.
The same principal on another exact network binding remains a separate identity.

The declaration research above applies unchanged. The additional boundary check
read legacy [fleet import](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/import/mod.rs)
and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/import/tests.rs).
That workflow reviews exact source identities, controller transfer, code/state
clearing and bounded debits, then applies retained approval. Its tests keep new
review inputs separate from replay authority. The path history includes
`aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`; comparing the tests with parent
`3700c6e1bf78cb04a60dcfaf0f789d4dd992c9e0` found them unchanged.

Local observation declarations do not have those destructive effects, so this
increment introduces no import command, Root, capacity enrollment or approval
mechanism. The applicable invariant is that declaring an identity cannot silently
approve effects: CLI tests reject identity/network/apply inputs on `add`; host
tests refuse new creation in the declared slot and preserve config bytes on
invalid IDs, duplicate membership and attempted rebinding. Controller authority
and existence still require independent evidence at the existing observation and
deployment boundaries.

Validation: all 20 host configuration tests and 6 CLI configuration/help tests
passed, including `existing_id_declarations_are_network_scoped_and_do_not_authorize_creation`
and `existing_canister_declaration_lists_exact_id_without_deployment_authority`.
M1 Clippy/Rustdoc, formatting and tooling agreement passed; `make canic` rebuilt
the CLI. No deployment, mainnet observation or browser launch was needed.

### Initializing additional apps

`app init` now also adds a new app to an existing config. This closes the manual
`apps` map edit while retaining a single initialization command. The declaration
and snapshot research above remains applicable at the same pinned legacy
revision: refuse duplicate identities, validate the whole result, preserve
unrelated declarations and keep local configuration separate from effects.
Legacy's one-app configuration assumption does not apply to the selected
App → Fleets → Canisters catalogue; its ownership and refusal invariants do.

The host owns both initialization branches. A missing file uses exclusive
creation; an existing file uses the current sidecar lock, reload and atomic
replacement. A first-creation race may refuse one caller, but cannot overwrite a
successful declaration. Invalid existing files and symlinks are never treated as
missing configs. Duplicate app names direct the operator to `fleet init` rather
than modifying the app's fleets implicitly. JSON returns the complete resulting
config from that mutation, without an independent reload.

Regression cases cover preservation of existing IDs and journal bytes, separate
default journals for the same fleet name under different apps, concurrent first
creation, lock contention, duplicate names, malformed/oversized config, symlinks
including dangling links, the 16-app ceiling, and explicit app selection through
the CLI. This changes local declaration behavior only.

Validation: 23 host configuration tests and 7 CLI configuration/help tests passed,
along with M1 Clippy/Rustdoc, formatting and tooling agreement. The CLI was rebuilt
with `make canic`; the checkout's mainnet config and deployments were unchanged.

### Platform 0.33 adoption for the 0.200.1 GitHub snapshot

Registry availability was checked on 2026-11-14: Platform 0.33.0 is published
and non-yanked. Both workspace manifests now select it; unchanged feature sets
leave its new `nns-registry-types` codecs disabled. Testkit 0.104.0 retains
Platform 0.32.3 in its independent native harness. Canic's host graph uses 0.33.0;
the manager and M0 Wasm graphs select one Runtime/guard 0.22.3 and Types 0.20.3,
without Agent, PocketIC or Testkit in those Wasm graphs. Locked offline metadata
and consumer compilation admit both workspaces, rather than requiring matching
facade versions across independent executables.

The first-party registry review found the retained locks current: Host
FS/Process/Artifacts 0.77.2, Text 0.75.0, Storage 0.74.1, Timers 0.44.2, Wire
0.22.1, Testkit 0.104.0 and Toko Tooling 0.43.0, alongside Runtime and Types
above. Compatible manifest minimums remain unchanged; this is not a bulk
dependency refresh. Publication remains disabled for every Canic package.

Validation on Linux passed: both locked offline metadata graphs, the scoped
host/manager/M0 dependency checks, `make lint`, and `make test m0-test
m0-artifact-test m1-test profile-test growth-test`. The installed fixtures cover
creation/funding interruption, owner maintenance, retained artifact custody,
CLI deployment/recovery/upgrade, private profile preservation and autonomous
growth through all five injected interruption windows. These use synthetic cycles;
no mainnet deployment or browser launch occurred. Native macOS was not rerun.

### Slot summaries preserve conflicting evidence

The next increment corrects a presentation gap: the host already refused to
rebind retained plans after a config network edit, but the slot summary could
still label the old principal as the current slot identity. Both renderers now
compare the exact network binding before presenting a configured or retained
identity. A conflicting configured principal is also explicit beside the slot.
Operations remain in the report, with their original network and principal; no
stored plan, approval or receipt is rewritten.

This review read legacy [network resolution](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/network/mod.rs)
and [network regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/network/tests.rs),
especially `changed_anchor_cannot_rebind_an_existing_profile` and contradictory
anchor/digest refusal. Path history includes `8c87ae4c8e09cdbb6f2eb3562ecb0a821e871bcf`;
the changed-anchor case already exists in its parent
`fa92d886ef9beee824de3ec900832722fef85bd2`. Legacy derived a canonical network
identity from enrolled trust bytes. Canic currently binds the exact URL, root
key and routing principal, so it retains that existing digest encoding rather
than importing environment profiles or silently changing identity semantics.

The adapted regression keeps app, fleet, slot, URL and principal equal while
changing only the root key. Terminal and HTML must show a network conflict and
retain original evidence. It also checks same-network principal conflicts and
an unresolved operation against a newly configured ID. Host checks preserve the
previous fingerprint encoding and distinguish URL, root and routing changes.

Validation passed: all 36 Host library tests and 22 CLI tests, plus
`make m1-clippy m1-doc canic fmt-check toko-tooling-check`. The CLI tests use
loopback viewer fixtures; this increment performs no mainnet deployment.

### Readable retained-operation status

The pending 0.200.2 CLI increment gives `fleet status OPERATION` a human summary
of its existing local evidence. It uses IC Host's table and escaping APIs and
leaves `--json`, scope admission and journal ownership unchanged. It does not
query the network, infer success from retained replies, or treat an expired
approval as evidence of no effect. Upgrade creation is inapplicable only when
the retained creation class is `not_submitted`; contradictory evidence stays
visible. Receipt details are labelled historical, without a current-health claim.

Research at legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` read
[progress rendering](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/progress/render/mod.rs)
and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/progress/tests.rs),
especially `unknown_readiness_and_all_applied_counts_never_claim_deployment_success`.
The [Host progress projection](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/ops/progress/mod.rs)
and [operation tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/tests.rs)
also require exact request correlation and preserve uncertain intent. Path history
includes `afc2521ed02744c1e1a376d59dae5e2b80d9dec5`; its parent
`ac55e50334dd6479ec36f404e89e60bcfe9184d6` already has the same identity checks,
with earlier package imports. This change adapts the truthful-output invariant
to a local journal summary; it does not import legacy live progress machinery.

Regression cases cover retained replies without receipts, expired uncertain
attempts, upgrade evidence, historical receipt wording and hostile display text.
The configured CLI journey checks human output alongside unchanged JSON and
existing cross-fleet rejection.

Validation passed: the three status renderer regressions, the configured CLI
planning/inspection test, CLI Clippy and `make canic fmt-check`. A rebuilt-binary
smoke check created an isolated local declaration and plan under `target/`,
printed its status, and compared JSON with the original plan. It submitted no
effect and used neither the private mainnet journal nor a browser.

### Compact fleet terminal view

Maintainer feedback on the four-canister mainnet profile display identified
duplicated identities, stacked role badges, excessive colour and routine help
text that obscured the fleet. The default view now uses one IC Host table per
network binding, with named canisters, one principal per row, one role,
controller references and code/controller evidence. External controller
principals appear once below their network's table. Display references use
`external:N`, which cannot collide with valid configured slot names. They are
snapshot-local, not configuration or authority identifiers.

Names resolve through the same slot-state decision as conflict summaries,
including exact network binding. Conflicted or unresolved slots stay visible
without lending their names to contradictory retained identities. Existing
observation errors, unknowns, incomplete operations and inventory issues remain
in the default view. Hashes, timestamps, manager attempt counts and explanatory
text move to `--details`. The JSON and browser report remain unchanged.

Legacy research reused revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, reading
[list rendering](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/render.rs),
[renderer regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/tests.rs)
and [live-query failure tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/live/tests.rs).
Its shared table owns escaping/alignment; caller-selected colour is applied
after rendering, and failed observations remain explicit. Those mechanisms
already exist before path-history commit `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`,
in parent `3700c6e1bf78cb04a60dcfaf0f789d4dd992c9e0`. The initial compact view
adapted that table approach. The following correction restores explicit tree
semantics while keeping readiness claims separate.

Validation passed: seven terminal regressions, the slot conflict regression,
the configured CLI planning/inspection test, CLI Clippy and `make canic fmt-check`.
The profile-shaped fixture checks a compact layout, one occurrence of each
principal, colour-independent alignment, opt-in detail, preserved JSON, and
name/role isolation across trust bindings. Existing escaping, failed observation
and uncertain-effect regressions continue to pass.

A read-only mainnet check with the rebuilt `canic fleet show --app profile`
confirmed four named rows, verified code/controllers, the manager's uncertified
Ready report, and five retained receipts. The command is available through the
existing `~/.local/bin/canic` link; no browser or deployment was started.

### Application tree and role correction

The maintainer explicitly values legacy Canic's application tree design. Reading
the actual [registry entry](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/registry/mod.rs)
and [tree renderer](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/support/registry_tree.rs)
showed independent `role` and `parent_pid` fields. Tree order follows immediate
parents, not IC controllers or routing. The renderer sorts siblings, preserves
parent-before-child ordering and exposes children whose parent is outside the
visible inventory. Its [list regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/tests.rs)
exercise nested branches, subtree selection and repeated-role ambiguity.
Path history includes `d32fa61ed431e40fdda25c6dc05963f49b52e1f5`; the same tree
and ambiguity handling exist in parent
`51e99819cf55ddc7f9b5f27541a8185ad33d76e9` before its error-type migration.

The research also read legacy [topology planning](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/mod.rs),
its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/tests.rs)
and [child-binding validation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-contracts/src/deployment/topology/validation/mod.rs).
Those protect admitted roles, immediate parent identities, placement and resource
bounds. A display relationship cannot substitute for those authorities.

Current concrete slot declarations now have optional application-defined `role`
and immediate `parent` slot fields. Validation rejects absent/cross-fleet parents,
self-parenting, cycles and invalid role names within the existing config bounds.
`canister add --role ... --parent ...` uses the existing atomic config writer.
Repeated roles are allowed; commands continue to select unique slots. Listings
and HTML declarations share one tree layout, while observed rows bind names and
roles only on the original exact network and unambiguous slot identity. A missing
or conflicted parent is reported and its observed child remains visible without
nesting. The renderer is bounded against malformed cyclic snapshots as well.

The profile config declares entry, worker and manager responsibilities and only
the manager/managed-worker parent edge. Entry/shared-worker discovery is still
a routing relationship. Controller observations stay in their own column;
controlling a child no longer renames a manager to `controller`. Config role
names do not select query adapters, authorize effects or renew existing plans.
General topology deployment and dynamic-child admission remain unimplemented;
this increment preserves the model without importing a Root/Coordinator stack.

Validation passed: 24 Host config tests, two tree regressions, seven terminal
regressions, the slot conflict regression and eight CLI context/config tests;
Host/CLI Clippy, Host Rustdoc and `make canic fmt-check toko-tooling-check`.
The tests preserve named plan identity after role/parent edits, reject malformed
topology without replacement, retain unknown observations and separate routes
from parentage. Public canister identities, journals and network selections
remain unchanged.
The rebuilt read-only mainnet view confirmed the manager role and nested managed
worker, with four observed canisters and five retained receipts. No management
effects or browser launches were performed.

## First capability and consumer invariant

M1 exercises build, reviewed installation, invocation and status of one greeting
canister, followed by a changed executable with explicit preservation semantics.
It needs no database, browser frontend, Fleet admission or autonomous scaling.

The proposed M2 capability is managed installation and readiness of a shared
stateful worker, followed in M3 by a qualified maintenance-window upgrade.
The application retains user registration, routing, database schema and resource
authorization. The selected consumer is the Canic-owned profile demo; Toko
integration is outside the requested scope.

The first multi-canister demo also requires a runnable browser fleet view,
delivered with that increment and no later than M2. The team must be able to
inspect deployed canisters, their relationships, observed state and interrupted
operations. The [visualization acceptance contract](../design/canic-system-design.md#demo-fleet-visualization-from-the-first-multi-canister-increment)
owns the scope; the local growth slice extends the same view to an autonomous
worker using `--manager`. The CLI supplies a read-only `canic fleet show` JSON/HTML snapshot,
qualified against two local greeting deployments, retained uncertain upgrades and
an unavailable provider. It groups principals by exact network trust binding and
shows certified module/controller observations separately from historical receipts.
The report remains a point-in-time view of locally retained deployments; stateful
cycle balances, broader application call edges and general autonomous discovery still belong
to the full multi-canister acceptance gate.

The design also admits an optional `ic-relay` module alongside direct platform
calls. Its [managed pilot and comparison](../design/canic-system-design.md#direct-calls-and-optional-ic-relay)
use two demo canisters and one bounded application command once that journey is
available. Select a compatible supplier revision and measure batching gains,
latency, storage and recovery against the direct path before adoption. Relay's
existing supplier experiments do not qualify this rewrite. This choice neither
adds a Relay dependency to M1 nor expands the M0 closure conditions.

The selected invariant is concrete: an application-supplied user ID and its owner
principal mapping remain unchanged; private reads/writes by another caller remain
rejected. The upgrade preserves the profile name and revision. The application
owns these records; Canic's deployment layer does not reinterpret them.

On 2026-11-13 the maintainer selected a simple Canic-focused journey and excluded
Toko integration. The earlier read-only Toko observation at
`44d4e2c6d41e3575b2503e79ffa369129ff3ecf2` remains research only. No Toko build,
port, dependency update or adoption measurement is needed to advance this demo.

## Topology candidate and retained responsibilities

### Runnable stateful baseline

Use a Canic-owned profile demo: a standalone worker owns a
supplied nonzero user ID, immutable caller-to-user mapping, private profile read,
bounded display name and optimistic revision. The native demo client invokes
that application's Wire contract through ICP CLI. The existing Canic deployment
path installs the worker; its domain, storage and endpoints remain application-owned.
This is the selected hands-on consumer and baseline for subsequent managed
composition. A standalone worker does not by itself complete M2's multi-canister,
management-effect and composition requirements.

Before implementation, select these initial qualification ceilings: 16 profiles,
128 UTF-8 bytes per display name, 4 KiB RPC frames, two declared Storage
allocations and at most 32 physical stable pages (2 MiB). The complete worker
must fit the current CLI's 1,500,000-byte install limit. No timers or idle work
are admitted. Measure installation and full-capacity footprint against these
bounds; report supplier overhead separately from profile payloads. IC Storage
0.74.0 owns the backing and collection engines. Do not reuse raw-memory M0
fixtures as the worker's persistence implementation.

Executable B changes its reported application version while keeping the same
record format. Qualification must preserve IDs, owners, names and revisions,
reject unauthorized reads/writes and stale revisions, permit exact replay without
another revision, and refuse the first profile beyond capacity. A failed upgrade
must retain executable A and its data. The entry increment is recorded below;
management autonomy remains separate. This baseline creates no
Coordinator, Store or generic runtime framework.

The original standalone rehearsal passed on 2026-11-13 (`make profile-test`). Canic created
and installed the worker on PocketIC; the native client read readiness through
ICP CLI. Distinct PocketIC caller identities exercised private reads, immutable
ownership, bounded registration and revision replay. At full capacity, a failed
post-upgrade retained executable A and its records; a subsequent Canic upgrade
to B preserved all records and refused further registration. Reapplying the
completed operation with no signer returned its retained receipt. The generated
fleet view identified the current module and both deployment operations.

| Baseline measurement | Observed | Selected ceiling |
| --- | ---: | ---: |
| Raw worker Wasm | 238,598 bytes | 1,500,000 bytes |
| Code section / functions | 162,328 bytes / 394 | Included in total Wasm |
| Initial stable memory | 1,179,648 bytes (18 pages) | 2,097,152 bytes (32 pages) |
| Stable memory with 16 profiles | 1,179,648 bytes (18 pages) | 2,097,152 bytes (32 pages) |
| Initial / full-capacity Wasm memory | 1,179,648 / 1,179,648 bytes | Measured baseline |

The initial stable allocation is supplier/collection overhead with no profiles.
The filled fixture adds fifteen maximum-length names and one shorter updated
name (less than 2,048 name bytes), plus IDs, owners, revisions and record encoding,
without another physical page. This measures allocated
memory, not exact live heap or a comparative efficiency claim. There are no
application timers. The worker owns both endpoints and lifecycle hooks; Storage
owns backing, allocation admission and collections. The native client imports
the application protocol and host process supplier, not the worker implementation.

Original standalone inputs (SHA-256; preceding the entry increment):

- Root lockfile: `924c4a447808a40964daf8e326635a8bc493df6ffcc139748e8e83750902ea65`.
- Qualification lockfile: `03c2afa061914716f75d673b2ad2957dfa8b548ec3ecb5df3fa67bbb24925553`.
- Executable A: `1ec70bb3652ed781ac08476e9d36fd9fc067b223f36d33a651a25d6ca6f3c774`.
- Executable B: `edbb2137db47e6bd1d9802d5b90bb104ac5424b3cc5a74e895f54bc9876aefa4`.

The test source is `qualification/m1-tests/tests/profile.rs`; the build entrypoint
is `scripts/dev/test-profile.sh`. This qualifies the unchanged storage schema
across two executables, not management-operation preservation or entry/worker
composition. Mainnet use is described in [the demo guide](../demos/profile.md).

### Entry and worker composition

The next installed increment passed on 2026-11-13 using the same focused
`make profile-test` entrypoint. Canic deploys both canisters. The entry owns one
immutable worker address in an IC Storage cell; its controller may bind it once
or replay the exact binding. Other callers, invalid targets and replacement
bindings are refused. Its separate lifecycle strictly restores the binding on
upgrade. The worker's storage schema and ownership rules remain unchanged.

The client resolves that route through a certified replicated response, then
sends the original user's signed operation directly to the worker. No forwarded
caller assertion or impersonation authority is introduced. The entry separately
makes a controller-requested, bounded direct `probe` call to the worker. One
active check, a five-second wait, zero attached cycles and no retries bound that
observation; a callback failure can occupy the volatile slot until an entry
upgrade clears it. No management effects or spending allowance depend on that slot.

Qualification covers unbound and unauthorized requests, same-binding replay,
replacement refusal, oversized frames, native-client resolution, anonymous
identity preservation, direct worker readiness, strict entry upgrade restoration,
stopped-worker failure and successful observation after restart. The generated
fleet report contains two canisters, three deployment operations and an explicitly
operator-declared route. That arrow is not an on-chain observation. Existing
profile preservation and wrong-owner rejection cases remain in the same journey.

The current worker is 238,276 raw Wasm bytes (162,903 code-section bytes, 391
functions), with unchanged 18-page stable allocation. The entry is 325,566 raw
Wasm bytes (229,961 code-section bytes, 446 functions) and uses 10 stable pages
(655,360 bytes), within its selected
1,500,000-byte install and 16-page stable ceilings. It owns one named allocation
and no timers. These are allocation/size measurements, not a performance comparison.

Current input SHA-256 values:

- Root lockfile: `19685a9502f37b44116b7c37540bbc0f530ac91d66745f79246c6c2639eabf5c`.
- Qualification lockfile: `69b432f33babce34bd1badba9fee5a7d80800500c4ce0939b3a7f249ca183c14`.
- Worker A: `16dcabb91273504d2bc4f4cced4d0f68b2525a14c398fd8b3914aea5b520b638`.
- Worker B: `2017e8b6d4fe89ff7bd1fbd6f18ea0faa9f6b413ffc49ef3e5cfef0140aa8e85`.
- Entry: `9a8fe4927940940512f99fef91fa8784ea835aca2202622dc0453188d1bf8bfd`.

This proves discovery, direct observation and separate persistent ownership.
It does not yet qualify managed growth, surviving management-operation custody,
Relay transport or fleet-wide admission. M0/M2 remain open for those selected
management requirements; the entry is not the management-effect owner E.

### Management-effect owner candidate

Use R → E → application as the first experiment candidate. R is the operator
principal; E is one subnet-local management owner. Compare direct-host bootstrap
for its authority and cost, not as a substitute for operator-absent execution.
Select the product arrangement after the custody experiment establishes its limits.

E retains intent, reservations, exact effect identities and committed outcomes.
R retains E's maintenance operation and the inventory/exposure needed to recover
it outside E. Application code cannot clear E's unresolved obligations.
For autonomous installation, first evaluate a bounded set of approved artifact
bytes retained by E before granting growth authority; unplugging the host must
not remove those bytes. No Coordinator or separate Store enters this experiment.
Artifact size limits and storage costs must be measured before selecting that path.

## First effect experiment

The [runnable fixture](../../qualification/README.md) exercises actual management
creation and direct-cycle funding with explicit trap injection. It introduces no
product operation engine or configuration language.

| Boundary | Evidence the experiment must establish |
| --- | --- |
| Host submission | Exact signed-attempt bytes/identity survive process interruption before submission; restart does not invent another attempt |
| Executor dispatch | Intent, reservation and outgoing-call enqueueing commit together; a trap before return produces no external call from that execution |
| Returned result | Creation principal or funding completion/refund evidence survives a committed callback; writing then trapping in that callback is a separate negative case |
| Subsequent work | A later message can fail without erasing the committed outcome; reconciliation uses that outcome without repeating the paid effect |
| Maintenance | R can stop/upgrade/recover E while keeping its operation and exposure outside the affected canister; governance is investigated only if selected |

The [system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/)
defines the dispatch commit boundary; the
[execution properties](https://docs.internetcomputer.org/references/message-execution-properties/)
define rollback within a trapping callback. These are specification evidence,
not results for a selected executor implementation. Rust `await` syntax alone
does not establish a new message boundary.

Classify each injected window as proven no effect, proven effect with retained
evidence, reconstructible uncertainty, or blocked uncertainty within a stated
bound. Harness inventory may establish what happened in a test, but cannot be
used as a production recovery mechanism unavailable to R or E. Never infer the
created principal from inventory similarity or payment identity from balances.

### Creation results on 2026-11-13

`make m0-test` passed on Linux x86-64 with Rust 1.100.0, PocketIC 16.1.0,
Testkit 0.102.0, Platform 0.32.1, Runtime 0.22.1 and Wire 0.22.0. The lockfile
records the complete selected graph. The maximum-evidence codec check and all
five installed scenarios passed; native/Wasm Clippy, Rustdoc, shell and tooling
agreement checks also passed.

| Injected window | New canisters | Committed executor evidence | Outcome |
| --- | --- | --- | --- |
| After enqueue, before dispatch commit | 0 | Empty slot; no reservation | Proven no creation effect; execution still costs cycles |
| Before result write | 1 | Issued intent, no principal, 2T cycles exposure | Blocked uncertainty; another creation is refused |
| After result write, in the same callback | 1 | Issued intent, no principal, 2T cycles exposure | Blocked uncertainty; writing the record did not commit it |
| Separate continuation after result commit | 1 | Principal/refund retained; continuation write rolled back | Original outcome reconstructed by reading committed evidence; replay creates nothing |
| No fault | 1 | Principal/refund retained; child controlled by executor | Proven effect with retained evidence; replay creates nothing |

The implementation consequence is to keep irreplaceable-result capture small and
commit it before fallible subsequent work. This does not eliminate traps before
that commit. A single uncertain attempt blocks this fixture's creation capacity;
product acceptance of that blocked exposure and its disposition remain decisions.
No claim of automatic recovery for every creation window is justified.

### Direct-cycle funding results on 2026-11-13

The same `make m0-test` run passed all ten funding interruption cases: five
windows against each of a live empty recipient and a deleted recipient on the
same subnet. Each fresh executor attaches 1T synthetic cycles. The
[management API](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/)
defines `deposit_cycles`; the selected path uses an unbounded call and captures
the matching callback refund before further work. No ledger is involved.

| Injected window | Live recipient | Deleted recipient | Committed executor evidence |
| --- | --- | --- | --- |
| After enqueue, before dispatch commit | No credit | No deposit dispatched | Empty slot, zero reservation; proven no deposit effect |
| Before result write | Credited 1T | Attachment refunded | Issued intent, no receipt or refund observation, full 1T exposure blocked |
| After result write, in the same callback | Credited 1T | Attachment refunded | Same blocked uncertainty; uncommitted writes were discarded |
| Separate continuation after result commit | Credited 1T | Attachment refunded | Completion/zero refund, or rejection code 3/full 1T refund retained; zero unresolved exposure |
| No fault | Credited 1T | Attachment refunded | Same retained outcomes; identical replay returns evidence without another deposit |

For dispatch traps, repetition still produces no credit. For every occupied
slot, changed amount and changed destination are refused. Admission tests also
reject zero/over-limit amounts, oversized principal bytes and non-controller
callers without consuming the slot, then successfully admit one valid deposit.
Both maximum-evidence Wire codec checks pass under the original finite limits.
Native/Wasm Clippy, Rustdoc, formatting and shell checks pass.

Recipient balances and executor balance decreases are independent test oracles,
not inputs to recovery. In the deleted-recipient callback-trap cases, observed
executor decreases were 13,337,427 and 13,337,441 cycles respectively, rather than
the 1T attachment. This establishes a physical refund in this fixture but supplies
no committed operation receipt to the executor. It therefore keeps 1T reserved
and refuses replay. A reject code alone would not release exposure: the probe
retains any unrefunded portion even when a rejection record commits.

The bounded result is deliberately conservative: no callback-loss window was
reconstructed from balances. A retained receipt survives later work; absent that
receipt, a single uncertain attempt blocks the funding slot. Product acceptance
and disposition of this bound remain open. Local dispatch-resource failures,
cross-subnet deposits and upgrade persistence have not been qualified here.

### Qualified inputs

Creation was rerun successfully after sharing its dispatch injection with the
funding probe. SHA-256 digests below identify that combined passing run; paths
are relative to the repository:

```text
6a0f87ca7a9b37a369e6a8f8fd020da8de0268c0266767b4022121a57638f709  qualification/Cargo.lock
5d2801ce434ba03e75d53acf8944eaa9678f8850d3b92984f73d389c777253d8  qualification/m0-probe/src/lib.rs
add2c1182af05ae42107016b8d799e755e07dafee88dce6d6b48a3dcf9da81da  qualification/m0-probe/src/canister/creation.rs
b943d5b8079997ce47bbd8ccde6c17f474c35df0a4f2fe386c874941c08f4cc2  qualification/m0-probe/src/canister/funding.rs
e92fcc8110ade8fbf27ce0389108c019c7a87423662ce9c37045709afd942d8a  qualification/m0-probe/src/canister/mod.rs
6cdbc9aeb059bc5d1bdb1f8fa0d20c757bc2d676e415c1a6735873b9e08765ae  qualification/m0-probe/src/tests.rs
097984a2af24b618afda29c1b47055874360255266ba2c966a72b93af0c77cb9  qualification/m0-tests/tests/creation.rs
b742937cc45f496e7a48a0013430dce579e10b7afeebd19ab4f63868c5dfc48b  qualification/m0-tests/tests/funding.rs
18f497e7f4d307e816dd4e2c074f29e3640f699c5537f27bccfba27bfad9fcde  qualification/m0-tests/tests/support/mod.rs
bc1737fc22075d576fe26887b098052b7412bcbe72d5958a15498022e44bbdde  qualification/target/wasm32-unknown-unknown/release/canic_m0_probe.wasm
b44e1eccd66e02328146b3e209e3403e3a3428a25d25555aca2f67500e5ae9db  target/m0/server/16.1.0-x86_64-linux/pocket-ic
```

### Owner-maintenance results on 2026-11-13

`make m0-test` passed again with the maintenance hooks, on Linux x86-64 with
Rust 1.100.0, PocketIC 16.1.0, Testkit 0.102.0, Platform 0.32.2, Runtime 0.22.2
and Wire 0.22.1 from the current qualification lockfile. This includes the original
five creation cases, ten funding cases, funding admission and both bounded codec
checks. `make m0-maintenance-test` selects the new rehearsal alone.
Native/Wasm Clippy, Rustdoc, formatting and shell checks also passed. Native
macOS execution remains unqualified.

Six maintenance cases cover empty journals after dispatch traps, retained
creation/deposit successes, lost callback evidence, retained evidence after later
failure, a retained funding rejection/refund and a rejection whose callback
evidence was lost. Each case establishes R → E and, when its creation result is
retained, E → child. R is a distinct simulated operator principal; an unrelated
principal cannot upgrade E.

The harness retains original evidence outside E, stops E, and attempts a changed
executable whose post-upgrade hook restores the journals, overwrites stable bytes
and traps. The old module, stable bytes and both journals survive unchanged. R
can restart E, then stop and successfully upgrade it to the changed executable.
Repeating the failing upgrade with populated stable memory also preserves the
running candidate and its evidence. These observations agree with the atomic
upgrade contract in the [system API specification](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#canister-upgrades).

Replaying the original commands after maintenance creates no extra canister or
deposit. Lost outcomes still reserve the full 2T creation and 1T deposit exposure,
even when the test oracle observed a refund. Retained outcomes remain readable;
the upgrade does not turn uncertainty into another allowance.

The test owns one raw 64 KiB stable page containing two independently bounded
Wire frames. It reuses the existing journal schemas and limits. This is an
isolated persistence experiment, not a production Storage integration. Host
observations are retained in harness memory, so the result does not qualify a
durable R journal or host crash recovery. It also excludes callbacks still in
flight, schema migration, recovery from an unconditionally trapping pre-upgrade
hook, governance and mainnet executor maintenance. Those limits prevent this
result from closing the entire infrastructure-recovery requirement.

The qualification lockfile SHA-256 is
`9d196c90c9f4ed0dbc5b95b2cf8d218da7c330d074f6d90c94424ce58676285a`.
The two separately retained executables are `target/m0/original.wasm`
(`bd1624f19e1f569f7267b983d5d22fd1b300617ebdce7ac8c80e24fecb180094`)
and `target/m0/maintenance-candidate.wasm`
(`8e91ccfef31e28fd0074effc2ef88f243a1986c3491550a4a5c524897e6c558c`).
The maintenance source SHA-256 is
`e70078f9cc972fad74b8c93f2bfc1c4f001fc4493a969bc45877a5c571eed12d`;
its installed test is
`1ed73914f578c3722625c0507841a8ce40044eea1d29e7ba5371a00f91bcae90`.

### Artifact-custody results on 2026-11-13

`make m0-artifact-test` passed two native admission/codec checks and four installed
scenarios on Linux x86-64, Rust 1.100.0 and PocketIC 16.1.0. The selected suppliers
are Platform 0.32.2, Runtime 0.22.2, Wire 0.22.1, Testkit 0.102.0 and SHA-256
implementation `sha2` 0.11.1. Native/Wasm Clippy, Rustdoc and shell checks passed.

The fixture retains one approved raw Wasm, exact hash and target before a future
dispatch time. A failed upgrade preserves the approval; a successful upgrade
restores it into a changed executable. Advancing only the simulator clock and
rounds then installs the actual Hello World worker, which returns its greeting.
No operator update or host artifact delivery occurs during that interval.

Platform's released direct `InstallCodeArgs` export is native-only. The qualified
on-chain alternative uses `upload_chunk` into E's own management chunk store,
then `install_chunked_code` with the approved hash and target, following the
[management API](https://docs.internetcomputer.org/references/management-canister/#install_chunked_code).
There is one chunk and no separate Store canister. Each request commitment is
retained before dispatch; an unknown result does not create another attempt.

| Case | Retained evidence and disposition |
| --- | --- |
| Successful autonomous installation | One upload and one install; callback retained; module hash and sole executor controller verified |
| Upload callback writes then traps | One upload, zero installs; original upload commitment remains blocked; physical chunk presence is only a test oracle |
| Install callback writes then traps | One install, no committed success reply; after executor maintenance, module/controller observation reconstructs the installed binding without reinstalling |
| Target already has code | Installation rejected; original code/version retained; approval cannot be replaced to retry |

All cases also reject unauthorized staging, oversized upload frames, wrong hashes
and changed approvals. Replaying the original approval never repeats an upload
or install. Worker versions independently prove no reinstall; observing the
installed binding does not manufacture a missing callback receipt. These claims
assume the selected sole executor and its operator do not independently replace
the worker; application readiness remains a separate future obligation.

The rebuilt worker is 43,632 uncompressed bytes; its upload frame is 43,699 bytes.
At the observed post-install point E uses 1,441,792 bytes of Wasm heap, 65,536
bytes of stable memory and 1,048,576 bytes of IC-managed chunk storage. These are
total fixture measurements, not incremental attribution or a production budget.
The 65,536-byte artifact ceiling fits a 65,619-byte upload and a 65,758-byte
maximum-evidence checkpoint, requiring two stable pages. Chunk storage is retained
after this experiment; cleanup and retention economics have not been qualified.
In the successful 61-second simulated interval, E's balance fell 374,600,091
cycles and the worker's 136,804,576 cycles. These include fixture execution,
storage time and observations; they are not installation-only costs or mainnet
prices. The heartbeat is test scheduling, not the product's idle-cost policy.

This supports executor-local custody for the bounded first autonomous experiment.
The host-only path remains simpler for M1 but requires the operator to supply
installation. A separate Store would add a canister without a demonstrated need
for this one-artifact workload; its costs and benefits have not been measured.
Product selection still needs governed storage, demand-driven scheduling and the
stateful consumer's actual artifact budget. This fixture pre-creates the worker
and does not yet compose creation, installation and readiness into the M2 path.

SHA-256 inputs: qualification lockfile
`a14cd0d337142a9dc550e4fa3c2e5f9c14de6eabca99164523de3071e52b4bf9`;
executor A `3ebb48eecbc840651390fcdb727a88f37e0726127753193dc43ee7ea994986bd`;
executor B `b4fc51bb87a334050c61c97d23b8b1657db7630437eea66aec83f6fa63d59d60`;
worker `0ac16f95cca67500fc8f6615012365cd81fcc09bde253035cde783476e89635f`.
The executables are retained separately under `target/m0/artifact/`. The earlier
digests identify their own qualification runs, not this expanded lockfile.

## First CLI journey on 2026-11-13

The direct-host arrangement is selected for the stateless M1 greeting: one
operator controls one application canister. Host retains the approval, artifact,
signed attempts and returned results in `.canic/`; ICP CLI retains key custody.
The application-scoped manager selected below supplies the first bounded enduring
owner; this host-only path remains its bootstrap and comparison.

`make m1-test` passed on Linux x86-64 with Rust 1.100.0, PocketIC 16.1.0 and
repository-prepared ICP CLI 1.7.0. Eight host checks cover plan/artifact admission,
writer exclusion, immutable retained bytes, external creation DTOs, signed
request/status binding and refusal to replace expired uncertain attempts.
The installed test invokes the actual CLI process to deploy executable A, call
`Hello from Canic, team!`, recover its receipt without a signer, and upgrade the
same principal to executable B returning `Hello again from Canic, team!`.

The harness also removes the saved creation reply, and later the upgrade reply,
to model result loss after network completion. Apply recovers using the original
signed requests, without a signer, unchanged signed bytes or an extra canister.
These cases qualify resubmission within the original validity window. They do
not emulate host power failure; synchronized file/parent publication belongs to
Host FS. Expired unresolved attempts remain blocked. Deleting the only real
deployment journal is never a recovery instruction.

Native/Wasm Clippy, library Rustdoc, formatting, shell and tooling agreement
checks passed. The demo's Wasm dependency graph has no Host, Agent, CLI or simulator
dependency. Qualification invokes the CLI process so PocketIC's older pinned
`thiserror` does not force a downgrade of the product graph.

An anonymous mainnet `icrc1_fee` query returned 100,000,000 cycles, matching the
[provider contract](https://github.com/dfinity/cycles-ledger/blob/main/README.md).
The operator subsequently completed ledger/CMC creation and mainnet installation
through this CLI. Native macOS execution remains unqualified. This path assumes
the ledger fee remains unchanged
between preflight and submission, and one coordinated operator uses the retained
state directory. Cross-host fencing and concurrent external upgrades are not
qualified. The stateless demo's upgrade proves principal/controller preservation,
not the M3 application-data or infrastructure-journal promise.

The preceding M0 input digests identify their original combined probe run. The
qualification lockfile now also includes the M1 process harness; that addition
does not replace the older evidence's source identity.

### Mainnet observation on 2026-11-13

The operator applied operation
`50dea196b16ff449b86f0000fde720971745b50f8b1c349bdc3e0dc7cdb1f786`,
creating `z4fqh-uqaaa-aaaan-q6yrq-cai` under the sole controller
`o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe`.
The retained creation and installation replies and terminal receipt are present.
An independent completed-apply invocation with no available signer revalidated
the certified controller and module hash
`cdb19d3a2a115eea571730fe9598af5b0319e9aa1b3545f9c595cb3ac19dd42b`.
A fresh, explicitly uncertified greeting query returned
`Hello from Canic, team!` at approximately 17:04 UTC.

The next executable's hash is
`187f6d99668f93609c8adc72a800fb6c0ca1f8ff2d85147f22497137f0f7832c`.
Its prepared upgrade binds the same canister and exact previous module, with
zero creation funding. Mainnet execution of that upgrade remains pending; the
local same-canister rehearsal already passed. A subsequent read-only fleet
snapshot found the original one-hour upgrade approval expired with both effects
still `not_submitted`; a new reviewed plan is needed before signing. This does
not authorize replacement of an uncertain submitted attempt. Authority-bearing signed files
remain in `.canic/`, outside Git; these observations are not a backup of them.

## Bounds and preservation witness

### Profile mainnet deployment and preservation

On 2026-11-13 the operator unlocked `canic-mainnet` directly in a desktop terminal.
ICP CLI minted 2,000,000,012,799 cycles (rounding around the requested 2T), bringing
the ledger balance to 2,405,555,110,034. The conversion result is retained in
`.canic/profile-mainnet-funding/result.json`; it must not be used to justify
repeating a payment after uncertainty. Each following creation had an approved
1,000,000,000,000-cycle debit ceiling, including the ledger fee.

| Step | Canister | Operation digest |
| --- | --- | --- |
| Worker creation/install | `4pkhu-oaaaa-aaaai-ra2ta-cai` | `fe6ff221535c6938683da088b22abd72b94776e166b094988dbae1dad85a4563` |
| Entry creation/install | `bisqm-biaaa-aaaas-qhfeq-cai` | `b2aa17980e465da49cd9f416f5665161ad972aee00b6d0c6167ede4d5a4be3e7` |
| Worker upgrade | `4pkhu-oaaaa-aaaai-ra2ta-cai` | `96c32c1933b72bb68ea99475497f09fe1d885c5251418dd407b877f9fe7f9be3` |

The creation artifacts match the worker A and entry hashes recorded above. The
upgrade installed worker B (`2017e8b6d4fe89ff7bd1fbd6f18ea0faa9f6b413ffc49ef3e5cfef0140aa8e85`).
Certified metadata independently confirmed both current modules and the original
sole controller. All three terminal receipts and exact attempts remain under
their original operation directories in `.canic/`.

The entry's binding and direct readiness check succeeded. User 1 was created,
updated to revision 2, read, and then read again after the worker upgrade. The
owner bytes, user ID, display name and revision matched exactly. These private
reads remain uncertified queries; the write replies, deployment metadata and
entry's replicated readiness response use certified paths. The post-upgrade
readiness response reported `profile-v2` and `ready: true` under the same route.

The final account observation was 405,555,110,034 cycles and 6.42852601 ICP. The
browser snapshot contained both profile canisters and the existing Hello World
canister, with no inventory or metadata-observation failures. Its declared route
remains distinct from observed controller/module metadata. Localhost serving
does not imply that the canisters themselves are local.

### Remaining management witness

Start with one executor, one application and at most one unresolved paid attempt;
an unresolved creation pauses new creation instead of replenishing its allowance.
Use synthetic local cycles first. Record attached-cycle ceilings, protected
recovery reserve and observed fees separately. The operator restored
`canic-mainnet`; the first proposed deployment budgets 1T cycles from its default
cycles-ledger account. Approval is an explicit apply of the reviewed digest and
requires local identity unlock. Local results do not authorize a live debit.

Measure the Hello World setup and operator steps first. Before M2, measure the
selected stateful consumer's current lifecycle glue, endpoint/memory/timer owners,
dependencies, total Wasm and code-section sizes, initial stable memory and idle
work. Record numeric acceptance budgets before implementing that adoption slice;
no footprint or cost advantage has yet been measured.

For M3, executable B changes observable application behavior while preserving
the user ID, owner mapping, profile revision and private-read rule from executable A.
Across the transition, an interrupted management operation retains its original
identity, evidence and outstanding exposure at its surviving owner; B cannot
turn it into a new allowance. Separate this from the experiment upgrading E:
application preservation does not prove infrastructure-owner recovery.

M0 closes on the bounded effect results, selected topology/custody/artifact path,
agreed consumer scope and baseline, numeric budgets and this concrete preservation
scenario. A documented blocked window is acceptable only with a bounded exposure
and an explicit disposition; it is not automatic recovery. The profile mainnet
upgrade is now qualified above; the earlier expired Hello World plan remains
unsubmitted. The standalone
consumer baseline and budgets are now recorded above. The manager below selects
the first product custody/scheduling path. Normal mainnet growth has passed;
comparative adoption measurements remain outstanding, and broader lifecycle work
needs consumer admission. These
results alone do not close M0.

## Application-scoped autonomous manager: local qualification

On 2026-11-13, select `operator → profile manager → one new profile worker` for
the first autonomous slice. The manager owns management authority, the immutable
one-use approval, retained worker Wasm and operation journal. The worker owns
its application lifecycle and private records. The existing mainnet entry keeps
its immutable original route; the new worker is addressed directly. This is not
a subnet-wide executor or general scaling policy.

Two IC Storage cells and one IC Timers 0.44.1 registration retain the work through
operator absence and manager maintenance. No separate Store or Coordinator is
needed for this bounded artifact set. The operator's controller authority supplies
the maintenance path; no governance arrangement is selected.

Qualification bounds: one approval; one create/upload/install issuance each;
1T creation attachment; at least 0.5T liquid reserve before each effect (plus the
attachment before creation); 512 KiB raw worker Wasm; 64 stable pages. Status
frames remain 4 KiB, with a 64 KiB allocation allowance and 1,024 nodes. The larger
staging envelope accounts for Wire's conservative allocation charge; native tests
cover maximum artifact size and fully populated evidence. These are resource
ceilings, not a lifetime funding or total execution-cost guarantee.

`make growth-test` passed with the actual Canic CLI and profile client:

| Evidence | Observed result |
| --- | --- |
| Admit once, then no mutating operator ingress | One ready worker, sole manager controller, same subnet; counts `[1, 1, 1]` |
| Authenticated profile write on the created worker | Original caller owns the retained profile |
| Distinct manager maintenance executable | Original authority, child and dispatch counts survive; no additional child |
| Trap before create dispatch commit | No child or committed attempt; original scheduled authority resumes after maintenance |
| Create callback writes then traps | Child exists, principal/refund absent; `CreateIssued` never creates again |
| Separate upload message traps before dispatch | Previously committed child principal survives |
| Upload callback writes then traps | Stored-chunk observation reconstructs outcome without another upload |
| Install callback writes then traps | Module/controller observation reconstructs outcome without another installation |
| Changed approval, unauthorized caller, bad hash | Refused without replacing original authority |
| Wrong native network/root key, expired dispatch, inadequate admission reserve | Refused or stopped; no extra child |
| Exact approval replay, including after maintenance | Same slot and evidence; no new allowance |

Independent subnet counts verify effects; the manager is never given that oracle
to recover a lost principal. Creation-result loss remains blocked with full 1T
attachment exposure after maintenance. This is an experimental limitation, not
automatic recovery or production approval. Intent and outgoing-call issuance
commit with their IC message; callback writes commit only on successful message
completion. The next effect starts in a separate timer message, so a later
failure cannot erase a committed outcome.

The native growth file binds the selected network/root key and exact Wire request,
retaining both before invoking ICP CLI. Manager bootstrap uses the existing signed
host-attempt journal. Growth admission instead relies on the manager's identical-
approval idempotence; the client does not claim custody of ICP CLI's signed ingress.

Measured default manager: 684,139 raw Wasm bytes, 500,616 code-section bytes,
875 defined functions and 18 physical stable pages (1,179,648 bytes), including
retained worker bytes. The management chunk store additionally retains one worker
module; its bytes are outside the stable-page measurement. SHA-256:
`dd316c9ab7bc628abea68a5384eff3c689a12476382b413f26b8841e17e925b8`.
Worker: 238,329 raw bytes, SHA-256
`0be0704fb8e13a43650b457a98dfb88ee4a409ff349c6a03b53ab5603845b2a5`.
These are whole-artifact measurements, not supplier attribution or evidence of
size/cost savings. Earlier mainnet artifact hashes remain historical.

The fleet snapshot reports both manager and created worker. The manager journal
is labelled uncertified; module/controller metadata is independently certified.
Observation never resumes work. See the [growth guide](../demos/growth.md) for
reproduction, mainnet preparation and maintenance limits. No autonomous manager
was deployed to mainnet in this qualification run.

## Autonomous mainnet worker

On 2026-11-13, deploy the exact default manager qualified above through operation
`2cfe76adf294be8d03c2afeca2f5f64dc4be51ca4ba470dd66517f629dd6ce42`:

| Role | Principal | Sole controller |
| --- | --- | --- |
| Manager | `rk6n2-wyaaa-aaaad-qmoua-cai` | `canic-mainnet` (`o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe`) |
| Worker | `rn7lo-3aaaa-aaaad-qmouq-cai` | `rk6n2-wyaaa-aaaad-qmoua-cai` |

The initial wallet observation was 6.42852601 ICP and 405,555,110,034 cycles.
The single requested 2.5T conversion deposited 2,500,000,005,152 cycles; the small
rounding difference is retained in `.canic/growth-mainnet/funding.json`.
The manager's creation budget was 2.5T. Its single immutable worker approval
attached 1T with the selected 0.5T minimum liquid reserve check. Transfers and
provider fees remain separate concepts; no cost advantage is claimed.

The certified admission reply was `Scheduled` with counts `[0, 0, 0]`. Scheduled
dispatch was 21:00:24 UTC. Until an anonymous query reported `Ready`, host commands
were observations only. A subsequent controller update returned certified
`Ready` evidence: counts `[1, 1, 1]`, all three request commitments retained, zero
creation refund, no failure, matching approved/observed worker hash and 18 stable
pages. The maintenance executable marker was false.

Independent certified fleet observations matched the manager hash
`dd316c9ab7bc628abea68a5384eff3c689a12476382b413f26b8841e17e925b8`
and worker hash
`0be0704fb8e13a43650b457a98dfb88ee4a409ff349c6a03b53ab5603845b2a5`,
with the controller graph above. The snapshot contained five canisters, six host
operations and no inventory or observation failures. Historical expired Hello
World approval remains unsubmitted; it is not this manager's authority.

An authenticated update created user 1 on the new worker at revision 1 with the
name `Canic autonomous mainnet worker`. Its private query returned the same full
profile. The write reply was certified; the private read was explicitly an
uncertified query. Existing entry routing and its original worker were unchanged.

The final wallet observation was **5.10925125 ICP** and **405,555,115,186 cycles**.
Host signed attempts remain in the operation directory; approval, admission,
anonymous observation sequence, certified outcome, profile checks and fleet
snapshot are retained in `.canic/growth-mainnet/`. The completed execution artifact
reported exit zero. Do not rerun its funding payment or replace the spent slot.

This qualifies the normal one-worker mainnet path. At this point, interruption
and maintenance evidence was local; the subsequent mainnet maintenance run is
recorded below. General worker lifecycle is not qualified. The [current demo commands](../demos/growth.md#current-mainnet-demo)
use the existing canisters without another deployment.

## Mainnet manager maintenance

On 2026-11-14, upgrade `rk6n2-wyaaa-aaaad-qmoua-cai` in place using the exact
maintenance artifact from the passing local growth fixture: 684,263 raw bytes,
SHA-256 `5824826b6b715762223e5a0cec21239379d4ff415478341dddfa1b5a12078118`.
Operation `8b54397d04474aa916e93bcb89f46b8db7e96fccd05391d4eeb4224930f601d6`
binds the original manager module, same controller and zero creation funding.
Only `maintenance-candidate` was selected; no fault-injection build was used.

The first apply stopped after retaining the successful installation reply and
before persisting its terminal receipt. The exact cause of that initial
verification failure was not retained. A subsequent query observed the changed
executable. Resuming the original operation with identity `anonymous` and an
unavailable signer path `/nonexistent/canic-no-signer` settled the receipt through
fresh certified module/controller reads. The existing reply was reused; there
was no second signature, installation or replacement operation. This is direct
mainnet evidence of post-install receipt recovery, not a generic retry guarantee.

Certified manager observations before and after maintenance matched in every
field except the intended executable marker (`false` → `true`). This includes
the original approval and deadlines, worker principal, `[1, 1, 1]` dispatch
counts, all request commitments, zero refund, matching observed module, no
failure, `Ready` phase and 18 stable pages. Replaying the original, now expired,
approval returned identical certified evidence; it did not renew the spent slot.

The worker `rn7lo-3aaaa-aaaad-qmouq-cai` kept its exact module hash and sole manager
controller. Its before/after private profile queries matched the full record:
user 1, original owner, revision 1 and `Canic autonomous mainnet worker`. These
private reads remain explicitly uncertified. The fleet snapshot contained five
canisters and seven host operations, with no inventory or observation failures.

Retained evidence lives in `.canic/growth-mainnet-maintenance/`, including the
before/after observations, recovered receipt, replay result, profile checks and
fleet snapshot. The first execution artifact records exit 1; the subsequent
verification records exit 0. The original host operation directory retains its
signed attempt, reply and now-complete receipt.

The wallet preflight remained 5.10925125 ICP and 405,555,115,186 cycles; no new
deposit was visible. Maintenance used the manager's existing cycles, without a
wallet conversion or new canister creation. The preservation guarantee remains
limited to this same-format upgrade. Injected failures and blocked-creation
preservation were tested locally; cross-schema migration and general worker
administration remain separate scope.

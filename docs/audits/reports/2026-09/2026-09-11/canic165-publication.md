# CANIC-165 reviewed source publication

The host now compiles bounded fixture publication into the existing Fleet
journal and content-addressed plan owner. This qualifies source publication
and recovery; the complete CANIC-165 provisioning batch remains open.

## Maintained behavior

The exact Root manifest selects each source descriptor. The host emits one
Root preparation per distinct content identity, then one direct Store upload
per ordered chunk, before Root bootstrap. Those actions retain existing
intent-before-effect and cycle accounting. Store owns source-byte progress;
the application will own its separate import cursor and completion receipt.

Preparation observes protected Root source status. Upload reconciliation accepts
an exact committed prefix or a later valid cursor, including completed replay.
Only typed NotFound permits an absent-source observation; other Store failures
retain their typed cause. Plan content hydration verifies the prepared Store,
role/content descriptor, expected prefix and bytes, including rejection of a
substituted object with its own valid SHA-256. Reports carry hashes, sizes and
local paths instead of payload arrays. Per-chunk expected metadata has constant
size; descriptors are not copied into every upload action.

`canic_root_fixture_status` is a controller-authenticated composite query.
Prepared-Root policy admits exactly that endpoint/kind; other call kinds and
non-Root roles remain fenced. Ordinary Root status retains its existing query
behavior. A separate endpoint is necessary because the
[IC query contract](https://docs.internetcomputer.org/references/ic-interface-spec/https-interface/)
requires composite queries for same-subnet query calls and excludes them from
update calls. The real runtime test exposed the missing Prepared admission;
its corrected path is now covered by both policy and canister tests.

The earlier standalone publication selector is removed. No additional mutable
source cursor, operator daemon or Root payload relay was introduced.

## Qualification

- [Native checks](canic165-publication-evidence/native-tests.log): 20 host and
  30 core replay-policy tests pass. They cover retained source files, descriptor
  and destination substitution, report externalization, deterministic compilation,
  durable journal interruption at every effect and effect-free terminal replay.
  The journal test uses the existing mock platform to isolate persistence.
- [Prepared policy checks](canic165-publication-evidence/policy-tests.log): all
  five pass after the exact endpoint admission correction, including wrong-kind
  and non-Root rejection. This is the only subsequent core behavior change.
- [Scoped Clippy](canic165-publication-evidence/clippy.log): affected host, core,
  control-plane, facade and internal test libraries/tests pass with warnings denied.
- [Actual Root/Store journey](canic165-publication-evidence/pocketic.log): the
  production Store-sequence compiler selects a real two-chunk fixture. The
  canister adapter discards upload replies, reconstructs each action, reads the
  retained exact byte prefix, completes bootstrap, and immediately observes every
  action terminal without further updates. The new composite query executes
  while Root is Prepared. PASS in 201.20s including artifact builds; 240s runner.

The canister journey uses the existing PocketIC typed-command adapter. It is
not a complete CLI-generated Fleet apply with target import/receipt delivery.
That combined provisioning proof remains part of FP2.

[Commands](canic165-publication-evidence/commands.json) and the final
[1,615-input snapshot](canic165-publication-evidence/source.sha256) retain the
qualification boundary. All recorded Rust, Cargo/config and Candid inputs
stayed unchanged through final lint, policy and runtime qualification. The
host/replay tests precede only the separately tested final activation-fence
correction and removal of the redundant endpoint-body variant check.

## Remaining batch

Next derive grants from verified Root installation intent and connect the
registered importer and durable receipt to runtime scheduling, initial parent/
child placement and dispatch. Later-Shard delivery, funding/backoff, source
reference release and terminal Store retirement remain. Fixture-bearing Fleet
generation stays disabled until that lifecycle is complete. The combined
worktree is not push-ready; Unreleased remains unassigned. No version, broad
gate, Git publication, deployment or sibling repository mutation occurred.

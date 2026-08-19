# 0.104 B1 Timer And Async-Job Ownership Baseline

Date: 2026-08-18

## Result

B1 was accepted by the maintainer on 2026-08-18. It changes no Rust source,
Candid, stable state or runtime behavior; its accepted inventory authorizes
the bounded B2 native-provider surface hard cut.

The hard-cut decisions are:

- keep `ic-timers` as the one scheduling provider;
- delete Canic's public application timer facade and transient claim model;
- dissolve the generic `TimerWorkflow` scheduler into exact native
  registration custody owned by each Canic consumer;
- keep neutral runtime status and metrics as projections of the provider's
  atomic inventory;
- retain core stable-memory ID 60, but hard-cut its schema-1 meaning to minimal
  domain async-job attempt fences and the one cycle-top-up replay identity that
  is not already owned elsewhere; and
- add the paired synchronous lifecycle participant only to ordinary root and
  managed `start!` expansions, with optional local-fixture support and no
  specialized Coordinator or Store form.

The mechanically reviewable inputs are:

- [consumer inventory](consumer-inventory.tsv), covering the 45 production
  Rust files discovered from parsed timer/recovery types, imports, paths and
  macro tokens across applications, canisters and crates;
- [native claim inventory](native-claims.tsv), classifying every provider
  registration family and its target custody;
- [durable state inventory](durable-state.tsv), classifying every field in the
  current memory-ID-60 record; and
- [propagation contract](propagation.md), fixing the guide, fixtures,
  lifecycle grammar, IcyDB boundary and B2-B8 work map.

The accepted [B2 native-provider surface report](b2-native-provider-surface.md)
records the public hard cut, direct-provider fixture and targeted validation.
Its phase measurements are historical observations, not closeout evidence.
The accepted
[B3 domain async-job recovery report](b3-domain-async-job-recovery.md) records
the memory-ID-60 hard cut and exact business-attempt invariants. The completed
[B4 domain-native custody report](b4-domain-native-custody.md) records lazy
auth/cycle/placement registration ownership and snapshot propagation. Its
phase measurements have the same historical status. The accepted B5
completion evidence is retained below because the working-audit directory is
already at its bounded eight-file limit.

## Closeout Evidence Correction

The independent closeout audit found that the exact B2-B6 phase source states
were not retained in Git. Their size and instruction tables cannot therefore
be rebuilt independently and are not acceptance evidence. They remain in these
working notes only as explicitly historical development observations. No
closeout or release-size conclusion depends on them.

The release comparison was rerun from the immutable `v0.103.0` and `v0.104.0`
trees in equal-length source directories with equal-length isolated Cargo
targets, `CARGO_NET_OFFLINE=true`, `CARGO_INCREMENTAL=0`, locked Cargo, the
`fast` profile and the same `app`, `root`, `fleet_coordinator` and `wasm_store`
roles from `apps/test/canic.toml`. Each tree used the canonical
`canic-host --example build_artifact` executable built from that exact tree.
The toolchain was `rustc 1.97.1 (8bab26f4f 2026-07-14)`. For each role the
locked offline command shape was:

```text
cargo run --offline --locked -q --profile fast -p canic-host \
  --example build_artifact -- \
  <role> fast <release-root> <release-root> \
  <release-root>/apps/test/canic.toml
```

The 0.103.0 builder SHA-256 was
`6add9d7d3edce7fcc03eaf9462813130dcdc8289d832c47178e685ab3e939d7d`;
the 0.104.0 builder SHA-256 was
`4ccba956cfd3b03c9bf70ab8904788a7e378dbd88a2646baec10599c25d4632e`.

| Role | 0.103.0 raw | 0.103.0 gzip | 0.104.0 raw | 0.104.0 gzip |
| --- | ---: | ---: | ---: | ---: |
| managed Component | 3,571,984 | 944,397 | 3,560,554 | 941,683 |
| Fleet Subnet Root | 8,424,621 | 2,183,333 | 8,406,365 | 2,183,574 |
| Fleet Coordinator | 4,071,122 | 1,013,961 | 3,818,092 | 950,589 |
| Wasm Store | 3,357,121 | 889,005 | 3,339,306 | 883,883 |
| **Four-role total** | **19,424,848** | **5,030,696** | **19,124,317** | **4,959,729** |

| Release | Role | Raw SHA-256 | Builder-gzip SHA-256 |
| --- | --- | --- | --- |
| 0.103.0 | managed Component | `28d664a4a1753e9cc5f86a19a3abd11f3da38fc101af647510cf1cdb522e94be` | `3516c5a93c87df8a135ba138242dbe42f4432c82833031200a61d4fe01865258` |
| 0.103.0 | Fleet Subnet Root | `3081ed485224d009147e142c4b94d59018c720fcc5ba3182489e6c2fa4725638` | `7cfc4fb0a8ff0b5cd2f4ba2673462246902ccd7c490f70f7a2d8d9d6d42c1c18` |
| 0.103.0 | Fleet Coordinator | `18f5401cbd3926fe95e8c2e2ff6b04d6e795a0562c67089ec262618436e80a3d` | `36e1e08a668cd5bedf12a7ea157857813a4d9065543eafec216bb5bd24df1793` |
| 0.103.0 | Wasm Store | `dbad8e83d796f3ff22653cd2947fa4467331b8cf12cea6fe94c848a61b7c192a` | `708ed0b7a1b8fe5b689d83b91e53d49ccfa7ba12b286180e42f08e28be406bd0` |
| 0.104.0 | managed Component | `26b7bb0ca8e4f63715fb9caf892d9234ac6a61cd8c88db200c8fbe4da8a0da53` | `32fdc8094a0b1a53076a7d6eac8f74aa294f3c785793ce99daa855b226e32fc3` |
| 0.104.0 | Fleet Subnet Root | `efbee3f577f40780636ed2a3a49033d53c6e95ea8aaede4280b36451f7e09b42` | `d31c2304d5d4203e81377980196fd6aff0c7cea4d539c60854925ca4bc09542a` |
| 0.104.0 | Fleet Coordinator | `6ccef1a9c85818300d3fe075521e3271495f818256ddd511a8a54ff6310e76f5` | `2d5cde42c14f7d781196bb5fde34375f2f65ad3e0739adabce9a65a36c6ac77a` |
| 0.104.0 | Wasm Store | `30fc82a6ac91c42f6a03c6a50b17e420f2e9eb0be16e82dd02400f730f521de9` | `b1b7f293e92002cc640ce9e8c5ca66e1c96f89cfa2d37ce3fd8ab9aa3077bd14` |

These are reproducible absolute release footprints. The builders have different
source identities and the current builder correctly refuses to build a
different Canic version. The arithmetic difference is not presented as a
controlled causal percentage. This supersedes the former 0.103 baseline and
B8 candidate tables below. The current runtime observation remains directly
reproducible: two work samples total 46,593 instructions with no Wasm- or
stable-memory-page growth.

The closeout correction touches only tests, the managed test fixture, a
development-only parser dependency, guards and documentation. It changes no
shipped product-role source or runtime dependency; no new product Wasm delta is
claimed beyond the exact `v0.104.0` release-tree footprint above.

## B5 Pool, Lifecycle And Snapshot Completion

B5 was accepted by maintainer continuation on 2026-08-18. The remaining
central `CLAIMS` map, `TimerClaim` union and snapshot/recovery function-pointer
registries are deleted. Intent cleanup and log retention hold their exact
native once registrations directly. Root Canister-pool maintenance holds one
native after-completion registration, while Root owns one native watchdog that
dispatches expired core and pool business attempts without acquiring ordinary
deadline authority.

Lifecycle deferrals register direct remove-when-stopped native once claims.
Pending private lifecycle work therefore blocks an authority snapshot as an
unmanaged row rather than being copied into a Canic registry. Root snapshot
prepare/resume invokes exact core and control-plane owners. Coordinator uses a
separate empty-fixed-owner path, so it neither links nor executes Root timer
workflows. A source guard freezes those distinct calls after the first B5
artifact build exposed and then rejected a 420,266-byte Coordinator linkage
regression.

The real restored-Root journey observes exactly one
`canic/canister_pool/maintain` row and one
`canic/async_job_recovery/watchdog` row. Both are scheduled before prepare,
unregistered while the live authority is sealed, scheduled again after live
resume, and unregistered in the restored sealed snapshot. The exact
Coordinator snapshot/restore journey also passes with no fixed background
claims.

### Historical Provider Performance Observation

The original development run reported two work samples and no separate
scheduler callback. The B4/B5 source states were not retained, so this table
is not independently reproducible and is not closeout evidence:

| Observation | B4 | B5 | B5 minus B4 |
| --- | ---: | ---: | ---: |
| Scheduler instruction samples | 0 | 0 | 0 |
| Work instruction samples | 2 | 2 | 0 |
| Latest work instructions | 23,709 | 23,229 | -480 (-2.0245%) |
| Maximum work instructions | 23,898 | 23,449 | -449 (-1.8788%) |
| Total work instructions | 47,607 | 46,678 | -929 (-1.9514%) |
| Maximum Wasm-memory growth | 0 pages | 0 pages | 0 pages |
| Maximum stable-memory growth | 0 pages | 0 pages | 0 pages |

This historical observation motivated the cleanup, but no causal B4-to-B5
performance improvement is claimed at closeout.

### Historical Fast-Profile Wasm Observation

The development run recorded the following phase table. Neither the B4 nor B5
source state was retained, so the values cannot be rebuilt independently and
are not release acceptance evidence:

| Role | B5 raw bytes | Delta from B4 | Delta from 0.103.0 | Raw SHA-256 | B5 gzip bytes | Delta from B4 | Delta from 0.103.0 | Gzip SHA-256 |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| managed Component | 3,560,417 | -10,023 (-0.2807%) | +175,556 (+5.1865%) | `afda78793f71e638af166627e33fd0808778d250a0cd2c0da89fd74237e8f21d` | 941,716 | -2,642 (-0.2798%) | +58,449 (+6.6174%) | `5bb127e7e4ff17c746381b717fa0b6204b4847e458441e6fa491bb706d6e54b5` |
| Fleet Subnet Root | 8,406,294 | -26,317 (-0.3121%) | +290,608 (+3.5808%) | `665461985552b46871fae4128104fb5a852775334831eff7e71f85679e90ce92` | 2,183,382 | -2,772 (-0.1268%) | +96,807 (+4.6395%) | `016155c5c4e916262afc8e9b0509d70b9fd01781b0a08934e46d1f8790afe3c6` |
| Fleet Coordinator | 3,818,092 | -245,629 (-6.0444%) | -253,043 (-6.2155%) | `ae3cd03983348b3fc0c4f788f5986aff1d16d1e5047161fabd8fb22fe61a6dd5` | 950,608 | -60,310 (-5.9659%) | -63,300 (-6.2432%) | `810f6fb7ec7a6db2776f6cb8c129d9896bbcea25126cffc9c9525973de7d41fe` |
| Wasm Store | 3,339,170 | -10,519 (-0.3140%) | +8,980 (+0.2697%) | `42b8eb226d54afe3be8792e361c6c42afab2a829ef030ef9179881dc058508b5` | 883,868 | -1,310 (-0.1480%) | +4,235 (+0.4815%) | `67d977d7b986a4a98690a29e08f5257828813ae9ed9b1a31ab44be2c4780ffcb` |
| **Four-role total** | **19,123,973** | **-292,488 (-1.5064%)** | **+222,101 (+1.1750%)** | — | **4,959,574** | **-67,034 (-1.3336%)** | **+96,191 (+1.9779%)** | — |

The exact release-tree footprints in the closeout correction supersede every
comparison to `v0.103.0` in this historical phase table.

### Focused Validation

Passed on the locked `ic-testkit 0.8.8` graph:

- locked affected-package compilation and warning-denied Clippy for core,
  control plane, facade, runtime probe and the touched test packages;
- lifecycle and timer source guards, ten focused Canister-pool tests and all
  36 facade protocol-surface tests;
- all three timer-authority PocketIC journeys in 6.04 seconds with the three
  labeled artifacts reused;
- the exact Coordinator authority snapshot/restore journey in 11.49 seconds;
- the real restored-Root inventory and snapshot/resume journey in 170.55
  seconds cold; and
- all nine canonical fast build artifacts, including the four measured
  product roles.

Two preliminary PocketIC invocations expired their separately launched
server during cold host/fixture compilation and failed with connection
refused before any canister assertion. The governed retry used an explicit
ten-minute server lifetime; both warmed runtime journeys passed. The complete
workspace, release matrix and broad PocketIC suites were not run.

## B6 Native Adoption And Lifecycle Participant

B6 was accepted by maintainer continuation on 2026-08-18. The maintained
[native-timer guide](../../../features/runtime/native-timers.md) now shows the
exact `ic-timers = "=0.6.1"` dependency, direct once and after-completion
registrations, cancellation/unregister/detachment behavior, volatile custody
versus durable demand, native reconstruction and the shared inventory. The
`runtime_probe` fixture implements that contract directly and no replacement
Canic application scheduler is introduced.

Ordinary managed and Root `start!` expansions now accept exactly one paired
`lifecycle_participant(init = path, post_upgrade = path)` declaration.
`start_local!` accepts the same development-only grammar. Both paths coerce to
safe synchronous `fn() -> ()`; partial pairs, async functions, closures,
arguments, results and duplicate pairs fail at compile time. Store and
Coordinator start macros remain closed. The expansion invokes the matching
participant once after Canic restores its synchronous invariants and exact
native claims, but before any Active/Prepared branch, bootstrap registration
or deferred application hook. It adds no registry, stable state, Candid
method, readiness claim or lifecycle export.

The managed Prepared fixture proves init and repeated post-upgrade
participation without activation. A trapping init participant leaves the same
funded canister without a committed module; a later round cannot commit
deferred work, and corrected installation succeeds on that exact canister. A
trapping post-upgrade participant leaves the prior module hash and Prepared
state committed; rebuilding without the test trap then succeeds from that
exact boundary. The Root fixture observes
restored Root authority plus the exact
`canic/async_job_recovery/watchdog` and
`canic/canister_pool/maintain` claims before participation. The local runtime
probe reconstructs both application-owned rows synchronously; its deferred
setup/install/upgrade hooks independently assert that reconstruction already
occurred.

The normal, init-trapping and post-upgrade-trapping managed artifacts have the
same extracted Candid SHA-256
`b7b1bd43d8fecab0da1d06396f3d3b4dbc7dee59d2dc762298cd0f780f250a0d`
and normalized export SHA-256
`22d4829a03a3651ff28f2b252021fd05d337b75d5f5839d2871fff635f34c593`.
Each contains exactly one `canister_init` and one `canister_post_upgrade`.

### Current Provider Performance

The final application after-completion journey reports two work samples and no
separate scheduler callback:

| Observation | B5 | B6 | B6 minus B5 |
| --- | ---: | ---: | ---: |
| Scheduler instruction samples | 0 | 0 | 0 |
| Work instruction samples | 2 | 2 | 0 |
| Latest work instructions | 23,229 | 23,383 | +154 (+0.6630%) |
| Maximum work instructions | 23,449 | 23,383 | -66 (-0.2815%) |
| Total work instructions | 46,678 | 46,593 | -85 (-0.1821%) |
| Maximum Wasm-memory growth | 0 pages | 0 pages | 0 pages |
| Maximum stable-memory growth | 0 pages | 0 pages | 0 pages |

The B5 source state was not retained, so the signed B5/B6 cells are historical
only. The B6/current absolute result of 46,593 total instructions is the
reproducible closeout observation; no causal phase improvement is claimed.

### Historical Fast-Profile Wasm Observation

The development run recorded the following B5/B6 phase table. The B5 source
state was not retained, so exact equality and the one-byte gzip movement are
not independently reproducible and are not closeout evidence.

| Role | B6 raw bytes | Delta from B5 | Raw SHA-256 | B6 gzip bytes | Delta from B5 | Gzip SHA-256 |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| managed Component | 3,560,417 | 0 | `c5e0ba4217a664328926bf6064721fa6277a1d66c678eb28260fd960a3431de3` | 941,714 | -2 | `eeac48b45fb58d513583b2e258b89b5f3c5b476a5be67c418c39ab74184452ec` |
| Fleet Subnet Root | 8,406,294 | 0 | `744d7e46bb12c5677235be9733f0c713273a8eb19c7a1a944d4d3f1661ebcaad` | 2,183,383 | +1 | `1df3b08eebc2c69676e2780476604ab87b7e7cbd6fabe16558deb9cb9d89d929` |
| Fleet Coordinator | 3,818,092 | 0 | `ae3cd03983348b3fc0c4f788f5986aff1d16d1e5047161fabd8fb22fe61a6dd5` | 950,608 | 0 | `810f6fb7ec7a6db2776f6cb8c129d9896bbcea25126cffc9c9525973de7d41fe` |
| Wasm Store | 3,339,170 | 0 | `42b8eb226d54afe3be8792e361c6c42afab2a829ef030ef9179881dc058508b5` | 883,868 | 0 | `67d977d7b986a4a98690a29e08f5257828813ae9ed9b1a31ab44be2c4780ffcb` |
| **Four-role total** | **19,123,973** | **0** | — | **4,959,573** | **-1** | — |

The maintained `runtime_probe` test fixture grows from 3,630,976 to
3,632,919 raw bytes (+1,943, +0.0535%) and from 892,510 to 893,017
deterministic-gzip bytes (+507, +0.0568%). That is test-only lifecycle evidence,
not a shipped-role regression.

### Focused Validation

Passed on the locked `ic-testkit 0.8.8` graph:

- both public compile-fail examples, the six lifecycle-boundary guards, five
  managed-endpoint gate checks and all 36 facade protocol-surface checks;
- warning-denied Clippy for the facade, core lifecycle guard, maintained
  managed/local/Root fixtures, internal harness and both touched integration
  targets;
- all four managed lifecycle journeys, including participant rollback and
  corrected retry, plus the real Root lifecycle journey;
- all four timer-authority journeys in 7.86 seconds against one explicitly
  owned PocketIC 15 server; and
- exact B5/B6 Candid and normalized canister-export equality for the managed,
  Root and runtime-probe fixtures, each with exactly one `canister_init` and
  one `canister_post_upgrade`, plus all nine canonical product artifacts.

The first direct runtime-probe check exposed that the upgrade helper advances
zero-delay hooks before returning, so the maintained deferred hooks now assert
the ordering internally. A provisional managed counter query was correctly
fenced while Prepared and was removed; rollback evidence uses the committed
module hash instead, preserving the exact Candid surface. The complete
workspace, release matrix and broad PocketIC suites were not run.

## B7 Combined Canic And IcyDB Qualification

B7 was accepted by maintainer continuation on 2026-08-19. The Canic-owned
managed composition fixture pins exact published `icydb = "=0.230.2"`; its
crates.io archive has SHA-256
`6a43703b839835ed1a532f280b5ec370aae4e690860da8aa0e8335347924ff24`.
The schema uses IcyDB memory IDs 100 through 106, outside Canic's reserved
range, and no shipped Canic crate imports IcyDB.

The locked graph resolves exactly one `ic-timers` package ID at version 0.6.1.
Canic retains the only `canister_init` and `canister_post_upgrade` exports;
IcyDB supplies the paired synchronous participant invoked after Canic restores
its invariants and before deferred application work. Normal and deliberately
trapping fixture builds expose identical Candid with SHA-256
`18516eda125afbf9291e03146ca99cf5204180fc92f877413437f5467556a0a6`
and exactly one lifecycle export pair.

The focused PocketIC journey proves all of the following on real stable state:

- IcyDB initializes and admits database work while the managed Canic runtime
  remains Prepared;
- a Prepared same-release upgrade reconstructs the IcyDB participant and its
  row without running Canic's deferred application callback;
- a participant trap rolls back both state and module hash, and the corrected
  retry succeeds from the prior committed boundary;
- activation observes distinct Canic and IcyDB timer rows simultaneously; and
- an Active same-release upgrade reconstructs both rows and preserves the
  participant-before-deferred-callback ordering.

The final normal fast artifact is 5,959,481 raw bytes with SHA-256
`acfd53e545f52bdf5353be5083431549aed65aaad07c9a888852673d8e118f34`
and 1,519,923 deterministic-gzip bytes with SHA-256
`0df708d65462a6cec76f0422e02d5928155707599ecababdefc534f30d8eb8d4`.
This is a test-only composition artifact, not a product-role size or
performance change.

Locked affected-package compilation, warning-denied Clippy, the lifecycle
payload unit check and the 37-target workspace-test inventory guard pass. The
single focused PocketIC composition test passes in 19.74 seconds against the
pinned PocketIC 15 server. The complete workspace, release matrix and broad
PocketIC suites were not run.

## B8 Semantic Ownership And Closeout

B8 was accepted by maintainer continuation on 2026-08-19 and structurally
strengthened by the closeout correction. The former lexical call-site count is
replaced by a parsed Rust ownership contract. Its
45-file [shared inventory](consumer-inventory.tsv) covers `apps/`, shipped
crates and every executable canister fixture, while excluding ordinary test
modules and the dedicated internal test-harness crate. Each discovered file
must be classified as fixed Canic consumption, private lifecycle consumption,
native registration custody, domain async-job recovery, DTO/metrics
projection or independent application custody; the prohibited scheduling-
authority class must remain empty. An exact per-file native registration
capability map makes a new or duplicated scheduling call fail even inside an
already classified file.

Class-specific checks prevent recovery records and projections from acquiring
native registrations, private lifecycle consumers from retaining registration
custody, and independent applications from claiming the `canic` owner. Parsed
imports reject scheduling-capability aliases and public re-exports; self-tests
prove comments and strings do not classify a file while unclassified and
duplicate native calls remain observable. Global checks reject the removed
generic claim/key/workflow/handle vocabulary and any direct `ic-cdk-timers` or
`cdk::timers` access, including aliased imports and raw global-timer mechanics.
Maintained runtime documents
may name the removed surface only as migration history and do not advertise a
callable facade.

The final lock contains exactly one `ic-timers 0.6.1` package, one provider-
implementation `ic-cdk-timers 1.0.0` package and exact outer `icydb 0.230.2`.
No manifest declares the raw provider. The root workspace pin plus eight
package manifests declare `ic-timers`: `canic-core`, `canic-control-plane`,
`canic-tests`, Saltz burner, the maintained application test fixture,
`runtime_probe`, `delegation_root_stub` and the IcyDB composition probe.

### Final Provider Inventory And Performance

The focused PocketIC closeout reports:

| Representative role | Declared rows after readiness | Scheduled rows | Automatic top-up row |
| --- | ---: | ---: | --- |
| Runtime-only managed probe | 4 | 1 | absent |
| active Fleet Subnet Root | 4 | 2 | absent |
| sealed Fleet Subnet Root | 4 | 0 | absent |
| Fleet Coordinator | 0 | 0 | absent |
| Wasm Store | 0 | 0 | absent |

Coordinator and Store are the role-pruned negative controls: their source and
artifacts retain no fixed background claim. The managed scheduled row is the
application-owned after-completion interval. The active Root scheduled rows
are pool maintenance and the closed async-job recovery watchdog; sealing
unregisters both without copying provider state.

The B6 column is historical; the B8/current column is the reproducible final
observation:

| Observation | B6 | B8 | B8 minus B6 |
| --- | ---: | ---: | ---: |
| Scheduler instruction samples | 0 | 0 | 0 |
| Work instruction samples | 2 | 2 | 0 |
| Latest work instructions | 23,383 | 23,383 | 0 |
| Maximum work instructions | 23,383 | 23,383 | 0 |
| Total work instructions | 46,593 | 46,593 | 0 |
| Maximum Wasm-memory growth | 0 pages | 0 pages | 0 pages |
| Maximum stable-memory growth | 0 pages | 0 pages | 0 pages |

### Superseded B8 Fast-Profile Wasm Observation

The development run recorded the following B6/B8 phase table. The B6 source
state was not retained, so exact equality and compression-noise claims are not
independently reproducible. The immutable release-tree table in the closeout
correction is the final quantitative evidence.

| Role | B8 raw bytes | Delta from B6 | Raw SHA-256 | B8 gzip bytes | Delta from B6 | Gzip SHA-256 |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| managed Component | 3,560,417 | 0 | `2bac8813c6a513359dff129a732202ece0072366cdfe3eb427ccd869e1bd4317` | 941,678 | -36 | `74be0d58bb5c71d7e85ce8a7e31567c1de3a3047dbddd6bebc01a5b2c8f1a1e5` |
| Fleet Subnet Root | 8,406,294 | 0 | `cfbba90e407346444149375ab62c8936328475500de02018eb0d32cd11e527ba` | 2,183,414 | +31 | `ffd407790924701b8865d56be8062f65c1840ef2dc1b571d5189104a54d0bbae` |
| Fleet Coordinator | 3,818,092 | 0 | `ae3cd03983348b3fc0c4f788f5986aff1d16d1e5047161fabd8fb22fe61a6dd5` | 950,608 | 0 | `810f6fb7ec7a6db2776f6cb8c129d9896bbcea25126cffc9c9525973de7d41fe` |
| Wasm Store | 3,339,170 | 0 | `57d57c8a815bcd6b5e562e8e869812464c4b7efe9fc5dffd1432ca83c84f327d` | 883,866 | -2 | `f285579bb02190860132d6d9c6e2831688f605e0aae5db72f7803a3132bca066` |
| **Four-role total** | **19,123,973** | **0** | — | **4,959,566** | **-7** | — |

### Focused Validation

The thirteen structural timer-ownership/provider/document/wait/snapshot guards pass.
Warning-denied Clippy passes for the semantic guard, timer-authority target and
internal Root harness; all five lifecycle-boundary journeys, changelog governance
and current-document semantics also pass.
All four timer-authority PocketIC journeys pass in 14.01 seconds after their
three artifacts were rebuilt; the isolated measurement journey then passes
warm in 3.24 seconds with all three reused. The exact restored-Root inventory,
snapshot and resume journey passes in 47.50 seconds. B7's exact IcyDB
composition journey remains the accepted lifecycle-composition proof. The
complete workspace, release matrix and broad PocketIC suites were not run.

## Immutable 0.103.0 Source Identity

| Identity | Exact value |
| --- | --- |
| Annotated tag | `v0.103.0`, tag object `b571f51a1e599677752b61b1f7ad1fae9e455186` |
| Release commit | `89be28f8edf0d55035ddac0c864d6c99771fc49c` |
| Source tree | `bb04909002bdc30e1db8c8be43f57c7e53ee1bcd` |
| Root `Cargo.toml` SHA-256 | `12631bd43582dc80607401097f60b0cdb158c8e00d5fd3c49f0fa8c3321f2d62` |
| `Cargo.lock` SHA-256 | `38eb123df77ee35603af1b395c979f075a601469dae1c3b92da2f310e6208327` |
| Package version | `0.103.0` |

The remote `main`, annotated tag and peeled tag were checked on 2026-08-18 and
all resolve to the same release commit. The local working tree's separate
`ic-testkit 0.8.2` manifest edit is not part of this baseline; the hashes above
come from Git objects at the immutable tag.

## Exact Provider Graph

| Package | Exact release-lock identity | Disposition |
| --- | --- | --- |
| `ic-timers` | crates.io `0.6.1`, checksum `5259def17470219f98157dac4258d5fec77f6a98e1239043fe128211333a751f` | Sole timer provider; workspace pin remains `=0.6.1` |
| `ic-cdk-timers` | crates.io `1.0.0`, checksum `6852b9c1d4a82ff50fc7318599298aee8bfb082bd7e9fe7e5c1420692b2170f7` | Provider implementation dependency only; no direct Canic production use |

The release graph contains one package ID for each row. Direct manifest
consumers of `ic-timers` are:

| Consumer | Purpose | 0.104 treatment |
| --- | --- | --- |
| `canic-core` | Canic fixed jobs, lifecycle work and inventory projection | Keep the dependency; remove duplicated mechanics and move claims to exact owners |
| `canic-tests` | Integration assertions against provider capacity | Keep as test-only direct evidence |
| `runtime_probe` | Shared-inventory fixture and current facade consumer | Convert all Canic-facade use to direct provider custody |
| `saltz_burner` | Standalone application-owned native once registration | Retain unchanged as a separately owned direct-provider example |

No production Rust source imports `ic_cdk_timers` or `cdk::timers`. The B8
semantic guard must retain that rule while expanding the classified source
roots from `crates/` and selected `canisters/` to `apps/`, `crates/` and all
executable canister fixtures.

## Surface Disposition

| Current surface | Current role | 0.104 disposition | Batch |
| --- | --- | --- | --- |
| `canic::timer!` | Public application one-shot macro | Delete without alias | B2 |
| `canic::timer_interval!` | Public application after-completion macro | Delete without alias | B2 |
| `canic::api::timer` and prelude re-exports | Public facade namespace | Delete without replacement wrapper | B2 |
| Public `TimerHandle` and `TimerApi::{set,set_interval,cancel}` | Canic-owned transient identity and cancellation | Delete; downstream code holds native registrations | B2 |
| `TimerClaimId`, `ClaimKey::Transient`, `NEXT_TRANSIENT_ID` | Generic transient custody | Delete | B2 |
| Canic `TimerDirective` and callback `TimerRunResult` | One-for-one provider vocabulary | Delete; callbacks return native provider types | B2 |
| Public Canic `TimerError` | Facade/adapter error vocabulary | Delete from the public API; exact owners use provider errors or their domain error | B2-B5 |
| `TimerKey` | Central fixed-job selector and identity factory | Delete; each owner declares its typed native identity locally | B4 |
| `TimerWorkflow` | Generic scheduler, claim map and provider adapter | Dissolve into provider initialization plus exact owner custody; no scheduler wrapper remains | B2-B5 |
| `CLAIMS` and `TimerClaim` | Central native-handle union | Delete after each fixed registration moves to its owner | B4-B5 |
| snapshot/async-recovery function-pointer registries | Indirect control-plane participation | Delete; lifecycle invokes the exact root/control-plane owner | B5 |
| `TIMERS_SUSPENDED` | Volatile projection of the durable authority fence | Retain only in snapshot/authority ownership, not as provider state | B5 |
| `CanisterTimerStatus` and timer metrics DTOs | Neutral boundary projection | Retain; populate from one atomic provider inventory | B4/B8 |
| DTO `TimerExecutionOutcome` | Public observation enum | Retain only as a projection; no callback returns it internally | B2/B4 |

## Memory ID 60 Decision

Core stable-memory ID 60 is retained and reshaped in place. Existing owner
records do not all contain the serial-attempt lease needed to deny overlap and
stale completion, so removing the allocation would duplicate that invariant.
The current uniform record nevertheless over-retains operation generations:
only cycle top-up consumes `AsyncRecoveryAttempt::operation_id()`. Auth renewal
uses its durable delegation batch, placement acknowledgement uses durable
receipt operation IDs, and pool maintenance uses its own creation/reset/
handoff journals.

B3 therefore hard-cuts the current schema-1 key and types to the current
`async_job` meaning:

```text
memory ID 60
  AsyncJobRecoveryRecord
    auth_renewal: AsyncAttemptFenceRecord
    canister_pool_maintenance: AsyncAttemptFenceRecord
    cycle_topup: ReplaySafeAsyncAttemptFenceRecord
    placement_receipt_acknowledgement: AsyncAttemptFenceRecord
```

Every row retains checked attempt generation and an optional active attempt
lease. Only the cycle-top-up row retains checked operation generation and an
optional exact retry generation. B3 uses one current key such as
`canic.core.async_job_recovery.v1`, recomputes the bounded encoded size and
updates the state-contract manifest. Reinstall-only policy permits this hard
cut; no old-key reader, migration, `v2` lane or fallback is added.

## Authoritative Demand And Retry Reconstruction

| Owner | Authoritative demand after restart | Exact retry identity | Provider schedule derivation |
| --- | --- | --- | --- |
| Root issuer renewal | Root issuer capability, enabled delegated-token configuration, durable delegation batch and proof-validity timing | Existing delegation batch/proof identity; no shared generated operation ID | Derive one native deadline from current auth timing; stop when no enabled template/due proof exists |
| Automatic cycle top-up | Compiled `AutomaticTopup` capability, current balance, funding policy/history and parent authority | The cycle row's exact pending operation generation, replayed through the parent funding request | Register only for the exact capability; derive retry/cooldown from funding facts, never a generic timer record |
| Placement acknowledgement | Durable terminal-receipt index and each receipt's exact operation ID | Receipt operation ID already stored by the intent owner | Empty index leaves the retained native once declaration inactive; retry derives from remaining receipts/domain eligibility |
| Root canister-pool maintenance | Fleet Prepared/Active/draining state plus durable pending creation, reset, handoff and recovery journals | Existing pool journal/platform operation identities | Root owns one native after-completion registration; demand and stop/restart come from pool/Fleet state |
| Intent cleanup | Durable intent-expiry index | Not an async recovery owner | Reconcile one native once deadline from the earliest expiry |
| Log retention | Durable log index and retention policy | Not an async recovery owner | Reconcile one native once deadline from current policy/index |

The recovery watchdog remains one native registration over the closed set of
four async owners. It may notice an expired business-attempt lease, confirm
current domain demand and dispatch one fenced takeover. It may not own ordinary
deadlines, persist scheduling commands or generate a new paid operation after
uncertainty.

## Superseded Wasm And Performance Baseline

B1 was evidence-only. The following development artifacts were originally
used as the phase baseline, but the later independent release-tree rebuild did
not reproduce them. They are retained as historical observations and are
superseded by the closeout-correction table above:

| Role | Raw bytes | Raw SHA-256 | Gzip bytes | Gzip SHA-256 |
| --- | ---: | --- | ---: | --- |
| managed Component | 3,384,861 | `504458f66d1d8a8bd62e18a11daf641be50dda57c3292c55328f3f30e3efb361` | 883,267 | `0ccd0d33ecfbea9c57c87b83e3a37f7a022690cf9f4dd9e76125e15bc05de7e7` |
| Fleet Subnet Root | 8,115,686 | `7f56d4fc998f9a6c8d41ff5afb88355430248a83fefd5084ffa4b23936793829` | 2,086,575 | `d29af86ab4505b9e16806375b26770d802ba995366809d3539ae18c23bddbd9a` |
| Fleet Coordinator | 4,071,135 | `8d6dbfe377aaf90a8895609a181ddcde27b601cf656c5c0077b5d74a445e4ebc` | 1,013,908 | `73ac90b6db527fb1a8a736d4693d48750ded3c3fdb8e5718eed272a382bdf58b` |
| Wasm Store | 3,330,190 | `682ab5140b54603542dcabead4bbc80f8a2130f0ba8ffbe728175067f2b7f922` | 879,633 | `9bc2bec7d6bc5e2e91dd6790dd98b4b612e02865d0701bb32740a863f4c4a730` |

The first source-mutating comparison must rebuild the same four roles with the
same canonical fast builder and report signed raw/gzip byte deltas. Managed
Component and Root are the primary size signals; Coordinator and Store are
negative controls because they must not acquire fixed jobs they do not own.

Performance comparison uses provider-owned observations rather than wall-clock
test duration: scheduler/work instruction sample count, latest/maximum/total
instructions and maximum Wasm-memory growth pages from
`CanisterTimerStatus`. The closeout comparison also records declared and
scheduled registration counts by role and proves idle, capability-pruned jobs
execute zero callbacks over the existing 24-hour PocketIC window. B1 changes
neither those counters nor the provider runtime, so it makes no performance
improvement claim.

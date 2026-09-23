# B1 prepared documentation and historical comparisons

Date: 2026-09-23. State: source, runner and selected artifact qualification
complete; both repeated measurements retained and independently verified.
These are disposable audit inputs, not production
features, release candidates or behavior-parity evidence.

## Accepted role expansion disposition

Row 7 asks for current macro expansion versus exact role/capability expansion.
The frozen `.5` source already derives exact role capabilities in `build!`,
emits closed destination-crate cfgs, selects the role lifecycle bundle in
`start!`, and cfg-selects managed command/status variants, provider dispatch
and metrics tiers. Coordinator and Store have dedicated bundles. The
[source record](b1-prepared-source-review/role-expansion-disposition.json)
binds the seven relevant source/config files; the maintained
[inventory](../../../working/0.110-fleet-runtime-contraction/b1-generated-surface-inventory.md)
traces the role matrix and distinguishes later working-tree changes.

There is no separately defined exact-expansion counterfactual left to measure
at that baseline. Rebuilding the same selection would manufacture a zero-cost
experiment; enabling otherwise unselected capabilities would change the
protocol. Removing selected providers would duplicate rows 12–16 and would
measure behavior removal, not expansion overhead.

The maintainer explicitly accepts this disposition on 2026-09-23: retain row 7 as an already-selected source/protocol
finding, claim no byte/function saving, and keep optimized role-inapplicable
reachability proof under the provider experiments and mandatory B4 work.
This is not a claim that every unselected implementation is absent from Wasm.
The executable catalog records `source_satisfied` and refuses measurement
execution for this finding. No zero-cost artifact pair is invented, and this
acceptance neither completes B1 nor authorizes B2/B3.

## Derived Candid documentation

Row 9 binds immutable `v0.110.5`. Its common preparation contains an exact copy
of registry `candid_derive 0.10.35`, including its license. The archive digest
matches the frozen lockfile. The sole upstream-derive modification selects original
generated `_ty_doc()` bodies with `CANIC_AUDIT_CANDID_TYPE_DOCS=1`, or
`TypeDoc::default()` with `0`; missing or other values reject in the macro.
Rust's `option_env!` tracks this input when compiling the derive crate.

Both conditions use the same path override and prepared lock. No existing
package version or dependency edge changes; only the derive package's registry
source/checksum becomes the common path identity. `_ty()`, serialization and
Rust documentation remain untouched. Endpoint method documentation remains.
The switch covers all reachable derive-generated type documentation, including
dependency types, so any result is inclusive rather than Canic-DTO-only.

The [source provenance](b1-prepared-source-review/candid-provenance.json)
binds the archive, upstream/prepared files and exact patch. The
[native probe](b1-prepared-source-review/candid-probe.rs),
[manifest](b1-prepared-source-review/candid-probe-Cargo.toml) and
[lock](b1-prepared-source-review/candid-probe-Cargo.lock) cover a renamed record,
generic page, and unit/tuple/record enum variants. Running input `1`, then `0`,
then `1` in the same private target proves switch invalidation and restoration.
Type documentation changes as intended; representative wire bytes, decoded
values and Candid types remain identical. The
[verification](b1-prepared-source-review/candid-probe-verification.json) also
checks that the probe uses a subset of the frozen dependency versions. This
bounded probe does not prove all application serialization or runtime parity.

Reproduce by putting the retained probe source at `src/main.rs` in a private
Cargo package, using the retained manifest/lock, applying the common patch to a
disposable `.5` worktree, and invoking offline locked Cargo from that worktree
with an explicit private target. The working directory matters: Cargo must
discover the audit worktree's `.cargo/config.toml` path override.

The runner applies common preparation before resolving its harness lock and
hashes both tracked differences and added audit files. Its focused regression
injects a change into an added derive file during metadata preparation and
requires rejection before artifact compilation. Failed preparation restores
the clean product. The row may become `ready` only after both conditions compile
all fourteen selected artifacts; qualification never supplies a repeated measurement claim.

The first full qualification compiles all fourteen controls and the first ten
variants, then Store correctly rejects a materialized declaration that still
contains the intentionally removed type comments. Those partial results are
compile evidence only. The final audit patch also gives Store's native
materializer the same documentation input: control retains the checked-in
canonical declaration, while the disabled condition materializes its already
compiled declaration. The exact declaration/profile equality check remains
unchanged, and missing/invalid audit inputs reject. No checked-in Candid or
product source is regenerated. The derivation code, lock graph and all other
artifact build paths are unchanged from the first qualification.

Focused paired qualification completes Store plus the three remaining
fixtures. The [composite qualification](b1-row9-qualification/verification.json)
verifies all 28 selected payload vectors, exact exports and structural Candid
equality in both directions. Its [provenance](b1-row9-qualification/run-metadata.json)
explicitly distinguishes unchanged ten-role compilation from the completed
corrected Store/fixture run; the earlier failed run is not represented as a
successful complete run. Patch comparison proves only native Store handling
was added. Row 9 is now `ready`, and the complete final-input 56-build retained
matrix subsequently completes with exact repetitions. An initial propagation
attempt stopped at the native driver's denied dead-code lint; the corrected
switch retains the canonical path for control and suppresses no lint. Neither
failed run supplies a retained size claim.

### Retained type-documentation result

The [complete matrix](b1-row9-measurement/artifact-metrics.tsv) retains 56 builds
from the final patch and both clean repetitions. Independent
[verification](b1-row9-measurement/verification.json) checks every payload,
metric, method identity and repeated vector, clean source/lock restoration,
exact export identities and structural Candid equality in both directions.
The [metadata](b1-row9-measurement/run-metadata.tsv) binds the common prepared
source and sole condition input. Compare only this prepared control with its
variant, not with unprepared `.5` or another experiment.

| Artifact | Code-byte delta | Defined-function delta | Candid-byte delta |
| --- | ---: | ---: | ---: |
| App | -22,008 | -9 | -7,582 |
| Index Hub | -19,113 | -8 | -7,551 |
| Test | -22,752 | -8 | -8,188 |
| User Hub | -21,991 | -8 | -7,582 |
| Scale Hub | -22,034 | -8 | -7,582 |
| Index Child | -19,055 | -8 | -7,551 |
| User Shard | -21,982 | -8 | -7,582 |
| Scale Replica | -22,008 | -9 | -7,582 |
| Root | -53,780 | -6 | -21,164 |
| Fleet Coordinator | -29,383 | -7 | 0 |
| Wasm Store | -9,796 | +1 | -2,105 |
| Runtime fixture | -9,224 | +1 | -1,590 |
| Payload-limit fixture | -9,232 | +1 | -1,523 |
| Blob-storage fixture | -14,220 | +1 | -2,864 |

The eleven separately deployed canonical artifacts sum to 263,902 fewer code
bytes, 455,478 fewer raw bytes, 163,723 fewer gzip bytes and 78 fewer defined
functions. Table minima and element counts are unchanged in every artifact.
Unlike row 8, this switch changes optimized executable sections in every
artifact, including Coordinator despite its identical Candid bytes. The exact
section comparison is retained in the verification record; Store and all
three fixtures gain one function while their code shrinks. Do not infer a
uniform function benefit or add these overlapping results to other ablations.

The scope includes dependency-derived documentation. Rust and endpoint method
documentation remain, but type comments in declarations change. The native
wire/type probe and bidirectional Candid comparison are bounded interface
evidence, not representative runtime-instruction or recovery parity. This
measurement routes later B4 analysis; it does not authorize a production
derive fork or documentation removal.

## Historical recovery family

Row 18 binds the last release containing the family, `v0.110.2` at
`f9009d5ae7be78d4f9dd746431584368770e8364`. Its common roster preparation
copies only the two index fixtures and their configuration from frozen `.5`,
registers the two local packages, and adds their exact lock entries. Every
existing package identity, checksum and dependency list remains unchanged.
Offline locked metadata succeeds. Both conditions therefore have all eleven
canonical roles under one resolved graph.

The separate removal patch extracts the recovery-owned production changes from
immediate successor `d5aa319dc6e6d9af48d8833931e076519b80968a`: Root DTOs,
stable records, ops/workflow and catalog admission, plus helper construction,
host dispatch, planning and publication. It excludes unrelated native ICP
management changes, funding-bound increases, dependency upgrades, tool pins,
and release or CI edits. The
[source record](b1-prepared-source-review/historical-preparation.json) binds
all selected paths and both patch hashes. Some excluded historical test
fixtures still name the family; this build-only audit patch is not a maintained
source migration or a claim that the historical test suite passes.

The runner checks preparation and removal against their exact historical tree
and requires disjoint file ownership. It applies preparation before harness
locking, builds the family-present control, applies the removal, then builds
the family-absent variant. Both source states are hashed, including added
fixtures. Cleanup reverses removal before preparation. Focused regressions
cover wrong-source rejection before output/Cargo, preparation digest mismatch,
control-family presence and clean restoration after failed metadata.

Canonical paired qualification passes all 23 selected vectors: eleven roles in
both conditions and the control-only helper. Independent
[verification](b1-row18-qualification/verification.json) checks complete
membership, payload/method hashes, direct section/function counts, interface
boundaries and clean source/lock restoration. Row 18's complete 46-build
repeated measurement now passes and is retained in the
[historical-family ledger](../../../working/0.110-fleet-runtime-contraction/b1-pool-ledger-recovery-hard-cut.md#retained-family-only-result--2026-09-23).
Root loses 95,690 code bytes and 84 functions; the separately deleted helper is
418,859 code bytes and 1,305 functions. Store's bytes are identical. The removed helper also has its earlier
separately verified [qualification vector](b1-row18-helper-qualification/artifact-metrics.tsv)
and [payload/function/source verification](b1-row18-helper-qualification/verification.json):
418,859 code bytes and 1,305 defined functions in one control-only pass.
That earlier qualification alone is not a repeated measurement or a Root/Store
saving; the complete retained matrix now measures the helper separately.
No instruction result is claimed. Current code is not changed by
either audit patch, and this comparison adds no predecessor reader or runtime
compatibility lane.

The first canonical control pass compiled all eleven roles but failed its
source check: Cargo moved the two new fixture entries into lexicographic
lockfile order. Complete package-record comparison verifies no dependency
change. That run is rejected as qualification. The corrected common patch
uses Cargo's canonical ordering; its old/new digests and unchanged records
are retained in the source record. Full qualification restarts with the helper
included. Per-artifact source checks now stop this drift before accepting a
payload or starting the next role.

The runner also separates native tooling from measured Wasm targets. It builds
and hashes the native driver once per exact source condition, then invokes that
executable for both clean measured repetitions. A helper qualification through
the revised method produces the exact same complete metric vector and payload
digests as the original helper qualification. Both checkouts restore cleanly.
The helper verification records both methods; this removes repeated tooling
work without claiming a measured complete-run speedup or warming Wasm targets.

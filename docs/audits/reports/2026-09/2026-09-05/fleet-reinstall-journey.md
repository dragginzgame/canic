# Generated Fleet reinstall journey

This working-tree qualification addresses the September 6 Toko review: a
management reinstall alone does not prove generation, reviewed Ensure execution,
current Fleet readiness or interruption recovery. The complete generated
changed-release journey now passes in 522.04 seconds, including setup, builds,
replacement and replay. This is functional evidence, not a deployment benchmark.

## Maintained workflow

Generation observes the installed Root through management and seals replacement
input without calling its old protected interface. Ensure first produces a
reviewed Root reset prerequisite: Stop, Install with Reinstall mode, then Start.
The existing effect journal retains intent and the pre-install version.

After that prerequisite completes, a separately reviewed full Ensure plan owns
Coordinator/Store replacement, imported-asset reconciliation and bounded current
protocol convergence. The prerequisite alone does not claim Fleet readiness.
No persistent reinstall flag, predecessor reader, temporary Wasm, custody owner
or additional recovery journal is introduced.

## Acceptance case

`generated_reinstall_recovers_lost_install_and_reaches_working_fleet` starts a
real working Fleet with one Workload and one Ready asset, then builds and seals
a different release-build identity. Its replacement input comes through the
actual generator with exact retained canister identities and a fresh Fleet ID.
Previous host plan, journal and state files are discarded.

The case checks controller drift before reset, a lost real install response,
reconstruction of the production adapter, reset replay, full Fleet convergence,
and both original-plan and newly planned effect-free replay. It verifies exact
Root-only pool control, a retained nonzero Root Ledger balance, bounded native
cycle debit and one host install per infrastructure canister.

## Retained evidence

- [Complete PocketIC journey](artifacts/fleet-reinstall/generated-pocketic.log):
  three Root prerequisite effects, eighteen full-plan effects across bounded
  successor phases, and effect-free reset/original-plan/new-plan replays.
- [Fleet host regressions](artifacts/fleet-reinstall/host-tests.log): 138 pass;
  the two governed cases remain excluded from the native invocation.
- [Scoped warning-denied Clippy](artifacts/fleet-reinstall/clippy.log): passes.
- [Fleet CLI regressions](artifacts/fleet-reinstall/cli-tests.log): 47 pass,
  including plan input, reporting and terminal Fleet discovery.
- [Canonical Coordinator Candid](artifacts/fleet-retirement/coordinator-protocol-tests.log):
  five pass, including structural equality with the Rust receipt types.
- [Final check records](artifacts/fleet-reinstall/checks.json): log hashes and
  selected source digests captured at closeout, with the subsequent test-only
  explicit `HashSet::default()` correction recorded separately.
- [Protocol-surface Clippy](artifacts/fleet-reinstall/protocol-surface-clippy.log):
  the corrected test target passes with all features and warnings denied.
  Comparison behavior and runtime source are unchanged; existing behavioral
  evidence remains applicable.
- [Combined test-target Clippy](artifacts/fleet-reinstall/affected-package-clippy.log):
  all targets/features of `canic`, `canic-host` and `canic-tests` pass after
  extracting JSON fixture setup and releasing the native delegation PocketIC
  fixture after its last use. The
  [JSON regression](artifacts/fleet-reinstall/json-report-test.log) passes;
  product behavior and the generated reinstall fixture remain unchanged.

The Root Ledger account remains at 1B cycles. The aggregate native balance of
Coordinator, Root, Store and both pool assets is nonincreasing, with debit below
10T across replacement. Exactly three host installs occur: Root, Coordinator
and Store, each once. The final pool contains one Workload, one Ready asset and
zero PendingReset assets, with an active Registry and exact current inventory.

## Corrections exposed by this journey

- The retained-estate seed must omit fresh creation-fee authority.
- Starting a canister advances its management version. The test checks actual
  install-call count to prove no duplicate reinstall.
- An imported workload can remain installed while its current Root records
  `PendingReset`. Balance accounting must include it under exact Root-only
  control; fresh creation and funding keep their empty-module requirements.
- A sealed infrastructure phase cannot treat pending imports as a demand for
  additional creation slots. Creation funding waits for real Ready receipts
  after reconciliation, within the existing bounded continuation contract.

The superseded Store-adoption plan validator, predecessor-module checks,
special retry and compatibility action ordering have also been removed.

## Limits

This case qualifies the tested retained controller and topology contract. It
does not qualify a live Toko installation, arbitrary controller layouts, Fleet
deletion, cross-release state preservation or mainnet timing. Full evacuation
is conditional on deleting the Fleet and is not a prerequisite to reinstall.

The accepted correction batch is ready for release approval. See the
[batch handoff](fleet-feedback-readiness.md) for release-process boundaries,
optional follow-ups and the Toko adoption checklist.

# Fixture build throughput — 2026-09-18

Base: `933a35b66403f6a94f99803252cc52c0aec32958` (`v0.110.24`), preserving
the existing .25 planning and duplicate-test cleanup work. This follow-up changes
fixture construction and adds host-owned package preparation. Production runtime
semantics, dependencies, package versions and downstream repositories are unchanged.

## Late generated inputs

The initial exact native-child recovery run failed before starting PocketIC
effects. It spent 410.198 seconds constructing artifacts, then rejected cache
commit with `CargoBuildInputsChanged` for `literal-zero-release-cargo`.
The runner took 421 seconds; this failed run is not performance evidence for a
successful recovery journey.

The selected `root_probe` package includes its generated `.canic` subtree.
Canonical Root preparation already preceded the input snapshot, but Coordinator
and Store manifests/locks were materialized later during compilation. Their
observed modification times fall after the snapshot. The initial failure did
not retain individual before/after input hashes; the subsequent cold-directory
regression independently reproduces the invalidation caused by this ordering.

The cache recipe now prepares and admits both infrastructure packages before
snapshotting enclosing Cargo inputs. The host's existing package generation and
contract checks remain the owners. No exclusions, retry-on-mutation, weakened
fingerprint comparison or reuse across release identities were added.
The regression proves that late preparation changes the frozen inputs, early
preparation is repeatable without compilation, and a later source edit still
invalidates the snapshot.

## Avoided canonical Root builds

`native-child-recovery.toml` and `retained-estate.toml` installed an audit Root
after first building a canonical Root whose artifact was discarded. Their
fixture builder now omits that canonical Root's declaration, Candid extraction,
runtime compilation and finalization. The audit Root still uses the admitted
canonical role contract, exact selected release, existing feature selection,
declaration/runtime passes, shrink, public Candid metadata, endpoint validation
and gzip. Normal canonical-Root fixtures keep their existing build path.

The complete native-child release set has **21 byte-identical outputs** between
the reference and candidate: all Wasms, gzip files, Candid files, release plan,
manifests and fixture content. The comparison uses the repaired snapshot ordering
on both sides. It does not compare against an older published measurement.
[Artifact sizes, raw SHA-256 values, cache fingerprints and candidate source hashes](fixture-build-artifact-parity.json)
retain the comparison. Reference artifacts remain locally at
`/tmp/canic-direct-audit-root-reference`; no new measurement framework was added.

## Focused qualification

All selections below pass. PocketIC cases ran separately through
`make test-pocketic-case CASE=<exact test name>`; no broad suite ran.

| Check | Result | Local log |
| --- | --- | --- |
| Host `generated_infrastructure_inputs_are_stable_before_compilation` | 1 pass, 4.01 s execution | `/tmp/canic-infrastructure-preparation.log` |
| Reference `native_withdrawal_recovers_the_same_initial_child_claim` with early preparation | 1 pass, 108.56 s execution; artifact-build phase 38.563 s | `/tmp/canic-direct-audit-root-prepared-baseline.log` |
| Candidate same native-child case | 1 pass, 83.64 s execution; artifact-build phase 13.934 s | `/tmp/canic-direct-audit-root-candidate.log` |
| `generated_reinstall_recovers_lost_install_and_reaches_working_fleet` | 1 pass, 543.44 s execution; initial and replacement artifact-build phases 152.969 / 36.133 s | `/tmp/canic-direct-audit-root-reinstall.log` |
| Warning-denied Clippy | Host and internal-testing libraries/tests, with `governed-pocketic-tests` | `/tmp/canic-direct-audit-root-clippy.log` |

The retained-estate case preserves 19 Workloads and five Ready assets, distinct
initial/replacement releases, authority rejection, lost-install response recovery,
successor convergence, terminal conservation and effect-free replay. It validates
the second changed configuration without shrinking its accepted topology.
Formatting, whitespace and report-link checks also pass.

The warm native-child artifact phase is 24.629 seconds shorter in this sequential
pair. Cache history and concurrent machine load were not controlled; this is an
observed phase reduction, not a statistically qualified or whole-release speedup.
The retained-estate run included cold artifact work and must not be compared
directly with its warmed .24 full-suite duration. Larger shared host/runtime
fixture isolation remains a separate design opportunity.

The selected .25 planning/fixture batch is ready for the maintainer-selected
release flow. Its open changelog is updated; no version bump, Git publication,
deployment or live recovery was performed.

# Canic 0.109 B6 Runtime Convergence

Date: 2026-08-23

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Candidate | uncommitted 0.109 B1-B6 working tree on `main`; `v0.108.2-dirty` |
| Release posture | reinstall-only hard cut; same-release interruption and exact replay only |
| Effects | repository source, local build artifacts and local PocketIC only; no remote, paid, canister, Ledger or network effect |

The candidate includes the cumulative open 0.109 worktree. Pre-existing 0.108
changelog compaction remains concurrent maintainer work and is not B6 evidence.

## Sole ownership and durable flow

The Coordinator's memory-ID-64 current-plus-last record is the sole Fleet
policy and transition authority. Each registered Root uses its one
memory-ID-65 distribution journal to derive and drive the exact participants
in its protected Registry mirror. Each managed non-Root target uses its one
memory-ID-61 active/prepared projection and retained receipt. No new timer,
policy cache, participant journal or remote admission lookup was added.

An effective mutation advances monotonically:

```text
Planned -> Preparing -> PerimeterFenced -> Activating -> Opening -> Converged
```

Prepare stores the complete successor and fences the target before returning a
receipt. Registry publication occurs only after every Root is fenced. Activate
replaces the active projection while keeping ingress fenced. Open occurs only
after every Root reports activation. Exact response-loss replay returns the
retained receipt; it does not repeat a target effect. A current operation is
restored and resumed before another mutation can begin.

## Participant-catalog binding

`canic admission plan` is read-only and retains the complete sorted managed
target catalog for every Root. The plan binds the predecessor Registry,
predecessor and successor policy, selector, Principal, exact target bindings,
aggregate participant-catalog digest/count and derived operation identity.
`apply` re-reads the protected live catalogs and refuses a changed plan before
sending the command.

The public Coordinator request carries only the fixed digest and `u32` count,
not an unbounded catalog vector, so the command remains within the 16 KiB role
envelope. Each Root snapshots the complete target bindings and successor
projections it derives from its protected Registry mirror. The Coordinator
retains each exact Root receipt and accepts the aggregate only when the sorted
Root/digest/count tuples reproduce the plan-bound authority. A mismatch leaves
the predecessor Registry unpublished and the operation in `Preparing`.

The live catalog remains the authority for an idle Root status response; it is
not reconstructed from a stale last-result record. Component allocation is
rejected while a transition is active. After convergence, a newly provisioned
Component starts with the current generation and opens only through its normal
managed activation.

## Public and composed surfaces

The role contracts and replay manifests include `MutateAdmission`, the three
Root and target prepare/activate/open commands, protected admission status and
operation status. The canonical Coordinator Candid exposes the compact
`participant_catalog_digest : blob` and `participant_count : nat32` request;
it exposes no participant-catalog vector. CLI subcommands remain ASCII ordered
as `apply`, `plan`, `status`, and Medic reports convergence without storing or
deciding policy.

The managed IcyDB fixture now exercises the actual
`#[icydb::request_execution]` wrapper together with `#[canic_update(public)]`.
Its handler calls `canic::fleet_admission::require_caller()` before application
work and observes the same transport caller and local projection as the Canic
endpoint predicate.

## Bounds

- Coordinator admission current plus last: 4,096 Root rows in each retained
  operation, 8,192 total, 2,055,610 encoded bytes in memory ID 64's 8 MiB.
- Coordinator Registry admission history: 4,096 immutable publication rows,
  5,565,526 encoded bytes in its existing 32 MiB cell.
- Root transition journal: memory ID 65, 8 MiB.
- Managed target projection: memory ID 61, 32 KiB.

The capacity checks use the actual stable codecs and maximum permitted
identities, digests, policies and progress rows.

## Targeted qualification

The final B6 source passed:

```text
cargo test -p canic-core fleet_admission -- --nocapture
# 30 passed

cargo test -p canic-control-plane fleet_admission -- --nocapture
# 9 passed; maximum Coordinator record 2,055,610 bytes

cargo test -p canic-control-plane \
  stable_root_journal_replays_every_phase_without_a_second_target_effect \
  -- --nocapture
# 1 passed

cargo test -p canic-cli admission -- --nocapture
# 5 passed

cargo test -p canic \
  fleet_coordinator_candid_contains_protected_admission_and_funding_protocol_types \
  -- --nocapture
# 1 passed

cargo clippy -p canic-core -p canic-control-plane -p canic-cli \
  -p canic-testing-internal -p canic --all-targets -- -D warnings
# passed
```

The governed targeted PocketIC runner passed these final-source journeys:

```text
bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::fleet_admission_add_and_remove_converge_across_real_root_and_components
# 1 passed in 315s cold; 408,328 kB high-water, 97 threads

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::fleet_admission_add_and_remove_converge_across_two_roots
# 1 passed in 21s warm; 565,636 kB high-water, 19 threads

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::unavailable_admission_participant_blocks_activation_until_exact_retry
# 1 passed in 16s warm; 434,144 kB high-water, 99 threads

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::lifecycle::tests::composed_framework_guard_matches_canic_endpoint_on_direct_ingress
# 1 passed in 33s; 327,472 kB high-water, 19 threads
```

The one-Root journey proves add, terminal replay and remove across two real
managed Components. The two-Root journey restarts the Coordinator once at each
retained `Preparing`, `PerimeterFenced`, `Activating` and `Opening` boundary,
then removes the Principal across both Roots. The unavailable-target journey
keeps the predecessor Registry authoritative, fences concurrent allocation,
resumes the same operation after the target returns and provisions a new
Component with the converged generation. The composed journey proves the real
IcyDB request wrapper's direct-ingress ordering.

The complete maintainer validation gate and external Toko adoption were not
run. Repository policy reserves the broad gate for explicit maintainer
authorization, and the downstream workspace remains read-only.

## Result

B6 is ready within its accepted boundary. Live additions and removals no
longer stop at `Planned`; they converge through one exact replay-safe operation
without a second authority. B7 still owns security closeout, generated-surface
and residue review, final measurements and read-only Toko adoption. Therefore
0.109 is not yet ready for publication or minor closeout.

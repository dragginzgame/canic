# Canic 0.109 B7 Security Closeout And Propagation

Date: 2026-08-23

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Candidate | uncommitted 0.109 B1-B7 working tree on `main`; `v0.108.2-dirty` |
| Canic effects | repository source, documentation, local build artifacts and local PocketIC only |
| Toko effects | read-only inspection only; no source, dependency, artifact, canister, network or deployment mutation |

B7 is a focused implementation closeout, not the human-owned minor closeout
audit and not the complete maintainer validation gate.

## Current downstream trace

The read-only Toko Miner checkout was `main` at
`e61c15b54afd04744611724408dcceeae65dab7d`, described as `v0.1.6-dirty`, with
160 pre-existing changed paths. The inspected working files had these Git blob
identities:

| File | Blob identity | Current fact |
| --- | --- | --- |
| `apps/toko_miner/app/src/lib.rs` | `4abf6dfeb04cfc5b77f13c7b52e03b0117dd76e5` | singleton managed App composes `canic::start!` with IcyDB and still uses the 0.108 login predicate |
| `apps/toko_miner/app/src/robot/mod.rs` | `91caaf4ade6e28a9ca59dcb8a2185470065cbed8` | five caller-owned IcyDB methods derive all User/Robot authority from the transport Principal |
| `apps/toko_miner/app/src/observability/mod.rs` | `de6f43b6914da8dccd20105def6e1662bcb05afd` | public observability and exact controller refresh stay outside player admission |
| `apps/toko_miner/canic.toml` | `33c8040ebb79398bad60c5e51f1c134c648339c5` | one singleton managed App; old 0.108 whitelist remains until published adoption |
| `docs/upstream/canic.md` | `0cc84c4c5eec6afcb43832074ff3875a43573470` | CANIC-024/025 record the current adoption and real-wrapper requirements |

Toko has hard-removed its former Core. The current protected application
surface is one browser-login Canic endpoint plus these five direct
`#[icydb::request_execution]` endpoints:

- `get_my_robot`;
- `enroll_my_user`;
- `set_my_robot_username`;
- `set_my_admin_see_everything`; and
- `update_my_robot_appearance`.

They currently reject only anonymous callers before caller-derived domain
work. In downstream-owned adoption, browser login can hard-cut to
`caller::is_fleet_admitted()`, and each protected IcyDB body can use the
Principal returned by `canic::fleet_admission::require_caller()` instead of
reading `msg_caller()` independently. Toko's `Principal -> UserPrincipal ->
UserId -> Robot`, bootstrap-administrator and resource checks remain separate.
Public catalog/observability methods and the controller-only refresh do not
become player-admission endpoints.

## Participation and local-development decision

The downstream closeout feedback requires an explicit role enrollment for
composed frameworks. `[roles.<role>] fleet_admission = true` is therefore the
sole declaration that selects the projection capability and enrolls every
managed instance of that role in convergence. Omission selects no memory-ID-61
state, init projection, generated admission command/status surface or Root
participant entry. Root declarations reject the flag. This does not
blanket-guard every endpoint; each method remains explicitly public,
Fleet-admitted, application-member, trusted-service or infrastructure-
authorized.

The protected plan retains the canonical sorted enrolled-role list for every
participating Component Spec. Managed init requires projection presence to
match the compiled declaration exactly, and a Root with zero enrolled targets
advances no-effect prepare/activate receipts through its existing durable
journal rather than blocking Fleet convergence or acquiring another owner.

`canic::start_local!` has no managed projection. Consequently
`caller::is_fleet_admitted()` and `require_caller()` fail closed there. A
consumer may visibly select its own local-only branch, as Toko already does for
browser login, but Canic does not turn build locality into implicit production
authority or persist a local bypass in Fleet state.

## Surface and residue reconciliation

- Coordinator and Root enums contain their exact admission variants; only an
  enrolled managed role emits target admission command/status variants. The
  replay manifest classifies every mutation command by its durable operation
  identity.
- The canonical Coordinator Candid carries the fixed participant-catalog
  digest/count request and all protected status phases. Generated Root/managed
  role surfaces match the source contracts.
- `canic admission apply`, `plan` and `status` are ASCII ordered. The plan
  retains full per-Root catalogs; Medic consumes status without mutation or a
  second policy decision.
- Runtime-whitelist modules, DTOs, storage, commands and predicates are absent.
  Remaining source references occur only in negative hard-cut tests; 0.107
  design/evidence and the 0.109 problem statement retain truthful historical
  vocabulary.
- The focused role-contract sweep found one stale expected Root allocation
  list that omitted B6 memory ID 65. The runtime allocation and collision gates
  were already correct; the expectation now includes 65 and the complete
  21-test role-contract filter passes.

## Focused validation

Final-source checks include:

```text
make current-document-semantics-gate
# passed

make layering-gate
# passed

cargo test -p canic --test changelog_governance -- --nocapture
# 1 passed

cargo test -p canic --test managed_endpoint_gate -- --nocapture
# 6 passed

cargo test -p canic --test protocol_surface -- --nocapture
# 41 passed

cargo test -p canic-core role_command -- --nocapture
# 14 passed

cargo test -p canic-core role_contract -- --nocapture
# 21 passed after correcting the stale memory-ID-65 expectation

cargo test -p canic-host protected_admission -- --nocapture
cargo test -p canic-host admission_rejects -- --nocapture
cargo test -p canic-host \
  fresh_fleet_preflight_rejects_unknown_admission_topology -- --nocapture
cargo test -p canic-host \
  complete_decision_digest_binds_funding_and_admission_policy -- --nocapture
# all four focused host checks passed

cargo test -p canic-cli command_family_help_returns_ok -- --nocapture
cargo test -p canic-cli --test subcommand_order -- --nocapture
# both passed

cargo test -p canic-control-plane \
  maximum_admission_publication_history_fits_coordinator_registry_cell \
  -- --nocapture
# 1 passed; 5,565,526 encoded bytes

cargo clippy -p canic-core --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
# all passed
```

B6's final-source warning-denied Clippy, native suites and four governed
PocketIC journeys remain the direct runtime evidence. In particular, the real
IcyDB wrapper case passes in 33 seconds at a 327,472 kB high-water mark; the
two-Root add/remove and every-Coordinator-phase restart case passes in 21
seconds at 565,636 kB; and unavailable-target recovery plus post-convergence
Component creation passes in 16 seconds at 434,144 kB. Exact commands and the
one-Root cold run are retained in `b6-runtime-convergence.md`.

The explicit-enrollment correction was then requalified on its final source:

```text
cargo test --locked -p canic-core fleet_admission --lib
# 34 passed

cargo test --locked -p canic-core \
  empty_participant_catalogs_retain_nonzero_exact_identities --lib
# 1 passed

cargo test --locked -p canic-control-plane fleet_admission --lib
# 10 passed

cargo test --locked -p canic-control-plane root_admission --lib
# 1 passed

cargo test --locked -p canic-host fleet_install_plan --lib
# 20 passed

cargo test --locked -p canic --test managed_endpoint_gate
# 6 passed

cargo test --locked -p canic --test protocol_surface fleet_admission
# 2 passed

cargo test --locked -p canic-testing-internal --test fixture_payloads
# 4 passed

cargo clippy --locked -p canic-core -p canic-control-plane -p canic \
  -p canic-host -p canic-testing-internal -p canic_icydb_lifecycle_probe \
  --all-targets -- -D warnings
# passed
```

Representative generated builds prove that an enrolled role emits protected
admission status and prepare/activate/open commands, while an otherwise managed
omitted role emits none of those surfaces. The common managed init contract
retains the optional projection field so the Root can pass a projection only
to an enrolled target.

The governed composed-IcyDB wrapper journey passed on freshly rebuilt Wasm in
65 seconds at a 322,288 kB high-water mark and 19 PocketIC threads. The live
Fleet add/remove journey initially exposed a contradictory fixture: the
Component declared enrollment while its Root's compiled mirror omitted it.
The Root mirror was corrected, and the final exact journey passed both add and
remove convergence in 14 seconds with cached artifacts at a 417,400 kB
high-water mark and 97 shared-server threads. This confirms that participant
discovery follows the explicit role declaration through the real Root and
Component artifacts rather than a handwritten test-only catalog.

The complete maintainer validation gate was not run. Repository policy
reserves that gate for explicit maintainer authorization. No downstream Toko
build, test, dependency change or direct-call qualification was run; adoption
remains blocked on an immutable published Canic release.

## Result

B7 is ready. B1-B7 now form one coherent 0.109 candidate with current
downstream evidence, bounded generated/public surfaces and no retained
whitelist authority. The next action is the human-owned 0.109 closeout audit.
0.110 must not begin until the maintainer accepts that audit verdict.

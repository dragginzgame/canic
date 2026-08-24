# Canic 0.109 B2 Protected Policy Compilation

Date: 2026-08-23

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Candidate | uncommitted 0.109 B1/B2 working tree on `main`; `v0.108.2-dirty` |
| Release posture | reinstall-only hard cut; no 0.108 admission migration or compatibility decoder |
| Effects | source, generated Coordinator Candid and documentation only; no remote or paid effect |

The pre-existing 0.108 changelog compaction in `CHANGELOG.md` and
`docs/changelog/0.108.md` is preserved as concurrent maintainer work and is
not B2 evidence.

## Compiled authority path

Generation-one policy now follows one deterministic path:

```text
protected Fleet input
  -> host validation and canonical template digest
  -> no-effect preflight and per-Root effective projection summaries
  -> exact allocated Fleet binding and immutable install plan
  -> Coordinator install journal and init argument
  -> Registry genesis authority
```

The authoritative identities and invariant checks live in
`canic-core::{ids,model,policy,ops}`. Host parsing does not decide membership;
it converts strict boundary data into the named model input and invokes the
central compiler. The Registry and Coordinator independently validate the
retained digest and exact Fleet binding before accepting it.

Generation one accepts only `component_spec` and `fleet_subnet_root` narrower
selectors because an exact Component instance does not exist at fresh-input
time. Unknown targets, anonymous Principals, duplicate/noncanonical entries,
widening, wrong Fleet binding and unsupported generation all reject before an
install effect. Each immutable Root plan retains a deterministic summary for
every configured component, binding the exact target, effective members and
template digest without rendering the member list in ordinary CLI output.

## Hard cut and staged runtime boundary

`AppConfig::whitelist`, `[app.whitelist]`, the bootstrap seed and the config
accessor are removed. Every checked-in active configuration now supplies the
required protected `[admission]` input. A strict negative schema test proves
the removed table is rejected rather than ignored or accepted through an
alias.

B2 intentionally does not claim the B4 runtime hard cut. Fresh managed
non-Root runtime bootstrap initializes the retained 0.108 whitelist record
empty, so no protected caller is admitted from removed config while B3/B4 are
under development. B4 owns deletion of that record, its DTO/role variants and
`caller::is_whitelisted()`, followed by the new managed projection and local
predicate.

## Focused evidence

The following commands passed on the recorded working tree before final B2
closeout:

```text
cargo check --locked -p canic-core
cargo check --locked -p canic-host -p canic-control-plane -p canic-cli
cargo test --locked -p canic-core --lib --no-run
cargo test --locked -p canic-host --lib --no-run
cargo test --locked -p canic-control-plane --lib --no-run
cargo test --locked -p canic-cli --no-run
cargo test --locked -p canic-testing-internal --lib --no-run
cargo test --locked -p canic-core --lib fleet_admission
cargo test --locked -p canic-host --lib admission
cargo test --locked -p canic-host --lib fleet_install_input::tests
cargo test --locked -p canic-host --lib fleet_install_plan::tests
cargo test --locked -p canic-core --lib config::schema::tests
cargo test --locked -p canic-control-plane --lib protected_init_commits_exact_genesis_and_supports_exact_retry
cargo test --locked -p canic-cli --lib deploy_plan_text_avoids_apply_safety_claims
```

The canonical Coordinator interface was refreshed with:

```text
CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host --example build_artifact --locked -- fleet_coordinator debug . . apps/test/canic.toml --refresh-canonical-did
```

It contains the exact policy, rule and selector data in init and Registry
surfaces and adds no runtime admission command ahead of B3.

Final B2 closeout added these passing commands:

```text
cargo fmt --all -- --check
make layering-gate
cargo clippy --locked -p canic-core -p canic-host -p canic-control-plane -p canic-cli -p canic-testing-internal --all-targets -- -D warnings
cargo test --locked -p canic-core --lib ops::fleet_registry::tests
cargo test --locked -p canic-host --lib coordinator_install
cargo test --locked -p canic-host --lib release_set::tests
cargo test --locked -p canic-cli --lib deploy::tests::plan
cargo test --locked -p canic-core --lib fleet_admission
cargo test --locked -p canic-host --lib admission
git diff --check
```

The warning-denied pass first identified a missing panic contract in one test
helper, a checked test-index conversion, one long whole-journey fixture and a
large CLI transport enum. The final pass is clean after documenting the test
helper, making the conversion checked, applying a reasoned expectation to the
whole-journey fixture and boxing the internal Registry response fragment.

The complete Registry subset first exposed two intentionally frozen canonical
hash expectations that still represented the pre-admission encoding. Their
new expected values were taken from the deterministic canonical encoder after
the Registry began retaining the generation-one admission authority; all 20
Registry tests pass on rerun.

The complete maintainer validation gate and PocketIC runtime matrix are
deliberately not B2 substitutes: repository policy reserves the broad gate for
the maintainer flow, and B4/B6 own executable local enforcement and
convergence.

## Result

B2 is implementation-complete but not a release boundary. It supplies one
canonical protected-input and install-time authority for B3; it does not create
a second policy owner, runtime journal or compatibility lane. B3 must preserve
this exact compiled authority when it adds the sole Coordinator-owned mutable
record.

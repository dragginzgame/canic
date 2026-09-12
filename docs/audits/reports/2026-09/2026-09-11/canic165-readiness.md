# CANIC-165 receipt-gated readiness

Application dispatch and Root membership now require the fixture receipt for the
exact installed selection. Initial child bootstrap remains infrastructure work:
it can allocate, configure and grant a child before the parent's application data
is ready. This is targeted implementation evidence, not a release or closeout audit.

## Maintained behavior

Protected `canic_observability(Readiness)` carries the typed fixture result.
Overall readiness combines the existing runtime barrier with validated fixture
completion. Missing importers, pending progress, invalid receipts and retained
failures cannot satisfy the data prerequisite. Public health and bootstrap
observations remain infrastructure observations.

Application queries and updates are fenced before their handlers run. The exact
Canic configuration/status names and call modes remain available with their
existing authentication; a `canic_` application name has no exemption. Sharding
assignment also checks the parent's fixture prerequisite when called directly.
Initial child bootstrap does not pass through that application assignment gate.

Root reconstructs the expected fixture from its retained installation plan,
checks the installed assignment, and compares the observed receipt's complete
binding and completion summary before admitting Component or child membership.
A target cannot substitute its own selection as the expected authority. No
additional readiness flag, persisted receipt or application cursor is introduced.

The data gate reads the protected assignment without constructing and hashing
full Directory status. In the eight-row probe, commit/validation/receipt replay
measure 20,419,797 / 21,638,964 / 14,575,624 instructions. These are measured costs,
not production ceilings or a new instruction cap. The preceding consumer proof
measured 19,535,502 / 20,733,701 / 13,675,822 before the new gate.

## Qualification

- [Store/IcyDB cases](canic165-readiness-evidence/consumer.log): five pass in
  124.68s (175s runner including compilation). Actual application calls reject
  during outage, bad checkpoint and invalid receipt paths. Completion admits
  queries and remains admitted after revocation/restart. A permanent failure
  remains gated across restart. Raw test instrumentation retains controller auth.
- [Prepared-Root Hub/Shard case](canic165-readiness-evidence/bootstrap.log): one
  passes in 452.11s (503s runner including compilation and uncached artifacts).
  Both application imports are held; the initial child still appears while Root
  remains Prepared. Releasing the child yields its durable receipt while the
  parent stays held. Repeated observations preserve the selected targets.
  Releasing the parent reaches terminal membership with exact Store grants and
  receipts. Repeating one account assignment returns the existing initial Shard;
  pool workload count is unchanged. Existing terminal replay checks also pass.
- [Twenty-seven native checks](canic165-readiness-evidence/native-tests.log) pass
  on final source for endpoint classification, exact assignment/receipt binding,
  importer invariants, protected status and membership authority substitution.
- [Scoped Clippy](canic165-readiness-evidence/clippy.log) passes with warnings
  denied, including final core, Root and bootstrap test code. Scoped formatting
  and `git diff --check` also pass.

Both real-canister runs preserved all 1,633 Rust/config/Candid input hashes and
the source inventory. The [consumer snapshot](canic165-readiness-evidence/consumer-source.sha256)
precedes the final Root-plan authority tightening and additional bootstrap-only
assertions; those changes affect only the Root provisioning source and its
baseline test. The [Root snapshot](canic165-readiness-evidence/source.sha256)
covers the final runtime implementation. Exact commands and source lineage are
recorded in [commands.json](canic165-readiness-evidence/commands.json).

## Remaining batch work

Next are later Shards using retained sources after the operator exits, interrupted
grant effects and automatic in-flight source/reinstall recovery. Reviewed retry
funding, permanent failure classification, revocation before pool reuse, reference
release and terminal Store retirement remain. Current GC blocking is conservative;
fixture-bearing Fleet generation remains disabled until that lifecycle is complete.

CANIC-165 and the combined worktree are not push-ready. Its changelog remains in
root Unreleased; no version, Git publication, deployment or sibling edit occurred.

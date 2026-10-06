# Shared Tooling adoption

The [manifest](../../.shared-tooling.snapshot) records eighteen files from clean
reviewed revision `c0206f1943238e21bd00fbe01658e6a0864c24fa`.
[Root AGENTS.md](../../AGENTS.md) retains the product overlay. Refresh only through
the upstream distribution helper; verify the snapshot before release validation.

Standard `make release-patch`, `release-minor` and `release-major` use the
[common release contract](../releases.md), with `RELEASE_REMOTE=origin` and
`RELEASE_BRANCH=main`. Every standard increment runs complete validation. The
runner owns the Git effects and retained recovery plan; consumer adapters retain
Cargo/installer metadata and source-bound release evidence. Package publication
and artifact cleanup remain separate. The explicit non-runtime fast preparation
lane remains independently governed; it is never selected by a standard command.

After a preflight or validation-only failure, correct and commit the source, then
rerun the same standard release target. It repeats preflight and complete
validation against current source. An older preparation-free plan is preserved
unchanged in a unique `.git/release-state/X.Y.Z.attempt.*/` directory; do not delete
it or require exact-source resume merely because validation failed. The runner
checks the base version, destination, saved tree/file set and local/remote tags
before accepting that restart. New durable plans begin immediately before
preparation, after validation passes.

Once preparation has begun, inspect `.git/release-state/X.Y.Z.plan` and its lock owner before
`make release-resume VERSION=X.Y.Z`. Resume retains source, candidate, UTC date
and destination, reconciling replies lost after commit, tag or push.

Snapshot integrity and release-runner adoption do not establish complete baseline
implementation. The reviewed upstream formatting hook rejects any tracked
symlink, including Canic's historical audit links. Canic retains its existing
hook and installer until that integration is resolved; neither is declared in
this snapshot. No historical evidence links were removed to accommodate the
hook. Native macOS qualification remains separate from Linux command-stub checks.

Canic uses published registry `ic-metrics 0.1.5` with feature `ic` for shared
arithmetic and the Wasm call-context reader. Native zero and exclusive endpoint
attribution remain consumer-owned; no sibling checkout is required.
Four endpoint-accounting tests and strict selected core Clippy passed during
extraction. Command stubs exercise standard entry points and runner recovery;
Linux passes do not qualify native macOS or live IC measurements.

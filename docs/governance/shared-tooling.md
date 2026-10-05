# Shared Tooling adoption

The [manifest](../../.shared-tooling.snapshot) records sixteen files from clean
reviewed revision `b8537873ac124ad17b30e32aa23e9006a3e6ec21`.
[Root AGENTS.md](../../AGENTS.md) retains the product overlay. Refresh only through
the upstream distribution helper; verify the snapshot before release validation.

Standard `make release-patch`, `release-minor` and `release-major` use the
[common release contract](../releases.md), with `RELEASE_REMOTE=origin` and
`RELEASE_BRANCH=main`. Every standard increment runs complete validation. The
runner owns the Git effects and retained recovery plan; consumer adapters retain
Cargo/installer metadata and source-bound release evidence. Package publication
and artifact cleanup remain separate. The explicit non-runtime fast preparation
lane remains independently governed; it is never selected by a standard command.

Inspect `.git/release-state/X.Y.Z.plan` and its lock owner before
`make release-resume VERSION=X.Y.Z`. Resume retains source, candidate, UTC date
and destination, reconciling replies lost after commit, tag or push.

Canic uses explicit local `ic-metrics` integration pending package publication.
Four endpoint-accounting tests and strict selected core Clippy passed during
extraction. Command stubs exercise standard entry points and runner recovery;
Linux passes do not qualify native macOS or live IC measurements.

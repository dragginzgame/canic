# Developer Helper Scripts

Scripts in this directory are local developer and operator helpers. They are
kept intentionally even when they are not called by the `canic` CLI or CI.

Do not remove these scripts just because they look unused from automated
workflows. They cover manual setup, local measurement, and occasional repository
maintenance tasks.

- `gh-ci.sh` is an optional maintainer helper for inspecting GitHub Actions CI
  with an authenticated local GitHub CLI session. It is not required for normal
  Canic development or CI. Its shared implementation can select exact committed
  source with `--commit HEAD --all-workflows --limit 100`; this bounded list is
  evidence for inspection, not a complete CI verdict. `--failed` searches
  historical failures even when later runs passed.

Tag maintenance uses `perl scripts/dev/delete-github-tags-up-to.pl` with an
explicit cutoff. Review its dry-run output before selecting deletion; no default
cutoff is inferred. See [the shared maintenance contract](../../docs/tag-maintenance.md).

## Continue From Here

- [Read the testing rules](../../TESTING.md)
- [Review CI and deployment governance](../../docs/governance/ci-deployment.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)

Repository IC and parser provisioning comes from the reviewed Shared Tooling
snapshot: `make install-tools` installs explicitly; `make tools-check` verifies
offline. The matrix in `ci/ic-tools.tsv` owns common versions and archive digests.

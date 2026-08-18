# Developer Helper Scripts

Scripts in this directory are local developer and operator helpers. They are
kept intentionally even when they are not called by the `canic` CLI or CI.

Do not remove these scripts just because they look unused from automated
workflows. They cover manual setup, local measurement, and occasional repository
maintenance tasks.

- `gh-ci.sh` is an optional maintainer helper for inspecting GitHub Actions CI
  with an authenticated local GitHub CLI session. It is not required for normal
  Canic development or CI.
- `update-icp-cli-pin.sh` resolves the latest stable ICP CLI release in the
  currently supported major line, records the official Linux/macOS archive
  checksums in `tool-versions.env`, and aligns installation guidance. It
  refuses an automatic major-version transition. `make update-dev` runs it
  before installing the resulting exact pin.

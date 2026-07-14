# Modular Audit Playbooks

This directory owns the reusable Module Surface Hardening policy and its
implementation workflow. It does not contain dated audit results.

- [module-surface-hardening.md](module-surface-hardening.md) defines the
  `MSH-2.0` authority, exposure, deletion-pressure, and runtime-shape policy.
- [module-cleanup-runner.md](module-cleanup-runner.md) is the shorter workflow
  used only when implementation cleanup has been explicitly requested.

Store every resulting report under the dated
[report archive](../reports/README.md). Do not add one-off module findings or
generated evidence to this directory.

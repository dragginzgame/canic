# Current Status

Last updated: 2026-07-14

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the source, design, audit, or changelog files needed for the current task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.91.6`.
- `v0.91.6` is published at commit
  `5f7a89f9b966ebf2755d5630ddcba0cdf968ebb1`.
- The accepted line design is
  [0.92 holistic audit and audit-system validation](../design/0.92-holistic-audit-and-audit-system-validation/0.92-design.md).
- Detailed published release notes remain in the
  [0.91 changelog](../changelog/0.91.md).

## Current Decision

The 0.92 design treats Canic as feature complete for this line, audits and
improves the audit machinery first, then runs the retained improved suite
across the actual repository and fixes only accepted findings. It incorporates
immutable snapshot/method identities, independent result state machines,
execution safety, a post-freeze defect protocol, deterministic finding
identity, dual comparisons, constrained P1 waivers, and an executable
`v0.91.6` contract. This remains an audit/hardening authorization, not a 1.0
readiness claim.

Pre-1.0 removals remain hard cuts. Do not add aliases, compatibility wrappers,
duplicate command paths, deprecated APIs, or fallback behavior unless the
maintainer explicitly requests it. Named build environments resolve through
`icp.yaml`; only `local` and `ic` are implicit, and no staging/mainnet
aliases exist.

Toko mint remains downstream-owned. Canic provides generic primitives only;
automated work must not edit the Toko repository or move mint-specific
requests, receipts, evidence, retry, cancellation, or tests into Canic.

## 0.91 Outcome

- `0.91.0` added canonical lowercase snake_case role admission and bound root
  release-set publication to one complete build's exact outputs.
- `0.91.1` added the root-only issuer-readiness facade
  `AuthApi::provision_chain_key_delegation_proof_for_issuer_root` without
  restoring retired delegation-proof APIs.
- `0.91.2` updated allocation governance to `ic-memory 0.11.1` as a
  destructive reinstall boundary and rejects unsafe release-set artifact
  paths at admission.
- `0.91.3` bounded the audit archive and removed redundant generated exports.
- `0.91.4` made cost-guard settlement atomic, preserved snapshot restart
  causes, and made a failing installed `ic-wasm` shrink command fatal.
- `0.91.5` made ICP refill admission atomic and fail-closed, added durable CLI
  retry identity, and specified direct verified refill output.
- `0.91.6` made live conversion JSON match that direct-result contract and
  consolidated deployment output plus backup persistence support.

The accepted
[0.91 closeout audit](../audits/reports/2026-07/2026-07-13/0.91-closeout.md)
remains the published release-line baseline.

## Active 0.92

- Phase A is complete. Its
  [inventory report](../audits/reports/2026-07/2026-07-14/0.92-audit-system-inventory.md)
  found six confirmed P1 audit-system defects.
- Phase B has prepared and targeted-validated every correction. Its
  [hardening report](../audits/reports/2026-07/2026-07-14/0.92-audit-system-hardening.md)
  has `run_result: partial` only because the improved method snapshot is not
  committed and frozen.
- One canonical [method catalog](../audits/METHODS.md) owns 22 active
  definitions: 14 system, 7 authentication, and 1 manual-only module-surface
  method.
- The three competing access/ops/workflow purity definitions are merged into
  `CANIC-LAYERING-001/v1`.
- Standing 0.62 audit verdict docs and their literal CI guards are hard-cut.
  Current operator gates and dated release-line evidence own validation.
- Previously missing dependency, build/generated-code/unsafe, CI/security,
  provenance, reproducibility, and host/target topics now have explicit owners.
- Instruction and Wasm execution is offline/isolated, detects source mutation,
  records full method identities, and emits compact hashed/redacted evidence
  manifests.
- The prepared method manifest is
  `fa92c4102efe74391c51f1f829aec7ac9c0b64941da73ee6dad1ebf2b292df07`;
  it is deliberately marked `prepared_uncommitted`.
- The `v0.91.6` product-tree baseline is
  `8fce43e41ce430d9b505e19f8d596ed440b291d4c6ecb19c4a1cfdf71656a9b6`.
  The committed Phase B product-tree hash is pending.
- No runtime/public/serialized/stable/package behavior changed. The removal of
  stale readiness authority is an explicit operator/CI contract change.
- At least three months of real-world use remains a separate prerequisite for
  any future 1.0 discussion.

## Focused Validation

- Audit-method catalog/conformance/fingerprint guard passes for all 22 active
  definitions.
- Affected Bash syntax, release matrix, recovery, package/install, and
  `actionlint` guards pass.
- Both focused instruction method-identity and daily-baseline tests pass.
- `cargo fmt -p canic-tests -- --check` and `git diff --check` pass.
- The committed `v0.91.6` product-tree helper reproduces
  `8fce43e41ce430d9b505e19f8d596ed440b291d4c6ecb19c4a1cfdf71656a9b6`.
- Full workspace, broad PocketIC, deployment, publish, and release suites remain
  maintainer-owned and were not run for this audit-system slice.

## Next Action

Commit and push the complete prepared Phase A/Phase B slice. Then record the
full freeze commit, compute and review its committed product-tree delta, mark
the six findings fixed if that delta matches the declared operator/CI scope,
and begin Phase C. Do not run instruction/Wasm product baselines before the
freeze gate.

# Retired Audit Methods

This manifest preserves the identity of hard-cut methods without keeping an
active alias, wrapper, or duplicate authority. Source remains recoverable from
the immutable `v0.91.6` snapshot at
`5f7a89f9b966ebf2755d5630ddcba0cdf968ebb1`.

| Retired path | SHA-256 at source snapshot | Disposition | Canonical outcome |
| --- | --- | --- | --- |
| `docs/audits/recurring/system/access-purity.md` | `8702826d0d010e1451a91bea2fd2d5eab3dc676c9f90199dadca2490c8a73808` | merge | `CANIC-LAYERING-001/v1` owns access placement. |
| `docs/audits/recurring/system/ops-purity.md` | `8e2262a89e32eb910541fd1b8e7f3a9047863b31835f89070c41dcf35a4fa880` | merge | `CANIC-LAYERING-001/v1` owns ops responsibility. |
| `docs/audits/recurring/system/workflow-purity.md` | `4f1cf645d0bfc65b41f1f553672cf0c1ee7545a2254626bfc54d3eff4f16320c` | merge | `CANIC-LAYERING-001/v1` owns workflow responsibility. |
| `docs/operations/diagnostic-consistency-audit.md` | `266047b36b0ea68323af8d15de80a883b90b287225e00f9ba5f5f534360ec028` | retire | Historical 0.62 conclusion only; no standing audit verdict. |
| `docs/operations/upgrade-state-compatibility-audit.md` | `73e920067058fd345e144edd684d5a065486f7248b5139abc0e9f0b4771e22a6` | retire | Historical 0.62 conclusion only; current stable-state methods own evidence. |
| `docs/operations/rc-readiness-audit.md` | `6515fbfefb47ac8ae98a73e9d4824e7e3fb3acc91436f37475f086d7a97cdbe9` | retire | Current status and dated closeout own readiness decisions. |
| `scripts/ci/check-diagnostic-consistency-audit.sh` | `5e3f2d4acf9ab3f66cf8e6311e37db59bb8db6eaaa8ade2d26d48ac6d88c7c86` | retire | Literal historical-verdict guard removed. |
| `scripts/ci/check-upgrade-state-audit.sh` | `fc3b5891e5e8588ddfd47771e24908e0a92f518e0ee2c46033958598bc220e1c` | retire | Literal historical-verdict guard removed. |
| `scripts/ci/check-rc-readiness-audit.sh` | `8abbb699e822061766d6f9a42b1b0ea735fa746570b3d5e7d5e68d0ba8bf40a5` | retire | Literal historical-verdict guard removed. |

Historical reports and changelogs are not rewritten. When old evidence needs
the exact definition, retrieve the listed blob from the immutable snapshot.

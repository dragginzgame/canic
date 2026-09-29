# Cleanup audit method correction — 2026-09-29

- Finding kind: `audit_method_defect`
- Affected methods: `CANIC-DUPLICATION-001/v1`, `CANIC-MODULE-SURFACE-001/v2.1`
- Corrected methods: `CANIC-DUPLICATION-001/v2`, `CANIC-MODULE-SURFACE-001/v2.2`
- Affected-result disposition: `result_validity: invalid`
- Corrected baseline/current comparison: `blocked` pending paired audit reruns

The cleanup updated DRY search inputs from Project to Workspace evidence and
corrected module-hardening layering and pre-1.0 release policy, but retained the
previous method versions and fingerprints. The catalog gate correctly rejected
those mismatched identities. The corrected versions now bind the maintained
search and policy inputs; the fingerprint manifest preserves both old identities.

Any result relying on the changed Workspace search, policy-to-ops dependency
rule or cross-release compatibility retention under the old methods cannot serve
as corrected-method evidence. Unaffected observations are not invalidated by
this correction. This report supersedes those affected conclusions; it does not
claim new product audit results or an accepted minor closeout.

Before using those conclusions for comparative closeout, rerun the original
frozen product baseline and the current committed source with the corrected
methods, then compare only those results. These manual product audit reruns are
outside this targeted release-gate repair. Executable code regression results
retain their own independent evidence.

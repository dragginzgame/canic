# Toko Miner feedback cleanup — 2026-09-10

The read-only downstream ledger still ends at CANIC-161. Its older `Confirmed`
labels are not a current list of unimplemented Canic defects. This assessment
separates source completion from publication, downstream acceptance and live
recovery. It does not edit the downstream ledger or close another minor.

## Earlier launch blockers

| Feedback | Current Canic disposition | Remaining boundary |
| --- | --- | --- |
| CANIC-007/132 | Published plan-owned funding, creation margin and exact receipts; the four-Workload/four-Ready journey covers lost funding and creation replies, conservation and replay. | Actual downstream retained-estate acceptance remains separate. |
| CANIC-133 | Published complete physical inventory and pending-creation accounting; full-capacity Failed-asset repair has production-host evidence. | No new repair of the user's live estate was performed. |
| CANIC-135 | Published synchronous selected issuer restoration; the retained seven-case native delegation run covers corrupt-state lifecycle rejection and valid same-release recovery. | Historical capability measurements are not complete current B1/B2/B3 acceptance. Toko currently disables delegation. |
| CANIC-136 | Published capability-selected Root delegation, with retained auth-free Root activation and optimized-artifact evidence. | Broad contraction work remains governed by its existing acceptance gates. |
| CANIC-137 | Published generation, Ensure and Medic use the runtime funding-policy authority. The current host four/five-Root-grant regression passes. | Managed application adoption is not inferred from a native test. |

The [completion assessment](../2026-09-05/fleet-feedback-readiness.md)
and [auth artifact assessment](../2026-09-05/auth-feedback-toko-root.md)
retain original evidence and limits. Later refill and repair logs are copied
into this assessment's evidence directory; they are historical runtime
evidence, not reruns today. The [structured receipt](toko-feedback-triage.json)
records current source hashes and comparison with published `v0.110.13`.

## Funding count clarification

CANIC-137's historical wording describes a four-event cap on both Root and
Coordinator. The maintained contract bounds each Root's automatic grant count,
while Coordinator's count and cycles fit the sum of its Roots' allowances.
Workload count is a separate domain.

A new policy regression admits a four-grant Root plus a two-grant Root under a
six-grant Coordinator, then rejects seven grants and cycles above the combined
allowance with exact typed errors. It invokes independent Root and Coordinator
validators as well as aggregate capacity validation. This protects legitimate
multi-Root funding from an accidental universal cap; runtime limits are
unchanged. The [funding runbook](../../../../operations/fleet-funding.md#funding-event-counts)
now explains the distinction. All 17 focused checks pass: eleven host pool and
capacity cases, one host funding-admission case and five runtime-policy cases.

## Remaining work by owner

| Feedback | Next meaningful work |
| --- | --- |
| CANIC-087/139 | Exact dependency-closure reuse after changed inputs. Complete unchanged reuse and the candidate first-build correction are already qualified; finalized artifacts remain bound to their release identity. |
| CANIC-148/153/159 | Release and downstream adoption of the qualified .14 metrics and failure-origin corrections. CANIC-159's candidate runtime proof is complete despite the older downstream label. |
| CANIC-149/150/160 | Downstream end-to-end launcher/progress demonstration and retained-estate timing. Existing Canic renderer and bounded observation results do not establish live downstream timings. |
| CANIC-154–158 | Named downstream recovery/funding-tranche acceptance and separately authorized staging effects. Disposable proofs do not authorize applying the staging review. |
| CANIC-161 | Application-loop repair evidence and installed-policy/post-recovery qualification; see the [funding assessment](canic161-funding.md). No Canic loop defect is established. |
| CANIC-141 | Deferred subnet-bound creation-fee work; no priority change or new funding model here. |
| CANIC-002/008/010/017 | Existing deferred product/design requests, not new runtime regressions. |
| CANIC-014 | Governed release-evidence reconciliation; no version, publication or broad gate is part of this cleanup. |

The next substantial Canic implementation item is CANIC-087/139's changed-input
build reuse. It needs exact dependency evidence, clean-build parity and
release-bound output qualification. Copying finalized Wasms between release
identities would violate that boundary.

This cleanup extends the open .14 documentation/qualification batch. It
preserves prior dirty work and modifies only Canic. The receipt retains focused
validation; no broad suite or live Fleet call is included.

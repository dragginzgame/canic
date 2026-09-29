# Feedback to ic-blob-storage — 2026-09-29

## Reviewed snapshot and verdict

Reviewed the clean local repository at
`4a51992db99d89513e6e8e49187746ac9eb50ae5` (`Release 0.3.0`). Its Cargo
workspace and `docs/release.json` identify 0.3.0; the receipt identifies validated
source `3f814b33e2df37d8b9bbe1d9b50acd6ec767f65e` and `release-verify`.
Registry publication and remote refs were not queried.

The shared durable owners, explicit identity bindings, conservative uncertainty,
and distinction between provider facts and local substitutes fit Canic's needs.
**This snapshot is not ready to replace Canic's blob functionality.** That agrees
with the repository's own M1–M4 and Canic parity assessment. The missing service
journey, operational recovery and managed adapter are acknowledged delivery gaps,
not newly discovered regressions or reasons to weaken their current fences.

This review inspected source and ran one bounded local CLI reproduction using an
existing binary. It did not compile the sibling repository, rerun its suites,
contact a provider, send an external message or modify sibling files.

## Confirmed defect: non-regular CLI inputs can block before validation

Priority: P2, operator robustness.

In [the native CLI reader](../../../ic-blob-storage/crates/ic-blob-storage-cli/src/native/mod.rs),
`read()` calls blocking `File::open` before checking descriptor metadata. A FIFO
with no writer therefore never reaches `is_file()`. The reader is shared by
identity, root-key and saved-cursor inputs. The async network deadline is installed
later and cannot bound this wait.

Reproduction used a fresh FIFO as `funding-history --cursor`, valid explicit local
scope arguments, and nonexistent identity/root-key paths. Cursor loading precedes
both identity loading and network construction. An external two-second timeout
returned **124**, with zero stdout/stderr bytes; the process never returned a typed
file refusal. The temporary directory was removed. No request was made.

The inspected existing binary was `target/debug/blob-storage`, SHA-256
`aeb261d54104ba5745e445558ab5c02c483793c03f2c1e338e9140861d95d576`.
It was not rebuilt or independently bound to the reviewed commit; the source
ordering independently establishes the defect.

Requested fix: open with platform-appropriate nonblocking semantics, then validate
that same descriptor as a regular file before reading. Preserve the byte bounds
and redacted typed failures. A metadata precheck alone leaves a replacement race.
Add a subprocess regression with a FIFO and no writer: the CLI must promptly
return its file error without initializing transport. Retain regular-file,
empty-file and oversize coverage.

## Adoption blockers and requested evidence

### 1. Finish the trusted upload-completion journey

The [standalone endpoint adapter](../../../ic-blob-storage/canisters/standalone/src/api/mod.rs)
exports admission, manifest, reference and inspection operations, but does not
connect the certificate-issuance handler or a production trusted completion path.
The [shared certificate handler](../../../ic-blob-storage/crates/ic-blob-storage/src/workflow/uploads/certificate/mod.rs)
exists and correctly requires independently established host evidence. Library
availability does not yet let a standalone consumer complete the maintained
upload/reference/read/release journey.

Prioritize one bounded vertical slice: exact admission and prepared manifest,
certificate exposure, qualified gateway interaction, trusted completion correlated
to the same service/namespace/upload/object incarnation, first reference, and
verified read. Cover lost final replies, duplicate/stale completion and changed
bindings. Preserve uncertain reservations until authoritative reconciliation;
neither browser progress nor successful client download alone is a trusted
canister completion fact. Keep local substitute results distinct from deployed
provider evidence. This is consistent with the existing next priority, not a
request for another generic probe framework.

### 2. Operational recovery must eventually resume supported work

The [standalone restore path](../../../ic-blob-storage/canisters/standalone/src/ops/mod.rs)
opens the shared owners. [Upload reopening](../../../ic-blob-storage/crates/ic-blob-storage/src/ops/service/uploads/mod.rs)
sets `fenced: true`; funding, gateway and read owners do likewise. The current
contract deliberately has no mutation/unfence capability for restored owners.
That is safe inspection, but it does not yet satisfy supported same-release
operational recovery.

Keep the fences. Supply an explicit reconciliation path whose authority survives
the selected restore boundary, including exclusion of stale concurrent instances.
Acceptance must show an older backup restored after a later effect, no reused
operation identity or duplicated payment, retained obligations, and eventual
resumption of supported operations. Cross-release migration or compatibility is
not requested. Unsupported restore modes must remain explicitly refused.

### 3. Qualify the managed adapter and actual operator parity

The [roadmap](../../../ic-blob-storage/docs/roadmap.md) correctly records M4 as
unimplemented. The standalone host owns its memory runtime; a managed adapter must
instead compose with Canic's granted memory and synchronous lifecycle participant,
using the same shared workflows. Do not link the standalone host into a Component
as a second lifecycle/memory owner or reintroduce blob-specific Canic production
dependencies.

The native CLI currently supports `status` and `funding-history`. Its successful
local-status exit, including fenced observations, is intentionally not the old
readiness check. Before Canic removal, implement and qualify the promised operator
status/check-ready, gateway sync, explicit funding and post-action inspection
against both adapters. Preserve typed JSON, strict full-width decimal amounts,
effect-free dry runs, readiness failure signaling, and no repeated mutation when
post-status fails. A local fixture command is not production operator parity.

## Documentation and handoff follow-through

- Refresh the leading [status handoff](../../../ic-blob-storage/docs/status/current.md):
  it still describes Cargo/receipt 0.2.24 and a 0.3.0 draft, while this reviewed
  checkout and receipt are 0.3.0. Keep the unqualified service milestones explicit;
  release metadata does not close them.
- Preserve the old Canic source inventory as a dated checkpoint, then refresh
  the removal inventory against the actual selected Canic removal candidate.
  Canic's pending cleanup replaces the two inventoried Markdown gates with
  `scripts/ci/check-blob-storage-protocol-evidence.sh` and
  `docs/contracts/blob-storage-protocol-evidence.json`. Shared-file edits and
  generic lifecycle/guard/endpoint coverage still need named replacement owners.
- Canic's 0.110 human closeout and 0.111 removal authority remain separate from
  independent service development. This feedback accepts neither closeout nor
  source removal, deployment, payment or installation retirement.

Suggested order: fix the bounded-file defect alongside the next service slice;
finish trusted completion and its exact recovery contract; qualify that same
journey through the managed adapter and real operator client; then refresh the
source-bound Canic parity/removal evidence. Service guarantees need explicit
qualification before Canic's maintained implementation is removed.

# 0.103 Role-Owned Candid Surface Baseline

This directory freezes the first B1 input: the exact generated Candid surface
at released tag `v0.102.2` (`8cf4723cecd7579cbe3304b980c63b1bc3969d68`). It
does not authorize or describe a runtime/protocol mutation.

## Provenance

- source: released tag `v0.102.2`;
- Rust toolchain: `1.97.1` from `rust-toolchain.toml`;
- Canic build command: `canic 0.102.2`, `fast` profile, local build network;
- Candid extractor: `candid-extractor 0.1.6`;
- Root profile: `delegation_root_stub.root`, including its declared delegated-
  token and role-attestation capabilities;
- managed profile: `delegation_root_stub.issuer`, including its declared
  delegated-token and role-attestation capabilities;
- Coordinator and Store profiles: their canonical checked-in interfaces at the
  same released source tag; and
- build output root: an isolated `/tmp` directory, never the workspace `.icp`
  tree.

The representative Root and managed fixtures deliberately declare a small
number of application-owned test methods. `baseline-methods.tsv` classifies those
separately. It likewise classifies the emitted ICRC-10 discovery method as an
external standard rather than a Canic-owned control method.

The post-B5 current method and variant totals are recorded in the
[B6 representative surface report](b6-surface-report.md). The normalized B1
register and B3 totals remain immutable pre-cut and transitional evidence
rather than current endpoint authority. Raw pre-cut DIDs are deliberately not
retained in the current worktree because they resemble callable current
interfaces; the capture script reconstructs them only in temporary scratch
while deriving the normalized evidence and hashes below.

The [B7 hard-cut closeout](b7-closeout.md) records the final method reduction,
legacy-emitter deletion and representative current Wasm identities without
making an unsupported causal size claim.

## Frozen Counts

`manifest.tsv` is generated from the complete service blocks, not source-text
matches:

| Profile | Total | Canic-owned | External standard | Application-owned |
| --- | ---: | ---: | ---: | ---: |
| Fleet Subnet Root auth fixture | 124 | 117 | 1 | 6 |
| managed auth fixture | 34 | 23 | 1 | 10 |
| Fleet Coordinator | 24 | 24 | 0 | 0 |
| Wasm Store | 25 | 24 | 1 | 0 |

The previous 118-method Root signal is therefore reproduced without using an
application working tree: this profile emits 118 framework/standard methods
plus six fixture-owned methods. The split matters because external standards
and application methods do not consume the proposed Canic-owned role ceiling.

## Files and Reproduction

- `baseline-methods.tsv` contains every method, normalized signature, execution
  mode, endpoint source, compile condition, authorization/payload attribute,
  immediate delegate, replay policy, protocol constant and candidate
  in-repository references.
- `method-register.tsv` begins from that generated evidence and owns reviewed
  disposition decisions. Baseline recapture never overwrites it.
- `manifest.tsv` freezes the transiently generated interface hashes and counts.
- `capture-baseline.sh` rebuilds the source interfaces in temporary scratch and
  recaptures only normalized evidence from a clean `v0.102.2` checkout. It
  never installs a pre-cut DID into the current worktree.
- `capability-manifest.md` freezes the existing closed config derivation,
  invalid-combination boundary, external profile-binding bootstrap, exact
  request/response correlation and reserved names.

Run the capture script from this directory while its first argument names a
clean checkout at the released tag:

```bash
bash capture-baseline.sh /path/to/canic-v0.102.2 /tmp/canic-0.103-b1
```

The script refuses another source commit. It builds only the Root and managed
fixtures, reads the canonical Coordinator and Store DIDs, and writes baseline
audit artifacts atomically. `in_repo_references` is the complete lexical
candidate set. `executable_callers` is the reviewed transport subset: Canic
workflow/ops callers, production host/CLI callers, integration callers and
executable CI callers. Policy lists, presentation inventories, endpoint owners
and source-only assertions remain only in the lexical column.

`rust_signature` freezes the exact released request and success DTO names next
to the Candid signature. For a one-to-one retained variant, those types are the
target payload and success payload unless the final DTO review names a smaller
replacement. The remaining synthesized DTO work is narrow: role `Overview`,
role `Operation`, named envelopes for former multi-argument methods, the two
flattened Fleet/pool command enums and named wrappers for primitive responses.

## Review Boundary

The immutable source/toolchain/generated-Candid input, endpoint attribute,
immediate delegate and replay evidence are complete. The current extraction
also exposes two methods without protocol constants (`canic_bootstrap_status`
and `canic_icp_refill`) and one Root method with no reference outside its owner,
protocol and replay declarations (`canic_wasm_store_bootstrap_debug`). These
were review inputs; the register records the resulting explicit decisions.

The six-way disposition pass is complete for all 188 Canic-owned appearances,
including the bounded B4 correction accepted on 2026-08-17:

- 49 become role-command variants;
- 78 become role-status variants;
- two remain admitted Store byte lanes; and
- 59 become private/delete.

The query pass maps bounded observations to role status except the unreferenced
Root bootstrap debug query. Update review applies the high-level-intent rule:
allocation, provisioning, draining, subtree, registry synchronization, Store
removal and publication phase methods do not survive merely because they cross
an `await` or another canister. Scale-out synchronization is retained as its
own Root outcome because an affected existing Root is not provisioning a new
batch and must return exact convergence evidence. The Root template byte trio
is private/delete because large artifact bytes belong only to the admitted
Store data plane.

Every retained row now has an exact role-specific target name. The one current
update-only management-canister observation becomes the atomic
`RootCommand::InspectCanister`; no ordinary query pretends it can make that
call. Remote composite phase observations collapse into the local
`RootStatusRequest::Operation` lane, so B1 needs no composite-status exception.
The Root's emitted managed-Canister binding query is also deleted: its DTO can
represent only Component or ComponentChild identity, never Root identity.

The status map is deliberately flat: distinct cycle, runtime, auth, directory
and Registry observations are variants, not second-level family enums. The 78
retained status appearances yield 22 Root, six Coordinator, 12 managed and
seven Store targets; Coordinator additionally requires `Overview`, which has
no old-method row. The three unconditional `canic_cycle_topups` appearances in
the captured Root, managed-auth and Store profiles are deleted. Only an exact
managed profile with the new config-derived `AutomaticTopup` capability may
compile `CanisterStatusRequest::CycleTopups`. The two existing nested Root
admin inputs are flattened into their actual Fleet and pool intents. Private
phase rows have no shadow variant.

## Accepted Target Accounting

The target manifest counts variants as well as methods so consolidation cannot
hide the old phase tree inside two method names:

| Role/profile | Canic methods | Command requests | Command responses | Status requests/responses | Durable operation kinds | Atomic command kinds | Old methods eliminated |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Fleet Subnet Root auth fixture | 2 | 32 | 20 | 22 | 12 | 18 | 48 |
| managed auth fixture | 2 | 4 | 4 | 12 | 1 | 3 | 3 |
| Fleet Coordinator | 2 | 9 | 8 | 7 | 2 | 7 | 10 |
| Wasm Store | 4 | 10 | 8 | 7 | 2 | 7 | 3 |

The Store count is command, status and the two independently admitted byte
lanes; its external-standard method remains counted separately. A command
response count includes one shared `OperationAccepted` response when the role
has durable operations. Root has 14 asynchronous command variants sharing 12
operation kinds; Coordinator has two, managed has one and Store has three.

The corrected Coordinator count retains one Root-authenticated Registry
acknowledgement plus the two controller-authenticated external-deletion
evidence outcomes. Root snapshot reads reuse `CoordinatorStatusRequest::Registry`
under exact participating-Root authorization; the Root derives and validates
the manifest/version locally. Root-removal draining, removal publication and
readiness phases remain private and are reconciled from Root operation status.

Every request selector has maximum variant nesting depth one. Response selector
depth is at most two, solely for the role-local `Operation` status response and
its domain operation detail; no `Admin`, `Peer`, `Internal`, `Workflow`,
`Legacy` or former-family subtree is admitted. Exact old-method merges remain
visible in `method-register.tsv`; the largest are the 21 Root operation-status
queries merged into one local `Operation` selector and the flattened Fleet,
ICP-refill and pool command enums. Final generated reports must add exact
Candid service bytes, referenced type counts, protocol constants and
representative Wasm sizes for each profile.

The exact request/response contract is the normalized join of the register's
target and released Rust signature, the manifest's operation-owner table and
its correlation rule. This records the requested mapping without adding a
second 207-row authority that can drift.

The executable-caller column is also the binding-bootstrap inventory. Host and
CLI callers select the exact full binding by the protected artifact/release/
Directory protocol-profile digest before their first call. Static
inter-canister callers use only their generated request/response fragment after
the same protected metadata proves the exact target profile admits that
variant. Fixtures follow the same route. No executable caller is assigned to
trial decoding, a fallback binding or runtime schema negotiation.

Executable callers, synthesized DTOs, the exact capability-to-variant pruning
matrix, operation ownership and the 0.104 handoff are frozen. The four role
operation response enums contain only the accepted high-level detail variants;
six new detail views project existing domain state without creating a universal
operation store or exposing deleted phases.

This eight-file bundle was accepted as B1 evidence on 2026-08-17. The bounded
profile-bootstrap, cycle-capability, caller-cut and variant-accounting
clarifications are incorporated into that authority. B2/B3 implementation
evidence is separate in [`b3-profile-pruning.md`](b3-profile-pruning.md); it
does not mutate this released baseline or grant the old methods continuing
authority.

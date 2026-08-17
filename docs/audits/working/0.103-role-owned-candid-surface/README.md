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

- `fleet-subnet-root.did` and `managed-auth.did` are the exact generated fixture
  interfaces. The canonical Coordinator and Store DIDs remain checked in at
  their manifest-hashed source paths and released tag.
- `baseline-methods.tsv` contains every method, normalized signature, execution
  mode, endpoint source, compile condition, authorization/payload attribute,
  immediate delegate, replay policy, protocol constant and candidate
  in-repository references.
- `method-register.tsv` begins from that generated evidence and owns reviewed
  disposition decisions. Baseline recapture never overwrites it.
- `manifest.tsv` freezes the interface hashes and counts.
- `capture-baseline.sh` rebuilds and recaptures the evidence from a clean
  `v0.102.2` checkout.
- `capability-manifest.md` freezes the existing closed config derivation,
  invalid-combination boundary, compiled discovery owner and reserved names.

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

The six-way disposition pass is complete for all 188 Canic-owned appearances:

- 45 become role-command variants;
- 80 become role-status variants;
- two remain admitted Store byte lanes; and
- 61 become private/delete.

The query pass maps bounded observations to role status except the unreferenced
Root bootstrap debug query. Update review applies the high-level-intent rule:
allocation, provisioning, draining, subtree, registry synchronization, Store
removal and publication phase methods do not survive merely because they cross
an `await` or another canister. The Root template byte trio is private/delete
because large artifact bytes belong only to the admitted Store data plane.

Every retained row now has an exact role-specific target name. The one current
update-only management-canister observation becomes the atomic
`RootCommand::InspectCanister`; no ordinary query pretends it can make that
call. Remote composite phase observations collapse into the local
`RootStatusRequest::Operation` lane, so B1 needs no composite-status exception.
The Root's emitted managed-Canister binding query is also deleted: its DTO can
represent only Component or ComponentChild identity, never Root identity.

The status map is deliberately flat: distinct cycle, runtime, auth, directory
and Registry observations are variants, not second-level family enums. The 80
status appearances yield 23 Root, six Coordinator, 14 managed and eight Store
targets; Coordinator additionally requires the capability-discovery
`Overview` target that has no old-method row. The two existing nested Root
admin inputs are also flattened into their actual Fleet and pool intents.
Private phase rows have no shadow variant.

Executable callers, synthesized DTOs, the exact capability-to-variant pruning
matrix, operation ownership and the 0.104 handoff are frozen. The four role
operation response enums contain only the accepted high-level detail variants;
six new detail views project existing domain state without creating a universal
operation store or exposing deleted phases.

This eight-file bundle is B1 review-ready. It authorizes no runtime mutation;
B2 still requires explicit acceptance of the complete register and manifest.

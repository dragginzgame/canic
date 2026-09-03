# B1 Downstream Pressure Observation

## State

Read-only downstream inspection recovered the exact 2026-08-31
`project_instance` artifact behind the supplied B1 size row. The artifact and
Candid identities are bound, but the build lacks an immutable source commit,
clean-source assertion and complete build manifest. It remains non-binding
routing evidence rather than a reproducible B1 baseline or release gate.

The consumer's discard, export/reseed and cutover policy is outside Canic B1.
The Canic-owned consequence of this observation is the repository capability
fixture matrix covering authentication, payload adapters, blob economics,
lifecycle, timers, recovery and generic fanout.

No Toko file, artifact or repository state was modified during this inspection.

## Recovered Artifact

The existing files under Toko's
`.icp/local/canisters/project_instance/` all have a 2026-08-31 17:43 local
timestamp. Direct structure inspection reproduces both values supplied to B1:

| Measurement | Recovered value |
| --- | ---: |
| Wasm bytes | 11,118,208 |
| deterministic gzip bytes | 3,813,047 |
| code-section bytes | 10,275,629 |
| code-section headroom under 10 MiB | 210,131 |
| imported plus defined functions from `ic-wasm info` | 22,352 |
| defined functions | 22,312 |
| replica-limited function headroom under 50,000 | 27,688 |
| Wasm export-section entries | 274 |
| `ic-wasm` exported methods | 273 |
| Candid service methods | 268 |
| Candid bytes | 130,588 |

The distinction among export counts is intentional. B1's supplied value was
the 268-method public Candid service, not lifecycle/inspect exports or the raw
Wasm export-section count.

Retained local identities observed read-only:

| Artifact | SHA-256 |
| --- | --- |
| `project_instance.wasm` | `23367f5568683b46cc246836ec986a28d01429fe4015ea72858411f95b3a5a80` |
| `project_instance.wasm.gz` | `12c3d3bd02dd7d910e13f1f43c607549c9a83f97bf1f42a6f940ba22f71dfcf2` |
| `project_instance.did` | `387e594e371ea60a04c076be464d5c9290ac357662415f23dd26853774b31bfe` |

The gzip decodes byte-for-byte to the recorded Wasm hash. Embedded source-path
strings name `canic-core-0.109.32` and `icydb-core-0.250.0`; those versions match
the presently modified Toko manifest and lockfile. The `v0.109.32` Canic tag
declares Binaryen 132, but the artifact has no retained optimizer manifest or
build log proving which executable performed this particular transform.

## Source Identity Boundary

The present Toko checkout is not an immutable source anchor:

| Identity | Observation |
| --- | --- |
| base commit | `92fa602ea6ee3fcfa4f3732f9ac9e2a057cd9ac5` |
| base tree | `3c26f936097a9ed2ffd5b9943282c74fd63b5d4a` |
| exact tag at base | none |
| working-tree state | 23 modified tracked files; no clean-source assertion |
| current tracked-file content aggregate | `615c76664d221585ac71df39b01111d74fb719da1160682f244f1f1db640dddf` |
| current binary-diff hash | `01060c89957dd3cf859cc72b20e78be4dd6fb39828873763a05084c744140778` |
| current `Cargo.lock` hash | `749ddcd096daa9cb26c4e50dff37855d6035b1aa089644a02e5beef00b499685` |
| base `Cargo.lock` hash | `69e6ece3a7583c1cd417c19fde222c58f7de5ed915b6d864987fa5b85a7f6eed` |
| `apps/toko/canic.toml` hash | `145926d920a975917928a660d02e7e8040201e2292c19d78e1ddeb832773c274` |

The base commit pins Canic `0.109.27` and IcyDB `0.249.3`. The working tree
advances them to Canic `0.109.32` and IcyDB `0.250.0`. Every currently modified
file predates the recovered artifact by filesystem timestamp, and the embedded
dependency strings agree with the working lockfile. That is strong correlation,
not cryptographic source provenance. The artifact contains no source-tree or
dirty-patch digest tying it to the current 23-file snapshot.

The current project package does confirm the expected capability surface:

- Canic delegated-token verification;
- Canic Root canister-signature verification;
- blob storage and blob-storage billing;
- IcyDB generated schema/query/runtime machinery;
- project lifecycle, status, metrics and multiple application timers; and
- the `project_instance` role/capability bindings in `apps/toko/canic.toml`.

This combined surface motivated the Canic-owned fixture matrix. It is not a
binding canary and does not set a required application endpoint count.

## Missing Provenance Boundary

Promoting this row to reproducible consumer evidence would require:

1. one exact clean Toko commit containing the intended source and config;
2. clean predecessor and candidate lockfiles whose diff changes only the Canic
   package source/version/checksum graph required by the paired build;
3. the exact Rust/linker/finalizer tools, Binaryen executable identity and
   optimizer arguments;
4. two clean deterministic builds at one fixed execution path;
5. exact Candid, feature/capability and export parity;
6. the complete byte, replica-limited function, optimizer-defined cross-check,
   table/indirect and representative instruction vector; and
7. a direct reproduction of the recovered Wasm or an explicit superseding
   immutable canary row.

The recovered binary must not be relabelled as a `v0.110.5` build. It embeds
Canic `0.109.32`. B1 does not require the consumer to supply the missing items
and no further Toko/TokoMiner-specific work is planned.

## Discard And Reseed Authority

Toko's maintained local path is explicit and bounded:

- `make canisters-reinstall-local` delegates to `bin/dev_canic_local --fresh`;
- `--fresh` rejects outside the managed local development target;
- it stops the local replica and deletes the repository-local `.canic`, `.icp`
  and `.dfx` state; and
- it installs a new Fleet, recreates the local controller identity, restores
  pool capacity and provisions local platform/auth configuration.

That proves intentional local Fleet and application-data discard. It is not
staging/mainnet authority, and the launcher does not repopulate all Toko
business data. Separate developer seed helpers are test-data tools, not a
canonical application snapshot/reseed contract.

For staging/mainnet, `backend/scripts/app/upgrade_canisters_icp.sh` correctly
rejects direct upgrades and says to install a new Fleet and migrate/cut over.
`backend/README.md` repeats that policy. Neither surface defines:

- which application domains are discarded, exported or reconstructed;
- the source snapshot and schema that own each retained domain;
- how project, ledger, user/shard, registry, blob and external economic state
  are reconciled into the successor Fleet;
- the exact controller/routing/cycle-conservation checks before predecessor
  retirement; or
- the acceptance evidence and human authority for terminal residual discard.

Canic's pre-1.0 reinstall-only contract permits application data to be
discarded at a release boundary, but it cannot decide a consumer's product
policy. That policy is neither a Canic blocker nor a reason to add a
compatibility decoder or cross-release state adoption path.

## Decision

Retain the recovered artifact as exact, hash-bound, non-binding routing
evidence. Do not claim source reproducibility from file timestamps or the
moving dirty checkout. Canic qualification proceeds from its canonical roles
and repository-owned capability fixtures; consumer release policy remains
outside this line.

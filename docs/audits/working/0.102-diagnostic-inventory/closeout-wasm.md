# 0.102 Compact Diagnostic Closeout Wasm Evidence

Date: 2026-08-16

## Scope

This is the targeted role-scoped closeout measurement required by the 0.102
design. It is not a retained `CANIC-WASM-001/v3` run: it builds only release
artifacts for one representative Component plus Fleet Subnet Root, Fleet
Coordinator and Wasm Store. It does not build the nine-role release/debug
matrix and must not be cited as a replacement audit baseline.

The comparison baseline is the retained `CANIC-WASM-001/v3` run at immutable
tag `v0.101.53`, commit `23c0328f78b215580d734ef01b52b35fa3e38ade`.
The candidate was built from the working tree at
`b34d92ab115ecb7a9f884178ec10fae5bb563ace` plus the uncommitted 0.102
diagnostic cut. Because the working tree and the intervening release line also
contain changes outside the diagnostic cut, the raw and gzip deltas below are
release-line snapshots, not causal attribution to diagnostics.

## Method

Each role was built offline through the canonical host `build_artifact`
authority with the `release` profile, `apps/test/canic.toml`, Rust/Cargo
`1.97.1`, `ic-wasm 0.11.1` and `twiggy-opt 0.8.0`. No replica, deployment,
network access or direct Cargo Wasm build was used.

The builder-produced gzip for every role passed `gzip -t` and decompressed to
the exact SHA-256 of its paired raw Wasm. `ic-wasm info` parsed every artifact;
bounded `twiggy top` and retained-top measurements supplied structural
evidence.

## Results

| Role | Release Wasm | Baseline delta | Release gzip | Baseline delta | Functions | Data bytes | Baseline data delta | Exports |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `app` | 3,121,640 | +115,240 (+3.83%) | 1,027,748 | +46,863 (+4.78%) | 5,782 | 213,728 | -22,788 | 27 |
| `root` | 7,213,384 | -326,362 (-4.33%) | 2,326,922 | -103,705 (-4.27%) | 11,274 | 332,636 | -113,616 | 127 |
| `fleet_coordinator` | 3,516,812 | +77,009 (+2.24%) | 1,109,030 | +33,432 (+3.11%) | 5,487 | 194,828 | -48,112 | 29 |
| `wasm_store` | 2,695,059 | +97,808 (+3.77%) | 893,890 | +38,223 (+4.47%) | 5,337 | 190,956 | -25,268 | 32 |

Every artifact has three data sections. The largest shallow item remains
`data[0]`: 213,314 bytes for `app`, 331,614 for `root`, 194,402 for
`fleet_coordinator` and 190,442 for `wasm_store`. The largest retained item
remains `table[0]`: 1,394,984, 4,672,040, 1,615,433 and 1,226,759 bytes
respectively.

All four data sections are smaller than the retained baseline, consistent with
removing static diagnostic prose. Only Root is smaller in total raw and gzip
bytes across the full release-line comparison; the other three roles grew.
The non-isolated comparison cannot assign either result solely to the
diagnostic cut, so 0.102 makes no quantitative diagnostic-savings claim.

## Runtime-Absence Check

A bounded strings scan of all four release artifacts found no occurrence of:

- selected host-only summaries (`Access is unavailable.`, `Security conflicts
  with current state.`, `Configuration has reached its limit.`);
- selected symbolic catalogue labels (`ACCESS_UNAVAILABLE`,
  `CONFIGURATION_CAPACITY`);
- the `reasons.toml` repository path;
- the working diagnostic-audit path; or
- the generated cause-family register marker.

Together with the one-field Candid and generated-drift tests, this establishes
that the host catalogue and archived B1 register did not enter the measured
release Wasm. Source scans alone are not used as the proof.

## Decision

The compact public contract, typed raw/registered separation and host/runtime
ownership boundary remain justified independently of size. The role-scoped
evidence closes the 0.102 measurement requirement without authorizing another
low-level compaction programme. A future full `CANIC-WASM-001/v3` run remains
maintainer-owned push/release validation.


# BF4: reuse compiled declaration extraction — 2026-09-10

CANIC-087/139 requested reuse after backend edits. Every current Canic runtime
embeds the complete release identity, so allocating a successor changes every
runtime's inputs. Reusing a finalized predecessor runtime would violate that
contract. This correction reuses the independent declaration extraction stage;
it neither changes release identity nor claims cross-identity runtime reuse.

## Implementation

Both single configured-role and complete configured-role builds first run the
existing Cargo declaration command. Cargo remains responsible for dependency
freshness and feature resolution. The host then hashes the resulting Wasm and
looks up its normalized Candid under `.canic/build-reuse/declarations`.
The cache identity includes native extractor bytes, compiled extraction source,
Canic package version and environment. Hashing the small compiled implementation
avoids rereading the complete host executable solely for this stage.

Every hit verifies the record schema, extractor/implementation identity, exact
Wasm binding and returned Candid digest. Corrupt, oversized or missing records
use ordinary extraction. Failed extraction leaves no reusable record; changed
Wasm or extractor bytes during extraction refuse the result. Cache write
failure does not reject a successful extraction, and the cache-read size limit
is not a product Candid size limit. Unsupported scripted extractors keep the
existing uncached path.

Current role capabilities and protocol profiles are derived from the returned
bytes on every build, followed by ordinary runtime compilation and artifact
qualification. Complete-release cache decisions remain separate from new
per-role declaration hit/miss diagnostics. No new source fingerprint owner,
runtime protocol, compatibility path or compiler setting is introduced.

## Qualification

The focused build module passes 39 ordinary cases. Its one ignored real-tool
case is explicitly executed separately and passes. New cache regressions cover:

- A real two-package Cargo/Wasm build, one role's source edit and a shared
  `include_str!` edit. Cargo reports only the unaffected package fresh after
  the role edit; its compiled declaration hits, while the changed role misses.
  Both miss after the shared include changes. Extracted bytes match fresh work.
- Corrupt and misbound records, malformed JSON and oversized cache input.
- Exact extractor drift, including rejecting an old invocation and selecting
  a new cache identity for changed native tool bytes.
- Failed extraction followed by successful retry; no failed result is retained.
- Refused symlink inputs and cache redirection without modifying the target.

Ordinary tests compile a small native extractor fixture using Rust; they do
not add a candid-extractor installation to ordinary CI. The separately invoked
real-tool case uses the installed native extractor against six retained Canic
declaration Wasms: User Hub, User Shard, Scale Hub, Scale, Root and App. It
compares cached Candid with fresh extraction byte-for-byte.

| Extraction stage | Measured wall time |
| --- | ---: |
| Populate the empty cache | 2,234 ms |
| New cache session, six verified hits | 87 ms |
| Ordinary fresh extraction, same six Wasms | 1,922 ms |

Verified reuse saves 1,835 ms (95.5%) in this extraction-stage comparison.
Initial population adds 312 ms relative to the fresh extraction observation.
These are one local sequence with identical retained inputs, not an end-to-end
App build benchmark or a new qualification of the Wasms' runtime behaviour.
The [structured receipt](build-declaration-reuse.json) retains input/tool/source
hashes, Candid digests, commands and logs.

No PocketIC run is needed for this host-only extraction change: runtime source
and release binding are unchanged, and exact Candid is the downstream input.
Existing build regressions retain feature/profile and complete-cache checks.
No broad suite, version, Git publication, deployment or sibling mutation ran.
An initial targeted command briefly waited behind a concurrent Canic Clippy
invocation and was stopped; passing checks began after target ownership cleared.

BF4 completes this bounded declaration-reuse batch in the existing open .14
draft. CANIC-087/139 remain partial overall: every runtime still recompiles for
the newly embedded release identity. Further cross-identity runtime reuse
requires an explicit release-identity design, not broader cache admission.

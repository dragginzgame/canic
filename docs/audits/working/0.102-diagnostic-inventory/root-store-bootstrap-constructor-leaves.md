# Canic 0.102 Root Store Bootstrap Constructor Leaves

Date: 2026-08-15

## Status

This B1 evidence ledger classifies all 23 direct constructors in
`crates/canic-control-plane/src/workflow/bootstrap/root_store/mod.rs`. It
assigns no number and changes no runtime behavior. All 22 exact meanings now
name their producer functions/branches; the two remaining sites are transparent
typed topology adapters.

## Manifest Envelope And Protected Authority

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_RELEASE_SET_MANIFEST_SIZE_INVALID` | 1 | `load_and_validate_manifest`; payload size is zero or above `ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES` | self | Supply exact bytes within the maintained bound | public |
| `ROOT_STORE_RELEASE_SET_JSON_INVALID` | 1 | `load_and_validate_manifest`; `serde_json::from_slice` fails | self | Restage a valid host-produced manifest | public; nested parser cause masked |
| `ROOT_STORE_RELEASE_SET_CANONICALIZATION_FAILED` | 1 | `load_and_validate_manifest`; `serde_json::to_vec` fails | `COMPONENT_REGISTRY_STATE_INVALID` | Stop bootstrap and inspect the serializer/record | structured log |
| `ROOT_STORE_RELEASE_SET_BYTES_NONCANONICAL` | 1 | `load_and_validate_manifest`; re-encoded bytes differ from staged bytes | self | Restage canonical host-produced bytes | public |
| `ROOT_STORE_RELEASE_SET_DIGEST_MISMATCH` | 1 | `load_and_validate_manifest`; `wasm_hash` differs from protected manifest digest | self | Restage the exact protected manifest | public |
| `ROOT_STORE_RELEASE_SET_BUILD_MISMATCH` | 1 | `validate_manifest_projection`; release build differs from protected authority | self | Use the exact admitted release build | public |
| `ROOT_STORE_RELEASE_SET_TOPOLOGY_MISMATCH` | 1 | `validate_manifest_projection`; manifest topology digest differs from protected authority | self | Use the exact protected topology projection | public |
| transparent: typed Component-topology admission projection cause | 1 | Bootstrap currently stringifies an exact topology projection error | preserve the nested registered projection | Remove the text adapter and propagate the typed cause | public or structured owner of nested cause |
| transparent: typed Component-topology digest cause | 1 | Bootstrap currently stringifies an exact topology digest error | preserve the nested registered projection | Remove the text adapter and propagate the typed cause | public or structured owner of nested cause |
| `ROOT_STORE_ADMISSIONS_TOPOLOGY_DIGEST_MISMATCH` | 1 | `validate_manifest_projection`; projected digest differs from protected topology digest | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve authority and fail closed | recent failure |
| `ROOT_STORE_RELEASE_SET_ENTRY_COUNT_MISMATCH` | 1 | `validate_manifest_projection`; manifest and admitted closure lengths differ | self | Rebuild the root projection from protected admissions | public |
| `ROOT_STORE_RELEASE_SET_ENTRY_AUTHORITY_MISMATCH` | 1 | `validate_manifest_projection`; `RootReleaseSetEntryAuthority` differs at a canonical position | self | Restage the exact admitted entry authority | public |

The 12 sites add ten exact meanings; two are transparent typed-cause adapters.

## Artifact Shape And Capacity

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_ROLE_ARTIFACT_CONFLICT` | 1 | `validate_manifest_projection`; repeated role has a different compressed payload tuple | self | Rebuild a single qualified artifact per role | public |
| `ROOT_STORE_RELEASE_SET_BYTES_OVERFLOW` | 1 | `validate_manifest_projection`; checked deduplicated-byte addition fails | self | Reject the malformed/oversized manifest | public |
| `ROOT_STORE_BYTE_CAPACITY_EXCEEDED` | 1 | `validate_manifest_projection`; total exceeds protected `maximum_wasm_store_bytes` | self | Reduce admitted artifacts or raise protected root capacity | public |
| `ROOT_STORE_ARTIFACT_PATH_MISSING` / `ROOT_STORE_ARTIFACT_SIZE_ZERO` | 1 | `validate_artifact_shape`; a package/path is empty or raw/compressed size is zero | self for both leaves | Rebuild complete qualified artifact metadata | public |
| `ROOT_STORE_STAGED_ROLE_ARTIFACT_CONFLICT` | 1 | `exact_staged_manifest_metadata`; repeated role has a different compressed payload tuple | self | Clear/restage the conflicting role artifact | public |
| `ROOT_STORE_STAGED_ARTIFACT_AUTHORITY_MISMATCH` | 1 | `exact_staged_manifest_metadata`; observed authority or `is_exact_bootstrap_source` differs | self | Restage the exact protected artifact through the bootstrap binding | public |
| `ROOT_STORE_RAW_MODULE_HASH_CONFLICT` | 1 | `artifact_module_hashes`; repeated role has a different decoded raw hash | self | Rebuild a single qualified raw module per role | public |

The seven sites produce eight new exact meanings.

## Live Store Catalog And Digest Format

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_LIVE_CATALOG_MISMATCH` | 1 | `verify_live_catalog`; observed and expected canonical catalog tuples differ | self | Reconcile publication/status against the exact release set | public |
| `ROOT_STORE_CATALOG_RAW_MODULE_HASH_MISSING` | 1 | `verify_live_catalog`; observed role has no protected raw hash | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve catalog/manifest and fail closed | recent failure |
| `ROOT_STORE_CATALOG_PAYLOAD_HASH_INVALID` | 1 | `verify_live_catalog`; payload hash cannot convert to `[u8; 32]` | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve catalog and fail closed | recent failure |
| `ROOT_STORE_ARTIFACT_SHA256_FORMAT_INVALID` | 1 | `decode_sha256`; length or lowercase-hex validation fails | self | Supply canonical lowercase SHA-256 text | public |

All four sites add four exact meanings.

## Dynamic Public Context

The direct formatting sites contain six values:

- the manifest byte maximum, deduplicated required bytes, protected Store byte
  limit and staged artifact role are caller-derivable from the maintained
  contract plus exact host manifest/root plan and can be discarded;
- JSON decode and canonical-encoding dependency causes are sensitive
  implementation detail and may appear only in structured operator logs; and
- the two transparent topology errors introduce no bootstrap-owned context.
  Their typed variants and fields remain owned by the transitive topology/config
  inventories and must propagate without stringification.

## Reconciliation

The three tables sum to all 23 references. Two sites are transparent adapters;
the remaining 21 sites produce 22 new exact meanings and no new safe
projection.

## Required Tests

- reject zero, oversized, malformed and noncanonical manifest bytes
  independently;
- vary protected build, topology, entry count and every entry authority field
  independently;
- preserve every exact typed topology projection/digest cause through the
  bootstrap wrapper;
- reject conflicting compressed/raw role identities, overflow and protected
  Store capacity independently;
- vary every staged-artifact authority field independently; and
- reject catalog order/identity mismatch, missing raw hash, wrong payload-hash
  length and noncanonical hash text independently.

## Next Slice

Classify the root bootstrap owner and root Store stable-state facade, then
remaining Wasm Store lifecycle and Fleet Mirror synchronization owners.

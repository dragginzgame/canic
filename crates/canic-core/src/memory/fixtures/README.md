# Memory test images

The `.inc` files are sparse byte images produced with published
`ic-stable-structures 0.7.2` in an isolated generator. They are test data for
unmanaged-custody and diagnostic cases, not an allocation implementation.
`ic_memory::testing::memory_from_sparse_hex` expands them with a page ceiling.

The generator initialized `MemoryManager::init_with_bucket_size` over empty
`VectorMemory`, grew each selected slot by one virtual page, and wrote `custody`
at offset zero of slot 200. It then recorded each contiguous nonzero byte run;
long repeated bytes use Rust string repetition without changing the image.

| Image | Bucket pages | Populated slots | Physical pages |
| --- | --- | --- | --- |
| `unmanaged-one.inc` | 1 | 200 | 2 |
| `unmanaged-sixteen.inc` | 16 | 200 | 17 |
| `all-slots.inc` | 1 | 1 through 254 | 255 |

Collection tests instead bootstrap explicit keys through Memory's published
fixture runtime and transfer exclusive leases to Storage adapters. No deployed
code depends on the raw collection or memory-manager APIs.

# Binaryen runtime-library authority

[#464](https://github.com/dragginzgame/canic/issues/464) owns the macOS Rust
installer correction. The existing Binaryen 132 archive identities come from
the unchanged reviewed Shared Tooling matrix at
`a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`. Canic retains executable and runtime
library admission at its own authority boundary.

Both official release archives were downloaded to
`target/review-validation/binaryen-132-libraries/`. The repository checksum
verifier accepted each archive before tar read the named member
`binaryen-version_132/lib/libbinaryen.dylib`. These library identities are derived
from authenticated archive bytes, rather than from an arbitrary installed file.

| Host | Verified archive SHA-256 | Runtime-library SHA-256 |
| --- | --- | --- |
| Apple Silicon macOS | `98aad827847af7ef990ed7098d885725c8e5b5aae75073403635617ae4e259aa` | `6627f4f3f3655bfc14b3cd4816b0e7b0cb62ce6a530bab00cdd26855d5f6359b` |
| Intel macOS | `40c3de90bb3766bd0282a895e139a6f50253dba49b4f5bb89e66faca162d832e` | `f6d540a50c12af1769c10e30775625f70b65327ff8ee438c7059bc076147000a` |

The owning projection test binds those digests to Canic's pin record. Synthetic
native fixtures prove relative-library execution before/after atomic selection,
failed-candidate preservation and modified/missing-library refusal before version
execution. macOS binaries were not executed on Linux. Both native macOS CI cells
now run the owning tests, actual Rust CLI installation and representative canister
build; their execution evidence remains outstanding.

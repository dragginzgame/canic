# Candid extractor setup

Canic owns its exact `CANIC_CANDID_EXTRACTOR_VERSION` in
[`tool-versions.env`](../tool-versions.env). Shared Tooling owns Cargo installation,
receipt admission and retained builds; IC Build owns extraction orchestration.

After preparing the repository's compiler and host tools, run:

```bash
make install-candid-extractor
extractor="$(make --no-print-directory candid-extractor-check)"
export PATH="${extractor%/*}:$PATH"
```

`install-tools` and `tools-check` include this selection through Canic's local
extensions. The check is offline and does not install or repair anything. Source
this PATH selection in the shell that runs Canic builds; a setup subprocess
cannot change its parent's environment. CI publishes the admitted directory
before executing native build/fixture callers.

This preparation works with the currently pinned Shared installer and is required
before adopting a snapshot whose common Rust set omits the extractor. No snapshot
payload is patched, and older common selections or retained builds are not removed.

# Auth qualification inputs

These upstream dependency archives are retained in Canic so qualification does not
depend on a producer's ignored build directory. Generated test output belongs under
`target/`.

## Current backend driver

The [0.33.1 backend archive](backend/ic-auth-backend-conformance-0.33.1.crate) and
[supplier receipt](backend/delivery-0331.json) come from clean release source
`f9105b938aa76de80f1e83c635e1ab445133911c`. Its checksum is
`9a71171b494337da1c1254eb61df224f014d25a63dfeccb4989f461e417046ed`.
The exact extracted [supplier package](../../../../../integrations/auth-backend-conformance/Cargo.toml) is retained as an
excluded development dependency, with its normalized manifest and license intact.
It is not a Canic workspace member or production dependency. Cargo can resolve the
checkout without a preliminary extraction step or a sibling source override.
The extracted package lives outside every Canic member directory so Cargo honors
`workspace.exclude`. IC Auth owns it and its delivered version; it belongs to no
Canic release group. Its files remain byte-for-byte identical to the archive.

Canic Core's `delivered_backend_contract_uses_canic_stable_storage` test implements
its `Backend` driver using the actual session adapter. Reopen writes through the
stable import owner and reconstructs indexes from stored records. Admission retry,
logout, generation, bounded refusal, authorization and maximum-shape scenarios
pass. Failure injection refuses at the test transaction boundary; this does not
qualify IC trap atomicity, heap snapshot safety or production performance.

## Retained earlier paired delivery

The [0.33.0 backend archive](backend/ic-auth-backend-conformance-0.33.0.crate),
[intake receipt](backend/delivery.json), [browser archive](browser/dragginzgame-ic-auth-client-0.0.0.tgz),
[browser receipt](browser/delivery.json) and [input hashes](browser/inputs.tsv0)
retain their clean source `fa6823a25cedb04f55f91301efcf261ab4109b1f`.
The browser's private version remains 0.0.0; source and npm integrity identify it.
It has not been replaced or requalified by the backend-only 0.33.1 intake.

Archives contain upstream license and installation guidance. They do not install
production browser transports. The existing frontend consumer now selects this
browser archive by npm integrity and qualifies its real signed admission/logout
HTTP transports against two installed receivers and two devices. Run
`bash scripts/dev/qualify-auth-browser.sh` after preparing that consumer's locked
npm dependencies. Issuance remains fixture-driven; Node transport acceptance is
not browser IndexedDB or durable client recovery qualification.
The separate `durable` mode now qualifies actual Chromium IndexedDB and upstream
logout cleanup across page reopen and lost receiver responses. It requires an
explicit prepared browser executable. The `issuer` mode additionally qualifies
account-signed issuer HTTP, persisted intent, page reopen and exact reconciliation
before upstream admission on both receivers. Production custody and activation
remain outside these controlled-local proofs.
Canic still supplies account-authenticated
issuance/reconciliation and receiver admission/logout wiring, then proves signed
browser calls, lost replies, reopen recovery and durable logout. Remaining work is
tracked in the [authentication guide](../../../../docs/features/authentication/README.md#progress).

# Independent frontend consumer

This small consumer qualifies Canic's frontend manifest using the public ICP
JavaScript SDK. Its package lock pins core 5.4.0, auth 8.0.3 and TypeScript
6.0.3 for declaration checks. These are explicit
qualification versions; they are not a claim about the latest SDK or Toko's
complete dependency graph.

```sh
npm ci --ignore-scripts --no-audit --no-fund
node qualify.mjs verify /path/to/bundle <independent-manifest-sha256>
```

Use `verifyHandoff` from `handoff.mjs` with bounded fetched bytes, an independently
retained digest and a loader for relative artifact paths. Then statically import
the selected generated `.did.mjs` factory and call `createCanicActor` with the
verified handoff, exact selected canister ID and your authenticated SDK identity.
The caller owns HTTP fetch limits, timeouts and frontend build/deployment.

`createCanicActor` accepts only a verified, frozen handoff. It binds the agent to
the selected API origin and exact root key, with automatic root-key fetching
disabled. The native Node fixture supplies WebCrypto for Node 18; browsers use
their built-in WebCrypto. Verification and actor construction do not expose
operator credentials or create a privileged actor.

The application can use `AuthClient` from `@icp-sdk/auth/client`, passing the
manifest's provider origin as `identityProvider` and its derivation origin as
`derivationOrigin` during login. Pass the resulting `getIdentity()` to actor
construction. Login, renewal, presentation and certified static-asset delivery
belong to the application.

For the disposable PocketIC acceptance case, `qualify.mjs identity` creates a
private signing key in the test scratch directory. Its Principal is admitted
before installation. `call` asserts the application sees that exact caller;
`denied` asserts that a separately generated, unadmitted caller receives a typed
error. Keys are never included in the handoff and the runner removes its scratch.
This proves generated bindings, network trust and authenticated ingress. It does
not emulate the Internet Identity user interface.

See the [frontend guide](../../../../docs/features/operations/frontend-handoff.md)
for input fields, origins, artifact budgets and native asset-cycle checks.

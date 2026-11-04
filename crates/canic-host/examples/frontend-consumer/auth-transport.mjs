import './check-boundary.mjs';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import { Principal } from '@tokostudios/ic-platform/principal';
import { credentialFromBytes, httpAdmissionTransport, httpLogoutTransport } from '@dragginzgame/ic-auth-client';

Object.defineProperty(globalThis, 'crypto', { value: webcrypto });
const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
const input = JSON.parse(Buffer.concat(chunks).toString('utf8'));
const trust = { host: input.host, rootKey: Uint8Array.from(input.rootKey) };
let dropped = 0;
const lostReply = input.action.endsWith('-lost');
if (lostReply) {
  trust.fetch = async (request, options) => {
    const response = await fetch(request, options);
    const url = new URL(request instanceof Request ? request.url : request.toString());
    if (url.pathname.endsWith('/call')) {
      assert.ok(response.ok, 'gateway accepted the signed ingress');
      await response.arrayBuffer();
      dropped++;
      throw new TypeError('test transport lost the accepted ingress response');
    }
    return response;
  };
}
async function complete(attempts) {
  if (lostReply) {
    // Failure remains an unknown effect; the Rust caller observes actual receiver state.
    const results = await Promise.allSettled(attempts);
    assert.ok(results.every(result => result.status === 'rejected'));
    assert.equal(dropped, attempts.length, 'exactly one accepted dispatch per attempt');
  } else {
    await Promise.all(attempts);
  }
}
const signal = AbortSignal.timeout(30_000);
const targets = input.targets.map(target => Principal.fromText(target));
// Deterministic keys belong only to the disposable PocketIC acceptance fleet.
const identities = input.devices.map(device => Ed25519KeyIdentity.generate(new Uint8Array(32).fill(device)));
assert.deepEqual(identities.map(identity => identity.getPrincipal().toText()), input.callers);
if (input.action === 'admit' || input.action === 'admit-lost') {
  const admit = httpAdmissionTransport({ ...trust, methodName: 'canic_establish_portable_session', scopes: input.scopes });
  await complete(identities.flatMap((identity, index) => {
    const credential = credentialFromBytes(Uint8Array.from(input.credentials[index]), 131_072);
    return targets.map(target => admit(target, identity, credential, signal));
  }));
} else if (input.action === 'logout' || input.action === 'logout-lost') {
  const transport = httpLogoutTransport({ ...trust, methodName: 'canic_logout_portable_session', dispose: async () => {} });
  await complete(targets.map(async target => {
    const result = await transport.revoke(target, identities[0], signal);
    assert.equal(result.tag, 'Revoked');
  }));
} else {
  throw new Error('Expected admit or logout');
}
process.stdout.write(`${input.action}: ${lostReply ? 'accepted replies lost; outcome remains unknown to client' : 'authenticated upstream HTTP transport succeeded'}\n`);

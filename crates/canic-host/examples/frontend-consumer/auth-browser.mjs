// Browser-only acceptance harness. Auth owns all journal and retry transitions.
import { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import { Principal } from '@tokostudios/ic-platform/principal';
import { IndexedDbLogoutStore, PortableLogoutClient, ClientError, credentialFromBytes,
  httpAdmissionTransport, httpLogoutTransport } from '@dragginzgame/ic-auth-client';

let input, stores, clients, identities;
const lost = new Set();
const clock = { id: 'controlled-local-wall-clock', nowNs: () => BigInt(Date.now()) * 1_000_000n,
  wait: ms => new Promise(resolve => setTimeout(resolve, ms)) };
const generation = index => `device-${input.devices[index]}`;
function require(value, message) { if (!value) throw new Error(message); }

export async function open(config, create = false, admit = true) {
  input = config;
  identities = input.devices.map(device => Ed25519KeyIdentity.generate(new Uint8Array(32).fill(device)));
  stores = await Promise.all(input.devices.map(device => IndexedDbLogoutStore.open(indexedDB, `canic-auth-${device}`)));
  clients = stores.map(store => new PortableLogoutClient(store, clock, httpLogoutTransport({
    host: location.origin, rootKey: Uint8Array.from(input.rootKey), methodName: 'canic_logout_portable_session',
    dispose: async name => { localStorage.setItem(`disposed-${name}`, 'true'); },
    fetch: async (request, options) => {
      const response = await fetch(request, options);
      const path = new URL(request instanceof Request ? request.url : request.toString()).pathname;
      if (path.endsWith('/call') && !lost.has(path)) {
        require(response.ok, 'signed logout accepted');
        await response.arrayBuffer(); lost.add(path);
        throw new TypeError('test loses first accepted logout response');
      }
      return response;
    },
  }), () => 0));
  if (create) {
    await Promise.all(clients.map(async (client, index) => {
      await client.start(generation(index), identities[index], Principal.fromText(input.issuer));
      for (const target of admit ? input.targets : []) {
        await client.destination(generation(index), Principal.fromText(target), BigInt(input.until[index]));
      }
    }));
    if (!admit) return;
    const transport = httpAdmissionTransport({ host: location.origin, rootKey: Uint8Array.from(input.rootKey),
      methodName: 'canic_establish_portable_session', scopes: input.scopes });
    await Promise.all(clients.flatMap((client, index) => input.targets.map(target => transport(
      Principal.fromText(target), client.guardedIdentity(generation(index), identities[index]),
      credentialFromBytes(Uint8Array.from(input.credentials[index]), 131_072), AbortSignal.timeout(5_000)))));
  }
}
export function admissionContext() { return { clients, identities }; }
export async function begin() {
  await clients[0].begin(generation(0), clock.nowNs() + 60_000_000_000n);
  return state();
}
export async function state() {
  const record = await clients[0].status(generation(0));
  return { ...record, deadlineNs: record.deadlineNs.toString() };
}
export async function signingRefused() {
  try {
    await clients[0].guardedIdentity(generation(0), identities[0]).transformRequest({
      body: { request_type: 'call', canister_id: Principal.fromText(input.targets[0]) },
    });
  } catch (error) {
    require(error instanceof ClientError && error.code === 'stale_generation', 'typed stale signing refusal');
    return true;
  }
  throw new Error('cleanup login signed a new request');
}
export async function cleanup() {
  const record = await clients[0].cleanup(generation(0), identities[0]);
  require(record.phase === 'finished', 'cleanup finished');
  require(record.targets.every(target => target.confirmedUntilNs !== undefined && target.attempts === 2), 'lost replies retried and confirmed');
  require(lost.size === input.targets.length, 'lost one accepted reply per receiver');
  require(localStorage.getItem(`disposed-${generation(0)}`) === 'true', 'disposal callback ran');
  require((await clients[1].status(generation(1))).phase === 'active', 'other device remains active');
  return state();
}

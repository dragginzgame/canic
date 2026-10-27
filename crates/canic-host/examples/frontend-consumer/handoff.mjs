import { HttpAgent, IC_ROOT_KEY, createAgentOptions, queryRaw } from '@tokostudios/ic-platform/agent';
import { Principal } from '@tokostudios/ic-platform/principal';

const encoder = new TextEncoder();
const verified = new WeakMap();

function freeze(value) {
  if (value !== null && typeof value === 'object') {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]));
  }
  return value;
}

export function fromHex(hex) {
  if (typeof hex !== 'string' || !/^(?:[0-9a-f]{2})+$/.test(hex)) throw new Error('Invalid hexadecimal bytes');
  return Uint8Array.from(hex.match(/../g), pair => Number.parseInt(pair, 16));
}

export async function digest(bytes) {
  const hash = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  return Array.from(hash, byte => byte.toString(16).padStart(2, '0')).join('');
}

async function networkIdentity(key) {
  const domain = encoder.encode('canic:canonical-network-id\0');
  const trust = fromHex(await digest(key));
  const input = new Uint8Array(domain.length + trust.length);
  input.set(domain);
  input.set(trust, domain.length);
  return digest(input);
}

/** Verify a public manifest and every referenced artifact against an independently retained digest. */
export async function verifyHandoff(manifestBytes, expectedDigest, readFile) {
  if (manifestBytes.byteLength > 4 * 1024 * 1024) throw new Error('Manifest exceeds the host file budget');
  const manifest = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(manifestBytes));
  if (manifest.schema_version !== 1 || manifest.manifest_sha256 !== expectedDigest) throw new Error('Unexpected handoff identity');
  const hash = await digest(encoder.encode(JSON.stringify(canonical({ ...manifest, manifest_sha256: '' }))));
  if (hash !== expectedDigest) throw new Error('Manifest integrity mismatch');
  const rootKey = fromHex(manifest.local_root_key_der_hex ?? IC_ROOT_KEY);
  if (await networkIdentity(rootKey) !== manifest.canonical_network_id) throw new Error('Network trust mismatch');
  if (manifest.roles.length === 0 || manifest.roles.length > 128) throw new Error('Invalid role selection');
  const files = [manifest.alternative_origins, ...manifest.roles.flatMap(role => [role.wire_schema, role.javascript, role.typescript, ...role.modules])];
  const names = new Set();
  let total = 0;
  for (const file of files) {
    if (typeof file.path !== 'string' || file.path.split('/').some(part => !part || part === '.' || part === '..') || /[\\?#%:]/.test(file.path)) throw new Error('Unsafe artifact path');
    if (names.has(file.path)) throw new Error('Repeated artifact path');
    names.add(file.path);
    const bytes = await readFile(file.path);
    total += bytes.byteLength;
    if (bytes.byteLength > 4 * 1024 * 1024 || total > 32 * 1024 * 1024) throw new Error('Bundle exceeds the host byte budget');
    if (bytes.byteLength !== file.bytes || await digest(bytes) !== file.sha256) throw new Error('Artifact integrity mismatch');
  }
  const handoff = Object.freeze({ manifest: freeze(manifest) });
  verified.set(handoff, rootKey);
  return handoff;
}

/** Bind raw authenticated transport to verified release codecs and explicit trust. */
export async function createCanicClient(handoff, canisterId, bindings, identity) {
  if (!verified.has(handoff)) throw new Error('Verify the handoff before constructing a client');
  if (!handoff.manifest.roles.some(role => role.canister_id === canisterId)) throw new Error('Canister is outside this handoff');
  const target = Principal.fromText(canisterId);
  const agent = await HttpAgent.create(createAgentOptions({ host: new URL(handoff.manifest.api_origin), identity,
    rootKey: Uint8Array.from(verified.get(handoff)),
    retryTimes: 3, verifyQuerySignatures: true, shouldSyncTime: false }));
  return Object.freeze({
    async call(method, request = null) {
      const codec = bindings[method];
      if (!codec || codec.methodName !== method) throw new Error('Method is outside these bindings');
      const arg = codec.encodeRequest(request);
      let reply;
      if (codec.mode === 'query') {
        reply = await queryRaw(agent, target, { methodName: method, arg });
      } else if (codec.mode === 'update') {
        reply = (await agent.update(target, { methodName: method, arg, effectiveCanisterId: target })).reply;
      } else {
        throw new Error('Unsupported Wire transport mode');
      }
      return codec.decodeResponse(new Uint8Array(reply));
    },
  });
}

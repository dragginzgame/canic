import './check-boundary.mjs';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve, join, dirname } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import ts from 'typescript';
import { Principal } from '@tokostudios/ic-platform/principal';
import { createCanicClient, verifyHandoff } from './handoff.mjs';

Object.defineProperty(globalThis, 'crypto', { value: webcrypto });

async function checkDeclarations(bundle, manifest) {
  const consumer = dirname(fileURLToPath(import.meta.url));
  const paths = manifest.roles.flatMap(role => [role.typescript, ...role.modules])
    .filter(file => file.path.endsWith('.d.ts')).map(file => join(bundle, file.path));
  const options = { strict: true, noEmit: true, skipLibCheck: false, types: [], typeRoots: [],
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022,
    moduleResolution: ts.ModuleResolutionKind.Bundler };
  const program = ts.createProgram(paths, options);
  const errors = ts.getPreEmitDiagnostics(program);
  assert.equal(errors.length, 0, ts.formatDiagnostics(errors, {
    getCurrentDirectory: () => consumer, getCanonicalFileName: file => file, getNewLine: () => '\n',
  }));
}

const [action, directory, expectedDigest, canisterId, method] = process.argv.slice(2);
if (action === 'identity') {
  const identity = Ed25519KeyIdentity.generate();
  await writeFile(directory, JSON.stringify(identity.toJSON()), { mode: 0o600, flag: 'wx' });
  process.stdout.write(`${identity.getPrincipal().toText()}\n`);
} else if (action === 'verify') {
  const bundle = resolve(directory);
  const manifestBytes = await readFile(join(bundle, 'canic-frontend.json'));
  const handoff = await verifyHandoff(manifestBytes, expectedDigest, path => readFile(join(bundle, path)));
  assert.equal(handoff.manifest.manifest_sha256, expectedDigest);
  await checkDeclarations(bundle, handoff.manifest);
  process.stdout.write('Verified frontend handoff and generated artifacts\n');
} else if (action === 'call' || action === 'denied') {
  const bundle = resolve(directory);
  const manifestBytes = await readFile(join(bundle, 'canic-frontend.json'));
  const handoff = await verifyHandoff(manifestBytes, expectedDigest, path => readFile(join(bundle, path)));
  if (action === 'call') await checkDeclarations(bundle, handoff.manifest);
  const selected = handoff.manifest.roles.find(role => role.canister_id === canisterId);
  assert.ok(selected, 'exact selected role');
  const bindings = await import(pathToFileURL(join(bundle, selected.javascript.path)));
  const identity = Ed25519KeyIdentity.fromJSON(await readFile(process.env.CANIC_FRONTEND_IDENTITY, 'utf8'));
  const client = await createCanicClient(handoff, canisterId, bindings, identity);
  const response = await client.call(method);
  if (action === 'denied') {
    assert.ok(Object.hasOwn(response, 'err'), 'unadmitted identity is denied');
    process.stdout.write('Unadmitted application caller denied\n');
  } else {
    assert.ok(Object.hasOwn(response, 'ok'), 'authenticated application call returns typed success');
    assert.equal(Principal.fromUint8Array(response.ok).toText(), identity.getPrincipal().toText(), 'application observes the browser-owned caller');
    process.stdout.write(`Authenticated application call succeeded for ${identity.getPrincipal().toText()}\n`);
  }
} else {
  throw new Error('Select identity, verify or call');
}

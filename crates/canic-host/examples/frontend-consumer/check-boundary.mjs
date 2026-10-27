import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';

const root = new URL('./', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('package.json', root), 'utf8'));
assert.equal(typeof manifest.dependencies['@tokostudios/ic-platform'], 'string');
const directSdk = /(?:@icp-sdk\/core|@dfinity\/)/;
for (const section of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) {
  for (const [name, selection] of Object.entries(manifest[section] ?? {})) {
    assert.ok(!directSdk.test(name) && !directSdk.test(selection), `Direct SDK dependency: ${name}`);
  }
}
for (const file of await readdir(root)) {
  if (!/\.(?:mjs|js|ts)$/.test(file) || file === 'check-boundary.mjs') continue;
  const source = await readFile(new URL(file, root), 'utf8');
  assert.ok(!directSdk.test(source), `Direct SDK import in ${file}`);
}

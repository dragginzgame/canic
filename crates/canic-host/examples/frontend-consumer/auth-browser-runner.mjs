import './check-boundary.mjs';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { build } from 'esbuild';

const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
const input = JSON.parse(Buffer.concat(chunks).toString('utf8'));
const backend = new URL(input.host);
assert.ok(['localhost', '127.0.0.1', '[::1]'].includes(backend.hostname));
assert.ok(process.env.CANIC_AUTH_BROWSER_EXECUTABLE, 'select a prepared Chromium executable');
const bundle = await build({ entryPoints: [fileURLToPath(new URL(input.mode === 'issuer' ? 'auth-admission.mjs' : 'auth-browser.mjs', import.meta.url))],
  bundle: true, platform: 'browser', format: 'iife', globalName: 'authFixture', write: false,
  plugins: [{ name: 'rust-owned-wire', setup(build) {
    build.onResolve({ filter: /^canic-(issuer-codecs|wire-runtime)$/ }, args => ({ path: args.path, namespace: 'rust-wire' }));
    build.onLoad({ filter: /.*/, namespace: 'rust-wire' }, args => ({ contents: args.path === 'canic-issuer-codecs' ? input.codecs : input.runtime, loader: 'ts' }));
  } }] });
const server = createServer(async (request, response) => {
  try {
    const path = new URL(request.url, 'http://localhost').pathname;
    if (path === '/' && request.method === 'GET') { response.setHeader('content-type', 'text/html'); response.end('<script src="/bundle.js"></script>'); return; }
    if (path === '/bundle.js' && request.method === 'GET') { response.setHeader('content-type', 'text/javascript'); response.end(bundle.outputFiles[0].contents); return; }
    const match = /^\/api\/v[23]\/canister\/([^/]+)\/(call|read_state)$/.exec(path);
    if (request.method !== 'POST' || !match || !input.targets.includes(match[1])) { response.writeHead(404); response.end(); return; }
    const body = []; let length = 0;
    for await (const chunk of request) { length += chunk.length; assert.ok(length <= 1_048_576); body.push(chunk); }
    const reply = await fetch(new URL(path, backend), { method: 'POST', body: Buffer.concat(body),
      headers: { 'content-type': 'application/cbor' }, signal: AbortSignal.timeout(20_000) });
    response.writeHead(reply.status, { 'content-type': 'application/cbor' });
    response.end(Buffer.from(await reply.arrayBuffer()));
  } catch { response.writeHead(502); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
  browser = await chromium.launch({ executablePath: process.env.CANIC_AUTH_BROWSER_EXECUTABLE, headless: true });
  const context = await browser.newContext();
  const origin = `http://127.0.0.1:${server.address().port}`;
  const open = async create => {
    const page = await context.newPage();
    await page.goto(origin);
    await page.waitForFunction(() => !!globalThis.authFixture);
    await page.evaluate(({ input, create }) => authFixture.open(input, create), { input, create });
    return page;
  };
  let page = await open(true);
  if (input.mode === 'issuer') {
    await page.evaluate(() => authFixture.loseIssuance());
    await page.close();
    page = await open(false);
    await page.evaluate(() => authFixture.establish());
    await page.evaluate(() => authFixture.refuseOtherAccounts());
    console.log('Authenticated recovery isolation: browser, unrelated and anonymous callers cannot claim the account credential; original recovery still succeeds');
    console.log('Account-signed issuer HTTP: page reopen reconciles original credentials once per device and admits both receivers without reissuing');
  }
  const committed = await page.evaluate(() => authFixture.begin());
  assert.equal(committed.phase, 'cleanup');
  assert.equal(await page.evaluate(() => authFixture.signingRefused()), true);
  await page.close();
  page = await open(false);
  const reopened = await page.evaluate(() => authFixture.state());
  assert.equal(reopened.phase, 'cleanup');
  assert.equal(reopened.deadlineNs, committed.deadlineNs);
  assert.equal(await page.evaluate(() => authFixture.signingRefused()), true);
  const finished = await page.evaluate(() => authFixture.cleanup());
  assert.equal(finished.deadlineNs, committed.deadlineNs);
  await page.close();
  page = await open(false);
  assert.equal((await page.evaluate(() => authFixture.state())).phase, 'finished');
  assert.equal(await page.evaluate(() => authFixture.signingRefused()), true);
  console.log(`Chromium ${browser.version()}: IndexedDB logout survives page reopen, retries lost replies, confirms both receivers and keeps the other device active`);
} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}

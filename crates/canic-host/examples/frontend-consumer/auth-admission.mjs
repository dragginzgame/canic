// Canic consumer fixture: Auth owns issuance claiming, admission and durable logout.
import { HttpAgent } from '@tokostudios/ic-platform/agent';
import { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import { Principal } from '@tokostudios/ic-platform/principal';
import { PortableAdmissionClient, ClientError, httpAdmissionTransport } from '@dragginzgame/ic-auth-client';
import { test_issue_portable_credential as issueCodec, test_reconcile_portable_credential as lookupCodec } from 'canic-issuer-codecs';
import { open as openJournal, admissionContext } from './auth-browser.mjs';
export { begin, state, signingRefused, cleanup } from './auth-browser.mjs';

let input, admissions;
function require(value, message) { if (!value) throw new Error(message); }
function count(kind, index) {
  const key = `${kind}-${index}`;
  const next = Number(localStorage.getItem(key) ?? '0') + 1;
  localStorage.setItem(key, String(next));
  return next;
}
async function response(identity, signal, codec, bytes) {
  const agent = await HttpAgent.create({ host: location.origin, identity, retryTimes: 0,
    shouldFetchRootKey: false, shouldSyncTime: false, rootKey: Uint8Array.from(input.rootKey),
    fetch: (request, options) => fetch(request, { ...options, signal }) });
  const reply = await agent.update(Principal.fromText(input.issuer), {
    methodName: codec.methodName, arg: Uint8Array.from(bytes), callSync: false,
  }, { preSignReadStateRequest: false });
  return codec.decodeResponse(reply.reply);
}
async function call(identity, signal, codec, bytes) {
  const decoded = await response(identity, signal, codec, bytes);
  require('ok' in decoded, 'issuer returned a typed refusal');
  return decoded.ok;
}
function result(value) {
  return { credential: value.credential, admissionUntilNs: value.admission_deadline_ns };
}
async function fingerprint(credential) {
  const bytes = new TextEncoder().encode(JSON.stringify(credential,
    (_, value) => typeof value === 'bigint' ? value.toString() : value));
  return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).join(',');
}
export async function open(config, create = false) {
  input = config;
  await openJournal(input, create, false);
  const { clients, identities } = admissionContext();
  const account = Ed25519KeyIdentity.generate(new Uint8Array(32).fill(input.account));
  admissions = clients.map((journal, index) => {
    const key = `issuance-intent-${index}`;
    const exact = JSON.stringify({ request: input.requests[index], lookup: input.lookups[index] });
    if (create) { require(localStorage.getItem(key) === null, 'fresh intent'); localStorage.setItem(key, exact); }
    require(localStorage.getItem(key) === exact, 'reopened intent unchanged');
    const intent = JSON.parse(localStorage.getItem(key));
    return new PortableAdmissionClient(journal, `device-${input.devices[index]}`,
      { browser: identities[index], account }, Principal.fromText(input.issuer), {
        issue: async (identity, signal) => {
          require(count('issue', index) === 1, 'no duplicate issuance');
          const value = await call(identity, signal, issueCodec, intent.request);
          // Lose the completed result before the admission client can retain it.
          localStorage.setItem(`original-deadline-${index}`, value.admission_deadline_ns.toString());
          localStorage.setItem(`original-credential-${index}`, await fingerprint(value.credential));
          throw new ClientError('transport');
        },
        reconcile: async (identity, signal) => {
          count('lookup', index);
          const status = await call(identity, signal, lookupCodec, intent.lookup);
          require(status.tag === 'Issued', 'lookup returns original issued credential');
          require(status.value.admission_deadline_ns.toString() === localStorage.getItem(`original-deadline-${index}`), 'original expiry retained');
          return result(status.value);
        },
        admit: httpAdmissionTransport({ host: location.origin, rootKey: Uint8Array.from(input.rootKey),
          methodName: 'canic_establish_portable_session', scopes: input.scopes }),
      });
  });
}
export async function loseIssuance() {
  const results = await Promise.allSettled(admissions.map(client => client.establish(Principal.fromText(input.targets[0]))));
  require(results.every(result => result.status === 'rejected' && result.reason instanceof ClientError && result.reason.code === 'transport'), 'issuance outcome unknown');
  const { clients } = admissionContext();
  for (let index = 0; index < clients.length; index++) {
    require((await clients[index].status(`device-${input.devices[index]}`)).issuanceUnknown, 'durable issuance claim');
  }
}
export async function establish() {
  await Promise.all(admissions.flatMap(client => input.targets.map(target => client.establish(Principal.fromText(target)))));
  for (let index = 0; index < admissions.length; index++) {
    require(localStorage.getItem(`issue-${index}`) === '1', 'reopen never reissues');
    require(localStorage.getItem(`lookup-${index}`) === '1', 'one shared reconciliation per device');
  }
}

// Possession of exact public operation bytes never grants account authority.
export async function refuseOtherAccounts() {
  const { clients, identities } = admissionContext();
  const unrelated = Ed25519KeyIdentity.generate(new Uint8Array(32).fill(71));
  await Promise.all(clients.map(async (journal, index) => {
    const intent = JSON.parse(localStorage.getItem(`issuance-intent-${index}`));
    const before = await journal.status(`device-${input.devices[index]}`);
    await Promise.all([identities[index], unrelated, undefined].map(async identity => {
      const anonymous = identity === undefined;
      const lookup = await response(identity, undefined, lookupCodec, intent.lookup);
      require(anonymous ? lookup.err?.tag === 'Denied' : lookup.ok?.tag === 'Unresolved',
        'only the original account can reconcile a credential');
      const issue = await response(identity, undefined, issueCodec, intent.request);
      require(issue.err?.tag === 'Denied', 'browser and unrelated identities cannot issue');
    }));
    const after = await journal.status(`device-${input.devices[index]}`);
    require(before.issuanceUnknown === after.issuanceUnknown && before.phase === after.phase,
      'unauthorized requests leave the original journal unchanged');
    const account = Ed25519KeyIdentity.generate(new Uint8Array(32).fill(input.account));
    const retained = await call(account, undefined, lookupCodec, intent.lookup);
    require(retained.tag === 'Issued'
      && retained.value.admission_deadline_ns.toString() === localStorage.getItem(`original-deadline-${index}`),
      'original account still retrieves the retained result without extending expiry');
    require(await fingerprint(retained.value.credential) === localStorage.getItem(`original-credential-${index}`),
      'original account retrieves the exact credential after unauthorized requests');
  }));
}

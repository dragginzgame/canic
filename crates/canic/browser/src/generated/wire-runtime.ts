// Candidate interpreter for the admitted schema subset. See
// docs/design/browser-runtime.md for source provenance and the reuse contract.
// Generated metadata is trusted; allowances do not bound the JS engine heap.
export const schemaFormat = 'ic-wire-json';
export class WireError extends Error {
  override name = 'WireError';
  constructor(readonly code: 'limit' | 'malformed' | 'identity' | 'value') { super(`wire ${code}`); }
}
type Field = { name: string; value: Schema };
export type Schema = ({ k: 'unit' | 'bool' }
  | { k: 'u' | 'i'; bits: number; min?: number; max?: number }
  | { k: 'float'; bits: number; min?: number; max?: number }
  | { k: 'bytes'; min?: number; max?: number }
  | { k: 'text'; max?: number }
  | { k: 'packed'; max?: number; word: 'bool'; nonzero?: never }
  | { k: 'packed'; max?: number; word: 'i64'; nonzero?: true }
  | { k: 'packed'; max?: number; word: Exclude<keyof typeof packedArrays, 'i64'>; nonzero?: never }
  | { k: 'list'; item: Schema; max?: number }
  | { k: 'array'; item: Schema; max: number }
  | { k: 'tuple'; items: readonly Schema[] }
  | { k: 'result'; ok: Schema; err: Schema }
  | { k: 'option' | 'newtype'; item: Schema }
  | { k: 'record'; fields: readonly Field[] }
  | { k: 'enum'; variants: readonly (Field & { id: number })[] }
  | { k: 'ref'; name: string }) & {check?:string};
export type Checks=Readonly<Record<string,(value:unknown)=>void>>;
type Types = Readonly<Record<string, Schema>>;
type Payload = { schema: Schema; bytes: number; nodes: number; depth: number };
type Method = { id: string; request: Schema; response: Schema; requestBytes: number; responseBytes: number; requestNodes: number; responseNodes: number; requestDepth: number; responseDepth: number };
const fail = (code: WireError['code']): never => { throw new WireError(code); };
const utf8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });
const utf16 = new TextEncoder();
const littleEndian = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;
const packedArrays = {i8:Int8Array,i16:Int16Array,u16:Uint16Array,i32:Int32Array,u32:Uint32Array,i64:BigInt64Array,u64:BigUint64Array,hash16:Uint8Array,hash32:Uint8Array,'finite-f32':Float32Array,'finite-f64':Float64Array} as const;
function finiteWords(bytes: Uint8Array, width: number, little: boolean): boolean {
  const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  for(let offset=0;offset<bytes.length;offset+=width) {
    if(!Number.isFinite(width===4?view.getFloat32(offset,little):view.getFloat64(offset,little))) return false;
  }
  return true;
}
function swapWords(bytes: Uint8Array, width: number): void {
  for (let start=0; start<bytes.length; start+=width) {
    for (let i=0; i<width/2; i++) { const other=start+width-1-i, low=bytes[start+i]!; bytes[start+i]=bytes[other]!; bytes[other]=low; }
  }
}
function object(value: unknown, keys: readonly (string | Field)[]): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fail('value');
  if (Reflect.ownKeys(value).length !== keys.length || keys.some(key => !Object.hasOwn(value, typeof key === 'string' ? key : key.name))) return fail('value');
  return value as Record<string, unknown>;
}
class Budget {
  constructor(public remaining: number, readonly depth: number) {}
  visit(depth: number) { if (--this.remaining < 0 || depth > this.depth) fail('limit'); }
  length(length: number) { if (length > this.remaining) fail('limit'); }
}
function resolve(schema: Schema, types: Types, visit?: (schema:Schema)=>void): Schema {
  // Reference checkpoint algorithm: finite alias walk, no per-value registry copy.
  let checkpoint: string | undefined, distance = 0, span = 1;
  while (schema.k === 'ref') {
    if (schema.name === checkpoint || !Object.hasOwn(types, schema.name)) return fail('value');
    if (distance++ === span) { checkpoint = schema.name; span *= 2; distance = 1; }
    visit?.(schema);
    const next = types[schema.name];
    if (next === undefined) return fail('value');
    schema = next;
  }
  visit?.(schema);
  return schema;
}
function validate(schema:Schema,value:unknown,checks:Checks):void {
  if(schema.check!==undefined) {
    if(!Object.hasOwn(checks,schema.check)) return fail('value');
    const check=checks[schema.check];
    if(check===undefined) return fail('value');
    check(value);
  }
}
class Writer {
  bytes: Uint8Array;
  constructor(readonly limit: number, public offset = 0) { this.bytes = new Uint8Array(Math.min(limit, 64)); }
  private grow(required: number) {
    if (required > this.limit) fail('limit');
    const next = new Uint8Array(Math.min(this.limit, Math.max(required, this.bytes.length * 2)));
    next.set(this.bytes); this.bytes = next;
  }
  header(id: bigint, direction: number) {
    this.bytes.set([73, 87]); new DataView(this.bytes.buffer).setBigUint64(2, id, true); this.bytes[10] = direction;
  }
  byte(value: number) { if (this.offset === this.bytes.length) this.grow(this.offset + 1); this.bytes[this.offset++] = value; }
  varint(value: bigint) {
    do { const byte = Number(value & 127n); value >>= 7n; this.byte(value ? byte | 128 : byte); } while (value);
  }
  raw(bytes: Uint8Array) {
    if (bytes.length > this.bytes.length - this.offset) this.grow(this.offset + bytes.length);
    this.bytes.set(bytes, this.offset); this.offset += bytes.length;
  }
  finish(): Uint8Array { return this.offset === this.bytes.length ? this.bytes : this.bytes.slice(0, this.offset); }
}
class Reader {
  readonly view: DataView;
  constructor(readonly bytes: Uint8Array, public offset = 0) { this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength); }
  byte(): number { const value = this.bytes[this.offset++]; if (value === undefined) return fail('malformed'); return value; }
  varint(bits: number): bigint {
    let value = 0n;
    for (let i = 0; i < Math.ceil(bits / 7); i++) {
      const byte = this.byte(); value |= BigInt(byte & 127) << BigInt(i * 7);
      if (!(byte & 128)) {
        if (value >= (1n << BigInt(bits))) return fail('malformed');
        return value;
      }
    }
    return fail('malformed');
  }
  // Preserve Wasm32 collection lengths and admission refusal ordering.
  length(max: number): number { const n = Number(this.varint(32)); if (n > max) return fail('limit'); return n; }
  raw(length: number): Uint8Array {
    if (length > this.bytes.length - this.offset) return fail('malformed');
    const span = this.bytes.subarray(this.offset, this.offset + length); this.offset += length; return span;
  }
}
function write(schema: Schema, value: unknown, out: Writer, budget: Budget, depth: number, types: Types, checks:Checks): void {
  budget.visit(depth);
  const s = resolve(schema, types, schema=>validate(schema,value,checks));
  const child = (item: Schema, value: unknown) => write(item, value, out, budget, depth + 1, types,checks);
  switch (s.k) {
    case 'unit': if (value !== null) fail('value'); return;
    case 'bool': if (typeof value !== 'boolean') fail('value'); out.byte(value ? 1 : 0); return;
    case 'float': {
      if (typeof value !== 'number' || !Number.isFinite(value) || (s.bits === 32 && !Object.is(Math.fround(value), value))) return fail('value');
      if(s.min!==undefined && value<s.min || s.max!==undefined && value>s.max) return fail('value');
      const bytes = new Uint8Array(s.bits / 8), view = new DataView(bytes.buffer);
      if (s.bits === 32) view.setFloat32(0, value, true); else view.setFloat64(0, value, true);
      out.raw(bytes); return;
    }
    case 'u': case 'i': {
      if (s.bits <= 32 ? typeof value !== 'number' || !Number.isSafeInteger(value) : typeof value !== 'bigint') fail('value');
      const n = BigInt(value as number | bigint), bits = BigInt(s.bits), signed = s.k === 'i';
      const min = signed ? -(1n << (bits - 1n)) : 0n, max = (1n << (bits - (signed ? 1n : 0n))) - 1n;
      if (n < min || n > max) fail('limit');
      if(s.min!==undefined && n<BigInt(s.min) || s.max!==undefined && n>BigInt(s.max)) return fail('value');
      if (s.bits === 8) out.byte(Number(n & 255n));
      else out.varint(signed ? (n << 1n) ^ (n >> (bits - 1n)) : n);
      return;
    }
    case 'bytes': {
      if (!(value instanceof Uint8Array)) return fail('value');
      if (s.min !== undefined && value.length < s.min) fail('value');
      if (s.max !== undefined && value.length > s.max) fail('limit');
      out.varint(BigInt(value.length)); out.raw(value); return;
    }
    case 'packed': {
      if(s.word==='bool') {
        if(!Array.isArray(value)) return fail('value');
        if(s.max !== undefined && value.length > s.max) return fail('limit');
        out.varint(BigInt(value.length));
        if(value.length>out.limit-out.offset) return fail('limit');
        for(const item of value) { if(typeof item!=='boolean') return fail('value'); out.byte(item?1:0); }
        return;
      }

      const kind=s.word, ctor=packedArrays[kind];
      if (!(value instanceof ctor)) return fail('value');
      const opaque=kind==='hash16'||kind==='hash32', width=kind==='hash16'?16:kind==='hash32'?32:ctor.BYTES_PER_ELEMENT;
      if(value.byteLength%width) return fail('value');
      if (s.max !== undefined && value.byteLength / width > s.max) fail('limit');
      out.varint(BigInt(value.byteLength));
      if (value.byteLength > out.limit - out.offset) fail('limit');
      if(s.nonzero && (value as BigInt64Array).includes(0n)) return fail('value');
      const span = new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
      if((kind==='finite-f32'||kind==='finite-f64') && !finiteWords(span,width,littleEndian)) return fail('value');
      if (littleEndian || opaque) out.raw(span);
      else { const copy = new Uint8Array(span); swapWords(copy,width); out.raw(copy); }
      return;
    }
    case 'text': {
      if (typeof value !== 'string') return fail('value');
      const remaining = Math.min(s.max ?? out.limit, out.limit - out.offset);
      if (value.length > remaining) fail('limit');
      const buffer = new Uint8Array(Math.min(remaining, value.length * 3)), encoded = utf16.encodeInto(value, buffer);
      if (encoded.read !== value.length) fail('limit');
      const span = buffer.subarray(0, encoded.written);
      if (utf8.decode(span) !== value) fail('value');
      out.varint(BigInt(span.length)); out.raw(span); return;
    }
    case 'newtype': child(s.item, value); return;
    case 'list': case 'array': case 'tuple': {
      if (!Array.isArray(value)) return fail('value');
      if (s.k === 'array' && value.length !== s.max || s.k === 'list' && value.length > (s.max ?? budget.remaining)) fail('limit');
      if (s.k === 'tuple' && value.length !== s.items.length) fail('value');
      budget.length(value.length);
      if (s.k === 'list') out.varint(BigInt(value.length));
      for (let i = 0; i < value.length; i++) {
        if (!Object.hasOwn(value, i)) fail('value');
        const item = s.k === 'tuple' ? s.items[i] : s.item;
        if (item === undefined) return fail('value');
        child(item, value[i]);
      }
      return;
    }
    case 'option': {
      if (value === null) { out.byte(0); return; }
      const record = object(value, ['some']); out.byte(1); child(s.item, record.some); return;
    }
    case 'result': {
      const key = value && typeof value === 'object' && Object.hasOwn(value, 'ok') ? 'ok' : 'err';
      const record = object(value, [key]); out.byte(key === 'ok' ? 0 : 1); child(s[key], record[key]); return;
    }
    case 'record': {
      const record = object(value, s.fields);
      for (const field of s.fields) child(field.value, record[field.name]);
      return;
    }
    case 'enum': {
      const record = object(value, ['tag', 'value']), variant = s.variants.find(v => v.name === record.tag);
      if (!variant) return fail('value');
      out.varint(BigInt(variant.id)); child(variant.value, record.value); return;
    }
    default: return fail('value');
  }
}
function read(schema: Schema, input: Reader, budget: Budget, depth: number, types: Types, checks:Checks): unknown {
  const value=readValue(resolve(schema,types),input,budget,depth,types,checks);
  resolve(schema,types,schema=>validate(schema,value,checks));
  return value;
}
function readValue(schema: Schema, input: Reader, budget: Budget, depth: number, types: Types, checks:Checks): unknown {
  // The wrapper resolves aliases before charging the concrete value.
  const s = schema;
  budget.visit(depth);
  const child = (item: Schema) => read(item, input, budget, depth + 1, types,checks);
  switch (s.k) {
    case 'unit': return null;
    case 'bool': { const byte = input.byte(); if (byte > 1) return fail('malformed'); return byte === 1; }
    case 'float': {
      const bytes = input.raw(s.bits / 8), view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
      const value = s.bits === 32 ? view.getFloat32(0, true) : view.getFloat64(0, true);
      if (!Number.isFinite(value) || s.min!==undefined && value<s.min || s.max!==undefined && value>s.max) return fail('malformed'); return value;
    }
    case 'u': case 'i': {
      let n = s.bits === 8 ? BigInt(input.byte()) : input.varint(s.bits);
      if (s.k === 'i') n = s.bits === 8 ? (n >= 128n ? n - 256n : n) : (n >> 1n) ^ -(n & 1n);
      if(s.min!==undefined && n<BigInt(s.min) || s.max!==undefined && n>BigInt(s.max)) return fail('malformed');
      return s.bits <= 32 ? Number(n) : n;
    }
    case 'bytes': {
      const length = input.length(s.max ?? input.bytes.length - input.offset);
      if (s.min !== undefined && length < s.min) fail('malformed');
      return new Uint8Array(input.raw(length));
    }
    case 'packed': {
      if(s.word==='bool') {
        const span=input.raw(input.length(Math.min(s.max ?? input.bytes.length, input.bytes.length-input.offset)));
        for(const byte of span) if(byte>1) return fail('malformed');
        const values=new Array<boolean>(span.length);
        for(let i=0;i<span.length;i++) values[i]=span[i]===1;
        return values;
      }

      const kind=s.word, ctor=packedArrays[kind];
      const opaque=kind==='hash16'||kind==='hash32', width=kind==='hash16'?16:kind==='hash32'?32:ctor.BYTES_PER_ELEMENT;
      const length = input.length(Math.min(s.max === undefined ? input.bytes.length : s.max * width, input.bytes.length - input.offset));
      if (length % width) return fail('malformed');
      if (length === 0) return new ctor(0);
      const span=input.raw(length);
      if((kind==='finite-f32'||kind==='finite-f64') && !finiteWords(span,width,true)) return fail('malformed');
      const bytes = new Uint8Array(span);
      if (!littleEndian && !opaque) swapWords(bytes,width);
      const value=new ctor(bytes.buffer);
      if(s.nonzero && (value as BigInt64Array).includes(0n)) return fail('malformed');
      return value;
    }
    case 'text': return utf8.decode(input.raw(input.length(Math.min(s.max ?? input.bytes.length, input.bytes.length - input.offset))));
    case 'newtype': return child(s.item);
    case 'list': case 'array': case 'tuple': {
      const length = s.k === 'list' ? input.length(Math.min(s.max ?? budget.remaining, budget.remaining)) : s.k === 'array' ? s.max : s.items.length;
      budget.length(length);
      const values: unknown[] = [];
      for (let i = 0; i < length; i++) {
        const item = s.k === 'tuple' ? s.items[i] : s.item;
        if (item === undefined) return fail('value');
        values.push(child(item));
      }
      return values;
    }
    case 'option': {
      const tag = input.byte(); if (tag === 0) return null; if (tag !== 1) return fail('malformed');
      return { some: child(s.item) };
    }
    case 'result': {
      const tag = input.varint(32); if (tag > 1n) return fail('malformed');
      return tag === 0n ? { ok: child(s.ok) } : { err: child(s.err) };
    }
    case 'record': {
      const value = Object.create(null) as Record<string, unknown>;
      for (const field of s.fields) value[field.name] = child(field.value);
      return value;
    }
    case 'enum': {
      const id = Number(input.varint(32)), variant = s.variants.find(v => v.id === id);
      if (!variant) return fail('malformed');
      return { tag: variant.name, value: child(variant.value) };
    }
    default: return fail('value');
  }
}
function decodeError(error: unknown): never { if (error instanceof WireError) throw error; return fail('malformed'); }

// Factories consume generated private metadata. They do not validate arbitrary
// external schemas or clone/protect objects supplied by a custom trusted caller.
export function createPayloadCodec(metadata: unknown, registry: unknown = {}, checks:Checks = {}) {
  const meta = metadata as Payload, types = registry as Types;
  if (!Number.isSafeInteger(meta.bytes) || meta.bytes < 0) fail('value');
  for (const value of [meta.nodes, meta.depth]) if (!Number.isSafeInteger(value) || value < 1) fail('value');
  return {
    encode(value: unknown): Uint8Array {
      const out = new Writer(meta.bytes);
      write(meta.schema, value, out, new Budget(meta.nodes, meta.depth), 1, types,checks);
      return out.finish();
    },
    decode(bytes: Uint8Array): unknown {
      if (!(bytes instanceof Uint8Array)) return fail('value');
      if (bytes.length > meta.bytes) return fail('limit');
      try {
        const input = new Reader(bytes);
        const value = read(meta.schema, input, new Budget(meta.nodes, meta.depth), 1, types,checks);
        if (input.offset !== bytes.length) return fail('malformed');
        return value;
      } catch (error) { return decodeError(error); }
    }
  };
}
export function createCodec(metadata: unknown, registry: unknown = {}, checks:Checks = {}) {
  const meta = metadata as Method, types = registry as Types, id = BigInt(meta.id);
  if (id < 0n || id >= (1n << 64n)) fail('value');
  for (const value of [meta.requestBytes, meta.responseBytes, meta.requestNodes, meta.responseNodes, meta.requestDepth, meta.responseDepth]) {
    if (!Number.isSafeInteger(value) || value < 1) fail('value');
  }
  if (meta.requestBytes < 11 || meta.responseBytes < 11) fail('value');
  function encode(value: unknown, direction: 0 | 1): Uint8Array {
    const out = new Writer(direction ? meta.responseBytes : meta.requestBytes, 11);
    out.header(id, direction);
    write(direction ? meta.response : meta.request, value, out, new Budget(direction ? meta.responseNodes : meta.requestNodes, direction ? meta.responseDepth : meta.requestDepth), 1, types,checks);
    return out.finish();
  }
  function decode(bytes: Uint8Array, direction: 0 | 1): unknown {
    if (!(bytes instanceof Uint8Array)) return fail('value');
    if (bytes.length > (direction ? meta.responseBytes : meta.requestBytes)) return fail('limit');
    if (bytes.length < 11) return fail('malformed');
    if (bytes[0] !== 73 || bytes[1] !== 87 || bytes[10] !== direction) return fail('identity');
    try {
      const input = new Reader(bytes, 11);
      if (input.view.getBigUint64(2, true) !== id) return fail('identity');
      const value = read(direction ? meta.response : meta.request, input, new Budget(direction ? meta.responseNodes : meta.requestNodes, direction ? meta.responseDepth : meta.requestDepth), 1, types,checks);
      if (input.offset !== bytes.length) return fail('malformed');
      return value;
    } catch (error) { return decodeError(error); }
  }
  return { encodeRequest: (value: unknown) => encode(value, 0), decodeRequest: (bytes: Uint8Array) => decode(bytes, 0), encodeResponse: (value: unknown) => encode(value, 1), decodeResponse: (bytes: Uint8Array) => decode(bytes, 1) };
}

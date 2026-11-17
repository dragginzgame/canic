// Generated IC Wire client; no transport or execution.
import { schemaFormat as $format, createCodec as $create } from "./wire-runtime.js";
if ($format !== "ic-wire-json") throw new globalThis.Error('IC Wire runtime format mismatch');
export type Request = Wire_Logout_dd38589c16dd7f86;
export type Response = Wire_Result_b2e96e4fdf15f20a;
export type Wire_Bytes_ceafa818703233a0 = /* max 29 */ globalThis.Uint8Array;
export type Wire_Logout_dd38589c16dd7f86 = { readonly "account": Wire_Principal_f14095eb1cc0831f; readonly "login_epoch": Wire_u64_4d35d3193e8d66f2; readonly "login_id": Wire__u8__32__a6c3ffa4fc6f7348; };
export type Wire_Principal_f14095eb1cc0831f = Wire_Bytes_ceafa818703233a0;
export type Wire_Refusal_9145e7175e78ab04 = /* id 0 */ { readonly tag: "Invalid"; readonly value: null } | /* id 1 */ { readonly tag: "Denied"; readonly value: null } | /* id 2 */ { readonly tag: "Conflict"; readonly value: null } | /* id 3 */ { readonly tag: "Expired"; readonly value: null } | /* id 4 */ { readonly tag: "Capacity"; readonly value: null } | /* id 5 */ { readonly tag: "Throttled"; readonly value: null } | /* id 6 */ { readonly tag: "Unavailable"; readonly value: null };
export type Wire_Result_b2e96e4fdf15f20a = { readonly ok: Wire_Revoked_a2507b8ccbd1e0c6 } | { readonly err: Wire_Refusal_9145e7175e78ab04 };
export type Wire_Revoked_a2507b8ccbd1e0c6 = { readonly "login_deadline_ns": Wire_u64_4d35d3193e8d66f2; };
export type Wire__u8__32__a6c3ffa4fc6f7348 = /* max 32 */ globalThis.ReadonlyArray<Wire_u8_08c48207b56753d8>;
export type Wire_u64_4d35d3193e8d66f2 = /* u64 */ bigint;
export type Wire_u8_08c48207b56753d8 = /* u8 */ number;
const $types = globalThis.JSON.parse("{\"Request\":{\"k\":\"ref\",\"name\":\"Wire_Logout_dd38589c16dd7f86\"},\"Response\":{\"k\":\"ref\",\"name\":\"Wire_Result_b2e96e4fdf15f20a\"},\"Wire_Bytes_ceafa818703233a0\":{\"k\":\"bytes\",\"max\":29},\"Wire_Logout_dd38589c16dd7f86\":{\"k\":\"record\",\"fields\":[{\"name\":\"account\",\"value\":{\"k\":\"ref\",\"name\":\"Wire_Principal_f14095eb1cc0831f\"}},{\"name\":\"login_epoch\",\"value\":{\"k\":\"ref\",\"name\":\"Wire_u64_4d35d3193e8d66f2\"}},{\"name\":\"login_id\",\"value\":{\"k\":\"ref\",\"name\":\"Wire__u8__32__a6c3ffa4fc6f7348\"}}]},\"Wire_Principal_f14095eb1cc0831f\":{\"k\":\"ref\",\"name\":\"Wire_Bytes_ceafa818703233a0\"},\"Wire_Refusal_9145e7175e78ab04\":{\"k\":\"enum\",\"variants\":[{\"name\":\"Invalid\",\"id\":0,\"value\":{\"k\":\"unit\"}},{\"name\":\"Denied\",\"id\":1,\"value\":{\"k\":\"unit\"}},{\"name\":\"Conflict\",\"id\":2,\"value\":{\"k\":\"unit\"}},{\"name\":\"Expired\",\"id\":3,\"value\":{\"k\":\"unit\"}},{\"name\":\"Capacity\",\"id\":4,\"value\":{\"k\":\"unit\"}},{\"name\":\"Throttled\",\"id\":5,\"value\":{\"k\":\"unit\"}},{\"name\":\"Unavailable\",\"id\":6,\"value\":{\"k\":\"unit\"}}]},\"Wire_Result_b2e96e4fdf15f20a\":{\"k\":\"result\",\"ok\":{\"k\":\"ref\",\"name\":\"Wire_Revoked_a2507b8ccbd1e0c6\"},\"err\":{\"k\":\"ref\",\"name\":\"Wire_Refusal_9145e7175e78ab04\"}},\"Wire_Revoked_a2507b8ccbd1e0c6\":{\"k\":\"record\",\"fields\":[{\"name\":\"login_deadline_ns\",\"value\":{\"k\":\"ref\",\"name\":\"Wire_u64_4d35d3193e8d66f2\"}}]},\"Wire__u8__32__a6c3ffa4fc6f7348\":{\"k\":\"array\",\"max\":32,\"item\":{\"k\":\"ref\",\"name\":\"Wire_u8_08c48207b56753d8\"}},\"Wire_u64_4d35d3193e8d66f2\":{\"k\":\"u\",\"bits\":64},\"Wire_u8_08c48207b56753d8\":{\"k\":\"u\",\"bits\":8}}");
export const codec = (() => {
const $codec = $create({id:'73947883522067',request:{k:'ref',name:"Request"},response:{k:'ref',name:"Response"},requestBytes:8192, requestNodes:4096, requestDepth:12,responseBytes:131072, responseNodes:16384, responseDepth:16}, $types);
return globalThis.Object.freeze({
encodeRequest:(value: Request): globalThis.Uint8Array => $codec.encodeRequest(value), decodeRequest:(bytes: globalThis.Uint8Array): Request => $codec.decodeRequest(bytes) as Request,
encodeResponse:(value: Response): globalThis.Uint8Array => $codec.encodeResponse(value), decodeResponse:(bytes: globalThis.Uint8Array): Response => $codec.decodeResponse(bytes) as Response,
});
})();

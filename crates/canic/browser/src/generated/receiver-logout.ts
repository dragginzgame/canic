// Generated IC Wire client; no transport or execution.
import { schemaFormat as $format, createCodec as $create } from "./wire-runtime.js";
if ($format !== "ic-wire-json") throw new globalThis.Error('IC Wire runtime format mismatch');
export type Request = Wire____07e11f07b4a6665a;
export type Response = Wire_LogoutResult_983ac2b3d5afb6ea;
export type Wire_LogoutResult_983ac2b3d5afb6ea = /* id 0 */ { readonly tag: "Revoked"; readonly value: Wire_u64_4d35d3193e8d66f2 } | /* id 1 */ { readonly tag: "Unconfirmed"; readonly value: Wire_u64_4d35d3193e8d66f2 } | /* id 2 */ { readonly tag: "InvalidCaller"; readonly value: null } | /* id 3 */ { readonly tag: "Unavailable"; readonly value: Wire_LogoutUnavailable_6f86044fa22ff487 };
export type Wire_LogoutUnavailable_6f86044fa22ff487 = /* id 0 */ { readonly tag: "Capacity"; readonly value: null } | /* id 1 */ { readonly tag: "Storage"; readonly value: null } | /* id 2 */ { readonly tag: "Clock"; readonly value: null } | /* id 3 */ { readonly tag: "ResetPending"; readonly value: null };
export type Wire____07e11f07b4a6665a = null;
export type Wire_u64_4d35d3193e8d66f2 = /* u64 */ bigint;
const $types = globalThis.JSON.parse("{\"Request\":{\"k\":\"ref\",\"name\":\"Wire____07e11f07b4a6665a\"},\"Response\":{\"k\":\"ref\",\"name\":\"Wire_LogoutResult_983ac2b3d5afb6ea\"},\"Wire_LogoutResult_983ac2b3d5afb6ea\":{\"k\":\"enum\",\"variants\":[{\"name\":\"Revoked\",\"id\":0,\"value\":{\"k\":\"ref\",\"name\":\"Wire_u64_4d35d3193e8d66f2\"}},{\"name\":\"Unconfirmed\",\"id\":1,\"value\":{\"k\":\"ref\",\"name\":\"Wire_u64_4d35d3193e8d66f2\"}},{\"name\":\"InvalidCaller\",\"id\":2,\"value\":{\"k\":\"unit\"}},{\"name\":\"Unavailable\",\"id\":3,\"value\":{\"k\":\"ref\",\"name\":\"Wire_LogoutUnavailable_6f86044fa22ff487\"}}]},\"Wire_LogoutUnavailable_6f86044fa22ff487\":{\"k\":\"enum\",\"variants\":[{\"name\":\"Capacity\",\"id\":0,\"value\":{\"k\":\"unit\"}},{\"name\":\"Storage\",\"id\":1,\"value\":{\"k\":\"unit\"}},{\"name\":\"Clock\",\"id\":2,\"value\":{\"k\":\"unit\"}},{\"name\":\"ResetPending\",\"id\":3,\"value\":{\"k\":\"unit\"}}]},\"Wire____07e11f07b4a6665a\":{\"k\":\"unit\"},\"Wire_u64_4d35d3193e8d66f2\":{\"k\":\"u\",\"bits\":64}}");
export const codec = (() => {
const $codec = $create({id:'71834742030356',request:{k:'ref',name:"Request"},response:{k:'ref',name:"Response"},requestBytes:11, requestNodes:8, requestDepth:4,responseBytes:4096, responseNodes:128, responseDepth:8}, $types);
return globalThis.Object.freeze({
encodeRequest:(value: Request): globalThis.Uint8Array => $codec.encodeRequest(value), decodeRequest:(bytes: globalThis.Uint8Array): Request => $codec.decodeRequest(bytes) as Request,
encodeResponse:(value: Response): globalThis.Uint8Array => $codec.encodeResponse(value), decodeResponse:(bytes: globalThis.Uint8Array): Response => $codec.decodeResponse(bytes) as Response,
});
})();

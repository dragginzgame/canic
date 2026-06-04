# canic-ic-registry

Shared host-side adapter for reading the public Internet Computer NNS registry
into Canic subnet catalog structs.

This crate owns live registry transport, protobuf encoding/decoding, registry
key construction, high-capacity registry value reconstruction, SHA-256 chunk
validation, and conversion into `canic-subnet-catalog` domain data.

No protobuf transport type, Candid chunk request/response type, or registry
chunk key type is part of its public API.

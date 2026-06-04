# canic-ic-registry

Shared host-side adapter for reading the public Internet Computer NNS registry
into Canic subnet catalog structs.

This crate owns live registry transport, protobuf encoding/decoding, registry
key construction, and conversion into `canic-subnet-catalog` domain data. No
protobuf transport type is part of its public API.

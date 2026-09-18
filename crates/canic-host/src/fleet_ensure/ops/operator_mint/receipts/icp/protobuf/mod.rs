//! Narrow current ICP block projection from ic_ledger/pb/v1/types.proto.
//!
//! Unknown operations, extensions and fields fail the owning decoder's exact
//! re-encoding check. These types do not reproduce Ledger state or defaults.

#[derive(Clone, prost::Message, PartialEq)]
pub(super) struct Block {
    #[prost(message, optional, tag = "1")]
    pub parent_hash: Option<BytesValue>,
    #[prost(message, optional, tag = "2")]
    pub timestamp: Option<UnsignedValue>,
    #[prost(message, optional, tag = "3")]
    pub transaction: Option<Transaction>,
}

#[derive(Clone, prost::Message, PartialEq)]
pub(super) struct Transaction {
    #[prost(message, optional, tag = "3")]
    pub send: Option<Send>,
    #[prost(message, optional, tag = "4")]
    pub memo: Option<UnsignedValue>,
    #[prost(message, optional, tag = "6")]
    pub created_at_time: Option<UnsignedValue>,
}

#[derive(Clone, prost::Message, PartialEq)]
pub(super) struct Send {
    #[prost(message, optional, tag = "1")]
    pub from: Option<BytesValue>,
    #[prost(message, optional, tag = "2")]
    pub to: Option<BytesValue>,
    #[prost(message, optional, tag = "3")]
    pub amount: Option<UnsignedValue>,
    #[prost(message, optional, tag = "4")]
    pub fee: Option<UnsignedValue>,
}

// The upstream Hash/AccountIdentifier and Tokens/Memo/TimeStamp wrappers each
// use field 1 with the wire type represented here. Required presence is checked
// by the owner; absent wrappers never become default fees or timestamps.
#[derive(Clone, prost::Message, PartialEq)]
pub(super) struct BytesValue {
    #[prost(bytes = "vec", tag = "1")]
    pub value: Vec<u8>,
}

#[derive(Clone, prost::Message, PartialEq)]
pub(super) struct UnsignedValue {
    #[prost(uint64, tag = "1")]
    pub value: u64,
}

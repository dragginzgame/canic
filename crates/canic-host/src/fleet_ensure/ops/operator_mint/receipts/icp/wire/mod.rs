//! Module: fleet_ensure::ops::operator_mint::receipts::icp::wire
//!
//! Responsibility: exact ICP Ledger and archive read wire projections.
//! Boundary: decoding these passive values does not authenticate receipts.

use candid::CandidType;
use icrc_ledger_types::icrc3::archive::QueryArchiveFn;
use serde::Deserialize;
use serde_bytes::ByteBuf;
use thiserror::Error;

/// Typed archive failures retained as unresolved evidence by the receipt reader.
#[derive(CandidType, Debug, Deserialize, Error)]
pub enum ArchiveReadError {
    #[error("requested ICP block precedes the archive range")]
    BadFirstBlockIndex {
        /// Index requested from this archive.
        requested_index: u64,
        /// First available archive index.
        first_valid_index: u64,
    },
    #[error("ICP archive rejected the receipt read")]
    Other {
        /// Upstream archive error code.
        error_code: u64,
        /// Bounded upstream diagnostic, never used as an admission decision.
        error_message: String,
    },
}

#[derive(CandidType)]
pub(super) struct ReadArgs {
    pub start: u64,
    pub length: u64,
}

#[derive(CandidType, Deserialize)]
pub(super) struct EncodedBlocksReply {
    pub chain_length: u64,
    pub first_block_index: u64,
    pub blocks: Vec<ByteBuf>,
    pub archived_blocks: Vec<ArchivedRange>,
}

#[derive(CandidType, Deserialize)]
pub(super) struct ArchivedRange {
    pub callback: QueryArchiveFn<ReadArgs, Result<Vec<ByteBuf>, ArchiveReadError>>,
    pub start: u64,
    pub length: u64,
}

use crate::{
    dto::auth::{AttestationKey, AttestationKeyStatus},
    storage::stable::auth::{AttestationKeyStatusRecord, AttestationPublicKeyRecord},
};

///
/// AttestationPublicKeyRecordMapper
///

pub struct AttestationPublicKeyRecordMapper;

impl AttestationPublicKeyRecordMapper {
    #[must_use]
    pub fn dto_to_record(key: AttestationKey) -> AttestationPublicKeyRecord {
        AttestationPublicKeyRecord {
            key_id: key.key_id,
            public_key_sec1: key.public_key,
            status: match key.status {
                AttestationKeyStatus::Current => AttestationKeyStatusRecord::Current,
                AttestationKeyStatus::Previous => AttestationKeyStatusRecord::Previous,
            },
            valid_from: key.valid_from,
            valid_until: key.valid_until,
        }
    }

    #[must_use]
    pub fn record_to_dto(record: AttestationPublicKeyRecord) -> AttestationKey {
        AttestationKey {
            key_id: record.key_id,
            public_key: record.public_key_sec1,
            status: match record.status {
                AttestationKeyStatusRecord::Current => AttestationKeyStatus::Current,
                AttestationKeyStatusRecord::Previous => AttestationKeyStatus::Previous,
            },
            valid_from: record.valid_from,
            valid_until: record.valid_until,
        }
    }
}

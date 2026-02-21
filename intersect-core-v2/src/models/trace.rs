// enum RecordType {
//     UNKNOWN = 0;
//     ACCOUNT = 1;
//     FRAGMENT = 2;
//     INDEX = 3;
//     LINKS = 4;
// }

// message Trace {
//     RecordType type = 1;
//     // record key including default-encryption key (ensures that caching nodes never see plaintext)
//     veilid.RecordKey record = 2;
//     // intersect uses its own encryption on top of the default-encryption
//     // so that indexes can be referenced without neccessarily including their encryption key
//     Access access = 3;
// }

use veilid_core::RecordKey;

use crate::{
    models::{Access, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, SerialisableV1, SerialisationError, impl_v1_proto_conversions,
    },
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum RecordType {
    Account,
    Fragment,
    Index,
    Links,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Trace {
    record_type: RecordType,
    record: RecordKey,
    access: Access,
}

impl Trace {
    pub fn new(
        record_type: RecordType,
        record: &RecordKey,
        access: Access,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            record_type,
            record: record.clone(),
            access,
        })
    }

    pub fn record_type(&self) -> &RecordType {
        &self.record_type
    }

    pub fn record(&self) -> &RecordKey {
        &self.record
    }

    pub fn access(&self) -> &Access {
        &self.access
    }
}

impl SerialisableV1 for Trace {
    type Proto = proto::v1::intersect::Trace;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            r#type: match self.record_type {
                RecordType::Account => proto::v1::intersect::RecordType::Account as i32,
                RecordType::Fragment => proto::v1::intersect::RecordType::Fragment as i32,
                RecordType::Index => proto::v1::intersect::RecordType::Index as i32,
                RecordType::Links => proto::v1::intersect::RecordType::Links as i32,
            },
            record: Some((&self.record).try_into()?),
            access: Some(self.access.to_proto()?),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let record_type_proto = proto::v1::intersect::RecordType::try_from(proto.r#type)
            .map_err(|_| DeserialisationError::Failed("invalid record type".to_string()))?;

        let record_type = match record_type_proto {
            proto::v1::intersect::RecordType::Account => RecordType::Account,
            proto::v1::intersect::RecordType::Fragment => RecordType::Fragment,
            proto::v1::intersect::RecordType::Index => RecordType::Index,
            proto::v1::intersect::RecordType::Links => RecordType::Links,
            _ => Err(DeserialisationError::Failed(
                "invalid record type".to_string(),
            ))?,
        };
        let record = proto
            .record
            .ok_or(DeserialisationError::MissingField("record".to_owned()))?
            .into();
        let access = proto
            .access
            .ok_or(DeserialisationError::MissingField("access".to_owned()))?
            .try_into()?;
        Ok(Self::new(record_type, &record, access)?)
    }
}

impl_v1_proto_conversions! {Trace}

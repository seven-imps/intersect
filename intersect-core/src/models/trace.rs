use base58::{FromBase58, ToBase58};
use veilid_core::{RecordKey, SharedSecret};

use crate::{
    models::{Access, EncryptionError, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, Deserialise, SerialisableV1, SerialisationError, Serialise,
        impl_v1_proto_conversions,
    },
};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum DocumentType {
    Account,
    Fragment,
    Index,
    Links,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Trace {
    document_type: DocumentType,
    record: RecordKey,
    access: Access,
}

impl Trace {
    pub fn new(
        document_type: DocumentType,
        record: &RecordKey,
        access: Access,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            document_type,
            record: record.clone(),
            access,
        })
    }

    pub fn unlocked(
        document_type: DocumentType,
        record: &RecordKey,
        secret: &SharedSecret,
    ) -> Self {
        Self::new(document_type, record, Access::new_unlocked(secret)).unwrap()
    }

    pub fn locked(document_type: DocumentType, record: &RecordKey) -> Self {
        Self::new(document_type, record, Access::new_locked()).unwrap()
    }

    pub fn protected(
        document_type: DocumentType,
        record: &RecordKey,
        secret: &SharedSecret,
        password: &str,
    ) -> Result<Self, EncryptionError> {
        let access = Access::new_protected(secret, password)?;
        Ok(Self::new(document_type, record, access).unwrap())
    }

    pub fn document_type(&self) -> &DocumentType {
        &self.document_type
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
            document_type: match self.document_type {
                DocumentType::Account => proto::v1::intersect::DocumentType::Account as i32,
                DocumentType::Fragment => proto::v1::intersect::DocumentType::Fragment as i32,
                DocumentType::Index => proto::v1::intersect::DocumentType::Index as i32,
                DocumentType::Links => proto::v1::intersect::DocumentType::Links as i32,
            },
            record: Some((&self.record).try_into()?),
            access: Some(self.access.to_proto()?),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let document_type_proto = proto::v1::intersect::DocumentType::try_from(proto.document_type)
            .map_err(|_| DeserialisationError::Failed("invalid document type".to_string()))?;

        let document_type = match document_type_proto {
            proto::v1::intersect::DocumentType::Account => DocumentType::Account,
            proto::v1::intersect::DocumentType::Fragment => DocumentType::Fragment,
            proto::v1::intersect::DocumentType::Index => DocumentType::Index,
            proto::v1::intersect::DocumentType::Links => DocumentType::Links,
            _ => Err(DeserialisationError::Failed(
                "invalid document type".to_string(),
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
        Ok(Self::new(document_type, &record, access)?)
    }
}

impl_v1_proto_conversions! {Trace}

// string conversions

impl std::fmt::Display for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.serialise().map_err(|_| std::fmt::Error)?.to_base58()
        )
    }
}

impl std::str::FromStr for Trace {
    type Err = DeserialisationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s
            .from_base58()
            // .inspect_err(|e| match e {
            //     base58::FromBase58Error::InvalidBase58Character(c, _) => {
            //         println!("invalid char: {}", c)
            //     }
            //     base58::FromBase58Error::InvalidBase58Length => println!("invalid length"),
            // })
            .map_err(|_| DeserialisationError::Failed("invalid base58 encoding".to_string()))?;
        Self::deserialise(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn string_roundtrip() {
        // build trace
        let key = RecordKey::from_str(
            "VLD0:sX9L_EV3JAy5ozyK875WErKAyFhBy4jZ-6DZajlDr9c:KpS0JtGg9OfJhpsIVCFY8FI9arViozN3kw3duglNkmY",
        ).unwrap();
        let trace = Trace::new(DocumentType::Account, &key, Access::Locked).unwrap();

        // to string ...
        let trace_string = trace.to_string();
        // .. and back
        let deserialised_trace = Trace::from_str(&trace_string).unwrap();

        assert_eq!(trace, deserialised_trace);
    }
}

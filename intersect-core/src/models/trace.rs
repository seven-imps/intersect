use veilid_core::{RecordKey, SharedSecret};

use crate::{
    models::{Access, EncryptionError},
    proto,
    serialisation::{
        DeserialisationError, Deserialise, SerialisableV0, SerialisationError, Serialise,
        impl_string_conversions, impl_v0_proto_conversions,
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
    pub(crate) fn new(document_type: DocumentType, record: &RecordKey, access: Access) -> Self {
        Self {
            document_type,
            record: record.clone(),
            access,
        }
    }

    pub(crate) fn unlocked(
        document_type: DocumentType,
        record: &RecordKey,
        secret: &SharedSecret,
    ) -> Self {
        Self::new(document_type, record, Access::new_unlocked(secret))
    }

    pub(crate) fn locked(document_type: DocumentType, record: &RecordKey) -> Self {
        Self::new(document_type, record, Access::new_locked())
    }

    pub(crate) fn protected(
        document_type: DocumentType,
        record: &RecordKey,
        secret: &SharedSecret,
        password: &str,
    ) -> Result<Self, EncryptionError> {
        let access = Access::new_protected(secret, password)?;
        Ok(Self::new(document_type, record, access))
    }

    pub fn document_type(&self) -> &DocumentType {
        &self.document_type
    }

    pub(crate) fn record(&self) -> &RecordKey {
        &self.record
    }

    pub(crate) fn access(&self) -> &Access {
        &self.access
    }
}

impl SerialisableV0 for Trace {
    type Proto = proto::v0::intersect::Trace;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            document_type: match self.document_type {
                DocumentType::Account => proto::v0::intersect::DocumentType::Account as i32,
                DocumentType::Fragment => proto::v0::intersect::DocumentType::Fragment as i32,
                DocumentType::Index => proto::v0::intersect::DocumentType::Index as i32,
                DocumentType::Links => proto::v0::intersect::DocumentType::Links as i32,
            },
            record: Some((&self.record).try_into()?),
            access: Some(self.access.to_proto()?),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let document_type_proto = proto::v0::intersect::DocumentType::try_from(proto.document_type)
            .map_err(|_| DeserialisationError::Failed("invalid document type".to_string()))?;

        let document_type = match document_type_proto {
            proto::v0::intersect::DocumentType::Account => DocumentType::Account,
            proto::v0::intersect::DocumentType::Fragment => DocumentType::Fragment,
            proto::v0::intersect::DocumentType::Index => DocumentType::Index,
            proto::v0::intersect::DocumentType::Links => DocumentType::Links,
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
        Ok(Self::new(document_type, &record, access))
    }
}

impl_v0_proto_conversions! {Trace}
impl_string_conversions! {Trace}

/// opaque wrapper around a reference's symmetric encryption key.
/// used to unlock locked traces where the secret is shared out-of-band.
#[derive(Clone)]
pub struct TraceSecret(SharedSecret);

impl TraceSecret {
    pub(crate) fn new(secret: SharedSecret) -> Self {
        Self(secret)
    }

    pub(crate) fn inner(&self) -> &SharedSecret {
        &self.0
    }
}

impl SerialisableV0 for TraceSecret {
    type Proto = proto::v0::intersect::TraceSecret;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            secret: Some((&self.0).into()),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let secret = proto
            .secret
            .ok_or(DeserialisationError::MissingField("secret".to_owned()))?
            .into();
        Ok(Self(secret))
    }
}

impl_v0_proto_conversions! {TraceSecret}
impl_string_conversions! {TraceSecret}

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
        let trace = Trace::new(DocumentType::Account, &key, Access::Locked);

        // to string ...
        let trace_string = trace.to_string();
        // .. and back
        let deserialised_trace = Trace::from_str(&trace_string).unwrap();

        assert_eq!(trace, deserialised_trace);
    }
}

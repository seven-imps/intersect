use crate::{
    proto,
    serialisation::{DeserialisationError, Deserialise, Serialise},
};

#[derive(PartialEq, Debug, Clone)]
pub struct Fragment {
    pub data: Vec<u8>,
}

impl Fragment {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn from_str(text: &str) -> Self {
        Self::new(text.as_bytes().to_vec())
    }
}

impl From<&Fragment> for proto::intersect::v1::Fragment {
    fn from(value: &Fragment) -> Self {
        proto::intersect::v1::Fragment {
            data: Some(value.data.clone()),
        }
    }
}

impl Serialise for Fragment {
    fn serialise_v1_proto(&self) -> impl prost::Message {
        Into::<proto::intersect::v1::Fragment>::into(self)
    }
}

impl Deserialise for Fragment {
    fn deserialise_v1(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        let proto = Self::deserialise_proto::<proto::intersect::v1::Fragment>(bytes)?;

        Ok(Fragment {
            data: proto
                .data
                .ok_or(DeserialisationError::Failed("missing data".to_owned()))?,
        })
    }
}

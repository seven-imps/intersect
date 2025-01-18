use binrw::{binrw, helpers::until_eof};

#[binrw]
#[brw(big)]
#[derive(PartialEq, Debug, Clone)]
pub struct Fragment {
    #[br(parse_with = until_eof)]
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

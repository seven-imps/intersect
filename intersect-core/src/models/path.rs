use std::fmt;

use binrw::{binrw, NullString};
use lazy_regex::{lazy_regex, Lazy, Regex};
use thiserror::Error;

// use super::PathHash;

pub static MAX_SEGMENT_LENGTH: usize = 256;
// pub static MAX_PATH_DEPTH: usize = 16;
// pub static SEGMENT_REGEX: Lazy<Regex> = lazy_regex!("^[a-zA-Z0-9 ._-]+$");
pub static SEGMENT_REGEX: Lazy<Regex> =
    lazy_regex!(r"^( |\p{Alphabetic}|\d|\p{Pattern_Syntax}|\p{Emoji})+$");

#[binrw]
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Segment {
    #[brw(assert(Segment::is_valid(&segment)))]
    #[br(try_map = TryFrom::<NullString>::try_from)]
    #[bw(map = |x| NullString::from(x.clone()))]
    segment: String,
}

impl Segment {
    pub fn new<S: Into<String>>(segment: S) -> Result<Self, PathError> {
        let segment: String = segment.into();
        if !Self::is_valid(&segment) {
            return Err(PathError::InvalidSegment);
        }
        Ok(Segment { segment })
    }

    pub fn is_valid(segment: &str) -> bool {
        segment.len() < MAX_SEGMENT_LENGTH && SEGMENT_REGEX.is_match(segment)
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.segment)
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum PathError {
    #[error("invalid segment")]
    InvalidSegment,
    #[error("too many segments")]
    TooManySegments,
}

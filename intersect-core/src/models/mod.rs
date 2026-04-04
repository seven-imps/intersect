pub mod account;
pub use account::*;
pub mod encrypted;
pub use encrypted::*;
pub mod fragment;
pub use fragment::*;
pub mod trace;
pub use trace::*;
pub mod access;
pub use access::*;

use thiserror::Error;

// error to capture validation errors that can occur when creating or updating models
// these should be used for any constraint not represented in the type system or proto model
// e.g. max lengths for strings
#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ValidationError {
    #[error("field too long: {0}")]
    TooLong(String),
    #[error("invalid value: {0}")]
    Invalid(String),
}

mod account;
mod access;
mod encrypted;
mod fragment;
mod index;
mod trace;

// public types (re-exported from lib.rs)
pub use account::{AccountBio, AccountName, AccountPrivate, AccountPublicKey, AccountSecret};
pub use access::AccessError;
pub use encrypted::EncryptionError;
pub use fragment::{FragmentMime, FRAGMENT_SUBKEYS, MAX_CHUNK_BYTES, MAX_FRAGMENT_BYTES};
pub use index::IndexName;
pub use trace::{DocumentType, Trace, TraceSecret};

// crate-internal types
pub(crate) use account::AccountPublic;
pub(crate) use access::{Access, ProtectedSecret};
pub(crate) use encrypted::Encrypted;
pub(crate) use fragment::{FragmentContent, FragmentHeader};
pub(crate) use index::IndexHeader;

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

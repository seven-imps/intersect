use thiserror::Error;

use crate::{models::EncryptionError, record::NetworkError};

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum IntersectError {
    #[error("encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),
    #[error("network error: {0}")]
    NetworkError(#[from] NetworkError),
    #[error("unauthorized identity")]
    Unauthorized,
    #[error("tried opening locked or protected trace")]
    LockedTrace,
}

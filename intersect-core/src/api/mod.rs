mod document;
mod intersect;
mod reference;
mod trace;

// public types (re-exported from lib.rs)
pub use document::{Document, DocumentError, MutableDocument, OpenDocument};
pub use intersect::{Intersect, IntersectError};
pub use reference::TypedReference;
pub use trace::{LockedTypedReference, OpenedTrace, ProtectedTypedReference, WrongDocumentType};

// crate-internal types
pub(crate) use document::{LARGE_SUBKEYS, MANY_SUBKEYS};
pub(crate) use reference::Reference;

use veilid_core::KeyPair;

use crate::{
    api::{Document, DocumentError, LARGE_SUBKEYS, MutableDocument, OpenDocument, TypedReference},
    models::{DocumentType, Trace},
    veilid::RecordPool,
};

pub struct LinksDocument;

// TODO: a list of named traces pointing at other indexes
#[derive(PartialEq, Debug, Clone)]
pub struct LinksView {
    // TODO: this should also use a newtype for a length limited string
    pub links: Vec<(String, Trace)>,
}

impl std::fmt::Display for LinksView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(links not yet implemented)")
    }
}

impl Document for LinksDocument {
    const MAX_SUBKEYS: u16 = LARGE_SUBKEYS;
    const DOCUMENT_TYPE: DocumentType = DocumentType::Links;
    type View = LinksView;

    async fn read(
        _typed_ref: &TypedReference<LinksDocument>,
        _identity: Option<&KeyPair>,
        _force: bool,
        _pool: &RecordPool,
    ) -> Result<LinksView, DocumentError> {
        todo!("links document read not yet implemented")
    }

    async fn create(
        _view: LinksView,
        _identity: &KeyPair,
        _pool: &RecordPool,
    ) -> Result<TypedReference<LinksDocument>, DocumentError> {
        todo!("links document create not yet implemented")
    }
}

// TODO: add/remove/rename links
pub enum LinksUpdate {}

impl MutableDocument for LinksDocument {
    type Update = LinksUpdate;

    async fn update(
        _update: LinksUpdate,
        _doc: &OpenDocument<LinksDocument>,
        _identity: &KeyPair,
        _pool: &RecordPool,
    ) -> Result<(), DocumentError> {
        todo!("links document update not yet implemented")
    }
}

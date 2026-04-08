use veilid_core::KeyPair;

use crate::{
    api::{Document, DocumentError, LARGE_SUBKEYS, MutableDocument, OpenDocument, TypedReference},
    models::{DocumentType, Encrypted, IndexHeader, IndexName, Trace},
    veilid::RecordPool,
};

pub struct IndexDocument;

#[derive(PartialEq, Debug, Clone)]
pub struct IndexView {
    // user-readable name for the index, max 256 bytes
    name: IndexName,
    // author's account trace, unset for anonymous indexes
    author: Option<Trace>,
    // reference to the content fragment, if any
    fragment: Option<Trace>,
    // reference to the links record, if any
    links: Option<Trace>,
}

impl IndexView {
    pub fn new(
        name: IndexName,
        author: Option<Trace>,
        fragment: Option<Trace>,
        links: Option<Trace>,
    ) -> Self {
        Self {
            name,
            author,
            fragment,
            links,
        }
    }

    pub fn name(&self) -> &IndexName {
        &self.name
    }
    pub fn author(&self) -> Option<&Trace> {
        self.author.as_ref()
    }
    pub fn fragment(&self) -> Option<&Trace> {
        self.fragment.as_ref()
    }
    pub fn links(&self) -> Option<&Trace> {
        self.links.as_ref()
    }

    pub fn with_name(self, name: IndexName) -> Self {
        Self { name, ..self }
    }
    pub fn with_fragment(self, fragment: Option<Trace>) -> Self {
        Self { fragment, ..self }
    }
    pub fn with_links(self, links: Option<Trace>) -> Self {
        Self { links, ..self }
    }
}

impl std::fmt::Display for IndexView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# {}", self.name.as_ref())?;
        if let Some(author) = &self.author {
            writeln!(f, "author: {}", author)?;
        }
        if let Some(fragment) = &self.fragment {
            writeln!(f, "fragment: {}", fragment)?;
        }
        if let Some(links) = &self.links {
            writeln!(f, "links: {}", links)?;
        }
        Ok(())
    }
}

pub enum IndexUpdate {
    Name(IndexName),
    Fragment(Option<Trace>),
    Links(Option<Trace>),
}

impl Document for IndexDocument {
    const MAX_SUBKEYS: u16 = LARGE_SUBKEYS;
    const DOCUMENT_TYPE: DocumentType = DocumentType::Index;
    type View = IndexView;

    async fn read(
        typed_ref: &TypedReference<IndexDocument>,
        _identity: Option<&KeyPair>,
        force: bool,
        pool: &RecordPool,
    ) -> Result<IndexView, DocumentError> {
        let reference = typed_ref.reference();
        let header: IndexHeader = pool
            .read(reference, 0, force)
            .await?
            .decrypt(reference.secret())?;

        Ok(IndexView {
            name: header.name().clone(),
            author: header.author().cloned(),
            fragment: header.fragment().cloned(),
            links: header.links().cloned(),
        })
    }

    async fn create(
        view: IndexView,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<TypedReference<IndexDocument>, DocumentError> {
        let record = pool.create(identity, Self::MAX_SUBKEYS).await?;
        let reference = record.reference().clone();

        let header = IndexHeader::new(view.name, view.author, view.fragment, view.links);
        let encrypted = Encrypted::encrypt(&header, reference.secret())?;
        pool.write(&reference, 0, &encrypted, identity).await?;

        Ok(TypedReference::new(reference))
    }
}

impl MutableDocument for IndexDocument {
    type Update = IndexUpdate;

    async fn update(
        update: IndexUpdate,
        doc: &OpenDocument<Self>,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<(), DocumentError> {
        let view = doc.updates.borrow().clone()?;
        let reference = doc.reference.reference();

        let updated = match update {
            IndexUpdate::Name(name) => view.with_name(name),
            IndexUpdate::Fragment(fragment) => view.with_fragment(fragment),
            IndexUpdate::Links(links) => view.with_links(links),
        };
        let header = IndexHeader::new(
            updated.name,
            updated.author,
            updated.fragment,
            updated.links,
        );

        let encrypted = Encrypted::encrypt(&header, reference.secret())?;
        pool.write(reference, 0, &encrypted, identity).await?;

        Ok(())
    }
}

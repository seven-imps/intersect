use futures::future::{try_join, try_join_all};
use veilid_core::KeyPair;

use crate::{
    api::{Document, DocumentError, Reference, TypedReference},
    models::{
        DocumentType, Encrypted, FRAGMENT_SUBKEYS, FragmentContent, FragmentHeader, FragmentMime,
        MAX_CHUNK_BYTES, MAX_FRAGMENT_BYTES, ValidationError,
    },
    serialisation::{Deserialise, Serialise},
    veilid::{RecordError, RecordPool, with_crypto},
};

// subkeys available for chunk data in the primary record (subkey 0 is the header)
const MAX_PRIMARY_CHUNKS: usize = (FRAGMENT_SUBKEYS - 1) as usize;
// all subkeys in an overflow record are used for chunk data
const MAX_OVERFLOW_CHUNKS: usize = FRAGMENT_SUBKEYS as usize;

pub struct FragmentDocument;

#[derive(PartialEq, Debug, Clone)]
pub struct FragmentView {
    data: Vec<u8>,
    mime: FragmentMime,
}

impl FragmentView {
    pub fn new(data: Vec<u8>, mime: FragmentMime) -> Self {
        Self { data, mime }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn mime(&self) -> &FragmentMime {
        &self.mime
    }
}

impl Document for FragmentDocument {
    const MAX_SUBKEYS: u16 = FRAGMENT_SUBKEYS;
    const DOCUMENT_TYPE: DocumentType = DocumentType::Fragment;
    type View = FragmentView;

    async fn read(
        reference: &Reference,
        _identity: Option<&KeyPair>,
        _force: bool,
        pool: &RecordPool,
    ) -> Result<FragmentView, DocumentError> {
        // fragments are immutable, so force can safely be ignored since local cache can never go stale
        let header: FragmentHeader = pool
            .read(reference, 0, false)
            .await?
            .decrypt(reference.secret())?;

        let fragment_size = header.fragment_size() as usize;
        let total_chunks = fragment_size.div_ceil(MAX_CHUNK_BYTES);
        let num_primary = total_chunks.min(MAX_PRIMARY_CHUNKS);
        let num_overflow = total_chunks - num_primary;

        // validate that we have the expected amount of overflow records
        let num_overflow_records = num_overflow.div_ceil(MAX_OVERFLOW_CHUNKS);
        let keys = header.overflow_keys();
        if keys.len() != num_overflow_records {
            return Err(DocumentError::Corrupt(format!(
                "expected {} overflow key(s), got {}",
                num_overflow_records,
                keys.len(),
            )));
        }
        // and convert them into references
        let overflow_refs: Vec<Reference> = keys
            .iter()
            .map(|key| Reference::new(key.clone(), reference.secret().clone()))
            .collect();

        // precompute all the (reference, subkey) pairs we need to read from to assemble the full fragment
        let locations: Vec<(&Reference, u32)> = (1..=num_primary)
            .map(|i| (reference, i as u32))
            .chain((0..num_overflow).map(|i| {
                (
                    &overflow_refs[i / MAX_OVERFLOW_CHUNKS],
                    (i % MAX_OVERFLOW_CHUNKS) as u32,
                )
            }))
            .collect();

        // read everything in parallel
        let raw_chunks: Vec<Vec<u8>> = try_join_all(
            locations
                .iter()
                .map(|(r, subkey)| pool.read_raw(r, *subkey, false)),
        )
        .await?;

        // and assemble
        let mut assembled: Vec<u8> = raw_chunks.into_iter().flatten().collect();
        // making sure to trim any excess bytes from the last chunk
        assembled.truncate(fragment_size);

        // and if the hash matches...
        let valid = with_crypto(|c| c.validate_hash(&assembled, header.hash()))
            .map_err(|_| DocumentError::HashMismatch)?;
        if !valid {
            return Err(DocumentError::HashMismatch);
        }

        // ... then we can decrypt and return!
        let encrypted = Encrypted::deserialise(&assembled)?;
        let content: FragmentContent = encrypted.decrypt(reference.secret())?;

        Ok(FragmentView {
            data: content.into_data(),
            mime: header.mime().clone(),
        })
    }

    async fn create(
        view: FragmentView,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<TypedReference<FragmentDocument>, DocumentError> {
        let record = pool.create(identity, FRAGMENT_SUBKEYS).await?;
        let reference = record.reference().clone();

        // encrypt then serialize. the result is what will get hashed and chunked
        let content = FragmentContent::new(view.data);
        let data = Encrypted::encrypt(&content, reference.secret())?.serialise()?;

        if data.len() > MAX_FRAGMENT_BYTES {
            return Err(ValidationError::Invalid(format!(
                "fragment exceeds maximum size of {MAX_FRAGMENT_BYTES} bytes"
            ))
            .into());
        }
        let fragment_size = data.len() as u32;
        let hash = with_crypto(|c| c.generate_hash(&data));

        let chunks: Vec<&[u8]> = data.chunks(MAX_CHUNK_BYTES).collect();
        let (primary_chunks, overflow_chunks) =
            chunks.split_at(chunks.len().min(MAX_PRIMARY_CHUNKS));

        // construct futures for all the writes, both primary and overflow
        let write_primary = write_chunks(pool, identity, &reference, primary_chunks, 1);
        let write_overflow = try_join_all(
            overflow_chunks
                // split into groups of chunks that fit into a single overflow record
                .chunks(MAX_OVERFLOW_CHUNKS)
                // and then create each one and write all the subkeys for it
                .map(|group| async move {
                    let record = pool.create(identity, FRAGMENT_SUBKEYS).await?;
                    write_chunks(pool, identity, record.reference(), group, 0).await?;
                    Ok(record.key().clone())
                }),
        );
        // and then run them both concurrently since they don't depend on each other at all
        let (_, overflow_keys) = try_join(write_primary, write_overflow).await?;

        // finally, write the header after all other data has been written
        let header = FragmentHeader::new(hash, fragment_size, view.mime, overflow_keys)?;
        let header_encrypted = Encrypted::encrypt(&header, reference.secret())?;
        pool.write(&reference, 0, &header_encrypted, identity)
            .await?;

        // // wait for all records to flush to the network before returning
        // let secret = reference.secret().clone();
        // let all_refs = std::iter::once(reference.clone()).chain(
        //     header
        //         .overflow_keys()
        //         .iter()
        //         .map(|k| Reference::new(k.clone(), secret.clone())),
        // );
        // try_join_all(all_refs.map(|r| async move { pool.wait_for_sync(&r).await })).await?;

        Ok(TypedReference::new(reference))
    }
}

// helper for writing all subkeys of a record in parallel
async fn write_chunks(
    pool: &RecordPool,
    identity: &KeyPair,
    reference: &Reference,
    chunks: &[&[u8]],
    start: u32,
) -> Result<(), RecordError> {
    try_join_all(
        chunks
            .iter()
            .zip(start..)
            .map(|(chunk, subkey)| pool.write_raw(reference, subkey, chunk, identity)),
    )
    .await?;
    Ok(())
}

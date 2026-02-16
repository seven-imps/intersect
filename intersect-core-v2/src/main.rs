use intersect_core::{
    api::Intersect,
    debug, log,
    models::{account::AccountPublic, encrypted::Encrypted},
    serialisation::{Deserialise, Serialise},
};

#[tokio::main]
async fn main() {
    log!("starting...");

    let intersect = Intersect::init().await.unwrap();

    let keypair = intersect.connection.with_crypto(|c| c.generate_keypair());
    let name = "evelyn";
    let bio = "hi! <3";
    let account_public =
        AccountPublic::new(keypair.key(), Some(name.to_string()), Some(bio.to_string()));

    let serialised = account_public.serialise().unwrap();
    debug!(
        "serialised value ({} bytes): {}",
        serialised.len(),
        hex::encode_upper(&serialised)
    );
    debug!("as string: {}", String::from_utf8_lossy(&serialised));

    let (encrypted, _secret) =
        Encrypted::encrypt_with_random(&account_public, &intersect.connection).unwrap();
    let encrypted_serialised = encrypted.serialise().unwrap();
    debug!(
        "encrypted_serialised value ({} bytes): {}",
        encrypted_serialised.len(),
        hex::encode_upper(&encrypted_serialised)
    );
    debug!(
        "as string: {}",
        String::from_utf8_lossy(&encrypted_serialised)
    );

    let key = intersect.write(&serialised).await.unwrap();

    let value = intersect.read(key).await.unwrap();
    let read_account_public = AccountPublic::deserialise(&value).unwrap();
    log!("read value: {:?}", read_account_public);

    intersect.close().await;
}

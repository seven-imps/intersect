use base58::ToBase58;
use intersect_core::{
    api::Intersect,
    debug, log,
    models::{Access, RecordType, Trace, account::AccountPublic, encrypted::Encrypted},
    serialisation::{Deserialise, Serialise},
};
use veilid_core::{
    BarePublicKey, BareSecretKey, CRYPTO_KIND_VLD0_FOURCC, KeyPair, PublicKey, SecretKey,
    VLD0_PUBLIC_KEY_LENGTH, VLD0_SECRET_KEY_LENGTH,
};

#[tokio::main]
async fn main() {
    log!("starting...");

    let intersect = Intersect::init().await.unwrap();

    // write account

    let keypair = intersect.connection.with_crypto(|c| c.generate_keypair());
    let name = "evelyn";
    let bio = "hi! <3";
    let account_public =
        AccountPublic::new(keypair.key(), Some(name.to_string()), Some(bio.to_string())).unwrap();

    let serialised = account_public.serialise().unwrap();
    debug!(
        "serialised value ({} bytes): {}",
        serialised.len(),
        hex::encode_upper(&serialised)
    );
    debug!("as string: {}", String::from_utf8_lossy(&serialised));

    let (encrypted, secret) =
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

    let password = "hunter2 is my password";

    let access = Access::new_protected(&secret, password, &intersect.connection).unwrap();
    // let access = Access::new_unlocked(secret);
    // let access = Access::new_locked();

    // create trace

    let trace = Trace::new(RecordType::Account, &key, access).unwrap();
    let trace_serialised = trace.serialise().unwrap();
    debug!(
        "trace serialised value ({} bytes): {}",
        trace_serialised.len(),
        hex::encode_upper(&trace_serialised)
    );

    let base58 = trace_serialised.as_slice().to_base58();
    debug!("trace base58  ({} bytes): {}", base58.len(), base58);

    // unpack trace

    let deserialised_trace = Trace::deserialise(&trace_serialised).unwrap();
    let deserialised_access = deserialised_trace.access();

    let decrypted_secret = match deserialised_access {
        Access::Locked => todo!(),
        Access::Unlocked { secret } => todo!(),
        Access::Protected { protected_secret } => protected_secret
            .unlock(&password, &intersect.connection)
            .unwrap(),
    };

    debug!("decrypted secret: {:?}", decrypted_secret);

    let decrypted_value: AccountPublic = Encrypted::deserialise(&encrypted_serialised)
        .unwrap()
        .decrypt(&decrypted_secret, &intersect.connection)
        .unwrap();
    debug!("decrypted value: {:?}", decrypted_value);

    // read account

    let value = intersect.read(key).await.unwrap();
    let read_account_public = AccountPublic::deserialise(&value).unwrap();
    log!("read value: {:?}", read_account_public);

    intersect.close().await;
}

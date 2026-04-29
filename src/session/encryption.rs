//! Encrypted-password login helpers.
//!
//! Behind the optional `encryption` cargo feature. IG accepts an
//! RSA-encrypted password when `encryptedPassword=true` is set on the
//! login body. The encryption uses **PKCS#1 v1.5** padding with the
//! RSA public key returned by `GET /session/encryptionKey`.
//!
//! Workflow:
//!
//! ```no_run
//! # #[cfg(feature = "encryption")]
//! # async fn ex() -> trading_ig::Result<()> {
//! # let client: trading_ig::IgClient = todo!();
//! let key = client.session().encryption_key().await?;
//! let payload = trading_ig::session::encryption::encrypt_password(
//!     "my-real-password",
//!     &key.encryption_key,
//!     key.time_stamp,
//! )?;
//! // Pass `payload` as the `password` field of a login body with
//! // `encryptedPassword=true`.
//! # Ok(()) }
//! ```

use base64::Engine;
use rsa::pkcs8::DecodePublicKey;
use rsa::rand_core::OsRng;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

use crate::error::{Error, Result};

/// Encrypt the password using IG's published RSA public key.
///
/// `key_b64` is the base64 string returned in `EncryptionKey::encryption_key`
/// (DER-encoded SPKI). `time_stamp` is the server-supplied millisecond
/// timestamp; it is concatenated to the password before encryption.
pub fn encrypt_password(password: &str, key_b64: &str, time_stamp: i64) -> Result<String> {
    let der = base64::engine::general_purpose::STANDARD
        .decode(key_b64)
        .map_err(|e| Error::Auth(format!("invalid base64 in encryption key: {e}")))?;

    let public_key = RsaPublicKey::from_public_key_der(&der)
        .map_err(|e| Error::Auth(format!("invalid RSA public key: {e}")))?;

    let plaintext = format!("{password}|{time_stamp}");
    let ciphertext = public_key
        .encrypt(&mut OsRng, Pkcs1v15Encrypt, plaintext.as_bytes())
        .map_err(|e| Error::Auth(format!("RSA encryption failed: {e}")))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(ciphertext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::RsaPrivateKey;
    use rsa::pkcs8::EncodePublicKey;

    #[test]
    fn round_trip_decrypts_to_password_pipe_timestamp() {
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 1024).expect("generate key");
        let public_key = RsaPublicKey::from(&private_key);

        let der = public_key.to_public_key_der().expect("encode").into_vec();
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(&der);

        let encrypted = encrypt_password("hunter2", &key_b64, 1_700_000_000_000).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&encrypted)
            .unwrap();
        let plain = private_key
            .decrypt(rsa::Pkcs1v15Encrypt, &bytes)
            .expect("decrypt");
        assert_eq!(
            std::str::from_utf8(&plain).unwrap(),
            "hunter2|1700000000000"
        );
    }
}

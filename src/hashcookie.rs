use crate::{admin::users::User, debug_println};
use base64::{engine::general_purpose, Engine as _};
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20Legacy;

use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

#[derive(Debug)]
pub struct HashCookieToken {
    pub exp: i64,
    pub username: String,
    pub dbauth: Option<String>,
}

#[derive(Debug)]
pub struct HashCookieTokenPayload {
    pub exp: i64,
    pub username: String,
    pub payload: Vec<u8>,
    pub nonce: Option<Vec<u8>>,
}

impl HashCookieTokenPayload {
    pub fn new(token: String) -> Result<HashCookieTokenPayload, Box<dyn std::error::Error>> {
        // exp.b64(username).b64(payload).b64(signature)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() < 3 {
            return Err("Invalid token".into());
        }
        let exp = match parts[0].parse::<i64>() {
            Ok(exp) => exp,
            Err(e) => {
                debug_println!("Error parsing exp: {}", e);
                return Err("Invalid token".into());
            }
        };
        let username = String::from_utf8(general_purpose::URL_SAFE.decode(parts[1].as_bytes())?)?;
        let payload = general_purpose::URL_SAFE
            .decode(parts[2].as_bytes())?
            .to_vec();
        if parts.len() == 3 {
            return Ok(HashCookieTokenPayload {
                exp,
                username,
                payload,
                nonce: None,
            });
        }
        let nonce = general_purpose::URL_SAFE
            .decode(parts[3].as_bytes())?
            .to_vec();
        Ok(HashCookieTokenPayload {
            exp,
            username,
            payload,
            nonce: Some(nonce),
        })
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    pub fn try_get_json_payload(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let payload = String::from_utf8(self.payload.clone())?;
        let json_payload: serde_json::Value = serde_json::from_str(&payload)?;
        Ok(json_payload)
    }
}

const IV_SIZE: usize = 16;
const KEY_SIZE: usize = 32;

impl HashCookieToken {
    pub fn new_from_token(
        token: String,
        user: User,
    ) -> Result<HashCookieToken, Box<dyn std::error::Error>> {
        let mut parts = HashCookieTokenPayload::new(token)?;

        if parts.username != user.username {
            return Err("Encrypted username does not match user".into());
        }

        if parts.exp < chrono::Utc::now().timestamp() {
            return Err("Token has expired".into());
        }

        let nonce = match parts.nonce {
            Some(nonce) => nonce,
            None => {
                return Err("No nonce found".into());
            }
        };

        // load jwt_secret from user hex encoded
        let jwt_secret = hex::decode(user.jwt_secret)?;
        let salt = hex::decode(user.salt)?;

        let mut key_bytes: [u8; (IV_SIZE + KEY_SIZE)] = [0u8; (IV_SIZE + KEY_SIZE)];
        pbkdf2_hmac::<Sha256>(&jwt_secret, &salt, 10000, &mut key_bytes);

        let keyslice: [u8; KEY_SIZE] = key_bytes[IV_SIZE..].try_into().unwrap();
        let nonceslice: [u8; 8] = nonce.try_into().unwrap();
        let mut cipher = ChaCha20Legacy::new(&keyslice.into(), &nonceslice.into());

        cipher.apply_keystream(parts.payload.as_mut_slice());
        let decoded_token_payload = String::from_utf8(parts.payload.to_vec())?;
        let decoded_payload = HashCookieTokenPayload::new(decoded_token_payload)?;

        let json_payload = decoded_payload.try_get_json_payload()?;
        debug_println!("hashcookie decrypted json_payload: {:?}", json_payload);

        if decoded_payload.exp != parts.exp {
            return Err("Encrypted exp does not match payload".into());
        }

        if decoded_payload.username != parts.username {
            return Err("Encrypted username does not match payload".into());
        }

        Ok(HashCookieToken {
            exp: decoded_payload.exp,
            username: decoded_payload.username,
            dbauth: json_payload["dbauth"].as_str().map(|s| s.to_string()),
        })
    }
}

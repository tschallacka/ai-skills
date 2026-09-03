// MODE: DEV
// PACKAGE: PROD
//! Versioned challenge/proof primitives for authenticated TCP sessions.

use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::io;

type HmacSha256 = Hmac<Sha256>;

pub const AUTH_VERSION: &str = "ai-text-editor-auth-v1";
pub const NONCE_BYTES: usize = 32;

/// Generate an unpredictable, URL-safe challenge nonce.
pub fn nonce() -> io::Result<String> {
    let mut bytes = [0u8; NONCE_BYTES];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// Build the byte-exact transcript signed by both sides.
///
/// Every field is length-prefixed as an unsigned big-endian u64. This prevents
/// concatenation ambiguity while keeping the wire representation independent
/// of JSON member ordering or whitespace.
pub fn transcript(nonce: &[u8], request_id: &str, generation: &str) -> Vec<u8> {
    let fields = [
        AUTH_VERSION.as_bytes(),
        nonce,
        request_id.as_bytes(),
        generation.as_bytes(),
    ];
    let mut bytes = Vec::with_capacity(fields.iter().map(|field| 8 + field.len()).sum());
    for field in fields {
        bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    bytes
}

/// Return the base64 HMAC proof for a challenge and request identity.
pub fn proof(
    secret: &[u8],
    nonce: &[u8],
    request_id: &str,
    generation: &str,
) -> io::Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "empty authentication secret"))?;
    mac.update(&transcript(nonce, request_id, generation));
    Ok(base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

pub fn verify(
    secret: &[u8],
    nonce: &[u8],
    request_id: &str,
    generation: &str,
    encoded_proof: &str,
) -> bool {
    let Ok(provided) = base64::engine::general_purpose::STANDARD.decode(encoded_proof) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(&transcript(nonce, request_id, generation));
    mac.verify_slice(&provided).is_ok()
}

pub fn decode_nonce(encoded: &str) -> io::Result<Vec<u8>> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(io::Error::other)?;
    if bytes.len() != NONCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "authentication nonce must be exactly 32 bytes",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_is_length_prefixed_and_stable() {
        let bytes = transcript(b"nonce", "request", "generation");
        assert_eq!(&bytes[..8], &(AUTH_VERSION.len() as u64).to_be_bytes());
        assert!(bytes
            .windows(AUTH_VERSION.len())
            .any(|part| part == AUTH_VERSION.as_bytes()));
        assert_eq!(
            bytes.len(),
            (8 + AUTH_VERSION.len()) + (8 + 5) + (8 + 7) + (8 + 10)
        );
    }

    #[test]
    fn proof_verifies_only_for_the_exact_transcript() {
        let secret = b"secret";
        let encoded = proof(secret, b"nonce", "request", "generation").unwrap();
        assert!(verify(secret, b"nonce", "request", "generation", &encoded));
        assert!(!verify(secret, b"nonce", "other", "generation", &encoded));
        assert!(!verify(
            b"other",
            b"nonce",
            "request",
            "generation",
            &encoded
        ));
    }

    #[test]
    fn nonce_has_expected_entropy_and_decodes() {
        let encoded = nonce().unwrap();
        assert_eq!(decode_nonce(&encoded).unwrap().len(), NONCE_BYTES);
        assert!(decode_nonce("bad").is_err());
    }
}

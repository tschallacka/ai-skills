// MODE: DEV
// PACKAGE: PROD
//! Challenge/proof primitives for the control connection's loopback-TCP arm
//! (T111) -- the shape `ai_text_editor::auth` already uses for its own TCP
//! fallback, scoped down to `control.rs`'s much smaller wire protocol (a flat
//! `Request`/`Reply` pair, with no envelope, request id, or generation field
//! of its own). Not shared as a dependency on `ai-text-editor`: these are
//! separate skills with separate release lifecycles, so this is a small
//! sibling, not a cross-skill import.
//!
//! `OwnerRecord.started_at_ns` (already unique per owner instance) stands in
//! for `ai_text_editor::auth`'s random `server_generation` string: both exist
//! to bind a proof to one specific server/owner process rather than letting a
//! captured proof outlive a restart that happened to keep the same secret.
//
// On a unix build these are reached only from tests -- the real caller,
// `control.rs`'s TCP transport, is itself reachable only from a non-unix
// build's `mod imp`. See control.rs's own matching attribute for why that is
// exactly the point rather than something to route around.
#![cfg_attr(unix, allow(dead_code))]

use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::io;

type HmacSha256 = Hmac<Sha256>;

const AUTH_VERSION: &str = "chat-client-rs-control-auth-v1";
pub const NONCE_BYTES: usize = 32;

/// Generate an unpredictable, URL-safe challenge nonce.
pub fn nonce() -> io::Result<String> {
    let mut bytes = [0u8; NONCE_BYTES];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

pub fn decode_nonce(encoded: &str) -> io::Result<Vec<u8>> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(io::Error::other)?;
    if bytes.len() != NONCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "control auth nonce must be exactly 32 bytes",
        ));
    }
    Ok(bytes)
}

/// The byte-exact transcript signed by both sides. Every field is
/// length-prefixed as an unsigned big-endian u64, so concatenation cannot be
/// ambiguous.
fn transcript(nonce: &[u8], started_at_ns: u64) -> Vec<u8> {
    let generation = started_at_ns.to_be_bytes();
    let fields: [&[u8]; 3] = [AUTH_VERSION.as_bytes(), nonce, &generation];
    let mut bytes = Vec::with_capacity(fields.iter().map(|field| 8 + field.len()).sum());
    for field in fields {
        bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    bytes
}

/// Return the base64 HMAC proof for a challenge and this owner's identity.
pub fn proof(secret: &[u8], nonce: &[u8], started_at_ns: u64) -> io::Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "empty control auth secret"))?;
    mac.update(&transcript(nonce, started_at_ns));
    Ok(base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

pub fn verify(secret: &[u8], nonce: &[u8], started_at_ns: u64, encoded_proof: &str) -> bool {
    let Ok(provided) = base64::engine::general_purpose::STANDARD.decode(encoded_proof) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(&transcript(nonce, started_at_ns));
    mac.verify_slice(&provided).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_is_length_prefixed_and_stable() {
        let bytes = transcript(b"01234567890123456789012345678901", 7);
        assert_eq!(&bytes[..8], &(AUTH_VERSION.len() as u64).to_be_bytes());
        assert!(bytes
            .windows(AUTH_VERSION.len())
            .any(|part| part == AUTH_VERSION.as_bytes()));
    }

    #[test]
    fn proof_verifies_only_for_the_exact_transcript() {
        let secret = b"secret";
        let nonce = b"01234567890123456789012345678901";
        let encoded = proof(secret, nonce, 42).unwrap();
        assert!(verify(secret, nonce, 42, &encoded));
        assert!(
            !verify(secret, nonce, 43, &encoded),
            "a different owner generation must not verify"
        );
        assert!(
            !verify(b"other", nonce, 42, &encoded),
            "a different secret must not verify"
        );
        assert!(
            !verify(secret, b"11111111111111111111111111111111", 42, &encoded),
            "a different nonce must not verify"
        );
    }

    #[test]
    fn a_malformed_proof_does_not_verify() {
        assert!(!verify(b"secret", b"nonce", 1, "not-base64!!"));
    }

    #[test]
    fn nonce_has_expected_entropy_and_decodes() {
        let encoded = nonce().unwrap();
        assert_eq!(decode_nonce(&encoded).unwrap().len(), NONCE_BYTES);
        assert!(decode_nonce("bad").is_err());
    }

    #[test]
    fn two_nonces_are_not_the_same() {
        assert_ne!(nonce().unwrap(), nonce().unwrap());
    }
}

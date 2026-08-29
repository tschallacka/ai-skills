// MODE: DEV
// PACKAGE: PROD
//! SHA-256 (FIPS 180-4), streaming, no dependencies.
//!
//! The output must stay byte-identical to `sha256sum` forever: fix keys minted
//! by the shell chain this replaces are recorded in plan directories and have
//! to keep verifying. Tests pin the NIST vectors and the padding boundary.

/// FIPS 180-4 section 4.2.2: the first 32 bits of the fractional parts of the
/// cube roots of the first 64 primes.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// FIPS 180-4 section 5.3.3: the fractional parts of the square roots of the
/// first eight primes.
const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

pub struct Sha256 {
    state: [u32; 8],
    /// Bytes not yet consumed by a compression round; always < 64 between calls.
    buffer: [u8; 64],
    buffered: usize,
    /// Total message length in bytes. u64 counts to 2^64-1 bytes, far past the
    /// 2^61-1 the 64-bit bit-length field can express, so it cannot silently
    /// wrap on any input a process can actually feed it.
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            state: H0,
            buffer: [0u8; 64],
            buffered: 0,
            length: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.length = self
            .length
            .checked_add(data.len() as u64)
            .expect("message longer than 2^64 bytes");
        // Top up a partial block first, then run whole blocks straight out of
        // the caller's slice so a large read is not copied twice.
        if self.buffered > 0 {
            let want = 64 - self.buffered;
            let take = want.min(data.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while data.len() >= 64 {
            let (block, rest) = data.split_at(64);
            let mut fixed = [0u8; 64];
            fixed.copy_from_slice(block);
            self.compress(&fixed);
            data = rest;
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffered = data.len();
        }
    }

    /// FIPS 180-4 section 5.1.1 padding: a 0x80 byte, then zeros until eight
    /// bytes remain in the block, then the big-endian bit length. A message
    /// whose tail is 56..=63 bytes needs a second padding block; that is the
    /// boundary the 55/56/64-byte tests exist to pin.
    pub fn finish(mut self) -> [u8; 32] {
        let bit_length = self.length.wrapping_mul(8);
        let mut tail = [0u8; 72];
        tail[0] = 0x80;
        let pad_zeros = (55 - (self.length % 64) as i64).rem_euclid(64) as usize;
        let end = 1 + pad_zeros;
        tail[end..end + 8].copy_from_slice(&bit_length.to_be_bytes());
        let total = end + 8;
        // Zeroed first so update()'s own length accounting cannot overflow on a
        // message near u64::MAX; the bit length is already captured above.
        self.length = 0;
        self.update(&tail[..total]);
        debug_assert_eq!(self.buffered, 0, "padding must land on a block boundary");
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(input: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(input);
        hex(&h.finish())
    }

    /// NIST FIPS 180-2 / CAVS published vectors.
    #[test]
    fn nist_vectors() {
        assert_eq!(
            digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            digest(b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }

    /// The one-million-'a' vector: exercises many blocks and a length whose
    /// bit count exceeds 32 bits.
    #[test]
    fn million_a() {
        let mut h = Sha256::new();
        let chunk = vec![b'a'; 1000];
        for _ in 0..1000 {
            h.update(&chunk);
        }
        assert_eq!(
            hex(&h.finish()),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// The padding boundary. 55 fits its length field in the same block, 56
    /// does not and forces a second block, 64 is an exact block plus a whole
    /// padding block. Values cross-checked against sha256sum.
    #[test]
    fn padding_boundary() {
        let a55 = "a".repeat(55);
        let a56 = "a".repeat(56);
        let a63 = "a".repeat(63);
        let a64 = "a".repeat(64);
        let a65 = "a".repeat(65);
        assert_eq!(
            digest(a55.as_bytes()),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            digest(a56.as_bytes()),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            digest(a63.as_bytes()),
            "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34"
        );
        assert_eq!(
            digest(a64.as_bytes()),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
        assert_eq!(
            digest(a65.as_bytes()),
            "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0"
        );
    }

    /// Streaming must not change the answer: a message fed one byte at a time,
    /// and one fed in blocks that straddle the 64-byte boundary, must agree
    /// with the single-call digest. This is what catches a buffering bug that
    /// the one-shot vectors above cannot see.
    #[test]
    fn chunking_is_invisible() {
        let message: Vec<u8> = (0u16..300).map(|i| (i % 251) as u8).collect();
        let one_shot = digest(&message);
        let mut byte_at_a_time = Sha256::new();
        for byte in &message {
            byte_at_a_time.update(&[*byte]);
        }
        assert_eq!(hex(&byte_at_a_time.finish()), one_shot);
        for split in [1usize, 63, 64, 65, 127, 128, 129, 299] {
            let mut h = Sha256::new();
            h.update(&message[..split]);
            h.update(&message[split..]);
            assert_eq!(hex(&h.finish()), one_shot, "split at {split}");
        }
    }

    /// Every byte value, so a sign-extension or char-vs-byte bug shows up.
    #[test]
    fn all_byte_values() {
        let message: Vec<u8> = (0..=255u8).collect();
        assert_eq!(
            digest(&message),
            "40aff2e9d2d8922e47afd4648e6967497158785fbd1da870e7110266bf944880"
        );
    }
}

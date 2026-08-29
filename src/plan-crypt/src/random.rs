// MODE: DEV
// PACKAGE: PROD
//! Cryptographically secure random bytes, straight from the operating system.
//!
//! No `getrandom`, no `rand`: both platforms expose a CSPRNG the standard
//! library can reach with what is already in `std` plus one raw declaration.
//! There is no fallback and there must not be one — the value this produces
//! keys the fix-key gate, and a guessable "random" id is exactly the defect
//! this binary exists to remove.

use std::io;

/// Fill `out` with bytes from the OS CSPRNG, or fail. Never returns partially
/// filled output: a short read is an error, not a quietly weaker key.
pub fn fill(out: &mut [u8]) -> io::Result<()> {
    imp::fill(out)
}

#[cfg(unix)]
mod imp {
    use std::fs::File;
    use std::io::{self, Read};

    /// `/dev/urandom` rather than `getrandom(2)`: it is the one interface every
    /// unix in the target set (Linux, macOS, the BSDs) spells the same way, it
    /// needs no libc declaration, and after boot it is the same pool.
    ///
    /// `read_exact` is the load-bearing call. A plain `read` on `/dev/urandom`
    /// may legally return fewer bytes than asked for, and silently hexing a
    /// short buffer would emit an id with zeros where entropy should be.
    pub fn fill(out: &mut [u8]) -> io::Result<()> {
        let mut source = File::open("/dev/urandom")?;
        source.read_exact(out)
    }
}

#[cfg(windows)]
mod imp {
    use std::io;

    // BCryptGenRandom from bcrypt.dll. BCRYPT_USE_SYSTEM_PREFERRED_RNG lets the
    // algorithm handle be null, so no provider has to be opened or closed.
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;

    #[link(name = "bcrypt")]
    extern "system" {
        fn BCryptGenRandom(
            h_algorithm: *mut core::ffi::c_void,
            pb_buffer: *mut u8,
            cb_buffer: u32,
            dw_flags: u32,
        ) -> i32;
    }

    pub fn fill(out: &mut [u8]) -> io::Result<()> {
        // The API takes a 32-bit length, so a larger request is chunked rather
        // than truncated by the cast.
        for chunk in out.chunks_mut(u32::MAX as usize) {
            // SAFETY: the pointer and length describe `chunk` exactly, the
            // buffer outlives the call, and a null algorithm handle is what
            // BCRYPT_USE_SYSTEM_PREFERRED_RNG requires.
            let status = unsafe {
                BCryptGenRandom(
                    core::ptr::null_mut(),
                    chunk.as_mut_ptr(),
                    chunk.len() as u32,
                    BCRYPT_USE_SYSTEM_PREFERRED_RNG,
                )
            };
            if status != 0 {
                return Err(io::Error::other(format!(
                    "BCryptGenRandom failed with NTSTATUS 0x{status:08x}"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::fill;

    /// Two draws of 32 bytes must differ, and neither may be all zeros. A stub
    /// that returned a zeroed buffer would satisfy a "the call succeeded" test
    /// and fail both of these.
    #[test]
    fn draws_differ_and_are_not_zero() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        fill(&mut a).expect("OS CSPRNG unavailable");
        fill(&mut b).expect("OS CSPRNG unavailable");
        assert_ne!(a, b, "two draws produced identical bytes");
        assert_ne!(a, [0u8; 32], "draw was all zeros");
    }

    /// Every requested length is filled exactly, including one that is not a
    /// multiple of any internal block size.
    #[test]
    fn odd_lengths_are_filled() {
        for len in [1usize, 7, 8, 33, 4096] {
            let mut buf = vec![0u8; len];
            fill(&mut buf).expect("OS CSPRNG unavailable");
            assert_eq!(buf.len(), len);
            assert!(buf.iter().any(|b| *b != 0), "all-zero draw of {len} bytes");
        }
    }

    /// A zero-length request is legal and touches nothing.
    #[test]
    fn empty_request_succeeds() {
        let mut buf: [u8; 0] = [];
        fill(&mut buf).expect("zero-length draw must succeed");
    }
}

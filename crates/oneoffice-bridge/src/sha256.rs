//! Pure safe Rust FIPS 180-4 SHA-256 implementation for deterministic grid digests.
//!
//! Implemented directly without unsafe code or external dependencies so `oneoffice-bridge`
//! produces reproducible 256-bit hex digests across Native and `wasm32-unknown-unknown`.

use std::fmt::Write as _;

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

const INITIAL_STATE: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Streaming FIPS 180-4 SHA-256 hasher.
#[derive(Clone, Debug)]
pub struct Sha256Hasher {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_bytes: u64,
}

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256Hasher {
    /// Creates a new SHA-256 hasher initialized to the FIPS 180-4 IV.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            buf: [0u8; 64],
            buf_len: 0,
            total_bytes: 0,
        }
    }

    /// Absorbs `data` into the hash state.
    pub fn update(&mut self, mut data: &[u8]) {
        self.total_bytes = self.total_bytes.saturating_add(data.len() as u64);

        if self.buf_len > 0 {
            let need = 64usize.saturating_sub(self.buf_len);
            let take = data.len().min(need);
            if let (Some(dst), Some(src)) = (
                self.buf.get_mut(self.buf_len..self.buf_len + take),
                data.get(..take),
            ) {
                dst.copy_from_slice(src);
            }
            self.buf_len += take;
            data = data.get(take..).unwrap_or(&[]);
            if self.buf_len == 64 {
                let block = self.buf;
                compress_block(&mut self.state, &block);
                self.buf_len = 0;
            }
        }

        while data.len() >= 64 {
            let mut block = [0u8; 64];
            if let Some(src) = data.get(..64) {
                block.copy_from_slice(src);
            }
            compress_block(&mut self.state, &block);
            data = data.get(64..).unwrap_or(&[]);
        }

        if !data.is_empty() {
            if let Some(dst) = self.buf.get_mut(..data.len()) {
                dst.copy_from_slice(data);
            }
            self.buf_len = data.len();
        }
    }

    /// Finalizes padding and returns the 32-byte SHA-256 digest.
    #[must_use]
    pub fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_bytes.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buf_len != 56 {
            self.update(&[0x00]);
        }
        self.update(&bit_len.to_be_bytes());

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            let bytes = word.to_be_bytes();
            if let Some(slot) = out.get_mut(i * 4..i * 4 + 4) {
                slot.copy_from_slice(&bytes);
            }
        }
        out
    }

    /// Finalizes padding and returns the lowercase 64-character hexadecimal SHA-256 string.
    #[must_use]
    pub fn finalize_hex(self) -> String {
        let digest = self.finalize();
        let mut s = String::with_capacity(64);
        for b in digest {
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

fn compress_block(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        let b0 = block.get(i * 4).copied().unwrap_or(0);
        let b1 = block.get(i * 4 + 1).copied().unwrap_or(0);
        let b2 = block.get(i * 4 + 2).copied().unwrap_or(0);
        let b3 = block.get(i * 4 + 3).copied().unwrap_or(0);
        if let Some(slot) = w.get_mut(i) {
            *slot = u32::from_be_bytes([b0, b1, b2, b3]);
        }
    }
    for i in 16..64 {
        let w15 = w.get(i - 15).copied().unwrap_or(0);
        let w2 = w.get(i - 2).copied().unwrap_or(0);
        let w16 = w.get(i - 16).copied().unwrap_or(0);
        let w7 = w.get(i - 7).copied().unwrap_or(0);
        let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
        let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
        if let Some(slot) = w.get_mut(i) {
            *slot = w16.wrapping_add(s0).wrapping_add(w7).wrapping_add(s1);
        }
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for i in 0..64 {
        let ki = K.get(i).copied().unwrap_or(0);
        let wi = w.get(i).copied().unwrap_or(0);
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(ki).wrapping_add(wi);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

/// Computes the 32-byte FIPS 180-4 SHA-256 digest of `data`.
#[must_use]
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256Hasher::new();
    h.update(data);
    h.finalize()
}

/// Computes the lowercase 64-character hexadecimal FIPS 180-4 SHA-256 digest of `data`.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256Hasher::new();
    h.update(data);
    h.finalize_hex()
}

/// Computes a collision-free deterministic SHA-256 hex digest over a 2D cell display grid.
#[must_use]
pub fn hash_display_grid(display_cells: &[Vec<String>]) -> String {
    let mut h = Sha256Hasher::new();
    h.update(&(display_cells.len() as u64).to_be_bytes());
    for row in display_cells {
        h.update(&(row.len() as u64).to_be_bytes());
        for cell in row {
            h.update(&(cell.len() as u64).to_be_bytes());
            h.update(cell.as_bytes());
        }
    }
    h.finalize_hex()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nist_fips_180_4_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn display_grid_framing_is_collision_free() {
        let g1 = vec![vec!["a".to_string(), "bc".to_string()]];
        let g2 = vec![vec!["ab".to_string(), "c".to_string()]];
        assert_ne!(hash_display_grid(&g1), hash_display_grid(&g2));
        assert_eq!(hash_display_grid(&g1), hash_display_grid(&g1));
    }
}

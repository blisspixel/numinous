//! SHA-256 for record identity.
//!
//! Core stays dependency-free, so this is the compact FIPS 180-4 function
//! rather than a crate. It identifies bytes. It is not a password hash and
//! it does not authenticate a peer. NIST vectors in the tests lock the
//! output to the same digest the faces already computed with their own
//! SHA-256.

const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const ROUND: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// SHA-256 of `bytes`.
#[must_use]
pub(crate) fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut state = INITIAL;
    let mut offset = 0;
    while offset + 64 <= bytes.len() {
        let mut block = [0_u8; 64];
        block.copy_from_slice(&bytes[offset..offset + 64]);
        compress(&mut state, &block);
        offset += 64;
    }
    let mut tail = [0_u8; 64];
    let remaining = bytes.len() - offset;
    tail[..remaining].copy_from_slice(&bytes[offset..]);
    tail[remaining] = 0x80;
    // FIPS 180-4 stores the message length in bits, modulo 2^64.
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    if remaining >= 56 {
        compress(&mut state, &tail);
        tail = [0_u8; 64];
    }
    tail[56..].copy_from_slice(&bit_len.to_be_bytes());
    compress(&mut state, &tail);
    let mut out = [0_u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Lowercase hex, two characters per byte.
#[must_use]
pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    out
}

fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut words = [0_u32; 64];
    for (word, chunk) in words.iter_mut().zip(block.chunks_exact(4)) {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(chunk);
        *word = u32::from_be_bytes(bytes);
    }
    for index in 16..64 {
        let small = words[index - 15];
        let large = words[index - 2];
        let low = small.rotate_right(7) ^ small.rotate_right(18) ^ (small >> 3);
        let high = large.rotate_right(17) ^ large.rotate_right(19) ^ (large >> 10);
        words[index] = words[index - 16]
            .wrapping_add(low)
            .wrapping_add(words[index - 7])
            .wrapping_add(high);
    }
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];
    for index in 0..64 {
        let upper = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ (!e & g);
        let first = h
            .wrapping_add(upper)
            .wrapping_add(choose)
            .wrapping_add(ROUND[index])
            .wrapping_add(words[index]);
        let lower = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let second = lower.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(first);
        d = c;
        c = b;
        b = a;
        a = first.wrapping_add(second);
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

#[cfg(test)]
mod tests {
    use super::digest;

    fn expect(input: &[u8], hex_digest: &str) {
        let actual = super::hex(&digest(input));
        assert_eq!(actual, hex_digest);
    }

    #[test]
    fn nist_short_and_two_block_vectors() {
        expect(
            b"",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
        expect(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        );
        expect(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        );
    }

    #[test]
    fn nist_million_a_characters() {
        expect(
            &vec![b'a'; 1_000_000],
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
        );
    }
}

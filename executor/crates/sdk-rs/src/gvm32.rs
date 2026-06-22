//! # GenVM base32 encoder/decoder
//!
//! Uses [Crockford's Base32](https://www.crockford.com/base32.html): the
//! alphabet `0123456789abcdefghjkmnpqrstvwxyz` (excludes `i`, `l`, `o`, `u`),
//! no padding, big-endian bit packing. Encoding is lowercase by default;
//! decoding is case-insensitive, treats `i`/`l` as `1` and `o` as `0`, and
//! ignores `-`. Used for encoding hashes.

const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Encodes a byte slice as a Crockford Base32 string (uppercase, no padding).
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let mut value: u32 = 0;
    let mut bits: u32 = 0;
    for &b in bytes {
        value = (value << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((value >> bits) & 0x1f) as usize] as char);
        }
        value &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(ALPHABET[((value << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

fn decode_char(c: u8) -> Option<u8> {
    let c = match c.to_ascii_lowercase() {
        b'o' => b'0',
        b'i' | b'l' => b'1',
        other => other,
    };
    ALPHABET.iter().position(|&b| b == c).map(|p| p as u8)
}

/// Decodes a Crockford Base32 string. Case-insensitive; `i`/`l` are read as `1`,
/// `o` as `0` and `-` is ignored. Returns `None` on an invalid character or
/// non-zero trailing padding bits.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let mut value: u32 = 0;
    let mut bits: u32 = 0;
    for &c in s.as_bytes() {
        if c == b'-' {
            continue;
        }
        value = (value << 5) | decode_char(c)? as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((value >> bits) as u8);
            value &= (1 << bits) - 1;
        }
    }
    if value != 0 {
        return None;
    }
    Some(out)
}

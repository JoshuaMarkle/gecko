//! Stable content hashes (FNV-1a 64). Not cryptographic; used to identify inputs and dedupe layouts.

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

pub fn hex(h: u64) -> String {
    format!("{h:016x}")
}

/// Short 8-hex-digit form for ids and directory names.
pub fn short(h: u64) -> String {
    format!("{:08x}", h >> 32)
}

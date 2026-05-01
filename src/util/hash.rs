//! Fast hash map utilities
//!
//! Uses rustc-hash (FxHash) for faster hash maps when DoS resistance
//! is not needed (internal driver data structures).

pub type FxHashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type FxHashSet<K> = rustc_hash::FxHashSet<K>;

/// FNV-1a hash function for quick hashing
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// FNV-1a hash for u32 slices
pub fn fnv1a_hash_u32(data: &[u32]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &word in data {
        hash ^= word as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_hash() {
        let h1 = fnv1a_hash(b"hello");
        let h2 = fnv1a_hash(b"hello");
        let h3 = fnv1a_hash(b"world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_fnv1a_hash_u32() {
        let h1 = fnv1a_hash_u32(&[1, 2, 3]);
        let h2 = fnv1a_hash_u32(&[1, 2, 3]);
        let h3 = fnv1a_hash_u32(&[3, 2, 1]);
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_fx_hashmap() {
        let mut map = FxHashMap::default();
        map.insert(1u32, "hello");
        map.insert(2u32, "world");
        assert_eq!(map.get(&1), Some(&"hello"));
        assert_eq!(map.len(), 2);
    }
}
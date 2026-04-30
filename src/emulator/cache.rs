//! Pipeline cache for emulator workloads
//!
//! Emulators create many similar pipelines (same shaders, different state).
//! The pipeline cache avoids redundant shader compilation by caching
//! compiled shader programs keyed by their SPIR-V hash + state vector.
//!
//! ## Cache Strategy
//!
//! For emulators, we use an aggressive caching strategy:
//! 1. Hash the SPIR-V code + all non-dynamic state
//! 2. Look up in the in-memory cache
//! 3. If hit: reuse the compiled shader program
//! 4. If miss: compile, cache, and return
//!
//! This is critical for emulators because:
//! - Shader compilation is expensive (10-100ms per shader)
//! - Emulators use a finite set of shaders (typically 50-200)
//! - Shaders are reused every frame with different state

use crate::compiler::valhall::CompiledShader;
use crate::util::hash::FxHashMap;
use crate::LOG_TARGET;
use log::{debug, trace};
use parking_lot::RwLock;
use std::hash::{Hash, Hasher};

/// Pipeline cache key - uniquely identifies a pipeline
#[derive(Debug, Clone)]
pub struct PipelineCacheKey {
    /// Hash of the vertex shader SPIR-V
    pub vs_spirv_hash: u64,
    /// Hash of the fragment shader SPIR-V
    pub fs_spirv_hash: u64,
    /// Hash of the compute shader SPIR-V (if compute pipeline)
    pub cs_spirv_hash: u64,
    /// Vertex state hash (attribute format, binding stride, etc.)
    pub vertex_state_hash: u64,
    /// Blend state hash
    pub blend_state_hash: u64,
    /// Render pass format hash
    pub render_pass_hash: u64,
}

impl Hash for PipelineCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.vs_spirv_hash);
        state.write_u64(self.fs_spirv_hash);
        state.write_u64(self.cs_spirv_hash);
        state.write_u64(self.vertex_state_hash);
        state.write_u64(self.blend_state_hash);
        state.write_u64(self.render_pass_hash);
    }
}

impl PartialEq for PipelineCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.vs_spirv_hash == other.vs_spirv_hash
            && self.fs_spirv_hash == other.fs_spirv_hash
            && self.cs_spirv_hash == other.cs_spirv_hash
            && self.vertex_state_hash == other.vertex_state_hash
            && self.blend_state_hash == other.blend_state_hash
            && self.render_pass_hash == other.render_pass_hash
    }
}

impl Eq for PipelineCacheKey {}

/// Compute a fast hash of SPIR-V bytecode
pub fn hash_spirv(spirv: &[u32]) -> u64 {
    // Use a simple but fast hash (FNV-1a variant)
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &word in spirv {
        hash ^= word as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

/// Pipeline cache entry
#[derive(Debug, Clone)]
pub struct PipelineCacheEntry {
    /// Cache key
    pub key: PipelineCacheKey,
    /// Compiled vertex shader
    pub vs: Option<CompiledShader>,
    /// Compiled fragment shader
    pub fs: Option<CompiledShader>,
    /// Compiled compute shader
    pub cs: Option<CompiledShader>,
    /// GPU address of the pipeline descriptor
    pub gpu_addr: u64,
    /// Time when this entry was created (for LRU eviction)
    pub created_at: std::time::Instant,
    /// Number of times this pipeline has been used
    pub use_count: u64,
    /// Total size of compiled shaders in bytes
    pub total_size: u64,
}

/// Pipeline cache statistics
#[derive(Debug, Clone, Default)]
pub struct PipelineCacheStats {
    /// Total number of lookups
    pub lookups: u64,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
    /// Current number of entries
    pub entry_count: u32,
    /// Total memory used by cached pipelines
    pub total_memory: u64,
}

impl PipelineCacheStats {
    /// Get the hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f32 {
        if self.lookups == 0 {
            return 0.0;
        }
        self.hits as f32 / self.lookups as f32
    }
}

/// Maximum cache entries (prevent unbounded growth)
const MAX_CACHE_ENTRIES: usize = 512;

/// Pipeline cache - caches compiled shader programs
pub struct PipelineCache {
    /// Cache entries
    entries: RwLock<FxHashMap<PipelineCacheKey, PipelineCacheEntry>>,
    /// Statistics
    stats: RwLock<PipelineCacheStats>,
    /// Maximum number of entries
    max_entries: usize,
}

impl PipelineCache {
    /// Create a new pipeline cache
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(FxHashMap::default()),
            stats: RwLock::new(PipelineCacheStats::default()),
            max_entries: MAX_CACHE_ENTRIES,
        }
    }

    /// Create a pipeline cache with a custom max size
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(FxHashMap::default()),
            stats: RwLock::new(PipelineCacheStats::default()),
            max_entries,
        }
    }

    /// Look up a compiled pipeline in the cache
    pub fn lookup(&self, key: &PipelineCacheKey) -> Option<PipelineCacheEntry> {
        let mut stats = self.stats.write();
        stats.lookups += 1;

        let entries = self.entries.read();
        if let Some(entry) = entries.get(key) {
            stats.hits += 1;
            trace!(target: LOG_TARGET, "Pipeline cache HIT (use_count={})", entry.use_count);
            let mut entry = entry.clone();
            entry.use_count += 1;
            Some(entry)
        } else {
            stats.misses += 1;
            trace!(target: LOG_TARGET, "Pipeline cache MISS");
            None
        }
    }

    /// Insert a compiled pipeline into the cache
    pub fn insert(&self, key: PipelineCacheKey, entry: PipelineCacheEntry) {
        // Check if we need to evict
        {
            let entries = self.entries.read();
            if entries.len() >= self.max_entries {
                drop(entries);
                self.evict_lru();
            }
        }

        let total_size = entry.total_size;
        let entries = self.entries.write();
        let mut stats = self.stats.write();
        stats.entry_count = entries.len() as u32 + 1;
        stats.total_memory += total_size;

        drop(entries);
        drop(stats);

        let mut entries = self.entries.write();
        entries.insert(key, entry);

        debug!(target: LOG_TARGET, "Pipeline cache: inserted entry (total={})", entries.len());
    }

    /// Evict the least recently used entry
    fn evict_lru(&self) {
        let mut entries = self.entries.write();
        let mut stats = self.stats.write();

        let oldest_key = entries
            .iter()
            .min_by_key(|(_, v)| v.created_at)
            .map(|(k, _)| k.clone());

        if let Some(key) = oldest_key {
            if let Some(removed) = entries.remove(&key) {
                stats.evictions += 1;
                stats.total_memory -= removed.total_size;
                stats.entry_count = entries.len() as u32;
                trace!(target: LOG_TARGET, "Pipeline cache: evicted LRU entry");
            }
        }
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        let mut stats = self.stats.write();
        let count = entries.len();
        entries.clear();
        stats.total_memory = 0;
        stats.entry_count = 0;
        debug!(target: LOG_TARGET, "Pipeline cache: cleared {} entries", count);
    }

    /// Get cache statistics
    pub fn stats(&self) -> PipelineCacheStats {
        self.stats.read().clone()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Serialize the cache to bytes (for disk persistence)
    pub fn serialize(&self) -> Result<Vec<u8>, CacheError> {
        // In production, this would serialize all compiled shader binaries
        // and their associated keys for disk persistence across restarts.
        let entries = self.entries.read();
        let mut data = Vec::with_capacity(entries.len() * 256);
        for entry in entries.values() {
            // Serialize key hashes
            data.extend_from_slice(&entry.key.vs_spirv_hash.to_le_bytes());
            data.extend_from_slice(&entry.key.fs_spirv_hash.to_le_bytes());
            data.extend_from_slice(&entry.key.cs_spirv_hash.to_le_bytes());
            data.extend_from_slice(&entry.key.vertex_state_hash.to_le_bytes());
            data.extend_from_slice(&entry.key.blend_state_hash.to_le_bytes());
            data.extend_from_slice(&entry.key.render_pass_hash.to_le_bytes());
            // Serialize compiled shader data
            if let Some(ref vs) = entry.vs {
                data.extend_from_slice(&vs.binary());
            }
            if let Some(ref fs) = entry.fs {
                data.extend_from_slice(&fs.binary());
            }
        }
        Ok(data)
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache errors
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Serialization error
    #[error("Cache serialization error: {0}")]
    SerializationError(String),
    /// Deserialization error
    #[error("Cache deserialization error: {0}")]
    DeserializationError(String),
    /// Entry not found
    #[error("Cache entry not found")]
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spirv_hash() {
        let spirv = [0x07230203u32, 0x00010000, 0x00000001];
        let hash = hash_spirv(&spirv);
        assert_ne!(hash, 0);
        // Same input should produce same hash
        assert_eq!(hash, hash_spirv(&spirv));
    }

    #[test]
    fn test_pipeline_cache_basic() {
        let cache = PipelineCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        let key = PipelineCacheKey {
            vs_spirv_hash: 123,
            fs_spirv_hash: 456,
            cs_spirv_hash: 0,
            vertex_state_hash: 0,
            blend_state_hash: 0,
            render_pass_hash: 0,
        };

        // Cache miss
        assert!(cache.lookup(&key).is_none());
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);

        // Insert entry
        let entry = PipelineCacheEntry {
            key: key.clone(),
            vs: None,
            fs: None,
            cs: None,
            gpu_addr: 0x1000,
            created_at: std::time::Instant::now(),
            use_count: 0,
            total_size: 1024,
        };
        cache.insert(key.clone(), entry);
        assert_eq!(cache.len(), 1);

        // Cache hit
        let result = cache.lookup(&key);
        assert!(result.is_some());
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_pipeline_cache_lru_eviction() {
        let cache = PipelineCache::with_max_entries(2);

        for i in 0..3 {
            let key = PipelineCacheKey {
                vs_spirv_hash: i,
                fs_spirv_hash: 0,
                cs_spirv_hash: 0,
                vertex_state_hash: 0,
                blend_state_hash: 0,
                render_pass_hash: 0,
            };
            let entry = PipelineCacheEntry {
                key: key.clone(),
                vs: None,
                fs: None,
                cs: None,
                gpu_addr: 0,
                created_at: std::time::Instant::now(),
                use_count: 0,
                total_size: 100,
            };
            cache.insert(key, entry);
        }

        // After 3 inserts with max 2, one should be evicted
        assert_eq!(cache.len(), 2);
        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = PipelineCacheStats::default();
        stats.lookups = 100;
        stats.hits = 75;
        assert!((stats.hit_rate() - 0.75).abs() < 0.01);
    }
}
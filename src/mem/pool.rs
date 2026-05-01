//! Memory pool for typed GPU allocations
//!
//! Memory pools provide typed, suballocated memory regions optimized for
//! specific usage patterns. This is critical for emulators which create
//! many small buffers and images.

use crate::mem::bo::{BoFlags, BoError};
use crate::mem::slab::{SlabAllocator, SlabAllocation};
use crate::LOG_TARGET;
use log::debug;
use std::os::unix::io::RawFd;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Pool type for specialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolType {
    /// Texture/staging data
    Texture,
    /// Vertex/index buffers
    Vertex,
    /// Command buffers
    Command,
    /// Descriptor sets
    Descriptor,
    /// Shader programs
    Shader,
    /// Tiler heap
    Tiler,
    /// General purpose
    General,
}

impl PoolType {
    /// Get the recommended pool size for this type
    pub fn recommended_size(&self) -> u64 {
        match self {
            PoolType::Texture => 64 * 1024 * 1024,  // 64 MB
            PoolType::Vertex => 16 * 1024 * 1024,    // 16 MB
            PoolType::Command => 4 * 1024 * 1024,    // 4 MB
            PoolType::Descriptor => 2 * 1024 * 1024, // 2 MB
            PoolType::Shader => 4 * 1024 * 1024,     // 4 MB
            PoolType::Tiler => 8 * 1024 * 1024,      // 8 MB
            PoolType::General => 8 * 1024 * 1024,    // 8 MB
        }
    }

    /// Get the recommended alignment for this pool type
    pub fn recommended_alignment(&self) -> u64 {
        match self {
            PoolType::Texture => 256,    // Texture alignment
            PoolType::Vertex => 16,      // Vertex buffer alignment
            PoolType::Command => 64,     // Command buffer alignment
            PoolType::Descriptor => 32,  // Descriptor alignment
            PoolType::Shader => 64,      // Shader program alignment
            PoolType::Tiler => 4096,     // Tiler page alignment
            PoolType::General => 16,
        }
    }

    /// Get BO flags for this pool type
    pub fn bo_flags(&self) -> BoFlags {
        let base = BoFlags::GPU_READ | BoFlags::GPU_WRITE | BoFlags::GPU_CACHED;
        match self {
            PoolType::Texture => base | BoFlags::CPU_READ | BoFlags::CPU_WRITE,
            PoolType::Vertex => base | BoFlags::CPU_WRITE,
            PoolType::Command => base | BoFlags::CPU_WRITE | BoFlags::CMD_STREAM,
            PoolType::Descriptor => base | BoFlags::CPU_WRITE,
            PoolType::Shader => base | BoFlags::SHADER,
            PoolType::Tiler => base | BoFlags::TILER,
            PoolType::General => base | BoFlags::CPU_READ | BoFlags::CPU_WRITE,
        }
    }

    /// Get the pool type name
    pub fn name(&self) -> &'static str {
        match self {
            PoolType::Texture => "texture",
            PoolType::Vertex => "vertex",
            PoolType::Command => "command",
            PoolType::Descriptor => "descriptor",
            PoolType::Shader => "shader",
            PoolType::Tiler => "tiler",
            PoolType::General => "general",
        }
    }
}

/// Memory pool - manages suballocations for a specific usage type
pub struct MemoryPool {
    /// Pool type
    pool_type: PoolType,
    /// Slab allocator
    slab: RwLock<SlabAllocator>,
    /// Active allocations
    allocations: RwLock<HashMap<u64, SlabAllocation>>,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(drm_fd: RawFd, pool_type: PoolType) -> Self {
        let name_prefix = format!("pool_{}", pool_type.name());
        Self {
            pool_type,
            slab: RwLock::new(SlabAllocator::new(
                drm_fd,
                pool_type.recommended_size(),
                pool_type.recommended_alignment(),
                &name_prefix,
            )),
            allocations: RwLock::new(HashMap::new()),
        }
    }

    /// Allocate from this pool
    pub fn allocate(&self, size: u64) -> Result<SlabAllocation, PoolError> {
        let flags = self.pool_type.bo_flags();
        let alloc = self.slab.write().allocate(size, flags)?;
        self.allocations.write().insert(alloc.gpu_addr, alloc);
        Ok(alloc)
    }

    /// Free an allocation
    pub fn free(&self, alloc: SlabAllocation) {
        self.allocations.write().remove(&alloc.gpu_addr);
        self.slab.write().free(alloc);
    }

    /// Get the pool type
    pub fn pool_type(&self) -> PoolType {
        self.pool_type
    }

    /// Get the number of active allocations
    pub fn num_allocations(&self) -> usize {
        self.allocations.read().len()
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let slab_stats = self.slab.read().stats();
        PoolStats {
            pool_type: self.pool_type,
            num_allocations: self.allocations.read().len(),
            slab: slab_stats,
        }
    }
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Pool type
    pub pool_type: PoolType,
    /// Number of active allocations
    pub num_allocations: usize,
    /// Slab allocator statistics
    pub slab: crate::mem::slab::SlabStats,
}

/// Pool manager - manages all memory pools
pub struct PoolManager {
    /// DRM file descriptor
    drm_fd: RawFd,
    /// Memory pools by type
    pools: HashMap<PoolType, MemoryPool>,
}

impl PoolManager {
    /// Create a new pool manager
    pub fn new(drm_fd: RawFd) -> Self {
        let mut pools = HashMap::new();
        for pool_type in [
            PoolType::Texture,
            PoolType::Vertex,
            PoolType::Command,
            PoolType::Descriptor,
            PoolType::Shader,
            PoolType::Tiler,
            PoolType::General,
        ] {
            pools.insert(pool_type, MemoryPool::new(drm_fd, pool_type));
        }

        debug!(target: LOG_TARGET, "PoolManager: initialized 7 memory pools");
        Self { drm_fd, pools }
    }

    /// Allocate from a specific pool type
    pub fn allocate(&self, pool_type: PoolType, size: u64) -> Result<SlabAllocation, PoolError> {
        self.pools
            .get(&pool_type)
            .ok_or(PoolError::PoolNotFound)?
            .allocate(size)
    }

    /// Free an allocation to its pool
    pub fn free(&self, pool_type: PoolType, alloc: SlabAllocation) {
        if let Some(pool) = self.pools.get(&pool_type) {
            pool.free(alloc);
        }
    }

    /// Get a reference to a specific pool
    pub fn get_pool(&self, pool_type: PoolType) -> Option<&MemoryPool> {
        self.pools.get(&pool_type)
    }
}

/// Pool errors
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    /// Pool not found
    #[error("Pool not found")]
    PoolNotFound,
    /// Slab allocator error
    #[error("Slab error: {0}")]
    SlabError(#[from] crate::mem::slab::SlabError),
    /// BO error
    #[error("BO error: {0}")]
    BoError(#[from] BoError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_type_sizes() {
        assert_eq!(PoolType::Texture.recommended_size(), 64 * 1024 * 1024);
        assert_eq!(PoolType::Command.recommended_size(), 4 * 1024 * 1024);
    }

    #[test]
    fn test_pool_type_flags() {
        let cmd_flags = PoolType::Command.bo_flags();
        assert!(cmd_flags.contains(BoFlags::CMD_STREAM));
        assert!(cmd_flags.contains(BoFlags::GPU_CACHED));
    }

    #[test]
    fn test_pool_type_names() {
        assert_eq!(PoolType::Texture.name(), "texture");
        assert_eq!(PoolType::Shader.name(), "shader");
    }
}
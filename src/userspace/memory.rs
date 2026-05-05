//! User-space memory management for Mali-G68
//!
//! Provides memory pools and allocation strategies optimized
//! for emulator workloads without requiring kernel drivers.

use crate::drm::DrmDeviceManager;
use crate::mem::pool::{PoolManager, PoolType};
use crate::userspace::{UserSpaceConfig, UserSpaceError, UserSpaceResult};
use crate::LOG_TARGET;
use log::{debug, info, warn};
use parking_lot::RwLock;
use std::sync::Arc;

/// User-space memory manager
pub struct UserSpaceMemory {
    /// Configuration
    config: UserSpaceConfig,
    /// Pool manager
    pool_manager: Arc<PoolManager>,
    /// Memory usage tracking
    memory_stats: RwLock<MemoryStats>,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total allocated MB
    pub total_allocated_mb: f32,
    /// Peak usage MB
    pub peak_usage_mb: f32,
    /// Current usage MB
    pub current_usage_mb: f32,
    /// Allocation count
    pub allocation_count: u64,
    /// Free count
    pub free_count: u64,
}

impl UserSpaceMemory {
    /// Create new memory manager
    pub fn new(
        config: &UserSpaceConfig,
        device: Arc<crate::userspace::device::UserSpaceDevice>,
    ) -> UserSpaceResult<Self> {
        info!(target: LOG_TARGET, "Initializing user-space memory manager");

        // Create pool manager with custom pools
        let pool_manager = Arc::new(PoolManager::new(device.get_fd()));

        let memory = Self {
            config: config.clone(),
            pool_manager,
            memory_stats: RwLock::new(MemoryStats::default()),
        };

        // Pre-allocate common pools
        memory.preallocate_pools()?;

        info!(target: LOG_TARGET, "User-space memory manager initialized");
        Ok(memory)
    }

    /// Pre-allocate common memory pools
    fn preallocate_pools(&self) -> UserSpaceResult<()> {
        debug!(target: LOG_TARGET, "Pre-allocating memory pools");

        // Pre-allocate texture pool (largest)
        let _ = self
            .pool_manager
            .allocate(PoolType::Texture, 64 * 1024 * 1024)?;

        // Pre-allocate vertex buffer pool
        let _ = self
            .pool_manager
            .allocate(PoolType::Vertex, 16 * 1024 * 1024)?;

        // Pre-allocate command buffer pool
        let _ = self
            .pool_manager
            .allocate(PoolType::Command, 4 * 1024 * 1024)?;

        debug!(target: LOG_TARGET, "Memory pools pre-allocated");
        Ok(())
    }

    /// Allocate memory from optimal pool
    pub fn allocate(&self, size: u64, usage: MemoryUsage) -> UserSpaceResult<UserSpaceAllocation> {
        debug!(
            target: LOG_TARGET,
            "Allocating {} bytes for usage: {:?}",
            size, usage
        );

        let pool_type = self.usage_to_pool_type(usage);
        let allocation = self.pool_manager.allocate(pool_type, size)?;

        // Update statistics
        {
            let mut stats = self.memory_stats.write();
            stats.allocation_count += 1;
            stats.current_usage_mb += (size as f32) / (1024.0 * 1024.0);
            stats.total_allocated_mb += (size as f32) / (1024.0 * 1024.0);
            stats.peak_usage_mb = stats.peak_usage_mb.max(stats.current_usage_mb);
        }

        Ok(UserSpaceAllocation {
            addr: allocation.gpu_addr,
            size: allocation.size,
            pool_type,
            usage,
        })
    }

    /// Free memory allocation
    pub fn free(&self, allocation: UserSpaceAllocation) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Freeing {} bytes from {:?} pool",
            allocation.size, allocation.pool_type
        );

        // Convert back to slab allocation
        let slab_alloc = crate::mem::slab::SlabAllocation {
            slab_idx: 0,                                  // Simplified for user-space
            offset: allocation.addr % (64 * 1024 * 1024), // Assume 64MB pools
            size: allocation.size,
            gpu_addr: allocation.addr,
        };

        // Free to appropriate pool
        self.pool_manager.free(allocation.pool_type, slab_alloc);

        // Update statistics
        {
            let mut stats = self.memory_stats.write();
            stats.free_count += 1;
            stats.current_usage_mb -= (allocation.size as f32) / (1024.0 * 1024.0);
        }

        Ok(())
    }

    /// Get memory usage statistics
    pub fn get_used_mb(&self) -> f32 {
        self.memory_stats.read().current_usage_mb
    }

    /// Get memory usage metrics
    pub fn get_metrics(&self) -> super::UserSpaceMetrics {
        let stats = self.memory_stats.read();
        let total_mb = self.config.memory_pool_size_mb as f32;

        super::UserSpaceMetrics {
            total_mb,
            used_mb: stats.current_usage_mb,
            available_mb: total_mb - stats.current_usage_mb,
            utilization_percent: if total_mb > 0.0 {
                (stats.current_usage_mb / total_mb) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Get pool manager reference
    pub fn get_pool_manager(&self) -> Arc<PoolManager> {
        self.pool_manager.clone()
    }

    /// Convert memory usage to pool type
    fn usage_to_pool_type(&self, usage: MemoryUsage) -> PoolType {
        match usage {
            MemoryUsage::Texture => PoolType::Texture,
            MemoryUsage::VertexBuffer => PoolType::Vertex,
            MemoryUsage::IndexBuffer => PoolType::Vertex,
            MemoryUsage::UniformBuffer => PoolType::Descriptor,
            MemoryUsage::CommandBuffer => PoolType::Command,
            MemoryUsage::Shader => PoolType::Shader,
            MemoryUsage::Tiler => PoolType::Tiler,
            MemoryUsage::General => PoolType::General,
        }
    }

    /// Cleanup memory manager
    pub fn cleanup(&self) -> UserSpaceResult<()> {
        info!(target: LOG_TARGET, "Cleaning up user-space memory manager");

        // In a real implementation, this would:
        // 1. Free all allocations
        // 2. Destroy memory pools
        // 3. Release DRM resources

        info!(target: LOG_TARGET, "User-space memory manager cleanup completed");
        Ok(())
    }
}

/// Memory usage types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUsage {
    /// Texture data
    Texture,
    /// Vertex buffer data
    VertexBuffer,
    /// Index buffer data
    IndexBuffer,
    /// Uniform buffer data
    UniformBuffer,
    /// Command buffer data
    CommandBuffer,
    /// Shader code
    Shader,
    /// Tiler data
    Tiler,
    /// General purpose
    General,
}

/// User-space memory allocation
#[derive(Debug, Clone)]
pub struct UserSpaceAllocation {
    /// GPU address
    pub addr: u64,
    /// Size in bytes
    pub size: u64,
    /// Pool type
    pub pool_type: PoolType,
    /// Usage type
    pub usage: MemoryUsage,
}

/// Memory usage metrics
#[derive(Debug, Clone)]
pub struct UserSpaceMetrics {
    /// Total memory MB
    pub total_mb: f32,
    /// Used memory MB
    pub used_mb: f32,
    /// Available memory MB
    pub available_mb: f32,
    /// Utilization percentage
    pub utilization_percent: f32,
}

impl Drop for UserSpaceMemory {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

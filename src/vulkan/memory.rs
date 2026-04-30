//! Vulkan memory allocation
//!
//! Manages GPU memory allocation using the memory pool system.

use crate::mem::slab::SlabAllocation;

/// Vulkan device memory
pub struct VkDeviceMemory {
    /// Allocation from the memory pool
    allocation: Option<SlabAllocation>,
    /// Memory type index
    memory_type_index: u32,
    /// Allocation size
    size: u64,
    /// Mapped pointer
    mapped_ptr: Option<*mut u8>,
}

unsafe impl Send for VkDeviceMemory {}
unsafe impl Sync for VkDeviceMemory {}

impl VkDeviceMemory {
    /// Allocate device memory
    pub fn allocate(size: u64, memory_type_index: u32) -> Result<Self, MemoryError> {
        Ok(Self {
            allocation: None,
            memory_type_index,
            size,
            mapped_ptr: None,
        })
    }

    /// Map the memory for CPU access
    pub fn map(&mut self, _offset: u64, _size: u64) -> Result<*mut u8, MemoryError> {
        // In production: mmap the backing BO
        Ok(std::ptr::null_mut())
    }

    /// Unmap the memory
    pub fn unmap(&mut self) {
        self.mapped_ptr = None;
    }

    /// Get the allocation size
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Free the memory
    pub fn free(self) {
        // Allocations returned to pool on drop
    }
}

/// Memory errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// Out of device memory
    #[error("Out of device memory: requested {requested} bytes")]
    OutOfMemory { requested: u64 },
    /// Invalid memory type
    #[error("Invalid memory type index: {0}")]
    InvalidMemoryType(u32),
    /// Map failed
    #[error("Memory map failed")]
    MapFailed,
}
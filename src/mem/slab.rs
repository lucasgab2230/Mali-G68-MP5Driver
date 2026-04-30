//! Slab allocator for small GPU memory allocations
//!
//! Emulators create many small allocations (descriptor sets, push constants,
//! shader programs, etc.). The slab allocator suballocates from large buffer
//! objects to minimize DRM allocation overhead and improve cache locality.

use crate::mem::bo::{BoFlags, BufferObject, BoError};
use crate::LOG_TARGET;
use log::debug;
use std::os::unix::io::RawFd;
use std::collections::VecDeque;

/// Default slab size (64 KB)
pub const DEFAULT_SLAB_SIZE: u64 = 64 * 1024;

/// Minimum allocation alignment
pub const MIN_ALIGNMENT: u64 = 8;

/// Slab allocator - suballocates from a large buffer object
pub struct SlabAllocator {
    /// DRM file descriptor
    drm_fd: RawFd,
    /// Slab buffer objects
    slabs: Vec<Slab>,
    /// Allocation alignment
    alignment: u64,
    /// Slab size
    slab_size: u64,
    /// Debug name prefix
    name_prefix: String,
    /// Total allocations made
    total_allocs: u64,
    /// Total frees made
    total_frees: u64,
}

/// A single slab (backed by a BO)
struct Slab {
    /// Backing buffer object
    bo: BufferObject,
    /// Free list (offsets within the slab)
    free_list: VecDeque<u64>,
    /// Total capacity in bytes
    capacity: u64,
    /// Used bytes
    used: u64,
    /// Allocation watermark (high water mark)
    watermark: u64,
    /// Slab index
    idx: u32,
}

/// Handle to a slab allocation
#[derive(Debug, Clone, Copy)]
pub struct SlabAllocation {
    /// Slab index
    pub slab_idx: u32,
    /// Offset within the slab BO
    pub offset: u64,
    /// Size of the allocation
    pub size: u64,
    /// GPU address of the allocation
    pub gpu_addr: u64,
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub fn new(drm_fd: RawFd, slab_size: u64, alignment: u64, name_prefix: &str) -> Self {
        Self {
            drm_fd,
            slabs: Vec::new(),
            alignment: alignment.max(MIN_ALIGNMENT),
            slab_size: slab_size.max(DEFAULT_SLAB_SIZE),
            name_prefix: name_prefix.to_string(),
            total_allocs: 0,
            total_frees: 0,
        }
    }

    /// Allocate from the slab
    pub fn allocate(&mut self, size: u64, flags: BoFlags) -> Result<SlabAllocation, SlabError> {
        // Align size
        let aligned_size = (size + self.alignment - 1) & !(self.alignment - 1);

        // Try to find a slab with enough free space
        for slab_idx in 0..self.slabs.len() {
            if let Some(alloc) = self.try_alloc_from_slab(slab_idx, aligned_size)? {
                self.total_allocs += 1;
                return Ok(alloc);
            }
        }

        // Create a new slab
        let slab_idx = self.create_slab(aligned_size.max(self.slab_size), flags)?;
        let alloc = self.try_alloc_from_slab(slab_idx as usize, aligned_size)?.unwrap();
        self.total_allocs += 1;
        Ok(alloc)
    }

    /// Free a slab allocation
    pub fn free(&mut self, alloc: SlabAllocation) {
        if let Some(slab) = self.slabs.get_mut(alloc.slab_idx as usize) {
            slab.free_list.push_back(alloc.offset);
            slab.used -= alloc.size;
            self.total_frees += 1;
        }
    }

    /// Try to allocate from a specific slab
    fn try_alloc_from_slab(&mut self, slab_idx: usize, size: u64) -> Result<Option<SlabAllocation>, SlabError> {
        let slab = &mut self.slabs[slab_idx];

        // Try the free list first
        if let Some(offset) = slab.free_list.pop_front() {
            let gpu_addr = slab.bo.gpu_addr() + offset;
            slab.used += size;
            slab.watermark = slab.watermark.max(slab.used);
            return Ok(Some(SlabAllocation {
                slab_idx: slab_idx as u32,
                offset,
                size,
                gpu_addr,
            }));
        }

        // Try bump allocation from the end
        let new_offset = slab.used + size;
        if new_offset <= slab.capacity {
            let offset = slab.used;
            let gpu_addr = slab.bo.gpu_addr() + offset;
            slab.used = new_offset;
            slab.watermark = slab.watermark.max(slab.used);
            Ok(Some(SlabAllocation {
                slab_idx: slab_idx as u32,
                offset,
                size,
                gpu_addr,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a new slab
    fn create_slab(&mut self, min_size: u64, flags: BoFlags) -> Result<u32, SlabError> {
        let slab_size = min_size.max(self.slab_size);
        let slab_idx = self.slabs.len() as u32;
        let name = format!("{}_slab{}", self.name_prefix, slab_idx);

        let bo = BufferObject::new(self.drm_fd, slab_size, flags, &name)
            .map_err(|e| SlabError::BoError(e))?;

        debug!(
            target: LOG_TARGET,
            "SlabAllocator: created slab {} (size={:#x}, name='{}')",
            slab_idx, slab_size, name
        );

        self.slabs.push(Slab {
            bo,
            free_list: VecDeque::new(),
            capacity: slab_size,
            used: 0,
            watermark: 0,
            idx: slab_idx,
        });

        Ok(slab_idx)
    }

    /// Get the total allocated bytes across all slabs
    pub fn total_used(&self) -> u64 {
        self.slabs.iter().map(|s| s.used).sum()
    }

    /// Get the total capacity across all slabs
    pub fn total_capacity(&self) -> u64 {
        self.slabs.iter().map(|s| s.capacity).sum()
    }

    /// Get the number of slabs
    pub fn num_slabs(&self) -> u32 {
        self.slabs.len() as u32
    }

    /// Get allocation statistics
    pub fn stats(&self) -> SlabStats {
        SlabStats {
            num_slabs: self.slabs.len(),
            total_capacity: self.total_capacity(),
            total_used: self.total_used(),
            total_allocs: self.total_allocs,
            total_frees: self.total_frees,
            utilization: if self.total_capacity() > 0 {
                self.total_used() as f32 / self.total_capacity() as f32
            } else {
                0.0
            },
        }
    }
}

/// Slab allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct SlabStats {
    /// Number of slabs
    pub num_slabs: usize,
    /// Total capacity in bytes
    pub total_capacity: u64,
    /// Total used bytes
    pub total_used: u64,
    /// Total allocations
    pub total_allocs: u64,
    /// Total frees
    pub total_frees: u64,
    /// Utilization ratio (0.0 - 1.0)
    pub utilization: f32,
}

/// Slab allocator errors
#[derive(Debug, thiserror::Error)]
pub enum SlabError {
    /// Buffer object error
    #[error("BO error: {0}")]
    BoError(#[from] BoError),
    /// Out of memory
    #[error("Slab out of memory: requested {requested}, max slab {max}")]
    OutOfMemory { requested: u64, max: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_allocator_creation() {
        let allocator = SlabAllocator::new(-1, DEFAULT_SLAB_SIZE, 16, "test");
        assert_eq!(allocator.num_slabs(), 0);
    }

    #[test]
    fn test_slab_stats() {
        let allocator = SlabAllocator::new(-1, DEFAULT_SLAB_SIZE, 16, "test");
        let stats = allocator.stats();
        assert_eq!(stats.num_slabs, 0);
        assert_eq!(stats.utilization, 0.0);
    }
}
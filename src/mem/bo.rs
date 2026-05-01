//! Buffer Object (BO) management via DRM
//!
//! Buffer objects are the fundamental unit of GPU memory allocation.
//! They are allocated through the DRM subsystem and can be mapped
//! into both the GPU and CPU address spaces.

use crate::LOG_TARGET;
use log::{debug, trace, warn};
use std::os::unix::io::RawFd;

/// Buffer object creation flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BoFlags: u32 {
        /// Buffer can be read by GPU
        const GPU_READ = 1 << 0;
        /// Buffer can be written by GPU
        const GPU_WRITE = 1 << 1;
        /// Buffer can be read by CPU
        const CPU_READ = 1 << 2;
        /// Buffer can be written by CPU
        const CPU_WRITE = 1 << 3;
        /// Buffer is used for command stream
        const CMD_STREAM = 1 << 4;
        /// Buffer is used for tiler
        const TILER = 1 << 5;
        /// Buffer is used for shader programs
        const SHADER = 1 << 6;
        /// Buffer should be cached on GPU
        const GPU_CACHED = 1 << 7;
        /// Buffer should be cached on CPU
        const CPU_CACHED = 1 << 8;
        /// Buffer is scanout-capable (for display)
        const SCANOUT = 1 << 9;
        /// Buffer can be shared across processes
        const SHAREABLE = 1 << 10;
        /// Buffer uses AFBC compression
        const AFBC_COMPRESSED = 1 << 11;
        /// Buffer is protected (secure display)
        const PROTECTED = 1 << 12;
    }
}

impl Default for BoFlags {
    fn default() -> Self {
        Self::GPU_READ | Self::GPU_WRITE | Self::GPU_CACHED | Self::CPU_CACHED
    }
}

/// Buffer object memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoMemoryType {
    /// Device-local memory (GPU VRAM)
    DeviceLocal,
    /// Host-visible device-local (unified memory on mobile)
    DeviceLocalHostVisible,
    /// Host-visible, host-coherent (system RAM)
    HostVisible,
    /// Host-cached
    HostCached,
}

impl BoMemoryType {
    /// Get the default memory type for Mali-G68 (unified memory)
    pub fn default_for_mali() -> Self {
        // Mali GPUs on mobile SoCs use unified memory
        Self::DeviceLocalHostVisible
    }
}

/// Buffer Object - represents a GPU-accessible memory allocation
pub struct BufferObject {
    /// DRM file descriptor
    drm_fd: RawFd,
    /// DRM buffer handle
    handle: u32,
    /// Size in bytes
    size: u64,
    /// GPU virtual address
    gpu_addr: u64,
    /// CPU mapping address (if mapped)
    cpu_ptr: Option<*mut u8>,
    /// CPU mapping size
    map_size: u64,
    /// Creation flags
    flags: BoFlags,
    /// Memory type
    mem_type: BoMemoryType,
    /// Name for debugging
    name: String,
}

// BufferObject contains raw pointers but access is controlled
unsafe impl Send for BufferObject {}
unsafe impl Sync for BufferObject {}

impl BufferObject {
    /// Create a new buffer object via DRM
    ///
    /// # Arguments
    /// * `drm_fd` - Open DRM device file descriptor
    /// * `size` - Buffer size in bytes
    /// * `flags` - Buffer creation flags
    /// * `name` - Debug name for the buffer
    pub fn new(
        drm_fd: RawFd,
        size: u64,
        flags: BoFlags,
        name: &str,
    ) -> Result<Self, BoError> {
        // Align size to page boundary
        let aligned_size = (size + 4095) & !4095;

        debug!(
            target: LOG_TARGET,
            "BO alloc: name='{}' size={:#x} (aligned={:#x}) flags={:?}",
            name, size, aligned_size, flags
        );

        // In production, this calls DRM_IOCTL_PANFROST_BO_CREATE or
        // DRM_IOCTL_GEM_CREATE depending on the kernel driver.
        // For now, we simulate the allocation.
        let handle = Self::drm_bo_create(drm_fd, aligned_size, flags)?;
        let gpu_addr = Self::drm_bo_mmap_offset(drm_fd, handle)?;

        Ok(Self {
            drm_fd,
            handle,
            size: aligned_size,
            gpu_addr,
            cpu_ptr: None,
            map_size: 0,
            flags,
            mem_type: BoMemoryType::default_for_mali(),
            name: name.to_string(),
        })
    }

    /// Get the buffer size in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get the GPU virtual address
    pub fn gpu_addr(&self) -> u64 {
        self.gpu_addr
    }

    /// Get the DRM handle
    pub fn handle(&self) -> u32 {
        self.handle
    }

    /// Get the creation flags
    pub fn flags(&self) -> BoFlags {
        self.flags
    }

    /// Get the memory type
    pub fn mem_type(&self) -> BoMemoryType {
        self.mem_type
    }

    /// Get the debug name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Map the buffer into CPU address space
    pub fn mmap(&mut self) -> Result<*mut u8, BoError> {
        if self.cpu_ptr.is_some() {
            return Ok(self.cpu_ptr.unwrap());
        }

        // In production, this calls mmap() on the DRM buffer
        let ptr = Self::drm_bo_mmap(self.drm_fd, self.handle, self.size)?;
        self.cpu_ptr = Some(ptr);
        self.map_size = self.size;
        debug!(
            target: LOG_TARGET,
            "BO mmap: name='{}' ptr={:p} size={:#x}",
            self.name, ptr, self.size
        );
        Ok(ptr)
    }

    /// Unmap the buffer from CPU address space
    pub fn munmap(&mut self) -> Result<(), BoError> {
        if let Some(ptr) = self.cpu_ptr.take() {
            unsafe {
                let result = libc::munmap(ptr as *mut libc::c_void, self.map_size as usize);
                if result != 0 {
                    warn!(target: LOG_TARGET, "BO munmap failed for '{}'", self.name);
                }
            }
            self.map_size = 0;
        }
        Ok(())
    }

    /// Get the mapped CPU pointer (if mapped)
    pub fn mapped_ptr(&self) -> Option<*mut u8> {
        self.cpu_ptr
    }

    /// Check if the buffer is CPU-mapped
    pub fn is_mapped(&self) -> bool {
        self.cpu_ptr.is_some()
    }

    /// Write data to the buffer (must be mapped)
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<(), BoError> {
        let ptr = self.cpu_ptr.ok_or(BoError::NotMapped)?;
        if offset + data.len() as u64 > self.size {
            return Err(BoError::OutOfBounds {
                offset,
                size: data.len() as u64,
                buf_size: self.size,
            });
        }
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                ptr.add(offset as usize),
                data.len(),
            );
        }
        Ok(())
    }

    /// Read data from the buffer (must be mapped)
    pub fn read(&self, offset: u64, data: &mut [u8]) -> Result<(), BoError> {
        let ptr = self.cpu_ptr.ok_or(BoError::NotMapped)?;
        if offset + data.len() as u64 > self.size {
            return Err(BoError::OutOfBounds {
                offset,
                size: data.len() as u64,
                buf_size: self.size,
            });
        }
        unsafe {
            core::ptr::copy_nonoverlapping(
                ptr.add(offset as usize),
                data.as_mut_ptr(),
                data.len(),
            );
        }
        Ok(())
    }

    /// Flush CPU writes to GPU (cache coherency)
    pub fn flush(&self, offset: u64, size: u64) -> Result<(), BoError> {
        // On unified memory (ARM), this is typically a no-op
        // since the memory is coherent. But for host-cached memory,
        // we need to flush the CPU cache.
        if self.mem_type == BoMemoryType::HostCached {
            // DRM_IOCTL_SYNC_FLUSH
            trace!(target: LOG_TARGET, "BO flush: name='{}' offset={:#x} size={:#x}", self.name, offset, size);
        }
        Ok(())
    }

    /// Invalidate CPU cache from GPU writes
    pub fn invalidate(&self, offset: u64, size: u64) -> Result<(), BoError> {
        if self.mem_type == BoMemoryType::HostCached {
            trace!(target: LOG_TARGET, "BO invalidate: name='{}' offset={:#x} size={:#x}", self.name, offset, size);
        }
        Ok(())
    }

    /// Create a DRM buffer object (simulated)
    fn drm_bo_create(_fd: RawFd, _size: u64, _flags: BoFlags) -> Result<u32, BoError> {
        // In production: DRM_IOCTL_PANFROST_BO_CREATE or DRM_IOCTL_GEM_CREATE
        // Returns a GEM handle
        static NEXT_HANDLE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        Ok(NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }

    /// Get the mmap offset for a DRM buffer (simulated)
    fn drm_bo_mmap_offset(_fd: RawFd, _handle: u32) -> Result<u64, BoError> {
        // In production: DRM_IOCTL_GEM_MMAP_OFFSET
        // Returns the fake offset used by mmap()
        static NEXT_ADDR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x1_0000_0000);
        let addr = NEXT_ADDR.fetch_add(0x100_0000, std::sync::atomic::Ordering::Relaxed);
        Ok(addr)
    }

    /// Map a DRM buffer into CPU address space (simulated)
    fn drm_bo_mmap(fd: RawFd, _handle: u32, size: u64) -> Result<*mut u8, BoError> {
        // In production: mmap() with the DRM mmap offset
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size as usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0, // offset from DRM_IOCTL_GEM_MMAP_OFFSET
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(BoError::MmapFailed);
        }
        Ok(ptr as *mut u8)
    }
}

impl Drop for BufferObject {
    fn drop(&mut self) {
        if self.cpu_ptr.is_some() {
            let _ = self.munmap();
        }
        // In production: DRM_IOCTL_GEM_CLOSE to free the handle
        debug!(
            target: LOG_TARGET,
            "BO free: name='{}' handle={} size={:#x}",
            self.name, self.handle, self.size
        );
    }
}

impl std::fmt::Debug for BufferObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferObject")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("gpu_addr", &format_args!("{:#x}", self.gpu_addr))
            .field("handle", &self.handle)
            .field("flags", &self.flags)
            .field("mapped", &self.cpu_ptr.is_some())
            .finish()
    }
}

/// Buffer object errors
#[derive(Debug, thiserror::Error)]
pub enum BoError {
    /// DRM allocation failed
    #[error("DRM BO allocation failed: {0}")]
    AllocFailed(String),
    /// mmap failed
    #[error("BO mmap failed")]
    MmapFailed,
    /// Buffer not mapped
    #[error("Buffer not mapped")]
    NotMapped,
    /// Out of bounds access
    #[error("Out of bounds: offset={offset:#x}, size={size:#x}, buf_size={buf_size:#x}")]
    OutOfBounds {
        offset: u64,
        size: u64,
        buf_size: u64,
    },
    /// DRM ioctl failed
    #[error("DRM ioctl failed: {0}")]
    IoctlFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bo_flags_default() {
        let flags = BoFlags::default();
        assert!(flags.contains(BoFlags::GPU_READ));
        assert!(flags.contains(BoFlags::GPU_WRITE));
        assert!(flags.contains(BoFlags::GPU_CACHED));
    }

    #[test]
    fn test_bo_flags_combined() {
        let flags = BoFlags::GPU_READ | BoFlags::GPU_WRITE | BoFlags::AFBC_COMPRESSED;
        assert!(flags.contains(BoFlags::AFBC_COMPRESSED));
        assert!(!flags.contains(BoFlags::SCANOUT));
    }
}
//! GPU Address Space management
//!
//! Each GPU context has its own address space, managed by the MMU.
//! The address space contains page tables that map GPU virtual
//! addresses to physical pages.

use crate::gpu::regs::*;
use crate::mem::bo::{BoFlags, BufferObject};
use crate::mmu::{PteFlags, VaRegion};
use crate::LOG_TARGET;
use log::{debug, trace};
use std::os::unix::io::RawFd;

/// Address space ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressSpaceId(u8);

impl AddressSpaceId {
    /// Create a new address space ID
    pub fn new(id: u8) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn raw(&self) -> u8 {
        self.0
    }
}

/// GPU Address Space
///
/// Manages virtual-to-physical address mappings for GPU memory access.
pub struct AddressSpace {
    /// Address space ID
    id: AddressSpaceId,
    /// Page table buffer object
    page_table: BufferObject,
    /// GPU register base for MMU operations
    gpu_reg_base: *mut u32,
    /// Current VA allocations per region
    region_allocators: Vec<VaAllocator>,
    /// Whether this AS is currently active on the GPU
    active: bool,
}

// Raw pointer field makes it !Send by default, but we manage access safely
unsafe impl Send for AddressSpace {}

impl AddressSpace {
    /// Create a new address space
    pub fn new(id: u8, drm_fd: RawFd, gpu_reg_base: *mut u32) -> Result<Self, AsError> {
        let as_id = AddressSpaceId::new(id);

        // Allocate page table (4 levels × 4KB per level = 16KB minimum)
        // For Mali-G68, we need L3 (PTE) + L2 (PMD) + L1 (PGD) tables
        let pt_size = 4 * 4096 * 3; // 3 levels, 4 pages each
        let page_table = BufferObject::new(
            drm_fd,
            pt_size,
            BoFlags::GPU_READ | BoFlags::GPU_WRITE | BoFlags::CPU_WRITE,
            &format!("as{}_page_table", id),
        )?;

        // Set up VA region allocators
        let region_allocators = vec![
            VaAllocator::new(VaRegion::ShaderCode),
            VaAllocator::new(VaRegion::TilerHeap),
            VaAllocator::new(VaRegion::Buffers),
            VaAllocator::new(VaRegion::Textures),
            VaAllocator::new(VaRegion::Descriptors),
            VaAllocator::new(VaRegion::CommandBuffers),
        ];

        debug!(
            target: LOG_TARGET,
            "AddressSpace {}: created (pt_gpu_addr={:#x})",
            id, page_table.gpu_addr()
        );

        Ok(Self {
            id: as_id,
            page_table,
            gpu_reg_base,
            region_allocators,
            active: false,
        })
    }

    /// Get the address space ID
    pub fn id(&self) -> AddressSpaceId {
        self.id
    }

    /// Get the page table GPU address
    pub fn page_table_addr(&self) -> u64 {
        self.page_table.gpu_addr()
    }

    /// Map a buffer object into this address space
    pub fn map_bo(&mut self, bo: &BufferObject, region: VaRegion, flags: PteFlags) -> Result<u64, AsError> {
        // Find the VA region allocator
        let region_idx = self.region_allocators.iter().position(|r| r.region == region)
            .ok_or(AsError::InvalidRegion)?;

        // Allocate a VA range
        let va_offset = self.region_allocators[region_idx].allocate(bo.size())?;
        let va_addr = region.base() + va_offset;

        // Update page table entries
        self.update_page_table(va_addr, bo.gpu_addr(), bo.size(), flags)?;

        trace!(
            target: LOG_TARGET,
            "AS {}: mapped BO '{}' at VA={:#x} -> PA={:#x} (size={:#x})",
            self.id.raw(), bo.name(), va_addr, bo.gpu_addr(), bo.size()
        );

        Ok(va_addr)
    }

    /// Unmap a VA range from this address space
    pub fn unmap(&mut self, va_addr: u64, size: u64) -> Result<(), AsError> {
        // Invalidate page table entries
        self.invalidate_page_table(va_addr, size)?;

        // Find the region and free the VA
        for allocator in &mut self.region_allocators {
            let region_base = allocator.region.base();
            let region_end = region_base + allocator.region.size();
            if va_addr >= region_base && va_addr < region_end {
                allocator.free(va_addr - region_base, size);
                break;
            }
        }

        Ok(())
    }

    /// Flush the TLB for this address space
    pub fn flush_tlb(&self) {
        if self.gpu_reg_base.is_null() {
            return;
        }
        unsafe {
            let as_offset = self.id.raw() as u32 * 0x1000;
            reg_write32(
                self.gpu_reg_base,
                MMU_AS_COMMAND_BASE + as_offset,
                MMU_AS_CMD_FLUSH,
            );
        }
    }

    /// Update page table entries for a mapping
    fn update_page_table(&mut self, va: u64, pa: u64, size: u64, flags: PteFlags) -> Result<(), AsError> {
        // Walk the 3-level page table and create/update entries
        // Level 1: PGD (Page Global Directory)
        // Level 2: PMD (Page Middle Directory)
        // Level 3: PTE (Page Table Entry)

        let mut vaddr = va;
        let mut paddr = pa;
        let mut remaining = size;

        while remaining > 0 {
            let _l1_idx = (vaddr >> 39) & 0x1FF;
            let _l2_idx = (vaddr >> 30) & 0x1FF;
            let _l3_idx = (vaddr >> 21) & 0x1FF;
            let _page_offset = vaddr & 0xFFFFF;

            // Create L1 entry if not present
            // Create L2 entry if not present
            // Create L3 entry with flags

            let _pte = (paddr & MMU_ENTRY_ADDR_MASK) | flags.bits();

            // In production, write the PTE to the page table BO
            // For now, this is a simplified model
            trace!(
                target: LOG_TARGET,
                "PTE: VA={:#x} -> PA={:#x} (flags={:?})",
                vaddr, paddr, flags
            );

            let page_size = 0x200000; // 2 MB huge pages
            vaddr += page_size;
            paddr += page_size;
            remaining = remaining.saturating_sub(page_size);
        }

        Ok(())
    }

    /// Invalidate page table entries
    fn invalidate_page_table(&mut self, _va: u64, _size: u64) -> Result<(), AsError> {
        // Zero out the PTEs in the page table
        Ok(())
    }

    /// Activate this address space on the GPU
    pub fn activate(&mut self) {
        if self.gpu_reg_base.is_null() {
            return;
        }
        let as_offset = self.id.raw() as u32 * 0x1000;
        unsafe {
            // Write translation table base address
            reg_write64(
                self.gpu_reg_base,
                MMU_AS_TRANSTAB_LO_BASE + as_offset,
                self.page_table.gpu_addr(),
            );
            // Write memory attributes
            reg_write32(
                self.gpu_reg_base,
                MMU_AS_MEMATTR_LO_BASE + as_offset,
                0xFFFFFFFF, // All cacheable, inner/outer write-back
            );
            // Issue UPDATE command
            reg_write32(
                self.gpu_reg_base,
                MMU_AS_COMMAND_BASE + as_offset,
                MMU_AS_CMD_UPDATE,
            );
        }
        self.active = true;
        debug!(target: LOG_TARGET, "AS {}: activated", self.id.raw());
    }

    /// Check if this address space is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Simple bump allocator for a VA region
struct VaAllocator {
    /// The VA region this allocator manages
    region: VaRegion,
    /// Current allocation offset
    next_offset: u64,
    /// Free list (offset, size pairs)
    free_list: Vec<(u64, u64)>,
}

impl VaAllocator {
    fn new(region: VaRegion) -> Self {
        Self {
            region,
            next_offset: 0,
            free_list: Vec::new(),
        }
    }

    fn allocate(&mut self, size: u64) -> Result<u64, AsError> {
        // Try the free list first (first-fit)
        for i in 0..self.free_list.len() {
            if self.free_list[i].1 >= size {
                let (offset, _block_size) = self.free_list[i];
                self.free_list[i].0 += size;
                self.free_list[i].1 -= size;
                if self.free_list[i].1 == 0 {
                    self.free_list.remove(i);
                }
                return Ok(offset);
            }
        }

        // Bump allocate
        let offset = self.next_offset;
        if offset + size > self.region.size() {
            return Err(AsError::VaSpaceExhausted(self.region));
        }
        self.next_offset += size;
        Ok(offset)
    }

    fn free(&mut self, offset: u64, size: u64) {
        self.free_list.push((offset, size));
    }
}

/// Address space errors
#[derive(Debug, thiserror::Error)]
pub enum AsError {
    /// VA space exhausted
    #[error("Virtual address space exhausted in region {0:?}")]
    VaSpaceExhausted(VaRegion),
    /// Invalid region
    #[error("Invalid VA region")]
    InvalidRegion,
    /// BO error
    #[error("BO error: {0}")]
    BoError(#[from] crate::mem::bo::BoError),
    /// Page table error
    #[error("Page table error: {0}")]
    PageTableError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_va_regions() {
        assert_eq!(VaRegion::ShaderCode.base(), 0);
        assert_eq!(VaRegion::Buffers.base(), 0x0000_2000_0000);
        assert_eq!(VaRegion::Textures.base(), 0x0000_6000_0000);
    }

    #[test]
    fn test_va_region_sizes() {
        assert_eq!(VaRegion::ShaderCode.size(), 4 * 1024 * 1024);
        assert_eq!(VaRegion::Textures.size(), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_pte_flags() {
        let flags = PteFlags::VALID | PteFlags::WRITABLE | PteFlags::INNER_CACHEABLE;
        assert!(flags.contains(PteFlags::VALID));
        assert!(flags.contains(PteFlags::INNER_CACHEABLE));
        assert!(!flags.contains(PteFlags::READ_ONLY));
    }
}
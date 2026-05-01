//! GPU Memory Management Unit (MMU)
//!
//! The Mali-G68 MP5 uses an ARM MMU-600 to manage virtual address spaces.
//! On mobile SoCs with unified memory, the MMU translates GPU virtual
//! addresses to physical addresses using page tables.
//!
//! ## Address Space Layout
//!
//! | Region            | Start              | Size               |
//! |-------------------|--------------------|--------------------|
//! | Low region        | 0x0000_0000_0000   | 256 GB             |
//! | High region       | 0x1000_0000_0000   | 256 GB             |
//! | Shader programs   | 0x0000_0000_0000   | 4 MB               |
//! | Tiler heap        | 0x0000_1000_0000   | 64 MB              |
//! | Buffer objects    | 0x0000_2000_0000   | 1 GB               |
//! | Textures          | 0x0000_6000_0000   | 2 GB               |

pub mod as_;

pub use as_::AddressSpace;

/// Number of address spaces supported by Mali-G68 MMU
pub const NUM_ADDRESS_SPACES: u32 = 16;

/// Page table entry flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PteFlags: u64 {
        /// Page is valid (present)
        const VALID = 1 << 0;
        /// Page is read-only
        const READ_ONLY = 1 << 1;
        /// Page is writable
        const WRITABLE = 1 << 2;
        /// Page has execute permission
        const EXECUTABLE = 1 << 3;
        /// Page is cacheable (inner)
        const INNER_CACHEABLE = 1 << 4;
        /// Page is cacheable (outer)
        const OUTER_CACHEABLE = 1 << 5;
        /// Page is shareable
        const SHAREABLE = 1 << 6;
        /// Page is in protected mode
        const PROTECTED = 1 << 7;
        /// AFBC compressed page
        const AFBC = 1 << 8;
        /// Page has been accessed
        const ACCESSED = 1 << 9;
    }
}

/// Virtual address region identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaRegion {
    /// Shader programs and code
    ShaderCode,
    /// Tiler heap
    TilerHeap,
    /// Buffer objects
    Buffers,
    /// Texture images
    Textures,
    /// Descriptor sets
    Descriptors,
    /// Command buffers
    CommandBuffers,
}

impl VaRegion {
    /// Get the base address for this region
    pub fn base(&self) -> u64 {
        match self {
            VaRegion::ShaderCode => 0x0000_0000_0000,
            VaRegion::TilerHeap => 0x0000_1000_0000,
            VaRegion::Buffers => 0x0000_2000_0000,
            VaRegion::Textures => 0x0000_6000_0000,
            VaRegion::Descriptors => 0x0000_E000_0000,
            VaRegion::CommandBuffers => 0x0000_F000_0000,
        }
    }

    /// Get the size of this region
    pub fn size(&self) -> u64 {
        match self {
            VaRegion::ShaderCode => 4 * 1024 * 1024,          // 4 MB
            VaRegion::TilerHeap => 64 * 1024 * 1024,          // 64 MB
            VaRegion::Buffers => 1024 * 1024 * 1024,          // 1 GB
            VaRegion::Textures => 2 * 1024 * 1024 * 1024,     // 2 GB
            VaRegion::Descriptors => 256 * 1024 * 1024,        // 256 MB
            VaRegion::CommandBuffers => 256 * 1024 * 1024,     // 256 MB
        }
    }
}
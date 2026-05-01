//! Mali-G68 MP5 hardware register definitions (Valhall architecture)
//!
//! These registers define the GPU hardware programming interface for the
//! Valhall-architecture Mali-G68 MP5. Register offsets and bit fields
//! are derived from the ARM Mali GPU Architecture Reference Manual and
//! the Mesa freedreno/panfrost driver reverse-engineering efforts.

use bitflags::bitflags;

// ============================================================================
// GPU Management Registers
// ============================================================================

/// GPU ID register - identifies the GPU product
pub const GPU_ID: u32 = 0x0000;

/// L2_FEATURES register - L2 cache configuration
pub const L2_FEATURES: u32 = 0x0004;

/// CORE_FEATURES register - shader core features
pub const CORE_FEATURES: u32 = 0x0008;

/// MEM_FEATURES register - memory system features
pub const MEM_FEATURES: u32 = 0x000C;

/// MMU_FEATURES register - MMU features
pub const MMU_FEATURES: u32 = 0x0010;

/// TILER_FEATURES register - tiler features
pub const TILER_FEATURES: u32 = 0x0014;

/// TEXTURE_FEATURES(n) registers - texture format support
pub const TEXTURE_FEATURES_0: u32 = 0x0018;
pub const TEXTURE_FEATURES_1: u32 = 0x001C;
pub const TEXTURE_FEATURES_2: u32 = 0x0020;
pub const TEXTURE_FEATURES_3: u32 = 0x0024;

/// SHADER_FEATURES(n) registers - shader feature flags
pub const SHADER_FEATURES_0: u32 = 0x0028;
pub const SHADER_FEATURES_1: u32 = 0x002C;

/// AS_PRESENT register - address spaces present
pub const AS_PRESENT: u32 = 0x0030;

/// JS_PRESENT register - job slots present
pub const JS_PRESENT: u32 = 0x0034;

/// GPU_MAX_FREQ register - maximum GPU frequency
pub const GPU_MAX_FREQ: u32 = 0x0038;

/// GPU_MIN_FREQ register - minimum GPU frequency
pub const GPU_MIN_FREQ: u32 = 0x003C;

/// COHERENCY_FEATURES register - coherency support
pub const COHERENCY_FEATURES: u32 = 0x0040;

/// AFBC_FEATURES register - AFBC compression features
pub const AFBC_FEATURES: u32 = 0x0044;

// ============================================================================
// GPU Control Registers
// ============================================================================

/// GPU_COMMAND register - send commands to GPU
pub const GPU_COMMAND: u32 = 0x0100;

/// GPU_STATUS register - current GPU status
pub const GPU_STATUS: u32 = 0x0104;

/// GPU_FAULT_STATUS register - fault status
pub const GPU_FAULT_STATUS: u32 = 0x010C;

/// GPU_FAULT_ADDRESS_LO register - fault address low bits
pub const GPU_FAULT_ADDRESS_LO: u32 = 0x0110;

/// GPU_FAULT_ADDRESS_HI register - fault address high bits
pub const GPU_FAULT_ADDRESS_HI: u32 = 0x0114;

/// GPU_IRQ_STATUS register - IRQ status
pub const GPU_IRQ_STATUS: u32 = 0x0120;

/// GPU_IRQ_MASK register - IRQ mask
pub const GPU_IRQ_MASK: u32 = 0x0124;

/// GPU_IRQ_CLEAR register - IRQ clear
pub const GPU_IRQ_CLEAR: u32 = 0x0128;

/// GPU_COMMAND_RESET - Reset the GPU
pub const GPU_CMD_RESET: u32 = 0x00000001;

/// GPU_COMMAND_START - Start the GPU
pub const GPU_CMD_START: u32 = 0x00000002;

/// GPU_COMMAND_STOP - Stop the GPU
pub const GPU_CMD_STOP: u32 = 0x00000004;

/// GPU_COMMAND_SOFT_RESET - Soft reset (CSF only)
pub const GPU_CMD_SOFT_RESET: u32 = 0x00000008;

/// GPU_COMMAND_HARD_RESET - Hard reset
pub const GPU_CMD_HARD_RESET: u32 = 0x00000010;

// ============================================================================
// CSF (Command Stream Frontend) Registers
// ============================================================================

/// CSF_CSR_BASE register - CSF control status register base
pub const CSF_CSR_BASE: u32 = 0x1000;

/// CSF_CSR_QUEUES register - number of CSF queues
pub const CSF_NUM_QUEUES: u32 = 0x1004;

/// CSF_DOORBELL register - CSF doorbell register
pub const CSF_DOORBELL: u32 = 0x1008;

/// CSF_INTERRUPT_CLEAR register - CSF interrupt clear
pub const CSF_INTERRUPT_CLEAR: u32 = 0x100C;

/// CSF_FIRMWARE_STATUS register - CSF firmware status
pub const CSF_FIRMWARE_STATUS: u32 = 0x1010;

/// CSF_FIRMWARE_INPUT_BASE register - CSF firmware input base
pub const CSF_FIRMWARE_INPUT_BASE: u32 = 0x1014;

/// CSF_FIRMWARE_OUTPUT_BASE register - CSF firmware output base
pub const CSF_FIRMWARE_OUTPUT_BASE: u32 = 0x1018;

/// CSF_GROUP_CONTROL(n) - CSF group control registers
pub const CSF_GROUP_CONTROL_BASE: u32 = 0x2000;

/// CSF_QUEUE_CONTROL(n) - CSF queue control registers
pub const CSF_QUEUE_CONTROL_BASE: u32 = 0x3000;

// ============================================================================
// L2 Cache Registers
// ============================================================================

/// L2_CONTROL register - L2 cache control
pub const L2_CONTROL: u32 = 0x0200;

/// L2_STATUS register - L2 cache status
pub const L2_STATUS: u32 = 0x0204;

/// L2_MAINT_CONTROL register - L2 maintenance control
pub const L2_MAINT_CONTROL: u32 = 0x0208;

/// L2_MAINT_STATUS register - L2 maintenance status
pub const L2_MAINT_STATUS: u32 = 0x020C;

/// L2_INVALIDATE register - L2 invalidate command
pub const L2_INVALIDATE: u32 = 0x0210;

// ============================================================================
// MMU Registers
// ============================================================================

/// MMU_IRQ_STATUS register - MMU interrupt status
pub const MMU_IRQ_STATUS: u32 = 0x0300;

/// MMU_IRQ_MASK register - MMU interrupt mask
pub const MMU_IRQ_MASK: u32 = 0x0304;

/// MMU_IRQ_CLEAR register - MMU interrupt clear
pub const MMU_IRQ_CLEAR: u32 = 0x0308;

/// MMU_AS(n)_TRANSTAB_LO register - Address space translation table low
pub const MMU_AS_TRANSTAB_LO_BASE: u32 = 0x0400;
pub const MMU_AS_TRANSTAB_HI_BASE: u32 = 0x0404;
pub const MMU_AS_MEMATTR_LO_BASE: u32 = 0x0408;
pub const MMU_AS_MEMATTR_HI_BASE: u32 = 0x040C;
pub const MMU_AS_COMMAND_BASE: u32 = 0x0410;
pub const MMU_AS_FAULTSTATUS_BASE: u32 = 0x0414;
pub const MMU_AS_FAULTADDRESS_LO_BASE: u32 = 0x0418;
pub const MMU_AS_FAULTADDRESS_HI_BASE: u32 = 0x041C;

/// MMU AS command: update address space
pub const MMU_AS_CMD_UPDATE: u32 = 0x01;

/// MMU AS command: lock address space
pub const MMU_AS_CMD_LOCK: u32 = 0x02;

/// MMU AS command: unlock address space
pub const MMU_AS_CMD_UNLOCK: u32 = 0x03;

/// MMU AS command: flush address space
pub const MMU_AS_CMD_FLUSH: u32 = 0x04;

/// MMU page entry flags
pub const MMU_ENTRY_FLAGS_MASK: u64 = 0xFFFF000000000FFF;

/// MMU page address mask
pub const MMU_ENTRY_ADDR_MASK: u64 = !MMU_ENTRY_FLAGS_MASK;

/// MMU page size: 4KB
pub const MMU_PAGE_SIZE: u32 = 4096;

/// MMU page size: 64KB
pub const MMU_PAGE_SIZE_64KB: u32 = 65536;

// ============================================================================
// Shader Core Registers
// ============================================================================

/// SHADER_CORE_PRESENT register - which shader cores are present
pub const SHADER_CORE_PRESENT: u32 = 0x0500;

/// SHADER_CORE_READY register - which shader cores are ready
pub const SHADER_CORE_READY: u32 = 0x0504;

/// SHADER_PWRON_LO register - shader power on low bits
pub const SHADER_PWRON_LO: u32 = 0x0510;

/// SHADER_PWROFF_LO register - shader power off low bits
pub const SHADER_PWROFF_LO: u32 = 0x0514;

// ============================================================================
// Tiler Registers
// ============================================================================

/// TILER_PRESENT register - tiler present mask
pub const TILER_PRESENT: u32 = 0x0600;

/// TILER_READY register - tiler ready mask
pub const TILER_READY: u32 = 0x0604;

/// TILER_FEATURES register - tiler features
pub const TILER_FEATURES_REG: u32 = 0x0608;

// ============================================================================
// Power Management Registers
// ============================================================================

/// PWROFF_LO register - power off low bits
pub const PWROFF_LO: u32 = 0x0700;

/// PWROFF_HI register - power off high bits
pub const PWROFF_HI: u32 = 0x0704;

/// PWRON_LO register - power on low bits
pub const PWRON_LO: u32 = 0x0708;

/// PWRON_HI register - power on high bits
pub const PWRON_HI: u32 = 0x070C;

/// PWRTRANS_LO register - power transition low bits
pub const PWRTRANS_LO: u32 = 0x0710;

/// PWRTRANS_HI register - power transition high bits
pub const PWRTRANS_HI: u32 = 0x0714;

// ============================================================================
// Bit Field Definitions
// ============================================================================

bitflags! {
    /// GPU status flags from GPU_STATUS register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GpuStatusFlags: u32 {
        /// GPU is powered on and active
        const GPU_ACTIVE = 1 << 0;
        /// GPU is in powered-off state
        const GPU_POWERED_OFF = 1 << 1;
        /// GPU is in protected mode
        const GPU_PROTECTED = 1 << 2;
        /// Fault occurred
        const GPU_FAULT_OCCURRED = 1 << 3;
        /// Reset is in progress
        const GPU_RESET_IN_PROGRESS = 1 << 4;
        /// Power transition in progress
        const GPU_PWR_TRANS_IN_PROGRESS = 1 << 5;
        /// CSF is in CSG mode
        const GPU_CSF_IN_CSG_MODE = 1 << 6;
    }
}

bitflags! {
    /// GPU IRQ status flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GpuIrqFlags: u32 {
        /// GPU interrupt
        const GPU_IRQ = 1 << 0;
        /// Uncorrectable ECC error
        const ECC_UNCORRECTABLE = 1 << 1;
        /// Correctable ECC error
        const ECC_CORRECTABLE = 1 << 2;
        /// Power management interrupt
        const POWERMGMT = 1 << 3;
        /// Profiling interrupt
        const PROFILING = 1 << 4;
        /// CSF firmware interrupt
        const CSF_FIRMWARE = 1 << 5;
        /// CSF queue interrupt
        const CSF_QUEUE = 1 << 6;
        /// Software interrupt from firmware
        const CSF_SW = 1 << 7;
    }
}

bitflags! {
    /// L2 cache features
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct L2Features: u32 {
        /// L2 cache line size in log2
        const LINE_SIZE_LOG2_MASK = 0xF;
        /// L2 cache line size shift
        const LINE_SIZE_LOG2_SHIFT = 0;
        /// L2 cache associativity in log2
        const ASSOC_LOG2_MASK = 0xF;
        const ASSOC_LOG2_SHIFT = 4;
        /// L2 cache size in log2
        const SIZE_LOG2_MASK = 0xFF;
        const SIZE_LOG2_SHIFT = 8;
        /// External cache support
        const EXTERNAL_CACHE = 1 << 16;
        /// Writeback support
        const WRITEBACK = 1 << 17;
    }
}

bitflags! {
    /// MMU features from MMU_FEATURES register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MmuFeatures: u32 {
        /// Has a valid translation table
        const VALID = 1 << 0;
        /// 8-bit address space IDs
        const ASID_8BIT = 1 << 1;
        /// 16-bit address space IDs
        const ASID_16BIT = 1 << 2;
        /// 64KB page support
        const PAGE_64KB = 1 << 3;
        /// 32-bit virtual address space
        const VA_32BIT = 1 << 4;
        /// 48-bit virtual address space
        const VA_48BIT = 1 << 5;
        /// Host-based translation
        const HOST_VA = 1 << 6;
    }
}

bitflags! {
    /// AFBC compression features
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AfbcFeatures: u32 {
        /// AFBC v1.0 support
        const V1_0 = 1 << 0;
        /// AFBC v1.1 support (YUV transform)
        const V1_1 = 1 << 1;
        /// AFBC v1.2 support (wide block)
        const V1_2 = 1 << 2;
        /// AFBC v1.3 support (lossless + wide block)
        const V1_3 = 1 << 3;
        /// AFBC split block support
        const SPLIT_BLOCK = 1 << 4;
        /// AFBC wide block support
        const WIDE_BLOCK = 1 << 5;
        /// AFBC lossless encoding
        const LOSSLESS = 1 << 6;
        /// AFBC Tiled header support
        const TILED_HEADER = 1 << 7;
    }
}

bitflags! {
    /// Memory features from MEM_FEATURES register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemFeatures: u32 {
        /// Physical memory group count
        const GROUP_COUNT_MASK = 0xFF;
        /// Physical memory group count shift
        const GROUP_COUNT_SHIFT = 0;
        /// Virtual address space size
        const VA_SIZE_MASK = 0xF;
        const VA_SIZE_SHIFT = 8;
    }
}

bitflags! {
    /// CSF firmware status flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CsfFirmwareStatus: u32 {
        /// Firmware is loaded and running
        const FW_RUNNING = 1 << 0;
        /// Firmware interface is initialized
        const FW_INTERFACE_INIT = 1 << 1;
        /// Firmware is ready to accept commands
        const FW_READY = 1 << 2;
        /// Firmware encountered an error
        const FW_ERROR = 1 << 3;
        /// Firmware supports protected mode
        const FW_PROTECTED_MODE = 1 << 4;
    }
}

// ============================================================================
// Register Access Helpers
// ============================================================================

/// Read a 32-bit GPU register
///
/// # Safety
/// Caller must ensure the register offset is valid and the GPU is mapped.
#[inline]
pub unsafe fn reg_read32(base: *mut u32, offset: u32) -> u32 {
    unsafe { core::ptr::read_volatile(base.add(offset as usize / 4)) }
}

/// Write a 32-bit GPU register
///
/// # Safety
/// Caller must ensure the register offset is valid and the GPU is mapped.
#[inline]
pub unsafe fn reg_write32(base: *mut u32, offset: u32, value: u32) {
    unsafe { core::ptr::write_volatile(base.add(offset as usize / 4), value) }
}

/// Read a 64-bit GPU register pair
///
/// # Safety
/// Caller must ensure the register offset is valid and the GPU is mapped.
#[inline]
pub unsafe fn reg_read64(base: *mut u32, offset: u32) -> u64 {
    let lo = unsafe { reg_read32(base, offset) } as u64;
    let hi = unsafe { reg_read32(base, offset + 4) } as u64;
    (hi << 32) | lo
}

/// Write a 64-bit GPU register pair
///
/// # Safety
/// Caller must ensure the register offset is valid and the GPU is mapped.
#[inline]
pub unsafe fn reg_write64(base: *mut u32, offset: u32, value: u64) {
    unsafe { reg_write32(base, offset, value as u32) };
    unsafe { reg_write32(base, offset + 4, (value >> 32) as u32) };
}

/// Extract a bit field from a register value
#[inline]
pub const fn reg_field(value: u32, shift: u32, width: u32) -> u32 {
    (value >> shift) & ((1 << width) - 1)
}

/// Insert a bit field into a register value
#[inline]
pub const fn reg_field_set(value: u32, field: u32, shift: u32, width: u32) -> u32 {
    let mask = ((1u32 << width) - 1) << shift;
    (value & !mask) | ((field << shift) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reg_field_extraction() {
        let val = 0b1111_0000_0000_0000_0000_0000_0000_1111u32;
        assert_eq!(reg_field(val, 0, 4), 0b1111);
        assert_eq!(reg_field(val, 28, 4), 0b1111);
    }

    #[test]
    fn test_reg_field_insertion() {
        let val = 0u32;
        let result = reg_field_set(val, 0b1010, 8, 4);
        assert_eq!(result, 0b1010_0000_0000);
    }

    #[test]
    fn test_gpu_status_flags() {
        let flags = GpuStatusFlags::GPU_ACTIVE | GpuStatusFlags::GPU_CSF_IN_CSG_MODE;
        assert!(flags.contains(GpuStatusFlags::GPU_ACTIVE));
        assert!(flags.contains(GpuStatusFlags::GPU_CSF_IN_CSG_MODE));
        assert!(!flags.contains(GpuStatusFlags::GPU_FAULT_OCCURRED));
    }

    #[test]
    fn test_register_offsets() {
        assert_eq!(GPU_ID, 0x0000);
        assert_eq!(GPU_COMMAND, 0x0100);
        assert_eq!(L2_CONTROL, 0x0200);
        assert_eq!(MMU_IRQ_STATUS, 0x0300);
        assert_eq!(CSF_CSR_BASE, 0x1000);
    }
}
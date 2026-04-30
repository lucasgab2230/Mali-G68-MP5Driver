//! CSF Firmware Interface
//!
//! The CSF firmware runs on the GPU's embedded microcontroller and manages
//! the command stream processing. The host driver communicates with the
//! firmware through shared memory structures.
//!
//! ## Firmware Interface
//!
//! The firmware interface consists of:
//! - **Global Interface**: Shared between all groups/queues
//! - **Group Interface**: Per-group configuration and status
//! - **Queue Interface**: Per-queue ring buffer management
//!
//! The firmware is loaded by the kernel DRM driver during GPU initialization.

use crate::gpu::regs::*;
use crate::LOG_TARGET;
use log::{info, warn};

/// CSF firmware version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsfFirmwareVersion {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
}

impl std::fmt::Display for CsfFirmwareVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// CSF firmware interface structure (shared with GPU)
///
/// This structure is laid out in shared memory and must match the
/// firmware's expected layout exactly.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CsfFirmwareInterface {
    /// Magic number identifying the interface version
    pub magic: u32,
    /// Interface version
    pub version: u32,
    /// Number of supported groups
    pub num_groups: u32,
    /// Number of supported queues per group
    pub num_queues_per_group: u32,
    /// Total number of supported queues
    pub total_queues: u32,
    /// Firmware features bitmask
    pub features: u32,
    /// Global MMU AS count
    pub num_as: u32,
    /// Reserved
    _reserved: [u32; 56],
}

/// CSF firmware magic number
pub const CSF_FIRMWARE_MAGIC: u32 = 0x43534631; // "CSF1"

/// CSF firmware features
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CsfFirmwareFeatures: u32 {
        /// Supports protected mode
        const PROTECTED_MODE = 1 << 0;
        /// Supports idle throttling
        const IDLE_THROTTLING = 1 << 1;
        /// Supports power management
        const POWER_MANAGEMENT = 1 << 2;
        /// Supports auto group priority
        const AUTO_GROUP_PRIORITY = 1 << 3;
        /// Supports queue submission checks
        const QUEUE_SUBMISSION_CHECKS = 1 << 4;
        /// Supports memory pressure hints
        const MEMORY_PRESSURE = 1 << 5;
    }
}

/// CSF firmware management
pub struct CsfFirmware {
    /// Firmware version (if detected)
    version: Option<CsfFirmwareVersion>,
    /// Firmware features
    features: CsfFirmwareFeatures,
    /// Whether the firmware is loaded and running
    is_running: bool,
    /// Number of groups supported
    num_groups: u32,
    /// Number of queues per group
    num_queues_per_group: u32,
    /// GPU register base address (for doorbell writes, etc.)
    gpu_reg_base: Option<*mut u32>,
}

// The pointer field makes it !Send, but we manage access safely
unsafe impl Send for CsfFirmware {}

impl CsfFirmware {
    /// Create a new CSF firmware handle (not yet initialized)
    pub fn new() -> Self {
        Self {
            version: None,
            features: CsfFirmwareFeatures::empty(),
            is_running: false,
            num_groups: 1,
            num_queues_per_group: 3, // Graphics, Compute, Transfer
            gpu_reg_base: None,
        }
    }

    /// Initialize the CSF firmware interface
    ///
    /// This reads the firmware status and capabilities from the GPU registers.
    pub fn init(&mut self) -> Result<(), FirmwareError> {
        info!(target: LOG_TARGET, "Initializing CSF firmware interface...");

        // In production, this reads the CSF firmware interface structure
        // from shared memory that the kernel driver has set up.
        // For now, we set up the defaults for Mali-G68 MP5.

        self.version = Some(CsfFirmwareVersion {
            major: 1,
            minor: 0,
            patch: 0,
        });

        self.features = CsfFirmwareFeatures::PROTECTED_MODE
            | CsfFirmwareFeatures::POWER_MANAGEMENT
            | CsfFirmwareFeatures::AUTO_GROUP_PRIORITY
            | CsfFirmwareFeatures::QUEUE_SUBMISSION_CHECKS;

        self.is_running = true;

        info!(
            target: LOG_TARGET,
            "CSF firmware initialized: v{} ({} groups, {} queues/group)",
            self.version.as_ref().unwrap(),
            self.num_groups,
            self.num_queues_per_group,
        );
        info!(target: LOG_TARGET, "  Features: {:?}", self.features);

        Ok(())
    }

    /// Get the firmware version
    pub fn version(&self) -> Option<&CsfFirmwareVersion> {
        self.version.as_ref()
    }

    /// Get the firmware features
    pub fn features(&self) -> CsfFirmwareFeatures {
        self.features
    }

    /// Check if the firmware is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the number of supported groups
    pub fn num_groups(&self) -> u32 {
        self.num_groups
    }

    /// Get the number of queues per group
    pub fn num_queues_per_group(&self) -> u32 {
        self.num_queues_per_group
    }

    /// Get the total number of queues
    pub fn total_queues(&self) -> u32 {
        self.num_groups * self.num_queues_per_group
    }

    /// Set the GPU register base address
    pub fn set_gpu_reg_base(&mut self, base: *mut u32) {
        self.gpu_reg_base = Some(base);
    }

    /// Ring the CSF doorbell for a specific queue
    ///
    /// # Safety
    /// Caller must ensure the GPU register mapping is valid.
    pub unsafe fn ring_doorbell(&self, queue_idx: u32, seq_num: u32) {
        if let Some(base) = self.gpu_reg_base {
            let doorbell_val = (queue_idx << 16) | (seq_num & 0xFFFF);
            unsafe { reg_write32(base, CSF_DOORBELL, doorbell_val) };
        }
    }

    /// Reset the firmware (after GPU reset)
    pub fn reset(&mut self) {
        self.is_running = false;
        self.version = None;
        self.features = CsfFirmwareFeatures::empty();
        warn!(target: LOG_TARGET, "CSF firmware reset");
    }

    /// Check if a feature is supported
    pub fn has_feature(&self, feature: CsfFirmwareFeatures) -> bool {
        self.features.contains(feature)
    }
}

impl Default for CsfFirmware {
    fn default() -> Self {
        Self::new()
    }
}

/// Firmware errors
#[derive(Debug, thiserror::Error)]
pub enum FirmwareError {
    /// Firmware not found
    #[error("CSF firmware not found")]
    NotFound,
    /// Firmware version mismatch
    #[error("CSF firmware version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
    /// Firmware load failed
    #[error("CSF firmware load failed: {0}")]
    LoadFailed(String),
    /// Firmware timeout
    #[error("CSF firmware timeout: {0}")]
    Timeout(String),
    /// Interface initialization failed
    #[error("CSF interface init failed: {0}")]
    InterfaceInitFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firmware_init() {
        let mut fw = CsfFirmware::new();
        assert!(!fw.is_running());
        fw.init().unwrap();
        assert!(fw.is_running());
        assert_eq!(fw.total_queues(), 3); // 1 group × 3 queues
    }

    #[test]
    fn test_firmware_features() {
        let mut fw = CsfFirmware::new();
        fw.init().unwrap();
        assert!(fw.has_feature(CsfFirmwareFeatures::POWER_MANAGEMENT));
        assert!(!fw.has_feature(CsfFirmwareFeatures::MEMORY_PRESSURE));
    }

    #[test]
    fn test_firmware_version_display() {
        let v = CsfFirmwareVersion { major: 1, minor: 2, patch: 3 };
        assert_eq!(format!("{}", v), "1.2.3");
    }
}
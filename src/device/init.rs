//! Device initialization and management
//!
//! Handles GPU device discovery, initialization, and teardown.

use crate::csf::firmware::CsfFirmware;
use crate::csf::queue::{CsfQueue, QueueType, QueuePriority};
use crate::gpu::info::{GpuInfo, GpuDetectError};
use crate::mem::pool::PoolManager;
use crate::mmu::as_::AddressSpace;
use crate::emulator::cache::PipelineCache;
use crate::emulator::async_compute::AsyncComputeManager;
use crate::DriverConfig;
use crate::LOG_TARGET;
use log::info;
use std::os::unix::io::RawFd;

/// GPU Device - represents an initialized Mali-G68 MP5
pub struct MaliDevice {
    /// GPU information
    pub gpu_info: GpuInfo,
    /// DRM file descriptor
    drm_fd: RawFd,
    /// CSF firmware interface
    pub firmware: CsfFirmware,
    /// CSF queues
    pub queues: Vec<CsfQueue>,
    /// Memory pool manager
    pub pool_manager: PoolManager,
    /// Address space
    pub address_space: Option<AddressSpace>,
    /// Pipeline cache
    pub pipeline_cache: PipelineCache,
    /// Async compute manager
    pub async_compute: AsyncComputeManager,
    /// Driver configuration
    pub config: DriverConfig,
    /// GPU register base address (MMIO)
    gpu_reg_base: *mut u32,
    /// Whether the device is initialized
    initialized: bool,
}

// Raw pointer makes it !Send by default, but we manage access safely
unsafe impl Send for MaliDevice {}
unsafe impl Sync for MaliDevice {}

impl MaliDevice {
    /// Create and initialize a new Mali device
    pub fn new(config: DriverConfig) -> Result<Self, DeviceError> {
        info!(target: LOG_TARGET, "Initializing Mali-G68 MP5 device...");

        // 1. Open DRM device
        let drm_fd = Self::open_drm_device()?;

        // 2. Detect GPU
        let gpu_info = GpuInfo::detect_from_drm(drm_fd)?;

        // 3. Initialize CSF firmware
        let mut firmware = CsfFirmware::new();
        firmware.init()?;

        // 4. Create CSF queues
        let queues = Self::create_queues(&firmware);

        // 5. Initialize memory pools
        let pool_manager = PoolManager::new(drm_fd);

        // 6. Create address space
        let address_space = AddressSpace::new(0, drm_fd, std::ptr::null_mut())
            .ok();

        info!(target: LOG_TARGET, "Mali-G68 MP5 device initialized successfully");
        info!(target: LOG_TARGET, "  {} shader cores, {} MB L2", gpu_info.num_shader_cores, gpu_info.l2_cache_size / (1024 * 1024));
        info!(target: LOG_TARGET, "  {} queues, {} memory pools", queues.len(), 7);

        Ok(Self {
            gpu_info,
            drm_fd,
            firmware,
            queues,
            pool_manager,
            address_space,
            pipeline_cache: PipelineCache::new(),
            async_compute: AsyncComputeManager::new(),
            config,
            gpu_reg_base: std::ptr::null_mut(),
            initialized: true,
        })
    }

    /// Open the DRM device node
    fn open_drm_device() -> Result<RawFd, DeviceError> {
        // Try to open DRM render nodes (no auth needed)
        let drm_nodes = [
            "/dev/dri/renderD128",
            "/dev/dri/renderD129",
            "/dev/dri/card0",
            "/dev/dri/card1",
        ];

        for node in &drm_nodes {
            let fd = unsafe {
                libc::open(
                    node.as_ptr() as *const libc::c_char,
                    libc::O_RDWR | libc::O_CLOEXEC,
                )
            };
            if fd >= 0 {
                info!(target: LOG_TARGET, "Opened DRM device: {} (fd={})", node, fd);
                return Ok(fd);
            }
        }

        Err(DeviceError::DrmOpenFailed(
            "No Mali DRM device found".to_string(),
        ))
    }

    /// Create CSF queues for the device
    fn create_queues(_firmware: &CsfFirmware) -> Vec<CsfQueue> {
        let queue_configs = [
            (QueueType::Graphics, QueuePriority::High),
            (QueueType::Compute, QueuePriority::Medium),
            (QueueType::Transfer, QueuePriority::Low),
        ];

        queue_configs
            .iter()
            .enumerate()
            .map(|(i, (qtype, priority))| {
                CsfQueue::new(i as u32, 0, *qtype, *priority)
            })
            .collect()
    }

    /// Get the graphics queue
    pub fn graphics_queue(&self) -> Option<&CsfQueue> {
        self.queues.iter().find(|q| q.queue_type() == QueueType::Graphics)
    }

    /// Get the compute queue
    pub fn compute_queue(&self) -> Option<&CsfQueue> {
        self.queues.iter().find(|q| q.queue_type() == QueueType::Compute)
    }

    /// Get the transfer queue
    pub fn transfer_queue(&self) -> Option<&CsfQueue> {
        self.queues.iter().find(|q| q.queue_type() == QueueType::Transfer)
    }

    /// Check if the device is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the DRM file descriptor
    pub fn drm_fd(&self) -> RawFd {
        self.drm_fd
    }
}

impl Drop for MaliDevice {
    fn drop(&mut self) {
        if self.drm_fd >= 0 {
            unsafe {
                libc::close(self.drm_fd);
            }
        }
        info!(target: LOG_TARGET, "Mali-G68 MP5 device closed");
    }
}

/// Device initialization helper
pub struct DeviceInit;

impl DeviceInit {
    /// Create a device with default configuration
    pub fn create_default() -> Result<MaliDevice, DeviceError> {
        MaliDevice::new(DriverConfig::default())
    }

    /// Create a device optimized for emulator workloads
    pub fn create_emulator_optimized() -> Result<MaliDevice, DeviceError> {
        MaliDevice::new(DriverConfig::emulator_optimized())
    }

    /// Create a device with custom configuration
    pub fn create_with_config(config: DriverConfig) -> Result<MaliDevice, DeviceError> {
        MaliDevice::new(config)
    }
}

/// Device errors
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// DRM device open failed
    #[error("DRM open failed: {0}")]
    DrmOpenFailed(String),
    /// GPU detection failed
    #[error("GPU detection failed: {0}")]
    GpuDetectFailed(#[from] GpuDetectError),
    /// CSF firmware init failed
    #[error("CSF firmware init failed: {0}")]
    FirmwareInitFailed(#[from] crate::csf::firmware::FirmwareError),
    /// Address space creation failed
    #[error("Address space creation failed: {0}")]
    AddressSpaceFailed(#[from] crate::mmu::as_::AsError),
    /// Memory allocation failed
    #[error("Memory allocation failed: {0}")]
    MemoryAllocFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_config_default() {
        let config = DriverConfig::default();
        assert_eq!(config.emulator_opt_level, 3);
    }

    #[test]
    fn test_device_config_emulator() {
        let config = DriverConfig::emulator_optimized();
        assert_eq!(config.emulator_opt_level, 3);
    }
}
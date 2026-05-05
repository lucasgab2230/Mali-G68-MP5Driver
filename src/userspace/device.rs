//! User-space Mali-G68 device management
//!
//! Handles GPU device initialization and communication through DRM
//! without requiring root privileges or kernel drivers.

use crate::drm::{find_mali_drm_device, DrmDeviceManager};
use crate::gpu::DeviceInfo;
use crate::userspace::{UserSpaceConfig, UserSpaceError, UserSpaceResult};
use crate::LOG_TARGET;
use log::{debug, info, warn};
use nix::fcntl::{fcntl, OFlag};
use nix::unistd::close;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;

/// User-space GPU device wrapper
pub struct UserSpaceDevice {
    /// DRM device manager
    drm_manager: DrmDeviceManager,
    /// Device information
    device_info: crate::gpu::info::GpuInfo,
}

impl UserSpaceDevice {
    /// Create new user-space device
    pub fn new(_config: &UserSpaceConfig) -> UserSpaceResult<Self> {
        info!(target: LOG_TARGET, "Initializing user-space Mali-G68 device");

        // Find Mali DRM device
        let drm_path = find_mali_drm_device().map_err(|e| {
            UserSpaceError::DeviceNotFound(format!("Failed to find Mali DRM device: {}", e))
        })?;

        // Initialize DRM device manager
        let drm_manager = DrmDeviceManager::new(&drm_path).map_err(|e| {
            UserSpaceError::DeviceNotFound(format!("Failed to initialize DRM device: {}", e))
        })?;

        // Verify it's a Mali GPU
        if !drm_manager.is_mali_gpu() {
            return Err(UserSpaceError::DeviceNotFound(
                "Device is not a Mali GPU".to_string(),
            ));
        }

        // Get GPU information
        let device_info = crate::gpu::info::GpuInfo::detect_from_drm(drm_manager.get_fd())
            .map_err(|e| UserSpaceError::GpuInitFailed(format!("Failed to detect GPU: {}", e)))?;

        info!(target: LOG_TARGET, "User-space Mali-G68 device initialized");
        debug!(target: LOG_TARGET, "GPU: {}", device_info.device_name());

        Ok(Self {
            drm_manager,
            device_info,
        })
    }

    /// Find available DRM device
    fn find_drm_device(config: &UserSpaceConfig) -> UserSpaceResult<std::path::PathBuf> {
        // Use configured path if provided
        if let Some(ref path) = config.drm_device_path {
            let path = Path::new(path);
            if path.exists() {
                return Ok(path.to_path_buf());
            }
            return Err(UserSpaceError::DeviceNotFound(format!(
                "Configured DRM device not found: {}",
                path.display()
            )));
        }

        // Auto-detect DRM device
        let drm_dir = Path::new("/dev/dri");
        if !drm_dir.exists() {
            return Err(UserSpaceError::DeviceNotFound(
                "/dev/dri directory not found".to_string(),
            ));
        }

        // Look for Mali DRM device
        let entries = std::fs::read_dir(drm_dir)
            .map_err(|e| UserSpaceError::DeviceNotFound(format!("Cannot read /dev/dri: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                UserSpaceError::DeviceNotFound(format!("Cannot read DRM entry: {}", e))
            })?;
            let path = entry.path();

            // Check if it's a character device
            let metadata = std::fs::metadata(&path).map_err(|e| {
                UserSpaceError::DeviceNotFound(format!("Cannot stat {}: {}", path.display(), e))
            })?;

            if !metadata.file_type().is_char_device() {
                continue;
            }

            // Try to open and check if it's Mali
            if let Ok(file) = OpenOptions::new().read(true).open(&path) {
                if Self::is_mali_device(&file) {
                    info!(target: LOG_TARGET, "Found Mali DRM device: {}", path.display());
                    return Ok(path);
                }
            }
        }

        Err(UserSpaceError::DeviceNotFound(
            "No Mali DRM device found".to_string(),
        ))
    }

    /// Check if device is Mali GPU
    fn is_mali_device(file: &File) -> bool {
        // In a real implementation, this would:
        // 1. Query DRM version
        // 2. Check device name
        // 3. Verify Mali-G68 specifically

        // For now, assume any /dev/dri/card* could be Mali
        true
    }

    /// Query device information
    fn query_device_info(fd: std::os::unix::io::RawFd) -> UserSpaceResult<DeviceInfo> {
        // In a real implementation, this would:
        // 1. Use DRM_IOCTL_GET_MAGIC to verify it's DRM
        // 2. Use DRM_IOCTL_VERSION to get driver version
        // 3. Use DRM_IOCTL_GET_CAP to get capabilities
        // 4. Use Mali-specific IOCTLs to get GPU info

        let device_info = DeviceInfo {
            name: "Mali-G68 MP5".to_string(),
            vendor_id: 0x13B5, // ARM
            device_id: 0x7212, // G68 MP5
            driver_version: "0.1.0-optimized".to_string(),
            shader_cores: 5,
            max_frequency_mhz: 800,
            memory_size_mb: 4096, // Typical for phones with G68 MP5
            tiler_bins: 64,
            l2_cache_size_kb: 512,
            features: vec![
                "vulkan_1_3".to_string(),
                "draw_call_batching".to_string(),
                "snapdragon_optimizations".to_string(),
                "user_space_driver".to_string(),
            ],
        };

        debug!(target: LOG_TARGET, "Device info: {:?}", device_info);

        Ok(device_info)
    }

    /// Get device information
    pub fn get_info(&self) -> crate::gpu::info::GpuInfo {
        self.device_info.clone()
    }

    /// Get file descriptor for raw operations
    pub fn get_fd(&self) -> std::os::unix::io::RawFd {
        self.drm_manager.get_fd()
    }

    /// Submit commands to GPU
    pub fn submit_commands(&self, commands: &[u32]) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Submitting {} commands to GPU",
            commands.len()
        );

        // Allocate GPU memory for command buffer
        let cmd_size = (commands.len() * 4) as u64;
        let cmd_handle = self.drm_manager.create_buffer_object(cmd_size, 0)?;

        // Map buffer to write commands
        let cmd_addr = self
            .drm_manager
            .map_buffer_object(cmd_handle, 0, cmd_size)?;

        // Copy commands to GPU memory
        unsafe {
            let cmd_ptr = cmd_addr as *mut u8;
            std::ptr::copy_nonoverlapping(
                commands.as_ptr() as *const u8,
                cmd_ptr,
                cmd_size as usize,
            );
        }

        // Submit command buffer to GPU
        let fence = self
            .drm_manager
            .submit_command_buffer(cmd_addr, commands.len() as u32)?;

        debug!(target: LOG_TARGET, "Commands submitted, fence: {}", fence);

        Ok(())
    }

    /// Wait for GPU idle
    pub fn wait_idle(&self) -> UserSpaceResult<()> {
        debug!(target: LOG_TARGET, "Waiting for GPU idle");

        // Wait for GPU to finish all commands
        self.drm_manager.wait_idle(5000)?; // 5 second timeout

        Ok(())
    }

    /// Get GPU status
    pub fn get_status(&self) -> UserSpaceResult<crate::gpu::info::GpuStatus> {
        // Read GPU status registers via DRM
        // For now, return estimated status
        Ok(crate::gpu::info::GpuStatus {
            idle: true,
            temperature_celsius: 45.0,
            utilization_percent: 0.0,
            memory_usage_mb: 0,
        })
    }

    /// Cleanup device resources
    pub fn cleanup(&self) -> UserSpaceResult<()> {
        info!(target: LOG_TARGET, "Cleaning up user-space device");

        // DRM device will be closed when drm_manager is dropped
        // The File in drm_manager will close the fd automatically

        Ok(())
    }
}

/// GPU status information
#[derive(Debug, Clone)]
pub struct GpuStatus {
    /// Whether GPU is idle
    pub idle: bool,
    /// Current temperature in Celsius
    pub temperature_celsius: f32,
    /// Current utilization percentage
    pub utilization_percent: f32,
    /// Current memory usage in MB
    pub memory_usage_mb: u32,
}

impl Drop for UserSpaceDevice {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

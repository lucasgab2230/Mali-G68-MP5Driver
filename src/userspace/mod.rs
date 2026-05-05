//! User-space Mali-G68 MP5 driver for emulator integration
//!
//! This module provides a user-space driver implementation that can be
//! embedded directly into emulators without requiring root access or
//! system-level driver installation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   Emulator     │    │ User-space      │    │   Mali GPU      │
//! │   (Eden/etc)  │◄──►│ Mali Driver      │◄──►│   Hardware       │
//! │                 │    │ Library        │    │                 │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//! ```
//!
//! ## Features
//!
//! - **No Root Required**: Runs entirely in user-space
//! - **Direct GPU Access**: Uses DRM nodes for hardware access
//! - **Emulator Optimized**: Specific optimizations for emulator workloads
//! - **Drop-in Replacement**: Compatible with existing Vulkan calls
//! - **Memory Management**: Built-in memory pools and allocators
//! - **Performance Monitoring**: Real-time FPS and performance metrics

pub mod context;
pub mod device;
pub mod renderer;
pub mod memory;

pub use context::UserSpaceContext;
pub use device::UserSpaceDevice;
pub use renderer::UserSpaceRenderer;
pub use memory::UserSpaceMemory;

/// User-space driver configuration
#[derive(Debug, Clone)]
pub struct UserSpaceConfig {
    /// Enable performance optimizations
    pub enable_optimizations: bool,
    /// Target FPS for adaptive optimization
    pub target_fps: u32,
    /// Memory pool size in MB
    pub memory_pool_size_mb: u32,
    /// Enable debug logging
    pub enable_debug: bool,
    /// DRM device path (auto-detected if None)
    pub drm_device_path: Option<String>,
}

impl Default for UserSpaceConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            target_fps: 60,
            memory_pool_size_mb: 256, // 256MB memory pool
            enable_debug: false,
            drm_device_path: None, // Auto-detect
        }
    }
}

/// Initialize user-space Mali driver
pub fn init_user_space_driver(config: UserSpaceConfig) -> Result<UserSpaceContext, UserSpaceError> {
    context::UserSpaceContext::new(config)
}

/// Performance metrics for user-space driver
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

/// User-space driver errors
#[derive(Debug, thiserror::Error)]
pub enum UserSpaceError {
    /// DRM device not found
    #[error("DRM device not found: {0}")]
    DeviceNotFound(String),
    
    /// Permission denied (need access to DRM)
    #[error("Permission denied accessing DRM device: {0}")]
    PermissionDenied(String),
    
    /// GPU initialization failed
    #[error("GPU initialization failed: {0}")]
    GpuInitFailed(String),
    
    /// Memory allocation failed
    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),
    
    /// Vulkan initialization failed
    #[error("Vulkan initialization failed: {0}")]
    VulkanInitFailed(String),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<crate::drm::DrmError> for UserSpaceError {
    fn from(err: crate::drm::DrmError) -> Self {
        UserSpaceError::DeviceNotFound(format!("DRM error: {}", err))
    }
}

impl From<crate::mem::pool::PoolError> for UserSpaceError {
    fn from(err: crate::mem::pool::PoolError) -> Self {
        UserSpaceError::MemoryAllocationFailed(format!("Pool error: {}", err))
    }
}

/// Result type for user-space operations
pub type UserSpaceResult<T> = Result<T, UserSpaceError>;

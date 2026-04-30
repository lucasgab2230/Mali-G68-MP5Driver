//! Vulkan Instance implementation
//!
//! The VkInstance is the root of the Vulkan API. It provides:
//! - API version negotiation
//! - Layer and extension enumeration
//! - Physical device discovery

use crate::gpu::info::GpuInfo;
use crate::LOG_TARGET;
use crate::vulkan::physical::VkPhysicalDevice;
use log::{info, warn};

/// Re-export the Vulkan API version from crate root
pub use crate::VULKAN_API_VERSION;

/// Supported instance extensions
pub const INSTANCE_EXTENSIONS: &[&str] = &[
    "VK_KHR_surface",
    "VK_KHR_android_surface",
    "VK_KHR_wayland_surface",
    "VK_KHR_xcb_surface",
    "VK_EXT_debug_utils",
    "VK_EXT_debug_report",
];

/// Supported instance layers
pub const INSTANCE_LAYERS: &[&str] = &[
    "VK_LAYER_KHRONOS_validation",
];

/// Vulkan Instance
pub struct VkInstance {
    /// Application info
    app_info: VkApplicationInfo,
    /// Enabled extensions
    enabled_extensions: Vec<String>,
    /// Enabled layers
    enabled_layers: Vec<String>,
    /// Vulkan API version
    api_version: u32,
    /// Whether debug reporting is enabled
    debug_enabled: bool,
}

/// Application info provided at instance creation
#[derive(Debug, Clone)]
pub struct VkApplicationInfo {
    /// Application name
    pub app_name: String,
    /// Application version
    pub app_version: u32,
    /// Engine name
    pub engine_name: String,
    /// Engine version
    pub engine_version: u32,
    /// Requested API version
    pub api_version: u32,
}

impl Default for VkApplicationInfo {
    fn default() -> Self {
        Self {
            app_name: String::new(),
            app_version: 0,
            engine_name: String::new(),
            engine_version: 0,
            api_version: VULKAN_API_VERSION,
        }
    }
}

/// Instance creation parameters
#[derive(Debug, Clone)]
pub struct VkInstanceCreateInfo {
    /// Application info
    pub app_info: VkApplicationInfo,
    /// Requested extension names
    pub enabled_extensions: Vec<String>,
    /// Requested layer names
    pub enabled_layers: Vec<String>,
    /// Enable debug reporting
    pub debug_enabled: bool,
}

impl VkInstance {
    /// Create a new Vulkan instance
    pub fn create(create_info: &VkInstanceCreateInfo) -> Result<Self, InstanceError> {
        info!(
            target: LOG_TARGET,
            "Creating Vulkan instance (app='{}', api={}.{}.{})",
            create_info.app_info.app_name,
            ash::vk::api_version_major(create_info.app_info.api_version),
            ash::vk::api_version_minor(create_info.app_info.api_version),
            ash::vk::api_version_patch(create_info.app_info.api_version),
        );

        // Validate requested API version
        if create_info.app_info.api_version > VULKAN_API_VERSION {
            warn!(
                target: LOG_TARGET,
                "Requested API version {}.{}.{} exceeds supported {}.{}.{}",
                ash::vk::api_version_major(create_info.app_info.api_version),
                ash::vk::api_version_minor(create_info.app_info.api_version),
                ash::vk::api_version_patch(create_info.app_info.api_version),
                ash::vk::api_version_major(VULKAN_API_VERSION),
                ash::vk::api_version_minor(VULKAN_API_VERSION),
                ash::vk::api_version_patch(VULKAN_API_VERSION),
            );
        }

        // Validate extensions
        for ext in &create_info.enabled_extensions {
            if !INSTANCE_EXTENSIONS.contains(&ext.as_str()) {
                warn!(target: LOG_TARGET, "Unsupported instance extension: {}", ext);
            }
        }

        // Validate layers
        for layer in &create_info.enabled_layers {
            if !INSTANCE_LAYERS.contains(&layer.as_str()) {
                warn!(target: LOG_TARGET, "Unsupported instance layer: {}", layer);
            }
        }

        let api_version = create_info.app_info.api_version.min(VULKAN_API_VERSION);

        Ok(Self {
            app_info: create_info.app_info.clone(),
            enabled_extensions: create_info.enabled_extensions.clone(),
            enabled_layers: create_info.enabled_layers.clone(),
            api_version,
            debug_enabled: create_info.debug_enabled,
        })
    }

    /// Enumerate available physical devices
    pub fn enumerate_physical_devices(&self) -> Result<Vec<VkPhysicalDevice>, InstanceError> {
        // For Mali-G68 MP5, we typically have exactly one GPU
        let gpu_info = GpuInfo::mali_g68_mp5();
        let physical_device = VkPhysicalDevice::new(gpu_info, self.api_version);
        Ok(vec![physical_device])
    }

    /// Get the API version
    pub fn api_version(&self) -> u32 {
        self.api_version
    }

    /// Get the enabled extensions
    pub fn enabled_extensions(&self) -> &[String] {
        &self.enabled_extensions
    }

    /// Check if an extension is enabled
    pub fn is_extension_enabled(&self, name: &str) -> bool {
        self.enabled_extensions.iter().any(|e| e == name)
    }

    /// Destroy the instance
    pub fn destroy(self) {
        info!(target: LOG_TARGET, "Destroying Vulkan instance");
    }
}

/// Instance creation errors
#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    /// Unsupported API version
    #[error("Unsupported Vulkan API version: {}.{}.{}",
        ash::vk::api_version_major(*.0),
        ash::vk::api_version_minor(*.0),
        ash::vk::api_version_patch(*.0))]
    UnsupportedApiVersion(u32),
    /// Extension not supported
    #[error("Instance extension not supported: {0}")]
    ExtensionNotSupported(String),
    /// Layer not supported
    #[error("Instance layer not supported: {0}")]
    LayerNotSupported(String),
    /// Initialization failed
    #[error("Instance initialization failed: {0}")]
    InitFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_creation() {
        let create_info = VkInstanceCreateInfo {
            app_info: VkApplicationInfo::default(),
            enabled_extensions: vec!["VK_KHR_surface".to_string()],
            enabled_layers: vec![],
            debug_enabled: false,
        };
        let instance = VkInstance::create(&create_info).unwrap();
        assert_eq!(instance.api_version(), VULKAN_API_VERSION);
    }

    #[test]
    fn test_instance_extension_check() {
        let create_info = VkInstanceCreateInfo {
            app_info: VkApplicationInfo::default(),
            enabled_extensions: vec!["VK_KHR_surface".to_string()],
            enabled_layers: vec![],
            debug_enabled: false,
        };
        let instance = VkInstance::create(&create_info).unwrap();
        assert!(instance.is_extension_enabled("VK_KHR_surface"));
        assert!(!instance.is_extension_enabled("VK_KHR_wayland_surface"));
    }

    #[test]
    fn test_enumerate_physical_devices() {
        let create_info = VkInstanceCreateInfo {
            app_info: VkApplicationInfo::default(),
            enabled_extensions: vec![],
            enabled_layers: vec![],
            debug_enabled: false,
        };
        let instance = VkInstance::create(&create_info).unwrap();
        let devices = instance.enumerate_physical_devices().unwrap();
        assert_eq!(devices.len(), 1);
    }
}
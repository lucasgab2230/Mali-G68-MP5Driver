//! Vulkan Device implementation
//!
//! Represents a logical GPU device created from a physical device.

use crate::vulkan::physical::VkPhysicalDevice;
use crate::LOG_TARGET;
use log::info;

/// Device creation parameters
#[derive(Debug, Clone)]
pub struct VkDeviceCreateInfo {
    /// Enabled device extensions
    pub enabled_extensions: Vec<String>,
    /// Queue create infos
    pub queue_create_infos: Vec<QueueCreateInfo>,
    /// Enabled features
    pub enabled_features: DeviceFeatures,
}

/// Queue creation parameters
#[derive(Debug, Clone)]
pub struct QueueCreateInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Number of queues to create
    pub queue_count: u32,
    /// Queue priorities (0.0 - 1.0)
    pub priorities: Vec<f32>,
}

/// Device features
#[derive(Debug, Clone, Default)]
pub struct DeviceFeatures {
    /// Enable robust buffer access
    pub robust_buffer_access: bool,
    /// Enable full draw index uint32
    pub full_draw_index_uint32: bool,
    /// Enable image cube array
    pub image_cube_array: bool,
    /// Enable independent blend
    pub independent_blend: bool,
    /// Enable geometry shader
    pub geometry_shader: bool,
    /// Enable tessellation shader
    pub tessellation_shader: bool,
    /// Enable sample rate shading
    pub sample_rate_shading: bool,
    /// Enable dual src blend
    pub dual_src_blend: bool,
    /// Enable logic op
    pub logic_op: bool,
    /// Enable multi draw indirect
    pub multi_draw_indirect: bool,
    /// Enable draw indirect first instance
    pub draw_indirect_first_instance: bool,
    /// Enable depth clamp
    pub depth_clamp: bool,
    /// Enable depth bias clamp
    pub depth_bias_clamp: bool,
    /// Enable fill mode non-solid
    pub fill_mode_non_solid: bool,
    /// Enable depth bounds
    pub depth_bounds: bool,
    /// Enable wide lines
    pub wide_lines: bool,
    /// Enable large points
    pub large_points: bool,
    /// Enable texture compression ASTC LDR
    pub texture_compression_astc_ldr: bool,
    /// Enable shader float16
    pub shader_float16: bool,
    /// Enable shader int8
    pub shader_int8: bool,
    /// Enable shader int16
    pub shader_int16: bool,
    /// Enable dynamic rendering
    pub dynamic_rendering: bool,
}

/// Vulkan logical Device
pub struct VkDevice {
    /// Physical device this was created from
    physical_device: VkPhysicalDevice,
    /// Enabled extensions
    enabled_extensions: Vec<String>,
    /// Enabled features
    enabled_features: DeviceFeatures,
    /// Whether the device is active
    active: bool,
}

impl VkDevice {
    /// Create a new logical device
    pub fn create(physical_device: VkPhysicalDevice, create_info: &VkDeviceCreateInfo) -> Result<Self, DeviceError> {
        info!(target: LOG_TARGET, "Creating VkDevice on '{}'", physical_device.device_name());

        // Validate extensions
        for ext in &create_info.enabled_extensions {
            if !physical_device.is_extension_supported(ext) {
                return Err(DeviceError::UnsupportedExtension(ext.clone()));
            }
        }

        Ok(Self {
            physical_device,
            enabled_extensions: create_info.enabled_extensions.clone(),
            enabled_features: create_info.enabled_features.clone(),
            active: true,
        })
    }

    /// Get the physical device
    pub fn physical_device(&self) -> &VkPhysicalDevice {
        &self.physical_device
    }

    /// Check if an extension is enabled
    pub fn is_extension_enabled(&self, name: &str) -> bool {
        self.enabled_extensions.iter().any(|e| e == name)
    }

    /// Get the enabled features
    pub fn enabled_features(&self) -> &DeviceFeatures {
        &self.enabled_features
    }

    /// Wait for all GPU work to complete
    pub fn wait_idle(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Destroy the device
    pub fn destroy(self) {
        info!(target: LOG_TARGET, "Destroying VkDevice");
    }
}

/// Device errors
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// Unsupported extension
    #[error("Unsupported device extension: {0}")]
    UnsupportedExtension(String),
    /// Initialization failed
    #[error("Device initialization failed: {0}")]
    InitFailed(String),
    /// Device lost
    #[error("Device lost")]
    DeviceLost,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::info::GpuInfo;

    #[test]
    fn test_device_creation() {
        let gpu_info = GpuInfo::mali_g68_mp5();
        let physical = VkPhysicalDevice::new(gpu_info, ash::vk::API_VERSION_1_3);
        let create_info = VkDeviceCreateInfo {
            enabled_extensions: vec!["VK_KHR_swapchain".to_string()],
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index: 0,
                queue_count: 1,
                priorities: vec![1.0],
            }],
            enabled_features: DeviceFeatures::default(),
        };
        let device = VkDevice::create(physical, &create_info).unwrap();
        assert!(device.is_extension_enabled("VK_KHR_swapchain"));
    }
}
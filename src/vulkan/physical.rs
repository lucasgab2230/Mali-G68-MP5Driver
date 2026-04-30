//! Vulkan Physical Device implementation
//!
//! Represents the Mali-G68 MP5 GPU and its capabilities.

use crate::gpu::info::GpuInfo;

/// Supported device extensions
pub const DEVICE_EXTENSIONS: &[&str] = &[
    "VK_KHR_swapchain",
    "VK_KHR_maintenance1",
    "VK_KHR_maintenance2",
    "VK_KHR_maintenance3",
    "VK_KHR_maintenance4",
    "VK_KHR_dedicated_allocation",
    "VK_KHR_descriptor_update_template",
    "VK_KHR_dynamic_rendering",
    "VK_KHR_shader_float16_int8",
    "VK_KHR_shader_subgroup_extended_types",
    "VK_KHR_spirv_1_4",
    "VK_KHR_storage_buffer_storage_class",
    "VK_KHR_variable_pointers",
    "VK_KHR_vulkan_memory_model",
    "VK_EXT_custom_border_color",
    "VK_EXT_descriptor_indexing",
    "VK_EXT_fragment_shader_interlock",
    "VK_EXT_host_query_reset",
    "VK_EXT_inline_uniform_block",
    "VK_EXT_pipeline_creation_cache_control",
    "VK_EXT_scalar_block_layout",
    "VK_EXT_separate_stencil_usage",
    "VK_EXT_shader_stencil_export",
    "VK_EXT_subgroup_size_control",
    "VK_EXT_texture_compression_astc_hdr",
    "VK_ANDROID_external_memory_android_hardware_buffer",
];

/// Memory type indices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTypeIndex {
    /// Device-local (unified memory on mobile - fastest for GPU)
    DeviceLocal = 0,
    /// Host-visible, host-coherent (system RAM, CPU-accessible)
    HostVisible = 1,
    /// Host-cached (cached system RAM, best for readback)
    HostCached = 2,
}

/// Memory heap indices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryHeapIndex {
    /// Device-local heap (unified on mobile)
    DeviceLocal = 0,
    /// Host-visible heap (system RAM)
    HostVisible = 1,
}

/// Vulkan Physical Device - represents the Mali-G68 MP5 GPU
#[derive(Debug, Clone)]
pub struct VkPhysicalDevice {
    /// GPU information
    gpu_info: GpuInfo,
    /// Vulkan API version supported
    api_version: u32,
    /// Device properties
    properties: PhysicalDeviceProperties,
    /// Memory properties
    memory_properties: MemoryProperties,
    /// Queue family properties
    queue_family_properties: Vec<QueueFamilyProperties>,
}

/// Physical device properties
#[derive(Debug, Clone)]
pub struct PhysicalDeviceProperties {
    /// Device name
    pub device_name: String,
    /// Device type
    pub device_type: PhysicalDeviceType,
    /// Vendor ID (ARM = 0x13B5)
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Driver version
    pub driver_version: u32,
    /// API version
    pub api_version: u32,
    /// Max image dimension 2D
    pub max_image_dimension2_d: u32,
    /// Max framebuffer width
    pub max_framebuffer_width: u32,
    /// Max framebuffer height
    pub max_framebuffer_height: u32,
    /// Max framebuffer layers
    pub max_framebuffer_layers: u32,
    /// Max color attachments
    pub max_color_attachments: u32,
    /// Max sampler allocation count
    pub max_sampler_allocation_count: u32,
    /// Max bound descriptor sets
    pub max_bound_descriptor_sets: u32,
    /// Max per-stage descriptor samplers
    pub max_per_stage_descriptor_samplers: u32,
    /// Max per-stage descriptor uniform buffers
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Max per-stage descriptor storage buffers
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Max per-stage descriptor sampled images
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Max per-stage descriptor storage images
    pub max_per_stage_descriptor_storage_images: u32,
    /// Max push constants size
    pub max_push_constants_size: u32,
    /// Max memory allocation count
    pub max_memory_allocation_count: u32,
    /// Subgroup size
    pub subgroup_size: u32,
    /// Timestamp period (ns)
    pub timestamp_period: f32,
}

/// Physical device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalDeviceType {
    /// Integrated GPU (mobile SoC)
    IntegratedGpu,
    /// Discrete GPU
    DiscreteGpu,
    /// Virtual GPU
    VirtualGpu,
    /// CPU (software renderer)
    Cpu,
    /// Unknown
    Other,
}

/// Memory type description
#[derive(Debug, Clone)]
pub struct MemoryTypeDesc {
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

bitflags::bitflags! {
    /// Memory property flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryPropertyFlags: u32 {
        /// Device-local memory
        const DEVICE_LOCAL = 1 << 0;
        /// Host-visible memory
        const HOST_VISIBLE = 1 << 1;
        /// Host-coherent memory
        const HOST_COHERENT = 1 << 2;
        /// Host-cached memory
        const HOST_CACHED = 1 << 3;
        /// Lazily-allocated memory
        const LAZILY_ALLOCATED = 1 << 4;
        /// Protected memory
        const PROTECTED = 1 << 5;
    }
}

/// Memory heap description
#[derive(Debug, Clone)]
pub struct MemoryHeapDesc {
    /// Size in bytes
    pub size: u64,
    /// Flags
    pub flags: MemoryHeapFlags,
}

bitflags::bitflags! {
    /// Memory heap flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryHeapFlags: u32 {
        /// Device-local heap
        const DEVICE_LOCAL = 1 << 0;
    }
}

/// Memory properties
#[derive(Debug, Clone)]
pub struct MemoryProperties {
    /// Memory types
    pub memory_types: Vec<MemoryTypeDesc>,
    /// Memory heaps
    pub memory_heaps: Vec<MemoryHeapDesc>,
}

/// Queue family properties
#[derive(Debug, Clone)]
pub struct QueueFamilyProperties {
    /// Queue family index
    pub index: u32,
    /// Queue flags
    pub queue_flags: QueueFlags,
    /// Queue count
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Min image transfer granularity
    pub min_image_transfer_granularity: [u32; 3],
}

bitflags::bitflags! {
    /// Queue capability flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct QueueFlags: u32 {
        /// Graphics queue
        const GRAPHICS = 1 << 0;
        /// Compute queue
        const COMPUTE = 1 << 1;
        /// Transfer queue
        const TRANSFER = 1 << 2;
        /// Sparse binding queue
        const SPARSE_BINDING = 1 << 3;
        /// Protected queue
        const PROTECTED = 1 << 4;
    }
}

impl VkPhysicalDevice {
    /// Create a new physical device from GPU info
    pub fn new(gpu_info: GpuInfo, api_version: u32) -> Self {
        let device_name = gpu_info.device_name();

        let properties = PhysicalDeviceProperties {
            device_name: device_name.clone(),
            device_type: PhysicalDeviceType::IntegratedGpu,
            vendor_id: 0x13B5, // ARM Holdings
            device_id: 0x9080, // Mali-G68
            driver_version: gpu_info.driver_version_encoded(),
            api_version,
            max_image_dimension2_d: gpu_info.texture_features.max_texel_count_2d,
            max_framebuffer_width: 8192,
            max_framebuffer_height: 8192,
            max_framebuffer_layers: 256,
            max_color_attachments: 8,
            max_sampler_allocation_count: 4000,
            max_bound_descriptor_sets: 8,
            max_per_stage_descriptor_samplers: 16,
            max_per_stage_descriptor_uniform_buffers: 12,
            max_per_stage_descriptor_storage_buffers: 16,
            max_per_stage_descriptor_sampled_images: 128,
            max_per_stage_descriptor_storage_images: 64,
            max_push_constants_size: 256,
            max_memory_allocation_count: 4096,
            subgroup_size: 8, // Valhall W8 wavefront
            timestamp_period: 1.0 / (gpu_info.max_freq_mhz as f32 * 1e6) * 1e9,
        };

        // Memory types for unified memory (mobile SoC)
        let memory_types = vec![
            MemoryTypeDesc {
                property_flags: MemoryPropertyFlags::DEVICE_LOCAL
                    | MemoryPropertyFlags::HOST_VISIBLE
                    | MemoryPropertyFlags::HOST_COHERENT,
                heap_index: 0,
            },
            MemoryTypeDesc {
                property_flags: MemoryPropertyFlags::HOST_VISIBLE
                    | MemoryPropertyFlags::HOST_COHERENT
                    | MemoryPropertyFlags::HOST_CACHED,
                heap_index: 1,
            },
            MemoryTypeDesc {
                property_flags: MemoryPropertyFlags::HOST_VISIBLE
                    | MemoryPropertyFlags::HOST_COHERENT
                    | MemoryPropertyFlags::HOST_CACHED
                    | MemoryPropertyFlags::DEVICE_LOCAL,
                heap_index: 0,
            },
        ];

        let memory_heaps = vec![
            MemoryHeapDesc {
                size: 4 * 1024 * 1024 * 1024, // 4 GB (unified on Exynos 1280)
                flags: MemoryHeapFlags::DEVICE_LOCAL,
            },
            MemoryHeapDesc {
                size: 6 * 1024 * 1024 * 1024, // 6 GB system RAM
                flags: MemoryHeapFlags::empty(),
            },
        ];

        let queue_family_properties = vec![
            QueueFamilyProperties {
                index: 0,
                queue_flags: QueueFlags::GRAPHICS | QueueFlags::COMPUTE | QueueFlags::TRANSFER,
                queue_count: 1,
                timestamp_valid_bits: 64,
                min_image_transfer_granularity: [1, 1, 1],
            },
            QueueFamilyProperties {
                index: 1,
                queue_flags: QueueFlags::COMPUTE | QueueFlags::TRANSFER,
                queue_count: 1,
                timestamp_valid_bits: 64,
                min_image_transfer_granularity: [1, 1, 1],
            },
            QueueFamilyProperties {
                index: 2,
                queue_flags: QueueFlags::TRANSFER,
                queue_count: 1,
                timestamp_valid_bits: 64,
                min_image_transfer_granularity: [1, 1, 1],
            },
        ];

        Self {
            gpu_info,
            api_version,
            properties,
            memory_properties: MemoryProperties {
                memory_types,
                memory_heaps,
            },
            queue_family_properties,
        }
    }

    /// Get the device properties
    pub fn properties(&self) -> &PhysicalDeviceProperties {
        &self.properties
    }

    /// Get the memory properties
    pub fn memory_properties(&self) -> &MemoryProperties {
        &self.memory_properties
    }

    /// Get the queue family properties
    pub fn queue_family_properties(&self) -> &[QueueFamilyProperties] {
        &self.queue_family_properties
    }

    /// Get the GPU info
    pub fn gpu_info(&self) -> &GpuInfo {
        &self.gpu_info
    }

    /// Check if a device extension is supported
    pub fn is_extension_supported(&self, name: &str) -> bool {
        DEVICE_EXTENSIONS.contains(&name)
    }

    /// Get the device name
    pub fn device_name(&self) -> &str {
        &self.properties.device_name
    }

    /// Get the device type
    pub fn device_type(&self) -> PhysicalDeviceType {
        self.properties.device_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physical_device_creation() {
        let gpu_info = GpuInfo::mali_g68_mp5();
        let device = VkPhysicalDevice::new(gpu_info, ash::vk::API_VERSION_1_3);
        assert_eq!(device.device_type(), PhysicalDeviceType::IntegratedGpu);
        assert_eq!(device.properties.vendor_id, 0x13B5);
    }

    #[test]
    fn test_memory_types() {
        let gpu_info = GpuInfo::mali_g68_mp5();
        let device = VkPhysicalDevice::new(gpu_info, ash::vk::API_VERSION_1_3);
        let mem_props = device.memory_properties();
        assert_eq!(mem_props.memory_types.len(), 3);
        assert!(mem_props.memory_types[0].property_flags.contains(MemoryPropertyFlags::DEVICE_LOCAL));
    }

    #[test]
    fn test_queue_families() {
        let gpu_info = GpuInfo::mali_g68_mp5();
        let device = VkPhysicalDevice::new(gpu_info, ash::vk::API_VERSION_1_3);
        let queue_families = device.queue_family_properties();
        assert_eq!(queue_families.len(), 3);
        assert!(queue_families[0].queue_flags.contains(QueueFlags::GRAPHICS));
        assert!(queue_families[1].queue_flags.contains(QueueFlags::COMPUTE));
        assert!(!queue_families[1].queue_flags.contains(QueueFlags::GRAPHICS));
    }

    #[test]
    fn test_extension_support() {
        let gpu_info = GpuInfo::mali_g68_mp5();
        let device = VkPhysicalDevice::new(gpu_info, ash::vk::API_VERSION_1_3);
        assert!(device.is_extension_supported("VK_KHR_swapchain"));
        assert!(device.is_extension_supported("VK_KHR_dynamic_rendering"));
        assert!(!device.is_extension_supported("VK_NV_ray_tracing"));
    }
}
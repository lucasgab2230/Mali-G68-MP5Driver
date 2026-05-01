//! Vulkan descriptor set management

/// Descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    DynamicUniformBuffer,
    DynamicStorageBuffer,
    InputAttachment,
    InlineUniformBlock,
}

/// Descriptor set layout binding
#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutBinding {
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Number of descriptors in this binding
    pub descriptor_count: u32,
    /// Shader stages that can access this binding
    pub stage_flags: ShaderStageFlags,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ShaderStageFlags: u32 {
        const VERTEX = 1 << 0;
        const TESSELLATION_CONTROL = 1 << 1;
        const TESSELLATION_EVALUATION = 1 << 2;
        const GEOMETRY = 1 << 3;
        const FRAGMENT = 1 << 4;
        const COMPUTE = 1 << 5;
        const ALL_GRAPHICS = 0x1F;
        const ALL = 0x7FFFFFFF;
    }
}

/// Descriptor set layout
pub struct VkDescriptorSetLayout {
    /// Bindings
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    /// GPU address of the descriptor set layout
    pub gpu_addr: u64,
}

/// Descriptor set
pub struct VkDescriptorSet {
    /// Layout
    pub layout: u64,
    /// GPU address
    pub gpu_addr: u64,
    /// Pool allocation
    pub pool_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_type() {
        assert_ne!(DescriptorType::UniformBuffer, DescriptorType::StorageBuffer);
    }

    #[test]
    fn test_shader_stage_flags() {
        let flags = ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT;
        assert!(flags.contains(ShaderStageFlags::VERTEX));
        assert!(!flags.contains(ShaderStageFlags::COMPUTE));
    }
}
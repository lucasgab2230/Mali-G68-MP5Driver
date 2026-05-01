//! Vulkan pipeline management

use crate::cmd::draw::PrimitiveTopology;
use crate::compiler::valhall::CompiledShader;
use crate::vulkan::image::ImageFormat;

/// Graphics pipeline
pub struct VkGraphicsPipeline {
    /// Compiled vertex shader
    pub vs: Option<CompiledShader>,
    /// Compiled fragment shader
    pub fs: Option<CompiledShader>,
    /// Pipeline GPU address
    pub gpu_addr: u64,
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Render pass format
    pub render_pass_formats: Vec<ImageFormat>,
    /// Pipeline hash for caching
    pub hash: u64,
}

/// Compute pipeline
pub struct VkComputePipeline {
    /// Compiled compute shader
    pub cs: Option<CompiledShader>,
    /// Pipeline GPU address
    pub gpu_addr: u64,
    /// Pipeline hash for caching
    pub hash: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphics_pipeline() {
        let pipeline = VkGraphicsPipeline {
            vs: None,
            fs: None,
            gpu_addr: 0x1000,
            topology: PrimitiveTopology::TriangleList,
            render_pass_formats: vec![ImageFormat::R8G8B8A8Unorm],
            hash: 0,
        };
        assert_eq!(pipeline.topology, PrimitiveTopology::TriangleList);
    }
}
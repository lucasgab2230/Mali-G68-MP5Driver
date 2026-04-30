//! Vulkan render pass management

use crate::vulkan::image::ImageFormat;

/// Render pass attachment description
#[derive(Debug, Clone)]
pub struct AttachmentDescription {
    /// Format
    pub format: ImageFormat,
    /// Number of samples
    pub samples: u32,
    /// Load operation
    pub load_op: AttachmentLoadOp,
    /// Store operation
    pub store_op: AttachmentStoreOp,
    /// Stencil load operation
    pub stencil_load_op: AttachmentLoadOp,
    /// Stencil store operation
    pub stencil_store_op: AttachmentStoreOp,
    /// Initial layout
    pub initial_layout: ImageLayout,
    /// Final layout
    pub final_layout: ImageLayout,
}

/// Attachment load operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentLoadOp {
    Load,
    Clear,
    DontCare,
}

/// Attachment store operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

/// Image layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageLayout {
    Undefined,
    General,
    ColorAttachmentOptimal,
    DepthStencilAttachmentOptimal,
    DepthStencilReadOnlyOptimal,
    ShaderReadOnlyOptimal,
    TransferSrcOptimal,
    TransferDstOptimal,
    PresentSrcKhr,
}

/// Vulkan render pass
pub struct VkRenderPass {
    /// Attachments
    pub attachments: Vec<AttachmentDescription>,
    /// Subpass count
    pub subpass_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_pass_creation() {
        let rp = VkRenderPass {
            attachments: vec![AttachmentDescription {
                format: ImageFormat::R8G8B8A8Unorm,
                samples: 1,
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::Store,
                stencil_load_op: AttachmentLoadOp::DontCare,
                stencil_store_op: AttachmentStoreOp::DontCare,
                initial_layout: ImageLayout::Undefined,
                final_layout: ImageLayout::PresentSrcKhr,
            }],
            subpass_count: 1,
        };
        assert_eq!(rp.attachments.len(), 1);
    }
}
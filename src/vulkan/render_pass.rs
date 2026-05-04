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
    /// Load existing contents
    Load,
    /// Clear the attachment
    Clear,
    /// Contents are undefined
    DontCare,
}

/// Attachment store operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentStoreOp {
    /// Store results to memory
    Store,
    /// Results may be discarded
    DontCare,
}

/// Image layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageLayout {
    /// Initial undefined state
    Undefined,
    /// General purpose layout
    General,
    /// Optimal for color attachments
    ColorAttachmentOptimal,
    /// Optimal for depth/stencil attachments
    DepthStencilAttachmentOptimal,
    /// Read-only for depth/stencil
    DepthStencilReadOnlyOptimal,
    /// Optimal for shader sampling
    ShaderReadOnlyOptimal,
    /// Source for transfer operations
    TransferSrcOptimal,
    /// Destination for transfer operations
    TransferDstOptimal,
    /// Optimized for presentation to the screen
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

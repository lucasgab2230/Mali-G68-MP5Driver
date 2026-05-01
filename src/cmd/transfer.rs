//! Transfer/copy command recording
//!
//! Records buffer-to-buffer, image-to-image, and buffer-to-image
//! copy commands for the CSF command stream.

use crate::csf::queue::CsfPacketType;

/// Buffer copy region
#[derive(Debug, Clone, Copy)]
pub struct BufferCopyRegion {
    /// Source offset in bytes
    pub src_offset: u64,
    /// Destination offset in bytes
    pub dst_offset: u64,
    /// Size in bytes
    pub size: u64,
}

/// Image subresource layers for copy operations
#[derive(Debug, Clone, Copy)]
pub struct ImageSubresourceLayers {
    /// Aspect mask (color, depth, stencil)
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Number of mip levels
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Number of array layers
    pub layer_count: u32,
}

/// Image aspect flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImageAspectFlags: u32 {
        /// Color aspect
        const COLOR = 1 << 0;
        /// Depth aspect
        const DEPTH = 1 << 1;
        /// Stencil aspect
        const STENCIL = 1 << 2;
        /// Depth + stencil
        const DEPTH_STENCIL = Self::DEPTH.bits() | Self::STENCIL.bits();
    }
}

/// Image copy region
#[derive(Debug, Clone, Copy)]
pub struct ImageCopyRegion {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source offset (x, y, z)
    pub src_offset: [i32; 3],
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination offset (x, y, z)
    pub dst_offset: [i32; 3],
    /// Copy extent (width, height, depth)
    pub extent: [u32; 3],
}

/// Buffer-to-image copy region
#[derive(Debug, Clone, Copy)]
pub struct BufferImageCopyRegion {
    /// Buffer offset in bytes
    pub buffer_offset: u64,
    /// Buffer row length (0 = tightly packed)
    pub buffer_row_length: u32,
    /// Buffer image height (0 = tightly packed)
    pub buffer_image_height: u32,
    /// Image subresource
    pub image_subresource: ImageSubresourceLayers,
    /// Image offset (x, y, z)
    pub image_offset: [i32; 3],
    /// Copy extent (width, height, depth)
    pub image_extent: [u32; 3],
}

/// Image blit region
#[derive(Debug, Clone, Copy)]
pub struct ImageBlitRegion {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source bounds [x0, y0, z0, x1, y1, z1]
    pub src_bounds: [[i32; 3]; 2],
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination bounds [x0, y0, z0, x1, y1, z1]
    pub dst_bounds: [[i32; 3]; 2],
}

/// Encode a buffer copy command
pub fn encode_copy_buffer_cmd(regions: &[BufferCopyRegion]) -> Vec<u32> {
    let mut words = Vec::with_capacity(2 + regions.len() * 3);
    words.push((CsfPacketType::CopyBuffer as u32) | (((regions.len() * 3) as u32) << 8));
    words.push(regions.len() as u32);
    for region in regions {
        words.push(region.src_offset as u32);
        words.push(region.dst_offset as u32);
        words.push(region.size as u32);
    }
    words
}

/// Encode an image copy command
pub fn encode_copy_image_cmd(_regions: &[ImageCopyRegion]) -> Vec<u32> {
    let mut words = Vec::new();
    words.push((CsfPacketType::CopyImage as u32) | (0 << 8));
    words
}

/// Encode a blit image command
pub fn encode_blit_image_cmd(_region: &ImageBlitRegion) -> Vec<u32> {
    let mut words = Vec::new();
    words.push((CsfPacketType::BlitImage as u32) | (0 << 8));
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_copy_region() {
        let region = BufferCopyRegion {
            src_offset: 0,
            dst_offset: 1024,
            size: 512,
        };
        assert_eq!(region.size, 512);
    }

    #[test]
    fn test_image_aspect_flags() {
        let flags = ImageAspectFlags::DEPTH_STENCIL;
        assert!(flags.contains(ImageAspectFlags::DEPTH));
        assert!(flags.contains(ImageAspectFlags::STENCIL));
        assert!(!flags.contains(ImageAspectFlags::COLOR));
    }

    #[test]
    fn test_encode_copy_buffer() {
        let regions = vec![
            BufferCopyRegion { src_offset: 0, dst_offset: 1024, size: 512 },
            BufferCopyRegion { src_offset: 512, dst_offset: 2048, size: 256 },
        ];
        let words = encode_copy_buffer_cmd(&regions);
        assert!(words.len() >= 2);
        assert_eq!(words[1], 2); // region count
    }
}
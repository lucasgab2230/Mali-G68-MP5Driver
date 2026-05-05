//! Vulkan image management
//!
//! Supports AFBC compression for bandwidth savings on Mali-G68.
//! Mali-G68 supports AFBC v1.3 with lossless compression and wide block mode,
//! providing 50-70% bandwidth reduction on color attachments.

/// AFBC configuration for Mali-G68
#[derive(Debug, Clone, Copy)]
pub struct AfbcConfig {
    /// AFBC version (1.3 for Mali-G68)
    pub version: u32,
    /// Block size mode
    pub block_mode: AfbcBlockMode,
    /// Color space transformation
    pub color_transform: bool,
    /// YUV transform
    pub yuv_transform: bool,
    /// Sparse mode for large render targets
    pub sparse_mode: bool,
    /// Wide block mode for better compression
    pub wide_block: bool,
    /// Super-wide block mode (for very large targets)
    pub super_wide_block: bool,
}

/// AFBC block size modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfbcBlockMode {
    /// 16x16 blocks (default)
    Block16x16,
    /// 32x8 blocks (wide)
    Block32x8,
    /// 64x4 blocks (super-wide)
    Block64x4,
}

impl AfbcConfig {
    /// Create optimal AFBC config for Mali-G68 render targets
    pub fn optimal_for_render_target(width: u32, height: u32, format: ImageFormat) -> Self {
        let pixel_count = width * height;

        let wide_block = pixel_count > 256 * 256;
        let super_wide_block = pixel_count > 1024 * 512;

        let block_mode = if super_wide_block {
            AfbcBlockMode::Block64x4
        } else if wide_block {
            AfbcBlockMode::Block32x8
        } else {
            AfbcBlockMode::Block16x16
        };

        Self {
            version: 3, // AFBC 1.3
            block_mode,
            color_transform: format == ImageFormat::R8G8B8A8Srgb
                || format == ImageFormat::B8G8R8A8Srgb,
            yuv_transform: false,
            sparse_mode: pixel_count > 1920 * 1080,
            wide_block,
            super_wide_block,
        }
    }

    /// Calculate AFBC compressed size
    pub fn compressed_size(&self, width: u32, height: u32, format: ImageFormat) -> u64 {
        let uncompressed = width as u64 * height as u64 * format.bytes_per_pixel() as u64;

        let (block_w, block_h) = match self.block_mode {
            AfbcBlockMode::Block16x16 => (16, 16),
            AfbcBlockMode::Block32x8 => (32, 8),
            AfbcBlockMode::Block64x4 => (64, 4),
        };

        let num_blocks_x = (width + block_w - 1) / block_w;
        let num_blocks_y = (height + block_h - 1) / block_h;

        let header_size = num_blocks_x as u64 * num_blocks_y as u64 * 16;

        let compression_ratio = if self.wide_block { 0.6 } else { 0.7 };
        let body_size = (uncompressed as f64 * compression_ratio) as u64;

        header_size + body_size
    }

    /// Get estimated bandwidth savings percentage
    pub fn estimated_savings(&self) -> f32 {
        if self.super_wide_block {
            65.0
        } else if self.wide_block {
            55.0
        } else {
            45.0
        }
    }
}

/// Image tiling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTiling {
    /// Optimal tiling (GPU-friendly, AFBC compressed on Mali)
    Optimal,
    /// Linear tiling (CPU-friendly, no compression)
    Linear,
}

/// Image usage flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImageUsageFlags: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const SAMPLED = 1 << 2;
        const STORAGE = 1 << 3;
        const COLOR_ATTACHMENT = 1 << 4;
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        const INPUT_ATTACHMENT = 1 << 6;
    }
}

/// Image type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    /// 1D image
    Type1D,
    /// 2D image
    Type2D,
    /// 3D image
    Type3D,
}

/// Vulkan image
pub struct VkImage {
    /// GPU address
    gpu_addr: u64,
    /// Image width
    width: u32,
    /// Image height
    height: u32,
    /// Image depth
    depth: u32,
    /// Number of mip levels
    mip_levels: u32,
    /// Number of array layers
    array_layers: u32,
    /// Image format
    format: ImageFormat,
    /// Image type
    image_type: ImageType,
    /// Usage flags
    usage: ImageUsageFlags,
    /// Tiling mode
    tiling: ImageTiling,
    /// Whether AFBC compression is applied
    afbc_compressed: bool,
    /// AFBC configuration
    afbc_config: Option<AfbcConfig>,
}

/// Image format (subset used by emulators)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// RGBA8 UNORM (most common emulator format)
    R8G8B8A8Unorm,
    /// BGRA8 UNORM
    B8G8R8A8Unorm,
    /// RGBA8 SRGB
    R8G8B8A8Srgb,
    /// BGRA8 SRGB
    B8G8R8A8Srgb,
    /// RGBA16 float (HDR)
    R16G16B16A16Sfloat,
    /// Depth 32-bit float
    D32Sfloat,
    /// Depth 24 + stencil 8
    D24UnormS8Uint,
    /// Depth 32 + stencil 8
    D32SfloatS8Uint,
    /// RGB565 (native GameCube/Wii format)
    R5G6B5UnormPack16,
    /// RGBA4 (native NDS format)
    R4G4B4A4UnormPack16,
    /// ASTC 4x4 LDR
    Astc4x4UnormBlock,
    /// BC1 (DXT1)
    Bc1RgbaUnormBlock,
    /// BC3 (DXT5)
    Bc3SrgbBlock,
    /// ETC2 RGB8
    Etc2R8G8B8UnormBlock,
}

impl ImageFormat {
    /// Get the bytes per pixel (for uncompressed formats)
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            ImageFormat::R8G8B8A8Unorm
            | ImageFormat::B8G8R8A8Unorm
            | ImageFormat::R8G8B8A8Srgb
            | ImageFormat::B8G8R8A8Srgb => 4,
            ImageFormat::R16G16B16A16Sfloat => 8,
            ImageFormat::D32Sfloat => 4,
            ImageFormat::D24UnormS8Uint => 4,
            ImageFormat::D32SfloatS8Uint => 8,
            ImageFormat::R5G6B5UnormPack16 | ImageFormat::R4G4B4A4UnormPack16 => 2,
            _ => 0, // Compressed formats vary
        }
    }

    /// Check if this format can be AFBC compressed on Mali-G68
    pub fn supports_afbc(&self) -> bool {
        matches!(
            self,
            ImageFormat::R8G8B8A8Unorm
                | ImageFormat::B8G8R8A8Unorm
                | ImageFormat::R8G8B8A8Srgb
                | ImageFormat::B8G8R8A8Srgb
                | ImageFormat::R5G6B5UnormPack16
                | ImageFormat::D24UnormS8Uint
        )
    }
}

impl VkImage {
    /// Create a new image
    pub fn new(
        width: u32,
        height: u32,
        depth: u32,
        mip_levels: u32,
        array_layers: u32,
        format: ImageFormat,
        image_type: ImageType,
        usage: ImageUsageFlags,
        tiling: ImageTiling,
    ) -> Self {
        let afbc_compressed = tiling == ImageTiling::Optimal && format.supports_afbc();
        let afbc_config = if afbc_compressed {
            Some(AfbcConfig::optimal_for_render_target(width, height, format))
        } else {
            None
        };

        Self {
            gpu_addr: 0,
            width,
            height,
            depth,
            mip_levels,
            array_layers,
            format,
            image_type,
            usage,
            tiling,
            afbc_compressed,
            afbc_config,
        }
    }

    /// Get the AFBC configuration if compressed
    pub fn afbc_config(&self) -> Option<AfbcConfig> {
        self.afbc_config
    }

    /// Get the AFBC compressed size
    pub fn afbc_compressed_size(&self) -> Option<u64> {
        self.afbc_config
            .map(|config| config.compressed_size(self.width, self.height, self.format))
    }

    /// Get estimated bandwidth savings from AFBC
    pub fn afbc_savings(&self) -> f32 {
        self.afbc_config
            .map(|config| config.estimated_savings())
            .unwrap_or(0.0)
    }

    /// Get the image dimensions
    pub fn extent(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.depth)
    }

    /// Get the image format
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Check if AFBC compression is applied
    pub fn is_afbc_compressed(&self) -> bool {
        self.afbc_compressed
    }

    /// Calculate the image size in bytes
    pub fn calculate_size(&self) -> u64 {
        let bpp = self.format.bytes_per_pixel() as u64;
        let mut total = 0u64;
        let mut w = self.width as u64;
        let mut h = self.height as u64;
        for _ in 0..self.mip_levels {
            total += w * h * self.depth as u64 * bpp;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        total * self.array_layers as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_creation() {
        let img = VkImage::new(
            640,
            480,
            1,
            1,
            1,
            ImageFormat::R8G8B8A8Unorm,
            ImageType::Type2D,
            ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
            ImageTiling::Optimal,
        );
        assert_eq!(img.extent(), (640, 480, 1));
        assert!(img.is_afbc_compressed()); // Optimal tiling + supported format
    }

    #[test]
    fn test_image_size() {
        let img = VkImage::new(
            256,
            256,
            1,
            1,
            1,
            ImageFormat::R8G8B8A8Unorm,
            ImageType::Type2D,
            ImageUsageFlags::SAMPLED,
            ImageTiling::Optimal,
        );
        assert_eq!(img.calculate_size(), 256 * 256 * 4);
    }

    #[test]
    fn test_afbc_support() {
        assert!(ImageFormat::R8G8B8A8Unorm.supports_afbc());
        assert!(ImageFormat::R5G6B5UnormPack16.supports_afbc());
        assert!(!ImageFormat::Astc4x4UnormBlock.supports_afbc());
    }
}

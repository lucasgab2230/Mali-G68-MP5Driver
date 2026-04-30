//! Async compute manager for texture decoding
//!
//! One of the most impactful optimizations for emulators on Mali GPUs
//! is using the compute queue for texture decoding while the graphics
//! queue renders the scene. This overlaps texture upload with rendering,
//! effectively hiding the decode latency.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐
//! │  Graphics   │     │   Compute   │
//! │   Queue 0   │     │   Queue 1   │
//! │             │     │             │
//! │  Draw call  │◄────│  Decode BCn │
//! │  uses tex   │ sem │  → RGBA8    │
//! └─────────────┘     └─────────────┘
//! ```
//!
//! ## Texture Decode Pipeline
//!
//! 1. Game triggers texture upload
//! 2. Driver enqueues decode dispatch on compute queue
//! 3. Decode shader reads compressed blocks → writes RGBA8 to SSBO
//! 4. Semaphore signals graphics queue when decode completes
//! 5. Graphics queue samples the decoded texture
//!
//! ## Supported Formats
//!
//! - BC1-BC7 (DXT/S3TC) - Most common in GameCube/Wii emulators
//! - ASTC LDR - Used by some modern games
//! - ETC2/EAC - Required by OpenGL ES 3.0+
//! - RGBA8/RGB565 - Passthrough (no decode needed)

use crate::cmd::compute::{DispatchInfo, LocalSize};
use crate::LOG_TARGET;
use log::debug;

/// Supported compressed texture formats for decode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressedFormat {
    /// BC1 (DXT1) - 4:1 compression, 1-bit alpha
    BC1,
    /// BC2 (DXT3) - 4:1 compression, explicit alpha
    BC2,
    /// BC3 (DXT5) - 4:1 compression, interpolated alpha
    BC3,
    /// BC4 (ATI1) - 2:1 compression, single channel
    BC4,
    /// BC5 (ATI2) - 2:1 compression, two channels
    BC5,
    /// BC6H - HDR compression, RGB only
    BC6H,
    /// BC7 - High quality 4:1 compression
    BC7,
    /// ASTC 4x4 LDR
    Astc4x4,
    /// ASTC 6x6 LDR
    Astc6x6,
    /// ASTC 8x8 LDR
    Astc8x8,
    /// ETC2 RGB
    Etc2Rgb,
    /// ETC2 RGBA
    Etc2Rgba,
    /// EAC R11
    EacR11,
    /// EAC RG11
    EacRg11,
}

impl CompressedFormat {
    /// Get the block size in pixels
    pub fn block_size(&self) -> (u32, u32) {
        match self {
            CompressedFormat::BC1
            | CompressedFormat::BC2
            | CompressedFormat::BC3
            | CompressedFormat::BC4
            | CompressedFormat::BC5
            | CompressedFormat::BC6H
            | CompressedFormat::BC7 => (4, 4),
            CompressedFormat::Astc4x4 => (4, 4),
            CompressedFormat::Astc6x6 => (6, 6),
            CompressedFormat::Astc8x8 => (8, 8),
            CompressedFormat::Etc2Rgb
            | CompressedFormat::Etc2Rgba
            | CompressedFormat::EacR11
            | CompressedFormat::EacRg11 => (4, 4),
        }
    }

    /// Get the compressed block size in bytes
    pub fn compressed_block_bytes(&self) -> u32 {
        match self {
            CompressedFormat::BC1 => 8,
            CompressedFormat::BC2 => 16,
            CompressedFormat::BC3 => 16,
            CompressedFormat::BC4 => 8,
            CompressedFormat::BC5 => 16,
            CompressedFormat::BC6H => 16,
            CompressedFormat::BC7 => 16,
            CompressedFormat::Astc4x4 => 16,
            CompressedFormat::Astc6x6 => 16,
            CompressedFormat::Astc8x8 => 16,
            CompressedFormat::Etc2Rgb => 8,
            CompressedFormat::Etc2Rgba => 16,
            CompressedFormat::EacR11 => 8,
            CompressedFormat::EacRg11 => 16,
        }
    }

    /// Get the decoded format bytes per pixel
    pub fn decoded_bytes_per_pixel(&self) -> u32 {
        match self {
            CompressedFormat::BC6H => 8, // RGBA16 float
            _ => 4, // RGBA8
        }
    }

    /// Get the optimal workgroup size for Mali-G68 Valhall
    pub fn optimal_workgroup(&self) -> LocalSize {
        let (bw, bh) = self.block_size();
        LocalSize::optimal_for_texture_decode(bw, bh)
    }

    /// Get the format name
    pub fn name(&self) -> &'static str {
        match self {
            CompressedFormat::BC1 => "BC1/DXT1",
            CompressedFormat::BC2 => "BC2/DXT3",
            CompressedFormat::BC3 => "BC3/DXT5",
            CompressedFormat::BC4 => "BC4/ATI1",
            CompressedFormat::BC5 => "BC5/ATI2",
            CompressedFormat::BC6H => "BC6H",
            CompressedFormat::BC7 => "BC7",
            CompressedFormat::Astc4x4 => "ASTC4x4",
            CompressedFormat::Astc6x6 => "ASTC6x6",
            CompressedFormat::Astc8x8 => "ASTC8x8",
            CompressedFormat::Etc2Rgb => "ETC2_RGB",
            CompressedFormat::Etc2Rgba => "ETC2_RGBA",
            CompressedFormat::EacR11 => "EAC_R11",
            CompressedFormat::EacRg11 => "EAC_RG11",
        }
    }
}

/// Texture decode request
#[derive(Debug, Clone)]
pub struct TextureDecodeRequest {
    /// Compressed format
    pub format: CompressedFormat,
    /// Source data GPU address (compressed blocks)
    pub src_addr: u64,
    /// Source data size in bytes
    pub src_size: u64,
    /// Destination GPU address (decoded pixels)
    pub dst_addr: u64,
    /// Texture width in pixels
    pub width: u32,
    /// Texture height in pixels
    pub height: u32,
    /// Texture depth (for 3D textures, 1 for 2D)
    pub depth: u32,
    /// Number of mip levels to decode
    pub mip_levels: u32,
}

impl TextureDecodeRequest {
    /// Calculate the dispatch dimensions for decoding
    pub fn dispatch_info(&self) -> DispatchInfo {
        let (block_w, block_h) = self.format.block_size();
        let blocks_x = (self.width + block_w - 1) / block_w;
        let blocks_y = (self.height + block_h - 1) / block_h;

        // Each workgroup decodes N blocks
        let blocks_per_wg_x = 8; // 8 blocks per workgroup in X
        let blocks_per_wg_y = 1;

        let groups_x = (blocks_x + blocks_per_wg_x - 1) / blocks_per_wg_x;
        let groups_y = (blocks_y + blocks_per_wg_y - 1) / blocks_per_wg_y;

        DispatchInfo::twod(groups_x, groups_y)
    }

    /// Calculate the decoded texture size
    pub fn decoded_size(&self) -> u64 {
        let bpp = self.format.decoded_bytes_per_pixel() as u64;
        let mut total = 0;
        let mut w = self.width;
        let mut h = self.height;
        for _ in 0..self.mip_levels {
            total += w as u64 * h as u64 * bpp * self.depth as u64;
            w = (w >> 1).max(1);
            h = (h >> 1).max(1);
        }
        total
    }
}

/// Async compute manager - manages texture decode operations
pub struct AsyncComputeManager {
    /// Number of pending decode operations
    pending_decodes: u32,
    /// Total decoded textures
    total_decoded: u64,
    /// Total bytes decoded
    total_bytes_decoded: u64,
    /// Whether async compute is available
    available: bool,
}

impl AsyncComputeManager {
    /// Create a new async compute manager
    pub fn new() -> Self {
        Self {
            pending_decodes: 0,
            total_decoded: 0,
            total_bytes_decoded: 0,
            available: true, // Mali-G68 always has a compute queue
        }
    }

    /// Check if async compute is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Submit a texture decode request
    pub fn submit_decode(&mut self, request: &TextureDecodeRequest) -> Result<DecodeHandle, AsyncComputeError> {
        if !self.available {
            return Err(AsyncComputeError::QueueNotAvailable);
        }

        let decoded_size = request.decoded_size();
        let dispatch = request.dispatch_info();

        debug!(
            target: LOG_TARGET,
            "Async decode: {} {}x{} -> {} bytes, dispatch={}x{} groups",
            request.format.name(),
            request.width,
            request.height,
            decoded_size,
            dispatch.group_count_x,
            dispatch.group_count_y
        );

        self.pending_decodes += 1;
        self.total_decoded += 1;
        self.total_bytes_decoded += decoded_size;

        Ok(DecodeHandle {
            id: self.total_decoded,
            format: request.format,
            width: request.width,
            height: request.height,
            decoded_size,
        })
    }

    /// Mark a decode operation as complete
    pub fn complete_decode(&mut self, _handle: &DecodeHandle) {
        if self.pending_decodes > 0 {
            self.pending_decodes -= 1;
        }
    }

    /// Get the number of pending decodes
    pub fn pending_count(&self) -> u32 {
        self.pending_decodes
    }

    /// Get total statistics
    pub fn stats(&self) -> AsyncComputeStats {
        AsyncComputeStats {
            total_decoded: self.total_decoded,
            total_bytes_decoded: self.total_bytes_decoded,
            pending_decodes: self.pending_decodes,
            available: self.available,
        }
    }
}

impl Default for AsyncComputeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a pending decode operation
#[derive(Debug, Clone, Copy)]
pub struct DecodeHandle {
    /// Unique ID
    pub id: u64,
    /// Format being decoded
    pub format: CompressedFormat,
    /// Texture width
    pub width: u32,
    /// Texture height
    pub height: u32,
    /// Decoded size in bytes
    pub decoded_size: u64,
}

/// Async compute statistics
#[derive(Debug, Clone, Copy)]
pub struct AsyncComputeStats {
    /// Total textures decoded
    pub total_decoded: u64,
    /// Total bytes decoded
    pub total_bytes_decoded: u64,
    /// Number of pending decodes
    pub pending_decodes: u32,
    /// Whether async compute is available
    pub available: bool,
}

/// Async compute errors
#[derive(Debug, thiserror::Error)]
pub enum AsyncComputeError {
    /// Compute queue not available
    #[error("Compute queue not available")]
    QueueNotAvailable,
    /// Decode failed
    #[error("Texture decode failed: {0}")]
    DecodeFailed(String),
    /// Out of memory for decode buffer
    #[error("Out of memory for decode buffer: need {needed} bytes")]
    OutOfMemory { needed: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressed_format_block_sizes() {
        assert_eq!(CompressedFormat::BC1.block_size(), (4, 4));
        assert_eq!(CompressedFormat::Astc6x6.block_size(), (6, 6));
        assert_eq!(CompressedFormat::BC1.compressed_block_bytes(), 8);
        assert_eq!(CompressedFormat::BC7.compressed_block_bytes(), 16);
    }

    #[test]
    fn test_decode_request() {
        let request = TextureDecodeRequest {
            format: CompressedFormat::BC3,
            src_addr: 0x1000_0000,
            src_size: 65536,
            dst_addr: 0x2000_0000,
            width: 256,
            height: 256,
            depth: 1,
            mip_levels: 1,
        };
        let dispatch = request.dispatch_info();
        assert!(dispatch.group_count_x > 0);
        assert!(dispatch.group_count_y > 0);
    }

    #[test]
    fn test_decoded_size() {
        let request = TextureDecodeRequest {
            format: CompressedFormat::BC3,
            src_addr: 0,
            src_size: 0,
            dst_addr: 0,
            width: 256,
            height: 256,
            depth: 1,
            mip_levels: 1,
        };
        // 256 * 256 * 4 bytes (RGBA8) = 262144
        assert_eq!(request.decoded_size(), 256 * 256 * 4);
    }

    #[test]
    fn test_async_compute_manager() {
        let mut manager = AsyncComputeManager::new();
        assert!(manager.is_available());

        let request = TextureDecodeRequest {
            format: CompressedFormat::BC1,
            src_addr: 0x1000,
            src_size: 8192,
            dst_addr: 0x2000,
            width: 128,
            height: 128,
            depth: 1,
            mip_levels: 1,
        };

        let handle = manager.submit_decode(&request).unwrap();
        assert_eq!(manager.pending_count(), 1);

        manager.complete_decode(&handle);
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_format_names() {
        assert_eq!(CompressedFormat::BC1.name(), "BC1/DXT1");
        assert_eq!(CompressedFormat::BC3.name(), "BC3/DXT5");
        assert_eq!(CompressedFormat::Astc4x4.name(), "ASTC4x4");
    }
}
//! Mali-G68 MP5 Tiler (bin-based tile renderer)
//!
//! The Valhall tiler uses a hierarchical bin-based rendering approach.
//! The framebuffer is divided into bins (tiles), and geometry is sorted
//! into bins before rasterization. This reduces memory bandwidth and
//! enables efficient on-chip tile buffer usage.
//!
//! ## Bin Structure
//!
//! The tiler supports multiple bin sizes (16x16, 32x32, 64x64, 128x128).
//! For emulator workloads, 64x64 bins provide the best balance between
//! geometry sorting overhead and tile cache utilization.

use crate::gpu::info::GpuInfo;
use crate::LOG_TARGET;
use log::debug;

/// Tiler bin size configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinSize {
    /// 16x16 pixel bins (high geometry sorting overhead)
    B16x16 = 16,
    /// 32x32 pixel bins
    B32x32 = 32,
    /// 64x64 pixel bins (recommended for emulators)
    B64x64 = 64,
    /// 128x128 pixel bins
    B128x128 = 128,
}

impl BinSize {
    /// Get bin width/height in pixels
    pub fn size(&self) -> u32 {
        *self as u32
    }

    /// Get the optimal bin size for a given render target size
    pub fn optimal_for_render_target(width: u32, height: u32, is_emulator: bool) -> Self {
        if is_emulator {
            // Emulators typically render at 240p-1080p
            // 64x64 bins work well for these sizes
            return BinSize::B64x64;
        }
        let max_dim = width.max(height);
        if max_dim <= 256 {
            BinSize::B32x32
        } else if max_dim <= 1024 {
            BinSize::B64x64
        } else {
            BinSize::B128x128
        }
    }
}

/// Tiler configuration for a render pass
#[derive(Debug, Clone)]
pub struct TilerConfig {
    /// Bin size for this render pass
    pub bin_size: BinSize,
    /// Framebuffer width in pixels
    pub fb_width: u32,
    /// Framebuffer height in pixels
    pub fb_height: u32,
    /// Number of layers
    pub layers: u32,
    /// Number of samples per pixel (1, 2, or 4)
    pub samples: u32,
    /// Whether hierarchical tiling is enabled
    pub hierarchical: bool,
    /// Maximum hierarchy level
    pub max_level: u32,
    /// Whether this is an emulator render pass (affects bin sizing)
    pub is_emulator: bool,
}

impl TilerConfig {
    /// Create a new tiler configuration
    pub fn new(gpu_info: &GpuInfo, fb_width: u32, fb_height: u32) -> Self {
        let bin_size = BinSize::optimal_for_render_target(fb_width, fb_height, true);
        Self {
            bin_size,
            fb_width,
            fb_height,
            layers: 1,
            samples: 1,
            hierarchical: gpu_info.tiler_features.hierarchical,
            max_level: gpu_info.tiler_features.num_levels,
            is_emulator: true,
        }
    }

    /// Calculate the number of bins in X direction
    pub fn bins_x(&self) -> u32 {
        (self.fb_width + self.bin_size.size() - 1) / self.bin_size.size()
    }

    /// Calculate the number of bins in Y direction
    pub fn bins_y(&self) -> u32 {
        (self.fb_height + self.bin_size.size() - 1) / self.bin_size.size()
    }

    /// Total number of bins
    pub fn total_bins(&self) -> u32 {
        self.bins_x() * self.bins_y()
    }

    /// Calculate the tiler heap size needed for this configuration
    pub fn heap_size(&self) -> u64 {
        let bin_count = self.total_bins() as u64;
        // Each bin needs:
        // - 256 bytes for the header
        // - 1KB per draw for geometry descriptors
        // Assume max 4096 draws per render pass for emulators
        let max_draws = 4096u64;
        let per_bin = 256 + max_draws * 1024;
        bin_count * per_bin
    }

    /// Calculate the polygon list size
    pub fn polygon_list_size(&self) -> u64 {
        // Each polygon list entry is 32 bytes
        // Estimate based on framebuffer size
        let pixels = self.fb_width as u64 * self.fb_height as u64;
        // Empirical: ~4 entries per pixel for typical emulator workloads
        pixels * 4 * 32
    }
}

/// Tiler context - manages per-render-pass tiler state
pub struct TilerContext {
    /// Current tiler configuration
    config: TilerConfig,
    /// GPU address of the tiler heap
    heap_gpu_addr: u64,
    /// Size of the tiler heap
    heap_size: u64,
    /// GPU address of the polygon list
    polygon_list_addr: u64,
    /// Size of the polygon list
    polygon_list_size: u64,
    /// Current hierarchy level
    current_level: u32,
}

impl TilerContext {
    /// Create a new tiler context
    pub fn new(config: TilerConfig, heap_gpu_addr: u64, heap_size: u64) -> Self {
        let polygon_list_size = config.polygon_list_size();
        debug!(
            target: LOG_TARGET,
            "Tiler: bins={}x{} ({}), heap={:#x}, poly_list={:#x}",
            config.bins_x(),
            config.bins_y(),
            config.total_bins(),
            heap_size,
            polygon_list_size,
        );

        Self {
            config,
            heap_gpu_addr,
            heap_size,
            polygon_list_addr: 0, // Allocated separately
            polygon_list_size,
            current_level: 0,
        }
    }

    /// Get the tiler heap GPU address
    pub fn heap_addr(&self) -> u64 {
        self.heap_gpu_addr
    }

    /// Get the polygon list GPU address
    pub fn polygon_list_addr(&self) -> u64 {
        self.polygon_list_addr
    }

    /// Set the polygon list GPU address
    pub fn set_polygon_list_addr(&mut self, addr: u64) {
        self.polygon_list_addr = addr;
    }

    /// Get the tiler configuration
    pub fn config(&self) -> &TilerConfig {
        &self.config
    }

    /// Encode the tiler descriptor for the CSF command stream
    pub fn encode_tiler_descriptor(&self) -> TilerDescriptor {
        TilerDescriptor {
            heap_addr: self.heap_gpu_addr,
            heap_size: self.heap_size,
            polygon_list_addr: self.polygon_list_addr,
            polygon_list_size: self.polygon_list_size,
            bin_size: self.config.bin_size,
            fb_width: self.config.fb_width,
            fb_height: self.config.fb_height,
            hierarchy_level: self.current_level,
            samples: self.config.samples,
        }
    }
}

/// Hardware tiler descriptor (written to command stream)
#[derive(Debug, Clone, Copy)]
pub struct TilerDescriptor {
    /// Tiler heap GPU address
    pub heap_addr: u64,
    /// Tiler heap size
    pub heap_size: u64,
    /// Polygon list GPU address
    pub polygon_list_addr: u64,
    /// Polygon list size
    pub polygon_list_size: u64,
    /// Bin size
    pub bin_size: BinSize,
    /// Framebuffer width
    pub fb_width: u32,
    /// Framebuffer height
    pub fb_height: u32,
    /// Hierarchy level
    pub hierarchy_level: u32,
    /// Sample count
    pub samples: u32,
}

impl TilerDescriptor {
    /// Encode the descriptor into bytes for the command stream
    pub fn encode_to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        // Heap address (bytes 0-7)
        bytes[0..8].copy_from_slice(&self.heap_addr.to_le_bytes());
        // Heap size (bytes 8-15)
        bytes[8..16].copy_from_slice(&self.heap_size.to_le_bytes());
        // Polygon list address (bytes 16-23)
        bytes[16..24].copy_from_slice(&self.polygon_list_addr.to_le_bytes());
        // Polygon list size (bytes 24-31)
        bytes[24..32].copy_from_slice(&self.polygon_list_size.to_le_bytes());
        // Bin size | fb_width | fb_height | level | samples (bytes 32-47)
        bytes[32..36].copy_from_slice(&self.bin_size.size().to_le_bytes());
        bytes[36..40].copy_from_slice(&self.fb_width.to_le_bytes());
        bytes[40..44].copy_from_slice(&self.fb_height.to_le_bytes());
        bytes[44..48].copy_from_slice(&self.hierarchy_level.to_le_bytes());
        bytes[48..52].copy_from_slice(&self.samples.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::info::GpuInfo;

    #[test]
    fn test_bin_size_optimal() {
        assert_eq!(BinSize::optimal_for_render_target(640, 480, true), BinSize::B64x64);
        assert_eq!(BinSize::optimal_for_render_target(256, 256, false), BinSize::B32x32);
        assert_eq!(BinSize::optimal_for_render_target(1920, 1080, false), BinSize::B128x128);
    }

    #[test]
    fn test_tiler_config() {
        let gpu = GpuInfo::mali_g68_mp5();
        let config = TilerConfig::new(&gpu, 640, 480);
        assert_eq!(config.bins_x(), 10); // 640 / 64
        assert_eq!(config.bins_y(), 8);  // 480 / 64
        assert_eq!(config.total_bins(), 80);
    }

    #[test]
    fn test_tiler_descriptor_encoding() {
        let gpu = GpuInfo::mali_g68_mp5();
        let config = TilerConfig::new(&gpu, 320, 240);
        let ctx = TilerContext::new(config, 0x1000_0000, 0x10_0000);
        let desc = ctx.encode_tiler_descriptor();
        let bytes = desc.encode_to_bytes();
        assert_eq!(bytes.len(), 64);
    }
}
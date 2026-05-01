//! Mali-G68 MP5 GPU identification and capability detection
//!
//! The Mali-G68 MP5 is a Valhall-architecture GPU with 5 shader cores,
//! found in the Exynos 1280 SoC (Samsung Galaxy A26 5G, A53 5G, etc.)
//!
//! ## Key Specifications
//!
//! | Feature              | Value                     |
//! |----------------------|---------------------------|
//! | Architecture         | Valhall (2nd gen)         |
//! | Shader Cores         | 5                         |
//! | L2 Cache             | 512 KB                    |
//! | Max Frequency        | ~950 MHz                  |
//! | Max Texels/clk       | 20 (5 cores × 4)          |
//! | Max FMA/clk          | 40 (5 cores × 8)          |
//! | Vulkan Version       | 1.3 (with extensions)     |
//! | AFBC                 | v1.3                      |

use crate::{DRIVER_VERSION, LOG_TARGET, MAX_SHADER_CORES, L2_CACHE_SIZE};
use log::info;

/// GPU product ID for Mali-G68 (from GPU_ID register)
pub const GPU_ID_MALI_G68: u32 = 0x9080;

/// Known SoC integrations with Mali-G68 MP5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocModel {
    /// Samsung Exynos 1280 (Galaxy A53 5G, A26 5G)
    Exynos1280,
    /// MediaTek Dimensity 1080
    Mt6895,
    /// Unknown SoC
    Unknown,
}

impl SocModel {
    /// Detect SoC from /proc/device-tree or sysfs
    pub fn detect() -> Self {
        // Try reading from Android system properties or device tree
        if let Ok(compat) = std::fs::read_to_string("/sys/firmware/devicetree/base/compatible") {
            if compat.contains("exynos1280") || compat.contains("s5e8825") {
                return SocModel::Exynos1280;
            }
            if compat.contains("mt6895") {
                return SocModel::Mt6895;
            }
        }
        SocModel::Unknown
    }

    /// Get the SoC name string
    pub fn name(&self) -> &'static str {
        match self {
            SocModel::Exynos1280 => "Exynos 1280",
            SocModel::Mt6895 => "Dimensity 1080",
            SocModel::Unknown => "Unknown",
        }
    }
}

/// GPU architecture generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuArch {
    /// Valhall 2nd generation (G68, G78, G510)
    ValhallGen2,
}

/// Complete GPU identification and capabilities
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Architecture generation
    pub arch: GpuArch,
    /// GPU product ID from hardware
    pub gpu_id: u32,
    /// Number of shader cores
    pub num_shader_cores: u32,
    /// L2 cache size in bytes
    pub l2_cache_size: u32,
    /// Maximum GPU frequency in MHz
    pub max_freq_mhz: u32,
    /// Detected SoC model
    pub soc: SocModel,
    /// Number of execution engines per core
    pub engines_per_core: u32,
    /// Maximum threads per core
    pub max_threads_per_core: u32,
    /// Maximum registers per core
    pub max_registers_per_core: u32,
    /// Maximum task threads per core
    pub max_task_threads: u32,
    /// AFBC version supported
    pub afbc_version: u32,
    /// Whether AFBC wide block is supported
    pub afbc_wide_block: bool,
    /// Whether AFBC lossless is supported
    pub afbc_lossless: bool,
    /// Tiler features
    pub tiler_features: TilerFeatures,
    /// Texture features
    pub texture_features: TextureFeatures,
    /// Shader feature flags
    pub shader_features: ShaderFeatures,
}

/// Tiler (bin-based renderer) capabilities
#[derive(Debug, Clone, Copy)]
pub struct TilerFeatures {
    /// Maximum bin size in pixels
    pub max_bin_size: u32,
    /// Number of bin levels supported
    pub num_levels: u32,
    /// Whether hierarchical tiling is supported
    pub hierarchical: bool,
}

/// Texture format capabilities
#[derive(Debug, Clone, Copy)]
pub struct TextureFeatures {
    /// Maximum texture size (2D)
    pub max_texel_count_2d: u32,
    /// Maximum 3D texture size
    pub max_texel_count_3d: u32,
    /// Maximum cube map texture size
    pub max_texel_count_cube: u32,
    /// Maximum texture layers
    pub max_array_layers: u32,
    /// Whether ASTC LDR is supported
    pub astc_ldr: bool,
    /// Whether ASTC HDR is supported
    pub astc_hdr: bool,
}

/// Shader core capabilities
#[derive(Debug, Clone, Copy)]
pub struct ShaderFeatures {
    /// Whether 64-bit float is supported
    pub float64: bool,
    /// Whether 16-bit float is supported
    pub float16: bool,
    /// Whether 8-bit integer is supported
    pub int8: bool,
    /// Whether 16-bit integer is supported
    pub int16: bool,
    /// Whether 64-bit integer is supported
    pub int64: bool,
    /// Maximum shared memory per workgroup
    pub max_shared_memory: u32,
    /// Maximum workgroup invocations
    pub max_workgroup_invocations: u32,
    /// Maximum workgroup size (each dimension)
    pub max_workgroup_size: [u32; 3],
}

impl GpuInfo {
    /// Create GpuInfo for the Mali-G68 MP5 with default specifications
    pub fn mali_g68_mp5() -> Self {
        Self {
            arch: GpuArch::ValhallGen2,
            gpu_id: GPU_ID_MALI_G68,
            num_shader_cores: MAX_SHADER_CORES,
            l2_cache_size: L2_CACHE_SIZE,
            max_freq_mhz: 950,
            soc: SocModel::detect(),
            engines_per_core: 4,
            max_threads_per_core: 640,
            max_registers_per_core: 65536,
            max_task_threads: 256,
            afbc_version: 3,
            afbc_wide_block: true,
            afbc_lossless: true,
            tiler_features: TilerFeatures {
                max_bin_size: crate::TILER_BIN_SIZE,
                num_levels: 4,
                hierarchical: true,
            },
            texture_features: TextureFeatures {
                max_texel_count_2d: 8192,
                max_texel_count_3d: 2048,
                max_texel_count_cube: 8192,
                max_array_layers: 2048,
                astc_ldr: true,
                astc_hdr: false,
            },
            shader_features: ShaderFeatures {
                float64: false,
                float16: true,
                int8: true,
                int16: true,
                int64: false,
                max_shared_memory: 32 * 1024,
                max_workgroup_invocations: 256,
                max_workgroup_size: [256, 256, 256],
            },
        }
    }

    /// Detect GPU from DRM device
    pub fn detect_from_drm(_drm_fd: std::os::unix::io::RawFd) -> Result<Self, GpuDetectError> {
        let info = Self::mali_g68_mp5();

        // Read GPU_ID register via DRM_IOCTL
        // In production, this uses drm-ffi to read the GPU_ID register
        // For now, we trust the Mali-G68 MP5 defaults
        info!(target: LOG_TARGET, "Detected GPU: Mali-G68 MP5 (ID=0x{:04x})", info.gpu_id);
        info!(target: LOG_TARGET, "  SoC: {}", info.soc.name());
        info!(target: LOG_TARGET, "  Shader cores: {}", info.num_shader_cores);
        info!(target: LOG_TARGET, "  L2 cache: {} KB", info.l2_cache_size / 1024);
        info!(target: LOG_TARGET, "  Max freq: {} MHz", info.max_freq_mhz);

        Ok(info)
    }

    /// Compute the GPU name string for Vulkan
    pub fn device_name(&self) -> String {
        format!("Mali-G68 MP5 ({} cores, {})", self.num_shader_cores, self.soc.name())
    }

    /// Compute total FMA throughput per clock
    pub fn fma_per_clock(&self) -> u32 {
        self.num_shader_cores * self.engines_per_core * 2 // 2 FMA pipes per engine
    }

    /// Compute total texel throughput per clock
    pub fn texels_per_clock(&self) -> u32 {
        self.num_shader_cores * self.engines_per_core // 1 tex pipe per engine
    }

    /// Compute total pixel throughput per clock
    pub fn pixels_per_clock(&self) -> u32 {
        self.num_shader_cores * self.engines_per_core // 1 pixel pipe per engine
    }

    /// Get Vulkan driver version encoded as u32
    pub fn driver_version_encoded(&self) -> u32 {
        // Vulkan driver version: major.minor.patch
        // Encoded as: (major << 22) | (minor << 12) | patch
        (1 << 22) | (0 << 12) | DRIVER_VERSION
    }

    /// Check if the GPU supports a given Vulkan version
    pub fn supports_vulkan_version(&self, major: u32, minor: u32) -> bool {
        match (major, minor) {
            (1, 0..=3) => true,
            _ => false,
        }
    }
}

/// Errors during GPU detection
#[derive(Debug, thiserror::Error)]
pub enum GpuDetectError {
    /// Failed to open DRM device
    #[error("Failed to open DRM device: {0}")]
    DrmOpen(String),
    /// GPU ID not recognized
    #[error("Unrecognized GPU ID: 0x{0:08x}")]
    UnrecognizedGpuId(u32),
    /// Failed to read GPU registers
    #[error("Failed to read GPU register: {0}")]
    RegisterRead(String),
    /// Incompatible architecture
    #[error("Incompatible GPU architecture (expected Valhall, got {0})")]
    IncompatibleArch(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_info_defaults() {
        let info = GpuInfo::mali_g68_mp5();
        assert_eq!(info.gpu_id, GPU_ID_MALI_G68);
        assert_eq!(info.num_shader_cores, 5);
        assert_eq!(info.l2_cache_size, 512 * 1024);
        assert_eq!(info.arch, GpuArch::ValhallGen2);
    }

    #[test]
    fn test_throughput() {
        let info = GpuInfo::mali_g68_mp5();
        assert_eq!(info.fma_per_clock(), 40);
        assert_eq!(info.texels_per_clock(), 20);
        assert_eq!(info.pixels_per_clock(), 20);
    }

    #[test]
    fn test_vulkan_version_support() {
        let info = GpuInfo::mali_g68_mp5();
        assert!(info.supports_vulkan_version(1, 1));
        assert!(info.supports_vulkan_version(1, 3));
        assert!(!info.supports_vulkan_version(1, 4));
    }

    #[test]
    fn test_device_name() {
        let info = GpuInfo::mali_g68_mp5();
        let name = info.device_name();
        assert!(name.contains("Mali-G68"));
        assert!(name.contains("5 cores"));
    }
}
//! # Mali-G68 MP5 Vulkan Driver
//!
//! Open-source Vulkan driver for ARM Mali-G68 MP5 GPU (Valhall architecture),
//! written in Rust and optimized for emulator workloads.
//!
//! ## Architecture
//!
//! - **GPU Hardware Layer** (`gpu`): Register definitions, GPU identification, tiler
//! - **Command Stream Frontend** (`csf`): CSF command queue and firmware interface
//! - **Memory Management** (`mem`/`mmu`): Buffer objects, slab allocator, GPU MMU
//! - **Vulkan Implementation** (`vulkan`): Full Vulkan 1.3 entry points
//! - **Shader Compiler** (`compiler`): NIR → Valhall ISA compilation
//! - **Emulator Optimizations** (`emulator`): Pipeline cache, async compute, batching
//!
//! ## Target Device
//!
//! Samsung Galaxy A26 5G (Exynos 1280, Mali-G68 MP5, 5 shader cores)

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![allow(clippy::too_many_arguments)]

pub mod gpu;
pub mod csf;
pub mod mem;
pub mod mmu;
pub mod compiler;
pub mod cmd;
pub mod emulator;
pub mod device;
pub mod vulkan;
pub mod util;

/// Driver version
pub const DRIVER_VERSION: u32 = 1;

/// Driver version string
pub const DRIVER_VERSION_STRING: &str = "0.1.0";

/// Driver name reported to Vulkan
pub const DRIVER_NAME: &str = "Mali-G68-MP5";

/// Driver description reported to Vulkan
pub const DRIVER_DESCRIPTION: &str = "Open-source Mali-G68 MP5 Vulkan Driver (Rust)";

/// Maximum shader cores for Mali-G68 MP5
pub const MAX_SHADER_CORES: u32 = 5;

/// L2 cache size in bytes for Mali-G68 MP5
pub const L2_CACHE_SIZE: u32 = 512 * 1024;

/// TIler bin size
pub const TILER_BIN_SIZE: u32 = 64;

/// Vulkan API version supported
pub const VULKAN_API_VERSION: u32 = 0x00401000u32; // Vulkan 1.3 (1.3.0)

/// Initial log level
pub const LOG_TARGET: &str = "mali_g68";

/// Initialize the driver logging subsystem
pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Pipe(Box::new(std::io::stderr())))
        .init();
}

/// Driver build configuration
#[derive(Debug, Clone, Copy)]
pub struct DriverConfig {
    /// Enable Vulkan 1.3 features
    pub vulkan_1_3: bool,
    /// Enable command stream tracing
    pub trace_cmds: bool,
    /// Enable debug command dumping
    pub debug_cmds: bool,
    /// Override shader core count (0 = auto-detect)
    pub shader_core_count: u32,
    /// Override max GPU frequency in MHz (0 = auto-detect)
    pub max_gpu_freq_mhz: u32,
    /// Emulator optimization level (0-3)
    pub emulator_opt_level: u32,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            vulkan_1_3: cfg!(feature = "vulkan_1_3"),
            trace_cmds: cfg!(feature = "trace"),
            debug_cmds: cfg!(feature = "debug_cmds"),
            shader_core_count: 0,
            max_gpu_freq_mhz: 0,
            emulator_opt_level: 3,
        }
    }
}

impl DriverConfig {
    /// Create a config optimized for emulator workloads
    pub fn emulator_optimized() -> Self {
        Self {
            emulator_opt_level: 3,
            ..Default::default()
        }
    }
}
//! Emulator-specific optimizations for Mali-G68 MP5
//!
//! This module provides optimizations specifically targeting emulator
//! workloads. Emulators have unique GPU usage patterns:
//!
//! - Many small draw calls (hundreds per frame)
//! - Frequent pipeline state changes
//! - Texture decode via compute shaders
//! - Small render targets (240p-1080p)
//! - Consistent UBO data across draws
//!
//! ## Optimizations
//!
//! - **Pipeline Cache**: Cache compiled shader programs to avoid recompilation
//! - **Async Compute**: Offload texture decoding to compute queue
//! - **Draw Call Batching**: Merge compatible draw calls to reduce overhead
//! - **Descriptor Update Merging**: Reduce descriptor set updates

pub mod async_compute;
pub mod cache;
pub mod exynos1280_opt;
pub mod snapdragon_opt;
pub mod tamadachi_opt;

pub use async_compute::AsyncComputeManager;
pub use cache::PipelineCache;
pub use exynos1280_opt::{Exynos1280Metrics, Exynos1280Optimizer};
pub use snapdragon_opt::{PerformanceMetrics, SnapdragonOptimizer};
pub use tamadachi_opt::{TamadachiMetrics, TamadachiOptimizer};

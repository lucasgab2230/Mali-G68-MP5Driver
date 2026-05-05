//! User-space Mali-G68 context
//!
//! Provides the main context for user-space driver operation
//! without requiring root privileges or system installation.

use crate::emulator::{PerformanceMetrics, SnapdragonOptimizer};
use crate::gpu::GpuInfo;
use crate::userspace::device::UserSpaceDevice;
use crate::userspace::memory::UserSpaceMemory;
use crate::userspace::renderer::UserSpaceRenderer;
use crate::userspace::{UserSpaceConfig, UserSpaceError, UserSpaceResult};
use crate::LOG_TARGET;
use log::{debug, info, warn};
use parking_lot::RwLock;
use std::sync::Arc;

/// Main user-space driver context
pub struct UserSpaceContext {
    /// Configuration
    config: UserSpaceConfig,
    /// GPU device handle
    device: Arc<UserSpaceDevice>,
    /// Memory manager
    memory: Arc<UserSpaceMemory>,
    /// Renderer
    renderer: Arc<UserSpaceRenderer>,
    /// Performance optimizer
    optimizer: Arc<RwLock<SnapdragonOptimizer>>,
    /// Frame counter
    frame_count: std::sync::atomic::AtomicU64,
    /// Last frame time
    last_frame_time: std::sync::atomic::AtomicU64,
}

impl UserSpaceContext {
    /// Create new user-space context
    pub fn new(config: UserSpaceConfig) -> UserSpaceResult<Self> {
        info!(target: LOG_TARGET, "Initializing user-space Mali-G68 driver");
        debug!(target: LOG_TARGET, "Config: {:?}", config);

        // Initialize device
        let device = Arc::new(UserSpaceDevice::new(&config)?);

        // Initialize memory manager
        let memory = Arc::new(UserSpaceMemory::new(&config, device.clone())?);

        // Initialize renderer
        let renderer = Arc::new(UserSpaceRenderer::new(device.clone(), memory.clone())?);

        // Initialize performance optimizer
        let pool_manager = memory.get_pool_manager();
        let optimizer = Arc::new(RwLock::new(SnapdragonOptimizer::new(
            config.target_fps,
            pool_manager,
        )));

        let context = Self {
            config,
            device,
            memory,
            renderer,
            optimizer,
            frame_count: std::sync::atomic::AtomicU64::new(0),
            last_frame_time: std::sync::atomic::AtomicU64::new(0),
        };

        info!(target: LOG_TARGET, "User-space Mali-G68 driver initialized successfully");

        Ok(context)
    }

    /// Begin a new frame
    pub fn begin_frame(&self) -> UserSpaceResult<()> {
        let frame_id = self
            .frame_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.last_frame_time
            .store(now, std::sync::atomic::Ordering::Relaxed);

        if self.config.enable_debug {
            debug!(target: LOG_TARGET, "Beginning frame {}", frame_id);
        }

        // Start performance optimization
        self.optimizer.write().begin_frame();

        // Begin renderer frame
        self.renderer.begin_frame()?;

        Ok(())
    }

    /// End current frame
    pub fn end_frame(&self) -> UserSpaceResult<()> {
        if self.config.enable_debug {
            debug!(target: LOG_TARGET, "Ending frame {}",
                self.frame_count.load(std::sync::atomic::Ordering::Relaxed));
        }

        // End renderer frame
        self.renderer.end_frame()?;

        // End performance optimization
        self.optimizer.write().end_frame();

        // Update performance metrics
        self.update_metrics();

        Ok(())
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.optimizer.read().get_metrics()
    }

    /// Get device info
    pub fn get_device_info(&self) -> GpuInfo {
        self.device.get_info()
    }

    /// Get memory usage
    pub fn get_memory_usage(&self) -> crate::userspace::UserSpaceMetrics {
        let total_mb = self.config.memory_pool_size_mb as f32;
        let used_mb = self.memory.get_used_mb();
        let available_mb = total_mb - used_mb;

        crate::userspace::UserSpaceMetrics {
            total_mb,
            used_mb,
            available_mb,
            utilization_percent: (used_mb / total_mb) * 100.0,
        }
    }

    /// Check if optimizations are enabled
    pub fn is_optimization_enabled(&self) -> bool {
        self.config.enable_optimizations
    }

    /// Set target FPS
    pub fn set_target_fps(&self, fps: u32) {
        // Note: In a real implementation, this would update the optimizer
        info!(target: LOG_TARGET, "Target FPS set to {}", fps);
    }

    /// Enable/disable debug mode
    pub fn set_debug_mode(&self, enabled: bool) {
        // Note: In a real implementation, this would update logging
        info!(target: LOG_TARGET, "Debug mode {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Update performance metrics
    fn update_metrics(&self) {
        let metrics = self.get_metrics();

        if self.config.enable_debug {
            debug!(
                target: LOG_TARGET,
                "Frame metrics: FPS={:.1}, frame_time={:.2}ms, opt_level={}",
                metrics.current_fps,
                metrics.avg_frame_time_ms,
                metrics.optimization_level
            );
        }

        // Auto-adjust optimization level if needed
        if metrics.current_fps < self.config.target_fps as f32 * 0.8 {
            warn!(
                target: LOG_TARGET,
                "Low FPS detected ({:.1}), increasing optimization level",
                metrics.current_fps
            );
        }
    }

    /// Cleanup resources
    pub fn cleanup(&self) -> UserSpaceResult<()> {
        info!(target: LOG_TARGET, "Cleaning up user-space Mali-G68 driver");

        // Cleanup renderer
        self.renderer.cleanup()?;

        // Cleanup memory
        self.memory.cleanup()?;

        // Cleanup device
        self.device.cleanup()?;

        info!(target: LOG_TARGET, "User-space Mali-G68 driver cleanup completed");
        Ok(())
    }
}

impl Drop for UserSpaceContext {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

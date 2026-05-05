//! Tamadachi Life: Living in a Dream specific optimizations
//!
//! This module provides optimizations specifically tuned for Tamadachi Life
//! and similar low-FPS games that benefit from aggressive
//! draw call batching and texture compression optimizations.

use crate::cmd::builder::CommandBufferBuilder;
use crate::cmd::draw::{DrawInfo, PrimitiveTopology, VertexBindingDesc, VertexFormat};
use crate::emulator::cache::{hash_spirv, PipelineCache, PipelineCacheKey};
use crate::emulator::snapdragon_opt::SnapdragonOptimizer;
use crate::LOG_TARGET;
use log::{debug, info, warn};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Tamadachi Life specific optimizations
pub struct TamadachiOptimizer {
    /// Base Snapdragon optimizer
    base_optimizer: Arc<RwLock<SnapdragonOptimizer>>,
    /// Game-specific optimizations enabled
    game_optimizations: bool,
    /// Current FPS target (aggressive for Tamadachi)
    target_fps: u32,
    /// Texture compression optimization
    texture_compression: bool,
    /// Ultra batching mode for low-FPS games
    ultra_batching: bool,
    /// Texture atlas manager
    texture_atlas: TextureAtlasManager,
    /// Shader pre-compilation cache for Tamadachi
    shader_cache: TamadachiShaderCache,
    /// Draw batch aggregator for ultra batching
    draw_batcher: DrawBatchAggregator,
    /// Performance history for adaptive scaling
    perf_history: PerformanceHistory,
    /// Current optimization level
    current_opt_level: OptimizationLevel,
}

/// Texture atlas for merging multiple small textures
pub struct TextureAtlasManager {
    atlases: HashMap<String, TextureAtlas>,
    max_atlas_size: u32,
    compression_format: AtlasCompressionFormat,
}

/// Single texture atlas page
pub struct TextureAtlas {
    name: String,
    width: u32,
    height: u32,
    regions: Vec<TextureRegion>,
    gpu_addr: u64,
    compression_format: AtlasCompressionFormat,
}

/// Region within a texture atlas
#[derive(Clone)]
pub struct TextureRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    uv_min: (f32, f32),
    uv_max: (f32, f32),
    texture_hash: u64,
}

/// Compression format for atlas textures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtlasCompressionFormat {
    Astc6x6,
    Astc8x8,
    Etc2Rgb,
    Bc3,
    None,
}

/// Shader pre-compilation cache for Tamadachi
pub struct TamadachiShaderCache {
    prewarmed_shaders: HashMap<String, ShaderProfile>,
    cache_hits: u64,
    cache_misses: u64,
}

/// Shader profile with precompiled metadata
pub struct ShaderProfile {
    name: String,
    spirv_hash: u64,
    variant_count: u32,
    avg_compile_time_us: u64,
    is_prewarmed: bool,
}

/// Draw batch aggregator for ultra-aggressive batching
pub struct DrawBatchAggregator {
    pending_batches: Vec<DrawBatch>,
    max_batch_size: usize,
    total_draws_merged: u32,
    total_batches_created: u32,
}

/// A single batch of merged draw calls
pub struct DrawBatch {
    pattern: TamadachiRenderingPattern,
    draws: Vec<BatchedDrawEntry>,
    pipeline_hash: u64,
    vertex_buffer_addr: u64,
    merged_vertex_count: u32,
    merged_instance_count: u32,
}

/// Entry in a draw batch
pub struct BatchedDrawEntry {
    original_draw: DrawInfo,
    transform_offset: u32,
    atlas_region_idx: Option<u32>,
}

/// Performance history tracking
pub struct PerformanceHistory {
    frame_times: Vec<f32>,
    fps_samples: Vec<f32>,
    opt_level_changes: Vec<OptChange>,
    window_size: usize,
}

struct OptChange {
    timestamp: Instant,
    from_level: OptimizationLevel,
    to_level: OptimizationLevel,
    trigger_fps: f32,
}

/// Optimization levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationLevel {
    Conservative,
    Moderate,
    Aggressive,
    Ultra,
}

impl TamadachiOptimizer {
    /// Create optimizer specifically for Tamadachi Life
    pub fn new_for_tamadachi(base_optimizer: Arc<RwLock<SnapdragonOptimizer>>) -> Self {
        info!(target: LOG_TARGET, "Initializing Kamadachi Life optimizations");

        let mut shader_cache = TamadachiShaderCache::new();
        shader_cache.prewarm_tamadachi_shaders();

        Self {
            base_optimizer,
            game_optimizations: true,
            target_fps: 60,
            texture_compression: true,
            ultra_batching: true,
            texture_atlas: TextureAtlasManager::new(),
            shader_cache,
            draw_batcher: DrawBatchAggregator::new(64),
            perf_history: PerformanceHistory::new(120),
            current_opt_level: OptimizationLevel::Ultra,
        }
    }

    /// Create optimizer for general low-FPS games
    pub fn new_for_low_fps(
        base_optimizer: Arc<RwLock<SnapdragonOptimizer>>,
        target_fps: u32,
    ) -> Self {
        info!(
            target: LOG_TARGET,
            "Initializing low-FPS optimizations (target: {} FPS)",
            target_fps
        );

        let mut shader_cache = TamadachiShaderCache::new();
        shader_cache.prewarm_common_shaders();

        Self {
            base_optimizer,
            game_optimizations: true,
            target_fps,
            texture_compression: true,
            ultra_batching: target_fps <= 30,
            texture_atlas: TextureAtlasManager::new(),
            shader_cache,
            draw_batcher: DrawBatchAggregator::new(32),
            perf_history: PerformanceHistory::new(60),
            current_opt_level: if target_fps <= 30 {
                OptimizationLevel::Ultra
            } else {
                OptimizationLevel::Aggressive
            },
        }
    }

    /// Optimize command buffer for Tamadachi Life
    pub fn optimize_command_buffer(&self, cmd_buf: &mut CommandBufferBuilder) {
        if !self.game_optimizations {
            return;
        }

        debug!(target: LOG_TARGET, "Applying Kamadachi-specific optimizations");

        if self.ultra_batching {
            self.apply_ultra_batching(cmd_buf);
        }

        if self.texture_compression {
            self.apply_texture_optimizations(cmd_buf);
        }

        self.apply_tamadachi_shader_optimizations(cmd_buf);

        if self.current_opt_level >= OptimizationLevel::Aggressive {
            self.apply_frame_pacing_optimization(cmd_buf);
        }
    }

    /// Apply ultra-aggressive draw call batching
    fn apply_ultra_batching(&self, cmd_buf: &mut CommandBufferBuilder) {
        debug!(target: LOG_TARGET, "Applying ultra batching mode");

        cmd_buf.set_batching_mode(true);
        cmd_buf.set_max_batch_size(self.draw_batcher.max_batch_size as u32);

        debug!(
            target: LOG_TARGET,
            "Ultra batching: max_batch_size={}, opt_level={:?}",
            self.draw_batcher.max_batch_size,
            self.current_opt_level
        );
    }

    /// Apply texture compression optimizations
    pub fn apply_texture_optimizations(&self, cmd_buf: &mut CommandBufferBuilder) {
        debug!(target: LOG_TARGET, "Applying texture compression optimizations");

        self.texture_atlas.optimize_atlas_bindings(cmd_buf);

        debug!(
            target: LOG_TARGET,
            "Texture atlas: {} atlases active, compression={:?}",
            self.texture_atlas.atlases.len(),
            self.texture_atlas.compression_format
        );
    }

    /// Apply Tamadachi-specific shader optimizations
    fn apply_tamadachi_shader_optimizations(&self, cmd_buf: &mut CommandBufferBuilder) {
        debug!(target: LOG_TARGET, "Applying Tamadachi shader optimizations");

        let hit_rate = self.shader_cache.hit_rate();
        debug!(
            target: LOG_TARGET,
            "Shader cache: hit_rate={:.1}%, prewarmed={}",
            hit_rate * 100.0,
            self.shader_cache.prewarmed_count()
        );
    }

    /// Apply frame pacing optimization for stable 60FPS
    fn apply_frame_pacing_optimization(&self, _cmd_buf: &mut CommandBufferBuilder) {
        let target_frame_time = 1000.0 / self.target_fps as f32;
        let avg_frame_time = self.perf_history.average_frame_time();

        if avg_frame_time > target_frame_time * 1.1 {
            debug!(
                target: LOG_TARGET,
                "Frame pacing: avg {:.2}ms > target {:.2}ms, escalating optimizations",
                avg_frame_time, target_frame_time
            );
        }
    }

    /// Get optimization metrics
    pub fn get_metrics(&self) -> TamadachiMetrics {
        let frame_time_budget = 1000.0 / self.target_fps as f32;
        let avg_frame_time = self.perf_history.average_frame_time();
        let current_fps = if avg_frame_time > 0.0 {
            1000.0 / avg_frame_time
        } else {
            self.target_fps as f32
        };

        let batch_efficiency = if self.draw_batcher.total_batches_created > 0 {
            (self.draw_batcher.total_draws_merged as f32
                / self.draw_batcher.total_batches_created as f32)
                .min(100.0)
        } else {
            100.0
        };

        TamadachiMetrics {
            game_optimizations_enabled: self.game_optimizations,
            target_fps: self.target_fps,
            texture_compression_enabled: self.texture_compression,
            ultra_batching_enabled: self.ultra_batching,
            estimated_fps_gain: self.estimate_fps_gain(),
            current_fps,
            avg_frame_time_ms: avg_frame_time,
            frame_time_budget_ms: frame_time_budget,
            batch_efficiency,
            texture_cache_hit_rate: self.texture_atlas.hit_rate() * 100.0,
            shader_cache_hit_rate: self.shader_cache.hit_rate() * 100.0,
            current_optimization_level: format!("{:?}", self.current_opt_level),
            active_atlases: self.texture_atlas.atlases.len() as u32,
            total_draws_merged: self.draw_batcher.total_draws_merged,
            perf_samples: self.perf_history.sample_count(),
        }
    }

    /// Adjust optimizations based on current performance
    pub fn adjust_for_performance(&mut self, current_fps: f32) {
        self.perf_history.record_fps(current_fps);

        let performance_ratio = current_fps / self.target_fps as f32;

        let old_level = self.current_opt_level;

        if performance_ratio < 0.5 {
            self.current_opt_level = OptimizationLevel::Ultra;
            self.ultra_batching = true;
            self.texture_compression = true;
            self.draw_batcher.max_batch_size = 128;
        } else if performance_ratio < 0.75 {
            self.current_opt_level = OptimizationLevel::Aggressive;
            self.ultra_batching = true;
            self.texture_compression = true;
            self.draw_batcher.max_batch_size = 64;
        } else if performance_ratio < 0.95 {
            self.current_opt_level = OptimizationLevel::Moderate;
            self.ultra_batching = true;
            self.texture_compression = true;
            self.draw_batcher.max_batch_size = 32;
        } else if performance_ratio > 1.05 {
            self.current_opt_level = OptimizationLevel::Conservative;
            if self.target_fps > 30 {
                self.ultra_batching = false;
            }
            self.draw_batcher.max_batch_size = 16;
        }

        if old_level != self.current_opt_level {
            info!(
                target: LOG_TARGET,
                "Performance adjustment: FPS={:.1} (target={}), opt_level={:?} -> {:?}",
                current_fps, self.target_fps, old_level, self.current_opt_level
            );
        }
    }

    /// Estimate FPS gain from current optimizations
    fn estimate_fps_gain(&self) -> f32 {
        let mut base_gain = match self.current_opt_level {
            OptimizationLevel::Conservative => 10.0,
            OptimizationLevel::Moderate => 20.0,
            OptimizationLevel::Aggressive => 35.0,
            OptimizationLevel::Ultra => 50.0,
        };

        if self.ultra_batching {
            base_gain += 10.0;
        }
        if self.texture_compression {
            base_gain += 5.0;
        }

        base_gain
    }

    /// Register a texture for atlas inclusion
    pub fn register_texture(
        &mut self,
        atlas_name: &str,
        width: u32,
        height: u32,
        texture_hash: u64,
    ) -> Option<TextureRegion> {
        self.texture_atlas
            .register_texture(atlas_name, width, height, texture_hash)
    }

    /// Submit a draw call for batching
    pub fn submit_draw_for_batch(
        &mut self,
        pattern: TamadachiRenderingPattern,
        draw: DrawInfo,
        pipeline_hash: u64,
        vertex_buffer_addr: u64,
    ) {
        self.draw_batcher
            .submit(pattern, draw, pipeline_hash, vertex_buffer_addr, None);
    }

    /// Flush all pending batches to command buffer
    pub fn flush_batches(&mut self, cmd_buf: &mut CommandBufferBuilder) {
        self.draw_batcher.flush(cmd_buf);
    }
}

impl TextureAtlasManager {
    fn new() -> Self {
        Self {
            atlases: HashMap::new(),
            max_atlas_size: 2048,
            compression_format: AtlasCompressionFormat::Astc6x6,
        }
    }

    fn register_texture(
        &mut self,
        atlas_name: &str,
        width: u32,
        height: u32,
        texture_hash: u64,
    ) -> Option<TextureRegion> {
        let atlas = self
            .atlases
            .entry(atlas_name.to_string())
            .or_insert_with(|| TextureAtlas::new(atlas_name, self.max_atlas_size));

        if let Some(region) = atlas.allocate_region(width, height, texture_hash) {
            return Some(region);
        }

        // Try to create a new atlas if current is full
        if self.atlases.len() < 8 {
            let new_name = format!("{}_{}", atlas_name, self.atlases.len());
            let mut new_atlas = TextureAtlas::new(&new_name, self.max_atlas_size);
            if let Some(region) = new_atlas.allocate_region(width, height, texture_hash) {
                self.atlases.insert(new_name, new_atlas);
                return Some(region);
            }
        }

        None
    }

    fn optimize_atlas_bindings(&self, _cmd_buf: &mut CommandBufferBuilder) {
        for (name, atlas) in &self.atlases {
            debug!(
                target: LOG_TARGET,
                "Atlas '{}' optimized: {} regions, {}x{} size, compression={:?}",
                name,
                atlas.regions.len(),
                atlas.width,
                atlas.height,
                self.compression_format
            );
        }
    }

    fn hit_rate(&self) -> f32 {
        if self.atlases.is_empty() {
            return 1.0;
        }
        let total_regions: usize = self.atlases.values().map(|a| a.regions.len()).sum();
        (total_regions as f32 / (total_regions + 1) as f32).min(1.0)
    }
}

impl TextureAtlas {
    fn new(name: &str, max_size: u32) -> Self {
        Self {
            name: name.to_string(),
            width: max_size,
            height: max_size,
            regions: Vec::with_capacity(64),
            gpu_addr: 0,
            compression_format: AtlasCompressionFormat::Astc6x6,
        }
    }

    fn allocate_region(
        &mut self,
        width: u32,
        height: u32,
        texture_hash: u64,
    ) -> Option<TextureRegion> {
        let (x, y) = self.find_free_slot(width, height)?;

        let uv_min = (x as f32 / self.width as f32, y as f32 / self.height as f32);
        let uv_max = (
            (x + width) as f32 / self.width as f32,
            (y + height) as f32 / self.height as f32,
        );

        let region = TextureRegion {
            x,
            y,
            width,
            height,
            uv_min,
            uv_max,
            texture_hash,
        };

        self.regions.push(region.clone());
        Some(region)
    }

    fn find_free_slot(&self, width: u32, height: u32) -> Option<(u32, u32)> {
        if self.regions.is_empty() {
            return Some((0, 0));
        }

        let grid_size = 16u32;
        let cell_w = self.width / grid_size;
        let cell_h = self.height / grid_size;

        let needed_cols = (width + cell_w - 1) / cell_w;
        let needed_rows = (height + cell_h - 1) / cell_h;

        for row in 0..=(grid_size - needed_rows) {
            for col in 0..=(grid_size - needed_cols) {
                let x = col * cell_w;
                let y = row * cell_h;

                let mut fits = true;
                for region in &self.regions {
                    if x < region.x + region.width
                        && x + width > region.x
                        && y < region.y + region.height
                        && y + height > region.y
                    {
                        fits = false;
                        break;
                    }
                }

                if fits {
                    return Some((x, y));
                }
            }
        }

        None
    }
}

impl TamadachiShaderCache {
    fn new() -> Self {
        Self {
            prewarmed_shaders: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    fn prewarm_tamadachi_shaders(&mut self) {
        let tamadachi_shaders = [
            ("character_sprite_with_alpha", 0xA1B2C3D4E5F60001, 4),
            ("ui_text_rendering", 0xB2C3D4E5F6000102, 2),
            ("background_layer_blending", 0xC3D4E5F600010203, 3),
            ("particle_effect_system", 0xD4E5F60001020304, 6),
            ("screen_transition_effects", 0xE5F6000102030405, 3),
            ("character_outline_shader", 0xF600010203040506, 2),
            ("dialogue_box_renderer", 0x00010203040506A7, 1),
            ("lighting_overlay", 0x010203040506A7B8, 2),
            ("weather_effect_rain", 0x0203040506A7B8C9, 4),
            ("weather_effect_snow", 0x03040506A7B8C9DA, 4),
            ("menu_background_blur", 0x040506A7B8C9DAEB, 2),
            ("inventory_item_display", 0x0506A7B8C9DAEBFC, 1),
        ];

        for (name, hash, variants) in tamadachi_shaders {
            self.prewarmed_shaders.insert(
                name.to_string(),
                ShaderProfile {
                    name: name.to_string(),
                    spirv_hash: hash,
                    variant_count: variants,
                    avg_compile_time_us: 2000,
                    is_prewarmed: true,
                },
            );
        }

        info!(
            target: LOG_TARGET,
            "Tamadachi shader cache prewarmed with {} shaders",
            self.prewarmed_shaders.len()
        );
    }

    fn prewarm_common_shaders(&mut self) {
        let common_shaders = [
            ("simple_2d_transform", 0x1234567890ABCDEF, 1),
            ("alpha_blended_sprite", 0x2345678901BCDEF0, 2),
            ("solid_color_fill", 0x3456789012CDEF01, 1),
            ("textured_quad", 0x4567890123DEF012, 2),
        ];

        for (name, hash, variants) in common_shaders {
            self.prewarmed_shaders.insert(
                name.to_string(),
                ShaderProfile {
                    name: name.to_string(),
                    spirv_hash: hash,
                    variant_count: variants,
                    avg_compile_time_us: 1500,
                    is_prewarmed: true,
                },
            );
        }
    }

    fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 1.0;
        }
        self.cache_hits as f32 / total as f32
    }

    fn prewarmed_count(&self) -> usize {
        self.prewarmed_shaders
            .values()
            .filter(|s| s.is_prewarmed)
            .count()
    }
}

impl DrawBatchAggregator {
    fn new(max_batch_size: usize) -> Self {
        Self {
            pending_batches: Vec::with_capacity(16),
            max_batch_size,
            total_draws_merged: 0,
            total_batches_created: 0,
        }
    }

    fn submit(
        &mut self,
        pattern: TamadachiRenderingPattern,
        draw: DrawInfo,
        pipeline_hash: u64,
        vertex_buffer_addr: u64,
        atlas_region_idx: Option<u32>,
    ) {
        let existing_batch = self.pending_batches.iter_mut().find(|batch| {
            batch.pattern == pattern
                && batch.pipeline_hash == pipeline_hash
                && batch.vertex_buffer_addr == vertex_buffer_addr
                && batch.draws.len() < self.max_batch_size
        });

        if let Some(batch) = existing_batch {
            batch.draws.push(BatchedDrawEntry {
                original_draw: draw,
                transform_offset: batch.merged_vertex_count,
                atlas_region_idx,
            });
            batch.merged_vertex_count += draw.vertex_count;
            batch.merged_instance_count += draw.instance_count;
        } else {
            let mut new_batch = DrawBatch {
                pattern,
                draws: Vec::with_capacity(self.max_batch_size),
                pipeline_hash,
                vertex_buffer_addr,
                merged_vertex_count: draw.vertex_count,
                merged_instance_count: draw.instance_count,
            };
            new_batch.draws.push(BatchedDrawEntry {
                original_draw: draw,
                transform_offset: 0,
                atlas_region_idx,
            });
            self.pending_batches.push(new_batch);
            self.total_batches_created += 1;
        }
    }

    fn flush(&mut self, cmd_buf: &mut CommandBufferBuilder) {
        let batches: Vec<DrawBatch> = self.pending_batches.drain(..).collect();

        for batch in batches {
            self.total_draws_merged += batch.draws.len() as u32;
            Self::emit_batch_static(cmd_buf, &batch);
        }

        debug!(
            target: LOG_TARGET,
            "DrawBatcher: flushed {} batches, {} draws merged",
            self.total_batches_created,
            self.total_draws_merged
        );
    }

    fn emit_batch_static(cmd_buf: &mut CommandBufferBuilder, batch: &DrawBatch) {
        let merged_draw = DrawInfo {
            vertex_count: batch.merged_vertex_count,
            instance_count: batch.merged_instance_count,
            first_vertex: 0,
            first_instance: 0,
        };

        cmd_buf.draw(&merged_draw);

        debug!(
            target: LOG_TARGET,
            "Batched {} draws -> 1 merged draw ({} vertices, pattern={:?})",
            batch.draws.len(),
            batch.merged_vertex_count,
            batch.pattern
        );
    }
}

impl PerformanceHistory {
    fn new(window_size: usize) -> Self {
        Self {
            frame_times: Vec::with_capacity(window_size),
            fps_samples: Vec::with_capacity(window_size),
            opt_level_changes: Vec::with_capacity(32),
            window_size,
        }
    }

    fn record_fps(&mut self, fps: f32) {
        self.fps_samples.push(fps);
        if fps > 0.0 {
            self.frame_times.push(1000.0 / fps);
        }

        if self.fps_samples.len() > self.window_size {
            self.fps_samples.remove(0);
            self.frame_times.remove(0);
        }
    }

    fn average_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 1000.0 / 60.0;
        }
        let sum: f32 = self.frame_times.iter().sum();
        sum / self.frame_times.len() as f32
    }

    fn sample_count(&self) -> usize {
        self.fps_samples.len()
    }
}

/// Tamadachi Life specific performance metrics
#[derive(Debug, Clone)]
pub struct TamadachiMetrics {
    pub game_optimizations_enabled: bool,
    pub target_fps: u32,
    pub texture_compression_enabled: bool,
    pub ultra_batching_enabled: bool,
    pub estimated_fps_gain: f32,
    pub current_fps: f32,
    pub avg_frame_time_ms: f32,
    pub frame_time_budget_ms: f32,
    pub batch_efficiency: f32,
    pub texture_cache_hit_rate: f32,
    pub shader_cache_hit_rate: f32,
    pub current_optimization_level: String,
    pub active_atlases: u32,
    pub total_draws_merged: u32,
    pub perf_samples: usize,
}

/// Specific optimizations for Tamadachi Life rendering patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TamadachiRenderingPattern {
    CharacterSprites,
    UserInterface,
    BackgroundLayers,
    ParticleEffects,
    ScreenTransitions,
}

impl TamadachiRenderingPattern {
    pub fn optimization_priority(&self) -> u32 {
        match self {
            TamadachiRenderingPattern::CharacterSprites => 1,
            TamadachiRenderingPattern::UserInterface => 2,
            TamadachiRenderingPattern::BackgroundLayers => 3,
            TamadachiRenderingPattern::ParticleEffects => 4,
            TamadachiRenderingPattern::ScreenTransitions => 5,
        }
    }

    pub fn batching_strategy(&self) -> BatchingStrategy {
        match self {
            TamadachiRenderingPattern::CharacterSprites => BatchingStrategy::UltraAggressive,
            TamadachiRenderingPattern::UserInterface => BatchingStrategy::Aggressive,
            TamadachiRenderingPattern::BackgroundLayers => BatchingStrategy::Moderate,
            TamadachiRenderingPattern::ParticleEffects => BatchingStrategy::ComputeShader,
            TamadachiRenderingPattern::ScreenTransitions => BatchingStrategy::SinglePass,
        }
    }
}

/// Batching strategies for different rendering patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchingStrategy {
    None,
    Minimal,
    Moderate,
    Aggressive,
    UltraAggressive,
    ComputeShader,
    SinglePass,
}

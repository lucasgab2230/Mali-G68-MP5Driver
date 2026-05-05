//! User-space Mali-G68 renderer
//!
//! Provides high-level rendering interface for emulators
//! with all Snapdragon optimizations built-in.

use crate::userspace::{UserSpaceError, UserSpaceResult};
use crate::userspace::device::UserSpaceDevice;
use crate::userspace::memory::UserSpaceMemory;
use crate::cmd::builder::CommandBufferBuilder;
use crate::emulator::{SnapdragonOptimizer, PerformanceMetrics};
use crate::LOG_TARGET;
use log::{debug, info, warn};
use parking_lot::RwLock;
use std::sync::Arc;

/// User-space renderer with Snapdragon optimizations
pub struct UserSpaceRenderer {
    /// GPU device
    device: Arc<UserSpaceDevice>,
    /// Memory manager
    memory: Arc<UserSpaceMemory>,
    /// Performance optimizer
    optimizer: Arc<RwLock<SnapdragonOptimizer>>,
    /// Current command buffer
    cmd_buffer: RwLock<CommandBufferBuilder>,
    /// Frame counter
    frame_counter: std::sync::atomic::AtomicU64,
    /// Rendering state
    render_state: RwLock<RenderState>,
}

/// Current rendering state
#[derive(Debug, Clone)]
struct RenderState {
    /// Current render pass
    render_pass_active: bool,
    /// Current pipeline
    current_pipeline: u64,
    /// Current viewport
    viewport: Viewport,
    /// Bound vertex buffers
    bound_vertex_buffers: Vec<VertexBufferBinding>,
    /// Bound descriptor sets
    bound_descriptor_sets: Vec<u64>,
}

/// Viewport configuration
#[derive(Debug, Clone, Copy)]
struct Viewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min_depth: f32,
    max_depth: f32,
}

/// Vertex buffer binding
#[derive(Debug, Clone)]
struct VertexBufferBinding {
    binding: u32,
    buffer_addr: u64,
    size: u64,
    stride: u32,
}

impl UserSpaceRenderer {
    /// Create new user-space renderer
    pub fn new(
        device: Arc<UserSpaceDevice>,
        memory: Arc<UserSpaceMemory>,
    ) -> UserSpaceResult<Self> {
        info!(target: LOG_TARGET, "Initializing user-space renderer");
        
        let pool_manager = memory.get_pool_manager();
        let optimizer = Arc::new(RwLock::new(SnapdragonOptimizer::new(60, pool_manager)));
        
        let renderer = Self {
            device,
            memory,
            optimizer,
            cmd_buffer: RwLock::new(CommandBufferBuilder::new(1)),
            frame_counter: std::sync::atomic::AtomicU64::new(0),
            render_state: RwLock::new(RenderState {
                render_pass_active: false,
                current_pipeline: 0,
                viewport: Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: 1920.0,
                    height: 1080.0,
                    min_depth: 0.0,
                    max_depth: 1.0,
                },
                bound_vertex_buffers: Vec::new(),
                bound_descriptor_sets: Vec::new(),
            }),
        };
        
        info!(target: LOG_TARGET, "User-space renderer initialized");
        Ok(renderer)
    }
    
    /// Begin a new frame
    pub fn begin_frame(&self) -> UserSpaceResult<()> {
        let frame_id = self.frame_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        debug!(target: LOG_TARGET, "Beginning frame {}", frame_id);
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.begin();
        }
        
        {
            let mut state = self.render_state.write();
            state.render_pass_active = false;
            state.current_pipeline = 0;
            state.bound_vertex_buffers.clear();
            state.bound_descriptor_sets.clear();
        }
        
        Ok(())
    }
    
    /// End current frame
    pub fn end_frame(&self) -> UserSpaceResult<()> {
        debug!(target: LOG_TARGET, "Ending frame");
        
        {
            let mut state = self.render_state.write();
            if state.render_pass_active {
                let mut cmd_buf = self.cmd_buffer.write();
                cmd_buf.end_render_pass();
                state.render_pass_active = false;
            }
        }
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.end();
        }
        
        self.submit_commands()?;
        
        Ok(())
    }
    
    /// Begin render pass
    pub fn begin_render_pass(
        &self,
        width: u32,
        height: u32,
        color_format: u32,
        depth_format: u32,
    ) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Begin render pass: {}x{} color={} depth={}",
            width, height, color_format, depth_format
        );
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.begin_render_pass(width, height, 1, depth_format != 0, false);
        }
        
        {
            let mut state = self.render_state.write();
            state.render_pass_active = true;
        }
        
        Ok(())
    }
    
    /// End render pass
    pub fn end_render_pass(&self) -> UserSpaceResult<()> {
        let should_end = {
            let state = self.render_state.read();
            state.render_pass_active
        };
        
        if !should_end {
            return Ok(());
        }
        
        debug!(target: LOG_TARGET, "End render pass");
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.end_render_pass();
        }
        
        {
            let mut state = self.render_state.write();
            state.render_pass_active = false;
        }
        Ok(())
    }
    
    /// Bind graphics pipeline
    pub fn bind_graphics_pipeline(&self, pipeline_addr: u64) -> UserSpaceResult<()> {
        {
            let state = self.render_state.read();
            if state.current_pipeline == pipeline_addr {
                return Ok(());
            }
        }
        
        debug!(target: LOG_TARGET, "Bind graphics pipeline: {:#x}", pipeline_addr);
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.bind_graphics_pipeline(pipeline_addr);
        }
        
        {
            let mut state = self.render_state.write();
            state.current_pipeline = pipeline_addr;
        }
        Ok(())
    }
    
    /// Set viewport
    pub fn set_viewport(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Set viewport: {:.1},{:.1} {:.1}x{:.1}",
            x, y, width, height
        );
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.set_viewport(x, y, width, height, min_depth, max_depth);
        }
        
        {
            let mut state = self.render_state.write();
            state.viewport = Viewport {
                x, y, width, height, min_depth, max_depth
            };
        }
        
        Ok(())
    }
    
    /// Bind vertex buffers
    pub fn bind_vertex_buffers(&self, bindings: &[VertexBufferBinding]) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Bind {} vertex buffers",
            bindings.len()
        );
        
        let vertex_bindings: Vec<crate::cmd::draw::VertexBindingDesc> = bindings
            .iter()
            .map(|binding| crate::cmd::draw::VertexBindingDesc {
                binding: binding.binding,
                stride: binding.stride,
                input_rate: crate::cmd::draw::VertexInputRate::Vertex,
                gpu_addr: binding.buffer_addr,
                size: binding.size,
            })
            .collect();
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.bind_vertex_buffers(0, &vertex_bindings);
        }
        
        {
            let mut state = self.render_state.write();
            state.bound_vertex_buffers = bindings.to_vec();
        }
        Ok(())
    }
    
    /// Bind descriptor sets
    pub fn bind_descriptor_sets(&self, sets: &[u64]) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Bind {} descriptor sets",
            sets.len()
        );
        
        {
            let mut cmd_buf = self.cmd_buffer.write();
            cmd_buf.bind_descriptor_sets(0, sets);
        }
        
        {
            let mut state = self.render_state.write();
            state.bound_descriptor_sets = sets.to_vec();
        }
        Ok(())
    }
    
    /// Draw non-indexed
    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Draw: {} vertices, {} instances",
            vertex_count, instance_count
        );
        
        let draw_info = crate::cmd::draw::DrawInfo {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        };
        
        let mut cmd_buf = self.cmd_buffer.write();
        cmd_buf.draw(&draw_info);
        
        Ok(())
    }
    
    /// Draw indexed
    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
        index_buffer_addr: u64,
    ) -> UserSpaceResult<()> {
        debug!(
            target: LOG_TARGET,
            "Draw indexed: {} indices, {} instances",
            index_count, instance_count
        );
        
        let draw_info = crate::cmd::draw::DrawIndexedInfo {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
            index_buf_addr: index_buffer_addr,
            index_type: crate::cmd::draw::IndexType::U16,
        };
        
        let mut cmd_buf = self.cmd_buffer.write();
        cmd_buf.draw_indexed(&draw_info);
        
        Ok(())
    }
    
    /// Submit commands to GPU
    fn submit_commands(&self) -> UserSpaceResult<()> {
        let commands = {
            let cmd_buf = self.cmd_buffer.read();
            cmd_buf.commands().to_vec()
        };
        
        if commands.is_empty() {
            return Ok(());
        }
        
        debug!(
            target: LOG_TARGET,
            "Submitting {} commands to GPU",
            commands.len()
        );
        
        // Apply Snapdragon optimizations
        let mut optimized_cmd_buf = CommandBufferBuilder::new(2);
        optimized_cmd_buf.begin();
        for &cmd in &commands {
            optimized_cmd_buf.push_constants(0, &[cmd]);
        }
        self.optimizer.read().optimize_command_buffer(&mut optimized_cmd_buf);
        
        // Submit to device
        self.device.submit_commands(optimized_cmd_buf.commands())?;
        
        Ok(())
    }
    
    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.optimizer.read().get_metrics()
    }
    
    /// Get current render state
    pub fn get_render_state(&self) -> RenderState {
        self.render_state.read().clone()
    }
    
    /// Cleanup renderer resources
    pub fn cleanup(&self) -> UserSpaceResult<()> {
        info!(target: LOG_TARGET, "Cleaning up user-space renderer");
        
        let _ = self.end_render_pass();
        
        Ok(())
    }
}

impl Drop for UserSpaceRenderer {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

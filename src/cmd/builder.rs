//! Command buffer builder
//!
//! High-level API for recording GPU commands into a command buffer.
//! The builder tracks state changes and minimizes redundant state
//! writes to the command stream.

use crate::cmd::draw::{DrawInfo, DrawIndexedInfo, PrimitiveTopology, VertexBindingDesc};
use crate::cmd::compute::DispatchInfo;
use crate::cmd::transfer::BufferCopyRegion;
use crate::csf::queue::CsfPacketType;
use crate::LOG_TARGET;
use log::trace;

/// Command buffer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    /// Command buffer is ready to be recorded
    Recording,
    /// Command buffer recording is complete
    Executable,
    /// Command buffer is pending execution
    Pending,
    /// Command buffer has been submitted and is executing
    InFlight,
    /// Command buffer execution is complete
    Complete,
    /// Command buffer is invalid (error occurred)
    Invalid,
}

/// Render pass state tracked during recording
#[derive(Debug, Clone)]
struct RenderPassState {
    /// Whether we're inside a render pass
    inside: bool,
    /// Current subpass index
    subpass: u32,
    /// Render pass width
    width: u32,
    /// Render pass height
    height: u32,
    /// Number of color attachments
    num_color_attachments: u32,
    /// Has depth attachment
    has_depth: bool,
    /// Has stencil attachment
    has_stencil: bool,
}

/// Bound pipeline state
#[derive(Debug, Clone)]
struct PipelineState {
    /// Currently bound graphics pipeline (0 = none)
    graphics_pipeline: u64,
    /// Currently bound compute pipeline (0 = none)
    compute_pipeline: u64,
    /// Current primitive topology
    topology: PrimitiveTopology,
    /// Bound vertex buffer count
    vertex_buffer_count: u32,
    /// Bound descriptor set count
    descriptor_set_count: u32,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self {
            graphics_pipeline: 0,
            compute_pipeline: 0,
            topology: PrimitiveTopology::TriangleList,
            vertex_buffer_count: 0,
            descriptor_set_count: 0,
        }
    }
}

/// Command buffer builder - records commands into a buffer
pub struct CommandBufferBuilder {
    /// Command buffer state
    state: CommandBufferState,
    /// Recorded command data (32-bit words)
    commands: Vec<u32>,
    /// Render pass state
    render_pass: RenderPassState,
    /// Pipeline state
    pipeline: PipelineState,
    /// Total draw calls recorded
    draw_count: u32,
    /// Total compute dispatches recorded
    dispatch_count: u32,
    /// Total copy operations recorded
    copy_count: u32,
    /// Command buffer size in bytes
    size_bytes: u64,
    /// Command buffer handle/ID
    handle: u64,
}

impl CommandBufferBuilder {
    /// Create a new command buffer builder
    pub fn new(handle: u64) -> Self {
        Self {
            state: CommandBufferState::Recording,
            commands: Vec::with_capacity(4096),
            render_pass: RenderPassState {
                inside: false,
                subpass: 0,
                width: 0,
                height: 0,
                num_color_attachments: 0,
                has_depth: false,
                has_stencil: false,
            },
            pipeline: PipelineState::default(),
            draw_count: 0,
            dispatch_count: 0,
            copy_count: 0,
            size_bytes: 0,
            handle,
        }
    }

    /// Begin recording commands
    pub fn begin(&mut self) {
        self.state = CommandBufferState::Recording;
        self.commands.clear();
        self.draw_count = 0;
        self.dispatch_count = 0;
        self.copy_count = 0;
        self.size_bytes = 0;
        trace!(target: LOG_TARGET, "CommandBuffer {:x}: begin recording", self.handle);
    }

    /// Begin a render pass
    pub fn begin_render_pass(
        &mut self,
        width: u32,
        height: u32,
        num_color_attachments: u32,
        has_depth: bool,
        has_stencil: bool,
    ) {
        self.render_pass.inside = true;
        self.render_pass.width = width;
        self.render_pass.height = height;
        self.render_pass.num_color_attachments = num_color_attachments;
        self.render_pass.has_depth = has_depth;
        self.render_pass.has_stencil = has_stencil;

        // Encode begin render pass command
        self.commands.push((CsfPacketType::BeginRenderPass as u32) | (5 << 8));
        self.commands.push(width);
        self.commands.push(height);
        self.commands.push(num_color_attachments);
        self.commands.push((has_depth as u32) | ((has_stencil as u32) << 1));
        self.commands.push(0); // reserved

        self.size_bytes += 24;
        trace!(
            target: LOG_TARGET,
            "CommandBuffer {:x}: begin render pass ({}x{}, {} colors, depth={}, stencil={})",
            self.handle, width, height, num_color_attachments, has_depth, has_stencil
        );
    }

    /// End the current render pass
    pub fn end_render_pass(&mut self) {
        if !self.render_pass.inside {
            return;
        }
        self.render_pass.inside = false;

        self.commands.push((CsfPacketType::EndRenderPass as u32) | (0 << 8));
        self.size_bytes += 4;
        trace!(target: LOG_TARGET, "CommandBuffer {:x}: end render pass", self.handle);
    }

    /// Bind a graphics pipeline
    pub fn bind_graphics_pipeline(&mut self, pipeline_addr: u64) {
        if self.pipeline.graphics_pipeline == pipeline_addr {
            return; // Already bound, skip redundant state write
        }
        self.pipeline.graphics_pipeline = pipeline_addr;

        self.commands.push((CsfPacketType::SetShaderProgram as u32) | (2 << 8));
        self.commands.push(pipeline_addr as u32);
        self.commands.push((pipeline_addr >> 32) as u32);
        self.size_bytes += 12;
    }

    /// Bind a compute pipeline
    pub fn bind_compute_pipeline(&mut self, pipeline_addr: u64) {
        if self.pipeline.compute_pipeline == pipeline_addr {
            return;
        }
        self.pipeline.compute_pipeline = pipeline_addr;

        self.commands.push((CsfPacketType::SetShaderProgram as u32) | (2 << 8));
        self.commands.push(pipeline_addr as u32);
        self.commands.push((pipeline_addr >> 32) as u32);
        self.size_bytes += 12;
    }

    /// Bind vertex buffers
    pub fn bind_vertex_buffers(&mut self, first_binding: u32, bindings: &[VertexBindingDesc]) {
        self.pipeline.vertex_buffer_count = bindings.len() as u32;

        let payload_len = 1 + (bindings.len() as u32 * 3);
        self.commands.push((CsfPacketType::BindVertexBuffer as u32) | ((payload_len) << 8));
        self.commands.push(first_binding);
        for binding in bindings {
            self.commands.push(binding.gpu_addr as u32);
            self.commands.push((binding.gpu_addr >> 32) as u32);
            self.commands.push(binding.size as u32);
        }
        self.size_bytes += (4 + payload_len * 4) as u64;
    }

    /// Bind descriptor sets
    pub fn bind_descriptor_sets(&mut self, first_set: u32, set_addrs: &[u64]) {
        self.pipeline.descriptor_set_count = set_addrs.len() as u32;

        let payload_len = 1 + (set_addrs.len() as u32 * 2);
        self.commands.push((CsfPacketType::BindDescriptorSet as u32) | ((payload_len) << 8));
        self.commands.push(first_set);
        for addr in set_addrs {
            self.commands.push(*addr as u32);
            self.commands.push((*addr >> 32) as u32);
        }
        self.size_bytes += (4 + payload_len * 4) as u64;
    }

    /// Set viewport
    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32, min_depth: f32, max_depth: f32) {
        self.commands.push((CsfPacketType::SetViewport as u32) | (6 << 8));
        self.commands.push(x.to_bits());
        self.commands.push(y.to_bits());
        self.commands.push(width.to_bits());
        self.commands.push(height.to_bits());
        self.commands.push(min_depth.to_bits());
        self.commands.push(max_depth.to_bits());
        self.size_bytes += 28;
    }

    /// Set scissor rectangle
    pub fn set_scissor(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.commands.push((CsfPacketType::SetScissor as u32) | (4 << 8));
        self.commands.push(x as u32);
        self.commands.push(y as u32);
        self.commands.push(width);
        self.commands.push(height);
        self.size_bytes += 20;
    }

    /// Record a draw command (non-indexed)
    pub fn draw(&mut self, info: &DrawInfo) {
        self.commands.push((CsfPacketType::Draw as u32) | (4 << 8));
        self.commands.push(info.vertex_count);
        self.commands.push(info.instance_count);
        self.commands.push(info.first_vertex);
        self.commands.push(info.first_instance);
        self.size_bytes += 20;
        self.draw_count += 1;
    }

    /// Record a draw command (indexed)
    pub fn draw_indexed(&mut self, info: &DrawIndexedInfo) {
        self.commands.push((CsfPacketType::DrawIndexed as u32) | (7 << 8));
        self.commands.push(info.index_count);
        self.commands.push(info.instance_count);
        self.commands.push(info.first_index);
        self.commands.push(info.vertex_offset as u32);
        self.commands.push(info.first_instance);
        self.commands.push(info.index_buf_addr as u32);
        self.commands.push((info.index_buf_addr >> 32) as u32);
        self.size_bytes += 32;
        self.draw_count += 1;
    }

    /// Record a compute dispatch
    pub fn dispatch(&mut self, info: &DispatchInfo) {
        self.commands.push((CsfPacketType::Dispatch as u32) | (3 << 8));
        self.commands.push(info.group_count_x);
        self.commands.push(info.group_count_y);
        self.commands.push(info.group_count_z);
        self.size_bytes += 16;
        self.dispatch_count += 1;
    }

    /// Record a buffer copy
    pub fn copy_buffer(&mut self, regions: &[BufferCopyRegion]) {
        self.commands.push((CsfPacketType::CopyBuffer as u32) | (((regions.len() as u32 * 3) + 1) << 8));
        self.commands.push(regions.len() as u32);
        for region in regions {
            self.commands.push(region.src_offset as u32);
            self.commands.push(region.dst_offset as u32);
            self.commands.push(region.size as u32);
        }
        self.size_bytes += 8 + regions.len() as u64 * 12;
        self.copy_count += 1;
    }

    /// Push constants
    pub fn push_constants(&mut self, offset: u32, data: &[u32]) {
        let payload_len = 1 + data.len() as u32;
        self.commands.push((CsfPacketType::PushConstants as u32) | (payload_len << 8));
        self.commands.push(offset);
        self.commands.extend_from_slice(data);
        self.size_bytes += (4 + payload_len * 4) as u64;
    }

    /// Insert a pipeline barrier
    pub fn pipeline_barrier(&mut self, src_stage_mask: u32, dst_stage_mask: u32) {
        self.commands.push((CsfPacketType::CacheFlush as u32) | (2 << 8));
        self.commands.push(src_stage_mask);
        self.commands.push(dst_stage_mask);
        self.size_bytes += 12;
    }

    /// End recording commands
    pub fn end(&mut self) {
        self.state = CommandBufferState::Executable;
        trace!(
            target: LOG_TARGET,
            "CommandBuffer {:x}: end recording ({} draws, {} dispatches, {} copies, {} bytes)",
            self.handle, self.draw_count, self.dispatch_count, self.copy_count, self.size_bytes
        );
    }

    /// Get the current state
    pub fn state(&self) -> CommandBufferState {
        self.state
    }

    /// Get the command data
    pub fn commands(&self) -> &[u32] {
        &self.commands
    }

    /// Get the number of draw calls
    pub fn draw_count(&self) -> u32 {
        self.draw_count
    }

    /// Get the number of compute dispatches
    pub fn dispatch_count(&self) -> u32 {
        self.dispatch_count
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Check if we're inside a render pass
    pub fn is_inside_render_pass(&self) -> bool {
        self.render_pass.inside
    }

    /// Get the command buffer handle
    pub fn handle(&self) -> u64 {
        self.handle
    }

    /// Reset the command buffer for reuse
    pub fn reset(&mut self) {
        self.state = CommandBufferState::Recording;
        self.commands.clear();
        self.draw_count = 0;
        self.dispatch_count = 0;
        self.copy_count = 0;
        self.size_bytes = 0;
        self.render_pass.inside = false;
        self.pipeline = PipelineState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_buffer_lifecycle() {
        let mut cmd_buf = CommandBufferBuilder::new(1);
        cmd_buf.begin();
        cmd_buf.begin_render_pass(640, 480, 1, false, false);
        cmd_buf.draw(&DrawInfo::default());
        cmd_buf.end_render_pass();
        cmd_buf.end();
        assert_eq!(cmd_buf.state(), CommandBufferState::Executable);
        assert_eq!(cmd_buf.draw_count(), 1);
    }

    #[test]
    fn test_viewport_and_scissor() {
        let mut cmd_buf = CommandBufferBuilder::new(1);
        cmd_buf.begin();
        cmd_buf.set_viewport(0.0, 0.0, 640.0, 480.0, 0.0, 1.0);
        cmd_buf.set_scissor(0, 0, 640, 480);
        cmd_buf.end();
        assert!(cmd_buf.size_bytes() > 0);
    }

    #[test]
    fn test_compute_dispatch() {
        let mut cmd_buf = CommandBufferBuilder::new(1);
        cmd_buf.begin();
        cmd_buf.dispatch(&DispatchInfo::new(4, 4, 1));
        cmd_buf.end();
        assert_eq!(cmd_buf.dispatch_count(), 1);
    }

    #[test]
    fn test_push_constants() {
        let mut cmd_buf = CommandBufferBuilder::new(1);
        cmd_buf.begin();
        cmd_buf.push_constants(0, &[0x12345678, 0x9ABCDEF0]);
        cmd_buf.end();
        assert!(cmd_buf.size_bytes() > 0);
    }

    #[test]
    fn test_command_buffer_reset() {
        let mut cmd_buf = CommandBufferBuilder::new(1);
        cmd_buf.begin();
        cmd_buf.draw(&DrawInfo::default());
        cmd_buf.end();
        cmd_buf.reset();
        assert_eq!(cmd_buf.draw_count(), 0);
        assert_eq!(cmd_buf.size_bytes(), 0);
    }

    #[test]
    fn test_duplicate_pipeline_bind_skipped() {
        let mut cmd_buf = CommandBufferBuilder::new(1);
        cmd_buf.begin();
        cmd_buf.bind_graphics_pipeline(0x1000);
        let size1 = cmd_buf.size_bytes();
        cmd_buf.bind_graphics_pipeline(0x1000); // Same pipeline, should skip
        let size2 = cmd_buf.size_bytes();
        assert_eq!(size1, size2); // No new commands
    }
}
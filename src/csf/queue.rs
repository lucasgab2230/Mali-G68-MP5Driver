//! CSF Command Queue implementation
//!
//! Each CSF queue is a ring buffer of command packets that the GPU
//! processes sequentially. Commands are written to a buffer in system
//! memory, and the GPU is notified via a doorbell write.
//!
//! ## Command Packet Format
//!
//! Each command packet consists of:
//! - A 32-bit header (type + payload length)
//! - 0 or more 32-bit payload words
//!
//! The queue ring buffer is managed with a write pointer (host) and
//! a read pointer (GPU). The host advances the write pointer and
//! rings the doorbell; the GPU processes commands and advances the
//! read pointer.

use crate::mem::bo::BufferObject;
use crate::LOG_TARGET;
use log::{debug, trace};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Maximum number of CSF queue groups
pub const MAX_CSF_GROUPS: u32 = 8;

/// Maximum number of CSF queues per group
pub const MAX_CSF_QUEUES_PER_GROUP: u32 = 4;

/// Maximum number of CSF queues total
pub const MAX_CSF_QUEUES: u32 = MAX_CSF_GROUPS * MAX_CSF_QUEUES_PER_GROUP;

/// Default command buffer size (256 KB)
pub const DEFAULT_CMDBUF_SIZE: u64 = 256 * 1024;

/// Command buffer page alignment
pub const CMDBUF_ALIGN: u64 = 4096;

/// Queue priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    /// Low priority (background transfers)
    Low = 0,
    /// Medium priority (general work)
    Medium = 1,
    /// High priority (real-time rendering)
    High = 2,
}

/// Queue type for specialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    /// Graphics queue (render + fragment)
    Graphics,
    /// Compute queue (dispatch only)
    Compute,
    /// Transfer queue (copy/blit only)
    Transfer,
}

/// CSF command packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CsfPacketType {
    /// No operation
    Nop = 0x00,
    /// Wait for sync object
    Wait = 0x01,
    /// Signal sync object
    Signal = 0x02,
    /// Flush cache
    CacheFlush = 0x03,
    /// Set tiler descriptor
    SetTiler = 0x04,
    /// Draw call (indexed)
    DrawIndexed = 0x05,
    /// Draw call (non-indexed)
    Draw = 0x06,
    /// Compute dispatch
    Dispatch = 0x07,
    /// Copy buffer
    CopyBuffer = 0x08,
    /// Copy image
    CopyImage = 0x09,
    /// Blit image
    BlitImage = 0x0A,
    /// Begin render pass
    BeginRenderPass = 0x10,
    /// End render pass
    EndRenderPass = 0x11,
    /// Set shader program
    SetShaderProgram = 0x12,
    /// Set uniform remap table
    SetUniformRemap = 0x13,
    /// Set blend constants
    SetBlendConstants = 0x14,
    /// Set viewport
    SetViewport = 0x15,
    /// Set scissor
    SetScissor = 0x16,
    /// Set vertex attributes
    SetVertexAttributes = 0x17,
    /// Bind descriptor set
    BindDescriptorSet = 0x18,
    /// Bind vertex buffer
    BindVertexBuffer = 0x19,
    /// Bind index buffer
    BindIndexBuffer = 0x1A,
    /// Set stencil reference
    SetStencilRef = 0x1B,
    /// Set depth bias
    SetDepthBias = 0x1C,
    /// Push constants
    PushConstants = 0x1D,
}

/// CSF command packet header
#[derive(Debug, Clone, Copy)]
pub struct CsfPacketHeader {
    /// Packet type
    pub pkt_type: CsfPacketType,
    /// Number of 32-bit payload words following the header
    pub payload_len: u16,
    /// Reserved bits
    pub reserved: u8,
}

impl CsfPacketHeader {
    /// Create a new packet header
    pub fn new(pkt_type: CsfPacketType, payload_len: u16) -> Self {
        Self {
            pkt_type,
            payload_len,
            reserved: 0,
        }
    }

    /// Encode as a 32-bit word
    pub fn encode(&self) -> u32 {
        (self.pkt_type as u32) | ((self.payload_len as u32) << 8) | ((self.reserved as u32) << 24)
    }

    /// Decode from a 32-bit word
    pub fn decode(word: u32) -> Self {
        Self {
            pkt_type: CsfPacketType::try_from(word as u8)
                .unwrap_or(CsfPacketType::Nop),
            payload_len: ((word >> 8) & 0xFFFF) as u16,
            reserved: ((word >> 24) & 0xFF) as u8,
        }
    }
}

impl CsfPacketType {
    /// Try to convert from u8
    pub fn try_from(val: u8) -> Result<Self, ()> {
        match val {
            0x00 => Ok(Self::Nop),
            0x01 => Ok(Self::Wait),
            0x02 => Ok(Self::Signal),
            0x03 => Ok(Self::CacheFlush),
            0x04 => Ok(Self::SetTiler),
            0x05 => Ok(Self::DrawIndexed),
            0x06 => Ok(Self::Draw),
            0x07 => Ok(Self::Dispatch),
            0x08 => Ok(Self::CopyBuffer),
            0x09 => Ok(Self::CopyImage),
            0x0A => Ok(Self::BlitImage),
            0x10 => Ok(Self::BeginRenderPass),
            0x11 => Ok(Self::EndRenderPass),
            0x12 => Ok(Self::SetShaderProgram),
            0x13 => Ok(Self::SetUniformRemap),
            0x14 => Ok(Self::SetBlendConstants),
            0x15 => Ok(Self::SetViewport),
            0x16 => Ok(Self::SetScissor),
            0x17 => Ok(Self::SetVertexAttributes),
            0x18 => Ok(Self::BindDescriptorSet),
            0x19 => Ok(Self::BindVertexBuffer),
            0x1A => Ok(Self::BindIndexBuffer),
            0x1B => Ok(Self::SetStencilRef),
            0x1C => Ok(Self::SetDepthBias),
            0x1D => Ok(Self::PushConstants),
            _ => Err(()),
        }
    }
}

/// CSF Command Queue
///
/// Manages a single command queue with its ring buffer, write pointer,
/// and doorbell mechanism.
pub struct CsfQueue {
    /// Queue index within the CSF
    queue_idx: u32,
    /// Group index for this queue
    group_idx: u32,
    /// Queue type
    queue_type: QueueType,
    /// Queue priority
    priority: QueuePriority,
    /// Command buffer (ring buffer)
    cmdbuf: Option<BufferObject>,
    /// Write pointer (host side, offset into cmdbuf)
    write_ptr: AtomicU32,
    /// Read pointer cache (last known GPU read pointer)
    read_ptr_cache: AtomicU32,
    /// Command buffer size in bytes
    cmdbuf_size: u32,
    /// Number of commands submitted
    cmds_submitted: AtomicU64,
    /// Whether the queue is active
    active: AtomicU32,
}

impl CsfQueue {
    /// Create a new CSF queue handle (not yet initialized)
    pub fn new(queue_idx: u32, group_idx: u32, queue_type: QueueType, priority: QueuePriority) -> Self {
        Self {
            queue_idx,
            group_idx,
            queue_type,
            priority,
            cmdbuf: None,
            write_ptr: AtomicU32::new(0),
            read_ptr_cache: AtomicU32::new(0),
            cmdbuf_size: DEFAULT_CMDBUF_SIZE as u32,
            cmds_submitted: AtomicU64::new(0),
            active: AtomicU32::new(0),
        }
    }

    /// Get the queue index
    pub fn index(&self) -> u32 {
        self.queue_idx
    }

    /// Get the queue type
    pub fn queue_type(&self) -> QueueType {
        self.queue_type
    }

    /// Get the queue priority
    pub fn priority(&self) -> QueuePriority {
        self.priority
    }

    /// Check if the queue is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire) != 0
    }

    /// Initialize the queue with a command buffer
    pub fn init(&mut self, cmdbuf: BufferObject) -> Result<(), QueueError> {
        if self.cmdbuf.is_some() {
            return Err(QueueError::AlreadyInitialized);
        }
        self.cmdbuf_size = cmdbuf.size() as u32;
        self.cmdbuf = Some(cmdbuf);
        self.write_ptr.store(0, Ordering::Release);
        self.read_ptr_cache.store(0, Ordering::Release);
        self.active.store(1, Ordering::Release);
        debug!(
            target: LOG_TARGET,
            "CSF Queue {} (type={:?}, priority={:?}): initialized, cmdbuf={:#x} bytes",
            self.queue_idx, self.queue_type, self.priority, self.cmdbuf_size
        );
        Ok(())
    }

    /// Get available space in the command buffer (in bytes)
    pub fn available_space(&self) -> u32 {
        let write = self.write_ptr.load(Ordering::Acquire);
        let read = self.read_ptr_cache.load(Ordering::Acquire);
        if write >= read {
            self.cmdbuf_size - (write - read)
        } else {
            read - write
        }
    }

    /// Begin writing a command packet
    ///
    /// Returns the offset where the packet will be written.
    /// Caller must call `finish_packet` after writing payload.
    pub fn begin_packet(&self, pkt_type: CsfPacketType, payload_len: u16) -> u32 {
        let offset = self.write_ptr.load(Ordering::Acquire);
        trace!(
            target: LOG_TARGET,
            "Queue {}: begin_packet type={:?} payload_len={} at offset={:#x}",
            self.queue_idx, pkt_type, payload_len, offset
        );
        let header = CsfPacketHeader::new(pkt_type, payload_len);
        if let Some(ref cmdbuf) = self.cmdbuf {
            // Write header at current position
            let header_offset = offset as usize;
            if header_offset + 4 + (payload_len as usize * 4) <= self.cmdbuf_size as usize {
                let header_bytes = header.encode().to_le_bytes();
                // Safe: we checked bounds above
                unsafe {
                    let ptr = cmdbuf.mapped_ptr().unwrap().add(header_offset) as *mut u8;
                    core::ptr::copy_nonoverlapping(header_bytes.as_ptr(), ptr, 4);
                }
            }
        }
        offset
    }

    /// Finish writing a command packet and ring the doorbell
    pub fn finish_packet(&self, start_offset: u32, payload_len: u16) {
        let total_size = 4 + (payload_len as u32 * 4);
        let new_write_ptr = start_offset + total_size;
        self.write_ptr.store(new_write_ptr, Ordering::Release);
        self.cmds_submitted.fetch_add(1, Ordering::Relaxed);

        // Ring doorbell to notify GPU
        self.ring_doorbell();
    }

    /// Write a NOP packet (padding / synchronization)
    pub fn write_nop(&self, padding_words: u16) {
        let offset = self.begin_packet(CsfPacketType::Nop, padding_words);
        self.finish_packet(offset, padding_words);
    }

    /// Wait for the GPU to finish processing up to the current write pointer
    pub fn wait_idle(&self) {
        // In a real implementation, we'd wait for the GPU read pointer
        // to catch up to our write pointer. For now, this is a placeholder.
        let write = self.write_ptr.load(Ordering::Acquire);
        let mut read = self.read_ptr_cache.load(Ordering::Acquire);
        while read < write {
            // Spin-wait (in production, use a fence or interrupt)
            std::hint::spin_loop();
            read = self.read_ptr_cache.load(Ordering::Acquire);
        }
    }

    /// Ring the doorbell to notify the GPU of new commands
    fn ring_doorbell(&self) {
        trace!(
            target: LOG_TARGET,
            "Queue {}: doorbell ring at write_ptr={:#x}",
            self.queue_idx,
            self.write_ptr.load(Ordering::Acquire)
        );
        // In a real implementation, this writes to the CSF_DOORBELL register:
        // unsafe {
        //     reg_write32(gpu_base, CSF_DOORBELL, (queue_idx << 16) | write_ptr);
        // }
    }

    /// Reset the queue (after GPU reset)
    pub fn reset(&self) {
        self.write_ptr.store(0, Ordering::Release);
        self.read_ptr_cache.store(0, Ordering::Release);
        self.cmds_submitted.store(0, Ordering::Release);
    }

    /// Get statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            queue_idx: self.queue_idx,
            queue_type: self.queue_type,
            cmds_submitted: self.cmds_submitted.load(Ordering::Relaxed),
            write_ptr: self.write_ptr.load(Ordering::Acquire),
            read_ptr: self.read_ptr_cache.load(Ordering::Acquire),
        }
    }
}

/// Queue statistics
#[derive(Debug, Clone, Copy)]
pub struct QueueStats {
    /// Queue index
    pub queue_idx: u32,
    /// Queue type
    pub queue_type: QueueType,
    /// Number of commands submitted
    pub cmds_submitted: u64,
    /// Current write pointer
    pub write_ptr: u32,
    /// Current read pointer
    pub read_ptr: u32,
}

/// Queue errors
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    /// Queue already initialized
    #[error("Queue already initialized")]
    AlreadyInitialized,
    /// Queue not initialized
    #[error("Queue not initialized")]
    NotInitialized,
    /// Command buffer overflow
    #[error("Command buffer overflow: need {needed} bytes, have {available}")]
    Overflow { needed: u32, available: u32 },
    /// Invalid packet type
    #[error("Invalid CSF packet type: 0x{0:02x}")]
    InvalidPacketType(u8),
    /// Hardware error
    #[error("Hardware error: {0}")]
    HardwareError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header_encode_decode() {
        let header = CsfPacketHeader::new(CsfPacketType::Draw, 5);
        let encoded = header.encode();
        let decoded = CsfPacketHeader::decode(encoded);
        assert_eq!(decoded.pkt_type as u8, CsfPacketType::Draw as u8);
        assert_eq!(decoded.payload_len, 5);
    }

    #[test]
    fn test_queue_creation() {
        let queue = CsfQueue::new(0, 0, QueueType::Graphics, QueuePriority::High);
        assert_eq!(queue.index(), 0);
        assert_eq!(queue.queue_type(), QueueType::Graphics);
        assert!(!queue.is_active());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(QueuePriority::High > QueuePriority::Medium);
        assert!(QueuePriority::Medium > QueuePriority::Low);
    }
}
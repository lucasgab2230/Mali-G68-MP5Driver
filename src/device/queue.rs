//! Device queue management
//!
//! Provides high-level queue submission interface wrapping
//! the CSF command queues.

use crate::csf::queue::{CsfQueue, QueueType, QueueStats};
use crate::cmd::builder::CommandBufferBuilder;
use crate::LOG_TARGET;
use log::debug;

/// Device queue - wraps a CSF queue with submission logic
pub struct DeviceQueue {
    /// The underlying CSF queue
    queue: CsfQueue,
    /// Submitted command buffer count
    submitted_count: u64,
}

impl DeviceQueue {
    /// Create a new device queue from a CSF queue
    pub fn new(queue: CsfQueue) -> Self {
        Self {
            queue,
            submitted_count: 0,
        }
    }

    /// Submit a command buffer to this queue
    pub fn submit(&mut self, cmd_buf: &CommandBufferBuilder) -> Result<u64, QueueSubmitError> {
        if cmd_buf.commands().is_empty() {
            return Err(QueueSubmitError::EmptyCommandBuffer);
        }

        debug!(
            target: LOG_TARGET,
            "Queue {}: submitting command buffer ({} draws, {} dispatches, {} bytes)",
            self.queue.index(),
            cmd_buf.draw_count(),
            cmd_buf.dispatch_count(),
            cmd_buf.size_bytes()
        );

        // In production, this would:
        // 1. Copy command data to the CSF queue ring buffer
        // 2. Ring the doorbell to notify the GPU
        // 3. Return a submission ID for synchronization

        self.submitted_count += 1;
        Ok(self.submitted_count)
    }

    /// Wait for all submitted work to complete
    pub fn wait_idle(&self) {
        self.queue.wait_idle();
    }

    /// Get the queue type
    pub fn queue_type(&self) -> QueueType {
        self.queue.queue_type()
    }

    /// Get the queue statistics
    pub fn stats(&self) -> QueueStats {
        self.queue.stats()
    }

    /// Get the number of submitted command buffers
    pub fn submitted_count(&self) -> u64 {
        self.submitted_count
    }
}

/// Queue submission errors
#[derive(Debug, thiserror::Error)]
pub enum QueueSubmitError {
    /// Empty command buffer
    #[error("Cannot submit empty command buffer")]
    EmptyCommandBuffer,
    /// Queue not initialized
    #[error("Queue not initialized")]
    NotInitialized,
    /// Hardware error
    #[error("Hardware error during submission: {0}")]
    HardwareError(String),
}
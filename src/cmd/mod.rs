//! Command buffer recording and submission
//!
//! This module provides the command buffer builder that records GPU commands
//! (draw calls, compute dispatches, copies, etc.) into CSF command packets.
//!
//! ## Command Buffer Lifecycle
//!
//! 1. **Begin**: Reset the command buffer and prepare for recording
//! 2. **Record**: Record draw, compute, transfer, and sync commands
//! 3. **End**: Finalize the command buffer
//! 4. **Submit**: Submit to a CSF queue for execution

pub mod draw;
pub mod compute;
pub mod transfer;
pub mod builder;

pub use builder::CommandBufferBuilder;
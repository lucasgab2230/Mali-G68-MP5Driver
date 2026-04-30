//! Device abstraction layer
//!
//! This module provides the device abstraction that wraps the GPU
//! hardware, CSF queues, memory management, and shader compiler
//! into a coherent interface.

pub mod init;
pub mod queue;

pub use init::DeviceInit;
pub use queue::DeviceQueue;
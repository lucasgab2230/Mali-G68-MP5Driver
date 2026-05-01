//! GPU hardware abstraction layer for Mali-G68 MP5
//!
//! This module provides:
//! - Hardware register definitions for the Valhall architecture
//! - GPU identification and capability detection
//! - Tiler (bin-based rendering) configuration
//! - Shader core management

pub mod info;
pub mod regs;
pub mod tiler;

pub use info::GpuInfo;
pub use regs::*;